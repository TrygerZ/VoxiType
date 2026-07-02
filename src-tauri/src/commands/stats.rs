//! Usage statistics command — returns aggregated usage data for the dashboard.

use serde::Serialize;
use tauri::State;

use crate::error::AppError;
use crate::storage::StatsRepository;
use crate::AppStateInner;

/// Aggregate usage stats computed server-side.
#[derive(Debug, Clone, Default, Serialize)]
pub struct UsageStats {
    pub total_words: i64,
    pub total_duration_ms: i64,
    pub total_sessions: i64,
}

#[tauri::command]
pub fn get_usage_stats(
    state: State<'_, AppStateInner>,
) -> std::result::Result<UsageStats, AppError> {
    let recent = StatsRepository::new(&state.db).recent(365)?;
    let mut stats = UsageStats::default();
    for day in &recent {
        stats.total_words += day.total_words;
        stats.total_duration_ms += day.total_duration_ms;
        stats.total_sessions += day.transcription_count;
    }
    Ok(stats)
}
