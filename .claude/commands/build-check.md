Run a full build verification and report results grouped by frontend vs backend:

1. Frontend checks:
   - `npx tsc --noEmit` (TypeScript type check)
   - `npm run lint` (ESLint)

2. Backend checks:
   - `cd src-tauri && cargo clippy -- -D warnings` (Rust linting)
   - `cd src-tauri && cargo test` (Rust tests)

3. Full build check:
   - `npm run tauri build --debug`

Report each result as PASS or FAIL with the error output if failed.
