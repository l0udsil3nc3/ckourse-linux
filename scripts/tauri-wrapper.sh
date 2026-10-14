#!/usr/bin/env bash
set -euo pipefail
ROOT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

if [[ "${1:-}" == "build" ]]; then
  shift
  exec bash scripts/build-appimage.sh "$@"
fi

exec ./node_modules/.bin/tauri "$@"
