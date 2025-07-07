#!/usr/bin/env bash
set -euo pipefail

# Directory for resulting screenshots
OUT_DIR="${1:-$(git rev-parse --show-toplevel)/docs/screenshots}"
mkdir -p "$OUT_DIR"

# Build release binary
cargo build --release --package googlepicz
BIN="target/release/googlepicz"

# Detect operating system
if [[ "${OS:-}" == "Windows_NT" ]]; then
  OS_TYPE="windows"
else
  case "$(uname -s)" in
    Linux*)  OS_TYPE="linux" ;;
    Darwin*) OS_TYPE="darwin" ;;
    *)       OS_TYPE="unknown" ;;
  esac
fi

# Capture screenshot depending on OS
capture_screenshot() {
  local out_file=$1
  case "$OS_TYPE" in
    linux)
      import -window root "$out_file"
      ;;
    darwin)
      if command -v screencapture >/dev/null; then
        screencapture -x "$out_file"
      else
        echo "screencapture not available" >&2
        return 1
      fi
      ;;
    windows)
      powershell -Command \
        "Add-Type -AssemblyName System.Windows.Forms; \
         Add-Type -AssemblyName System.Drawing; \
         \$bmp = New-Object System.Drawing.Bitmap([System.Windows.Forms.Screen]::PrimaryScreen.Bounds.Width, [System.Windows.Forms.Screen]::PrimaryScreen.Bounds.Height); \
         \$g = [System.Drawing.Graphics]::FromImage(\$bmp); \
         \$g.CopyFromScreen(0,0,0,0,\$bmp.Size); \
         \$bmp.Save('$out_file', [System.Drawing.Imaging.ImageFormat]::Png)"
      ;;
    *)
      echo "Unsupported OS: $OS_TYPE" >&2
      return 1
      ;;
  esac
}

# Helper to run and capture screenshot
run_and_capture() {
  local out_file=$1
  shift
  if [[ "$OS_TYPE" == "linux" ]]; then
    xvfb-run -a "$BIN" "$@" &
  else
    "$BIN" "$@" &
  fi
  local pid=$!
  sleep 5
  capture_screenshot "$out_file"
  kill $pid
  wait $pid 2>/dev/null || true
}

# Main screen
run_and_capture "$OUT_DIR/main.png" --sync-interval-minutes 1

# Settings screen
OPEN_SETTINGS=1 run_and_capture "$OUT_DIR/settings.png" --sync-interval-minutes 1
