# Session Handoff - 2026-07-06

## Phase
Offline whisper.cpp dictation support.

## Built
- Added a real `whisper_cpp` STT engine that writes captured audio to a temp WAV, calls a configured `whisper-cli` binary, reads the generated text output, and returns it through the existing STT trait.
- Wired `stt_engine`, `whisper_cpp_binary_path`, `whisper_cpp_model_path`, and `whisper_cpp_threads` into runtime engine selection and caching.
- Added `test_whisper_cpp` IPC command and frontend controls in `Settings -> STT`.
- Added offline setup and usage guide at `docs/offline-whisper-cpp.md`.
- Updated README to mention offline whisper.cpp support.

## Decisions
- Used the whisper.cpp CLI instead of adding a Rust binding or new dependency. This keeps the feature usable with the user's local `whisper-cli` install and avoids CMake/libclang dependency churn inside the app build.
- Kept Groq as the default STT engine; users opt into offline mode from settings.
- The offline test command runs a short silent WAV through the configured binary/model to validate that VoxiType can execute whisper.cpp and load the model.

## Validation
- `rtk cargo test --no-default-features`: 51 passed.
- `rtk cargo clippy --no-default-features -- -D warnings`: passed.
- `rtk npx tsc --noEmit`: passed.
- `rtk npm run build`: passed.
- `rtk npm run tauri build -- --bundles nsis`: passed and produced `src-tauri/target/release/bundle/nsis/VoxiType_0.3.2_x64-setup.exe`.

## Known Issues
- Full `rtk npm run tauri build` with all bundle targets still fails at MSI bundling via WiX `light.exe`; the release executable and NSIS bundle build successfully.
- A real whisper.cpp model/binary runtime test on this machine was attempted but blocked by local tooling/TLS issues: no CMake, MinGW build failed on missing `intrin.h`, and Windows Schannel downloads failed. Git LFS did fetch `ggml-tiny.bin`, but test assets were removed after cleanup.

HANDOFF: offline whisper.cpp dictation completed - ready for next session

## Follow-up - 2026-07-06

### Phase
First-run setup guidance inside the app.

### Built
- Reworked first-run onboarding so new users choose between Groq API and offline whisper.cpp setup before configuring the hotkey.
- Added clear in-app Groq instructions: open Groq Console, create/copy API key, paste it, and test the connection.
- Added clear in-app offline instructions for users with and without CMake:
  - No CMake: use the official whisper.cpp prebuilt releases.
  - With CMake: clone/build whisper.cpp with CMake commands.
  - Model setup: open the GGML model page, download a model such as `ggml-base.bin`, paste the full model path, and test the offline engine.
- Added bilingual Indonesian/English onboarding copy and links to official whisper.cpp release/source/model pages.

### Validation
- `rtk npx tsc --noEmit`: passed.
- `rtk npm run build`: passed.
- `rtk git diff --check`: passed.
- `rtk npm run tauri build -- --bundles nsis`: passed and produced `src-tauri/target/release/bundle/nsis/VoxiType_0.3.2_x64-setup.exe`.

### Known Issues
- Full all-target Tauri build still has the previous MSI/WiX issue; NSIS and release executable build successfully.

HANDOFF: onboarding setup guidance completed - ready for next session

## Follow-up - 2026-08-24 Custom Data Migration

### Phase
Copy-on-migrate backend data directory support.

### Built
- Added idempotent migration for `data/voxitype.db` and `master.key` before `Database::open`.
- Critical copies use temporary files, SHA-256 plus size verification, then rename.
- Invalid/partial migration removes the marker and falls back to the default directory.
- `logs/` copies best-effort; failures warn only.
- Added fresh-target, existing-target, and missing-source tests.

### Validation
- `rtk cargo fmt --all -- --check`: passed.
- `rtk cargo test --no-default-features`: 136 passed.
- `rtk cargo clippy --no-default-features -- -D warnings`: passed.
- `graphify . --update --no-viz`: blocked; graphify requires semantic-extraction API key for 62 non-code files.

HANDOFF: custom data migration completed - ready for next session

## Follow-up - 2026-08-24 Frontend Data Directory

### Phase
Frontend integration for optional data-directory selection.

### Built
- Added typed `pickDataDirectory`, `setDataDirectory`, and `getDataDirectory` IPC wrappers.
- Added optional onboarding step after `stt_setup`, before `hotkey`; skip preserves default behavior.
- Added Settings -> General -> System storage controls with preview, apply, success, restart warning, and friendly validation errors.
- Added Indonesian/English copy, including explicit restart behavior.

### Validation
- `rtk npx tsc --noEmit`: passed.
- `rtk npm run build`: passed.
- `rtk npm run test`: passed, 4 tests.
- `graphify update .`: passed; 1 SQL parser dependency warning.

HANDOFF: frontend data-directory integration completed - ready for next session

## Follow-up - 2026-07-06 Path Picker

### Phase
Clearer offline setup path guidance and file picker.

### Built
- Added `pick_setup_file` IPC command for selecting `whisper-cli.exe` or `ggml-*.bin` through a native Windows file picker.
- Added Browse buttons to first-run offline setup for the whisper.cpp binary and GGML model paths.
- Added Browse buttons to `Settings -> STT` so users can fix paths later.
- Expanded onboarding copy to explain what a path is, which exact files to choose, and how to fill paths using File Explorer or manual `PATH`.

### Validation
- `rtk npx tsc --noEmit`: passed.
- `rtk npm run build`: passed.
- `rtk cargo test --no-default-features`: 51 passed.
- `rtk cargo clippy --no-default-features -- -D warnings`: passed.
- `rtk npm run tauri build`: passed and produced `src-tauri/target/release/bundle/nsis/VoxiType_0.4.0_x64-setup.exe`.

HANDOFF: path picker setup guidance completed - ready for next session

## Follow-up - 2026-07-06 Onboarding Redesign

### Phase
First-run onboarding hierarchy and usability pass.

### Built
- Reworked onboarding into a guided wizard with a persistent desktop step rail and compact mobile progress bar.
- Added a clearer opening explanation that frames the setup as basic preferences, transcription engine, and hotkey.
- Moved offline whisper.cpp path guidance to the beginning of the offline setup section.
- Kept Browse buttons beside the `whisper-cli.exe` and `ggml-*.bin` path fields, with responsive layout for narrow screens.
- Added Indonesian/English copy for the new wizard labels, setup notes, and completion state.

### Validation
- `rtk npx tsc --noEmit`: passed.
- `rtk npm run build`: passed.
- `rtk cargo test --no-default-features`: 51 passed.
- `rtk cargo clippy --no-default-features -- -D warnings`: passed.
- `rtk npm run tauri build`: passed and produced `src-tauri/target/release/bundle/nsis/VoxiType_0.4.0_x64-setup.exe`.
- `rtk git diff --check`: passed.

HANDOFF: onboarding redesign completed - ready for next session

## Follow-up - 2026-07-06 Deep Debug

### Phase
Systematic project debug, reproducibility checks, fixes, and re-validation.

### Fixed
- Fixed snippet/dictionary add flows so backend failures are surfaced and forms are only cleared after successful saves.
- Added a duplicate snippet trigger regression test to document the backend failure case.
- Fixed LLM `Off` mode so it leaves STT text unchanged instead of running the rule-based cleaner.
- Fixed empty Ollama responses so they are treated as errors and can fall back instead of injecting blank text.
- Fixed WAV parser handling for valid odd-sized RIFF chunks with padding.
- Fixed hotkey persistence order so a failed rebind is not saved as the active setting.
- Fixed tray `About` navigation, which previously emitted a route the frontend ignored.
- Removed unused clipboard read, hotkey unregister wrapper, and dictionary usage increment dead code.
- Replaced `any` in the debounce helper with tuple generics.
- Corrected `open_url` failures to use an internal/system error instead of an STT error.

### Validation
- `rtk npx tsc --noEmit`: passed.
- `rtk npm run build`: passed.
- `rtk cargo check --all-features`: passed.
- `rtk cargo test --no-default-features`: 56 passed.
- `rtk cargo clippy --no-default-features -- -D warnings`: passed.
- `rtk npm run tauri build`: passed and produced `src-tauri/target/release/bundle/nsis/VoxiType_0.4.0_x64-setup.exe`.

### Known Issues
- Tauri build warns that bundle identifier `com.voxitype.app` ends with `.app`; Windows NSIS packaging still succeeds.
- `rtk gain` could not open its tracking database in this environment, but all project checks/builds ran through `rtk`.

HANDOFF: deep debug cleanup completed - ready for next session

## Follow-up - 2026-07-06 Documentation Refresh

### Phase
README and offline whisper.cpp guide updated for VoxiType 0.4.0.

### Built
- Updated README version, feature list, setup notes, command table, project structure, settings overview, recording flow, and IPC command count to match the current app.
- Updated `docs/offline-whisper-cpp.md` with the current onboarding flow, native Browse buttons, prebuilt whisper.cpp setup path, local/offline LLM behavior, and troubleshooting notes.

### Validation
- `rtk git diff --check`: passed.
- Checked README and offline guide for stale `0.3.2` and `31 commands` references; none remain in those docs.

HANDOFF: documentation refresh completed - ready for next session

## Follow-up - 2026-07-08 New Icon & Preview Image

### Phase
Brand refresh with custom app icon and README preview.

### Built
- Replaced all Tauri platform icons (Windows .ico, macOS .icns, iOS, Android) with the new VoxiType icon.
- Added `icon/VoxiType_Icon.png` as the README header logo.
- Updated `public/logo.png` with the new icon.
- Replaced the README preview screenshot with the latest app UI.

### Validation
- `rtk npm run tauri build -- --bundles nsis`: passed and produced `src-tauri/target/release/bundle/nsis/VoxiType_0.4.0_x64-setup.exe`.

HANDOFF: brand refresh completed - ready for next session

## Follow-up - 2026-07-08 NSIS Installer Customization

### Phase
Custom installer icons and shortcut repair hooks.

### Built
- Added custom NSIS installer and uninstaller icons (`icons/icon.ico`).
- Added `installer-hooks.nsh` with a `RepairShortcutIfExists` macro that re-creates Start Menu and Desktop shortcuts with the correct AppUserModelId after installation.
- Added `build.rs` triggers for icon and hook file changes.
- Updated `lib.rs` to apply the new window icon at runtime.

### Validation
- `rtk npm run tauri build -- --bundles nsis`: passed and produced `src-tauri/target/release/bundle/nsis/VoxiType_0.4.1_x64-setup.exe`.

### Known Issues
- Tauri build still warns that bundle identifier `com.voxitype.app` ends with `.app`; Windows NSIS packaging still succeeds.

HANDOFF: NSIS installer customization completed - ready for next session
## Follow-up - 2026-07-12 Bug Fixes

### Phase
Fix updater 404 crash and dev-mode overlay white-square glitch.

### Built
- Corrected updater hardcoded repo from oxitype/voxitype to actual TrygerZ/VoxiType.
- Made 404 responses from GitHub Releases API silent ("no update") instead of a hard network error; only 5xx and JSON parse failures surface to the user.
- Added 4 unit tests covering 404, 200, draft, and server-error cases.
- Added eveal_floating_widget IPC and frontend wrapper so the overlay window stays hidden until React mounts and paints its transparent content.
- Guarded pply_enabled and ensure_visible with a WIDGET_READY flag so show() never fires before the DOM is ready, eliminating the white flash in 	auri dev.

### Validation
- tk cargo test --no-default-features: 81 passed.
- tk cargo clippy --no-default-features -- -D warnings: passed.
- tk npx tsc --noEmit: passed.

HANDOFF: updater crash + dev-mode white-square glitch fixed - ready for next session
---

## Session: Full Review + Remediation (23 Agustus 2026)
- Full review multi-agent: 15 temuan (1 HIGH bug, 4 MEDIUM, 7 LOW, 3 INFO) — laporan di FULL_REVIEW_REPORT.md / BUG_HUNT_REPORT.md / SECURITY_AUDIT_REPORT.md
- Remediasi 11/11 fix selesai & terverifikasi (REMEDIATION_REPORT.md): cargo test 94→127 PASS (+33), clippy/fmt/tsc/build bersih
- Key decisions: DPAPI utk master.key Windows (format dpapi:v1:, migrasi legacy otomatis); whisper path dikeluarkan dari IPC allowlist → command gated via dialog OS; retry fail-fast utk HTTP 4xx permanen; translasi jalan utk source unresolved dengan auto-detect
- Known issues: 3 temuan INFO tidak diremediasi (arsitektural/advisori) — S-I1 CSP unsafe-inline, S-I2 enkripsi DB at-rest, S-I3 cloud disclosure onboarding
- Pending: commit changes (belum di-commit), verifikasi manual race drill PTT & whisper.cpp setup flow
HANDOFF: review+remediation completed - ready for next session

## Follow-up - 2026-08-24 Documentation Handoff

### Phase
Onboarding, data-directory, and frontend testing documentation refresh.

### Built
- Refactored onboarding documentation to match `OnboardingFlow.tsx`, `types.ts`, `shared/`, and `steps/`; documented the eight-step flow: `welcome`, `quick_settings`, `microphone`, `stt_setup`, `data_directory`, `hotkey`, `smoke_test`, `complete`.
- Updated IPC documentation from 37 to 39 commands, including `pick_data_directory`, `set_data_directory`, and `get_data_directory`.
- Documented `data_dir.txt`, JSON `current`/`pending` markers, plain-path backward compatibility, restart-required behavior, active-previous-directory migration source, DB/`master.key`/log migration, SHA-256 verification, atomic rename, and Windows remote/removable/UNC validation.
- Documented Vitest + React Testing Library setup: `vitest.config.ts`, `src/test/setup.ts`, two test files, four tests, and `npm run test`.
- Documented the General settings System group as the re-entry point for onboarding.

### Decisions
- Kept data-directory state outside key-value settings; the default app-data marker remains the control plane for startup resolution.
- Used `current` plus `pending` marker paths so the selected directory activates only after restart.
- Migrated from the active previous directory rather than always copying from the default directory.

### QA / Bug Hunter Findings
- Fixed onboarding re-entry detection in `src/App.tsx` by checking `settings.onboarding_completed !== true`; falsy or missing values reopen onboarding, while only explicit `true` suppresses it.
- Low-priority items deferred: full fallback banner UI (M-2), pending-path display (L-2), language double-write (L-4), hotkey `as` cast (L-5), `GeneralTab` JSX cleanup (LOW-2), and view reset on re-run (LOW-6).

### Known Issues / Follow-up
- Technical verification found that `SttSetupStep`'s normal save callback currently advances directly to `hotkey`; `data_directory` is reached through the STT skip callback. Confirm whether the intended sequence requires changing that runtime transition before publishing the onboarding sequence as universal.
- Re-run `rtk npm run test`, `rtk npx tsc --noEmit`, and `rtk git diff --check` after resolving the onboarding transition decision.

HANDOFF: documentation refresh completed - ready for next session

---

# Session Handoff - 2026-09-12

## Phase
Bug fixes v0.4.3: PTT hotkey race and onboarding re-trigger.

## Built
- `fix/hotkey-ptt-race` (1998ad3): removed per-event `async_runtime::spawn` in `src-tauri/src/hotkey/mod.rs`; added pure `ptt_action` evaluator with `PTT_KEY_DOWN` AtomicBool guard (duplicate press/release, out-of-order, lost-release self-heal); reset on register/rebind; 7 regression tests.
- `fix/onboarding-visibility` (5d0fecd): settingsStore gains `error` state; App gates onboarding on `loaded && !error && onboarding_completed` and shows retry-able storage-error view; i18n keys id+en; backend logs active DB path at startup, unified `fallback_error_message` for data-dir fallbacks, `get_app_info` exposes `data_dir`/`db_path`.
- RCA artifacts: `laporan-rca-bug-2-startup-0.4.3.md` (c128711) plus prior full RCA in session report.

## Decisions
- Hotkey events dispatched synchronously in plugin callback; ordering guaranteed by single callback thread, key_down flag absorbs duplicates. No new deps.
- Kept intentional data-dir fallback from e3bf3c8 (marker graduation), but made it loud in logs and diagnosable via `get_app_info` instead of fail-hard.
- Bug 2 trigger narrowed to silent `get_settings` IPC failure (backend data-dir paths disproven via `%APPDATA%` logs); fix is visibility + gating, so next occurrence is recorded, not masked.

## Validation
- cargo test --no-default-features: 148 passed (hotkey branch); 142 passed (onboarding branch).
- clippy -D warnings: clean both branches. tsc --noEmit: clean. vitest: 11 passed. vite build: ok.

## Pending
- Push + merge both branches: awaiting user approval.
- Runtime soak test: spam PTT on built binary; if onboarding reappears, check new startup logs for the recorded DB path.
- `graphify update .` after merge.

HANDOFF: v0.4.3 bug fixes completed - ready for merge approval

---

## Follow-up - 2026-09-12 Data Directory Persistence

### Phase
Fix data directory UX and persistence after user report of location reset and app disappearance.

### Built
- Backend `get_data_directory` returns structured `DataDirectoryStatus { active, default, pending, lastError }` instead of string (commit d33a292, branch fix/data-directory-persist).
- Atomic writes for marker (`write_marker`, `graduate_marker`) and diagnostics via temp + rename.
- Fallback failures recorded to `data_dir_error.txt` and exposed via `lastError`; marker deletion now leaves diagnostic trail.
- Healthy startup clears stale diagnostics (`finish_startup` unconditional `clear_error` when no fallback active).
- Frontend Settings GeneralTab: active location label preserved after Apply; pending-path and lastError display; loading state; busy guard; role=status/alert a11y.
- Onboarding DataDirectoryStep: payload update, busy guard.
- New tests: atomic write, pending round-trip, invalid target diagnostic, clean startup clears error. Total: cargo test 153 passed, vitest 18 passed, tsc clean.

### Decisions
- `get_data_directory` payload changed to object (breaking IPC change, internal API).
- Atomic write pattern (temp + rename) for marker and diagnostics prevents partial state.
- Fallback to default preserved from e3bf3c8; now loud in logs and diagnosable via `lastError`.
- Restart button skipped (no process plugin available).
- Follow-up deferred: temp file sweep (F-5), error code enum (F-6), onboarding pending/error display (F-7).

### Validation
- cargo test --no-default-features: 153 passed.
- cargo clippy --no-default-features -D warnings: clean.
- npx tsc --noEmit: clean.
- npm run test: 18 passed.
- npm run build: ok.

### Known Issues / Follow-up
- Push awaiting user approval.
- AV signing requires Authenticode certificate; publish SHA-256 checksum; submit false positive to Microsoft.
- Windows Security Protection History evidence needed to confirm quarantine hypothesis for user's "app disappeared" report.
- Follow-up F-5: sweep orphaned `.tmp-*` files after crash.
- Follow-up F-6: replace `lastError` string with typed error codes.
- Follow-up F-7: show pending/lastError in onboarding DataDirectoryStep.

HANDOFF: data directory persistence fix completed - ready for push approval

---

## Follow-up - 2026-09-12 Release v0.4.4

### Phase
Manual release v0.4.4 (Plan B) and CI Release workflow root cause identified.

### Built
- Released v0.4.4 manually via `gh release create`: https://github.com/TrygerZ/VoxiType/releases/tag/v0.4.4
- Asset: VoxiType_0.4.4_x64-setup.exe, tag v0.4.4 = commit 51c3617 (main).
- CHANGELOG folded [Unreleased] into [0.4.4] - 2026-09-12.
- Added Release Procedure section to AGENTS.md (primary CI path + fallback manual + release description guidelines).

### Root Cause (CI Release Workflow Failure)
- CI Release workflow failed in 1m24s (precedent: v0.4.3 also failed).
- Root cause: test devDependencies (`vitest`, `@testing-library/react`, `@testing-library/user-event`, related packages) exist in local node_modules but missing from package.json.
- `npm ci` in CI does not install them, TypeScript compilation fails TS2307 on test imports.
- NOT YET FIXED.

### Pending
- Fix package.json: add missing test devDependencies, regenerate package-lock.json.
- Check Windows Security Protection History (Defender user) for AV false positive evidence (user report "app disappeared").
- Authenticode signing remains open (no certificate).

HANDOFF: v0.4.4 released manually, CI failure root cause identified - ready for devDeps fix

---

## Follow-up - 2026-09-13 Remediation Handoff

### Phase
Full remediation of `dokumen/Full_Codebase_Review.md` completed: 44 findings plus extras.

### Versions
- v0.4.5: quick wins. Version bumped and changelogged.
- v0.4.6: lifecycle. Version bumped and changelogged.
- v0.4.7: storage and injection. Version bumped and changelogged.
- v0.4.8: frontend errors. Version bumped and changelogged.
- v0.4.9: UX and accessibility. Version bumped and changelogged.
- v0.4.10: security. Version bumped and changelogged.

### Branch Stack
`fix/remediation-phase-1-quick-wins` -> `fix/remediation-phase-2-lifecycle` -> `fix/remediation-phase-3-storage` -> `fix/remediation-phase-4-frontend` -> `fix/remediation-phase-5-ux-a11y` -> `fix/remediation-phase-6-security`.

All branches are linear and based on main at `4c8effb`.

### Push State
- Phase 1 branch partially pushed: 5 commits on origin, 6 later commits local.
- Phases 2 through 6 are local only.
- Nothing is merged to main.
- Push and merge await user approval.

### Final Verification
- qa-agent audit: 50/50 findings verified. E9 closed by commit `6e04028`.
- Rust tests: 192 passed.
- Vitest: 51 passed.
- TypeScript check and builds: green.
- `npm audit`: 0 vulnerabilities.
- `cargo audit`: 0 vulnerabilities, 9 transitive warnings.

### Known Accepted Items
- VX-05 `unsafe-inline` remains for the WebView2 transparency workaround. Documented in the changelog.
- cargo audit warnings for unmaintained or unsound transitive dependencies are not CI-gated. Dependencies arrive through Tauri. An optional `audit.toml` ignore can be added later.
- `STTTab` local settings helpers remain because their semantics differ from `settingsGuards`.

### Suggested Next
Decide whether to merge the branch stack into main. Merge in order or squash per phase. If desired, tag the v0.4.10 release using the release procedure in `AGENTS.md`.

HANDOFF: full remediation completed - ready for push and merge approval

## 2026-09-13 - v0.4.10 released

- Tag v0.4.10 pushed; Release CI succeeded (run 34755353069) and reused the manual draft with the locally built NSIS installer.
- Release published at https://github.com/TrygerZ/VoxiType/releases/tag/v0.4.10
- Updater channel now serves v0.4.10.
- No code changed post-merge; graphify not required.

