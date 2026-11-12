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
| Idle, model not loaded yet | 13.8 | 91.2 | 105.0 |
| Model loaded (parakeet-110m) | 579.1 | 94.0 | 673.1 |
| Settings window open | 574.8 | 134.9 | 709.7 |
| Idle, model evicted | 50.4 | 101.1 | 151.5 |
| Idle, model evicted, overlay destroyed | 0.0 | 0.0 | 0.0 |

Per process:

- Idle, model not loaded yet: localyapper.exe 13.8, msedgewebview2.exe 37.3, msedgewebview2.exe 2.5, msedgewebview2.exe 11.2, msedgewebview2.exe 12.2, msedgewebview2.exe 7.4, msedgewebview2.exe 20.7
- Model loaded (parakeet-110m): localyapper.exe 579.1, msedgewebview2.exe 37.4, msedgewebview2.exe 2.5, msedgewebview2.exe 11.2, msedgewebview2.exe 12.2, msedgewebview2.exe 7.4, msedgewebview2.exe 23.4
- Settings window open: localyapper.exe 574.8, msedgewebview2.exe 42.6, msedgewebview2.exe 2.5, msedgewebview2.exe 11.3, msedgewebview2.exe 14.8, msedgewebview2.exe 7.5, msedgewebview2.exe 23.4, msedgewebview2.exe 33.1
- Idle, model evicted: localyapper.exe 50.4, msedgewebview2.exe 42.8, msedgewebview2.exe 3.6, msedgewebview2.exe 11.1, msedgewebview2.exe 14.7, msedgewebview2.exe 7.5, msedgewebview2.exe 21.4
- Idle, model evicted, overlay destroyed: localyapper.exe 0.0

Overlay window kept alive costs 151.5 MB (P6.3 threshold: 30 MB).
Idle CPU over 10 s with the model evicted: 0.3 %.
A7 (idle, settings closed, model evicted: main process under 40 MB private, no settings WebView): main process 50.4 MB, settings WebView not alive: not met.

### macOS (macOS-15.8.2-arm64-arm-64bit)

| State | Main process MB | WebView helpers MB | Total MB |
|---|---|---|---|
| Idle, model not loaded yet | 71.4 | 54.9 | 126.3 |
| Model loaded (parakeet-110m) | 829.5 | 67.3 | 896.8 |
| Settings window open | 832.7 | 129.0 | 961.7 |
| Idle, model evicted | 579.4 | 67.0 | 646.3 |
| Idle, model evicted, overlay destroyed | 0.0 | 0.0 | 0.0 |

Per process:

- Idle, model not loaded yet: localyapper 71.4, com.apple.WebKit.WebContent 36.8, com.apple.WebKit.Networking 18.1
- Model loaded (parakeet-110m): localyapper 829.5, com.apple.WebKit.WebContent 49.3, com.apple.WebKit.Networking 18.0
- Settings window open: localyapper 832.7, com.apple.WebKit.WebContent 49.0, com.apple.WebKit.Networking 18.1, com.apple.WebKit.WebContent 61.8
- Idle, model evicted: localyapper 579.4, com.apple.WebKit.WebContent 48.8, com.apple.WebKit.Networking 18.1
- Idle, model evicted, overlay destroyed: localyapper 0.0

Overlay window kept alive costs 646.3 MB (P6.3 threshold: 30 MB).
Idle CPU over 10 s with the model evicted: 0.1 %.
A7 (idle, settings closed, model evicted: main process under 40 MB private, no settings WebView): main process 579.4 MB, settings WebView not alive: not met.

### Linux (Linux-6.8.0-1070-azure-x86_64-with-glibc2.35)

| State | Main process MB | WebView helpers MB | Total MB |
|---|---|---|---|
| Idle, model not loaded yet | 75.5 | 64.6 | 140.1 |
| Model loaded (parakeet-110m) | 617.5 | 146.7 | 764.2 |
| Settings window open | 618.1 | 226.6 | 844.7 |
| Idle, model evicted | 112.6 | 144.3 | 256.9 |
| Idle, model evicted, overlay destroyed | 0.0 | 0.0 | 0.0 |

Per process:

- Idle, model not loaded yet: localyapper 75.5, WebKitNetworkProcess 8.7, WebKitWebProcess 55.9
- Model loaded (parakeet-110m): localyapper 617.5, WebKitNetworkProcess 8.7, WebKitWebProcess 138.0
- Settings window open: localyapper 618.1, WebKitNetworkProcess 8.7, WebKitWebProcess 84.2, WebKitWebProcess 133.6
- Idle, model evicted: localyapper 112.6, WebKitNetworkProcess 8.8, WebKitWebProcess 135.5
- Idle, model evicted, overlay destroyed: localyapper 0.0

Overlay window kept alive costs 256.9 MB (P6.3 threshold: 30 MB).
Idle CPU over 10 s with the model evicted: 0.1 %.
A7 (idle, settings closed, model evicted: main process under 40 MB private, no settings WebView): main process 112.6 MB, settings WebView not alive: not met.
