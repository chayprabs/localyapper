Scaffold a new Tauri IPC command called $ARGUMENTS. Follow this exact process:

1. Create the Rust handler in src-tauri/src/commands/ (use existing files as pattern)
   - Add #[tauri::command] attribute
   - Use tauri::State<'_, AppState> for state access
   - Return Result<T, String>
   - Add doc comment explaining what it does

2. Create or update the model struct in src-tauri/src/models/ if needed
   - Derive Clone, Debug, Serialize, Deserialize

3. Register in src-tauri/src/lib.rs generate_handler![]

4. Create the TypeScript type in src/types/commands.ts matching the Rust struct

5. Create the typed invoke wrapper in src/lib/commands/

6. Run: cd src-tauri && cargo clippy -- -D warnings
7. Run: npx tsc --noEmit

Report any errors.
