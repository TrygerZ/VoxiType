//! Audio file decoding for file transcription.
//!
//! Decodes any supported container/codec (mp3, wav, flac, ogg/vorbis,
//! m4a/aac/alac) with `symphonia`, then reuses [`Resampler`] to produce the
//! pipeline's canonical 16 kHz mono stream. [`ChunkSplitter`] cuts that stream
//! into STT-sized chunks at the quietest point so words are not split.

use std::fs::File;
use std::path::Path;

use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::{Decoder, DecoderOptions, CODEC_TYPE_NULL};
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::formats::{FormatOptions, FormatReader};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

use super::resampler::Resampler;
use super::TARGET_SAMPLE_RATE;
use crate::error::{AppError, Result};

/// Window used to measure loudness when searching for a cut point (100 ms).
const SILENCE_WINDOW_SAMPLES: usize = TARGET_SAMPLE_RATE as usize / 10;

fn decode_error(e: SymphoniaError) -> AppError {
    AppError::invalid_input(format!("Unsupported or corrupt audio file: {e}"))
}

/// An opened audio file positioned at its first audio track.
pub struct AudioFile {
    format: Box<dyn FormatReader>,
    decoder: Box<dyn Decoder>,
    track_id: u32,
    /// Duration from container metadata; `None` when the container omits it.
    pub duration_secs: Option<f64>,
}

impl AudioFile {
    pub fn open(path: &Path) -> Result<Self> {
        let file = File::open(path)?;
        let stream = MediaSourceStream::new(Box::new(file), Default::default());
        let mut hint = Hint::new();
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            hint.with_extension(ext);
        }
        let probed = symphonia::default::get_probe()
            .format(
                &hint,
                stream,
                &FormatOptions::default(),
                &MetadataOptions::default(),
            )
            .map_err(decode_error)?;
        let format = probed.format;
        let track = format
            .tracks()
            .iter()
            .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
            .ok_or_else(|| AppError::invalid_input("File contains no audio track"))?;
        let params = &track.codec_params;
        let duration_secs = match (params.n_frames, params.sample_rate) {
            (Some(frames), Some(rate)) if rate > 0 => Some(frames as f64 / rate as f64),
            _ => None,
        };
        let decoder = symphonia::default::get_codecs()
            .make(params, &DecoderOptions::default())
            .map_err(decode_error)?;
        Ok(Self {
            track_id: track.id,
            format,
            decoder,
            duration_secs,
        })
    }

    /// Decode the whole track, handing 16 kHz mono samples to `on_samples`.
    ///
    /// Fails once output exceeds `max_secs`, which also guards files whose
    /// container metadata under-reports (or omits) the duration. An error
    /// returned by `on_samples` stops decoding immediately.
    pub fn decode_16k_mono(
        mut self,
        max_secs: u64,
        mut on_samples: impl FnMut(Vec<f32>) -> Result<()>,
    ) -> Result<()> {
        let max_samples = max_secs as usize * TARGET_SAMPLE_RATE as usize;
        let mut resampler: Option<Resampler> = None;
        let mut emitted = 0usize;
        while let Some(interleaved) = self.next_interleaved()? {
            let (samples, rate, channels) = interleaved;
            let resampler = match resampler.as_mut() {
                Some(r) => r,
                None => resampler.insert(Resampler::new(rate, TARGET_SAMPLE_RATE, channels)?),
            };
            let mono = resampler.process(&samples)?;
            emitted += mono.len();
            if emitted > max_samples {
                return Err(too_long_error(max_secs));
            }
            if !mono.is_empty() {
                on_samples(mono)?;
            }
        }
        if let Some(mut r) = resampler {
            let tail = r.flush()?;
            if !tail.is_empty() {
                on_samples(tail)?;
            }
        }
        Ok(())
    }

    /// Next decoded packet as `(interleaved samples, sample rate, channels)`,
    /// or `None` at end of stream. Corrupt packets are skipped, matching how
    /// media players tolerate damaged frames.
    fn next_interleaved(&mut self) -> Result<Option<(Vec<f32>, u32, u16)>> {
        loop {
            let packet = match self.format.next_packet() {
                Ok(p) => p,
                Err(SymphoniaError::IoError(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                    return Ok(None)
                }
                // ponytail: chained streams (rare, e.g. Ogg radio dumps) stop at the
                // first reset; re-create the decoder here if users report truncation.
                Err(SymphoniaError::ResetRequired) => return Ok(None),
                Err(e) => return Err(decode_error(e)),
            };
            if packet.track_id() != self.track_id {
                continue;
            }
            let decoded = match self.decoder.decode(&packet) {
                Ok(d) => d,
                Err(SymphoniaError::DecodeError(e)) => {
                    tracing::warn!("Skipping corrupt audio packet: {e}");
                    continue;
                }
                Err(e) => return Err(decode_error(e)),
            };
            let spec = *decoded.spec();
            let mut buffer = SampleBuffer::<f32>::new(decoded.capacity() as u64, spec);
            buffer.copy_interleaved_ref(decoded);
            let channels = spec.channels.count().max(1) as u16;
            return Ok(Some((buffer.samples().to_vec(), spec.rate, channels)));
        }
    }
}

pub fn too_long_error(max_secs: u64) -> AppError {
    AppError::invalid_input(format!(
        "Audio file is longer than the {}-minute limit",
        max_secs / 60
    ))
}

/// Splits a sample stream into chunks of at most `chunk_len` samples, cutting
/// at the quietest window within the last `search_len` samples of each chunk.
pub struct ChunkSplitter {
    buffer: Vec<f32>,
    chunk_len: usize,
    search_len: usize,
}

impl ChunkSplitter {
    pub fn new(chunk_len: usize, search_len: usize) -> Self {
        Self {
            buffer: Vec::new(),
            chunk_len,
            search_len: search_len.min(chunk_len / 2),
        }
    }

    /// Append samples and return every chunk that is now complete.
    pub fn push(&mut self, samples: &[f32]) -> Vec<Vec<f32>> {
        self.buffer.extend_from_slice(samples);
        let mut chunks = Vec::new();
        while self.buffer.len() >= self.chunk_len {
            let cut = quietest_cut(&self.buffer[..self.chunk_len], self.search_len);
            let rest = self.buffer.split_off(cut);
            chunks.push(std::mem::replace(&mut self.buffer, rest));
        }
        chunks
    }

    /// Return the trailing partial chunk, if any.
    pub fn finish(self) -> Option<Vec<f32>> {
        (!self.buffer.is_empty()).then_some(self.buffer)
    }
}

fn quietest_cut(chunk: &[f32], search_len: usize) -> usize {
    let start = chunk.len() - search_len;
    chunk[start..]
        .chunks(SILENCE_WINDOW_SAMPLES)
        .enumerate()
        .map(|(i, w)| (i, w.len(), w.iter().map(|s| s * s).sum::<f32>()))
        .min_by(|a, b| a.2.total_cmp(&b.2))
        .map(|(i, len, _)| start + i * SILENCE_WINDOW_SAMPLES + len / 2)
        .unwrap_or(chunk.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stt::groq_stt::encode_wav_16k_mono;

    fn write_temp_wav(samples: &[f32]) -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!("voxitype-decode-{}.wav", uuid::Uuid::new_v4()));
        std::fs::write(&path, encode_wav_16k_mono(samples)).unwrap();
        path
    }

    fn decode_all(path: &Path, max_secs: u64) -> Result<Vec<f32>> {
        let mut out = Vec::new();
        AudioFile::open(path)?.decode_16k_mono(max_secs, |s| {
            out.extend(s);
            Ok(())
        })?;
        Ok(out)
    }

    #[test]
    fn decodes_wav_to_16k_mono_with_duration() {
        let input: Vec<f32> = (0..32_000).map(|i| (i as f32 * 0.01).sin() * 0.5).collect();
        let path = write_temp_wav(&input);
        let file = AudioFile::open(&path).unwrap();
        assert_eq!(file.duration_secs, Some(2.0));
        let out = decode_all(&path, 60).unwrap();
        std::fs::remove_file(&path).unwrap();
        assert_eq!(out.len(), input.len());
        assert!((out[100] - input[100]).abs() < 1e-3);
    }

    #[test]
    fn rejects_audio_over_limit() {
        let path = write_temp_wav(&vec![0.0; 32_000]);
        let err = decode_all(&path, 1).unwrap_err();
        std::fs::remove_file(&path).unwrap();
        assert_eq!(err.code, crate::error::ErrorCode::InvalidInput);
    }

    #[test]
    fn rejects_non_audio_file() {
        let path = std::env::temp_dir().join(format!("voxitype-decode-{}.mp3", uuid::Uuid::new_v4()));
        std::fs::write(&path, b"definitely not audio").unwrap();
        let result = AudioFile::open(&path);
        std::fs::remove_file(&path).unwrap();
        assert!(result.is_err());
    }

    #[test]
    fn splitter_cuts_at_quietest_window_and_keeps_every_sample() {
        let window = SILENCE_WINDOW_SAMPLES;
        let mut stream = vec![0.5f32; window * 25];
        // Silent window inside the search region (last 10 windows of the first 20).
        stream[window * 14..window * 15].fill(0.0);
        let mut splitter = ChunkSplitter::new(window * 20, window * 10);
        let chunks = splitter.push(&stream);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].len(), window * 14 + window / 2);
        let rest = splitter.finish().unwrap();
        assert_eq!(chunks[0].len() + rest.len(), stream.len());
    }

    #[test]
    fn splitter_never_exceeds_chunk_len() {
        let mut splitter = ChunkSplitter::new(10_000, 4_000);
        let chunks = splitter.push(&vec![0.3; 55_000]);
        assert!(chunks.iter().all(|c| c.len() <= 10_000 && !c.is_empty()));
        let total: usize = chunks.iter().map(Vec::len).sum::<usize>()
            + splitter.finish().map_or(0, |c| c.len());
        assert_eq!(total, 55_000);
    }
}
