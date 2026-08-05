//! UI state fan-out: tray tooltip, recording-indicator overlay, and events.

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, Runtime};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum UiState {
    Idle,
    Recording,
    Transcribing,
}

impl UiState {
    fn tooltip(self) -> &'static str {
        match self {
            UiState::Idle => "Murmur — idle",
            UiState::Recording => "Murmur — recording…",
            UiState::Transcribing => "Murmur — transcribing…",
        }
    }

    fn tray_icon_bytes(self) -> &'static [u8] {
        match self {
            UiState::Idle => include_bytes!("../icons/tray-idle.png"),
            UiState::Recording => include_bytes!("../icons/tray-recording.png"),
            UiState::Transcribing => include_bytes!("../icons/tray-transcribing.png"),
        }
    }
}

/// Reflect a dictation state change across every UI surface.
/// Safe to call from any thread: the dictation pipeline runs on background
/// threads, but tray/window handles must only be touched on the main thread
/// (see [`crate::on_main_thread`]).
pub fn apply<R: Runtime>(app: &AppHandle<R>, state: UiState) {
    crate::on_main_thread(app, move |app| apply_on_main(app, state));
}

fn apply_on_main<R: Runtime>(app: &AppHandle<R>, state: UiState) {
    if let Some(tray) = app.tray_by_id(crate::tray::TRAY_ID) {
        let _ = tray.set_tooltip(Some(state.tooltip()));
        match tauri::image::Image::from_bytes(state.tray_icon_bytes()) {
            Ok(icon) => {
                let _ = tray.set_icon(Some(icon));
            }
            Err(e) => log::warn!("could not decode tray icon: {e}"),
        }
    }

    // The overlay listens for this to swap its label/animation.
    let _ = app.emit("dictation-state", state);

    if let Some(overlay) = app.get_webview_window("overlay") {
        match state {
            UiState::Idle => {
                let _ = overlay.hide();
            }
            UiState::Recording | UiState::Transcribing => {
                position_bottom_center(&overlay);
                let _ = overlay.show();
            }
        }
    }
}

/// Park the overlay pill bottom-center on the monitor with the cursor
/// (falling back to the primary monitor).
fn position_bottom_center<R: Runtime>(window: &tauri::WebviewWindow<R>) {
    let cursor = window.cursor_position().map(|p| (p.x, p.y)).ok();
    let monitor = cursor
        .and_then(|(x, y)| window.monitor_from_point(x, y).ok().flatten())
        .or_else(|| window.primary_monitor().ok().flatten());
    let Some(monitor) = monitor else { return };
    let Ok(size) = window.outer_size() else {
        return;
    };
    let mpos = monitor.position();
    let msize = monitor.size();
    let x = mpos.x + ((msize.width as i32 - size.width as i32) / 2).max(0);
    let y = mpos.y + msize.height as i32 - size.height as i32 - 96;
    let _ = window.set_position(tauri::PhysicalPosition::new(x, y));
}

/// One-time overlay setup: clicks fall through to whatever is underneath.
pub fn init<R: Runtime>(app: &AppHandle<R>) {
    if let Some(overlay) = app.get_webview_window("overlay") {
        let _ = overlay.set_ignore_cursor_events(true);
    }
}
