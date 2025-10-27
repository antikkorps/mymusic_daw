use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use mymusic_daw::audio::timing::AudioTiming;
use mymusic_daw::synth::filter::{FilterParams, FilterType, StateVariableFilter};
use mymusic_daw::synth::modulation::{ModDestination, ModRouting, ModSource, ModulationMatrix};
use mymusic_daw::synth::oscillator::{Oscillator, SimpleOscillator, WaveformType};
use mymusic_daw::synth::voice::Voice;
use mymusic_daw::synth::voice_manager::VoiceManager;

/// Benchmark oscillator generation (critical for real-time performance)
fn bench_oscillator_generation(c: &mut Criterion) {
    let mut group = c.benchmark_group("oscillator");
    let sample_rate = 48000.0;
    let buffer_size = 512;

    for waveform in [
        WaveformType::Sine,
        WaveformType::Square,
        WaveformType::Saw,
        WaveformType::Triangle,
    ] {
        let mut osc = SimpleOscillator::new(waveform, sample_rate);
        osc.set_frequency(440.0);

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{:?}", waveform)),
            &buffer_size,
            |b, &size| {
                b.iter(|| {
                    for _ in 0..size {
                        black_box(osc.next_sample());
                    }
                });
            },
        );
    }
    group.finish();
}

/// Benchmark voice processing (polyphony is critical)
fn bench_voice_processing(c: &mut Criterion) {
    let sample_rate = 48000.0;
    let buffer_size = 512;

    c.bench_function("voice_single_note", |b| {
        let mut voice = Voice::new(sample_rate);
        voice.note_on(69, 100, 0); // A4, velocity 100

        b.iter(|| {
            for _ in 0..buffer_size {
                black_box(voice.next_sample());
            }
        });
    });
}

/// Benchmark VoiceManager with polyphony
fn bench_voice_manager(c: &mut Criterion) {
    let mut group = c.benchmark_group("voice_manager");
    let sample_rate = 48000.0;
    let buffer_size = 512;

    for num_voices in [1, 4, 8, 16] {
        let mut vm = VoiceManager::new(sample_rate);

        // Trigger num_voices notes
        for i in 0..num_voices {
            vm.note_on(60 + i, 100);
        }

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}_voices", num_voices)),
            &buffer_size,
            |b, &size| {
                b.iter(|| {
                    for _ in 0..size {
                        black_box(vm.next_sample());
                    }
                });
            },
        );
    }
    group.finish();
}

/// Benchmark MIDI event processing (timing-critical)
fn bench_midi_processing(c: &mut Criterion) {
    let sample_rate = 48000.0;
    let mut vm = VoiceManager::new(sample_rate);

    c.bench_function("midi_note_on_off", |b| {
        b.iter(|| {
            vm.note_on(black_box(60), black_box(100));
            vm.note_off(black_box(60));
        });
    });
}

/// Benchmark AudioTiming conversions (used in MIDI timing)
fn bench_audio_timing(c: &mut Criterion) {
    let timing = AudioTiming::new(48000.0);

    c.bench_function("micros_to_samples", |b| {
        b.iter(|| {
            black_box(timing.micros_to_samples(black_box(1000)));
        });
    });
}

/// Benchmark MIDI → Audio latency (critical for real-time performance)
/// Target: < 10ms total latency for professional DAW use
fn bench_midi_to_audio_latency(c: &mut Criterion) {
    let sample_rate = 48000.0;
    let buffer_size = 512;

    // Buffer latency alone = 512 / 48000 = 10.67ms
    // So processing must be VERY fast to stay under 10ms total

    c.bench_function("midi_to_audio_full_pipeline", |b| {
        let mut vm = VoiceManager::new(sample_rate);

        b.iter(|| {
            // 1. MIDI event processing
            vm.note_on(black_box(60), black_box(100));

            // 2. Generate one buffer worth of audio
            for _ in 0..buffer_size {
                black_box(vm.next_sample());
            }

            // 3. Turn off note
            vm.note_off(black_box(60));
        });
    });

    // Benchmark different buffer sizes (affects latency)
    let mut group = c.benchmark_group("latency_by_buffer_size");
    for &size in &[128, 256, 512, 1024, 2048] {
        let mut vm = VoiceManager::new(sample_rate);
        vm.note_on(60, 100);

        // Calculate expected latency
        let latency_ms = (size as f32 / sample_rate) * 1000.0;

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}samples_{}ms", size, latency_ms as u32)),
            &size,
            |b, &size| {
                b.iter(|| {
                    for _ in 0..size {
                        black_box(vm.next_sample());
                    }
                });
            },
        );
    }
    group.finish();
}

/// Benchmark filter processing (different filter types)
fn bench_filter_types(c: &mut Criterion) {
    let mut group = c.benchmark_group("filter_types");
    let sample_rate = 48000.0;
    let buffer_size = 512;

    for filter_type in [
        FilterType::LowPass,
        FilterType::HighPass,
        FilterType::BandPass,
        FilterType::Notch,
    ] {
        let params = FilterParams {
            cutoff: 1000.0,
            resonance: 2.0,
            filter_type,
            enabled: true,
        };
        let mut filter = StateVariableFilter::new(params, sample_rate);

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{:?}", filter_type)),
            &buffer_size,
            |b, &size| {
                b.iter(|| {
                    for _ in 0..size {
                        black_box(filter.process(black_box(0.5)));
                    }
                });
            },
        );
    }
    group.finish();
}

/// Benchmark filter with different resonance values
fn bench_filter_resonance(c: &mut Criterion) {
    let mut group = c.benchmark_group("filter_resonance");
    let sample_rate = 48000.0;
    let buffer_size = 512;

    for resonance in [0.707, 2.0, 5.0, 10.0] {
        let params = FilterParams {
            cutoff: 1000.0,
            resonance,
            filter_type: FilterType::LowPass,
            enabled: true,
        };
        let mut filter = StateVariableFilter::new(params, sample_rate);

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("Q_{}", resonance)),
            &buffer_size,
            |b, &size| {
                b.iter(|| {
                    for _ in 0..size {
                        black_box(filter.process(black_box(0.5)));
                    }
                });
            },
        );
    }
    group.finish();
}

/// Benchmark filter with modulated cutoff
fn bench_filter_modulation(c: &mut Criterion) {
    let sample_rate = 48000.0;
    let buffer_size = 512;

    c.bench_function("filter_modulated_cutoff", |b| {
        let params = FilterParams {
            cutoff: 500.0,
            resonance: 2.0,
            filter_type: FilterType::LowPass,
            enabled: true,
        };
        let mut filter = StateVariableFilter::new(params, sample_rate);

        b.iter(|| {
            for i in 0..buffer_size {
                // Simulate LFO modulation: cutoff varies from 250Hz to 2000Hz
                let modulated_cutoff = 500.0 + 500.0 * (i as f32 / 100.0).sin();
                black_box(filter.process_modulated(black_box(0.5), black_box(modulated_cutoff)));
            }
        });
    });
}

/// Benchmark voice with and without filter
fn bench_voice_filter_overhead(c: &mut Criterion) {
    let mut group = c.benchmark_group("voice_filter_overhead");
    let sample_rate = 48000.0;
    let buffer_size = 512;

    // Voice without filter (bypassed)
    {
        let mut voice = Voice::new(sample_rate);
        let mut filter_params = FilterParams::default();
        filter_params.enabled = false; // Bypass
        voice.set_filter(filter_params);
        voice.note_on(60, 100, 0);

        group.bench_function("voice_filter_bypassed", |b| {
            b.iter(|| {
                for _ in 0..buffer_size {
                    black_box(voice.next_sample());
                }
            });
        });
    }

    // Voice with filter enabled
    {
        let mut voice = Voice::new(sample_rate);
        let filter_params = FilterParams {
            cutoff: 1000.0,
            resonance: 2.0,
            filter_type: FilterType::LowPass,
            enabled: true,
        };
        voice.set_filter(filter_params);
        voice.note_on(60, 100, 0);

        group.bench_function("voice_filter_enabled", |b| {
            b.iter(|| {
                for _ in 0..buffer_size {
                    black_box(voice.next_sample());
                }
            });
        });
    }

    group.finish();
}

/// Benchmark voice with filter and modulation matrix
fn bench_voice_filter_with_modulation(c: &mut Criterion) {
    let sample_rate = 48000.0;
    let buffer_size = 512;

    c.bench_function("voice_envelope_to_filter", |b| {
        let mut voice = Voice::new(sample_rate);
        let filter_params = FilterParams {
            cutoff: 200.0,
            resonance: 2.0,
            filter_type: FilterType::LowPass,
            enabled: true,
        };
        voice.set_filter(filter_params);

        let mut matrix = ModulationMatrix::new_empty();
        matrix.set_routing(
            0,
            ModRouting {
                source: ModSource::Envelope,
                destination: ModDestination::FilterCutoff,
                amount: 10.0,
                enabled: true,
            },
        );

        voice.note_on(60, 100, 0);

        b.iter(|| {
            for _ in 0..buffer_size {
                black_box(voice.next_sample_with_matrix(black_box(&matrix)));
            }
        });
    });
}

/// Benchmark polyphonic voices with filters
fn bench_polyphony_with_filters(c: &mut Criterion) {
    let mut group = c.benchmark_group("polyphony_filters");
    let sample_rate = 48000.0;
    let buffer_size = 512;

    for num_voices in [1, 4, 8, 16] {
        let mut vm = VoiceManager::new(sample_rate);

        // Enable filter on all voices
        let filter_params = FilterParams {
            cutoff: 1000.0,
            resonance: 2.0,
            filter_type: FilterType::LowPass,
            enabled: true,
        };
        vm.set_filter(filter_params);

        // Trigger num_voices notes
        for i in 0..num_voices {
            vm.note_on(60 + i, 100);
        }

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}_voices", num_voices)),
            &buffer_size,
            |b, &size| {
                b.iter(|| {
                    for _ in 0..size {
                        black_box(vm.next_sample());
                    }
                });
            },
        );
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_oscillator_generation,
    bench_voice_processing,
    bench_voice_manager,
    bench_midi_processing,
    bench_audio_timing,
    bench_midi_to_audio_latency,
    bench_filter_types,
    bench_filter_resonance,
    bench_filter_modulation,
    bench_voice_filter_overhead,
    bench_voice_filter_with_modulation,
    bench_polyphony_with_filters
);
criterion_main!(benches);
