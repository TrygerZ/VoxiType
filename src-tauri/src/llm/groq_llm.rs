//! Groq cloud LLM client (OpenAI-compatible chat completions).
//!
//! Uses non-streaming completions; the formatted output is short enough that
//! streaming adds little value here.

use std::time::Duration;

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::json;

use super::prompts::{
    format_user_prefix, system_prompt, translation_prompt, translation_user_prefix,
};
use super::types::{GroqLlmConfig, LlmMode};
use super::LlmFormatter;
use crate::error::{AppError, ErrorCode, Result};

const GROQ_CHAT_URL: &str = "https://api.groq.com/openai/v1/chat/completions";

/// Rough output-token budget per input word for the formatting/translation
/// prompt. Formatting rarely expands text, so this is a generous ceiling.
const TOKENS_PER_WORD: u32 = 3;
/// Slack tokens added on top of the word-scaled budget for punctuation and
/// short inputs.
const TOKENS_MARGIN: u32 = 200;
/// `llama-3.1-8b-instant` maximum output tokens.
const MAX_COMPLETION_TOKENS: u32 = 8192;
/// Per-request timeout for Groq chat completions, applied on the request
/// builder so it never depends on the shared client's defaults.
const GROQ_LLM_REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

/// Completion cap scaled with input length, bounded below by the configured
/// value and above by the model's output limit.
fn effective_max_tokens(word_count: u32, configured: u32) -> u32 {
    (word_count * TOKENS_PER_WORD + TOKENS_MARGIN)
        .clamp(configured.min(MAX_COMPLETION_TOKENS), MAX_COMPLETION_TOKENS)
}

pub struct GroqLlmFormatter {
    client: reqwest::Client,
    config: GroqLlmConfig,
}

impl GroqLlmFormatter {
    pub fn new(config: GroqLlmConfig) -> Result<Self> {
        Ok(Self {
            client: crate::util::http_client()?,
            config,
        })
    }

    async fn chat(&self, system: &str, user: &str, max_tokens: u32) -> Result<String> {
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
            "max_tokens": max_tokens,
            "stream": false,
        });

        // Retry transient failures with exponential backoff; auth fails fast.
        let text = crate::util::retry_with_backoff(3, std::time::Duration::from_secs(1), || {
            let body = body.clone();
            async move {
                let resp = self
                    .client
                    .post(GROQ_CHAT_URL)
                    .timeout(GROQ_LLM_REQUEST_TIMEOUT)
                    .bearer_auth(&self.config.api_key)
                    .json(&body)
                    .send()
                    .await?;

                let status = resp.status();
                // Capture the header before `text()` consumes the response.
                let retry_after = crate::util::retry_after_secs(resp.headers());
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
                    let mut err = AppError::llm(format!("Groq LLM error {status}: {safe_text}"))
                        .with_http_status(status.as_u16());
                    if let Some(secs) = retry_after {
                        err = err.with_retry_after_secs(secs);
                    }
                    return Err(err);
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

fn word_count(text: &str) -> u32 {
    text.split_whitespace().count() as u32
}

fn parse_chat_response(text: &str) -> Result<String> {
    let parsed: ChatResponse = serde_json::from_str(text)?;
    let choice = parsed.choices.into_iter().next();
    if let Some(ref c) = choice {
        if c.finish_reason.as_deref() == Some("length") {
            tracing::warn!(
                "Groq LLM response truncated at max_tokens; falling back to rule-based formatting"
            );
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
        let max_tokens = effective_max_tokens(word_count(text), self.config.max_tokens);
        self.chat(&system, &user, max_tokens).await
    }

    async fn translate(&self, text: &str, source: &str, target: &str) -> Result<String> {
        let system = translation_prompt(source, target);
        let user = format!("{}\n\n{}", translation_user_prefix(), text);
        let max_tokens = effective_max_tokens(word_count(text), self.config.max_tokens);
        self.chat(&system, &user, max_tokens).await
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

    #[test]
    fn effective_max_tokens_floors_at_configured_for_short_input() {
        assert_eq!(effective_max_tokens(0, 500), 500);
        assert_eq!(effective_max_tokens(10, 500), 500);
    }

    #[test]
    fn effective_max_tokens_scales_with_long_input() {
        // 1000 words * 3 + 200 = 3200, above the configured 500.
        assert_eq!(effective_max_tokens(1000, 500), 3200);
    }

    #[test]
    fn effective_max_tokens_caps_at_model_limit() {
        assert_eq!(effective_max_tokens(100_000, 500), MAX_COMPLETION_TOKENS);
    }

    #[test]
    fn effective_max_tokens_clamps_configured_above_model_limit() {
        // A configured value above the model limit must not raise the cap.
        assert_eq!(effective_max_tokens(0, 100_000), MAX_COMPLETION_TOKENS);
    }

    #[tokio::test]
    async fn missing_api_key_returns_llm_api_key_invalid() {
        let formatter = GroqLlmFormatter::new(GroqLlmConfig {
            api_key: String::new(),
            ..Default::default()
        })
        .unwrap();
        let result = formatter.format("hello", &LlmMode::Dictation, "en").await;
        let err = result.unwrap_err();
        assert_eq!(err.code, ErrorCode::LlmApiKeyInvalid);
    }
}
