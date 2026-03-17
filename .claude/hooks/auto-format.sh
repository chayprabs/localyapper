#!/bin/bash
# Auto-format files after Claude Code writes them
# Prettier for TypeScript/JS/CSS, rustfmt for Rust

INPUT=$(cat)
FILE_PATH=$(echo "$INPUT" | jq -r '.tool_input.file_path // .tool_input.path // empty')

if [ -z "$FILE_PATH" ] || [ ! -f "$FILE_PATH" ]; then
  exit 0
fi

EXT="${FILE_PATH##*.}"

case "$EXT" in
  ts|tsx|js|jsx|json|css|html|md)
    npx prettier --write "$FILE_PATH" 2>/dev/null || true
    ;;
  rs)
    rustfmt "$FILE_PATH" 2>/dev/null || true
    ;;
esac

exit 0
