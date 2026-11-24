#!/usr/bin/env bash
# Run inside the Tier 0 QEMU guest to verify M0 + M1 acceptance gates (G7, G8, N1, N6)
set -euo pipefail

echo "=== [1/6] Brand & Lineage (N6) ==="
grep -E '^(NAME|ID|ID_LIKE|PRETTY_NAME)=' /etc/os-release

echo "=== [2/6] Hardware Tier Detection (M1 / 6.3) ==="
forge-tier

echo "=== [3/6] Boot Time (1.3: <= 22s HDD) ==="
systemd-analyze || true

echo "=== [4/6] Running Services Allowlist Check (G7) ==="
systemctl list-units --type=service --state=running --no-pager

echo "=== [5/6] No Open TCP Listeners (G8) ==="
command -v ss >/dev/null || { echo "FAIL: ss (iproute2) not installed"; exit 1; }
if ss -tlnH | grep -q .; then
    echo "FAIL: unexpected TCP listeners:"
    ss -tlnp
    exit 1
else
    echo "[PASS] Zero TCP listeners."
fi

echo "=== [6/6] Idle PSS Budget (N1) ==="
sudo /usr/local/bin/forge-idle-ram
