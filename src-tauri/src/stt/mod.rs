//! Speech-to-text engines behind a common trait so the rest of the app (and
//! the test suite) never depends on a concrete inference stack.

mod mock;
mod sherpa;
mod whisper;

use std::path::Path;

pub use mock::MockEngine;

use crate::models::ModelId;

/// A loaded speech-to-text engine. Implementations run fully on-device.
pub trait SttEngine: Send {
    /// Transcribe mono f32 samples in [-1, 1] at `sample_rate` Hz.
    fn transcribe(&mut self, samples: &[f32], sample_rate: u32) -> anyhow::Result<String>;
}

/// Load the engine for `model` from an installed model directory.
pub fn load_engine(model: ModelId, model_dir: &Path) -> anyhow::Result<Box<dyn SttEngine>> {
    let started = std::time::Instant::now();
    let engine: Box<dyn SttEngine> = match model {
        ModelId::ParakeetTdt06bV2Int8 => Box::new(sherpa::SherpaTransducerEngine::load(model_dir)?),
        ModelId::WhisperBaseEn => Box::new(whisper::WhisperCppEngine::load(model_dir)?),
    };
    log::info!(
        "loaded {:?} in {:.1}s",
        model,
        started.elapsed().as_secs_f32()
    );
    Ok(engine)
}
