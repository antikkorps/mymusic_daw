use std::path::Path;
use hound::{WavReader, SampleFormat};
use claxon::FlacReader;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;
use symphonia::core::audio::{AudioBufferRef, Signal};
use symphonia::default::get_probe;
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
    pub reverse: bool,
    pub volume: f32,
    pub pan: f32,
}

pub fn load_sample(path: &Path) -> Result<Sample, String> {
    let extension = path.extension().and_then(|s| s.to_str()).unwrap_or("");

    match extension.to_lowercase().as_str() {
        "wav" => load_wav(path),
        "flac" => load_flac(path),
        "mp3" => load_mp3(path),
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
    let loop_end = resampled.len();

    Ok(Sample {
        name: path.file_name().unwrap_or_default().to_string_lossy().to_string(),
        data: SampleData::F32(resampled),
        sample_rate: TARGET_SAMPLE_RATE,
        source_channels: spec.channels,
        loop_mode: LoopMode::Off,
        loop_start: 0,
        loop_end,
        reverse: false,
        volume: 2.0, // Boost sample volume by default for better audibility
        pan: 0.0,
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
    let loop_end = resampled.len();

    Ok(Sample {
        name: path.file_name().unwrap_or_default().to_string_lossy().to_string(),
        data: SampleData::F32(resampled),
        sample_rate: TARGET_SAMPLE_RATE,
        source_channels: spec.channels as u16,
        loop_mode: LoopMode::Off,
        loop_start: 0,
        loop_end,
        reverse: false,
        volume: 2.0, // Boost sample volume by default for better audibility
        pan: 0.0,
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

fn load_mp3(path: &Path) -> Result<Sample, String> {
    // Open the file
    let file = std::fs::File::open(path).map_err(|e| format!("Failed to open file: {}", e))?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    // Create a hint to help the format registry guess what format this is
    let mut hint = Hint::new();
    if let Some(extension) = path.extension().and_then(|s| s.to_str()) {
        hint.with_extension(extension);
    }

    // Probe the format
    let probed = get_probe().format(&hint, mss, &FormatOptions::default(), &MetadataOptions::default())
        .map_err(|e| format!("Failed to probe format: {}", e))?;

    let mut format = probed.format;
    let track_id = format.tracks().iter()
        .find(|t| t.codec_params.codec != symphonia::core::codecs::CODEC_TYPE_NULL)
        .ok_or("No audio track found")?
        .id;

    let track = format.tracks().iter()
        .find(|t| t.id == track_id)
        .ok_or("Track not found")?;

    let codec_params = &track.codec_params;
    
    let sample_rate = codec_params.sample_rate.ok_or("No sample rate")?;
    let channels = codec_params.channels.ok_or("No channel info")?.count() as u16;

    // Create a decoder
    let mut decoder = symphonia::default::get_codecs()
        .make(&codec_params, &Default::default())
        .map_err(|e| format!("Failed to create decoder: {}", e))?;

    let mut samples = Vec::new();

    // Decode the entire file
    loop {
        let packet = match format.next_packet() {
            Ok(packet) => packet,
            Err(symphonia::core::errors::Error::ResetRequired) => {
                // The decoder needs to be reset
                continue;
            }
            Err(symphonia::core::errors::Error::IoError(ref e))
                if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
            Err(e) => return Err(format!("Decode error: {}", e)),
        };

        // Only decode packets from the track we're interested in
        if packet.track_id() != track_id {
            continue;
        }

        match decoder.decode(&packet) {
            Ok(decoded) => {
                // Convert the decoded audio buffer to f32 samples
                match decoded {
                    AudioBufferRef::U8(buf) => {
                        let num_channels = buf.spec().channels.count();
                        let frames = buf.frames();
                        for frame_idx in 0..frames {
                            let mut sample_sum = 0.0f32;
                            for chan_idx in 0..num_channels {
                                let sample = buf.chan(chan_idx)[frame_idx];
                                sample_sum += (sample as f32 - 128.0) / 128.0;
                            }
                            let mono_sample = sample_sum / num_channels as f32;
                            samples.push(mono_sample);
                        }
                    }
                    AudioBufferRef::U16(buf) => {
                        let num_channels = buf.spec().channels.count();
                        let frames = buf.frames();
                        for frame_idx in 0..frames {
                            let mut sample_sum = 0.0f32;
                            for chan_idx in 0..num_channels {
                                let sample = buf.chan(chan_idx)[frame_idx];
                                sample_sum += (sample as f32 - 32768.0) / 32768.0;
                            }
                            let mono_sample = sample_sum / num_channels as f32;
                            samples.push(mono_sample);
                        }
                    }
                    AudioBufferRef::U24(buf) => {
                        let num_channels = buf.spec().channels.count();
                        let frames = buf.frames();
                        for frame_idx in 0..frames {
                            let mut sample_sum = 0.0f32;
                            for chan_idx in 0..num_channels {
                                let sample = buf.chan(chan_idx)[frame_idx];
                                sample_sum += (sample.inner() as f32 - 8388608.0) / 8388608.0;
                            }
                            let mono_sample = sample_sum / num_channels as f32;
                            samples.push(mono_sample);
                        }
                    }
                    AudioBufferRef::U32(buf) => {
                        let num_channels = buf.spec().channels.count();
                        let frames = buf.frames();
                        for frame_idx in 0..frames {
                            let mut sample_sum = 0.0f32;
                            for chan_idx in 0..num_channels {
                                let sample = buf.chan(chan_idx)[frame_idx];
                                sample_sum += (sample as f32 - 2147483648.0) / 2147483648.0;
                            }
                            let mono_sample = sample_sum / num_channels as f32;
                            samples.push(mono_sample);
                        }
                    }
                    AudioBufferRef::S8(buf) => {
                        let num_channels = buf.spec().channels.count();
                        let frames = buf.frames();
                        for frame_idx in 0..frames {
                            let mut sample_sum = 0.0f32;
                            for chan_idx in 0..num_channels {
                                let sample = buf.chan(chan_idx)[frame_idx];
                                sample_sum += sample as f32 / 128.0;
                            }
                            let mono_sample = sample_sum / num_channels as f32;
                            samples.push(mono_sample);
                        }
                    }
                    AudioBufferRef::S16(buf) => {
                        let num_channels = buf.spec().channels.count();
                        let frames = buf.frames();
                        for frame_idx in 0..frames {
                            let mut sample_sum = 0.0f32;
                            for chan_idx in 0..num_channels {
                                let sample = buf.chan(chan_idx)[frame_idx];
                                sample_sum += sample as f32 / 32768.0;
                            }
                            let mono_sample = sample_sum / num_channels as f32;
                            samples.push(mono_sample);
                        }
                    }
                    AudioBufferRef::S24(buf) => {
                        let num_channels = buf.spec().channels.count();
                        let frames = buf.frames();
                        for frame_idx in 0..frames {
                            let mut sample_sum = 0.0f32;
                            for chan_idx in 0..num_channels {
                                let sample = buf.chan(chan_idx)[frame_idx];
                                sample_sum += sample.inner() as f32 / 8388608.0;
                            }
                            let mono_sample = sample_sum / num_channels as f32;
                            samples.push(mono_sample);
                        }
                    }
                    AudioBufferRef::S32(buf) => {
                        let num_channels = buf.spec().channels.count();
                        let frames = buf.frames();
                        for frame_idx in 0..frames {
                            let mut sample_sum = 0.0f32;
                            for chan_idx in 0..num_channels {
                                let sample = buf.chan(chan_idx)[frame_idx];
                                sample_sum += sample as f32 / 2147483648.0;
                            }
                            let mono_sample = sample_sum / num_channels as f32;
                            samples.push(mono_sample);
                        }
                    }
                    AudioBufferRef::F32(buf) => {
                        let num_channels = buf.spec().channels.count();
                        let frames = buf.frames();
                        for frame_idx in 0..frames {
                            let mut sample_sum = 0.0f32;
                            for chan_idx in 0..num_channels {
                                let sample = buf.chan(chan_idx)[frame_idx];
                                sample_sum += sample;
                            }
                            let mono_sample = sample_sum / num_channels as f32;
                            samples.push(mono_sample);
                        }
                    }
                    AudioBufferRef::F64(buf) => {
                        let num_channels = buf.spec().channels.count();
                        let frames = buf.frames();
                        for frame_idx in 0..frames {
                            let mut sample_sum = 0.0f32;
                            for chan_idx in 0..num_channels {
                                let sample = buf.chan(chan_idx)[frame_idx];
                                sample_sum += sample as f32;
                            }
                            let mono_sample = sample_sum / num_channels as f32;
                            samples.push(mono_sample);
                        }
                    }
                }
            }
            Err(symphonia::core::errors::Error::IoError(ref e))
                if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
            Err(symphonia::core::errors::Error::DecodeError(_)) => {
                // Skip decode errors that may occur with some MP3 files
                continue;
            }
            Err(e) => return Err(format!("Decode error: {}", e)),
        }
    }

    if samples.is_empty() {
        return Err("No samples decoded".to_string());
    }

    // Resample if needed
    let resampled = resample_if_needed(samples, sample_rate, TARGET_SAMPLE_RATE)?;
    let loop_end = resampled.len();

    Ok(Sample {
        name: path.file_name().unwrap_or_default().to_string_lossy().to_string(),
        data: SampleData::F32(resampled),
        sample_rate: TARGET_SAMPLE_RATE,
        source_channels: channels,
        loop_mode: LoopMode::Off,
        loop_start: 0,
        loop_end,
        reverse: false,
        volume: 2.0, // Boost sample volume by default for better audibility
        pan: 0.0,
    })
}