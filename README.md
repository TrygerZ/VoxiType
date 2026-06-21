# VoxiType

**Privacy-first, open-source voice-to-text for everyone.**

VoxiType is a desktop application that converts your speech into text in real-time. Built with a focus on privacy, it supports both local processing (Whisper.cpp, Ollama) and cloud APIs (Groq) for transcription and LLM-based text formatting.

## Features

- 🎙️ **Real-time Speech-to-Text** — Speak and see text appear instantly
- 🔒 **Privacy-First** — Choose local processing (Whisper.cpp) or cloud APIs
- 🧠 **AI-Powered Formatting** — LLM-based text correction and formatting (local Ollama or cloud Groq)
- 🔑 **Global Hotkey** — Start/stop recording from any application
- 📝 **Smart Text Injection** — Paste transcribed text directly into any app
- 🌐 **Multilingual** — Supports 50+ languages
- 🎚️ **Voice Activity Detection** — Automatic silence detection (Silero VAD)
- 📋 **History & Dictionary** — Searchable history and custom snippets

## Tech Stack

| Layer | Technology |
|-------|-----------|
| Desktop | Tauri 2.x |
| Frontend | React 19 + Vite 6 + Tailwind CSS 4 |
| State | Zustand 5.x |
| Backend | Rust 1.85+ |
| Storage | SQLite (via rusqlite) |
| Audio | cpal + rubato + ringbuf |
| STT Local | Whisper.cpp (via whisper-rs) |
| STT Cloud | Groq Whisper API |
| LLM Local | Ollama (Qwen2.5 3B) |
| LLM Cloud | Groq Llama 3.1 8B |
| VAD | Silero (via ONNX) |

## Getting Started

### Prerequisites
- Node.js 20+
- Rust 1.85+

### Development

```bash
# Install frontend dependencies
npm install

# Run development server (uses Groq API for STT by default)
npm run tauri dev
```

> **Note:** By default, the dev server builds without heavy `whisper.cpp` and `ort` dependencies. Add a Groq API key in the app settings to test transcription.

### Build Full Features

```bash
cargo build --features full
```

### Commands

| Task | Command |
|------|---------|
| Dev server | `npm run tauri dev` |
| Build app | `npm run tauri build` |
| Rust tests | `cargo test` (in `src-tauri/`) |
| Rust lint | `cargo clippy -- -D warnings` |
| Frontend tests | `npm test` |
| Typecheck | `npx tsc --noEmit` |
| Rust check | `cargo check` |

## Project Structure

```
src/                  # React frontend
src-tauri/src/        # Rust backend
├── audio/            # Audio capture & processing
├── hotkey/           # Global hotkey registration
├── injection/        # Text injection (keystroke, clipboard)
├── llm/              # LLM formatting (Ollama, Groq)
├── pipeline/         # State machine orchestrator
├── storage/          # SQLite database
├── stt/              # Speech-to-text (local Whisper, Groq)
├── vad/              # Voice activity detection
├── tray/             # System tray
└── commands.rs       # Tauri IPC handlers (25 commands)
```

## Architecture

VoxiType enforces strict separation of concerns:
- **Modules = traits + factories** — STT, LLM, Audio, VAD, Text Injection
- **Pipeline orchestrates** — `state_machine.rs` controls all flow (Idle → Recording → Processing → Error)
- **IPC only** — Frontend uses `invoke`/events, never touches system APIs
- **Storage isolated** — Only `storage/` module accesses SQLite

## Contributing

Please read [CONTRIBUTING.md](CONTRIBUTING.md) and [AGENTS.md](AGENTS.md) for guidelines.

## License

This project is licensed under the MIT License — see the [LICENSE](LICENSE) file for details.

## Acknowledgments

- Built with [Tauri](https://tauri.app/), [React](https://react.dev/), and [Rust](https://www.rust-lang.org/)
- STT powered by [Whisper.cpp](https://github.com/ggerganov/whisper.cpp) and [Groq](https://groq.com/)
- LLM formatting by [Ollama](https://ollama.ai/) and [Groq](https://groq.com/)
- VAD by [Silero](https://github.com/snakers4/silero-vad)
