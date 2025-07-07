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

  # Determine platform to choose screenshot method
  local uname_out="$(uname 2>/dev/null || true)"

  if [[ "${OS:-}" == "Windows_NT" ]]; then
    # Windows: use PowerShell with System.Windows.Forms
    "$BIN" "$@" &
    local pid=$!
    sleep 5
    powershell -NoProfile -Command "\
      Add-Type -AssemblyName System.Windows.Forms;\
      Add-Type -AssemblyName System.Drawing;\
      $bmp = New-Object System.Drawing.Bitmap([System.Windows.Forms.Screen]::PrimaryScreen.Bounds.Width, [System.Windows.Forms.Screen]::PrimaryScreen.Bounds.Height);\
      $graphics = [System.Drawing.Graphics]::FromImage($bmp);\
      $graphics.CopyFromScreen(0,0,0,0,$bmp.Size);\
      $bmp.Save('$out_file', [System.Drawing.Imaging.ImageFormat]::Png);\
    "
    kill $pid
    wait $pid 2>/dev/null || true
  elif [[ "$uname_out" == "Darwin" ]] && command -v screencapture >/dev/null; then
    # macOS: screencapture if available
    "$BIN" "$@" &
    local pid=$!
    sleep 5
    screencapture -x "$out_file"
    kill $pid
    wait $pid 2>/dev/null || true
  else
    # Linux or fallback: use xvfb and ImageMagick import
    xvfb-run -a "$BIN" "$@" &
    local pid=$!
    sleep 5
    import -window root "$out_file"
    kill $pid
    wait $pid 2>/dev/null || true
  fi
}

# Main screen
run_and_capture "$OUT_DIR/main.png" --sync-interval-minutes 1

# Settings screen
OPEN_SETTINGS=1 run_and_capture "$OUT_DIR/settings.png" --sync-interval-minutes 1
