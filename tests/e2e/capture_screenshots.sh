#!/usr/bin/env bash
set -euo pipefail

# Directory to store screenshots
OUTPUT_DIR="${OUTPUT_DIR:-docs/screenshots}"
BIN="target/release/googlepicz"
DISPLAY_NUM="${DISPLAY_NUM:-99}"

mkdir -p "$OUTPUT_DIR"

# Build the application if needed
if [ ! -f "$BIN" ]; then
    cargo build --release --package app
fi

# Run the UI under Xvfb and capture screenshot using ImageMagick's import
xvfb-run -n "$DISPLAY_NUM" -a --server-args="-screen 0 1280x720x24" \
    bash -c "\
        $BIN &\n        PID=$!\n        sleep 5\n        import -display :$DISPLAY_NUM -window root $OUTPUT_DIR/main_view.png\n        kill $PID\n        wait $PID 2>/dev/null || true\n    "
