# Rust Backend Context

## Structure
- src/main.rs — Tauri entry point (DO NOT EDIT — only lib.rs)
- src/lib.rs — Command handler registration (generate_handler![])
- src/commands/ — Tauri command modules (one file per domain)
- src/db/ — SQLite layer (schema.rs for migrations, queries.rs for typed queries)
- src/models/ — Data structs (Serialize, Deserialize, Clone, Debug)
- src/audio/ — capture.rs (cpal), vad.rs (energy-based silence detection)
- src/stt/ — whisper.rs (whisper-rs wrapper)
- src/llm/ — engine.rs (llama-cpp-rs), prompt.rs (system prompt builder)
- src/correction/ — engine.rs (dictionary lookup), learner.rs (diff + confidence)
- src/context/ — detector.rs (focused window name per OS)
- src/injection/ — injector.rs (clipboard flow), platform.rs (OS detection)
- src/hotkey/ — manager.rs (global shortcut: hold, release, double-tap)
- src/tray/ — manager.rs (system tray icon + menu)
- src/state.rs — AppState managed by Tauri (Arc<Mutex<>> for models)
- src/error.rs — Custom error types via thiserror

## Coding rules
- All public functions need doc comments
- Use thiserror for error types, not bare String
- Commands return Result<T, String> for Tauri IPC compatibility
- rusqlite with parameterized queries — NEVER string interpolation
- Derive Clone, Debug, Serialize, Deserialize on all IPC-facing structs
- Group related commands in module files (one per domain)
- whisper-rs: load model ONCE in AppState at startup, share via Arc<Mutex<>>
- llama-cpp-rs: same pattern — model in AppState
- ML inference runs on blocking tokio tasks (spawn_blocking), never block IPC
- No unwrap() — use ? operator or explicit error handling
- Every unsafe block must have // SAFETY: comment
- #![forbid(unsafe_code)] on crates that don't need unsafe

## New command pattern
```rust
#[tauri::command]
pub async fn my_command(
    state: tauri::State<'_, AppState>,
    param: String,
) -> Result<MyResponse, String> {
    // implementation using state.inner()
    Ok(response)
}
```
Register in lib.rs: `.invoke_handler(tauri::generate_handler![commands::domain::my_command])`

## Database rules
- Migrations in src/db/schema.rs, run sequentially on app start
- NEVER modify existing migrations — always create new ones
- All queries use parameterized statements: `params![value]`
- Transactions for multi-step writes
- 6 tables: transcription_history, corrections, personal_dictionary, modes, app_profiles, settings

## Cross-platform rules
- Text injection: enigo for all platforms, with OS-specific fallbacks
- macOS: Cmd+V, Windows: Ctrl+V, Linux X11: xclip+xdotool, Wayland: wl-clipboard+wtype
- Clipboard operations MUST be atomic: save → set → paste → wait 80ms → restore
- Detect X11 vs Wayland at runtime on Linux
- Audio: cpal handles cross-platform automatically
