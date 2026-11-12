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

- On Linux and macOS the main process is already 70 to 80 MB before any
  model is loaded: that is the GTK/WebKitGTK or AppKit/WebKit UI side of
  the process with the overlay WebView, before LocalYapper's own work.
- After eviction the main process stays above its pre-load size on
  every OS. The recogniser is dropped and the allocators are asked to
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
| Idle, model not loaded yet | 13.8 | 92.5 | 106.3 |
| Model loaded (parakeet-110m) | 585.7 | 96.6 | 682.3 |
| Settings window open | 582.1 | 141.6 | 723.7 |
| Idle, model evicted | 57.9 | 105.7 | 163.6 |
| Idle, model evicted, overlay destroyed | 57.9 | 0.0 | 57.9 |

Per process:

- Idle, model not loaded yet: localyapper.exe 13.8, msedgewebview2.exe 36.7, msedgewebview2.exe 2.3, msedgewebview2.exe 12.1, msedgewebview2.exe 10.9, msedgewebview2.exe 7.3, msedgewebview2.exe 23.2
- Model loaded (parakeet-110m): localyapper.exe 585.7, msedgewebview2.exe 36.8, msedgewebview2.exe 2.3, msedgewebview2.exe 12.2, msedgewebview2.exe 10.9, msedgewebview2.exe 7.3, msedgewebview2.exe 27.1
- Settings window open: localyapper.exe 582.1, msedgewebview2.exe 44.4, msedgewebview2.exe 2.3, msedgewebview2.exe 14.6, msedgewebview2.exe 11.0, msedgewebview2.exe 7.3, msedgewebview2.exe 27.1, msedgewebview2.exe 34.8
- Idle, model evicted: localyapper.exe 57.9, msedgewebview2.exe 44.4, msedgewebview2.exe 3.3, msedgewebview2.exe 14.6, msedgewebview2.exe 10.9, msedgewebview2.exe 7.3, msedgewebview2.exe 25.1
- Idle, model evicted, overlay destroyed: localyapper.exe 57.9

Overlay window kept alive costs 105.6 MB (P6.3 threshold: 30 MB).
Idle CPU over 10 s with the model evicted: 0.2 %.
A7 (idle, settings closed, model evicted: main process under 40 MB private, no settings WebView): main process 57.9 MB, settings WebView not alive: not met.

### macOS (macOS-15.8.2-arm64-arm-64bit)

| State | Main process MB | WebView helpers MB | Total MB |
|---|---|---|---|
| Idle, model not loaded yet | 16.3 | 16.0 | 32.3 |
| Model loaded (parakeet-110m) | 380.9 | 28.1 | 409.1 |
| Settings window open | 381.0 | 61.9 | 442.9 |
| Idle, model evicted | 27.4 | 21.2 | 48.6 |
| Idle, model evicted, overlay destroyed | 27.4 | 20.9 | 48.3 |

Per process:

- Idle, model not loaded yet: localyapper 16.3, com.apple.WebKit.WebContent 11.0, com.apple.WebKit.Networking 4.9
- Model loaded (parakeet-110m): localyapper 380.9, com.apple.WebKit.WebContent 23.3, com.apple.WebKit.Networking 4.8
- Settings window open: localyapper 381.0, com.apple.WebKit.WebContent 16.6, com.apple.WebKit.Networking 4.8, com.apple.WebKit.WebContent 40.5
- Idle, model evicted: localyapper 27.4, com.apple.WebKit.WebContent 16.4, com.apple.WebKit.Networking 4.8
- Idle, model evicted, overlay destroyed: localyapper 27.4, com.apple.WebKit.WebContent 16.1, com.apple.WebKit.Networking 4.8

Overlay window kept alive costs 0.3 MB (P6.3 threshold: 30 MB).
Idle CPU over 10 s with the model evicted: 0.2 %.
A7 (idle, settings closed, model evicted: main process under 40 MB private, no settings WebView): main process 27.4 MB, settings WebView not alive: met.

### Linux (Linux-6.8.0-1070-azure-x86_64-with-glibc2.35)

| State | Main process MB | WebView helpers MB | Total MB |
|---|---|---|---|
| Idle, model not loaded yet | 75.0 | 64.9 | 139.8 |
| Model loaded (parakeet-110m) | 612.8 | 143.9 | 756.7 |
| Settings window open | 618.6 | 234.4 | 853.0 |
| Idle, model evicted | 115.4 | 141.3 | 256.7 |
| Idle, model evicted, overlay destroyed | 133.8 | 10.0 | 143.8 |

Per process:

- Idle, model not loaded yet: localyapper 75.0, WebKitNetworkProcess 8.8, WebKitWebProcess 56.1
- Model loaded (parakeet-110m): localyapper 612.8, WebKitNetworkProcess 8.7, WebKitWebProcess 135.2
- Settings window open: localyapper 618.6, WebKitNetworkProcess 8.7, WebKitWebProcess 80.9, WebKitWebProcess 144.8
- Idle, model evicted: localyapper 115.4, WebKitNetworkProcess 8.8, WebKitWebProcess 132.5
- Idle, model evicted, overlay destroyed: localyapper 133.8, WebKitNetworkProcess 10.0

Overlay window kept alive costs 112.9 MB (P6.3 threshold: 30 MB).
Idle CPU over 10 s with the model evicted: 0.1 %.
A7 (idle, settings closed, model evicted: main process under 40 MB private, no settings WebView): main process 115.4 MB, settings WebView not alive: not met.
