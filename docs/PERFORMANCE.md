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

Release-to-text timings (A3) are in `docs/MODEL_BENCHMARK.md` (decode
speed per model and thread count) and in the `PERF` lines of the CI smoke
suite (`tests/smoke.rs`, end-to-end on each runner).


## Results

### Windows (Windows-2025Server-10.0.26100-SP0)

| State | Main process MB | WebView helpers MB | Total MB |
|---|---|---|---|
| Idle, model not loaded yet | 13.8 | 95.3 | 109.1 |
| Model loaded (parakeet-110m) | 584.7 | 98.1 | 682.8 |
| Settings window open | 580.3 | 142.1 | 722.4 |
| Idle, model evicted | 57.6 | 106.7 | 164.3 |
| Idle, model evicted, overlay destroyed | 57.5 | 0.0 | 57.5 |

Per process:

- Idle, model not loaded yet: localyapper.exe 13.8, msedgewebview2.exe 37.7, msedgewebview2.exe 2.4, msedgewebview2.exe 11.2, msedgewebview2.exe 12.2, msedgewebview2.exe 8.4, msedgewebview2.exe 23.4
- Model loaded (parakeet-110m): localyapper.exe 584.7, msedgewebview2.exe 37.7, msedgewebview2.exe 2.4, msedgewebview2.exe 11.2, msedgewebview2.exe 12.3, msedgewebview2.exe 8.4, msedgewebview2.exe 26.1
- Settings window open: localyapper.exe 580.3, msedgewebview2.exe 44.3, msedgewebview2.exe 2.4, msedgewebview2.exe 11.3, msedgewebview2.exe 14.8, msedgewebview2.exe 8.4, msedgewebview2.exe 26.1, msedgewebview2.exe 34.8
- Idle, model evicted: localyapper.exe 57.6, msedgewebview2.exe 44.5, msedgewebview2.exe 3.5, msedgewebview2.exe 11.2, msedgewebview2.exe 14.8, msedgewebview2.exe 8.4, msedgewebview2.exe 24.3
- Idle, model evicted, overlay destroyed: localyapper.exe 57.5

Overlay window kept alive costs 106.8 MB (P6.3 threshold: 30 MB).
Idle CPU over 10 s with the model evicted: 0.5 %.
A7 (idle, settings closed, model evicted: main process under 40 MB private, no settings WebView): main process 57.6 MB, settings WebView not alive: not met.

### macOS (macOS-15.8.2-arm64-arm-64bit)

| State | Main process MB | WebView helpers MB | Total MB |
|---|---|---|---|
| Idle, model not loaded yet | 70.7 | 55.0 | 125.7 |
| Model loaded (parakeet-110m) | 735.7 | 67.2 | 802.9 |
| Settings window open | 739.0 | 127.1 | 866.1 |
| Idle, model evicted | 426.1 | 153.3 | 579.4 |
| Idle, model evicted, overlay destroyed | 0.0 | 51.3 | 51.3 |

Per process:

- Idle, model not loaded yet: localyapper 70.7, com.apple.WebKit.WebContent 36.7, com.apple.WebKit.Networking 18.3
- Model loaded (parakeet-110m): localyapper 735.7, com.apple.WebKit.WebContent 49.0, com.apple.WebKit.Networking 18.2
- Settings window open: localyapper 739.0, com.apple.WebKit.WebContent 48.7, com.apple.WebKit.Networking 18.2, com.apple.WebKit.WebContent 60.2
- Idle, model evicted: localyapper 426.1, com.apple.WebKit.WebContent 48.5, com.apple.WebKit.Networking 18.3, com.apple.WebKit 0.0, com.apple.WebKit 0.0, com.apple.WebKit.WebContent 35.6, com.apple.WebKit.WebContent 35.2, com.apple.WebKit.Networking 15.7
- Idle, model evicted, overlay destroyed: localyapper 0.0, com.apple.WebKit.WebContent 35.6, com.apple.WebKit.Networking 15.6

Overlay window kept alive costs 528.2 MB (P6.3 threshold: 30 MB).
Idle CPU over 10 s with the model evicted: 0.1 %.
A7 (idle, settings closed, model evicted: main process under 40 MB private, no settings WebView): main process 426.1 MB, settings WebView not alive: not met.

### Linux (Linux-6.8.0-1070-azure-x86_64-with-glibc2.35)

| State | Main process MB | WebView helpers MB | Total MB |
|---|---|---|---|
| Idle, model not loaded yet | 77.6 | 65.6 | 143.2 |
| Model loaded (parakeet-110m) | 617.3 | 147.4 | 764.7 |
| Settings window open | 629.6 | 238.3 | 867.9 |
| Idle, model evicted | 121.4 | 145.0 | 266.4 |
| Idle, model evicted, overlay destroyed | 140.3 | 10.0 | 150.3 |

Per process:

- Idle, model not loaded yet: localyapper 77.6, WebKitNetworkProcess 9.3, WebKitWebProcess 56.3
- Model loaded (parakeet-110m): localyapper 617.3, WebKitNetworkProcess 9.2, WebKitWebProcess 138.3
- Settings window open: localyapper 629.6, WebKitNetworkProcess 8.8, WebKitWebProcess 84.1, WebKitWebProcess 145.4
- Idle, model evicted: localyapper 121.4, WebKitNetworkProcess 9.2, WebKitWebProcess 135.8
- Idle, model evicted, overlay destroyed: localyapper 140.3, WebKitNetworkProcess 10.0

Overlay window kept alive costs 116.1 MB (P6.3 threshold: 30 MB).
Idle CPU over 10 s with the model evicted: 0.1 %.
A7 (idle, settings closed, model evicted: main process under 40 MB private, no settings WebView): main process 121.4 MB, settings WebView not alive: not met.
