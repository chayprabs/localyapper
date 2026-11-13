# Contributor guide

Read `CLAUDE.md` first for the stack and the rules, and
`docs/ARCHITECTURE.md` for how a dictation flows through the code.

## Where things go

| Change | Place |
|---|---|
| Dictation behaviour (gestures, timeouts, what happens when) | `core/session.rs` (pure state machine; add a unit test) |
| Running an effect the session asks for | `controller.rs` |
| Audio, segmentation, VAD | `core/audio/`, `platform/audio_in.rs` |
| Speech engine and models | `core/stt/`, `src-tauri/models.json` (regenerate with `scripts/gen-model-manifest.py`) |
| OS-specific behaviour | `platform/<area>/<os>.rs` |
| A new setting | `settings/keys.rs` (type, default, validator, hook), a `hook_<key>` test in `settings/mod.rs`, a control in the matching page |
| A new IPC command | `commands/*.rs`, `app_commands!` in `lib.rs`, a typed wrapper in `src/lib/commands/` |
| Settings UI | `src/components/<page>/`, shared controls in `src/components/settings/controls.tsx` |
| Overlay UI | `overlay.html`, `src/overlay-main.ts`, `src/overlay.css` |

## Conventions

- Errors cross IPC as `String`; show them inline next to the control.
- Rust: no `unwrap()`/`expect()` outside tests, `// SAFETY:` on every
  `unsafe` block, `clippy -D warnings` clean on all three OSes.
- TypeScript: strict, no `any`.
- Colours come from the theme tokens (`black/xx`, `surface`, `canvas`,
  `sidebar`) so both themes work.
- Hot paths read settings from `AppState.settings`, never SQLite.
- Anything that touches GTK or window geometry runs on the main thread.

## Tests

- Unit tests: `cargo test --manifest-path src-tauri/Cargo.toml`.
- Pipeline with a real model: `tests/pipeline.rs` (needs
  `LOCALYAPPER_TEST_MODEL_DIR`).
- Smoke suite: `tests/smoke.rs` drives the real app through the
  `test-hooks` control channel and a text-target window on each OS. New
  user-visible behaviour should get a smoke test.
- CI must stay green on `ubuntu-22.04`, `windows-latest` and `macos-15`.
  Never skip or weaken a test to get there; on runners that cannot do
  something (macOS synthetic keys without a grant), tests print `SKIP:`
  with the reason.

## Releases

Bump the version in `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json`
and `package.json`, add `docs/RELEASE_NOTES_v<version>.md`, and push a
`v<version>` tag. See `docs/BUILD.md`.
