//! The dictation state machine: record, transcribe, and (in a later
//! milestone) paste the transcript.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

use tauri::{AppHandle, Emitter, Manager, Runtime};

use crate::audio::{self, Recorder};
use crate::models::{self, ModelId};
use crate::overlay::{self, UiState};
use crate::settings::{DictationMode, SettingsState};
use crate::stt::SttEngine;

pub struct DictationState {
    recorder: Recorder,
    recording: Mutex<bool>,
    /// Set while the stop→transcribe pipeline runs so a rapid re-trigger
    /// can't start a new recording mid-transcription.
    busy: AtomicBool,
    /// Tracks physical hotkey state to swallow OS key-repeat Pressed events.
    hotkey_down: AtomicBool,
    /// Lazily-loaded engine, kept alive between dictations.
    engine: Mutex<Option<(ModelId, Box<dyn SttEngine>)>>,
}

impl DictationState {
    pub fn new() -> Self {
        Self {
            recorder: Recorder::spawn(),
            recording: Mutex::new(false),
            busy: AtomicBool::new(false),
            hotkey_down: AtomicBool::new(false),
            engine: Mutex::new(None),
        }
    }
}

/// Entry point for global hotkey events. Interprets press/release according
/// to the configured dictation mode.
pub fn on_hotkey<R: Runtime>(app: &AppHandle<R>, pressed: bool) {
    let state = app.state::<DictationState>();
    if pressed {
        // Key repeat while held: the OS may deliver repeated Pressed events.
        if state.hotkey_down.swap(true, Ordering::SeqCst) {
            return;
        }
    } else {
        state.hotkey_down.store(false, Ordering::SeqCst);
    }

    let mode = app.state::<SettingsState>().get().mode;
    match mode {
        DictationMode::Toggle => {
            if pressed {
                toggle(app);
            }
        }
        DictationMode::PushToTalk => {
            if pressed {
                start(app);
            } else {
                request_stop(app);
            }
        }
    }
}

/// Toggle dictation: start recording if idle, otherwise stop and transcribe.
/// Used by toggle mode and the tray menu (the Wayland-safe fallback).
pub fn toggle<R: Runtime>(app: &AppHandle<R>) {
    let is_recording = *app
        .state::<DictationState>()
        .recording
        .lock()
        .expect("recording flag");
    if is_recording {
        request_stop(app);
    } else {
        start(app);
    }
}

fn start<R: Runtime>(app: &AppHandle<R>) {
    let state = app.state::<DictationState>();
    if *state.recording.lock().expect("recording flag") {
        return;
    }
    if state.busy.load(Ordering::SeqCst) {
        log::warn!("ignoring start: still processing previous dictation");
        return;
    }
    // The permission gate may block on the OS consent prompt, so the rest
    // of the start path runs off the main thread.
    let app = app.clone();
    std::thread::spawn(move || start_after_permission_gate(&app));
}

/// Resolve microphone permission, then start capture. The microphone is
/// never opened (not even for a format query) unless access is authorized —
/// opening it while permission is undetermined makes macOS queue one system
/// prompt per HAL access.
fn start_after_permission_gate<R: Runtime>(app: &AppHandle<R>) {
    const PROMPT_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(120);
    match crate::mic::ensure_authorized(PROMPT_TIMEOUT) {
        crate::mic::MicPermission::Authorized => {}
        crate::mic::MicPermission::Denied => {
            log::error!("microphone access denied; guiding the user to System Settings");
            crate::on_main_thread(app, |app| {
                let _ = app.emit("mic-denied", ());
            });
            crate::show_settings(app);
            return;
        }
        crate::mic::MicPermission::NotDetermined => {
            log::warn!("microphone permission prompt not answered; not recording");
            return;
        }
    }

    let state = app.state::<DictationState>();
    let mut recording = state.recording.lock().expect("recording flag");
    if *recording {
        return;
    }
    match state.recorder.start() {
        Ok(()) => {
            *recording = true;
            drop(recording);
            overlay::apply(app, UiState::Recording);
            log::info!("recording started");
        }
        Err(e) => log::error!("failed to start recording: {e:#}"),
    }
}

fn request_stop<R: Runtime>(app: &AppHandle<R>) {
    let state = app.state::<DictationState>();
    let mut recording = state.recording.lock().expect("recording flag");
    if !*recording {
        return;
    }
    *recording = false;
    drop(recording);
    let app = app.clone();
    // Transcription can take seconds; never block the main thread.
    std::thread::spawn(move || stop_and_process(&app));
}

fn stop_and_process<R: Runtime>(app: &AppHandle<R>) {
    let state = app.state::<DictationState>();
    state.busy.store(true, Ordering::SeqCst);
    overlay::apply(app, UiState::Transcribing);
    let finish = |app: &AppHandle<R>| {
        let state = app.state::<DictationState>();
        state.busy.store(false, Ordering::SeqCst);
        overlay::apply(app, UiState::Idle);
    };

    let samples = match state.recorder.stop() {
        Ok(s) => s,
        Err(e) => {
            log::error!("failed to stop recording: {e:#}");
            finish(app);
            return;
        }
    };
    log::info!(
        "recording stopped: {:.2}s at {} Hz",
        samples.len() as f32 / audio::TARGET_SAMPLE_RATE as f32,
        audio::TARGET_SAMPLE_RATE
    );

    // Keep the last capture around as a WAV: invaluable when debugging
    // "why did it transcribe that?" reports.
    match save_debug_wav(app, &samples) {
        Ok(path) => log::info!("saved recording to {}", path.display()),
        Err(e) => log::warn!("failed to save WAV: {e:#}"),
    }

    match transcribe(app, &samples) {
        Ok(raw) => {
            let transcript = crate::postprocess::post_process(&raw);
            log::info!("transcript: {transcript:?} (raw: {raw:?})");
            if transcript.is_empty() {
                log::info!("nothing to paste (empty transcript)");
            } else {
                let event_payload = transcript.clone();
                crate::on_main_thread(app, move |app| {
                    let _ = app.emit("transcript", &event_payload);
                });
                let auto_paste = app.state::<SettingsState>().get().auto_paste;
                if let Err(e) = crate::paste::deliver(app, &transcript, auto_paste) {
                    log::error!("delivering transcript failed: {e:#}");
                }
            }
        }
        Err(e) => log::error!("transcription failed: {e:#}"),
    }
    finish(app);
}

fn transcribe<R: Runtime>(app: &AppHandle<R>, samples: &[f32]) -> anyhow::Result<String> {
    let state = app.state::<DictationState>();
    let model = app.state::<SettingsState>().get().model;
    let data_dir = app.path().app_data_dir()?;

    if !models::is_installed(&data_dir, model) {
        crate::on_main_thread(app, move |app| {
            let _ = app.emit("model-required", model);
        });
        crate::show_settings(app);
        anyhow::bail!(
            "model {model:?} is not installed — open Settings to download it ({} MB)",
            model.spec().total_bytes() / 1_000_000
        );
    }

    let mut guard = state.engine.lock().expect("engine lock");
    match &*guard {
        Some((loaded, _)) if *loaded == model => {}
        _ => {
            log::info!("loading model {model:?}…");
            let engine = crate::stt::load_engine(model, &models::model_dir(&data_dir, model))?;
            *guard = Some((model, engine));
        }
    }
    let (_, engine) = guard.as_mut().expect("engine just loaded");

    let started = std::time::Instant::now();
    let text = engine.transcribe(samples, audio::TARGET_SAMPLE_RATE)?;
    log::info!(
        "transcribed {:.2}s of audio in {:.2}s",
        samples.len() as f32 / audio::TARGET_SAMPLE_RATE as f32,
        started.elapsed().as_secs_f32()
    );
    Ok(text)
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
