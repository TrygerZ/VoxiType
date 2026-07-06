//! Local whisper.cpp CLI STT engine.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use async_trait::async_trait;
use uuid::Uuid;

use super::groq_stt::encode_wav_16k_mono;
use super::types::WhisperCppConfig;
use super::{SttConfig, SttEngine, TranscriptionResult};
use crate::error::{AppError, Result};

#[cfg(windows)]
use std::os::windows::process::CommandExt;

const SAMPLE_RATE: u64 = 16_000;
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;

pub struct WhisperCppEngine {
    config: WhisperCppConfig,
}

impl WhisperCppEngine {
    pub fn new(config: WhisperCppConfig) -> Self {
        Self { config }
    }
}

#[async_trait]
impl SttEngine for WhisperCppEngine {
    async fn transcribe(&self, audio: &[f32], config: &SttConfig) -> Result<TranscriptionResult> {
        validate_config(&self.config)?;

        let run = WhisperRun {
            binary_path: self.config.binary_path.clone(),
            model_path: self.config.model_path.clone(),
            threads: self.config.threads.max(1),
            language: config.language.clone(),
            prompt: config.initial_prompt.clone(),
            temperature: config.temperature,
            audio: audio.to_vec(),
        };
        let duration_ms = audio.len() as u64 * 1000 / SAMPLE_RATE;
        let text = tokio::task::spawn_blocking(move || run_whisper(run))
            .await
            .map_err(|e| AppError::stt(format!("whisper.cpp task failed: {e}")))??;

        Ok(TranscriptionResult {
            text,
            confidence: 1.0,
            language: normalize_language(&config.language),
            duration_ms,
            raw_response: None,
        })
    }

    fn name(&self) -> &'static str {
        "whisper_cpp"
    }
}

struct WhisperRun {
    binary_path: String,
    model_path: String,
    threads: u32,
    language: String,
    prompt: Option<String>,
    temperature: f32,
    audio: Vec<f32>,
}

struct TempRunDir {
    path: PathBuf,
}

impl TempRunDir {
    fn new() -> Result<Self> {
        let path = std::env::temp_dir().join(format!("voxitype-whisper-{}", Uuid::new_v4()));
        fs::create_dir_all(&path)?;
        Ok(Self { path })
    }
}

impl Drop for TempRunDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn validate_config(config: &WhisperCppConfig) -> Result<()> {
    if config.binary_path.trim().is_empty() {
        return Err(AppError::stt("Set the whisper.cpp binary path"));
    }
    if config.model_path.trim().is_empty() {
        return Err(AppError::model_not_found("Set the whisper.cpp model path"));
    }
    let model_path = Path::new(&config.model_path);
    if !model_path.is_file() {
        return Err(AppError::model_not_found(format!(
            "whisper.cpp model not found: {}",
            config.model_path
        )));
    }
    let binary_path = Path::new(&config.binary_path);
    if is_path_like(&config.binary_path) && !binary_path.is_file() {
        return Err(AppError::stt(format!(
            "whisper.cpp binary not found: {}",
            config.binary_path
        )));
    }
    Ok(())
}

fn is_path_like(value: &str) -> bool {
    value.contains('/') || value.contains('\\') || Path::new(value).is_absolute()
}

fn run_whisper(run: WhisperRun) -> Result<String> {
    let dir = TempRunDir::new()?;
    let wav_path = dir.path.join("audio.wav");
    let output_prefix = dir.path.join("transcript");
    fs::write(&wav_path, encode_wav_16k_mono(&run.audio))?;

    let mut command = Command::new(&run.binary_path);
    command.args(build_args(&run, &wav_path, &output_prefix));
    #[cfg(windows)]
    command.creation_flags(CREATE_NO_WINDOW);

    let output = command
        .output()
        .map_err(|e| AppError::stt(format!("Failed to run whisper.cpp: {e}")))?;
    if !output.status.success() {
        return Err(AppError::stt(format!(
            "whisper.cpp failed: {}",
            command_error(&output)
        )));
    }

    let txt_path = output_prefix.with_extension("txt");
    match fs::read_to_string(&txt_path) {
        Ok(text) => Ok(clean_text(&text)),
        Err(_) => Ok(clean_text(&String::from_utf8_lossy(&output.stdout))),
    }
}

fn build_args(run: &WhisperRun, wav_path: &Path, output_prefix: &Path) -> Vec<String> {
    let mut args = vec![
        "-m".to_string(),
        run.model_path.clone(),
        "-f".to_string(),
        wav_path.to_string_lossy().into_owned(),
        "-l".to_string(),
        normalize_language_arg(&run.language),
        "-t".to_string(),
        run.threads.to_string(),
        "-tp".to_string(),
        run.temperature.to_string(),
        "-otxt".to_string(),
        "-of".to_string(),
        output_prefix.to_string_lossy().into_owned(),
        "-nt".to_string(),
        "-np".to_string(),
    ];

    if let Some(prompt) = run
        .prompt
        .as_deref()
        .map(str::trim)
        .filter(|p| !p.is_empty())
    {
        args.push("--prompt".to_string());
        args.push(prompt.to_string());
    }

    args
}

fn command_error(output: &std::process::Output) -> String {
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    if stderr.is_empty() {
        String::from_utf8_lossy(&output.stdout).trim().to_string()
    } else {
        stderr
    }
}

fn clean_text(text: &str) -> String {
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

fn normalize_language(language: &str) -> String {
    if language == "auto" {
        "unknown".to_string()
    } else {
        normalize_language_arg(language)
    }
}

fn normalize_language_arg(language: &str) -> String {
    match language.trim() {
        "" => "auto".to_string(),
        lang => lang.to_lowercase(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_run() -> WhisperRun {
        WhisperRun {
            binary_path: "whisper-cli".to_string(),
            model_path: "model.bin".to_string(),
            threads: 2,
            language: "id".to_string(),
            prompt: Some("VoxiType".to_string()),
            temperature: 0.0,
            audio: vec![0.0; 16],
        }
    }

    #[test]
    fn builds_cli_args_for_output_file() {
        let run = sample_run();
        let args = build_args(
            &run,
            Path::new("audio.wav"),
            Path::new("voxitype-transcript"),
        );
        assert!(args.windows(2).any(|w| w == ["-m", "model.bin"]));
        assert!(args.windows(2).any(|w| w == ["-l", "id"]));
        assert!(args.contains(&"-otxt".to_string()));
        assert!(args.contains(&"-np".to_string()));
        assert!(args.windows(2).any(|w| w == ["--prompt", "VoxiType"]));
    }

    #[test]
    fn cleans_multiline_text() {
        assert_eq!(clean_text("\n hello \n\n world \n"), "hello world");
    }

    #[test]
    fn auto_language_is_unknown_in_result() {
        assert_eq!(normalize_language("auto"), "unknown");
        assert_eq!(normalize_language_arg("ID"), "id");
    }

    #[tokio::test]
    async fn transcribe_reads_cli_text_output() {
        let dir = TempRunDir::new().unwrap();
        let binary_path = fake_whisper_cli(&dir.path);
        let model_path = dir.path.join("model.bin");
        fs::write(&model_path, "dummy").unwrap();

        let engine = WhisperCppEngine::new(WhisperCppConfig {
            binary_path: binary_path.to_string_lossy().into_owned(),
            model_path: model_path.to_string_lossy().into_owned(),
            threads: 1,
        });
        let result = engine
            .transcribe(
                &[0.0; 16_000],
                &SttConfig {
                    language: "id".to_string(),
                    initial_prompt: None,
                    temperature: 0.0,
                },
            )
            .await
            .unwrap();

        assert_eq!(result.text, "halo offline");
        assert_eq!(result.language, "id");
        assert_eq!(result.duration_ms, 1000);
    }

    #[cfg(windows)]
    fn fake_whisper_cli(dir: &Path) -> PathBuf {
        let path = dir.join("fake-whisper.cmd");
        fs::write(
            &path,
            r#"@echo off
set out=
:loop
if "%~1"=="" goto done
if "%~1"=="-of" (
  set out=%~2
  shift
)
shift
goto loop
:done
echo halo offline > "%out%.txt"
exit /b 0
"#,
        )
        .unwrap();
        path
    }

    #[cfg(unix)]
    fn fake_whisper_cli(dir: &Path) -> PathBuf {
        use std::os::unix::fs::PermissionsExt;

        let path = dir.join("fake-whisper");
        fs::write(
            &path,
            r#"#!/bin/sh
out=""
while [ "$#" -gt 0 ]; do
  if [ "$1" = "-of" ]; then
    out="$2"
    shift
  fi
  shift
done
printf "halo offline\n" > "$out.txt"
"#,
        )
        .unwrap();
        let mut permissions = fs::metadata(&path).unwrap().permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&path, permissions).unwrap();
        path
    }
}
