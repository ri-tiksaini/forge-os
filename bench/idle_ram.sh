#!/usr/bin/env bash
# Normative Idle PSS measurement (Blueprint 9.2)
# Run 60 s after desktop ready with no input.
set -euo pipefail

total_pss_kb=0
printf "%-8s %-24s %10s %10s\n" "PID" "COMM" "PSS_KiB" "RSS_KiB"
for p in /proc/[0-9]*; do
    pid="${p##*/}"
    [ -r "$p/smaps_rollup" ] || continue
    comm="$(tr '\0' ' ' < "$p/comm" 2>/dev/null | tr -d '\n' || true)"
    pss="$(awk '/^Pss:/ {print $2}' "$p/smaps_rollup" 2>/dev/null || echo 0)"
    rss="$(awk '/^Rss:/ {print $2}' "$p/smaps_rollup" 2>/dev/null || echo 0)"
    pss="${pss:-0}"
    rss="${rss:-0}"
    if [ "$pss" -gt 0 ]; then
        printf "%-8s %-24s %10d %10d\n" "$pid" "${comm:0:24}" "$pss" "$rss"
        total_pss_kb=$((total_pss_kb + pss))
    fi
done

total_pss_mib=$((total_pss_kb / 1024))
echo "--------------------------------------------------------"
echo "TOTAL IDLE PSS: ${total_pss_mib} MiB (${total_pss_kb} KiB)"
echo "Tier 0 Budget : <= 300 MiB (N1)"
if [ "$total_pss_mib" -le 300 ]; then
    echo "[PASS] Within Tier 0 PSS budget."
else
    echo "[FAIL] Exceeds Tier 0 PSS budget (300 MiB)."
    exit 1
fi
