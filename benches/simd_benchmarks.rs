//! SIMD optimization benchmarks
//! 
//! This module benchmarks SIMD-optimized operations against scalar implementations
//! to measure performance improvements.

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use mymusic_daw::audio::simd::*;
use mymusic_daw::synth::oscillator::{SimpleOscillator, WaveformType, Oscillator};
use mymusic_daw::synth::filter::{StateVariableFilter, FilterParams};
use num_traits::Float;

fn bench_oscillator_scalar_vs_simd(c: &mut Criterion) {
    let mut group = c.benchmark_group("oscillator_generation");
    
    for &sample_count in &[64, 256, 512, 1024, 4096] {
        group.bench_with_input(
            BenchmarkId::new("scalar", sample_count),
            &sample_count,
            |b, &sample_count| {
                b.iter(|| {
                    let mut osc = SimpleOscillator::new(WaveformType::Sine, 44100.0);
                    osc.set_frequency(440.0);
                    for _ in 0..sample_count {
                        black_box(osc.next_sample());
                    }
                });
            },
        );
        
        group.bench_with_input(
            BenchmarkId::new("simd", sample_count),
            &sample_count,
            |b, &sample_count| {
                b.iter(|| {
                    let mut osc = SimdOscillator::new(440.0, 44100.0);
                    for _ in 0..(sample_count / 4) {
                        black_box(osc.next_samples());
                    }
                });
            },
        );
    }
    
    group.finish();
}

fn bench_gain_staging(c: &mut Criterion) {
    let mut group = c.benchmark_group("gain_staging");
    
    for &voice_count in &[4, 8, 16] {
        group.bench_with_input(
            BenchmarkId::new("scalar", voice_count),
            &voice_count,
            |b, &voice_count| {
                b.iter(|| {
                    let mut voices = vec![[1.0, 1.0]; voice_count];
                    let gain = 1.0 / (voice_count as f32).sqrt();
                    for voice in voices.iter_mut().take(voice_count) {
                        voice[0] *= gain;
                        voice[1] *= gain;
                    }
                    black_box(voices);
                });
            },
        );
        
        group.bench_with_input(
            BenchmarkId::new("simd", voice_count),
            &voice_count,
            |b, &voice_count| {
                b.iter(|| {
                    let mut voices = [[1.0, 1.0]; 4];
                    let active_count = voice_count.min(4);
                    simd_gain_stage_voices(&mut voices, active_count);
                    black_box(voices);
                });
            },
        );
    }
    
    group.finish();
}

fn bench_soft_clip(c: &mut Criterion) {
    let mut group = c.benchmark_group("soft_clip");
    
    for &sample_count in &[64, 256, 512, 1024, 4096] {
        group.bench_with_input(
            BenchmarkId::new("scalar", sample_count),
            &sample_count,
            |b, &sample_count| {
                b.iter(|| {
                    let mut samples = vec![0.8; sample_count];
                    for sample in samples.iter_mut() {
                        *sample = sample.tanh();
                    }
                    black_box(samples);
                });
            },
        );
        
        group.bench_with_input(
            BenchmarkId::new("simd", sample_count),
            &sample_count,
            |b, &sample_count| {
                b.iter(|| {
                    let mut samples = vec![0.8; sample_count];
                    simd_soft_clip(&mut samples);
                    black_box(samples);
                });
            },
        );
    }
    
    group.finish();
}

fn bench_flush_denormals(c: &mut Criterion) {
    let mut group = c.benchmark_group("flush_denormals");
    
    for &sample_count in &[64, 256, 512, 1024, 4096] {
        group.bench_with_input(
            BenchmarkId::new("scalar", sample_count),
            &sample_count,
            |b, &sample_count| {
                b.iter(|| {
                    let mut samples = vec![1e-15; sample_count];
                    const THRESHOLD: f32 = 1e-10;
                    for sample in samples.iter_mut() {
                        if sample.abs() < THRESHOLD {
                            *sample = 0.0;
                        }
                    }
                    black_box(samples);
                });
            },
        );
        
        group.bench_with_input(
            BenchmarkId::new("simd", sample_count),
            &sample_count,
            |b, &sample_count| {
                b.iter(|| {
                    let mut samples = vec![1e-15; sample_count];
                    simd_flush_denormals(&mut samples);
                    black_box(samples);
                });
            },
        );
    }
    
    group.finish();
}

fn bench_stereo_mixing(c: &mut Criterion) {
    let mut group = c.benchmark_group("stereo_mixing");
    
    for &sample_count in &[64, 256, 512, 1024, 4096] {
        group.bench_with_input(
            BenchmarkId::new("scalar", sample_count),
            &sample_count,
            |b, &sample_count| {
                b.iter(|| {
                    let left = vec![0.5; sample_count];
                    let right = vec![0.3; sample_count];
                    let gain = 0.8;
                    let mut output = vec![0.0; sample_count * 2];
                    
                    for i in 0..sample_count {
                        output[i * 2] = left[i] * gain;
                        output[i * 2 + 1] = right[i] * gain;
                    }
                    black_box(output);
                });
            },
        );
        
        group.bench_with_input(
            BenchmarkId::new("simd", sample_count),
            &sample_count,
            |b, &sample_count| {
                b.iter(|| {
                    let left = vec![0.5; sample_count];
                    let right = vec![0.3; sample_count];
                    let gain = 0.8;
                    let output = simd_mix_stereo(&left, &right, gain);
                    black_box(output);
                });
            },
        );
    }
    
    group.finish();
}

fn bench_filter_scalar_vs_simd(c: &mut Criterion) {
    let mut group = c.benchmark_group("filter_processing");
    
    for &sample_count in &[64, 256, 512, 1024, 4096] {
        group.bench_with_input(
            BenchmarkId::new("scalar", sample_count),
            &sample_count,
            |b, &sample_count| {
                b.iter(|| {
                    let params = FilterParams {
                        cutoff: 1000.0,
                        resonance: 1.0,
                        filter_type: mymusic_daw::synth::filter::FilterType::LowPass,
                        enabled: true,
                    };
                    let mut filter = StateVariableFilter::new(params, 44100.0);
                    
                    for _ in 0..sample_count {
                        black_box(filter.process(0.5));
                    }
                });
            },
        );
        
        group.bench_with_input(
            BenchmarkId::new("simd", sample_count),
            &sample_count,
            |b, &sample_count| {
                b.iter(|| {
                    let mut simd_filter = SimdStateVariableFilter::new(1000.0, 1.0, 44100.0);
                    
                    for _ in 0..(sample_count / 4) {
                        let inputs = [0.5, 0.5, 0.5, 0.5];
                        black_box(simd_filter.process(inputs, FilterType::LowPass));
                    }
                });
            },
        );
    }
    
    group.finish();
}

criterion_group!(
    benches,
    bench_oscillator_scalar_vs_simd,
    bench_gain_staging,
    bench_soft_clip,
    bench_flush_denormals,
    bench_stereo_mixing,
    bench_filter_scalar_vs_simd
);

criterion_main!(benches);