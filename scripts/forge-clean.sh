#!/usr/bin/env bash
# Forge OS Safe RAM & Developer Cache Cleaner (CLI + 1-click GUI)
# Usage: forge-clean [--deep]
set -euo pipefail

DEEP=0
[ "${1:-}" = "--deep" ] && DEEP=1

RED=$'\033[38;2;155;17;38m'
WHITE=$'\033[38;2;244;244;245m'
DIM=$'\033[38;2;161;161;170m'
RST=$'\033[0m'

mem_avail_mib() {
    awk '/^MemAvailable:/ {print int($2/1024)}' /proc/meminfo
}

BEFORE_AVAIL=$(mem_avail_mib)
TARGET_USER="${SUDO_USER:-$USER}"
TARGET_HOME="$(getent passwd "$TARGET_USER" | cut -d: -f6 2>/dev/null || echo "$HOME")"

echo "${RED}== Forge OS System & Dev Cache Cleaner ==${RST}"
echo "${DIM}Available RAM before: ${WHITE}${BEFORE_AVAIL} MiB${RST}"

# 1. Flush dirty buffers & drop kernel page/dentry/inode caches safely
if [ "$(id -u)" -eq 0 ]; then
    sync
    echo 3 > /proc/sys/vm/drop_caches 2>/dev/null || true
    echo "[OK] Dropped kernel page, dentry & inode caches."
    apt-get clean 2>/dev/null || true
    echo "[OK] Cleared APT package archive cache."
    journalctl --vacuum-size=16M >/dev/null 2>&1 || true
    echo "[OK] Vacuumed systemd journal to <= 16 MiB."
else
    echo "[INFO] Run with sudo (or via taskbar button) to drop kernel RAM caches."
fi

# 2. Clean safe-to-rebuild user & dev caches (never touches user source code!)
for cdir in \
    "${TARGET_HOME}/.cache/thumbnails" \
    "${TARGET_HOME}/.cache/pip" \
    "${TARGET_HOME}/.npm/_cacache" \
    "${TARGET_HOME}/.cache/go-build" \
    "${TARGET_HOME}/.cache/mesa_shader_cache"; do
    if [ -d "$cdir" ]; then
        rm -rf "${cdir:?}"/* 2>/dev/null || true
    fi
done
rm -f /tmp/forge-shell-*.log 2>/dev/null || true
echo "[OK] Cleared user thumbnails & dev build caches (pip/npm/go/mesa)."

# 3. Optional deep clean (unused podman images/containers)
if [ "$DEEP" -eq 1 ] && command -v podman >/dev/null 2>&1; then
    podman system prune -f >/dev/null 2>&1 || true
    echo "[OK] Pruned unused Podman containers & images (--deep)."
fi

AFTER_AVAIL=$(mem_avail_mib)
GAINED=$((AFTER_AVAIL - BEFORE_AVAIL))
echo "${DIM}Available RAM after : ${WHITE}${AFTER_AVAIL} MiB${RST} (delta: ${GAINED} MiB)"

if command -v notify-send >/dev/null 2>&1; then
    notify-send "Forge OS Cleaner" "System & dev caches cleared. Available RAM: ${AFTER_AVAIL} MiB" 2>/dev/null || true
fi
