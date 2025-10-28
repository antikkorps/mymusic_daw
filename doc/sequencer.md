# Sequencer Module - Phase 4

## Overview

The sequencer module provides the foundation for timeline-based music production in MyMusic DAW. It handles musical time representation, tempo management, and transport control.

## Architecture

```
sequencer/
├── mod.rs           # Module exports
├── timeline.rs      # Musical time representation
├── transport.rs     # Playback control
└── metronome.rs     # Click track generator
```

## Core Concepts

### Musical Time

The sequencer uses a **dual time representation**:

1. **Absolute time** (samples): Raw audio samples since start
2. **Musical time** (bars:beats:ticks): Human-readable musical position

This allows seamless conversion between technical (sample-accurate) and musical (bars/beats) representations.

### Time Resolution

- **PPQN** (Pulses Per Quarter Note): 480 ticks
- Standard MIDI resolution for precise timing
- Supports quantization down to 1/128 notes

### Timeline Components

#### TimeSignature

Represents musical meter (e.g., 4/4, 3/4, 6/8).

```rust
let ts = TimeSignature::four_four();  // 4/4 time
let ts = TimeSignature::new(6, 8);    // 6/8 time
```

#### Tempo

BPM (Beats Per Minute) with range validation (20-999 BPM).

```rust
let tempo = Tempo::new(120.0);
let beat_duration = tempo.beat_duration_seconds();  // 0.5s at 120 BPM
```

#### MusicalTime

Position in bars:beats:ticks format (1-based for bars/beats, 0-based for ticks).

```rust
let time = MusicalTime::new(2, 3, 240);  // Bar 2, beat 3, tick 240
println!("{}", time);  // Output: "2:03:240"

// Quantization
let quantized = time.quantize_to_beat(&time_signature);
let sixteenth = time.quantize_to_subdivision(&time_signature, 4);
```

#### Position

Combined representation of both sample and musical time.

```rust
let pos = Position::from_samples(48000, sample_rate, &tempo, &ts);
println!("{}", pos.musical);  // Shows bar:beat:tick

let pos2 = Position::from_musical(
    MusicalTime::new(1, 1, 0),
    sample_rate,
    &tempo,
    &ts
);
```

### Transport Control

The `Transport` manages playback state and position.

#### Transport States

- `Stopped`: Not playing, position at zero
- `Playing`: Active playback
- `Recording`: Playing + recording MIDI/audio
- `Paused`: Stopped but preserves position

#### Shared State

The transport uses **atomic thread-safe state** for communication with the audio thread:

```rust
let transport = Transport::new(48000.0);
let shared_state = transport.shared_state();  // Arc for audio thread

// UI thread
transport.play();
transport.set_tempo(Tempo::new(140.0));

// Audio thread (reads atomic state)
let is_playing = shared_state.state().is_playing();
let pos = shared_state.advance_position(buffer_size);
```

#### Loop Region

Support for seamless looping:

```rust
transport.set_loop_region_samples(0, 96000);  // 2 seconds at 48kHz
transport.set_loop_enabled(true);

// In audio callback:
// When position >= loop_end, automatically wraps to loop_start
```

## Usage Examples

### Basic Transport Control

```rust
let mut transport = Transport::new(48000.0);

// Set musical context
transport.set_tempo(Tempo::new(120.0));
transport.set_time_signature(TimeSignature::four_four());

// Control playback
transport.play();
let pos = transport.position();
println!("Playing at {}", pos.musical);

transport.pause();
transport.stop();  // Resets position to 0
```

### Sample-Accurate Timing

```rust
// Convert musical position to samples for scheduling
let start_beat_2 = MusicalTime::new(1, 2, 0);  // Beat 2 of bar 1
let pos = Position::from_musical(
    start_beat_2,
    48000.0,
    &Tempo::new(120.0),
    &TimeSignature::four_four()
);

// Schedule event at exact sample
schedule_event_at_sample(pos.samples);
```

### Quantization

```rust
let time_sig = TimeSignature::four_four();

// User clicks at arbitrary position
let user_click = MusicalTime::new(1, 1, 237);

// Snap to nearest beat
let snapped = user_click.quantize_to_beat(&time_sig);
// Result: 1:01:000 (start of beat 1)

// Snap to sixteenth notes (4 subdivisions per beat)
let snapped_16th = user_click.quantize_to_subdivision(&time_sig, 4);
// Result: 1:01:240 (second sixteenth note)
```

### Time Conversions

```rust
let ts = TimeSignature::four_four();

// Musical to total ticks
let musical = MusicalTime::new(2, 1, 0);  // Bar 2, beat 1
let ticks = musical.to_total_ticks(&ts);  // 1920 ticks (4 beats * 480)

// Ticks to musical (round-trip)
let back = MusicalTime::from_total_ticks(ticks, &ts);
assert_eq!(back, musical);
```

## Thread Safety

### Audio Thread (Real-time)

The audio callback reads from `SharedTransportState` using **atomic operations**:

- ✅ Lock-free reads (no blocking)
- ✅ Position updates via `advance_position()`
- ✅ Loop handling (automatic wraparound)
- ✅ No allocations

```rust
// In audio callback
let samples_to_generate = buffer.len() as u64;
let new_pos = shared_state.advance_position(samples_to_generate);

if shared_state.state().is_playing() {
    // Generate audio
}
```

### UI Thread (Non-real-time)

The UI thread controls the `Transport` directly:

```rust
// UI controls
if play_button.clicked() {
    transport.play();
}

if stop_button.clicked() {
    transport.stop();
}

// Display current position
let pos = transport.position();
ui.label(format!("Position: {}", pos.musical));
```

## Testing

All timeline and transport functionality is fully tested:

- ✅ 14 unit tests covering:
  - Time signature calculations
  - Tempo conversions (BPM ↔ samples)
  - Musical time arithmetic
  - Quantization (beats, subdivisions)
  - Position conversions (samples ↔ musical)
  - Transport state transitions
  - Loop wrapping behavior
  - Thread-safe state updates

Run tests:
```bash
cargo test --lib sequencer
```

## Next Steps (Phase 4 Continuation)

### Immediate (Week 1-2)
- [ ] Metronome (click track generation)
- [ ] UI integration (timeline ruler, transport buttons)
- [ ] Position cursor with snap-to-grid

### Short-term (Week 3-4)
- [ ] Event types (Note, Automation)
- [ ] Event storage and retrieval
- [ ] Timeline zoom/pan

### Medium-term (Week 5-8)
- [ ] Piano roll editor
- [ ] MIDI recording
- [ ] Automation lanes
- [ ] Project save/load (with timeline state)

## Metronome

The metronome provides sample-accurate click track generation for musical timing.

### Features

- **Dual click sounds**: Accent (strong) for downbeats, Regular for other beats
- **Pre-generated waveforms**: Short sine wave clicks (10ms) with exponential decay
- **Sample-accurate scheduling**: Clicks triggered at exact beat positions
- **Configurable**: Volume control (0.0-1.0), enable/disable
- **Low CPU overhead**: Pre-generated samples, no real-time synthesis

### Components

#### MetronomeSound

Generates and stores click waveforms:

```rust
let sound = MetronomeSound::new(48000.0);
let accent_click = sound.get_click(ClickType::Accent);   // 1200 Hz, louder
let regular_click = sound.get_click(ClickType::Regular); // 800 Hz, softer
```

#### Metronome

Manages playback state and generates audio:

```rust
let mut metronome = Metronome::new(48000.0);
metronome.set_enabled(true);
metronome.set_volume(0.7);

// Trigger a click
metronome.trigger_click(ClickType::Accent);

// In audio callback
let click_sample = metronome.process_sample();
```

#### MetronomeScheduler

Determines when clicks should occur based on musical time:

```rust
let mut scheduler = MetronomeScheduler::new();

// In audio callback
if let Some((offset, click_type)) = scheduler.check_for_click(
    position_samples,
    buffer_size,
    sample_rate,
    &tempo,
    &time_signature,
) {
    // Click should happen at `offset` samples into the buffer
    metronome.trigger_click(click_type);
}
```

### Usage in Audio Callback

```rust
// Setup (once)
let mut metronome = Metronome::new(48000.0);
let mut scheduler = MetronomeScheduler::new();

// In audio callback (per buffer)
let position = shared_transport_state.position_samples();

// Check for clicks
if let Some((offset, click_type)) = scheduler.check_for_click(
    position,
    buffer.len(),
    sample_rate,
    &tempo,
    &time_signature,
) {
    metronome.trigger_click(click_type);
}

// Generate metronome audio
for sample in buffer.iter_mut() {
    let click = metronome.process_sample();
    *sample += click;  // Mix with other audio sources
}
```

### Click Pattern

The metronome automatically determines click types based on time signature:

- **4/4 time**: Accent, Regular, Regular, Regular (repeats)
- **3/4 time**: Accent, Regular, Regular (repeats)
- **6/8 time**: Accent, Regular, Regular, Regular, Regular, Regular (repeats)

First beat of each bar gets an accent click (higher frequency, louder).

### Real-Time Safety

✅ **RT-safe** in audio callback:
- No allocations (clicks pre-generated)
- No locks or blocking operations
- Deterministic execution time
- Simple integer/float arithmetic

The metronome is designed to be called directly from the audio callback without any RT-safety violations.

### Testing

9 comprehensive unit tests covering:
- Click sound generation (waveform quality, duration)
- Playback state (trigger, volume, enable/disable)
- Buffer processing (efficient batch mode)
- Scheduler accuracy (beat detection, accent pattern)
- Time signature handling (4/4, 3/4, 6/8)
- Position reset and seeking

### Example

See `doc/examples/metronome_example.rs` for a complete integration example.

## Design Decisions

### Why 480 PPQN?

- Standard MIDI resolution
- Divides evenly by 2, 3, 4, 5, 6, 8, 10, 12, 16
- Supports tuplets and complex rhythms
- Compatible with external hardware

### Why Dual Time Representation?

- **Samples**: Required for sample-accurate audio scheduling
- **Musical**: Human-readable, tempo-independent
- Seamless conversion allows both worlds to coexist

### Why Atomic State?

- Audio thread must **never block**
- Atomics provide lock-free communication
- Simple primitives (bool, u64) are sufficient for transport state
- Complex state (tempo changes, events) handled via ringbuffers

## Performance Characteristics

- **Position conversion**: O(1) - simple arithmetic
- **Quantization**: O(1) - single division + rounding
- **State updates**: O(1) - atomic operations
- **Loop wrapping**: O(1) - modulo arithmetic

No allocations, no locks, fully deterministic for real-time audio.

## References

- MIDI Specification 1.0 (PPQN standard)
- Digital Audio Workstation design patterns
- Real-time audio programming best practices (Ross Bencina)
