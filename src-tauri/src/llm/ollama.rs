//! Ollama local LLM HTTP client.
//!
//! Uses the non-streaming `/api/generate` endpoint for simplicity (the whole
//! formatted output is short). Falls back paths are handled by the factory.

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::json;

use super::prompts::{system_prompt, translation_prompt};
use super::types::{LlmMode, OllamaConfig};
use super::LlmFormatter;
use crate::error::{AppError, Result};

pub struct OllamaFormatter {
    client: reqwest::Client,
    config: OllamaConfig,
}

impl OllamaFormatter {
    pub fn new(config: OllamaConfig) -> Self {
        Self {
            client: crate::util::http_client(),
            config,
        }
    }

    async fn generate(&self, system: &str, prompt: &str) -> Result<String> {
        let url = format!(
            "{}/api/generate",
            self.config.endpoint.trim_end_matches('/')
        );
        let body = json!({
            "model": self.config.model,
            "prompt": prompt,
            "system": system,
            "stream": false,
            "options": {
                "temperature": self.config.temperature,
                "top_p": self.config.top_p,
            }
        });

        let resp = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                if e.is_connect() {
                    AppError::llm_connection_refused(format!("Ollama connection refused: {e}"))
                } else {
                    AppError::from(e)
                }
            })?;
        let status = resp.status();
        let text = resp.text().await?;
        if !status.is_success() {
            return Err(AppError::llm(format!("Ollama error {status}: {text}")));
        }
        let parsed: OllamaResponse = serde_json::from_str(&text)?;
        Ok(parsed.response.trim().to_string())
    }
}

#[derive(Debug, Deserialize)]
struct OllamaResponse {
    response: String,
}

#[async_trait]
impl LlmFormatter for OllamaFormatter {
    async fn format(&self, text: &str, mode: &LlmMode) -> Result<String> {
        let system = system_prompt(mode);
        self.generate(&system, text).await
    }

    async fn translate(&self, text: &str, source: &str, target: &str) -> Result<String> {
        let system = translation_prompt(source, target);
        self.generate(&system, text).await
    }

    fn name(&self) -> &'static str {
        "ollama"
    }
}
