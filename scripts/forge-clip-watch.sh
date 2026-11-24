#!/usr/bin/env bash
# Records text clipboard copies into /tmp/forge-clip-history.txt (last 50 entries).
HIST="/tmp/forge-clip-history.txt"
touch "$HIST"
exec wl-paste --type text --watch sh -c '
    entry="$(cat | tr "\n" " " | cut -c1-160)"
    [ -n "$entry" ] || exit 0
    (printf "%s\n" "$entry"; grep -Fxv "$entry" /tmp/forge-clip-history.txt 2>/dev/null || true) | head -n 50 > /tmp/forge-clip-history.tmp
    mv /tmp/forge-clip-history.tmp /tmp/forge-clip-history.txt
'
