//! Audio profiling utility
//! 
//! This binary runs the audio engine with profiling enabled to generate
//! performance reports and flamegraphs for analysis.

use mymusic_daw::audio::profiling::global_profiler;
use mymusic_daw::audio::engine::AudioEngine;
use mymusic_daw::messaging::channels::{create_command_channel, create_notification_channel};
use mymusic_daw::plugin::PluginHost;
use ringbuf::traits::producer::Producer;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 MyMusic DAW - Audio Profiling Tool");
    println!("=====================================");
    
    // Initialize profiling
    global_profiler().reset();
    
    // Create communication channels
    let (command_tx_ui, command_rx_ui) = create_command_channel(512);
    let (command_tx_midi, command_rx_midi) = create_command_channel(512);
    let (notification_tx, _notification_rx) = create_notification_channel(256);
    
    // Create plugin host
    let plugin_host = Arc::new(PluginHost::new());
    
    println!("🎵 Initializing audio engine with profiling...");
    
    // Initialize audio engine
    let audio_engine = AudioEngine::new(
        command_rx_ui,
        command_rx_midi,
        Arc::new(Mutex::new(notification_tx)),
        plugin_host.clone(),
    )?;
    
    println!("✅ Audio engine initialized successfully");
    println!("📊 Starting profiling session...");
    
    // Start a thread to generate test audio load
    let load_thread = thread::spawn(move || {
        println!("🎹 Generating test audio load...");
        
        // For now, just sleep to simulate load generation
        // The actual audio engine will run with profiling enabled
        thread::sleep(Duration::from_millis(2000));
        
        println!("✅ Test load generation completed");
    });
    
    // Monitor profiling for 10 seconds
    println!("⏱️  Profiling for 10 seconds...");
    thread::sleep(Duration::from_secs(10));
    
    // Wait for load thread to complete
    let _ = load_thread.join();
    
    // Generate profiling report
    println!("📈 Generating profiling report...");
    let stats = global_profiler().get_stats();
    
    println!("\n🔥 AUDIO PERFORMANCE REPORT");
    println!("===========================");
    println!("Total callbacks: {}", stats.callback_count);
    println!("Avg callback time: {:.2}μs", stats.avg_callback_time as f64 / 1000.0);
    println!("Max callback time: {:.2}μs", stats.max_callback_time as f64 / 1000.0);
    println!("Min callback time: {:.2}μs", stats.min_callback_time as f64 / 1000.0);
    
    // Calculate CPU usage percentage
    let buffer_size = 512; // Typical buffer size
    let sample_rate = 44100.0;
    let buffer_duration_ms = (buffer_size as f64 / sample_rate) * 1000.0;
    let avg_cpu_percent = (stats.avg_callback_time as f64 / 1000.0) / buffer_duration_ms * 100.0;
    let max_cpu_percent = (stats.max_callback_time as f64 / 1000.0) / buffer_duration_ms * 100.0;
    
    println!("Avg CPU usage: {:.1}%", avg_cpu_percent);
    println!("Max CPU usage: {:.1}%", max_cpu_percent);
    
    println!("\n📊 OPERATION BREAKDOWN");
    println!("=====================");
    for (operation, op_stats) in &stats.operation_stats {
        let op_percent = (op_stats.avg_time as f64 / 1000.0) / buffer_duration_ms * 100.0;
        println!(
            "{}: {} calls, avg {:.2}μs ({:.1}% CPU), total {:.2}ms",
            operation,
            op_stats.call_count,
            op_stats.avg_time as f64 / 1000.0,
            op_percent,
            op_stats.total_time as f64 / 1_000_000.0
        );
    }
    
    // Generate flamegraph report
    println!("\n🔥 Generating flamegraph report...");
    let flamegraph_report = global_profiler().generate_flamegraph_report();
    
    // Save report to file
    std::fs::write("audio_profile_report.txt", flamegraph_report)?;
    println!("📄 Report saved to: audio_profile_report.txt");
    
    // Performance analysis
    println!("\n🎯 PERFORMANCE ANALYSIS");
    println!("=======================");
    
    if avg_cpu_percent < 10.0 {
        println!("✅ Excellent performance - CPU usage well within limits");
    } else if avg_cpu_percent < 30.0 {
        println!("✅ Good performance - CPU usage acceptable");
    } else if avg_cpu_percent < 60.0 {
        println!("⚠️  Moderate performance - consider optimization");
    } else {
        println!("❌ Poor performance - optimization required");
    }
    
    if max_cpu_percent > 90.0 {
        println!("⚠️  High CPU spikes detected - risk of audio dropouts");
    }
    
    // Find bottlenecks
    if let Some((bottleneck_op, bottleneck_stats)) = stats.operation_stats
        .iter()
        .max_by_key(|(_, stats)| stats.total_time) {
        
        let bottleneck_percent = (bottleneck_stats.total_time as f64 / stats.total_callback_time as f64) * 100.0;
        println!("🔍 Primary bottleneck: {} ({:.1}% of total time)", bottleneck_op, bottleneck_percent);
    }
    
    println!("\n🚀 OPTIMIZATION RECOMMENDATIONS");
    println!("==============================");
    
    if stats.operation_stats.contains_key("audio_generation") {
        let audio_gen_stats = &stats.operation_stats["audio_generation"];
        let audio_gen_percent = (audio_gen_stats.total_time as f64 / stats.total_callback_time as f64) * 100.0;
        
        if audio_gen_percent > 50.0 {
            println!("🎹 Consider SIMD optimization for voice synthesis");
            println!("🎛️  Optimize voice stealing algorithm");
        }
    }
    
    if stats.operation_stats.contains_key("plugin_processing") {
        let plugin_stats = &stats.operation_stats["plugin_processing"];
        let plugin_percent = (plugin_stats.total_time as f64 / stats.total_callback_time as f64) * 100.0;
        
        if plugin_percent > 30.0 {
            println!("🔌 Consider plugin processing optimization");
            println!("📊 Implement plugin-side profiling");
        }
    }
    
    if stats.callback_count < 100 {
        println!("⚠️  Limited sample size - run profiling longer for better accuracy");
    }
    
    println!("\n✅ Profiling completed successfully!");
    
    Ok(())
}