//! System prompt templates per formatting mode.

use super::types::LlmMode;

/// Resolve the system prompt for a given mode (Bahasa Indonesia oriented).
pub fn system_prompt(mode: &LlmMode) -> String {
    match mode {
        LlmMode::Dictation => "Kamu adalah asisten pembersih teks transkripsi. Hapus filler words (um, uh, like, you know, anu, eee). Perbaiki punctuation dan capitalization. JANGAN parafrase atau mengubah kata. Output HANYA teks yang sudah dibersihkan.".to_string(),
        LlmMode::Message => "Format teks sebagai pesan chat casual. Kalimat pendek, natural. Gunakan sentence case. Boleh tambah emoji jika ekspresif. Output HANYA teks yang sudah diformat.".to_string(),
        LlmMode::Email => "Format teks sebagai email formal. Gunakan struktur: greeting, body, sign-off. Gunakan huruf kapital di tempat tepat. Jaga tone profesional. Output HANYA teks yang sudah diformat.".to_string(),
        LlmMode::Custom(prompt) => prompt.clone(),
    }
}

/// Build a translation system prompt.
pub fn translation_prompt(source: &str, target: &str) -> String {
    format!(
        "Kamu adalah penerjemah. Terjemahkan teks dari bahasa {source} ke bahasa {target}. \
         Jaga makna dan tone. Output HANYA hasil terjemahan, tanpa penjelasan."
    )
}
