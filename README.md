# VoxiType

**Version 0.3.0** — Open-source voice-to-text for everyone.

VoxiType is a free and open-source desktop application that converts your voice into text in real-time. Designed for productivity — just press a key, speak, and text appears instantly in any application.

---

## Features

- **Real-time Speech-to-Text** — Speak, text appears instantly
- **Cloud Transcription** — Fast and accurate transcription via Groq Whisper API
- **AI-Powered Formatting** — Automatic text correction and formatting (local Ollama or Groq cloud)
- **Global Hotkey** — Start/stop recording from any application (Push-to-Talk or Toggle)
- **Smart Text Injection** — Automatic text injection via keystroke, clipboard, or hybrid mode
- **Floating Overlay Widget** — Always-on-top widget with mic animation and waveform, draggable
- **Voice Command Mode** — Speak commands like "new line", "select all", "save" — executed as keystrokes
- **Per-App Formatting Modes** — Format mode automatically changes based on the active application (detected via Windows API)
- **Dictionary Hotword Boosting** — Custom dictionary with word-bounded replacement, import/export JSON
- **Snippet Expansion** — Trigger phrase automatically expanded into full content
- **Translation Pipeline** — Automatic translation of transcribed text to a target language
- **Multilingual** — Supports 50+ languages via Groq Whisper
- **Encrypted API Key Storage** — API keys encrypted with AES-256-GCM at rest
- **Sound Cues** — Optional audio feedback when starting/stopping recording
- **Usage Stats** — Local usage statistics (opt-in, never sent anywhere)
- **Update Checker** — New version notification via GitHub Releases API

## Tech Stack

| Layer | Technology |
|-------|-----------|
| Desktop | Tauri 2.x |
| Frontend | React 19 + Vite 7 + Tailwind CSS 4.3 |
| State | Zustand 5.x |
| Backend | Rust 1.85+ |
| Storage | SQLite (via rusqlite, bundled) |
| Audio | cpal + rubato + ringbuf |
| STT | Groq Whisper API (sole engine) |
| LLM Local | Ollama (Qwen2.5 3B, fallback rule-based) |
| LLM Cloud | Groq Llama 3.1 8B |
| Text Injection | enigo (keystroke) + arboard (clipboard) + hybrid |
| Crypto | AES-256-GCM (aes-gcm + base64) |
| Sound Cues | Generated tones via cpal output thread |
| Update Checker | GitHub Releases API (unsigned OSS builds) |
| Hotkey | Tauri global-shortcut plugin |

## Getting Started

### Download Pre-built Binaries

Head to the **[Releases page](https://github.com/TrygerZ/VoxiType/releases)** and download the latest installer for your platform. No compilation or API keys required to try it out.

### Build from Source

#### Prerequisites
- Node.js 20+
- Rust 1.85+
- Groq API key ([sign up for free](https://console.groq.com/))

#### Development

```bash
# Install frontend dependencies
npm install

# Run development server
npm run tauri dev
```

> Set your Groq API key in the application settings (Settings → STT) to start using transcription.

#### Commands

| Task | Command |
|------|---------|
| Dev server | `npm run tauri dev` |
| Build app | `npm run tauri build` |
| Rust tests | `cargo test` (in `src-tauri/`) |
| Rust lint | `cargo clippy -- -D warnings` |
| Typecheck | `npx tsc --noEmit` |
| Rust check | `cargo check` |
| Rust single test | `cargo test <test_name>` |

## Project Structure

```
src/                  # React frontend
src-tauri/src/        # Rust backend
├── active_window.rs  # Per-app mode detection via Windows API
├── audio/            # Audio capture (cpal) + resampler (rubato)
├── commands/         # Tauri IPC handlers (31 commands, 8 sub-modules)
│   ├── mod.rs        # Re-exports
│   ├── recording.rs  # start_recording, stop_recording
│   ├── settings.rs   # get_settings, update_setting, set_floating_widget_enabled
│   ├── misc.rs       # get_microphones, set_hotkey, get_app_info, check_updates, open_url, test_groq_api
│   ├── dictionary.rs # CRUD + import/export dictionary
│   ├── history.rs    # CRUD + search + export + re-inject
│   ├── snippets.rs   # CRUD snippets
│   ├── per_app.rs    # CRUD per-app mode mappings + get_active_app
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
│   ├── per_app.rs    # PerAppModeRepository
│   └── stats.rs      # Local-only usage stats
├── stt/              # Speech-to-text (Groq Whisper, sole engine)
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
│   └── ui/              # Button, Input, Select, Switch, Card
├── hooks/               # useTauriEvents (backend → frontend)
├── lib/                 # tauri.ts (typed invoke/listen), i18n.ts (ID/EN)
├── stores/              # Zustand: appStore, settingsStore, historyStore, dictionaryStore, snippetStore
├── styles/              # index.css (Tailwind 4, dark theme, glassmorphism)
└── types/               # app.ts, events.ts
```

### Settings Panel (8 Tabs)

| Tab | Content |
|-----|---------|
| **General** | UI language, command mode toggle, usage stats opt-in, onboarding |
| **Audio** | Microphone device selection, input device list |
| **STT** | Groq API key (encrypted at rest, masked in UI), language |
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
       ↓ (VAD detects silence / hotkey toggle)
[Recording → Processing]
       ├── STT: Groq Whisper API (with retry/backoff + hotword boosting)
       ├── LLM: Format text (Ollama / Groq / rule-based, with fallback chain)
       ├── Translation: Optional, translate to target language
       └── Post-Process: Dictionary replacements → Snippet expansion
       ↓
[Processing → Idle]  — Inject text into active application + save to history
```

### IPC Surface

VoxiType exposes **31 Tauri commands** organized into 8 sub-modules:

| Group | Commands |
|-------|----------|
| Recording | `start_recording`, `stop_recording` |
| Settings | `get_settings`, `update_setting`, `set_floating_widget_enabled` |
| Misc | `get_microphones`, `set_hotkey`, `get_app_info`, `check_updates`, `open_url`, `test_groq_api` |
| Dictionary | `get_dictionary`, `add_dictionary_word`, `set_dictionary_active`, `delete_dictionary_word`, `export_dictionary`, `import_dictionary` |
| History | `get_history`, `search_history`, `delete_history`, `pin_history`, `clear_history`, `re_inject`, `export_history` |
| Snippets | `get_snippets`, `add_snippet`, `delete_snippet` |
| Per-App | `get_per_app_modes`, `set_per_app_mode`, `delete_per_app_mode`, `get_active_app` |

## License

This project is licensed under the MIT License — see the [LICENSE](LICENSE) file for details.

## Acknowledgments

- Built with [Tauri](https://tauri.app/), [React](https://react.dev/), and [Rust](https://www.rust-lang.org/)
- STT powered by [Groq](https://groq.com/) — Whisper large-v3-turbo
- LLM formatting by [Ollama](https://ollama.ai/) (Qwen2.5 3B) and [Groq](https://groq.com/) (Llama 3.1 8B)
- Audio capture by [cpal](https://github.com/RustAudio/cpal), resampling by [rubato](https://github.com/HDegroote/rubato)
- Icons by [Lucide](https://lucide.dev/)
- Inspired by various open-source voice-to-text community projects