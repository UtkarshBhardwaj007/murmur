//! WAV file output for recorded audio.

use std::path::Path;

use anyhow::Context;

/// Write mono f32 samples as a 16-bit PCM WAV at the given sample rate.
pub fn write_wav_mono(path: &Path, samples: &[f32], sample_rate: u32) -> anyhow::Result<()> {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(path, spec)
        .with_context(|| format!("creating WAV file at {}", path.display()))?;
    for &s in samples {
        let clamped = s.clamp(-1.0, 1.0);
        writer.write_sample((clamped * i16::MAX as f32) as i16)?;
    }
    writer.finalize().context("finalizing WAV file")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn written_wav_has_expected_rate_and_duration() {
        let dir = std::env::temp_dir().join("murmur-wav-test");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("test.wav");

        // 2.5 seconds of a 440 Hz tone at 16 kHz.
        let rate = 16_000u32;
        let n = (rate as f32 * 2.5) as usize;
        let samples: Vec<f32> = (0..n)
            .map(|i| 0.5 * (2.0 * std::f32::consts::PI * 440.0 * i as f32 / rate as f32).sin())
            .collect();

        write_wav_mono(&path, &samples, rate).unwrap();

        let reader = hound::WavReader::open(&path).unwrap();
        let spec = reader.spec();
        assert_eq!(spec.sample_rate, 16_000);
        assert_eq!(spec.channels, 1);
        assert_eq!(spec.bits_per_sample, 16);
        let duration_secs = reader.duration() as f32 / spec.sample_rate as f32;
        assert!(
            (duration_secs - 2.5).abs() < 0.01,
            "duration was {duration_secs}"
        );

        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn clipping_samples_are_clamped() {
        let dir = std::env::temp_dir().join("murmur-wav-test");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("clip.wav");

        write_wav_mono(&path, &[2.0, -2.0, 0.0], 16_000).unwrap();

        let mut reader = hound::WavReader::open(&path).unwrap();
        let samples: Vec<i16> = reader.samples::<i16>().map(Result::unwrap).collect();
        assert_eq!(samples, vec![i16::MAX, i16::MIN + 1, 0]);

        std::fs::remove_file(&path).unwrap();
    }
}
