#!/usr/bin/env bash
# Forge OS System-Wide Network & Browser Ad Blocker (0 MB idle RAM)
# Usage: sudo forge-adblock [on|off|status|update]
set -euo pipefail

HOSTS="/etc/hosts"
BLOCK_SRC="/usr/share/forge/adblock/hosts.block"
BEGIN_MARK="# BEGIN FORGE ADBLOCK"
END_MARK="# END FORGE ADBLOCK"

CMD="${1:-status}"

remove_block() {
    if grep -qF "$BEGIN_MARK" "$HOSTS" 2>/dev/null; then
        python3 -c '
from pathlib import Path
p = Path("/etc/hosts")
lines = p.read_text().splitlines()
out, skip = [], False
for l in lines:
    if l.strip() == "# BEGIN FORGE ADBLOCK": skip = True; continue
    if l.strip() == "# END FORGE ADBLOCK": skip = False; continue
    if not skip: out.append(l)
p.write_text("\n".join(out) + "\n")
'
    fi
}

case "$CMD" in
    on)
        [ "$(id -u)" -eq 0 ] || exec sudo "$0" "$@"
        remove_block
        {
            echo "$BEGIN_MARK"
            cat "$BLOCK_SRC"
            echo "$END_MARK"
        } >> "$HOSTS"
        COUNT="$(grep -c '^0\.0\.0\.0 ' "$BLOCK_SRC" || echo 0)"
        echo "[OK] System-wide Ad Blocker ENABLED (${COUNT} domains blocked in /etc/hosts)."
        ;;
    off)
        [ "$(id -u)" -eq 0 ] || exec sudo "$0" "$@"
        remove_block
        echo "[OK] System-wide Ad Blocker DISABLED (/etc/hosts restored)."
        ;;
    update)
        [ "$(id -u)" -eq 0 ] || exec sudo "$0" "$@"
        echo "Downloading latest unified hosts blocklist..."
        TMP="$(mktemp)"
        trap 'rm -f "$TMP"' EXIT
        curl -fsSL "https://raw.githubusercontent.com/StevenBlack/hosts/master/hosts" -o "$TMP"
        grep -E '^0\.0\.0\.0 [a-zA-Z0-9.-]+' "$TMP" | grep -v '0\.0\.0\.0 0\.0\.0\.0' > "$BLOCK_SRC"
        "$0" on
        ;;
    status)
        if grep -qF "$BEGIN_MARK" "$HOSTS" 2>/dev/null; then
            COUNT="$(sed -n "/$BEGIN_MARK/,/$END_MARK/p" "$HOSTS" | grep -c '^0\.0\.0\.0 ' || echo 0)"
            echo "Ad Blocker: ON (${COUNT} domains blocked network-wide)"
        else
            echo "Ad Blocker: OFF"
        fi
        ;;
    *)
        echo "Usage: forge-adblock [on|off|status|update]"
        exit 2
        ;;
esac
