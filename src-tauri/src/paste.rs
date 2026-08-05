//! Deliver a transcript: clipboard write, synthetic paste keystroke, and
//! clipboard restore.
//!
//! Threading contract: [`deliver`] runs on a background worker (it sleeps),
//! but the paste keystroke itself is dispatched to the main thread.
//! macOS's Text Input Sources services (which enigo consults to map keys)
//! `dispatch_assert_queue` on the main queue and **kill the process with
//! SIGTRAP** when called from any other thread.

use std::time::Duration;

use anyhow::Context;
use arboard::Clipboard;
use enigo::{Direction, Enigo, Key, Keyboard, Settings as EnigoSettings};
use tauri::{AppHandle, Runtime};

/// How a transcript should reach the user.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeliveryPlan {
    /// Copy to the clipboard and stop (auto-paste off, or no permission
    /// to synthesize input).
    ClipboardOnly,
    /// Copy, synthesize Cmd/Ctrl+V, then restore the previous clipboard.
    PasteAtCursor,
}

/// Pure decision: what to do with a finished transcript.
pub fn plan_delivery(auto_paste: bool, can_synthesize: bool) -> DeliveryPlan {
    if auto_paste && can_synthesize {
        DeliveryPlan::PasteAtCursor
    } else {
        DeliveryPlan::ClipboardOnly
    }
}

/// What was on the clipboard before we replaced it.
enum ClipboardBackup {
    Text(String),
    Image(arboard::ImageData<'static>),
    Empty,
}

fn backup(clipboard: &mut Clipboard) -> ClipboardBackup {
    if let Ok(text) = clipboard.get_text() {
        return ClipboardBackup::Text(text);
    }
    if let Ok(image) = clipboard.get_image() {
        return ClipboardBackup::Image(image.to_owned_img());
    }
    ClipboardBackup::Empty
}

fn restore(clipboard: &mut Clipboard, saved: ClipboardBackup) {
    let result = match saved {
        ClipboardBackup::Text(text) => clipboard.set_text(text),
        ClipboardBackup::Image(image) => clipboard.set_image(image),
        ClipboardBackup::Empty => clipboard.clear(),
    };
    if let Err(e) = result {
        log::warn!("could not restore previous clipboard contents: {e}");
    }
}

/// Put `text` on the clipboard. When the delivery plan allows it, also send
/// Cmd/Ctrl+V at the cursor position (on the main thread) and restore the
/// previous clipboard contents about a second later.
///
/// Runs on a background thread (it sleeps).
pub fn deliver<R: Runtime>(app: &AppHandle<R>, text: &str, auto_paste: bool) -> anyhow::Result<()> {
    let mut clipboard = Clipboard::new().context("opening clipboard")?;

    match plan_delivery(auto_paste, can_synthesize_input()) {
        DeliveryPlan::ClipboardOnly => {
            clipboard.set_text(text).context("writing clipboard")?;
            if auto_paste {
                log::warn!(
                    "auto-paste unavailable (missing input permission); transcript left on clipboard"
                );
            } else {
                log::info!("transcript copied to clipboard (auto-paste off)");
            }
            Ok(())
        }
        DeliveryPlan::PasteAtCursor => {
            let saved = backup(&mut clipboard);
            clipboard.set_text(text).context("writing clipboard")?;

            // Give the clipboard a moment to settle before the paste
            // keystroke — some targets read it asynchronously.
            std::thread::sleep(Duration::from_millis(80));

            match paste_on_main_thread(app) {
                Ok(()) => {
                    log::info!("pasted transcript at cursor");
                    // Restore the user's clipboard after the target has
                    // read ours.
                    std::thread::sleep(Duration::from_millis(1000));
                    restore(&mut clipboard, saved);
                }
                Err(e) => {
                    log::error!("synthetic paste failed: {e:#}; transcript left on clipboard");
                    // Leave the transcript on the clipboard so the user can
                    // paste manually — do NOT restore the backup over it.
                }
            }
            Ok(())
        }
    }
}

/// Dispatch the paste keystroke to the main thread and wait for its result.
fn paste_on_main_thread<R: Runtime>(app: &AppHandle<R>) -> anyhow::Result<()> {
    let (tx, rx) = std::sync::mpsc::sync_channel(1);
    app.run_on_main_thread(move || {
        let _ = tx.send(send_paste_keystroke());
    })
    .context("scheduling paste on the main thread")?;
    rx.recv_timeout(Duration::from_secs(5))
        .context("main thread did not run the paste keystroke in time")?
}

/// Synthesize Cmd/Ctrl+V.
///
/// Must run on the main thread on macOS: enigo's key mapping calls into
/// HIToolbox's Text Input Sources, which aborts the process (SIGTRAP via
/// `dispatch_assert_queue`) off the main queue. The guard turns that crash
/// into an error.
pub fn send_paste_keystroke() -> anyhow::Result<()> {
    #[cfg(target_os = "macos")]
    anyhow::ensure!(
        objc2_foundation::NSThread::isMainThread_class(),
        "synthetic keystrokes must be sent from the main thread on macOS \
         (Text Input Sources aborts the process otherwise)"
    );

    let mut enigo =
        Enigo::new(&EnigoSettings::default()).map_err(|e| anyhow::anyhow!("enigo init: {e}"))?;
    let modifier = if cfg!(target_os = "macos") {
        Key::Meta
    } else {
        Key::Control
    };
    enigo
        .key(modifier, Direction::Press)
        .map_err(|e| anyhow::anyhow!("modifier press: {e}"))?;
    let result = enigo
        .key(Key::Unicode('v'), Direction::Click)
        .map_err(|e| anyhow::anyhow!("V press: {e}"));
    // Always release the modifier, even if the V press failed.
    enigo
        .key(modifier, Direction::Release)
        .map_err(|e| anyhow::anyhow!("modifier release: {e}"))?;
    result?;
    Ok(())
}

/// Whether the OS will let us send synthetic keystrokes.
#[cfg(target_os = "macos")]
pub fn can_synthesize_input() -> bool {
    macos_accessibility_client::accessibility::application_is_trusted()
}

#[cfg(not(target_os = "macos"))]
pub fn can_synthesize_input() -> bool {
    true
}

/// Ask macOS to show the "grant Accessibility access" prompt for this app.
#[cfg(target_os = "macos")]
pub fn prompt_for_permission() -> bool {
    macos_accessibility_client::accessibility::application_is_trusted_with_prompt()
}

#[cfg(not(target_os = "macos"))]
pub fn prompt_for_permission() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auto_paste_with_permission_pastes_at_cursor() {
        assert_eq!(plan_delivery(true, true), DeliveryPlan::PasteAtCursor);
    }

    #[test]
    fn auto_paste_without_permission_falls_back_to_clipboard() {
        assert_eq!(plan_delivery(true, false), DeliveryPlan::ClipboardOnly);
    }

    #[test]
    fn clipboard_only_when_auto_paste_is_off() {
        assert_eq!(plan_delivery(false, true), DeliveryPlan::ClipboardOnly);
        assert_eq!(plan_delivery(false, false), DeliveryPlan::ClipboardOnly);
    }

    /// Regression test for the SIGTRAP crash: sending the paste keystroke
    /// from a background thread must return an error — never reach enigo's
    /// Text Input Sources lookup, which would abort the whole process.
    #[cfg(target_os = "macos")]
    #[test]
    fn keystroke_off_main_thread_errors_instead_of_crashing() {
        let result = std::thread::spawn(send_paste_keystroke)
            .join()
            .expect("guard must return an error, not panic or abort");
        let error = result.expect_err("off-main-thread keystroke must be rejected");
        assert!(
            error.to_string().contains("main thread"),
            "unexpected error: {error:#}"
        );
    }
}
