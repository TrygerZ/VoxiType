# Session Log

## Session 1 - 2026-06-20
Phase: Foundation (Phase 1)

### Built

**Rust Backend (src-tauri/src/):**
- `error.rs` — unified AppError + ErrorCode enum with conversions from rusqlite/reqwest/serde/io/tauri
- `audio/` — AudioCapture trait, cpal capture (dedicated thread for !Send Stream), rubato resampler (48k→16k stereo→mono), ring buffer (30s circular)
- `vad/` — VadEngine trait, energy-based fallback VAD, Silero ONNX VAD (feature-gated `local-vad`)
- `stt/` — SttEngine trait, Groq Whisper REST client (always available), whisper.cpp binding (feature-gated `local-stt`), factory with fallback chain, WAV encoder, types
- `llm/` — LlmFormatter trait, rule-based regex cleaner, Ollama HTTP client, Groq LLM (OpenAI-compat), factory, prompt templates (Dictation/Message/Email)
- `injection/` — TextInjector trait, clipboard (arboard), keystroke (enigo), hybrid injector (clipboard paste + keystroke fallback + clipboard restore)
- `pipeline/` — AppState enum (Idle→Recording→Processing→Error), state machine with validated transitions, batch orchestrator (STT→LLM→inject)
- `hotkey/` — HotkeyConfig + HotkeyMode (PTT/Toggle), global shortcut registration via tauri-plugin-global-shortcut
- `tray/` — system tray icon + context menu (Record, Settings, History, Dictionary, About, Quit)
- `storage/` — Database (SQLite + WAL + migrations), schema.sql (7 tables, FTS5, triggers, seed data), HistoryRepository (CRUD + FTS search), DictionaryRepository (upsert + hotwords), SettingsManager (key-value JSON)
- `commands.rs` — 18 Tauri IPC commands wiring settings→engines→pipeline→storage
- `events.rs` — typed event emitters (state_changed, transcription_complete, transcription_error, audio_level)
- `lib.rs` — AppStateInner, plugin registration, setup (DB, tray, hotkey)

**Frontend (src/):**
- `styles/index.css` — Tailwind 4 + @theme custom colors (vx-bg-*, vx-accent, vx-success, etc.)
- `types/` — app.ts (AppStateEnum, TranscriptionEntry, DictionaryEntry, etc.), events.ts
- `lib/tauri.ts` — typed invoke/listen wrappers for all 18 commands
- `lib/i18n.ts` — ID/EN translation dictionary
- `stores/` — appStore (state, audioLevel, mode), settingsStore (load/update/get), historyStore (CRUD+search), dictionaryStore (CRUD)
- `hooks/useTauriEvents.ts` — subscribes to backend events, updates stores
- `components/ui/` — Button, Input, Select, Switch, Card
- `components/floating-widget/` — FloatingWidget, Waveform, StatusIndicator, Timer, ModeLabel
- `components/common/` — TopBar (window controls), Sidebar (nav), HomeView
- `components/settings/` — SettingsPanel (5 tabs), GeneralTab, AudioTab, STTTab, LLMTab, AboutTab
- `components/history/` — HistoryPanel (search, pin, copy, re-inject, delete)
- `components/dictionary/` — DictionaryPanel (add, delete, list)
- `components/onboarding/` — OnboardingFlow (welcome + complete)
- `App.tsx` — root with routing (home/settings/history/dictionary/about), onboarding gate

**Config:**
- `tauri.conf.json` — VoxiType, tray-icon, 900x600, capabilities
- `Cargo.toml` — all deps, feature-gated local-vad/local-stt/full
- `capabilities/default.json` — global-shortcut, store, log, window, event permissions

### Verification
- `cargo check` ✅ (0 errors, 0 warnings)
- `cargo clippy` ✅ (0 warnings)
- `cargo test` ✅ (26 tests passed)
- `npx tsc --noEmit` ✅ (0 errors)

### Key Decisions
1. **Feature-gated native engines**: `whisper-rs` (local-stt) and `ort` (local-vad) require cmake/libclang which aren't installed. Full implementations are written but gated behind Cargo features. Default build uses Groq STT + energy VAD.
2. **cpal::Stream !Send workaround**: Audio stream runs on a dedicated OS thread, communicating via Arc<Shared> + mpsc channels. This keeps PipelineOrchestrator Send+Sync for Tauri managed state.
3. **Tauri 2.x tray**: No separate `tauri-plugin-system-tray`; tray is built-in via `tray-icon` feature in tauri core.
4. **Tailwind 4**: Uses `@theme` directive for custom colors instead of tailwind.config.ts.

### Pending (Phase 2)
- Groq Whisper STT integration test (needs API key)
- Groq LLM integration test (needs API key)
- Settings UI: Modes, Dictionary, Shortcuts tabs
- Sound cues
- API key encryption (AES-256-GCM)
- Toggle recording mode in state machine
- Translation pipeline
- Performance optimization
- Install cmake + LLVM for local-stt/local-vad features

### Blockers
- cmake/libclang not installed → can't compile with `--features full` (local whisper.cpp + Silero ONNX)

### Next (Session 2)
- Phase 2: cloud engines integration, full settings UI, dictionary hotword boosting, translation mode, sound cues, performance pass
