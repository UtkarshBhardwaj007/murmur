//! The dictation state machine: start/stop recording and (for now) dump the
//! capture to a WAV file. The STT milestone replaces the WAV dump with
//! transcription.

use std::sync::Mutex;

use tauri::{AppHandle, Manager, Runtime};

use crate::audio::{self, Recorder};

pub struct DictationState {
    recorder: Recorder,
    recording: Mutex<bool>,
}

impl DictationState {
    pub fn new() -> Self {
        Self {
            recorder: Recorder::spawn(),
            recording: Mutex::new(false),
        }
    }
}

/// Toggle dictation: start recording if idle, otherwise stop and process.
/// Called from the global hotkey handler and the tray menu.
pub fn toggle<R: Runtime>(app: &AppHandle<R>) {
    let state = app.state::<DictationState>();
    let mut recording = state.recording.lock().expect("recording flag");
    if *recording {
        *recording = false;
        drop(recording);
        stop_and_process(app);
    } else {
        match state.recorder.start() {
            Ok(()) => {
                *recording = true;
                set_tray_tooltip(app, "Murmur — recording…");
                log::info!("recording started");
            }
            Err(e) => log::error!("failed to start recording: {e:#}"),
        }
    }
}

fn stop_and_process<R: Runtime>(app: &AppHandle<R>) {
    let state = app.state::<DictationState>();
    set_tray_tooltip(app, "Murmur — transcribing…");
    let samples = match state.recorder.stop() {
        Ok(s) => s,
        Err(e) => {
            log::error!("failed to stop recording: {e:#}");
            set_tray_tooltip(app, "Murmur — idle");
            return;
        }
    };
    log::info!(
        "recording stopped: {:.2}s at {} Hz",
        samples.len() as f32 / audio::TARGET_SAMPLE_RATE as f32,
        audio::TARGET_SAMPLE_RATE
    );

    // Milestone 2: persist the capture as a WAV for inspection. Later
    // milestones feed `samples` to the STT engine instead.
    match save_debug_wav(app, &samples) {
        Ok(path) => log::info!("saved recording to {}", path.display()),
        Err(e) => log::error!("failed to save WAV: {e:#}"),
    }
    set_tray_tooltip(app, "Murmur — idle");
}

fn save_debug_wav<R: Runtime>(
    app: &AppHandle<R>,
    samples: &[f32],
) -> anyhow::Result<std::path::PathBuf> {
    let dir = app.path().app_data_dir()?.join("recordings");
    std::fs::create_dir_all(&dir)?;
    let path = dir.join("last-recording.wav");
    audio::wav::write_wav_mono(&path, samples, audio::TARGET_SAMPLE_RATE)?;
    Ok(path)
}

fn set_tray_tooltip<R: Runtime>(app: &AppHandle<R>, text: &str) {
    if let Some(tray) = app.tray_by_id(crate::tray::TRAY_ID) {
        let _ = tray.set_tooltip(Some(text));
    }
}
