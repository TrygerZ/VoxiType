//! Ollama local LLM HTTP client.
//!
//! Uses the non-streaming `/api/generate` endpoint for simplicity (the whole
//! formatted output is short). Falls back paths are handled by the factory.

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::json;

use super::prompts::{format_user_prefix, system_prompt, translation_prompt};
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
        parse_response_text(&text)
    }
}

#[derive(Debug, Deserialize)]
struct OllamaResponse {
    response: String,
}

fn parse_response_text(text: &str) -> Result<String> {
    let parsed: OllamaResponse = serde_json::from_str(text)?;
    let trimmed = parsed.response.trim();
    if trimmed.is_empty() {
        return Err(AppError::llm("Ollama returned an empty response"));
    }
    Ok(trimmed.to_string())
}

#[async_trait]
impl LlmFormatter for OllamaFormatter {
    async fn format(&self, text: &str, mode: &LlmMode, language: &str) -> Result<String> {
        let system = system_prompt(mode, language);
        let prompt = format!("{}\n\n{}", format_user_prefix(language), text);
        self.generate(&system, &prompt).await
    }

    async fn translate(&self, text: &str, source: &str, target: &str) -> Result<String> {
        let system = translation_prompt(source, target);
        let prompt = format!("Dictated text (format only, do NOT answer):\n\n{}", text);
        self.generate(&system, &prompt).await
    }

    fn name(&self) -> &'static str {
        "ollama"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_response_is_an_error() {
        assert!(parse_response_text(r#"{"response":""}"#).is_err());
        assert!(parse_response_text(r#"{"response":"   "}"#).is_err());
    }

    #[test]
    fn response_is_trimmed() {
        assert_eq!(
            parse_response_text(r#"{"response":"  Halo dunia. \n"}"#).unwrap(),
            "Halo dunia."
        );
    }
}
