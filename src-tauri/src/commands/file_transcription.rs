//! File transcription commands. Jobs run beside hotkey dictation, outside
//! the recording state machine, one job at a time.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, Runtime, State};
use tokio::sync::mpsc;
use uuid::Uuid;

use super::misc::{
    canonical_to_clean_string, ensure_picker_backed_path, pick_directory_blocking,
    pick_gated_files, remember_picker_paths, PICKER_KIND_AUDIO_FILE,
};
use super::runtime::{
    build_stt_config_for_language, build_stt_for_kind, llm_configs_from_settings, SettingsSnapshot,
};
use crate::audio::file_decode::{too_long_error, AudioFile, ChunkSplitter};
use crate::audio::TARGET_SAMPLE_RATE;
use crate::error::{AppError, Result};
use crate::export::{self, ExportFormat};
use crate::llm::{LlmEngineKind, LlmFactory, LlmFormatter, RuleBasedConfig};
use crate::pipeline::file_job::{self, PostOptions, RawTranscript, StageIssue};
use crate::storage::{DictionaryRepository, HistoryRepository, TranscriptionEntry};
use crate::stt::SttEngineKind;
use crate::AppStateInner;

pub const MAX_FILE_DURATION_SECS: u64 = 60 * 60;
/// Files per batch; the frontend queues them through `transcribe_file`.
pub const MAX_BATCH_FILES: usize = 20;
const PICKER_KIND_EXPORT_DIRECTORY: &str = "export_directory";
/// 10 minutes of 16-bit 16 kHz WAV is ~19 MB, under Groq's 25 MB upload cap.
const CHUNK_SECS: usize = 600;
const CUT_SEARCH_SECS: usize = 5;
pub const FILE_HISTORY_MODE: &str = "file";
const SUPPORTED_LANGUAGES: [&str; 3] = ["auto", "id", "en"];
/// A 180-word segment on a local 3B model can take well over the 30 s
/// dictation timeout on CPU-only machines.
const FILE_OLLAMA_TIMEOUT: Duration = Duration::from_secs(120);

/// Tracks the single in-flight file job and the open file picker.
#[derive(Default)]
pub struct FileJobState {
    running: AtomicBool,
    cancel: AtomicBool,
    picking: AtomicBool,
}

/// Holds a busy flag and clears it on every exit path, including panics
/// and early returns.
struct FlagGuard<'a>(&'a AtomicBool);

impl<'a> FlagGuard<'a> {
    fn acquire(flag: &'a AtomicBool) -> Option<Self> {
        flag.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .ok()
            .map(|_| Self(flag))
    }
}

impl Drop for FlagGuard<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::SeqCst);
    }
}

impl FileJobState {
    fn begin(&self) -> Result<FlagGuard<'_>> {
        let guard = FlagGuard::acquire(&self.running)
            .ok_or_else(|| AppError::invalid_input("A file transcription is already running"))?;
        self.cancel.store(false, Ordering::SeqCst);
        Ok(guard)
    }
}

#[derive(Debug, Deserialize)]
pub struct FileTranscriptionRequest {
    pub path: String,
    pub stt_engine: SttEngineKind,
    /// "auto", "id", or "en". An explicit language skips Groq's
    /// auto-detect verification pass, halving uploads for long files.
    pub language: String,
    /// `None` skips LLM formatting.
    pub llm_engine: Option<LlmEngineKind>,
    pub apply_dictionary: bool,
}

#[derive(Debug, Serialize)]
pub struct FileTranscriptionResult {
    pub id: String,
    pub text: String,
    pub word_count: u32,
    pub duration_ms: u64,
    /// Set when transcription stopped early; `text` holds the finished part.
    pub stt_issue: Option<StageIssue>,
    /// Set when some segments were left as raw text.
    pub llm_issue: Option<StageIssue>,
}

/// Opens at most one picker; repeat calls while it is open return no files.
/// The dialog runs in a separate PowerShell process, so without this guard
/// every click would spawn another explorer window.
#[tauri::command]
pub async fn pick_audio_files(
    state: State<'_, AppStateInner>,
    jobs: State<'_, FileJobState>,
) -> std::result::Result<Vec<String>, AppError> {
    let Some(_picking) = FlagGuard::acquire(&jobs.picking) else {
        return Ok(Vec::new());
    };
    pick_gated_files(&state, PICKER_KIND_AUDIO_FILE, MAX_BATCH_FILES).await
}

const EXPORT_DIRECTORY_TITLE: &str = "Select a folder for the exported transcripts";

#[tauri::command]
pub async fn pick_export_directory(
    state: State<'_, AppStateInner>,
) -> std::result::Result<Option<String>, AppError> {
    let picked = tokio::task::spawn_blocking(|| pick_directory_blocking(EXPORT_DIRECTORY_TITLE))
        .await
        .map_err(|e| AppError::internal(format!("Folder picker failed: {e}")))??;
    let Some(path) = picked else {
        return Ok(None);
    };
    let canonical = Path::new(&path)
        .canonicalize()
        .map_err(|e| AppError::invalid_input(format!("Invalid export folder: {e}")))?;
    let clean = canonical_to_clean_string(&canonical);
    remember_picker_paths(&state, PICKER_KIND_EXPORT_DIRECTORY, vec![canonical]);
    Ok(Some(clean))
}

#[derive(Debug, Deserialize)]
pub struct TranscriptExport {
    /// Audio file path; only its file name is used, to name the output.
    pub source_path: String,
    pub text: String,
}

/// Writes one file per transcript into a folder chosen via
/// `pick_export_directory`. Returns the written paths.
#[tauri::command]
pub fn export_transcripts(
    state: State<'_, AppStateInner>,
    directory: String,
    format: ExportFormat,
    items: Vec<TranscriptExport>,
) -> std::result::Result<Vec<String>, AppError> {
    ensure_picker_backed_path(&state, PICKER_KIND_EXPORT_DIRECTORY, &directory)?;
    if items.len() > MAX_BATCH_FILES {
        return Err(AppError::invalid_input(format!(
            "Export at most {MAX_BATCH_FILES} transcripts at a time"
        )));
    }
    let dir = Path::new(&directory);
    items
        .iter()
        .map(|item| {
            export::write_transcript(dir, &item.source_path, format, &item.text)
                .map(|p| canonical_to_clean_string(&p))
        })
        .collect()
}

#[tauri::command]
pub fn cancel_file_transcription(jobs: State<'_, FileJobState>) {
    jobs.cancel.store(true, Ordering::SeqCst);
}

#[tauri::command]
pub async fn transcribe_file<R: Runtime>(
    app: AppHandle<R>,
    jobs: State<'_, FileJobState>,
    request: FileTranscriptionRequest,
) -> std::result::Result<FileTranscriptionResult, AppError> {
    let state = app.state::<AppStateInner>();
    ensure_picker_backed_path(&state, PICKER_KIND_AUDIO_FILE, &request.path)?;
    let _guard = jobs.begin()?;
    let result = run_job(&app, &state, &jobs.cancel, &request).await;
    if let Err(e) = &result {
        tracing::warn!("File transcription failed: {e}");
    }
    result
}

/// Build the selected formatter without the dictation fallback wrapper:
/// silently substituting rule-based output would hide failures the user
/// asked to be told about, and per-segment fallbacks after a timeout would
/// multiply the wait across every segment.
fn build_file_formatter(
    state: &AppStateInner,
    settings: &SettingsSnapshot,
    kind: LlmEngineKind,
) -> Arc<dyn LlmFormatter> {
    let (mut ollama, groq) = llm_configs_from_settings(state, settings);
    ollama.request_timeout = FILE_OLLAMA_TIMEOUT;
    match kind {
        LlmEngineKind::Ollama => LlmFactory::ollama(ollama),
        LlmEngineKind::Groq => LlmFactory::groq(groq),
        LlmEngineKind::RuleBased => LlmFactory::rule_based(RuleBasedConfig::default()),
        LlmEngineKind::Off => LlmFactory::off(),
    }
}

/// Reject bad input and missing setup before decoding, so a long file is
/// not transcribed only to fail at the cleanup step.
fn validate_request(
    request: &FileTranscriptionRequest,
    state: &AppStateInner,
    settings: &SettingsSnapshot,
) -> Result<()> {
    if !SUPPORTED_LANGUAGES.contains(&request.language.as_str()) {
        return Err(AppError::invalid_input(format!(
            "Unsupported language: {}",
            request.language
        )));
    }
    let groq_key_missing = || {
        llm_configs_from_settings(state, settings)
            .1
            .api_key
            .trim()
            .is_empty()
    };
    if request.llm_engine == Some(LlmEngineKind::Groq) && groq_key_missing() {
        return Err(AppError::llm_api_key_missing("Groq API key is not set"));
    }
    Ok(())
}

async fn run_job<R: Runtime>(
    app: &AppHandle<R>,
    state: &AppStateInner,
    cancel: &AtomicBool,
    request: &FileTranscriptionRequest,
) -> Result<FileTranscriptionResult> {
    let settings = SettingsSnapshot::load(&state.db)?;
    validate_request(request, state, &settings)?;
    let stt = build_stt_for_kind(state, &settings, request.stt_engine)?;
    let formatter = request
        .llm_engine
        .map(|kind| build_file_formatter(state, &settings, kind));
    let stt_config = build_stt_config_for_language(&state.db, request.language.clone());
    let report = |p| crate::events::emit_file_transcription_progress(app, p);

    let raw = transcribe_path(
        PathBuf::from(&request.path),
        stt.clone(),
        &stt_config,
        cancel,
        report,
    )
    .await?;
    if raw.text.trim().is_empty() {
        return Err(AppError::invalid_input("No speech detected in the file"));
    }

    let options = PostOptions {
        replacements: request.apply_dictionary.then(|| {
            DictionaryRepository::new(&state.db)
                .get_replacements()
                .unwrap_or_default()
        }),
        formatter: formatter.clone(),
    };
    let processed = file_job::post_process(&raw, &options, cancel, report).await;

    let entry = history_entry(
        request,
        &raw,
        &processed.text,
        stt.name(),
        formatter.map(|f| f.name()),
    );
    HistoryRepository::new(&state.db).insert(&entry)?;
    Ok(FileTranscriptionResult {
        id: entry.id,
        word_count: entry.word_count as u32,
        text: processed.text,
        duration_ms: raw.duration_ms,
        stt_issue: raw.stt_issue,
        llm_issue: processed.llm_issue,
    })
}

/// Decode on a blocking thread while transcribing chunks as they arrive, so
/// at most two chunks are held in memory regardless of file length.
async fn transcribe_path(
    path: PathBuf,
    stt: std::sync::Arc<dyn crate::stt::SttEngine>,
    config: &crate::stt::SttConfig,
    cancel: &AtomicBool,
    report: impl Fn(file_job::Progress),
) -> Result<RawTranscript> {
    let file = tokio::task::spawn_blocking(move || AudioFile::open(&path))
        .await
        .map_err(|e| AppError::internal(format!("Audio open task failed: {e}")))??;
    let total_chunks = match file.duration_secs {
        Some(secs) if secs > MAX_FILE_DURATION_SECS as f64 => {
            return Err(too_long_error(MAX_FILE_DURATION_SECS))
        }
        Some(secs) => (secs / CHUNK_SECS as f64).ceil().max(1.0) as u32,
        None => 0,
    };

    let (tx, rx) = mpsc::channel(1);
    let decode = tokio::task::spawn_blocking(move || decode_into_chunks(file, tx));
    let transcript =
        file_job::transcribe_chunks(rx, stt, config, cancel, total_chunks, report).await;
    decode
        .await
        .map_err(|e| AppError::internal(format!("Audio decode task failed: {e}")))??;
    transcript
}

fn decode_into_chunks(file: AudioFile, tx: mpsc::Sender<Vec<f32>>) -> Result<()> {
    let rate = TARGET_SAMPLE_RATE as usize;
    let mut splitter = ChunkSplitter::new(CHUNK_SECS * rate, CUT_SEARCH_SECS * rate);
    let decoded = file.decode_16k_mono(MAX_FILE_DURATION_SECS, |samples| {
        for chunk in splitter.push(&samples) {
            tx.blocking_send(chunk).map_err(|_| consumer_gone())?;
        }
        Ok(())
    });
    match decoded {
        // The consumer already failed or was cancelled; its error is the one to report.
        Err(e) if e.message == CONSUMER_GONE => return Ok(()),
        other => other?,
    }
    if let Some(tail) = splitter.finish() {
        let _ = tx.blocking_send(tail);
    }
    Ok(())
}

const CONSUMER_GONE: &str = "transcription consumer stopped";

fn consumer_gone() -> AppError {
    AppError::internal(CONSUMER_GONE)
}

fn file_name(path: &str) -> Option<String> {
    Path::new(path)
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
}

fn history_entry(
    request: &FileTranscriptionRequest,
    raw: &RawTranscript,
    text: &str,
    stt_name: &str,
    llm_name: Option<&str>,
) -> TranscriptionEntry {
    TranscriptionEntry {
        id: Uuid::new_v4().to_string(),
        created_at: String::new(),
        text_raw: raw.text.clone(),
        text_formatted: text.to_string(),
        source_lang: raw.language.clone(),
        target_lang: None,
        mode: FILE_HISTORY_MODE.to_string(),
        stt_engine: stt_name.to_string(),
        stt_confidence: None,
        llm_engine: llm_name.map(str::to_string),
        duration_ms: Some(raw.duration_ms as i64),
        word_count: text.split_whitespace().count() as i64,
        character_count: text.chars().count() as i64,
        is_pinned: false,
        app_context: file_name(&request.path),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_one_job_runs_at_a_time_and_guard_releases() {
        let jobs = FileJobState::default();
        let guard = jobs.begin().unwrap();
        assert!(jobs.begin().is_err());
        drop(guard);
        assert!(jobs.begin().is_ok());
    }

    #[test]
    fn begin_clears_a_stale_cancel_flag() {
        let jobs = FileJobState::default();
        jobs.cancel.store(true, Ordering::SeqCst);
        let _guard = jobs.begin().unwrap();
        assert!(!jobs.cancel.load(Ordering::SeqCst));
    }

    #[test]
    fn second_picker_is_refused_while_one_is_open() {
        let jobs = FileJobState::default();
        let open = FlagGuard::acquire(&jobs.picking).unwrap();
        assert!(FlagGuard::acquire(&jobs.picking).is_none());
        drop(open);
        assert!(FlagGuard::acquire(&jobs.picking).is_some());
    }

    #[test]
    fn request_deserializes_engine_ids_used_by_the_frontend() {
        let req: FileTranscriptionRequest = serde_json::from_str(
            r#"{"path":"a.mp3","stt_engine":"whisper_cpp","language":"id","llm_engine":"rule_based","apply_dictionary":true}"#,
        )
        .unwrap();
        assert_eq!(req.stt_engine, SttEngineKind::WhisperCpp);
        assert_eq!(req.llm_engine, Some(LlmEngineKind::RuleBased));
        assert!(SUPPORTED_LANGUAGES.contains(&req.language.as_str()));
    }

    #[test]
    fn export_request_deserializes_frontend_payload() {
        let format: ExportFormat = serde_json::from_str(r#""docx""#).unwrap();
        let items: Vec<TranscriptExport> =
            serde_json::from_str(r#"[{"source_path":"C:\\a\\b.mp3","text":"halo"}]"#).unwrap();
        assert_eq!(format, ExportFormat::Docx);
        assert_eq!(items[0].text, "halo");
    }

    #[test]
    fn chunk_fits_groq_upload_limit() {
        const GROQ_UPLOAD_LIMIT_BYTES: usize = 25 * 1024 * 1024;
        let wav_bytes = 44 + CHUNK_SECS * TARGET_SAMPLE_RATE as usize * 2;
        assert!(wav_bytes < GROQ_UPLOAD_LIMIT_BYTES);
    }
}
