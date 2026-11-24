#!/usr/bin/env bash
# Opens the offline Forge OS Helpbook in Firefox, or prints a terminal cheat sheet with --cli.
set -euo pipefail

BOOK="/usr/share/forge/docs/helpbook.html"

if [ "${1:-}" = "--cli" ] || [ -z "${WAYLAND_DISPLAY:-}${DISPLAY:-}" ]; then
    cat <<'EOF'
=== FORGE OS 1.0 QUICK HELPBOOK ===
Shortcuts : [Win] Apps | [Alt+Tab] Switch | [Alt+F4] Close | [Win+A] Quick Settings
            [Win+E] Files | [Win+I] Settings | [Win+N] Code Editor | [Ctrl+V] Term Paste
Catalog   : dev search <q> | dev add <id> | dev list | dev on/off <id> | dev doctor
Stacks    : dev add web | dev add systems | dev add data | dev add devops
Cleaner   : sudo forge-clean [--deep]   (drops kernel RAM + pip/npm/go/apt caches)
Repair    : forge-repair [configs|packages|full]
Full Docs : /usr/share/forge/docs/helpbook.html
EOF
else
    exec firefox-esr "file://${BOOK}"
fi
