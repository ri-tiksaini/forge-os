#!/usr/bin/env bash
# Single-command system recovery, Developer Labs Rollback & Factory Core Restore
# Strictly NEVER touches user data in /home!
set -euo pipefail

LEVEL="${1:-configs}"
FACTORY_DIR="/var/lib/forge/factory-core"
ROLLBACK_BASE="/var/lib/forge/lab-rollback"

escalate() {
    if [ "$(id -u)" -eq 0 ]; then
        return 0
    fi
    if [ -t 0 ] && command -v sudo >/dev/null 2>&1; then
        exec sudo "$0" "$@"
    elif command -v pkexec >/dev/null 2>&1; then
        exec pkexec "$0" "$@"
    else
        echo "[repair] Root privileges required (sudo/pkexec not available)." >&2
        exit 1
    fi
}

case "$LEVEL" in
    configs)
        command -v forge-conf >/dev/null 2>&1 && forge-conf
        echo "[repair] Regenerated compositor & GTK4 configs."
        ;;
    packages)
        escalate "$@"
        dpkg --configure -a
        apt-get -f install -y
        ;;
    rollback)
        escalate "$@"
        if [ ! -d "$ROLLBACK_BASE" ]; then
            echo "[repair] No Developer Labs rollback snapshots found at $ROLLBACK_BASE."
            exit 1
        fi
        LATEST="$(ls -1 "$ROLLBACK_BASE" | sort -n | tail -n 1 || true)"
        if [ -z "$LATEST" ]; then
            echo "[repair] Rollback directory is empty."
            exit 1
        fi
        SNAP="$ROLLBACK_BASE/$LATEST"
        echo "[repair] Rolling back Developer Labs patch snapshot $LATEST (user files in /home untouched)..."
        if [ -f "$SNAP/created.list" ]; then
            while IFS= read -r f; do
                [ -n "$f" ] && rm -f "$f"
            done < "$SNAP/created.list"
        fi
        if [ -d "$SNAP/root" ] && [ -n "$(ls -A "$SNAP/root" 2>/dev/null)" ]; then
            cp -a "$SNAP/root/." /
        fi
        systemctl daemon-reload >/dev/null 2>&1 || true
        sysctl --system >/dev/null 2>&1 || true
        echo "[repair] Rolled back cleanly to snapshot $LATEST."
        ;;
    factory)
        escalate "$@"
        [ -d "$FACTORY_DIR" ] || { echo "Factory core snapshot not found at $FACTORY_DIR"; exit 1; }
        echo "[repair] Restoring Core OS files from $FACTORY_DIR (user files in /home untouched)..."
        cp -a "${FACTORY_DIR}/." /
        systemctl daemon-reload >/dev/null 2>&1 || true
        sysctl --system >/dev/null 2>&1 || true
        echo "[repair] Core OS restored to factory defaults!"
        ;;
    full)
        escalate "$@"
        dpkg --configure -a
        apt-get -f install -y
        command -v forge-conf >/dev/null 2>&1 && forge-conf || true
        ;;
    *)
        echo "Usage: forge-repair [configs|packages|rollback|factory|full]"
        exit 2
        ;;
esac
