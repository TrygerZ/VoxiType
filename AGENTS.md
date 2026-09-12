# VoxiType - Agent Instructions

## Stack
| Layer | Tool | File Pattern |
|-------|------|-------------|
| Desktop | Tauri 2.x | src-tauri/ |
| Frontend | React 19 + Vite 7 + Tailwind 4 | src/ |
| Frontend testing | Vitest + React Testing Library | vitest.config.ts, src/test/ |
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
| Data Directory | Marker resolution, validation, and copy-on-migrate | src-tauri/src/data_dir.rs |

## Global Rules

### RTK Proxy
Always use **RTK (Rust Token Killer)** as a proxy for all terminal commands. RTK saves 60-90% tokens by filtering unnecessary output. Examples: `rtk cargo build`, `rtk git status`, `rtk npm run dev`. Use `rtk gain` to view token savings analytics.

### Ask When Uncertain
If you encounter unclear, ambiguous instructions or anything that needs clarification — **ASK ME FIRST**. Do not proceed on assumptions. This includes:
- Incomplete or contradictory feature specifications
- Design/architecture choices not explicitly stated
- Priorities between implementation options
- Whether a change is safe to make without side effects
- Whether to use cloud vs local for a component

### Clean Code
All code in this project must follow **Clean Code** principles:
- Clean, readable, and self-documenting code
- Descriptive and meaningful variable, function, and type names
- Small functions with a single responsibility
- No magic numbers or string literals without named constants
- Explicit and meaningful error handling, not panic
- Comments only for "why", not "what" or "how"
- DRY (Don't Repeat Yourself) — avoid code duplication
- Testing as part of the definition of "done"

## Commands
| Task | Command |
|------|---------|
| Dev server | rtk npm run tauri dev |
| Build app | rtk npm run tauri build |
| Rust tests | rtk cargo test --no-default-features (in src-tauri/) |
| Rust lint | rtk cargo clippy --no-default-features -- -D warnings |
| TypeScript check | rtk npx tsc --noEmit |
| Frontend build | rtk npm run build |
| Frontend tests | rtk npm run test |
| Rust build check | rtk cargo check (in src-tauri/) |
| Rust single test | rtk cargo test test_name |

## IPC Commands (39 total across 8 modules)
Commands are registered in `src-tauri/src/commands/mod.rs` and exposed via `lib.rs`.

| Module | Commands |
|--------|----------|
| `recording` | `start_recording`, `stop_recording` |
| `settings` | `get_settings`, `update_setting`, `set_floating_widget_enabled` |
| `history` | `get_history`, `search_history`, `delete_history`, `pin_history`, `clear_history`, `re_inject`, `export_history` |
| `dictionary` | `get_dictionary`, `add_dictionary_word`, `set_dictionary_active`, `delete_dictionary_word`, `export_dictionary`, `import_dictionary` |
| `snippets` | `get_snippets`, `add_snippet`, `delete_snippet` |
| `per_app` | `get_per_app_modes`, `set_per_app_mode`, `delete_per_app_mode`, `get_active_app` |
| `misc` | `get_microphones`, `set_hotkey`, `get_app_info`, `check_updates`, `open_url`, `reveal_floating_widget`, `pick_setup_file`, `set_whisper_cpp_paths`, `pick_data_directory`, `set_data_directory`, `get_data_directory`, `test_groq_api`, `test_whisper_cpp` |
| `stats` | `get_usage_stats` |

## Critical Files
- `src-tauri/src/main.rs` - Tauri entry, plugin registration
- `src-tauri/src/lib.rs` - Module declarations, AppStateInner, Tauri builder setup
- `src-tauri/src/commands/mod.rs` - IPC handler registration (39 commands)
- `src-tauri/src/pipeline/state_machine.rs` - Idle→Recording→Processing→Error (Error→Recording)
- `src-tauri/src/pipeline/batch.rs` - run_batch: STT → LLM → translate → replacements → snippets → injection
- `src-tauri/src/stt/mod.rs` - SttEngine trait + factory (Groq + whisper.cpp)
- `src-tauri/src/llm/mod.rs` - LlmFormatter trait + factory with FallbackFormatter
- `src-tauri/src/storage/db.rs` - SQLite schema + migrations
- `src-tauri/src/events.rs` - Event emitters (state_changed, transcription_complete, transcription_error, audio_level)
- `src-tauri/src/crypto.rs` - AES-256-GCM API key encryption
- `src-tauri/src/data_dir.rs` - Data-directory marker resolution, validation, and copy-on-migrate
- `src-tauri/src/error.rs` - Unified AppError with typed ErrorCode
- `src-tauri/src/injection/mod.rs` - TextInjector trait (clipboard, keystroke, command)
- `src-tauri/src/active_window.rs` - Win32 foreground process detection
- `src-tauri/src/logging.rs` - tracing init (stderr + file rotation)
- `tauri.conf.json` - Tauri config, capabilities, windows
- `vite.config.ts` - Vite + Tailwind + React plugin config

## Frontend Components
| Module | Files |
|--------|-------|
| `ui/` | Button, Input, Select, Switch, Toast |
| `common/` | FloatingDock, HomeView, PanelHeader |
| `floating-widget/` | FloatingWidget, Waveform |
| `settings/` | SettingsLayout, GeneralTab, AudioTab, STTTab, LLMTab, ModesTab, PerAppTab, ShortcutsTab, AboutTab, HotkeyRecorder |
| `history/` | HistoryPanel |
| `dictionary/` | DictionaryPanel, SnippetsPanel |
| `onboarding/` | `OnboardingFlow.tsx`, `types.ts`, `shared/` (`StepShell`, `StepProgress`), `steps/` (`WelcomeStep`, `QuickSettingsStep`, `MicrophoneStep`, `SttSetupStep`, `DataDirectoryStep`, `HotkeyStep`, `SmokeTestStep`, `CompleteStep`) |

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
| `stt_engine` | string | "groq" (default) or "whisper_cpp" |
| `whisper_cpp_binary_path` | string | Path to whisper-cli executable |
| `whisper_cpp_model_path` | string | Path to GGML model file (e.g. ggml-base.bin) |
| `whisper_cpp_threads` | number | Thread count for whisper.cpp (default: 4) |
| `telemetry` | bool | Opt-in local usage statistics |
| `per_app_mode` | bool | Enable per-app mode routing |
| `hotkey` | HotkeyConfig | Global hotkey key + modifiers |

Data-directory selection is not a key-value setting. The marker file `data_dir.txt` lives in the default app-data directory; it stores JSON `current`/`pending` paths, while plain-path markers remain supported for backward compatibility. On first use, migration copies the active previous directory's DB, `master.key`, and logs; DB/key copies use SHA-256 verification and atomic rename. Remote, removable, and UNC targets are rejected. A selected directory takes effect after restart.

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
- Clean Code: functions ≤ 30 lines, one level of abstraction per function
- Rust: use `Result<T, AppError>` over `unwrap()`/`expect()`, provide context with `.context()` or `.map_err()`
- Rust: prefer `match` or `if let` over panic macros (`unwrap`, `expect`, `panic!`)
- React: extract logic into custom hooks, keep components pure for UI
- TypeScript: avoid `as` casting, use type guards or assertion functions
- Imports: group by (standard library → third-party → internal), separate with blank lines

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

## Git Workflow

### Branch & Commit Rules

| Change Type | Branch | Commit | Push | Merge |
|---|---|---|---|---|
| **Significant** (new features, refactoring, architecture changes) | `feature/<name>` or `fix/<name>` | Automatic | User approval required | User approval required |
| **Minor** (documentation, typos, comments) | Direct to `main` | Automatic | User approval required | User approval required |

#### Criteria:
1. **New branch required** for: adding features, refactoring, API changes, modifying business logic, significant UI changes
2. **Direct to `main`** for: documentation fixes, typos, comments, README updates, changelog updates
3. **Commit automatically** — agent determines appropriate commit message
4. **Push & Merge** — always require explicit user approval

### Workflow:
1. Agent determines whether changes are *significant* or *minor*
2. If *significant*: create new branch from `main`, work on that branch, commit automatically
3. If *minor*: commit directly to `main`, commit automatically
4. Agent **never** pushes or merges without user approval
5. After commit is complete, agent notifies user and requests approval for push + merge

## Session Handoff
After completing a phase or significant task:
- Save all changes, commit if applicable
- Log key decisions, blockers, and partial progress in `SESSION.md`
- Report summary: what was built, what is pending, known issues
- Signal: `HANDOFF: [phase] completed - ready for next session`
- Wait for user confirmation before starting the next phase

## graphify

This project has a knowledge graph at graphify-out/ with god nodes, community structure, and cross-file relationships.

When the user types `/graphify`, use the installed graphify skill or instructions before doing anything else.

Rules:
- For codebase questions, first run `graphify query "<question>"` when graphify-out/graph.json exists. Use `graphify path "<A>" "<B>"` for relationships and `graphify explain "<concept>"` for focused concepts. These return a scoped subgraph, usually much smaller than GRAPH_REPORT.md or raw grep output.
- Dirty graphify-out/ files are expected after hooks or incremental updates; dirty graph files are not a reason to skip graphify. Only skip graphify if the task is about stale or incorrect graph output, or the user explicitly says not to use it.
- If graphify-out/wiki/index.md exists, use it for broad navigation instead of raw source browsing.
- Read graphify-out/GRAPH_REPORT.md only for broad architecture review or when query/path/explain do not surface enough context.
- After modifying code, run `graphify update .` to keep the graph current (AST-only, no API cost).

## Documentation Style

- In prose, use ONLY single ASCII hyphens (`-`). Never use em dashes (U+2014), en dashes (U+2013), or double hyphens (`--`) as punctuation or parenthetical separators; replace them with periods, commas, colons, or parentheses. Machine rule: `test/i18n.test.ts` rejects U+2013/U+2014 in catalogs, extended here repo-wide.
- CRITICAL EXCEPTION: the dash rule applies ONLY to prose. In inline code, code blocks, commands, and argv, keep `--` intact. `--` before user paths in `src/git.ts` is a security invariant; CLI flags (`--noEmit`, `--watch`, `--production`, `--cached`) must remain intact. NEVER run a global find-replace for `--`.
- Zero emojis, zero stickers, zero decorative ASCII art, zero badges. Plain Markdown only.
- Professional English, declarative sentences, high density over expressiveness (tables and bullets over paragraphs). No hype, no pleasantries ("we are excited to"), no filler words (very, really, simply, just, obviously).
- Write instructions in imperative mood ("Run X", "Never import Y"), not narrative or optional advice.
- Scope: applies to `AGENTS.md`, `README.md`, `CHANGELOG.md`, `dokumen/**`, code docblocks, commit messages, and PR descriptions.