#!/usr/bin/env bash
# Win+V Windows-style clipboard history picker (copies selected line back to clipboard).
set -euo pipefail
HIST="/tmp/forge-clip-history.txt"
[ -s "$HIST" ] || { notify-send "Clipboard" "Clipboard history is empty." 2>/dev/null || true; exit 0; }
PICK="$(fuzzel --dmenu -p 'Clipboard > ' --background=070709ff --text-color=ececefff --selection-color=9b1126ff --border-color=9b1126ff < "$HIST")" || exit 0
printf '%s' "$PICK" | wl-copy
