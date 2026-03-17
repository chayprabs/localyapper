# Implementation Progress

## CURRENT PHASE: Phase 2 — Audio Capture

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

### Phase 2: Audio Capture
- [ ] audio/vad.rs — energy-based silence detection
- [ ] audio/capture.rs — cpal 16kHz mono, 0.5s pre-roll ring buffer
- [ ] Expose start_recording() and stop_recording() commands

### Phase 3: Speech to Text
- [ ] stt/whisper.rs — whisper-rs wrapper loading bundled ggml-tiny.en.bin
- [ ] Model loaded once at startup, reused for all transcriptions
- [ ] Run transcription on blocking thread (whisper-rs is sync)

### Phase 4: Text Injection
- [ ] injection/platform.rs — OS detection, X11 vs Wayland check
- [ ] injection/injector.rs — clipboard save/set/paste/restore flow
- [ ] Hold Shift variant for auto-send

### Phase 5: Correction Engine
- [ ] correction/engine.rs — exact-match substitution from DB
- [ ] Sub-5ms performance (pre-load at startup, refresh on change)

### Phase 6: LLM Integration
- [ ] llm/engine.rs — llama-cpp-rs wrapper
- [ ] llm/prompt.rs — system prompt builder with mode + app context
- [ ] context/detector.rs — focused window name per OS

### Phase 7: Full Pipeline Wire-up
- [ ] Wire: hotkey → capture → VAD → whisper → correction → LLM → inject
- [ ] hotkey/manager.rs — hold + release + double-tap
- [ ] correction/learner.rs — diff computation, DB writes, confidence calc

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
