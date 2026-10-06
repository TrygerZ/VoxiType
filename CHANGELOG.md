# Changelog

All notable changes to VoxiType are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.6.8] - 2026-10-06

### Added
- Added a `set_mic_meter` command, increasing the IPC command count from 48 to 49.

### Fixed
- Fixed the onboarding microphone level meter staying at zero because audio levels were only emitted during recording. The selected device now streams levels while the onboarding step is open.
- Fixed onboarding meter emitters remaining active after a rapid device change. Generation checks stop superseded emitters and cancel stale captures.

## [0.6.7] - 2026-10-06

### Fixed
- Fixed API keys typed shortly before leaving Settings being dropped within the debounce window. The pending value is now saved on unmount.
- Fixed file-transcription batches remaining locked in the running state when progress-listener subscription failed. The failure now resets the batch state and shows an error.
- Fixed history rows reverting to stale data when an in-flight fetch completed after a delete, clear, or pin action. History mutations now invalidate older fetches, and dictionary and snippet stores reject out-of-order load results.
- Fixed list loading indicators remaining active after successful mutations invalidated an in-flight fetch. Mutations now clear the loading state.
- Fixed waveform listener failures producing unhandled promise rejections.
- Fixed unknown mode IDs displaying raw translation keys. External IDs now display as readable labels.
- Fixed onboarding language-save failures providing no user feedback.
- Fixed copy buttons showing a success indicator when the clipboard write failed.

### Changed
- Kept error toasts visible for 8 seconds and announced them to screen readers. Success and info toasts remain visible for 3 seconds.
- Delayed downloaded file cleanup by one second so WebView2 can commit the download.
- Consolidated six local error-code helpers into one shared helper, removing duplicate code and type casts.
- Removed unused audio-level state and the waveform level prop.

## [0.6.6] - 2026-10-06

### Fixed
- Fixed mid-file audio decode failures discarding completed transcript text. Non-empty partial transcripts are now retained, with the decode reason reported unless an earlier STT issue already explains the incomplete result. Failures before any text is transcribed still return the decode error.
- Fixed history insert failures discarding completed file transcripts. The failure is logged, and the result is still returned.
- Fixed the resampler flush appending raw interleaved samples from an incomplete frame to mono audio. The incomplete frame is now dropped.
- Fixed PDF exports replacing 18 defined CP1252 WinAnsi characters with question marks. The mapping now covers all defined characters in the 0x80-0x9F range; undefined positions still export as question marks.

## [0.6.5] - 2026-10-06

### Fixed
- Fixed clipboard restoration replacing content copied while dictated text was on the clipboard. Restoration now skips when the clipboard no longer contains the dictated text.
- Fixed quiet speech passing the noise gate only to be discarded by the voice threshold without notice. The gate now derives from the voice threshold, and discarded no-speech recordings show an informational notice.
- Fixed captured audio growing without bound if the level-emitter task failed. A 300-second sample cap now bounds the capture ring independently; resampler flush can add at most one block beyond the cap.
- Fixed command-mode verification accepting an unknown recording target or confusing two windows in the same process. It now checks the exact window handle and fails closed when verification is unavailable.
- Fixed command-mode keystrokes running on an asynchronous runtime worker. Command execution now runs on the blocking pool.
- Fixed startup hotkey conflicts leaving the app without a working hotkey or user-visible feedback. The failure is persisted and shown once as an error toast; rebinding clears it only after registration succeeds.
- Fixed process paths longer than the 260-character buffer disabling per-app mode detection. The process path buffer now supports up to 1024 UTF-16 units.

### Changed
- Logged sound cue playback and stream failures at warn level instead of debug.
- Renamed the tray item to "Toggle Recording" to reflect that it toggles recording.

## [0.6.4] - 2026-10-06

### Fixed
- Fixed the rule-based cleaner inserting spaces inside numbers, times, and web addresses. `3.14` became `3. 14`, `12:30` became `12: 30`, and `example.com` became `example. Com`.
- Fixed the default filler list removing the legitimate words "like" and "you know" from dictated text.
- Fixed long dictations silently falling back to the rule-based cleaner because the Groq completion limit was fixed at 500 tokens. The limit now scales with the input length up to 8192 tokens.
- Fixed an empty whisper.cpp transcription being saved as an empty history entry instead of reporting an error.
- Fixed history and telemetry crediting the selected engine when the rule-based fallback actually produced the text.
- Fixed missing confidence data from Groq silently skipping the English verification pass. Unknown confidence now triggers verification.
- Fixed whisper.cpp failure output reaching logs and the UI unsanitized.

### Security
- Redacted Groq API keys from debug output and stored a SHA-256 hash instead of the raw key in the speech-to-text engine cache key.

### Changed
- Honored server Retry-After headers on Groq rate limits and added jitter to exponential retry backoff.
- Added an explicit 30-second timeout to Groq LLM requests.
- Returned an error instead of panicking when building the shared HTTP client fails.

## [0.6.3] - 2026-10-06

### Fixed
- Fixed unknown engine, language, and hotkey mode values being silently accepted or coerced. Settings are now validated when written and when read; an unrecognized value is rejected instead of falling back to a different engine.
- Fixed whisper.cpp thread counts above 32 being persisted. Thread counts are now clamped to 1 through 32 in both the settings inputs and the backend.
- Fixed `re_inject` inserting text while a dictation was still recording or processing. Re-injection is now rejected unless the pipeline is idle or in the error state.
- Fixed restarting the app while a recording or file transcription was active. Restart now asks for confirmation before interrupting the work.
- Fixed concurrent file dialogs opening twice on rapid double-click. The export directory and data directory pickers now share the same single-dialog guard as the audio picker.
- Added length limits to all IPC inputs that previously accepted unbounded text, including dictionary, snippet, per-app, settings, search, and export fields.

### Security
- Escaped leading `=`, `+`, `-`, `@`, tab, and carriage return characters in exported CSV history, preventing spreadsheet formula injection.
- Restricted the update release link to `https://github.com/` URLs; other schemes and hosts are discarded.
- Opened external URLs through ShellExecuteW instead of `cmd /c start`, removing the command-line percent-expansion surface.

## [0.6.2] - 2026-10-04

### Fixed
- Fixed data-directory recovery treating a graduated JSON marker as a filesystem path, which could replace a valid database with an empty one when the target master key was missing. Migration now uses the marker's active directory as its source.
- Fixed interrupted master key writes corrupting `master.key` and making stored API keys unreadable. Key files are now written atomically.

### Security
- Rejected custom data directories whose ACL grants write or control rights to an unapproved principal. Common read grants are also rejected when they overlap those rights. The error identifies the offending account; allowed principals are the current user, SYSTEM, Administrators, CREATOR OWNER, and OWNER RIGHTS.
- Required the whisper.cpp binary path to match a token written when the binary is selected in Settings. Existing whisper.cpp setups must re-select the binary once after updating.
- Restricted legacy unencrypted `master.key` files to the default data directory. A raw key in a custom directory now fails closed and requires the API key to be entered again in Settings; migration re-wraps legitimate legacy keys.

### Changed
- Limited log retention to 14 daily files and stored logs under the active data directory.

## [0.6.1] - 2026-10-04

### Fixed
- Fixed dictated text going to the floating widget instead of the target app after the widget reappeared. The widget window no longer takes keyboard focus.
- Fixed duplicate start or stop requests causing an "Invalid state transition" error. The losing request is now ignored.
- Fixed a late result from an earlier dictation forcing a newer recording into the error state. Results from older sessions are now ignored.
- Fixed the keystroke fallback cutting text at the first line break. Multi-line text is now typed in full, and the recorded character count matches the injected text.
- Fixed paste and voice command shortcuts failing on non-Latin keyboard layouts. Shortcuts now use layout-independent keys.

### Security
- Restricted the Content Security Policy `connect-src` directive to the app's own origin. The UI makes no direct network calls, so the Groq and Ollama endpoints were removed.

## [0.6.0] - 2026-10-03

### Added
- Added file transcription for MP3, WAV, M4A, AAC, FLAC, and OGG files up to 60 minutes. Users choose Groq Whisper or whisper.cpp and the audio language per job, can toggle LLM cleanup with a selectable engine and dictionary replacements, and can cancel a running job. An STT failure partway through keeps the finished part of the transcript and reports what was skipped. Results are saved to History under the File mode. Dictation keeps working while a file is processed.
- Added bulk file transcription: up to 20 files per batch, processed one at a time. A failed file is reported and the batch continues; cancel stops the remaining files. Each file shows a live processing timer and its final duration.
- Added transcript export as TXT, Word (DOCX), or PDF, one file per audio file, named after the audio and never overwriting existing files. PDF uses a built-in Latin font; characters outside the supported WinAnsi set print as `?`.

### Changed
- whisper.cpp process timeout now scales with audio length (minimum 300 s) instead of a fixed 300 s.
- Usage totals and the WPM gauge exclude file transcriptions.

## [0.5.2] - 2026-10-02

### Fixed
- Fixed the floating widget remaining invisible after a long idle period with auto-hide enabled. The WebView2 renderer could stop responding while hidden. The widget now acknowledges reveal requests, and the backend reloads the widget webview when no acknowledgement arrives within 1.5 s, then resynchronizes pipeline state.
- Fixed short utterances of 1 s or less being discarded by the accidental-tap guard. The guard now discards only recordings without at least 250 ms of sustained voice.
- Fixed the stop cue peaking above the 0.4 playback limit. The cue is now normalized to a -9 dBFS peak and its DC offset is removed; duration, sample rate, and timbre are unchanged.

### Changed
- Split the start sound cue into a two-stage woodblock pair: a deep press cue plays the moment the hotkey registers, and a consonant ready cue a perfect fifth higher plays once the microphone stream is live, signaling when speech can begin. Hotkey-to-ready latency was logged at debug level.

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

[unreleased]: https://github.com/TrygerZ/VoxiType/compare/v0.6.8...HEAD
[0.6.8]: https://github.com/TrygerZ/VoxiType/compare/v0.6.7...v0.6.8
[0.6.7]: https://github.com/TrygerZ/VoxiType/compare/v0.6.6...v0.6.7
[0.6.6]: https://github.com/TrygerZ/VoxiType/compare/v0.6.5...v0.6.6
[0.6.5]: https://github.com/TrygerZ/VoxiType/compare/v0.6.4...v0.6.5
[0.6.4]: https://github.com/TrygerZ/VoxiType/compare/v0.6.3...v0.6.4
[0.6.3]: https://github.com/TrygerZ/VoxiType/compare/v0.6.2...v0.6.3
[0.6.2]: https://github.com/TrygerZ/VoxiType/compare/v0.6.1...v0.6.2
[0.6.1]: https://github.com/TrygerZ/VoxiType/compare/v0.6.0...v0.6.1
[0.6.0]: https://github.com/TrygerZ/VoxiType/compare/v0.5.2...v0.6.0
[0.5.2]: https://github.com/TrygerZ/VoxiType/compare/v0.5.1...v0.5.2
[0.5.1]: https://github.com/TrygerZ/VoxiType/compare/v0.5.0...v0.5.1
[0.5.0]: https://github.com/TrygerZ/VoxiType/compare/v0.4.10...v0.5.0
[0.4.10]: https://github.com/TrygerZ/VoxiType/compare/v0.4.9...v0.4.10
[0.4.9]: https://github.com/TrygerZ/VoxiType/compare/v0.4.8...v0.4.9
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
