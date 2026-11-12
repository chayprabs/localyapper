#!/usr/bin/env bash
# Private memory of LocalYapper and its WebView helpers in the states of
# plan P6.6 (Linux and macOS). See scripts/measure-memory.py.
#
#   scripts/measure-memory.sh APP_BINARY MODEL_DIR OUT_MARKDOWN
#
# APP_BINARY: cargo build --release --features test-hooks,tauri/custom-protocol
# On Linux run it inside a display (xvfb-run) and a D-Bus session.
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
python3 -c "import psutil" 2>/dev/null || python3 -m pip install --quiet psutil
exec python3 "$here/measure-memory.py" "$@"
