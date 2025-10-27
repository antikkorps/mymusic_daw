use mymusic_daw::sampler::loader::{LoopMode, Sample, SampleData};
use mymusic_daw::sampler::{SampleBank, SampleMapping};
use std::path::PathBuf;
use tempfile::tempdir;

#[test]
fn test_sample_bank_save_load_integration() {
    let dir = tempdir().unwrap();
    let bank_path = dir.path().join("test_bank.json");

    // Create test samples
    let sample1 = Sample {
        name: "Kick Drum".to_string(),
        data: SampleData::F32(vec![0.1, -0.1, 0.2, -0.2]),
        sample_rate: 48000,
        source_channels: 1,
        loop_mode: LoopMode::Off,
        loop_start: 0,
        loop_end: 4,
        reverse: false,
        volume: 1.5,
        pan: 0.0,
        pitch_offset: 0,
    };

    let sample2 = Sample {
        name: "Snare".to_string(),
        data: SampleData::F32(vec![0.3, -0.3, 0.1, -0.1]),
        sample_rate: 48000,
        source_channels: 1,
        loop_mode: LoopMode::Forward,
        loop_start: 1,
        loop_end: 3,
        reverse: true,
        volume: 1.2,
        pan: -0.5,
        pitch_offset: 2,
    };

    let samples = vec![sample1, sample2];

    // Create note mappings
    let note_mappings: Vec<Option<String>> = (0..128)
        .map(|i| match i {
            36 => Some("Kick Drum".to_string()), // C1 - kick
            38 => Some("Snare".to_string()),     // D1 - snare
            _ => None,
        })
        .collect();

    // Create bank from samples and mappings
    let bank = SampleBank::from_samples_and_mappings(
        "Test Drum Kit".to_string(),
        &samples,
        &note_mappings,
        dir.path(),
    );

    // Save bank
    bank.save_to_file(&bank_path).unwrap();

    // Load bank back
    let loaded_bank = SampleBank::load_from_file(&bank_path).unwrap();

    // Verify bank properties
    assert_eq!(loaded_bank.name, "Test Drum Kit");
    assert_eq!(loaded_bank.version, "1.0");
    assert_eq!(loaded_bank.samples.len(), 2);

    // Verify mappings
    let sorted_mappings = loaded_bank.get_sorted_mappings();
    assert_eq!(sorted_mappings.len(), 2);

    // Check kick mapping (C1/36)
    let kick_mapping = sorted_mappings.iter().find(|m| m.note == 36).unwrap();
    assert_eq!(kick_mapping.name, "Kick Drum");
    assert_eq!(kick_mapping.volume, 1.5);
    assert_eq!(kick_mapping.pan, 0.0);
    assert_eq!(kick_mapping.loop_mode, LoopMode::Off);
    assert!(!kick_mapping.reverse);
    assert_eq!(kick_mapping.pitch_offset, 0);

    // Check snare mapping (D1/38)
    let snare_mapping = sorted_mappings.iter().find(|m| m.note == 38).unwrap();
    assert_eq!(snare_mapping.name, "Snare");
    assert_eq!(snare_mapping.volume, 1.2);
    assert_eq!(snare_mapping.pan, -0.5);
    assert_eq!(snare_mapping.loop_mode, LoopMode::Forward);
    assert_eq!(snare_mapping.loop_start, 1);
    assert_eq!(snare_mapping.loop_end, 3);
    assert!(snare_mapping.reverse);
    assert_eq!(snare_mapping.pitch_offset, 2);
}

#[test]
fn test_sample_bank_empty_mappings() {
    let dir = tempdir().unwrap();
    let bank_path = dir.path().join("empty_bank.json");

    // Create empty bank
    let bank = SampleBank::new("Empty Bank".to_string());

    // Save and load
    bank.save_to_file(&bank_path).unwrap();
    let loaded_bank = SampleBank::load_from_file(&bank_path).unwrap();

    // Verify empty bank
    assert_eq!(loaded_bank.name, "Empty Bank");
    assert_eq!(loaded_bank.samples.len(), 0);
    assert_eq!(loaded_bank.get_sorted_mappings().len(), 0);
}

#[test]
fn test_sample_bank_duplicate_notes() {
    let mut bank = SampleBank::new("Test Bank".to_string());

    // Add mapping for note 60
    let mapping1 = SampleMapping {
        note: 60,
        sample_path: PathBuf::from("sample1.wav"),
        name: "Sample 1".to_string(),
        volume: 1.0,
        pan: 0.0,
        loop_mode: LoopMode::Off,
        loop_start: 0,
        loop_end: 1000,
        reverse: false,
        pitch_offset: 0,
    };

    // Add another mapping for same note 60
    let mapping2 = SampleMapping {
        note: 60,
        sample_path: PathBuf::from("sample2.wav"),
        name: "Sample 2".to_string(),
        volume: 1.5,
        pan: 0.5,
        loop_mode: LoopMode::Forward,
        loop_start: 100,
        loop_end: 900,
        reverse: true,
        pitch_offset: -2,
    };

    bank.add_mapping(mapping1);
    bank.add_mapping(mapping2); // Should replace the first one

    // Verify only one mapping exists and it's the second one
    assert_eq!(bank.samples.len(), 1);
    assert_eq!(bank.samples[0].note, 60);
    assert_eq!(bank.samples[0].name, "Sample 2");
    assert_eq!(bank.samples[0].volume, 1.5);
    assert_eq!(bank.samples[0].pan, 0.5);
}
