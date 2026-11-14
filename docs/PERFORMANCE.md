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
| Idle, model not loaded yet | 14.9 | 92.5 | 107.3 |
| Model loaded (parakeet-110m) | 545.2 | 95.3 | 640.5 |
| Settings window open | 542.3 | 137.2 | 679.6 |
| Idle, model evicted | 58.6 | 103.2 | 161.8 |
| Idle, model evicted, overlay destroyed | 59.3 | 0.0 | 59.3 |

Per process:

- Idle, model not loaded yet: localyapper.exe 14.9, msedgewebview2.exe 37.3, msedgewebview2.exe 2.5, msedgewebview2.exe 11.0, msedgewebview2.exe 12.3, msedgewebview2.exe 7.4, msedgewebview2.exe 22.0
- Model loaded (parakeet-110m): localyapper.exe 545.2, msedgewebview2.exe 37.4, msedgewebview2.exe 2.5, msedgewebview2.exe 11.0, msedgewebview2.exe 12.4, msedgewebview2.exe 7.4, msedgewebview2.exe 24.6
- Settings window open: localyapper.exe 542.3, msedgewebview2.exe 44.0, msedgewebview2.exe 2.5, msedgewebview2.exe 11.1, msedgewebview2.exe 14.8, msedgewebview2.exe 7.5, msedgewebview2.exe 24.6, msedgewebview2.exe 32.8
- Idle, model evicted: localyapper.exe 58.6, msedgewebview2.exe 44.0, msedgewebview2.exe 3.5, msedgewebview2.exe 11.1, msedgewebview2.exe 14.8, msedgewebview2.exe 7.5, msedgewebview2.exe 22.4
- Idle, model evicted, overlay destroyed: localyapper.exe 59.3

Overlay window kept alive costs 102.5 MB (P6.3 threshold: 30 MB).
Idle CPU over 10 s with the model evicted: 0.4 %.
A7 (idle, settings closed, model evicted: main process under 40 MB private, no settings WebView): main process 58.6 MB, settings WebView not alive: not met.

### macOS (macOS-15.8.2-arm64-arm-64bit)

| State | Main process MB | WebView helpers MB | Total MB |
|---|---|---|---|
| Idle, model not loaded yet | 16.4 | 15.9 | 32.3 |
| Model loaded (parakeet-110m) | 319.0 | 29.5 | 348.4 |
| Settings window open | 320.6 | 61.6 | 382.1 |
| Idle, model evicted | 25.6 | 20.9 | 46.5 |
| Idle, model evicted, overlay destroyed | 25.7 | 20.9 | 46.6 |

Per process:

- Idle, model not loaded yet: localyapper 16.4, com.apple.WebKit.WebContent 10.9, com.apple.WebKit.Networking 5.0
- Model loaded (parakeet-110m): localyapper 319.0, com.apple.WebKit.WebContent 24.6, com.apple.WebKit.Networking 4.8
- Settings window open: localyapper 320.6, com.apple.WebKit.WebContent 16.7, com.apple.WebKit.Networking 4.8, com.apple.WebKit.WebContent 40.0
- Idle, model evicted: localyapper 25.6, com.apple.WebKit.WebContent 16.0, com.apple.WebKit.Networking 4.8
- Idle, model evicted, overlay destroyed: localyapper 25.7, com.apple.WebKit.WebContent 16.0, com.apple.WebKit.Networking 4.8

Overlay window kept alive costs -0.0 MB (P6.3 threshold: 30 MB).
Idle CPU over 10 s with the model evicted: 0.0 %.
A7 (idle, settings closed, model evicted: main process under 40 MB private, no settings WebView): main process 25.6 MB, settings WebView not alive: met.

### Linux (Linux-6.8.0-1070-azure-x86_64-with-glibc2.35)

| State | Main process MB | WebView helpers MB | Total MB |
|---|---|---|---|
| Idle, model not loaded yet | 77.4 | 65.2 | 142.6 |
| Model loaded (parakeet-110m) | 597.7 | 146.3 | 744.0 |
| Settings window open | 603.9 | 234.0 | 837.9 |
| Idle, model evicted | 118.3 | 143.9 | 262.2 |
| Idle, model evicted, overlay destroyed | 136.8 | 10.2 | 146.9 |

Per process:

- Idle, model not loaded yet: localyapper 77.4, WebKitNetworkProcess 9.4, WebKitWebProcess 55.8
- Model loaded (parakeet-110m): localyapper 597.7, WebKitNetworkProcess 9.3, WebKitWebProcess 137.0
- Settings window open: localyapper 603.9, WebKitNetworkProcess 8.8, WebKitWebProcess 83.2, WebKitWebProcess 142.0
- Idle, model evicted: localyapper 118.3, WebKitNetworkProcess 9.4, WebKitWebProcess 134.6
- Idle, model evicted, overlay destroyed: localyapper 136.8, WebKitNetworkProcess 10.2

Overlay window kept alive costs 115.3 MB (P6.3 threshold: 30 MB).
Idle CPU over 10 s with the model evicted: 0.0 %.
A7 (idle, settings closed, model evicted: main process under 40 MB private, no settings WebView): main process 118.3 MB, settings WebView not alive: not met.
