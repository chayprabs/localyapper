# Implementation Progress

## CURRENT PHASE: Phase 8 — Overlay UI

### Phase 1: Foundation (COMPLETE)
Goal: App launches, database initializes, no crashes.
- [x] Create Tauri 2 project with exact Cargo.toml dependencies from PRD
- [x] Configure package.json with React 19, Vite 5, Tailwind CSS 3
- [x] Implement db/schema.rs — all 6 tables, all seeds, idempotent migration
- [x] Implement db/queries.rs — typed query functions for all tables
- [x] Implement basic commands stubs for all 34 IPC commands
- [x] Register all commands in generate_handler![]
- [x] Configure two windows in tauri.conf.json (main + overlay)
- [x] Verification: eslint, tsc --noEmit, cargo clippy — all zero errors

### Phase 2: Audio Capture (COMPLETE)
- [x] audio/vad.rs — energy-based silence detection
- [x] audio/capture.rs — cpal 16kHz mono, 0.5s pre-roll ring buffer
- [x] Expose start_recording() and stop_recording() commands

### Phase 3: Speech to Text (COMPLETE)
- [x] stt/whisper.rs — whisper-rs wrapper loading bundled ggml-tiny.en.bin
- [x] Model loaded once at startup, reused for all transcriptions
- [x] Run transcription on blocking thread (whisper-rs is sync)

### Phase 4: Text Injection (COMPLETE)
- [x] injection/platform.rs — OS detection, X11 vs Wayland check
- [x] injection/injector.rs — clipboard save/set/paste/restore flow
- [x] Hold Shift variant for auto-send

### Phase 5: Correction Engine (COMPLETE)
- [x] correction/engine.rs — exact-match substitution from DB
- [x] Sub-5ms performance (pre-load at startup, refresh on change)

### Phase 6: LLM Integration (COMPLETE)
- [x] llm/engine.rs — llama-cpp-2 wrapper (load GGUF, tokenize, decode, sample)
- [x] llm/prompt.rs — ChatML prompt builder with mode system prompt + app context
- [x] context/detector.rs — focused window name per OS (Windows/macOS/Linux)
- [x] LLM wired into stop_recording pipeline (skip if no model or mode.skip_llm)
- [x] context detector wired into get_focused_app command
- [x] download_model — streaming download from HuggingFace with progress events + cancellation
- [x] cancel_model_download — AtomicBool cancellation flag
- [x] check_ollama — HTTP check to localhost:11434 with 2s timeout
- [x] test_byok_connection — OpenAI/Anthropic/Groq API key test with latency

### Phase 7: Full Pipeline Wire-up (COMPLETE)
- [x] Wire: hotkey → capture → VAD → whisper → correction → LLM → inject
- [x] hotkey/manager.rs — hold + release + double-tap
- [x] correction/learner.rs — diff computation, DB writes, confidence calc

### Phase 8: Overlay UI
- [ ] Overlay.tsx — transparent, always-on-top pill
- [ ] All 5 overlay states wired to Tauri events

### Phase 9: Settings Window Shell
- [ ] Main.tsx — sidebar navigation (5 pages)
- [ ] Jotai stores + typed invoke wrappers

### Phase 10: Dashboard Page
- [ ] Stats cards, Ollama status, last dictation

### Phase 11: History Page
- [ ] Card list, pagination, copy, delete, clear all

### Phase 12: Dictionary Pages
- [ ] Corrections tab + Training tab + training flow

### Phase 13: Hotkeys Page
- [ ] Remappable hotkeys with key picker

### Phase 14: Models Page
- [ ] Whisper dropdown + LLM config + BYOK test connection

### Phase 15: First-Launch Wizard
- [ ] All 10 wizard screens + model download flow

### Phase 16: System Tray + Autostart
- [ ] Tray icon, states, menu, autostart

### Phase 17: Cross-Platform Polish
- [ ] Test all features on Windows, macOS, Linux

### Phase 18: GitHub Release
- [ ] CI/CD workflow, binaries for all platforms, README
