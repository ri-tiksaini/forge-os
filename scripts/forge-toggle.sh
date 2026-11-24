#!/usr/bin/env bash
# Toggles forge-shell popups (launcher | quick | power | menu) safely via PID file.
set -euo pipefail

MODE="${1:-launcher}"
PIDFILE="/tmp/forge-shell-${MODE}.pid"

if [ -f "$PIDFILE" ]; then
    OLD_PID="$(cat "$PIDFILE" 2>/dev/null || true)"
    if [ -n "$OLD_PID" ] && kill -0 "$OLD_PID" 2>/dev/null; then
        kill "$OLD_PID" 2>/dev/null || true
        rm -f "$PIDFILE"
        exit 0
    fi
    rm -f "$PIDFILE"
fi

export GSK_RENDERER="${GSK_RENDERER:-cairo}"
forge-shell --mode "$MODE" --catalog-dir /usr/share/forge/catalog >/tmp/forge-shell-"$MODE".log 2>&1 &
echo $! > "$PIDFILE"
