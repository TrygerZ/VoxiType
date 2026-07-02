//! Tauri command handlers (IPC surface).
//!
//! Sub-modules own their command groups; `runtime` holds shared config
//! builders and the core recording pipeline (used by both commands and
//! the global hotkey callbacks).

mod dictionary;
mod history;
mod misc;
mod per_app;
mod recording;
pub mod runtime;
mod settings;
mod snippets;
mod stats;

pub use dictionary::*;
pub use history::*;
pub use misc::*;
pub use per_app::*;
pub use recording::*;
pub use settings::*;
pub use snippets::*;
pub use stats::*;
