# VoxiType

**Version 0.2.0** — Open-source voice-to-text for everyone.

VoxiType is a desktop application that converts your speech into text in real-time. Built with Groq API for transcription and LLM-based text formatting (Ollama or Groq).

## Features

- **Real-time Speech-to-Text** — Speak and see text appear instantly
- **Cloud Transcription** — Fast, accurate Groq Whisper API
- **AI-Powered Formatting** — LLM-based text correction and formatting (local Ollama or cloud Groq)
- **Global Hotkey** — Start/stop recording from any application
- **Smart Text Injection** — Paste transcribed text directly into any app
- **Floating Overlay Widget** — Persistent always-on-top widget with live recording animation and drag support
- **Per-App Formatting Modes** — Auto-switch formatting rules based on the active application
- **Multilingual** — Supports 50+ languages
- **Encrypted API Key Storage** — API keys encrypted at rest using AES-256-GCM
- **Sound Cues** — Optional audio feedback on recording start and stop

## Tech Stack

| Layer | Technology |
|-------|-----------|
| Desktop | Tauri 2.x |
| Frontend | React 19 + Vite 6 + Tailwind CSS 4 |
| State | Zustand 5.x |
| Backend | Rust 1.85+ |
| Storage | SQLite (via rusqlite) |
| Audio | cpal + rubato |
| STT | Groq Whisper API |
| LLM Local | Ollama (Qwen2.5 3B) |
| LLM Cloud | Groq Llama 3.1 8B |
| Text Injection | enigo + arboard (keystroke, clipboard, hybrid) |
| Sound Cues | WAV audio files |
| Update Checker | GitHub Releases API |
| Hotkey | Tauri global-shortcut plugin |

## Getting Started

### Prerequisites
- Node.js 20+
- Rust 1.85+

### Development

```bash
# Install frontend dependencies
npm install

# Run development server (uses Groq API for STT)
npm run tauri dev
```

> **Note:** Add a Groq API key in the app settings to test transcription.

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
├── active_window.rs  # Per-app mode detection
├── audio/            # Audio capture & processing
├── commands.rs       # Tauri IPC handlers (21 commands)
├── crypto.rs         # API key encryption (AES-256-GCM)
├── error.rs          # Unified AppError types
├── events.rs         # Tauri event system
├── hotkey/           # Global hotkey registration
├── injection/        # Text injection (keystroke, clipboard, hybrid)
├── llm/              # LLM formatting (Ollama, Groq)
├── logging.rs        # Application logging
├── main.rs           # Tauri entry point
├── overlay.rs        # Floating widget overlay
├── pipeline/         # State machine orchestrator
├── sound.rs          # Optional recording sound cues
├── storage/          # SQLite database
├── stt/              # Speech-to-text (Groq Whisper)
├── tray/             # System tray
├── updater.rs        # Update checker
├── util.rs           # Shared utilities
```

## Frontend Structure

```
src/
├── components/
│   ├── common/         # Shared components (FloatingDock, HomeView, PanelHeader)
│   ├── dictionary/     # Dictionary & Snippets panels
│   ├── floating-widget/# Persistent overlay widget
│   ├── history/        # Transcription history
│   ├── onboarding/     # First-run setup flow
│   ├── settings/       # Settings tabs (Audio, STT, LLM, Hotkey, Modes, PerApp, etc.)
│   └── ui/             # Base UI components (Button, Card, Input, Select, Switch)
├── hooks/              # Custom React hooks
├── lib/                # Utilities (i18n, Tauri bridge)
├── stores/             # Zustand state stores
├── styles/             # Global CSS (Tailwind)
└── types/              # TypeScript type definitions
```

## Architecture

VoxiType enforces strict separation of concerns:
- **Modules = traits + factories** — STT, LLM, Audio, Text Injection
- **Pipeline orchestrates** — `state_machine.rs` controls the recording lifecycle as a finite state machine
- **IPC only** — Frontend uses `invoke`/events, never touches system APIs
- **Storage isolated** — Only `storage/` module accesses SQLite
- **Module error types** — Unified AppError with structured error codes across all modules

### State Machine

The recording pipeline follows this state machine:

```
Idle → Recording → Processing → Idle (success)
                        ↓
                     Error (reachable from any state)
```

On transient errors (network, timeout) the pipeline may auto-retry. Permanent errors (missing API key, model not found) transition to Error and require user action.feat!: remove local Whisper STT, switch to Groq-only engine

BREAKING: All local STT (whisper.cpp) code paths removed.
Simplify to Groq Whisper as the sole STT engine.
Drop whisper-rs, ort, ndarray deps & local-stt/local-vad features.
Bump version to 0.3.0.

## License

This project is licensed under the MIT License — see the [LICENSE](LICENSE) file for details.

## Acknowledgments

- Built with [Tauri](https://tauri.app/), [React](https://react.dev/), and [Rust](https://www.rust-lang.org/)
- STT powered by [Groq](https://groq.com/)
- LLM formatting by [Ollama](https://ollama.ai/) and [Groq](https://groq.com/)
