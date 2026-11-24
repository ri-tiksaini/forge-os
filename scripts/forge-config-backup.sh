#!/usr/bin/env bash
# Allowlisted daily config backup (Blueprint 6.10)
# Never archives raw ~/.config wholesale (excludes browser/Electron/VS Code caches).
set -euo pipefail

BACKUP_DIR="${FORGE_BACKUP_DIR:-/var/lib/forge/backups}"
TARGET_HOME="${FORGE_BACKUP_HOME:-/home/forge}"
ETC_DIR="${FORGE_BACKUP_ETC:-/etc}"
KEEP=7

mkdir -p "$BACKUP_DIR"
STAMP="$(date +%Y%m%d-%H%M%S)"
OUT="${BACKUP_DIR}/forge-backup-${STAMP}.tar.gz"

PATHS=()
[ -d "$ETC_DIR" ] && PATHS+=("$ETC_DIR")
for sub in labwc sway gtk-4.0 foot forge git mise; do
    [ -e "${TARGET_HOME}/.config/${sub}" ] && PATHS+=("${TARGET_HOME}/.config/${sub}")
done
for f in .bashrc .profile .gitconfig; do
    [ -e "${TARGET_HOME}/${f}" ] && PATHS+=("${TARGET_HOME}/${f}")
done

if [ "${#PATHS[@]}" -eq 0 ]; then
    echo "Nothing to back up."
    exit 0
fi

tar -czf "$OUT"     --exclude='*/Cache/*'     --exclude='*/CachedData/*'     --exclude='*/GPUCache/*'     --exclude='*.sqlite*'     --exclude='shadow-'     "${PATHS[@]}" 2>/dev/null || true

# Prune older than KEEP
ls -1t "${BACKUP_DIR}"/forge-backup-*.tar.gz 2>/dev/null | tail -n +$((KEEP + 1)) | xargs -r rm -f
echo "Backup created: ${OUT} ($(du -h "$OUT" | awk '{print $1}'))"
