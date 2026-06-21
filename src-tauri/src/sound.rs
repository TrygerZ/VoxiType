//! Optional sound cues for recording start/stop.
//!
//! Plays short generated tones via a dedicated cpal output stream. `cpal::Stream`
//! is `!Send`, so playback runs on its own short-lived OS thread; the public API
//! is fire-and-forget and never blocks the caller.

use std::sync::mpsc;
use std::time::Duration;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::SampleFormat;

/// Which cue to play.
#[derive(Debug, Clone, Copy)]
pub enum Cue {
    /// Rising tone — recording started.
    Start,
    /// Falling tone — recording stopped / processing.
    Stop,
}

impl Cue {
    /// Tone frequency in Hz.
    fn frequency(self) -> f32 {
        match self {
            Cue::Start => 880.0,
            Cue::Stop => 523.25,
        }
    }
}

/// Play a cue without blocking. Errors are logged, never propagated, because a
/// missing output device must not break the recording pipeline.
pub fn play(cue: Cue) {
    std::thread::spawn(move || {
        if let Err(e) = play_blocking(cue) {
            tracing::debug!("Sound cue skipped: {e}");
        }
    });
}

fn play_blocking(cue: Cue) -> Result<(), String> {
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .ok_or_else(|| "no default output device".to_string())?;
    let config = device.default_output_config().map_err(|e| e.to_string())?;

    let sample_rate = config.sample_rate().0 as f32;
    let channels = config.channels() as usize;
    let freq = cue.frequency();
    let total_samples = (sample_rate * 0.14) as usize;

    let (done_tx, done_rx) = mpsc::channel::<()>();
    let mut clock = 0usize;

    // Amplitude envelope avoids clicks at start/end.
    let next_sample = move || -> Option<f32> {
        if clock >= total_samples {
            return None;
        }
        let t = clock as f32 / sample_rate;
        let progress = clock as f32 / total_samples as f32;
        let envelope = (progress * std::f32::consts::PI).sin();
        let value = (2.0 * std::f32::consts::PI * freq * t).sin() * 0.2 * envelope;
        clock += 1;
        Some(value)
    };

    let err_fn = |e| tracing::debug!("sound stream error: {e}");

    macro_rules! build {
        ($t:ty, $convert:expr) => {{
            let mut next = next_sample;
            let done_tx = done_tx.clone();
            device.build_output_stream(
                &config.clone().into(),
                move |data: &mut [$t], _| {
                    for frame in data.chunks_mut(channels) {
                        match next() {
                            Some(v) => {
                                let s = $convert(v);
                                for sample in frame.iter_mut() {
                                    *sample = s;
                                }
                            }
                            None => {
                                let _ = done_tx.send(());
                                for sample in frame.iter_mut() {
                                    *sample = $convert(0.0);
                                }
                            }
                        }
                    }
                },
                err_fn,
                None,
            )
        }};
    }

    let stream = match config.sample_format() {
        SampleFormat::F32 => build!(f32, |v: f32| v),
        SampleFormat::I16 => build!(i16, |v: f32| (v * i16::MAX as f32) as i16),
        SampleFormat::U16 => build!(u16, |v: f32| ((v * 0.5 + 0.5) * u16::MAX as f32) as u16),
        other => return Err(format!("unsupported sample format: {other:?}")),
    }
    .map_err(|e| e.to_string())?;

    stream.play().map_err(|e| e.to_string())?;

    // Wait until the tone finished (with a safety timeout), then drop the stream.
    let _ = done_rx.recv_timeout(Duration::from_millis(400));
    std::thread::sleep(Duration::from_millis(20));
    drop(stream);
    Ok(())
}
