//! Shared STT data types.

use serde::{Deserialize, Serialize};

/// Result of a transcription request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptionResult {
    pub text: String,
    pub confidence: f32,
    /// Detected language: "id", "en", "unknown".
    pub language: String,
    pub duration_ms: u64,
    /// Raw provider response for debugging.
    pub raw_response: Option<String>,
}

impl TranscriptionResult {
    pub fn text_only(text: impl Into<String>, language: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            confidence: 1.0,
            language: language.into(),
            duration_ms: 0,
            raw_response: None,
        }
    }
}

/// Engine selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SttEngineKind {
    Groq,
    WhisperCpp,
}

/// Runtime config passed to a transcription call.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SttConfig {
    /// "auto", "id", "en".
    pub language: String,
    /// Optional hotwords / initial prompt to bias decoding.
    pub initial_prompt: Option<String>,
    /// Decoding temperature.
    pub temperature: f32,
}

impl Default for SttConfig {
    fn default() -> Self {
        Self {
            language: "auto".to_string(),
            initial_prompt: None,
            temperature: 0.0,
        }
    }
}

/// Groq Whisper REST configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroqSttConfig {
    pub api_key: String,
    pub model: String,
    pub language: String,
    pub temperature: f32,
}

impl Default for GroqSttConfig {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            model: "whisper-large-v3-turbo".to_string(),
            language: "auto".to_string(),
            temperature: 0.0,
        }
    }
}

/// Local whisper.cpp CLI configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WhisperCppConfig {
    pub binary_path: String,
    pub model_path: String,
    pub threads: u32,
}

impl Default for WhisperCppConfig {
    fn default() -> Self {
        Self {
            binary_path: "whisper-cli".to_string(),
            model_path: String::new(),
            threads: 4,
        }
    }
}
