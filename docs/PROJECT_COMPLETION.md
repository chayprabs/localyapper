# LocalYapper Project Completion Report

Date: 2026-05-30  
Branch: `main`  
Scope: Five-agent full-stack audit + remediation pass

---

## Executive summary

LocalYapper is a **local-first, STT-only** voice dictation desktop app (Tauri 2 + React 19 + Parakeet via sherpa-onnx). There is **no LLM cleanup**, no cloud STT, and no Ollama integration — by design.

Five parallel audits covered the Rust backend, frontend UI/responsiveness, IPC layer, UX flows, and build/deployment readiness. Automated verification (lint, TypeScript, Clippy, 36 Rust tests, production build) all pass after this remediation pass.

---

## What was fixed in this pass

### Critical backend

| Fix | Files |
|-----|-------|
| Capture focused app **before** overlay shows; use for history | `hotkey/manager.rs`, `state.rs` |
| Pause during recording no longer traps session (abort + cancel) | `tray/mod.rs`, `hotkey/manager.rs` |
| Hotkey state machine persists across reloads (shared `HotkeyState`) | `state.rs`, `hotkey/manager.rs` |
| STT engine reloads when `speech_model` setting changes | `commands/models.rs`, `state.rs` |
| Idle eviction scheduled on pipeline error / no-speech paths | `hotkey/manager.rs`, `commands/recording.rs` |
| Clipboard restored if paste keystroke fails mid-injection | `injection/injector.rs` |
| `reload_models` always re-validates engine from disk | `commands/models.rs` |

### Frontend UX / UI

| Fix | Files |
|-----|-------|
| Minimum window size 560×480 (Tauri + runtime builder) | `tauri.conf.json`, `lib.rs` |
| Responsive dashboard stat grids | `DashboardPage.tsx` |
| Responsive history card layout | `HistoryCard.tsx`, `HistoryPage.tsx` |
| Wizard card scales on narrow windows | `WizardChrome.tsx` |
| Hotkey selector chevron alignment | `HotkeysPage.tsx` |
| Hands-free overlay state (double-tap) | `Overlay.tsx`, `useOverlayState.ts`, `overlay.ts` |
| Model loading hint after idle eviction | `useOverlayState.ts`, `Overlay.tsx` |
| Dashboard empty state only when stats loaded | `DashboardPage.tsx` |
| History delete confirmation + pagination offset fix | `HistoryCard.tsx`, `useHistory.ts` |
| Idle-eviction copy no longer says "restart required" | `ModelsPage.tsx` |
| Paused dictation shows tray-menu hint on hotkey press | `hotkey/manager.rs` |

---

## Verified working (no code change needed)

- Hold-to-talk + double-tap hands-free state machine
- 105s warning / 120s hard cap
- 30s pipeline safety timeout
- Clipboard save → paste → restore injection
- History saved before injection (paste-failure recovery)
- Lazy model load + idle eviction with generation counter
- SQLite schema, hotkey rollback, download resume
- IPC: 27/27 command names aligned frontend ↔ Rust
- CI verify gates: lint, tsc, clippy, tests, build

---

## Intentionally out of scope (not bugs)

| Item | Reason |
|------|--------|
| LLM / Ollama / BYOK cleanup | Removed deliberately; app is STT-only |
| Dictionary / Training / Corrections | Removed deliberately |
| Whisper | Replaced by Parakeet via sherpa-onnx |

---

## Remaining known gaps (future work)

### Deployment blockers (need credentials / manual QA)

- **Code signing** — macOS notarization and Windows Authenticode not wired in CI
- **Manual mic → STT → inject QA** — 7 ignored desktop smoke tests; full `docs/MANUAL_QA.md` pass recommended before public release
- **Release notes drift** — `docs/RELEASE_NOTES_v0.1.0.md` still mentions `Ctrl+F8` hands-free; code uses double-tap of record key

### Medium (non-blocking)

- Audio capture assumes f32 sample format (no I16 fallback) — affects some hardware
- Linux injection does not verify `xdotool`/`wtype` exit status
- Silero VAD download failure can still report success at 100%
- macOS accessibility permission not surfaced in onboarding wizard
- Wizard "Skip setup" bypasses model download and hotkey persistence
- Frontend has no automated test suite
- Hotkey state machine has no unit tests (650+ lines)
- `npm run build` uses `tsc -b` which only checks Vite config project

### Minor polish

- Overlay pill dimensions differ slightly from `DESIGN_SYSTEM.md` (320px vs 280px spec)
- Sidebar active tint uses iOS blue vs brand primary
- Stats "words today" uses SQLite UTC date (midnight edge cases)
- Dead IPC wrappers in `src/lib/commands/recording.ts` (hotkey path is canonical)

---

## Verification commands (all passing)

```bash
npm run lint && npx tsc --noEmit
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml
npm run build
```

---

## Architecture reference

```
hotkey → audio/capture → audio/vad → stt (Parakeet) → injection (clipboard paste)
         ↑ overlay (pipeline-state events)              ↑ history (SQLite)
```

Two windows: `main` (settings, destroyed on close) + `overlay` (always alive). Models lazy-load on first dictation; evicted after `idle_unload_seconds` (default 60s, toggle on Speech page).

---

## Agent audit coverage

| Agent | Focus | Key findings addressed |
|-------|-------|------------------------|
| 1 | Rust backend | Focus capture, model cache, pause trap, eviction |
| 2 | Frontend UI | Responsive grids, min window, hotkey layout |
| 3 | IPC layer | model-state listener wired |
| 4 | UX flows | Hands-free UI, confirms, dashboard/history fixes |
| 5 | Build/deploy | Documented blockers; no false "LLM missing" flags |

---

## Sign-off status

**Development complete for v0.1.0 RC feature set.** The app builds, tests pass, and critical runtime bugs from the audit are fixed. Public distribution still requires signing credentials and platform manual QA per `docs/MANUAL_QA.md`.
