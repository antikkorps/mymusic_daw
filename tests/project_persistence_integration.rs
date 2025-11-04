// Integration test for project persistence system
// Tests the complete save/load cycle with realistic data

use mymusic_daw::project::{ProjectLoadOptions, ProjectManager};
use mymusic_daw::sequencer::pattern::Pattern;
use mymusic_daw::sequencer::{Note, Position, Tempo, TimeSignature, generate_note_id};
use mymusic_daw::synth::envelope::AdsrParams;
use mymusic_daw::synth::oscillator::WaveformType;

#[test]
fn test_complete_project_persistence() {
    // Create project manager
    let manager = ProjectManager::new(48000.0);

    // Create a new project
    let mut project = manager.create_new_project("Integration Test Project".to_string());
    project.metadata.author = Some("Test User".to_string());
    project.metadata.description = Some("A test project for integration testing".to_string());
    project.metadata.tempo = 120.0;

    // Modify synth parameters
    project.synth_params.waveform = WaveformType::Saw;
    project.synth_params.volume = 0.85;
    project.synth_params.adsr = AdsrParams::new(0.1, 0.3, 0.7, 0.5);

    // Create a pattern with some notes
    let pattern_id = mymusic_daw::project::generate_pattern_id();
    let mut pattern = Pattern::new(pattern_id, "Melody Pattern".to_string(), 4);

    // Add some notes to create a simple melody
    let tempo = Tempo::new(120.0);
    let time_sig = TimeSignature::four_four();
    let sample_rate = 48000.0;

    let notes = vec![
        Note::new(
            generate_note_id(),
            60,
            Position::from_samples(0, sample_rate, &tempo, &time_sig),
            sample_rate as u64, // 1 second duration
            127,
        ), // C4
        Note::new(
            generate_note_id(),
            62,
            Position::from_samples(sample_rate as u64, sample_rate, &tempo, &time_sig),
            sample_rate as u64, // 1 second duration
            127,
        ), // D4
        Note::new(
            generate_note_id(),
            64,
            Position::from_samples((sample_rate * 2.0) as u64, sample_rate, &tempo, &time_sig),
            sample_rate as u64, // 1 second duration
            127,
        ), // E4
        Note::new(
            generate_note_id(),
            65,
            Position::from_samples((sample_rate * 3.0) as u64, sample_rate, &tempo, &time_sig),
            sample_rate as u64, // 1 second duration
            127,
        ), // F4
    ];

    for note in notes {
        pattern.add_note(note);
    }

    // Convert pattern to serializable form and add to project
    let serializable_pattern =
        mymusic_daw::project::serialization::pattern_to_serializable(&pattern);
    project.patterns.insert(pattern_id, serializable_pattern);

    // Update track to reference the pattern
    if let Some(track) = project.tracks.get_mut(&0) {
        track.pattern_id = Some(pattern_id);
        track.name = "Melody Track".to_string();
        track.volume = 0.9;
    }

    // Save project
    let project_path = std::env::temp_dir().join("integration_test_project.mymusic");
    manager
        .save_project(&project, &project_path)
        .expect("Failed to save project");

    // Verify file was created and has reasonable size
    assert!(project_path.exists());
    let metadata = std::fs::metadata(&project_path).expect("Failed to get file metadata");
    assert!(metadata.len() > 100); // Should be more than 100 bytes

    // Load project with validation
    let options = ProjectLoadOptions {
        validate: true,
        load_samples: true,
        sample_rate_override: None,
    };

    let loaded_project = manager
        .load_project(&project_path, &options)
        .expect("Failed to load project");

    // Verify project metadata
    assert_eq!(loaded_project.metadata.name, "Integration Test Project");
    assert_eq!(
        loaded_project.metadata.author,
        Some("Test User".to_string())
    );
    assert_eq!(
        loaded_project.metadata.description,
        Some("A test project for integration testing".to_string())
    );
    assert_eq!(loaded_project.metadata.tempo, 120.0);
    assert_eq!(loaded_project.metadata.sample_rate, 48000.0);

    // Verify synth parameters
    assert_eq!(loaded_project.synth_params.waveform, WaveformType::Saw);
    assert_eq!(loaded_project.synth_params.volume, 0.85);
    assert_eq!(loaded_project.synth_params.adsr.attack, 0.1);
    assert_eq!(loaded_project.synth_params.adsr.decay, 0.3);
    assert_eq!(loaded_project.synth_params.adsr.sustain, 0.7);
    assert_eq!(loaded_project.synth_params.adsr.release, 0.5);

    // Verify patterns and tracks
    assert_eq!(loaded_project.patterns.len(), 2); // Default pattern + our melody pattern
    assert_eq!(loaded_project.tracks.len(), 1);

    let track = loaded_project.tracks.get(&0).unwrap();
    assert_eq!(track.name, "Melody Track");
    assert_eq!(track.volume, 0.9);
    assert!(track.pattern_id.is_some());

    // Verify the pattern was loaded correctly
    let loaded_pattern_id = track.pattern_id.unwrap();
    let loaded_pattern = loaded_project.patterns.get(&loaded_pattern_id).unwrap();
    assert_eq!(loaded_pattern.name, "Melody Pattern");
    assert_eq!(loaded_pattern.length_bars, 4);
    assert_eq!(loaded_pattern.notes.len(), 4);

    // Verify notes
    let notes: Vec<_> = loaded_pattern
        .notes
        .iter()
        .map(|n| (n.pitch, n.velocity, n.start_samples, n.duration_samples))
        .collect();

    // Check that we have 4 notes with correct pitches and velocities
    assert_eq!(notes.len(), 4);
    assert_eq!(notes[0].0, 60); // C4
    assert_eq!(notes[1].0, 62); // D4
    assert_eq!(notes[2].0, 64); // E4
    assert_eq!(notes[3].0, 65); // F4
    assert_eq!(notes[0].1, 127); // All velocities should be 127
    assert_eq!(notes[1].1, 127);
    assert_eq!(notes[2].1, 127);
    assert_eq!(notes[3].1, 127);

    // Test sample rate override
    let options_with_override = ProjectLoadOptions {
        validate: false,
        load_samples: false,
        sample_rate_override: Some(96000.0),
    };

    let overridden_project = manager
        .load_project(&project_path, &options_with_override)
        .expect("Failed to load project with sample rate override");

    assert_eq!(overridden_project.metadata.sample_rate, 96000.0);

    // Cleanup
    std::fs::remove_file(&project_path).ok();

    println!("✅ Integration test passed! Project persistence system is working correctly.");
}

#[test]
fn test_project_file_format() {
    let manager = ProjectManager::new(44100.0);
    let project = manager.create_new_project("Format Test".to_string());

    let project_path = std::env::temp_dir().join("format_test.mymusic");
    manager
        .save_project(&project, &project_path)
        .expect("Failed to save project");

    // Verify it's a valid ZIP file
    let zip_file = std::fs::File::open(&project_path).expect("Failed to open project file");
    let mut zip_archive = zip::ZipArchive::new(zip_file).expect("Failed to read ZIP archive");

    // Check expected files exist
    let mut found_manifest = false;
    let mut found_project = false;
    let mut found_tracks_dir = false;

    for i in 0..zip_archive.len() {
        if let Ok(file) = zip_archive.by_index(i) {
            let name = file.name();
            if name == "manifest.json" {
                found_manifest = true;
            } else if name == "project.ron" {
                found_project = true;
            } else if name.starts_with("tracks/") {
                found_tracks_dir = true;
            }
        }
    }

    assert!(found_manifest, "manifest.json should exist in project file");
    assert!(found_project, "project.ron should exist in project file");
    assert!(
        found_tracks_dir,
        "tracks/ directory should exist in project file"
    );

    // Cleanup
    std::fs::remove_file(&project_path).ok();

    println!("✅ Project file format test passed! ZIP structure is correct.");
}
