#!/usr/bin/env bash
# View or clear local failed-command & session error history: forge-track [tail|clear]
set -euo pipefail

LOG="${HOME}/.local/share/forge/session-errors.log"
CMD="${1:-tail}"

if [ "$CMD" = "clear" ]; then
    : > "$LOG" 2>/dev/null || true
    echo "[OK] Cleared session error log."
    exit 0
fi

if [ ! -s "$LOG" ]; then
    echo "No failed commands recorded yet."
    exit 0
fi

printf "%-19s  %-16s  %-8s  %-22s  %s\n" "TIME" "SESSION" "STATUS" "DIR" "COMMAND"
printf "%-19s  %-16s  %-8s  %-22s  %s\n" "-------------------" "----------------" "--------" "----------------------" "-------"
tail -n 25 "$LOG" | while IFS=$'\t' read -r ts sess code dir cmd; do
    printf "%-19s  %-16s  %-8s  %-22s  %s\n" "$ts" "${sess:0:16}" "$code" "${dir:0:22}" "$cmd"
done
