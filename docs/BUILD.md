# Building LocalYapper

## Prerequisites

- Rust stable and Node.js 20 or newer.
- The Tauri 2 system packages for your OS:
  https://v2.tauri.app/start/prerequisites/
- Linux: `libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev
  libssl-dev libasound2-dev libxdo-dev`.
- Python 3 for the helper scripts in `scripts/`.

The `sherpa-onnx` crate downloads its prebuilt static library during the
first build (from the k2-fsa GitHub releases). Where that host is not
reachable, build with shared libraries instead:

```bash
export SHERPA_ONNX_LIB_DIR=/path/to/sherpa-onnx/lib
cargo build --manifest-path src-tauri/Cargo.toml --no-default-features --features sherpa-shared
```

## Develop

```bash
npm ci
npm run tauri dev
```

The app keeps its data in the platform data folder
(`%APPDATA%\com.localyapper.desktop`, `~/Library/Application Support/com.localyapper.desktop`,
`~/.local/share/com.localyapper.desktop`). Set `LOCALYAPPER_DEBUG=1` for
debug logging.

## Check

These run in CI (`.github/workflows/ci.yml`) on every push:

```bash
npm run lint
npx tsc --noEmit
npm run build
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml
```

Tests that need a speech model are ignored by default. Fetch the models
(checks every SHA-256 against `src-tauri/models.json`), then run them:

```bash
python3 scripts/fetch-models.py src-tauri/models.json ~/models parakeet-110m
LOCALYAPPER_TEST_MODEL_DIR=~/models/parakeet-110m \
  cargo test --manifest-path src-tauri/Cargo.toml --test pipeline -- --ignored
```

### Smoke suite

`src-tauri/tests/smoke.rs` starts the real app with the `test-hooks`
feature, feeds it a recording instead of the microphone, presses the
hotkey through a local control channel and checks what lands in a test
window (`tests/helpers/text_target`): text, focus, clipboard restore,
overlay behaviour, single instance, window lifetimes.

```bash
npm run build
LOCALYAPPER_TEST_MODEL_DIR=~/models/parakeet-110m \
  cargo test --manifest-path src-tauri/Cargo.toml \
  --features test-hooks,tauri/custom-protocol --test smoke -- --test-threads=1
```

On Linux run it under Xvfb and a D-Bus session:
`dbus-run-session -- xvfb-run -a cargo test ...`.

`test-hooks` is never enabled in release builds.

### Measurements

- `docs/MODEL_BENCHMARK.md`: `cargo run --release --example bench` per
  model and thread count, written by `.github/workflows/bench.yml`.
- `docs/PERFORMANCE.md`: `scripts/measure-memory.sh` /
  `scripts/measure-memory.ps1`, written by `.github/workflows/perf.yml`.

## Package

```bash
python3 scripts/gen-notices.py
npm run tauri build
```

Bundles (`src-tauri/tauri.conf.json`): NSIS per-user installer on
Windows, app and DMG on macOS (one per architecture), deb, rpm and
AppImage on Linux. All of them carry `resources/silero_vad.onnx` and
`THIRD_PARTY_NOTICES.md`; speech models are downloaded on first run.

## Release

`.github/workflows/release.yml`:

- On a branch, changing packaging inputs builds every installer as a
  workflow artifact and fails if the Windows installer is over 25 MB.
- A `v*` tag runs the full CI first, then builds the installers and
  attaches them to a published release whose notes are
  `docs/RELEASE_NOTES_<tag>.md`.

Builds are unsigned; the README explains the first-launch steps.
