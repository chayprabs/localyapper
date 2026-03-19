# LocalYapper

## What this is
Local-first, open-source, cross-platform voice dictation desktop app.
Privacy-focused alternative to Wispr Flow and SuperWhisper. Fully offline.
Windows 10+ / macOS 12+ / Linux (X11 + Wayland).
License: MIT.

## Stack — exact versions, do not deviate
- **Backend**: Tauri 2, Rust (stable 1.75+), rusqlite 0.31 (bundled), cpal 0.15, whisper-rs 0.11, llama-cpp-rs, tokio 1.x, serde, enigo 0.2
- **Frontend**: React 19, TypeScript 5, Vite 5, Tailwind CSS 3, shadcn/ui, Jotai 2, Recharts 2
- **IPC**: Tauri command system — frontend calls Rust via invoke() from @tauri-apps/api/core

## Architecture
- Two Tauri windows: "main" (settings app, 900×650) and "overlay" (floating pill, 320×80)
- src-tauri/src/ — All Rust backend code
- src/ — React/TypeScript frontend
- All commands use #[tauri::command] and register in generate_handler![] in lib.rs
- SQLite stores all data — 6 tables (transcription_history, corrections, personal_dictionary, modes, app_profiles, settings)
- No cloud processing ever — everything local

## Voice pipeline data flow
hotkey → audio/capture.rs (cpal 16kHz mono + 0.5s pre-roll) → audio/vad.rs (energy filter) → stt/whisper.rs (ggml-tiny.en.bin) → correction/engine.rs (dictionary lookup) → context/detector.rs (focused app) → llm/prompt.rs (mode system prompt) → llm/engine.rs (llama-cpp-rs) → injection/injector.rs (clipboard save → paste → restore) → text appears in app

## Models — FINAL, never change
- STT: ggml-tiny.en.bin bundled in src-tauri/resources/ (~75MB)
- LLM: qwen2.5-0.5b-q4.gguf downloaded to app data dir on first launch (~400MB)
- LLM runtime: llama-cpp-rs crate (no Ollama dependency)
- BYOK alternative: OpenAI / Anthropic / Groq via user API key

## Session limits
- Max recording: 120 seconds
- Warning state at 105 seconds (last 15s = red countdown)
- Overlay countdown timer: max 15s safety cap
- Auto-inject delay: 10 seconds after transcription complete

## Commands
- npm run dev — Vite dev server
- npm run tauri dev — full Tauri dev mode
- npm run build — production build
- cargo test --manifest-path src-tauri/Cargo.toml — Rust tests
- npm run lint — ESLint
- cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings — Rust linter

## Verification — ALWAYS run after changes
1. Frontend: npm run lint && npx tsc --noEmit
2. Backend: cd src-tauri && cargo clippy -- -D warnings
3. Run relevant tests for changed code

## Implementation phases
18 phases total. See docs/PROGRESS.md for current phase.
ALWAYS read docs/PROGRESS.md before starting ANY work.
IMPORTANT: Only work on the CURRENT phase. Never skip ahead.

## Instance coordination
Two Claude Code instances may be active on this repo:
- **Bedrock instance**: Primary coder. Handles all heavy implementation work (Rust AND React). Does the building.
- **Pro instance**: Reviewer and helper. Quick questions, code reviews, small fixes, explanations. Lightweight tasks only.
Both instances can touch any file. If both are running, check with the user before making large changes to avoid conflicts.

## Critical rules — NEVER break these
- NEVER modify main.rs directly — only modify lib.rs
- All commands must register in generate_handler![] macro
- IPC permissions must be in src-tauri/capabilities/
- rusqlite must use bundled feature — never system SQLite
- Text injection = clipboard save → paste simulation → clipboard restore
- No cloud processing ever — everything local
- No unwrap() in production Rust code — use ? operator
- No `any` type in TypeScript — strict mode always
- Windows, macOS, Linux are ALL first-class platforms
- Audio is NEVER written to disk — RAM only during processing
- BYOK API keys stored encrypted, never logged

## Design system
- See DESIGN_SYSTEM.md for all colors, typography, spacing, component specs
- Light mode only
- Apple macOS HIG design language (macOS Ventura System Settings as gold standard)
- Primary accent: #0058bc
- Font: SF Pro / Inter, two weights only: 400 regular, 600 bold

## Current status
Fresh repo. Starting Phase 1 — Foundation.
