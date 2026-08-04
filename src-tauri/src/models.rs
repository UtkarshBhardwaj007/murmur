//! STT model registry and download manager.
//!
//! Model files are fetched from their official Hugging Face repositories on
//! first use and verified against SHA-256 checksums pinned here. Nothing in
//! this module performs inference; see [`crate::stt`].

use std::io::Read;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, bail, Context};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModelId {
    /// NVIDIA Parakeet TDT 0.6B v2, int8 ONNX (sherpa-onnx export). Default.
    #[serde(rename = "parakeet-tdt-0.6b-v2-int8")]
    ParakeetTdt06bV2Int8,
    /// whisper.cpp base.en (ggml). Smaller download, slightly lower quality.
    #[serde(rename = "whisper-base-en")]
    WhisperBaseEn,
}

impl ModelId {
    pub const ALL: &'static [ModelId] = &[ModelId::ParakeetTdt06bV2Int8, ModelId::WhisperBaseEn];

    pub fn spec(self) -> &'static ModelSpec {
        match self {
            ModelId::ParakeetTdt06bV2Int8 => &PARAKEET,
            ModelId::WhisperBaseEn => &WHISPER_BASE_EN,
        }
    }
}

pub struct ModelFile {
    /// File name inside the model directory.
    pub name: &'static str,
    pub url: &'static str,
    pub sha256: &'static str,
    pub size: u64,
}

pub struct ModelSpec {
    pub id: ModelId,
    pub display_name: &'static str,
    /// Directory name under `<app-data>/models/`.
    pub dir_name: &'static str,
    pub files: &'static [ModelFile],
}

impl ModelSpec {
    pub fn total_bytes(&self) -> u64 {
        self.files.iter().map(|f| f.size).sum()
    }
}

static PARAKEET: ModelSpec = ModelSpec {
    id: ModelId::ParakeetTdt06bV2Int8,
    display_name: "Parakeet TDT 0.6B v2 (int8) — best quality, English",
    dir_name: "parakeet-tdt-0.6b-v2-int8",
    files: &[
        ModelFile {
            name: "encoder.int8.onnx",
            url: "https://huggingface.co/csukuangfj/sherpa-onnx-nemo-parakeet-tdt-0.6b-v2-int8/resolve/main/encoder.int8.onnx",
            sha256: "a32b12d17bbbc309d0686fbbcc2987b5e9b8333a7da83fa6b089f0a2acd651ab",
            size: 652_184_296,
        },
        ModelFile {
            name: "decoder.int8.onnx",
            url: "https://huggingface.co/csukuangfj/sherpa-onnx-nemo-parakeet-tdt-0.6b-v2-int8/resolve/main/decoder.int8.onnx",
            sha256: "b6bb64963457237b900e496ee9994b59294526439fbcc1fecf705b31a15c6b4e",
            size: 7_257_753,
        },
        ModelFile {
            name: "joiner.int8.onnx",
            url: "https://huggingface.co/csukuangfj/sherpa-onnx-nemo-parakeet-tdt-0.6b-v2-int8/resolve/main/joiner.int8.onnx",
            sha256: "7946164367946e7f9f29a122407c3252b680dbae9a51343eb2488d057c3c43d2",
            size: 1_739_080,
        },
        ModelFile {
            name: "tokens.txt",
            url: "https://huggingface.co/csukuangfj/sherpa-onnx-nemo-parakeet-tdt-0.6b-v2-int8/resolve/main/tokens.txt",
            sha256: "ec182b70dd42113aff6c5372c75cac58c952443eb22322f57bbd7f53977d497d",
            size: 9_384,
        },
    ],
};

static WHISPER_BASE_EN: ModelSpec = ModelSpec {
    id: ModelId::WhisperBaseEn,
    display_name: "Whisper base.en (ggml) — smaller download, English",
    dir_name: "whisper-base-en",
    files: &[ModelFile {
        name: "ggml-base.en.bin",
        url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.en.bin",
        sha256: "a03779c86df3323075f5e796cb2ce5029f00ec8869eee3fdfb897afe36c6d002",
        size: 147_964_211,
    }],
};

/// Root directory that holds one subdirectory per model.
pub fn models_root(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join("models")
}

pub fn model_dir(app_data_dir: &Path, id: ModelId) -> PathBuf {
    models_root(app_data_dir).join(id.spec().dir_name)
}

/// Cheap installed check: every file exists with the expected size.
/// Integrity is guaranteed by checksum verification at download time.
pub fn is_installed(app_data_dir: &Path, id: ModelId) -> bool {
    let dir = model_dir(app_data_dir, id);
    id.spec().files.iter().all(|f| {
        dir.join(f.name)
            .metadata()
            .map(|m| m.len() == f.size)
            .unwrap_or(false)
    })
}

/// Compute the SHA-256 of a file as a lowercase hex string.
pub fn file_sha256(path: &Path) -> anyhow::Result<String> {
    let mut file = std::fs::File::open(path)
        .with_context(|| format!("opening {} for hashing", path.display()))?;
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 1 << 20];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hex::encode(hasher.finalize()))
}

/// Progress reported while downloading a model.
#[derive(Debug, Clone, Serialize)]
pub struct DownloadProgress {
    pub model: ModelId,
    pub file: String,
    /// Bytes downloaded across the whole model so far.
    pub downloaded: u64,
    /// Total bytes for the whole model.
    pub total: u64,
}

/// Download every file of `id` into the model directory, verifying each
/// against its pinned SHA-256. Files that are already present and valid are
/// skipped. `on_progress` is called at most a few times per second.
pub async fn download_model(
    app_data_dir: &Path,
    id: ModelId,
    on_progress: impl Fn(DownloadProgress),
) -> anyhow::Result<()> {
    let spec = id.spec();
    let dir = model_dir(app_data_dir, id);
    tokio::fs::create_dir_all(&dir)
        .await
        .with_context(|| format!("creating {}", dir.display()))?;

    let client = reqwest::Client::builder()
        .user_agent(concat!("murmur/", env!("CARGO_PKG_VERSION")))
        .build()?;

    let total = spec.total_bytes();
    let mut done_bytes: u64 = 0;

    for file in spec.files {
        let dest = dir.join(file.name);
        if dest
            .metadata()
            .map(|m| m.len() == file.size)
            .unwrap_or(false)
        {
            log::info!("{} already present, skipping", file.name);
            done_bytes += file.size;
            continue;
        }

        log::info!("downloading {} ({} bytes)", file.url, file.size);
        let part = dir.join(format!("{}.part", file.name));
        let mut hasher = Sha256::new();
        {
            let mut out = tokio::fs::File::create(&part)
                .await
                .with_context(|| format!("creating {}", part.display()))?;
            let resp = client.get(file.url).send().await?.error_for_status()?;
            let mut stream = resp.bytes_stream();
            let mut file_bytes: u64 = 0;
            let mut last_emit = std::time::Instant::now();
            while let Some(chunk) = stream.next().await {
                let chunk = chunk?;
                hasher.update(&chunk);
                tokio::io::AsyncWriteExt::write_all(&mut out, &chunk).await?;
                file_bytes += chunk.len() as u64;
                if last_emit.elapsed().as_millis() >= 200 {
                    last_emit = std::time::Instant::now();
                    on_progress(DownloadProgress {
                        model: id,
                        file: file.name.to_string(),
                        downloaded: done_bytes + file_bytes,
                        total,
                    });
                }
            }
            tokio::io::AsyncWriteExt::flush(&mut out).await?;
        }

        let actual = hex::encode(hasher.finalize());
        if actual != file.sha256 {
            let _ = tokio::fs::remove_file(&part).await;
            bail!(
                "checksum mismatch for {}: expected {}, got {actual}",
                file.name,
                file.sha256
            );
        }
        let written = part.metadata()?.len();
        if written != file.size {
            let _ = tokio::fs::remove_file(&part).await;
            bail!(
                "size mismatch for {}: expected {} bytes, got {written}",
                file.name,
                file.size
            );
        }
        tokio::fs::rename(&part, &dest)
            .await
            .with_context(|| format!("renaming {} into place", part.display()))?;
        done_bytes += file.size;
        on_progress(DownloadProgress {
            model: id,
            file: file.name.to_string(),
            downloaded: done_bytes,
            total,
        });
        log::info!("verified {} ({})", file.name, file.sha256);
    }

    Ok(())
}

/// Verify all files of an installed model against their pinned checksums.
/// Slow for large models; intended for tests and a future "repair" action.
pub fn verify_installed(app_data_dir: &Path, id: ModelId) -> anyhow::Result<()> {
    let dir = model_dir(app_data_dir, id);
    for file in id.spec().files {
        let path = dir.join(file.name);
        let actual = file_sha256(&path)?;
        if actual != file.sha256 {
            return Err(anyhow!(
                "checksum mismatch for {}: expected {}, got {actual}",
                path.display(),
                file.sha256
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_is_sane() {
        for &id in ModelId::ALL {
            let spec = id.spec();
            assert!(!spec.files.is_empty());
            assert!(spec.total_bytes() > 0);
            for f in spec.files {
                assert!(f.url.starts_with("https://"), "{} not https", f.url);
                assert_eq!(f.sha256.len(), 64, "{} bad sha length", f.name);
                assert!(
                    f.sha256.chars().all(|c| c.is_ascii_hexdigit()),
                    "{} bad sha chars",
                    f.name
                );
                assert!(f.size > 0);
            }
        }
    }

    #[test]
    fn model_ids_serialize_to_stable_strings() {
        assert_eq!(
            serde_json::to_string(&ModelId::ParakeetTdt06bV2Int8).unwrap(),
            "\"parakeet-tdt-0.6b-v2-int8\""
        );
        assert_eq!(
            serde_json::to_string(&ModelId::WhisperBaseEn).unwrap(),
            "\"whisper-base-en\""
        );
    }

    #[test]
    fn file_sha256_matches_known_vector() {
        // SHA-256 of the ASCII string "murmur" (no newline).
        let dir = std::env::temp_dir().join("murmur-sha-test");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("vector.txt");
        std::fs::write(&path, b"murmur").unwrap();
        assert_eq!(
            file_sha256(&path).unwrap(),
            "6200f53485b683973d0c8cb0da433414326ca268363546ece184689555b06568"
        );
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn is_installed_false_on_missing_and_true_on_size_match() {
        let root = std::env::temp_dir().join("murmur-install-test");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        assert!(!is_installed(&root, ModelId::WhisperBaseEn));

        // Fabricate a file with the right name and size but bogus content:
        // is_installed is a cheap size check by design.
        let dir = model_dir(&root, ModelId::WhisperBaseEn);
        std::fs::create_dir_all(&dir).unwrap();
        let f = &ModelId::WhisperBaseEn.spec().files[0];
        let file = std::fs::File::create(dir.join(f.name)).unwrap();
        file.set_len(f.size).unwrap();
        assert!(is_installed(&root, ModelId::WhisperBaseEn));

        // But full verification catches the bogus content.
        assert!(verify_installed(&root, ModelId::WhisperBaseEn).is_err());

        std::fs::remove_dir_all(&root).unwrap();
    }
}
