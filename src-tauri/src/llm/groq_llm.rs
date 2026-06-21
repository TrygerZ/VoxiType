//! Groq cloud LLM client (OpenAI-compatible chat completions).
//!
//! Uses non-streaming completions; the formatted output is short enough that
//! streaming adds little value here.

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::json;

use super::prompts::{system_prompt, translation_prompt};
use super::types::{GroqLlmConfig, LlmMode};
use super::LlmFormatter;
use crate::error::{AppError, ErrorCode, Result};

const GROQ_CHAT_URL: &str = "https://api.groq.com/openai/v1/chat/completions";

pub struct GroqLlmFormatter {
    client: reqwest::Client,
    config: GroqLlmConfig,
}

impl GroqLlmFormatter {
    pub fn new(config: GroqLlmConfig) -> Self {
        Self {
            client: crate::util::http_client(),
            config,
        }
    }

    async fn chat(&self, system: &str, user: &str) -> Result<String> {
        if self.config.api_key.trim().is_empty() {
            return Err(AppError::api_key_missing("Groq API key is not set"));
        }
        let body = json!({
            "model": self.config.model,
            "messages": [
                { "role": "system", "content": system },
                { "role": "user", "content": format!("Text to format:\n\n{}", user) },
            ],
            "temperature": 0.0, // Force 0.0 for strict formatting
            "max_tokens": self.config.max_tokens,
            "stream": false,
        });

        // Retry transient failures with exponential backoff; auth fails fast.
        let text = crate::util::retry_with_backoff(3, std::time::Duration::from_secs(1), || {
            let body = body.clone();
            async move {
                let resp = self
                    .client
                    .post(GROQ_CHAT_URL)
                    .bearer_auth(&self.config.api_key)
                    .json(&body)
                    .send()
                    .await?;

                let status = resp.status();
                let text = resp.text().await?;
                if status == reqwest::StatusCode::UNAUTHORIZED {
                    return Err(AppError::new(
                        ErrorCode::LlmApiKeyInvalid,
                        "Groq rejected the API key (401)",
                    ));
                }
                if !status.is_success() {
                    return Err(AppError::llm(format!("Groq LLM error {status}: {text}")));
                }
                Ok(text)
            }
        })
        .await?;

        let parsed: ChatResponse = serde_json::from_str(&text)?;
        let content = parsed
            .choices
            .into_iter()
            .next()
            .map(|c| c.message.content)
            .unwrap_or_default();
        Ok(content.trim().to_string())
    }
}

#[derive(Debug, Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: Message,
}

#[derive(Debug, Deserialize)]
struct Message {
    content: String,
}

#[async_trait]
impl LlmFormatter for GroqLlmFormatter {
    async fn format(&self, text: &str, mode: &LlmMode) -> Result<String> {
        let system = system_prompt(mode);
        self.chat(&system, text).await
    }

    async fn translate(&self, text: &str, source: &str, target: &str) -> Result<String> {
        let system = translation_prompt(source, target);
        self.chat(&system, text).await
    }

    fn name(&self) -> &'static str {
        "groq_llm"
    }
}
