//! Tauri command handlers (IPC surface) and hotkey-driven flows.

use std::sync::Arc;

use serde_json::Value;
use tauri::{AppHandle, Manager, Runtime, State};
use uuid::Uuid;

use crate::audio::{AudioConfig, DeviceInfo};
use crate::error::{AppError, Result};
use crate::injection::{HybridInjector, TextInjector};
use crate::llm::{
    GroqLlmConfig, LlmEngineKind, LlmFactory, LlmMode, OllamaConfig, RuleBasedConfig,
};
use crate::pipeline::{batch, PipelineOrchestrator};
use crate::storage::{
    DictFilter, Database, DictionaryEntry, DictionaryRepository, HistoryFilter, HistoryRepository,
    SettingsManager, TranscriptionEntry,
};
use crate::stt::{GroqSttConfig, SttConfig, SttEngineKind, SttFactory, WhisperCppConfig};
use crate::{events, AppStateInner};

// ============================================================
// Settings-derived runtime config
// ============================================================

fn string_setting(db: &Database, key: &str, default: &str) -> String {
    SettingsManager::new(db)
        .get::<String>(key)
        .ok()
        .flatten()
        .unwrap_or_else(|| default.to_string())
}

fn build_audio_config(db: &Database) -> AudioConfig {
    AudioConfig {
        mic_device: string_setting(db, "mic_device", "default"),
        ..Default::default()
    }
}

fn build_stt(db: &Database) -> Result<Arc<dyn crate::stt::SttEngine>> {
    let engine = string_setting(db, "stt_engine", "whisper_cpp");
    let kind = match engine.as_str() {
        "groq" => SttEngineKind::Groq,
        _ => SttEngineKind::WhisperCpp,
    };
    let whisper = WhisperCppConfig {
        model: string_setting(db, "stt_model", "small"),
        ..Default::default()
    };
    let groq = GroqSttConfig {
        api_key: string_setting(db, "groq_api_key", ""),
        language: string_setting(db, "language", "id"),
        ..Default::default()
    };
    SttFactory::create(kind, whisper, groq)
}

fn build_llm(db: &Database) -> Arc<dyn crate::llm::LlmFormatter> {
    let engine = string_setting(db, "llm_engine", "ollama");
    let kind = match engine.as_str() {
        "off" => LlmEngineKind::Off,
        "groq" => LlmEngineKind::Groq,
        "rule_based" => LlmEngineKind::RuleBased,
        _ => LlmEngineKind::Ollama,
    };
    let ollama = OllamaConfig {
        model: string_setting(db, "llm_model", "qwen2.5:3b"),
        ..Default::default()
    };
    let groq = GroqLlmConfig {
        api_key: string_setting(db, "groq_api_key", ""),
        ..Default::default()
    };
    LlmFactory::create(kind, ollama, groq, RuleBasedConfig::default())
}

fn build_stt_config(db: &Database) -> SttConfig {
    let language = string_setting(db, "language", "auto");
    let hotwords = DictionaryRepository::new(db)
        .get_hotwords()
        .unwrap_or_default();
    let initial_prompt = if hotwords.is_empty() {
        None
    } else {
        Some(hotwords.join(", "))
    };
    SttConfig {
        language,
        initial_prompt,
        temperature: 0.0,
    }
}

fn active_mode(db: &Database) -> LlmMode {
    LlmMode::from_id(&string_setting(db, "active_mode", "dictation"))
}

// ============================================================
// Core recording flow (shared by commands and hotkey)
// ============================================================

/// Process the captured audio through STT -> LLM -> injection, persist, emit.
async fn process_audio<R: Runtime>(app: AppHandle<R>, audio: Vec<f32>) {
    let state = app.state::<AppStateInner>();

    if audio.is_empty() {
        let _ = state.pipeline.finish_processing();
        events::emit_state(&app, state.pipeline.state_tag());
        return;
    }

    let stt = match build_stt(&state.db) {
        Ok(e) => e,
        Err(e) => return fail(&app, &state.pipeline, &e),
    };
    let stt_config = build_stt_config(&state.db);
    let llm = build_llm(&state.db);
    let mode = active_mode(&state.db);
    let injector = HybridInjector::default();

    let stt_name = stt.name().to_string();
    let llm_name = llm.name().to_string();

    let outcome = batch::run_batch(&audio, stt, &stt_config, llm, &mode, &injector).await;

    match outcome {
        Ok(out) => {
            let id = Uuid::new_v4().to_string();
            let word_count = out.formatted_text.split_whitespace().count() as u32;
            let entry = TranscriptionEntry {
                id: id.clone(),
                created_at: String::new(),
                text_raw: out.transcription.text.clone(),
                text_formatted: out.formatted_text.clone(),
                source_lang: out.transcription.language.clone(),
                target_lang: None,
                mode: string_setting(&state.db, "active_mode", "dictation"),
                stt_engine: stt_name,
                stt_confidence: Some(out.transcription.confidence),
                llm_engine: Some(llm_name),
                duration_ms: Some(out.transcription.duration_ms as i64),
                word_count: word_count as i64,
                character_count: out.formatted_text.chars().count() as i64,
                is_pinned: false,
                app_context: None,
            };
            let _ = HistoryRepository::new(&state.db).insert(&entry);

            let _ = state.pipeline.finish_processing();
            events::emit_transcription_complete(&app, &id, &out.formatted_text, word_count);
            events::emit_state(&app, state.pipeline.state_tag());
        }
        Err(e) => fail(&app, &state.pipeline, &e),
    }
}

fn fail<R: Runtime>(app: &AppHandle<R>, pipeline: &PipelineOrchestrator, e: &AppError) {
    tracing::error!("Pipeline error: {e}");
    pipeline.set_error(e);
    events::emit_transcription_error(app, &e.message, &format!("{:?}", e.code));
    events::emit_state(app, pipeline.state_tag());
}

// ============================================================
// Hotkey entry points (called from the shortcut callback)
// ============================================================

pub fn hotkey_start<R: Runtime>(app: &AppHandle<R>) {
    let state = app.state::<AppStateInner>();
    if state.pipeline.state_tag() != crate::pipeline::AppStateTag::Idle {
        return;
    }
    let config = build_audio_config(&state.db);
    match state.pipeline.start_recording(&config) {
        Ok(tag) => events::emit_state(app, tag),
        Err(e) => fail(app, &state.pipeline, &e),
    }
}

pub fn hotkey_stop<R: Runtime>(app: &AppHandle<R>) {
    let state = app.state::<AppStateInner>();
    if state.pipeline.state_tag() != crate::pipeline::AppStateTag::Recording {
        return;
    }
    match state.pipeline.stop_recording() {
        Ok(audio) => {
            events::emit_state(app, state.pipeline.state_tag());
            let app2 = app.clone();
            tauri::async_runtime::spawn(async move {
                process_audio(app2, audio).await;
            });
        }
        Err(e) => fail(app, &state.pipeline, &e),
    }
}

pub fn hotkey_toggle<R: Runtime>(app: &AppHandle<R>) {
    let state = app.state::<AppStateInner>();
    match state.pipeline.state_tag() {
        crate::pipeline::AppStateTag::Idle => hotkey_start(app),
        crate::pipeline::AppStateTag::Recording => hotkey_stop(app),
        _ => {}
    }
}

// ============================================================
// Tauri commands
// ============================================================

#[tauri::command]
pub async fn start_recording<R: Runtime>(app: AppHandle<R>) -> std::result::Result<(), AppError> {
    hotkey_start(&app);
    Ok(())
}

#[tauri::command]
pub async fn stop_recording<R: Runtime>(app: AppHandle<R>) -> std::result::Result<(), AppError> {
    hotkey_stop(&app);
    Ok(())
}

#[tauri::command]
pub async fn cancel_recording<R: Runtime>(app: AppHandle<R>) -> std::result::Result<(), AppError> {
    let state = app.state::<AppStateInner>();
    state.pipeline.cancel_recording()?;
    events::emit_state(&app, state.pipeline.state_tag());
    Ok(())
}

#[tauri::command]
pub fn get_state(state: State<'_, AppStateInner>) -> String {
    format!("{:?}", state.pipeline.state_tag())
}

#[tauri::command]
pub fn get_audio_level(state: State<'_, AppStateInner>) -> f32 {
    state.pipeline.audio_level()
}

// --- Settings ---

#[tauri::command]
pub fn get_settings(state: State<'_, AppStateInner>) -> std::result::Result<Value, AppError> {
    SettingsManager::new(&state.db).all()
}

#[tauri::command]
pub fn update_setting(
    state: State<'_, AppStateInner>,
    key: String,
    value: Value,
) -> std::result::Result<(), AppError> {
    let encoded = serde_json::to_string(&value)?;
    SettingsManager::new(&state.db).set_raw(&key, &encoded)
}

// --- History ---

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
pub fn re_inject(
    state: State<'_, AppStateInner>,
    id: String,
) -> std::result::Result<(), AppError> {
    let entry = HistoryRepository::new(&state.db)
        .get(&id)?
        .ok_or_else(|| AppError::storage("History item not found"))?;
    HybridInjector::default().inject(&entry.text_formatted)?;
    Ok(())
}

// --- Dictionary ---

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
pub fn delete_dictionary_word(
    state: State<'_, AppStateInner>,
    id: String,
) -> std::result::Result<(), AppError> {
    DictionaryRepository::new(&state.db).delete(&id)
}

// --- Microphones ---

#[tauri::command]
pub fn get_microphones() -> std::result::Result<Vec<DeviceInfo>, AppError> {
    crate::audio::device::list_input_devices()
}

// --- App info ---

#[tauri::command]
pub fn get_app_info<R: Runtime>(app: AppHandle<R>) -> Value {
    serde_json::json!({
        "name": "VoxiType",
        "version": app.package_info().version.to_string(),
        "tauri": "2",
    })
}
