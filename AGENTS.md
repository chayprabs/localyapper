# LocalYapper

## What this is

Local-first, open-source, cross-platform voice dictation desktop app.
Privacy-focused alternative to Wispr Flow and SuperWhisper. Fully offline.
Windows 10+, macOS 12+, Linux (X11 + Wayland).
License: MIT.

## Stack — exact versions, do not deviate

- **Backend**: Tauri 2, Rust (stable 1.75+), `rusqlite` 0.31 (bundled),
  `cpal` 0.15, `sherpa-onnx` 1.12, `tokio` 1.x, `serde`, `enigo` 0.2
- **Frontend**: React 19, TypeScript 5, Vite 5, Tailwind CSS 3, shadcn/ui
  (used minimally), Jotai 2, Recharts 2, Material Symbols (font-based icons)
- **IPC**: Tauri command system. Frontend calls Rust via `invoke()` from
  `@tauri-apps/api/core` through typed wrappers in `src/lib/commands/`.

## Architecture

- Two Tauri windows: `main` (settings app) and `overlay` (floating pill).
- `src-tauri/src/` — all Rust backend code.
- `src/` — React/TypeScript frontend.
- All commands use `#[tauri::command]` and register in
  `tauri::generate_handler![]` in `lib.rs`.
- SQLite stores app data in **two** active tables: `transcription_history`
  and `settings`. Schema lives in `src-tauri/src/db/schema.rs`.
- Nothing leaves the machine. There is no telemetry, no cloud STT, no remote
  LLM cleanup.

## Voice pipeline data flow

`hotkey -> audio/capture.rs -> audio/vad.rs -> stt/whisper.rs -> injection/injector.rs -> text appears in app`

Notes:

- Audio is captured at 16 kHz mono with `cpal` and stays in memory only.
- VAD uses Silero VAD when its model is available, with an energy fallback
  when it is not.
- STT is Parakeet via `sherpa-onnx`. There is no Whisper or whisper-rs.
- Injection is clipboard save -> set -> paste -> restore. The previously
  focused window is captured before the overlay shows.
- There is no correction pass, no learning loop, and no LLM cleanup stage.

## Models

- STT: Parakeet via `sherpa-onnx`, downloaded to the app data directory on
  first launch. Default model is `parakeet-110m`.
- VAD: Silero VAD model, downloaded alongside the speech model.
- No local LLM. No Ollama. No BYOK / API-key cleanup providers.

## Session limits

- Hard cap on a single recording: 120 seconds. Backend stops and processes the
 active session at the cap.
- Warning state at 105 seconds. The overlay turns red and counts down 15 to 0.
- Pipeline safety timeout: 30 seconds.
- Transcribed overlay stays visible for ~3 seconds, then auto-dismisses.

## Idle model eviction

Parakeet (`sherpa-onnx::OfflineRecognizer`) and Silero VAD pin hundreds of
megabytes when loaded. To keep idle RAM low we:

- Lazy-load both on first dictation (no preload at startup).
- Drop them from `AppState` after `idle_unload_seconds` of inactivity.
 Default 60 seconds; override via the setting key of the same name.
- Cancel any pending eviction the moment a new dictation starts via a
 generation counter in `stt::lifecycle::ModelLifecycle`.
- Emit `model-state` events (`loaded` / `loading` / `unloaded`) so the
 overlay can show a brief loading hint on the first dictation after
 eviction. Frontend can also call `get_model_state` to hydrate on mount.

## Window lifecycle

- The main settings window is destroyed (not hidden) on close. Tray and
 the open-app hotkey both call `show_or_create_main_window()` to rebuild
 it on demand. Reopening from tray costs ~250-400ms.
- The overlay window stays alive at all times so the dictation hotkey
 has zero perceived latency. Its bundle is loaded from `overlay.html`
 (separate Vite entry); it does not pull the settings or wizard graph.
- mimalloc is the global allocator. Without it, sherpa-onnx's alloc/free
 pattern fragments badly on Windows and partially undoes the eviction.

## Default hotkeys

- Record: `F8` (hold to dictate)
- Hands-free: double-tap of the record hotkey (toggle on/off)
- Cancel: `Escape` (only registered while recording)
- Paste last dictation: `Ctrl+Alt+J`
- Open app: `Ctrl+Alt+O`

These are user-configurable from the Hotkeys page.

## Main app pages

- Dashboard — stats, last dictation card, model status
- History — paginated list with copy/delete per entry, clear all
- Hotkeys — rebind via key-listening mode, reset to defaults
- Speech — local Parakeet model: download, load, remove

There is no Dictionary page, no Training page, no Models picker for remote
providers, and no LLM settings.

## Onboarding

- Wizard appears on first launch only. Its steps are persisted granularly so
  the user resumes where they left off if they quit the app.
- Steps: Welcome -> Microphone permission -> Hotkey -> Speech files -> Done
- Once `setup_complete` is `true`, the wizard never reappears.

## Commands

- `npm run dev` — Vite dev server only
- `npm run tauri dev` — full Tauri dev mode
- `npm run build` — production build
- `cargo test --manifest-path src-tauri/Cargo.toml` — Rust tests
- `npm run lint` — ESLint
- `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`

## Verification — ALWAYS run after changes

1. `npm run lint && npx tsc --noEmit`
2. `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`
3. `cargo test --manifest-path src-tauri/Cargo.toml`

## Critical rules — NEVER break these

- NEVER modify `src-tauri/src/main.rs` directly. Edit `src-tauri/src/lib.rs`.
- All commands must register in `tauri::generate_handler![]`.
- IPC permissions live in `src-tauri/capabilities/`.
- `rusqlite` must use the bundled feature. Never the system SQLite.
- Text injection stays clipboard save -> set -> paste -> restore.
- Audio must never be written to disk. RAM only during processing.
- No `unwrap()` or `expect()` on production paths in Rust. Use `?` or
  explicit handling.
- No `any` in TypeScript. Strict mode always.
- Windows, macOS, Linux are first-class platforms.
- Light mode only. Apple-style design language. Primary accent `#0058bc`.

## Do not reintroduce — direction notes

These features were deliberately removed. Do not re-add them unless the user
explicitly requests it again:

- Ollama integration / Ollama as an STT or cleanup provider.
- BYOK / API-key providers of any kind.
- Local LLM cleanup stage (e.g. via `mistral.rs`, `llama-cpp-rs`, Candle).
- Whisper / `whisper-rs`. The app uses Parakeet via `sherpa-onnx`.
- Correction engine, confidence-decay learning, personal dictionary, and
  training paragraphs.
- Dictionary page or Training tab in the UI.
- Processing modes or app profile routing.
- Auto-inject delay setting beyond the fixed transcribed overlay window.

## Design system

- See `DESIGN_SYSTEM.md` for colors, typography, spacing, and component specs.
- Light mode only.
- Apple macOS HIG design language.
- Primary accent: `#0058bc`.
- Font: SF Pro / Inter, two weights only — 400 regular, 600 bold.

## Source of truth

- `README.md` — public overview.
- `AGENTS.md` and `CLAUDE.md` — agent and contributor guidance.
- `DESIGN_SYSTEM.md` — design tokens and component specs.
- `docs/FINAL_AUDIT.md` — release-hardening audit.
- `docs/PRODUCT_AUDIT.md` — current spec-vs-code reconciliation and gap plan.
- `docs/MANUAL_QA.md` — manual desktop QA checklist.
- `docs/BUILD.md` — build instructions per platform.
- `docs/RELEASE_NOTES_v0.1.0.md` — release notes for the current candidate.

## Current status

v0.1.0 release candidate. The app is speech-recognition-only by design.
Phase B (onboarding refresh), Phase C (double-tap hands-free), and Phase D-F
of `docs/PRODUCT_AUDIT.md` are the remaining tracked work before declaring
the release feature-complete.
