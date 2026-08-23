//! Sample-rate conversion and channel down-mixing.
//!
//! Converts arbitrary input rate / channel count into the pipeline's canonical
//! 16 kHz mono `f32` stream using `rubato`'s sinc FFT resampler.

use rubato::{
    Resampler as _, SincFixedIn, SincInterpolationParameters, SincInterpolationType, WindowFunction,
};

use crate::error::{AppError, Result};

/// Streaming resampler that mixes to mono and converts to a target rate.
pub struct Resampler {
    channels: usize,
    resampler: Option<SincFixedIn<f32>>,
    chunk_frames: usize,
    /// Mono samples awaiting a full resampler chunk.
    pending: Vec<f32>,
    /// Leftover samples from mix_to_mono (incomplete channel frame).
    leftover_buffer: Vec<f32>,
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
        })
    }

    /// Down-mix interleaved frames to a single mono channel (average).
    pub fn mix_to_mono(&mut self, interleaved: &[f32]) -> Vec<f32> {
        // Prepend any leftover samples from the previous call
        let total: Vec<f32> = [self.leftover_buffer.as_slice(), interleaved].concat();
        self.leftover_buffer.clear();

        if self.channels <= 1 {
            return total;
        }

        let frames = total.len() / self.channels;
        let leftover = total.len() % self.channels;
        if leftover != 0 {
            // Save leftover samples for next call instead of dropping
            let split_point = frames * self.channels;
            self.leftover_buffer
                .extend_from_slice(&total[split_point..]);
        }

        let mut mono = Vec::with_capacity(frames);
        for f in 0..frames {
            let base = f * self.channels;
            let mut sum = 0.0f32;
            for c in 0..self.channels {
                sum += total[base + c];
            }
            mono.push(sum / self.channels as f32);
        }
        mono
    }

    /// Feed interleaved input samples, returning any newly available 16 kHz
    /// mono output. Output may lag input because the FFT resampler consumes
    /// fixed-size chunks.
    pub fn process(&mut self, interleaved: &[f32]) -> Result<Vec<f32>> {
        let mono = self.mix_to_mono(interleaved);

        let Some(resampler) = self.resampler.as_mut() else {
            // No rate conversion needed.
            return Ok(mono);
        };

        self.pending.extend_from_slice(&mono);
        let mut out = Vec::new();
        while self.pending.len() >= self.chunk_frames {
            let chunk: Vec<f32> = self.pending.drain(..self.chunk_frames).collect();
            let processed = resampler
                .process(&[chunk], None)
                .map_err(|e| AppError::audio(format!("Resample failed: {e}")))?;
            if let Some(first) = processed.into_iter().next() {
                out.extend(first);
            }
        }
        Ok(out)
    }

    /// Flush any buffered remainder by zero-padding to a final chunk.
    pub fn flush(&mut self) -> Result<Vec<f32>> {
        let Some(resampler) = self.resampler.as_mut() else {
            let mut rem = std::mem::take(&mut self.pending);
            rem.extend_from_slice(&self.leftover_buffer);
            self.leftover_buffer.clear();
            return Ok(rem);
        };
        if self.pending.is_empty() && self.leftover_buffer.is_empty() {
            return Ok(Vec::new());
        }
        let mut chunk: Vec<f32> = std::mem::take(&mut self.pending);
        chunk.extend_from_slice(&self.leftover_buffer);
        self.leftover_buffer.clear();
        chunk.resize(self.chunk_frames, 0.0);
        let processed = resampler
            .process(&[chunk], None)
            .map_err(|e| AppError::audio(format!("Resample flush failed: {e}")))?;
        Ok(processed.into_iter().next().unwrap_or_default())
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
        assert_eq!(mono, vec![0.5, -1.0]);
    }

    #[test]
    fn same_rate_is_passthrough() {
        let mut r = Resampler::new(16_000, 16_000, 1).unwrap();
        let out = r.process(&[0.1, 0.2, 0.3]).unwrap();
        assert_eq!(out, vec![0.1, 0.2, 0.3]);
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
