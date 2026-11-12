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

Accuracy and speed for each model are measured in CI on recorded
speech: see [docs/MODEL_BENCHMARK.md](docs/MODEL_BENCHMARK.md). The model
is loaded on the first dictation and unloaded after a period without
dictation (2 minutes by default, set on the Speech page).

## Memory and speed

Measured on the GitHub Actions runners, not estimated: see
[docs/PERFORMANCE.md](docs/PERFORMANCE.md) for private memory per state
(idle, model loaded, settings open) on Windows, macOS and Linux, and the
smoke-suite timings for each runner.

## Privacy

- Audio is kept in memory and never written to disk.
- Dictation history is stored only on this computer (SQLite in the data
  folder shown in About). You can turn it off, limit how long it is
  kept, export it or clear it.
- Logs never contain dictated or clipboard text.

## Building from source

See [docs/BUILD.md](docs/BUILD.md). The architecture is described in
[docs/ARCHITECTURE.md](docs/ARCHITECTURE.md).

## Licence

MIT. Third-party components and model licences are listed in
[THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).
