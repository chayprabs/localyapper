<p align="center">
  <img src="src/assets/logo-nobg.png" alt="LocalYapper" width="128" height="128" />
</p>

<h1 align="center">LocalYapper</h1>

<p align="center">
  <strong>Local-first voice dictation that respects your privacy.</strong><br />
  Open-source alternative to Wispr Flow and SuperWhisper. Fully offline. Zero subscriptions.
</p>

<p align="center">
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-blue.svg" alt="MIT License" /></a>
  <img src="https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux-brightgreen" alt="Platforms" />
  <img src="https://img.shields.io/badge/Tauri-2-orange?logo=tauri&logoColor=white" alt="Tauri 2" />
  <img src="https://img.shields.io/badge/Rust-stable-b7410e?logo=rust&logoColor=white" alt="Rust" />
  <img src="https://img.shields.io/badge/React-19-61dafb?logo=react&logoColor=white" alt="React 19" />
</p>

---

> **Status: On Hold — Rebuilding the inference engine**
>
> Development is paused. The current architecture hits a latency wall with on-device LLM inference — and we refuse to ship something that doesn't feel instant. We're building a completely new inference backend designed to run 3B-parameter models on consumer hardware at 100+ tokens/sec. All research, benchmarks, and results are being published at **[edgeLM](https://github.com/chayprabs/edgeLM)**. LocalYapper will resume once that engine is ready.

---

Hold a hotkey, speak naturally, release — polished text appears wherever you're typing. Everything runs on your machine. No cloud, no account, no subscription.

<!-- TODO: Replace with actual screenshot once available -->
<!-- <p align="center"><img src="dashboard-preview.png" alt="LocalYapper Dashboard" width="720" /></p> -->

## Features

### Privacy First
- **100% offline** — speech recognition and text processing never leave your device
- **Audio stays in RAM** — never written to disk, discarded after processing
- **No telemetry** — zero analytics, zero network calls (except optional BYOK API and one-time model download)
- **Encrypted API keys** — BYOK keys stored encrypted locally, never logged

### Voice Pipeline
- **Hold-to-record** — hold `Ctrl+Shift+Space`, speak, release to transcribe
- **Hands-free mode** — double-tap the hotkey to toggle continuous recording
- **Context-aware cleanup** — detects the focused app and adapts LLM output accordingly
- **Custom AI modes** — create custom system prompts for different use cases (email, code comments, chat)
- **Floating overlay** — translucent always-on-top pill shows recording state, waveform, and transcription preview

### Smart Corrections
- **Self-improving** — learns from your post-injection edits automatically (confidence-weighted)
- **Personal dictionary** — add custom word corrections that apply before the LLM ever sees the text
- **Voice training** — built-in 15-paragraph session that teaches the app your vocabulary and accent
- **Import/Export** — back up or share your correction dictionary as JSON

### Multi-Model Support
- **Local STT** — Whisper base.en (~148 MB), downloaded on first launch
- **Local LLM** — Qwen3 0.6B Q4_K_M (~397 MB), downloaded on first launch
- **Ollama** — connect to a local Ollama instance and pick any model
- **Bring Your Own Key** — use OpenAI, Anthropic, or Groq APIs with your own API key

### Desktop Integration
- **System tray** — lives in tray, hides on close, hotkeys work globally
- **Autostart** — launches at login by default
- **Paste Last** — re-inject the last transcription with `Alt+Shift+V`
- **Customizable hotkeys** — remap every shortcut from the Settings UI
- **First-launch wizard** — guided 9-step setup: model selection, download, hotkey config

### Cross-Platform
- **Windows 10+** — fully supported
- **macOS 12+** — fully supported
- **Linux** — X11 and Wayland, fully supported

## Comparison

| Feature | LocalYapper | Wispr Flow | SuperWhisper |
|---|:---:|:---:|:---:|
| Fully offline | Yes | No | STT only |
| Free & open source | MIT | $15/mo | $8.49/mo |
| Windows | Yes | Yes | Beta |
| macOS | Yes | Yes | Yes |
| Linux | Yes | No | No |
| Auto-learns corrections | Yes | Yes | No |
| Voice training | Yes | No | No |
| Custom AI modes | Yes | No | Yes |
| BYOK API key | Yes | No | Yes |

## How It Works

```
Hold Ctrl+Shift+Space
        |
Audio capture (cpal, device default -> resample 16 kHz mono)
        |
Voice activity detection (energy filter + 0.5s pre-roll buffer)
        |
Speech-to-text (Whisper base.en, on-device)
        |
Correction engine (personal dictionary lookup)
        |
Context detection (identifies focused app)
        |
LLM cleanup (Qwen3 0.6B, on-device via mistral.rs)
        |
Text injection (clipboard save -> paste -> clipboard restore)
        |
Text appears in your app
```

After injection, if you correct a word, LocalYapper learns the mapping and applies it automatically next time — before the text ever reaches the LLM.

## Keyboard Shortcuts

| Action | Default | Description |
|---|---|---|
| Record | `Ctrl+Shift+Space` | Hold to record, release to transcribe |
| Hands-Free | `Ctrl+Shift+Space` (double-tap) | Toggle continuous recording |
| Cancel | `Escape` | Cancel current recording |
| Paste Last | `Alt+Shift+V` | Re-inject last transcription |
| Open App | `Alt+L` | Show/hide the settings window |

All shortcuts are remappable from **Settings > Hotkeys**.

## Tech Stack

| Layer | Technology |
|---|---|
| Framework | [Tauri 2](https://tauri.app/) |
| Backend | Rust (stable 1.75+), tokio, serde |
| Frontend | React 19, TypeScript 5, Vite 5, Tailwind CSS 3 |
| UI Kit | shadcn/ui, Jotai 2, Recharts 2 |
| Audio | cpal 0.15 (cross-platform audio I/O) |
| STT | whisper-rs 0.16 (whisper.cpp bindings) |
| LLM | mistral.rs 0.7 (Candle backend, GGUF inference) |
| Database | rusqlite 0.31 (bundled SQLite) |
| Text injection | enigo 0.2 + arboard 3 (keyboard sim + clipboard) |
| IPC | Tauri command system (43 commands) |

## Installation

Download the latest release for your platform from [GitHub Releases](https://github.com/chayprabs/localyapper/releases):

| Platform | Download |
|---|---|
| Windows 10+ | `.msi` installer |
| macOS 12+ | `.dmg` disk image |
| Linux | `.AppImage` or `.deb` |

On first launch, the setup wizard will guide you through downloading the speech and language models (~545 MB total). After that, the app works fully offline.

## Build from Source

### Prerequisites

- [Rust](https://rustup.rs/) 1.75+
- [Node.js](https://nodejs.org/) 18+
- [LLVM/Clang](https://releases.llvm.org/) (for whisper-rs bindgen)
- [CMake](https://cmake.org/) (for whisper.cpp build)

**Windows:** Set `LIBCLANG_PATH="C:/Program Files/LLVM/bin"` after installing LLVM.

**Linux:** Install system dependencies:
```bash
# Debian/Ubuntu
sudo apt install libasound2-dev libwebkit2gtk-4.1-dev \
  xclip xdotool          # X11
  # or: wl-clipboard wtype  # Wayland
```

### Build

```bash
git clone https://github.com/chayprabs/localyapper.git
cd localyapper
npm install
npm run tauri build
```

The compiled binary will be in `src-tauri/target/release/`.

### Development

```bash
npm run tauri dev       # Full Tauri dev mode (frontend + backend)
npm run dev             # Vite dev server only (frontend)
npm run lint            # ESLint
npx tsc --noEmit        # TypeScript type check
cd src-tauri && cargo clippy -- -D warnings  # Rust linter
```

## Architecture

```
src/                    React frontend (TypeScript)
  components/           UI components (settings pages, overlay, wizard)
  hooks/                Custom React hooks (7 hooks)
  lib/commands/         Typed Tauri IPC wrappers (8 files, 43 commands)
  stores/               Jotai global state atoms

src-tauri/src/          Rust backend
  audio/                Audio capture + voice activity detection
  stt/                  Whisper speech-to-text engine
  llm/                  LLM inference engine (mistral.rs)
  correction/           Dictionary engine + learning system
  injection/            Text injection (clipboard + paste simulation)
  hotkey/               Global shortcut manager + state machine
  context/              Focused app detection (per-OS)
  commands/             Tauri IPC command handlers
  db/                   SQLite schema, migrations, queries (6 tables)
  tray/                 System tray menu + autostart
  models/               Shared data types
```

Two Tauri windows:
- **Main** (900x650) — settings app with sidebar navigation: Dashboard, History, Dictionary, Hotkeys, Models
- **Overlay** (320x80) — floating translucent pill that shows recording/processing/transcription state

## Privacy

LocalYapper is built on a simple principle: **your voice data never leaves your machine.**

- All speech recognition and text processing happens on-device
- Audio is held in RAM only during processing — never written to disk
- No telemetry, no analytics, no network calls (except optional BYOK API and one-time model download)
- API keys are stored encrypted and never logged
- The app works fully offline after the initial model download

## Contributing

Contributions are welcome! Feel free to open an [issue](https://github.com/chayprabs/localyapper/issues) or submit a pull request.

## Contact

Built by **Chaitanya Prabuddha**.

- Twitter: [@chayprabs](https://twitter.com/chayprabs)
- LinkedIn: [chaitanya-prabuddha-bits94](https://linkedin.com/in/chaitanya-prabuddha-bits94)

For bugs, questions, or feature requests, reach out at **chaitanyaplusplus@gmail.com** or [open an issue](https://github.com/chayprabs/localyapper/issues).

## License

[MIT](LICENSE)
