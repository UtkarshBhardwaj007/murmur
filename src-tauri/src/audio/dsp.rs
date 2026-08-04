//! Pure DSP helpers: channel downmixing and sample-rate conversion.
//!
//! Everything here is free of I/O so it can be unit-tested headlessly.

/// Downmix interleaved multi-channel f32 samples to mono by averaging
/// each frame's channels.
pub fn downmix_to_mono(interleaved: &[f32], channels: usize) -> Vec<f32> {
    if channels <= 1 {
        return interleaved.to_vec();
    }
    interleaved
        .chunks_exact(channels)
        .map(|frame| frame.iter().sum::<f32>() / channels as f32)
        .collect()
}

/// Resample `input` from `from_rate` to `to_rate`.
///
/// Downsampling first applies a moving-average low-pass sized to the
/// decimation ratio to tame aliasing, then linearly interpolates. This is
/// intentionally simple — speech STT models are tolerant of it and it keeps
/// the pipeline dependency-free and testable.
pub fn resample(input: &[f32], from_rate: u32, to_rate: u32) -> Vec<f32> {
    assert!(
        from_rate > 0 && to_rate > 0,
        "sample rates must be non-zero"
    );
    if from_rate == to_rate || input.is_empty() {
        return input.to_vec();
    }

    let filtered: Vec<f32> = if from_rate > to_rate {
        let window = (from_rate as f32 / to_rate as f32).ceil() as usize;
        moving_average(input, window)
    } else {
        input.to_vec()
    };

    let ratio = from_rate as f64 / to_rate as f64;
    let out_len = ((input.len() as f64) / ratio).round() as usize;
    let mut out = Vec::with_capacity(out_len);
    for i in 0..out_len {
        let src_pos = i as f64 * ratio;
        let idx = src_pos as usize;
        let frac = (src_pos - idx as f64) as f32;
        let a = filtered[idx.min(filtered.len() - 1)];
        let b = filtered[(idx + 1).min(filtered.len() - 1)];
        out.push(a + (b - a) * frac);
    }
    out
}

fn moving_average(input: &[f32], window: usize) -> Vec<f32> {
    if window <= 1 {
        return input.to_vec();
    }
    let half = window / 2;
    let mut out = Vec::with_capacity(input.len());
    let mut sum: f64 = input.iter().take(half + 1).map(|&s| s as f64).sum();
    let mut count = input.len().min(half + 1);
    out.push((sum / count as f64) as f32);
    for i in 1..input.len() {
        if i + half < input.len() {
            sum += input[i + half] as f64;
            count += 1;
        }
        if i > half {
            sum -= input[i - half - 1] as f64;
            count -= 1;
        }
        out.push((sum / count as f64) as f32);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sine(freq: f32, rate: u32, seconds: f32) -> Vec<f32> {
        let n = (rate as f32 * seconds) as usize;
        (0..n)
            .map(|i| (2.0 * std::f32::consts::PI * freq * i as f32 / rate as f32).sin())
            .collect()
    }

    fn zero_crossings(samples: &[f32]) -> usize {
        samples.windows(2).filter(|w| w[0] * w[1] < 0.0).count()
    }

    #[test]
    fn downmix_stereo_averages_channels() {
        let stereo = [1.0, 0.0, 0.5, 0.5, -1.0, 1.0];
        assert_eq!(downmix_to_mono(&stereo, 2), vec![0.5, 0.5, 0.0]);
    }

    #[test]
    fn downmix_mono_is_identity() {
        let mono = [0.1, 0.2, 0.3];
        assert_eq!(downmix_to_mono(&mono, 1), mono.to_vec());
    }

    #[test]
    fn resample_same_rate_is_identity() {
        let input = sine(440.0, 16_000, 0.1);
        assert_eq!(resample(&input, 16_000, 16_000), input);
    }

    #[test]
    fn resample_48k_to_16k_length() {
        let input = sine(440.0, 48_000, 1.0);
        let out = resample(&input, 48_000, 16_000);
        let expected = 16_000;
        assert!(
            (out.len() as i64 - expected).unsigned_abs() <= 2,
            "expected ~{expected} samples, got {}",
            out.len()
        );
    }

    #[test]
    fn resample_preserves_tone_frequency() {
        // A 440 Hz tone must still be ~440 Hz after 48 kHz -> 16 kHz.
        let input = sine(440.0, 48_000, 1.0);
        let out = resample(&input, 48_000, 16_000);
        // A sine at f Hz over 1 s has ~2f zero crossings.
        let zc = zero_crossings(&out);
        assert!(
            (850..=910).contains(&zc),
            "expected ~880 zero crossings, got {zc}"
        );
    }

    #[test]
    fn resample_upsamples_too() {
        let input = sine(440.0, 8_000, 0.5);
        let out = resample(&input, 8_000, 16_000);
        assert!(
            (out.len() as i64 - 8_000).unsigned_abs() <= 2,
            "got {}",
            out.len()
        );
        let zc = zero_crossings(&out);
        assert!(
            (420..=460).contains(&zc),
            "expected ~440 zero crossings, got {zc}"
        );
    }

    #[test]
    fn resample_empty_input() {
        assert!(resample(&[], 48_000, 16_000).is_empty());
    }
}
