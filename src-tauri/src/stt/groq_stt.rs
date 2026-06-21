//! Groq Whisper REST STT client.
//!
//! POSTs a 16 kHz mono WAV (built in-memory from the captured samples) to
//! `https://api.groq.com/openai/v1/audio/transcriptions` and parses the
//! `verbose_json` response.

use async_trait::async_trait;
use serde::Deserialize;

use super::types::GroqSttConfig;
use super::{SttConfig, SttEngine, TranscriptionResult};
use crate::error::{AppError, Result};

const GROQ_STT_URL: &str = "https://api.groq.com/openai/v1/audio/transcriptions";

pub struct GroqSttEngine {
    client: reqwest::Client,
    config: GroqSttConfig,
}

impl GroqSttEngine {
    pub fn new(config: GroqSttConfig) -> Self {
        Self {
            client: crate::util::http_client(),
            config,
        }
    }
}

#[derive(Debug, Deserialize)]
struct GroqSegment {
    #[serde(default)]
    confidence: Option<f32>,
}

#[derive(Debug, Deserialize)]
struct GroqResponse {
    text: String,
    #[serde(default)]
    language: Option<String>,
    #[serde(default)]
    duration: Option<f64>,
    #[serde(default)]
    segments: Vec<GroqSegment>,
}

#[async_trait]
impl SttEngine for GroqSttEngine {
    async fn transcribe(&self, audio: &[f32], config: &SttConfig) -> Result<TranscriptionResult> {
        if self.config.api_key.trim().is_empty() {
            return Err(AppError::api_key_missing("Groq API key is not set"));
        }

        let wav = encode_wav_16k_mono(audio);
        let language = if config.language == "auto" {
            // Groq Whisper doesn't accept "auto" as a language parameter.
            // When auto-detecting, omit the language field.
            None
        } else {
            Some(config.language.clone())
        };

        // Network call wrapped in retry-with-backoff (max 3 retries) for
        // transient failures. Auth errors fail fast.
        let body = crate::util::retry_with_backoff(3, std::time::Duration::from_secs(1), || {
            let wav = wav.clone();
            let language = language.clone();
            async move {
                let part = reqwest::multipart::Part::bytes(wav)
                    .file_name("audio.wav")
                    .mime_str("audio/wav")
                    .map_err(|e| AppError::stt(format!("multipart error: {e}")))?;

                let mut form = reqwest::multipart::Form::new()
                    .part("file", part)
                    .text("model", self.config.model.clone())
                    .text("temperature", self.config.temperature.to_string())
                    .text("response_format", "verbose_json");

                if let Some(lang) = language {
                    form = form.text("language", lang);
                }

                let resp = self
                    .client
                    .post(GROQ_STT_URL)
                    .bearer_auth(&self.config.api_key)
                    .multipart(form)
                    .send()
                    .await?;

                let status = resp.status();
                let body = resp.text().await?;

                if status == reqwest::StatusCode::UNAUTHORIZED {
                    return Err(AppError::new(
                        crate::error::ErrorCode::SttApiKeyInvalid,
                        "Groq rejected the API key (401)",
                    ));
                }
                if !status.is_success() {
                    return Err(AppError::stt_api(format!(
                        "Groq STT error {status}: {body}"
                    )));
                }
                Ok(body)
            }
        })
        .await?;

        let parsed: GroqResponse = serde_json::from_str(&body)?;
        let confidence = if parsed.segments.is_empty() {
            1.0
        } else {
            let sum: f32 = parsed
                .segments
                .iter()
                .map(|s| s.confidence.unwrap_or(1.0))
                .sum();
            sum / parsed.segments.len() as f32
        };

        Ok(TranscriptionResult {
            text: parsed.text.trim().to_string(),
            confidence,
            language: normalize_language(parsed.language.as_deref()),
            duration_ms: parsed.duration.map(|d| (d * 1000.0) as u64).unwrap_or(0),
            words: Vec::new(),
            raw_response: Some(body),
        })
    }

    fn name(&self) -> &'static str {
        "groq_whisper"
    }
}

/// Map Whisper's verbose language names to ISO codes used internally.
fn normalize_language(lang: Option<&str>) -> String {
    match lang.map(|l| l.to_lowercase()) {
        Some(l) if l == "indonesian" || l == "id" => "id".to_string(),
        Some(l) if l == "english" || l == "en" => "en".to_string(),
        Some(l) => l,
        None => "unknown".to_string(),
    }
}

/// Encode mono 16 kHz `f32` samples as a 16-bit PCM WAV byte buffer.
pub fn encode_wav_16k_mono(samples: &[f32]) -> Vec<u8> {
    let sample_rate: u32 = 16_000;
    let channels: u16 = 1;
    let bits_per_sample: u16 = 16;
    let byte_rate = sample_rate * channels as u32 * (bits_per_sample / 8) as u32;
    let block_align = channels * (bits_per_sample / 8);
    let data_len = (samples.len() * 2) as u32;
    let riff_len = 36 + data_len;

    let mut buf = Vec::with_capacity(44 + samples.len() * 2);
    buf.extend_from_slice(b"RIFF");
    buf.extend_from_slice(&riff_len.to_le_bytes());
    buf.extend_from_slice(b"WAVE");
    buf.extend_from_slice(b"fmt ");
    buf.extend_from_slice(&16u32.to_le_bytes()); // fmt chunk size
    buf.extend_from_slice(&1u16.to_le_bytes()); // PCM
    buf.extend_from_slice(&channels.to_le_bytes());
    buf.extend_from_slice(&sample_rate.to_le_bytes());
    buf.extend_from_slice(&byte_rate.to_le_bytes());
    buf.extend_from_slice(&block_align.to_le_bytes());
    buf.extend_from_slice(&bits_per_sample.to_le_bytes());
    buf.extend_from_slice(b"data");
    buf.extend_from_slice(&data_len.to_le_bytes());
    for &s in samples {
        let v = (s.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
        buf.extend_from_slice(&v.to_le_bytes());
    }
    buf
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wav_header_is_well_formed() {
        let wav = encode_wav_16k_mono(&[0.0, 0.5, -0.5]);
        assert_eq!(&wav[0..4], b"RIFF");
        assert_eq!(&wav[8..12], b"WAVE");
        // 44-byte header + 3 samples * 2 bytes.
        assert_eq!(wav.len(), 44 + 6);
    }

    #[test]
    fn normalize_language_maps_names() {
        assert_eq!(normalize_language(Some("indonesian")), "id");
        assert_eq!(normalize_language(Some("English")), "en");
        assert_eq!(normalize_language(None), "unknown");
    }
}
