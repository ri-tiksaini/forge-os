#!/usr/bin/env bash
# Forge OS Security Lock & Firewall Helper: forge-security [status|lockdown|unlock|lock-screen]
set -euo pipefail

CMD="${1:-status}"

case "$CMD" in
    status)
        echo "=== Forge OS Security Audit ==="
        if command -v ufw >/dev/null 2>&1; then
            sudo ufw status verbose 2>/dev/null || ufw status 2>/dev/null || echo "ufw: inactive"
        else
            echo "ufw: not installed"
        fi
        echo ""
        echo "Open TCP listeners (should be empty or parked proxies only):"
        ss -tlnH || true
        echo ""
        if [ -f /usr/share/polkit-1/actions/org.forge.catalog.policy ]; then
            echo "[OK] Polkit org.forge.catalog.install policy active"
        fi
        if [ -f /usr/share/forge/catalog/catalog.sha256 ]; then
            echo "[OK] Catalog SHA-256 manifest present"
        fi
        ;;
    lockdown)
        [ "$(id -u)" -eq 0 ] || exec sudo "$0" "$@"
        if command -v ufw >/dev/null 2>&1; then
            ufw default deny incoming
            ufw default allow outgoing
            ufw --force enable
            echo "[OK] Firewall LOCKDOWN enabled (deny all incoming, allow outgoing)."
        else
            echo "ufw not installed."
            exit 1
        fi
        ;;
    unlock)
        [ "$(id -u)" -eq 0 ] || exec sudo "$0" "$@"
        if command -v ufw >/dev/null 2>&1; then
            ufw disable
            echo "[OK] Firewall disabled."
        fi
        ;;
    lock-screen)
        exec swaylock -f -c 070709
        ;;
    *)
        echo "Usage: forge-security [status|lockdown|unlock|lock-screen]"
        exit 2
        ;;
esac
