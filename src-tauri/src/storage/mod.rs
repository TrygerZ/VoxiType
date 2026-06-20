//! SQLite-backed persistence: history, dictionary, settings.
//!
//! The whole module owns the only SQLite connection. Other layers never touch
//! SQL directly (see architecture rules in AGENTS.md).

pub mod db;
pub mod dictionary;
pub mod history;
pub mod settings;

pub use db::Database;
pub use dictionary::{DictFilter, DictionaryEntry, DictionaryRepository};
pub use history::{HistoryFilter, HistoryRepository, TranscriptionEntry};
pub use settings::SettingsManager;
