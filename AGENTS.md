# VoxiType - Agent Instructions

## Stack
| Layer | Tool | File Pattern |
|-------|------|-------------|
| Desktop | Tauri 2.x | src-tauri/ |
| Frontend | React 19 + Vite 6 + Tailwind 4 | src/ |
| Backend | Rust 1.85+ | src-tauri/src/ |
| State | Zustand 5.x | src/stores/ |
| Storage | SQLite via rusqlite | src-tauri/src/storage/ |
| Audio | cpal + rubato + ringbuf | src-tauri/src/audio/ |
| VAD | Energy-based (default) + Silero ONNX (feature-gated `local-vad`) | src-tauri/src/audio/capture.rs |
| STT | Groq Whisper (REST primary) + whisper.cpp (feature-gated `local-stt`) | src-tauri/src/stt/ |
| LLM | Ollama Qwen2.5 3B, Groq Llama 3.1 8B, rule-based fallback, Off mode | src-tauri/src/llm/ |
| Dictionary | Word/replacement pairs with hotword boosting | src-tauri/src/storage/dictionary.rs |
| Snippets | Trigger phrase expansion | src-tauri/src/storage/snippets.rs |
| Translation | Optional LLM-based translation pipeline | src-tauri/src/llm/ |
| Command Mode | Voice commands → keystroke macros | src-tauri/src/injection/command.rs |
| Text Injection | enigo + arboard (clipboard paste + keystroke fallback) | src-tauri/src/injection/ |
| Hotkey | Tauri global-shortcut plugin | src-tauri/src/hotkey/ |
| Crypto | AES-256-GCM API key encryption at rest | src-tauri/src/crypto.rs |
| Logging | tracing to stderr + daily-rotated files | src-tauri/src/logging.rs |
| Active Window | Win32 API foreground process detection | src-tauri/src/active_window.rs |
| Overlay | Floating widget window control | src-tauri/src/overlay.rs |
| Sound Cues | Start/stop tones | src-tauri/src/sound.rs |
| Updater | GitHub Releases version checker | src-tauri/src/updater.rs |
| Util | Shared HTTP client + retry/backoff | src-tauri/src/util.rs |
| Tray | System tray icon + menu | src-tauri/src/tray/ |

## Commands
| Task | Command |
|------|---------|
| Dev server | npm run tauri dev |
| Build app | npm run tauri build |
| Rust tests | cargo test --no-default-features (in src-tauri/) |
| Rust lint | cargo clippy --no-default-features -- -D warnings |
| TypeScript check | npx tsc --noEmit |
| Frontend build | npm run build |
| Rust build check | cargo check (in src-tauri/) |
| Rust single test | cargo test test_name |

## IPC Commands (31 total across 7 modules)
Commands are registered in `src-tauri/src/commands/mod.rs` and exposed via `lib.rs`.

| Module | Commands |
|--------|----------|
| `recording` | `start_recording`, `stop_recording` |
| `settings` | `get_settings`, `update_setting`, `set_floating_widget_enabled` |
| `history` | `get_history`, `search_history`, `delete_history`, `pin_history`, `clear_history`, `re_inject`, `export_history` |
| `dictionary` | `get_dictionary`, `add_dictionary_word`, `set_dictionary_active`, `delete_dictionary_word`, `export_dictionary`, `import_dictionary` |
| `snippets` | `get_snippets`, `add_snippet`, `delete_snippet` |
| `per_app` | `get_per_app_modes`, `set_per_app_mode`, `delete_per_app_mode`, `get_active_app` |
| `misc` | `get_microphones`, `set_hotkey`, `get_app_info`, `check_updates`, `open_url`, `test_groq_api` |

## Critical Files
- `src-tauri/src/main.rs` - Tauri entry, plugin registration
- `src-tauri/src/lib.rs` - Module declarations, AppStateInner, Tauri builder setup
- `src-tauri/src/commands/mod.rs` - IPC handler registration (31 commands)
- `src-tauri/src/pipeline/state_machine.rs` - Idle→Recording→Processing→Error (Error→Recording)
- `src-tauri/src/pipeline/batch.rs` - run_batch: STT → LLM → translate → replacements → snippets → injection
- `src-tauri/src/stt/mod.rs` - SttEngine trait + factory (Groq + whisper.cpp)
- `src-tauri/src/llm/mod.rs` - LlmFormatter trait + factory with FallbackFormatter
- `src-tauri/src/storage/db.rs` - SQLite schema + migrations
- `src-tauri/src/events.rs` - Event emitters (state_changed, transcription_complete, transcription_error, audio_level)
- `src-tauri/src/crypto.rs` - AES-256-GCM API key encryption
- `src-tauri/src/error.rs` - Unified AppError with typed ErrorCode
- `src-tauri/src/injection/mod.rs` - TextInjector trait (clipboard, keystroke, command)
- `src-tauri/src/active_window.rs` - Win32 foreground process detection
- `src-tauri/src/logging.rs` - tracing init (stderr + file rotation)
- `tauri.conf.json` - Tauri config, capabilities, windows
- `vite.config.ts` - Vite + Tailwind + React plugin config

## Frontend Components
| Module | Files |
|--------|-------|
| `ui/` | Button, Input, Select, Switch |
| `common/` | FloatingDock, HomeView, PanelHeader |
| `floating-widget/` | FloatingWidget, Waveform |
| `settings/` | SettingsLayout, GeneralTab, AudioTab, STTTab, LLMTab, ModesTab, PerAppTab, ShortcutsTab, AboutTab, HotkeyRecorder |
| `history/` | HistoryPanel |
| `dictionary/` | DictionaryPanel, SnippetsPanel |
| `onboarding/` | OnboardingFlow |

## Settings (key-value, JSON-encoded)
| Key | Type | Description |
|-----|------|-------------|
| `floating_widget` | bool | Show floating widget overlay |
| `mic_device` | string | Selected microphone device ID |
| `stt_language` | string | STT language code (e.g. "id", "en") |
| `groq_api_key` | string | Encrypted at rest via AES-256-GCM |
| `llm_engine` | string | "off", "ollama", "groq", "rule_based" |
| `llm_model` | string | Model name for the selected engine |
| `active_mode` | string | "dictation", "message", "email", or custom |
| `sound_cues` | bool | Play start/stop sounds |
| `translation_enabled` | bool | Enable LLM-based translation |
| `translation_target` | string | Target language code for translation |
| `command_mode` | bool | Enable voice command mode |
| `telemetry` | bool | Opt-in local usage statistics |
| `per_app_mode` | bool | Enable per-app mode routing |
| `hotkey` | HotkeyConfig | Global hotkey key + modifiers |

## Events (backend → frontend)
| Event | Payload | Description |
|-------|---------|-------------|
| `state_changed` | `{ state: "idle"\|"recording"\|"processing"\|"error" }` | Pipeline state transition |
| `transcription_complete` | `{ id, text, word_count }` | Successful transcription result |
| `transcription_error` | `{ message, code }` | Transcription failure |
| `audio_level` | `{ level: f32 }` | Real-time microphone input level (0.0–1.0) |

## Architecture Rules
1. **Modules = traits + factories** - SttEngine, LlmFormatter, AudioCapture, VAD, TextInjector
2. **Pipeline orchestrates** - state_machine.rs controls all flow, modules do not call each other
3. **IPC only** - frontend uses invoke/events, never touches system APIs
4. **Storage isolated** - only `storage/` module accesses SQLite
5. **Module error types** - unified `AppError` in `error.rs` with typed `ErrorCode`
6. **Post-processing pipeline** - dictionary replacements → snippet expansion applied after LLM + translation
7. **Per-app mode routing** - active window detection overrides default mode when `per_app_mode` enabled
8. **Feature gates** - `local-stt` (whisper.cpp), `local-vad` (Silero ONNX) behind Cargo features, requires cmake/libclang

## State Machine
```
IDLE -(hotkey press)> RECORDING -(vad end / stop)> PROCESSING -(done)> IDLE
  ^                        |                                               |
  |                        '--(cancel)--> IDLE                             |
  '----(start_recording)---<---- ERROR <------------(fail)-----------------'
```
Transitions: Idle→Recording, Recording→Processing, Recording→Idle (cancel), Processing→Idle, Processing→Error, Error→Recording (retry).

## Storage Module (`src-tauri/src/storage/`)
```
mod.rs exports:
  Database, DictionaryRepository, HistoryRepository, SettingsManager,
  SnippetRepository, StatsRepository, PerAppModeRepository
  apply_replacements(), expand_snippets(), DictFilter
```

## Conventions
- Rust: snake_case files/fns, PascalCase types/traits
- React: PascalCase components, camelCase hooks (prefix `use`)
- Tauri: camelCase commands, snake_case events
- SQL: snake_case columns, plural tables
- TypeScript strict - no `any`, prefer `unknown`
- Frontend stores: Zustand with `create<T>((set, get) => ({...}))` pattern

## Cloud APIs (Free Tier)
| Endpoint | Model | API Key |
|----------|-------|---------|
| `api.groq.com/openai/v1/audio/transcriptions` | whisper-large-v3-turbo | `VX_GROQ_KEY` (encrypted at rest) |
| `api.groq.com/openai/v1/chat/completions` | llama-3.1-8b-instant | `VX_GROQ_KEY` (encrypted at rest) |
| `localhost:11434/api/generate` (Ollama) | qwen2.5:3b | none |

## CI/CD
| Workflow | Trigger | Steps |
|----------|---------|-------|
| CI | Push/PR to `main` | tsc, vite build, cargo fmt, clippy, test (all `--no-default-features`) |
| Release | Push tag `v*` | tauri-action unsigned build + GitHub Release draft |

## Session Handoff
After completing a phase or significant task:
- Save all changes, commit if applicable
- Log key decisions, blockers, and partial progress in `SESSION.md`
- Report summary: what was built, what is pending, known issues
- Signal: `HANDOFF: [phase] completed - ready for next session`
- Wait for user confirmation before starting the next phase
