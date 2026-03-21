<p align="center">
  <img src="src-tauri/icons/icon.png" alt="LocalYapper" width="128" height="128" />
</p>

<h1 align="center">LocalYapper</h1>

<p align="center">
  <strong>Voice dictation that works for everyone, everywhere, privately.</strong>
</p>

<p align="center">
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-blue.svg" alt="MIT License" /></a>
  <img src="https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux-brightgreen" alt="Platforms" />
  <img src="https://img.shields.io/badge/built%20with-Tauri%202-orange" alt="Tauri 2" />
</p>

---

LocalYapper is a **local-first, open-source, cross-platform voice dictation desktop app** for Windows, macOS, and Linux. It is a privacy-respecting alternative to Wispr Flow and SuperWhisper that runs entirely on-device with zero telemetry, zero subscriptions, and zero cloud dependencies. Hold a hotkey, speak naturally, release, and polished text appears wherever you're typing.

## Features

- **Fully offline** — all processing happens on your machine, nothing leaves your device
- **Cross-platform** — first-class support for Windows 10+, macOS 12+, and Linux (X11 + Wayland)
- **Zero setup** — one-click install, bundled speech model, LLM downloads in-app
- **Self-improving** — learns from your corrections automatically with confidence decay
- **Voice training** — built-in 15-paragraph session that teaches the app your vocabulary
- **Lightweight** — ~30 MB RAM idle (vs. 800 MB for Electron-based alternatives)
- **Custom AI modes** — context-aware text cleanup adapts to the app you're dictating into
- **Bring your own key** — optionally use OpenAI, Anthropic, or Groq APIs instead of the local model

## Comparison

| Feature | LocalYapper | Wispr Flow | SuperWhisper |
|---|:---:|:---:|:---:|
| Fully local/offline | :white_check_mark: | :x: | STT only |
| Free | :white_check_mark: | :x: $15/mo | :x: $8.49/mo |
| Open source | :white_check_mark: MIT | :x: | :x: |
| Windows | :white_check_mark: | :white_check_mark: | Beta |
| macOS | :white_check_mark: | :white_check_mark: | :white_check_mark: |
| Linux | :white_check_mark: | :x: | :x: |
| Auto-learns corrections | :white_check_mark: | :white_check_mark: | :x: |
| Voice training session | :white_check_mark: | :x: | :x: |
| Custom AI modes | :white_check_mark: | :x: | :white_check_mark: |
| Bundled model (no setup) | :white_check_mark: | N/A | :x: |
| BYOK API key | :white_check_mark: | :x: | :white_check_mark: |
| RAM usage (idle) | ~30 MB | ~800 MB | ~120 MB |

## Screenshots

<!-- screenshot: main settings window -->
<!-- screenshot: floating overlay pill during dictation -->

## Installation

Download the latest release for your platform from [GitHub Releases](https://github.com/chaitanya/localyapper/releases):

| Platform | Download |
|---|---|
| Windows 10+ | `.msi` installer |
| macOS 12+ | `.dmg` disk image |
| Linux | `.AppImage` or `.deb` package |

On first launch, LocalYapper will offer to download the LLM model (~400 MB). The speech recognition model (~75 MB) is bundled with the installer.

## Build from source

### Prerequisites

- [Rust](https://rustup.rs/) 1.75+
- [Node.js](https://nodejs.org/) 18+
- [LLVM/Clang](https://releases.llvm.org/) (for whisper-rs bindgen)
- [CMake](https://cmake.org/) (for whisper.cpp build)

**Linux only:** `libasound2-dev` (ALSA), `libwebkit2gtk-4.1-dev`, `xclip`, `xdotool` (X11) or `wl-clipboard`, `wtype` (Wayland)

### Build

```bash
npm install
npm run tauri build
```

The compiled binary will be in `src-tauri/target/release/`.

## How it works

```
Hold hotkey
    ↓
Audio capture (cpal, 16 kHz mono)
    ↓
Voice activity detection (energy filter + 0.5 s pre-roll)
    ↓
Speech-to-text (Whisper base.en, local)
    ↓
Correction engine (personal dictionary lookup)
    ↓
Context detection (identifies focused app)
    ↓
LLM cleanup (Qwen 2.5 0.5B, local)
    ↓
Text injection (clipboard → paste → restore)
    ↓
Text appears in your app
```

The app also monitors your edits after injection. When you correct a word, LocalYapper learns the mapping and applies it automatically next time — before the text even reaches the LLM.

## Privacy

LocalYapper is built on a simple principle: **your voice data never leaves your machine.**

- All speech recognition and text processing happens on-device
- Audio is held in RAM only during processing — never written to disk
- No telemetry, no analytics, no network calls (except optional BYOK API and model download)
- API keys are stored encrypted and never logged
- The app works fully offline after the initial model download

## Contributing

Contributions are welcome! Feel free to open an [issue](https://github.com/chaitanya/localyapper/issues) or submit a pull request.

## Contact

Built by **Chaitanya Prabuddha**.

- Twitter: [@chayprabs](https://twitter.com/chayprabs)
- LinkedIn: [chaitanya-prabuddha-bits94](https://linkedin.com/in/chaitanya-prabuddha-bits94)

For bugs, questions, or feature requests, reach out at **chaitanyaplusplus@gmail.com** or [open an issue](https://github.com/chaitanya/localyapper/issues).

## License

[MIT](LICENSE)
