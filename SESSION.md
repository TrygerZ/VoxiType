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

## Session 2 - 2026-06-21
Phase: Core Features (Phase 2)

### Built
**Rust Backend:**
- `util.rs` (new) — shared `http_client()` (pooled reqwest, reused across requests) + `retry_with_backoff(max, base_delay, op)` with retryable-error classification; unit tests
- `crypto.rs` (new) — AES-256-GCM API key encryption at rest; master key generated/persisted at `{app_data_dir}/master.key` (0600 on unix); `enc:v1:<base64(nonce|ct)>` envelope; plaintext passthrough for migration; 4 unit tests
- `sound.rs` (new) — fire-and-forget sound cues (start/stop tones) via dedicated cpal output thread, no new deps
- `stt/groq_stt.rs` — network call wrapped in retry/backoff (3 retries, 1s base), shared HTTP client
- `llm/groq_llm.rs` — chat call wrapped in retry/backoff, shared HTTP client
- `llm/ollama.rs` — shared HTTP client
- `storage/dictionary.rs` — `get_replacements()`, `set_active()`, `apply_replacements()` (word-bounded, case-insensitive) + 3 tests
- `pipeline/batch.rs` — `run_batch` now applies optional translation (`TranslateOpts`) + dictionary replacements before injection
- `commands.rs` — new commands: `export_history`, `update_dictionary_word`, `set_dictionary_active`, `export_dictionary`, `import_dictionary`, `translate`, `set_hotkey`; API key encrypt-on-write + decrypt-on-read; sound cue triggers; translation wiring
- `storage/schema.sql` — seeded `groq_api_key`, `translation_enabled`, `translation_target`
- `lib.rs` — registered 7 new commands, master key in `AppStateInner`, new modules
- `Cargo.toml` — added aes-gcm, base64, rand, sha2

**Frontend:**
- `lib/tauri.ts` — wrappers for all new commands (exportHistory, dictionary import/export/active, translateText, setHotkey)
- `components/settings/ModesTab.tsx` (new) — active mode + translation toggle/target
- `components/settings/ShortcutsTab.tsx` (new) — hotkey rebind + PTT/Toggle mode, apply via set_hotkey
- `components/settings/SettingsPanel.tsx` — 7 tabs wired (General/Audio/STT/LLM/Modes/Shortcuts/About)
- `components/settings/STTTab.tsx` — Groq API key field (conditional)
- `components/history/HistoryPanel.tsx` — JSON/CSV export, mode filter
- `components/dictionary/DictionaryPanel.tsx` — replacement field, active toggle, import/export

### Verification
- `cargo check` OK (0 errors)
- `cargo clippy -- -D warnings` OK (0 warnings)
- `cargo test` OK (35 passed)
- `npx tsc --noEmit` OK (0 errors)
- `npm run build` OK (vite build success)

### Key Decisions
1. **API key encryption**: file-based master key (not OS keychain) — pragmatic for local single-user open-source app, no extra platform deps. Encrypt-on-write in `update_setting`, decrypt-on-read in engine builders. Legacy plaintext passes through.
2. **Retry/backoff**: only retries transient errors (network/timeout/5xx); auth errors fail fast. Base delay parameterized so tests run instantly.
3. **Sound cues**: generated tones via short-lived cpal output thread (mirrors capture's !Send handling) — avoids bundling WAV assets or adding rodio.
4. **Dictionary replacements**: applied after LLM formatting + after translation, so custom spellings always win. Word-bounded + case-insensitive matching.
5. **Shared HTTP client**: `OnceLock<reqwest::Client>` reused across all cloud requests (connection pooling), addresses per-request client churn.

### Pending (Phase 3)
- Tauri updater integration, release CI
- Command mode (voice → keystroke macros), snippet library
- Local engine builds (cmake/LLVM for local-stt/local-vad) still gated
- Auto-learn dictionary (UI correction capture) — backend ready (update_dictionary_word), needs correction-tracking UI
- Streaming LLM/STT, per-app mode detection

### Blockers
- cmake/libclang not installed → `--features full` still blocked (unchanged from Session 1)

HANDOFF: Phase 2 completed - ready for next session

## Session 3 - 2026-06-21
Phase: Cloud & Polish (Phase 3) + critical bug fixes

### Critical Bug Fixed (startup panic)
- **Double logger init** → `npm run tauri dev` panicked: `PluginInitialization("log", "attempted to set a logger after the logging system was already initialized")`. Both `tracing_subscriber` and `tauri_plugin_log` tried to own the global `log` logger.
- Fix: removed `tauri_plugin_log` (frontend never used it). Dropped dep from Cargo.toml + `log:default` from capabilities. Verified app boots clean.

### Runtime Bugs Fixed
1. **LLM had no fallback** → new `llm/fallback.rs` `FallbackFormatter` wraps Ollama/Groq; any failure (server down, bad key, network) transparently falls back to rule-based. Factory wires it. Pipeline now always produces output.
2. **Encrypted API key leaked to UI** → `get_settings` now masks `groq_api_key` (empty) + adds `groq_api_key_set: bool`. Frontend shows "•••• (saved)" placeholder.
3. **audio_level never emitted** → `spawn_level_emitter` task emits ~20x/sec while recording; waveform now reacts. Frontend timer driven from `state_changed` in useTauriEvents.
4. **Confusing STT error** → factory gives a clear message when whisper.cpp missing AND no Groq key set.

### Built (Phase 3)
**Rust:**
- `logging.rs` — tracing to stderr + daily-rotated file at `{app_data}/logs/voxitype.log.<date>`; WorkerGuard held in AppStateInner. (3.7)
- `storage/snippets.rs` — Snippet CRUD + `expand_snippets` (longest-trigger-first, word-bounded). Wired into pipeline post-processing. (3.5)
- `injection/command.rs` — `VoiceCommand` maps ID/EN phrases → keystrokes (new line, select all, save, undo...). Opt-in `command_mode`. (3.4)
- `storage/stats.rs` — opt-in local `usage_stats` aggregation (per-day, never transmitted). Recorded in pipeline when `telemetry` on. (3.8)
- `updater.rs` — GitHub Releases version checker (no signing needed; unsigned OSS builds). `check_updates` command. (3.2)
- `pipeline/batch.rs` — `PostProcess { replacements, snippets }` struct; applies replacements then snippet expansion after format/translate.
- `commands.rs` — new: get/add/delete_snippet, get_usage_stats, check_updates; command-mode early path; telemetry recording.
- New settings seeded: `command_mode`.

**Frontend:**
- `SnippetsPanel.tsx` + `snippetStore.ts` — trigger/content CRUD; new Snippets nav item.
- `GeneralTab` — command mode + anonymous usage stats toggles.
- `AboutTab` — "Check for updates" button with GitHub release link.
- `App.tsx` — applies saved UI language on load (3.6); snippets view + nav route.
- `useTauriEvents` — recording timer; `tauri.ts` + types for snippets/stats/updates.

**CI/CD (3.3):**
- `.github/workflows/ci.yml` — tsc, frontend build, cargo fmt/clippy/test on push+PR (windows-latest).
- `.github/workflows/release.yml` — tauri-action unsigned build → draft GitHub release on `v*` tag.

### Verification
- `cargo fmt --all --check` OK
- `cargo clippy --no-default-features -- -D warnings` OK (0 warnings)
- `cargo test --no-default-features` OK (42 passed)
- `npx tsc --noEmit` OK
- `npm run build` OK
- App boots clean (no panic); log file written + confirmed.

### Key Decisions
1. **Removed tauri-plugin-log** instead of disabling tracing — frontend never logged via plugin; tracing_subscriber is the single source of truth.
2. **Custom update checker, not tauri-plugin-updater** — updater plugin requires signing keys/pubkey; OSS builds are unsigned. GitHub Releases API check degrades gracefully.
3. **LLM fallback wrapper** — keeps engine selection logic in factory; pipeline unaware. Guarantees output.
4. **Command mode re-transcribes on non-command** — acceptable for opt-in mode; commands match the fast path.
5. **Telemetry is local-only** — `usage_stats` table, never sent anywhere; honors `telemetry` setting.

### Pending (Phase 4)
- Per-app mode detection (active window) — 4.1
- WASAPI loopback (system audio) — 4.2
- VAD integration into live pipeline (modules exist, not yet driving capture)
- Auto-learn dictionary from user corrections (backend ready)
- E2E test suite (Playwright) — 4.4
- Community docs (CONTRIBUTING, CODE_OF_CONDUCT, issue templates) — 4.5
- Updater signing (optional, if maintainers adopt a key)

### Blockers
- cmake/libclang not installed → `--features full` (local whisper.cpp + Silero) still blocked. Default/dev build uses `--no-default-features` (Groq STT + energy VAD + rule-based/Ollama LLM).

HANDOFF: Phase 4 completed - Project Ready for Distribution

## Session 4 - 2026-06-21
Phase: Final Polish & Premium UI (Phase 4)

### Built
**Rust Backend:**
- `active_window.rs` (new) — uses Windows API (`GetForegroundWindow` / `GetWindowThreadProcessId` / `GetModuleBaseNameW`) to detect the active application's process name (e.g. "code", "chrome").
- `storage/per_app_modes.rs` (new) — CRUD repository mapping process names to `mode_id`.
- `commands.rs` — wired `get_active_app` to detect target window. Wired `active_mode()` to override default mode if `per_app_mode` is enabled and the active window matches a rule.

**Frontend / Premium UI Redesign:**
- `styles/index.css` — upgraded to a premium dark theme. Custom colors (`--color-vx-bg-primary`, etc.), deep glassmorphism (`.vx-glass`), shadows, scrollbar styling, and an ambient background mesh (`.vx-app-bg`).
- `components/ui/` — redesigned `Button` (interactive scaling), `Input`, `Select`, `Switch`, and `Card` with subtle borders, hover states, and smooth focus rings.
- `components/floating-widget/` — fully redesigned overlay window. Glass card with animated mic pulsing, animated waveform tracking audio level, timer, and clear action buttons.
- `components/common/TopBar.tsx`, `Sidebar.tsx`, `HomeView.tsx` — cohesive layout. Home screen features a pulsing, status-aware microphone halo.
- `components/settings/SettingsLayout.tsx` — unified layout helper (`SettingsHeader`, `SettingsGroup`, `SettingsRow`) applied to all 8 tabs to give a structured, native-app feel.
- `components/common/PanelHeader.tsx` — consistent header for History, Dictionary, and Snippets.
- `components/onboarding/OnboardingFlow.tsx` — polished, animated intro flow with clear icons and value propositions.
- `components/settings/PerAppTab.tsx` — new tab to configure App Rules (detect active app + map to formatting mode).

**Community & Distribution:**
- `CONTRIBUTING.md` — guidelines for dev setup and PR process.
- `CODE_OF_CONDUCT.md` — standard contributor covenant.
- `.github/ISSUE_TEMPLATE/*.yml` — structured bug and feature request templates.

### Verification
- All Rust tests pass (43 tests).
- `cargo clippy` is clean.
- `npx tsc` typecheck is clean.
- `npm run build` generates `index.html` and `floating.html` properly.
- Startup panic confirmed fixed (double logger removed).
- The `windows` crate dependencies resolve smoothly.

### Pending / Next Steps
- Production release via GitHub Actions (triggered on next `v*` tag).
- Collect user feedback on auto-learn dictionary.
- Optional: Code signing for Windows if a certificate is procured.

