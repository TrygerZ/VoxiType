//! Shared runtime helpers for STT/LLM config building and the core
//! recording → process → inject → persist pipeline.

use std::path::Path;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::{Duration, Instant};

use tauri::{AppHandle, Manager, Runtime};
use uuid::Uuid;

use crate::audio::{has_enough_voice, AudioConfig, TARGET_SAMPLE_RATE};
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

// Maximum recording duration in seconds to prevent runaway recordings. Shares
// the capture module's hard sample cap so the two limits cannot diverge.
pub const MAX_RECORDING_DURATION_SECS: u64 = crate::audio::MAX_CAPTURE_SECS;

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
    let decrypted = crate::crypto::decrypt_api_key(raw, master_key).map_err(|e| {
        tracing::error!("Failed to decrypt API key: {e}");
        AppError::api_key_decrypt_failed(
            "Failed to decrypt stored Groq API key: master key mismatch or corrupted key",
        )
    })?;
    // Trim so legacy rows written with stray whitespace self-heal at read time.
    Ok(decrypted.trim().to_string())
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

/// whisper.cpp `-t` thread flag bounds. A stored out-of-range value (legacy
/// rows written before write-side validation, or a hand-edited DB) is clamped
/// here, the single place both dictation and file-job engines route through.
pub(crate) const MIN_WHISPER_THREADS: u32 = 1;
pub(crate) const MAX_WHISPER_THREADS: u32 = 32;
const DEFAULT_WHISPER_THREADS: u32 = 4;

fn whisper_cpp_config_from_settings(settings: &SettingsSnapshot) -> WhisperCppConfig {
    let threads =
        clamp_whisper_threads(settings.u32("whisper_cpp_threads", DEFAULT_WHISPER_THREADS));
    WhisperCppConfig {
        binary_path: settings.string("whisper_cpp_binary_path", "whisper-cli"),
        model_path: settings.string("whisper_cpp_model_path", ""),
        threads,
    }
}

/// Clamp a caller-supplied thread count to the supported whisper.cpp range.
pub(crate) fn clamp_whisper_threads(threads: u32) -> u32 {
    threads.clamp(MIN_WHISPER_THREADS, MAX_WHISPER_THREADS)
}

/// Setting key holding the picker-issued token that vouches for the binary path.
pub const WHISPER_BINARY_TOKEN_KEY: &str = "whisper_cpp_binary_token";

/// User-facing instruction shown when the whisper.cpp binary cannot be trusted.
const WHISPER_BINARY_RE_SELECT: &str =
    "Re-select the whisper.cpp binary in Settings to use local transcription";

/// Reject a stored whisper.cpp binary that the picker-issued token does not vouch for.
///
/// `set_whisper_cpp_paths` seals the canonical path under the master key; the
/// engine builder opens that token and requires it to match the stored binary's
/// canonical path. A missing token (an install configured before this gate) or
/// any mismatch fails closed with a re-select message. There is no silent
/// fallback to another engine: falling back would hide a tampered or stale path.
fn verify_whisper_binary(state: &AppStateInner, config: &WhisperCppConfig) -> Result<()> {
    let token = SettingsManager::new(&state.db)
        .get::<String>(WHISPER_BINARY_TOKEN_KEY)
        .ok()
        .flatten()
        .unwrap_or_default();
    if token.trim().is_empty() {
        return Err(AppError::stt(format!(
            "whisper.cpp binary was not selected through Settings. {WHISPER_BINARY_RE_SELECT}"
        )));
    }
    let sealed_path = crate::crypto::open_token(&token, &state.master_key).map_err(|_| {
        AppError::stt(format!(
            "the stored whisper.cpp binary token is invalid. {WHISPER_BINARY_RE_SELECT}"
        ))
    })?;
    let canonical = Path::new(&config.binary_path)
        .canonicalize()
        .map_err(|error| {
            AppError::stt(format!(
                "whisper.cpp binary not found: {error}. {WHISPER_BINARY_RE_SELECT}"
            ))
        })?;
    if super::misc::canonical_to_clean_string(&canonical) != sealed_path {
        return Err(AppError::stt(format!(
            "the stored whisper.cpp binary path does not match its token. {WHISPER_BINARY_RE_SELECT}"
        )));
    }
    Ok(())
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

fn stt_configs_from_settings(
    state: &AppStateInner,
    settings: &SettingsSnapshot,
    kind: SttEngineKind,
) -> Result<(GroqSttConfig, WhisperCppConfig)> {
    let api_key = match kind {
        SttEngineKind::Groq => decrypted_api_key_from_settings(state, settings)?,
        SttEngineKind::WhisperCpp => String::new(),
    };
    let groq = GroqSttConfig {
        api_key,
        model: normalize_stt_model(settings.string("stt_model", "whisper-large-v3-turbo")),
        language: settings.string("stt_language", "auto"),
        ..Default::default()
    };
    Ok((groq, whisper_cpp_config_from_settings(settings)))
}

pub fn build_stt_from_settings(
    state: &AppStateInner,
    settings: &SettingsSnapshot,
) -> Result<Arc<dyn crate::stt::SttEngine>> {
    let kind = stt_engine_kind(&settings.string("stt_engine", "groq"))?;
    let (groq, whisper_cpp) = stt_configs_from_settings(state, settings, kind)?;
    if kind == SttEngineKind::WhisperCpp {
        verify_whisper_binary(state, &whisper_cpp)?;
    }
    let cache_key = stt_cache_key(kind, &groq.api_key, &groq.model, &whisper_cpp);

    let mut cache = state.stt_engine.lock_recover();
    if let Some((cached_kind, cached_key, cached_engine)) = &*cache {
        if *cached_kind == kind && *cached_key == cache_key {
            return Ok(cached_engine.clone());
        }
    }

    let new_engine = SttFactory::create(kind, groq, whisper_cpp)?;
    *cache = Some((kind, cache_key, new_engine.clone()));
    Ok(new_engine)
}

/// Build an engine of an explicit kind, bypassing the dictation cache so a
/// file job never evicts the engine the hotkey pipeline is using.
///
/// Fails fast on missing setup so a long file is not decoded only to hit
/// the same error on the first chunk.
pub fn build_stt_for_kind(
    state: &AppStateInner,
    settings: &SettingsSnapshot,
    kind: SttEngineKind,
) -> Result<Arc<dyn crate::stt::SttEngine>> {
    let (groq, whisper_cpp) = stt_configs_from_settings(state, settings, kind)?;
    match kind {
        SttEngineKind::Groq if groq.api_key.trim().is_empty() => {
            return Err(AppError::api_key_missing("Groq API key is not set"))
        }
        SttEngineKind::WhisperCpp => {
            verify_whisper_binary(state, &whisper_cpp)?;
            crate::stt::whisper_cpp::validate_config(&whisper_cpp)?;
        }
        SttEngineKind::Groq => {}
    }
    SttFactory::create(kind, groq, whisper_cpp)
}

/// Map a stored `stt_engine` string to its engine kind. An unknown value is an
/// error, never a silent fallback: coercion would hide config corruption and
/// run a different engine than the user selected.
fn stt_engine_kind(value: &str) -> Result<SttEngineKind> {
    match value {
        "groq" => Ok(SttEngineKind::Groq),
        "whisper_cpp" => Ok(SttEngineKind::WhisperCpp),
        other => Err(AppError::invalid_input(format!(
            "Unknown stt_engine '{other}'"
        ))),
    }
}

/// Cache key for the selected STT engine. The API key is hashed so the raw
/// secret is never held in a key string (it would otherwise live in memory and
/// could surface in a panic or debug dump); the digest still changes whenever
/// the key does, so the engine rebuilds correctly.
fn stt_cache_key(
    kind: SttEngineKind,
    api_key: &str,
    model: &str,
    whisper_cpp: &WhisperCppConfig,
) -> String {
    match kind {
        SttEngineKind::Groq => format!("{}|{}", api_key_digest(api_key), model),
        SttEngineKind::WhisperCpp => format!(
            "{}|{}|{}",
            whisper_cpp.binary_path, whisper_cpp.model_path, whisper_cpp.threads
        ),
    }
}

fn api_key_digest(api_key: &str) -> String {
    use sha2::{Digest, Sha256};
    let digest = Sha256::digest(api_key.as_bytes());
    digest.iter().map(|b| format!("{b:02x}")).collect()
}

pub fn build_llm(state: &AppStateInner) -> Result<Arc<dyn crate::llm::LlmFormatter>> {
    let settings = SettingsSnapshot::load(&state.db).unwrap_or_default();
    build_llm_from_settings(state, &settings)
}

/// Map a stored `llm_engine` string to its engine kind. An unknown value is an
/// error, never a silent fallback to Ollama.
fn llm_engine_kind(value: &str) -> Result<LlmEngineKind> {
    match value {
        "off" => Ok(LlmEngineKind::Off),
        "groq" => Ok(LlmEngineKind::Groq),
        "rule_based" => Ok(LlmEngineKind::RuleBased),
        "ollama" => Ok(LlmEngineKind::Ollama),
        other => Err(AppError::invalid_input(format!(
            "Unknown llm_engine '{other}'"
        ))),
    }
}

pub fn build_llm_from_settings(
    state: &AppStateInner,
    settings: &SettingsSnapshot,
) -> Result<Arc<dyn crate::llm::LlmFormatter>> {
    let kind = llm_engine_kind(&settings.string("llm_engine", "ollama"))?;
    let (ollama, groq) = llm_configs_from_settings(state, settings);
    LlmFactory::create(kind, ollama, groq, RuleBasedConfig::default())
}

/// Ollama and Groq LLM configs with the user's model and API key.
pub fn llm_configs_from_settings(
    state: &AppStateInner,
    settings: &SettingsSnapshot,
) -> (OllamaConfig, GroqLlmConfig) {
    let ollama = OllamaConfig {
        model: settings.string("llm_model", "qwen2.5:3b"),
        ..Default::default()
    };
    let groq = GroqLlmConfig {
        api_key: decrypted_api_key_from_settings(state, settings).unwrap_or_default(),
        ..Default::default()
    };
    (ollama, groq)
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
    build_stt_config_for_language(db, settings.string("stt_language", "auto"))
}

pub fn build_stt_config_for_language(db: &Database, language: String) -> SttConfig {
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

/// Escape a value for CSV: neutralize spreadsheet formula injection, then
/// quote and flatten newlines. A leading `=`, `+`, `-`, `@`, tab, or CR would
/// otherwise be interpreted as a formula by Excel/Sheets.
pub fn csv_escape(s: &str) -> String {
    let needs_formula_guard = s
        .chars()
        .next()
        .is_some_and(|c| matches!(c, '=' | '+' | '-' | '@' | '\t' | '\r'));
    let body = s.replace('"', "\"\"").replace('\r', "").replace('\n', " ");
    if needs_formula_guard {
        format!("'{body}")
    } else {
        body
    }
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
///
/// `session_generation` identifies the recording this audio belongs to.
/// Completion and failure transitions are ignored when a newer session has
/// started, so a delayed batch cannot destroy a live recording.
pub async fn process_audio<R: Runtime>(
    app: AppHandle<R>,
    audio: Vec<f32>,
    session_generation: u64,
) {
    let state = app.state::<AppStateInner>();

    if audio.is_empty() {
        finish_session(&state.pipeline, session_generation, "empty audio");
        events::emit_state(&app, state.pipeline.state_tag());
        crate::overlay::reset_idle_timer(&state);
        crate::overlay::maybe_hide(&app);
        return;
    }

    let settings = match SettingsSnapshot::load(&state.db) {
        Ok(s) => s,
        Err(e) => return fail_session(&app, &state.pipeline, session_generation, &e),
    };

    let stt = match build_stt_from_settings(&state, &settings) {
        Ok(e) => e,
        Err(e) => return fail_session(&app, &state.pipeline, session_generation, &e),
    };
    let stt_config = build_stt_config_from_settings(&state.db, &settings);

    // Command mode: transcribe first so we can intercept editing commands.
    let command_mode = settings.bool("command_mode", false);
    let mut precomputed = None;
    if command_mode {
        match stt.transcribe(&audio, &stt_config).await {
            Ok(tr) => {
                if let Some(cmd) = crate::injection::VoiceCommand::from_phrase(&tr.text) {
                    let expected_window = state.pipeline.active_window_id();
                    let current_window = crate::active_window::foreground_window_id();
                    if let Err(e) = crate::injection::command::verify_target_window(
                        expected_window,
                        current_window,
                    ) {
                        return fail_session(&app, &state.pipeline, session_generation, &e);
                    }
                    // enigo drives the OS input queue and must not run on a tokio
                    // worker thread; offload it to the blocking pool like capture init.
                    let executed = tokio::task::spawn_blocking(move || {
                        crate::injection::command::execute(cmd)
                    })
                    .await
                    .unwrap_or_else(|join_err| {
                        Err(AppError::injection(format!(
                            "Command execution task failed: {join_err}"
                        )))
                    });
                    if let Err(e) = executed {
                        return fail_session(&app, &state.pipeline, session_generation, &e);
                    }
                    finish_session(&state.pipeline, session_generation, "command");
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
            Err(e) => return fail_session(&app, &state.pipeline, session_generation, &e),
        }
    }

    let llm = match build_llm_from_settings(&state, &settings) {
        Ok(l) => l,
        Err(e) => return fail_session(&app, &state.pipeline, session_generation, &e),
    };
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
                stt_confidence: out.transcription.confidence,
                llm_engine: Some(out.llm_engine.clone()),
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

            if let Err(e) = state.pipeline.finish_processing(session_generation) {
                tracing::error!("Failed to finish processing transcription: {e}");
            }
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
            fail_session(&app, &state.pipeline, session_generation, &e)
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

/// Fail the batch only when it still belongs to the current session.
///
/// A stale batch (its session was superseded by a new recording) logs and
/// returns without emitting an error or changing state, so a late failure
/// cannot clobber a live recording.
fn fail_session<R: Runtime>(
    app: &AppHandle<R>,
    pipeline: &PipelineOrchestrator,
    session_generation: u64,
    e: &AppError,
) {
    if pipeline.set_error_if_generation(session_generation, e) {
        tracing::error!("Pipeline error: {e}");
        events::emit_transcription_error(app, &e.message, &format!("{:?}", e.code));
        events::emit_state(app, pipeline.state_tag());
        let state = app.state::<AppStateInner>();
        crate::overlay::reset_idle_timer(&state);
        hide_overlay_soon(app.clone());
    } else {
        tracing::debug!("Ignoring stale pipeline error from superseded session: {e}");
    }
}

/// Complete processing for the current session, ignoring a stale completion.
fn finish_session(pipeline: &PipelineOrchestrator, session_generation: u64, context: &str) {
    match pipeline.finish_processing(session_generation) {
        Ok(true) => {}
        Ok(false) => tracing::debug!("Ignoring stale completion for {context}"),
        Err(e) => tracing::error!("Failed to finish processing ({context}): {e}"),
    }
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
    let started_at = Instant::now();
    let state = app.state::<AppStateInner>();

    // A recording must never run beside the onboarding meter: two emitters on
    // the same `audio_level` event would fight. Stop the meter on entry so an
    // open device is released early, and again once the transition succeeds:
    // the second stop bumps the meter generation after the state change, so a
    // meter that raced its way in while this start was in flight is cancelled
    // and any later commit is rejected as stale.
    super::misc::stop_mic_meter(app);

    let active_app = crate::active_window::foreground_process_name();
    // Capture the HWND too: command-mode verification compares the exact window,
    // not just the process name, so two windows of one app cannot be confused.
    let active_window_id = crate::active_window::foreground_window_id();
    // Atomic check-and-start: a concurrent start (hotkey plus IPC) that loses
    // the race is benign, not a user-facing error.
    let Some(tag) = state
        .pipeline
        .try_start_recording(active_app, active_window_id)
    else {
        tracing::debug!("Ignoring duplicate start: pipeline is not startable");
        return;
    };
    super::misc::stop_mic_meter(app);

    state
        .widget_timer
        .hidden_by_timeout
        .store(false, Ordering::SeqCst);
    crate::overlay::reset_idle_timer(&state);
    crate::overlay::ensure_visible(app);
    events::emit_state(app, tag);
    // Acknowledge the key press immediately; the ready cue follows once the
    // microphone stream is actually live.
    if sound_cues_enabled(&state.db) {
        crate::sound::play(crate::sound::Cue::Press);
    }
    spawn_level_emitter(app.clone());
    spawn_capture_task(app.clone(), started_at);
}

fn spawn_capture_task<R: Runtime>(app: AppHandle<R>, started_at: Instant) {
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

        handle_capture_start_result(&app, capture_result, started_at);
    });
}

fn handle_capture_start_result<R: Runtime>(
    app: &AppHandle<R>,
    capture_result: std::result::Result<Result<bool>, tokio::task::JoinError>,
    started_at: Instant,
) {
    let state = app.state::<AppStateInner>();
    match capture_result {
        Ok(Ok(true)) => announce_capture_ready(&state, started_at),
        Ok(Ok(false)) => tracing::debug!("Capture start skipped: recording already ended"),
        Ok(Err(e)) => {
            let _ = state.pipeline.cancel_recording();
            fail(app, &state.pipeline, &e);
        }
        Err(join_err) => {
            let err = AppError::audio(format!("Capture initialization task failed: {join_err}"));
            let _ = state.pipeline.cancel_recording();
            fail(app, &state.pipeline, &err);
        }
    }
}

/// Announce a live microphone stream: log the hotkey-to-ready latency and play
/// the ready cue, so the cue marks when the user can actually begin speaking.
/// Only reached when capture started successfully; a recording that already
/// ended takes the `Ok(Ok(false))` path and stays silent.
fn announce_capture_ready(state: &AppStateInner, started_at: Instant) {
    let latency_ms = started_at.elapsed().as_millis() as u64;
    tracing::debug!(latency_ms, "Microphone stream ready");
    if sound_cues_enabled(&state.db) {
        crate::sound::play(crate::sound::Cue::Ready);
    }
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

    // Atomic check-and-stop: a concurrent stop (hotkey plus IPC plus auto-stop)
    // that loses the race is benign, not a user-facing error.
    let (audio, session_generation) = match state.pipeline.try_stop_recording() {
        Ok(Some(session)) => session,
        Ok(None) => {
            tracing::debug!("Ignoring duplicate stop: pipeline is not recording");
            return;
        }
        Err(e) => return fail(app, &state.pipeline, &e),
    };
    crate::overlay::reset_idle_timer(&state);

    // Recordings without sustained voice are taps or clicks, not speech.
    if !has_enough_voice(&audio, TARGET_SAMPLE_RATE) {
        return discard_silent_recording(app, &state, session_generation);
    }

    if sound_cues_enabled(&state.db) {
        crate::sound::play(crate::sound::Cue::Stop);
    }
    events::emit_state(app, state.pipeline.state_tag());
    let app2 = app.clone();
    tauri::async_runtime::spawn(async move {
        process_audio(app2, audio, session_generation).await;
    });
}

/// Return to idle after discarding audio that holds no usable speech.
/// Mirrors the cancel path: no STT, no stop cue. Emits a notice so the user
/// learns the recording was discarded rather than silently lost.
fn discard_silent_recording<R: Runtime>(
    app: &AppHandle<R>,
    state: &AppStateInner,
    session_generation: u64,
) {
    tracing::debug!("Discarding recording without sustained voice");
    events::emit_notice(app, "no_speech_detected", "No speech detected");
    finish_session(&state.pipeline, session_generation, "discarded audio");
    events::emit_state(app, state.pipeline.state_tag());
    crate::overlay::maybe_hide(app);
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
    fn stt_cache_key_hashes_api_key_and_is_stable() {
        let whisper = WhisperCppConfig::default();
        let key_a = stt_cache_key(SttEngineKind::Groq, "secret-a", "model", &whisper);
        let key_a_again = stt_cache_key(SttEngineKind::Groq, "secret-a", "model", &whisper);
        let key_b = stt_cache_key(SttEngineKind::Groq, "secret-b", "model", &whisper);

        // Same key -> stable; different key -> different cache key.
        assert_eq!(key_a, key_a_again);
        assert_ne!(key_a, key_b);
        // The raw secret never appears in the cache key.
        assert!(!key_a.contains("secret-a"));
        assert!(!key_b.contains("secret-b"));
    }

    #[test]
    fn stt_engine_kind_maps_known_and_rejects_unknown() {
        assert_eq!(stt_engine_kind("groq").unwrap(), SttEngineKind::Groq);
        assert_eq!(
            stt_engine_kind("whisper_cpp").unwrap(),
            SttEngineKind::WhisperCpp
        );
        let err = stt_engine_kind("garbage").unwrap_err();
        assert_eq!(err.code, crate::error::ErrorCode::InvalidInput);
    }

    #[test]
    fn llm_engine_kind_maps_known_and_rejects_unknown() {
        assert_eq!(llm_engine_kind("off").unwrap(), LlmEngineKind::Off);
        assert_eq!(llm_engine_kind("ollama").unwrap(), LlmEngineKind::Ollama);
        assert_eq!(llm_engine_kind("groq").unwrap(), LlmEngineKind::Groq);
        assert_eq!(
            llm_engine_kind("rule_based").unwrap(),
            LlmEngineKind::RuleBased
        );
        let err = llm_engine_kind("gpt").unwrap_err();
        assert_eq!(err.code, crate::error::ErrorCode::InvalidInput);
    }

    #[test]
    fn whisper_threads_are_clamped_to_range() {
        let snapshot = |threads: u32| {
            let mut map = serde_json::Map::new();
            map.insert(
                "whisper_cpp_threads".to_string(),
                serde_json::json!(threads),
            );
            SettingsSnapshot::new(map)
        };
        assert_eq!(whisper_cpp_config_from_settings(&snapshot(0)).threads, 1);
        assert_eq!(whisper_cpp_config_from_settings(&snapshot(8)).threads, 8);
        assert_eq!(
            whisper_cpp_config_from_settings(&snapshot(9999)).threads,
            32
        );
    }

    #[test]
    fn clamp_whisper_threads_bounds_input() {
        assert_eq!(clamp_whisper_threads(0), MIN_WHISPER_THREADS);
        assert_eq!(
            clamp_whisper_threads(MIN_WHISPER_THREADS),
            MIN_WHISPER_THREADS
        );
        assert_eq!(clamp_whisper_threads(8), 8);
        assert_eq!(
            clamp_whisper_threads(MAX_WHISPER_THREADS),
            MAX_WHISPER_THREADS
        );
        assert_eq!(clamp_whisper_threads(u32::MAX), MAX_WHISPER_THREADS);
    }

    #[test]
    fn csv_escape_guards_formula_leads_and_normalizes() {
        // Formula-leading characters get a single-quote prefix.
        for lead in ["=1+1", "+SUM(A1)", "-2+3", "@cmd", "\ttab", "\rcr"] {
            assert!(
                csv_escape(lead).starts_with('\''),
                "expected guard for {lead:?}"
            );
        }
        // Normal cells are untouched except for quote doubling and newline flattening.
        assert_eq!(csv_escape("hello"), "hello");
        assert_eq!(csv_escape("a\"b"), "a\"\"b");
        assert_eq!(csv_escape("a\nb"), "a b");
        // A formula char not at the start is not a lead and stays as-is.
        assert_eq!(csv_escape("a=b"), "a=b");
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
            mic_meter: crate::audio::MicMeterState::new(),
        }
    }

    /// A scratch binary file and its canonical clean-path string.
    fn scratch_binary(name: &str) -> (PathBuf, String) {
        let dir = std::env::temp_dir().join(format!("voxitype-gate-{name}-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("whisper-cli.exe");
        std::fs::write(&file, b"stub").unwrap();
        let canonical = file.canonicalize().unwrap();
        let clean = crate::commands::misc::canonical_to_clean_string(&canonical);
        (canonical, clean)
    }

    fn whisper_config(binary_path: String) -> WhisperCppConfig {
        WhisperCppConfig {
            binary_path,
            model_path: "model.bin".to_string(),
            threads: 4,
        }
    }

    #[test]
    fn whisper_binary_gate_accepts_matching_token() {
        let state = test_app_state([21u8; 32]);
        let (_canonical, clean) = scratch_binary("match");
        let token = crate::crypto::seal_token(&clean, &state.master_key).unwrap();
        SettingsManager::new(&state.db)
            .set(WHISPER_BINARY_TOKEN_KEY, &token)
            .unwrap();

        assert!(verify_whisper_binary(&state, &whisper_config(clean)).is_ok());
    }

    #[test]
    fn whisper_binary_gate_rejects_missing_token() {
        let state = test_app_state([22u8; 32]);
        let (_canonical, clean) = scratch_binary("missing");
        let err = verify_whisper_binary(&state, &whisper_config(clean)).unwrap_err();
        assert_eq!(err.code, ErrorCode::SttEngineError);
        assert!(err.message.contains("Settings"));
    }

    #[test]
    fn whisper_binary_gate_rejects_tampered_path() {
        let state = test_app_state([23u8; 32]);
        let (_canonical, clean) = scratch_binary("tampered");
        // Token vouches for a different (planted) binary than the stored path.
        let token =
            crate::crypto::seal_token("C:\\attacker\\payload.exe", &state.master_key).unwrap();
        SettingsManager::new(&state.db)
            .set(WHISPER_BINARY_TOKEN_KEY, &token)
            .unwrap();

        let err = verify_whisper_binary(&state, &whisper_config(clean)).unwrap_err();
        assert_eq!(err.code, ErrorCode::SttEngineError);
        assert!(err.message.contains("does not match"));
    }

    #[test]
    fn whisper_binary_gate_rejects_undecryptable_token() {
        let state = test_app_state([24u8; 32]);
        let (_canonical, clean) = scratch_binary("badtoken");
        let token = crate::crypto::seal_token(&clean, &[99u8; 32]).unwrap();
        SettingsManager::new(&state.db)
            .set(WHISPER_BINARY_TOKEN_KEY, &token)
            .unwrap();

        let err = verify_whisper_binary(&state, &whisper_config(clean)).unwrap_err();
        assert_eq!(err.code, ErrorCode::SttEngineError);
        assert!(err.message.contains("token is invalid"));
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
            build_llm(&state).unwrap().name(),
            build_llm_from_settings(&state, &snapshot).unwrap().name()
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
            build_llm(&state).unwrap().name(),
            build_llm_from_settings(&state, &snapshot).unwrap().name()
        );
    }

    #[test]
    fn settings_snapshot_identical_to_per_key_reads_fully_populated_db() {
        let master_key = [5u8; 32];
        let state = test_app_state(master_key);
        let secret_key = "gsk_full_test_key_xyz123";
        let encrypted = crate::crypto::encrypt_api_key(secret_key, &master_key).unwrap();

        // The whisper.cpp engine gate requires a picker-issued token matching
        // the stored binary's canonical path, so use a real scratch binary.
        let (_canonical, binary) = scratch_binary("snapshot");

        let mgr = SettingsManager::new(&state.db);
        mgr.set("stt_engine", &"whisper_cpp").unwrap();
        mgr.set("stt_model", &"ggml-base.bin").unwrap();
        mgr.set("whisper_cpp_binary_path", &binary).unwrap();
        mgr.set("whisper_cpp_model_path", &"/models/base.bin")
            .unwrap();
        mgr.set(
            WHISPER_BINARY_TOKEN_KEY,
            &crate::crypto::seal_token(&binary, &master_key).unwrap(),
        )
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
            binary
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
            build_llm(&state).unwrap().name(),
            build_llm_from_settings(&state, &snapshot).unwrap().name()
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

    #[test]
    fn decrypt_api_key_str_roundtrips_trimmed() {
        let key = [4u8; 32];
        let enc = crate::crypto::encrypt_api_key("key123", &key).unwrap();
        assert_eq!(decrypt_api_key_str(&enc, &key).unwrap(), "key123");
    }

    #[test]
    fn decrypt_api_key_str_trims_legacy_whitespace() {
        let key = [7u8; 32];
        // A legacy row encrypted before the write-side trim still carries spaces.
        let enc = crate::crypto::encrypt_api_key("  key123  ", &key).unwrap();
        assert_eq!(decrypt_api_key_str(&enc, &key).unwrap(), "key123");
    }

    #[test]
    fn decrypt_api_key_str_keeps_empty() {
        assert_eq!(decrypt_api_key_str("", &[8u8; 32]).unwrap(), "");
    }
}
