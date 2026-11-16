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
| Idle, model not loaded yet | 14.3 | 92.0 | 106.3 |
| Model loaded (parakeet-110m) | 547.4 | 94.5 | 641.9 |
| Settings window open | 543.1 | 136.1 | 679.2 |
| Idle, model evicted | 59.8 | 102.5 | 162.3 |
| Idle, model evicted, overlay destroyed | 59.9 | 0.0 | 59.9 |

Per process:

- Idle, model not loaded yet: localyapper.exe 14.3, msedgewebview2.exe 37.6, msedgewebview2.exe 2.5, msedgewebview2.exe 11.0, msedgewebview2.exe 12.2, msedgewebview2.exe 7.4, msedgewebview2.exe 21.2
- Model loaded (parakeet-110m): localyapper.exe 547.4, msedgewebview2.exe 37.8, msedgewebview2.exe 2.5, msedgewebview2.exe 11.1, msedgewebview2.exe 12.3, msedgewebview2.exe 7.4, msedgewebview2.exe 23.5
- Settings window open: localyapper.exe 543.1, msedgewebview2.exe 44.3, msedgewebview2.exe 2.5, msedgewebview2.exe 11.1, msedgewebview2.exe 14.7, msedgewebview2.exe 7.5, msedgewebview2.exe 23.5, msedgewebview2.exe 32.5
- Idle, model evicted: localyapper.exe 59.8, msedgewebview2.exe 44.4, msedgewebview2.exe 3.5, msedgewebview2.exe 11.1, msedgewebview2.exe 14.7, msedgewebview2.exe 7.5, msedgewebview2.exe 21.4
- Idle, model evicted, overlay destroyed: localyapper.exe 59.9

Overlay window kept alive costs 102.5 MB (P6.3 threshold: 30 MB).
Idle CPU over 10 s with the model evicted: 0.5 %.
A7 (idle, settings closed, model evicted: main process under 40 MB private, no settings WebView): main process 59.8 MB, settings WebView not alive: not met.

### macOS (macOS-15.8.2-arm64-arm-64bit)

| State | Main process MB | WebView helpers MB | Total MB |
|---|---|---|---|
| Idle, model not loaded yet | 16.3 | 15.7 | 32.0 |
| Model loaded (parakeet-110m) | 402.3 | 25.9 | 428.3 |
| Settings window open | 403.2 | 58.6 | 461.8 |
| Idle, model evicted | 27.4 | 20.6 | 48.0 |
| Idle, model evicted, overlay destroyed | 27.4 | 20.6 | 48.0 |

Per process:

- Idle, model not loaded yet: localyapper 16.3, com.apple.WebKit.WebContent 10.6, com.apple.WebKit.Networking 5.1
- Model loaded (parakeet-110m): localyapper 402.3, com.apple.WebKit.WebContent 21.0, com.apple.WebKit.Networking 5.0
- Settings window open: localyapper 403.2, com.apple.WebKit.WebContent 16.2, com.apple.WebKit.Networking 5.0, com.apple.WebKit.WebContent 37.4
- Idle, model evicted: localyapper 27.4, com.apple.WebKit.WebContent 15.6, com.apple.WebKit.Networking 5.0
- Idle, model evicted, overlay destroyed: localyapper 27.4, com.apple.WebKit.WebContent 15.6, com.apple.WebKit.Networking 5.0

Overlay window kept alive costs -0.0 MB (P6.3 threshold: 30 MB).
Idle CPU over 10 s with the model evicted: 0.0 %.
A7 (idle, settings closed, model evicted: main process under 40 MB private, no settings WebView): main process 27.4 MB, settings WebView not alive: met.

### Linux (Linux-6.8.0-1070-azure-x86_64-with-glibc2.35)

| State | Main process MB | WebView helpers MB | Total MB |
|---|---|---|---|
| Idle, model not loaded yet | 77.5 | 64.7 | 142.3 |
| Model loaded (parakeet-110m) | 603.2 | 137.5 | 740.7 |
| Settings window open | 605.4 | 196.0 | 801.4 |
| Idle, model evicted | 116.7 | 135.3 | 252.0 |
| Idle, model evicted, overlay destroyed | 135.6 | 10.1 | 145.7 |

Per process:

- Idle, model not loaded yet: localyapper 77.5, WebKitNetworkProcess 8.8, WebKitWebProcess 55.9
- Model loaded (parakeet-110m): localyapper 603.2, WebKitNetworkProcess 8.8, WebKitWebProcess 128.7
- Settings window open: localyapper 605.4, WebKitNetworkProcess 8.8, WebKitWebProcess 75.0, WebKitWebProcess 112.2
- Idle, model evicted: localyapper 116.7, WebKitNetworkProcess 8.9, WebKitWebProcess 126.4
- Idle, model evicted, overlay destroyed: localyapper 135.6, WebKitNetworkProcess 10.1

Overlay window kept alive costs 106.3 MB (P6.3 threshold: 30 MB).
Idle CPU over 10 s with the model evicted: 0.0 %.
A7 (idle, settings closed, model evicted: main process under 40 MB private, no settings WebView): main process 116.7 MB, settings WebView not alive: not met.
