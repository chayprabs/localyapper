# Implementation Progress

## CURRENT PHASE: Phase 13 — Hotkeys Page

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

### Phase 8: Overlay UI (COMPLETE)
- [x] Overlay.tsx — transparent, always-on-top pill with 6 visual states
- [x] All states wired to Tauri "pipeline-state" events
- [x] Waveform.tsx — animated 5-bar waveform (blue/red)
- [x] CountdownTimer.tsx — elapsed (processing) + countdown (stopping-soon)
- [x] YappingEmoji.tsx — pulsing speaker emoji
- [x] useOverlayState.ts — event listener, state machine, auto-hide timers
- [x] overlayStore.ts — Jotai atoms for overlay data
- [x] types/overlay.ts — OverlayVisualState, PipelineEvent, OverlayData
- [x] lib/commands/recording.ts — injectText, cancelRecording wrappers
- [x] All 6 overlay states visually verified against Stitch designs

### Phase 9: Settings Window Shell (COMPLETE)
- [x] SettingsLayout.tsx — sidebar + content area shell
- [x] Sidebar.tsx — 240px sidebar with Material Symbols icons, 5 nav items
- [x] Jotai appStore.ts — PageId type + activePageAtom
- [x] 5 page stub components (Dashboard, History, Dictionary, Hotkeys, Models)
- [x] Typed invoke wrappers — all 34 IPC commands covered across 7 files
- [x] shadcn/ui foundation — components.json + cn() utility
- [x] Colors aligned to DESIGN_SYSTEM.md across all components
- [x] Sidebar visually matched to Stitch design

### Phase 10: Dashboard Page (COMPLETE)
- [x] Stats cards (Words Today, Words This Week, Words All Time, Avg WPM, Total Sessions)
- [x] Model/Ollama status indicator (green/red dot + model name)
- [x] Last dictation preview card with copy/delete
- [x] Empty state for first-time users
- [x] Wired to get_stats(), get_history(limit=1), check_ollama() commands
- [x] Matches Stitch designs exactly

### Phase 11: History Page (COMPLETE)
- [x] HistoryPage.tsx — scrollable card list with empty state
- [x] HistoryCard.tsx — timestamp, word count, app badge, copy/delete
- [x] useHistory.ts hook — pagination (20/page), optimistic delete, clear all
- [x] formatHistoryTimestamp() — "Today, 2:34 PM" / "Yesterday" / "Mar 15" format
- [x] Load More button for pagination (hasMore detection)
- [x] Clear All with confirmation dialog
- [x] Empty state with history icon, hotkey hint, Start Dictating button
- [x] Matches Stitch designs exactly

### Phase 12: Dictionary Pages (COMPLETE)
- [x] DictionaryPage.tsx — Tab switching (Corrections/Training), header with Export JSON + Add Correction buttons
- [x] CorrectionsTab.tsx — Table with Whisper Heard/Corrected To/Times Used/Actions columns, pagination footer
- [x] Inline Add Correction form (blue-tinted row, two inputs, Save/Close)
- [x] Corrections empty state with Start Training CTA
- [x] TrainingTab.tsx — Paragraph display (15 paragraphs), Start/Stop Recording, progress, Previous/Next nav
- [x] TrainingComplete.tsx — Green check, corrections learned count, Done button
- [x] useCorrections.ts hook — pagination, add, optimistic delete, count
- [x] training-paragraphs.ts — 15 paragraph constants from docs/training-paragraphs.md
- [x] Backend: get_corrections_count + compute_training_diffs commands (7 total corrections commands)
- [x] Info cards (How it works + Smart Suggestions) below both tabs
- [x] Export JSON copies to clipboard with "Copied!" feedback
- [x] Training paragraph index persisted in settings for cross-session resume
- [x] All 5 Stitch screens matched (Corrections, Add Active, Empty State, Training, Voice Profile Ready)

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
