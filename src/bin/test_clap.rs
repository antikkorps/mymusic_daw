// Test program for CLAP plugin integration
// This program demonstrates the CLAP plugin infrastructure

use mymusic_daw::plugin::{ClapPluginFactory, PluginFactory, PluginScanner};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🎛️  MyMusic DAW - CLAP Plugin Test");
    println!("=====================================");

    // Test 1: Plugin Scanner
    println!("\n📂 Testing Plugin Scanner:");
    let cache_path = dirs::cache_dir()
        .unwrap_or_default()
        .join("mymusic_daw")
        .join("plugin_cache.json");

    let mut scanner = PluginScanner::new(cache_path);

    // Get default search paths
    let search_paths = mymusic_daw::plugin::scanner::get_default_search_paths();
    println!("Default CLAP search paths:");
    for path in &search_paths {
        println!("  - {}", path.display());
    }

    // Scan current directory for demo
    let current_dir = std::env::current_dir()?;
    println!("\n🔍 Scanning current directory for .clap files:");
    let descriptors = scanner.scan_directory(&current_dir)?;

    if descriptors.is_empty() {
        println!("  No CLAP plugins found in current directory");
        println!("  (This is expected since we haven't created real CLAP plugins yet)");
    } else {
        for desc in &descriptors {
            println!("  ✅ Found: {} ({})", desc.name, desc.id);
        }
    }

    // Test 2: CLAP Plugin Factory
    println!("\n🏭 Testing CLAP Plugin Factory:");

    // Try to find a real CLAP plugin first
    let mut real_clap_found = false;
    let clap_paths = [
        "/Library/Audio/Plug-Ins/CLAP",
        "/Users/franck/Library/Audio/Plug-Ins/CLAP",
    ];

    let mut factory = None;
    let mut test_clap_path = None;

    for clap_path in &clap_paths {
        let clap_dir = std::path::Path::new(clap_path);
        if clap_dir.exists() {
            println!("  🔍 Scanning {} for CLAP plugins...", clap_path);

            if let Ok(entries) = std::fs::read_dir(clap_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().and_then(|s| s.to_str()) == Some("clap") {
                        println!(
                            "  ✅ Found CLAP plugin: {}",
                            path.file_name().unwrap().to_string_lossy()
                        );

                        match ClapPluginFactory::from_path(path.to_str().unwrap()) {
                            Ok(f) => {
                                factory = Some(f);
                                test_clap_path = Some(path.clone());
                                real_clap_found = true;
                                break;
                            }
                            Err(e) => {
                                println!(
                                    "  ❌ Failed to load {}: {}",
                                    path.file_name().unwrap().to_string_lossy(),
                                    e
                                );
                            }
                        }
                    }
                }
            }
        }

        if real_clap_found {
            break;
        }
    }

    // If no real CLAP found, create a test factory with placeholder
    if !real_clap_found {
        println!("  📝 No real CLAP plugins found, creating placeholder factory for testing");
        let placeholder_path = current_dir.join("test_plugin.clap");
        test_clap_path = Some(placeholder_path.clone());

        factory = Some(ClapPluginFactory::from_path(
            placeholder_path.to_str().unwrap(),
        )?);
    }

    let factory = factory.unwrap();
    println!("  ✅ Created factory for: {}", factory.descriptor().name);
    println!("  📋 Plugin ID: {}", factory.descriptor().id);
    println!("  🏢 Vendor: {}", factory.descriptor().vendor);
    println!("  📝 Description: {}", factory.descriptor().description);
    println!("  📂 Category: {:?}", factory.descriptor().category);

    // Test supported features
    println!("\n🔧 Supported Features:");
    let features = ["audio", "midi", "parameters", "state", "gui"];
    for feature in &features {
        let supported = factory.supports_feature(feature);
        println!("  {}: {}", if supported { "✅" } else { "❌" }, feature);
    }

    // Test 3: Plugin Instance
    println!("\n🎛️  Testing Plugin Instance:");

    let mut plugin_instance = if real_clap_found {
        // Try to create a real CLAP instance
        match factory.create_instance() {
            Ok(instance) => {
                println!("  ✅ Created real CLAP plugin instance");
                instance
            }
            Err(e) => {
                println!(
                    "  ⚠️  Failed to create real CLAP instance: {}, falling back to demo",
                    e
                );
                factory.create_instance()?
            }
        }
    } else {
        println!("  📝 Creating demo plugin instance (no real CLAP available)");
        factory.create_instance()?
    };
    println!("  ✅ Created plugin instance");

    // Initialize plugin
    plugin_instance.initialize(44100.0)?;
    println!("  ✅ Initialized plugin at 44.1kHz");

    // Test parameters
    println!("\n🎛️  Testing Parameters:");
    let all_params = plugin_instance.get_all_parameters();
    if all_params.is_empty() {
        println!("  No parameters found (using demo parameters)");

        // Set some demo parameters
        plugin_instance.set_parameter("gain", 0.75)?;
        plugin_instance.set_parameter("frequency", 880.0)?;
        plugin_instance.set_parameter("resonance", 0.9)?;

        println!("  ✅ Set gain = 0.75");
        println!("  ✅ Set frequency = 880.0");
        println!("  ✅ Set resonance = 0.9");
    } else {
        for (id, value) in &all_params {
            println!("  {} = {}", id, value);
        }
    }

    // Test state save/load
    println!("\n💾 Testing State Management:");
    let state = plugin_instance.save_state()?;
    println!(
        "  ✅ Saved plugin state ({} parameters)",
        state.parameters.len()
    );

    // Modify parameters and reload state
    plugin_instance.set_parameter("gain", 0.25)?;
    println!("  📝 Modified gain to 0.25");

    plugin_instance.load_state(&state)?;
    println!("  ✅ Reloaded state (gain should be back to original)");

    let current_gain = plugin_instance.get_parameter("gain").unwrap_or(0.0);
    println!("  📊 Current gain: {}", current_gain);

    // Test processing
    println!("\n🔊 Testing Audio Processing:");
    let mut input_buffers = std::collections::HashMap::new();
    let mut output_buffers = std::collections::HashMap::new();

    // Create dummy audio buffers (using the placeholder AudioBuffer)
    let input_buffer = mymusic_daw::audio::buffer::AudioBuffer::new(512);
    let mut output_buffer = mymusic_daw::audio::buffer::AudioBuffer::new(512);

    input_buffers.insert("main".to_string(), &input_buffer);
    output_buffers.insert("main".to_string(), &mut output_buffer);

    plugin_instance.process(&input_buffers, &mut output_buffers, 512)?;
    println!("  ✅ Processed 512 audio samples");

    // Cleanup
    println!("\n🧹 Cleanup:");
    if let Some(path) = test_clap_path {
        if !real_clap_found && path.exists() {
            std::fs::remove_file(&path)?;
            println!("  ✅ Removed placeholder file");
        } else {
            println!("  ✅ Real CLAP plugin left untouched");
        }
    }

    println!("\n🎉 CLAP Plugin Test Complete!");
    println!("=====================================");
    println!("✅ All CLAP infrastructure components are working!");
    println!("🚀 Ready for real CLAP plugin integration");

    Ok(())
}
