<p align="center">
  <img src="src/assets/logo-nobg.png" alt="LocalYapper" width="128" height="128" />
</p>

<h1 align="center">LocalYapper</h1>

<p align="center">
  <strong>Fully offline desktop voice dictation.</strong><br/>
  Open-source alternative to Wispr Flow and SuperWhisper. No cloud. No subscription. No audio leaves your machine.
</p>

---

## Overview

LocalYapper is a local-first desktop dictation app for Windows, macOS, and Linux.
Press a global hotkey, speak, and the app will:

- capture microphone audio locally
- remove silence with VAD
- transcribe speech with a local speech model
- apply learned corrections from your dictionary
- paste the result into the focused app

The current app is speech-only by design. There is no Ollama integration, no BYOK providers, and no local LLM cleanup step in the pipeline.

## Current Features

- Fully offline dictation
- First-launch wizard that downloads the speech model and sets your hotkey
- Floating overlay with listening, processing, transcribed, and no-speech states
- Global hotkeys for record, cancel, paste last, and open app
- History page for previous dictations
- Dictionary page for learned corrections and manual correction entries
- Voice training flow that helps build up correction data
- Models page for downloading, deleting, and reloading the local speech model
- Tray app with open, pause dictation, and quit actions
- Local SQLite storage for settings, history, and corrections
- Clipboard-based text injection across platforms

## Current Pipeline

`hotkey -> audio capture -> VAD -> local speech recognition -> correction engine -> text injection`

Notes:

- Audio stays in memory only
- Silero VAD is used when available, with an energy-based fallback
- The default speech model is Parakeet 110M, downloaded on first launch
- The current download size is about 458 MB
- Correction learning improves substitutions over time from accepted dictations

## What Is Not In The App Anymore

- Ollama integration
- local LLM cleanup
- API-key cleanup providers
- processing modes
- app profile routing

## First Launch

On first launch, LocalYapper opens a setup wizard that:

1. downloads the local speech model
2. lets you confirm your hotkey
3. marks setup complete and enables normal dictation flow

If the speech model is not present yet, the app will prompt you to open Settings and download it.

## Default Hotkeys

- Record: `Ctrl+Shift+Space`
- Cancel: `Escape`
- Paste last dictation: `Alt+Shift+V`
- Open app: `Alt+L`

These can be changed in the Hotkeys page.

## Current Status

LocalYapper is currently in active development.

- Current phase: `Phase 17 - Cross-Platform Polish`
- Core dictation flow is implemented and working
- Public release packaging and CI/CD are still upcoming

## Stack

| Layer | Tech |
|:------|:-----|
| Desktop shell | Tauri 2 |
| Backend | Rust |
| Frontend | React 19 + TypeScript 5 + Vite 5 |
| Audio capture | cpal |
| VAD | Silero VAD + energy fallback |
| Speech recognition | sherpa-onnx + Parakeet |
| Storage | rusqlite (bundled SQLite) |
| Injection | enigo + clipboard flow |

## Repo Layout

```text
src-tauri/
  src/
    audio/
    commands/
    context/
    correction/
    db/
    hotkey/
    injection/
    models/
    stt/
    tray/
    lib.rs
    state.rs

src/
  components/
  hooks/
  lib/
  stores/
  types/
```

## Development

Install dependencies:

```bash
npm install
```

Run the frontend only:

```bash
npm run dev
```

Run the full Tauri app:

```bash
npm run tauri dev
```

Build the frontend bundle:

```bash
npm run build
```

## Verification

Run these after changes:

```bash
npm run lint
npx tsc --noEmit
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml
```

## Source Of Truth

Some older planning docs in the repo still describe an earlier LLM/Ollama architecture. For the current app, use these as the main references:

- `AGENTS.md`
- `CLAUDE.md`
- `docs/PROGRESS.md`
- the live code in `src/` and `src-tauri/`

## License

MIT
