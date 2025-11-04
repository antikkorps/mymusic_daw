// Types for project persistence

use serde::{Deserialize, Serialize};

use crate::sampler::bank::SampleBank;
use crate::sequencer::note::NoteId;

/// Project version information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProjectVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl ProjectVersion {
    pub fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }

    pub fn current() -> Self {
        Self::new(1, 2, 0) // Version 1.2.0 (to test migration)
    }
}

impl std::fmt::Display for ProjectVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// Project metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectMetadata {
    /// Project name
    pub name: String,
    /// Version of the project format
    pub version: ProjectVersion,
    /// Creation timestamp
    pub created: String,
    /// Last modification timestamp
    pub modified: String,
    /// Default tempo (BPM)
    pub tempo: f64,
    /// Default time signature
    pub time_signature: crate::sequencer::timeline::TimeSignature,
    /// Sample rate used for the project
    pub sample_rate: f64,
    /// Author/creator information
    pub author: Option<String>,
    /// Project description
    pub description: Option<String>,
    /// Metronome enabled (v1.1+)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metronome_enabled: Option<bool>,
    /// Metronome volume (v1.1+)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metronome_volume: Option<f32>,
    /// Loop enabled (v1.2+)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub loop_enabled: Option<bool>,
    /// Loop start in bars (v1.2+)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub loop_start_bars: Option<u32>,
    /// Loop end in bars (v1.2+)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub loop_end_bars: Option<u32>,
}

/// Serializable pattern structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternSerializable {
    /// Pattern identifier (matches Pattern.id)
    pub id: crate::sequencer::pattern::PatternId,
    /// Pattern name
    pub name: String,
    /// Pattern length in bars
    pub length_bars: u32,
    /// Serialized notes (only data needed for recreation)
    pub notes: Vec<SerializableNote>,
}

/// Serializable note structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializableNote {
    /// Note identifier
    pub id: NoteId,
    /// MIDI note number (0-127)
    pub pitch: u8,
    /// Start position in samples
    pub start_samples: u64,
    /// Duration in samples
    pub duration_samples: u64,
    /// MIDI velocity (0-127)
    pub velocity: u8,
}

/// Track configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Track {
    /// Track identifier
    pub id: u32,
    /// Track name
    pub name: String,
    /// Associated pattern ID (None for empty track)
    pub pattern_id: Option<crate::sequencer::pattern::PatternId>,
    /// Track color (optional, for UI)
    pub color: Option<[u8; 3]>,
    /// Track volume (0.0 - 2.0)
    pub volume: f32,
    /// Track pan (-1.0 left, 0.0 center, 1.0 right)
    pub pan: f32,
    /// Track is muted
    pub muted: bool,
    /// Track is soloed
    pub soloed: bool,
    /// Track type
    pub track_type: TrackType,
}

/// Track type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TrackType {
    /// Synthesizer track
    Synth,
    /// Sampler track
    Sampler,
    /// Audio track (future)
    Audio,
    /// MIDI track (future)
    Midi,
}

/// Synthesizer parameters (simplified version for persistence)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SynthParams {
    /// Volume (0.0 - 2.0)
    pub volume: f32,
    /// Global pan (-1.0 left, 0.0 center, 1.0 right)
    pub pan: f32,
    /// Pan spread for polyphony (0.0 - 1.0)
    pub pan_spread: f32,
    /// Waveform type
    pub waveform: crate::synth::oscillator::WaveformType,
    /// ADSR envelope parameters
    pub adsr: crate::synth::envelope::AdsrParams,
    /// LFO parameters
    pub lfo: crate::synth::lfo::LfoParams,
    /// Filter parameters
    pub filter: crate::synth::filter::FilterParams,
    /// Portamento/glide parameters
    pub portamento: crate::synth::portamento::PortamentoParams,
    /// Polyphony mode
    pub poly_mode: crate::synth::poly_mode::PolyMode,
    /// Effect chain (simplified)
    pub effects: EffectChainSerializable,
}

/// Serializable effect chain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EffectChainSerializable {
    /// Delay effect parameters
    pub delay: Option<crate::synth::delay::DelayParams>,
    /// Reverb effect parameters
    pub reverb: Option<crate::synth::reverb::ReverbParams>,
    /// Filter is enabled
    pub filter_enabled: bool,
    /// Delay is enabled
    pub delay_enabled: bool,
    /// Reverb is enabled
    pub reverb_enabled: bool,
}

/// Main project structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    /// Project metadata
    pub metadata: ProjectMetadata,
    /// All tracks in the project
    pub tracks: std::collections::HashMap<u32, Track>,
    /// All patterns in the project
    pub patterns:
        std::collections::HashMap<crate::sequencer::pattern::PatternId, PatternSerializable>,
    /// Synthesizer parameters
    pub synth_params: SynthParams,
    /// Sample bank configuration (if any)
    pub sample_bank: Option<SampleBank>,
}

impl Default for Project {
    fn default() -> Self {
        let now = chrono::Utc::now();

        Self {
            metadata: ProjectMetadata {
                name: "Untitled Project".to_string(),
                version: ProjectVersion::current(),
                created: now.to_rfc3339(),
                modified: now.to_rfc3339(),
                tempo: 120.0,
                time_signature: crate::sequencer::timeline::TimeSignature::four_four(),
                sample_rate: 48000.0,
                author: None,
                description: None,
                metronome_enabled: Some(true),
                metronome_volume: Some(0.5),
                loop_enabled: Some(false),
                loop_start_bars: Some(1),
                loop_end_bars: Some(8),
            },
            tracks: std::collections::HashMap::new(),
            patterns: std::collections::HashMap::new(),
            synth_params: SynthParams {
                volume: 0.8,
                pan: 0.0,
                pan_spread: 0.0,
                waveform: crate::synth::oscillator::WaveformType::Sine,
                adsr: crate::synth::envelope::AdsrParams::new(0.01, 0.1, 0.7, 0.3),
                lfo: crate::synth::lfo::LfoParams::default(),
                filter: crate::synth::filter::FilterParams::default(),
                portamento: crate::synth::portamento::PortamentoParams::default(),
                poly_mode: crate::synth::poly_mode::PolyMode::default(),
                effects: EffectChainSerializable {
                    delay: None,
                    reverb: None,
                    filter_enabled: true,
                    delay_enabled: false,
                    reverb_enabled: false,
                },
            },
            sample_bank: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_project_version() {
        let version = ProjectVersion::new(1, 2, 3);
        assert_eq!(version.major, 1);
        assert_eq!(version.minor, 2);
        assert_eq!(version.patch, 3);
        assert_eq!(version.to_string(), "1.2.3");

        let current = ProjectVersion::current();
        assert!(current.major >= 1);
    }

    #[test]
    fn test_project_metadata_defaults() {
        let metadata = ProjectMetadata {
            name: "Test".to_string(),
            version: ProjectVersion::current(),
            created: "2023-01-01T00:00:00Z".to_string(),
            modified: "2023-01-01T00:00:00Z".to_string(),
            tempo: 120.0,
            time_signature: crate::sequencer::timeline::TimeSignature::four_four(),
            sample_rate: 48000.0,
            author: Some("Test Author".to_string()),
            description: Some("Test Description".to_string()),
            metronome_enabled: Some(true),
            metronome_volume: Some(0.5),
            loop_enabled: Some(false),
            loop_start_bars: Some(1),
            loop_end_bars: Some(8),
        };

        assert_eq!(metadata.name, "Test");
        assert_eq!(metadata.author, Some("Test Author".to_string()));
        assert_eq!(metadata.tempo, 120.0);
        assert_eq!(metadata.sample_rate, 48000.0);
    }

    #[test]
    fn test_pattern_serializable() {
        let pattern = PatternSerializable {
            id: 42,
            name: "Test Pattern".to_string(),
            length_bars: 4,
            notes: vec![SerializableNote {
                id: 1,
                pitch: 60,
                start_samples: 0,
                duration_samples: 48000,
                velocity: 100,
            }],
        };

        assert_eq!(pattern.id, 42);
        assert_eq!(pattern.name, "Test Pattern");
        assert_eq!(pattern.length_bars, 4);
        assert_eq!(pattern.notes.len(), 1);
        assert_eq!(pattern.notes[0].pitch, 60);
    }

    #[test]
    fn test_track_types() {
        use crate::project::types::TrackType;

        let synth_track = Track {
            id: 1,
            name: "Synth".to_string(),
            pattern_id: None,
            color: Some([255, 0, 0]),
            volume: 0.8,
            pan: 0.0,
            muted: false,
            soloed: false,
            track_type: TrackType::Synth,
        };

        let sampler_track = Track {
            id: 2,
            name: "Sampler".to_string(),
            pattern_id: None,
            color: Some([0, 255, 0]),
            volume: 0.9,
            pan: 0.1,
            muted: false,
            soloed: true,
            track_type: TrackType::Sampler,
        };

        assert_eq!(synth_track.track_type, TrackType::Synth);
        assert_eq!(sampler_track.track_type, TrackType::Sampler);
        assert!(!synth_track.soloed);
        assert!(sampler_track.soloed);
    }

    #[test]
    fn test_synth_params_default() {
        let params = SynthParams {
            volume: 1.0,
            pan: 0.0,
            pan_spread: 0.0,
            waveform: crate::synth::oscillator::WaveformType::Sine,
            adsr: crate::synth::envelope::AdsrParams::new(0.01, 0.1, 0.7, 0.3),
            lfo: crate::synth::lfo::LfoParams::default(),
            filter: crate::synth::filter::FilterParams::default(),
            portamento: crate::synth::portamento::PortamentoParams::default(),
            poly_mode: crate::synth::poly_mode::PolyMode::default(),
            effects: EffectChainSerializable {
                delay: None,
                reverb: None,
                filter_enabled: true,
                delay_enabled: false,
                reverb_enabled: false,
            },
        };

        assert_eq!(params.volume, 1.0);
        assert_eq!(params.pan, 0.0);
        assert_eq!(
            params.waveform,
            crate::synth::oscillator::WaveformType::Sine
        );
        assert!(params.effects.filter_enabled);
        assert!(!params.effects.delay_enabled);
    }
}
