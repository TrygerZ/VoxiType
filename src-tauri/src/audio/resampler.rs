//! Sample-rate conversion and channel down-mixing.
//!
//! Converts arbitrary input rate / channel count into the pipeline's canonical
//! 16 kHz mono `f32` stream using `rubato`'s sinc FFT resampler.

use rubato::{
    Resampler as _, SincFixedIn, SincInterpolationParameters, SincInterpolationType, WindowFunction,
};

use crate::error::{AppError, Result};

/// Streaming resampler that mixes to mono and converts to a target rate.
///
/// All scratch buffers are owned and reused across calls so the audio callback
/// path performs no heap allocations after warmup.
pub struct Resampler {
    channels: usize,
    resampler: Option<SincFixedIn<f32>>,
    chunk_frames: usize,
    /// Mono samples awaiting a full resampler chunk.
    pending: Vec<f32>,
    /// Leftover samples from mix_to_mono (incomplete channel frame).
    leftover_buffer: Vec<f32>,
    /// Scratch: leftover + interleaved samples combined for down-mixing.
    combined: Vec<f32>,
    /// Scratch: the down-mixed mono samples.
    mono: Vec<f32>,
    /// Scratch: one resampler input chunk.
    chunk: Vec<f32>,
    /// The output of the last `process` call (borrowed out, cleared per call).
    output: Vec<f32>,
}

impl Resampler {
    /// Create a resampler for the given input characteristics. When the input
    /// rate already equals the output rate, resampling is a no-op (mix only).
    pub fn new(input_rate: u32, output_rate: u32, channels: u16) -> Result<Self> {
        let channels = channels.max(1) as usize;
        let chunk_frames = 1024usize;

        let resampler = if input_rate == output_rate {
            None
        } else {
            let params = SincInterpolationParameters {
                sinc_len: 256,
                f_cutoff: 0.95,
                interpolation: SincInterpolationType::Linear,
                oversampling_factor: 256,
                window: WindowFunction::BlackmanHarris2,
            };
            let ratio = output_rate as f64 / input_rate as f64;
            let r = SincFixedIn::<f32>::new(ratio, 2.0, params, chunk_frames, 1)
                .map_err(|e| AppError::audio(format!("Resampler init failed: {e}")))?;
            Some(r)
        };

        Ok(Self {
            channels,
            resampler,
            chunk_frames,
            pending: Vec::new(),
            leftover_buffer: Vec::new(),
            combined: Vec::new(),
            mono: Vec::new(),
            chunk: Vec::new(),
            output: Vec::new(),
        })
    }

    /// Down-mix interleaved frames into the internal mono buffer (average).
    pub fn mix_to_mono(&mut self, interleaved: &[f32]) -> &[f32] {
        // Combine any leftover samples from the previous call with the new input.
        self.combined.clear();
        self.combined.extend_from_slice(&self.leftover_buffer);
        self.combined.extend_from_slice(interleaved);
        self.leftover_buffer.clear();

        self.mono.clear();
        if self.channels <= 1 {
            self.mono.extend_from_slice(&self.combined);
            return &self.mono;
        }

        let frames = self.combined.len() / self.channels;
        let leftover = self.combined.len() % self.channels;
        if leftover != 0 {
            // Save leftover samples for next call instead of dropping.
            let split_point = frames * self.channels;
            self.leftover_buffer
                .extend_from_slice(&self.combined[split_point..]);
        }

        for f in 0..frames {
            let base = f * self.channels;
            let mut sum = 0.0f32;
            for c in 0..self.channels {
                sum += self.combined[base + c];
            }
            self.mono.push(sum / self.channels as f32);
        }
        &self.mono
    }

    /// Feed interleaved input samples, returning a borrow of the internal
    /// output buffer holding any newly available 16 kHz mono output. Output may
    /// lag input because the FFT resampler consumes fixed-size chunks.
    pub fn process(&mut self, interleaved: &[f32]) -> Result<&[f32]> {
        self.mix_to_mono(interleaved);
        let chunk_frames = self.chunk_frames;

        // Split the field borrows so `resampler` and the scratch buffers can be
        // used together.
        let Self {
            resampler,
            pending,
            chunk,
            output,
            mono,
            ..
        } = self;

        let Some(resampler) = resampler.as_mut() else {
            // No rate conversion needed: the down-mix is the output.
            output.clear();
            output.extend_from_slice(mono);
            return Ok(output);
        };

        pending.extend_from_slice(mono);
        output.clear();
        while pending.len() >= chunk_frames {
            chunk.clear();
            chunk.extend_from_slice(&pending[..chunk_frames]);
            pending.copy_within(chunk_frames.., 0);
            let remaining = pending.len() - chunk_frames;
            pending.truncate(remaining);

            let start = output.len();
            let frames = resampler.output_frames_next();
            output.resize(start + frames, 0.0);
            let (_, written) = resampler
                .process_into_buffer(&[chunk.as_slice()], &mut [&mut output[start..]], None)
                .map_err(|e| AppError::audio(format!("Resample failed: {e}")))?;
            output.truncate(start + written);
        }
        Ok(output)
    }

    /// Flush any buffered remainder by zero-padding to a final chunk.
    pub fn flush(&mut self) -> Result<Vec<f32>> {
        // Leftover holds at most `channels - 1` raw interleaved samples from a
        // partial frame; they are not mono output, so drop them.
        self.leftover_buffer.clear();
        let chunk_frames = self.chunk_frames;
        let Self {
            resampler,
            pending,
            chunk,
            output,
            ..
        } = self;

        let Some(resampler) = resampler.as_mut() else {
            return Ok(std::mem::take(pending));
        };
        if pending.is_empty() {
            return Ok(Vec::new());
        }
        chunk.clear();
        chunk.extend_from_slice(pending);
        pending.clear();
        chunk.resize(chunk_frames, 0.0);

        output.clear();
        let frames = resampler.output_frames_next();
        output.resize(frames, 0.0);
        let (_, written) = resampler
            .process_into_buffer(&[chunk.as_slice()], &mut [output.as_mut_slice()], None)
            .map_err(|e| AppError::audio(format!("Resample flush failed: {e}")))?;
        output.truncate(written);
        Ok(std::mem::take(output))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mix_stereo_to_mono_averages_channels() {
        let mut r = Resampler::new(16_000, 16_000, 2).unwrap();
        // L=1.0, R=0.0 -> 0.5 ; L=-1.0 R=-1.0 -> -1.0
        let mono = r.mix_to_mono(&[1.0, 0.0, -1.0, -1.0]);
        assert_eq!(mono, &[0.5, -1.0]);
    }

    #[test]
    fn same_rate_is_passthrough() {
        let mut r = Resampler::new(16_000, 16_000, 1).unwrap();
        let out = r.process(&[0.1, 0.2, 0.3]).unwrap();
        assert_eq!(out, &[0.1, 0.2, 0.3]);
    }

    #[test]
    fn process_output_is_overwritten_across_calls() {
        // The returned slice borrows an internal buffer reused per call: the
        // second call must replace, not append to, the first result.
        let mut r = Resampler::new(16_000, 16_000, 1).unwrap();
        assert_eq!(r.process(&[0.1, 0.2, 0.3]).unwrap(), &[0.1, 0.2, 0.3]);
        assert_eq!(r.process(&[0.4, 0.5]).unwrap(), &[0.4, 0.5]);
    }

    #[test]
    fn flush_drops_partial_frame_leftover() {
        let mut r = Resampler::new(16_000, 16_000, 2).unwrap();
        // One full stereo frame [1.0, 1.0] -> mono 1.0 ; the trailing 0.5 is
        // half a frame and must not leak into the output as a raw sample.
        assert_eq!(r.process(&[1.0, 1.0, 0.5]).unwrap(), &[1.0]);
        assert_eq!(r.flush().unwrap(), Vec::<f32>::new());
    }

    #[test]
    fn downsample_produces_fewer_samples() {
        let mut r = Resampler::new(48_000, 16_000, 1).unwrap();
        let input = vec![0.0f32; 4096];
        let mut total = r.process(&input).unwrap().len();
        total += r.flush().unwrap().len();
        // ~1/3 of input frames at 48k -> 16k.
        assert!(total > 0 && total < input.len());
    }
}
