//! Microphone capture on a dedicated audio thread.
//!
//! `cpal::Stream` is not `Send`, so the stream is created, owned, and dropped
//! entirely on one thread; the rest of the app talks to it through channels.

use std::sync::{
    mpsc::{self, Receiver, Sender, SyncSender},
    Arc, Mutex,
};

use anyhow::{anyhow, Context};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, Sample, SizedSample};

use super::dsp;

/// Sample rate every recording is normalized to before STT.
pub const TARGET_SAMPLE_RATE: u32 = 16_000;

enum Cmd {
    Start(SyncSender<anyhow::Result<()>>),
    /// Stop recording and return mono samples resampled to
    /// [`TARGET_SAMPLE_RATE`].
    Stop(SyncSender<anyhow::Result<Vec<f32>>>),
}

/// Handle to the audio-capture thread.
#[derive(Clone)]
pub struct Recorder {
    tx: Sender<Cmd>,
}

impl Recorder {
    pub fn spawn() -> Self {
        let (tx, rx) = mpsc::channel();
        std::thread::Builder::new()
            .name("murmur-audio".into())
            .spawn(move || audio_thread(rx))
            .expect("failed to spawn audio thread");
        Self { tx }
    }

    pub fn start(&self) -> anyhow::Result<()> {
        let (reply_tx, reply_rx) = mpsc::sync_channel(1);
        self.tx
            .send(Cmd::Start(reply_tx))
            .map_err(|_| anyhow!("audio thread is gone"))?;
        reply_rx.recv().context("audio thread dropped reply")?
    }

    pub fn stop(&self) -> anyhow::Result<Vec<f32>> {
        let (reply_tx, reply_rx) = mpsc::sync_channel(1);
        self.tx
            .send(Cmd::Stop(reply_tx))
            .map_err(|_| anyhow!("audio thread is gone"))?;
        reply_rx.recv().context("audio thread dropped reply")?
    }
}

struct ActiveRecording {
    // Held only to keep the input stream alive; dropped to stop capture.
    _stream: cpal::Stream,
    /// Mono samples at the device's native rate, appended by the callback.
    buffer: Arc<Mutex<Vec<f32>>>,
    source_rate: u32,
}

fn audio_thread(rx: Receiver<Cmd>) {
    let mut active: Option<ActiveRecording> = None;
    while let Ok(cmd) = rx.recv() {
        match cmd {
            Cmd::Start(reply) => {
                let result = if active.is_some() {
                    Err(anyhow!("already recording"))
                } else {
                    start_capture().map(|rec| {
                        active = Some(rec);
                    })
                };
                let _ = reply.send(result);
            }
            Cmd::Stop(reply) => {
                let result = match active.take() {
                    None => Err(anyhow!("not recording")),
                    Some(rec) => {
                        // Dropping the stream stops capture.
                        let source_rate = rec.source_rate;
                        let buffer = rec.buffer;
                        drop(rec._stream);
                        let native = std::mem::take(&mut *buffer.lock().expect("audio buffer"));
                        log::info!(
                            "captured {} samples at {} Hz ({:.2}s)",
                            native.len(),
                            source_rate,
                            native.len() as f32 / source_rate as f32
                        );
                        Ok(dsp::resample(&native, source_rate, TARGET_SAMPLE_RATE))
                    }
                };
                let _ = reply.send(result);
            }
        }
    }
}

fn start_capture() -> anyhow::Result<ActiveRecording> {
    let host = cpal::default_host();
    let device = host
        .default_input_device()
        .ok_or_else(|| anyhow!("no default input device — is a microphone connected?"))?;
    let config = device
        .default_input_config()
        .context("querying default input config")?;
    log::info!(
        "recording from '{}' ({} ch @ {} Hz, {:?})",
        device
            .description()
            .map(|d| d.name().to_string())
            .unwrap_or_else(|_| "unknown".into()),
        config.channels(),
        config.sample_rate(),
        config.sample_format()
    );

    let source_rate = config.sample_rate();
    let channels = config.channels() as usize;
    let buffer = Arc::new(Mutex::new(Vec::<f32>::new()));

    let stream = match config.sample_format() {
        cpal::SampleFormat::F32 => build_stream::<f32>(&device, config.into(), &buffer, channels),
        cpal::SampleFormat::I16 => build_stream::<i16>(&device, config.into(), &buffer, channels),
        cpal::SampleFormat::U16 => build_stream::<u16>(&device, config.into(), &buffer, channels),
        cpal::SampleFormat::I8 => build_stream::<i8>(&device, config.into(), &buffer, channels),
        cpal::SampleFormat::U8 => build_stream::<u8>(&device, config.into(), &buffer, channels),
        cpal::SampleFormat::I32 => build_stream::<i32>(&device, config.into(), &buffer, channels),
        other => Err(anyhow!("unsupported sample format {other:?}")),
    }?;
    stream.play().context("starting input stream")?;

    Ok(ActiveRecording {
        _stream: stream,
        buffer,
        source_rate,
    })
}

fn build_stream<T>(
    device: &cpal::Device,
    config: cpal::StreamConfig,
    buffer: &Arc<Mutex<Vec<f32>>>,
    channels: usize,
) -> anyhow::Result<cpal::Stream>
where
    T: SizedSample,
    f32: FromSample<T>,
{
    let buffer = Arc::clone(buffer);
    let stream = device.build_input_stream(
        config,
        move |data: &[T], _| {
            let mono: Vec<f32> = if channels <= 1 {
                data.iter().map(|&s| f32::from_sample(s)).collect()
            } else {
                let as_f32: Vec<f32> = data.iter().map(|&s| f32::from_sample(s)).collect();
                dsp::downmix_to_mono(&as_f32, channels)
            };
            buffer
                .lock()
                .expect("audio buffer")
                .extend_from_slice(&mono);
        },
        |err| log::error!("audio stream error: {err}"),
        None,
    )?;
    Ok(stream)
}
