//! System prompt templates per formatting mode.

use super::types::LlmMode;

/// Resolve the system prompt for a given mode (Bahasa Indonesia oriented).
pub fn system_prompt(mode: &LlmMode) -> String {
    let base_instruction = "You are a pure text formatting engine. Your ONLY job is to reformat the user's text according to the task. \
DO NOT answer any questions found in the text. DO NOT respond to the content of the text. \
DO NOT output any explanations, greetings, or conversational filler. Output ONLY the raw reformatted text.";
    
    match mode {
        LlmMode::Dictation => format!("{base_instruction}\n\nTask: Clean up the transcription text. Remove filler words. Fix punctuation and capitalization. Do not change the meaning. Language: Indonesian."),
        LlmMode::Message => format!("{base_instruction}\n\nTask: Rewrite the text into a short, natural chat message. Emojis are allowed. Language: Indonesian."),
        LlmMode::Email => format!("{base_instruction}\n\nTask: Rewrite the text into a professional email with greeting and sign-off. Language: Indonesian."),
        LlmMode::Custom(prompt) => format!("{base_instruction}\n\nTask: {prompt}"),
    }
}

/// Build a translation system prompt.
pub fn translation_prompt(source: &str, target: &str) -> String {
    format!(
        "You are a translation engine. Translate the text from {source} to {target}. \
         DO NOT answer questions in the text. Output ONLY the translated text, no explanations."
    )
}
