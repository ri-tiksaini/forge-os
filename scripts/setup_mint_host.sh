#!/usr/bin/env bash
# One-time host setup for developing Forge OS on Linux Mint (Ubuntu-based)
set -euo pipefail

echo "[1/4] Installing OS build, QEMU, and GTK4 dev packages..."
sudo apt-get update
sudo apt-get install -y \
    debootstrap debian-archive-keyring squashfs-tools xorriso \
    qemu-system-x86 qemu-utils ovmf parted dosfstools btrfs-progs \
    build-essential pkg-config git curl \
    libgtk-4-dev libadwaita-1-dev

echo "[2/4] Ensuring debootstrap knows Debian 13 'trixie'..."
if [ ! -e /usr/share/debootstrap/scripts/trixie ]; then
    echo "Adding trixie debootstrap script symlink -> sid..."
    sudo ln -s sid /usr/share/debootstrap/scripts/trixie
fi

echo "[3/4] Checking KVM group membership..."
if getent group kvm >/dev/null && ! id -nG "$USER" | grep -qw kvm; then
    sudo usermod -aG kvm "$USER"
    echo "Added $USER to 'kvm' group. Log out/in (or run 'newgrp kvm') for fast QEMU."
fi

echo "[4/4] Checking Rust toolchain..."
if ! command -v rustup >/dev/null 2>&1; then
    echo "Rustup not found. Install it with:"
    echo "  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
else
    echo "rustup found: $(rustup --version)"
fi

echo "Host setup complete. Next: run 'make host-check'."
