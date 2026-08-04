//! Tauri commands exposed to the settings window.

use std::sync::atomic::{AtomicBool, Ordering};

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, Runtime};

use crate::dictation::DictationState;
use crate::models::{self, ModelId};

/// Guards against two concurrent model downloads.
pub struct DownloadGuard(pub AtomicBool);

#[derive(Serialize)]
pub struct ModelStatus {
    pub id: ModelId,
    pub name: String,
    pub installed: bool,
    pub active: bool,
    pub total_bytes: u64,
}

#[tauri::command]
pub fn model_status<R: Runtime>(app: AppHandle<R>) -> Result<Vec<ModelStatus>, String> {
    let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let active = app.state::<DictationState>().active_model();
    Ok(ModelId::ALL
        .iter()
        .map(|&id| ModelStatus {
            id,
            name: id.spec().display_name.to_string(),
            installed: models::is_installed(&data_dir, id),
            active: id == active,
            total_bytes: id.spec().total_bytes(),
        })
        .collect())
}

#[tauri::command]
pub async fn download_model<R: Runtime>(app: AppHandle<R>, id: ModelId) -> Result<(), String> {
    let guard = app.state::<DownloadGuard>();
    if guard
        .0
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return Err("a model download is already in progress".into());
    }

    let result = do_download(&app, id).await;
    app.state::<DownloadGuard>()
        .0
        .store(false, Ordering::SeqCst);
    match &result {
        Ok(()) => {
            log::info!("model {id:?} downloaded and verified");
            let _ = app.emit("model-download-complete", id);
        }
        Err(e) => {
            log::error!("model download failed: {e}");
            let _ = app.emit("model-download-error", e.clone());
        }
    }
    result
}

async fn do_download<R: Runtime>(app: &AppHandle<R>, id: ModelId) -> Result<(), String> {
    let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let emitter = app.clone();
    models::download_model(&data_dir, id, move |progress| {
        let _ = emitter.emit("model-download-progress", &progress);
    })
    .await
    .map_err(|e| format!("{e:#}"))
}
