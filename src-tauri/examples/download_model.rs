//! Developer utility: download and verify a model outside the app.
//!
//! ```sh
//! cargo run --example download_model -- <app-data-dir> [parakeet|whisper]
//! ```

use murmur_lib::models::{self, ModelId};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let dir = std::env::args()
        .nth(1)
        .map(std::path::PathBuf::from)
        .expect("usage: download_model <app-data-dir> [parakeet|whisper]");
    let id = match std::env::args().nth(2).as_deref() {
        Some("whisper") => ModelId::WhisperBaseEn,
        _ => ModelId::ParakeetTdt06bV2Int8,
    };

    models::download_model(&dir, id, |p| {
        eprint!(
            "\r{}: {:.1} / {:.1} MB   ",
            p.file,
            p.downloaded as f64 / 1e6,
            p.total as f64 / 1e6
        );
    })
    .await?;
    eprintln!();

    println!("running full checksum verification…");
    models::verify_installed(&dir, id)?;
    println!("{id:?} downloaded and verified OK");
    Ok(())
}
