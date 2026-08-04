//! whisper.cpp engine (ggml base.en) via whisper-rs.

use std::path::Path;

use anyhow::Context;
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

use super::SttEngine;

pub struct WhisperCppEngine {
    context: WhisperContext,
}

impl WhisperCppEngine {
    pub fn load(model_dir: &Path) -> anyhow::Result<Self> {
        let model_path = model_dir.join("ggml-base.en.bin");
        anyhow::ensure!(
            model_path.is_file(),
            "missing model file {}",
            model_path.display()
        );
        let context =
            WhisperContext::new_with_params(&model_path, WhisperContextParameters::default())
                .context("loading whisper.cpp model")?;
        Ok(Self { context })
    }
}

impl SttEngine for WhisperCppEngine {
    fn transcribe(&mut self, samples: &[f32], sample_rate: u32) -> anyhow::Result<String> {
        anyhow::ensure!(
            sample_rate == 16_000,
            "whisper.cpp requires 16 kHz input, got {sample_rate}"
        );
        let mut state = self.context.create_state().context("creating state")?;
        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
        params.set_language(Some("en"));
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_special(false);
        params.set_print_timestamps(false);
        let threads = std::thread::available_parallelism()
            .map(|n| (n.get() / 2).clamp(2, 8) as i32)
            .unwrap_or(4);
        params.set_n_threads(threads);

        state.full(params, samples).context("running whisper")?;

        let mut text = String::new();
        for segment in state.as_iter() {
            text.push_str(&segment.to_str_lossy().context("segment text")?);
        }
        Ok(text)
    }
}
