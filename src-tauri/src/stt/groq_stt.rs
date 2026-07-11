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

    /// Single-pass transcription: encode audio, call Groq, parse response.
    async fn transcribe_once(
        &self,
        audio: &[f32],
        config: &SttConfig,
    ) -> Result<TranscriptionResult> {
        let wav = encode_wav_16k_mono(audio);
        let language = if config.language == "auto" {
            None
        } else {
            Some(config.language.clone())
        };

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
                    .text("temperature", config.temperature.to_string())
                    .text("response_format", "verbose_json");

                if let Some(ref prompt) = config.initial_prompt {
                    form = form.text("prompt", prompt.clone());
                }

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
        let confidence = compute_confidence(&parsed.segments);

        let whisper_lang = normalize_language(parsed.language.as_deref());
        let text = parsed.text.trim().to_string();
        let language = reconcile_language(&text, &whisper_lang);
        Ok(TranscriptionResult {
            text,
            confidence,
            language,
            duration_ms: parsed.duration.map(|d| (d * 1000.0) as u64).unwrap_or(0),
            raw_response: Some(body),
        })
    }
}

#[derive(Debug, Deserialize)]
struct GroqSegment {
    /// Average log-probability of tokens in this segment (from verbose_json).
    /// Higher (less negative) = more confident transcription. Used as a
    /// language-correctness signal: when Whisper transcribes in the wrong
    /// language, avg_logprob drops significantly.
    #[serde(default)]
    avg_logprob: Option<f32>,
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

        let first = self.transcribe_once(audio, config).await?;

        if config.language != "auto" {
            return Ok(first);
        }

        // Whisper's auto-detection can misdetect English as Indonesian,
        // especially for longer clips. When it does, Whisper transcribes the
        // English audio *as* Indonesian text — so the text itself looks
        // Indonesian and simple heuristics can't catch the error.
        //
        // Strategy: when auto-detect says "id", re-transcribe with an explicit
        // "en" hint and compare. The correct-language pass produces text with
        // more function-word markers of its own language.
        if first.language == "id" {
            let en_config = SttConfig {
                language: "en".to_string(),
                ..config.clone()
            };
            match self.transcribe_once(audio, &en_config).await {
                Ok(en_result) => {
                    let winner = pick_better_transcription(first, en_result);
                    tracing::info!(
                        "STT auto-detect: id→en verification picked '{}'",
                        winner.language
                    );
                    return Ok(winner);
                }
                Err(e) => {
                    tracing::warn!("STT en re-transcription failed, using first pass: {e}");
                }
            }
        }

        Ok(first)
    }

    fn name(&self) -> &'static str {
        "groq_whisper"
    }
}

/// Derive a 0.0–1.0 confidence score from segment avg_logprob values.
///
/// Whisper's `verbose_json` includes `avg_logprob` per segment — the average
/// log-probability of generated tokens. When Whisper transcribes in the
/// *correct* language, tokens match the audio well and avg_logprob is high
/// (e.g. −0.3). When it transcribes in the *wrong* language, it forces
/// tokens that don't match the audio and avg_logprob drops (e.g. −1.2).
///
/// We convert avg_logprob to a 0–1 scale where 0 ≈ −2.0 (very poor) and
/// 1.0 ≈ 0.0 (perfect). This gives `pick_better_transcription` a
/// language-agnostic quality signal that works in both directions.
fn compute_confidence(segments: &[GroqSegment]) -> f32 {
    if segments.is_empty() {
        return 1.0;
    }
    let logprobs: Vec<f32> = segments.iter().filter_map(|s| s.avg_logprob).collect();
    if logprobs.is_empty() {
        return 1.0;
    }
    let avg = logprobs.iter().sum::<f32>() / logprobs.len() as f32;
    // Map [−2.0, 0.0] → [0.0, 1.0]
    (1.0 + avg / 2.0).clamp(0.0, 1.0)
}

/// Compare two transcription results (one auto-detected as "id", one forced
/// "en") and return whichever text is more internally consistent with its
/// own detected language.
///
/// When Whisper misdetects English as Indonesian, the "id" pass produces
/// Indonesian-ish text while the "en" pass produces clear English. When the
/// audio is genuinely Indonesian, the "id" pass produces clear Indonesian
/// while the "en" pass produces English-ish gibberish or mixed text. We pick
/// the result whose text has more function-word markers of its own language.
fn pick_better_transcription(
    id_result: TranscriptionResult,
    en_result: TranscriptionResult,
) -> TranscriptionResult {
    // Primary signal: confidence (derived from avg_logprob).
    // The correct-language pass has measurably higher confidence because
    // Whisper's token probabilities are higher when the language matches.
    const CONFIDENCE_THRESHOLD: f32 = 0.1;
    if en_result.confidence > id_result.confidence + CONFIDENCE_THRESHOLD {
        return en_result;
    }
    if id_result.confidence > en_result.confidence + CONFIDENCE_THRESHOLD {
        return id_result;
    }

    // Confidence is too close to call — fall back to text markers.
    let id_lower = format!(" {} ", id_result.text.to_lowercase());

    let id_id_count = ID_LANGUAGE_MARKERS
        .iter()
        .filter(|&&m| id_lower.contains(m))
        .count();
    let id_en_count = EN_LANGUAGE_MARKERS
        .iter()
        .filter(|&&m| id_lower.contains(m))
        .count();

    // Only override auto-detect when the text *contradicts* the detection:
    // the "id" pass produced English markers (and no Indonesian), meaning
    // Whisper transcribed English audio but labeled it Indonesian.
    if id_en_count > 0 && id_id_count == 0 {
        return en_result;
    }
    // Otherwise trust Whisper's auto-detection — both passes produced
    // plausible text and we have no strong reason to override.
    id_result
}

/// Common English function words for language detection.
/// These are language-specific — they don't appear in Indonesian text.
const EN_LANGUAGE_MARKERS: &[&str] = &[
    " the ", " and ", " is ", " to ", " of ", " a ", " in ", " for ", " it ", " with ", " you ",
    " that ", " have ", " this ", " from ", " or ", " be ", " are ", " was ", " will ", " can ",
    " do ", " not ", " on ", " at ", " we ", " he ", " she ", " they ", " but ", " what ",
    " when ", " how ", " why ", " who ", " there ", " been ",
];

/// Common Indonesian function words for language detection.
/// These are language-specific — they don't appear in English text.
const ID_LANGUAGE_MARKERS: &[&str] = &[
    " yang ", " dan ", " di ", " ini ", " itu ", " dari ", " dengan ", " untuk ", " tidak ",
    " adalah ", " juga ", " karena ", " pada ", " saya ", " anda ", " dia ", " kita ", " mereka ",
    " bisa ", " akan ", " sudah ", " atau ", " ya ", " nah ", " lagi ", " belum ", " hanya ",
    " masih ", " sangat ",
];

/// Map Whisper's verbose language names to ISO codes used internally.
fn normalize_language(lang: Option<&str>) -> String {
    match lang.map(|l| l.to_lowercase()) {
        Some(l) if l == "indonesian" || l == "id" => "id".to_string(),
        Some(l) if l == "english" || l == "en" => "en".to_string(),
        Some(l) if l.trim().is_empty() => "unknown".to_string(),
        Some(l) => l,
        None => "unknown".to_string(),
    }
}

/// Cross-check Whisper's detected language against the transcribed text.
///
/// These function words are language-specific — "the"/"is"/"of" never
/// appear in Indonesian, "yang"/"dan"/"di" never appear in English — so
/// even a single marker is a strong signal. The text is padded with
/// spaces so the first and last words are matched correctly.
///
/// Returns the corrected language code, or the original when the text
/// is ambiguous (no markers of either language).
fn reconcile_language(text: &str, whisper_lang: &str) -> String {
    if whisper_lang != "id" && whisper_lang != "en" && whisper_lang != "unknown" {
        return whisper_lang.to_string();
    }
    let lower = format!(" {} ", text.to_lowercase());
    let en_count = EN_LANGUAGE_MARKERS
        .iter()
        .filter(|&&m| lower.contains(m))
        .count();
    let id_count = ID_LANGUAGE_MARKERS
        .iter()
        .filter(|&&m| lower.contains(m))
        .count();
    // Function words are language-specific, so a single marker is decisive
    // when the other language has none.
    if en_count >= 1 && id_count == 0 {
        return "en".to_string();
    }
    if id_count >= 1 && en_count == 0 {
        return "id".to_string();
    }
    // When both appear (code-switching), trust the majority.
    if en_count > id_count {
        return "en".to_string();
    }
    if id_count > en_count {
        return "id".to_string();
    }
    whisper_lang.to_string()
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
    buf.extend_from_slice(&16u32.to_le_bytes());
    buf.extend_from_slice(&1u16.to_le_bytes());
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
    fn reconcile_overrides_id_for_clearly_english_text() {
        assert_eq!(
            reconcile_language("This is the best app for me", "id"),
            "en"
        );
    }

    #[test]
    fn reconcile_detects_english_with_single_marker() {
        assert_eq!(reconcile_language("the report is ready", "id"), "en");
        assert_eq!(reconcile_language("what time is it", "id"), "en");
    }

    #[test]
    fn reconcile_detects_indonesian_with_single_marker() {
        assert_eq!(reconcile_language("makan dulu ya", "en"), "id");
        assert_eq!(reconcile_language("belum selesai", "en"), "id");
    }

    #[test]
    fn reconcile_matches_first_and_last_word() {
        // Boundary words must be matched (text is padded with spaces).
        assert_eq!(reconcile_language("the quick brown fox", "id"), "en");
        assert_eq!(reconcile_language("saya suka makan", "en"), "id");
    }

    #[test]
    fn reconcile_trusts_whisper_for_code_switched_text() {
        // When both languages have equal markers, keep Whisper's detection.
        assert_eq!(reconcile_language("the file ini", "id"), "id");
        assert_eq!(reconcile_language("the file ini", "en"), "en");
    }

    #[test]
    fn reconcile_keeps_id_for_actually_indonesian_text() {
        assert_eq!(
            reconcile_language("Saya pergi ke pasar dengan mereka kemarin", "id"),
            "id"
        );
    }

    #[test]
    fn reconcile_defaults_to_whisper_for_short_ambiguous_text() {
        assert_eq!(reconcile_language("yes", "id"), "id");
        assert_eq!(reconcile_language("yes", "en"), "en");
    }

    #[test]
    fn wav_header_is_well_formed() {
        let wav = encode_wav_16k_mono(&[0.0, 0.5, -0.5]);
        assert_eq!(&wav[0..4], b"RIFF");
        assert_eq!(&wav[8..12], b"WAVE");
        assert_eq!(wav.len(), 44 + 6);
    }

    #[test]
    fn pick_better_uses_confidence_as_primary_signal() {
        // "id" pass has low confidence (wrong language), "en" pass has high
        // confidence (correct language). Should pick "en" regardless of markers.
        let id_res = TranscriptionResult {
            text: "the system is working today".to_string(),
            confidence: 0.3,
            language: "id".to_string(),
            duration_ms: 0,
            raw_response: None,
        };
        let en_res = TranscriptionResult {
            text: "the system is working today".to_string(),
            confidence: 0.85,
            language: "en".to_string(),
            duration_ms: 0,
            raw_response: None,
        };
        let winner = pick_better_transcription(id_res, en_res);
        assert_eq!(winner.language, "en");
    }

    #[test]
    fn pick_better_keeps_id_when_confidence_is_higher() {
        // Indonesian audio correctly detected: "id" pass has high confidence,
        // "en" pass has low confidence. Should keep "id".
        let id_res = TranscriptionResult {
            text: "saya pergi ke pasar dengan mereka".to_string(),
            confidence: 0.85,
            language: "id".to_string(),
            duration_ms: 0,
            raw_response: None,
        };
        let en_res = TranscriptionResult {
            text: "sahya pergee ke pasar".to_string(),
            confidence: 0.3,
            language: "en".to_string(),
            duration_ms: 0,
            raw_response: None,
        };
        let winner = pick_better_transcription(id_res, en_res);
        assert_eq!(winner.language, "id");
    }

    #[test]
    fn pick_better_falls_back_to_markers_on_tie() {
        // Confidence is equal (both 1.0). The "id" pass text has English
        // markers and no Indonesian markers → contradiction → pick "en".
        let id_res = TranscriptionResult::text_only("the system is working", "id");
        let en_res = TranscriptionResult::text_only("the system is working", "en");
        let winner = pick_better_transcription(id_res, en_res);
        assert_eq!(winner.language, "en");
    }

    #[test]
    fn pick_better_trusts_autodetect_when_ambiguous() {
        // Confidence is equal, "id" text has no English markers (no
        // contradiction). Should trust auto-detect ("id").
        let id_res = TranscriptionResult::text_only("halo dunia", "id");
        let en_res = TranscriptionResult::text_only("hello world", "en");
        let winner = pick_better_transcription(id_res, en_res);
        assert_eq!(winner.language, "id");
    }

    #[test]
    fn normalize_language_maps_names() {
        assert_eq!(normalize_language(Some("indonesian")), "id");
        assert_eq!(normalize_language(Some("English")), "en");
        assert_eq!(normalize_language(None), "unknown");
        assert_eq!(normalize_language(Some("")), "unknown");
        assert_eq!(normalize_language(Some("  ")), "unknown");
    }
}
