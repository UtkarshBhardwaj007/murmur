//! Deliver a transcript: clipboard write, synthetic paste keystroke, and
//! clipboard restore.

use std::time::Duration;

use anyhow::Context;
use arboard::Clipboard;
use enigo::{Direction, Enigo, Key, Keyboard, Settings as EnigoSettings};

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

/// Put `text` on the clipboard. When `auto_paste` is true and the platform
/// allows synthetic input, also send Cmd/Ctrl+V at the cursor position and
/// restore the previous clipboard contents about a second later.
///
/// Runs on a background thread (it sleeps).
pub fn deliver(text: &str, auto_paste: bool) -> anyhow::Result<()> {
    let mut clipboard = Clipboard::new().context("opening clipboard")?;

    if !auto_paste {
        clipboard.set_text(text).context("writing clipboard")?;
        log::info!("transcript copied to clipboard (auto-paste off)");
        return Ok(());
    }

    if !can_synthesize_input() {
        clipboard.set_text(text).context("writing clipboard")?;
        log::warn!(
            "auto-paste unavailable (missing input permission); transcript left on clipboard"
        );
        return Ok(());
    }

    let saved = backup(&mut clipboard);
    clipboard.set_text(text).context("writing clipboard")?;

    // Give the clipboard a moment to settle before the paste keystroke —
    // some targets read it asynchronously.
    std::thread::sleep(Duration::from_millis(80));

    match send_paste_keystroke() {
        Ok(()) => {
            log::info!("pasted transcript at cursor");
            // Restore the user's clipboard after the target has read ours.
            std::thread::sleep(Duration::from_millis(1000));
            restore(&mut clipboard, saved);
        }
        Err(e) => {
            log::error!("synthetic paste failed: {e:#}; transcript left on clipboard");
            // Leave the transcript on the clipboard so the user can paste
            // manually — do NOT restore the backup over it.
        }
    }
    Ok(())
}

fn send_paste_keystroke() -> anyhow::Result<()> {
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
