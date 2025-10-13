use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use mymusic_daw::audio::timing::AudioTiming;
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
        voice.note_on(69, 100); // A4, velocity 100

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

criterion_group!(
    benches,
    bench_oscillator_generation,
    bench_voice_processing,
    bench_voice_manager,
    bench_midi_processing,
    bench_audio_timing,
    bench_midi_to_audio_latency
);
criterion_main!(benches);
