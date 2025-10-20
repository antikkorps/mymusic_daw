
use std::path::Path;
use hound::{WavReader, WavSpec};
use claxon::{FlacReader, FlacReaderOptions};

pub enum SampleData {
    F32(Vec<f32>),
    // Other formats if needed later
}

pub struct Sample {
    pub name: String,
    pub data: SampleData,
    pub sample_rate: u32,
    pub channels: u16,
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
    // TODO: Implementation for loading WAV files
    let reader = WavReader::open(path).map_err(|e| e.to_string())?;
    let spec = reader.spec();

    let samples: Vec<f32> = reader
        .into_samples::<i16>() // Assuming i16 samples for now
        .filter_map(Result::ok)
        .map(|s| s as f32 / i16::MAX as f32)
        .collect();

    Ok(Sample {
        name: path.file_name().unwrap_or_default().to_string_lossy().to_string(),
        data: SampleData::F32(samples),
        sample_rate: spec.sample_rate,
        channels: spec.channels,
    })
}

fn load_flac(path: &Path) -> Result<Sample, String> {
    // TODO: Implementation for loading FLAC files
    let mut reader = FlacReader::open(path).map_err(|e| e.to_string())?;
    let spec = reader.streaminfo();

    let samples: Vec<f32> = reader
        .samples()
        .filter_map(Result::ok)
        .map(|s| s as f32 / (1 << (spec.bits_per_sample - 1)) as f32)
        .collect();

    Ok(Sample {
        name: path.file_name().unwrap_or_default().to_string_lossy().to_string(),
        data: SampleData::F32(samples),
        sample_rate: spec.sample_rate,
        channels: spec.channels as u16,
    })
}

