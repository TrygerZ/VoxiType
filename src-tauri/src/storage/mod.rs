//! SQLite-backed persistence: history, dictionary, settings.
//!
//! The whole module owns the only SQLite connection. Other layers never touch
//! SQL directly (see architecture rules in AGENTS.md).

pub mod db;
pub mod dictionary;
pub mod history;
pub mod per_app_modes;
pub mod settings;
pub mod snippets;
pub mod stats;

pub use db::Database;
pub use dictionary::{apply_replacements, DictFilter, DictionaryEntry, DictionaryRepository};
pub use history::{HistoryFilter, HistoryRepository, TranscriptionEntry};
pub use per_app_modes::{PerAppMode, PerAppModeRepository};
pub use settings::SettingsManager;
pub use snippets::{expand_snippets, Snippet, SnippetRepository};
pub use stats::{DailyStats, EngineKind, StatsRepository};
