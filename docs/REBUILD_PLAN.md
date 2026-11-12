# LocalYapper Rebuild Plan (v3)

**Audience:** the implementing agent. Read sections 0 to 4 fully before touching code. The appendices are reference material to use when you reach the matching task.
**Written against:** commit `2115b4b` (version 1.0.0).
**What this is:** a build plan produced by reading every source file and by checking the dependency APIs against their published sources. Nothing in the app has been changed yet. Appendices A to C contain code that was compiled and tested (26 tests, clippy clean) as a standalone crate. Appendices D to G are sketches that were not compiled.

---

## 0. Ground rules

1. Work in phase order. Each phase has a gate. Do not start the next phase until the gate passes.
2. One commit per task id (for example `P1.3`), with the id in the commit message.
3. Run section 14 after every task.
4. The owner has granted full freedom over architecture, dependencies, models and UI. Where this plan conflicts with `CLAUDE.md` or `AGENTS.md`, this plan wins. Task `P7.9` rewrites those files.
5. Decisions in this plan are final unless a gate fails. Do not reopen them, and do not add features that are not listed.
6. **This runs unattended, start to finish.** Never stop to ask a question or wait for a person. When something is unclear, take the option this plan implies, note it in `docs/BUILD_REPORT.md`, and continue.
7. **All verification is automated.** Your container is Linux, so Windows and macOS code is exercised by the CI smoke suite on GitHub's Windows and macOS runners (section 8A). Push, read the CI result with the GitHub tools, fix, repeat. A phase gate passes only when CI is green on all three OSes. There is no manual testing step anywhere in this plan.
8. If CI cannot exercise something (a physical microphone, or macOS key events when the runner lacks the Accessibility grant), the test must skip with a printed reason, and `docs/BUILD_REPORT.md` must list it under "Not exercised by CI". Never describe such code as verified.
9. Nothing in this plan needs a secret, a certificate or a paid account. Code signing and the auto-updater are out of scope (section 15).

### Rules that stay

- Nothing leaves the device except model downloads and an opt-in update check.
- Audio is never written to disk.
- No `unwrap()` / `expect()` in production Rust. No `any` in TypeScript.
- Never edit `src-tauri/src/main.rs`. Only `lib.rs`.
- `rusqlite` keeps the `bundled` feature.
- No LLM, Ollama or BYOK.

### Rules that change

| Old rule | New rule |
|---|---|
| Injection must be clipboard save, set, paste, restore | That flow is the default. Direct typing and copy-only are allowed fallbacks and user settings |
| sherpa-onnx 1.12 | sherpa-onnx 2.0.x |
| Parakeet only | A small verified catalog with automatic choice |
| Overlay is a React page | Overlay is a plain TypeScript page driven from Rust |

---

## 1. Acceptance criteria

| # | Criterion | Measured by |
|---|---|---|
| A1 | Hotkey press starts audio capture within 150 ms and shows the overlay within 150 ms | Stage timings in the log (`P1.11`) |
| A2 | The overlay waveform follows the real microphone level | Level-meter unit test, and a smoke test that the overlay receives changing `overlay-level` values during a fake-audio recording |
| A3 | After release, text is in the target app in under 2 s at the median, and under 5 s in the worst supported case (cold model, 60 s of speech, 4-core laptop) | Stage timings, bench harness (`P0.3`) |
| A4 | Focus never leaves the target app | CI smoke test per OS: foreground window is the same before, during and after a dictation |
| A5 | The clipboard holds the same content after dictation as before | Automated under Xvfb, manual elsewhere |
| A6 | Every setting in the UI changes backend behaviour at once and survives restart | One test per key (`P5.2`) |
| A7 | Idle, settings closed, model evicted: main process under 40 MB private memory and no settings WebView alive | `scripts/measure-memory` (`P6.6`) |
| A8 | Windows installer under 25 MB. Models download on first run and are checksum-verified | CI size check (`P7.6`) |
| A9 | A second launch focuses the running instance | CI smoke test: second process exits, first stays |
| A10 | Works on Windows, macOS and Linux X11. Wayland works with the documented limits | CI smoke suite green on `windows-latest`, `macos-15`, `ubuntu-22.04` (Xvfb) |

A3 and A7 depend on the machine. Measure them on the CI runners, write the numbers to `docs/PERFORMANCE.md`, and report them as measured, whatever they are.

### Where the time goes (10 s utterance, 4-core laptop, cold model)

| Stage | Budget | What makes it possible |
|---|---|---|
| Press to capture running | 150 ms | Audio thread already alive, only the stream is built on press |
| Press to model ready | 1.5 s, hidden behind the user's own speech | Model load starts on press |
| Decoding during speech | none added | Segments are decoded as they close |
| Release to last text | 0.3 to 1.5 s | Only the tail segment (15 s at most) is left |
| Paste | 150 ms | Clipboard restore happens afterwards, off the critical path |

The current app loads the model after release, decodes the whole recording in one pass, then sleeps 3 s. Those three things are most of the gap to A3.

---

## 2. Findings in the current code

Each item was read in the source at commit `2115b4b`.

### 2.1 Bugs that break the core flow

| ID | Finding | Where |
|---|---|---|
| F1 | The input stream is always built as `f32`. Devices whose default format is `i16` or `u16` fail to open, so dictation never starts | `src-tauri/src/audio/capture.rs:114-117` |
| F2 | Resampling is linear interpolation with no low-pass filter, so 44.1/48 kHz input aliases and accuracy drops. One sample is dropped and the phase reset at every buffer boundary | `capture.rs:141-159` |
| F3 | The audio callback allocates, takes two mutexes, and silently drops audio when `try_lock` fails | `capture.rs:124-171` |
| F4 | `cpal::Stream` is forced `Send + Sync` with `unsafe impl` | `capture.rs:23-30` |
| F5 | After a paste the state machine sleeps 3 s before going idle. Presses in that window are ignored | `src-tauri/src/hotkey/manager.rs:591-592` |
| F6 | Press and release are each handled in their own spawned task, so a fast tap can run the release before the press | `manager.rs:124-133` |
| F7 | The model loads only after recording stops. The first dictation after start, and after each 60 s idle eviction, pays a full 458 MB load before decoding | `src-tauri/src/commands/recording.rs:52` |
| F8 | The overlay is shown from JavaScript, after an event round trip and two settings reads, with `show()`. The window has no non-activating style, so it can take focus and the paste goes nowhere | `src/hooks/useOverlayState.ts:62-86`, `src-tauri/tauri.conf.json:42-55` |
| F9 | Paste sends `Key::Unicode('v')`. On Windows that is typed as a character, not the V key, so Ctrl+V fails on non-Latin layouts | `src-tauri/src/injection/injector.rs:50` |
| F10 | Paste fires while the user may still hold the hotkey's modifiers | `injector.rs:47-62` |
| F11 | The clipboard is restored 80 ms after paste. Slow apps then paste the old content. Only text is saved, so a copied image is lost | `injector.rs:7,31,64,75` |
| F12 | Linux shells out to `xclip`, `xdotool`, `wl-copy`, `wtype`. None is a declared dependency | `injector.rs:83-211` |
| F13 | Global shortcuts do not work on Wayland. The `global-hotkey` crate has Windows, macOS and X11 back ends only | `manager.rs:118` |
| F14 | `parakeet-0.6b` is offered but cannot work. Its URL has no single `model.onnx`, and the loader only builds a CTC config while that model is a transducer | `src-tauri/src/stt/whisper.rs:51-60,149-151` |
| F15 | A model file is "valid" when larger than 1 MB, so a truncated download passes and then fails to load forever | `whisper.rs:66-91` |
| F16 | The macOS bundle has no `Info.plist` microphone usage string and no entitlements | no `src-tauri/Info.plist` |
| F17 | VAD keeps only detected speech with no padding, which clips word starts. If VAD finds nothing the recording is discarded even when it has clear energy | `src-tauri/src/audio/vad.rs:39-47,100-124` |
| F18 | Up to 120 s is decoded in one pass. Memory and time grow faster than linearly with length | `whisper.rs:193-195` |
| F19 | The 30 s pipeline timeout does not cancel the blocking work, and it includes model load | `manager.rs:469-497` |
| F20 | Two callers of `ensure_speech_model_loaded` can both load the model | `src-tauri/src/commands/models.rs:22-89` |
| F21 | **Linux X11 hold-to-talk stops by itself.** `global-hotkey` 0.7 forwards every X11 `KeyRelease`. X11 auto-repeat sends release/press pairs while a key is held, so a fake release arrives after the keyboard repeat delay | `global-hotkey-0.7.0/src/platform_impl/x11/mod.rs:262-292` |

### 2.2 Efficiency problems

| ID | Finding | Where |
|---|---|---|
| E1 | The settings window is in the config with `visible: false`, so its WebView is created at every start and stays alive until the user opens and closes it | `tauri.conf.json:28-40` |
| E2 | The fp32 model is downloaded (458 MB on disk, about that in RAM) | `whisper.rs:41-50` |
| E3 | The overlay loads React and Jotai (about 200 KB of JS) to draw one pill | `src/overlay-main.tsx` |
| E4 | Pipeline events are broadcast to every window | `manager.rs:670` |
| E5 | Nothing checks that memory returns to the OS after eviction | `src-tauri/src/stt/lifecycle.rs:92-118` |
| E6 | The waveform is a CSS loop unrelated to the microphone | `src/components/overlay/Waveform.tsx` |
| E7 | `blank_penalty: 1.2` is an untested tweak that can insert tokens | `whisper.rs:160` |

### 2.3 Missing features

| ID | Finding |
|---|---|
| M1 | No single-instance guard |
| M2 | Launch at login is forced on at first start, with no UI to turn it off (`src-tauri/src/lib.rs:198-215`) |
| M3 | No microphone choice, no handling of a device that disappears |
| M4 | No model choice in the UI |
| M5 | No general settings: paste method, clipboard, overlay position, history retention, sounds, theme |
| M6 | `set_setting` accepts any key and value, with no validation and no effect (`src-tauri/src/commands/settings.rs:16-24`) |
| M7 | No log file. Release builds on Windows have no console |
| M8 | No updater, no code signing |
| M9 | Dashboard "today" uses UTC (`src-tauri/src/db/queries.rs:79`) |
| M10 | History is kept forever with no retention or opt-out |
| M11 | Overlay position is stored in physical pixels with no monitor check. Default is (100, 100) |
| M12 | Unused IPC commands are registered: `start_recording`, `stop_recording`, `run_pipeline`, `inject_text`, `get_focused_app` (`lib.rs:233-239`) |
| M13 | Escape is grabbed globally for the whole recording, including long hands-free sessions |
| M14 | Models come from mutable `resolve/main` URLs with no checksum |
| M15 | No third-party notices. The Parakeet models are CC-BY-4.0 and need attribution |
| M16 | Version is `1.0.0` in the manifests and "v0.1.0 release candidate" in `CLAUDE.md`. Five audit documents contradict each other |
| M17 | The engine type is named `WhisperEngine` in `whisper.rs` |
| M18 | No test pushes audio through the pipeline |

### 2.4 Keep these

- The SQLite tables and WAL pragmas.
- mimalloc as the global allocator.
- The tray menu and destroy-on-close for the settings window.
- The settings look (`DESIGN_SYSTEM.md`), the wizard flow, the history and dashboard pages.
- History is saved before injection.
- Hotkey update with rollback (`src-tauri/src/commands/hotkeys.rs:86-121`).
- The release profile.

---

## 3. Facts checked against dependency sources

These were read in the crate sources from crates.io. Build on them without re-checking.

| Fact | Source |
|---|---|
| Tauri 2.10.3 has `WebviewWindowBuilder::focusable(bool)`, `WebviewWindow::set_focusable`, `set_ignore_cursor_events`, and config keys `focusable` and `create` per window | `tauri-2.10.3/src/webview/webview_window.rs:607,2085,2155`, `tauri-utils/src/config.rs` |
| On Windows a non-focusable tao window gets `WS_EX_NOACTIVATE`. tao still calls `ShowWindow(SW_SHOW)` unless an internal marker is set, so show the overlay yourself with `SW_SHOWNOACTIVATE` (Appendix F) | `tao-0.34.6/src/platform_impl/windows/window_state.rs:296-333` |
| On Linux `focusable(false)` maps to GTK `accept_focus(false)`. On macOS it makes the window refuse key status | `tao-0.34.6/src/platform_impl/{linux,macos}/window.rs` |
| `global-hotkey` 0.7 emits `Released` on Windows (by polling the key), macOS and X11. Windows registers with `MOD_NOREPEAT`. X11 has no auto-repeat handling (F21). There is no Wayland back end | `global-hotkey-0.7.0/src/platform_impl/` |
| enigo 0.2.1: on Windows `Key::Other(v)` sends virtual-key `v`, and `Key::Unicode` is sent as a character. On macOS `raw(keycode, dir)` sends that hardware key code. On Linux with the default `xdo` feature `raw()` is `todo!()` and **panics**, so never call it there | `enigo-0.2.1/src/keycodes.rs:1021-1025`, `src/macos/macos_impl.rs:495`, `src/linux/xdo.rs:235-240` |
| enigo 0.2.1 has optional features `wayland` and `libei` | `enigo-0.2.1/Cargo.toml` |
| arboard 3.6.1: `Clipboard::set()` returns a builder with `.text()`, `.html()`, `.image()`. `exclude_from_history()` exists on all three desktop platforms through `SetExtWindows`, `SetExtApple`, `SetExtLinux`. Windows also has `exclude_from_monitoring()` and `exclude_from_cloud()` | `arboard-3.6.1/src/lib.rs:166-246`, `src/platform/` |
| sherpa-onnx 2.0.1 is the current release. Its `LinearResampler::create(in_hz, out_hz)` is band-limited (cut-off 0.99 x Nyquist of the lower rate, 6 zero crossings). Use it. No other resampler is needed | `sherpa-onnx-2.0.1/src/resampler.rs:19-37` |
| sherpa-onnx 2.0.1 offline config has `transducer { encoder, decoder, joiner }`, `nemo_ctc { model }`, `moonshine { preprocessor, encoder, uncached_decoder, cached_decoder, merged_decoder }`, and `model_type`. Parakeet TDT needs `model_type = "nemo_transducer"`. Its file names are `encoder.int8.onnx`, `decoder.int8.onnx`, `joiner.int8.onnx`, `tokens.txt` | `sherpa-onnx-2.0.1/src/offline_asr.rs:49-63,164-171,656-671` |
| `OfflineRecognizer`, `OfflineStream`, `VoiceActivityDetector` are `Send + Sync` in the crate, so the app's own `unsafe impl` lines can go | `offline_asr.rs:685-686`, `vad.rs:196-197` |
| `VoiceActivityDetector` has `accept_waveform`, `is_empty`, `front() -> Option<SpeechSegment>`, `pop`, `flush`, `detected`, `reset`. `SpeechSegment` has `start() -> i64` (absolute sample index) and `samples()` | `sherpa-onnx-2.0.1/src/vad.rs:169-269` |
| `sherpa-onnx-sys` downloads prebuilt static libraries from github.com releases at build time. `SHERPA_ONNX_ARCHIVE_DIR` or `SHERPA_ONNX_LIB_DIR` point it at a local copy | `sherpa-onnx-sys-2.0.1/build.rs:45-46,130-200` |
| cpal 0.15.3 sample formats: `I8, I16, I32, I64, U8, U16, U32, U64, F32, F64` | `cpal-0.15.3/src/samples_formats.rs` |
| `tauri-nspanel` 2.1 is on crates.io. It has `PanelBuilder`, `PanelLevel`, and the `tauri_panel!` macro with config keys such as `can_become_key_window` and `is_floating_panel` | crate README |
| `tauri-plugin-single-instance` 2.5: `init(|app, args, cwd| ...)` | `src/lib.rs:50` |
| ringbuf 0.4: `HeapRb::<f32>::new(n)`, `split()`, `push_slice`, `pop_slice`, `occupied_len`, with `use ringbuf::traits::*` | crate README and `src/traits/` |
| The frontend lint, typecheck and production build pass at `2115b4b` | run during planning |

Known model repository names (from the sherpa-onnx crate docs): `sherpa-onnx-nemo-parakeet-tdt-0.6b-v3-int8`, `sherpa-onnx-nemo-parakeet-tdt-0.6b-v2-int8`, `sherpa-onnx-moonshine-tiny-en-int8`.

**Not confirmed** (huggingface.co and github.com release assets were unreachable while planning): the exact name and size of the int8 build of Parakeet 110m, the size of each model file, and every checksum. Task `P0.4` resolves these with a script, so no guessing is needed.

---

## 4. Target architecture

### 4.1 Threads

| Unit | Kind | Owns | Lives |
|---|---|---|---|
| Controller | one tokio task with an `mpsc` inbox | the `Session` state machine (Appendix A) | whole app |
| Audio thread | OS thread | the `cpal` stream and the ring-buffer producer | whole app. The stream exists only while recording |
| Segmenter | OS thread | ring-buffer consumer, resampler, VAD, rolling buffer | one per recording |
| STT worker | OS thread | the recogniser, a queue of segments | from first use to eviction |
| Injector | `spawn_blocking` | clipboard and key events | one per paste |

Every unit reports to the controller through one channel of `session::Event`. The controller is the only owner of dictation state. It calls `Session::handle(event)` and executes the returned `Effect`s in order. No other code decides what happens next.

### 4.2 Module layout

```
src-tauri/src/
  lib.rs                 setup and wiring only
  error.rs
  controller.rs          runs Session, executes Effects, owns timers
  settings/
    mod.rs               typed Settings, load, validate, apply hooks
    keys.rs              one entry per key: type, default, validator
  core/                  no `tauri` imports anywhere in this tree
    session.rs           Appendix A, verbatim
    audio/
      convert.rs         sample format to f32, downmix to mono
      level.rs           RMS meter
      rolling.rs         Appendix B, verbatim
      segmenter.rs       Appendix D.1
    stt/
      catalog.rs         model manifest
      engine.rs          Appendix D.2
      manager.rs         load once, warm up, evict, trim
    download.rs          resumable verified downloader
  platform/
    audio_in.rs          cpal devices and stream
    hotkey/
      mod.rs             HotkeyBackend trait
      shortcut.rs        tauri-plugin-global-shortcut back end
      repeat_filter.rs   Appendix C, verbatim (used on Linux X11)
      portal.rs          Wayland GlobalShortcuts portal
    inject/              Appendix E
    overlay/             Appendix F
    focus.rs
    permissions.rs
    memory.rs
  commands/              thin IPC handlers
  db/
  tray/
```

### 4.3 Controller responsibilities

The controller translates between the pure state machine and the world:

| Effect | What the controller does |
|---|---|
| `StartCapture { id }` | Read the foreground app into `target_app` first. Then tell the audio thread to open the stream and spawn the segmenter for `id` |
| `EnsureModelLoaded` | `manager.ensure_loaded()` on the STT worker. On failure send `ModelFailed` |
| `StopCapture { id }` | Stop the stream. The segmenter flushes, queues the tail, and sends `CaptureEnded` |
| `DiscardCapture { id }` | Stop the stream, drop the segmenter, clear queued segments for `id` |
| `Overlay(view)` | `overlay.show(view)` or `hide()`. `Done` and `NoSpeech` start a hide timer owned by the overlay back end |
| `Tray(state)` | Swap the tray icon and tooltip |
| `Cue(kind)` | Play the cue if `sound_cues` is on |
| `RegisterCancelKey` / `UnregisterCancelKey` | Register or unregister the combo held in a controller field, never re-read from settings |
| `StartTimer { kind, id, after_ms }` | `tokio::time::sleep`, then send `Event::Timer { kind, id }` |
| `Inject { id, text }` | Apply `trailing_space`, call the injector on a blocking thread, send `InjectDone` |
| `SaveHistory { text, audio_ms }` | Insert the row with `target_app` |
| `ScheduleEvict` | Reset the idle-eviction timer |

Timestamps passed in `RecordDown` and `RecordUp` are milliseconds from one `Instant` taken at start-up.

The segmenter sends `SegmentQueued { id, index }` when it hands a segment to the STT worker. The worker sends `SegmentText { id, index, text }`. Late messages from an old `id` are ignored by the state machine, which is why cancel needs no locking.

---

## 5. Phase 0: baseline

- **P0.1 Baseline.** Run section 14 on a clean checkout and record the results in the first commit message.
- **P0.2 Upgrade sherpa-onnx** from 1.12.34 to 2.0.x with the default `static` feature. The offline ASR and VAD APIs have the same shape. If the Windows static-CRT link fails in CI, pin back to 1.12.34 (it already has the CTC, transducer and Moonshine v1 configs) and continue.
- **P0.3 Bench harness.** `src-tauri/examples/bench.rs`. Input: a model id, a model directory, a folder of `*.wav` with matching `*.txt`. Output per run: load ms, warm-up ms, real-time factor per clip, peak resident memory, word error rate. Run with 1, 2 and 4 threads. Add a CI workflow (`bench.yml`, runs on push to the working branch when `models.json` or the harness changes) that downloads the models, runs the harness on the Linux runner, and commits `docs/MODEL_BENCHMARK.md` back to the branch.
- **P0.4 Model manifest script.** `scripts/gen-model-manifest.py` (Appendix G). It asks the Hugging Face API for the pinned commit, file sizes and SHA-256 of each candidate repository and writes `src-tauri/models.json`. If your container can reach huggingface.co, run it and commit the output. If not, add a CI workflow (`manifest.yml`) that runs the script on a GitHub runner and commits `src-tauri/models.json` back to the branch, then pull. Until the file exists, the app keeps the current fp32 110m entry so nothing is blocked.
- **P0.5 CI split.** `ci.yml`: lint, typecheck, fmt, clippy, tests on Linux, Windows and macOS with `Swatinem/rust-cache`. `release.yml`: tag builds only.

**Gate:** CI green on three OSes, `models.json` and `docs/MODEL_BENCHMARK.md` committed.

---

## 6. Phase 1: pipeline

Copy Appendix A, B and C into place first. They compile and their tests pass as written.

- **P1.1 Audio thread** (`platform/audio_in.rs`). A thread created at start-up owns the stream so it never crosses threads. Commands: `Start { device, producer }`, `Stop`, `Shutdown`. Delete the `unsafe impl Send/Sync for StreamHandle`.
- **P1.2 Sample formats.** Match on `sample_format()` and build a typed stream for `F32`, `I16`, `U16`, `I32`, `U8`, `I8`. Convert with `core/audio/convert.rs` (`i16 / 32768.0`, `(u16 - 32768) / 32768.0`, and so on). Return a clear error for any other format.
- **P1.3 Callback.** Downmix to mono into a fixed stack buffer in chunks and `push_slice` into a `ringbuf` producer sized for 2 s at the native rate. No allocation, lock or log in the callback. Count overflowed samples in an atomic and log the count once after the recording.
- **P1.4 Segmenter** (Appendix D.1). Pops from the ring, resamples to 16 kHz with `sherpa_onnx::LinearResampler`, feeds the VAD in 512-sample windows, cuts padded segments from the rolling buffer.
- **P1.5 Level meter.** RMS over 20 ms windows computed in the segmenter, smoothed with attack 0.6 and release 0.15, sent to the overlay at 30 Hz as one `f32` in `[0, 1]`.
- **P1.6 VAD parameters.** Threshold 0.4, minimum silence 0.35 s, minimum speech 0.15 s, maximum segment 15 s, pre-roll 200 ms, post-roll 150 ms. Tune only with the bench fixtures, and record any change in `docs/MODEL_BENCHMARK.md`.
- **P1.7 No-speech safety net.** If the recording is longer than 0.4 s, the VAD produced no segment, and overall RMS is above 0.01, send the last 15 s as one segment.
- **P1.8 Devices.** Setting `input_device` is `default` or a device name. If the named device is missing, use the default and emit a warning event. The stream error callback sends `CaptureFailed`. Add IPC `list_input_devices`.
- **P1.9 Length.** One setting, `max_recording_seconds`, default 600, range 30 to 3600. Delete the three hard-coded 120 s limits.
- **P1.10 Engine** (Appendix D.2). Rename `WhisperEngine` to `SherpaEngine` in `core/stt/engine.rs`. Build the config from the catalog family. Drop `blank_penalty`.
- **P1.11 Manager.** A `tokio::sync::Mutex` around loading, so only one load runs. `ensure_loaded()` is idempotent. After loading, decode 0.5 s of silence once. Log one line per dictation: `capture_start_ms model_ready_ms segments audio_ms decode_ms tail_decode_ms inject_ms release_to_text_ms`. Never log text.
- **P1.12 Threads.** `num_threads = min(4, physical cores)`.
- **P1.13 Eviction.** `idle_unload_seconds`: 0 (never), 30, 120 (default), 300, 900. On eviction drop the recogniser and VAD, then `platform::memory::trim()`: `mi_collect(true)`, plus `EmptyWorkingSet` on Windows and `malloc_trim(0)` on glibc. Log resident memory before and after. If it does not drop, find out why before continuing.
- **P1.14 Controller.** Implement section 4.3. Delete `hotkey/manager.rs`, `stt/lifecycle.rs`, `state::HotkeyState`, and the dictation logic in `commands/recording.rs`.
- **P1.15 Bundle the VAD.** Put `silero_vad.onnx` in `src-tauri/resources/` and load it from the resource directory. Delete the runtime VAD download and keep the energy VAD as the fallback if loading fails.

**Gate:**

- `cargo test` passes with no audio device and no display.
- An integration test feeds WAV fixtures through `convert`, resample, segmenter, engine and join with a fake audio source and checks word error rate. It runs when `LOCALYAPPER_TEST_MODEL_DIR` is set. CI sets it and caches the model.
- Fixtures: 5 to 10 LibriSpeech test-clean clips (CC-BY-4.0, add the attribution) including one at 44.1 kHz stereo, one 16-bit, one with a 3 s pause, and one 90 s clip made by joining clips.
- Resampler test: a 1 kHz sine at 48 kHz comes out at 1 kHz within 1 Hz and 0.5 dB. A 10 kHz sine at 48 kHz comes out at least 40 dB down.

---

## 7. Phase 2: hotkeys, injection, permissions

- **P2.1 Hotkey back ends.** `trait HotkeyBackend { fn register(&self, id: HotkeyId, combo: &str) -> Result<()>; fn unregister(&self, id: HotkeyId) -> Result<()>; }`. Back ends send `RecordDown`, `RecordUp`, `Cancel` and the other actions into the controller channel from the callback itself. Do not spawn a task per event.
  - `shortcut.rs`: `tauri-plugin-global-shortcut` for Windows, macOS and X11. On Linux, wrap the record key in `RepeatFilter::new(30)` and run a 10 ms timer only while a release is pending (fixes F21).
  - `portal.rs`: on Wayland use the `org.freedesktop.portal.GlobalShortcuts` portal through `ashpd`. If the portal is missing, report it in the UI and point to the CLI.
  - CLI: `localyapper --toggle | --start | --stop | --cancel`, forwarded to the running instance by the single-instance plugin (`P7.1`). Any desktop can bind these.
- **P2.2 Defaults.** Record: `F8` on Windows and Linux, `Alt+Space` on macOS. The wizard requires the user to press the key and see it detected.
- **P2.3 Cancel key.** Settings `cancel_key_enabled` (true) and `cancel_key_in_hands_free` (false). The state machine already implements the policy.
- **P2.4 Pause.** Unregister record, cancel and paste-last. Keep open-app.
- **P2.5 Injection** (Appendix E). One function per OS, all in-process.
- **P2.6 Paste method.** `paste_method`: `auto` (default), `ctrl_v`, `ctrl_shift_v`, `shift_insert`, `type`, `clipboard_only`. `auto` means Cmd+V on macOS, Ctrl+Shift+V when the focused app is a known terminal on Windows or Linux, Ctrl+V otherwise.
- **P2.7 Focus** (`platform/focus.rs`). Read the foreground app on `StartCapture` before any overlay call. On macOS replace the `osascript` process with `NSWorkspace.frontmostApplication` through `objc2-app-kit`. On Windows, if the foreground window belongs to an elevated process and LocalYapper is not elevated, skip the paste and show "Cannot type into an administrator window" with the copy button.
- **P2.8 Permissions.** macOS: add `src-tauri/Info.plist` with `NSMicrophoneUsageDescription`, add `entitlements.plist` with `com.apple.security.device.audio-input`, reference both from `tauri.conf.json`. Request microphone access through AVFoundation in the wizard. Check Accessibility with `AXIsProcessTrustedWithOptions` (prompt true) and poll every second in the wizard so the step advances on grant. Windows: when the stream fails with access denied, link to `ms-settings:privacy-microphone`.
- **P2.9 Remove dead IPC.** Delete `start_recording`, `stop_recording`, `run_pipeline`, `inject_text`, `get_focused_app` and their TypeScript wrappers. Route `paste_last` and `cancel_recording` through the controller.

**Gate:**

- Under Xvfb, a test opens a GTK or xterm text target, runs the injector, and asserts the target text and that the clipboard was restored.
- The injection smoke tests in section 8A pass on all three runners.

---

## 8. Phase 3: overlay

- **P3.1 Trait.** `trait OverlayBackend { fn show(&self, view: &View); fn level(&self, v: f32); fn hide(&self); }`. First implementation: `WebviewOverlay` (Appendix F).
- **P3.2 Rust owns the window.** Rust positions and shows it. The page only renders what it is told. Nothing in the page calls `show()`, `hide()` or `setPosition()`.
- **P3.3 Never take focus.** Build with `.focusable(false)`. Windows: show with `SW_SHOWNOACTIVATE` (Appendix F). macOS: convert to a non-activating `NSPanel` with `tauri-nspanel`, floating level, joins all spaces, visible over full-screen apps. Linux X11: `focusable(false)`, `skip_taskbar(true)`, `always_on_top(true)`. Wayland: the compositor places the window, accept that.
- **P3.4 Click-through.** `set_ignore_cursor_events(true)` except in views with a button (`Error` with text).
- **P3.5 Placement.** `overlay_position`: `bottom_center` (default), `top_center`, `custom`. Compute against the monitor under the cursor, in logical pixels, 48 px margin. `custom` stores a fraction of the work area and falls back to the default if it is off-screen. Size the window to the pill plus shadow.
- **P3.6 Page.** Rewrite `overlay.html` and `src/overlay-main.ts` in plain TypeScript and CSS, no React, no Jotai. Target under 10 KB of JS. Five bars scaled by `level` events. Honour `prefers-reduced-motion` and `prefers-color-scheme`.
- **P3.7 Views.** Exactly the `session::View` enum. Send with `emit_to("overlay", "overlay-view", view)` and `emit_to("overlay", "overlay-level", f32)`. `Done` hides after 1.2 s when `overlay_preview` is on, at once when off. `NoSpeech` hides after 1.5 s. `Error` with text stays until dismissed or the next recording.
- **P3.8 Tray.** Three icons: idle, recording, processing. Add macOS template variants. The tray is the indicator that never depends on a WebView.
- **P3.9 Cues.** `sound_cues` (default off). Two short embedded PCM clips played through a `cpal` output stream.
- **P3.10 Off switch.** `overlay_enabled` (default true).

**Gate:** the overlay and end-to-end smoke tests in section 8A pass on all three runners.

---

## 8A. CI smoke suite (replaces all manual testing)

GitHub's hosted runners are real Windows and macOS machines with a desktop session. Use them. Add `src-tauri/tests/smoke.rs` and a `smoke` job in `ci.yml` that runs on `windows-latest`, `macos-15` and `ubuntu-22.04` (under `xvfb-run`). Build this suite during P2 and P3 and keep extending it. Every later phase must keep it green.

To make the app testable without a person:

- **Fake audio source.** `LOCALYAPPER_FAKE_AUDIO=/path/to.wav` makes the audio thread play that file into the ring buffer at real-time speed instead of opening a device. Runners have no microphone.
- **Test control channel.** A debug-only command (compiled with `--features test-hooks`, never in release) lets a test send `RecordDown`, `RecordUp` and `Cancel` to the controller and read back the last stage timings and the overlay's last view. This stands in for pressing the hotkey.
- **Text target.** A tiny helper binary (`src-tauri/tests/helpers/text_target`) opens a native window with one focused multi-line text field and prints its content on request. Use a plain Win32 `EDIT` control on Windows, an `NSTextView` on macOS, a GTK `TextView` on Linux. Do not depend on Notepad or TextEdit.

Tests, each run on all three OSes unless noted:

| Test | Asserts |
|---|---|
| `inject_types_into_target` | After `inject("Hello, world.")` the target holds exactly that text |
| `inject_restores_clipboard` | Clipboard text set before the call is back after the restore delay. Repeat with an image on the clipboard |
| `inject_with_modifier_down` | With Alt held by a synthetic key-down, the paste still lands as plain text once the wait in Appendix E completes (send the key-up from a second thread after 150 ms) |
| `overlay_does_not_take_focus` | The foreground window is the text target before `show`, 300 ms after `show`, and after `hide` |
| `overlay_is_click_through` | A synthetic click at the pill's centre reaches the window underneath |
| `overlay_receives_levels` | During a fake-audio recording the overlay page reports at least 10 distinct `overlay-level` values |
| `end_to_end_dictation` | Fake audio of a known sentence, `RecordDown`, wait for the clip length, `RecordUp`. The target holds the expected words (word error rate under 15 %), focus never moved, and `release_to_text_ms` is under 5000 |
| `end_to_end_cold_and_warm` | Same, once right after start (cold model) and once again 2 s later (warm). Record both timings to `docs/PERFORMANCE.md` |
| `cancel_discards` | `RecordDown`, `Cancel`: target stays empty, state returns to idle |
| `back_to_back` | Two dictations with no pause between them both land (proves there is no post-paste lockout) |
| `second_instance_exits` | A second process exits within 3 s and the first keeps running |
| `settings_window_not_created_at_start` | With `setup_complete = true`, no window labelled `main` exists after start |
| `x11_repeat_hold` (Linux) | With xdotool holding the record key for 2 s under Xvfb, exactly one `RecordDown` and one `RecordUp` reach the controller |
| `hotkey_registers` | Each default hotkey registers without error on that OS |

Rules for this suite:

- macOS runners may not grant Accessibility to the test process, in which case synthetic key events are dropped by the OS. Check `AXIsProcessTrusted()` at the start of each test that sends keys. If it is false, print `SKIP: no Accessibility grant on runner`, exit 0, and list the test in `docs/BUILD_REPORT.md`. Do not try to edit the TCC database.
- Cache the model between CI runs with `actions/cache`, keyed on the `models.json` hash.
- A test that needs a display must fail loudly, not hang: give every wait a timeout of 20 s or less.
- Flaky tests are bugs. Fix the cause. Do not add retries, and do not skip or disable a test to get green.

---

## 9. Phase 4: models

### 9.1 Catalog

| Id | sherpa family | Languages | Role |
|---|---|---|---|
| `parakeet-110m` | `nemo_ctc` | English | **Default.** Use the int8 build if `P0.4` finds one and the bench shows its word error rate within 1.0 point of fp32. Otherwise keep fp32 |
| `parakeet-0.6b-v3` | `transducer`, `model_type = "nemo_transducer"`, int8 | 25 European languages, auto-detected | Offered as "Higher accuracy and more languages" on machines with 16 GB RAM or more |
| `moonshine-tiny-en` | `moonshine`, int8 | English | Used on machines with less than 6 GB RAM |

Do not add Whisper, and do not add `parakeet-0.6b-v2`: v3 covers English too. A model is dropped from the catalog if its bench real-time factor on 4 threads is above 0.3, since it would break A3.

- **P4.1 Manifest.** `core/stt/catalog.rs` embeds `models.json` with `include_str!`. Per model: id, name, family, languages, licence, files. Per file: URL pinned to a commit (`resolve/<sha>/<path>`), size, SHA-256.
- **P4.2 Downloader.** One shared `reqwest::Client`. Stream to `<file>.part`, resume with `Range`. A `200` reply to a range request means restart from zero. A `416` means verify what is on disk. Hash while writing. On completion check size and SHA-256, then rename. Progress events use real totals. Check free disk space first. Cancel keeps the `.part` file. Test against a local HTTP server: resume, corruption, cancel, 416.
- **P4.3 Validity.** Installed means every file matches the manifest size and a `.verified` marker written after the hash check is present.
- **P4.4 Automatic choice.** `speech_model = auto` resolves by total RAM: under 6 GB `moonshine-tiny-en`, otherwise `parakeet-110m`. Read RAM with `GlobalMemoryStatusEx`, `sysctl hw.memsize`, or `/proc/meminfo`. No new dependency.
- **P4.5 Models page.** Each model shows size, languages, state and actions (download, use, delete). Switching evicts the old model. Deleting the active model returns the app to "needs download".
- **P4.6 Migration.** An existing `models/parakeet-tdt-ctc-110m/model.onnx` keeps working as a legacy install. If the catalog default becomes int8, show a one-time offer to switch and free the space. Map the removed `parakeet-0.6b` id to `auto`.

**Gate:** downloader tests pass. A fresh profile downloads, verifies, and dictates.

---

## 10. Phase 5: settings

- **P5.1 Typed settings.** `settings/keys.rs` lists every key with type, default and validator. `set_setting` rejects unknown keys and bad values. A cached `Settings` struct sits behind an `RwLock` in `AppState`, so hot paths never query SQLite.
- **P5.2 Apply hooks.** Every write runs the key's hook and emits `settings-changed`. Write one test per key that proves the hook runs.
- **P5.3 Migrations.** Use `PRAGMA user_version`. Version 1 is today's schema. Version 2 adds the new keys, removes `overlay_x` and `overlay_y`, sets `auto_start` to its current OS state.

### Settings

| Page | Key | Values | Default | Hook |
|---|---|---|---|---|
| General | `auto_start` | bool | false, asked in the wizard | autostart plugin with `--autostart` |
| General | `theme` | `system`, `light`, `dark` | `system` | both windows |
| Dictation | `activation_mode` | `hold`, `toggle`, `hybrid` | `hybrid` | `Event::ConfigChanged` |
| Dictation | `hotkey_record`, `hotkey_cancel`, `hotkey_paste_last`, `hotkey_open_app` | combo | per OS | re-register with rollback |
| Dictation | `cancel_key_enabled`, `cancel_key_in_hands_free` | bool | true, false | `Event::ConfigChanged` |
| Dictation | `max_recording_seconds` | 30 to 3600 | 600 | `Event::ConfigChanged` |
| Dictation | `trailing_space` | bool | true | injector |
| Audio | `input_device` | `default` or name | `default` | audio thread |
| Audio | `sound_cues` | bool | false | controller |
| Output | `paste_method` | see `P2.6` | `auto` | injector |
| Output | `restore_clipboard` | bool | true | injector |
| Output | `clipboard_restore_delay_ms` | 100 to 2000 | 250 | injector |
| Speech | `speech_model` | `auto` or id | `auto` | evict and reload |
| Speech | `idle_unload_seconds` | 0, 30, 120, 300, 900 | 120 | manager |
| Overlay | `overlay_enabled` | bool | true | overlay back end |
| Overlay | `overlay_position` | `bottom_center`, `top_center`, `custom` | `bottom_center` | overlay back end |
| Overlay | `overlay_preview` | bool | true | overlay back end |
| Privacy | `history_enabled` | bool | true | `Event::ConfigChanged` |
| Privacy | `history_retention_days` | 0, 7, 30, 90 | 0 | prune at start and daily |

- **P5.4 Pages.** Sidebar: Dashboard, History, Dictation, Audio, Speech, General. Reuse the existing components. Every control saves on change and shows the backend error inline.
- **P5.5 Audio page.** Device list, live meter, and a "Test dictation" box that runs the real pipeline and shows the text and stage timings.
- **P5.6 About.** Version, data folder, log folder, "Copy diagnostics" (OS, CPU, RAM, model, last timings, never text), notices, "Reset all settings".
- **P5.7 Wizard.** Welcome, Microphone, Accessibility (macOS), Hotkey, Speech model, Try it, Done (with the launch-at-login checkbox). Retry on every failure. Resume at the saved step.
- **P5.8 History.** Local-time grouping, search, copy, delete, export through a save dialog, clear all. Store `created_at` as UTC ISO-8601 with `Z`.
- **P5.9 Stats.** Pass local day and week boundaries from Rust (`chrono::Local`) into the queries.
- **P5.10 Dark theme** with CSS variables.

**Gate:** every table row has a hook test. No control exists without a backend effect.

---

## 11. Phase 6: memory and size

- **P6.1** Set `"create": false` on the `main` window in `tauri.conf.json` and build it only for the wizard or when the user opens it. Keep destroy-on-close.
- **P6.2** With `--autostart`, never create the settings window.
- **P6.3** Create the overlay at start. Destroy it after 10 minutes without a dictation and recreate it in the background on the next press. Recording never waits for it. Measure the saving with the `P6.6` script on the Windows CI runner: if it is under 30 MB, keep the overlay alive and delete this code.
- **P6.4** No timers, polling or animation while idle. Idle CPU must read 0.0 %.
- **P6.6** `scripts/measure-memory.ps1` and `.sh` print private memory of the main process and each WebView helper in three states: idle and evicted, model loaded, settings open. Run them in CI on the Windows, macOS and Linux runners against the release build, and commit the results to `docs/PERFORMANCE.md`. Replace the unmeasured claims in the README with those numbers.
- **P6.7** Run `cargo bloat --release --crates`. Remove `tauri-plugin-notification`, `futures-util`, `anyhow`, `env_logger` if unused. Trim `chrono` and `tokio` features. Keep `opt-level = 3`.
- **P6.8** Replace the 50 KB logo PNG with an SVG or a smaller PNG.

**Gate:** `docs/PERFORMANCE.md` has measured numbers that meet A7, or explains the gap.

---

## 12. Phase 7: packaging and release

- **P7.1 Single instance.** `tauri-plugin-single-instance` registered first. No arguments: open settings. With a CLI flag: forward to the controller.
- **P7.2 Logging.** `tauri-plugin-log`, two files of 1 MB at most, `info`, or `debug` with `LOCALYAPPER_DEBUG=1`. Never log transcript or clipboard text.
- **P7.3 Autostart.** Remove the forced enable in `lib.rs:198-215`. Follow the setting only.
- **P7.4 Updates.** No auto-updater (it needs a signing key). The About section shows the version and a "Releases" link that opens the GitHub releases page in the browser.
- **P7.5 Bundles.** Windows: NSIS, `installMode: currentUser`, `compression: lzma`, WebView2 `downloadBootstrapper`. macOS: DMG per architecture, plist and entitlements, hardened runtime, minimum 12.0. Linux: deb with `depends` `libwebkit2gtk-4.1-0, libasound2, libxdo3` and `recommends` `wl-clipboard, wtype`, plus rpm and AppImage. All: bundle `silero_vad.onnx` and `THIRD_PARTY_NOTICES.md`.
- **P7.6 CI.** Fail the release if the Windows installer is over 25 MB. Run tests on three OSes. Run the model integration test on Linux with a cached model. Use the tag's own release notes file.
- **P7.7 Unsigned builds.** Builds are unsigned. Put the install steps in the README: on macOS, `xattr -dr com.apple.quarantine /Applications/LocalYapper.app` and re-grant Accessibility after each update. On Windows, "More info", then "Run anyway" on the SmartScreen prompt.
- **P7.8 Notices.** `THIRD_PARTY_NOTICES.md`: Parakeet (NVIDIA, CC-BY-4.0), Moonshine (MIT), Silero VAD (MIT), sherpa-onnx (Apache-2.0), ONNX Runtime (MIT), the test fixtures, and the generated Rust and npm licence lists.
- **P7.9 Docs.** Delete `docs/FINAL_AUDIT.md`, `docs/PRODUCT_AUDIT.md`, `docs/PROJECT_COMPLETION.md`, `docs/RELEASE_NOTES_v0.1.0.md`. Delete `docs/MANUAL_QA.md` too. Rewrite `README.md`, `CLAUDE.md`, `AGENTS.md`, `docs/BUILD.md`. Add `docs/ARCHITECTURE.md`, `docs/PERFORMANCE.md`, `docs/MODEL_BENCHMARK.md`, `docs/BUILD_REPORT.md`.
- **P7.10 Version.** `2.0.0`.

**Gate:** a tag build produces installers for every target.

### Dependency changes

| Change | Task |
|---|---|
| `sherpa-onnx` 1.12 to 2.0 | `P0.2` |
| add `ringbuf = "0.4"` | `P1.3` |
| add `sha2` | `P4.2` |
| add `tauri-plugin-single-instance`, `tauri-plugin-log`, `tauri-plugin-dialog`, `tauri-plugin-opener` | `P7.1`, `P7.2`, `P5.8`, `P7.4` |
| add `tauri-nspanel = "2.1"` and `objc2-app-kit` (macOS), `ashpd` (Linux) | `P3.3`, `P2.7`, `P2.1` |
| `windows` features: add `Win32_UI_Input_KeyboardAndMouse`, `Win32_System_ProcessStatus`, `Win32_Security`, `Win32_System_SystemInformation` | `P2.5`, `P1.13`, `P2.7`, `P4.4` |
| `arboard`: add feature `wayland-data-control` | `P2.5` |
| `enigo`: add feature `wayland` on Linux | `P2.5` |
| remove `env_logger`, and `tauri-plugin-notification`, `futures-util`, `anyhow` if unused | `P6.7` |

Do not add `rubato`, `sysinfo` or `rdev`.

---

## 13. Phase order and stopping points

| Phase | Content | Gate |
|---|---|---|
| P0 | Baseline, upgrade, bench, manifest script, CI | CI green |
| P1 | Audio, segmenter, engine, controller | Headless pipeline test |
| P2 | Hotkeys, injection, permissions | Injection smoke tests on three OSes |
| P3 | Overlay, tray, cues, smoke suite | Smoke suite green on three OSes |
| P4 | Catalog, downloader, models UI | Downloader tests |
| P5 | Settings, pages, wizard, history | Hook test per key |
| P6 | Window lifetimes, eviction, measurement, size | Measured numbers |
| P7 | Packaging, logging, docs | Installers built in CI |
| P8 | Final run of every acceptance test, `docs/BUILD_REPORT.md` | All CI green, report written |

Do all phases. Finish only when P8 is done.

---

## 14. Verification commands

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

```bash
LOCALYAPPER_TEST_MODEL_DIR=/path/to/model cargo test --manifest-path src-tauri/Cargo.toml -- --include-ignored pipeline
```

Building the Rust crate needs network access to github.com release assets for the sherpa-onnx libraries. If that is blocked, set `SHERPA_ONNX_ARCHIVE_DIR` to a folder holding the archive.

---

## 15. Left out on purpose

| Item | Why |
|---|---|
| LLM clean-up, cloud STT, BYOK | Product decision |
| Learning corrections, personal dictionary | Removed on purpose earlier |
| GPU providers | CPU int8 meets the budget. GPU adds installer size and driver failures |
| Streaming partial text into the target app | Needs a streaming model with worse accuracy, and editing text in a foreign app is fragile |
| Bare modifier or Fn as the hotkey | Needs low-level keyboard hooks on each OS. Can be added later as another `HotkeyBackend` |
| Code signing and notarisation | Need a paid certificate and secrets |
| Auto-updater | Needs a signing key |
| Native overlay with no WebView | Separate drawing code per OS, large risk for a modest saving |
| Starting a new recording while the previous tail is decoding | The window is short. The state machine drops the press |

## 16. Phase 8: final run and report

1. Run section 14 locally and confirm every CI job is green on the final commit.
2. Write `docs/BUILD_REPORT.md` with these sections, and nothing else:
   - **Acceptance results.** One row per criterion A1 to A10: pass or fail, the measured number where there is one, and the CI job that proves it.
   - **Measured performance.** Cold and warm `release_to_text_ms`, idle and loaded memory, installer sizes, per OS, copied from CI.
   - **Model choice.** The catalog as shipped and the bench numbers behind it.
   - **Not exercised by CI.** Every skipped test and every code path no automated test reached (for example a physical microphone, or macOS key events if the runner had no Accessibility grant), one line each.
   - **Decisions taken.** Each place you chose between options the plan left open, one line each.
3. Bump the version to `2.0.0`, commit, push. Do not create a tag, a release or a pull request unless told to.

---

## Appendix A: `core/session.rs` (compiled, 20 tests pass)

Copy verbatim. Do not add I/O to this file.

```rust
//! Dictation state machine. Pure: no I/O, no clocks, no tauri.
//! The controller feeds `Event`s (with a monotonic millisecond timestamp
//! where needed) and executes the returned `Effect`s in order.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActivationMode {
    /// Record while the key is held.
    Hold,
    /// Press to start, press again to stop.
    Toggle,
    /// Hold to record; a quick double-tap latches hands-free.
    Hybrid,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Config {
    pub mode: ActivationMode,
    pub tap_threshold_ms: u64,
    pub double_tap_window_ms: u64,
    pub max_recording_ms: u64,
    pub warning_before_end_ms: u64,
    pub cancel_key_enabled: bool,
    pub cancel_key_in_hands_free: bool,
    pub history_enabled: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            mode: ActivationMode::Hybrid,
            tap_threshold_ms: 250,
            double_tap_window_ms: 350,
            max_recording_ms: 600_000,
            warning_before_end_ms: 15_000,
            cancel_key_enabled: true,
            cancel_key_in_hands_free: false,
            history_enabled: true,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RecKind {
    Hold,
    TapPending,
    HandsFree,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TimerKind {
    TapWindow,
    Warning,
    MaxDuration,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum View {
    Hidden,
    Listening,
    HandsFree,
    StoppingSoon { remaining_ms: u64 },
    Processing,
    Done { text: String },
    NoSpeech,
    Error { message: String, text: Option<String> },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tray {
    Idle,
    Recording,
    Processing,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cue {
    Start,
    Stop,
}

/// Session ids make every late message from an older recording harmless.
pub type SessionId = u64;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Event {
    RecordDown { at_ms: u64 },
    RecordUp { at_ms: u64 },
    /// Cancel hotkey or the `cancel_recording` IPC command.
    Cancel,
    Timer { kind: TimerKind, id: SessionId },
    /// The segmenter closed a speech segment and queued it for decoding.
    SegmentQueued { id: SessionId, index: usize },
    SegmentText { id: SessionId, index: usize, text: String },
    /// Capture stopped and flushed; `segments` is the final segment count.
    CaptureEnded { id: SessionId, segments: usize, audio_ms: u64 },
    CaptureFailed { id: SessionId, message: String },
    ModelFailed { message: String },
    InjectDone { id: SessionId, result: Result<(), String> },
    SetPaused(bool),
    ConfigChanged(Config),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Effect {
    StartCapture { id: SessionId },
    /// Stop the stream, flush the segmenter, then send `CaptureEnded`.
    StopCapture { id: SessionId },
    /// Stop the stream and drop everything queued for this session.
    DiscardCapture { id: SessionId },
    EnsureModelLoaded,
    ScheduleEvict,
    Overlay(View),
    Tray(Tray),
    Cue(Cue),
    RegisterCancelKey,
    UnregisterCancelKey,
    StartTimer { kind: TimerKind, id: SessionId, after_ms: u64 },
    Inject { id: SessionId, text: String },
    SaveHistory { text: String, audio_ms: u64 },
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum State {
    Idle,
    Recording {
        id: SessionId,
        kind: RecKind,
        pressed_at_ms: u64,
        texts: Vec<Option<String>>,
        cancel_registered: bool,
    },
    Finishing {
        id: SessionId,
        texts: Vec<Option<String>>,
        total: Option<usize>,
        audio_ms: u64,
    },
    Injecting {
        id: SessionId,
        text: String,
    },
}

pub struct Session {
    cfg: Config,
    state: State,
    next_id: SessionId,
    paused: bool,
}

impl Session {
    pub fn new(cfg: Config) -> Self {
        Self { cfg, state: State::Idle, next_id: 1, paused: false }
    }

    pub fn is_idle(&self) -> bool {
        self.state == State::Idle
    }

    pub fn is_recording(&self) -> bool {
        matches!(self.state, State::Recording { .. })
    }

    pub fn handle(&mut self, event: Event) -> Vec<Effect> {
        let mut fx = Vec::new();
        match event {
            Event::ConfigChanged(cfg) => self.cfg = cfg,
            Event::SetPaused(p) => {
                self.paused = p;
                if p {
                    self.abort(&mut fx, View::Hidden);
                }
            }
            Event::RecordDown { at_ms } => self.on_down(at_ms, &mut fx),
            Event::RecordUp { at_ms } => self.on_up(at_ms, &mut fx),
            Event::Cancel => self.abort(&mut fx, View::Hidden),
            Event::Timer { kind, id } => self.on_timer(kind, id, &mut fx),
            Event::SegmentQueued { id, index } => {
                if let Some(texts) = self.texts_for(id) {
                    if texts.len() <= index {
                        texts.resize(index + 1, None);
                    }
                }
            }
            Event::SegmentText { id, index, text } => {
                if let Some(texts) = self.texts_for(id) {
                    if texts.len() <= index {
                        texts.resize(index + 1, None);
                    }
                    texts[index] = Some(text);
                }
                self.try_complete(&mut fx);
            }
            Event::CaptureEnded { id, segments, audio_ms } => {
                if let State::Finishing { id: cur, texts, total, audio_ms: a } = &mut self.state {
                    if *cur == id {
                        texts.resize(segments, None);
                        *total = Some(segments);
                        *a = audio_ms;
                    }
                }
                self.try_complete(&mut fx);
            }
            Event::CaptureFailed { id, message } => {
                if self.current_id() == Some(id) && !matches!(self.state, State::Injecting { .. }) {
                    self.abort(&mut fx, View::Error { message, text: None });
                }
            }
            Event::ModelFailed { message } => {
                if matches!(self.state, State::Recording { .. } | State::Finishing { .. }) {
                    self.abort(&mut fx, View::Error { message, text: None });
                }
            }
            Event::InjectDone { id, result } => {
                if let State::Injecting { id: cur, text } = &self.state {
                    if *cur == id {
                        let text = text.clone();
                        self.state = State::Idle;
                        fx.push(Effect::Tray(Tray::Idle));
                        fx.push(Effect::Overlay(match result {
                            Ok(()) => View::Done { text },
                            Err(message) => View::Error { message, text: Some(text) },
                        }));
                        fx.push(Effect::ScheduleEvict);
                    }
                }
            }
        }
        fx
    }

    fn current_id(&self) -> Option<SessionId> {
        match &self.state {
            State::Idle => None,
            State::Recording { id, .. }
            | State::Finishing { id, .. }
            | State::Injecting { id, .. } => Some(*id),
        }
    }

    fn texts_for(&mut self, id: SessionId) -> Option<&mut Vec<Option<String>>> {
        match &mut self.state {
            State::Recording { id: cur, texts, .. } | State::Finishing { id: cur, texts, .. }
                if *cur == id =>
            {
                Some(texts)
            }
            _ => None,
        }
    }

    fn wants_cancel_key(&self, kind: RecKind) -> bool {
        self.cfg.cancel_key_enabled
            && (kind != RecKind::HandsFree || self.cfg.cancel_key_in_hands_free)
    }

    fn on_down(&mut self, at_ms: u64, fx: &mut Vec<Effect>) {
        match &mut self.state {
            State::Idle => {
                if self.paused {
                    fx.push(Effect::Overlay(View::Error {
                        message: "Dictation is paused. Resume from the tray menu.".into(),
                        text: None,
                    }));
                    return;
                }
                let id = self.next_id;
                self.next_id += 1;
                let kind = match self.cfg.mode {
                    ActivationMode::Toggle => RecKind::HandsFree,
                    ActivationMode::Hold | ActivationMode::Hybrid => RecKind::Hold,
                };
                let cancel = self.wants_cancel_key(kind);
                self.state = State::Recording {
                    id,
                    kind,
                    pressed_at_ms: at_ms,
                    texts: Vec::new(),
                    cancel_registered: cancel,
                };
                // Capture first: nothing may delay the microphone.
                fx.push(Effect::StartCapture { id });
                fx.push(Effect::EnsureModelLoaded);
                fx.push(Effect::Tray(Tray::Recording));
                fx.push(Effect::Overlay(View::Listening));
                fx.push(Effect::Cue(Cue::Start));
                if cancel {
                    fx.push(Effect::RegisterCancelKey);
                }
                let max = self.cfg.max_recording_ms;
                let warn = max.saturating_sub(self.cfg.warning_before_end_ms);
                if warn > 0 && warn < max {
                    fx.push(Effect::StartTimer { kind: TimerKind::Warning, id, after_ms: warn });
                }
                fx.push(Effect::StartTimer { kind: TimerKind::MaxDuration, id, after_ms: max });
            }
            State::Recording { kind, cancel_registered, .. } => match *kind {
                // Key auto-repeat while held.
                RecKind::Hold => {}
                RecKind::TapPending => {
                    *kind = RecKind::HandsFree;
                    let keep = self.cfg.cancel_key_enabled && self.cfg.cancel_key_in_hands_free;
                    if *cancel_registered && !keep {
                        *cancel_registered = false;
                        fx.push(Effect::UnregisterCancelKey);
                    }
                    fx.push(Effect::Overlay(View::HandsFree));
                }
                RecKind::HandsFree => self.finish(fx),
            },
            // A press while the tail is decoding or pasting is dropped.
            State::Finishing { .. } | State::Injecting { .. } => {}
        }
    }

    fn on_up(&mut self, at_ms: u64, fx: &mut Vec<Effect>) {
        let State::Recording { id, kind, pressed_at_ms, .. } = &mut self.state else {
            return;
        };
        if *kind != RecKind::Hold {
            return;
        }
        let held = at_ms.saturating_sub(*pressed_at_ms);
        if self.cfg.mode == ActivationMode::Hybrid && held <= self.cfg.tap_threshold_ms {
            *kind = RecKind::TapPending;
            fx.push(Effect::StartTimer {
                kind: TimerKind::TapWindow,
                id: *id,
                after_ms: self.cfg.double_tap_window_ms,
            });
        } else {
            self.finish(fx);
        }
    }

    fn on_timer(&mut self, timer: TimerKind, timer_id: SessionId, fx: &mut Vec<Effect>) {
        let State::Recording { id, kind, .. } = &self.state else {
            return;
        };
        if *id != timer_id {
            return;
        }
        match timer {
            TimerKind::TapWindow => {
                if *kind == RecKind::TapPending {
                    self.abort(fx, View::Hidden);
                }
            }
            TimerKind::Warning => fx.push(Effect::Overlay(View::StoppingSoon {
                remaining_ms: self.cfg.warning_before_end_ms,
            })),
            TimerKind::MaxDuration => self.finish(fx),
        }
    }

    fn finish(&mut self, fx: &mut Vec<Effect>) {
        let State::Recording { id, texts, cancel_registered, .. } = &mut self.state else {
            return;
        };
        let id = *id;
        if *cancel_registered {
            fx.push(Effect::UnregisterCancelKey);
        }
        let texts = std::mem::take(texts);
        self.state = State::Finishing { id, texts, total: None, audio_ms: 0 };
        fx.push(Effect::StopCapture { id });
        fx.push(Effect::Cue(Cue::Stop));
        fx.push(Effect::Tray(Tray::Processing));
        fx.push(Effect::Overlay(View::Processing));
    }

    /// Leave any non-idle, non-injecting state without producing text.
    fn abort(&mut self, fx: &mut Vec<Effect>, view: View) {
        match &self.state {
            State::Idle | State::Injecting { .. } => return,
            State::Recording { id, cancel_registered, .. } => {
                if *cancel_registered {
                    fx.push(Effect::UnregisterCancelKey);
                }
                fx.push(Effect::DiscardCapture { id: *id });
            }
            State::Finishing { id, .. } => fx.push(Effect::DiscardCapture { id: *id }),
        }
        self.state = State::Idle;
        fx.push(Effect::Tray(Tray::Idle));
        fx.push(Effect::Overlay(view));
        fx.push(Effect::ScheduleEvict);
    }

    fn try_complete(&mut self, fx: &mut Vec<Effect>) {
        let State::Finishing { id, texts, total: Some(total), audio_ms } = &self.state else {
            return;
        };
        if texts.len() < *total || texts.iter().take(*total).any(Option::is_none) {
            return;
        }
        let id = *id;
        let audio_ms = *audio_ms;
        let text = join_segments(texts.iter().flatten().map(String::as_str));
        if text.is_empty() {
            self.state = State::Idle;
            fx.push(Effect::Tray(Tray::Idle));
            fx.push(Effect::Overlay(View::NoSpeech));
            fx.push(Effect::ScheduleEvict);
            return;
        }
        // History first, so a failed paste never loses the words.
        if self.cfg.history_enabled {
            fx.push(Effect::SaveHistory { text: text.clone(), audio_ms });
        }
        fx.push(Effect::Inject { id, text: text.clone() });
        self.state = State::Injecting { id, text };
    }
}

/// Join per-segment transcripts with single spaces, dropping empties.
pub fn join_segments<'a>(parts: impl Iterator<Item = &'a str>) -> String {
    let mut out = String::new();
    for part in parts {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        if !out.is_empty() {
            out.push(' ');
        }
        out.push_str(part);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(mode: ActivationMode) -> Session {
        Session::new(Config { mode, ..Config::default() })
    }

    fn has(fx: &[Effect], want: &Effect) -> bool {
        fx.iter().any(|e| e == want)
    }

    /// Drive a recording that is already in Finishing to completion.
    fn complete(sess: &mut Session, id: SessionId, texts: &[&str]) -> Vec<Effect> {
        let mut all = Vec::new();
        for (i, t) in texts.iter().enumerate() {
            all.extend(sess.handle(Event::SegmentQueued { id, index: i }));
            all.extend(sess.handle(Event::SegmentText { id, index: i, text: (*t).into() }));
        }
        all.extend(sess.handle(Event::CaptureEnded { id, segments: texts.len(), audio_ms: 1000 }));
        all
    }

    #[test]
    fn hold_mode_happy_path() {
        let mut x = s(ActivationMode::Hold);
        let fx = x.handle(Event::RecordDown { at_ms: 0 });
        assert_eq!(fx[0], Effect::StartCapture { id: 1 });
        assert_eq!(fx[1], Effect::EnsureModelLoaded);
        assert!(has(&fx, &Effect::Overlay(View::Listening)));
        assert!(has(&fx, &Effect::RegisterCancelKey));

        // Even a very short hold finishes in Hold mode.
        let fx = x.handle(Event::RecordUp { at_ms: 100 });
        assert!(has(&fx, &Effect::StopCapture { id: 1 }));
        assert!(has(&fx, &Effect::UnregisterCancelKey));

        let fx = complete(&mut x, 1, &["Hello there.", " How are you?"]);
        assert!(has(&fx, &Effect::Inject { id: 1, text: "Hello there. How are you?".into() }));
        assert!(has(
            &fx,
            &Effect::SaveHistory { text: "Hello there. How are you?".into(), audio_ms: 1000 }
        ));

        let fx = x.handle(Event::InjectDone { id: 1, result: Ok(()) });
        assert!(has(&fx, &Effect::Overlay(View::Done { text: "Hello there. How are you?".into() })));
        // Idle immediately: no post-inject lockout.
        assert!(x.is_idle());
        let fx = x.handle(Event::RecordDown { at_ms: 5000 });
        assert_eq!(fx[0], Effect::StartCapture { id: 2 });
    }

    #[test]
    fn auto_repeat_down_is_ignored_while_holding() {
        let mut x = s(ActivationMode::Hold);
        x.handle(Event::RecordDown { at_ms: 0 });
        assert!(x.handle(Event::RecordDown { at_ms: 30 }).is_empty());
        assert!(x.is_recording());
    }

    #[test]
    fn up_before_down_is_ignored() {
        let mut x = s(ActivationMode::Hybrid);
        assert!(x.handle(Event::RecordUp { at_ms: 0 }).is_empty());
        assert!(x.is_idle());
    }

    #[test]
    fn hybrid_single_tap_cancels_after_window() {
        let mut x = s(ActivationMode::Hybrid);
        x.handle(Event::RecordDown { at_ms: 0 });
        let fx = x.handle(Event::RecordUp { at_ms: 120 });
        assert_eq!(fx, vec![Effect::StartTimer { kind: TimerKind::TapWindow, id: 1, after_ms: 350 }]);
        let fx = x.handle(Event::Timer { kind: TimerKind::TapWindow, id: 1 });
        assert!(has(&fx, &Effect::DiscardCapture { id: 1 }));
        assert!(has(&fx, &Effect::Overlay(View::Hidden)));
        assert!(x.is_idle());
    }

    #[test]
    fn hybrid_double_tap_latches_then_tap_stops() {
        let mut x = s(ActivationMode::Hybrid);
        x.handle(Event::RecordDown { at_ms: 0 });
        x.handle(Event::RecordUp { at_ms: 100 });
        let fx = x.handle(Event::RecordDown { at_ms: 250 });
        assert!(has(&fx, &Effect::Overlay(View::HandsFree)));
        // Escape goes back to the focused app in hands-free by default.
        assert!(has(&fx, &Effect::UnregisterCancelKey));
        // The stale tap timer and the second release do nothing.
        assert!(x.handle(Event::Timer { kind: TimerKind::TapWindow, id: 1 }).is_empty());
        assert!(x.handle(Event::RecordUp { at_ms: 300 }).is_empty());
        assert!(x.is_recording());
        let fx = x.handle(Event::RecordDown { at_ms: 9000 });
        assert!(has(&fx, &Effect::StopCapture { id: 1 }));
        // The cancel key was already released: not unregistered twice.
        assert!(!has(&fx, &Effect::UnregisterCancelKey));
    }

    #[test]
    fn hybrid_long_hold_finishes_on_release() {
        let mut x = s(ActivationMode::Hybrid);
        x.handle(Event::RecordDown { at_ms: 0 });
        let fx = x.handle(Event::RecordUp { at_ms: 251 });
        assert!(has(&fx, &Effect::StopCapture { id: 1 }));
    }

    #[test]
    fn toggle_mode_press_press() {
        let mut x = s(ActivationMode::Toggle);
        let fx = x.handle(Event::RecordDown { at_ms: 0 });
        assert!(!has(&fx, &Effect::RegisterCancelKey));
        assert!(x.handle(Event::RecordUp { at_ms: 80 }).is_empty());
        assert!(x.is_recording());
        let fx = x.handle(Event::RecordDown { at_ms: 4000 });
        assert!(has(&fx, &Effect::StopCapture { id: 1 }));
    }

    #[test]
    fn cancel_while_recording_discards() {
        let mut x = s(ActivationMode::Hold);
        x.handle(Event::RecordDown { at_ms: 0 });
        let fx = x.handle(Event::Cancel);
        assert!(has(&fx, &Effect::DiscardCapture { id: 1 }));
        assert!(has(&fx, &Effect::UnregisterCancelKey));
        assert!(x.is_idle());
        // Late results from the cancelled session are ignored.
        assert!(x.handle(Event::SegmentText { id: 1, index: 0, text: "x".into() }).is_empty());
        assert!(x
            .handle(Event::CaptureEnded { id: 1, segments: 1, audio_ms: 10 })
            .is_empty());
    }

    #[test]
    fn cancel_while_finishing_discards() {
        let mut x = s(ActivationMode::Hold);
        x.handle(Event::RecordDown { at_ms: 0 });
        x.handle(Event::RecordUp { at_ms: 900 });
        let fx = x.handle(Event::Cancel);
        assert!(has(&fx, &Effect::DiscardCapture { id: 1 }));
        assert!(x.is_idle());
    }

    #[test]
    fn text_arriving_before_and_after_release_in_any_order() {
        let mut x = s(ActivationMode::Hold);
        x.handle(Event::RecordDown { at_ms: 0 });
        x.handle(Event::SegmentQueued { id: 1, index: 0 });
        x.handle(Event::SegmentText { id: 1, index: 0, text: "one".into() });
        x.handle(Event::SegmentQueued { id: 1, index: 1 });
        x.handle(Event::RecordUp { at_ms: 5000 });
        // Capture ends before the last two texts arrive, out of order.
        assert!(x
            .handle(Event::CaptureEnded { id: 1, segments: 3, audio_ms: 5000 })
            .is_empty());
        assert!(x.handle(Event::SegmentText { id: 1, index: 2, text: "three".into() }).is_empty());
        let fx = x.handle(Event::SegmentText { id: 1, index: 1, text: "two".into() });
        assert!(has(&fx, &Effect::Inject { id: 1, text: "one two three".into() }));
    }

    #[test]
    fn no_segments_means_no_speech() {
        let mut x = s(ActivationMode::Hold);
        x.handle(Event::RecordDown { at_ms: 0 });
        x.handle(Event::RecordUp { at_ms: 900 });
        let fx = x.handle(Event::CaptureEnded { id: 1, segments: 0, audio_ms: 900 });
        assert!(has(&fx, &Effect::Overlay(View::NoSpeech)));
        assert!(x.is_idle());
    }

    #[test]
    fn empty_transcripts_mean_no_speech() {
        let mut x = s(ActivationMode::Hold);
        x.handle(Event::RecordDown { at_ms: 0 });
        x.handle(Event::RecordUp { at_ms: 900 });
        let fx = complete(&mut x, 1, &["  ", ""]);
        assert!(has(&fx, &Effect::Overlay(View::NoSpeech)));
        assert!(!fx.iter().any(|e| matches!(e, Effect::Inject { .. })));
    }

    #[test]
    fn max_duration_finishes_and_warning_shows() {
        let mut x = s(ActivationMode::Toggle);
        let fx = x.handle(Event::RecordDown { at_ms: 0 });
        assert!(has(&fx, &Effect::StartTimer { kind: TimerKind::Warning, id: 1, after_ms: 585_000 }));
        assert!(has(
            &fx,
            &Effect::StartTimer { kind: TimerKind::MaxDuration, id: 1, after_ms: 600_000 }
        ));
        let fx = x.handle(Event::Timer { kind: TimerKind::Warning, id: 1 });
        assert_eq!(fx, vec![Effect::Overlay(View::StoppingSoon { remaining_ms: 15_000 })]);
        let fx = x.handle(Event::Timer { kind: TimerKind::MaxDuration, id: 1 });
        assert!(has(&fx, &Effect::StopCapture { id: 1 }));
    }

    #[test]
    fn stale_timers_from_an_old_session_are_ignored() {
        let mut x = s(ActivationMode::Hold);
        x.handle(Event::RecordDown { at_ms: 0 });
        x.handle(Event::Cancel);
        x.handle(Event::RecordDown { at_ms: 1000 });
        assert!(x.handle(Event::Timer { kind: TimerKind::MaxDuration, id: 1 }).is_empty());
        assert!(x.is_recording());
    }

    #[test]
    fn capture_and_model_failures_surface_and_reset() {
        let mut x = s(ActivationMode::Hold);
        x.handle(Event::RecordDown { at_ms: 0 });
        let fx = x.handle(Event::CaptureFailed { id: 1, message: "No microphone".into() });
        assert!(has(
            &fx,
            &Effect::Overlay(View::Error { message: "No microphone".into(), text: None })
        ));
        assert!(x.is_idle());

        x.handle(Event::RecordDown { at_ms: 10 });
        x.handle(Event::RecordUp { at_ms: 900 });
        let fx = x.handle(Event::ModelFailed { message: "Model missing".into() });
        assert!(has(&fx, &Effect::DiscardCapture { id: 2 }));
        assert!(x.is_idle());
        // A model failure while idle is not shown.
        assert!(x.handle(Event::ModelFailed { message: "x".into() }).is_empty());
    }

    #[test]
    fn inject_failure_keeps_text_for_copy() {
        let mut x = s(ActivationMode::Hold);
        x.handle(Event::RecordDown { at_ms: 0 });
        x.handle(Event::RecordUp { at_ms: 900 });
        complete(&mut x, 1, &["keep me"]);
        let fx = x.handle(Event::InjectDone { id: 1, result: Err("blocked".into()) });
        assert!(has(
            &fx,
            &Effect::Overlay(View::Error { message: "blocked".into(), text: Some("keep me".into()) })
        ));
        assert!(x.is_idle());
    }

    #[test]
    fn presses_during_finishing_and_injecting_are_dropped() {
        let mut x = s(ActivationMode::Hold);
        x.handle(Event::RecordDown { at_ms: 0 });
        x.handle(Event::RecordUp { at_ms: 900 });
        assert!(x.handle(Event::RecordDown { at_ms: 950 }).is_empty());
        complete(&mut x, 1, &["a"]);
        assert!(x.handle(Event::RecordDown { at_ms: 960 }).is_empty());
        // Cancel cannot interrupt a paste that is already in flight.
        assert!(x.handle(Event::Cancel).is_empty());
    }

    #[test]
    fn paused_blocks_start_and_pausing_aborts() {
        let mut x = s(ActivationMode::Hold);
        x.handle(Event::RecordDown { at_ms: 0 });
        let fx = x.handle(Event::SetPaused(true));
        assert!(has(&fx, &Effect::DiscardCapture { id: 1 }));
        let fx = x.handle(Event::RecordDown { at_ms: 10 });
        assert!(matches!(fx.as_slice(), [Effect::Overlay(View::Error { .. })]));
        x.handle(Event::SetPaused(false));
        assert_eq!(x.handle(Event::RecordDown { at_ms: 20 })[0], Effect::StartCapture { id: 2 });
    }

    #[test]
    fn history_can_be_disabled() {
        let mut x = Session::new(Config {
            mode: ActivationMode::Hold,
            history_enabled: false,
            ..Config::default()
        });
        x.handle(Event::RecordDown { at_ms: 0 });
        x.handle(Event::RecordUp { at_ms: 900 });
        let fx = complete(&mut x, 1, &["secret"]);
        assert!(!fx.iter().any(|e| matches!(e, Effect::SaveHistory { .. })));
        assert!(has(&fx, &Effect::Inject { id: 1, text: "secret".into() }));
    }

    #[test]
    fn join_trims_and_skips_empty() {
        assert_eq!(join_segments(["  a ", "", "b", "   "].into_iter()), "a b");
        assert_eq!(join_segments(std::iter::empty()), "");
    }
}
```

## Appendix B: `core/audio/rolling.rs` (compiled, 2 tests pass)

```rust
//! Rolling 16 kHz buffer addressed by absolute sample index.
//!
//! The VAD reports each speech segment as (absolute start, length). This
//! buffer keeps recent audio so a segment can be cut with pre- and
//! post-roll padding, and forgets audio that can no longer be needed, so
//! memory stays flat however long the recording runs.

use std::collections::VecDeque;

pub struct RollingBuffer {
    buf: VecDeque<f32>,
    /// Absolute index of `buf[0]`.
    base: u64,
}

impl RollingBuffer {
    pub fn new() -> Self {
        Self { buf: VecDeque::new(), base: 0 }
    }

    /// Absolute index one past the newest sample.
    pub fn end(&self) -> u64 {
        self.base + self.buf.len() as u64
    }

    pub fn len(&self) -> usize {
        self.buf.len()
    }

    pub fn is_empty(&self) -> bool {
        self.buf.is_empty()
    }

    pub fn push(&mut self, samples: &[f32]) {
        self.buf.extend(samples.iter().copied());
    }

    /// Copy `[start - pre, start + len + post)`, clamped to what is held.
    /// `floor` is the lowest index allowed, used to stop the pre-roll from
    /// reaching back into the previous segment.
    pub fn slice_padded(&self, start: u64, len: u64, pre: u64, post: u64, floor: u64) -> Vec<f32> {
        let lo = start.saturating_sub(pre).max(floor).max(self.base);
        let hi = (start + len + post).min(self.end());
        if hi <= lo {
            return Vec::new();
        }
        let a = (lo - self.base) as usize;
        let b = (hi - self.base) as usize;
        self.buf.range(a..b).copied().collect()
    }

    /// Forget everything before absolute index `keep_from`.
    pub fn drop_before(&mut self, keep_from: u64) {
        let n = keep_from.saturating_sub(self.base).min(self.buf.len() as u64) as usize;
        self.buf.drain(..n);
        self.base += n as u64;
    }
}

impl Default for RollingBuffer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ramp(from: u32, to: u32) -> Vec<f32> {
        (from..to).map(|v| v as f32).collect()
    }

    #[test]
    fn slices_with_padding_and_clamps() {
        let mut r = RollingBuffer::new();
        r.push(&ramp(0, 100));
        assert_eq!(r.slice_padded(10, 5, 3, 2, 0), ramp(7, 17));
        // Pre-roll clamps at the start, post-roll at the end.
        assert_eq!(r.slice_padded(1, 2, 10, 0, 0), ramp(0, 3));
        assert_eq!(r.slice_padded(95, 5, 0, 50, 0), ramp(95, 100));
        // Floor stops pre-roll overlapping the previous segment.
        assert_eq!(r.slice_padded(10, 5, 8, 0, 6), ramp(6, 15));
    }

    #[test]
    fn drop_before_keeps_absolute_indexing() {
        let mut r = RollingBuffer::new();
        r.push(&ramp(0, 100));
        r.drop_before(40);
        assert_eq!(r.len(), 60);
        assert_eq!(r.end(), 100);
        r.push(&ramp(100, 120));
        assert_eq!(r.slice_padded(50, 3, 0, 0, 0), ramp(50, 53));
        // Asking for forgotten audio returns what is left.
        assert_eq!(r.slice_padded(30, 15, 0, 0, 0), ramp(40, 45));
        assert!(r.slice_padded(0, 10, 0, 0, 0).is_empty());
        // Dropping past the end or backwards is safe.
        r.drop_before(10);
        assert_eq!(r.len(), 80);
        r.drop_before(10_000);
        assert!(r.is_empty());
        assert_eq!(r.end(), 120);
    }
}
```

## Appendix C: `platform/hotkey/repeat_filter.rs` (compiled, 4 tests pass)

```rust
//! Filter for X11 key auto-repeat.
//!
//! On X11 a held key arrives as Release/Press pairs a few milliseconds
//! apart. The `global-hotkey` crate forwards both, so hold-to-talk would
//! stop after the keyboard repeat delay. This filter holds each Release
//! back for a short window and drops it, together with the Press that
//! follows, when they form a repeat pair.
//!
//! Call `on_press` / `on_release` from the hotkey callback, and
//! `poll(now_ms)` from a timer (10 ms) or on the next event. All three
//! return the events to forward.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key {
    Down { at_ms: u64 },
    Up { at_ms: u64 },
}

pub struct RepeatFilter {
    window_ms: u64,
    pending_up: Option<u64>,
    down: bool,
}

impl RepeatFilter {
    /// 30 ms is far above the gap inside a repeat pair and far below a
    /// deliberate double-tap.
    pub fn new(window_ms: u64) -> Self {
        Self { window_ms, pending_up: None, down: false }
    }

    pub fn on_press(&mut self, now_ms: u64) -> Vec<Key> {
        let mut out = self.poll(now_ms);
        if self.pending_up.take().is_some() {
            // Release then Press inside the window: auto-repeat. Drop both.
            return out;
        }
        if !self.down {
            self.down = true;
            out.push(Key::Down { at_ms: now_ms });
        }
        out
    }

    pub fn on_release(&mut self, now_ms: u64) -> Vec<Key> {
        let out = self.poll(now_ms);
        if self.down {
            self.pending_up = Some(now_ms);
        }
        out
    }

    pub fn poll(&mut self, now_ms: u64) -> Vec<Key> {
        match self.pending_up {
            Some(at) if now_ms.saturating_sub(at) >= self.window_ms => {
                self.pending_up = None;
                self.down = false;
                vec![Key::Up { at_ms: at }]
            }
            _ => Vec::new(),
        }
    }

    /// Milliseconds until `poll` should run again, if a release is held.
    pub fn next_deadline(&self) -> Option<u64> {
        self.pending_up.map(|at| at + self.window_ms)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn real_press_and_release_pass_through() {
        let mut f = RepeatFilter::new(30);
        assert_eq!(f.on_press(0), vec![Key::Down { at_ms: 0 }]);
        assert!(f.on_release(500).is_empty());
        assert!(f.poll(520).is_empty());
        // The Up keeps its true time, so tap-length maths stays right.
        assert_eq!(f.poll(530), vec![Key::Up { at_ms: 500 }]);
    }

    #[test]
    fn auto_repeat_pairs_are_swallowed() {
        let mut f = RepeatFilter::new(30);
        assert_eq!(f.on_press(0), vec![Key::Down { at_ms: 0 }]);
        for t in [500u64, 533, 566, 600] {
            assert!(f.on_release(t).is_empty());
            assert!(f.on_press(t + 1).is_empty());
        }
        assert!(f.on_release(700).is_empty());
        assert_eq!(f.poll(731), vec![Key::Up { at_ms: 700 }]);
    }

    #[test]
    fn double_tap_survives() {
        let mut f = RepeatFilter::new(30);
        assert_eq!(f.on_press(0), vec![Key::Down { at_ms: 0 }]);
        assert!(f.on_release(100).is_empty());
        // The second press arrives 150 ms later: flush the Up, then Down.
        assert_eq!(
            f.on_press(250),
            vec![Key::Up { at_ms: 100 }, Key::Down { at_ms: 250 }]
        );
    }

    #[test]
    fn stray_release_is_ignored() {
        let mut f = RepeatFilter::new(30);
        assert!(f.on_release(0).is_empty());
        assert!(f.poll(100).is_empty());
        assert_eq!(f.next_deadline(), None);
    }
}
```

## Appendix D: segmenter and engine (sketches, not compiled)

API names are from section 3. Adjust syntax as the compiler requires. Keep the logic.

### D.1 Segmenter loop

```rust
// One per recording, on its own thread. `rx` is the ring-buffer consumer.
pub fn run_segmenter(
    id: SessionId,
    native_rate: u32,
    mut rx: impl ringbuf::traits::Consumer<Item = f32>,
    stop: Arc<AtomicBool>,          // set by StopCapture
    discard: Arc<AtomicBool>,       // set by DiscardCapture
    vad_model: &Path,
    stt: SttQueue,                  // sends (id, index, Vec<f32>) to the STT worker
    events: Sender<Event>,
    levels: impl Fn(f32),
) -> Result<(), String> {
    const RATE: u64 = 16_000;
    const PRE: u64 = RATE / 5;          // 200 ms
    const POST: u64 = RATE * 15 / 100;  // 150 ms
    const WINDOW: usize = 512;

    let resampler = if native_rate == 16_000 {
        None
    } else {
        Some(LinearResampler::create(native_rate as i32, 16_000).ok_or("resampler")?)
    };
    let vad = VoiceActivityDetector::create(&vad_config(vad_model), 30.0).ok_or("vad")?;

    let mut rolling = RollingBuffer::new();
    let mut pending: Vec<f32> = Vec::new();   // 16 kHz samples not yet given to the VAD
    let mut native = vec![0.0f32; 4096];
    let mut fed: u64 = 0;                     // samples given to the VAD so far
    let mut prev_end: u64 = 0;                // end of the previous segment
    let mut index = 0usize;
    let mut total_16k: u64 = 0;

    loop {
        if discard.load(Ordering::Relaxed) {
            return Ok(());
        }
        let stopping = stop.load(Ordering::Relaxed);
        let n = rx.pop_slice(&mut native);
        if n == 0 && !stopping {
            std::thread::sleep(Duration::from_millis(5));
            continue;
        }

        let chunk: Vec<f32> = match &resampler {
            Some(r) => r.resample(&native[..n], stopping && n == 0),
            None => native[..n].to_vec(),
        };
        levels(rms(&chunk));
        total_16k += chunk.len() as u64;
        rolling.push(&chunk);
        pending.extend_from_slice(&chunk);

        while pending.len() >= WINDOW {
            vad.accept_waveform(&pending[..WINDOW]);
            pending.drain(..WINDOW);
            fed += WINDOW as u64;
            emit_ready(&vad, &rolling, &mut prev_end, &mut index, id, &stt, &events, PRE, POST);
        }
        // Keep only what a future segment's pre-roll could need.
        rolling.drop_before(prev_end.max(fed.saturating_sub(RATE * 20)));

        if stopping && n == 0 {
            if !pending.is_empty() {
                pending.resize(WINDOW, 0.0);
                vad.accept_waveform(&pending);
            }
            vad.flush();
            emit_ready(&vad, &rolling, &mut prev_end, &mut index, id, &stt, &events, PRE, POST);
            // P1.7: safety net when the VAD found nothing in audible audio.
            if index == 0 && total_16k > RATE * 4 / 10 && rolling_rms(&rolling) > 0.01 {
                let all = rolling.slice_padded(rolling.end() - rolling.len() as u64,
                                               rolling.len() as u64, 0, 0, 0);
                stt.push(id, 0, all);
                let _ = events.send(Event::SegmentQueued { id, index: 0 });
                index = 1;
            }
            let _ = events.send(Event::CaptureEnded {
                id, segments: index, audio_ms: total_16k * 1000 / RATE,
            });
            return Ok(());
        }
    }
}

fn emit_ready(vad: &VoiceActivityDetector, rolling: &RollingBuffer, prev_end: &mut u64,
              index: &mut usize, id: SessionId, stt: &SttQueue, events: &Sender<Event>,
              pre: u64, post: u64) {
    while !vad.is_empty() {
        if let Some(seg) = vad.front() {
            let start = seg.start().max(0) as u64;
            let len = seg.samples().len() as u64;
            let audio = rolling.slice_padded(start, len, pre, post, *prev_end);
            // If the rolling buffer no longer holds it, fall back to the VAD's copy.
            let audio = if audio.is_empty() { seg.samples().to_vec() } else { audio };
            *prev_end = start + len;
            stt.push(id, *index, audio);
            let _ = events.send(Event::SegmentQueued { id, index: *index });
            *index += 1;
        }
        vad.pop();
    }
}
```

Note the ring buffer carries native-rate samples, so `SegmentQueued` is always sent before the STT worker can answer with `SegmentText` for the same index. Send `SegmentQueued` before `stt.push` if your queue can run the decode synchronously.

`vad_config`: `silero_vad { model, threshold: 0.4, min_silence_duration: 0.35, min_speech_duration: 0.15, max_speech_duration: 15.0, window_size: 512 }`, `sample_rate: 16000`, `num_threads: 1`, `provider: "cpu"`.

### D.2 Engine config per family

```rust
pub enum Family { NemoCtc, NemoTransducer, Moonshine }

pub fn recognizer_config(family: Family, dir: &Path, threads: i32) -> OfflineRecognizerConfig {
    let p = |name: &str| Some(dir.join(name).to_string_lossy().into_owned());
    let mut cfg = OfflineRecognizerConfig::default();
    cfg.model_config.num_threads = threads;
    cfg.model_config.provider = Some("cpu".into());
    cfg.model_config.tokens = p("tokens.txt");
    cfg.decoding_method = Some("greedy_search".into());
    match family {
        Family::NemoCtc => {
            // File name comes from the manifest: model.int8.onnx or model.onnx.
            cfg.model_config.nemo_ctc.model = p("model.int8.onnx");
        }
        Family::NemoTransducer => {
            cfg.model_config.transducer.encoder = p("encoder.int8.onnx");
            cfg.model_config.transducer.decoder = p("decoder.int8.onnx");
            cfg.model_config.transducer.joiner = p("joiner.int8.onnx");
            cfg.model_config.model_type = Some("nemo_transducer".into());
        }
        Family::Moonshine => {
            // Take the exact file names from the manifest. v1 uses preprocessor,
            // encoder, uncached_decoder, cached_decoder. v2 uses encoder and merged_decoder.
            cfg.model_config.moonshine.preprocessor = p("preprocess.onnx");
            cfg.model_config.moonshine.encoder = p("encode.int8.onnx");
            cfg.model_config.moonshine.uncached_decoder = p("uncached_decode.int8.onnx");
            cfg.model_config.moonshine.cached_decoder = p("cached_decode.int8.onnx");
        }
    }
    cfg
}

pub fn transcribe(rec: &OfflineRecognizer, samples: &[f32]) -> Result<String, String> {
    if samples.len() < 1_600 {            // under 0.1 s
        return Ok(String::new());
    }
    let stream = rec.create_stream();
    stream.accept_waveform(16_000, samples);
    rec.decode(&stream);
    Ok(stream.get_result().ok_or("no result")?.text.trim().to_string())
}
```

Store file names in the manifest and read them from there. Do not hard-code them as above in the final code.

## Appendix E: injection sequences (sketch, not compiled)

Common sequence, run on a blocking thread:

1. Wait until Ctrl, Alt, Shift and the OS key are all up, polling every 10 ms for at most 400 ms. Windows: `GetAsyncKeyState` for `VK_CONTROL`, `VK_MENU`, `VK_SHIFT`, `VK_LWIN`, `VK_RWIN`. macOS: `CGEventSourceFlagsState(kCGEventSourceStateHIDSystemState)`. X11: skip the wait if no cheap query is available and sleep 60 ms instead.
2. If `restore_clipboard`: save `get_text()`, and if that fails `get().image()`. Keep whichever succeeds.
3. Set the text, excluded from history:

```rust
#[cfg(windows)] use arboard::SetExtWindows;
#[cfg(target_os = "macos")] use arboard::SetExtApple;
#[cfg(target_os = "linux")] use arboard::SetExtLinux;

clipboard.set().exclude_from_history().text(text)?;
```

4. Sleep 20 ms, then send paste:

```rust
use enigo::{Direction::{Click, Press, Release}, Enigo, Key, Keyboard, Settings};
let mut enigo = Enigo::new(&Settings::default())?;

#[cfg(windows)] {
    enigo.key(Key::Control, Press)?;
    enigo.key(Key::Other(0x56), Click)?;      // VK_V, layout independent
    enigo.key(Key::Control, Release)?;
}
#[cfg(target_os = "macos")] {
    enigo.key(Key::Meta, Press)?;
    enigo.raw(9, Click)?;                     // kVK_ANSI_V
    enigo.key(Key::Meta, Release)?;
}
#[cfg(target_os = "linux")] {
    // Never call enigo.raw() here: it panics with the xdo back end.
    enigo.key(Key::Control, Press)?;
    enigo.key(Key::Unicode('v'), Click)?;
    enigo.key(Key::Control, Release)?;
}
```

   Always release the modifier in a guard so an error between press and release cannot leave Ctrl stuck down. For `ctrl_shift_v` add Shift around the V. For `shift_insert` send Shift plus `Key::Insert`. For `type` call `enigo.text(text)` and skip the clipboard. For `clipboard_only` stop after step 3 and skip step 6.

5. Return `Ok` to the controller now.
6. On a detached thread: sleep `clipboard_restore_delay_ms`, then put the saved content back, but only if the clipboard still holds the dictated text (the user may have copied something else meanwhile).

Wayland: clipboard through arboard with `wayland-data-control`. Keys: try enigo's `wayland` feature, then spawn `wtype -M ctrl v -m ctrl`, then `ydotool key 29:1 47:1 47:0 29:0`. If all fail, fall back to `clipboard_only` and tell the user in the overlay.

## Appendix F: overlay window (sketch, not compiled)

```rust
let overlay = tauri::WebviewWindowBuilder::new(app, "overlay", WebviewUrl::App("overlay.html".into()))
    .decorations(false)
    .transparent(true)
    .shadow(false)
    .resizable(false)
    .always_on_top(true)
    .skip_taskbar(true)
    .visible_on_all_workspaces(true)
    .focusable(false)
    .focused(false)
    .visible(false)
    .inner_size(340.0, 96.0)
    .build()?;
overlay.set_ignore_cursor_events(true)?;
```

Remove the `overlay` entry from `tauri.conf.json` and build it in code as above.

Show without activating. Never call `overlay.show()` on Windows:

```rust
#[cfg(windows)]
fn show_no_activate(w: &tauri::WebviewWindow) -> tauri::Result<()> {
    use windows::Win32::UI::WindowsAndMessaging::*;
    let hwnd = w.hwnd()?;
    unsafe {
        let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
        let _ = SetWindowPos(hwnd, HWND_TOPMOST, 0, 0, 0, 0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_SHOWWINDOW);
    }
    Ok(())
}
```

Position before showing: take the monitor under the cursor (`app.cursor_position()` then `monitor_from_point`, or the primary monitor if that fails), compute the pill rectangle in logical pixels from its work area and scale factor, and call `set_position` with a `LogicalPosition`.

macOS: register `tauri_nspanel::init()` and build the overlay as a panel with `can_become_key_window: false` and `is_floating_panel: true`. Read the crate README for the builder calls that set the non-activating style mask, the level, and the collection behaviour (all spaces, full-screen auxiliary). Show it with the panel's own show method, not with `makeKeyAndOrderFront`.

Page protocol: the page listens for `overlay-view` (the serialised `View`) and `overlay-level` (`f32`). It sends nothing back except `overlay-copy` and `overlay-dismiss` clicks, through `invoke`.

## Appendix G: model manifest script (sketch, not run)

```python
#!/usr/bin/env python3
"""Write src-tauri/models.json with pinned URLs, sizes and SHA-256 values."""
import json, sys, urllib.request

CANDIDATES = {
    "parakeet-110m": {
        "family": "nemo_ctc",
        # Try the int8 repository first, then the fp32 one the app uses today.
        "repos": ["csukuangfj/sherpa-onnx-nemo-parakeet_tdt_ctc_110m-en-36000-int8",
                  "csukuangfj/sherpa-onnx-nemo-parakeet_tdt_ctc_110m-en-36000"],
        "want": ["model.int8.onnx", "model.onnx", "tokens.txt"],
    },
    "parakeet-0.6b-v3": {
        "family": "nemo_transducer",
        "repos": ["csukuangfj/sherpa-onnx-nemo-parakeet-tdt-0.6b-v3-int8"],
        "want": ["encoder.int8.onnx", "decoder.int8.onnx", "joiner.int8.onnx", "tokens.txt"],
    },
    "moonshine-tiny-en": {
        "family": "moonshine",
        "repos": ["csukuangfj/sherpa-onnx-moonshine-tiny-en-int8"],
        "want": None,   # take every .onnx and tokens.txt
    },
}

def get(url):
    with urllib.request.urlopen(url, timeout=30) as r:
        return json.load(r)

out = {}
for model_id, spec in CANDIDATES.items():
    for repo in spec["repos"]:
        try:
            sha = get(f"https://huggingface.co/api/models/{repo}")["sha"]
            tree = get(f"https://huggingface.co/api/models/{repo}/tree/{sha}")
        except Exception as e:
            print(f"skip {repo}: {e}", file=sys.stderr)
            continue
        files = []
        for f in tree:
            name = f["path"]
            wanted = (name in spec["want"]) if spec["want"] else (
                name.endswith(".onnx") or name == "tokens.txt")
            if f.get("type") != "file" or not wanted:
                continue
            lfs = f.get("lfs")
            files.append({
                "name": name,
                "url": f"https://huggingface.co/{repo}/resolve/{sha}/{name}",
                "size": (lfs or f)["size"],
                # LFS files report their SHA-256 as `oid`. Small files do not:
                # download those and hash them here.
                "sha256": lfs["oid"] if lfs else None,
            })
        if files:
            out[model_id] = {"family": spec["family"], "repo": repo, "revision": sha, "files": files}
            break

for m in out.values():
    for f in m["files"]:
        if f["sha256"] is None:
            import hashlib
            with urllib.request.urlopen(f["url"], timeout=60) as r:
                f["sha256"] = hashlib.sha256(r.read()).hexdigest()

json.dump(out, open("src-tauri/models.json", "w"), indent=2)
print(json.dumps({k: sum(f["size"] for f in v["files"]) for k, v in out.items()}, indent=2))
```

If a repository has both `model.int8.onnx` and `model.onnx`, keep only the int8 file in the manifest once the bench confirms it (section 9.1).
