#!/usr/bin/env bash
set -euo pipefail

echo "[TEST 6.1] Checking generated target/brand/os-release..."
test -f target/brand/os-release
grep -q '^ID_LIKE=debian$' target/brand/os-release
grep -q '^LOGO=forge-logo$' target/brand/os-release

echo "[TEST 6.1] Checking no upstream brand strings leaked into UI crates..."
if grep -rnE '"[^"]*(Ubuntu|Linux Mint|Debian)[^"]*"' crates/forge-shell crates/forge-settings 2>/dev/null; then
    echo "FAIL: upstream brand string found in UI crate"
    exit 1
fi

echo "[PASS] Section 6.1 forge-brand acceptance passed."
