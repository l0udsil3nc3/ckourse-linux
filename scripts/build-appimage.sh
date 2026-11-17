#!/usr/bin/env bash
# Backwards-compatible shim.
#
# The AppImage build script now lives at the project root:
#   ./build-appimage.sh
#
# This wrapper delegates so existing references (docs, CI, muscle memory)
# keep working.
set -Eeuo pipefail

exec bash "$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)/build-appimage.sh" "$@"
