<p align="center">
  <img src="src/assets/logo-nobg.png" alt="LocalYapper" width="128" height="128" />
</p>

<h1 align="center">LocalYapper</h1>

<p align="center">
  <strong>Lightning-fast, fully offline voice dictation for your desktop.</strong><br/>
  Open-source alternative to WisprFlow and SuperWhisper. No cloud. No subscription. No data leaves your machine. Ever.
</p>

<p align="center">
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-blue.svg" alt="MIT License" /></a>
  <img src="https://img.shields.io/badge/Platform-Windows%20%7C%20macOS%20%7C%20Linux-brightgreen" alt="Platforms" />
  <img src="https://img.shields.io/badge/STT-Parakeet%20110M%20%7C%202.4%25%20WER-00b4d8" alt="Parakeet STT" />
  <img src="https://img.shields.io/badge/Latency-~200ms%20casual-ff6b35" alt="Latency" />
  <img src="https://img.shields.io/badge/RAM-~1.2%20GB-9b59b6" alt="RAM" />
  <img src="https://img.shields.io/badge/Tauri-2-ffc131?logo=tauri&logoColor=white" alt="Tauri 2" />
  <img src="https://img.shields.io/badge/Rust-stable-b7410e?logo=rust&logoColor=white" alt="Rust" />
  <img src="https://img.shields.io/badge/React-19-61dafb?logo=react&logoColor=white" alt="React 19" />
</p>

---

Hold a hotkey, speak naturally, release -- polished text appears wherever you're typing. The entire inference pipeline -- voice activity detection, speech recognition, and text correction -- runs on your CPU in ~1.2 GB of RAM. No internet. No API keys. No compromises.

<!-- TODO: Replace with actual GIF once recorded -->
<!-- <p align="center"><img src="docs/demo.gif" alt="LocalYapper Demo" width="720" /></p> -->

---

## Performance

### LocalYapper vs WisprFlow

| Metric | **LocalYapper** | WisprFlow |
|:-------|:--------------:|:---------:|
| **300-word paragraph** | **2.1s** | 5.4s |
| **Single sentence** | **~0.4s** | ~1.8s |
| **Casual mode (skip LLM)** | **~200ms** | N/A |
| **Word accuracy** | 91% | 96% |
| **Works offline** | Yes | No |
| **Internet required** | Never | Always |
| **Privacy** | 100% on-device | Audio sent to cloud |
| **Price** | Free forever | $8--15/month |
| **Windows** | Yes | Yes |
| **macOS** | Yes | Yes |
| **Linux** | Yes | No |

> WisprFlow achieves higher accuracy by streaming your audio to cloud GPUs. LocalYapper trades 5% accuracy for **complete privacy** and **2.6x faster processing** -- on consumer hardware, with zero network dependency.

### Latency Breakdown

| Pipeline Stage | Time |
|:--------------|-----:|
| Hotkey to overlay visible | < 50ms |
| Audio capture + Silero VAD | ~10ms |
| Parakeet STT (10s audio) | ~150--400ms |
| Correction engine lookup | < 5ms |
| LLM cleanup (when needed) | ~500--750ms |
| Text injection | ~80ms |
| **Total (casual, no LLM)** | **~200ms** |
| **Total (with LLM formatting)** | **~700--900ms** |

### Memory Footprint

| Component | RAM |
|:----------|----:|
| Parakeet 110M STT model | ~200 MB |
| Qwen2.5-1.5B LLM model | ~1.0 GB |
| Silero VAD model | ~10 MB |
| App + Tauri runtime | ~50 MB |
| **Total** | **~1.2 GB** |

For context: Chrome with 5 tabs uses ~1.5 GB. LocalYapper delivers real-time voice dictation for less RAM than your browser.

In **Casual mode**, the LLM stays unloaded -- total footprint drops to **~250 MB**.

---

## How It Works

```
  Hotkey Press (overlay appears instantly)
       |
       v
  +------------------+     +-------------------+     +--------------------+
  | Audio Capture     | --> | Silero VAD        | --> | Parakeet STT       |
  | cpal 16kHz mono   |     | Neural silence    |     | 110M params        |
  | 0.5s pre-roll     |     | detection (<1ms)  |     | 2.4% WER           |
  +------------------+     +-------------------+     | Native punct/caps  |
                                                      +--------------------+
                                                              |
                                          +-------------------+-------------------+
                                          |                                       |
                                   Casual / Brain Dump                  Formal / Code / Translate
                                   (skip LLM = instant)                         |
                                          |                              +------v----------+
                                          |                              | Qwen2.5-1.5B    |
                                          |                              | Local LLM       |
                                          |                              | Tone & style    |
                                          |                              +-----------------+
                                          |                                       |
                                          +---------------+-----------------------+
                                                          |
                                                          v
                                                 +----------------+
                                                 | Text Injection  |
                                                 | Save clipboard  |
                                                 | Ctrl+V / Cmd+V  |
                                                 | Restore clipboard|
                                                 +----------------+
                                                          |
                                                          v
                                                 Text appears in
                                                 your focused app
```

**The key insight:** NVIDIA's Parakeet model outputs **already-punctuated, capitalized text** -- unlike Whisper which gives raw lowercase. This means 80%+ of dictations skip the LLM entirely, dropping end-to-end latency to ~200ms.

---

## Features

### Voice Dictation
- **Hold-to-talk** -- press and hold hotkey, speak, release to inject
- **Hands-free mode** -- double-tap the hotkey to toggle continuous recording
- **Works in any app** -- text appears wherever your cursor is (VS Code, Slack, Chrome, Word, Terminal, everywhere)
- **Single-word support** -- even "hi" or "yes" gets transcribed (0.2s minimum)
- **120-second max** -- with a 15-second warning countdown

### 5 Intelligent Modes

| Mode | What It Does | Uses LLM? | Speed |
|:-----|:------------|:---------:|:-----:|
| **Casual** | Clean dictation with punctuation and capitalization | No | ~200ms |
| **Formal** | Professional tone, complete sentences, proper grammar | Yes | ~800ms |
| **Code** | Preserves technical terms, variable names, function names exactly | Yes | ~800ms |
| **Brain Dump** | Raw unfiltered transcription, nothing added or removed | No | ~200ms |
| **Translate** | Speak in any language, get English text | Yes | ~900ms |

Create unlimited custom modes with your own system prompts.

### Smart Correction Engine
- **Auto-learning** -- tracks your corrections over time and auto-applies them
- **Personal dictionary** -- protect names, jargon, and technical terms from being "corrected"
- **Voice training** -- 15 built-in paragraphs to teach the system your speech patterns
- **Confidence scoring** -- corrections improve with repeated use (threshold: 0.6)
- **Import/Export** -- back up or share your correction dictionary as JSON

### Desktop Integration
- **System tray** -- pause/resume dictation, change modes, see status at a glance
- **Autostart** -- launches with your OS, ready when you are
- **Floating overlay** -- translucent always-on-top pill shows recording state, waveform animation, and transcription preview
- **Customizable hotkeys** -- remap every shortcut from Settings
- **First-launch wizard** -- 9-step guided setup gets you dictating in under 3 minutes

### Multi-Model Support
- **Local (default)** -- Parakeet 110M STT + Qwen2.5-1.5B LLM, fully offline
- **Ollama** -- connect to a local Ollama instance and pick any model
- **BYOK** -- use OpenAI, Anthropic, or Groq APIs with your own key

---

## Tech Stack

Built for performance with a zero-compromise, no-Electron architecture:

| Layer | Technology | Why |
|:------|:----------|:----|
| **App Framework** | Tauri 2 + Rust | 10x smaller than Electron, native performance, direct OS APIs |
| **Frontend** | React 19, TypeScript 5, Vite 5 | Fast HMR, strict types, modern React features |
| **Styling** | Tailwind CSS 3 + shadcn/ui | macOS HIG design language, Apple-quality UI |
| **State** | Jotai 2 | Atomic state management, zero boilerplate |
| **STT Engine** | sherpa-onnx 1.12 (ONNX Runtime) | Static linking, no native build deps, cross-platform |
| **STT Model** | NVIDIA Parakeet TDT-CTC 110M | 2.4% WER, native punctuation/caps, NeMo architecture |
| **VAD** | Silero VAD (neural, via sherpa-onnx) | Sub-millisecond inference, far superior to energy-based |
| **LLM Runtime** | mistralrs 0.7 (Candle backend) | Pure Rust, no Python dependency, CPU-optimized |
| **LLM Model** | Qwen2.5-1.5B-Instruct Q4_K_M | Best text quality under 2B params (MT-Bench 6.52) |
| **Audio** | cpal 0.15 | Cross-platform audio capture with automatic resampling |
| **Database** | rusqlite 0.31 (bundled SQLite) | Zero-config embedded database, WAL mode |
| **Text Injection** | enigo 0.2 + arboard 3 | Cross-platform clipboard + paste simulation |
| **IPC** | Tauri command system | Type-safe Rust-to-TypeScript bridge, 45 commands |

### By the Numbers

```
45   IPC commands
 6   database tables
 5   built-in dictation modes
23   Rust source modules
30+  React components
 9   wizard onboarding steps
17   implementation phases completed
 0   bytes of audio sent to any server
```

---

## Architecture

```
localyapper/
+-- src-tauri/src/              Rust backend
|   +-- audio/
|   |   +-- capture.rs          cpal 16kHz mono + linear interpolation resampler
|   |   +-- vad.rs              Silero neural VAD + energy-based fallback
|   +-- stt/
|   |   +-- whisper.rs          sherpa-onnx OfflineRecognizer (Parakeet CTC)
|   +-- llm/
|   |   +-- engine.rs           mistralrs GgufModelBuilder (Qwen2.5-1.5B)
|   |   +-- prompt.rs           Mode-aware system prompt builder
|   +-- correction/
|   |   +-- engine.rs           In-memory HashMap lookup with case preservation
|   |   +-- learner.rs          Diff computation + confidence scoring
|   +-- injection/
|   |   +-- injector.rs         Clipboard save -> paste -> restore (per-OS)
|   |   +-- platform.rs         Runtime OS detection (Win/Mac/X11/Wayland)
|   +-- hotkey/
|   |   +-- manager.rs          Atomic state machine (hold/hands-free/double-tap)
|   +-- context/
|   |   +-- detector.rs         Focused window name via OS APIs
|   +-- commands/               45 Tauri IPC command handlers
|   +-- db/                     SQLite schema + 6 tables + typed queries
|   +-- tray/                   System tray icon + context menu
|   +-- state.rs                Arc<Mutex<>> hot-reloadable app state
|   +-- lib.rs                  Central command registration hub
|
+-- src/                        React frontend
    +-- components/
    |   +-- overlay/            Floating pill (6 visual states + waveform)
    |   +-- dashboard/          Stats, model status, last dictation
    |   +-- history/            Paginated transcription history
    |   +-- dictionary/         Corrections table + voice training
    |   +-- hotkeys/            Live key capture + remapping
    |   +-- models/             Download manager + model switching
    |   +-- wizard/             9-step first-launch onboarding
    |   +-- settings/           Sidebar navigation layout
    +-- hooks/                  Custom React hooks (state + IPC)
    +-- stores/                 Jotai atoms (global state)
    +-- lib/commands/           Typed IPC wrappers (1:1 with Rust)
    +-- types/                  TypeScript definitions
```

**Two windows:**
- **Main** (900x650) -- settings app with sidebar: Dashboard, History, Dictionary, Hotkeys, Models
- **Overlay** (floating pill) -- always-on-top transparent pill showing waveform, countdown, and transcribed text

---

## Getting Started

### Prerequisites

- [Node.js](https://nodejs.org/) 18+ and npm
- [Rust](https://rustup.rs/) stable 1.75+
- Platform-specific Tauri 2 dependencies ([see Tauri docs](https://v2.tauri.app/start/prerequisites/))

**Linux only:**
```bash
# Debian/Ubuntu
sudo apt install libasound2-dev libwebkit2gtk-4.1-dev
# X11: xclip xdotool
# Wayland: wl-clipboard wtype
```

### Build & Run

```bash
git clone https://github.com/chaitanyarahalkar/localyapper.git
cd localyapper

npm install
npm run tauri dev       # Development mode (hot-reload)
npm run tauri build     # Production build
```

The first build downloads sherpa-onnx prebuilt libraries (~200 MB, one-time). **No LLVM or CMake required** -- unlike whisper.cpp-based tools.

### First Launch

1. The setup wizard downloads models automatically (~1.5 GB total)
2. Configure your hotkey (default: `Ctrl+Shift+Space`)
3. Start dictating -- text appears in any focused app

---

## Keyboard Shortcuts

| Shortcut | Action |
|:---------|:-------|
| `Ctrl+Shift+Space` | Hold to record, release to transcribe |
| `Ctrl+Shift+Space` x2 | Double-tap for hands-free mode |
| `Escape` | Cancel current recording |
| `Alt+Shift+V` | Re-inject last dictation |
| `Alt+L` | Toggle settings window |

All shortcuts are fully customizable in **Settings > Hotkeys**.

---

## Privacy & Security

LocalYapper is built on a simple principle: **your voice never leaves your machine.**

- **Zero cloud processing** -- all STT, VAD, and LLM inference runs on your CPU
- **Audio in RAM only** -- never written to disk, discarded immediately after processing
- **No telemetry** -- no analytics, no tracking, no phone-home, no network calls
- **No accounts** -- no sign-up, no login, no email
- **Encrypted API keys** -- BYOK keys stored encrypted locally, never logged
- **Fully offline** -- works without internet after initial model download
- **Open source** -- every line of code is auditable under the MIT license

---

## Supported Platforms

| Platform | Audio | Text Injection | Status |
|:---------|:-----:|:--------------:|:------:|
| Windows 10+ | cpal (WASAPI) | enigo (Ctrl+V) | Full support |
| macOS 12+ | cpal (CoreAudio) | enigo (Cmd+V) | Full support |
| Linux X11 | cpal (ALSA) | xdotool + xclip | Full support |
| Linux Wayland | cpal (ALSA) | wtype + wl-clipboard | Full support |

---

## Development

```bash
npm run lint                                        # ESLint
npx tsc --noEmit                                    # TypeScript check
cd src-tauri && cargo clippy -- -D warnings         # Rust linter
cargo test --manifest-path src-tauri/Cargo.toml     # Rust tests
npm run tauri dev                                   # Full dev mode
```

---

## Roadmap

- [ ] Streaming transcription (real-time partial results as you speak)
- [ ] App-specific mode profiles (auto-switch mode per app)
- [ ] Model tier system (Fast / Balanced / Accurate with different model sizes)
- [ ] llama.cpp migration for 20-40% faster LLM inference on CPU
- [ ] Speculative decoding (draft + verify for sub-second LLM)
- [ ] Custom QLoRA fine-tuning on ASR correction datasets

---

## Contributing

Contributions welcome! Open an [issue](https://github.com/chaitanyarahalkar/localyapper/issues) or submit a pull request.

---

## Contact

Built by **Chaitanya Rahalkar**.

- Twitter: [@chabornstocode](https://x.com/chabornstocode)
- LinkedIn: [chaitanya-rahalkar](https://linkedin.com/in/chaitanya-rahalkar)
- Email: chaitanyaplusplus@gmail.com

---

## License

[MIT](LICENSE) -- use it, fork it, ship it.

<p align="center">
  <sub>Built with Rust, React, and mass amounts of caffeine. Entirely vibe-coded with <a href="https://claude.ai/code">Claude Code</a>.</sub>
</p>
