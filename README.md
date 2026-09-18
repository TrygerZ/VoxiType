<p align="center">
  <img src="icon/VoxiType_Icon.png" alt="VoxiType icon" width="120" />
</p>

# VoxiType

**Version 0.5.1** - Open-source voice-to-text for every app.

VoxiType is a desktop voice dictation application for Windows. Press a global hotkey, speak, and VoxiType transcribes with Groq Whisper or local whisper.cpp, optionally formats the result with an LLM, then inserts the final text into the active application.

<img width="1292" height="1087" alt="VoxiType screenshot" src="https://github.com/user-attachments/assets/6af16b9e-0b5f-47aa-8c3a-5c3fd91cea09" />

## Platform Support

| Platform | Status |
|----------|--------|
| Windows 10/11 | Fully supported, verified in CI |
| macOS | Experimental (builds configured but not CI-tested) |
| Linux | Not supported |

Releases are unsigned. Windows users will see a SmartScreen warning on first install.

## Features

- **Speech-to-Text Anywhere** - Dictate into any active desktop application
- **Cloud Transcription** - Groq Whisper API (whisper-large-v3-turbo)
- **Offline Transcription** - Local whisper.cpp with GGML models (see [docs/offline-whisper-cpp.md](docs/offline-whisper-cpp.md))
- **AI-Powered Formatting** - Off/pass-through, rule-based cleanup, local Ollama (Qwen2.5 3B), or Groq cloud (Llama 3.1 8B)
- **Global Hotkey** - Start and stop recording from any application
- **Smart Text Injection** - Keystroke injection, clipboard paste, or hybrid mode via enigo + arboard
- **Floating Overlay Widget** - Always-on-top widget with mic animation and waveform, draggable
- **Voice Command Mode** - Speak commands like "new line", "select all", "save" - executed as keystrokes
- **Per-App Formatting Modes** - Format mode automatically switches based on active application (Windows API detection)
- **Dictionary Hotword Boosting** - Custom dictionary with word-bounded replacement, import/export JSON
- **Snippet Expansion** - Trigger phrase automatically expanded into full content
- **Translation Pipeline** - Automatic LLM-based translation to target language
- **Configurable Transcription Language** - Set transcription language via `stt_language` setting (Whisper-backed)
- **Encrypted API Key Storage** - API keys encrypted with AES-256-GCM at rest
- **First-Run Onboarding** - Guided setup for Groq or offline whisper.cpp, microphone, hotkey, and smoke test
- **Sound Cues** - Optional audio feedback when starting/stopping recording
- **Usage Stats** - Local lifetime totals and opt-in telemetry, never sent anywhere
- **Configurable Data Directory** - Choose storage location with validated migration (DB + master key + logs)
- **Update Checker** - New version notification via GitHub Releases API

## Tech Stack

| Layer | Technology |
|-------|-----------|
| Desktop | Tauri 2.x |
| Frontend | React 19 + Vite 7 + Tailwind 4 |
| State | Zustand 5.x |
| Backend | Rust 1.85+ |
| Storage | SQLite (rusqlite) |
| Audio | cpal + rubato + ringbuf |
| VAD | Energy-based (default) |
| STT | Groq Whisper API + whisper.cpp (selectable via `stt_engine` setting) |
| LLM | Ollama (Qwen2.5 3B), Groq (Llama 3.1 8B), rule-based, off |
| Text Injection | enigo (keystroke) + arboard (clipboard) |
| Crypto | AES-256-GCM (aes-gcm + base64) |
| Hotkey | Tauri global-shortcut plugin |

## Requirements

- Windows 10 or Windows 11
- Node.js 20+
- Rust 1.85+
- Groq API key for cloud transcription ([sign up for free](https://console.groq.com/))
- Optional for offline mode: whisper-cli binary + GGML model

## Getting Started

### Download Pre-built Binaries

Download the latest installer from the [Releases page](https://github.com/TrygerZ/VoxiType/releases). Choose Groq for cloud transcription or configure offline whisper.cpp during first-run setup.

### Build from Source

#### Development

```bash
# Install frontend dependencies
npm install

# Run development server
npm run tauri dev
```

Set your Groq API key in Settings → STT to start using transcription.

For offline dictation without Groq, follow [docs/offline-whisper-cpp.md](docs/offline-whisper-cpp.md).

#### Commands

| Task | Command |
|------|---------|
| Dev server | `npm run tauri dev` |
| Build app | `npm run tauri build` |
| Rust tests | `cargo test --no-default-features` (in `src-tauri/`) |
| Rust lint | `cargo clippy --no-default-features -- -D warnings` (in `src-tauri/`) |
| TypeScript check | `npx tsc --noEmit` |
| Frontend build | `npm run build` |
| Frontend tests | `npm run test` |
| Rust build check | `cargo check` (in `src-tauri/`) |

## Architecture

VoxiType enforces strict separation of concerns:

- **Modules = traits + factories** - Each module (STT, LLM, Audio, Text Injection) is a trait with factory instantiation
- **Pipeline orchestrates** - `pipeline/` controls the entire recording lifecycle as a finite state machine. Modules do not call each other
- **IPC only** - Frontend communicates via `invoke`/events, never touches system APIs
- **Storage isolated** - Only `storage/` module accesses SQLite
- **Module error types** - Unified `AppError` with typed `ErrorCode` across all modules

### State Machine

```
Idle → Recording → Processing → Idle (success)
 ↑                               ↓
 '-------- Error ←---------------'
```

Error→Recording retry for transient failures (network, timeout) with exponential backoff (3 retries, 1s base delay). Permanent errors (missing API key, engine unavailable) transition to Error state, requiring user action. All state changes emit Tauri events to frontend.

### Data Flow

```
User presses hotkey
       │
[Idle → Recording]  - Audio capture via cpal, resample 48k → 16k to ring buffer
        │ (hotkey release / toggle stop)
[Recording → Processing]
       ├── STT: Groq Whisper API or local whisper.cpp (with hotword boosting)
       ├── LLM: Off/pass-through, rule-based cleanup, Ollama, or Groq with fallback chain
       ├── Translation: Optional, translate to target language
       ├── Post-Process: Dictionary replacements → Snippet expansion
       │
[Processing → Idle]  - Inject text into active application + save to history
```

### IPC Surface

VoxiType exposes **42 Tauri commands** organized into 8 modules:

| Module | Commands |
|--------|----------|
| `recording` | `start_recording`, `stop_recording` |
| `settings` | `get_settings`, `update_setting`, `set_floating_widget_enabled` |
| `history` | `get_history`, `search_history`, `delete_history`, `pin_history`, `clear_history`, `re_inject`, `export_history` |
| `dictionary` | `get_dictionary`, `add_dictionary_word`, `set_dictionary_active`, `delete_dictionary_word`, `export_dictionary`, `import_dictionary` |
| `snippets` | `get_snippets`, `add_snippet`, `delete_snippet` |
| `per_app` | `get_per_app_modes`, `set_per_app_mode`, `delete_per_app_mode`, `get_active_app` |
| `misc` | `get_microphones`, `set_hotkey`, `get_app_info`, `check_updates`, `open_url`, `reveal_floating_widget`, `reset_widget_idle_timer`, `ack_widget_hide`, `pick_setup_file`, `set_whisper_cpp_paths`, `pick_data_directory`, `set_data_directory`, `get_data_directory`, `test_groq_api`, `test_whisper_cpp`, `restart_app` |
| `stats` | `get_usage_stats` |

### Events (Backend → Frontend)

| Event | Payload | Description |
|-------|---------|-------------|
| `state_changed` | `{ state: "idle"\|"recording"\|"processing"\|"error" }` | Pipeline state transition |
| `transcription_complete` | `{ id, text, word_count, duration_ms }` | Successful transcription result |
| `transcription_error` | `{ message, code }` | Transcription failure |
| `audio_level` | `{ level: f32 }` | Real-time microphone input level (0.0 to 1.0) |
| `floating_widget_hide_requested` | `{ id }` | Overlay requests animated hide before window hide |
| `floating_widget_reveal_requested` | `{ id }` | Overlay requests animated reveal after window show |

## Project Structure

```
src/                  # React frontend
├── components/
│   ├── common/          # FloatingDock, HomeView, PanelHeader, WpmHalfRing
│   ├── dictionary/      # DictionaryPanel, SnippetsPanel
│   ├── floating-widget/ # FloatingWidget + Waveform (overlay window)
│   ├── history/         # HistoryPanel (search, pin, export, re-inject)
│   ├── onboarding/      # OnboardingFlow (first-run setup: 8 steps)
│   ├── settings/        # SettingsLayout, SettingsPanel + 8 tabs (General, Audio, STT, LLM, Modes, App Rules, Shortcuts, About)
│   ├── ui/              # Button, Input, Select, Switch, Toast
│   └── ErrorBoundary.tsx # Top-level React error boundary
├── hooks/               # useTauriEvents (backend event subscriptions)
├── lib/                 # tauri.ts (typed invoke/listen), i18n.ts (EN/ID)
├── stores/              # Zustand: appStore, settingsStore, historyStore, dictionaryStore, snippetStore, statsStore
├── styles/              # index.css (Tailwind 4, dark theme, glassmorphism)
└── types/               # app.ts, events.ts

src-tauri/src/        # Rust backend
├── active_window.rs  # Per-app mode detection via Win32 API
├── audio/            # Audio capture (cpal) + resampler (rubato) + VAD
├── commands/         # Tauri IPC handlers (42 commands across 8 modules + runtime helpers)
├── crypto.rs         # AES-256-GCM API key encryption
├── data_dir.rs       # Data directory marker resolution, validation, copy-on-migrate
├── error.rs          # Unified AppError + typed ErrorCode
├── events.rs         # Tauri event emitters (state_changed, audio_level, etc.)
├── hotkey/           # Global hotkey registration + rebind
├── injection/        # Text injection (keystroke, clipboard, hybrid, command mode)
├── llm/              # LLM formatting (Ollama, Groq, rule-based, fallback chain)
├── logging.rs        # tracing to stderr + rotating file
├── main.rs           # Tauri entry point
├── overlay.rs        # Floating widget window control + position persistence
├── pipeline/         # State machine orchestrator + batch processing
├── sound.rs          # Optional recording sound cues (start/stop tones)
├── storage/          # SQLite database
│   ├── db.rs         # Database open + migrations
│   ├── settings.rs   # SettingsManager (key-value JSON)
│   ├── history.rs    # HistoryRepository (CRUD + FTS5 search)
│   ├── dictionary.rs # DictionaryRepository (word-bounded replacements, hotword boosting)
│   ├── snippets.rs   # SnippetRepository (trigger expansion)
│   ├── per_app_modes.rs # PerAppModeRepository
│   └── stats.rs      # StatsRepository (local usage totals)
├── stt/              # Speech-to-text (Groq Whisper + whisper.cpp)
├── tray/             # System tray icon + context menu
├── updater.rs        # GitHub Releases version checker
└── util.rs           # Shared HTTP client + retry/backoff helpers
```

## Configuration

Settings are stored as key-value pairs in SQLite, JSON-encoded. Key settings:

| Key | Type | Description |
|-----|------|-------------|
| `floating_widget` | bool | Show floating widget overlay |
| `mic_device` | string | Selected microphone device ID |
| `stt_language` | string | STT language code (e.g. "id", "en") |
| `stt_engine` | string | "groq" (default) or "whisper_cpp" |
| `groq_api_key` | string | Encrypted at rest via AES-256-GCM |
| `whisper_cpp_binary_path` | string | Path to whisper-cli executable |
| `whisper_cpp_model_path` | string | Path to GGML model file |
| `whisper_cpp_threads` | number | Thread count for whisper.cpp (default: 4) |
| `llm_engine` | string | "off", "ollama", "groq", "rule_based" |
| `llm_model` | string | Model name for selected engine |
| `active_mode` | string | "dictation", "message", "email", or custom |
| `translation_enabled` | bool | Enable LLM-based translation |
| `translation_target` | string | Target language code for translation |
| `command_mode` | bool | Enable voice command mode |
| `sound_cues` | bool | Play start/stop sounds |
| `telemetry` | bool | Opt-in local usage statistics |
| `per_app_mode` | bool | Enable per-app mode routing |
| `hotkey` | HotkeyConfig | Global hotkey key + modifiers |
| `floating_widget_auto_hide_seconds` | number | Idle seconds before the floating widget auto-hides (0 disables) |
| `floating_widget_pos` | object | Persisted floating widget screen position |
| `onboarding_completed` | bool | First-run onboarding completion flag |
| `language` | string | UI language code ("en" or "id") |
| `stt_model` | string | Model name for the selected STT engine |
| `auto_start` | bool | Launch VoxiType at Windows sign-in |
| `auto_update` | bool | Check for updates automatically |

Data directory selection uses a marker file (`data_dir.txt`) in the default app-data directory. On first use, migration copies the active previous directory's DB, `master.key`, and logs. DB and key copies use SHA-256 verification and atomic rename. Remote, removable, and UNC targets are rejected. Selected directory takes effect after restart.

## Offline STT Setup

For full offline transcription without Groq, see [docs/offline-whisper-cpp.md](docs/offline-whisper-cpp.md).

## CI/CD

| Workflow | Trigger | Steps |
|----------|---------|-------|
| CI | Push/PR to `main` | TypeScript check, Vite build, Rust format, clippy, test (all `--no-default-features`) |
| Release | Push tag `v*` | tauri-action unsigned build + GitHub Release draft (Windows only) |

## License

This project is licensed under the MIT License. See the [LICENSE](LICENSE) file for details.

## Acknowledgments

- Built with [Tauri](https://tauri.app/), [React](https://react.dev/), and [Rust](https://www.rust-lang.org/)
- STT powered by [Groq](https://groq.com/) (Whisper large-v3-turbo) and [whisper.cpp](https://github.com/ggml-org/whisper.cpp)
- LLM formatting by [Ollama](https://ollama.ai/) (Qwen2.5 3B) and [Groq](https://groq.com/) (Llama 3.1 8B)
- Audio capture by [cpal](https://github.com/RustAudio/cpal), resampling by [rubato](https://github.com/HDegroote/rubato)
- Icons by [Lucide](https://lucide.dev/)
