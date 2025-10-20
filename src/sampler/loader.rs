use std::path::Path;
use hound::{WavReader, SampleFormat};
use claxon::FlacReader;
use rubato::{Resampler, SincFixedIn, SincInterpolationParameters, SincInterpolationType, WindowFunction};

const TARGET_SAMPLE_RATE: u32 = 48000;

#[derive(Debug, Clone)]
pub enum SampleData {
    F32(Vec<f32>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoopMode {
    Off,
    Forward,
}

#[derive(Debug, Clone)]
pub struct Sample {
    pub name: String,
    pub data: SampleData,
    pub sample_rate: u32,
    pub source_channels: u16,
    pub loop_mode: LoopMode,
    pub loop_start: usize,
    pub loop_end: usize,
}

pub fn load_sample(path: &Path) -> Result<Sample, String> {
    let extension = path.extension().and_then(|s| s.to_str()).unwrap_or("");

    match extension.to_lowercase().as_str() {
        "wav" => load_wav(path),
        "flac" => load_flac(path),
        _ => Err(format!("Unsupported file format: {}", extension)),
    }
}

fn load_wav(path: &Path) -> Result<Sample, String> {
    let mut reader = WavReader::open(path).map_err(|e| e.to_string())?;
    let spec = reader.spec();

    let samples_mono: Vec<f32> = match (spec.sample_format, spec.bits_per_sample) {
        (SampleFormat::Int, 16) => {
            let samples = reader.samples::<i16>().filter_map(Result::ok).collect::<Vec<_>>();
            if spec.channels == 2 {
                samples.chunks_exact(2).map(|chunk| (chunk[0] as f32 + chunk[1] as f32) * 0.5 / i16::MAX as f32).collect()
            } else {
                samples.into_iter().map(|s| s as f32 / i16::MAX as f32).collect()
            }
        },
        (SampleFormat::Int, 24) | (SampleFormat::Int, 32) => {
            let samples = reader.samples::<i32>().filter_map(Result::ok).collect::<Vec<_>>();
            let divisor = (1 << (spec.bits_per_sample - 1)) as f32;
            if spec.channels == 2 {
                samples.chunks_exact(2).map(|chunk| (chunk[0] as f32 + chunk[1] as f32) * 0.5 / divisor).collect()
            } else {
                samples.into_iter().map(|s| s as f32 / divisor).collect()
            }
        },
        (SampleFormat::Float, 32) => {
            let samples = reader.samples::<f32>().filter_map(Result::ok).collect::<Vec<_>>();
            if spec.channels == 2 {
                samples.chunks_exact(2).map(|chunk| (chunk[0] + chunk[1]) * 0.5).collect()
            } else {
                samples
            }
        },
        _ => return Err(format!("Unsupported WAV format: {:?}, {} bits", spec.sample_format, spec.bits_per_sample)),
    };

    let resampled = resample_if_needed(samples_mono, spec.sample_rate, TARGET_SAMPLE_RATE)?;

    Ok(Sample {
        name: path.file_name().unwrap_or_default().to_string_lossy().to_string(),
        data: SampleData::F32(resampled),
        sample_rate: TARGET_SAMPLE_RATE,
        source_channels: spec.channels,
        loop_mode: LoopMode::Off,
        loop_start: 0,
        loop_end: 0,
    })
}

fn load_flac(path: &Path) -> Result<Sample, String> {
    let mut reader = FlacReader::open(path).map_err(|e| e.to_string())?;
    let spec = reader.streaminfo();
    let divisor = (1 << (spec.bits_per_sample - 1)) as f32;

    let samples = reader.samples().filter_map(Result::ok).collect::<Vec<_>>();

    let samples_mono: Vec<f32> = if spec.channels == 2 {
        samples.chunks_exact(2).map(|chunk| (chunk[0] as f32 + chunk[1] as f32) * 0.5 / divisor).collect()
    } else {
        samples.into_iter().map(|s| s as f32 / divisor).collect()
    };

    let resampled = resample_if_needed(samples_mono, spec.sample_rate, TARGET_SAMPLE_RATE)?;

    Ok(Sample {
        name: path.file_name().unwrap_or_default().to_string_lossy().to_string(),
        data: SampleData::F32(resampled),
        sample_rate: TARGET_SAMPLE_RATE,
        source_channels: spec.channels as u16,
        loop_mode: LoopMode::Off,
        loop_start: 0,
        loop_end: 0,
    })
}

fn resample_if_needed(samples: Vec<f32>, source_rate: u32, target_rate: u32) -> Result<Vec<f32>, String> {
    if source_rate == target_rate {
        return Ok(samples);
    }

    let params = SincInterpolationParameters {
        sinc_len: 256,
        f_cutoff: 0.95,
        interpolation: SincInterpolationType::Linear,
        oversampling_factor: 256,
        window: WindowFunction::BlackmanHarris2,
    };

    let mut resampler = SincFixedIn::<f32>::new(
        target_rate as f64 / source_rate as f64,
        2.0,
        params,
        samples.len(),
        1,
    ).map_err(|e| e.to_string())?;

    let waves_in = vec![samples];
    let waves_out = resampler.process(&waves_in, None).map_err(|e| e.to_string())?;

    Ok(waves_out.into_iter().next().unwrap())
}