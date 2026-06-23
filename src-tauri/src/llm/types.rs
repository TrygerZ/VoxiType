//! LLM configuration and mode types.

use serde::{Deserialize, Serialize};

/// Formatting mode. `Custom` carries a user-defined system prompt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LlmMode {
    Dictation,
    Message,
    Email,
    Custom(String),
}

impl LlmMode {
    /// Resolve a mode id string from settings into a mode.
    pub fn from_id(id: &str) -> Self {
        match id {
            "dictation" => Self::Dictation,
            "message" => Self::Message,
            "email" => Self::Email,
            other => Self::Custom(other.to_string()),
        }
    }

    /// The settings id string for this mode (inverse of [`from_id`]).
    pub fn id(&self) -> String {
        match self {
            Self::Dictation => "dictation".to_string(),
            Self::Message => "message".to_string(),
            Self::Email => "email".to_string(),
            Self::Custom(id) => id.clone(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LlmEngineKind {
    Off,
    Ollama,
    Groq,
    RuleBased,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaConfig {
    pub endpoint: String,
    pub model: String,
    pub temperature: f32,
    pub top_p: f32,
}

impl Default for OllamaConfig {
    fn default() -> Self {
        Self {
            endpoint: "http://localhost:11434".to_string(),
            model: "qwen2.5:3b".to_string(),
            temperature: 0.1,
            top_p: 0.9,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroqLlmConfig {
    pub api_key: String,
    pub model: String,
    pub temperature: f32,
    pub max_tokens: u32,
}

impl Default for GroqLlmConfig {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            model: "llama-3.1-8b-instant".to_string(),
            temperature: 0.1,
            max_tokens: 500,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleBasedConfig {
    pub filler_words: Vec<String>,
}

impl Default for RuleBasedConfig {
    fn default() -> Self {
        Self {
            filler_words: ["um", "uh", "like", "you know", "anu", "eee", "ah"]
                .iter()
                .map(|s| s.to_string())
                .collect(),
        }
    }
}
