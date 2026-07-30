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

/// Max length of a snippet trigger phrase / content. Caps CPU on every
/// transcription's snippet-expansion pass and rejects unbounded IPC input.
const MAX_TRIGGER_LEN: usize = 100;
const MAX_CONTENT_LEN: usize = 4096;

#[tauri::command]
pub fn add_snippet(
    state: State<'_, AppStateInner>,
    mut snippet: crate::storage::Snippet,
) -> std::result::Result<(), AppError> {
    if snippet.trigger_phrase.len() > MAX_TRIGGER_LEN {
        return Err(AppError::internal(format!(
            "snippet trigger exceeds {MAX_TRIGGER_LEN} chars"
        )));
    }
    if snippet.content.len() > MAX_CONTENT_LEN {
        return Err(AppError::internal(format!(
            "snippet content exceeds {MAX_CONTENT_LEN} chars"
        )));
    }
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
