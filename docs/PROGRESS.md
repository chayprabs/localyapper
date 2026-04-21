# Implementation Progress

## CURRENT PHASE: Phase 17 - Cross-Platform Polish

## Current Architecture
The live app is now:
- audio capture
- VAD
- Parakeet speech recognition
- correction engine
- text injection

Removed from the current codebase:
- local LLM cleanup
- Ollama integration
- BYOK/API-key cleanup
- processing modes
- app profile routing

## Completed Work

### Phase 1: Foundation (COMPLETE)
- [x] Tauri 2 + React app boots correctly
- [x] SQLite initializes on startup
- [x] Shared command wiring is registered in `lib.rs`

### Phase 2: Audio Capture (COMPLETE)
- [x] `cpal` microphone capture
- [x] 16kHz mono pipeline with pre-roll
- [x] start / stop recording commands

### Phase 3: Speech to Text (COMPLETE)
- [x] Parakeet speech recognition via `sherpa-onnx`
- [x] Hot-loaded speech model reuse through app state
- [x] Blocking inference moved off the async runtime

### Phase 4: Text Injection (COMPLETE)
- [x] Cross-platform injection path
- [x] Clipboard save -> paste -> restore behavior

### Phase 5: Correction Engine (COMPLETE)
- [x] Dictionary-based correction lookup
- [x] Learner refresh after successful dictations

### Phase 6: Overlay UI (COMPLETE)
- [x] Overlay states wired to pipeline events
- [x] Listening / processing / transcribed / no-speech feedback

### Phase 7: Settings UI (COMPLETE)
- [x] Dashboard, History, Dictionary, Hotkeys, Models pages
- [x] Typed frontend command wrappers

### Phase 8: History + Dictionary (COMPLETE)
- [x] History browsing and deletion
- [x] Correction management and training flow

### Phase 9: Hotkeys (COMPLETE)
- [x] Configurable record / cancel / paste-last / open-app shortcuts
- [x] Immediate hotkey reload after updates

### Phase 10: Models Page Simplification (COMPLETE)
- [x] Speech-model-only models page
- [x] Download / delete / load speech model controls
- [x] Removed LLM/Ollama/BYOK settings UI

### Phase 11: First-Launch Wizard Simplification (COMPLETE)
- [x] Wizard now downloads only the speech model
- [x] Removed model-selection, Ollama, BYOK, and warning branches

### Phase 12: Tray + Startup (COMPLETE)
- [x] Tray menu for open / pause / quit
- [x] Startup notifications for speech-model readiness
- [x] Autostart enabled by default

### Phase 13: Cleanup Pass (COMPLETE)
- [x] Removed Rust LLM modules and related dependencies
- [x] Removed Ollama/BYOK/local-model commands
- [x] Removed legacy frontend model flows
- [x] Removed stale mode/app-profile command surface

## Verification
- [x] `npm run lint`
- [x] `npx tsc --noEmit`
- [x] `cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings`

## Next Work
### Phase 18: GitHub Release
- [ ] CI/CD workflow
- [ ] Binaries for all platforms
- [ ] Public README refresh
