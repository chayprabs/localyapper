# LocalYapper Final Audit

Date: 2026-05-16
Branch: main

Reference code-bearing verification checkpoint: `9b59c39`
Reference verification run: `25969254251`
Result: success for Verify plus Windows, Linux, macOS Intel, and macOS Apple
Silicon build jobs.
Artifacts: `localyapper-windows-x64`, `localyapper-linux-x64`,
`localyapper-macos-x64`, and `localyapper-macos-aarch64`.
Earlier reference code-bearing verification runs include `25965215093` for
checkpoint `8e5bdae`, `25963631719` for checkpoint `5a5eb24`, `25962865906`
for checkpoint `b545f5f`, `25962071164` for checkpoint `229a669`,
`25961358640` for checkpoint `c7be818` via empty retry checkpoint `322df92`,
`25960350839` for checkpoint `ea0ba6d`, `25959625219` for checkpoint
`1025593`, `25959323267` for checkpoint `747fae5`, `25956466390` for
checkpoint `ecd543a`, and `25938753998` for checkpoint `6c5e82d`.
Ignore-policy checkpoint: `aa8eb8a`.

## Source-Of-Truth Decision

The release objective contains requirements from an older product direction: Whisper
via `whisper-rs`, local LLM cleanup via `mistralrs`, Ollama, BYOK providers,
dictionary training, correction learning, and six database tables.

The current repo has already pivoted. The current app is speech-recognition-only:

- `README.md` says there is no dictionary training flow, no Ollama integration,
  no BYOK providers, and no local LLM cleanup step.
- `docs/PROGRESS.md` says Phase 17 is complete and the live architecture is
  audio capture, VAD, Parakeet speech recognition, and text injection.
- `src-tauri/CLAUDE.md` says the active backend shape is capture -> VAD -> STT
  -> injection, with no LLM, Ollama, or BYOK commands.
- The live code uses `sherpa-onnx` and Parakeet, not `whisper-rs`.

Therefore the release path should continue the current speech-only app unless
the product direction is explicitly changed again. Reintroducing correction
learning, dictionary training, Ollama, BYOK, or local LLM cleanup would be a
large product reversal, not a release hardening fix.

If a local LLM cleanup path is reintroduced later, the better backend direction
is `mistral.rs`/Candle rather than `llama-cpp-rs`. Current upstream evidence:

- `mistral.rs` describes itself as a fast, flexible LLM inference engine with a
  Rust crate and support for Hugging Face models and GGUF.
  Source: https://github.com/EricLBuehler/mistral.rs
- Candle's stated goal is lightweight Rust inference without Python production
  overhead. Source: https://github.com/huggingface/candle
- `llama_cpp-rs` is a binding over llama.cpp's C API.
  Source: https://github.com/edgenai/llama_cpp-rs
- `whisper-rs` is a binding over whisper.cpp.
  Source: https://github.com/tazz4843/whisper-rs

That supports avoiding `llama-cpp-rs` in a future mixed speech plus LLM runtime,
but it does not justify adding LLM features back into this release.

## Prompt-To-Artifact Checklist

| Requirement | Current Evidence | Status |
|---|---|---|
| Read PRD | `docs/PRD.md` is not present locally or tracked in Git. | Missing source doc |
| Read progress/design/Claude docs | `docs/PROGRESS.md`, `DESIGN_SYSTEM.md`, `CLAUDE.md`, `src-tauri/CLAUDE.md`, and `src/CLAUDE.md` were read from local files. | Done |
| Audit whole codebase | Repo map, active source under `src/` and `src-tauri/src/`, CI config, packaging config, public docs, and tracked release artifacts were inspected. | Done for current product surface |
| Write final audit | This file. | Done |
| Do not use llama-cpp-rs | `Cargo.toml` has no llama dependency. | Pass |
| Use mistralrs | Current app intentionally has no LLM backend. | Not applicable to current product |
| Whisper via ggml-base.en.bin | Current app uses `sherpa-onnx` Parakeet models. | Intentionally superseded |
| Correction engine and dictionary training | Removed from code and docs as current product direction. | Intentionally superseded |
| Hotkey hold-to-record | `hotkey/manager.rs` handles press/release. | Implemented |
| Hands-free mode | Implemented as `Ctrl+F8` toggle, not double-tap same hotkey. | Product mismatch |
| 16 kHz mono PCM | `audio/capture.rs` resamples to 16 kHz mono. | Implemented |
| 120 second cap | `MAX_RECORDING_SAMPLES` caps the buffer and `hotkey/manager.rs` auto-stops active hotkey sessions at 120 seconds. | Implemented |
| Warning at 105 seconds | `hotkey/manager.rs` emits `stopping-soon` at 105 seconds and the overlay starts the final countdown from the backend event duration. | Implemented |
| Clipboard save/paste/restore | `injection/injector.rs` implements save -> paste -> restore. | Implemented |
| Injection failure shows text in overlay | Backend error events include the transcript after paste failure; the overlay keeps it visible and copyable with a paste-failed indicator. | Pass |
| Paste-failure history refresh | Dashboard and History refresh when a paste-failure event includes saved transcript text, not only on successful injection. | Pass |
| Pipeline errors visible | Overlay shows a visible error state for pipeline failures before transcript text is available, such as missing model files or STT load failures. | Pass |
| Overlay five states | Current states include hidden, listening, stopping-soon, processing, long-recording, transcribed, no-speech. | Implemented with extra states |
| Backend-only overlay timers | Current overlay uses frontend timers for elapsed, warning, processing countdown, and auto-hide. | Product mismatch |
| First-launch onboarding | Current wizard is keyed by `settings.setup_complete`, not an onboarding table. | Implemented differently |
| Main app pages | Current pages: Dashboard, History, Hotkeys, Speech. No Dictionary page. | Current product pass |
| Database tables | Current active tables are `transcription_history` and `settings`; legacy corrections/dictionary tables are dropped. | Current product pass |
| Typed IPC wrappers | Commands have wrappers under `src/lib/commands` and types in `src/types/commands.ts`; the unused placeholder update-check command was removed. | Pass |
| No TypeScript `any` | ESLint rule rejects explicit `any`; search did not find TS `any` usage. | Pass |
| No `unwrap()`/`expect()` in production paths | Startup `.expect()` calls were replaced with logged error handling. Search is clean for `unwrap(` and `expect(` in `src-tauri/src`. | Pass |
| No dead code suppressions | Global dead-code suppression was removed and clippy passes with `-D warnings`. | Pass |
| Permission commands | `check_permissions` uses microphone/input capability checks and settings-open commands route to OS settings where supported. | Pass |
| Production logging | Backend diagnostics use the logger; direct stdout logging and transcript-content log previews were removed. | Pass |
| Tauri plugin surface | Unused frontend FS/Shell permissions and unused FS/Shell Rust plugins were removed; Windows NSIS build still passes. | Pass |
| npm production audit | `npm audit --omit=dev` reports 0 vulnerabilities after non-breaking transitive lockfile fixes and is enforced by the Verify CI job. | Pass |
| Frontend dependency surface | Unused `lucide-react` and `class-variance-authority` dependencies were removed; the required Recharts 2 stack dependency was left in place. | Pass |
| Speech model setting normalization | Removed legacy Whisper directory mapping from active model resolution; old DB values still migrate/fall back to the current Parakeet default. | Pass |
| User-visible errors | Dashboard, History, Hotkeys, Speech, and Wizard setup actions now surface operation failures inline instead of only logging to DevTools. | Pass |
| Post-download recovery UI | Speech model download flows distinguish file download failure from successful download followed by engine-start failure. | Pass |
| Inert UI controls | History empty-state action now navigates to Hotkeys instead of being a no-op. | Pass |
| History and dashboard data queries | In-memory query tests verify reverse-chronological history paging, stats aggregation, delete-not-found errors, and clear-all behavior. | Pass |
| Build docs | `docs/BUILD.md` added for Windows, macOS, and Linux build paths. | Done |
| Release notes | `docs/RELEASE_NOTES_v0.1.0.md` added for the current release candidate. | Done |
| Verification | lint, production audit, typecheck, fmt check, all-targets clippy, tests, frontend build, dev-launch smoke, synthetic speech STT smoke, and GitHub Actions passed after fixes. | Pass |
| Tauri dev launch | Hidden smoke test reached Vite, compiled Rust, started `target/debug/localyapper.exe`, loaded STT/VAD, registered hotkeys, and initialized tray. | Pass |
| Tauri build | Windows NSIS bundle and Linux AppImage bundle passed locally. | Pass |
| CI/CD workflow | `.github/workflows/release.yml` run `25969254251` enforced production audit and all-targets clippy, then built Windows NSIS, Linux DEB/AppImage, macOS Intel DMG, and macOS Apple Silicon DMG artifacts for code-bearing checkpoint `9b59c39`. | Pass |
| CI queue behavior | Branch workflows now cancel older in-progress runs for the same ref; tag release runs are preserved. | Pass |
| Model download recovery | Speech model downloads validate completed temp files and replace stale incomplete destination files before rename, avoiding Windows overwrite failures. Startup and model status now report installed only when both ONNX and tokens files are valid. | Pass |
| Hotkey registration failures | Backend hotkey updates reject empty/duplicate values, report OS registration failures, and restore previous settings if reload fails. | Pass |
| Fresh database setup defaults | In-memory schema tests verify `setup_complete=false`, current hotkey defaults, current speech model default, removed legacy tables/settings cleanup, and stale speech model normalization. | Pass |
| Git ignore policy | `.gitignore` excludes local agent files, ignored progress/PRD/source docs, cloud state, secrets, databases, models, build output, and release artifacts; no tracked file currently matches ignore rules. | Pass |
| Manual desktop QA | `docs/MANUAL_QA.md` defines the remaining real microphone, overlay, hotkey, model, and external-app injection validation steps. | Checklist ready; not yet executed |
| Windows external-app injection smoke | Ignored test `manual_windows_notepad_injection_smoke` launches Notepad, uses the real injector, verifies saved pasted text, and checks clipboard restoration. | Passed in the interactive Windows desktop session; still not a substitute for full spoken dictation QA |
| Microphone transcription smoke | Ignored test `manual_microphone_transcription_smoke` records from the selected or default microphone after a configurable countdown, can optionally wait for Enter, can optionally require expected transcript words, can optionally play a Windows speech prompt, prints all input devices plus RMS/peak diagnostics, runs VAD, loads the installed speech model, and requires a non-empty transcript. | Added; requires human speaker or speaker-to-mic setup to execute |
| Windows synthetic speech STT smoke | Ignored test `manual_windows_tts_file_transcription_smoke` generates a Windows SAPI WAV, runs VAD, loads the installed speech model, and requires a non-empty transcript. | Passed locally on Windows |
| Windows generated speech to Notepad smoke | Ignored test `manual_windows_tts_to_notepad_pipeline_smoke` generates Windows SAPI speech, runs VAD and STT, injects the transcript into Notepad with the real injector, saves the file, and verifies clipboard restoration. | Passed locally on Windows; still not a substitute for real microphone QA |
| Windows microphone to Notepad smoke | Ignored test `manual_windows_microphone_to_notepad_pipeline_smoke` records from the selected or default microphone, can optionally play a tunable Windows speech prompt during recording, runs VAD and STT, validates optional expected words, injects the transcript into Notepad with the real injector, saves the file, and verifies clipboard restoration. | Added; requires human speaker or speaker-to-mic setup to execute |
| Windows external textbox injection smoke | Ignored test `manual_windows_textbox_injection_smoke` launches a separate WinForms textbox process, uses the real injector, verifies the pasted text through that process's output file, and checks clipboard restoration. | Passed locally on Windows |
| Windows microphone to textbox smoke | Ignored test `manual_windows_microphone_to_textbox_pipeline_smoke` records from the selected or default microphone, can optionally play a tunable Windows speech prompt during recording, runs VAD and STT, validates optional expected words, injects the transcript into a separate WinForms textbox process, verifies the pasted text through that process's output file, and checks clipboard restoration. | Added; needs stable human speaker or speaker-to-mic setup to execute |

## Findings To Fix Before Release

### P0: Production startup panics remain in `lib.rs` - Fixed

`src-tauri/src/lib.rs` still uses `.expect()` for app data directory lookup,
database initialization, and final Tauri run. The app should log and return
setup errors instead of crashing without context where Tauri allows it.

Remediation: startup setup errors are now logged and returned through Tauri
setup, final Tauri runtime errors are logged, and the production source search
is clean for `unwrap(` and `expect(`.

### P1: Clippy/dead-code suppressions hide real release quality issues - Fixed

`src-tauri/src/lib.rs` allows `dead_code` globally. This should be removed if
possible, and any resulting warnings should be fixed or scoped narrowly.

Remediation: the global allow was removed, unused items were cleaned up or
scoped to tests, and clippy passes with `-D warnings`.

### P1: Injection failures lose the transcribed text - Fixed

If paste injection fails, the backend emits `error` without preserving the text
for the overlay. For a dictation app, losing the visible transcript after a
successful transcription is a serious user-facing failure mode.

Remediation: injection failure events now include the transcript, and the
overlay keeps it visible and copyable with a paste-failed indicator. Dashboard
and History also refresh on paste-failure events with transcript text because
the history entry is saved before injection is attempted.

### P1: Pipeline errors without text were hidden - Fixed

If the pipeline failed before producing text, for example because model files
were missing or STT failed to load, the overlay transitioned to hidden with an
error value. The user could return from a recording with no visible indication
of what went wrong.

Remediation: the overlay now has a visible `error` state for failures without
transcript text. It shows the backend error for several seconds, while
paste-failure errors with transcript text continue to use the copyable
transcribed state.

### P1: Build and release documentation are missing - Fixed

`docs/BUILD.md` and `docs/RELEASE_NOTES_v0.1.0.md` need to be written for the
current speech-only release.

Remediation: both documents were added for the current speech-only release
candidate.

### P2: README source-of-truth section references ignored local docs - Fixed

The repo now ignores local agent files and `docs/PROGRESS.md`, but `README.md`
still lists them as source-of-truth references. The public README should point
to tracked public docs and live code instead.

Remediation: README now points to `README.md`, `DESIGN_SYSTEM.md`,
`docs/FINAL_AUDIT.md`, and the live source tree.

### P2: Permission commands are placeholders - Fixed

`check_permissions`, `open_accessibility_settings`, and `open_mic_settings` were
registered IPC commands but returned hardcoded placeholder values.

Remediation: permission checks now report microphone availability and
accessibility/injection readiness using platform-specific checks, and the
settings commands open OS settings panels where supported.

### P2: Placeholder update check command is registered - Fixed

`check_update` was registered as an IPC command and exposed through a frontend
wrapper, but it always returned `None` and had no UI caller. Shipping that
surface would imply update-check behavior that does not exist.

Remediation: the unused command, frontend wrapper, and handler registration were
removed.

### P2: Overlay debug logging shipped in production hook - Fixed

The overlay hook logged pipeline events and show calls to the browser console.
Those logs were useful during development but noisy for a release candidate.

Remediation: development `console.log` calls were removed while keeping
`console.error` failure reporting.

### P2: Backend writes production diagnostics directly to stdout - Fixed

Release-path backend code used `println!` for hotkey, VAD, STT, pipeline, and
startup diagnostics. One STT log also included transcript content, which is not
appropriate for a privacy-first dictation app.

Remediation: backend diagnostics now go through the configured logger, and
transcription logs report only sample and character counts.

### P2: Unused Tauri FS/Shell plugin surface - Fixed

The frontend did not use Tauri FS or Shell APIs, but the app still shipped
their plugin permissions and Rust/JS dependencies.

Remediation: unused FS/Shell permissions, plugins, and dependencies were
removed. A Windows NSIS Tauri build passed after the removal.

### P2: Stale settings were seeded but never read - Fixed

Older settings such as sound effects, media muting, language, max recording
seconds, and auto-inject delay remained in the database seed data without
active product behavior.

Remediation: stale defaults are no longer seeded and are cleaned from existing
databases. The remaining `auto_start` setting now controls autostart enable and
disable behavior at startup.

### P2: Production npm audit had a high transitive finding - Fixed

`npm audit --omit=dev` reported a high-severity transitive `lodash` finding via
Recharts.

Remediation: non-breaking transitive lockfile fixes updated `lodash` and other
patched packages. `npm audit --omit=dev` now reports 0 vulnerabilities. The
remaining non-production audit advisory is Vite/esbuild dev-server-only and npm
requires a forced Vite 8 upgrade to clear it; this repo remains on the required
Vite 5 line.

### P2: Frequent checkpoint pushes create redundant release workflow runs - Fixed

The working cadence uses frequent commits and pushes, which started multiple
full cross-platform release workflows for intermediate commits.

Remediation: the release workflow now uses branch-level concurrency to cancel
older in-progress runs for the same ref. Tag release runs are not cancelled.

### P2: Unused frontend dependencies were shipped - Fixed

`lucide-react` and `class-variance-authority` were present in the production
dependency manifest but not imported by the current UI.

Remediation: both unused dependencies were removed from `package.json` and
`package-lock.json`. The current UI still uses Material Symbols, and Recharts 2
was left in place because it is part of the declared frontend stack.

### P2: Legacy Whisper settings leaked into current model resolution - Fixed

Old Whisper model values were still mapped to `whisper-*` directories in active
STT path helpers, even though the current app uses Parakeet via `sherpa-onnx`.

Remediation: active model selection now normalizes unsupported or removed model
settings to the current Parakeet default. Database migration still maps old
`whisper_model` values to `speech_model = parakeet-110m`.

### P2: Several UI errors were console-only - Fixed

Hotkey update/reset/load failures, History load/delete/clear failures,
Dashboard load/delete failures, Speech settings/delete failures, and Wizard
finish/skip failures were not consistently visible to the user.

Remediation: those flows now show compact inline error messages in the relevant
page or wizard step while preserving existing logging for diagnostics.

### P2: Speech model download success could be mislabeled as download failure - Fixed

The first-launch wizard and Speech settings page awaited model reload directly
after file download. If file download succeeded but engine startup failed, the
UI could label the whole operation as "Download failed" instead of showing that
the files were present but the engine needed attention.

Remediation: both flows now distinguish download errors from post-download
engine-start errors. Settings still refreshes file/status state after download,
and the wizard advances after a successful download while preserving the engine
attention message for setup completion.

### P2: History empty-state action was inert - Fixed

The History empty-state button said "Start Dictating" but had no handler.

Remediation: the action now navigates to the Hotkeys page, where the user can
see or change the shortcut that starts dictation.

### P2: Hotkey registration failures could leave stale settings - Fixed

Hotkey update and reset commands previously wrote settings before reloading OS
hotkeys. If registration failed at the OS layer, the user could be left with a
saved shortcut that was not actually active.

Remediation: hotkey updates now reject empty values and case-insensitive
duplicates, return registration errors to the UI, restore the previous settings
on reload failure, and attempt to reload the previous known-good hotkeys.
Regression tests cover uniqueness, current-setting reads, and restore behavior.

### P2: Ignore rules missed common local release artifacts - Fixed

The repo already ignored major build folders and local agent files, but the
ignore policy did not cover several common leak points such as cloud provider
state, signing keys, package-manager caches, model resource directories, temp
files, and packaged installers.

Remediation: `.gitignore` now excludes those categories while keeping tracked
source, docs, package locks, icons, and workflow files visible. A tracked-file
check confirmed no existing Git-tracked file is being hidden by the ignore
rules.

### P2: Incomplete speech model files could block Windows re-downloads - Fixed

If a corrupt or partial speech model file already existed at the final
destination, the downloader wrote a fresh `.download` file and then attempted to
rename it over the destination. On Windows, that replacement can fail when the
destination file already exists.

Remediation: completed temp files are now validated for a minimum expected
size, stale incomplete destination files are removed before rename, and
non-file destination paths fail with a clear error. Startup model resolution and
the model status command now share the same minimum-file checks before reporting
the speech model as installed, so the UI and preloader no longer treat partial
ONNX-only directories as ready.

### P2: Overlay timer authority is split

The backend now emits the 105-second `stopping-soon` event and auto-stops active
hotkey sessions at the 120-second cap. The frontend still owns display timers,
processing estimates, and auto-hide. This matches the current product behavior
but not the older prompt's backend-only timer model. Moving every display timer
behind backend events would be a larger refactor and is not required for the
current speech-only release unless product direction changes.

## Completed Release Hardening

1. Removed production `.expect()` calls from `lib.rs`.
2. Removed global dead-code suppressions and fixed the fallout.
3. Preserved transcribed text on injection failure so the overlay can still show
   the user something copyable.
4. Updated public docs for the current product direction.
5. Added build and release notes docs.
6. Reran lint, typecheck, clippy, tests, frontend build, and Tauri build.
7. Removed stale placeholder IPC/update-check surfaces.
8. Removed production-only overlay debug logging.
9. Removed stale auto-inject setting and clarified overlay dismiss progress.
10. Routed backend diagnostics through the logger without logging transcript text.
11. Removed unused Tauri FS/Shell plugin surface.
12. Applied non-breaking npm audit fixes for production dependencies.
13. Pruned stale DB settings and honored `auto_start` at startup.
14. Added release-workflow concurrency for frequent branch checkpoints.
15. Removed unused frontend dependencies while keeping the declared Recharts 2 stack dependency.
16. Normalized stale speech model settings to the current Parakeet default.
17. Made page and wizard operation failures visible inline.
18. Wired the History empty-state action to real navigation.
19. Reported hotkey registration failures and rolled back failed updates.
20. Hardened `.gitignore` for local-only release, cloud, model, secret, and agent artifacts.
21. Added a manual desktop QA checklist for real microphone and external-app injection validation.
22. Added and ran an ignored Windows Notepad injection smoke test for clipboard save -> paste -> restore.
23. Added an ignored microphone transcription smoke test for default mic -> VAD -> STT validation.
24. Added optional Windows TTS prompting and RMS/peak diagnostics to the microphone transcription smoke.
25. Added and ran an ignored Windows synthetic speech-file smoke test for VAD -> STT validation.
26. Added a backend recording watchdog that emits the 105-second warning state
    and auto-stops/processes active hotkey sessions at the 120-second cap.
27. Hardened speech model downloads against incomplete temp files and stale
    corrupt destination files.
28. Hardened speech model installed-status checks so partial model directories
    without valid ONNX and token files are reported as not installed.
29. Reused the same speech model completeness checks for startup preload,
    on-demand load resolution, model status, and model deletion decisions.
30. Split speech model download errors from post-download engine-start errors
    in Settings and first-launch wizard flows.
31. Refreshed Dashboard and History after paste-failure events that still save
    a completed transcript.
32. Added a visible overlay error state for pipeline failures that occur before
    transcript text is available.
33. Added wait-for-Enter support, optional input-device selection, optional
    expected-word validation, and input device, capture-format, RMS, and peak
    diagnostics to the ignored microphone transcription smoke test.
34. Added schema tests for fresh setup defaults, removed legacy state cleanup,
    and stale speech model normalization.
35. Added history/query tests for paging, dashboard statistics, missing-entry
    deletion errors, and clear-all behavior.
36. Added and ran a Windows generated speech to Notepad smoke that chains generated
    speech, VAD, local STT, clipboard injection, file save, and clipboard
    restoration in one interactive check.
37. Added a Windows microphone to Notepad smoke that chains real microphone
    capture, VAD, local STT, expected-word validation, clipboard injection,
    file save, and clipboard restoration in one interactive check.
38. Added and ran a Windows external textbox injection smoke that validates
    clipboard save -> paste -> restore against a separate GUI process when
    Notepad focus is blocked.
39. Added a Windows microphone to textbox smoke that chains real microphone
    capture, VAD, local STT, expected-word validation, clipboard injection,
    external-process text verification, and clipboard restoration.

Windows NSIS and Linux AppImage bundling were verified locally earlier in the
release run. GitHub Actions run `25969254251` for code-bearing checkpoint
`9b59c39` completed successfully for Verify plus Windows, Linux, macOS Intel,
and macOS Apple Silicon build jobs, and uploaded all four platform artifacts.
That Verify job includes `npm audit --omit=dev` and
`cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`.

The most recent local source gates at audit time passed with:

- `npm audit --omit=dev`
- `npm run lint`
- `npx tsc --noEmit`
- `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`
- `cargo test --manifest-path src-tauri/Cargo.toml`

The most recent local Rust test run passed 35 tests with 7 ignored manual desktop
smokes. A controlled `npm run tauri dev` smoke reached Vite on port 1420,
compiled Rust, launched `target\debug\localyapper.exe`, and began loading the
Parakeet speech model before the test process was stopped. The ignored Windows
synthetic speech-file smoke also passed and produced a
non-empty transcript through VAD plus STT.

Remaining manual validation gap: a real microphone recording injected into an
external target application still needs hands-on end-to-end QA on a desktop
session using `docs/MANUAL_QA.md`. The ignored Windows Notepad smoke test
validated the external-app clipboard save -> paste -> restore path in this
interactive desktop session. A local microphone smoke captured audio from the
USB condenser mic and completed VAD plus STT, but the transcript was only a tiny
fragment and was not accepted as release-completing evidence. The real mic plus
STT path still needs a hands-on spoken pass using
`manual_windows_microphone_to_notepad_pipeline_smoke` with
`LOCALYAPPER_MIC_SMOKE_EXPECTED_WORDS`, or the full manual dictation checklist.
Strict physical speaker-to-microphone attempts using the Windows speech prompt
failed on both available inputs because captured audio was too quiet for VAD:
the built-in microphone array reached RMS `0.000237` / peak `0.013403`, and
the USB condenser microphone reached RMS `0.000868` / peak `0.039918`.
Windows output volume was already `100%` and unmuted during follow-up testing,
so this was not caused by low system volume. Longer repeated prompts through
the USB condenser reached STT with RMS up to `0.005834` / peak `0.117552`, but
expected-word validation still failed because transcripts were distorted, for
example `Hello world visits a microbone test`.
After adding prompt rate and voice controls, a physical speaker-to-microphone
run with `Microsoft Zira Desktop` at rate `-4` produced transcripts containing
the expected words `world` and `microphone`, but the combined Notepad smoke
still did not pass because this desktop session could not reliably focus
Notepad for Enigo paste injection. The standalone Notepad injection smoke also
failed to focus Notepad in the same session, so this is recorded as an
interactive desktop focus limitation rather than release-completing evidence.
The separate external textbox injection smoke passed in the same session,
verifying clipboard save -> paste -> restore against a focused GUI process. A
microphone-to-textbox run produced a valid transcript containing expected words
but did not complete injection before the target lost focus; subsequent retries
failed earlier because the physical speaker-to-microphone path became too quiet
or returned an empty transcript.
The synthetic Windows speech-file smoke validates VAD plus STT against
generated spoken audio, and the generated speech to Notepad smoke validates the
same generated-speech transcript through the real Notepad injector and passed
locally, but neither validates OS microphone capture.
