# LocalYapper — Product Requirements Document
**Version:** 1.0.0  
**Author:** Chaitanya  
**Status:** Final  
**Repository:** github.com/chaitanya/localyapper  
**Last Updated:** March 2026  

---

## Table of Contents

1. [Executive Summary](#1-executive-summary)
2. [Problem Statement](#2-problem-statement)
3. [Competitive Analysis](#3-competitive-analysis)
4. [Target Users](#4-target-users)
5. [Product Vision & Goals](#5-product-vision--goals)
6. [Tech Stack](#6-tech-stack)
7. [System Architecture](#7-system-architecture)
8. [Design System](#8-design-system)
9. [Screen Inventory](#9-screen-inventory)
10. [Feature Specifications](#10-feature-specifications)
11. [Data Models](#11-data-models)
12. [IPC Command Reference](#12-ipc-command-reference)
13. [Implementation Phases](#13-implementation-phases)
14. [Non-Functional Requirements](#14-non-functional-requirements)
15. [Security Requirements](#15-security-requirements)
16. [Open Source & Distribution](#16-open-source--distribution)
17. [Out of Scope — v0.1.0](#17-out-of-scope--v010)
18. [Future Roadmap](#18-future-roadmap)
19. [Success Metrics](#19-success-metrics)
20. [Appendix](#20-appendix)

---

## 1. Executive Summary

LocalYapper is a **local-first, open-source, cross-platform voice dictation desktop application** for Windows, macOS, and Linux. It is a privacy-respecting alternative to Wispr Flow and SuperWhisper — two cloud-dependent paid dictation tools — that runs entirely on-device with zero telemetry, zero subscriptions, and zero cloud dependencies by default.

The user holds a configurable hotkey, speaks naturally, releases the key, and polished text appears in whatever application they were typing in. Everything — speech transcription, AI text cleanup, personal dictionary, correction learning — happens locally on the user's machine.

**Core value proposition:** The speed and polish of Wispr Flow, the privacy of SuperWhisper, the price of open source.

**Platform:** Windows 10+, macOS 12+, Linux (X11 + Wayland)  
**License:** MIT  
**App download size:** ~15MB (models downloaded on first launch, ~545MB total)
**LLM download:** ~397MB on first launch (optional, can skip)  

---

## 2. Problem Statement

### The current landscape is broken for privacy-conscious users

Voice dictation tools in 2026 fall into two categories:

**Category A — Cloud-dependent, fast, polished, expensive, privacy-invasive:**
- Wispr Flow ($15/month): all audio goes to Baseten-hosted GPUs on AWS, then through OpenAI and Llama models. Electron app consuming 800MB RAM. History of privacy controversies including capturing screenshots of active windows. Trustpilot: 2.6/5.
- SuperWhisper ($249 lifetime / $8.49/month): local STT but optional cloud LLM cleanup. Mac-only primary, Windows in beta with significant feature gaps.

**Category B — Fully local but painful to set up:**
- Requires installing Ollama separately, downloading models manually, configuring settings files. Not accessible to non-technical users.

**The gap:** There is no application that is simultaneously (1) fully local, (2) works out of the box with zero setup, (3) cross-platform, (4) free and open source, and (5) polished enough for daily professional use.

**LocalYapper fills this gap.**

### User pain points being solved

| Pain Point | Current Solution | LocalYapper Solution |
|---|---|---|
| Privacy — audio sent to cloud | Accept it or don't use dictation | Everything processed on-device |
| Cost — $15/month subscription | Pay or don't use | Completely free forever |
| Setup complexity | Manual Ollama install + config | One-click install, model downloads in-app |
| Linux support | No good options exist | First-class Linux support |
| App size bloat | Electron (800MB RAM idle) | Tauri (~30MB RAM idle) |
| Self-improvement | Manual dictionary management | Auto-learns from every correction |
| Voice training | None | Built-in 15-paragraph training session |

---

## 3. Competitive Analysis

### Feature Matrix

| Feature | LocalYapper | Wispr Flow | SuperWhisper |
|---|---|---|---|
| Fully local/offline | ✅ | ❌ | ✅ STT only |
| Free | ✅ | ❌ $15/mo | ❌ $8.49/mo |
| Open source | ✅ MIT | ❌ | ❌ |
| Windows | ✅ First class | ✅ | ⚠️ Beta |
| macOS | ✅ First class | ✅ | ✅ |
| Linux | ✅ First class | ❌ | ❌ |
| Auto-learns corrections | ✅ | ✅ | ❌ |
| Voice training session | ✅ | ❌ | ❌ |
| Custom AI modes | ✅ | ❌ | ✅ |
| Bundled model (no setup) | ✅ | N/A | ❌ |
| Hold-to-record | ✅ | ✅ | ✅ |
| Double-tap hands-free | ✅ | ✅ | ❌ |
| Overlay with timer | ✅ | ⚠️ partial | ✅ |
| BYOK API key | ✅ | ❌ | ✅ |
| Paste Last shortcut | ✅ | ✅ | ❌ |
| Translation mode | ✅ | ❌ | ✅ |
| RAM usage idle | ~30MB | ~800MB | ~120MB |

### Key differentiators for LocalYapper
1. **Only fully local + cross-platform + free option** — no competitor covers all three
2. **Voice training session** — teaches the app your specific vocabulary and pronunciation patterns
3. **Self-improving corrections with confidence decay** — learns and unlearns automatically
4. **Countdown timer in overlay** — eliminates the "is it frozen?" frustration during processing
5. **In-app model download** — no separate tool installation required

---

## 4. Target Users

### Primary Persona — "The Privacy-Conscious Professional"
- **Age:** 25-45
- **Role:** Developer, writer, researcher, knowledge worker
- **Tech level:** Intermediate to advanced
- **Pain:** Uses voice dictation daily but uncomfortable with audio going to cloud servers
- **Goal:** Dictate emails, Slack messages, code comments, documents at 3-4x typing speed
- **Device:** Mid-range laptop, no dedicated GPU, 8-16GB RAM
- **Quote:** "I would pay for Wispr Flow if it didn't send everything to OpenAI's servers"

### Secondary Persona — "The Linux Developer"
- **Age:** 22-40
- **Role:** Software developer, sysadmin, open source contributor
- **Tech level:** Advanced
- **Pain:** No good dictation tool exists for Linux. Period.
- **Goal:** Voice dictation in VS Code, terminal, and browser on Arch/Ubuntu/Fedora
- **Device:** Desktop or laptop running X11 or Wayland
- **Quote:** "I've tried every dictation tool. None of them work on Linux properly."

### Tertiary Persona — "The Accessibility User"
- **Age:** Any
- **Role:** Any — RSI, Parkinson's, dyslexia, motor impairment
- **Tech level:** Low to intermediate
- **Pain:** Typing is painful or impossible. Cloud tools feel unsafe for personal content.
- **Goal:** Use computer normally using voice as primary input
- **Device:** Any
- **Quote:** "This has made my computer accessible again"

---

## 5. Product Vision & Goals

### Vision Statement
> "Voice dictation that works for everyone, everywhere, privately."

### v0.1.0 Goals
1. Working end-to-end dictation pipeline on all three platforms
2. Zero required external dependencies — app works out of the box after install
3. Self-improving personal dictionary that learns from corrections
4. Polished overlay experience that doesn't feel like a developer tool
5. GitHub release with downloadable binaries for Windows, macOS, Linux

### Design Principles
1. **Privacy by default** — no data ever leaves the machine without explicit user consent
2. **Works offline always** — no internet required after initial model download
3. **Zero friction** — hold key, speak, release, done
4. **Self-improving** — gets better the more you use it without manual effort
5. **First class everywhere** — Windows, macOS, and Linux are equal citizens

---

## 6. Tech Stack

### Exact versions — do not deviate

| Layer | Technology | Version | Purpose |
|---|---|---|---|
| App framework | Tauri | 2.x | Desktop app wrapper |
| Backend language | Rust | stable (1.75+) | All business logic |
| Frontend framework | React | 19.x | Settings + overlay UI |
| Frontend language | TypeScript | 5.x | Type safety |
| Build tool | Vite | 5.x | Frontend bundler |
| Styling | Tailwind CSS | 3.x | Utility CSS |
| UI components | shadcn/ui | latest | Base components |
| State management | Jotai | 2.x | Global React state |
| Charts | Recharts | 2.x | Dashboard stats |
| Database | SQLite via rusqlite | 0.31 (bundled) | All persistence |
| Audio capture | cpal | 0.15 | Cross-platform audio |
| Speech-to-text | whisper-rs | 0.16 | Whisper model wrapper |
| LLM inference | llama-cpp-rs | latest | Local LLM runtime |
| HTTP client | reqwest | 0.12 | Model download + BYOK |
| Async runtime | tokio | 1.x (full) | Async Rust |
| Text injection | enigo | 0.2 | Keyboard simulation |
| Serialization | serde + serde_json | 1.x | Data serialization |
| Unique IDs | uuid | 1.x (v4) | Record IDs |
| Date/time | chrono | 0.4 (serde) | Timestamps |
| Error handling | anyhow + thiserror | 1.x | Error types |
| Logging | log + env_logger | 0.4 / 0.11 | Debug logging |
| Tauri plugins | global-shortcut, autostart, shell, notification | 2.x | OS integration |

### Bundled model files
| File | Size | Location | Purpose |
|---|---|---|---|
| ggml-base.en.bin | ~148MB | {APP_DATA}/models/ | Whisper STT — downloaded on first launch |
| qwen2.5-0.5b-q4.gguf | ~400MB | {APP_DATA}/models/ | LLM cleanup — downloaded on first launch |

### Platform-specific injection dependencies
| OS | Primary | Fallback |
|---|---|---|
| macOS | enigo Cmd+V | AppleScript keystroke |
| Windows | enigo Ctrl+V | SendInput API |
| Linux X11 | xclip + xdotool | enigo keystrokes |
| Linux Wayland | wl-clipboard + wtype | ydotool |

---

## 7. System Architecture

### Two Tauri windows

**Window 1: main**
- Label: `main`
- Purpose: Settings app — Dashboard, History, Dictionary, Hotkeys, Models, first-launch wizard
- Size: 900×650px, resizable, normal window decorations
- Always on top: false
- Skip taskbar: false

**Window 2: overlay**
- Label: `overlay`
- Purpose: Floating pill shown during dictation
- Size: 320×80px (expands to 320×100px in transcribed state)
- Always on top: true
- Decorations: false (transparent, borderless)
- Skip taskbar: true
- Position: remembers last dragged position, stored in settings

### Voice pipeline — complete data flow

```
[User holds hotkey]
        ↓
[audio/capture.rs] — opens default input device via cpal
        ↓ 16kHz mono PCM f32 samples
[audio/vad.rs] — energy-based silence filter
        ↓ filtered audio buffer (includes 0.5s pre-roll)
[stt/whisper.rs] — whisper-rs with ggml-base.en.bin
        ↓ raw text string (e.g. "helli can you schedule a meeting")
[correction/engine.rs] — exact-match substitution from personal_dictionary
        ↓ corrected text (e.g. "hello can you schedule a meeting")
[context/detector.rs] — get focused app name
        ↓ app name string (e.g. "Slack")
[llm/prompt.rs] — build context-aware system prompt
        ↓ system prompt + corrected text
[llm/engine.rs] — llama-cpp-rs inference with qwen2.5-0.5b-q4.gguf
        ↓ final cleaned text
[injection/injector.rs] — clipboard save → set → Cmd/Ctrl+V → restore
        ↓
[Text appears in focused app]
        ↓
[correction/learner.rs] — monitors post-inject edits, learns corrections
```

### Self-improvement loop

```
[User edits injected text]
        ↓
[correction/learner.rs] computes word-level diff
        ↓
["helli" → "hello"] written to corrections table
        ↓
[count increments, confidence recalculates]
        ↓
[Next time Whisper outputs "helli"]
        ↓
[correction/engine.rs finds it, substitutes "hello" BEFORE LLM]
        ↓
[If count ≥ 3 AND confidence ≥ threshold: skip LLM for this word]
```

### Confidence decay
- Every 30 days of non-use: confidence halves
- If confidence drops below 0.2: entry is soft-deleted (hidden, not removed)
- User can manually set confidence_threshold in settings (default: 0.6)

### Model download flow (first launch)

```
[User selects "Download Qwen" on model selection screen]
        ↓
[downloader.rs] GET https://huggingface.co/.../qwen2.5-0.5b-instruct-q4_k_m.gguf
        ↓ HTTP Range headers for resumable download
[Progress events streamed to frontend via app_handle.emit()]
        ↓ "model_download_progress" events with {percent, downloaded_mb, total_mb, speed_mbps}
[Frontend CountdownTimer/progress bar updates in real time]
        ↓
[File saved to {APP_DATA}/models/qwen2.5-0.5b-q4.gguf]
        ↓
[Wizard advances to hotkey setup step]
```

---

## 8. Design System

### Philosophy
Apple macOS Human Interface Guidelines. Light mode only. Every decision references macOS Ventura System Settings as the gold standard. Nothing that Apple would never put there.

### Color palette

| Token | Value | Usage |
|---|---|---|
| window-bg | #EDEDED | App window background |
| sidebar-bg | #EBEBEB | Sidebar background |
| card-bg | #FFFFFF | All card surfaces |
| card-border | rgba(0,0,0,0.07) | Card borders |
| primary | #007AFF | Buttons, active states, accents |
| primary-tint | rgba(0,122,255,0.12) | Active sidebar item bg |
| success | #28CD41 | Running status, success states |
| destructive | #FF3B30 | Delete, error, warning accents |
| text-primary | #1C1C1E | All primary text |
| text-secondary | rgba(0,0,0,0.50) | Descriptions, subtitles |
| text-tertiary | rgba(0,0,0,0.35) | Hints, captions, timestamps |
| text-label | rgba(0,0,0,0.40) | Uppercase section labels |
| separator | rgba(0,0,0,0.08) | Dividers inside cards |
| overlay-bg | rgba(255,255,255,0.95) | Floating overlay pill |
| overlay-border | rgba(0,0,0,0.10) | Overlay pill border |

### Typography

| Role | Size | Weight | Usage |
|---|---|---|---|
| Large Title | 26px | 600 | Page titles |
| Title 2 | 17px | 600 | Card titles, hero numbers |
| Headline | 13px | 600 | Row labels, action names |
| Body | 13px | 400 | All regular text |
| Callout | 12px | 400 | Descriptions under headlines |
| Caption | 11px | 400 | Timestamps, counts, hints |
| Label | 10px | 500 | UPPERCASE section headers (letter-spacing 0.06em) |
| Stat | 28px | 600 | Dashboard stat numbers |
| Font | SF Pro / Inter | — | System font stack |

### Component specs

**Cards:**
- Background: #FFFFFF
- Border: 1px solid rgba(0,0,0,0.07)
- Border-radius: 10px
- Padding: 16px
- Shadow: none (flat)

**Buttons — Primary:**
- Background: #007AFF
- Text: white, 13px weight 500
- Height: 36px
- Border-radius: 8px

**Buttons — Secondary:**
- Background: #FFFFFF
- Border: 1px solid rgba(0,0,0,0.15)
- Text: #1C1C1E, 13px
- Height: 36px
- Border-radius: 8px

**Buttons — Destructive text:**
- Background: none
- Text: #FF3B30, 13px
- No border

**Sidebar:**
- Width: 220px
- Background: #EBEBEB
- Active item: rgba(0,122,255,0.12) bg + #007AFF text + icon
- Item height: 36px
- Item border-radius: 6px
- Section labels: 10px uppercase rgba(0,0,0,0.40)

**Overlay pill:**
- Width: 280px
- Height: 52px (listening/processing) / 72px (transcribed) / 72px (long recording)
- Border-radius: 999px
- Background: rgba(255,255,255,0.95)
- Border: 1px solid rgba(0,0,0,0.10)
- Shadow: 0 4px 24px rgba(0,0,0,0.15)
- Padding: 12px 16px

**Wizard modal:**
- Width: 480px
- Background: #FFFFFF
- Border-radius: 12px
- Padding: 28px
- Shadow: 0 8px 40px rgba(0,0,0,0.15)
- Background behind modal: flat #E8E8E8

---

## 9. Screen Inventory

Complete list of all 29 screens in the application, grouped by feature area.
Screen names match the Stitch design project exactly.

### Dashboard (2 screens)
| # | Screen | Description |
|---|---|---|
| 1 | Refined Sidebar Layout | 4 stat cards + sessions + Ollama status + last dictation |
| 2 | Empty State Final Refined | Stat cards with "—" + empty last dictation card |

### Onboarding (10 screens)
| # | Screen | Description |
|---|---|---|
| 3 | Welcome | App intro, Get Started CTA, Skip setup link |
| 4 | Model Selection | 4 options: Download Qwen, Use Ollama, BYOK, Whisper Only |
| 5 | Ollama Detected | Model picker dropdown, 3 available models listed |
| 6 | Ollama Not Detected | 3-step fix guide, Go Back + Retry buttons |
| 7 | Whisper Warning | What's missing list (×4 items), Go Back + Continue Anyway |
| 8 | BYOK Setup | Provider dropdown, API key input, Test Connection, success state |
| 9 | Model Download | Progress bar, speed, time remaining, Cancel option |
| 10 | Model Download Complete | Success state, model info, Continue |
| 11 | Hotkey Setup | Key cap display card, Back + Continue |
| 12 | Ready | Green checkmark, key badges, Start Yapping |

### Dictionary (5 screens)
| # | Screen | Description |
|---|---|---|
| 13 | Corrections Refined Layout | Table: raw→corrected, count, delete. Manual add. Export JSON |
| 14 | Add Correction Active | Inline add-correction form in active/focused state |
| 15 | Corrections Empty State | Centered empty state + Start Training button |
| 16 | Training Refined Final | Paragraph display, Start Recording, progress, Prev/Next |
| 17 | Voice Profile Ready | Green check, corrections learned count, Done |

### Models (3 screens)
| # | Screen | Description |
|---|---|---|
| 18 | Local State Refined Final | Whisper dropdown + LLM segmented (Local selected) + status |
| 19 | BYOK State Refined Final | BYOK selected + provider + API key + test |
| 20 | Ollama State Refined Final | Ollama selected + model picker + connection status |

### Hotkeys (1 screen)
| # | Screen | Description |
|---|---|---|
| 21 | Uniform Native Shortcuts | Action table with key cap dropdowns, Reset to Defaults |

### History (2 screens)
| # | Screen | Description |
|---|---|---|
| 22 | History Refined Final | Card list, 20/page, copy + delete per card, Load More, Clear All |
| 23 | History Empty State | Centered empty state with clock icon + Start Dictating button |

### Floating Overlay (6 screens)
| # | Screen | Description |
|---|---|---|
| 24 | Listening 1 | Blue waveform bars + "Listening..." |
| 25 | Stopping Soon 1B | Red waveform + red countdown (last 15s) + red depleting bar |
| 26 | Processing 2 | Spinner + "X.Xs" timer + "Processing..." |
| 27 | Long Recording 2B | Spinner + timer + "2 min recording" second line |
| 28 | Transcribed Short 3 | Text preview + Copy + blue auto-inject progress bar |
| 29 | Transcribed Long 3 | Long text preview with scroll + Copy + auto-inject bar |

---

## 10. Feature Specifications

### F-001: Hold-to-Record

**Description:** User holds a configurable hotkey. Recording begins. User releases. Transcription starts.

**Acceptance Criteria:**
```gherkin
Scenario: Basic hold-to-record flow
  Given LocalYapper is running in system tray
  And the user has completed onboarding
  When the user holds Alt+Space (default hotkey)
  Then the overlay pill appears within 100ms
  And the pill shows the listening state (🗣️ + waveform)
  And audio capture begins immediately
  And a 0.5 second pre-roll buffer is included

Scenario: User releases hotkey
  Given the user is recording
  When the user releases the hotkey
  Then audio capture stops
  And the overlay transitions to processing state
  And the countdown timer begins
  And the pipeline runs (correction → LLM → inject)

Scenario: Recording exceeds 120 seconds
  Given the user has been recording for 105 seconds
  Then the overlay enters "stopping soon" state
  And the waveform bars turn red
  And a red countdown number appears (15, 14, 13...)
  And a red depleting progress bar appears at pill bottom
  When the recording reaches 120 seconds
  Then recording stops automatically
  And the pipeline begins processing
```

### F-002: Double-tap hands-free mode

**Description:** User double-taps the hotkey within 300ms. Recording toggles on. Records until double-tapped again or 120s limit reached.

**Acceptance Criteria:**
```gherkin
Scenario: Double-tap to start hands-free
  Given LocalYapper is idle
  When the user taps the hotkey twice within 300ms
  Then hands-free recording mode activates
  And the overlay shows listening state
  And recording continues without holding the key

Scenario: Double-tap to stop hands-free
  Given hands-free recording is active
  When the user taps the hotkey once
  Then recording stops
  And the pipeline begins processing

Scenario: Hands-free hits 120s limit
  Given hands-free recording is active
  When 120 seconds have elapsed
  Then the stopping-soon state activates at 105s (15s warning)
  And recording auto-stops at 120s
  And the pipeline begins processing
```

### F-003: Overlay countdown timer

**Description:** During processing, an estimated countdown timer is shown. This eliminates the "is it frozen?" feeling, especially for long recordings.

**Acceptance Criteria:**
```gherkin
Scenario: Short recording processing
  Given a recording of under 30 seconds was made
  When processing begins
  Then the overlay shows a single countdown (e.g. "2.4s")
  And the countdown decrements in real time
  And a maximum safety cap of 15 seconds is enforced

Scenario: Long recording processing
  Given a recording of over 30 seconds was made
  When processing begins
  Then the overlay shows the timer (e.g. "8.4s")
  And below the timer shows "2 min recording"
  And the pill height increases to 72px to accommodate

Scenario: Processing completes
  Given processing is running
  When the final text is ready
  Then the overlay transitions to transcribed state
  And the text is shown in the pill
  And auto-inject begins (10s countdown progress bar)
```

### F-004: Text injection

**Description:** Cleaned text is injected into the currently focused application using clipboard paste simulation.

**Acceptance Criteria:**
```gherkin
Scenario: Standard text injection
  Given the user has a text field focused in any app
  When dictation processing completes
  Then the current clipboard content is saved
  And the dictated text is set to clipboard
  And Cmd+V (Mac) or Ctrl+V (Win/Linux) is simulated
  And the original clipboard content is restored after 80ms
  And the text appears in the focused app

Scenario: Auto-inject after overlay timer
  Given the transcribed text is showing in the overlay
  When the 10-second auto-inject timer completes
  Then the text is automatically injected
  And the overlay dismisses

Scenario: Hold Shift to auto-send
  Given text has been injected into an app like Slack or WhatsApp
  When the user holds Shift during the hotkey release
  Then Enter is simulated after injection
  And the message is sent automatically

Scenario: Paste Last shortcut
  Given the user has previously dictated text
  When the user presses Alt+Shift+V (default)
  Then the last dictated text is injected again
  Without re-recording
```

### F-005: Self-improving correction engine

**Description:** The app learns from every user correction and applies them automatically in future dictations.

**Acceptance Criteria:**
```gherkin
Scenario: Learning from a correction
  Given Whisper transcribed "helli" 
  And the injected text shows "helli" in the overlay
  When the user edits it to "hello" after injection
  Then LocalYapper detects the word-level diff
  And writes ("helli" → "hello", count=1) to corrections table
  And confidence is calculated as count/total_occurrences

Scenario: Applying a learned correction
  Given the corrections table has ("helli" → "hello", count=1)
  When Whisper next outputs text containing "helli"
  Then correction/engine.rs substitutes "hello" before LLM
  And count increments to 2

Scenario: Bypassing LLM for high-confidence corrections
  Given a correction has count ≥ 3 AND confidence ≥ 0.6
  When that specific word appears in Whisper output
  Then the correction is applied
  And the LLM call is skipped for that word
  And latency is sub-5ms for that substitution

Scenario: Confidence decay
  Given a correction has not been triggered for 30 days
  Then its confidence score halves automatically
  And if confidence drops below 0.2, the entry is soft-deleted
```

### F-006: First-launch wizard

**Description:** On first launch, a 6-step wizard guides the user through model selection, setup, and hotkey configuration.

**Wizard flow:**
```
Step 1: Welcome
Step 2: Model selection (4 options)
  → Option A: Download Qwen → Step 3A (downloading) → Step 4A (complete)
  → Option B: Use Ollama → Step 3B (detected/not detected)
  → Option C: BYOK → Step 3C (API key setup)
  → Option D: Whisper Only → Step 3D (warning screen)
Step 5: Hotkey setup
Step 6: You're all set
```

**Acceptance Criteria:**
```gherkin
Scenario: Download Qwen model path
  Given the user selects "Download Qwen 2.5" on model selection
  When they click Continue
  Then the download begins automatically
  And a progress bar shows percent, MB downloaded, speed, time remaining
  And download is resumable if interrupted (HTTP Range headers)
  And the model is saved to {APP_DATA}/models/
  And on completion the wizard advances to hotkey setup

Scenario: Ollama detected path
  Given the user selects "Use Ollama"
  And Ollama is running at localhost:11434
  When they click Continue
  Then available models are fetched and listed
  And the user picks one from a dropdown
  And the selected model is saved to settings

Scenario: Ollama not detected path
  Given the user selects "Use Ollama"
  And Ollama is NOT running
  When the detection screen loads
  Then 3 fix steps are shown (install, download model, retry)
  And a Retry button re-pings localhost:11434
  And Go Back returns to model selection

Scenario: BYOK path
  Given the user selects "API Key (BYOK)"
  When they enter a valid API key and click Test Connection
  Then the key is validated against the selected provider
  And "Connected · Xms" is shown on success
  And the wizard advances on Continue

Scenario: Whisper Only path
  Given the user selects "Whisper Only"
  When they click Continue
  Then a warning screen lists 4 missing features
  And "Continue Anyway" saves whisper_only=true to settings
  And the wizard advances to hotkey setup

Scenario: Skip setup
  Given a user has previously completed setup
  When they click "Already configured? Skip setup"
  Then the wizard is dismissed
  And the main app window opens
```

### F-007: Personal dictionary — training session

**Description:** User reads 15 built-in paragraphs aloud. The app compares Whisper output to the original text and auto-populates the corrections table.

**Acceptance Criteria:**
```gherkin
Scenario: Starting training
  Given the user opens Dictionary → Training tab
  When they click "Start Recording" on paragraph 1
  Then recording begins (same pipeline as normal dictation)
  And the paragraph text is shown above for reading

Scenario: Evaluating a training paragraph
  Given the user finishes reading a paragraph
  When they release the hotkey
  Then Whisper transcribes the audio
  And a word-level diff is computed against the original paragraph
  And every mismatch is written to the corrections table
  And progress advances to the next paragraph

Scenario: Training completion
  Given the user has completed all 15 paragraphs
  Then the training complete screen is shown
  And the total corrections learned is displayed
  And "Done" returns to the Dictionary corrections tab
```

### F-008: Custom AI modes

**Description:** Users can create named AI mode presets with custom system prompts. 5 built-in modes ship with the app.

**Built-in modes:**
| Mode | Skip LLM | Behavior |
|---|---|---|
| Casual | No | Remove fillers, casual punctuation, preserve abbreviations |
| Formal | No | Full sentences, professional tone, proper grammar |
| Code | No | Preserve technical terms, variable names, minimal cleanup |
| Brain dump | Yes | Raw Whisper output only — maximum speed |
| Translate → EN | No | Detect language, translate everything to English |

**Acceptance Criteria:**
```gherkin
Scenario: Active mode is applied to pipeline
  Given the user has "Formal" mode selected
  When dictation processing runs
  Then the LLM system prompt includes the formal mode instructions
  And output is formatted as full professional sentences

Scenario: Brain dump skips LLM
  Given the user has "Brain dump" mode selected
  When dictation processing runs
  Then the LLM step is skipped entirely
  And raw Whisper output (after correction engine) is injected
  And latency is reduced by ~1-2 seconds

Scenario: Creating a custom mode
  Given the user opens Dictionary (future: Modes page)
  When they create a new mode with a custom system prompt
  Then it appears in the modes list
  And can be selected as the active mode
```

### F-009: BYOK (Bring Your Own API Key)

**Description:** Users who want cloud-quality LLM cleanup can use their own OpenAI, Anthropic, or Groq API key instead of the local model.

**Acceptance Criteria:**
```gherkin
Scenario: BYOK replaces local LLM
  Given the user has configured a valid BYOK API key
  When dictation processing runs
  Then the HTTP call goes to the selected provider's API
  And the local llama.cpp model is NOT used
  And the same system prompt is sent to the cloud API

Scenario: BYOK fails gracefully
  Given BYOK is configured
  When the API call fails (network, invalid key, rate limit)
  Then LocalYapper falls back to local LLM automatically
  And a notification is shown explaining the fallback

Scenario: Test connection
  Given the user enters an API key in Models settings
  When they click "Test Connection"
  Then a minimal API call is made to verify the key
  And "Connected · Xms" is shown on success
  And a specific error message is shown on failure
```

### F-010: Transcription history

**Description:** Every dictation is stored with metadata. Users can view, copy, and delete entries.

**Acceptance Criteria:**
```gherkin
Scenario: History entry created
  Given a dictation has completed
  Then an entry is written to transcription_history
  With raw_text, final_text, app_name, mode_id, duration_ms, word_count

Scenario: Viewing history
  Given the user opens the History page
  Then entries are shown as cards in reverse chronological order
  And each card shows timestamp, app badge, word count, text preview
  And 20 entries are loaded initially
  And "Load More" loads the next 20

Scenario: Copying a history entry
  Given the user clicks the copy icon on a history card
  Then the final_text is copied to clipboard
  And a brief visual confirmation is shown

Scenario: Clearing all history
  Given the user clicks "Clear All"
  Then a confirmation dialog appears
  And on confirmation all history entries are deleted
  And the empty state is shown
```

### F-011: Hotkey remapping

**Description:** Every action in the app has a remappable hotkey stored in settings.

**Default hotkeys:**
| Action | Default (Mac) | Default (Win/Linux) |
|---|---|---|
| Record (hold) | ⌥ Space | Alt+Space |
| Hands-free (double-tap) | ⌥⌥ Space | Alt+Alt+Space |
| Cancel | Escape | Escape |
| Paste Last | ⌥⇧V | Alt+Shift+V |
| Open App | ⌥L | Alt+L |

**Acceptance Criteria:**
```gherkin
Scenario: Remapping a hotkey
  Given the user opens Hotkeys settings
  When they click the dropdown for "Record"
  And press a new key combination
  Then the new combination is saved to settings
  And the global shortcut listener updates immediately
  And the new hotkey works without restarting the app

Scenario: Reset to defaults
  Given the user has customized hotkeys
  When they click "Reset to Defaults"
  Then all hotkeys revert to their platform defaults
  And the listener updates immediately

Scenario: Conflict detection (future v0.2.0)
  Given the user sets a hotkey already in use
  Then a conflict warning is shown in red
```

### F-012: System tray

**Description:** LocalYapper lives in the system tray. Left-click opens main window. Right-click shows menu.

**Acceptance Criteria:**
```gherkin
Scenario: Tray icon states
  Given LocalYapper is running
  Then the tray icon shows:
    - Idle: normal 🗣️ icon
    - Recording: animated/pulsing icon
    - Processing: spinner icon

Scenario: Tray right-click menu
  Given the user right-clicks the tray icon
  Then a menu appears with: Open, active mode name, Pause, Quit

Scenario: Auto-start on login
  Given LocalYapper is installed
  Then it is registered to auto-start on system login by default
  And this can be disabled in General settings (future)
```

### F-013: Models page

**Description:** Users can switch Whisper model size and configure LLM settings.

**Acceptance Criteria:**
```gherkin
Scenario: Switching Whisper model
  Given the user selects "base.en" in the Whisper dropdown
  Then the new model is set in settings
  And the next dictation uses base.en
  And if base.en is not downloaded, a download prompt appears

Scenario: Switching between Local and BYOK
  Given the user is on Local (Ollama/bundled) mode
  When they click BYOK tab
  Then the BYOK configuration section appears
  And on saving a valid key, BYOK becomes active

Scenario: LLM status indicator
  Given Local mode is selected
  Then the current model name is shown
  And a green/red dot shows if the LLM engine is ready
```

### F-014: Dashboard

**Description:** Overview of usage statistics and system status.

**Stats shown:**
- Words Today
- Words This Week
- Words All Time
- Avg WPM (words per minute, calculated from session duration)
- Total Sessions
- Ollama/LLM Status (running/not running)
- Last Dictation (text preview, timestamp, app, word count)

**Acceptance Criteria:**
```gherkin
Scenario: Stats are accurate
  Given the user has made dictations
  When they open the Dashboard
  Then all stats are calculated from transcription_history
  And "Words Today" counts only entries from today (midnight to now)
  And "Avg WPM" = total_words / total_duration_minutes across all sessions

Scenario: Ollama status
  Given Local LLM mode is active
  When the Dashboard loads
  Then the LLM engine is pinged
  And green dot + model name is shown if ready
  And red dot + "Start" button shown if not ready

Scenario: Empty state
  Given no dictations have been made yet
  Then stat cards show "—" placeholders
  And the last dictation card shows the empty state
  And "Start Dictating" button is shown
```

### F-015: Update checker

**Description:** On app open, check GitHub releases API for a newer version. Show a banner if one exists.

**Acceptance Criteria:**
```gherkin
Scenario: New version available
  Given the user opens LocalYapper
  When the GitHub releases API returns a version > current
  Then a subtle banner appears in Settings with the new version
  And a "Download" link opens the GitHub releases page in browser
  And update is NEVER automatic — always manual

Scenario: No update available
  Given the current version is latest
  Then no banner is shown
  And the check happens silently in background
```

---

## 11. Data Models

### SQLite schema — all 6 tables

```sql
-- Migration 001: Initial schema
-- Run on first launch, idempotent

CREATE TABLE IF NOT EXISTS transcription_history (
  id           TEXT PRIMARY KEY,           -- UUID v4
  raw_text     TEXT NOT NULL,              -- Whisper output before correction/LLM
  final_text   TEXT NOT NULL,              -- Text after full pipeline
  app_name     TEXT,                       -- Focused app name at time of dictation
  mode_id      TEXT,                       -- FK to modes.id (nullable)
  duration_ms  INTEGER,                    -- Audio duration in milliseconds
  word_count   INTEGER,                    -- Word count of final_text
  created_at   DATETIME DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS corrections (
  id            TEXT PRIMARY KEY,          -- UUID v4
  raw_word      TEXT NOT NULL,             -- What Whisper heard (e.g. "helli")
  corrected     TEXT NOT NULL,             -- What user intended (e.g. "hello")
  count         INTEGER DEFAULT 1,         -- Times this correction has been applied
  confidence    REAL DEFAULT 0.0,          -- count / total_occurrences (0.0 to 1.0)
  last_used_at  DATETIME,                  -- Last time this correction was triggered
  created_at    DATETIME DEFAULT (datetime('now')),
  UNIQUE(raw_word, corrected)              -- Prevent duplicates
);

CREATE TABLE IF NOT EXISTS personal_dictionary (
  id        TEXT PRIMARY KEY,              -- UUID v4
  word      TEXT NOT NULL UNIQUE,          -- Custom vocabulary word
  count     INTEGER DEFAULT 1,            -- Times seen/used
  added_at  DATETIME DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS modes (
  id             TEXT PRIMARY KEY,         -- UUID v4 or "builtin_casual" etc.
  name           TEXT NOT NULL,            -- Display name
  system_prompt  TEXT NOT NULL,            -- Full system prompt for LLM
  skip_llm       INTEGER DEFAULT 0,        -- 1 = bypass LLM entirely (brain dump)
  is_builtin     INTEGER DEFAULT 0,        -- 1 = cannot be deleted
  color          TEXT DEFAULT 'purple',    -- UI accent color
  created_at     DATETIME DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS app_profiles (
  id        TEXT PRIMARY KEY,              -- UUID v4
  app_name  TEXT NOT NULL UNIQUE,          -- e.g. "Code", "Slack", "Mail"
  mode_id   TEXT NOT NULL,                 -- FK to modes.id
  FOREIGN KEY (mode_id) REFERENCES modes(id)
);

CREATE TABLE IF NOT EXISTS settings (
  key         TEXT PRIMARY KEY,            -- Setting key (see below)
  value       TEXT NOT NULL,              -- Setting value (always stored as string)
  updated_at  DATETIME DEFAULT (datetime('now'))
);
```

### Default settings seeds

```sql
INSERT OR IGNORE INTO settings VALUES ('hotkey_record',           'Alt+Space',      datetime('now'));
INSERT OR IGNORE INTO settings VALUES ('hotkey_hands_free',       'Alt+Alt+Space',  datetime('now'));
INSERT OR IGNORE INTO settings VALUES ('hotkey_cancel',           'Escape',         datetime('now'));
INSERT OR IGNORE INTO settings VALUES ('hotkey_paste_last',       'Alt+Shift+V',    datetime('now'));
INSERT OR IGNORE INTO settings VALUES ('hotkey_open_app',         'Alt+L',          datetime('now'));
INSERT OR IGNORE INTO settings VALUES ('whisper_model',           'base.en',        datetime('now'));
INSERT OR IGNORE INTO settings VALUES ('llm_mode',                'local',          datetime('now')); -- local | byok | whisper_only
INSERT OR IGNORE INTO settings VALUES ('ollama_model',            'qwen2.5:0.5b',   datetime('now'));
INSERT OR IGNORE INTO settings VALUES ('byok_provider',           'openai',         datetime('now')); -- openai | anthropic | groq
INSERT OR IGNORE INTO settings VALUES ('byok_api_key',            '',               datetime('now')); -- encrypted at rest
INSERT OR IGNORE INTO settings VALUES ('active_mode_id',          'builtin_casual', datetime('now'));
INSERT OR IGNORE INTO settings VALUES ('auto_start',              'true',           datetime('now'));
INSERT OR IGNORE INTO settings VALUES ('sound_effects',           'true',           datetime('now'));
INSERT OR IGNORE INTO settings VALUES ('mute_media',              'true',           datetime('now'));
INSERT OR IGNORE INTO settings VALUES ('confidence_threshold',    '0.6',            datetime('now'));
INSERT OR IGNORE INTO settings VALUES ('correction_decay_days',   '30',             datetime('now'));
INSERT OR IGNORE INTO settings VALUES ('language',                'en',             datetime('now'));
INSERT OR IGNORE INTO settings VALUES ('overlay_x',               '100',            datetime('now'));
INSERT OR IGNORE INTO settings VALUES ('overlay_y',               '100',            datetime('now'));
INSERT OR IGNORE INTO settings VALUES ('setup_complete',          'false',          datetime('now'));
INSERT OR IGNORE INTO settings VALUES ('model_path',              '',               datetime('now')); -- path to qwen gguf
INSERT OR IGNORE INTO settings VALUES ('max_recording_seconds',   '120',            datetime('now'));
INSERT OR IGNORE INTO settings VALUES ('auto_inject_delay_ms',    '10000',          datetime('now'));
```

### Default modes seeds

```sql
INSERT OR IGNORE INTO modes VALUES (
  'builtin_casual', 'Casual',
  'You are a voice dictation cleanup assistant. Remove filler words (um, uh, like, you know, basically, literally). Fix punctuation and capitalization. Preserve abbreviations and casual tone. Keep it conversational. Output ONLY the cleaned text, nothing else.',
  0, 1, 'blue', datetime('now')
);

INSERT OR IGNORE INTO modes VALUES (
  'builtin_formal', 'Formal',
  'You are a voice dictation cleanup assistant. Remove filler words. Write in complete, professional sentences. Fix grammar. Ensure proper punctuation and capitalization. Maintain a formal, professional tone throughout. Output ONLY the cleaned text, nothing else.',
  0, 1, 'purple', datetime('now')
);

INSERT OR IGNORE INTO modes VALUES (
  'builtin_code', 'Code',
  'You are a voice dictation cleanup assistant for a developer. Preserve all technical terms, variable names, function names, and programming concepts exactly as spoken. Minimal cleanup only — fix obvious pronunciation errors but never change technical vocabulary. Output ONLY the cleaned text, nothing else.',
  0, 1, 'green', datetime('now')
);

INSERT OR IGNORE INTO modes VALUES (
  'builtin_braindump', 'Brain dump',
  '', -- system prompt unused
  1, 1, 'gray', datetime('now') -- skip_llm = 1
);

INSERT OR IGNORE INTO modes VALUES (
  'builtin_translate', 'Translate → EN',
  'You are a voice dictation assistant. The user may speak in any language. Detect the language and translate the content to English. Clean up the translation — remove fillers, fix grammar. Output ONLY the English translation, nothing else.',
  0, 1, 'orange', datetime('now')
);
```

---

## 12. IPC Command Reference

All Tauri commands exposed to the React frontend via `invoke()`.

### Recording & pipeline

```rust
/// Start audio capture (begins pre-roll buffer)
#[tauri::command]
async fn start_recording(state: State<'_, AppState>) -> Result<(), String>

/// Stop capture, run full pipeline, return result
#[tauri::command]  
async fn stop_recording(
  state: State<'_, AppState>,
  app_handle: AppHandle
) -> Result<PipelineResult, String>

/// Run pipeline on provided audio data
#[tauri::command]
async fn run_pipeline(
  audio: Vec<f32>,
  state: State<'_, AppState>
) -> Result<PipelineResult, String>

/// Inject text into focused application
#[tauri::command]
async fn inject_text(
  text: String,
  hold_shift: bool
) -> Result<(), String>

/// Re-inject the last dictated text
#[tauri::command]
async fn paste_last(state: State<'_, AppState>) -> Result<(), String>

/// Cancel ongoing recording/processing
#[tauri::command]
async fn cancel_recording(state: State<'_, AppState>) -> Result<(), String>
```

### Model management

```rust
/// Check if Ollama is running and return available models
#[tauri::command]
async fn check_ollama() -> Result<OllamaStatus, String>
// OllamaStatus: { running: bool, models: Vec<String> }

/// Begin downloading the bundled LLM model
/// Emits "model_download_progress" events with DownloadProgress
#[tauri::command]
async fn download_model(app_handle: AppHandle) -> Result<(), String>
// DownloadProgress: { percent: f64, downloaded_mb: u64, total_mb: u64, speed_mbps: f64 }

/// Cancel an in-progress model download
#[tauri::command]
async fn cancel_model_download(state: State<'_, AppState>) -> Result<(), String>

/// Get list of available Ollama models
#[tauri::command]
async fn get_ollama_models() -> Result<Vec<String>, String>

/// Test BYOK API key connection
#[tauri::command]
async fn test_byok_connection(
  provider: String,  // "openai" | "anthropic" | "groq"
  api_key: String
) -> Result<ConnectionResult, String>
// ConnectionResult: { success: bool, latency_ms: u64, error: Option<String> }
```

### Modes

```rust
#[tauri::command] async fn get_modes() -> Result<Vec<Mode>, String>
#[tauri::command] async fn create_mode(mode: NewMode) -> Result<Mode, String>
#[tauri::command] async fn update_mode(mode: Mode) -> Result<(), String>
#[tauri::command] async fn delete_mode(id: String) -> Result<(), String>
#[tauri::command] async fn set_active_mode(id: String) -> Result<(), String>
#[tauri::command] async fn get_active_mode() -> Result<Mode, String>
```

### Corrections & dictionary

```rust
#[tauri::command] async fn get_corrections(
  limit: i64,
  offset: i64
) -> Result<Vec<Correction>, String>

#[tauri::command] async fn add_correction(
  raw_word: String,
  corrected: String
) -> Result<Correction, String>

#[tauri::command] async fn delete_correction(id: String) -> Result<(), String>
#[tauri::command] async fn export_dictionary() -> Result<String, String>  // JSON string
#[tauri::command] async fn import_dictionary(json: String) -> Result<ImportResult, String>
```

### History

```rust
#[tauri::command] async fn get_history(
  limit: i64,
  offset: i64
) -> Result<Vec<HistoryEntry>, String>

#[tauri::command] async fn delete_history_entry(id: String) -> Result<(), String>
#[tauri::command] async fn clear_history() -> Result<(), String>
#[tauri::command] async fn get_stats() -> Result<Stats, String>
// Stats: { words_today, words_week, words_all_time, avg_wpm, total_sessions }
```

### Settings

```rust
#[tauri::command] async fn get_setting(key: String) -> Result<String, String>
#[tauri::command] async fn set_setting(key: String, value: String) -> Result<(), String>
#[tauri::command] async fn get_all_settings() -> Result<HashMap<String, String>, String>
```

### System

```rust
#[tauri::command] async fn get_focused_app() -> Result<String, String>
#[tauri::command] async fn check_update() -> Result<Option<String>, String>
#[tauri::command] async fn check_permissions() -> Result<PermissionsStatus, String>
#[tauri::command] async fn open_accessibility_settings() -> Result<(), String>
#[tauri::command] async fn open_mic_settings() -> Result<(), String>
```

---

## 13. Implementation Phases

Dependencies must be respected. Each phase must compile and run before the next begins. One Claude Code session = one phase.

### Phase 1 — Foundation (no UI)
**Goal:** App launches, database initializes, no crashes.
- Create Tauri project with exact Cargo.toml and package.json from this PRD
- Implement `db/schema.rs` — all 6 tables, all seeds, idempotent migration
- Implement `db/queries.rs` — typed query functions for all tables
- Implement basic `commands.rs` stubs for all IPC commands
- Register all commands in `generate_handler![]`
- Configure two windows in `tauri.conf.json` (main + overlay)
- **Test:** `cargo tauri dev` launches without errors, SQLite file is created

### Phase 2 — Audio capture
**Goal:** Audio is captured when triggered.
- Implement `audio/vad.rs` — energy-based silence detection (threshold configurable)
- Implement `audio/capture.rs` — cpal 16kHz mono, 0.5s pre-roll ring buffer
- Expose `start_recording()` and `stop_recording()` commands
- **Test:** Call start/stop from Tauri devtools, verify audio buffer is non-empty

### Phase 3 — Speech to text
**Goal:** Audio is transcribed to raw text.
- Implement `stt/whisper.rs` — whisper-rs wrapper loading ggml-base.en.bin from app data
- Model loaded once at startup, reused for all transcriptions
- Run transcription on a blocking thread (not async — whisper-rs is sync)
- Emit `transcription_complete` event to frontend when done
- **Test:** Record 5 seconds, verify raw_text is returned

### Phase 4 — Text injection
**Goal:** Text appears in focused application.
- Implement `injection/platform.rs` — OS detection, X11 vs Wayland check
- Implement `injection/injector.rs` — clipboard save/set/paste/restore flow
- Hold Shift variant for auto-send
- Store last injected text for Paste Last feature
- **Test:** `inject_text("hello world", false)` → verify text appears in Notepad/gedit

### Phase 5 — Correction engine
**Goal:** Personal dictionary is applied before LLM.
- Implement `correction/engine.rs` — load corrections from DB, apply exact-match substitution
- Sub-5ms performance requirement (pre-load corrections at startup, refresh on change)
- **Test:** Add ("helli" → "hello") to corrections table, verify substitution works

### Phase 6 — LLM integration
**Goal:** Text is cleaned up by local LLM.
- Implement `llm/engine.rs` — llama-cpp-rs wrapper, load model from app_data/models/
- Implement `llm/prompt.rs` — system prompt builder, reads active mode, detects app context
- Implement `context/detector.rs` — focused window name per OS
- Graceful fallback if model not loaded (pass through without LLM)
- **Test:** Dictate text with fillers, verify they are removed

### Phase 7 — Full pipeline wire-up
**Goal:** Hold key → speak → release → text appears in focused app.
- Wire all phases: hotkey → capture → VAD → whisper → correction → LLM → inject
- Implement `hotkey/manager.rs` — global shortcut (hold + release + double-tap)
- Emit events to frontend for overlay state changes
- Implement `correction/learner.rs` — diff computation, DB writes, confidence calc
- **Test:** Complete end-to-end dictation works without UI

### Phase 8 — Overlay UI
**Goal:** Floating pill appears with correct states.
- Build `Overlay.tsx` window (transparent, always-on-top, pill shape)
- Build `YappingEmoji.tsx` — animated 🗣️ component
- Build `Waveform.tsx` — 5 animated bars synced to audio level events
- Build `CountdownTimer.tsx` — decrementing timer with 15s max cap
- Implement all 5 overlay states (listening, stopping-soon, processing, long-recording, transcribed)
- Wire to Tauri events from pipeline
- **Test:** Complete dictation with overlay visible and all state transitions working

### Phase 9 — Settings window shell
**Goal:** Main window opens with sidebar navigation.
- Build `Main.tsx` — settings window with sidebar
- Build sidebar with 5 nav items (Dashboard, History, Dictionary, Hotkeys, Models)
- Set up React Router or state-based page switching
- Build `stores/appStore.ts` — all Jotai atoms
- Build `lib/tauri.ts` — all typed invoke() wrappers
- **Test:** Can navigate between all 5 pages without errors

### Phase 10 — Dashboard page
**Goal:** Real stats are shown.
- Build `Dashboard.tsx` with all components
- Wire to `get_stats()` and `get_history(limit=1)` commands
- Real-time Ollama status check on page load
- Empty state for first-time users
- **Test:** Stats update after making a dictation

### Phase 11 — History page
**Goal:** History is viewable and manageable.
- Build `History.tsx` — card list, load more (20 per page)
- Wire to `get_history()`, `delete_history_entry()`, `clear_history()`
- Copy button copies final_text to clipboard
- Empty state
- **Test:** History cards appear after dictations, delete works

### Phase 12 — Dictionary pages
**Goal:** Corrections are visible and manageable.
- Build `Dictionary.tsx` with Corrections and Training tabs
- Corrections tab: table wired to DB, manual add, delete, export JSON
- Training tab: paragraph display, recording flow, progress
- Training complete screen
- Corrections empty state
- **Test:** Training session adds corrections to table

### Phase 13 — Hotkeys page
**Goal:** All hotkeys are remappable.
- Build `Hotkeys.tsx` — action table with dropdowns
- Build `HotkeyPicker.tsx` — captures keypress, formats as badge string
- Wire to `set_setting()`, `hotkey/manager.rs` reloads on change
- Reset to Defaults button
- **Test:** Changing hotkey works immediately without restart

### Phase 14 — Models page
**Goal:** Model configuration is manageable.
- Build `Models.tsx` — Whisper dropdown + LLM segmented control
- Local state: model name + status indicator + start LLM button
- BYOK state: provider dropdown + API key + test connection
- Wire test connection to `test_byok_connection()` command
- **Test:** BYOK test returns latency for valid key, error for invalid

### Phase 15 — First-launch wizard
**Goal:** New users are onboarded successfully.
- Build all 10 wizard screens
- Implement wizard state machine (which step, which path)
- Implement `downloader.rs` — streamed model download with progress events
- Wire Ollama detection to `check_ollama()` command
- Mark `setup_complete=true` on finish
- **Test:** All 4 paths complete successfully on a clean install

### Phase 16 — System tray + autostart
**Goal:** App lives in tray, starts with system.
- Implement `tray/manager.rs` — tray icon, 3 states, right-click menu
- Configure `tauri-plugin-autostart` — enabled by default
- Tray menu: Open, active mode name, Pause, Quit
- **Test:** App minimizes to tray, auto-starts after reboot

### Phase 17 — Cross-platform polish
**Goal:** All features work on all three platforms.
- Test and fix text injection on Windows (Ctrl+V), macOS (Cmd+V), Linux X11 (xclip), Linux Wayland (wl-clipboard)
- macOS: graceful Accessibility permission prompt with guide UI
- Windows: graceful mic permission via OS dialog
- Linux: detect X11 vs Wayland at runtime, check for xclip/wl-clipboard, guide if missing
- **Test:** Full dictation on all three platforms

### Phase 18 — GitHub release
**Goal:** Downloadable binaries on GitHub.
- Write `.github/workflows/release.yml` using tauri-action
- Build targets: macos-universal, windows-x64, linux-x64
- Upload .dmg, .exe/.msi, .deb/.AppImage as release assets
- Write README.md with install instructions
- Tag v0.1.0
- **Test:** Download fresh binary on each OS, complete onboarding

---

## 14. Non-Functional Requirements

### Performance

| Metric | Target | Hard Limit |
|---|---|---|
| Overlay appears on keypress | < 100ms | 200ms |
| Whisper transcription (30s audio) | < 3s on modern CPU | 8s |
| Whisper transcription (120s audio) | < 12s on modern CPU | 20s |
| LLM cleanup (500 tokens) | < 2s on modern CPU | 5s |
| Correction engine pre-LLM pass | < 5ms | 15ms |
| Text injection after processing | < 10ms | 50ms |
| SQLite query (corrections load) | < 20ms | 50ms |
| App startup to tray | < 3s | 5s |
| RAM usage at idle | < 50MB | 100MB |
| RAM during active recording | < 150MB | 250MB |

### Reliability

- App must not crash during audio capture even if mic is unplugged mid-recording
- App must recover gracefully from LLM model not loaded (pass-through mode)
- App must handle clipboard operations atomically (save → set → paste → restore never leaves clipboard in wrong state)
- Model download must be fully resumable (HTTP Range headers)
- SQLite writes must be transactional — no partial writes

### Offline

- 100% of core features work with zero internet connection after initial setup
- BYOK is the only feature that requires internet (by definition)
- Model download requires internet (one time only)
- Update check fails silently if no internet — never blocks usage

### Accessibility

- All overlay states visible for minimum 10 seconds (configurable)
- Sound effects on recording start/stop (configurable, on by default)
- High contrast between overlay pill and typical desktop backgrounds

---

## 15. Security Requirements

### Data privacy
- No audio data is ever sent to any server unless BYOK is explicitly configured
- Audio is never written to disk — kept in RAM only during processing
- BYOK API keys are stored encrypted in SQLite using OS keychain integration
- API keys are never logged
- SQLite database stores only text (no audio files)
- No analytics, no telemetry, no crash reporting (unless opt-in added in future)

### Tauri security configuration
```json
// tauri.conf.json capability — principle of least privilege
{
  "csp": "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'",
  "permissions": [
    "core:default",
    "global-shortcut:allow-register",
    "global-shortcut:allow-unregister",
    "fs:allow-read-dir",
    "fs:allow-read-file",
    "fs:allow-write-file",
    "fs:allow-create-dir",
    "autostart:allow-enable",
    "autostart:allow-disable",
    "shell:allow-open"
  ]
}
```

### Rust code requirements
- `#![forbid(unsafe_code)]` on all crates that don't require unsafe
- Every `unsafe` block must have `// SAFETY:` comment
- No `unwrap()` in production code — use `?` operator or explicit error handling
- No hard-coded credentials or model URLs in source code (use constants file)
- Clipboard operations must always restore previous content even on error

### Automated security checks (CI)
```bash
cargo audit          # Check dependencies for known vulnerabilities
cargo deny check     # License and advisory checks  
cargo clippy         # Lint with -W clippy::unwrap_used -W clippy::expect_used
npm audit            # Frontend dependency audit
```

---

## 16. Open Source & Distribution

### License
**MIT License** — completely free for any use, modification, distribution, or commercialization.

### Repository structure
```
localyapper/
├── .github/
│   └── workflows/
│       └── release.yml      ← tauri-action CI/CD
├── src-tauri/
│   ├── src/                 ← All Rust code
│   ├── resources/           ← Bundled assets (icons)
│   ├── capabilities/        ← IPC security permissions
│   ├── Cargo.toml
│   ├── build.rs
│   └── tauri.conf.json
├── src/                     ← React/TypeScript frontend
├── CLAUDE.md                ← AI agent instructions
├── DESIGN_SYSTEM.md         ← Design tokens and rules
├── docs/
│   ├── architecture.md      ← This document (abbreviated)
│   ├── functional.md        ← Feature specs
│   └── steps.md             ← Implementation sequence
├── .gitignore               ← Includes *.bin, *.gguf (model files not committed)
├── LICENSE                  ← MIT
├── README.md
├── package.json
├── vite.config.ts
├── tailwind.config.ts
└── tsconfig.json
```

### GitHub Actions release workflow
Triggers on git tag matching `v*` (e.g. `v0.1.0`):
1. Download model files from HuggingFace (not committed to repo)
2. Build on `ubuntu-latest` → produces `.deb` and `.AppImage`
3. Build on `macos-latest` → produces universal `.dmg` (x64 + ARM)
4. Build on `windows-latest` → produces `.exe` (NSIS) and `.msi`
5. Upload all 5 artifacts as GitHub Release assets

### README requirements
- Project description and feature list
- Screenshot of overlay + main window
- Installation instructions for all 3 platforms
- How to build from source (prerequisites: Rust, Node, download model files)
- How to run in dev mode
- Contributing guidelines
- MIT license badge

---

## 17. Out of Scope — v0.1.0

Features explicitly NOT in v0.1.0 that will be considered for future versions:

| Feature | Reason deferred |
|---|---|
| App Profiles (auto-switch mode per app) | Architecture ready, UI deferred |
| Command Mode (highlight + speak to transform) | Complex UX, post-MVP |
| Custom AI modes UI (create/edit modes) | Backend ready, UI deferred |
| Real-time streaming transcription | Requires different model (Parakeet) |
| File/audio transcription (.wav, .m4a input) | Not core use case |
| System audio recording | Platform complexity |
| Speaker diarization | Requires separate model |
| Mobile apps (iOS, Android) | Out of scope |
| Multi-language simultaneous | Later with better models |
| Team/collaboration features | Single-user tool |
| Usage analytics dashboard | Low priority |
| Hotkey conflict detection | Nice-to-have |
| Markdown formatting detection | Minor feature |
| Code backtick auto-wrapping | Minor feature |
| Self-correction mid-speech ("no wait, I mean X") | Needs custom model training |
| Auto-update installer | Security concerns, manual preferred |

---

## 18. Future Roadmap

### v0.2.0 — Intelligence upgrade
- App Profiles: auto-switch mode based on focused app
- Custom modes UI: full create/edit/delete interface
- Command Mode: highlight text, speak to transform it
- Hotkey conflict detection with visual warning
- Confidence threshold slider in UI
- Corrections export/import via JSON

### v0.3.0 — Performance + languages
- Chunked audio processing for recordings over 60s
- Streaming transcription with real-time word display
- Multi-language support (Hindi, Spanish, French, German, Hinglish)
- Whisper small.en model option for better accuracy
- GPU acceleration detection and use if available

### v0.4.0 — Advanced features
- File transcription (.wav, .m4a, .mp3 drag-and-drop)
- System audio transcription (meetings, videos)
- Real-time streaming via Parakeet model
- IDE-specific features (Cursor/VS Code file tagging)
- Code-aware formatting (camelCase, backtick wrapping)

### v1.0.0 — Production ready
- Complete test coverage
- Full accessibility audit
- Performance optimization for all hardware tiers
- App store distribution (Mac App Store, Microsoft Store, Flathub)
- Community plugin system for custom modes sharing

---

## 19. Success Metrics

### Launch metrics (30 days post v0.1.0)
| Metric | Target |
|---|---|
| GitHub stars | 500+ |
| Total downloads | 1,000+ |
| GitHub issues (bugs) | < 20 open |
| Platforms represented in issues | All 3 (Win/Mac/Linux) |

### Quality metrics
| Metric | Target |
|---|---|
| App crash rate | < 0.1% of sessions |
| Successful dictation rate | > 99% (no silent failures) |
| Pipeline latency p50 (30s recording) | < 4s total |
| Pipeline latency p99 (30s recording) | < 8s total |
| Memory at idle | < 50MB |

### User satisfaction signals
- Users completing full onboarding wizard: > 80%
- Users who complete voice training session: > 40%
- Users returning after day 1: > 50%
- GitHub discussions with positive feedback: present

---

## 20. Appendix

### A. System prompt templates

**Casual mode:**
```
You are a voice dictation cleanup assistant.
Remove filler words (um, uh, like, you know, basically, literally, I mean).
Fix punctuation and capitalization.
Preserve casual tone and abbreviations.
If the user self-corrects ("no wait, actually..."), output only the final intended text.
Do not add information that wasn't spoken.
Output ONLY the cleaned text. No explanation, no quotes, no prefix.
```

**Formal mode:**
```
You are a voice dictation cleanup assistant.
Remove filler words and false starts.
Write in complete, professional sentences.
Fix grammar and ensure proper punctuation.
Capitalize correctly (proper nouns, sentence starts, I).
Maintain a formal, professional tone throughout.
If the user self-corrects, output only the final intended text.
Do not add information that wasn't spoken.
Output ONLY the cleaned text. No explanation, no quotes, no prefix.
```

**Code mode:**
```
You are a voice dictation cleanup assistant for a software developer.
Preserve ALL technical terms, variable names, function names, class names, and programming concepts exactly as they sound.
Minimal cleanup only: fix obvious transcription errors but never change technical vocabulary.
Preserve camelCase, snake_case, and other naming conventions as spoken.
Do not add punctuation to code snippets.
Output ONLY the cleaned text. No explanation, no quotes, no prefix.
```

**Translate → EN mode:**
```
You are a voice dictation and translation assistant.
The user may speak in any language.
Detect the language and translate the entire content into English.
Clean up the translation: remove fillers, fix grammar, ensure natural English.
Output ONLY the English translation. No explanation, no original text, no prefix.
```

### B. Model download URL

```
https://huggingface.co/Qwen/Qwen2.5-0.5B-Instruct-GGUF/resolve/main/qwen2.5-0.5b-instruct-q4_k_m.gguf
```

File size: ~397MB  
Format: GGUF Q4_K_M quantization  
Context: 32,768 tokens  
Output: up to 8,192 tokens  

### C. Whisper model specs

**ggml-base.en.bin** (default)
- Download: `https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.en.bin`
- Size: 148MB
- Language: English only
- Speed: ~4.5s for 30s audio on modern CPU
- WER: 4.3% clean, 12.8% noisy
- Accuracy: Good balance of speed and accuracy for dictation

**ggml-tiny.en.bin** (legacy fallback)
- Download: `https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.en.bin`
- Size: 75MB
- Speed: ~1-3s for 30s audio
- Accuracy: Fast but lower quality, struggles with accents

### D. Platform text injection implementation details

**macOS:**
```rust
// 1. Save clipboard: NSPasteboard.general.string(forType: .string)
// 2. Set text: NSPasteboard.general.setString(text, forType: .string)
// 3. Simulate: CGEvent Cmd+V via CGEventCreateKeyboardEvent
// 4. Wait 80ms
// 5. Restore: NSPasteboard.general.setString(previous, forType: .string)
```

**Windows:**
```rust
// 1. Save: OpenClipboard → GetClipboardData(CF_UNICODETEXT)
// 2. Set: SetClipboardData(CF_UNICODETEXT, GlobalAlloc(text))
// 3. Simulate: SendInput with Ctrl+V key events
// 4. Wait 80ms
// 5. Restore: SetClipboardData with saved content
```

**Linux X11:**
```bash
# Save: xclip -selection clipboard -o
# Set: echo "text" | xclip -selection clipboard
# Paste: xdotool key ctrl+v
# Restore: echo "original" | xclip -selection clipboard
```

**Linux Wayland:**
```bash
# Save: wl-paste
# Set: echo "text" | wl-copy
# Paste: wtype -k ctrl+v (or ydotool)
# Restore: echo "original" | wl-copy
```

### E. Confidence score calculation

```rust
// When a correction is applied:
// new_confidence = (count as f64) / (total_occurrences_of_raw_word as f64)

// Decay (run nightly or on app open):
// days_since_use = (now - last_used_at).num_days()
// if days_since_use > decay_threshold (default 30):
//   correction.confidence *= 0.5_f64.powf(days_since_use as f64 / decay_threshold as f64)
//   if correction.confidence < 0.2:
//     soft_delete (set deleted_at = now)
```

### F. Training paragraphs (to be written)

15 paragraphs to be authored covering:
1. Common English words + punctuation
2. Technical/developer vocabulary (API, function, variable, repository)
3. Medical terminology (for medical professionals)
4. Names (proper nouns, common first names, place names)
5. Numbers and dates
6. Abbreviations (e.g., "AI", "URL", "CSS")
7. Mixed sentence lengths (short + long)
8. Questions and exclamations
9. Domain: email/business writing
10. Domain: casual conversation
11. Domain: technical documentation
12. Domain: creative writing
13. Long compound sentences
14. Uncommon words and vocabulary
15. Mixed all of the above

*Paragraph content to be authored separately — not included in this PRD.*

---

*End of document. Version 1.0.0. This is a living document — update as decisions change.*
