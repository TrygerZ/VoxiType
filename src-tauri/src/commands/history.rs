//! History commands.

use tauri::State;

use crate::error::AppError;
use crate::injection::{HybridInjector, TextInjector};
use crate::storage::{HistoryFilter, HistoryRepository, TranscriptionEntry};
use crate::AppStateInner;

use super::runtime;

#[tauri::command]
pub fn get_history(
    state: State<'_, AppStateInner>,
    filter: Option<HistoryFilter>,
) -> std::result::Result<Vec<TranscriptionEntry>, AppError> {
    HistoryRepository::new(&state.db).list(&filter.unwrap_or_default())
}

#[tauri::command]
pub fn search_history(
    state: State<'_, AppStateInner>,
    query: String,
) -> std::result::Result<Vec<TranscriptionEntry>, AppError> {
    HistoryRepository::new(&state.db).search(&query)
}

#[tauri::command]
pub fn delete_history(
    state: State<'_, AppStateInner>,
    id: String,
) -> std::result::Result<(), AppError> {
    HistoryRepository::new(&state.db).delete(&id)
}

#[tauri::command]
pub fn pin_history(
    state: State<'_, AppStateInner>,
    id: String,
    pinned: bool,
) -> std::result::Result<(), AppError> {
    HistoryRepository::new(&state.db).set_pinned(&id, pinned)
}

#[tauri::command]
pub fn clear_history(
    state: State<'_, AppStateInner>,
    keep_pinned: Option<bool>,
) -> std::result::Result<usize, AppError> {
    HistoryRepository::new(&state.db).clear(keep_pinned.unwrap_or(true))
}

#[tauri::command]
pub async fn re_inject(
    window: tauri::Window,
    state: State<'_, AppStateInner>,
    id: String,
) -> std::result::Result<(), AppError> {
    let _ = window.minimize();
    tokio::time::sleep(std::time::Duration::from_millis(150)).await;
    let entry = HistoryRepository::new(&state.db)
        .get(&id)?
        .ok_or_else(|| AppError::storage("History item not found"))?;
    // Injection blocks the OS event loop for hundreds of milliseconds
    // (clipboard propagation sleeps + enigo keystroke simulation), so it
    // must never run directly on a tokio worker thread.
    let text = entry.text_formatted;
    tokio::task::spawn_blocking(move || HybridInjector::new().inject(&text))
        .await
        .map_err(|e| AppError::injection(format!("Re-injection task failed: {e}")))??;
    Ok(())
}

pub const MAX_EXPORT_RECORDS: usize = 10_000;

pub fn validate_export_count(total: usize) -> std::result::Result<(), AppError> {
    if total > MAX_EXPORT_RECORDS {
        return Err(AppError::storage(format!(
            "export exceeds maximum of {MAX_EXPORT_RECORDS} records"
        )));
    }
    Ok(())
}

fn format_csv_export(items: &[TranscriptionEntry]) -> String {
    let mut out = String::from("created_at,mode,source_lang,word_count,text_formatted\n");
    for it in items {
        out.push_str(&format!(
            "\"{}\",\"{}\",\"{}\",{},\"{}\"\n",
            runtime::csv_escape(&it.created_at),
            runtime::csv_escape(&it.mode),
            runtime::csv_escape(&it.source_lang),
            it.word_count,
            runtime::csv_escape(&it.text_formatted),
        ));
    }
    out
}

#[tauri::command]
pub fn export_history(
    state: State<'_, AppStateInner>,
    format: String,
) -> std::result::Result<String, AppError> {
    let repo = HistoryRepository::new(&state.db);
    let total = repo.count()? as usize;
    validate_export_count(total)?;
    let items = repo.list(&HistoryFilter {
        limit: Some(MAX_EXPORT_RECORDS as u32),
        ..Default::default()
    })?;
    match format.as_str() {
        "csv" => Ok(format_csv_export(&items)),
        _ => serde_json::to_string_pretty(&items).map_err(AppError::from),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_export_count_permits_under_and_at_cap() {
        assert!(validate_export_count(0).is_ok());
        assert!(validate_export_count(MAX_EXPORT_RECORDS).is_ok());
    }

    #[test]
    fn validate_export_count_rejects_over_cap() {
        let err = validate_export_count(MAX_EXPORT_RECORDS + 1).unwrap_err();
        assert!(err.message.contains("exceeds maximum of 10000 records"));
    }
}
