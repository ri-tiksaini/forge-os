#!/usr/bin/env bash
# Forge OS Local Intrusion & Background Attacker Check (0 MB idle RAM)
set -u

echo "=== Forge OS Background Security Watch ==="

# 1. Unexpected open TCP listeners (only 127.0.0.1 parked DB proxies expected)
EXT_PORTS="$(ss -tlnH 2>/dev/null | awk '$4 !~ /^127\.0\.0\.1:/ && $4 !~ /^\[::1\]:/ {print $4}' || true)"
if [ -n "$EXT_PORTS" ]; then
    echo "[ALERT] External TCP listeners detected:"
    echo "$EXT_PORTS"
else
    echo "[PASS]  Zero external TCP listeners open."
fi

# 2. Active logged-in users / remote sessions
echo ""
echo "Active Sessions:"
who 2>/dev/null || echo "  (none)"

# 3. Suspicious executables in /tmp or /dev/shm (common attacker drop zones)
TMP_EXECS="$(find /tmp /dev/shm -maxdepth 2 -type f -executable 2>/dev/null | head -n 10 || true)"
echo ""
if [ -n "$TMP_EXECS" ]; then
    echo "[WARN]  Executable files found in /tmp or /dev/shm:"
    echo "$TMP_EXECS"
else
    echo "[PASS]  No executable binaries in /tmp or /dev/shm."
fi

# 4. Recent failed authentication attempts
echo ""
echo "Recent Auth Failures (last 24h):"
journalctl -q --since "24 hours ago" -g "authentication failure|Failed password" -n 5 --no-pager 2>/dev/null || echo "  [PASS] None detected."
