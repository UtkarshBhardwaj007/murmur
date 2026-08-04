//! Mock engine for tests and CI: no model files, no inference.

use super::SttEngine;

/// Returns a canned transcript and records what it was asked to transcribe.
pub struct MockEngine {
    pub canned: String,
    pub calls: Vec<(usize, u32)>,
}

impl MockEngine {
    pub fn new(canned: impl Into<String>) -> Self {
        Self {
            canned: canned.into(),
            calls: Vec::new(),
        }
    }
}

impl SttEngine for MockEngine {
    fn transcribe(&mut self, samples: &[f32], sample_rate: u32) -> anyhow::Result<String> {
        self.calls.push((samples.len(), sample_rate));
        Ok(self.canned.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mock_engine_satisfies_the_trait() {
        let mut engine: Box<dyn SttEngine> = Box::new(MockEngine::new("hello world"));
        let out = engine.transcribe(&[0.0; 1600], 16_000).unwrap();
        assert_eq!(out, "hello world");
    }

    #[test]
    fn mock_engine_records_calls() {
        let mut engine = MockEngine::new("x");
        engine.transcribe(&[0.0; 320], 16_000).unwrap();
        engine.transcribe(&[0.0; 640], 48_000).unwrap();
        assert_eq!(engine.calls, vec![(320, 16_000), (640, 48_000)]);
    }
}
