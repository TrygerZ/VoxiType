# Changelog

All notable changes to VoxiType are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.5.1] - 2026-09-18

### Fixed
- Fixed the first launch showing "Failed to load settings" because the application window loaded before the backend was ready
- Fixed the microphone remaining active after recording stopped and stale audio appearing in a later session when stop or cancel overlapped audio device initialization
- Fixed history and dictionary exports failing to save without showing an error
- Fixed long recordings failing on slow connections after audio was uploaded up to four times; upload timeouts now match the maximum recording duration and timed-out uploads are not retried
- Fixed log files not being written when the selected data directory failed validation and the app fell back to the default directory
- Fixed a stored Groq API key being reported as unset when decryption failed after a data directory migration or backup restore; the error now identifies the decryption problem
- Fixed the History search keyword being cleared when a transcription completed while the user was typing
- Fixed failed history searches showing no error while retaining the previous results
- Fixed a manually selected onboarding microphone being replaced by the default device when device scanning finished late
- Fixed dictated text remaining in the clipboard when the previous clipboard content was not text

### Changed
- Reduced Indonesian transcription retries by running second-language verification only when confidence in the first result is low; explicitly selecting Indonesian skips verification
- Read settings once per transcription instead of approximately fifteen times, reducing repeated database blocking on the interface
- Stopped the floating widget from loading unused history and statistics data for each transcription
- Moved audio device initialization off the application runtime so level indicators and the overlay remain responsive when a device responds slowly
- Debounced floating widget position saves until dragging stops instead of scheduling a database write for every movement
- Stopped the widget idle monitor cleanly during application shutdown and removed its twice-per-second duplicate database read
- Versioned database schema migrations and ran them in transactions so failures are surfaced; existing databases remain readable without data loss

### Security
- Removed the unused permission that allowed the web interface to emit forged internal events
- Tightened the Content Security Policy with `object-src`, `base-uri`, and `frame-ancestors`
- Pinned runtime dependency versions exactly so lockfile updates cannot introduce untested versions into release builds

## [0.5.0] - 2026-09-16

### Added
- Floating widget auto-hide after configurable idle period (3-60 seconds or disabled, default off)
- Animated widget visibility transitions with capsule shrink, fade out, and reverse reveal sequence
- Auto-hide setting with duration input in General tab Appearance section
- Idle timer reset on hover, click, or hold interactions with the floating widget
- Auto-hide suspension during recording and processing states
- Reduced-motion preference support for widget visibility animations

## [0.4.10] - 2026-09-13

### Security
- Pin all GitHub Actions to immutable commit SHAs in CI and release workflows
- Add cargo audit to the CI pipeline
- Upgrade h2 past RUSTSEC-2026-0258 and quick-xml past RUSTSEC-2026-0194 and RUSTSEC-2026-0195
- Stop echoing the decrypted Groq API key to the UI and expose a configured indicator instead

## [0.4.9] - 2026-09-13

### Fixed
- Reveal hover action rows on keyboard focus across history, dictionary, snippets, and home views
- Add accessible labels to icon-only buttons and restore focus-visible rings on interactive controls
- Cancel stale test status timers on retest in STT settings, onboarding, and LLM settings
- Clear copy feedback and toast timers on unmount and cleanup
- Cancel debounced API key updates when their component unmounts
- Move i18n locale mutation out of the render path into store subscriptions
- Guard dictionary, snippet, shortcut, and per-app mutations against duplicate in-flight submissions
- Scope audio level updates to the waveform so HomeView no longer rerenders at 20 Hz
- Wrap onboarding in the application error boundary
- Replace blind settings casts with type guards across all settings tabs and onboarding
- Route dictionary panel strings through the i18n catalog
- Detach the waveform audio listener reliably on fast unmount
- Normalize empty usage statistics to zeroed defaults to avoid error boundary crashes

### Changed
- Document that CSP style-src unsafe-inline remains required for the WebView2 transparency workaround

## [0.4.8] - 2026-09-13

### Fixed
- Expose load errors with a retry action in history, dictionary, snippets, and stats panels instead of rendering empty states
- Surface mutation failures across settings, history, dictionary, snippets, and per-app actions through a shared invoke wrapper and toast
- Sequence settings loads so a slow earlier response can no longer overwrite newer state
- Reload settings after a failed optimistic update to reconcile displayed values
- Surface recording start and stop failures instead of leaving the button silent
- Re-check the foreground window before command-mode injection and abort when focus drifted

## [0.4.7] - 2026-09-13

### Fixed
- Preserve non-text clipboard content such as images during injection
- Restore the original clipboard content on every paste outcome through an RAII guard
- Release modifier keys when keystroke paste errors mid-sequence
- Persist whisper binary and model paths atomically in one transaction
- Reject non-string API key values instead of silently clearing the stored key
- Create the encryption master key exclusively and re-read on concurrent first-run conflicts
- Checkpoint the source WAL file before data-directory migration copies the database
- Reject history exports above the record cap instead of silently truncating
- Use an LLM-specific error code for missing LLM API keys so frontend labels are correct
- Cap the whisper initial prompt to its effective context window
- Serialize start and stop sound cues so they no longer overlap
- Fail closed when plaintext legacy API keys cannot be encrypted during migration
- Bind whisper execution to the canonical binary path selected in the picker

### Changed
- Drop redundant snippets trigger index covered by the unique constraint

## [0.4.6] - 2026-09-13

### Fixed
- Cancel partially initialized audio capture on startup failure instead of leaving it orphaned
- Kill hung whisper-cli processes on timeout and return a typed error instead of wedging the pipeline in Processing
- Serialize clipboard injection transactions process-wide to prevent concurrent read, write, paste, and restore races
- Keep stop and cancel responsive while the audio device initializes by invalidating aborted sessions via generation IDs

## [0.4.5] - 2026-09-13

### Fixed
- Prevent recovery-path panic by making logging initialization idempotent
- Reject truncated LLM responses with finish_reason length instead of injecting incomplete text
- Stop recordings at the exact 300-second cap boundary
- Reject backslash URL authority ambiguity in open_url allowlist
- Sanitize Ollama provider error bodies before logging
- Resolve npm audit advisories in dev dependencies (esbuild, postcss, browserslist, nanoid, js-yaml, baseline-browser-mapping)

### Security
- Ignore local master key, database, and data marker files in git
- Set explicit least-privilege permissions in CI workflows
- Remove unused asset protocol entry from CSP

## [0.4.4] - 2026-09-12

### Fixed
- Surface settings load failure and gate onboarding on clean state
- Serialize hotkey dispatch and guard PTT key-down state
- Persist data directory active location label in Settings after Apply
- Record data directory fallback errors in diagnostic file with recovery path
- Clear stale diagnostic errors on healthy startup

### Added
- Add RCA report for v0.4.3 onboarding and PTT race bugs
- Add MIT LICENSE file and rewrite README for v0.4.3 accuracy
- Add Restart now button in Settings General and onboarding after data directory change

### Changed
- Update AGENTS.md documentation

## [0.4.3] - 2026-08-24

### Added
- Modularize onboarding flow with step-by-step configuration
- Configurable data directory with validated migration (DB, master key, logs)
- DPAPI-backed encryption hardening for Windows
- Offload text injection to background thread

### Fixed
- Recover from custom data directory failures and graduate marker
- Resolve clipboard injection race condition
- Eliminate white flash on floating widget overlay (workaround for Tauri issue 14515)

### Changed
- Harden IPC input validation
- Improve security, safety, and frontend robustness
- Secure whisper paths with validation

## [0.4.2] - 2026-07-12

### Added
- Prevent white-square flash and handle 404 gracefully
- Navigation controls, how-it-works section, and next-steps guidance in onboarding
- Language-aware STT re-verification, LLM prompting, and translation guard

### Changed
- Enhance onboarding UX with improved flow and refined transitions
- Redesign onboarding welcome step
- Extract debounced API key input into reusable hook

## [0.4.1] - 2026-07-08

### Added
- Full i18n system with Indonesian and English translations

### Changed
- Improve accessibility in UI components and onboarding UX

## [0.4.0] - 2026-07-07

### Added
- Offline whisper.cpp STT engine with multi-engine support (feature-gated)
- Off LLM engine mode (pass-through without formatting)
- Native file picker for whisper binary and model paths
- Toast notification system and Groq API test connection command
- Custom NSIS installer with custom icons and shortcut repair hooks

### Fixed
- Add error-safe store operations and UI fallbacks for error states
- Fix WAV odd chunk padding

### Changed
- Hook optimizations
- Update README and all Tauri icons with new VoxiType icon
- Update offline whisper.cpp documentation

## [0.3.2] - 2026-07-03

### Added
- Usage statistics dashboard with server-side aggregation
- Redesign dashboard stat cards with custom SVG icons and WPM half-ring gauge
- Compute usage stats from full transcription history instead of telemetry rollup
- Localize hardcoded unit strings in HomeView stats

### Fixed
- Hardening, filler cleanup, and state hygiene

### Changed
- Align stat card values and units with CSS grid layout

## [0.3.1] - 2026-06-30

### Added
- Recording timeout, prefix FTS5 search in history, and debounced history updates
- CSP hardening

### Fixed
- Improve clipboard injection reliability with retry backoff
- Improve silence trimming

## [0.3.0] - 2026-06-27

### Added
- 5-step onboarding flow with Groq API test and hotkey setup
- I18n keys for all onboarding steps
- Test Groq API command to verify API key and IPC wrapper
- Prevent double instance with named mutex (WebView2 conflict prevention)

### Removed
- Remove local Whisper STT (simplify to Groq-only engine)
- Remove VAD and ring buffer
- Cleanup dead code and unused commands

### Fixed
- Add error handling to all save functions, i18n features, and hotkey operations
- Add i18n for onboarding labels

### Changed
- Improve onboarding UX, modularize commands structure, and replace sound files with WAV format

## [0.2.0] - 2026-06-25

### Added
- Persistent floating widget, home dashboard redesign, and STT engine caching

### Fixed
- Prevent double STT transcription in command mode
- Fix Unicode panic in dictionary replacement

### Changed
- Complete visual redesign of UI
- Improve resilience with mutex poisoning recovery and processing timeout
- Apply cargo fmt across audio/capture, commands, lib, and overlay

## [0.1.0] - 2026-06-21

### Added
- Initial VoxiType foundation with Tauri 2.x voice-to-text desktop app
- API key encryption with AES-256-GCM
- Per-app formatting modes with active window detection
- Voice command mode for keystroke macros
- Enhanced UI with settings panels
- Clear history with keep-pinned option
- Groq STT language parameter support
- Harden LLM prompts
- State machine error handling overhaul
- Overlay positioning improvements
- I18n reactivity

[unreleased]: https://github.com/TrygerZ/VoxiType/compare/v0.4.8...HEAD
[0.4.8]: https://github.com/TrygerZ/VoxiType/compare/v0.4.7...v0.4.8
[0.4.7]: https://github.com/TrygerZ/VoxiType/compare/v0.4.6...v0.4.7
[0.4.6]: https://github.com/TrygerZ/VoxiType/compare/v0.4.5...v0.4.6
[0.4.5]: https://github.com/TrygerZ/VoxiType/compare/v0.4.4...v0.4.5
[0.4.4]: https://github.com/TrygerZ/VoxiType/compare/v0.4.3...v0.4.4
[0.4.3]: https://github.com/TrygerZ/VoxiType/compare/v0.4.2...v0.4.3
[0.4.2]: https://github.com/TrygerZ/VoxiType/compare/v0.4.1...v0.4.2
[0.4.1]: https://github.com/TrygerZ/VoxiType/compare/v0.4.0...v0.4.1
[0.4.0]: https://github.com/TrygerZ/VoxiType/compare/v0.3.2...v0.4.0
[0.3.2]: https://github.com/TrygerZ/VoxiType/compare/v0.3.1...v0.3.2
[0.3.1]: https://github.com/TrygerZ/VoxiType/compare/v0.3.0...v0.3.1
[0.3.0]: https://github.com/TrygerZ/VoxiType/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/TrygerZ/VoxiType/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/TrygerZ/VoxiType/releases/tag/v0.1.0
