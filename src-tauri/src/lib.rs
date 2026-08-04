mod audio;
mod dictation;
mod tray;

use tauri::Manager;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

/// Hardcoded default hotkey until the settings milestone makes it
/// configurable.
#[cfg(target_os = "macos")]
const DEFAULT_HOTKEY: &str = "Cmd+Shift+Space";
#[cfg(not(target_os = "macos"))]
const DEFAULT_HOTKEY: &str = "Ctrl+Shift+Space";

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(
            tauri_plugin_log::Builder::new()
                .level(log::LevelFilter::Info)
                .build(),
        )
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    if event.state() == ShortcutState::Pressed {
                        dictation::toggle(app);
                    }
                })
                .build(),
        )
        .setup(|app| {
            app.manage(dictation::DictationState::new());
            tray::create_tray(app.handle())?;
            match app.global_shortcut().register(DEFAULT_HOTKEY) {
                Ok(()) => log::info!("registered global hotkey {DEFAULT_HOTKEY}"),
                Err(e) => log::error!("failed to register hotkey {DEFAULT_HOTKEY}: {e}"),
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            // Murmur is a tray app: closing the settings window hides it
            // instead of quitting.
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                if window.label() == "main" {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// Show and focus the settings window, restoring it when hidden.
pub fn show_settings<R: tauri::Runtime>(app: &tauri::AppHandle<R>) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}
