#!/usr/bin/env bash
# Verifies Blueprint 6.10: allowlisted backup includes dotfiles and excludes browser/Electron caches.
set -euo pipefail

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

mkdir -p "$TMP/home/.config/labwc" "$TMP/home/.config/Code/Cache" "$TMP/etc" "$TMP/backups"
echo "<openbox_config/>" > "$TMP/home/.config/labwc/rc.xml"
dd if=/dev/zero of="$TMP/home/.config/Code/Cache/big.bin" bs=1M count=2 status=none
echo "forge" > "$TMP/etc/hostname"

FORGE_BACKUP_DIR="$TMP/backups" FORGE_BACKUP_HOME="$TMP/home" FORGE_BACKUP_ETC="$TMP/etc"     bash scripts/forge-config-backup.sh

ARCHIVE="$(ls -1 "$TMP/backups"/forge-backup-*.tar.gz | head -n 1)"
tar -tzf "$ARCHIVE" | grep -q "labwc/rc.xml"
if tar -tzf "$ARCHIVE" | grep -q "Code/Cache"; then
    echo "FAIL: VS Code cache leaked into backup!"
    exit 1
fi
echo "[PASS] Section 6.10 allowlisted backup verified (caches excluded)."
