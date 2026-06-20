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
| VAD | Silero via ort (ONNX) | src-tauri/src/vad/ |
| STT Local | Whisper.cpp via whisper-rs | src-tauri/src/stt/ |
| STT Cloud | Groq Whisper (REST) | src-tauri/src/stt/ |
| LLM Local | Ollama Qwen2.5 3B | src-tauri/src/llm/ |
| LLM Cloud | Groq Llama 3.1 8B | src-tauri/src/llm/ |
| Text Injection | enigo + arboard | src-tauri/src/injection/ |
| Hotkey | Tauri global-shortcut plugin | src-tauri/src/hotkey/ |

## Commands
| Task | Command |
|------|---------|
| Dev server | npm run tauri dev |
| Build app | npm run tauri build |
| Rust tests | cargo test (in src-tauri/) |
| Rust lint | cargo clippy -- -D warnings |
| Frontend tests | npm test |
| Download models | npm run download-models |
| Typecheck | npx tsc --noEmit |
| Rust build check | cargo check |
| Rust single test | cargo test test_name |

## Critical Files
- src-tauri/src/main.rs - Tauri entry, plugin registration
- src-tauri/src/commands.rs - IPC handlers (25 commands)
- src-tauri/src/pipeline/state_machine.rs - Idle->Recording->Processing->Error
- src-tauri/src/stt/mod.rs - SttEngine trait + factory
- src-tauri/src/llm/mod.rs - LlmFormatter trait + factory
- src-tauri/src/storage/db.rs - SQLite schema + migrations
- tauri.conf.json - Tauri config, capabilities, windows

## Architecture Rules
1. **Modules = traits + factories** - SttEngine, LlmFormatter, AudioCapture, VAD, TextInjector
2. **Pipeline orchestrates** - state_machine.rs controls all flow, modules do not call each other
3. **IPC only** - frontend uses invoke/events, never touches system APIs
4. **Storage isolated** - only storage/ module accesses SQLite
5. **Module error types** - unified AppError in commands.rs

## State Machine
IDLE -(hotkey press)> RECORDING -(vad end)> PROCESSING -(done)> IDLE
                                               ERROR <-(fail)

## Conventions
- Rust: snake_case files/fns, PascalCase types/traits
- React: PascalCase components, camelCase hooks (prefix use)
- Tauri: camelCase commands, snake_case events
- SQL: snake_case columns, plural tables
- TypeScript strict - no any, prefer unknown

## Cloud APIs (Free Tier)
| Endpoint | Model | API Key |
|----------|-------|---------|
| api.groq.com/openai/v1/audio/transcriptions | whisper-large-v3-turbo | VX_GROQ_KEY |
| api.groq.com/openai/v1/chat/completions | llama-3.1-8b-instant | VX_GROQ_KEY |
| localhost:11434/api/generate (Ollama) | qwen2.5:3b | none |

## Commit Attribution
AI commits MUST include:
```
Co-Authored-By: OpenCode <noreply@opencode.ai>
```

## Session Handoff
After completing a phase or significant task:
- Save all changes, commit if applicable
- Log key decisions, blockers, and partial progress in SESSION.md
- Report summary: what was built, what is pending, known issues
- Signal: HANDOFF: [phase] completed - ready for next session
- Wait for user confirmation before starting the next phase

## SESSION.md
Create SESSION.md at project root to track:
```
# Session Log

## Session 1 - YYYY-MM-DD
Phase: Foundation
Built: [files, modules]
Pending: [items to continue]
Blockers: [issues found]
Next: [what to do in session 2]
```
