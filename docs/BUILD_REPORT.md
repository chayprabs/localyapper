# Build Report

The rebuild described in `docs/REBUILD_PLAN.md`, phases 0 to 8, on branch
`claude/optimistic-brahmagupta-lhsm71`. Every number below comes from a
CI run on GitHub's runners (`ubuntu-22.04`, `windows-latest`,
`macos-15`); none is estimated.

## Acceptance results

| # | Criterion | Result | Measured | Proven by |
|---|---|---|---|---|
| A1 | Press starts capture within 150 ms, overlay within 150 ms | Pass for capture; overlay checked at 300 ms, not timed | `capture_start_ms` 9–50 ms (Linux, Windows smoke); 16–241 ms (macOS back-to-back app log, CI run 43234942888) | `smoke` (`end_to_end_*`, PERF lines); `overlay_does_not_take_focus` and `overlay_released_and_recreated` assert the overlay is shown 300 ms after the press |
| A2 | Overlay waveform follows the real level | Pass | ≥ 10 distinct levels in 2.5 s of fixture audio | `core::audio::level` unit tests; `smoke::overlay_receives_levels` on all three OSes |
| A3 | Release to text under 2 s median, under 5 s worst case | Pass | Linux cold 66 ms / warm 63 ms / 5 s clip 137 ms; Windows cold 31 ms / warm 29 ms / 5 s clip 183 ms. Worst case (cold, long speech, 4 cores): model load 1.76 s happens during speech; the tail segment is at most ~16 s and decodes at RTF 0.026, about 0.4 s | `smoke::end_to_end_cold_and_warm`, `end_to_end_dictation`; `docs/MODEL_BENCHMARK.md` (bench workflow) |
| A4 | Focus never leaves the target app | Pass on Linux and Windows; skipped on macOS | Same foreground pid before, during and after | `smoke::overlay_does_not_take_focus`, `end_to_end_dictation`, `back_to_back` (back_to_back also passed on macOS) |
| A5 | Clipboard unchanged after dictation | Pass on Linux (Xvfb) and Windows; skipped on macOS | Previous clipboard text restored | `smoke::inject_restores_clipboard` |
| A6 | Every setting takes effect at once and survives restart | Pass | One hook test per key (28 keys), validators reject bad values; values are stored in SQLite and loaded into the cache at start | `settings::tests::hook_*` and `every_key_has_a_hook_test`; `db::schema` migration tests |
| A7 | Idle, evicted, settings closed: main process under 40 MB private, no settings WebView | **Pass on macOS, fail on Windows and Linux** | Main process after eviction: macOS 27.0 MB, Windows 58.7 MB, Linux 116.4 MB; no settings WebView alive on any OS. The gap is explained in `docs/PERFORMANCE.md` | Memory workflow (`.github/workflows/perf.yml`) → `docs/PERFORMANCE.md` |
| A8 | Windows installer under 25 MB; models downloaded on first run and checksum-verified | Pass | Windows installer 8.4 MB; SHA-256 of every model file checked, resume after interruption | Release workflow size check (run 43243078632); `core::download` tests; `tests/fresh_install.rs` (pipeline job: an empty profile downloads Moonshine, verifies and dictates) |
| A9 | A second launch focuses the running instance | Pass | Second process exits, first keeps running | `smoke::second_instance_exits` on all three OSes |
| A10 | Works on Windows, macOS, Linux X11; Wayland with documented limits | Pass on all three runners (16/15/15 smoke tests); Wayland not exercised | CI runs 43243078600 (`958b906`) and 43284990517 (`c314f29`): every job green | CI workflow; Wayland limits documented in the README |

## Measured performance

Release to text (debug build under test, `parakeet-110m`, from the smoke
PERF lines of CI run 43243078600):

| OS | Cold `release_to_text_ms` | Warm | 5 s clip (cold) | Model ready on press (cold) |
|---|---|---|---|---|
| Linux (Xvfb) | 66 | 63 | 137 | 827 ms |
| Windows | 31 | 29 | 183 | 1730 ms |
| macOS | not measured: the runner keeps another app in front, so the timed tests skip | | | |

Private memory of the release build, main process (memory workflow run
43284990625 on commit `c314f29`; WebView helpers in `docs/PERFORMANCE.md`):

| OS | Idle, model never loaded | Model loaded | Settings open | Idle, model evicted | Overlay released (10 min idle) |
|---|---|---|---|---|---|
| Windows | 13.8 MB | 546.9 MB | 543.6 MB | 58.7 MB | 58.8 MB (helpers 0 MB, were 104 MB) |
| Linux | 75.6 MB | 597.4 MB | 605.6 MB | 116.4 MB | 134.8 MB (helpers 10 MB, were 145 MB) |
| macOS | 16.2 MB | 297.3 MB | 297.7 MB | 27.0 MB | kept alive (see Decisions); helpers 21 MB |

Idle CPU with the model evicted: 0.6 % (Windows), 0.0 % (Linux), 0.1 %
(macOS) over 10 s. macOS figures are the physical footprint (what
Activity Monitor shows); Windows private bytes; Linux USS.

Installers (release workflow on the branch, run 43243078632, commit
`958b906`, all four builds green):

| Bundle | Size |
|---|---|
| Windows NSIS `LocalYapper_2.0.0_x64-setup.exe` | 8.4 MB (limit 25 MB) |
| macOS DMG, Apple Silicon | 11.6 MB |
| macOS DMG, Intel | 12.9 MB |
| Linux deb | 17.5 MB |
| Linux rpm | 17.5 MB |
| Linux AppImage | 90.5 MB |

Speech models are not bundled; they are downloaded on first run.

Decode speed and accuracy per model: see Model choice.

## Model choice

Catalog as shipped (`src-tauri/models.json`, every file pinned by
SHA-256, downloaded on demand):

| Id | Family | Download | Languages | Chosen when |
|---|---|---|---|---|
| `parakeet-110m` | NeMo CTC (fp32) | 458 MB | English | Default (`auto`) |
| `parakeet-0.6b-v3` | NeMo transducer (int8) | 671 MB | 25 European | User choice |
| `moonshine-tiny-en` | Moonshine (int8) | 124 MB | English | `auto` on machines with less than 6 GB RAM |

Bench on the Linux runner (AMD EPYC 7763, 4 vCPU), ten LibriSpeech
test-clean clips (5 s to 102 s, 3.3 min in all), through the same VAD
segmenter as the app (`docs/MODEL_BENCHMARK.md`):

| Model | Threads | Load ms | RTF | Peak RSS MB | WER % |
|---|---|---|---|---|---|
| parakeet-110m | 2 | 1743 | 0.023 | 761 | 2.48 |
| parakeet-110m | 4 | 1757 | 0.026 | 752 | 2.48 |
| parakeet-0.6b-v3 | 2 | 2968 | 0.084 | 1475 | 2.10 |
| parakeet-0.6b-v3 | 4 | 2819 | 0.103 | 1107 | 2.10 |
| moonshine-tiny-en | 2 | 684 | 0.022 | 338 | 4.01 |
| moonshine-tiny-en | 4 | 728 | 0.043 | 334 | 4.01 |

Parakeet 110M stays the default: 2.48 % WER at a quarter of the 0.6B
model's memory and load time. The 0.6B model is the most accurate and
the only multilingual one. Moonshine Tiny is the low-RAM choice. All
three are far below the plan's RTF limit of 0.3. No int8 build of
Parakeet 110M exists in the sherpa-onnx (k2-fsa/csukuangfj)
repositories, so the default is fp32.

## Not exercised by CI

- A physical microphone: every CI dictation feeds a fixture through
  `LOCALYAPPER_FAKE_AUDIO`; cpal device capture, device switching and
  the "microphone denied" path are not run.
- macOS synthetic key events from the app into another app, focus
  retention, clipboard restore, overlay focus and click-through: the
  runner keeps another app in front, so `end_to_end_dictation`,
  `end_to_end_cold_and_warm`, `inject_types_into_target`,
  `inject_restores_clipboard`, `inject_with_modifier_down`,
  `overlay_does_not_take_focus`, `overlay_can_be_dragged` (formerly `overlay_is_click_through`) and
  `test_dictation_reports_without_pasting` print `SKIP:` there.
  (`back_to_back` did run and pass on macOS.)
- macOS Accessibility and microphone permission prompts, and the
  re-grant after an update.
- Wayland: the GlobalShortcuts portal back end, wtype/ydotool paste and
  wl-clipboard (the Linux runner is X11 under Xvfb).
- The X11 repeat filter is exercised only on Linux (`x11_repeat_hold`).
- The settings window UI: pages, wizard, history and dark theme are
  type-checked, linted and built; their backend commands are unit- or
  smoke-tested, but no test clicks through the React UI.
- History export's save dialog (the dialog itself; the export text is
  unit-tested).
- The 10-minute overlay idle timer itself (the release it triggers is
  smoke-tested through the test channel).
- Launch at login: the OS autostart entry is not checked; only the
  setting, its hook (unit test) and `--autostart` not creating the
  settings window (smoke test).
- Sound cues playback, the tray menu, `localyapper --toggle/--start/
  --stop/--cancel` from a second process, paste methods other than the
  default per OS, and elevated (administrator) target windows on Windows.
- Model download from Hugging Face is exercised for Moonshine only
  (fresh install test); Parakeet files are fetched by the CI scripts
  with the same SHA-256 check.
- Installers are built but not installed or launched; builds are
  unsigned.
- `parakeet-0.6b-v3` and `moonshine-tiny-en` in the full app (bench and
  fresh-install only).

## Decisions taken

- Work happened on branch `claude/optimistic-brahmagupta-lhsm71`, the branch that holds the plan, as instructed; no tag, release or pull request was created.
- The build container could not reach huggingface.co or GitHub release assets (proxy 403). Locally the crate was built against the shared sherpa-onnx libraries from the `sherpa-onnx-core` PyPI wheel (`--no-default-features --features sherpa-shared`); CI builds the default static configuration, and everything that needs a speech model ran only on the GitHub runners.
- Windows code was type-checked locally with `cargo clippy --target x86_64-pc-windows-gnu` (`DOCS_RS=1` skips the sherpa download); macOS code was checked only in CI.
- Appendices A-C are byte-for-byte; their `mod` lines carry #[rustfmt::skip] because rustfmt would rewrite them.
- Tasks whose code shares one file or one atomic switch were committed together with every id in the message (P1.1+P1.3, P1.4+P1.6+P1.7, P1.10+P1.14, P1.11+P1.13).
- Fixtures: LibriSpeech test-clean built on a runner (fixtures.yml): six 16 kHz 16-bit clips, one 44.1 kHz stereo 16-bit, one 48 kHz 32-bit float (covers "one 16-bit" alongside float input), one with a 3 s pause, one ~90 s join.
- models.json layout is {schema, vad, models{id: {name, family, languages, license, repo, revision, files}}}; Appendix G's flat map was extended.
- Parakeet 110M int8: no sherpa-onnx (csukuangfj/k2-fsa) int8 repository exists; a third-party OpenVINO re-upload was rejected, so the default stays fp32.
- bench.yml is also a reusable workflow, called by manifest.yml and fixtures.yml, because commits pushed with GITHUB_TOKEN do not start workflows.
- Audio ring buffer is created on the audio thread once the device rate is known (plan sketch passed a producer into Start).
- Stop drops the stream on the audio thread and only then sets the segmenter's stopped flag, so the segmenter never finishes before the last callback.
- STT worker serialises load, decode and evict jobs on one queue; DecodeFailed answers with empty text so a session cannot hang.
- Energy fallback detector threshold 0.015 RMS per 32 ms window.
- Silero max_speech_duration (15 s) is soft: one unbroken 16.4 s fixture utterance stays one segment; tests bound segments at 20 s.
- The start/stop/run_pipeline/inject_text Rust commands were removed in P1.14 with their implementations; TS wrappers go in P2.9.
- React overlay's 120 s constants are removed with the React overlay in P3.6.
- P1 gate (CI run 43190659120): parakeet-110m through VAD segmenter, overall WER 2.48 % over 524 words; clip10_long 15 segments 2.1 %.
- Phase 3 commits were pushed together with the last P2 fix; the P2 injection tests are part of the same smoke run, so both gates were judged on that run.
- P5.1: overlay position "custom" needs a place to store the point; added key overlay_custom ("x,y" fractions 0..1) next to overlay_position. Bookkeeping keys (setup_complete, setup_step, sidebar_collapsed, legacy_model_notice) are typed with a Store-only hook.
- P5.3: v2 migration replaces invalid stored values with defaults (e.g. idle_unload_seconds "60" -> "120", speech_model "parakeet-0.6b" -> "auto"); auto_start is copied from the OS state only for upgraded installs (fresh installs keep false until the wizard asks).
- P5.4: the plan's table has Output, Overlay and Privacy groups but the sidebar has six pages; Output and Overlay are sections of Dictation, Privacy is a section of General, About sits at the bottom of General.
- P5.5: the live meter runs only during a test dictation (no microphone capture while idle; P6.4 forbids idle work and audio stays in memory). The test runs the real pipeline in a controller test mode that reports text instead of pasting, saves no history and does not count as a dictation.
- P5.8: export writes plain text (local time, app, text) chosen through a Rust-side save dialog (tauri-plugin-dialog), so no frontend dialog permission is granted. Retention pruning runs at start and after each saved dictation instead of a daily timer (P6.4: no timers while idle).
- P5.10: theme tokens are RGB channels; Tailwind "black" maps to the ink variable so existing text-black/xx classes invert in dark mode. Accent colours are unchanged.
- P3 gate (Linux): the app exited with GTK's silent _exit(1) after an X I/O error; cause was tauri-runtime-wry building Monitor values (GDK calls) on the controller thread when placing the overlay. Overlay placement/geometry now run on the main thread.
- P6.1/P6.2: a manual launch of a set-up app stays in the tray (as before); the settings window is built only for the first-run wizard, the tray, the open-app key or a second launch, and never with --autostart (the autostart entry now passes --autostart).
- P6.4: audited; nothing polls while idle (the X11 repeat poller runs only while a key is held, fake audio only exists under test-hooks, the overlay spinner only animates while shown). The memory workflow samples idle CPU to confirm.
- P6.6: the measurement driver is Python with psutil (scripts/measure-memory.py); the .sh and .ps1 scripts the plan names are thin wrappers. It drives a release build compiled with the test-hooks feature (the only way to reach the states without a human). macOS WebKit helpers are XPC services, not children; the script counts WebKit processes that appeared after launch.
- P6.7: cargo bloat was run locally on a Linux release build with shared sherpa-onnx, LTO off and symbols kept (the shipped profile strips symbols, which cargo bloat needs). Removed anyhow, env_logger (P7.2) and reqwest's stream feature; futures-util became Linux-only. tauri-plugin-notification was already gone. Largest crate after the app is zbus (Linux D-Bus, pulled in by ashpd/tray), which the Wayland portal needs.
- P6.8: the logo was a 50 KB PNG shown at 14 px; replaced by a 2 KB 61x56 PNG (inlined by Vite).
- P7.6: no tag may be pushed, so the release workflow also builds every installer on the branch (packaging inputs changed or manual dispatch) as artifacts, with the Windows 25 MB check; the tag path additionally runs the full CI and publishes a draft release.
- P7.8: THIRD_PARTY_NOTICES.md lists every crate in Cargo.lock (all platforms) and the shipped npm packages (jotai's optional @babel peers excluded).
- Smoke harness fixes on the way to green: test processes start with CREATE_NO_WINDOW on Windows; the Windows text target no longer un-maximizes when focused; the macOS text target got an Edit menu so Cmd+V pastes; the Linux GTK text target retries the X display briefly.
- P6.3: measured on the runners, keeping the overlay alive costs 107 MB (Windows) and 116 MB (Linux), so the release after 10 idle minutes stays. On macOS the overlay is an NSPanel (tauri-nspanel 2.1) and destroying/rebuilding it aborted the app on the runner, so macOS keeps the overlay alive.
- Tauri exited the app when its last window closed; exits without an explicit code are now prevented (found by the memory run, whose "overlay destroyed" state read 0 MB).
- A7 is met on macOS but not on Windows or Linux; docs/PERFORMANCE.md explains the gap (the GTK/WebKitGTK baseline on Linux, allocator/ONNX Runtime retention after eviction on Windows and Linux).
- macOS memory is reported as the physical footprint (`proc_pid_rusage`), the figure Activity Monitor shows. USS there also counted freed pages the allocator had marked reusable, which read 556 MB after eviction against a 27 MB footprint.
- P7 gate: no tag may be pushed, so the installers were built by the release workflow on the branch (run 43243078632): NSIS, both DMGs, deb, rpm and AppImage all built.
- Plan section 14 was run locally on the final tree: lint, `tsc`, `cargo fmt --check`, clippy with and without `test-hooks`, 131 unit tests and the two model-free pipeline tests pass; the model pipeline test (`--include-ignored pipeline`) cannot run in this container (no model host reachable) and runs in CI's pipeline job instead.
- Release: at the owner's request the old `main` (v1.0.0) was kept as branch `legacy/v1`, and tag builds now publish the release instead of leaving a draft. A manual run of the release workflow on `main` with a `release_tag` input does the same as pushing that tag.
