#!/usr/bin/env bash
# Captures region with slurp+grim, copies to Wayland clipboard, saves PNG, and notifies.
set -euo pipefail

DIR="${HOME}/Pictures/Screenshots"
mkdir -p "$DIR"
FILE="${DIR}/forge-$(date +%Y%m%d-%H%M%S).png"

GEOM="$(slurp)" || exit 0
grim -g "$GEOM" "$FILE"
wl-copy < "$FILE"

if command -v notify-send >/dev/null 2>&1; then
    notify-send "Screenshot Saved" "Copied to clipboard & saved to ${FILE}" 2>/dev/null || true
fi
