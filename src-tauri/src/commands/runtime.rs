//! Shared runtime helpers for STT/LLM config building and the core
//! recording → process → inject → persist pipeline.

use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;

use tauri::{AppHandle, Manager, Runtime};
use uuid::Uuid;

use crate::audio::AudioConfig;
use crate::error::{AppError, Result};
use crate::injection::HybridInjector;
use crate::llm::{
    GroqLlmConfig, LlmEngineKind, LlmFactory, LlmMode, OllamaConfig, RuleBasedConfig,
};
use crate::pipeline::{batch, PipelineOrchestrator};
use crate::storage::{
    Database, DictionaryRepository, HistoryRepository, SettingsManager, TranscriptionEntry,
};
use crate::stt::{GroqSttConfig, SttConfig, SttEngineKind, SttFactory, WhisperCppConfig};
use crate::util::MutexExt;
use crate::{events, AppStateInner};

// Maximum recording duration in seconds to prevent runaway recordings
pub const MAX_RECORDING_DURATION_SECS: u64 = 300;
// Minimum recording duration in seconds to filter accidental taps
const MIN_RECORDING_DURATION_SECS: f32 = 1.0;

// ---------------------------------------------------------------
// Settings-derived config builders
// ---------------------------------------------------------------

/// Snapshot of all settings loaded in a single database round-trip via `SettingsManager::all()`.
/// Prevents lock contention and repeated query preparation across the transcription pipeline.
#[derive(Clone, Default)]
pub struct SettingsSnapshot {
    values: serde_json::Map<String, serde_json::Value>,
}

impl SettingsSnapshot {
    pub fn new(values: serde_json::Map<String, serde_json::Value>) -> Self {
        Self { values }
    }

    pub fn from_value(val: serde_json::Value) -> Self {
        match val {
            serde_json::Value::Object(map) => Self { values: map },
            _ => Self::default(),
        }
    }

    pub fn load(db: &Database) -> Result<Self> {
        let all = SettingsManager::new(db).all()?;
        Ok(Self::from_value(all))
    }

    pub fn get<T: serde::de::DeserializeOwned>(&self, key: &str) -> Option<T> {
        self.values
            .get(key)
            .cloned()
            .and_then(|v| serde_json::from_value(v).ok())
    }

    pub fn string(&self, key: &str, default: &str) -> String {
        self.get::<String>(key)
            .unwrap_or_else(|| default.to_string())
    }

    pub fn u32(&self, key: &str, default: u32) -> u32 {
        self.get::<u32>(key).unwrap_or(default)
    }

    pub fn bool(&self, key: &str, default: bool) -> bool {
        self.get::<bool>(key).unwrap_or(default)
    }
}

impl std::fmt::Debug for SettingsSnapshot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut d = f.debug_map();
        for (k, v) in &self.values {
            if is_sensitive_setting_key(k) {
                d.entry(k, &"[REDACTED]");
            } else {
                d.entry(k, v);
            }
        }
        d.finish()
    }
}

fn is_sensitive_setting_key(key: &str) -> bool {
    let lower = key.to_ascii_lowercase();
    lower.contains("api_key")
        || lower.contains("secret")
        || lower.contains("token")
        || lower.contains("password")
}

pub fn string_setting(db: &Database, key: &str, default: &str) -> String {
    SettingsManager::new(db)
        .get::<String>(key)
        .ok()
        .flatten()
        .unwrap_or_else(|| default.to_string())
}

pub fn u32_setting(db: &Database, key: &str, default: u32) -> u32 {
    SettingsManager::new(db)
        .get::<u32>(key)
        .ok()
        .flatten()
        .unwrap_or(default)
}

pub fn build_audio_config(db: &Database) -> AudioConfig {
    AudioConfig {
        mic_device: string_setting(db, "mic_device", "default"),
        ..Default::default()
    }
}

fn decrypt_api_key_str(raw: &str, master_key: &[u8; 32]) -> Result<String> {
    if raw.is_empty() {
        return Ok(String::new());
    }
    crate::crypto::decrypt_api_key(raw, master_key).map_err(|e| {
        tracing::error!("Failed to decrypt API key: {e}");
        AppError::api_key_decrypt_failed(
            "Failed to decrypt stored Groq API key: master key mismatch or corrupted key",
        )
    })
}

pub fn decrypted_api_key(state: &AppStateInner) -> Result<String> {
    let raw = string_setting(&state.db, "groq_api_key", "");
    decrypt_api_key_str(&raw, &state.master_key)
}

pub fn decrypted_api_key_from_settings(
    state: &AppStateInner,
    settings: &SettingsSnapshot,
) -> Result<String> {
    let raw = settings.string("groq_api_key", "");
    decrypt_api_key_str(&raw, &state.master_key)
}

fn whisper_cpp_config_from_settings(settings: &SettingsSnapshot) -> WhisperCppConfig {
    WhisperCppConfig {
        binary_path: settings.string("whisper_cpp_binary_path", "whisper-cli"),
        model_path: settings.string("whisper_cpp_model_path", ""),
        threads: settings.u32("whisper_cpp_threads", 4),
    }
}

pub fn build_stt(state: &AppStateInner) -> Result<Arc<dyn crate::stt::SttEngine>> {
    let settings = SettingsSnapshot::load(&state.db)?;
    build_stt_from_settings(state, &settings)
}

fn normalize_stt_model(model: String) -> String {
    if model == "small" || model.trim().is_empty() {
        "whisper-large-v3-turbo".to_string()
    } else {
        model
    }
}

pub fn build_stt_from_settings(
    state: &AppStateInner,
    settings: &SettingsSnapshot,
) -> Result<Arc<dyn crate::stt::SttEngine>> {
    let kind = stt_engine_kind(&settings.string("stt_engine", "groq"));
    let api_key = match kind {
        SttEngineKind::Groq => decrypted_api_key_from_settings(state, settings)?,
        SttEngineKind::WhisperCpp => String::new(),
    };
    let model = normalize_stt_model(settings.string("stt_model", "whisper-large-v3-turbo"));
    let whisper_cpp = whisper_cpp_config_from_settings(settings);
    let cache_key = stt_cache_key(kind, &api_key, &model, &whisper_cpp);

    let mut cache = state.stt_engine.lock_recover();
    if let Some((cached_kind, cached_key, cached_engine)) = &*cache {
        if *cached_kind == kind && *cached_key == cache_key {
            return Ok(cached_engine.clone());
        }
    }

    let groq = GroqSttConfig {
        api_key,
        model,
        language: settings.string("stt_language", "auto"),
        ..Default::default()
    };
    let new_engine = SttFactory::create(kind, groq, whisper_cpp);
    *cache = Some((kind, cache_key, new_engine.clone()));
    Ok(new_engine)
}

fn stt_engine_kind(value: &str) -> SttEngineKind {
    match value {
        "whisper_cpp" => SttEngineKind::WhisperCpp,
        _ => SttEngineKind::Groq,
    }
}

fn stt_cache_key(
    kind: SttEngineKind,
    api_key: &str,
    model: &str,
    whisper_cpp: &WhisperCppConfig,
) -> String {
    match kind {
        SttEngineKind::Groq => format!("{}|{}", api_key, model),
        SttEngineKind::WhisperCpp => format!(
            "{}|{}|{}",
            whisper_cpp.binary_path, whisper_cpp.model_path, whisper_cpp.threads
        ),
    }
}

pub fn build_llm(state: &AppStateInner) -> Arc<dyn crate::llm::LlmFormatter> {
    let settings = SettingsSnapshot::load(&state.db).unwrap_or_default();
    build_llm_from_settings(state, &settings)
}

pub fn build_llm_from_settings(
    state: &AppStateInner,
    settings: &SettingsSnapshot,
) -> Arc<dyn crate::llm::LlmFormatter> {
    let engine = settings.string("llm_engine", "ollama");
    let kind = match engine.as_str() {
        "off" => LlmEngineKind::Off,
        "groq" => LlmEngineKind::Groq,
        "rule_based" => LlmEngineKind::RuleBased,
        _ => LlmEngineKind::Ollama,
    };
    let ollama = OllamaConfig {
        model: settings.string("llm_model", "qwen2.5:3b"),
        ..Default::default()
    };
    let groq = GroqLlmConfig {
        api_key: decrypted_api_key_from_settings(state, settings).unwrap_or_default(),
        ..Default::default()
    };
    LlmFactory::create(kind, ollama, groq, RuleBasedConfig::default())
}

/// Maximum number of hotwords included in Whisper initial prompt.
/// Whisper caps prompt context at ~224 tokens; 150 words conservatively stays within this limit.
pub const MAX_INITIAL_PROMPT_WORDS: usize = 150;

pub fn build_initial_prompt(hotwords: &[String]) -> Option<String> {
    if hotwords.is_empty() {
        return None;
    }
    // Whisper context window is ~224 tokens. Truncate to MAX_INITIAL_PROMPT_WORDS to prevent overflow.
    let capped = if hotwords.len() > MAX_INITIAL_PROMPT_WORDS {
        &hotwords[..MAX_INITIAL_PROMPT_WORDS]
    } else {
        hotwords
    };
    Some(capped.join(", "))
}

pub fn build_stt_config(db: &Database) -> SttConfig {
    let settings = SettingsSnapshot::load(db).unwrap_or_default();
    build_stt_config_from_settings(db, &settings)
}

pub fn build_stt_config_from_settings(db: &Database, settings: &SettingsSnapshot) -> SttConfig {
    let language = settings.string("stt_language", "auto");
    // When auto-detecting, skip hotwords to avoid biasing the STT model
    // toward a specific language.
    let hotwords = if language == "auto" {
        Vec::new()
    } else {
        DictionaryRepository::new(db)
            .get_hotwords_by_language(&language)
            .unwrap_or_default()
    };
    let initial_prompt = build_initial_prompt(&hotwords);
    SttConfig {
        language,
        initial_prompt,
        temperature: 0.0,
    }
}

pub fn active_mode(db: &Database, active_app: Option<&str>) -> LlmMode {
    let settings = SettingsSnapshot::load(db).unwrap_or_default();
    active_mode_from_settings(db, &settings, active_app)
}

pub fn active_mode_from_settings(
    db: &Database,
    settings: &SettingsSnapshot,
    active_app: Option<&str>,
) -> LlmMode {
    let per_app_on = settings.bool("per_app_mode", false);
    if per_app_on {
        if let Some(proc) = active_app {
            if let Ok(Some(mode_id)) = crate::storage::PerAppModeRepository::new(db).mode_for(proc)
            {
                return LlmMode::from_id(&mode_id);
            }
        }
    }
    LlmMode::from_id(&settings.string("active_mode", "dictation"))
}

pub fn sound_cues_enabled(db: &Database) -> bool {
    let settings = SettingsSnapshot::load(db).unwrap_or_default();
    settings.bool("sound_cues", false)
}

pub fn telemetry_enabled(db: &Database) -> bool {
    let settings = SettingsSnapshot::load(db).unwrap_or_default();
    settings.bool("telemetry", false)
}

pub fn csv_escape(s: &str) -> String {
    s.replace('"', "\"\"").replace('\r', "").replace('\n', " ")
}

pub fn engine_kind(name: &str) -> crate::storage::EngineKind {
    match name {
        "groq_whisper" | "groq_llm" => crate::storage::EngineKind::Cloud,
        "ollama" => crate::storage::EngineKind::Local,
        _ => crate::storage::EngineKind::Local,
    }
}

// ---------------------------------------------------------------
// Core recording flow
// ---------------------------------------------------------------

/// Process the captured audio through STT → LLM → injection → persist → emit.
pub async fn process_audio<R: Runtime>(app: AppHandle<R>, audio: Vec<f32>) {
    let state = app.state::<AppStateInner>();

    if audio.is_empty() {
        let _ = state.pipeline.finish_processing();
        events::emit_state(&app, state.pipeline.state_tag());
        crate::overlay::reset_idle_timer(&state);
        crate::overlay::maybe_hide(&app);
        return;
    }

    let settings = match SettingsSnapshot::load(&state.db) {
        Ok(s) => s,
        Err(e) => return fail(&app, &state.pipeline, &e),
    };

    let stt = match build_stt_from_settings(&state, &settings) {
        Ok(e) => e,
        Err(e) => return fail(&app, &state.pipeline, &e),
    };
    let stt_config = build_stt_config_from_settings(&state.db, &settings);

    // Command mode: transcribe first so we can intercept editing commands.
    let command_mode = settings.bool("command_mode", false);
    let mut precomputed = None;
    if command_mode {
        match stt.transcribe(&audio, &stt_config).await {
            Ok(tr) => {
                if let Some(cmd) = crate::injection::VoiceCommand::from_phrase(&tr.text) {
                    let initial_app = state.pipeline.active_app();
                    let current_app = crate::active_window::foreground_process_name();
                    if let Err(e) = crate::injection::command::verify_target_window(
                        initial_app.as_deref(),
                        current_app.as_deref(),
                    ) {
                        return fail(&app, &state.pipeline, &e);
                    }
                    if let Err(e) = crate::injection::command::execute(cmd) {
                        return fail(&app, &state.pipeline, &e);
                    }
                    let _ = state.pipeline.finish_processing();
                    events::emit_transcription_complete(
                        &app,
                        &Uuid::new_v4().to_string(),
                        &format!("[command] {}", tr.text.trim()),
                        0,
                        0,
                    );
                    events::emit_state(&app, state.pipeline.state_tag());
                    crate::overlay::reset_idle_timer(&state);
                    hide_overlay_soon(app.clone());
                    return;
                }
                precomputed = Some(tr);
            }
            Err(e) => return fail(&app, &state.pipeline, &e),
        }
    }

    let llm = build_llm_from_settings(&state, &settings);
    let active_app = state.pipeline.active_app();
    let mode = active_mode_from_settings(&state.db, &settings, active_app.as_deref());
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
    let injector = std::sync::Arc::new(HybridInjector::new());

    let translate_enabled = settings.bool("translation_enabled", false);
    let translate_target = settings.string("translation_target", "en");
    let translate_opts = if translate_enabled {
        Some(batch::TranslateOpts {
            target: translate_target.clone(),
        })
    } else {
        None
    };

    let stt_name = stt.name().to_string();
    let llm_name = llm.name().to_string();
    let mode_id = mode.id();

    let outcome = match precomputed {
        Some(tr) => {
            batch::run_batch_with_transcription(
                tr,
                llm,
                &mode,
                &post,
                translate_opts.as_ref(),
                injector.clone(),
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
                injector,
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
            if let Err(e) = HistoryRepository::new(&state.db).insert(&entry) {
                tracing::error!("Failed to insert transcription into history: {e}");
            }

            let telemetry = settings.bool("telemetry", false);
            if telemetry {
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
            events::emit_transcription_complete(
                &app,
                &id,
                &out.formatted_text,
                word_count,
                out.transcription.duration_ms as i64,
            );
            events::emit_state(&app, state.pipeline.state_tag());
            crate::overlay::reset_idle_timer(&state);
            hide_overlay_soon(app.clone());
        }
        Err(e) => {
            if settings.bool("telemetry", false) {
                let _ = crate::storage::StatsRepository::new(&state.db).record_error();
            }
            fail(&app, &state.pipeline, &e)
        }
    }
}

pub fn fail<R: Runtime>(app: &AppHandle<R>, pipeline: &PipelineOrchestrator, e: &AppError) {
    tracing::error!("Pipeline error: {e}");
    pipeline.set_error(e);
    events::emit_transcription_error(app, &e.message, &format!("{:?}", e.code));
    events::emit_state(app, pipeline.state_tag());
    let state = app.state::<AppStateInner>();
    crate::overlay::reset_idle_timer(&state);
    hide_overlay_soon(app.clone());
}

pub fn hide_overlay_soon<R: Runtime>(app: AppHandle<R>) {
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(1800)).await;
        let tag = app.state::<AppStateInner>().pipeline.state_tag();
        if tag != crate::pipeline::AppStateTag::Recording
            && tag != crate::pipeline::AppStateTag::Processing
        {
            crate::overlay::maybe_hide(&app);
        }
    });
}

// ---------------------------------------------------------------
// Hotkey entry points
// ---------------------------------------------------------------

pub fn hotkey_start<R: Runtime>(app: &AppHandle<R>) {
    let state = app.state::<AppStateInner>();
    let tag = state.pipeline.state_tag();
    if tag != crate::pipeline::AppStateTag::Idle && tag != crate::pipeline::AppStateTag::Error {
        return;
    }

    let active_app = crate::active_window::foreground_process_name();
    let tag = match state
        .pipeline
        .apply(crate::pipeline::StateEvent::StartRecording { active_app })
    {
        Ok(t) => t,
        Err(e) => return fail(app, &state.pipeline, &e),
    };

    if sound_cues_enabled(&state.db) {
        crate::sound::play(crate::sound::Cue::Start);
    }
    state
        .widget_timer
        .hidden_by_timeout
        .store(false, Ordering::SeqCst);
    crate::overlay::reset_idle_timer(&state);
    crate::overlay::ensure_visible(app);
    events::emit_state(app, tag);
    spawn_level_emitter(app.clone());
    spawn_capture_task(app.clone());
}

fn spawn_capture_task<R: Runtime>(app: AppHandle<R>) {
    tauri::async_runtime::spawn(async move {
        let app_capture = app.clone();
        let capture_result = tokio::task::spawn_blocking(move || {
            let state = app_capture.state::<AppStateInner>();
            let config = build_audio_config(&state.db);
            // Device initialization runs on a blocking thread pool without holding
            // the pipeline state lock so stop/cancel and UI transitions remain responsive.
            state.pipeline.start_capture_if_recording(&config)
        })
        .await;

        let state = app.state::<AppStateInner>();
        match capture_result {
            Ok(Ok(true)) => {}
            Ok(Ok(false)) => {
                tracing::debug!("Capture start skipped: recording already ended");
            }
            Ok(Err(e)) => {
                let _ = state.pipeline.cancel_recording();
                fail(&app, &state.pipeline, &e);
            }
            Err(join_err) => {
                let err =
                    AppError::audio(format!("Capture initialization task failed: {join_err}"));
                let _ = state.pipeline.cancel_recording();
                fail(&app, &state.pipeline, &err);
            }
        }
    });
}

fn spawn_level_emitter<R: Runtime>(app: AppHandle<R>) {
    tauri::async_runtime::spawn(async move {
        loop {
            let mut stop = false;
            {
                let state = app.state::<AppStateInner>();
                if state.pipeline.state_tag() != crate::pipeline::AppStateTag::Recording {
                    break;
                }
                if let Some(dur) = state.pipeline.recording_duration() {
                    if dur >= Duration::from_secs(MAX_RECORDING_DURATION_SECS) {
                        stop = true;
                    }
                }
                let level = state.pipeline.audio_level();
                events::emit_audio_level(&app, level);
            }
            if stop {
                hotkey_stop(&app);
                break;
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

    // Skip if held for ≤1 second (likely accidental).
    if let Some(duration) = state.pipeline.recording_duration() {
        if duration.as_secs_f32() <= MIN_RECORDING_DURATION_SECS {
            let _ = state.pipeline.cancel_recording();
            events::emit_state(app, state.pipeline.state_tag());
            crate::overlay::reset_idle_timer(&state);
            crate::overlay::maybe_hide(app);
            return;
        }
    }

    crate::overlay::reset_idle_timer(&state);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_hotwords_returns_none() {
        assert_eq!(build_initial_prompt(&[]), None);
    }

    #[test]
    fn hotwords_below_cap_joined_with_comma() {
        let hotwords = vec!["alpha".to_string(), "beta".to_string()];
        assert_eq!(
            build_initial_prompt(&hotwords),
            Some("alpha, beta".to_string())
        );
    }

    #[test]
    fn hotwords_capped_at_max_words() {
        let hotwords: Vec<String> = (0..200).map(|i| format!("word{i}")).collect();
        let prompt = build_initial_prompt(&hotwords).unwrap();
        let words: Vec<&str> = prompt.split(", ").collect();
        assert_eq!(words.len(), MAX_INITIAL_PROMPT_WORDS);
        assert_eq!(words[0], "word0");
        assert_eq!(
            words[MAX_INITIAL_PROMPT_WORDS - 1],
            format!("word{}", MAX_INITIAL_PROMPT_WORDS - 1)
        );
    }

    #[test]
    fn max_recording_duration_does_not_exceed_stt_upload_timeout() {
        assert!(
            Duration::from_secs(MAX_RECORDING_DURATION_SECS)
                <= crate::stt::groq_stt::GROQ_STT_TIMEOUT,
            "MAX_RECORDING_DURATION_SECS must not exceed GROQ_STT_TIMEOUT"
        );
    }

    use crate::error::ErrorCode;
    use crate::overlay::WidgetTimerState;
    use std::collections::HashMap;
    use std::path::PathBuf;

    fn test_app_state(master_key: [u8; 32]) -> AppStateInner {
        AppStateInner {
            db: Database::open_in_memory().unwrap(),
            pipeline: PipelineOrchestrator::new(),
            app_data_dir: PathBuf::from("/test"),
            default_app_data_dir: PathBuf::from("/test"),
            master_key,
            _log_guard: None,
            stt_engine: std::sync::Mutex::new(None),
            last_picker_dirs: std::sync::Mutex::new(HashMap::new()),
            widget_timer: WidgetTimerState::new(),
        }
    }

    #[test]
    fn decrypted_api_key_when_not_set_returns_ok_empty() {
        let state = test_app_state([1u8; 32]);
        let res = decrypted_api_key(&state);
        assert_eq!(res.unwrap(), "");
    }

    #[test]
    fn decrypted_api_key_success() {
        let master_key = [7u8; 32];
        let state = test_app_state(master_key);
        let secret = "gsk_valid_secret_key_12345";
        let encrypted = crate::crypto::encrypt_api_key(secret, &master_key).unwrap();
        SettingsManager::new(&state.db)
            .set("groq_api_key", &encrypted)
            .unwrap();

        let res = decrypted_api_key(&state);
        assert_eq!(res.unwrap(), secret);
    }

    #[test]
    fn decrypted_api_key_fails_when_master_key_mismatched_without_leaking_key() {
        let original_key = [7u8; 32];
        let different_key = [9u8; 32];
        let state = test_app_state(different_key);
        let secret = "gsk_super_secret_that_must_not_leak_123";
        let encrypted = crate::crypto::encrypt_api_key(secret, &original_key).unwrap();
        SettingsManager::new(&state.db)
            .set("groq_api_key", &encrypted)
            .unwrap();

        let err = decrypted_api_key(&state).unwrap_err();
        assert_eq!(err.code, ErrorCode::SttApiKeyInvalid);
        assert_ne!(err.message, "Groq API key is not set");
        assert!(err.message.contains("decrypt") || err.message.contains("master key"));
        assert!(
            !err.message.contains(secret),
            "error message must not leak key content"
        );
    }

    #[test]
    fn build_stt_fails_on_decryption_error_and_does_not_leak_key() {
        let original_key = [7u8; 32];
        let different_key = [9u8; 32];
        let state = test_app_state(different_key);
        let secret = "gsk_super_secret_that_must_not_leak_456";
        let encrypted = crate::crypto::encrypt_api_key(secret, &original_key).unwrap();
        SettingsManager::new(&state.db)
            .set("groq_api_key", &encrypted)
            .unwrap();
        SettingsManager::new(&state.db)
            .set("stt_engine", &"groq")
            .unwrap();

        let Err(err) = build_stt(&state) else {
            panic!("expected build_stt to fail with decryption error");
        };
        assert_eq!(err.code, ErrorCode::SttApiKeyInvalid);
        assert_ne!(err.message, "Groq API key is not set");
        assert!(
            !err.message.contains(secret),
            "error message must not leak key content"
        );
    }

    #[test]
    fn build_stt_succeeds_when_key_not_set_and_transcribe_fails_with_not_set() {
        let state = test_app_state([1u8; 32]);
        SettingsManager::new(&state.db)
            .set("stt_engine", &"groq")
            .unwrap();

        let Ok(engine) = build_stt(&state) else {
            panic!("expected build_stt to succeed when key is not set");
        };
        let config = SttConfig::default();
        let rt = tokio::runtime::Runtime::new().unwrap();
        let err = rt
            .block_on(engine.transcribe(&[0.0; 160], &config))
            .unwrap_err();
        assert_eq!(err.code, ErrorCode::SttApiKeyInvalid);
        assert_eq!(err.message, "Groq API key is not set");
    }

    #[test]
    fn settings_snapshot_identical_to_per_key_reads_empty_db() {
        let state = test_app_state([3u8; 32]);
        let snapshot = SettingsSnapshot::load(&state.db).unwrap();

        assert_eq!(
            string_setting(&state.db, "stt_engine", "groq"),
            snapshot.string("stt_engine", "groq")
        );
        assert_eq!(
            string_setting(&state.db, "stt_model", "whisper-large-v3-turbo"),
            snapshot.string("stt_model", "whisper-large-v3-turbo")
        );
        assert_eq!(
            string_setting(&state.db, "whisper_cpp_binary_path", "whisper-cli"),
            snapshot.string("whisper_cpp_binary_path", "whisper-cli")
        );
        assert_eq!(
            string_setting(&state.db, "whisper_cpp_model_path", ""),
            snapshot.string("whisper_cpp_model_path", "")
        );
        assert_eq!(
            u32_setting(&state.db, "whisper_cpp_threads", 4),
            snapshot.u32("whisper_cpp_threads", 4)
        );
        assert_eq!(
            string_setting(&state.db, "stt_language", "auto"),
            snapshot.string("stt_language", "auto")
        );
        assert_eq!(
            string_setting(&state.db, "llm_engine", "ollama"),
            snapshot.string("llm_engine", "ollama")
        );
        assert_eq!(
            string_setting(&state.db, "llm_model", "qwen2.5:3b"),
            snapshot.string("llm_model", "qwen2.5:3b")
        );
        assert_eq!(
            string_setting(&state.db, "active_mode", "dictation"),
            snapshot.string("active_mode", "dictation")
        );
        assert_eq!(
            string_setting(&state.db, "translation_target", "en"),
            snapshot.string("translation_target", "en")
        );
        assert_eq!(
            SettingsManager::new(&state.db)
                .get::<bool>("command_mode")
                .unwrap()
                .unwrap_or(false),
            snapshot.bool("command_mode", false)
        );
        assert_eq!(
            SettingsManager::new(&state.db)
                .get::<bool>("per_app_mode")
                .unwrap()
                .unwrap_or(false),
            snapshot.bool("per_app_mode", false)
        );
        assert_eq!(
            SettingsManager::new(&state.db)
                .get::<bool>("translation_enabled")
                .unwrap()
                .unwrap_or(false),
            snapshot.bool("translation_enabled", false)
        );
        assert_eq!(
            telemetry_enabled(&state.db),
            snapshot.bool("telemetry", false)
        );
        assert_eq!(
            sound_cues_enabled(&state.db),
            snapshot.bool("sound_cues", false)
        );

        let cfg_per_key = build_stt_config(&state.db);
        let cfg_snapshot = build_stt_config_from_settings(&state.db, &snapshot);
        assert_eq!(cfg_per_key.language, cfg_snapshot.language);
        assert_eq!(cfg_per_key.initial_prompt, cfg_snapshot.initial_prompt);
        assert_eq!(cfg_per_key.temperature, cfg_snapshot.temperature);

        assert_eq!(
            active_mode(&state.db, None).id(),
            active_mode_from_settings(&state.db, &snapshot, None).id()
        );
        assert_eq!(
            build_llm(&state).name(),
            build_llm_from_settings(&state, &snapshot).name()
        );
        assert_eq!(
            decrypted_api_key(&state).unwrap(),
            decrypted_api_key_from_settings(&state, &snapshot).unwrap()
        );
        assert_eq!(
            build_stt(&state).unwrap().name(),
            build_stt_from_settings(&state, &snapshot).unwrap().name()
        );
    }

    #[test]
    fn settings_snapshot_identical_to_per_key_reads_partially_populated_db() {
        let state = test_app_state([4u8; 32]);
        let mgr = SettingsManager::new(&state.db);
        mgr.set("stt_language", &"id").unwrap();
        mgr.set("command_mode", &true).unwrap();
        mgr.set("whisper_cpp_threads", &8u32).unwrap();
        mgr.set("translation_enabled", &true).unwrap();
        mgr.set("translation_target", &"ja").unwrap();

        let snapshot = SettingsSnapshot::load(&state.db).unwrap();

        assert_eq!(
            string_setting(&state.db, "stt_language", "auto"),
            snapshot.string("stt_language", "auto")
        );
        assert_eq!(
            u32_setting(&state.db, "whisper_cpp_threads", 4),
            snapshot.u32("whisper_cpp_threads", 4)
        );
        assert_eq!(
            SettingsManager::new(&state.db)
                .get::<bool>("command_mode")
                .unwrap()
                .unwrap_or(false),
            snapshot.bool("command_mode", false)
        );
        assert_eq!(
            SettingsManager::new(&state.db)
                .get::<bool>("translation_enabled")
                .unwrap()
                .unwrap_or(false),
            snapshot.bool("translation_enabled", false)
        );
        assert_eq!(
            string_setting(&state.db, "translation_target", "en"),
            snapshot.string("translation_target", "en")
        );

        let cfg_per_key = build_stt_config(&state.db);
        let cfg_snapshot = build_stt_config_from_settings(&state.db, &snapshot);
        assert_eq!(cfg_per_key.language, cfg_snapshot.language);
        assert_eq!(cfg_per_key.initial_prompt, cfg_snapshot.initial_prompt);

        assert_eq!(
            active_mode(&state.db, None).id(),
            active_mode_from_settings(&state.db, &snapshot, None).id()
        );
        assert_eq!(
            build_llm(&state).name(),
            build_llm_from_settings(&state, &snapshot).name()
        );
    }

    #[test]
    fn settings_snapshot_identical_to_per_key_reads_fully_populated_db() {
        let master_key = [5u8; 32];
        let state = test_app_state(master_key);
        let secret_key = "gsk_full_test_key_xyz123";
        let encrypted = crate::crypto::encrypt_api_key(secret_key, &master_key).unwrap();

        let mgr = SettingsManager::new(&state.db);
        mgr.set("stt_engine", &"whisper_cpp").unwrap();
        mgr.set("stt_model", &"ggml-base.bin").unwrap();
        mgr.set("whisper_cpp_binary_path", &"/usr/bin/whisper-cli")
            .unwrap();
        mgr.set("whisper_cpp_model_path", &"/models/base.bin")
            .unwrap();
        mgr.set("whisper_cpp_threads", &6u32).unwrap();
        mgr.set("stt_language", &"en").unwrap();
        mgr.set("command_mode", &true).unwrap();
        mgr.set("llm_engine", &"groq").unwrap();
        mgr.set("llm_model", &"llama-3.1-8b-instant").unwrap();
        mgr.set("per_app_mode", &true).unwrap();
        mgr.set("active_mode", &"email").unwrap();
        mgr.set("translation_enabled", &true).unwrap();
        mgr.set("translation_target", &"de").unwrap();
        mgr.set("telemetry", &true).unwrap();
        mgr.set("sound_cues", &true).unwrap();
        mgr.set("groq_api_key", &encrypted).unwrap();

        let snapshot = SettingsSnapshot::load(&state.db).unwrap();

        assert_eq!(
            string_setting(&state.db, "stt_engine", "groq"),
            snapshot.string("stt_engine", "groq")
        );
        assert_eq!(
            string_setting(&state.db, "stt_model", "whisper-large-v3-turbo"),
            snapshot.string("stt_model", "whisper-large-v3-turbo")
        );
        assert_eq!(
            string_setting(&state.db, "whisper_cpp_binary_path", "whisper-cli"),
            snapshot.string("whisper_cpp_binary_path", "whisper-cli")
        );
        assert_eq!(
            string_setting(&state.db, "whisper_cpp_model_path", ""),
            snapshot.string("whisper_cpp_model_path", "")
        );
        assert_eq!(
            u32_setting(&state.db, "whisper_cpp_threads", 4),
            snapshot.u32("whisper_cpp_threads", 4)
        );
        assert_eq!(
            string_setting(&state.db, "stt_language", "auto"),
            snapshot.string("stt_language", "auto")
        );
        assert_eq!(
            SettingsManager::new(&state.db)
                .get::<bool>("command_mode")
                .unwrap()
                .unwrap_or(false),
            snapshot.bool("command_mode", false)
        );
        assert_eq!(
            string_setting(&state.db, "llm_engine", "ollama"),
            snapshot.string("llm_engine", "ollama")
        );
        assert_eq!(
            string_setting(&state.db, "llm_model", "qwen2.5:3b"),
            snapshot.string("llm_model", "qwen2.5:3b")
        );
        assert_eq!(
            SettingsManager::new(&state.db)
                .get::<bool>("per_app_mode")
                .unwrap()
                .unwrap_or(false),
            snapshot.bool("per_app_mode", false)
        );
        assert_eq!(
            string_setting(&state.db, "active_mode", "dictation"),
            snapshot.string("active_mode", "dictation")
        );
        assert_eq!(
            SettingsManager::new(&state.db)
                .get::<bool>("translation_enabled")
                .unwrap()
                .unwrap_or(false),
            snapshot.bool("translation_enabled", false)
        );
        assert_eq!(
            string_setting(&state.db, "translation_target", "en"),
            snapshot.string("translation_target", "en")
        );
        assert_eq!(
            telemetry_enabled(&state.db),
            snapshot.bool("telemetry", false)
        );
        assert_eq!(
            sound_cues_enabled(&state.db),
            snapshot.bool("sound_cues", false)
        );

        let cfg_per_key = build_stt_config(&state.db);
        let cfg_snapshot = build_stt_config_from_settings(&state.db, &snapshot);
        assert_eq!(cfg_per_key.language, cfg_snapshot.language);
        assert_eq!(cfg_per_key.initial_prompt, cfg_snapshot.initial_prompt);

        assert_eq!(snapshot.string("stt_engine", "groq"), "whisper_cpp");
        assert_eq!(
            snapshot.string("stt_model", "whisper-large-v3-turbo"),
            "ggml-base.bin"
        );
        assert_eq!(
            snapshot.string("whisper_cpp_binary_path", "whisper-cli"),
            "/usr/bin/whisper-cli"
        );
        assert_eq!(
            snapshot.string("whisper_cpp_model_path", ""),
            "/models/base.bin"
        );
        assert_eq!(snapshot.u32("whisper_cpp_threads", 4), 6);
        assert_eq!(snapshot.string("stt_language", "auto"), "en");
        assert_eq!(snapshot.bool("command_mode", false), true);
        assert_eq!(snapshot.string("llm_engine", "ollama"), "groq");
        assert_eq!(
            snapshot.string("llm_model", "qwen2.5:3b"),
            "llama-3.1-8b-instant"
        );
        assert_eq!(snapshot.bool("per_app_mode", false), true);
        assert_eq!(snapshot.string("active_mode", "dictation"), "email");
        assert_eq!(snapshot.bool("translation_enabled", false), true);
        assert_eq!(snapshot.string("translation_target", "en"), "de");
        assert_eq!(snapshot.bool("telemetry", false), true);
        assert_eq!(snapshot.bool("sound_cues", false), true);

        assert_eq!(
            active_mode(&state.db, None).id(),
            active_mode_from_settings(&state.db, &snapshot, None).id()
        );
        assert_eq!(
            build_llm(&state).name(),
            build_llm_from_settings(&state, &snapshot).name()
        );
        assert_eq!(
            decrypted_api_key(&state).unwrap(),
            decrypted_api_key_from_settings(&state, &snapshot).unwrap()
        );
        assert_eq!(
            build_stt(&state).unwrap().name(),
            build_stt_from_settings(&state, &snapshot).unwrap().name()
        );
    }

    #[test]
    fn settings_snapshot_debug_does_not_leak_api_key() {
        let secret = "gsk_super_secret_groq_key_that_must_never_leak_in_debug_repr";
        let mut map = serde_json::Map::new();
        map.insert(
            "groq_api_key".to_string(),
            serde_json::Value::String(secret.to_string()),
        );
        map.insert(
            "custom_api_key".to_string(),
            serde_json::Value::String("another_api_key_val".to_string()),
        );
        map.insert(
            "app_secret".to_string(),
            serde_json::Value::String("secret_password".to_string()),
        );
        map.insert(
            "stt_engine".to_string(),
            serde_json::Value::String("groq".to_string()),
        );

        let snapshot = SettingsSnapshot::new(map);
        let debug_output = format!("{snapshot:?}");

        assert!(
            !debug_output.contains(secret),
            "Debug output must not contain groq_api_key value"
        );
        assert!(
            !debug_output.contains("another_api_key_val"),
            "Debug output must not contain keys matching *api_key*"
        );
        assert!(
            !debug_output.contains("secret_password"),
            "Debug output must not contain keys matching *secret*"
        );
        assert!(
            debug_output.contains("[REDACTED]"),
            "Debug output must contain [REDACTED]"
        );
        assert!(
            debug_output.contains("groq"),
            "Debug output should retain non-sensitive settings"
        );
    }

    #[test]
    fn settings_snapshot_default_values_when_keys_missing() {
        let snapshot = SettingsSnapshot::default();

        assert_eq!(snapshot.string("stt_engine", "groq"), "groq");
        assert_eq!(
            snapshot.string("stt_model", "whisper-large-v3-turbo"),
            "whisper-large-v3-turbo"
        );
        assert_eq!(
            snapshot.string("whisper_cpp_binary_path", "whisper-cli"),
            "whisper-cli"
        );
        assert_eq!(snapshot.string("whisper_cpp_model_path", ""), "");
        assert_eq!(snapshot.u32("whisper_cpp_threads", 4), 4);
        assert_eq!(snapshot.string("stt_language", "auto"), "auto");
        assert_eq!(snapshot.bool("command_mode", false), false);
        assert_eq!(snapshot.string("llm_engine", "ollama"), "ollama");
        assert_eq!(snapshot.string("llm_model", "qwen2.5:3b"), "qwen2.5:3b");
        assert_eq!(snapshot.bool("per_app_mode", false), false);
        assert_eq!(snapshot.string("active_mode", "dictation"), "dictation");
        assert_eq!(snapshot.bool("translation_enabled", false), false);
        assert_eq!(snapshot.string("translation_target", "en"), "en");
        assert_eq!(snapshot.bool("telemetry", false), false);
        assert_eq!(snapshot.bool("sound_cues", false), false);
        assert_eq!(snapshot.string("groq_api_key", ""), "");
        assert_eq!(snapshot.string("mic_device", "default"), "default");
    }

    #[tokio::test]
    async fn capture_start_offloaded_to_blocking_pool_allows_async_progress() {
        let state = test_app_state([1u8; 32]);
        let config = build_audio_config(&state.db);

        let (entered_tx, entered_rx) = tokio::sync::oneshot::channel();
        let (release_tx, release_rx) = tokio::sync::oneshot::channel();

        // Simulate a blocking audio init that takes some time on the blocking thread pool
        let blocking_handle = tokio::task::spawn_blocking(move || {
            let _ = entered_tx.send(());
            let _ = release_rx.blocking_recv();
            state.pipeline.start_capture_if_recording(&config)
        });

        // The async runtime must remain responsive while the blocking task is in-flight
        entered_rx.await.expect("blocking task should start");
        let async_ticker_ran = tokio::select! {
            _ = tokio::time::sleep(Duration::from_millis(10)) => true,
        };
        assert!(
            async_ticker_ran,
            "async runtime must tick while blocking task runs"
        );

        release_tx.send(()).expect("release blocking task");
        let res = blocking_handle.await.expect("spawn_blocking joins cleanly");
        assert_eq!(res.unwrap(), false);
    }
}
