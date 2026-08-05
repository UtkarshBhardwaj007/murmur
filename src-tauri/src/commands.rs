//! Tauri commands exposed to the settings window.

use std::sync::atomic::{AtomicBool, Ordering};

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, Runtime};

use crate::models::{self, ModelId};
use crate::settings::SettingsState;

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
    let active = app.state::<SettingsState>().get().model;
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
pub fn get_settings<R: Runtime>(app: AppHandle<R>) -> crate::settings::Settings {
    app.state::<SettingsState>().get()
}

/// Persist new settings and apply their side effects (hotkey registration,
/// launch-at-login). Returns an error message when the hotkey can't be
/// registered; the previous hotkey stays active in that case.
#[tauri::command]
pub fn set_settings<R: Runtime>(
    app: AppHandle<R>,
    new: crate::settings::Settings,
) -> Result<(), String> {
    let state = app.state::<SettingsState>();
    let old = state.get();

    if new.hotkey != old.hotkey {
        if let Err(e) = crate::apply_hotkey(&app, &new.hotkey) {
            // Try to keep the old binding working.
            if let Err(revert) = crate::apply_hotkey(&app, &old.hotkey) {
                log::error!("failed to restore previous hotkey: {revert:#}");
            }
            return Err(format!("could not register {:?}: {e:#}", new.hotkey));
        }
    }

    if new.launch_at_login != old.launch_at_login {
        use tauri_plugin_autostart::ManagerExt;
        let autolaunch = app.autolaunch();
        let result = if new.launch_at_login {
            autolaunch.enable()
        } else {
            autolaunch.disable()
        };
        if let Err(e) = result {
            log::error!("launch-at-login change failed: {e}");
            return Err(format!("could not update launch at login: {e}"));
        }
    }

    if new.model != old.model {
        log::info!("active model switched to {:?}", new.model);
    }

    state.update(new).map_err(|e| format!("{e:#}"))
}

/// Current microphone permission, without prompting.
#[tauri::command]
pub fn microphone_status() -> crate::mic::MicPermission {
    crate::mic::status()
}

/// Open the OS settings pane for microphone privacy (no-op elsewhere).
#[tauri::command]
pub fn open_microphone_settings() -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_Microphone")
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// True when the OS lets us send the synthetic paste keystroke.
#[tauri::command]
pub fn accessibility_status() -> bool {
    crate::paste::can_synthesize_input()
}

/// Trigger the macOS Accessibility permission prompt (no-op elsewhere).
#[tauri::command]
pub fn request_accessibility() -> bool {
    crate::paste::prompt_for_permission()
}

/// Open the OS settings pane where the user can grant the permission.
#[tauri::command]
pub fn open_accessibility_settings() -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility")
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    Ok(())
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
