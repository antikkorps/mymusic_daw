// Quick demonstration of the project persistence system
// Run with: cargo run --bin demo_project_persistence

use mymusic_daw::project::{ProjectLoadOptions, ProjectManager};
use mymusic_daw::synth::oscillator::WaveformType;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🎵 MyMusic DAW - Project Persistence System Demo");
    println!("==================================================");

    // Create project manager
    let manager = ProjectManager::new(48000.0);

    // Create a new project
    let mut project = manager.create_new_project("Demo Project".to_string());
    project.metadata.author = Some("Demo User".to_string());
    project.metadata.description = Some("Demonstration of project persistence system".to_string());

    // Modify synth parameters
    project.synth_params.waveform = WaveformType::Saw;
    project.synth_params.volume = 0.75;

    println!("✅ Created new project: {}", project.metadata.name);
    println!("   - Author: {:?}", project.metadata.author);
    println!("   - Sample Rate: {} Hz", project.metadata.sample_rate);
    println!("   - Waveform: {:?}", project.synth_params.waveform);
    println!("   - Volume: {:.2}", project.synth_params.volume);
    println!("   - Tracks: {}", project.tracks.len());
    println!("   - Patterns: {}", project.patterns.len());

    // Save project
    let project_path = std::env::temp_dir().join("demo_project.mymusic");
    manager.save_project(&project, &project_path)?;

    println!("\n💾 Saved project to: {}", project_path.display());

    // Get file size
    let metadata = std::fs::metadata(&project_path)?;
    println!("   - File size: {} bytes", metadata.len());

    // Load project back
    let options = ProjectLoadOptions::default();
    let loaded_project = manager.load_project(&project_path, &options)?;

    println!("\n📂 Loaded project successfully:");
    println!("   - Name: {}", loaded_project.metadata.name);
    println!("   - Author: {:?}", loaded_project.metadata.author);
    println!(
        "   - Description: {:?}",
        loaded_project.metadata.description
    );
    println!(
        "   - Sample Rate: {} Hz",
        loaded_project.metadata.sample_rate
    );
    println!("   - Waveform: {:?}", loaded_project.synth_params.waveform);
    println!("   - Volume: {:.2}", loaded_project.synth_params.volume);
    println!("   - Tracks: {}", loaded_project.tracks.len());
    println!("   - Patterns: {}", loaded_project.patterns.len());

    // Verify data integrity
    assert_eq!(project.metadata.name, loaded_project.metadata.name);
    assert_eq!(project.metadata.author, loaded_project.metadata.author);
    assert_eq!(
        project.synth_params.waveform,
        loaded_project.synth_params.waveform
    );
    assert_eq!(
        project.synth_params.volume,
        loaded_project.synth_params.volume
    );

    println!("\n✅ Data integrity verified - all values match!");

    // Test sample rate override
    let options_with_override = ProjectLoadOptions {
        validate: false,
        load_samples: false,
        sample_rate_override: Some(96000.0),
    };

    let overridden_project = manager.load_project(&project_path, &options_with_override)?;
    println!("\n🔄 Sample rate override test:");
    println!("   - Original: {} Hz", project.metadata.sample_rate);
    println!(
        "   - Override: {} Hz",
        overridden_project.metadata.sample_rate
    );

    // Cleanup
    std::fs::remove_file(&project_path)?;
    println!("\n🧹 Cleaned up demo file");

    println!("\n🎉 Project persistence system demo completed successfully!");
    println!("   The system supports:");
    println!("   - ✅ ZIP container format with manifest.json + project.ron");
    println!("   - ✅ Complete project metadata serialization");
    println!("   - ✅ Synth parameters persistence");
    println!("   - ✅ Track and pattern data");
    println!("   - ✅ Sample rate override on load");
    println!("   - ✅ Project validation");
    println!("   - ✅ Error handling and recovery");

    Ok(())
}
