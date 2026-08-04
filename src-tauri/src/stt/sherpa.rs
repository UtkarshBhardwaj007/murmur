//! sherpa-onnx offline transducer engine (Parakeet TDT).

use std::path::Path;

use anyhow::{anyhow, Context};
use sherpa_rs::transducer::{TransducerConfig, TransducerRecognizer};

use super::SttEngine;

pub struct SherpaTransducerEngine {
    recognizer: TransducerRecognizer,
}

impl SherpaTransducerEngine {
    pub fn load(model_dir: &Path) -> anyhow::Result<Self> {
        let path = |name: &str| -> anyhow::Result<String> {
            let p = model_dir.join(name);
            anyhow::ensure!(p.is_file(), "missing model file {}", p.display());
            Ok(p.to_string_lossy().into_owned())
        };

        let threads = std::thread::available_parallelism()
            .map(|n| (n.get() / 2).clamp(2, 8) as i32)
            .unwrap_or(4);

        let config = TransducerConfig {
            encoder: path("encoder.int8.onnx")?,
            decoder: path("decoder.int8.onnx")?,
            joiner: path("joiner.int8.onnx")?,
            tokens: path("tokens.txt")?,
            model_type: "nemo_transducer".into(),
            num_threads: threads,
            sample_rate: 16_000,
            feature_dim: 80,
            decoding_method: "greedy_search".into(),
            debug: false,
            ..Default::default()
        };
        let recognizer = TransducerRecognizer::new(config)
            .map_err(|e| anyhow!("{e}"))
            .context("creating sherpa-onnx transducer recognizer")?;
        Ok(Self { recognizer })
    }
}

impl SttEngine for SherpaTransducerEngine {
    fn transcribe(&mut self, samples: &[f32], sample_rate: u32) -> anyhow::Result<String> {
        Ok(self.recognizer.transcribe(sample_rate, samples))
    }
}
