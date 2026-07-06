//! Optional sound cues for recording start/stop.
//!
//! Plays short premium WAV files via a dedicated cpal output stream.

use std::sync::mpsc;
use std::time::Duration;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::SampleFormat;

/// Which cue to play.
#[derive(Debug, Clone, Copy)]
pub enum Cue {
    /// Woodblock tock — recording started.
    Start,
    /// Woodblock tock — recording stopped / processing.
    Stop,
}

struct WavData {
    sample_rate: u32,
    samples: Vec<i16>,
}

fn parse_wav(bytes: &[u8]) -> Result<WavData, String> {
    if bytes.len() < 44 {
        return Err("WAV too short".to_string());
    }
    if &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return Err("Not a RIFF WAVE file".to_string());
    }

    let mut pos = 12;
    let mut channels = None;
    let mut sample_rate = None;
    let mut data_slice = None;

    while pos + 8 <= bytes.len() {
        let chunk_id = &bytes[pos..pos + 4];
        let chunk_size = u32::from_le_bytes(bytes[pos + 4..pos + 8].try_into().unwrap()) as usize;
        pos += 8;

        if pos + chunk_size > bytes.len() {
            return Err("Chunk size out of bounds".to_string());
        }

        match chunk_id {
            b"fmt " => {
                if chunk_size >= 16 {
                    let format = u16::from_le_bytes(bytes[pos..pos + 2].try_into().unwrap());
                    if format != 1 {
                        return Err("Only uncompressed PCM WAV supported".to_string());
                    }
                    channels = Some(u16::from_le_bytes(
                        bytes[pos + 2..pos + 4].try_into().unwrap(),
                    ));
                    sample_rate = Some(u32::from_le_bytes(
                        bytes[pos + 4..pos + 8].try_into().unwrap(),
                    ));
                    let bits_per_sample =
                        u16::from_le_bytes(bytes[pos + 14..pos + 16].try_into().unwrap());
                    if bits_per_sample != 16 {
                        return Err("Only 16-bit WAV supported".to_string());
                    }
                }
            }
            b"data" => {
                data_slice = Some(&bytes[pos..pos + chunk_size]);
            }
            _ => {}
        }
        pos += chunk_size;
        if chunk_size % 2 == 1 && pos < bytes.len() {
            pos += 1;
        }
    }

    let channels = channels.ok_or("Missing fmt chunk")?;
    if channels != 1 {
        return Err("Only mono WAV supported".to_string());
    }
    let sample_rate = sample_rate.ok_or("Missing fmt chunk")?;
    let raw_data = data_slice.ok_or("Missing data chunk")?;

    if raw_data.len() % 2 != 0 {
        return Err("Data length is not even".to_string());
    }

    let num_samples = raw_data.len() / 2;
    let mut samples = Vec::with_capacity(num_samples);
    for chunk in raw_data.chunks_exact(2) {
        samples.push(i16::from_le_bytes([chunk[0], chunk[1]]));
    }

    Ok(WavData {
        sample_rate,
        samples,
    })
}

/// Play a cue without blocking. Errors are logged, never propagated.
pub fn play(cue: Cue) {
    std::thread::spawn(move || {
        if let Err(e) = play_blocking(cue) {
            tracing::debug!("Sound cue skipped: {e}");
        }
    });
}

fn play_blocking(cue: Cue) -> Result<(), String> {
    let wav_bytes = match cue {
        Cue::Start => include_bytes!("../assets/sound/start.wav") as &[u8],
        Cue::Stop => include_bytes!("../assets/sound/stop.wav") as &[u8],
    };

    let wav = parse_wav(wav_bytes)?;

    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .ok_or_else(|| "no default output device".to_string())?;
    let config = device.default_output_config().map_err(|e| e.to_string())?;

    let stream_sample_rate = config.sample_rate().0 as f64;
    let wav_sample_rate = wav.sample_rate as f64;
    let step = wav_sample_rate / stream_sample_rate;

    let play_duration_ms = ((wav.samples.len() as f64 / wav_sample_rate) * 1000.0) as u64;

    let channels = config.channels() as usize;
    let (done_tx, done_rx) = mpsc::channel::<()>();
    let mut clock = 0usize;

    // Linear interpolation resampler returning float sample values (-1.0 to 1.0)
    let next_sample = move || -> Option<f32> {
        let pos = clock as f64 * step;
        let idx = pos.floor() as usize;
        let fract = (pos - idx as f64) as f32;

        if idx >= wav.samples.len() {
            return None;
        }

        let s0 = wav.samples[idx] as f32 / 32768.0;
        let s1 = if idx + 1 < wav.samples.len() {
            wav.samples[idx + 1] as f32 / 32768.0
        } else {
            0.0
        };

        let value = (1.0 - fract) * s0 + fract * s1;
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

    // Wait until the sound has finished playing, then drop the stream
    let _ = done_rx.recv_timeout(Duration::from_millis(play_duration_ms + 100));
    std::thread::sleep(Duration::from_millis(150));
    drop(stream);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_wav_valid() {
        let start_bytes = include_bytes!("../assets/sound/start.wav");
        let parsed = parse_wav(start_bytes).unwrap();
        assert_eq!(parsed.sample_rate, 48000);
        assert!(!parsed.samples.is_empty());
    }

    #[test]
    fn test_parse_wav_invalid() {
        let invalid_bytes = b"RIFFxxxxWAVEfmt \x10\x00\x00\x00\x01\x00\x01\x00\x80\xbb\x00\x00\x00\x77\x01\x00\x02\x00\x10\x00data\x01\x00\x00";
        assert!(parse_wav(invalid_bytes).is_err());
    }

    #[test]
    fn test_parse_wav_skips_odd_chunk_padding() {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"RIFF");
        bytes.extend_from_slice(&0u32.to_le_bytes());
        bytes.extend_from_slice(b"WAVE");
        bytes.extend_from_slice(b"JUNK");
        bytes.extend_from_slice(&1u32.to_le_bytes());
        bytes.extend_from_slice(&[0, 0]);
        bytes.extend_from_slice(b"fmt ");
        bytes.extend_from_slice(&16u32.to_le_bytes());
        bytes.extend_from_slice(&1u16.to_le_bytes());
        bytes.extend_from_slice(&1u16.to_le_bytes());
        bytes.extend_from_slice(&48_000u32.to_le_bytes());
        bytes.extend_from_slice(&96_000u32.to_le_bytes());
        bytes.extend_from_slice(&2u16.to_le_bytes());
        bytes.extend_from_slice(&16u16.to_le_bytes());
        bytes.extend_from_slice(b"data");
        bytes.extend_from_slice(&2u32.to_le_bytes());
        bytes.extend_from_slice(&0i16.to_le_bytes());

        let parsed = parse_wav(&bytes).unwrap();
        assert_eq!(parsed.sample_rate, 48_000);
        assert_eq!(parsed.samples, vec![0]);
    }
}
