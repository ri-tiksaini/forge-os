#!/usr/bin/env bash
# Forge OS 1-Command Network, DNS, Wi-Fi & Adblock Troubleshooter
set -u

RED=$'\033[38;2;155;17;38m'
WHITE=$'\033[38;2;244;244;245m'
DIM=$'\033[38;2;161;161;170m'
RST=$'\033[0m'

echo "${RED}== Forge OS Network Troubleshooter ==${RST}"
echo "${DIM}Interfaces & IPs:${RST}"
ip -brief addr show 2>/dev/null || true
echo ""

GW="$(ip route 2>/dev/null | awk '/^default/ {print $3; exit}')"
echo "${DIM}Default Gateway :${RST} ${WHITE}${GW:-NONE}${RST}"
if [ -n "$GW" ]; then
    if ping -c 1 -W 2 "$GW" >/dev/null 2>&1; then
        echo "[OK]   Gateway reachable ($GW)"
    else
        echo "[FAIL] Gateway unreachable ($GW)"
    fi
fi

if getent hosts deb.debian.org >/dev/null 2>&1; then
    echo "[OK]   DNS resolution working (deb.debian.org)"
else
    echo "[FAIL] DNS resolution failed (check /etc/resolv.conf or NetworkManager)"
fi

if command -v forge-adblock >/dev/null 2>&1; then
    forge-adblock status
fi

if command -v nmcli >/dev/null 2>&1; then
    echo ""
    echo "${DIM}Wi-Fi / NetworkManager Status:${RST}"
    nmcli -t -f DEVICE,TYPE,STATE,CONNECTION dev status 2>/dev/null || true
fi
