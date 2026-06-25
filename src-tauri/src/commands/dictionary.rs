//! Dictionary commands.

use tauri::State;
use uuid::Uuid;

use crate::error::AppError;
use crate::storage::{DictFilter, DictionaryEntry, DictionaryRepository};
use crate::AppStateInner;

#[tauri::command]
pub fn get_dictionary(
    state: State<'_, AppStateInner>,
    filter: Option<DictFilter>,
) -> std::result::Result<Vec<DictionaryEntry>, AppError> {
    DictionaryRepository::new(&state.db).get_all(&filter.unwrap_or_default())
}

#[tauri::command]
pub fn add_dictionary_word(
    state: State<'_, AppStateInner>,
    mut entry: DictionaryEntry,
) -> std::result::Result<(), AppError> {
    if entry.id.is_empty() {
        entry.id = Uuid::new_v4().to_string();
    }
    DictionaryRepository::new(&state.db).upsert(&entry)
}

#[tauri::command]
pub fn update_dictionary_word(
    state: State<'_, AppStateInner>,
    mut entry: DictionaryEntry,
) -> std::result::Result<(), AppError> {
    if entry.id.is_empty() {
        entry.id = Uuid::new_v4().to_string();
    }
    DictionaryRepository::new(&state.db).upsert(&entry)
}

#[tauri::command]
pub fn set_dictionary_active(
    state: State<'_, AppStateInner>,
    id: String,
    active: bool,
) -> std::result::Result<(), AppError> {
    DictionaryRepository::new(&state.db).set_active(&id, active)
}

#[tauri::command]
pub fn delete_dictionary_word(
    state: State<'_, AppStateInner>,
    id: String,
) -> std::result::Result<(), AppError> {
    DictionaryRepository::new(&state.db).delete(&id)
}

#[tauri::command]
pub fn export_dictionary(state: State<'_, AppStateInner>) -> std::result::Result<String, AppError> {
    let entries = DictionaryRepository::new(&state.db).get_all(&DictFilter::default())?;
    serde_json::to_string_pretty(&entries).map_err(AppError::from)
}

#[tauri::command]
pub fn import_dictionary(
    state: State<'_, AppStateInner>,
    data: String,
) -> std::result::Result<u32, AppError> {
    let mut entries: Vec<DictionaryEntry> = serde_json::from_str(&data)?;
    let repo = DictionaryRepository::new(&state.db);
    let mut count = 0u32;
    for entry in &mut entries {
        if entry.id.is_empty() {
            entry.id = Uuid::new_v4().to_string();
        }
        repo.upsert(entry)?;
        count += 1;
    }
    Ok(count)
}
