#!/usr/bin/env bash
# Forge OS Double-Click .deb Installer (runs when double-clicking a .deb in Thunar)
set -euo pipefail

DEB="${1:-}"
if [ -z "$DEB" ] || [ ! -f "$DEB" ]; then
    echo "Usage: forge-deb-install <package.deb>"
    exit 2
fi

ABS_DEB="$(readlink -f "$DEB")"
RED=$'\033[38;2;155;17;38m'
WHITE=$'\033[38;2;244;244;245m'
RST=$'\033[0m'

clear
echo "${RED}== Forge OS Package Installer (.deb) ==${RST}"
dpkg-deb -I "$ABS_DEB" | grep -E '^\s*(Package|Version|Installed-Size|Description):' || true
echo ""
read -r -p "${WHITE}Install this package now? [y/N] ${RST}" ANS
if [[ "$ANS" =~ ^[Yy]$ ]]; then
    sudo apt-get install -y --no-install-recommends "$ABS_DEB"
    echo ""
    echo "${WHITE}Installed cleanly.${RST}"
else
    echo "Cancelled."
fi
read -r -p "Press Enter to close..." _
