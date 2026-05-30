<p align="center">
  <img src="src/assets/logo-nobg.png" alt="LocalYapper" width="128" height="128" />
</p>

<h1 align="center">LocalYapper</h1>

<p align="center">
  <strong>Fully offline voice dictation for your desktop.</strong><br/>
  A privacy-first, open-source alternative to Wispr Flow and SuperWhisper.<br/>
  No cloud. No subscription. No audio ever leaves your machine.
</p>

<p align="center">
  <em>Press a key. Speak. Your words appear in whatever app you were using.</em>
</p>

---

## How it works

```
hotkey -> microphone capture -> VAD -> Parakeet speech model -> clipboard paste
                                       (everything stays on this machine)
```

1. You press (or hold, or double-tap) the global record hotkey.
2. LocalYapper captures 16 kHz mono audio from your default microphone into RAM.
3. Silero VAD trims silence; if the VAD model is unavailable an energy-based fallback is used.
4. Parakeet (via `sherpa-onnx`) transcribes the audio to text.
5. The text is briefly placed on the clipboard, pasted into the window you were just using, and the original clipboard contents are restored.

Audio is **never written to disk**. There is no telemetry, no cloud STT, and no LLM cleanup stage — the recognised text is the final text.

## Features

- **Offline-only dictation.** Parakeet 110M runs fully on-device.
- **Three recording gestures from a single hotkey:**
  - **Hold** the record key — release to send.
  - **Double-tap** the record key (within ~350 ms) to enter hands-free mode; tap once more to stop.
  - Each session has a 120-second hard cap, with a red 15-second countdown starting at 105 s.
- **Floating overlay** that shows the live state: listening, processing, transcribed, no-speech, stopping-soon.
- **First-run wizard with granular resume.** Welcome → Microphone permission → Hotkey → Speech files → Done. The active step is persisted so quitting mid-setup picks up where you left off.
- **Customisable hotkeys** for record, cancel, paste-last, and open-app, with a soft warning when you bind a combo the OS commonly reserves.
- **System tray** with Open, Pause Dictation, Quit. The main-window title bar shows a *Paused* chip whenever dictation is paused from the tray.
- **Local dictation history** with copy, delete per entry, and clear-all.
- **Local SQLite storage** (`rusqlite`, bundled). Two active tables: `transcription_history` and `settings`. Nothing else.
- **Low idle footprint by design.** The recognizer and VAD are lazy-loaded on first dictation, the settings WebView is freed when its window is closed, and the speech engine is dropped from RAM after ~60 s of inactivity. Idle memory drops from ~500 MB to ~30 MB; CPU sits near 0 % when you're not dictating.
- **Cross-platform.** Windows 10+, macOS 12+, Linux on X11 and Wayland.

## Default hotkeys

| Action       | Default       | Notes                                                |
|--------------|---------------|------------------------------------------------------|
| Record       | `F8`          | Hold to dictate; double-tap to toggle hands-free     |
| Cancel       | `Escape`      | Only registered while a recording is active          |
| Paste last   | `Ctrl+Alt+J`  | Re-injects the most recent transcription             |
| Open app     | `Ctrl+Alt+O`  | Brings the LocalYapper main window to focus          |

All four are rebindable from the **Hotkeys** page.

## App pages

- **Dashboard** — today / week / all-time stats, the last dictation, model status (name + on-disk size), a *speech files missing* banner if the model isn't installed, and a welcoming empty state when you haven't dictated anything yet.
- **History** — paginated list with copy and delete per entry, plus clear-all.
- **Hotkeys** — rebind any shortcut via key-listening mode and reset to defaults. Soft warning if your binding overlaps a known OS-reserved combo.
- **Speech** — manage the local Parakeet model: download, load, or remove. Includes a **Memory & Performance** toggle to keep the engine resident at all times if you'd rather trade ~30 MB of idle RAM for instant first-dictation latency.

There is intentionally no Dictionary page, no Training tab, no remote-model picker, and no LLM settings.

## Privacy

- Audio is captured into RAM and discarded as soon as the pipeline finishes.
- The only outbound network calls are the one-time downloads of the Parakeet speech model and the Silero VAD model. After that, the app is fully offline.
- No analytics, no crash reporting, no remote logging, no autoupdate ping.
- All settings and history live in a single SQLite file in your platform's app-data directory.

## Resource footprint

LocalYapper is built to disappear when you're not using it.

| State                                            | RAM       | CPU        |
|--------------------------------------------------|-----------|------------|
| Cold start, before first dictation               | ~30 MB    | ~0 %       |
| Actively dictating (capture + Parakeet + paste)  | ~400 MB   | ~5–10 %    |
| Idle ≥ 60 s after a dictation                    | ~30 MB    | ~0 %       |
| Settings window closed                           | ~30 MB    | ~0 %       |

How this is achieved:

- **Lazy model loading.** Parakeet and Silero VAD are not loaded at startup. The first dictation pulls them into memory; subsequent dictations reuse the warm engine.
- **Idle eviction.** A lifecycle controller drops the recognizer ~60 s after the last dictation finishes. The next hotkey press reloads it (~0.8–1.5 s extra) and emits a `model-state: loading` event so the overlay can show a subtle hint. The window is configurable in **Speech → Memory & Performance** — toggle it off to stay always-resident.
- **Two separate WebView bundles.** The settings UI and the floating overlay each have their own Vite entry, so the overlay WebView only loads the ~10 kB it actually needs instead of the full settings module graph.
- **Settings WebView is freed on close.** Closing the main window destroys its WebView; reopening from the tray or `Ctrl+Alt+O` rebuilds it on demand. The overlay WebView stays alive so the hotkey response stays instant.
- **`mimalloc` global allocator.** Returns memory to the OS more aggressively than the system allocator does after `sherpa-onnx` runs, especially on Windows.
- **Trimmed dependencies and a tuned release profile.** `tokio`, `chrono`, and `reqwest` are stripped to the features actually used; `recharts` and Material Symbols (a ~3.9 MB icon font) were replaced with tree-shakable `lucide-react`; the release build runs with `lto = "fat"`, `codegen-units = 1`, and `strip = "symbols"`.

## Stack

| Layer               | Tech                                                |
|---------------------|-----------------------------------------------------|
| Desktop shell       | Tauri 2 (two windows: main + overlay, each with its own JS bundle) |
| Backend             | Rust (stable 1.95+), `mimalloc` global allocator    |
| Frontend            | React 19 + TypeScript 5 + Vite 5 + Tailwind CSS 3   |
| State               | Jotai 2                                             |
| Icons               | `lucide-react` (tree-shakable SVGs)                 |
| Audio capture       | `cpal` 0.15 at 16 kHz mono                          |
| VAD                 | Silero VAD, with an energy-based fallback           |
| Speech recognition  | `sherpa-onnx` 1.12 + Parakeet 110M (default)        |
| Storage             | `rusqlite` 0.31 (bundled SQLite)                    |
| Text injection      | `enigo` 0.2 + clipboard save → set → paste → restore|

On Linux the injector dispatches between X11 (`xclip` + `xdotool`) and Wayland (`wl-clipboard` + `wtype`) automatically.

## Repo layout

```
src-tauri/
  src/
    audio/        cpal capture, Silero VAD, energy fallback
    commands/     #[tauri::command] IPC handlers
    context/      focused-window detection
    db/           rusqlite schema + queries
    hotkey/       global shortcuts + state machine (hold / tap / double-tap)
    injection/    clipboard-paste-restore injector + per-platform helpers
    models/       data types shared with the frontend
    stt/          Parakeet wrapper around sherpa-onnx
      lifecycle.rs idle-eviction controller (mark_used / schedule_evict)
    tray/         system tray menu + paused-state event
    lib.rs        entrypoint, setup(), generate_handler!, mimalloc allocator
    state.rs      Tauri-managed AppState
index.html        main settings-window entry  (-> src/main.tsx)
overlay.html      overlay-window entry        (-> src/overlay-main.tsx)
src/
  components/     dashboard, history, hotkeys, models, overlay, wizard, settings, ui
                  ui/Icon.tsx + ui/Switch.tsx are the shared primitives
  hooks/          useWizard, useHotkeys, useDashboard, usePausedState, useModels, ...
  lib/            typed wrappers around invoke() in lib/commands/*
  stores/         Jotai atoms
  types/          shared TypeScript types
docs/             BUILD, MANUAL_QA, FINAL_AUDIT, PRODUCT_AUDIT, RELEASE_NOTES
```

## Development

Install dependencies:

```bash
npm install
```

Run the frontend only (no native window):

```bash
npm run dev
```

Run the full Tauri app:

```bash
npm run tauri dev
```

Build the frontend bundle (also runs `tsc -b`):

```bash
npm run build
```

Build a real installer for your platform:

```bash
npm run tauri build
```

See `docs/BUILD.md` for platform-specific prerequisites (notably the Linux apt packages and macOS code-signing flow).

## Verification

These are the same checks GitHub Actions runs on every push to `main`. Run them locally before pushing:

```bash
npm audit --omit=dev
npm run lint
npx tsc --noEmit
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml
npm run build
```

Keep your toolchain on the latest stable Rust (`rustup update stable`). CI does, and the clippy lint set tightens between releases.

Before tagging a release, also run the manual desktop QA checklist in `docs/MANUAL_QA.md` (real microphone capture, real external-app injection on each OS).

## Release status

LocalYapper is currently a **v0.1.0 release candidate**.

- The core pipeline (capture → VAD → Parakeet → injection) is implemented and shipping.
- Onboarding, hotkeys, history, dashboard, and the system tray are feature-complete for v0.1.0.
- The GitHub Actions `Release` workflow builds cross-platform installers — Windows NSIS, macOS Apple Silicon + Intel DMG, and Linux DEB + AppImage — on every push to `main`.
- See `docs/RELEASE_NOTES_v0.1.0.md` for the candidate notes.

## What is intentionally NOT in the app

These were considered, prototyped, or previously present, and have been deliberately removed. Please don't reintroduce them in a PR without opening an issue first:

- Cloud or BYOK STT providers of any kind.
- A local LLM cleanup stage (e.g. `mistral.rs`, `llama-cpp-rs`, Candle).
- Whisper / `whisper-rs` (the app uses Parakeet via `sherpa-onnx`).
- Ollama integration, processing modes, or app-profile routing.
- Correction engine, confidence-decay learning, personal dictionary, training paragraphs, the Dictionary page, the Training tab.
- An auto-inject-delay setting beyond the fixed transcribed-overlay window.

## Documentation

- `AGENTS.md` and `CLAUDE.md` — guidance for human contributors and AI agents working in this repo.
- `DESIGN_SYSTEM.md` — colours, typography, spacing, component specs.
- `docs/BUILD.md` — per-platform build instructions.
- `docs/MANUAL_QA.md` — pre-release manual QA checklist.
- `docs/FINAL_AUDIT.md` — release-hardening audit.
- `docs/PRODUCT_AUDIT.md` — current spec-vs-code reconciliation.
- `docs/RELEASE_NOTES_v1.0.0.md` — current release notes.

## License

MIT — see `LICENSE`.
