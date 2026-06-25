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
    Database, DictFilter, DictionaryEntry, DictionaryRepository, HistoryFilter, HistoryRepository,
    SettingsManager, TranscriptionEntry,
};
use crate::stt::{GroqSttConfig, SttConfig, SttEngineKind, SttFactory, WhisperCppConfig};
use crate::{events, AppStateInner};
use crate::util::MutexExt;

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

fn decrypted_api_key(state: &AppStateInner) -> String {
    let raw = string_setting(&state.db, "groq_api_key", "");
    crate::crypto::decrypt_api_key(&raw, &state.master_key).unwrap_or(raw)
}

fn build_stt(state: &AppStateInner) -> Result<Arc<dyn crate::stt::SttEngine>> {
    let engine = string_setting(&state.db, "stt_engine", "whisper_cpp");
    let kind = match engine.as_str() {
        "groq" => SttEngineKind::Groq,
        _ => SttEngineKind::WhisperCpp,
    };

    let model = string_setting(&state.db, "stt_model", "small");
    let api_key = decrypted_api_key(state);

    let cache_key = match kind {
        SttEngineKind::WhisperCpp => model.clone(),
        SttEngineKind::Groq => api_key.clone(),
    };

    // Check if the cached engine in AppStateInner matches the current config
    let mut cache = state.stt_engine.lock_recover();
    if let Some((cached_kind, cached_key, cached_engine)) = &*cache {
        if *cached_kind == kind && *cached_key == cache_key {
            return Ok(cached_engine.clone());
        }
    }

    let whisper = WhisperCppConfig {
        model: model.clone(),
        model_path: state
            .app_data_dir
            .join("models")
            .join(format!("ggml-{}.bin", model))
            .to_string_lossy()
            .into_owned(),
        ..Default::default()
    };
    let groq = GroqSttConfig {
        api_key,
        language: string_setting(&state.db, "stt_language", "auto"),
        ..Default::default()
    };

    let new_engine = SttFactory::create(kind, whisper, groq)?;
    *cache = Some((kind, cache_key, new_engine.clone()));
    Ok(new_engine)
}

fn build_llm(state: &AppStateInner) -> Arc<dyn crate::llm::LlmFormatter> {
    let engine = string_setting(&state.db, "llm_engine", "ollama");
    let kind = match engine.as_str() {
        "off" => LlmEngineKind::Off,
        "groq" => LlmEngineKind::Groq,
        "rule_based" => LlmEngineKind::RuleBased,
        _ => LlmEngineKind::Ollama,
    };
    let ollama = OllamaConfig {
        model: string_setting(&state.db, "llm_model", "qwen2.5:3b"),
        ..Default::default()
    };
    let groq = GroqLlmConfig {
        api_key: decrypted_api_key(state),
        ..Default::default()
    };
    LlmFactory::create(kind, ollama, groq, RuleBasedConfig::default())
}

fn build_stt_config(db: &Database) -> SttConfig {
    let language = string_setting(db, "stt_language", "auto");
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
    // If per-app mode is enabled and the focused app has a mapping, use it.
    let per_app_on = SettingsManager::new(db)
        .get::<bool>("per_app_mode")
        .ok()
        .flatten()
        .unwrap_or(false);
    if per_app_on {
        if let Some(proc) = crate::active_window::foreground_process_name() {
            if let Ok(Some(mode_id)) = crate::storage::PerAppModeRepository::new(db).mode_for(&proc)
            {
                return LlmMode::from_id(&mode_id);
            }
        }
    }
    LlmMode::from_id(&string_setting(db, "active_mode", "dictation"))
}

fn sound_cues_enabled(db: &Database) -> bool {
    SettingsManager::new(db)
        .get::<bool>("sound_cues")
        .ok()
        .flatten()
        .unwrap_or(false)
}

fn telemetry_enabled(db: &Database) -> bool {
    SettingsManager::new(db)
        .get::<bool>("telemetry")
        .ok()
        .flatten()
        .unwrap_or(false)
}

/// Escape a string for CSV: double any internal quotes.
fn csv_escape(s: &str) -> String {
    s.replace('"', "\"\"")
}

/// Classify an engine name as local or cloud for telemetry bucketing.
fn engine_kind(name: &str) -> crate::storage::EngineKind {
    match name {
        "groq_whisper" | "groq_llm" => crate::storage::EngineKind::Cloud,
        _ => crate::storage::EngineKind::Local,
    }
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
        crate::overlay::maybe_hide(&app);
        return;
    }

    let stt = match build_stt(&state) {
        Ok(e) => e,
        Err(e) => return fail(&app, &state.pipeline, &e),
    };
    let stt_config = build_stt_config(&state.db);

    // Command mode: transcribe, and if the phrase matches a known editing
    // command, execute the keystroke instead of formatting/injecting text.
    let command_mode = SettingsManager::new(&state.db)
        .get::<bool>("command_mode")
        .ok()
        .flatten()
        .unwrap_or(false);
    // In command mode we must transcribe up front to detect editing commands.
    // Keep the result so a non-command phrase doesn't get transcribed twice.
    let mut precomputed = None;
    if command_mode {
        match stt.transcribe(&audio, &stt_config).await {
            Ok(tr) => {
                if let Some(cmd) = crate::injection::VoiceCommand::from_phrase(&tr.text) {
                    if let Err(e) = crate::injection::command::execute(cmd) {
                        return fail(&app, &state.pipeline, &e);
                    }
                    let _ = state.pipeline.finish_processing();
                    events::emit_transcription_complete(
                        &app,
                        &Uuid::new_v4().to_string(),
                        &format!("[command] {}", tr.text.trim()),
                        0,
                    );
                    events::emit_state(&app, state.pipeline.state_tag());
                    hide_overlay_soon(app.clone());
                    return;
                }
                // Not a command: reuse this transcription for normal injection.
                precomputed = Some(tr);
            }
            Err(e) => return fail(&app, &state.pipeline, &e),
        }
    }

    let llm = build_llm(&state);
    let mode = active_mode(&state.db);
    let replacements = DictionaryRepository::new(&state.db)
        .get_replacements()
        .unwrap_or_default();
    let snippets = crate::storage::SnippetRepository::new(&state.db)
        .get_active_expansions()
        .unwrap_or_default();
    let post = batch::PostProcess {
        replacements,
        snippets,
    };
    let injector = HybridInjector::default();

    // Translation is opt-in via settings.
    let translate_enabled = SettingsManager::new(&state.db)
        .get::<bool>("translation_enabled")
        .ok()
        .flatten()
        .unwrap_or(false);
    let translate_target = string_setting(&state.db, "translation_target", "en");
    let translate_opts = if translate_enabled {
        Some(batch::TranslateOpts {
            target: translate_target.clone(),
        })
    } else {
        None
    };

    let stt_name = stt.name().to_string();
    let llm_name = llm.name().to_string();

    // The mode actually used for formatting (may be a per-app override), so it
    // is recorded in history instead of the global `active_mode` setting.
    let mode_id = mode.id();

    let outcome = match precomputed {
        Some(tr) => {
            batch::run_batch_with_transcription(
                tr,
                llm,
                &mode,
                &post,
                translate_opts.as_ref(),
                &injector,
            )
            .await
        }
        None => {
            batch::run_batch(
                &audio,
                stt,
                &stt_config,
                llm,
                &mode,
                &post,
                translate_opts.as_ref(),
                &injector,
            )
            .await
        }
    };

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
                target_lang: if translate_enabled {
                    Some(translate_target.clone())
                } else {
                    None
                },
                mode: mode_id,
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

            // Opt-in local telemetry.
            if telemetry_enabled(&state.db) {
                let stt_kind = engine_kind(&entry.stt_engine);
                let llm_kind = engine_kind(entry.llm_engine.as_deref().unwrap_or(""));
                let _ = crate::storage::StatsRepository::new(&state.db).record_transcription(
                    word_count as i64,
                    out.transcription.duration_ms as i64,
                    stt_kind,
                    llm_kind,
                );
            }

            let _ = state.pipeline.finish_processing();
            events::emit_transcription_complete(&app, &id, &out.formatted_text, word_count);
            events::emit_state(&app, state.pipeline.state_tag());
            hide_overlay_soon(app.clone());
        }
        Err(e) => {
            if telemetry_enabled(&state.db) {
                let _ = crate::storage::StatsRepository::new(&state.db).record_error();
            }
            fail(&app, &state.pipeline, &e)
        }
    }
}

fn fail<R: Runtime>(app: &AppHandle<R>, pipeline: &PipelineOrchestrator, e: &AppError) {
    tracing::error!("Pipeline error: {e}");
    pipeline.set_error(e);
    events::emit_transcription_error(app, &e.message, &format!("{:?}", e.code));
    events::emit_state(app, pipeline.state_tag());
    hide_overlay_soon(app.clone());
}

/// Hide the floating overlay after a short delay so the final state (done /
/// error) is briefly visible to the user.
fn hide_overlay_soon<R: Runtime>(app: AppHandle<R>) {
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(1800)).await;
        // Only hide if we're back to idle/error (not recording again).
        let tag = app.state::<AppStateInner>().pipeline.state_tag();
        if tag != crate::pipeline::AppStateTag::Recording
            && tag != crate::pipeline::AppStateTag::Processing
        {
            crate::overlay::maybe_hide(&app);
        }
    });
}

// ============================================================
// Hotkey entry points (called from the shortcut callback)
// ============================================================

pub fn hotkey_start<R: Runtime>(app: &AppHandle<R>) {
    let state = app.state::<AppStateInner>();
    let tag = state.pipeline.state_tag();
    if tag != crate::pipeline::AppStateTag::Idle && tag != crate::pipeline::AppStateTag::Error {
        return;
    }

    // Instantly transition state to Recording and notify UI/play tone
    let tag = match state
        .pipeline
        .apply(crate::pipeline::StateEvent::StartRecording)
    {
        Ok(t) => t,
        Err(e) => return fail(app, &state.pipeline, &e),
    };

    if sound_cues_enabled(&state.db) {
        crate::sound::play(crate::sound::Cue::Start);
    }
    crate::overlay::ensure_visible(app);
    events::emit_state(app, tag);
    spawn_level_emitter(app.clone());

    // Asynchronously initialize and start CPAL audio capture
    let app_clone = app.clone();
    tauri::async_runtime::spawn(async move {
        let state = app_clone.state::<AppStateInner>();
        let config = build_audio_config(&state.db);
        if let Err(e) = state.pipeline.start_capture(&config) {
            // Roll back state to Idle on capture failure and emit error
            let _ = state
                .pipeline
                .apply(crate::pipeline::StateEvent::CancelRecording);
            fail(&app_clone, &state.pipeline, &e);
        }
    });
}

/// Emit `audio_level` events ~20x/sec while recording so the UI waveform reacts.
/// The task self-terminates as soon as the pipeline leaves the Recording state.
fn spawn_level_emitter<R: Runtime>(app: AppHandle<R>) {
    tauri::async_runtime::spawn(async move {
        loop {
            {
                let state = app.state::<AppStateInner>();
                if state.pipeline.state_tag() != crate::pipeline::AppStateTag::Recording {
                    break;
                }
                let level = state.pipeline.audio_level();
                events::emit_audio_level(&app, level);
            }
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
    });
}

pub fn hotkey_stop<R: Runtime>(app: &AppHandle<R>) {
    let state = app.state::<AppStateInner>();
    if state.pipeline.state_tag() != crate::pipeline::AppStateTag::Recording {
        return;
    }

    // Skip transcription if hotkey is pressed for less than or equal to 1 second
    if let Some(duration) = state.pipeline.recording_duration() {
        if duration.as_secs_f32() <= 1.0 {
            let _ = state.pipeline.cancel_recording();
            events::emit_state(app, state.pipeline.state_tag());
            crate::overlay::maybe_hide(app);
            return;
        }
    }

    match state.pipeline.stop_recording() {
        Ok(audio) => {
            if sound_cues_enabled(&state.db) {
                crate::sound::play(crate::sound::Cue::Stop);
            }
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
        crate::pipeline::AppStateTag::Idle | crate::pipeline::AppStateTag::Error => {
            hotkey_start(app)
        }
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
    crate::overlay::maybe_hide(&app);
    Ok(())
}

#[tauri::command]
pub async fn cancel_processing<R: Runtime>(app: AppHandle<R>) -> std::result::Result<(), AppError> {
    let state = app.state::<AppStateInner>();
    state.pipeline.cancel_processing()?;
    events::emit_state(&app, state.pipeline.state_tag());
    crate::overlay::maybe_hide(&app);
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
    let mut all = SettingsManager::new(&state.db).all()?;
    // Never leak secrets to the frontend. Replace the stored (encrypted) API
    // key with a boolean flag indicating whether one is configured.
    if let Value::Object(map) = &mut all {
        let has_key = map
            .get("groq_api_key")
            .and_then(|v| v.as_str())
            .map(|s| !s.is_empty())
            .unwrap_or(false);
        map.insert("groq_api_key".to_string(), Value::String(String::new()));
        map.insert("groq_api_key_set".to_string(), Value::Bool(has_key));
    }
    Ok(all)
}

#[tauri::command]
pub fn update_setting(
    state: State<'_, AppStateInner>,
    key: String,
    value: Value,
) -> std::result::Result<(), AppError> {
    let encoded = if key == "groq_api_key" {
        let plain = value.as_str().unwrap_or_default();
        let enc = crate::crypto::encrypt_api_key(plain, &state.master_key)?;
        serde_json::to_string(&enc)?
    } else {
        serde_json::to_string(&value)?
    };
    SettingsManager::new(&state.db).set_raw(&key, &encoded)
}

/// Enable or disable the floating widget and immediately show/hide it.
/// Persists the `floating_widget` setting so the choice survives restarts.
#[tauri::command]
pub fn set_floating_widget_enabled<R: Runtime>(
    app: AppHandle<R>,
    enabled: bool,
) -> std::result::Result<(), AppError> {
    let state = app.state::<AppStateInner>();
    SettingsManager::new(&state.db).set("floating_widget", &enabled)?;
    crate::overlay::apply_enabled(&app, enabled);
    Ok(())
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

/// Clear all history. When `keep_pinned` is true (default), pinned entries are
/// preserved. Returns the number of removed entries.
#[tauri::command]
pub fn clear_history(
    state: State<'_, AppStateInner>,
    keep_pinned: Option<bool>,
) -> std::result::Result<usize, AppError> {
    HistoryRepository::new(&state.db).clear(keep_pinned.unwrap_or(true))
}

#[tauri::command]
pub fn re_inject(state: State<'_, AppStateInner>, id: String) -> std::result::Result<(), AppError> {
    let entry = HistoryRepository::new(&state.db)
        .get(&id)?
        .ok_or_else(|| AppError::storage("History item not found"))?;
    HybridInjector::default().inject(&entry.text_formatted)?;
    Ok(())
}

/// Export history as JSON or CSV.
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
                    csv_escape(&it.created_at),
                    csv_escape(&it.mode),
                    csv_escape(&it.source_lang),
                    it.word_count,
                    csv_escape(&it.text_formatted),
                ));
            }
            Ok(out)
        }
        _ => serde_json::to_string_pretty(&items).map_err(AppError::from),
    }
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

/// Export the entire dictionary as a pretty JSON string.
#[tauri::command]
pub fn export_dictionary(state: State<'_, AppStateInner>) -> std::result::Result<String, AppError> {
    let entries = DictionaryRepository::new(&state.db).get_all(&DictFilter::default())?;
    serde_json::to_string_pretty(&entries).map_err(AppError::from)
}

/// Import dictionary entries from a JSON array string (upsert each).
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

// --- Snippets ---

#[tauri::command]
pub fn get_snippets(
    state: State<'_, AppStateInner>,
) -> std::result::Result<Vec<crate::storage::Snippet>, AppError> {
    crate::storage::SnippetRepository::new(&state.db).get_all()
}

#[tauri::command]
pub fn add_snippet(
    state: State<'_, AppStateInner>,
    mut snippet: crate::storage::Snippet,
) -> std::result::Result<(), AppError> {
    if snippet.id.is_empty() {
        snippet.id = Uuid::new_v4().to_string();
    }
    crate::storage::SnippetRepository::new(&state.db).upsert(&snippet)
}

#[tauri::command]
pub fn delete_snippet(
    state: State<'_, AppStateInner>,
    id: String,
) -> std::result::Result<(), AppError> {
    crate::storage::SnippetRepository::new(&state.db).delete(&id)
}

// --- Usage stats (local telemetry) ---

#[tauri::command]
pub fn get_usage_stats(
    state: State<'_, AppStateInner>,
    days: Option<u32>,
) -> std::result::Result<Vec<crate::storage::DailyStats>, AppError> {
    crate::storage::StatsRepository::new(&state.db).recent(days.unwrap_or(30))
}

// --- Per-app modes ---

#[tauri::command]
pub fn get_per_app_modes(
    state: State<'_, AppStateInner>,
) -> std::result::Result<Vec<crate::storage::PerAppMode>, AppError> {
    crate::storage::PerAppModeRepository::new(&state.db).get_all()
}

#[tauri::command]
pub fn set_per_app_mode(
    state: State<'_, AppStateInner>,
    mapping: crate::storage::PerAppMode,
) -> std::result::Result<(), AppError> {
    crate::storage::PerAppModeRepository::new(&state.db).upsert(&mapping)
}

#[tauri::command]
pub fn delete_per_app_mode(
    state: State<'_, AppStateInner>,
    id: i64,
) -> std::result::Result<(), AppError> {
    crate::storage::PerAppModeRepository::new(&state.db).delete(id)
}

/// Return the currently focused application's process name (for UI mapping).
#[tauri::command]
pub fn get_active_app() -> Option<String> {
    crate::active_window::foreground_process_name()
}

// --- Translation ---

/// Translate arbitrary text from `source` to `target` using the active LLM.
#[tauri::command]
pub async fn translate<R: Runtime>(
    app: AppHandle<R>,
    text: String,
    source: String,
    target: String,
) -> std::result::Result<String, AppError> {
    let llm = {
        let state = app.state::<AppStateInner>();
        build_llm(&state)
    };
    llm.translate(&text, &source, &target).await
}

// --- Microphones ---

#[tauri::command]
pub fn get_microphones() -> std::result::Result<Vec<DeviceInfo>, AppError> {
    crate::audio::device::list_input_devices()
}

// --- Hotkey ---

/// Persist a new hotkey config and re-register the global shortcut.
#[tauri::command]
pub fn set_hotkey<R: Runtime>(
    app: AppHandle<R>,
    key: String,
    mode: String,
) -> std::result::Result<(), AppError> {
    let hk_mode = match mode.as_str() {
        "toggle" => crate::hotkey::HotkeyMode::Toggle,
        _ => crate::hotkey::HotkeyMode::Ptt,
    };
    let cfg = crate::hotkey::HotkeyConfig { key, mode: hk_mode };

    {
        let state = app.state::<AppStateInner>();
        SettingsManager::new(&state.db).set("hotkey", &cfg)?;
    }
    crate::hotkey::rebind(&app, &cfg)
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

/// Check GitHub releases for a newer version.
#[tauri::command]
pub async fn check_updates<R: Runtime>(
    app: AppHandle<R>,
) -> std::result::Result<crate::updater::UpdateInfo, AppError> {
    let current = app.package_info().version.to_string();
    crate::updater::check(&current).await
}
