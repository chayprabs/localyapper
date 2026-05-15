# LocalYapper Build Guide

This guide describes the build and release workflow for the current v0.1.0
speech-only LocalYapper app.

Official Tauri references:

- Prerequisites: https://v2.tauri.app/start/prerequisites/
- Distribution: https://v2.tauri.app/distribute/

## Repository Setup

Install JavaScript dependencies:

```bash
npm install
```

Verify the toolchains:

```bash
node -v
npm -v
rustc -V
cargo -V
```

## Development

Run the frontend only:

```bash
npm run dev
```

Run the full desktop app:

```bash
npm run tauri dev
```

The Tauri dev server uses Vite on port `1420`.

## Required Verification

Run these before tagging or publishing a release:

```bash
npm run lint
npx tsc --noEmit
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml
npm run build
npm run tauri build
```

## GitHub Actions Release Workflow

The repository includes `.github/workflows/release.yml`.

On pushes and pull requests to `main`, the workflow:

- installs the Linux system dependencies required by Tauri,
- runs frontend lint and TypeScript checks,
- runs Rust formatting, clippy, and tests,
- builds the frontend,
- builds Tauri artifacts on Windows, macOS Apple Silicon, macOS Intel, and Linux.

On tags matching `v*`, the workflow also creates a draft GitHub Release through
`tauri-apps/tauri-action` and uploads the built platform artifacts.

Release tag format:

```bash
git tag v0.1.0
git push origin v0.1.0
```

## Windows Build

Required:

- Windows 10 or newer for the supported app target.
- Microsoft C++ Build Tools with `Desktop development with C++`.
- Microsoft Edge WebView2 Runtime. It is already included on most Windows 10
  version 1803+ installations.
- Rust stable MSVC toolchain.
- Node.js LTS or newer.

Recommended Rust setup:

```powershell
rustup default stable-msvc
```

Build:

```powershell
npm install
npm run tauri build -- --bundles nsis
```

The repo sets static CRT linking for `x86_64-pc-windows-msvc` in
`src-tauri/.cargo/config.toml` because `sherpa-onnx-sys` must use the same CRT
as the rest of the app.

If MSI packaging fails with a `light.exe` error, confirm the Windows VBSCRIPT
optional feature is enabled.

## macOS Build

Required:

- macOS 12 or newer for the supported app target.
- Xcode Command Line Tools for desktop builds:

```bash
xcode-select --install
```

Build:

```bash
npm install
npm run tauri build -- --bundles dmg
```

For public distribution outside the App Store, Apple code signing and
notarization are required. Unsigned local builds are useful for internal QA but
are not suitable for broad end-user distribution.

## Linux Build

Required:

- Rust stable.
- Node.js LTS or newer.
- Tauri Linux system dependencies, including WebKitGTK, OpenSSL, appindicator,
  librsvg, curl, wget, file, and build tools. Exact package names vary by
  distribution; follow the official Tauri prerequisites page.

Runtime text injection dependencies:

- X11: `xclip` and `xdotool`
- Wayland: `wl-clipboard` (`wl-copy`, `wl-paste`) and `wtype`

Build:

```bash
npm install
npm run tauri build -- --bundles deb,appimage
```

Linux packages may be distributed as AppImage, Debian, RPM, Snap, Flatpak, or
distribution-specific package formats depending on the release target.

## Model Files

Speech model files are not committed to Git. The app downloads the selected
Parakeet speech model and Silero VAD model into the Tauri app data directory on
first launch or from Settings > Speech.

The default speech model is `parakeet-110m`, currently estimated at about
458 MB.

## Build Artifacts

Frontend build output:

```text
dist/
```

Tauri build output:

```text
src-tauri/target/
```

Both directories are ignored by Git.

The GitHub Actions release workflow currently produces and uploads these
workflow artifacts on `main`:

- `localyapper-windows-x64` (NSIS installer)
- `localyapper-macos-aarch64` (Apple Silicon DMG)
- `localyapper-macos-x64` (Intel DMG)
- `localyapper-linux-x64` (DEB and AppImage)
