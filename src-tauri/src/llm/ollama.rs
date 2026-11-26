//! Ollama local LLM HTTP client.
//!
//! Uses the non-streaming `/api/generate` endpoint for simplicity (the whole
//! formatted output is short). Falls back paths are handled by the factory.
//!
//! Trust assumption (L-33): dictated text is sent unauthenticated to
//! `http://localhost:11434`. A local process that binds that port while Ollama
//! is absent would receive the text and could return arbitrary formatted
//! output, which can only become literal injected text (never key macros).
//! This is accepted for a local-first tool; a per-install shared secret is the
//! upgrade path if that threat matters.

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::json;

use super::prompts::{
    format_user_prefix, system_prompt, translation_prompt, translation_user_prefix,
};
use super::types::{LlmMode, OllamaConfig};
use super::LlmFormatter;
use crate::error::{AppError, Result};

pub struct OllamaFormatter {
    client: reqwest::Client,
    config: OllamaConfig,
}

impl OllamaFormatter {
    pub fn new(config: OllamaConfig) -> Result<Self> {
        Ok(Self {
            client: crate::util::http_client()?,
            config,
        })
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
            .timeout(self.config.request_timeout)
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
        let text =
            crate::util::read_body_capped(resp, crate::util::MAX_RESPONSE_BODY_BYTES).await?;
        if !status.is_success() {
            let safe_text = crate::util::sanitize_error_body(&text);
            return Err(AppError::llm(format!("Ollama error {status}: {safe_text}"))
                .with_http_status(status.as_u16()));
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
        let prompt = format!("{}\n\n{}", translation_user_prefix(), text);
        self.generate(&system, &prompt).await
    }

    fn name(&self) -> &'static str {
        "ollama"
    }
}

const TAGS_CHECK_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);

#[derive(Debug, Deserialize)]
struct OllamaTagsResponse {
    #[serde(default)]
    models: Vec<OllamaTagEntry>,
}

#[derive(Debug, Deserialize)]
struct OllamaTagEntry {
    name: String,
}

/// Checks whether a requested Ollama model name matches any installed tag in `/api/tags`.
pub fn model_matches_tag_list(requested: &str, available: &[String]) -> bool {
    let req = requested.trim();
    if req.is_empty() {
        return false;
    }
    let latest = (!req.contains(':')).then(|| format!("{req}:latest"));
    available.iter().any(|entry| {
        let candidate = entry.trim();
        candidate == req || latest.as_deref() == Some(candidate)
    })
}

/// Verifies that the local Ollama daemon responds on `/api/tags` and hosts `model`.
pub async fn check_model_ready(endpoint: &str, model: &str) -> Result<()> {
    let trimmed = model.trim();
    if trimmed.is_empty() {
        return Err(AppError::invalid_input("Ollama model cannot be empty"));
    }

    let url = format!("{}/api/tags", endpoint.trim_end_matches('/'));
    let client = crate::util::http_client()?;
    let resp = client
        .get(&url)
        .timeout(TAGS_CHECK_TIMEOUT)
        .send()
        .await
        .map_err(|e| {
            if e.is_connect() {
                AppError::llm_connection_refused(format!("Ollama connection refused: {e}"))
            } else if e.is_timeout() {
                AppError::timeout(format!("Ollama readiness check timed out: {e}"))
            } else {
                AppError::from(e)
            }
        })?;

    let status = resp.status();
    let text = crate::util::read_body_capped(resp, crate::util::MAX_RESPONSE_BODY_BYTES).await?;
    if !status.is_success() {
        let safe_text = crate::util::sanitize_error_body(&text);
        return Err(AppError::llm(format!("Ollama error {status}: {safe_text}"))
            .with_http_status(status.as_u16()));
    }

    let parsed: OllamaTagsResponse = serde_json::from_str(&text)?;
    let names: Vec<String> = parsed.models.into_iter().map(|entry| entry.name).collect();
    if !model_matches_tag_list(trimmed, &names) {
        return Err(AppError::llm(format!(
            "Model '{trimmed}' is not installed in Ollama"
        )));
    }
    Ok(())
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

    #[test]
    fn model_matches_exact_and_latest_tags() {
        let installed = vec![
            "qwen2.5:3b".to_string(),
            "llama3.2:latest".to_string(),
            "mistral:7b-instruct".to_string(),
        ];

        assert!(model_matches_tag_list("qwen2.5:3b", &installed));
        assert!(model_matches_tag_list("llama3.2", &installed));
        assert!(model_matches_tag_list("llama3.2:latest", &installed));
        assert!(model_matches_tag_list("  qwen2.5:3b  ", &installed));
        assert!(!model_matches_tag_list("gemma2:9b", &installed));
        assert!(!model_matches_tag_list("", &installed));
        assert!(!model_matches_tag_list("   ", &installed));
        assert!(!model_matches_tag_list("qwen2.5:3b", &[]));
    }
}
