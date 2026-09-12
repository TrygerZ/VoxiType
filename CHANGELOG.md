# Changelog

All notable changes to VoxiType are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Fixed
- Surface settings load failure and gate onboarding on clean state
- Serialize hotkey dispatch and guard PTT key-down state

### Added
- Add RCA report for v0.4.3 onboarding and PTT race bugs
- Add MIT LICENSE file and rewrite README for v0.4.3 accuracy

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

[unreleased]: https://github.com/TrygerZ/VoxiType/compare/v0.4.3...HEAD
[0.4.3]: https://github.com/TrygerZ/VoxiType/compare/v0.4.2...v0.4.3
[0.4.2]: https://github.com/TrygerZ/VoxiType/compare/v0.4.1...v0.4.2
[0.4.1]: https://github.com/TrygerZ/VoxiType/compare/v0.4.0...v0.4.1
[0.4.0]: https://github.com/TrygerZ/VoxiType/compare/v0.3.2...v0.4.0
[0.3.2]: https://github.com/TrygerZ/VoxiType/compare/v0.3.1...v0.3.2
[0.3.1]: https://github.com/TrygerZ/VoxiType/compare/v0.3.0...v0.3.1
[0.3.0]: https://github.com/TrygerZ/VoxiType/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/TrygerZ/VoxiType/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/TrygerZ/VoxiType/releases/tag/v0.1.0
