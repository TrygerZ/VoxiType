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
pub fn re_inject(
    window: tauri::Window,
    state: State<'_, AppStateInner>,
    id: String,
) -> std::result::Result<(), AppError> {
    let _ = window.minimize();
    std::thread::sleep(std::time::Duration::from_millis(150));
    let entry = HistoryRepository::new(&state.db)
        .get(&id)?
        .ok_or_else(|| AppError::storage("History item not found"))?;
    HybridInjector::new().inject(&entry.text_formatted)?;
    Ok(())
}

#[tauri::command]
pub fn export_history(
    state: State<'_, AppStateInner>,
    format: String,
) -> std::result::Result<String, AppError> {
    let items = HistoryRepository::new(&state.db).list(&HistoryFilter {
        limit: Some(10_000),
        ..Default::default()
    })?;
    match format.as_str() {
        "csv" => {
            let mut out = String::from("created_at,mode,source_lang,word_count,text_formatted\n");
            for it in &items {
                out.push_str(&format!(
                    "\"{}\",\"{}\",\"{}\",{},\"{}\"\n",
                    runtime::csv_escape(&it.created_at),
                    runtime::csv_escape(&it.mode),
                    runtime::csv_escape(&it.source_lang),
                    it.word_count,
                    runtime::csv_escape(&it.text_formatted),
                ));
            }
            Ok(out)
        }
        _ => serde_json::to_string_pretty(&items).map_err(AppError::from),
    }
}
