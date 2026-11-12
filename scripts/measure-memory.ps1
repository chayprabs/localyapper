# Private memory of LocalYapper and its WebView2 helpers in the states of
# plan P6.6 (Windows). See scripts/measure-memory.py.
#
#   scripts\measure-memory.ps1 APP_EXE MODEL_DIR OUT_MARKDOWN
#
# APP_EXE: cargo build --release --features test-hooks,tauri/custom-protocol
param(
    [Parameter(Mandatory = $true)][string]$App,
    [Parameter(Mandatory = $true)][string]$ModelDir,
    [Parameter(Mandatory = $true)][string]$Out
)
$ErrorActionPreference = "Stop"
python -c "import psutil" 2>$null
if ($LASTEXITCODE -ne 0) { python -m pip install --quiet psutil }
python (Join-Path $PSScriptRoot "measure-memory.py") $App $ModelDir $Out
exit $LASTEXITCODE
