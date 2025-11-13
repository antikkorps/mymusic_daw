# MyMusic DAW - Synthesizer Controls Documentation

## Overview

The MyMusic DAW synthesizer provides a comprehensive set of controls for sound synthesis, including oscillators, envelopes, filters, LFOs, and modulation routing. This document explains each control and how to use them effectively.

## Basic Controls

### Volume
- **Range**: 0.0 to 1.0
- **Description**: Controls the master output volume of the synthesizer
- **Usage**: Use to adjust overall loudness of the synthesized sound

### Test Beep
- **Description**: Plays a test tone (A4, 440Hz) for audio system verification
- **Usage**: Useful for testing audio connectivity and engine status

## Oscillator Controls

### Waveform Selection
- **Options**: `sine`, `square`, `saw`, `triangle`
- **Description**: Selects the basic waveform shape
- **Characteristics**:
  - **Sine**: Pure, smooth tone, fundamental frequency only
  - **Square**: Bright, buzzy tone with odd harmonics
  - **Saw**: Rich, bright tone with all harmonics
  - **Triangle**: Mellow tone with odd harmonics, softer than square

## Envelope (ADSR)

The ADSR envelope controls how the amplitude of a note changes over time.

### Attack
- **Range**: 0.0 to 10.0 seconds
- **Description**: Time taken for the sound to reach peak volume after note onset
- **Usage**:
  - Short attack (0.01-0.1s): Percussive, plucked sounds
  - Long attack (0.5-2.0s): Pads, strings, slow swells

### Decay
- **Range**: 0.0 to 10.0 seconds
- **Description**: Time taken for the sound to fall from peak to sustain level
- **Usage**:
  - Short decay: Quick drop to sustain level
  - Long decay: Gradual fade to sustain

### Sustain
- **Range**: 0.0 to 1.0
- **Description**: Volume level maintained while the note is held
- **Usage**:
  - Low sustain (0.0-0.3): Percussive sounds that fade out
  - High sustain (0.7-1.0): Organ-like sustained sounds

### Release
- **Range**: 0.0 to 10.0 seconds
- **Description**: Time taken for the sound to fade out after note release
- **Usage**:
  - Short release (0.01-0.1s): Staccato, percussive
  - Long release (1.0-5.0s): Ambient, reverb-like tails

## Filter Controls

### Filter Type
- **Options**: `lowpass`, `highpass`, `bandpass`, `notch`
- **Description**: Selects the frequency response characteristics
- **Characteristics**:
  - **Lowpass**: Allows low frequencies, attenuates high frequencies (most common)
  - **Highpass**: Allows high frequencies, attenuates low frequencies
  - **Bandpass**: Allows a specific frequency band
  - **Notch**: Attenuates a specific frequency band

### Cutoff Frequency
- **Range**: 20.0 Hz to 20000.0 Hz
- **Description**: The frequency at which the filter begins to affect the signal
- **Usage**:
  - Low cutoff (100-500 Hz): Muffled, bass-heavy sounds
  - Mid cutoff (1000-3000 Hz): Balanced, vocal-range presence
  - High cutoff (5000-20000 Hz): Bright, airy sounds

### Resonance
- **Range**: 0.0 to 1.0
- **Description**: Emphasizes frequencies around the cutoff point
- **Usage**:
  - Low resonance (0.0-0.3): Gentle filtering
  - High resonance (0.7-1.0): Pronounced, resonant peaks, possible self-oscillation

## LFO (Low-Frequency Oscillator)

### LFO Waveform
- **Options**: `sine`, `square`, `saw`, `triangle`
- **Description**: Shape of the modulation waveform
- **Characteristics**:
  - **Sine**: Smooth, vibrato-like modulation
  - **Square**: Abrupt, trill-like modulation
  - **Saw**: Ramp-like modulation
  - **Triangle**: Linear up/down modulation

### LFO Rate
- **Range**: 0.1 Hz to 20.0 Hz
- **Description**: Speed of the LFO modulation
- **Usage**:
  - Slow (0.1-2.0 Hz): Slow sweeps, vibrato
  - Fast (5-20 Hz): Fast modulation, tremolo effects

### LFO Depth
- **Range**: 0.0 to 1.0
- **Description**: Intensity of the modulation effect
- **Usage**:
  - Low depth (0.0-0.3): Subtle modulation
  - High depth (0.7-1.0): Dramatic modulation effects

### LFO Destination
- **Options**: `pitch`, `volume`, `filter`
- **Description**: Parameter that the LFO modulates
- **Effects**:
  - **Pitch**: Vibrato, siren effects
  - **Volume**: Tremolo, rhythmic pulsing
  - **Filter**: Filter sweeps, wah-wah effects

## Polyphony Mode

### Poly Mode Options
- **Options**: `poly`, `mono`, `legato`
- **Description**: How multiple notes are handled
- **Characteristics**:
  - **Poly**: Multiple notes can play simultaneously (up to voice limit)
  - **Mono**: Only one note plays at a time; new notes replace previous ones
  - **Legato**: Mono mode with smooth pitch transitions between notes

## Portamento (Glide)

### Portamento Time
- **Range**: 0.0 to 5.0 seconds
- **Description**: Time taken for pitch to slide from one note to another
- **Usage**:
  - No portamento (0.0s): Instant pitch changes
  - Short portamento (0.05-0.2s): Subtle glide
  - Long portamento (0.5-2.0s): Dramatic synth slides

## Voice Mode

### Voice Mode Options
- **Options**: `synth`, `sampler`
- **Description**: Selects the synthesis engine type
- **Characteristics**:
  - **Synth**: Real-time synthesis using oscillators and filters
  - **Sampler**: Sample-based playback (future feature)

## Modulation Routing

### Modulation Sources
- **Options**: `lfo`, `velocity`, `aftertouch`, `envelope`
- **Description**: Signal that provides modulation
- **Characteristics**:
  - **LFO**: Cyclic modulation from LFO
  - **Velocity**: Note-on velocity (how hard a key is pressed)
  - **Aftertouch**: Pressure applied after note onset (MIDI channel pressure)
  - **Envelope**: ADSR envelope output

### Modulation Destinations
- **Options**: `pitch`, `amplitude`, `filter`, `pan`
- **Description**: Parameter that receives modulation
- **Effects**:
  - **Pitch**: Vibrato, pitch bend effects
  - **Amplitude**: Tremolo, velocity-sensitive volume
  - **Filter**: Dynamic filter sweeps, brightness control
  - **Pan**: Stereo positioning automation

### Modulation Amount
- **Range**: -1.0 to 1.0
- **Description**: Strength and direction of modulation
- **Usage**:
  - Positive values: Normal modulation direction
  - Negative values: Inverted modulation direction
  - Zero: No modulation

## Practical Examples

### Classic Bass Sound
```
Waveform: Saw
Filter: Lowpass, Cutoff 800Hz, Resonance 0.7
ADSR: Attack 0.01s, Decay 0.2s, Sustain 0.3, Release 0.1s
Poly Mode: Mono
Portamento: 0.05s
```

### Pad Sound
```
Waveform: Sine
Filter: Lowpass, Cutoff 2000Hz, Resonance 0.2
ADSR: Attack 1.5s, Decay 0.5s, Sustain 0.7, Release 3.0s
Poly Mode: Poly
LFO: Sine, Rate 0.3Hz, Depth 0.2, Destination: Filter
```

### Lead Sound
```
Waveform: Square
Filter: Lowpass, Cutoff 3000Hz, Resonance 0.8
ADSR: Attack 0.05s, Decay 0.1s, Sustain 0.9, Release 0.3s
Poly Mode: Mono
LFO: Sine, Rate 5.0Hz, Depth 0.1, Destination: Pitch
Modulation: Velocity → Filter (Amount: 0.5)
```

## Tips and Best Practices

1. **Start Simple**: Begin with basic waveform and volume, then add effects
2. **Use Subtle Modulation**: Small amounts of LFO and resonance often work best
3. **Layer Sounds**: Use multiple voices with different settings for complex timbres
4. **Experiment with Envelopes**: The ADSR envelope dramatically affects sound character
5. **Filter Automation**: Dynamic filter changes create expressive, evolving sounds
6. **Velocity Sensitivity**: Use velocity routing for more expressive playing

## MIDI Integration

The synthesizer responds to standard MIDI messages:
- **Note On/Off**: Triggers envelope and voice allocation
- **Velocity**: Affects initial amplitude (can be routed to other parameters)
- **Aftertouch**: Can be routed to filter, pitch, or amplitude for expression
- **Pitch Bend**: Standard MIDI pitch bend (±2 semitones by default)
- **Control Change**: MIDI CC messages can be mapped to parameters (future feature)

## Performance Considerations

- **Voice Limit**: Maximum polyphony is limited by available voices
- **CPU Usage**: Complex modulation and high polyphony increase CPU load
- **Filter Resonance**: Very high resonance can cause CPU spikes
- **LFO Speed**: Very fast LFO rates may cause audio artifacts

## Troubleshooting

### No Sound
1. Check volume is above 0
2. Verify audio device is working with test beep
3. Check if notes are being triggered (use active notes display)

### Distorted Sound
1. Reduce volume or resonance
2. Check for multiple voices playing same note
3. Verify filter cutoff isn't too high with high resonance

### No Modulation Effect
1. Verify LFO depth is above 0
2. Check modulation routing is enabled
3. Ensure modulation amount is not 0

### Slow Response
1. Reduce attack time if too long
2. Check for excessive voice usage
3. Monitor CPU usage indicator

## Future Features

- **Additional Waveforms**: PWM, noise, wavetable
- **More Filter Types**: Formant, comb, phaser
- **Effects**: Delay, reverb, distortion
- **Advanced Modulation**: Multiple LFOs, step sequencers
- **MIDI Learn**: Map MIDI controllers to parameters
- **Preset System**: Save and recall sound settings