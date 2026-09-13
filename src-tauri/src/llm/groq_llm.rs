//! Groq cloud LLM client (OpenAI-compatible chat completions).
//!
//! Uses non-streaming completions; the formatted output is short enough that
//! streaming adds little value here.

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::json;

use super::prompts::{format_user_prefix, system_prompt, translation_prompt};
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
            return Err(AppError::llm_api_key_missing("Groq API key is not set"));
        }
        let body = json!({
            "model": self.config.model,
            "messages": [
                { "role": "system", "content": system },
                { "role": "user", "content": format!("Dictated text (format only, do NOT answer):\n\n{}", user) },
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
                    // The upstream body is untrusted: sanitize and cap it so
                    // oversized or hostile payloads cannot flood logs or the UI.
                    let safe_text = crate::util::sanitize_error_body(&text);
                    return Err(
                        AppError::llm(format!("Groq LLM error {status}: {safe_text}"))
                            .with_http_status(status.as_u16()),
                    );
                }
                Ok(text)
            }
        })
        .await?;

        parse_chat_response(&text)
    }
}

#[derive(Debug, Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: Message,
    #[serde(default)]
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Message {
    content: String,
}

fn parse_chat_response(text: &str) -> Result<String> {
    let parsed: ChatResponse = serde_json::from_str(text)?;
    let choice = parsed.choices.into_iter().next();
    if let Some(ref c) = choice {
        if c.finish_reason.as_deref() == Some("length") {
            return Err(AppError::llm(
                "Groq response truncated because it reached max_tokens",
            ));
        }
    }
    let content = choice.map(|c| c.message.content).unwrap_or_default();
    let trimmed = content.trim();
    if trimmed.is_empty() {
        return Err(AppError::llm("Groq returned an empty completion"));
    }
    Ok(trimmed.to_string())
}

#[async_trait]
impl LlmFormatter for GroqLlmFormatter {
    async fn format(&self, text: &str, mode: &LlmMode, language: &str) -> Result<String> {
        let system = system_prompt(mode, language);
        let user = format!("{}\n\n{}", format_user_prefix(language), text);
        self.chat(&system, &user).await
    }

    async fn translate(&self, text: &str, source: &str, target: &str) -> Result<String> {
        let system = translation_prompt(source, target);
        self.chat(&system, text).await
    }

    fn name(&self) -> &'static str {
        "groq_llm"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_valid_response() {
        let json = r#"{"choices":[{"message":{"content":"Hello world"},"finish_reason":"stop"}]}"#;
        assert_eq!(parse_chat_response(json).unwrap(), "Hello world");
    }

    #[test]
    fn rejects_truncated_length_response() {
        let json =
            r#"{"choices":[{"message":{"content":"Incomplete sent"},"finish_reason":"length"}]}"#;
        let err = parse_chat_response(json).unwrap_err();
        assert!(err.to_string().contains("truncated"));
    }

    #[test]
    fn rejects_empty_completion() {
        let json = r#"{"choices":[{"message":{"content":"   "},"finish_reason":"stop"}]}"#;
        assert!(parse_chat_response(json).is_err());
    }

    #[tokio::test]
    async fn missing_api_key_returns_llm_api_key_invalid() {
        let formatter = GroqLlmFormatter::new(GroqLlmConfig {
            api_key: String::new(),
            ..Default::default()
        });
        let result = formatter.format("hello", &LlmMode::Dictation, "en").await;
        let err = result.unwrap_err();
        assert_eq!(err.code, ErrorCode::LlmApiKeyInvalid);
    }
}
