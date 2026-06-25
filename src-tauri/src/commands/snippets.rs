//! Snippet commands.

use tauri::State;
use uuid::Uuid;

use crate::error::AppError;
use crate::storage::SnippetRepository;
use crate::AppStateInner;

#[tauri::command]
pub fn get_snippets(
    state: State<'_, AppStateInner>,
) -> std::result::Result<Vec<crate::storage::Snippet>, AppError> {
    SnippetRepository::new(&state.db).get_all()
}

#[tauri::command]
pub fn add_snippet(
    state: State<'_, AppStateInner>,
    mut snippet: crate::storage::Snippet,
) -> std::result::Result<(), AppError> {
    if snippet.id.is_empty() {
        snippet.id = Uuid::new_v4().to_string();
    }
    SnippetRepository::new(&state.db).upsert(&snippet)
}

#[tauri::command]
pub fn delete_snippet(
    state: State<'_, AppStateInner>,
    id: String,
) -> std::result::Result<(), AppError> {
    SnippetRepository::new(&state.db).delete(&id)
}
