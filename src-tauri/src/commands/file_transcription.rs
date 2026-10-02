//! File transcription commands. Jobs run beside hotkey dictation, outside
//! the recording state machine, one job at a time.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, Runtime, State};
use tokio::sync::mpsc;
use uuid::Uuid;

use super::misc::{ensure_picker_backed_path, PICKER_KIND_AUDIO_FILE};
use super::runtime::{
    build_llm_for_kind, build_stt_config_from_settings, build_stt_for_kind, SettingsSnapshot,
};
use crate::audio::file_decode::{too_long_error, AudioFile, ChunkSplitter};
use crate::audio::TARGET_SAMPLE_RATE;
use crate::error::{AppError, Result};
use crate::llm::LlmEngineKind;
use crate::pipeline::file_job::{self, PostOptions, RawTranscript};
use crate::storage::{DictionaryRepository, HistoryRepository, TranscriptionEntry};
use crate::stt::SttEngineKind;
use crate::AppStateInner;

pub const MAX_FILE_DURATION_SECS: u64 = 60 * 60;
/// 10 minutes of 16-bit 16 kHz WAV is ~19 MB, under Groq's 25 MB upload cap.
const CHUNK_SECS: usize = 600;
const CUT_SEARCH_SECS: usize = 5;
pub const FILE_HISTORY_MODE: &str = "file";

/// Tracks the single in-flight file job.
#[derive(Default)]
pub struct FileJobState {
    running: AtomicBool,
    cancel: AtomicBool,
}

/// Clears `running` on every exit path, including panics and early returns.
struct RunningGuard<'a>(&'a FileJobState);

impl Drop for RunningGuard<'_> {
    fn drop(&mut self) {
        self.0.running.store(false, Ordering::SeqCst);
    }
}

impl FileJobState {
    fn begin(&self) -> Result<RunningGuard<'_>> {
        self.running
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .map_err(|_| AppError::invalid_input("A file transcription is already running"))?;
        self.cancel.store(false, Ordering::SeqCst);
        Ok(RunningGuard(self))
    }
}

#[derive(Debug, Deserialize)]
pub struct FileTranscriptionRequest {
    pub path: String,
    pub stt_engine: SttEngineKind,
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
}

#[tauri::command]
pub async fn pick_audio_file(
    state: State<'_, AppStateInner>,
) -> std::result::Result<Option<String>, AppError> {
    super::misc::pick_setup_file(state, PICKER_KIND_AUDIO_FILE.to_string()).await
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

async fn run_job<R: Runtime>(
    app: &AppHandle<R>,
    state: &AppStateInner,
    cancel: &AtomicBool,
    request: &FileTranscriptionRequest,
) -> Result<FileTranscriptionResult> {
    let settings = SettingsSnapshot::load(&state.db)?;
    let stt = build_stt_for_kind(state, &settings, request.stt_engine)?;
    let stt_config = build_stt_config_from_settings(&state.db, &settings);
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

    let formatter = request
        .llm_engine
        .map(|kind| build_llm_for_kind(state, &settings, kind));
    let options = PostOptions {
        replacements: request.apply_dictionary.then(|| {
            DictionaryRepository::new(&state.db)
                .get_replacements()
                .unwrap_or_default()
        }),
        formatter: formatter.clone(),
    };
    let text = file_job::post_process(&raw, &options, cancel, report).await?;

    let entry = history_entry(
        request,
        &raw,
        &text,
        stt.name(),
        formatter.map(|f| f.name()),
    );
    HistoryRepository::new(&state.db).insert(&entry)?;
    Ok(FileTranscriptionResult {
        id: entry.id,
        word_count: entry.word_count as u32,
        text,
        duration_ms: raw.duration_ms,
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
    fn request_deserializes_engine_ids_used_by_the_frontend() {
        let req: FileTranscriptionRequest = serde_json::from_str(
            r#"{"path":"a.mp3","stt_engine":"whisper_cpp","llm_engine":"rule_based","apply_dictionary":true}"#,
        )
        .unwrap();
        assert_eq!(req.stt_engine, SttEngineKind::WhisperCpp);
        assert_eq!(req.llm_engine, Some(LlmEngineKind::RuleBased));
    }

    #[test]
    fn chunk_fits_groq_upload_limit() {
        const GROQ_UPLOAD_LIMIT_BYTES: usize = 25 * 1024 * 1024;
        let wav_bytes = 44 + CHUNK_SECS * TARGET_SAMPLE_RATE as usize * 2;
        assert!(wav_bytes < GROQ_UPLOAD_LIMIT_BYTES);
    }
}
