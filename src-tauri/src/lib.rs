pub mod audio;
mod commands;
mod dictation;
pub mod mic;
pub mod models;
mod overlay;
mod paste;
pub mod postprocess;
pub mod settings;
pub mod stt;
mod tray;

use tauri::Manager;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

use settings::SettingsState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(
            tauri_plugin_log::Builder::new()
                .level(log::LevelFilter::Info)
                .build(),
        )
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    dictation::on_hotkey(app, event.state() == ShortcutState::Pressed);
                })
                .build(),
        )
        .setup(|app| {
            app.manage(SettingsState::load_or_default());
            app.manage(dictation::DictationState::new());
            app.manage(commands::DownloadGuard(std::sync::atomic::AtomicBool::new(
                false,
            )));
            tray::create_tray(app.handle())?;
            overlay::init(app.handle());

            let hotkey = app.state::<SettingsState>().get().hotkey;
            if let Err(e) = apply_hotkey(app.handle(), &hotkey) {
                log::error!("failed to register hotkey {hotkey:?}: {e}");
            }

            // Auto-paste needs Accessibility on macOS; surface the guidance
            // flow up front rather than failing on the first dictation.
            if app.state::<SettingsState>().get().auto_paste && !paste::can_synthesize_input() {
                log::warn!("input-synthesis permission missing; showing guidance");
                show_settings(app.handle());
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
        .invoke_handler(tauri::generate_handler![
            commands::get_settings,
            commands::set_settings,
            commands::model_status,
            commands::download_model,
            commands::accessibility_status,
            commands::request_accessibility,
            commands::open_accessibility_settings,
            commands::microphone_status,
            commands::open_microphone_settings
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// Replace whatever global shortcut is registered with `hotkey`.
/// Returns an error (leaving no shortcut registered) if the string does not
/// parse or the OS refuses the binding.
pub fn apply_hotkey<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    hotkey: &str,
) -> anyhow::Result<()> {
    let gs = app.global_shortcut();
    gs.unregister_all()
        .map_err(|e| anyhow::anyhow!("unregistering shortcuts: {e}"))?;
    gs.register(hotkey)
        .map_err(|e| anyhow::anyhow!("registering {hotkey:?}: {e}"))?;
    log::info!("registered global hotkey {hotkey}");
    Ok(())
}

/// Show and focus the settings window, restoring it when hidden.
pub fn show_settings<R: tauri::Runtime>(app: &tauri::AppHandle<R>) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}
