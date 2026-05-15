# LocalYapper Final Audit

Date: 2026-05-15
Branch: main

Reference code-verification run: `25931329886`
Verified code commit: `001ff6c`
Result: success for verify plus Windows, Linux, macOS Intel, and macOS Apple
Silicon build jobs.
Latest local source verification commit: `38b53fe`
Latest repository hygiene checkpoint: `aa8eb8a`

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
| 120 second cap | `MAX_RECORDING_SAMPLES` and elapsed guard cap recording at 120 seconds. | Implemented |
| Warning at 105 seconds | Frontend overlay timer switches to `stopping-soon`; backend does not emit this state. | Partial |
| Clipboard save/paste/restore | `injection/injector.rs` implements save -> paste -> restore. | Implemented |
| Injection failure shows text in overlay | Backend error events include the transcript after paste failure; the overlay keeps it visible and copyable with a paste-failed indicator. | Pass |
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
| npm production audit | `npm audit --omit=dev` reports 0 vulnerabilities after non-breaking transitive lockfile fixes. | Pass |
| Frontend dependency surface | Unused `lucide-react` and `class-variance-authority` dependencies were removed; the required Recharts 2 stack dependency was left in place. | Pass |
| Speech model setting normalization | Removed legacy Whisper directory mapping from active model resolution; old DB values still migrate/fall back to the current Parakeet default. | Pass |
| User-visible errors | Dashboard, History, Hotkeys, Speech, and Wizard setup actions now surface operation failures inline instead of only logging to DevTools. | Pass |
| Inert UI controls | History empty-state action now navigates to Hotkeys instead of being a no-op. | Pass |
| Build docs | `docs/BUILD.md` added for Windows, macOS, and Linux build paths. | Done |
| Release notes | `docs/RELEASE_NOTES_v0.1.0.md` added for the current release candidate. | Done |
| Verification | lint, typecheck, fmt check, clippy, tests, frontend build, and targeted post-cleanup source gates passed after fixes. | Pass |
| Tauri dev launch | Hidden smoke test reached Vite, compiled Rust, started `target/debug/localyapper.exe`, loaded STT/VAD, registered hotkeys, and initialized tray. | Pass |
| Tauri build | Windows NSIS bundle and Linux AppImage bundle passed locally. | Pass |
| CI/CD workflow | `.github/workflows/release.yml` verified and built Windows NSIS, Linux DEB/AppImage, macOS Intel DMG, and macOS Apple Silicon DMG artifacts. | Pass |
| CI queue behavior | Branch workflows now cancel older in-progress runs for the same ref; tag release runs are preserved. | Pass |
| Hotkey registration failures | Backend hotkey updates reject empty/duplicate values, report OS registration failures, and restore previous settings if reload fails. | Pass |
| Git ignore policy | `.gitignore` excludes local agent files, ignored progress/PRD/source docs, cloud state, secrets, databases, models, build output, and release artifacts; no tracked file currently matches ignore rules. | Pass |

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
overlay keeps it visible and copyable with a paste-failed indicator.

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

### P2: Overlay timer authority is split

The backend emits high-level pipeline state events, while the frontend owns
elapsed timers, warning transition, processing estimates, and auto-hide. This
matches the current code but not the older prompt. A backend-only overlay event
model would be a larger refactor and is not required for the current
speech-only release unless product direction changes.

## Next Implementation Steps

1. Remove production `.expect()` calls from `lib.rs`.
2. Remove global dead-code suppressions and fix any fallout.
3. Preserve transcribed text on injection failure so the overlay can still show
   the user something copyable.
4. Update public docs for the current product direction.
5. Add build and release notes docs.
6. Rerun lint, typecheck, clippy, tests, frontend build, and Tauri build.
7. Remove stale placeholder IPC/update-check surfaces.
8. Remove production-only overlay debug logging.
9. Remove stale auto-inject setting and clarify overlay dismiss progress.
10. Route backend diagnostics through the logger without logging transcript text.
11. Remove unused Tauri FS/Shell plugin surface.
12. Apply non-breaking npm audit fixes for production dependencies.
13. Prune stale DB settings and honor `auto_start` at startup.
14. Add release-workflow concurrency for frequent branch checkpoints.
15. Remove unused frontend dependencies while keeping the declared Recharts 2 stack dependency.
16. Normalize stale speech model settings to the current Parakeet default.
17. Make page and wizard operation failures visible inline.
18. Wire the History empty-state action to real navigation.
19. Report hotkey registration failures and roll back failed updates.
20. Harden `.gitignore` for local-only release, cloud, model, secret, and agent artifacts.

Completed in this run: items 1 through 20. Windows NSIS and Linux AppImage
bundling were verified locally. GitHub Actions run `25931329886` completed
successfully for verify plus Windows, Linux, macOS Intel, and macOS Apple
Silicon build jobs, and uploaded all four platform artifacts. Later source
hardening through `38b53fe` passed the relevant local frontend and Rust gates;
`aa8eb8a` is an ignore-policy-only checkpoint.

Remaining manual validation gap: a real microphone recording injected into an
external target application still needs hands-on end-to-end QA on a desktop
session. Automated and CI checks cover build, typing, linting, Rust tests, and
packaging, but they do not prove the real OS microphone and target-app injection
path by themselves.
