#!/usr/bin/env bash
set -euo pipefail

# Directory for resulting screenshots
OUT_DIR="${1:-$(git rev-parse --show-toplevel)/docs/screenshots}"
mkdir -p "$OUT_DIR"

# Build release binary
cargo build --release --package googlepicz
BIN="target/release/googlepicz"

# Helper to run and capture screenshot
run_and_capture() {
  local out_file=$1
  shift
  xvfb-run -a "$BIN" "$@" &
  local pid=$!
  sleep 5
  import -window root "$out_file"
  kill $pid
  wait $pid 2>/dev/null || true
}

# Main screen
run_and_capture "$OUT_DIR/main.png" --sync-interval-minutes 1

# Settings screen
OPEN_SETTINGS=1 run_and_capture "$OUT_DIR/settings.png" --sync-interval-minutes 1
