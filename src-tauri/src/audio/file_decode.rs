//! Audio file decoding for file transcription.
//!
//! Decodes any supported container/codec (mp3, wav, flac, ogg/vorbis,
//! m4a/aac/alac) with `symphonia`, then reuses [`Resampler`] to produce the
//! pipeline's canonical 16 kHz mono stream. [`ChunkSplitter`] cuts that stream
//! into STT-sized chunks at the quietest point so words are not split.

use std::fs::File;
use std::path::Path;

use symphonia::core::codecs::audio::{AudioDecoder, AudioDecoderOptions};
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::formats::probe::Hint;
use symphonia::core::formats::{FormatOptions, FormatReader, TrackType};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;

use super::resampler::Resampler;
use super::TARGET_SAMPLE_RATE;
use crate::error::{AppError, Result};

/// Window used to measure loudness when searching for a cut point (100 ms).
const SILENCE_WINDOW_SAMPLES: usize = TARGET_SAMPLE_RATE as usize / 10;

/// Amplitude threshold below which a window counts as silent. Mirrors the
/// capture module's `VOICE_THRESHOLD` concept; the mean-square comparison
/// squares this value, so the gate is stated in amplitude for readability.
const SILENT_WINDOW_RMS: f32 = 0.01;

fn decode_error(e: SymphoniaError) -> AppError {
    AppError::invalid_input(format!("Unsupported or corrupt audio file: {e}"))
}

/// An opened audio file positioned at its first audio track.
pub struct AudioFile {
    format: Box<dyn FormatReader>,
    decoder: Box<dyn AudioDecoder>,
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
        let format = symphonia::default::get_probe()
            .probe(
                &hint,
                stream,
                FormatOptions::default(),
                MetadataOptions::default(),
            )
            .map_err(decode_error)?;
        let track = format
            .default_track(TrackType::Audio)
            .ok_or_else(|| AppError::invalid_input("File contains no audio track"))?;
        let params = track
            .codec_params
            .as_ref()
            .and_then(|p| p.audio())
            .ok_or_else(|| AppError::invalid_input("File contains no audio track"))?;
        let duration_secs = match (track.num_frames, params.sample_rate) {
            (Some(frames), Some(rate)) if rate > 0 => Some(frames as f64 / rate as f64),
            _ => None,
        };
        let decoder = symphonia::default::get_codecs()
            .make_audio_decoder(params, &AudioDecoderOptions::default())
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
        mut on_samples: impl FnMut(&[f32]) -> Result<()>,
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
                on_samples(&tail)?;
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
                Ok(Some(p)) => p,
                Ok(None) => return Ok(None),
                // ponytail: chained streams (rare, e.g. Ogg radio dumps) stop at the
                // first reset; re-create the decoder here if users report truncation.
                Err(SymphoniaError::ResetRequired) => return Ok(None),
                Err(e) => return Err(decode_error(e)),
            };
            if packet.track_id != self.track_id {
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
            let spec = decoded.spec();
            let channels = spec.channels().count().max(1) as u16;
            let rate = spec.rate();
            let mut samples = Vec::new();
            decoded.copy_to_vec_interleaved(&mut samples);
            return Ok(Some((samples, rate, channels)));
        }
    }
}

pub fn too_long_error(max_secs: u64) -> AppError {
    AppError::invalid_input(format!(
        "Audio file is longer than the {}-minute limit",
        max_secs / 60
    ))
}

/// Splits a sample stream into chunks, cutting at the quietest window so words
/// are not split. A chunk is cut as soon as a window below [`SILENT_WINDOW_RMS`]
/// is found near the boundary; when the audio is uniformly loud, the buffer is
/// allowed to grow up to `extend_len` past `chunk_len` before a force-cut at the
/// quietest available window. No audio overlaps between chunks.
pub struct ChunkSplitter {
    buffer: Vec<f32>,
    chunk_len: usize,
    search_len: usize,
    extend_len: usize,
}

impl ChunkSplitter {
    pub fn new(chunk_len: usize, search_len: usize, extend_len: usize) -> Self {
        Self {
            buffer: Vec::new(),
            chunk_len,
            search_len: search_len.min(chunk_len / 2),
            extend_len,
        }
    }

    /// Append samples and return every chunk that is now complete.
    pub fn push(&mut self, samples: &[f32]) -> Vec<Vec<f32>> {
        self.buffer.extend_from_slice(samples);
        let mut chunks = Vec::new();
        while self.buffer.len() >= self.chunk_len {
            let search_start = self.chunk_len - self.search_len;
            let (quiet_cut, energy) = quietest_cut(&self.buffer, search_start, self.chunk_len);
            let cut = if energy < SILENT_WINDOW_RMS * SILENT_WINDOW_RMS {
                // A silent window exists near the boundary; cut there immediately.
                quiet_cut
            } else if self.buffer.len() >= self.chunk_len + self.extend_len {
                // Uniformly loud: the extended tail may hold a quieter spot.
                let limit = (self.chunk_len + self.extend_len).min(self.buffer.len());
                quietest_cut(&self.buffer, search_start, limit).0
            } else {
                // Not yet long enough to force a cut; wait for more samples.
                break;
            };
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

/// Quietest window's midpoint within `samples[start..end]`, paired with that
/// window's mean square energy.
///
/// Ties resolve to the later window so a force-cut on uniformly loud audio
/// lands as close to the extension limit as possible (longer chunks, fewer
/// boundaries), rather than snapping back to the start of the search region.
fn quietest_cut(samples: &[f32], start: usize, end: usize) -> (usize, f32) {
    let mut best: Option<(usize, f32)> = None;
    for (i, window) in samples[start..end]
        .chunks(SILENCE_WINDOW_SAMPLES)
        .enumerate()
    {
        let energy = mean_square(window);
        if best.is_none_or(|(_, best_energy)| energy <= best_energy) {
            // Cut at the window midpoint, staying inside the window even when
            // the trailing window is shorter than a full window.
            best = Some((
                start + i * SILENCE_WINDOW_SAMPLES + window.len() / 2,
                energy,
            ));
        }
    }
    best.unwrap_or((end, f32::INFINITY))
}

/// Mean square energy of a window (mean, not sum, so a shorter trailing window
/// compares fairly against full windows).
fn mean_square(window: &[f32]) -> f32 {
    if window.is_empty() {
        return 0.0;
    }
    window.iter().map(|s| s * s).sum::<f32>() / window.len() as f32
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stt::groq_stt::encode_wav_16k_mono;

    fn write_temp_wav(samples: &[f32]) -> std::path::PathBuf {
        let path =
            std::env::temp_dir().join(format!("voxitype-decode-{}.wav", uuid::Uuid::new_v4()));
        std::fs::write(&path, encode_wav_16k_mono(samples)).unwrap();
        path
    }

    fn decode_all(path: &Path, max_secs: u64) -> Result<Vec<f32>> {
        let mut out = Vec::new();
        AudioFile::open(path)?.decode_16k_mono(max_secs, |s| {
            out.extend_from_slice(s);
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
        let path =
            std::env::temp_dir().join(format!("voxitype-decode-{}.mp3", uuid::Uuid::new_v4()));
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
        let mut splitter = ChunkSplitter::new(window * 20, window * 10, window * 5);
        let chunks = splitter.push(&stream);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].len(), window * 14 + window / 2);
        let rest = splitter.finish().unwrap();
        assert_eq!(chunks[0].len() + rest.len(), stream.len());
    }

    #[test]
    fn splitter_extends_uniform_energy_then_force_cuts_within_extend_len() {
        let window = SILENCE_WINDOW_SAMPLES;
        let chunk_len = window * 6;
        let extend_len = window * 5;
        // Uniform loud audio: no silent window, so the chunk must grow past
        // chunk_len up to chunk_len + extend_len before a force-cut.
        let mut splitter = ChunkSplitter::new(chunk_len, window * 3, extend_len);
        let chunks = splitter.push(&vec![0.3; window * 35]);

        assert!(!chunks.is_empty());
        assert!(
            chunks
                .iter()
                .all(|c| !c.is_empty() && c.len() <= chunk_len + extend_len),
            "chunks must stay within chunk_len + extend_len"
        );
        assert!(
            chunks[0].len() > chunk_len,
            "uniform energy must extend past chunk_len, got {}",
            chunks[0].len()
        );
        assert!(
            (chunk_len + extend_len) - chunks[0].len() < window,
            "force-cut must land within one window of the extension limit"
        );

        let total: usize =
            chunks.iter().map(Vec::len).sum::<usize>() + splitter.finish().map_or(0, |c| c.len());
        assert_eq!(total, window * 35, "every sample must be preserved");
    }

    #[test]
    fn splitter_cuts_below_threshold_before_extension() {
        let window = SILENCE_WINDOW_SAMPLES;
        let chunk_len = window * 6;
        // Quiet (below-threshold) window inside the search region, past the
        // halfway point of the chunk: energy 0.005^2 = 2.5e-5 < threshold^2.
        let mut stream = vec![0.5f32; window * 10];
        stream[window * 4..window * 5].fill(0.005);
        let mut splitter = ChunkSplitter::new(chunk_len, window * 3, window * 5);
        let chunks = splitter.push(&stream);

        assert_eq!(chunks.len(), 1);
        assert!(
            chunks[0].len() < chunk_len,
            "a below-threshold window must cut before the extension, got {}",
            chunks[0].len()
        );
        assert_eq!(chunks[0].len(), window * 4 + window / 2);
    }

    #[test]
    fn splitter_force_cut_prefers_quieter_window_in_extension() {
        let window = SILENCE_WINDOW_SAMPLES;
        let chunk_len = window * 6;
        let extend_len = window * 5;
        // Loud audio with a quieter (but still above-threshold) window in the
        // extension region, past chunk_len: energy 0.05^2 = 2.5e-3 > threshold^2.
        let mut stream = vec![0.3f32; window * 11];
        stream[window * 7..window * 8].fill(0.05);
        let mut splitter = ChunkSplitter::new(chunk_len, window * 3, extend_len);
        let chunks = splitter.push(&stream);

        assert_eq!(chunks.len(), 1);
        // The force-cut must choose the quieter window, not the earlier loud one.
        assert_eq!(chunks[0].len(), window * 7 + window / 2);
    }

    #[test]
    fn splitter_waits_for_extend_len_before_cutting() {
        let window = SILENCE_WINDOW_SAMPLES;
        let chunk_len = window * 6;
        let extend_len = window * 5;
        // Uniform loud stream shorter than chunk_len + extend_len: no chunk yet.
        let mut splitter = ChunkSplitter::new(chunk_len, window * 3, extend_len);
        let chunks = splitter.push(&vec![0.3; window * 10]);

        assert!(
            chunks.is_empty(),
            "must not cut before reaching the extension limit"
        );
        assert_eq!(splitter.finish().map_or(0, |c| c.len()), window * 10);
    }
}
