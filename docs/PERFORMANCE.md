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
| Idle, model not loaded yet | 13.9 | 91.5 | 105.4 |
| Model loaded (parakeet-110m) | 545.4 | 94.2 | 639.6 |
| Settings window open | 543.1 | 136.3 | 679.5 |
| Idle, model evicted | 58.2 | 102.4 | 160.6 |
| Idle, model evicted, overlay destroyed | 58.1 | 0.0 | 58.1 |

Per process:

- Idle, model not loaded yet: localyapper.exe 13.9, msedgewebview2.exe 37.3, msedgewebview2.exe 2.5, msedgewebview2.exe 11.1, msedgewebview2.exe 12.2, msedgewebview2.exe 7.4, msedgewebview2.exe 21.0
- Model loaded (parakeet-110m): localyapper.exe 545.4, msedgewebview2.exe 37.4, msedgewebview2.exe 2.5, msedgewebview2.exe 11.0, msedgewebview2.exe 12.3, msedgewebview2.exe 7.4, msedgewebview2.exe 23.5
- Settings window open: localyapper.exe 543.1, msedgewebview2.exe 44.0, msedgewebview2.exe 2.5, msedgewebview2.exe 11.1, msedgewebview2.exe 14.8, msedgewebview2.exe 7.4, msedgewebview2.exe 23.5, msedgewebview2.exe 33.0
- Idle, model evicted: localyapper.exe 58.2, msedgewebview2.exe 44.2, msedgewebview2.exe 3.5, msedgewebview2.exe 11.0, msedgewebview2.exe 14.7, msedgewebview2.exe 7.4, msedgewebview2.exe 21.6
- Idle, model evicted, overlay destroyed: localyapper.exe 58.1

Overlay window kept alive costs 102.6 MB (P6.3 threshold: 30 MB).
Idle CPU over 10 s with the model evicted: 0.3 %.
A7 (idle, settings closed, model evicted: main process under 40 MB private, no settings WebView): main process 58.2 MB, settings WebView not alive: not met.

### macOS (macOS-15.8.2-arm64-arm-64bit)

| State | Main process MB | WebView helpers MB | Total MB |
|---|---|---|---|
| Idle, model not loaded yet | 16.2 | 16.2 | 32.4 |
| Model loaded (parakeet-110m) | 305.2 | 28.8 | 334.0 |
| Settings window open | 304.8 | 62.0 | 366.8 |
| Idle, model evicted | 29.1 | 21.4 | 50.5 |
| Idle, model evicted, overlay destroyed | 29.1 | 21.4 | 50.5 |

Per process:

- Idle, model not loaded yet: localyapper 16.2, com.apple.WebKit.WebContent 11.1, com.apple.WebKit.Networking 5.1
- Model loaded (parakeet-110m): localyapper 305.2, com.apple.WebKit.WebContent 23.9, com.apple.WebKit.Networking 4.9
- Settings window open: localyapper 304.8, com.apple.WebKit.WebContent 16.7, com.apple.WebKit.Networking 4.9, com.apple.WebKit.WebContent 40.4
- Idle, model evicted: localyapper 29.1, com.apple.WebKit.WebContent 16.5, com.apple.WebKit.Networking 4.9
- Idle, model evicted, overlay destroyed: localyapper 29.1, com.apple.WebKit.WebContent 16.5, com.apple.WebKit.Networking 4.9

Overlay window kept alive costs -0.0 MB (P6.3 threshold: 30 MB).
Idle CPU over 10 s with the model evicted: 0.0 %.
A7 (idle, settings closed, model evicted: main process under 40 MB private, no settings WebView): main process 29.1 MB, settings WebView not alive: met.

### Linux (Linux-6.8.0-1070-azure-x86_64-with-glibc2.35)

| State | Main process MB | WebView helpers MB | Total MB |
|---|---|---|---|
| Idle, model not loaded yet | 77.2 | 65.0 | 142.3 |
| Model loaded (parakeet-110m) | 605.5 | 145.9 | 751.5 |
| Settings window open | 611.6 | 235.9 | 847.5 |
| Idle, model evicted | 122.1 | 144.0 | 266.1 |
| Idle, model evicted, overlay destroyed | 140.9 | 10.0 | 150.8 |

Per process:

- Idle, model not loaded yet: localyapper 77.2, WebKitNetworkProcess 8.8, WebKitWebProcess 56.2
- Model loaded (parakeet-110m): localyapper 605.5, WebKitNetworkProcess 8.8, WebKitWebProcess 137.1
- Settings window open: localyapper 611.6, WebKitNetworkProcess 8.7, WebKitWebProcess 83.7, WebKitWebProcess 143.5
- Idle, model evicted: localyapper 122.1, WebKitNetworkProcess 8.9, WebKitWebProcess 135.2
- Idle, model evicted, overlay destroyed: localyapper 140.9, WebKitNetworkProcess 10.0

Overlay window kept alive costs 115.3 MB (P6.3 threshold: 30 MB).
Idle CPU over 10 s with the model evicted: 0.2 %.
A7 (idle, settings closed, model evicted: main process under 40 MB private, no settings WebView): main process 122.1 MB, settings WebView not alive: not met.
