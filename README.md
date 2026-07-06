# VoxiType

**Version 0.4.0** — Open-source voice-to-text for every app.

VoxiType is a free and open-source desktop app that turns speech into text from anywhere on Windows. Press a global hotkey, speak, and VoxiType transcribes with Groq Whisper or local whisper.cpp, optionally formats the result with an LLM, then inserts the final text into the active application.

<img width="1292" height="1087" alt="image" src="https://github.com/user-attachments/assets/6af16b9e-0b5f-47aa-8c3a-5c3fd91cea09" />


## Features

- **Speech-to-Text Anywhere** — Dictate into any active desktop application
- **Cloud Transcription** — Fast transcription via Groq Whisper API
- **Offline Transcription** — Local dictation with `whisper-cli` and GGML models
- **Guided First-Run Setup** — Choose Groq or offline whisper.cpp, configure paths, and set a hotkey
- **AI-Powered Formatting** — Off/pass-through, rule-based cleanup, local Ollama, or Groq cloud
- **Global Hotkey** — Start/stop recording from any application (Push-to-Talk or Toggle)
- **Smart Text Injection** — Automatic text injection via keystroke, clipboard, or hybrid mode
- **Floating Overlay Widget** — Always-on-top widget with mic animation and waveform, draggable
- **Voice Command Mode** — Speak commands like "new line", "select all", "save" — executed as keystrokes
- **Per-App Formatting Modes** — Format mode automatically changes based on the active application (detected via Windows API)
- **Dictionary Hotword Boosting** — Custom dictionary with word-bounded replacement, import/export JSON
- **Snippet Expansion** — Trigger phrase automatically expanded into full content
- **Translation Pipeline** — Automatic translation of transcribed text to a target language
- **Multilingual** — Supports 50+ languages through Groq Whisper or local whisper.cpp models
- **Encrypted API Key Storage** — API keys encrypted with AES-256-GCM at rest
- **Native File Picker** — Browse for `whisper-cli.exe` and `ggml-*.bin` instead of typing paths manually
- **Sound Cues** — Optional audio feedback when starting/stopping recording
- **Usage Stats** — Local lifetime totals and optional local telemetry, never sent anywhere
- **Update Checker** — New version notification via GitHub Releases API

## Tech Stack

| Layer | Technology |
|-------|-----------|
| Desktop | Tauri 2.x |
| Frontend | React 19 + Vite 7 + Tailwind CSS 4.3 |
| State | Zustand 5.x |
| Backend | Rust 1.85+ |
| Storage | SQLite (via rusqlite, bundled) |
| Audio | cpal + rubato |
| STT | Groq Whisper API + offline whisper.cpp CLI |
| LLM Local | Off/pass-through, rule-based cleaner, Ollama (Qwen2.5 3B) |
| LLM Cloud | Groq Llama 3.1 8B |
| Text Injection | enigo (keystroke) + arboard (clipboard) + hybrid |
| Crypto | AES-256-GCM (aes-gcm + base64) |
| Sound Cues | Generated tones via cpal output thread |
| Update Checker | GitHub Releases API (unsigned OSS builds) |
| Hotkey | Tauri global-shortcut plugin |

## Getting Started

### Download Pre-built Binaries

Head to the **[Releases page](https://github.com/TrygerZ/VoxiType/releases)** and download the latest installer for your platform. You can use Groq for cloud transcription or configure offline whisper.cpp during first-run setup.

### Build from Source

#### Prerequisites
- Node.js 20+
- Rust 1.85+
- Groq API key for cloud transcription ([sign up for free](https://console.groq.com/))
- Optional local `whisper-cli` binary and `ggml-*.bin` model for offline transcription

#### Development

```bash
# Install frontend dependencies
npm install

# Run development server
npm run tauri dev
```

> Set your Groq API key in the application settings (Settings → STT) to start using transcription.

> For offline dictation without Groq STT, follow
> [docs/offline-whisper-cpp.md](docs/offline-whisper-cpp.md).

#### Commands

| Task | Command |
|------|---------|
| Dev server | `npm run tauri dev` |
| Build app | `npm run tauri build` |
| Rust tests | `cargo test --no-default-features` (in `src-tauri/`) |
| Rust lint | `cargo clippy --no-default-features -- -D warnings` (in `src-tauri/`) |
| TypeScript check | `npx tsc --noEmit` |
| Frontend build | `npm run build` |
| Rust check | `cargo check` |
| Rust single test | `cargo test <test_name>` |

## Project Structure

```
src/                  # React frontend
src-tauri/src/        # Rust backend
├── active_window.rs  # Per-app mode detection via Windows API
├── audio/            # Audio capture (cpal) + resampler (rubato)
├── commands/         # Tauri IPC handlers (35 commands, 8 IPC groups + runtime helpers)
│   ├── mod.rs        # Re-exports
│   ├── recording.rs  # start_recording, stop_recording
│   ├── settings.rs   # get_settings, update_setting, set_floating_widget_enabled
│   ├── misc.rs       # hotkeys, microphones, app info, updates, setup picker, Groq/offline tests
│   ├── dictionary.rs # CRUD + import/export dictionary
│   ├── history.rs    # CRUD + search + export + re-inject
│   ├── snippets.rs   # CRUD snippets
│   ├── per_app.rs    # CRUD per-app mode mappings + get_active_app
│   ├── stats.rs      # get_usage_stats
│   └── runtime.rs    # Shared runtime helpers (config builders, pipeline orchestration)
├── crypto.rs         # API key encryption (AES-256-GCM at rest)
├── error.rs          # Unified AppError + ErrorCode enum
├── events.rs         # Tauri event system (state_changed, audio_level, etc.)
├── hotkey/           # Global hotkey registration + rebind
├── injection/        # Text injection (keystroke, clipboard, hybrid + voice commands)
├── llm/              # LLM formatting (Ollama, Groq, rule-based, fallback chain)
├── logging.rs        # Tracing to stderr + rotating file
├── main.rs           # Tauri entry point
├── overlay.rs        # Floating widget window control + position persistence
├── pipeline/         # State machine orchestrator + batch processing
├── sound.rs          # Optional recording sound cues (start/stop tones)
├── storage/          # SQLite database
│   ├── db.rs         # Database open + migrations
│   ├── settings.rs   # SettingsManager (key-value JSON)
│   ├── history.rs    # HistoryRepository (CRUD + FTS5 search)
│   ├── dictionary.rs # DictionaryRepository (word-bounded replacements)
│   ├── snippets.rs   # SnippetRepository (trigger expansion)
│   ├── per_app_modes.rs # PerAppModeRepository
│   └── stats.rs      # Local-only usage stats
├── stt/              # Speech-to-text (Groq Whisper + whisper.cpp)
├── tray/             # System tray icon + context menu
├── updater.rs        # GitHub Releases version checker
└── util.rs           # Shared HTTP client + retry/backoff helpers
```

## Frontend Structure

```
src/
├── components/
│   ├── common/          # FloatingDock, HomeView, PanelHeader
│   ├── dictionary/      # DictionaryPanel, SnippetsPanel
│   ├── floating-widget/ # FloatingWidget + Waveform (overlay window)
│   ├── history/         # HistoryPanel (search, pin, export, re-inject)
│   ├── onboarding/      # OnboardingFlow (first-run setup)
│   ├── settings/        # 8 tabs (see below)
│   └── ui/              # Button, Input, Select, Switch, Toast
├── hooks/               # useTauriEvents (backend → frontend)
├── lib/                 # tauri.ts (typed invoke/listen), i18n.ts (ID/EN)
├── stores/              # Zustand: appStore, settingsStore, historyStore, dictionaryStore, snippetStore, statsStore
├── styles/              # index.css (Tailwind 4, dark theme, glassmorphism)
└── types/               # app.ts, events.ts
```

### Settings Panel (8 Tabs)

| Tab | Content |
|-----|---------|
| **General** | UI language, floating widget, startup/update toggles, sound cues, command mode, local telemetry |
| **Audio** | Microphone device selection, input device list |
| **STT** | Groq API key, offline whisper.cpp paths with Browse buttons, language, offline engine test |
| **LLM** | Engine selection (Off / Ollama / Groq / Rule-based), model config |
| **Modes** | Active formatting mode, translation toggle + target language |
| **App Rules** | Per-app mode mapping: detect active app → auto-switch format mode |
| **Shortcuts** | Hotkey combination + mode (Push-to-Talk / Toggle) |
| **About** | App version, check for updates, open-source links |

## Architecture

VoxiType enforces a strict **separation of concerns**:

- **Modules = traits + factories** — Each module (STT, LLM, Audio, Text Injection) is defined as a trait with a factory pattern for instantiation
- **Pipeline orchestrates** — `pipeline/` controls the entire recording lifecycle as a *finite state machine*. Modules do not call each other — everything goes through the pipeline
- **IPC only** — The frontend communicates via `invoke`/events, never touches system APIs
- **Storage isolated** — Only the `storage/` module accesses SQLite
- **Module error types** — Centralized `AppError` with structured `ErrorCode` across all modules

### State Machine

The recording pipeline follows this state machine:

```
Idle → Recording → Processing → Idle (success)
                        ↓
                     Error (reachable from any state)
```

- **Transient errors** (network, timeout) → auto-retry with exponential backoff (3 retries, 1s base delay)
- **Permanent errors** (missing API key, engine unavailable) → transition to Error, requires user action
- All state changes are emitted as Tauri events to the frontend

### Recording Data Flow

```
User presses hotkey
       ↓
[Idle → Recording]  — Audio capture via cpal, resample 48k→16k to ring buffer
       ↓ (hotkey release / toggle stop)
[Recording → Processing]
       ├── STT: Groq Whisper API or local whisper.cpp (with hotword boosting)
       ├── LLM: Off/pass-through, rule-based cleanup, Ollama, or Groq with fallback chain
       ├── Translation: Optional, translate to target language
       └── Post-Process: Dictionary replacements → Snippet expansion
       ↓
[Processing → Idle]  — Inject text into active application + save to history
```

### IPC Surface

VoxiType exposes **35 Tauri commands** organized into 8 IPC groups:

| Group | Commands |
|-------|----------|
| Recording | `start_recording`, `stop_recording` |
| Settings | `get_settings`, `update_setting`, `set_floating_widget_enabled` |
| Misc | `get_microphones`, `set_hotkey`, `get_app_info`, `check_updates`, `open_url`, `pick_setup_file`, `test_groq_api`, `test_whisper_cpp` |
| Dictionary | `get_dictionary`, `add_dictionary_word`, `set_dictionary_active`, `delete_dictionary_word`, `export_dictionary`, `import_dictionary` |
| History | `get_history`, `search_history`, `delete_history`, `pin_history`, `clear_history`, `re_inject`, `export_history` |
| Snippets | `get_snippets`, `add_snippet`, `delete_snippet` |
| Per-App | `get_per_app_modes`, `set_per_app_mode`, `delete_per_app_mode`, `get_active_app` |
| Stats | `get_usage_stats` |

## License

This project is licensed under the MIT License — see the [LICENSE](LICENSE) file for details.

## Acknowledgments

- Built with [Tauri](https://tauri.app/), [React](https://react.dev/), and [Rust](https://www.rust-lang.org/)
- STT powered by [Groq](https://groq.com/) — Whisper large-v3-turbo
- LLM formatting by [Ollama](https://ollama.ai/) (Qwen2.5 3B) and [Groq](https://groq.com/) (Llama 3.1 8B)
- Audio capture by [cpal](https://github.com/RustAudio/cpal), resampling by [rubato](https://github.com/HDegroote/rubato)
- Icons by [Lucide](https://lucide.dev/)
- Inspired by various open-source voice-to-text community projects
