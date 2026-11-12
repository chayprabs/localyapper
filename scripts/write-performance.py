#!/usr/bin/env python3
"""Assemble docs/PERFORMANCE.md from the per-OS sections written by
scripts/measure-memory.py in CI (.github/workflows/perf.yml).

Usage: write-performance.py RESULTS_DIR OUT_MARKDOWN
"""

import sys
from pathlib import Path

ORDER = ["perf-windows.md", "perf-macos.md", "perf-linux.md"]

HEADER = """# Performance

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

"""


def main():
    if len(sys.argv) != 3:
        raise SystemExit(__doc__)
    src, out = Path(sys.argv[1]), Path(sys.argv[2])
    parts = [HEADER, "## Results\n"]
    found = False
    for name in ORDER:
        f = src / name
        if f.exists():
            parts.append(f.read_text(encoding="utf-8").strip() + "\n")
            found = True
        else:
            parts.append(f"### {name[5:-3].capitalize()}\n\nNo result from this runner in the last run.\n")
    if not found:
        raise SystemExit("no results found")
    out.write_text("\n".join(parts), encoding="utf-8")


if __name__ == "__main__":
    main()
