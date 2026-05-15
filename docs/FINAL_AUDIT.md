# LocalYapper Final Audit

Date: 2026-05-15
Branch: main

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
| Audit whole codebase | Repo map and all active source areas under `src/` and `src-tauri/src/` inspected. | In progress |
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
| Injection failure shows text in overlay | Current failure path emits `error` and hides overlay; text is not preserved. | Release risk |
| Overlay five states | Current states include hidden, listening, stopping-soon, processing, long-recording, transcribed, no-speech. | Implemented with extra states |
| Backend-only overlay timers | Current overlay uses frontend timers for elapsed, warning, processing countdown, and auto-hide. | Product mismatch |
| First-launch onboarding | Current wizard is keyed by `settings.setup_complete`, not an onboarding table. | Implemented differently |
| Main app pages | Current pages: Dashboard, History, Hotkeys, Speech. No Dictionary page. | Current product pass |
| Database tables | Current active tables are `transcription_history` and `settings`; legacy corrections/dictionary tables are dropped. | Current product pass |
| Typed IPC wrappers | Commands have wrappers under `src/lib/commands` and types in `src/types/commands.ts`. | Pass |
| No TypeScript `any` | ESLint rule rejects explicit `any`; search did not find TS `any` usage. | Pass |
| No `unwrap()`/`expect()` in production paths | `lib.rs` still has startup `.expect()` calls. | Fail |
| No dead code suppressions | `lib.rs` has `#![allow(clippy::duplicate_mod, dead_code)]`. | Fail |
| Permission commands | `check_permissions` returns false and settings-open commands are stubs. | Release risk if surfaced |
| Build docs | `docs/BUILD.md` does not exist. | Missing |
| Release notes | `docs/RELEASE_NOTES_v0.1.0.md` does not exist. | Missing |
| Verification | Previous run passed lint, typecheck, clippy, tests, and frontend build. Need rerun after fixes. | Pending |
| Tauri build | Not yet verified in this run. | Pending |

## Findings To Fix Before Release

### P0: Production startup panics remain in `lib.rs`

`src-tauri/src/lib.rs` still uses `.expect()` for app data directory lookup,
database initialization, and final Tauri run. The app should log and return
setup errors instead of crashing without context where Tauri allows it.

### P1: Clippy/dead-code suppressions hide real release quality issues

`src-tauri/src/lib.rs` allows `dead_code` globally. This should be removed if
possible, and any resulting warnings should be fixed or scoped narrowly.

### P1: Injection failures lose the transcribed text

If paste injection fails, the backend emits `error` without preserving the text
for the overlay. For a dictation app, losing the visible transcript after a
successful transcription is a serious user-facing failure mode.

### P1: Build and release documentation are missing

`docs/BUILD.md` and `docs/RELEASE_NOTES_v0.1.0.md` need to be written for the
current speech-only release.

### P2: README source-of-truth section references ignored local docs

The repo now ignores local agent files and `docs/PROGRESS.md`, but `README.md`
still lists them as source-of-truth references. The public README should point
to tracked public docs and live code instead.

### P2: Permission commands are placeholders

`check_permissions`, `open_accessibility_settings`, and `open_mic_settings` are
registered IPC commands but are not implemented. They are not currently exposed
in the simplified UI, so this is not blocking unless the UI starts calling them.

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

