# Rust Backend Context

## Structure
- `src/main.rs` — Tauri entry point (do not edit; use `lib.rs`)
- `src/lib.rs` — command registration and app startup
- `src/commands/` — Tauri command modules
- `src/db/` — SQLite schema and typed queries
- `src/models/` — shared IPC structs
- `src/audio/` — audio capture and VAD
- `src/stt/` — Parakeet speech recognition wrapper
- `src/context/` — focused window helpers
- `src/injection/` — clipboard-based text injection
- `src/hotkey/` — global shortcut registration and state machine
- `src/tray/` — tray icon and menu
- `src/state.rs` — `AppState` managed by Tauri
- `src/error.rs` — custom error types via `thiserror`

## Current backend shape
- Speech recognition only: capture -> VAD -> STT -> injection
- No LLM engine
- No Ollama commands
- No BYOK/API-key commands
- Active persistent tables: `transcription_history`, `settings`

## Coding rules
- All public functions need doc comments
- Use `thiserror` for error types
- Commands return `Result<T, String>` for Tauri IPC compatibility
- Use parameterized `rusqlite` queries only
- Derive `Clone`, `Debug`, `Serialize`, and `Deserialize` on IPC-facing structs
- Run inference on blocking tasks when necessary
- No `unwrap()` in production code
- Every unsafe block needs a `// SAFETY:` comment

## New command pattern
```rust
#[tauri::command]
pub async fn my_command(
    state: tauri::State<'_, AppState>,
    param: String,
) -> Result<MyResponse, String> {
    Ok(response)
}
```

## Database rules
- Migrations live in `src/db/schema.rs`
- Keep queries parameterized with `params![...]`
- Use transactions for multi-step writes
- Do not reintroduce the removed `modes` / `app_profiles` feature set unless explicitly requested

## Cross-platform rules
- Text injection stays clipboard save -> set -> paste -> restore
- Detect X11 vs Wayland at runtime on Linux
- Audio capture remains cross-platform via `cpal`
