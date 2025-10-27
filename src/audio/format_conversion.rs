// Format conversion for CPAL audio streams
//
// Supports conversion between internal f32 format and various CPAL sample formats:
// - f32: Native floating point (most common, no conversion needed)
// - i16: 16-bit signed integer (common on Windows/WASAPI)
// - u16: 16-bit unsigned integer (less common)
//
// All conversions are allocation-free and suitable for real-time audio callbacks.

use cpal::{FromSample, Sample};

/// Convert f32 sample to i16
///
/// Maps [-1.0, 1.0] to [i16::MIN, i16::MAX]
/// Clamps values outside the range to prevent overflow
#[inline]
pub fn f32_to_i16(sample: f32) -> i16 {
    // Clamp to valid range
    let clamped = sample.clamp(-1.0, 1.0);

    // Convert to i16 range
    // Note: We use i16::MAX (32767) instead of 32768 to avoid overflow
    if clamped >= 0.0 {
        (clamped * i16::MAX as f32) as i16
    } else {
        (clamped * -(i16::MIN as f32)) as i16
    }
}

/// Convert f32 sample to u16
///
/// Maps [-1.0, 1.0] to [u16::MIN, u16::MAX]
/// u16 uses offset binary encoding where 32768 is zero
#[inline]
pub fn f32_to_u16(sample: f32) -> u16 {
    // Clamp to valid range
    let clamped = sample.clamp(-1.0, 1.0);

    // Convert to u16 range (offset binary: 0.0 -> 32768)
    // Map [-1.0, 1.0] to [0, 65535]
    ((clamped + 1.0) * 0.5 * u16::MAX as f32) as u16
}

/// Convert i16 sample to f32
///
/// Maps [i16::MIN, i16::MAX] to [-1.0, 1.0]
#[inline]
pub fn i16_to_f32(sample: i16) -> f32 {
    if sample >= 0 {
        sample as f32 / i16::MAX as f32
    } else {
        sample as f32 / -(i16::MIN as f32)
    }
}

/// Convert u16 sample to f32
///
/// Maps [u16::MIN, u16::MAX] to [-1.0, 1.0]
/// u16 uses offset binary encoding where 32768 is zero
#[inline]
pub fn u16_to_f32(sample: u16) -> f32 {
    // Convert from offset binary to signed
    // Map [0, 65535] to [-1.0, 1.0]
    (sample as f32 / u16::MAX as f32) * 2.0 - 1.0
}

/// Write f32 sample to output buffer with automatic format conversion
///
/// This is the main function used in the audio callback.
/// It detects the output format and converts the f32 sample accordingly.
#[inline]
pub fn write_sample_to_buffer<T>(sample: f32, output: &mut T)
where
    T: Sample + FromSample<f32>,
{
    *output = Sample::from_sample::<f32>(sample);
}

/// Process interleaved audio buffer
///
/// Takes an internal f32 mono sample and writes it to all channels
/// of an interleaved output buffer (e.g., [L, R, L, R, L, R...])
///
/// # Arguments
/// * `internal_sample` - The mono f32 sample to write
/// * `output_frame` - A slice representing one audio frame (e.g., [L, R] for stereo)
#[inline]
pub fn write_mono_to_interleaved_frame<T>(internal_sample: f32, output_frame: &mut [T])
where
    T: Sample + FromSample<f32>,
{
    for channel_sample in output_frame.iter_mut() {
        *channel_sample = Sample::from_sample::<f32>(internal_sample);
    }
}

/// Process interleaved audio buffer for stereo samples
///
/// Takes an internal f32 stereo sample and writes it to the first two channels
/// of an interleaved output buffer (e.g., [L, R, L, R, L, R...])
///
/// # Arguments
/// * `(left_sample, right_sample)` - The stereo f32 sample to write
/// * `output_frame` - A slice representing one audio frame (e.g., [L, R] for stereo)
#[inline]
pub fn write_stereo_to_interleaved_frame<T>(
    (left_sample, right_sample): (f32, f32),
    output_frame: &mut [T],
) where
    T: Sample + FromSample<f32>,
{
    if output_frame.len() >= 2 {
        output_frame[0] = Sample::from_sample::<f32>(left_sample);
        output_frame[1] = Sample::from_sample::<f32>(right_sample);
        // For > 2 channels, we could either write silence or duplicate L/R
        for channel_sample in output_frame.iter_mut().skip(2) {
            *channel_sample = Sample::from_sample::<f32>(0.0);
        }
    } else if let Some(channel_sample) = output_frame.first_mut() {
        // Fallback for mono output: mix L and R
        let mono_sample = (left_sample + right_sample) * 0.5;
        *channel_sample = Sample::from_sample::<f32>(mono_sample);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_f32_to_i16_conversion() {
        // Test zero
        assert_eq!(f32_to_i16(0.0), 0);

        // Test positive max
        assert_eq!(f32_to_i16(1.0), i16::MAX);

        // Test negative max
        assert_eq!(f32_to_i16(-1.0), i16::MIN);

        // Test mid values
        let mid = f32_to_i16(0.5);
        assert!(mid > 0 && mid < i16::MAX);

        let neg_mid = f32_to_i16(-0.5);
        assert!(neg_mid < 0 && neg_mid > i16::MIN);
    }

    #[test]
    fn test_f32_to_u16_conversion() {
        // Test zero (should map to middle of u16 range)
        let zero = f32_to_u16(0.0);
        assert!((zero as i32 - 32768).abs() < 10); // Allow small rounding error

        // Test positive max
        assert_eq!(f32_to_u16(1.0), u16::MAX);

        // Test negative max
        assert_eq!(f32_to_u16(-1.0), u16::MIN);
    }

    #[test]
    fn test_i16_to_f32_conversion() {
        // Test zero
        assert_eq!(i16_to_f32(0), 0.0);

        // Test max values
        assert!((i16_to_f32(i16::MAX) - 1.0).abs() < 0.001);
        assert!((i16_to_f32(i16::MIN) - (-1.0)).abs() < 0.001);

        // Test symmetry
        let original = 0.5f32;
        let converted = f32_to_i16(original);
        let back = i16_to_f32(converted);
        assert!((back - original).abs() < 0.001);
    }

    #[test]
    fn test_u16_to_f32_conversion() {
        // Test zero (u16 0 should be -1.0)
        assert!((u16_to_f32(0) - (-1.0)).abs() < 0.001);

        // Test max (u16 MAX should be 1.0)
        assert!((u16_to_f32(u16::MAX) - 1.0).abs() < 0.001);

        // Test middle (u16 32768 should be ~0.0)
        assert!(u16_to_f32(32768).abs() < 0.001);
    }

    #[test]
    fn test_roundtrip_i16() {
        let test_values = [-1.0f32, -0.5, -0.1, 0.0, 0.1, 0.5, 0.9, 1.0];

        for &original in &test_values {
            let i16_val = f32_to_i16(original);
            let back = i16_to_f32(i16_val);
            // Allow small loss due to quantization
            assert!(
                (back - original).abs() < 0.001,
                "Roundtrip failed for {}: got {}",
                original,
                back
            );
        }
    }

    #[test]
    fn test_roundtrip_u16() {
        let test_values = [-1.0f32, -0.5, -0.1, 0.0, 0.1, 0.5, 0.9, 1.0];

        for &original in &test_values {
            let u16_val = f32_to_u16(original);
            let back = u16_to_f32(u16_val);
            // Allow small loss due to quantization
            assert!(
                (back - original).abs() < 0.001,
                "Roundtrip failed for {}: got {}",
                original,
                back
            );
        }
    }

    #[test]
    fn test_clamping() {
        // Test values outside [-1.0, 1.0] are clamped
        assert_eq!(f32_to_i16(2.0), i16::MAX);
        assert_eq!(f32_to_i16(-2.0), i16::MIN);

        assert_eq!(f32_to_u16(2.0), u16::MAX);
        assert_eq!(f32_to_u16(-2.0), u16::MIN);
    }

    #[test]
    fn test_write_mono_to_interleaved() {
        // Test with f32 output (stereo)
        let mut output: [f32; 2] = [0.0; 2];
        write_mono_to_interleaved_frame(0.5, &mut output);
        assert_eq!(output[0], 0.5);
        assert_eq!(output[1], 0.5);

        // Test with i16 output (stereo)
        let mut output_i16: [i16; 2] = [0; 2];
        write_mono_to_interleaved_frame(0.5, &mut output_i16);
        assert!(output_i16[0] > 0);
        assert!(output_i16[1] > 0);
        assert_eq!(output_i16[0], output_i16[1]);
    }
}
