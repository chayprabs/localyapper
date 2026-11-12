# Performance

Measured on the GitHub Actions runners by `.github/workflows/perf.yml`
with `scripts/measure-memory.sh` / `scripts/measure-memory.ps1` (plan
P6.6). Do not edit by hand: the workflow rewrites this file.

The app under test is the release build with the `test-hooks` feature,
which only adds a local control channel used to drive the states below.
Audio comes from a fixture file instead of a microphone. Private memory
is USS on Linux and macOS and private bytes on Windows; WebView helpers
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
| Idle, model not loaded yet | 14.3 | 92.1 | 106.4 |
| Model loaded (parakeet-110m) | 585.2 | 95.1 | 680.3 |
| Settings window open | 583.2 | 139.7 | 722.9 |
| Idle, model evicted | 57.9 | 105.1 | 163.1 |
| Idle, model evicted, overlay destroyed | 58.3 | 0.0 | 58.3 |

Per process:

- Idle, model not loaded yet: localyapper.exe 14.3, msedgewebview2.exe 37.4, msedgewebview2.exe 2.5, msedgewebview2.exe 11.2, msedgewebview2.exe 12.3, msedgewebview2.exe 8.5, msedgewebview2.exe 20.3
- Model loaded (parakeet-110m): localyapper.exe 585.2, msedgewebview2.exe 37.4, msedgewebview2.exe 2.5, msedgewebview2.exe 11.1, msedgewebview2.exe 12.3, msedgewebview2.exe 8.5, msedgewebview2.exe 23.3
- Settings window open: localyapper.exe 583.2, msedgewebview2.exe 44.9, msedgewebview2.exe 2.5, msedgewebview2.exe 11.3, msedgewebview2.exe 14.8, msedgewebview2.exe 8.5, msedgewebview2.exe 23.3, msedgewebview2.exe 34.6
- Idle, model evicted: localyapper.exe 57.9, msedgewebview2.exe 44.9, msedgewebview2.exe 3.5, msedgewebview2.exe 11.2, msedgewebview2.exe 14.8, msedgewebview2.exe 8.5, msedgewebview2.exe 22.3
- Idle, model evicted, overlay destroyed: localyapper.exe 58.3

Overlay window kept alive costs 104.8 MB (P6.3 threshold: 30 MB).
Idle CPU over 10 s with the model evicted: 0.5 %.
A7 (idle, settings closed, model evicted: main process under 40 MB private, no settings WebView): main process 57.9 MB, settings WebView not alive: not met.

### macOS (macOS-15.8.2-arm64-arm-64bit)

| State | Main process MB | WebView helpers MB | Total MB |
|---|---|---|---|
| Idle, model not loaded yet | 71.4 | 83.4 | 154.8 |
| Model loaded (parakeet-110m) | 737.3 | 94.0 | 831.3 |
| Settings window open | 740.8 | 154.0 | 894.7 |
| Idle, model evicted | 556.3 | 65.2 | 621.5 |
| Idle, model evicted, overlay destroyed | 556.4 | 65.2 | 621.6 |

Per process:

- Idle, model not loaded yet: localyapper 71.4, com.apple.WebKit 0.0, com.apple.WebKit 0.0, com.apple.WebKit.WebContent 28.4, com.apple.WebKit.WebContent 36.5, com.apple.WebKit.Networking 18.4
- Model loaded (parakeet-110m): localyapper 737.3, com.apple.WebKit.WebContent 28.4, com.apple.WebKit.WebContent 47.3, com.apple.WebKit.Networking 18.3
- Settings window open: localyapper 740.8, com.apple.WebKit.WebContent 28.4, com.apple.WebKit.WebContent 47.1, com.apple.WebKit.Networking 18.4, com.apple.WebKit.WebContent 60.1
- Idle, model evicted: localyapper 556.3, com.apple.WebKit.WebContent 46.9, com.apple.WebKit.Networking 18.3
- Idle, model evicted, overlay destroyed: localyapper 556.4, com.apple.WebKit.WebContent 46.9, com.apple.WebKit.Networking 18.3

Overlay window kept alive costs -0.0 MB (P6.3 threshold: 30 MB).
Idle CPU over 10 s with the model evicted: 0.1 %.
A7 (idle, settings closed, model evicted: main process under 40 MB private, no settings WebView): main process 556.3 MB, settings WebView not alive: not met.

### Linux (Linux-6.8.0-1070-azure-x86_64-with-glibc2.35)

| State | Main process MB | WebView helpers MB | Total MB |
|---|---|---|---|
| Idle, model not loaded yet | 75.4 | 65.1 | 140.5 |
| Model loaded (parakeet-110m) | 613.2 | 147.0 | 760.1 |
| Settings window open | 625.3 | 242.3 | 867.6 |
| Idle, model evicted | 117.3 | 144.4 | 261.7 |
| Idle, model evicted, overlay destroyed | 136.1 | 10.1 | 146.2 |

Per process:

- Idle, model not loaded yet: localyapper 75.4, WebKitNetworkProcess 8.8, WebKitWebProcess 56.3
- Model loaded (parakeet-110m): localyapper 613.2, WebKitNetworkProcess 8.8, WebKitWebProcess 138.2
- Settings window open: localyapper 625.3, WebKitNetworkProcess 8.7, WebKitWebProcess 84.3, WebKitWebProcess 149.3
- Idle, model evicted: localyapper 117.3, WebKitNetworkProcess 8.8, WebKitWebProcess 135.5
- Idle, model evicted, overlay destroyed: localyapper 136.1, WebKitNetworkProcess 10.1

Overlay window kept alive costs 115.5 MB (P6.3 threshold: 30 MB).
Idle CPU over 10 s with the model evicted: 0.1 %.
A7 (idle, settings closed, model evicted: main process under 40 MB private, no settings WebView): main process 117.3 MB, settings WebView not alive: not met.
