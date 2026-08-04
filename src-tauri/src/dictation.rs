//! The dictation state machine: record, transcribe, and (in a later
//! milestone) paste the transcript.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

use tauri::{AppHandle, Emitter, Manager, Runtime};

use crate::audio::{self, Recorder};
use crate::models::{self, ModelId};
use crate::stt::SttEngine;

pub struct DictationState {
    recorder: Recorder,
    recording: Mutex<bool>,
    /// Set while the stop→transcribe pipeline runs so a rapid re-toggle
    /// can't start a new recording mid-transcription.
    busy: AtomicBool,
    /// Lazily-loaded engine, kept alive between dictations.
    engine: Mutex<Option<(ModelId, Box<dyn SttEngine>)>>,
}

impl DictationState {
    pub fn new() -> Self {
        Self {
            recorder: Recorder::spawn(),
            recording: Mutex::new(false),
            busy: AtomicBool::new(false),
            engine: Mutex::new(None),
        }
    }

    /// The model used for transcription. Hardcoded until the settings
    /// milestone; the default is Parakeet.
    pub fn active_model(&self) -> ModelId {
        ModelId::ParakeetTdt06bV2Int8
    }
}

/// Toggle dictation: start recording if idle, otherwise stop and transcribe.
/// Called from the global hotkey handler and the tray menu.
pub fn toggle<R: Runtime>(app: &AppHandle<R>) {
    let state = app.state::<DictationState>();
    let mut recording = state.recording.lock().expect("recording flag");
    if *recording {
        *recording = false;
        drop(recording);
        let app = app.clone();
        // Transcription can take seconds; never block the main thread.
        std::thread::spawn(move || stop_and_process(&app));
    } else {
        if state.busy.load(Ordering::SeqCst) {
            log::warn!("ignoring start: still processing previous dictation");
            return;
        }
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
    state.busy.store(true, Ordering::SeqCst);
    set_tray_tooltip(app, "Murmur — transcribing…");
    let finish = |app: &AppHandle<R>| {
        let state = app.state::<DictationState>();
        state.busy.store(false, Ordering::SeqCst);
        set_tray_tooltip(app, "Murmur — idle");
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
        Ok(transcript) => {
            log::info!("transcript: {transcript:?}");
            let _ = app.emit("transcript", &transcript);
            // The paste-pipeline milestone consumes the transcript here.
        }
        Err(e) => log::error!("transcription failed: {e:#}"),
    }
    finish(app);
}

fn transcribe<R: Runtime>(app: &AppHandle<R>, samples: &[f32]) -> anyhow::Result<String> {
    let state = app.state::<DictationState>();
    let model = state.active_model();
    let data_dir = app.path().app_data_dir()?;

    if !models::is_installed(&data_dir, model) {
        let _ = app.emit("model-required", model);
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

fn set_tray_tooltip<R: Runtime>(app: &AppHandle<R>, text: &str) {
    if let Some(tray) = app.tray_by_id(crate::tray::TRAY_ID) {
        let _ = tray.set_tooltip(Some(text));
    }
}
