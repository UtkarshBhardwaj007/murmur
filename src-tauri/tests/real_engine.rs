//! Opt-in integration tests that exercise the real STT engines.
//!
//! These are `#[ignore]`d because they need multi-hundred-MB model files.
//! Run them locally with:
//!
//! ```sh
//! MURMUR_DATA_DIR=~/Library/Application\ Support/com.murmur.app \
//! MURMUR_TEST_WAV=/path/to/speech.wav \
//! cargo test --test real_engine -- --ignored
//! ```
//!
//! The WAV should contain the spoken phrase "hello world this is a test".

use std::path::PathBuf;

use murmur_lib::models::{self, ModelId};
use murmur_lib::stt;

fn data_dir() -> PathBuf {
    PathBuf::from(std::env::var("MURMUR_DATA_DIR").expect("set MURMUR_DATA_DIR"))
}

fn test_samples() -> Vec<f32> {
    let wav = std::env::var("MURMUR_TEST_WAV").expect("set MURMUR_TEST_WAV");
    let mut reader = hound::WavReader::open(&wav).expect("open test wav");
    let spec = reader.spec();
    assert_eq!(spec.channels, 1, "test wav must be mono");
    let samples: Vec<f32> = match spec.sample_format {
        hound::SampleFormat::Float => reader.samples::<f32>().map(Result::unwrap).collect(),
        hound::SampleFormat::Int => {
            let max = (1i64 << (spec.bits_per_sample - 1)) as f32;
            reader
                .samples::<i32>()
                .map(|s| s.unwrap() as f32 / max)
                .collect()
        }
    };
    murmur_lib::audio::dsp::resample(&samples, spec.sample_rate, 16_000)
}

fn assert_transcript_mentions_test_phrase(transcript: &str) {
    let lower = transcript.to_lowercase();
    for word in ["hello", "world", "test"] {
        assert!(
            lower.contains(word),
            "expected {word:?} in transcript {transcript:?}"
        );
    }
}

#[test]
#[ignore = "requires downloaded Parakeet model and a speech WAV"]
fn parakeet_transcribes_speech() {
    let dir = data_dir();
    assert!(
        models::is_installed(&dir, ModelId::ParakeetTdt06bV2Int8),
        "Parakeet not installed under {}",
        dir.display()
    );
    let mut engine = stt::load_engine(
        ModelId::ParakeetTdt06bV2Int8,
        &models::model_dir(&dir, ModelId::ParakeetTdt06bV2Int8),
    )
    .expect("load parakeet");
    let transcript = engine.transcribe(&test_samples(), 16_000).expect("stt");
    println!("parakeet transcript: {transcript:?}");
    assert_transcript_mentions_test_phrase(&transcript);
}

#[test]
#[ignore = "requires downloaded whisper model and a speech WAV"]
fn whisper_transcribes_speech() {
    let dir = data_dir();
    assert!(
        models::is_installed(&dir, ModelId::WhisperBaseEn),
        "whisper base.en not installed under {}",
        dir.display()
    );
    let mut engine = stt::load_engine(
        ModelId::WhisperBaseEn,
        &models::model_dir(&dir, ModelId::WhisperBaseEn),
    )
    .expect("load whisper");
    let transcript = engine.transcribe(&test_samples(), 16_000).expect("stt");
    println!("whisper transcript: {transcript:?}");
    assert_transcript_mentions_test_phrase(&transcript);
}
