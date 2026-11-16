<p align="center">
  <img src="src-tauri/icons/128x128.png" alt="LocalYapper" width="96" height="96" />
</p>

<h1 align="center">LocalYapper</h1>

<p align="center">
  <strong>Offline voice dictation for Windows, macOS and Linux.</strong><br/>
  Press a key, speak, let go. Your words appear in the app you were using.<br/>
  Speech recognition runs on your computer. Audio never leaves it.
</p>

---

## What it does

1. Hold the record key (or tap it to dictate hands-free) and speak.
2. A small pill at the bottom of the screen shows the microphone level.
   Drag it wherever you like; it stays there.
3. Let go. The text is pasted where your cursor is, and your clipboard is
   put back the way it was.

Speech is recognised locally by [sherpa-onnx](https://github.com/k2-fsa/sherpa-onnx)
with a Parakeet or Moonshine model. Silero VAD splits the recording into
phrases, which are decoded while you are still talking, so the text is
usually there a fraction of a second after you let go.

There is no account, no cloud speech service, no telemetry and no
language-model rewrite. The only network traffic is the speech model
download (checked against a pinned SHA-256) and the Releases link in
About, which opens your browser.

## Install

Download the installer for your system from the
[releases page](https://github.com/chayprabs/localyapper/releases). The
builds are not code-signed, so the first launch needs one extra step.

### Windows 10 and 11

Run `LocalYapper_x.y.z_x64-setup.exe`. It installs for your user only.
SmartScreen shows "Windows protected your PC": click **More info**, then
**Run anyway**. WebView2 is downloaded by the installer if it is missing.

### macOS 12 or later

Open the DMG for your Mac (Apple Silicon or Intel) and drag LocalYapper
to Applications. Because the app is not notarised, clear the quarantine
flag once:

```bash
xattr -dr com.apple.quarantine /Applications/LocalYapper.app
```

LocalYapper needs **Microphone** and **Accessibility** access (System
Settings → Privacy & Security). The setup wizard asks for both. macOS
ties the Accessibility grant to the exact app build, so after each
update remove LocalYapper from the Accessibility list and add it again.

### Linux

`.deb` (Debian, Ubuntu), `.rpm` (Fedora, openSUSE) and an AppImage are
provided.

- **X11** is fully supported.
- **Wayland** works with these limits:
  - Hotkeys go through the desktop's GlobalShortcuts portal (GNOME 48+,
    KDE Plasma 6). The desktop may ask you to confirm them. The cancel
    key is not bound on Wayland; use the overlay or `localyapper --cancel`.
  - Pasting needs `wl-clipboard` and `wtype` (or `ydotool`). Without
    them the text is copied and you paste it yourself.
  - The overlay's position is up to the compositor.

## First run

A short wizard checks the microphone (and Accessibility on macOS), lets
you pick the record key, downloads a speech model and has you try a
dictation. On machines with less than 6 GB of RAM it picks the smaller
Moonshine model.

Default keys:

| Action | Windows / Linux | macOS |
|---|---|---|
| Record (hold, or tap for hands-free) | F8 | Option+Space |
| Cancel the recording | Escape | Escape |
| Paste the last dictation again | Ctrl+Alt+J | Ctrl+Alt+J |
| Open LocalYapper | Ctrl+Alt+O | Ctrl+Alt+O |

The command line can drive a running instance:
`localyapper --toggle | --start | --stop | --cancel`.

## Speech models

| Model | Download | Languages | Notes |
|---|---|---|---|
| `parakeet-110m` | 458 MB | English | Default |
| `parakeet-0.6b-v3` | 671 MB | 25 European languages | Most accurate, needs more RAM |
| `moonshine-tiny-en` | 124 MB | English | Chosen automatically under 6 GB RAM |

The model is loaded on the first dictation and unloaded after a period
without dictation (2 minutes by default, set on the Voice page), and its
memory is returned to the system.

## Privacy

- Audio is kept in memory and never written to disk.
- Dictation history is stored only on this computer (SQLite in the data
  folder shown in About). You can turn it off, limit how long it is
  kept, export it or clear it.
- Logs never contain dictated or clipboard text.

## Building from source

You need Rust stable, Node.js 20 or newer, Python 3 and the
[Tauri 2 prerequisites](https://v2.tauri.app/start/prerequisites/) for your
system. On Debian or Ubuntu:

```bash
sudo apt-get install libwebkit2gtk-4.1-dev libayatana-appindicator3-dev \
  librsvg2-dev libssl-dev libasound2-dev libxdo-dev
```

Run it in development, or build the installers for your system:

```bash
npm ci
npm run tauri dev
python3 scripts/gen-notices.py && npm run tauri build
```

The first build downloads the prebuilt sherpa-onnx library from its GitHub
releases. Checks run in CI on every push:

```bash
npm run lint && npx tsc --noEmit
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml
```

Tests that need a speech model (`tests/pipeline.rs`, and the smoke suite
in `tests/smoke.rs`, which drives the real app) read it from
`LOCALYAPPER_TEST_MODEL_DIR`; `scripts/fetch-models.py` downloads and
verifies one. Set `LOCALYAPPER_DEBUG=1` for debug logging.

## Licence

MIT. Third-party components and model licences are listed in
[THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).
