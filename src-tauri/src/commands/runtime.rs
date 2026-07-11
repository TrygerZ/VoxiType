//! Shared runtime helpers for STT/LLM config building and the core
//! recording → process → inject → persist pipeline.

use std::sync::Arc;

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

// ---------------------------------------------------------------
// Settings-derived config builders
// ---------------------------------------------------------------

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

pub fn decrypted_api_key(state: &AppStateInner) -> String {
    let raw = string_setting(&state.db, "groq_api_key", "");
    crate::crypto::decrypt_api_key(&raw, &state.master_key).unwrap_or(raw)
}

pub fn build_stt(state: &AppStateInner) -> Result<Arc<dyn crate::stt::SttEngine>> {
    let api_key = decrypted_api_key(state);
    let kind = stt_engine_kind(&string_setting(&state.db, "stt_engine", "groq"));
    let mut model = string_setting(&state.db, "stt_model", "whisper-large-v3-turbo");
    if model == "small" || model.trim().is_empty() {
        model = "whisper-large-v3-turbo".to_string();
    }
    let whisper_cpp = WhisperCppConfig {
        binary_path: string_setting(&state.db, "whisper_cpp_binary_path", "whisper-cli"),
        model_path: string_setting(&state.db, "whisper_cpp_model_path", ""),
        threads: u32_setting(&state.db, "whisper_cpp_threads", 4),
    };
    let cache_key = stt_cache_key(kind, &api_key, &model, &whisper_cpp);

    let mut cache = state.stt_engine.lock_recover();
    if let Some((cached_kind, cached_key, cached_engine)) = &*cache {
        if *cached_kind == kind && *cached_key == cache_key {
            return Ok(cached_engine.clone());
        }
    }

    let groq = GroqSttConfig {
        api_key: api_key.clone(),
        model: model.clone(),
        language: string_setting(&state.db, "stt_language", "auto"),
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

pub fn build_stt_config(db: &Database) -> SttConfig {
    let language = string_setting(db, "stt_language", "auto");
    // When auto-detecting, skip hotwords to avoid biasing the STT model
    // toward a specific language.
    let hotwords = if language == "auto" {
        Vec::new()
    } else {
        DictionaryRepository::new(db)
            .get_hotwords_by_language(&language)
            .unwrap_or_default()
    };
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

pub fn active_mode(db: &Database, active_app: Option<&str>) -> LlmMode {
    let per_app_on = SettingsManager::new(db)
        .get::<bool>("per_app_mode")
        .ok()
        .flatten()
        .unwrap_or(false);
    if per_app_on {
        if let Some(proc) = active_app {
            if let Ok(Some(mode_id)) = crate::storage::PerAppModeRepository::new(db).mode_for(proc)
            {
                return LlmMode::from_id(&mode_id);
            }
        }
    }
    LlmMode::from_id(&string_setting(db, "active_mode", "dictation"))
}

pub fn sound_cues_enabled(db: &Database) -> bool {
    SettingsManager::new(db)
        .get::<bool>("sound_cues")
        .ok()
        .flatten()
        .unwrap_or(false)
}

pub fn telemetry_enabled(db: &Database) -> bool {
    SettingsManager::new(db)
        .get::<bool>("telemetry")
        .ok()
        .flatten()
        .unwrap_or(false)
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
        crate::overlay::maybe_hide(&app);
        return;
    }

    let stt = match build_stt(&state) {
        Ok(e) => e,
        Err(e) => return fail(&app, &state.pipeline, &e),
    };
    let stt_config = build_stt_config(&state.db);

    // Command mode: transcribe first so we can intercept editing commands.
    let command_mode = SettingsManager::new(&state.db)
        .get::<bool>("command_mode")
        .ok()
        .flatten()
        .unwrap_or(false);
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
                        0,
                    );
                    events::emit_state(&app, state.pipeline.state_tag());
                    hide_overlay_soon(app.clone());
                    return;
                }
                precomputed = Some(tr);
            }
            Err(e) => return fail(&app, &state.pipeline, &e),
        }
    }

    let llm = build_llm(&state);
    let active_app = state.pipeline.active_app();
    let mode = active_mode(&state.db, active_app.as_deref());
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
    let injector = HybridInjector::new();

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
            events::emit_transcription_complete(
                &app,
                &id,
                &out.formatted_text,
                word_count,
                out.transcription.duration_ms as i64,
            );
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

pub fn fail<R: Runtime>(app: &AppHandle<R>, pipeline: &PipelineOrchestrator, e: &AppError) {
    tracing::error!("Pipeline error: {e}");
    pipeline.set_error(e);
    events::emit_transcription_error(app, &e.message, &format!("{:?}", e.code));
    events::emit_state(app, pipeline.state_tag());
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
    crate::overlay::ensure_visible(app);
    events::emit_state(app, tag);
    spawn_level_emitter(app.clone());

    let app_clone = app.clone();
    tauri::async_runtime::spawn(async move {
        let state = app_clone.state::<AppStateInner>();
        let config = build_audio_config(&state.db);
        if let Err(e) = state.pipeline.start_capture(&config) {
            let _ = state
                .pipeline
                .apply(crate::pipeline::StateEvent::CancelRecording);
            fail(&app_clone, &state.pipeline, &e);
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
                    if dur.as_secs() > 300 {
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
