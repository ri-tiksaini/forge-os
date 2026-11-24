#!/usr/bin/env bash
# Inspect listening local ports or kill a stuck dev server by port: forge-ports [kill <port>]
set -euo pipefail

CMD="${1:-list}"
PORT="${2:-}"

if [ "$CMD" = "kill" ]; then
    [ -n "$PORT" ] || { echo "Usage: forge-ports kill <port>"; exit 2; }
    PIDS="$(ss -tulnp 2>/dev/null | awk -v p=":${PORT}" '$5 ~ p"$" {print $0}' | grep -oE 'pid=[0-9]+' | cut -d= -f2 | sort -u || true)"
    if [ -z "$PIDS" ]; then
        echo "No listening process found on port ${PORT}."
        exit 0
    fi
    for pid in $PIDS; do
        kill -TERM "$pid" 2>/dev/null || sudo kill -TERM "$pid"
        echo "[OK] Sent SIGTERM to PID ${pid} on port ${PORT}"
    done
    exit 0
fi

echo "=== Listening TCP/UDP Ports ==="
ss -tulnp 2>/dev/null || ss -tuln
