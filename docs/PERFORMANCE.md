# Performance

Measured on the GitHub Actions runners by `.github/workflows/perf.yml`
with `scripts/measure-memory.sh` / `scripts/measure-memory.ps1` (plan
P6.6). Do not edit by hand: the workflow rewrites this file.

The app under test is the release build with the `test-hooks` feature,
which only adds a local control channel used to drive the states below.
Audio comes from a fixture file instead of a microphone. Private memory
is USS on Linux, private bytes on Windows and the physical footprint on
macOS (what Activity Monitor shows; falls back to USS where it cannot be
read); WebView helpers
are the app's child processes (on macOS, the WebKit processes started
with the app).

States:

1. Idle, model not loaded yet: eight seconds after start.
2. Model loaded: after one dictation of a 5 s clip with `parakeet-110m`.
3. Settings window open.
4. Idle, model evicted: settings window closed and the model unloaded by
   the idle timer (30 s). This is the state acceptance criterion A7
   describes.
5. The same with the overlay window destroyed, to price keeping the
   overlay alive (plan P6.3).

Acceptance criterion A7 asks for under 40 MB private memory in the main
process when idle with the model evicted. Where the tables show more:

- On Linux the main process is already about 75 MB before any model is
  loaded: that is the GTK/WebKitGTK UI side of the process with the
  overlay WebView, before LocalYapper's own work.
- After eviction the main process stays above its pre-load size (on
  Windows by about 44 MB). The recogniser is dropped and the allocators are asked to
  return free memory (mimalloc `mi_collect`, glibc `malloc_trim`, macOS
  `malloc_zone_pressure_relief`, Windows `HeapCompact`), but part of what
  ONNX Runtime and sherpa-onnx allocated during the session is not
  returned to the OS.
- Ten minutes after the last dictation the overlay window is released
  on Windows and Linux (state 5 shows what that saves); on macOS the
  overlay panel is kept, so state 5 equals state 4 there.

Release-to-text timings (A3) are in `docs/MODEL_BENCHMARK.md` (decode
speed per model and thread count) and in the `PERF` lines of the CI smoke
suite (`tests/smoke.rs`, end-to-end on each runner).


## Results

### Windows (Windows-2025Server-10.0.26100-SP0)

| State | Main process MB | WebView helpers MB | Total MB |
|---|---|---|---|
| Idle, model not loaded yet | 13.8 | 92.8 | 106.6 |
| Model loaded (parakeet-110m) | 546.9 | 95.5 | 642.5 |
| Settings window open | 543.6 | 136.8 | 680.4 |
| Idle, model evicted | 58.7 | 104.0 | 162.7 |
| Idle, model evicted, overlay destroyed | 58.8 | 0.0 | 58.8 |

Per process:

- Idle, model not loaded yet: localyapper.exe 13.8, msedgewebview2.exe 37.2, msedgewebview2.exe 2.4, msedgewebview2.exe 12.2, msedgewebview2.exe 11.1, msedgewebview2.exe 8.4, msedgewebview2.exe 21.4
- Model loaded (parakeet-110m): localyapper.exe 546.9, msedgewebview2.exe 37.3, msedgewebview2.exe 2.4, msedgewebview2.exe 12.3, msedgewebview2.exe 11.0, msedgewebview2.exe 8.4, msedgewebview2.exe 24.1
- Settings window open: localyapper.exe 543.6, msedgewebview2.exe 43.9, msedgewebview2.exe 2.4, msedgewebview2.exe 14.8, msedgewebview2.exe 11.2, msedgewebview2.exe 8.4, msedgewebview2.exe 24.1, msedgewebview2.exe 32.0
- Idle, model evicted: localyapper.exe 58.7, msedgewebview2.exe 43.9, msedgewebview2.exe 3.5, msedgewebview2.exe 14.7, msedgewebview2.exe 11.1, msedgewebview2.exe 8.4, msedgewebview2.exe 22.3
- Idle, model evicted, overlay destroyed: localyapper.exe 58.8

Overlay window kept alive costs 103.9 MB (P6.3 threshold: 30 MB).
Idle CPU over 10 s with the model evicted: 0.6 %.
A7 (idle, settings closed, model evicted: main process under 40 MB private, no settings WebView): main process 58.7 MB, settings WebView not alive: not met.

### macOS (macOS-15.8.2-arm64-arm-64bit)

| State | Main process MB | WebView helpers MB | Total MB |
|---|---|---|---|
| Idle, model not loaded yet | 16.2 | 16.1 | 32.3 |
| Model loaded (parakeet-110m) | 297.3 | 28.5 | 325.8 |
| Settings window open | 297.7 | 61.5 | 359.2 |
| Idle, model evicted | 27.0 | 20.6 | 47.6 |
| Idle, model evicted, overlay destroyed | 27.1 | 44.5 | 71.5 |

Per process:

- Idle, model not loaded yet: localyapper 16.2, com.apple.WebKit.WebContent 11.0, com.apple.WebKit.Networking 5.1
- Model loaded (parakeet-110m): localyapper 297.3, com.apple.WebKit.WebContent 23.6, com.apple.WebKit.Networking 4.9
- Settings window open: localyapper 297.7, com.apple.WebKit.WebContent 16.3, com.apple.WebKit.Networking 4.9, com.apple.WebKit.WebContent 40.3
- Idle, model evicted: localyapper 27.0, com.apple.WebKit.WebContent 15.8, com.apple.WebKit.Networking 4.8
- Idle, model evicted, overlay destroyed: localyapper 27.1, com.apple.WebKit.WebContent 15.8, com.apple.WebKit.Networking 4.8, com.apple.WebKit 0.0, com.apple.WebKit 0.0, com.apple.WebKit.WebContent 9.8, com.apple.WebKit.WebContent 10.0, com.apple.WebKit.Networking 4.1

Overlay window kept alive costs -23.9 MB (P6.3 threshold: 30 MB).
Idle CPU over 10 s with the model evicted: 0.1 %.
A7 (idle, settings closed, model evicted: main process under 40 MB private, no settings WebView): main process 27.0 MB, settings WebView not alive: met.

### Linux (Linux-6.8.0-1070-azure-x86_64-with-glibc2.35)

| State | Main process MB | WebView helpers MB | Total MB |
|---|---|---|---|
| Idle, model not loaded yet | 75.6 | 64.9 | 140.5 |
| Model loaded (parakeet-110m) | 597.4 | 146.9 | 744.3 |
| Settings window open | 605.6 | 227.1 | 832.7 |
| Idle, model evicted | 116.4 | 145.0 | 261.4 |
| Idle, model evicted, overlay destroyed | 134.8 | 10.0 | 144.7 |

Per process:

- Idle, model not loaded yet: localyapper 75.6, WebKitNetworkProcess 8.7, WebKitWebProcess 56.2
- Model loaded (parakeet-110m): localyapper 597.4, WebKitNetworkProcess 8.8, WebKitWebProcess 138.1
- Settings window open: localyapper 605.6, WebKitNetworkProcess 8.7, WebKitWebProcess 84.6, WebKitWebProcess 133.8
- Idle, model evicted: localyapper 116.4, WebKitNetworkProcess 8.8, WebKitWebProcess 136.2
- Idle, model evicted, overlay destroyed: localyapper 134.8, WebKitNetworkProcess 10.0

Overlay window kept alive costs 116.7 MB (P6.3 threshold: 30 MB).
Idle CPU over 10 s with the model evicted: 0.0 %.
A7 (idle, settings closed, model evicted: main process under 40 MB private, no settings WebView): main process 116.4 MB, settings WebView not alive: not met.
