#!/usr/bin/env bash
# End-to-end M2 smoke demo on your Linux Mint host (no root needed, dry-run backend)
set -euo pipefail

cargo build -p forged -p dev-cli

SOCK="/tmp/forge-m2-demo-$$.sock"
JDIR="/tmp/forge-m2-journal-$$"
trap 'kill "${PID:-0}" 2>/dev/null || true; rm -rf "$SOCK" "$JDIR"' EXIT

target/debug/forged --socket "$SOCK" --catalog-dir data/catalog --journal-dir "$JDIR" --idle-timeout 10 --dry-run &
PID=$!

for _ in $(seq 1 40); do
    [ -S "$SOCK" ] && break
    sleep 0.05
done

export FORGE_SOCKET="$SOCK"
echo "=== 1. dev search postg ==="
target/debug/dev search postg
echo "=== 2. dev add postgres --yes ==="
target/debug/dev add postgres --yes
echo "=== 3. dev list (verify postgres is parked) ==="
target/debug/dev list
echo "=== 4. dev doctor ==="
target/debug/dev doctor
echo "[PASS] M2 local Unix-socket demo succeeded."
