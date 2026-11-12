# LocalYapper Rebuild Plan

**Audience:** the implementing agent. Read this whole file before touching code.
**Written against:** commit `2115b4b` (version 1.0.0), branch `claude/optimistic-brahmagupta-lhsm71`.
**Status of this document:** it is a plan, produced by reading every source file. Nothing in it has been implemented yet.

---

## 0. How to use this plan

1. Work phase by phase, in order. Each phase ends with a gate. Do not start the next phase until the gate passes.
2. One commit per numbered task (for example `P1.3`), with the task id in the commit message.
3. After every task run the verification commands in section 13.
4. The owner has granted full freedom to change architecture, dependencies, models and UI. Where this plan conflicts with `CLAUDE.md` or `AGENTS.md`, this plan wins, and task `P7.9` updates those files to match.
5. Items marked **VERIFY** are facts that could not be confirmed while writing this plan (the planning session had no access to huggingface.co, github.com release assets or the sherpa-onnx docs site). Confirm each one before building on it. If a VERIFY item turns out false, use the stated fallback.
6. Items marked **OWNER** need a decision or a secret from the repository owner. Do not block on them: build everything around them and list them in your final report.
7. Much of this app is platform code for Windows and macOS. If you are running in a Linux container you cannot exercise it. Compile-check it through CI, keep it behind small traits, and record every piece that was written without being run in `docs/MANUAL_QA.md` so the owner can test it on real hardware. Never report untested platform code as working.

### Rules that stay

- Nothing leaves the device except model downloads and an opt-in update check.
- Audio is never written to disk.
- No `unwrap()` / `expect()` in production Rust. No `any` in TypeScript.
- Never edit `src-tauri/src/main.rs`. Only `lib.rs`.
- `rusqlite` keeps the `bundled` feature.
- No LLM, Ollama or BYOK.

### Rules that change

- "Text injection must remain clipboard save -> set -> paste -> restore" becomes: that flow is the default, and direct typing is an allowed fallback and a user setting (section 6).
- "sherpa-onnx 1.12" becomes sherpa-onnx 2.0.x (task `P0.2`).
- "Parakeet only" becomes a small verified model catalog with automatic selection (section 7).
- The overlay stops being React.

---

## 1. Product target and acceptance criteria

The owner's target, turned into things that can be measured.

| # | Criterion | How it is measured |
|---|---|---|
| A1 | Pressing the record hotkey starts audio capture within 150 ms and shows the overlay within 150 ms | Stage timings in the log (`P1.11`) |
| A2 | The overlay shows a waveform driven by the real microphone level | Manual QA, plus a unit test of the level meter |
| A3 | After the hotkey is released, text appears in the previously focused app in under 2 s at the median and under 5 s in the worst supported case (cold model, 60 s of speech, 4-core laptop CPU) | Stage timings in the log, bench harness (`P0.3`) |
| A4 | Focus never leaves the target app during dictation | Manual QA on each OS |
| A5 | The user's clipboard content is the same after dictation as before | Automated on Linux under Xvfb, manual elsewhere |
| A6 | Every setting shown in the UI changes backend behaviour immediately, and survives restart | One test per setting key (`P5.2`) |
| A7 | Idle footprint with the settings window closed and models evicted: main process under 40 MB private memory, no settings WebView alive | `scripts/measure-memory` (`P6.6`) |
| A8 | Windows installer under 25 MB, models downloaded on first run with checksum verification | CI size check (`P7.6`) |
| A9 | A second launch focuses the running instance instead of starting another | Manual QA |
| A10 | Works on Windows 10+, macOS 12+, Linux X11. Linux Wayland works with documented limits | Manual QA matrix |

A3 depends on the CPU. Treat the numbers as the budget the design is built to, and report the measured values honestly.

### Latency budget (10 s utterance, 4-core laptop, cold model)

| Stage | Budget | Design decision that makes it possible |
|---|---|---|
| Hotkey press to capture running | 150 ms | Dedicated audio thread that is already alive, stream built on press |
| Hotkey press to model ready | 1.5 s, in parallel with speech | Model load starts on **press**, not on release |
| Speech segments transcribed while the user is still talking | n/a | Streaming VAD plus incremental transcription |
| Release to last segment decoded | 0.3 to 1.5 s | Only the tail segment (15 s at most) is left to decode |
| Paste | 150 ms | Modifier-release wait, physical-key paste, clipboard restore done afterwards in the background |

Today the model loads after release, the whole recording is decoded in one pass, and the state machine then sleeps for 3 s. Fixing those three things is most of A3.

---

## 2. Findings: what is wrong today

Every item was read in the source. File references are `path:line` at commit `2115b4b`.

### 2.1 Bugs that break the core flow

| ID | Finding | Where |
|---|---|---|
| F1 | Audio stream is always built as `f32`. Devices whose default format is `i16` or `u16` (common on Linux ALSA and on some Windows drivers) fail with "Failed to build audio stream", so dictation never starts | `src-tauri/src/audio/capture.rs:114-117` |
| F2 | Resampling is linear interpolation with no low-pass filter, so 44.1/48 kHz input aliases into the 16 kHz signal and accuracy drops. The last sample of each callback is dropped and the phase is clamped, which adds a discontinuity every buffer | `capture.rs:141-159` |
| F3 | The real-time audio callback allocates two `Vec`s, takes two mutexes, and silently drops audio when `try_lock` fails | `capture.rs:124-171` |
| F4 | `cpal::Stream` is forced `Send + Sync` with `unsafe impl`. It is not safe on every backend | `capture.rs:23-30` |
| F5 | After a successful paste the state machine sleeps 3 s before returning to idle. Any hotkey press in that window is ignored | `src-tauri/src/hotkey/manager.rs:591-592` |
| F6 | Each press and release is handled in its own spawned task, so a quick tap can run the release handler before the press handler | `manager.rs:124-133` |
| F7 | The model is loaded only after recording stops, so the first dictation after start or after the 60 s idle eviction pays a full 458 MB model load before any decoding | `src-tauri/src/commands/recording.rs:52` |
| F8 | The overlay is shown from JavaScript after an event round trip and two settings reads, using `show()`. There is no `focusable: false` or non-activating style, so on Windows and macOS the overlay can take focus and the paste then goes nowhere | `src/hooks/useOverlayState.ts:62-86`, `src-tauri/tauri.conf.json:42-55` |
| F9 | Paste sends `Key::Unicode('v')`. On non-Latin or remapped keyboard layouts this is not the physical V key and the paste fails | `src-tauri/src/injection/injector.rs:50` |
| F10 | Paste fires while the user may still be holding the hotkey's modifiers, turning Ctrl+V into Ctrl+Alt+V and similar | `injector.rs:47-62` |
| F11 | Clipboard is restored 80 ms after paste. Slow apps read the clipboard after that and paste the old content. Only text is saved, so a copied image or file is lost | `injector.rs:7,31,64,75` |
| F12 | Linux X11 shells out to `xclip` and `xdotool`, Wayland to `wl-copy` and `wtype`. None are declared as package dependencies. The process spawn adds latency | `injector.rs:83-211` |
| F13 | Global shortcuts do not work on Wayland at all (`tauri-plugin-global-shortcut` is X11 only on Linux). The README claims Wayland support | `manager.rs:118` |
| F14 | `parakeet-0.6b` is listed as a model but cannot work: the download URL points at a repo with no single `model.onnx`, and the loader only builds a `nemo_ctc` config while that model is a transducer | `src-tauri/src/stt/whisper.rs:51-60,149-151` |
| F15 | A model file counts as valid when it is larger than 1 MB. A truncated download passes, then the recogniser fails to load forever | `whisper.rs:66-91` |
| F16 | macOS bundle has no `Info.plist` with `NSMicrophoneUsageDescription` and no entitlements, so microphone access is denied or the app is killed | no `src-tauri/Info.plist` |
| F17 | Silero VAD keeps only detected speech, with a 0.5 threshold and no padding. Word onsets get clipped. If VAD reports no speech, the recording is discarded even when it has clear energy | `src-tauri/src/audio/vad.rs:39-47,100-124` |
| F18 | The whole recording (up to 120 s) is decoded in one pass. Memory use of these models grows faster than linearly with length, so long dictations spike RAM and are slow | `whisper.rs:193-195` |
| F19 | The 30 s pipeline timeout does not cancel the blocking work, and it includes model load time | `manager.rs:469-497` |
| F20 | Two concurrent callers of `ensure_speech_model_loaded` can both load the model | `src-tauri/src/commands/models.rs:22-89` |

### 2.2 Efficiency problems

| ID | Finding | Where |
|---|---|---|
| E1 | The settings window is declared in config with `visible: false`, so its WebView is created on every start and stays alive until the user opens and closes it once. Idle RAM is far higher than the docs claim | `tauri.conf.json:28-40` |
| E2 | The app downloads the fp32 model (458 MB on disk and roughly that much in RAM) when an int8 build of the same model exists at about a quarter of the size (**VERIFY**) | `whisper.rs:41-50` |
| E3 | The overlay loads React, ReactDOM and Jotai (about 200 KB of JS) to draw one pill | `src/overlay-main.tsx` |
| E4 | Pipeline events go to every window with `app.emit` | `manager.rs:670` |
| E5 | Dropping the engine does not guarantee memory returns to the OS. Nothing measures it | `src-tauri/src/stt/lifecycle.rs:92-118` |
| E6 | The waveform is a CSS animation unrelated to the microphone | `src/components/overlay/Waveform.tsx` |
| E7 | `blank_penalty: 1.2` is an untested tweak that can insert tokens | `whisper.rs:160` |
| E8 | No release build size or memory measurement in CI | `.github/workflows/release.yml` |

### 2.3 Missing features and settings

| ID | Finding |
|---|---|
| M1 | No single-instance guard. Two copies fight over hotkeys and the database |
| M2 | Launch-at-login is forced on at first start with no consent and no UI to turn it off (`src-tauri/src/lib.rs:198-215`) |
| M3 | No microphone selection, no handling of device unplug during recording |
| M4 | No model choice in the UI at all. `speech_model` can only be changed by editing the database |
| M5 | No general settings page: no paste method, no clipboard options, no overlay position, no history retention, no sounds, no theme |
| M6 | `set_setting` accepts any key and any value with no validation and no side effects (`src-tauri/src/commands/settings.rs:16-24`) |
| M7 | No log file. Release builds on Windows have no console, so there is nothing to attach to a bug report |
| M8 | No updater, no code signing. macOS builds are blocked by Gatekeeper and lose the Accessibility grant on every build. Windows shows SmartScreen |
| M9 | Dashboard "today" uses UTC, so it is wrong for most users (`src-tauri/src/db/queries.rs:79`) |
| M10 | History is kept forever in plain text with no retention or opt-out |
| M11 | Overlay position is stored in physical pixels with no check that it is still on a connected monitor. Default is (100, 100) |
| M12 | Dead IPC surface: `start_recording`, `stop_recording`, `run_pipeline` (takes audio as a JSON number array), `inject_text`, `get_focused_app` are registered and never called by the frontend (`lib.rs:233-239`) |
| M13 | The cancel key (Escape by default) is grabbed globally for the whole recording, including long hands-free sessions, so it never reaches the focused app |
| M14 | Models are fetched from mutable `resolve/main` URLs with no checksum. Silero VAD is fetched from GitHub with no checksum |
| M15 | No third-party notices. The Parakeet models are CC-BY-4.0 and need attribution |
| M16 | Version is `1.0.0` in manifests while `CLAUDE.md` says "v0.1.0 release candidate". Five overlapping audit documents contradict each other |
| M17 | Types are named `WhisperEngine` and live in `whisper.rs` though nothing is Whisper |
| M18 | Rust tests cover only helpers. There is no test that pushes audio through the pipeline. The 600 lines of "manual tests" inside `injector.rs` and `capture.rs` are Windows-only and ignored |

### 2.4 What is fine and should be kept

- SQLite schema for `transcription_history` and `settings`, and the WAL pragmas.
- mimalloc as the global allocator.
- Tray menu structure and the destroy-on-close behaviour of the settings window.
- The settings UI look (see `DESIGN_SYSTEM.md`), the wizard flow, the history and dashboard pages.
- History is saved before injection, so a failed paste never loses the text.
- Hotkey update with rollback on registration failure (`src-tauri/src/commands/hotkeys.rs:86-121`).
- The release profile (`lto = "fat"`, `codegen-units = 1`, `panic = "abort"`, `strip`).

---

## 3. Target architecture

### 3.1 Process model

One Rust process. Threads and tasks:

| Unit | Kind | Owns | Lifetime |
|---|---|---|---|
| Controller | one tokio task with an `mpsc` inbox | the dictation state machine | whole app |
| Audio thread | OS thread | the `cpal` stream, a lock-free ring buffer producer | whole app, stream only while recording |
| Segmenter | OS thread | resampler, streaming VAD, segment queue | per recording |
| STT worker | OS thread | the recogniser | from model load to eviction |
| Injector | `spawn_blocking` | clipboard and key events | per injection |

Everything talks to the controller through one typed event channel. The controller is the only writer of dictation state. This removes the atomics and the task-ordering races in `hotkey/manager.rs`.

### 3.2 Module layout

```
src-tauri/src/
  lib.rs                 setup and wiring only
  error.rs
  settings/
    mod.rs               typed Settings struct, load, validate, apply, change events
    keys.rs              one const per key, default, validator
  core/                  NO tauri imports allowed in this tree
    session.rs           pure state machine: (State, Event) -> (State, Vec<Effect>)
    audio/
      ring.rs            SPSC ring buffer
      convert.rs         sample format and channel downmix
      resample.rs        band-limited resampler to 16 kHz
      level.rs           RMS and peak meter for the overlay
      segmenter.rs       streaming VAD -> speech segments
    stt/
      catalog.rs         model manifest: ids, files, sha256, sizes, family
      engine.rs          SpeechEngine trait and sherpa implementation
      manager.rs         load once, warm up, idle eviction, memory trim
    text/
      join.rs            join segment texts, spacing, trailing space
    download.rs          resumable verified downloader
  platform/
    audio_in.rs          cpal device listing and stream (audio thread)
    hotkey/              global shortcut backends
    inject/              clipboard and paste backends per OS
    overlay/             OverlayBackend trait and implementations
    focus.rs             foreground app detection
    permissions.rs
    memory.rs            working-set trim per OS
  controller.rs          runs session.rs, executes Effects
  commands/              thin IPC handlers only
  db/
  tray/
```

`core/` has unit tests that run on any machine with no audio device and no display. That is where most of the test value is.

### 3.3 The state machine

States: `Idle`, `Recording { mode: Hold | TapPending | HandsFree }`, `Finishing` (released, waiting for tail decode), `Injecting`.

Events: `RecordDown`, `RecordUp`, `CancelKey`, `MaxDurationReached`, `SegmentReady(audio)`, `SegmentText(index, text)`, `CaptureFailed(reason)`, `ModelReady`, `ModelFailed(reason)`, `InjectDone(result)`, `Pause(bool)`, `Tick`.

Effects: `StartCapture`, `StopCapture`, `DiscardCapture`, `LoadModel`, `Transcribe(index, audio)`, `ShowOverlay(view)`, `HideOverlay`, `Inject(text)`, `SaveHistory`, `ScheduleEvict`, `RegisterCancelKey`, `UnregisterCancelKey`, `PlayCue(kind)`, `SetTray(state)`.

Required behaviour:

- `Injecting -> Idle` happens the moment injection returns. The overlay preview runs on its own timer and never blocks a new recording. A `RecordDown` during the preview starts a new recording and replaces the preview.
- Three activation modes, chosen by the `activation_mode` setting: `hold` (press and hold), `toggle` (press to start, press to stop), `hybrid` (today's behaviour: hold, or double-tap for hands-free).
- A repeat `RecordDown` while already recording in hold mode is ignored (key auto-repeat).
- Timing uses `Instant`, not wall-clock time.
- `session.rs` is pure. Write table-driven tests for every transition, including the out-of-order cases (up before down, cancel during finishing, model failure during recording).

---

## 4. Audio capture and segmentation

### P1 tasks (audio half)

- **P1.1 Audio thread.** A long-lived thread owns the `cpal` stream so it is never sent across threads. Commands in: `Start { device }`, `Stop`, `Shutdown`. Delete the `unsafe impl Send/Sync for StreamHandle`.
- **P1.2 Sample formats.** Match on `SupportedStreamConfig::sample_format()` and build the stream for `F32`, `I16`, `U16`, `I32`, `U8`, `I8`, `I24` where cpal offers it. Convert to `f32` in `[-1, 1]`. Fixes F1.
- **P1.3 Callback discipline.** The callback only downmixes to mono into a stack or pre-allocated buffer and pushes into the SPSC ring. No allocation, no mutex, no logging. On overflow, count the dropped samples in an atomic and report it after the recording. Fixes F3.
- **P1.4 Resampler.** Do it on the segmenter thread. Use `rubato` (sinc or FFT fixed-in) or `sherpa_onnx::LinearResampler` (which is the Kaldi band-limited resampler despite the name, **VERIFY** in `resampler.rs` of the crate). Test with a 1 kHz sine at 48 kHz and at 44.1 kHz: output frequency within 1 Hz, level within 0.5 dB, and a 10 kHz tone at 48 kHz attenuated by at least 40 dB. Fixes F2.
- **P1.5 Level meter.** Compute RMS over 20 ms windows on the segmenter thread, smooth it, and publish at 30 Hz to the overlay only. Stop publishing when not recording.
- **P1.6 Streaming segmenter.** Feed 16 kHz audio to the VAD in 512-sample windows as it arrives. Parameters to start from: threshold 0.4, minimum silence 0.35 s, minimum speech 0.15 s, maximum segment 15 s. Add 200 ms of pre-roll and 150 ms of post-roll to every segment, taken from a rolling buffer. Emit `SegmentReady` as each segment closes. On stop, flush and emit the tail.
- **P1.7 No-speech safety net.** If the VAD produced no segment but the recording is longer than 0.4 s and its RMS is above the noise threshold, send the whole recording as one segment. Fixes the false "No speech detected" in F17.
- **P1.8 Device handling.** Setting `input_device` holds a device name or `default`. If the named device is missing, fall back to default and tell the UI. The stream error callback sends `CaptureFailed` so the overlay shows the reason instead of hanging. Add `list_input_devices` IPC.
- **P1.9 Recording length.** Remove the three separate 120 s constants. One setting `max_recording_seconds`, default 600, range 30 to 3600. Because segments are decoded and dropped as they close, memory no longer grows with recording length. Keep the warning state for the last 15 s.

Silero VAD is small (about 2 MB, **VERIFY**). Ship it inside the installer under `src-tauri/resources/` so it is always present, and delete the runtime VAD download. Keep the energy VAD only as the fallback if the bundled file fails to load.

---

## 5. Speech engine

### P1 tasks (engine half)

- **P1.10 Engine trait.** `trait SpeechEngine { fn transcribe(&self, samples: &[f32]) -> Result<String>; }` with a sherpa implementation that builds the right config per model family (`NemoCtc`, `Transducer`, `Moonshine`). Rename `WhisperEngine` to `SherpaEngine`, `stt/whisper.rs` to `core/stt/engine.rs`. Remove `blank_penalty` unless the benchmark in `P0.3` shows it helps.
- **P1.11 Manager.** One async mutex guards loading so only one load can run. API: `ensure_loaded()` (idempotent, returns when ready), `transcribe(segment)`, `evict()`. On `RecordDown` the controller calls `ensure_loaded()` immediately. After loading, run one decode of 0.5 s of silence to warm the runtime. Log stage timings for every dictation in one line: `capture_start_ms, model_ready_ms, segments, audio_ms, decode_ms_total, tail_decode_ms, inject_ms, release_to_text_ms`. Never log transcript text.
- **P1.12 Incremental transcription.** Segments are decoded in order on the STT worker as they arrive. On release only the tail remains. `core/text/join.rs` joins segment texts with single spaces, trims, and applies the `trailing_space` setting.
- **P1.13 Threads.** `num_threads = min(4, physical cores)`. Use physical cores, not logical, to avoid oversubscription on SMT CPUs.
- **P1.14 Eviction.** Setting `idle_unload_seconds`: 0 (never), 30, 120 (default), 300, 900. On eviction drop the recogniser and the VAD, then call `platform::memory::trim()`: `mi_collect(true)` everywhere, plus `SetProcessWorkingSetSize(-1, -1)` or `EmptyWorkingSet` on Windows and `malloc_trim(0)` on glibc Linux. Measure resident memory before and after and log both. If memory does not come back, find out why before moving on (E5).
- **P1.15 Cancellation.** A cancelled recording drops queued segments and ignores results from a decode already in flight. Replace the 30 s timeout with a watchdog on the decode of a single segment (30 s per 15 s segment means the machine is too slow: surface a clear error that suggests a smaller model).

### Gate for phase 1

- `cargo test` passes with no audio device.
- An integration test feeds WAV fixtures through `convert -> resample -> segmenter -> engine -> join` using a fake audio source, and checks word error rate against the reference transcript. It runs only when `LOCALYAPPER_TEST_MODEL_DIR` is set. CI sets it and caches the model.
- Fixtures: 5 to 10 short public-domain clips (LibriSpeech test-clean is CC-BY-4.0, record the attribution), including one at 44.1 kHz stereo, one 16-bit, one with a 3 s pause in the middle, and one of 90 s.

---

## 6. Hotkeys, injection, focus, permissions

### P2 tasks

- **P2.1 Hotkey backend trait.** `trait HotkeyBackend { register(id, combo), unregister(id), events() }`. Implementations:
  - `GlobalShortcutBackend`: the existing plugin, for Windows, macOS and Linux X11. Its callback only sends `RecordDown` / `RecordUp` into the controller channel, preserving order (fixes F6).
  - `PortalBackend`: Linux Wayland through the `org.freedesktop.portal.GlobalShortcuts` portal using the `ashpd` crate. Works on KDE Plasma 6, GNOME 48+ and Hyprland (**VERIFY** current support).
  - `CliBackend`: always available. `localyapper --toggle`, `--start`, `--stop`, `--cancel` are forwarded to the running instance by the single-instance plugin (`P7.1`), so any desktop can bind them to a key. This is the documented fallback where the portal is missing.
- **P2.2 Default record hotkey.** Keep `F8` on Windows and Linux. On macOS default to `Alt+Space` (Option+Space), since F8 is a media key there. The wizard's hotkey step must make the user press the key and see it detected before continuing.
- **P2.3 Cancel key.** Setting `cancel_key_enabled` (default true). Register it only while recording, and in hands-free mode only if the user enabled `cancel_key_in_hands_free` (default false) so Escape keeps working in the focused app (M13). The controller remembers exactly which combo it registered and unregisters that one.
- **P2.4 Pause.** Pausing unregisters only the record, cancel and paste-last keys. Open-app stays registered.
- **P2.5 Injection rewrite.** One `Injector` trait with per-OS backends, all in-process:
  1. Wait until all modifier keys are physically up, polling for at most 400 ms (`GetAsyncKeyState` on Windows, `CGEventSourceFlagsState` on macOS, XQueryKeymap on X11). Fixes F10.
  2. Save the clipboard: text, HTML and image where `arboard` supports them. Skip the save when `restore_clipboard` is off.
  3. Set the text. Mark it as transient so clipboard managers skip it: on Windows add the `ExcludeClipboardContentFromMonitorProcessing` and `CanIncludeInClipboardHistory` formats, on macOS the `org.nspasteboard.TransientType` type.
  4. Send paste using the **physical** V key: virtual key `0x56` on Windows, key code `9` on macOS, the keycode that maps to keysym `v` in the current X11 layout or else keycode 55. Fixes F9.
  5. Return success to the controller now. Restore the clipboard from a background thread after `clipboard_restore_delay_ms` (default 250, range 100 to 2000). Fixes F11.
- **P2.6 Paste method setting.** `paste_method`: `auto` (default), `ctrl_v`, `ctrl_shift_v`, `shift_insert`, `type` (send the text as key events with no clipboard use), `clipboard_only` (copy and notify, no key events). `auto` uses Ctrl+Shift+V when the focused app is a known terminal on Linux and Windows (list in `platform/focus.rs`), Cmd+V on macOS, Ctrl+V otherwise.
- **P2.7 Linux backends.** X11: `arboard` plus `enigo` (x11rb or libxdo), no shell-outs. Wayland: `arboard` with its `wayland-data-control` feature for the clipboard, and for keys try in order `enigo` with its Wayland/libei feature, then `wtype`, `ydotool`, `dotool` if installed. If none works, fall back to `clipboard_only` and say so in the overlay. Declare the packages in the deb `depends` / `recommends` (`P7.5`). Fixes F12.
- **P2.8 Focus detection.** Capture the foreground app (name, and window handle on Windows) on `RecordDown` before any overlay call. macOS: replace the `osascript` spawn with `NSWorkspace.frontmostApplication` through `objc2`. On Windows, if the target window belongs to an elevated process and LocalYapper is not elevated, input is blocked by the OS: detect it and show "Cannot type into an administrator window" with the copy button.
- **P2.9 Permissions.** macOS: add `src-tauri/Info.plist` with `NSMicrophoneUsageDescription`, add `entitlements.plist` with `com.apple.security.device.audio-input`, wire both in `tauri.conf.json`. Request microphone access through AVFoundation in the wizard, not by opening a stream and hoping. Check Accessibility with `AXIsProcessTrustedWithOptions` and prompt. Poll in the wizard so the step advances when the grant lands. Fixes F16. Windows: detect the microphone privacy switch being off (stream open fails with access denied) and link to `ms-settings:privacy-microphone`.
- **P2.10 Remove dead IPC.** Delete `start_recording`, `stop_recording`, `run_pipeline`, `inject_text`, `get_focused_app` and their TypeScript wrappers (M12). Keep `paste_last` and `cancel_recording`, routed through the controller.

### Gate for phase 2

- State-machine tests cover all three activation modes.
- On Linux under Xvfb: an automated test opens a text widget, runs the injector, and asserts the widget text and that the clipboard was restored.
- `docs/MANUAL_QA.md` has a checklist for Windows and macOS covering focus, modifiers held, non-Latin layout, terminal paste, elevated window, and clipboard with an image.

---

## 7. Model catalog and selection

### 7.1 Candidates

All run through sherpa-onnx on CPU. Sizes and accuracy figures are from memory and third-party pages and are **VERIFY** items. `P0.3` replaces them with measured numbers.

| Id | Family | Languages | Disk (approx.) | Role |
|---|---|---|---|---|
| `parakeet-110m-int8` | NeMo CTC (`nemo_ctc`) | English | 130 MB | **Default.** Best accuracy per MB of RAM, punctuation and capitals built in |
| `parakeet-0.6b-v3-int8` | NeMo transducer (`transducer`, `model_type = "nemo_transducer"`) | 25 European languages, auto-detected | 650 to 700 MB | Best accuracy, and the multilingual option, for machines with 16 GB RAM or more |
| `parakeet-0.6b-v2-int8` | NeMo transducer | English | 650 MB | Only keep if the benchmark shows it clearly beats v3 on English |
| `moonshine-base-en` | Moonshine | English | 60 to 130 MB | Low-memory option for machines with 4 GB RAM or less |

Repository names to check on Hugging Face under `csukuangfj/` and in the `asr-models` release of `k2-fsa/sherpa-onnx`: `sherpa-onnx-nemo-parakeet_tdt_ctc_110m-en-36000-int8`, `sherpa-onnx-nemo-parakeet-tdt-0.6b-v3-int8`, `sherpa-onnx-nemo-parakeet-tdt-0.6b-v2-int8`, `sherpa-onnx-moonshine-base-en-int8`. If the 110m int8 build does not exist or loses more than 1 point of word error rate against fp32, keep fp32 as the default and say so in the UI.

Do not add Whisper. It is slower on CPU at equal accuracy and it hallucinates on silence.

### P0 tasks

- **P0.1 Baseline.** Run section 13 on a clean checkout and record the results. The frontend lint, typecheck and build passed when this plan was written. Rust could not be built in the planning session because the sherpa-onnx build script downloads its libraries from github.com.
- **P0.2 Upgrade sherpa-onnx** from 1.12.34 to the latest 2.0.x (2.0.1 when this was written), keeping the `static` feature. The crate's public API for offline ASR and VAD has the same shape. 2.0 adds Moonshine v2 config (`merged_decoder`) and TEN VAD. If the upgrade breaks the Windows static CRT link, stay on 1.12.34: it already supports the CTC and transducer families.
- **P0.3 Bench harness.** `src-tauri/examples/bench.rs`: for a model directory and a folder of WAV plus reference text, print load time, warm-up time, real-time factor per clip, peak resident memory, and word error rate. Run it for every candidate, with 1, 2 and 4 threads. Write the table to `docs/MODEL_BENCHMARK.md`. This decides the catalog. If you cannot download models in your environment, write the harness, wire it into CI as a manually triggered job, and leave the decision table for the owner to fill in, keeping the defaults above.
- **P0.4 CI baseline.** Split `release.yml` into `ci.yml` (lint, typecheck, fmt, clippy, tests on Linux, Windows and macOS, cached with `Swatinem/rust-cache`) and `release.yml` (tag builds only).

### P4 tasks

- **P4.1 Manifest.** `core/stt/catalog.rs` holds, per model: id, display name, family, languages, licence, approximate RAM, and for each file the URL pinned to a commit hash (`resolve/<sha>/...`, never `main`), exact size and SHA-256. Fixes M14.
- **P4.2 Downloader.** One shared `reqwest::Client`. Stream to `<file>.part`, resume with `Range`, handle a `200` reply to a range request by restarting, handle `416` by verifying what is on disk. Hash while writing. On completion compare size and SHA-256, then rename atomically. Progress events carry real totals from the manifest. Check free disk space first. Cancel keeps the `.part` file so the download can resume. Test against a local HTTP server: resume, corruption, cancel, 416.
- **P4.3 Validity.** A model is installed only if every file matches the manifest size. Verify SHA-256 once after download and store a marker file. Fixes F15.
- **P4.4 Automatic choice.** First run: read total RAM and physical cores (`sysinfo` crate with only the needed features, or direct OS calls to avoid the dependency). Rule: less than 6 GB RAM -> `moonshine-base-en`. Otherwise `parakeet-110m-int8`. Offer `parakeet-0.6b-v3-int8` as "Higher accuracy / other languages" when RAM is 16 GB or more. The wizard shows the pick and its size, with a link to change it. Setting `speech_model` = `auto` or an explicit id.
- **P4.5 Models UI.** List every catalog model with size, languages, RAM estimate, state (not downloaded, downloading, installed, active), and actions (download, use, delete). Switching model evicts the old one. Deleting the active model is allowed and returns the app to the "needs download" state.
- **P4.6 Migration.** An existing `models/parakeet-tdt-ctc-110m/model.onnx` (fp32) stays usable as a legacy install. Show a one-time offer: "Switch to the smaller build and free about 330 MB". Remove the `parakeet-0.6b` id and map it to `auto`.

---

## 8. Overlay and feedback

### P3 tasks

- **P3.1 Backend trait.** `trait OverlayBackend { show(view), update(view), level(f32), hide() }`. First implementation: `WebviewOverlay`. The trait exists so a native overlay can replace it per platform later (`P6.5`).
- **P3.2 Rust drives the window.** Rust positions and shows the overlay on `RecordDown`. The page only renders. No IPC round trip before the window appears. Fixes F8.
- **P3.3 Never take focus.** Windows: build with `focusable(false)` if the Tauri version in use has it (**VERIFY** on `WebviewWindowBuilder` for the locked Tauri version), and in all cases set `WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW` on the HWND and show with `SW_SHOWNOACTIVATE`. macOS: convert the window to a non-activating `NSPanel` with the `tauri-nspanel` crate, level above full-screen apps, collection behaviour `canJoinAllSpaces | fullScreenAuxiliary`. Linux X11: window type hint `notification` or `utility`, `skip_taskbar`, `accept_focus(false)`. Linux Wayland: a client cannot position its own window, so use `gtk-layer-shell` when the compositor supports it and otherwise accept compositor placement.
- **P3.4 Click-through.** `set_ignore_cursor_events(true)` while listening and processing. Turn it off only in states that have a button (paste failed, copy).
- **P3.5 Placement.** Setting `overlay_position`: `bottom_center` (default), `top_center`, `custom`. Compute against the monitor that holds the focused window, in logical pixels, with a margin. `custom` stores a position as a fraction of the monitor work area, and falls back to `bottom_center` if the result is off-screen. Fixes M11. Size the window to the pill, not 400 x 120.
- **P3.6 Vanilla page.** Rewrite `overlay.html` as plain TypeScript and CSS: no React, no Jotai, no Tailwind runtime classes beyond what is compiled in. Target under 10 KB of JS. A canvas or five `div` bars driven by `level` events gives the real waveform (E6). Respect `prefers-reduced-motion` and `prefers-color-scheme`.
- **P3.7 Views.** `listening`, `hands_free`, `stopping_soon` (countdown), `loading_model`, `processing`, `done` (short preview, default 1.2 s, setting `overlay_preview` on/off), `no_speech`, `error { message, text?, can_copy }`. Send events with `emit_to("overlay", ...)` only.
- **P3.8 Tray state.** Three tray icons: idle, recording, processing. Switch them from the controller. This is the instant indicator that does not depend on any WebView. Add the template icon variant for macOS.
- **P3.9 Sound cues.** Setting `sound_cues` (default off). Two short embedded PCM clips (start, stop) played through `cpal` output on the audio thread. No new audio dependency.
- **P3.10 Overlay off.** Setting `overlay_enabled` (default true). When off, only the tray icon and optional sounds signal state.

### Gate for phase 3

Manual QA on each OS: overlay appears over a full-screen app, the target app keeps its caret, clicking through the pill reaches the app below, the waveform follows the voice.

---

## 9. Settings

### 9.1 Typed settings

- **P5.1** `settings/keys.rs` defines every key with its type, default and validator. `set_setting` rejects unknown keys and invalid values with a clear error. Reads go through a cached `Settings` struct held in `AppState` behind an `RwLock`, refreshed on write, so hot paths never touch SQLite.
- **P5.2** Every write runs the key's `apply` hook (re-register hotkeys, toggle autostart, move the overlay, reschedule eviction, prune history) and emits `settings-changed`. Write one test per key that proves the hook runs.
- **P5.3** Database migrations move to `PRAGMA user_version` with numbered steps. Migration 1 is the current schema. Migration 2 renames and adds keys for this plan and removes `overlay_x`, `overlay_y`.

### 9.2 Final settings list

| Page | Key | Type / values | Default | Effect |
|---|---|---|---|---|
| General | `auto_start` | bool | false, asked in the wizard | Autostart plugin, launched with `--autostart` so the window stays closed |
| General | `theme` | `system`, `light`, `dark` | `system` | Settings window and overlay |
| General | `update_check` | `off`, `on_start` | `off` | See `P7.4` |
| Dictation | `activation_mode` | `hold`, `toggle`, `hybrid` | `hybrid` | State machine |
| Dictation | `hotkey_record`, `hotkey_cancel`, `hotkey_paste_last`, `hotkey_open_app` | combo | per OS | Hotkey backend |
| Dictation | `cancel_key_enabled`, `cancel_key_in_hands_free` | bool | true, false | `P2.3` |
| Dictation | `max_recording_seconds` | 30 to 3600 | 600 | Controller |
| Dictation | `trailing_space` | bool | true | `core/text/join.rs` |
| Audio | `input_device` | `default` or device name | `default` | Audio thread |
| Audio | `sound_cues` | bool | false | `P3.9` |
| Output | `paste_method` | see `P2.6` | `auto` | Injector |
| Output | `restore_clipboard` | bool | true | Injector |
| Output | `clipboard_restore_delay_ms` | 100 to 2000 | 250 | Injector |
| Speech | `speech_model` | `auto` or catalog id | `auto` | Manager |
| Speech | `idle_unload_seconds` | 0, 30, 120, 300, 900 | 120 | Manager |
| Overlay | `overlay_enabled` | bool | true | Overlay backend |
| Overlay | `overlay_position` | `bottom_center`, `top_center`, `custom` | `bottom_center` | `P3.5` |
| Overlay | `overlay_preview` | bool | true | `P3.7` |
| Privacy | `history_enabled` | bool | true | Controller skips `SaveHistory` when false |
| Privacy | `history_retention_days` | 0 (forever), 7, 30, 90 | 0 | Prune at start and daily |
| Internal | `setup_complete`, `setup_step`, `sidebar_collapsed` | | | unchanged |

### P5 UI tasks

- **P5.4** Sidebar becomes: Dashboard, History, Dictation (hotkeys, mode, output), Audio, Speech, General (start-up, theme, overlay, privacy, updates, about). Use the existing components and `DESIGN_SYSTEM.md`. Every control saves on change and shows the backend error inline if the save fails.
- **P5.5** Audio page: device dropdown, live level meter, and a "Test dictation" box that runs the real pipeline and shows the text and the stage timings.
- **P5.6** About section: version, data folder button, logs folder button, "Copy diagnostics" (OS, CPU, RAM, model, last stage timings, no transcript text), third-party notices, "Reset all settings".
- **P5.7** Wizard order: Welcome, Microphone (permission plus level meter), Accessibility (macOS only), Hotkey (must be pressed to continue), Speech model (automatic pick, size, download), Try it (dictate into a text box), Done (launch-at-login checkbox). Add a "Retry" on every failure. Resume at the saved step.
- **P5.8** History: local-time grouping, search box (SQL `LIKE` with an index-free scan is fine at this size), per-entry copy and delete, export to a text or JSON file through a save dialog, clear all. Store `created_at` as UTC ISO-8601 with a `Z`.
- **P5.9** Dashboard stats: compute "today" and "this week" from local time by passing the local day boundaries from Rust (`chrono::Local`) into the query. Fixes M9.
- **P5.10** Dark theme for the settings window, driven by CSS variables.

### Gate for phase 5

Every row in the table above has a test for its apply hook and a line in `docs/MANUAL_QA.md`. No control exists in the UI without a backend effect.

---

## 10. Memory, CPU and size

### P6 tasks

- **P6.1 No settings WebView at start.** Remove the `main` window from `tauri.conf.json` (or set `"create": false`) and build it only when the wizard is needed or the user opens it. Fixes E1. Keep destroy-on-close.
- **P6.2 Start minimised.** With `--autostart`, never create the settings window.
- **P6.3 Overlay lifetime.** Setting-free behaviour: the overlay WebView is created at start (so the first press is instant) and destroyed after 10 minutes without a dictation. The next press recreates it in the background. Recording does not wait for it, and the tray icon shows state at once. Measure whether this actually lowers total memory on Windows, where WebView2 processes are shared. If destroying it saves less than 30 MB, keep it alive and delete this task's code.
- **P6.4 Idle CPU.** No timers, polling or animation frames while idle. The level-event stream and overlay animations stop when the overlay hides. Check with a profiler or `top`: idle CPU must read 0.0 %.
- **P6.5 Native overlay (stretch, per platform).** A second `OverlayBackend` that draws the pill without a WebView: on Windows a layered window updated with `UpdateLayeredWindow` from a `tiny-skia` pixmap, on macOS an `NSPanel` with Core Animation layers. With it, the app holds no WebView at all while idle. Only start this after the owner confirms the WebView overlay works on that platform, and keep `WebviewOverlay` as the fallback.
- **P6.6 Measurement.** `scripts/measure-memory.{ps1,sh}` starts the app, waits, and prints private memory for the main process and each WebView helper in three states: idle with models evicted, model loaded, settings window open. Record results in `docs/PERFORMANCE.md`. Replace the unmeasured claims in the README with these numbers.
- **P6.7 Binary size.** Audit `cargo bloat --release --crates`. Likely wins: drop `tauri-plugin-notification` if the tray and overlay cover its one use, drop `futures-util` if the downloader no longer needs it, drop `anyhow` if unused, trim `chrono` and `tokio` features, make sure `reqwest` has no default features. Keep `opt-level = 3` for the app. Do not trade decode speed for size.
- **P6.8 Frontend size.** Settings bundle: keep code-splitting. Drop Recharts from the docs if it is not used (it is not in `package.json`). The 50 KB logo PNG becomes an SVG or a smaller PNG.

---

## 11. Packaging, updates, CI, docs

### P7 tasks

- **P7.1 Single instance.** Add `tauri-plugin-single-instance` as the first plugin. A second launch with no arguments opens the settings window of the first. With `--toggle`, `--start`, `--stop`, `--cancel` it forwards the command to the controller (`P2.1`). Fixes M1.
- **P7.2 Logging.** Replace `env_logger` with `tauri-plugin-log`: file in the app log directory, 2 files of 1 MB at most, level `info`, level `debug` with `LOCALYAPPER_DEBUG=1`. Never log transcript text or clipboard content. Fixes M7.
- **P7.3 Autostart.** Stop enabling it at start (`lib.rs:198-215`). It follows the `auto_start` setting only, and passes `--autostart`.
- **P7.4 Updates.** Add `tauri-plugin-updater` with signed artifacts from GitHub Releases. Default is off. "Check for updates" in General does a manual check. This is the only network call besides model downloads: state that in the UI next to the toggle. **OWNER:** generate the updater key pair and add `TAURI_SIGNING_PRIVATE_KEY` to the repository secrets. Until then leave the plugin wired but the endpoint empty and the UI hidden.
- **P7.5 Bundles.**
  - Windows: NSIS, `installMode: currentUser`, `compression: lzma`, WebView2 `downloadBootstrapper`. Add an ARM64 build if the sherpa-onnx static library for `win-arm64` links.
  - macOS: DMG per architecture, `Info.plist` and entitlements from `P2.9`, hardened runtime, minimum system version 12.
  - Linux: deb with `depends` on `libwebkit2gtk-4.1-0`, `libasound2`, `libxdo3`, and `recommends` on `wl-clipboard`, `wtype`. Add rpm. Keep AppImage and note that it is large because it carries WebKit.
  - All: bundle `silero_vad.onnx` and `THIRD_PARTY_NOTICES.md` as resources.
- **P7.6 CI checks.** Fail the release job if the Windows installer is over 25 MB. Upload size numbers as a job summary. Run `cargo test` on all three OSes. Run the model integration test on Linux with a cached model. Use the tag's release notes file instead of the hard-coded `RELEASE_NOTES_v1.0.0.md`.
- **P7.7 Signing.** **OWNER:** Apple Developer ID for notarisation and a Windows code-signing certificate (or Azure Trusted Signing). Without them macOS users must clear quarantine by hand and lose the Accessibility grant on each update, and Windows shows SmartScreen. Wire the CI steps so they activate when the secrets exist. Document the unsigned-install steps in the README until then.
- **P7.8 Notices.** `THIRD_PARTY_NOTICES.md`: Parakeet models (NVIDIA, CC-BY-4.0), Moonshine (MIT), Silero VAD (MIT), sherpa-onnx (Apache-2.0), ONNX Runtime (MIT), test fixtures. Generate the Rust and npm dependency licence list with `cargo about` and `license-checker` in CI. Fixes M15.
- **P7.9 Docs.** Delete `docs/FINAL_AUDIT.md`, `docs/PRODUCT_AUDIT.md`, `docs/PROJECT_COMPLETION.md`, `docs/RELEASE_NOTES_v0.1.0.md`. Rewrite `README.md`, `CLAUDE.md`, `AGENTS.md`, `docs/BUILD.md`, `docs/MANUAL_QA.md` to describe the rebuilt app. Add `docs/ARCHITECTURE.md` (section 3 of this plan, updated to what was built), `docs/PERFORMANCE.md`, `docs/MODEL_BENCHMARK.md`. Fixes M16.
- **P7.10 Version.** Release the rebuild as `2.0.0`, since settings keys, model ids and defaults change.

---

## 12. Phase order and gates

| Phase | Content | Gate |
|---|---|---|
| P0 | Baseline, sherpa-onnx upgrade, bench harness, CI split | CI green on three OSes, bench table exists or is handed to the owner |
| P1 | Audio thread, formats, resampler, segmenter, engine manager, incremental decode, controller and state machine | Headless pipeline test passes, stage timings logged |
| P2 | Hotkey backends, injection rewrite, focus, permissions, dead IPC removed | Xvfb injection test passes, manual checklist written |
| P3 | Overlay backend, non-activating window, vanilla page, tray states, cues | Manual QA per OS |
| P4 | Catalog, verified downloader, automatic choice, models UI, migration | Downloader tests pass against a local server |
| P5 | Typed settings, all pages, wizard, history, stats, dark theme | One apply-hook test per key |
| P6 | Window lifetimes, eviction and trim, measurements, size work | `docs/PERFORMANCE.md` has measured numbers that meet A7 or explain the gap |
| P7 | Single instance, logging, autostart, updater, bundles, CI, notices, docs, version | Tagged build produces installers for all targets |
| P8 | Full acceptance run of section 1 on real hardware | Owner sign-off |

P1 and P2 together are what make the app work. If time is short, stop after P3: at that point the core loop meets A1 to A5 and the rest is settings and polish.

Suggested dependency changes, all in `src-tauri/Cargo.toml`:

| Change | Reason |
|---|---|
| `sherpa-onnx` 1.12 -> 2.0 | `P0.2` |
| add `rubato` (or use the sherpa resampler) | `P1.4` |
| add `ringbuf` | `P1.3` |
| add `sha2` | `P4.2` |
| add `tauri-plugin-single-instance`, `tauri-plugin-log`, `tauri-plugin-updater`, `tauri-plugin-dialog` | `P7.1`, `P7.2`, `P7.4`, `P5.8` |
| add `tauri-nspanel` (macOS), `objc2` crates (macOS), `ashpd` (Linux), `gtk-layer-shell` (Linux, optional feature) | `P3.3`, `P2.8`, `P2.1` |
| extend `windows` features: `Win32_UI_Input_KeyboardAndMouse`, `Win32_System_DataExchange`, `Win32_System_ProcessStatus`, `Win32_Security` | `P2.5`, `P2.8`, `P1.14` |
| `arboard` features: `image-data`, `wayland-data-control` | `P2.5`, `P2.7` |
| remove `env_logger`, and `tauri-plugin-notification`, `futures-util`, `anyhow` if they end up unused | `P6.7` |

---

## 13. Verification commands

Run after every task:

```bash
npm run lint && npx tsc --noEmit
```

```bash
cargo fmt --manifest-path src-tauri/Cargo.toml --check
```

```bash
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
```

```bash
cargo test --manifest-path src-tauri/Cargo.toml
```

With a model available:

```bash
LOCALYAPPER_TEST_MODEL_DIR=/path/to/model cargo test --manifest-path src-tauri/Cargo.toml -- --include-ignored pipeline
```

Building the Rust side needs network access to `github.com` release assets, because `sherpa-onnx-sys` downloads its prebuilt libraries. If that host is blocked, download the archive elsewhere and set `SHERPA_ONNX_ARCHIVE_DIR` or `SHERPA_ONNX_LIB_DIR`. Model downloads need `huggingface.co`.

---

## 14. Things deliberately left out

| Item | Why |
|---|---|
| LLM clean-up, cloud STT, BYOK | Out of scope by product decision |
| Learning corrections and a personal dictionary | Removed on purpose earlier. A static find-and-replace list would be easy to add in `core/text/` later if the owner asks |
| GPU execution providers (DirectML, CoreML, CUDA) | CPU int8 already meets the latency budget, and GPU providers add installer size and driver-specific failures. Revisit only if the benchmark shows the budget is missed |
| Streaming partial text into the target app while speaking | Needs a streaming model with worse accuracy, and rewriting text in a foreign app is fragile |
| Bare modifier or Fn key as the hotkey | Needs low-level keyboard hooks on every OS. Worth doing later behind the `HotkeyBackend` trait |
| Mobile targets | Not a desktop dictation use case |

---

## 15. Owner decisions and inputs needed

1. Updater signing key pair (`P7.4`).
2. Apple Developer ID and Windows code-signing certificate (`P7.7`).
3. Confirmation of the default model after `docs/MODEL_BENCHMARK.md` is filled in (section 7).
4. A real-hardware test pass on Windows and macOS using `docs/MANUAL_QA.md`, since platform code written in a Linux container is compile-checked only.
5. Whether the native no-WebView overlay (`P6.5`) is worth the extra platform code once the measured numbers from `P6.6` are in.
