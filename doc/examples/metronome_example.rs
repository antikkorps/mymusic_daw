// Example: Using the metronome in audio callback
// This shows how to integrate the metronome with the transport and audio engine

use mymusic_daw::sequencer::{Metronome, MetronomeScheduler, Tempo, TimeSignature, Transport};

fn main() {
    // Setup
    let sample_rate = 48000.0;
    let buffer_size = 512;
    let mut transport = Transport::new(sample_rate);
    
    // Configure musical context
    transport.set_tempo(Tempo::new(120.0)); // 120 BPM
    transport.set_time_signature(TimeSignature::four_four()); // 4/4 time
    
    // Create metronome (UI thread)
    let mut metronome = Metronome::new(sample_rate as f32);
    metronome.set_enabled(true);
    metronome.set_volume(0.7);
    
    // Create scheduler (audio thread)
    let mut scheduler = MetronomeScheduler::new();
    
    // Start playback
    transport.play();
    
    // Simulate audio callback processing
    println!("Simulating 2 seconds of audio at 120 BPM, 4/4 time");
    println!("Expected: 4 clicks (one per beat)");
    println!("Pattern: Accent (strong), Regular, Regular, Regular\n");
    
    let num_buffers = (2.0 * sample_rate / buffer_size as f64) as usize;
    let shared_state = transport.shared_state();
    
    for buffer_num in 0..num_buffers {
        // Audio callback starts here
        let position_samples = shared_state.position_samples();
        
        // Check if a click should occur in this buffer
        if let Some((offset, click_type)) = scheduler.check_for_click(
            position_samples,
            buffer_size,
            sample_rate,
            transport.tempo(),
            transport.time_signature(),
        ) {
            println!(
                "Buffer #{:3} @ sample {:6}: {:?} click at offset {}",
                buffer_num, position_samples, click_type, offset
            );
            
            // Trigger the click
            metronome.trigger_click(click_type);
        }
        
        // Generate audio (simplified - in real code this would mix with synth output)
        let mut audio_buffer = vec![0.0f32; buffer_size];
        
        // Process metronome into buffer
        for sample in audio_buffer.iter_mut() {
            *sample = metronome.process_sample();
        }
        
        // Advance transport position
        shared_state.advance_position(buffer_size as u64);
        
        // Audio callback ends here
    }
    
    println!("\nMetronome integration example completed successfully!");
    println!("\nIn a real DAW:");
    println!("- Metronome audio would be mixed with synth/sampler output");
    println!("- UI would have enable/disable toggle and volume control");
    println!("- Transport controls (play/stop) would control metronome");
    println!("- Tempo and time signature changes would be handled live");
}
