#!/usr/bin/env bash
# Builds project-hub-server for the Tauri target and stages it at
# apps/desktop/binaries/project-hub-server, from where the bundle copies it
# next to the app executable (Project Hub.app/Contents/MacOS/project-hub-server).
# BoardDo and other tools find the headless API there.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OUT_DIR="$ROOT/apps/desktop/binaries"
TARGET_DIR="${CARGO_TARGET_DIR:-$ROOT/target}"
TRIPLE="${TAURI_ENV_TARGET_TRIPLE:-}"
EXE=""
case "$TRIPLE" in *windows*) EXE=".exe" ;; esac

build() { # $1 = triple ("" = host)
  if [ -n "$1" ]; then
    cargo build --release -p project-hub-server --manifest-path "$ROOT/Cargo.toml" --target "$1" >&2
    echo "$TARGET_DIR/$1/release/project-hub-server$EXE"
  else
    cargo build --release -p project-hub-server --manifest-path "$ROOT/Cargo.toml" >&2
    echo "$TARGET_DIR/release/project-hub-server$EXE"
  fi
}

mkdir -p "$OUT_DIR"
if [ "$TRIPLE" = "universal-apple-darwin" ]; then
  arm="$(build aarch64-apple-darwin)"
  x64="$(build x86_64-apple-darwin)"
  lipo -create -output "$OUT_DIR/project-hub-server" "$arm" "$x64"
else
  host="$(rustc -vV | sed -n 's/^host: //p')"
  if [ -z "$TRIPLE" ] || [ "$TRIPLE" = "$host" ]; then
    bin="$(build "")"
  else
    bin="$(build "$TRIPLE")"
  fi
  cp "$bin" "$OUT_DIR/project-hub-server$EXE"
fi
chmod +x "$OUT_DIR/project-hub-server$EXE"
echo "→ $OUT_DIR/project-hub-server$EXE"
