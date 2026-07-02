//! Usage statistics command — returns aggregated usage data for the dashboard.

use serde::Serialize;
use tauri::State;

use crate::error::AppError;
use crate::storage::HistoryRepository;
use crate::AppStateInner;

/// Aggregate usage stats computed server-side.
#[derive(Debug, Clone, Default, Serialize)]
pub struct UsageStats {
    pub total_words: i64,
    pub total_duration_ms: i64,
    pub total_sessions: i64,
}

/// Return lifetime usage totals.
///
/// Aggregates directly over the full `transcriptions` table so the numbers are
/// accurate and uncapped. The earlier implementation summed the daily
/// `usage_stats` rollup, which is only populated when the opt-in `telemetry`
/// setting is enabled — leaving the dashboard at zero for most users. Sourcing
/// from history means the dashboard reflects real usage regardless of the
/// telemetry preference and is never limited by the history list's page size.
#[tauri::command]
pub fn get_usage_stats(
    state: State<'_, AppStateInner>,
) -> std::result::Result<UsageStats, AppError> {
    let totals = HistoryRepository::new(&state.db).totals()?;
    Ok(UsageStats {
        total_words: totals.total_words,
        total_duration_ms: totals.total_duration_ms,
        total_sessions: totals.total_sessions,
    })
}
