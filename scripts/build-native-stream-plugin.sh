#!/usr/bin/env bash
set -euo pipefail

if [ "$(uname -s)" != Linux ]; then
  echo "Native GStreamer stream plugin is Linux-only; skipping."
  exit 0
fi

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "$ROOT/scripts/native-stream-plugin.lock"

CACHE="${XDG_CACHE_HOME:-$HOME/.cache}/ckourse-native-stream"
PLUGIN_DIR="$CACHE/$PLUGIN_COMMIT"
PLUGIN_FILE="$PLUGIN_DIR/target/release/libgsttauriasset.so"
SOURCE_FILE="$PLUGIN_DIR/src/tauri_asset/imp.rs"
DEST="$ROOT/src-tauri/resources/gstreamer/libgsttauriasset.so"

mkdir -p "$CACHE"

if [ ! -d "$PLUGIN_DIR/.git" ]; then
  git clone "$PLUGIN_URL" "$PLUGIN_DIR"
fi

git -C "$PLUGIN_DIR" fetch origin "$PLUGIN_COMMIT"

if [ "$(git -C "$PLUGIN_DIR" rev-parse HEAD)" != "$PLUGIN_COMMIT" ]; then
  git -C "$PLUGIN_DIR" checkout --detach "$PLUGIN_COMMIT"
fi

if [ ! -f "$PLUGIN_FILE" ] ||
   ! grep -Fq 'const ASSET_URI_SCHEME: &str = "stream";' "$SOURCE_FILE"; then

  git -C "$PLUGIN_DIR" reset --hard "$PLUGIN_COMMIT"

  python3 - "$SOURCE_FILE" <<'PY'
from pathlib import Path
import sys

p = Path(sys.argv[1])
text = p.read_text()
old = 'const ASSET_URI_SCHEME: &str = "asset";'
new = 'const ASSET_URI_SCHEME: &str = "stream";'

if old in text:
    text = text.replace(old, new, 1)
elif new not in text:
    sys.exit("Could not locate the expected URI scheme; aborting.")

p.write_text(text)
print("Patched the pinned plugin: asset -> stream")
PY

  cargo build --release --locked --manifest-path "$PLUGIN_DIR/Cargo.toml"
fi

test -f "$PLUGIN_FILE"
install -D -m 755 "$PLUGIN_FILE" "$DEST"

echo "Built plugin SHA-256:"
sha256sum "$DEST"

# Stage the FFmpeg and FFprobe sidecars required by the Tauri configuration.
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TARGET="$(rustc -vV | sed -n 's/^host: //p')"

FFMPEG_SRC="$(node -p 'require("ffmpeg-static")' 2>/dev/null)"
FFPROBE_SRC="$(node -p 'require("ffprobe-static").path' 2>/dev/null)"

if [ ! -f "$FFMPEG_SRC" ] || [ ! -f "$FFPROBE_SRC" ]; then
    echo "ERROR: FFmpeg/FFprobe packages are missing or their files cannot be found." >&2
    exit 1
fi

mkdir -p "$ROOT/src-tauri/binaries"
install -m 755 "$FFMPEG_SRC" "$ROOT/src-tauri/binaries/ffmpeg-$TARGET"
install -m 755 "$FFPROBE_SRC" "$ROOT/src-tauri/binaries/ffprobe-$TARGET"

echo "Staged FFmpeg sidecars for $TARGET"
