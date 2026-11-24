#!/usr/bin/env bash
set -euo pipefail

ok=1
for cmd in debootstrap qemu-system-x86_64 mkfs.ext4 cargo rustc; do
    if command -v "$cmd" >/dev/null 2>&1; then
        echo "[OK]   $cmd -> $(command -v "$cmd")"
    else
        echo "[MISS] $cmd (run scripts/setup_mint_host.sh)"
        ok=0
    fi
done

if [ -e /usr/share/debootstrap/scripts/trixie ]; then
    echo "[OK]   debootstrap trixie script present"
else
    echo "[MISS] /usr/share/debootstrap/scripts/trixie"
    ok=0
fi

if [ -f /usr/share/keyrings/debian-archive-keyring.gpg ]; then
    echo "[OK]   debian-archive-keyring present"
else
    echo "[MISS] /usr/share/keyrings/debian-archive-keyring.gpg"
    ok=0
fi

if [ -r /dev/kvm ] && [ -w /dev/kvm ]; then
    echo "[OK]   /dev/kvm accessible (hardware virtualization enabled)"
else
    echo "[WARN] /dev/kvm not accessible (QEMU will run slower in software mode)"
fi

[ "$ok" -eq 1 ] && echo "[PASS] Linux Mint host ready for Forge OS M0." || exit 1
