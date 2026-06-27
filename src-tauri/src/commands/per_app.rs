//! Per-app mode commands.

use tauri::State;

use crate::error::AppError;
use crate::storage::PerAppModeRepository;
use crate::AppStateInner;

#[tauri::command]
pub fn get_per_app_modes(
    state: State<'_, AppStateInner>,
) -> std::result::Result<Vec<crate::storage::PerAppMode>, AppError> {
    PerAppModeRepository::new(&state.db).get_all()
}

#[tauri::command]
pub fn set_per_app_mode(
    state: State<'_, AppStateInner>,
    mapping: crate::storage::PerAppMode,
) -> std::result::Result<(), AppError> {
    PerAppModeRepository::new(&state.db).upsert(&mapping)
}

#[tauri::command]
pub fn delete_per_app_mode(
    state: State<'_, AppStateInner>,
    id: i64,
) -> std::result::Result<(), AppError> {
    PerAppModeRepository::new(&state.db).delete(id)
}

#[tauri::command]
pub fn get_active_app() -> Option<String> {
    crate::active_window::foreground_process_name()
}
