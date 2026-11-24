#!/usr/bin/env bash
# Interactive UEFI (if OVMF present) or BIOS simulation with ISO + blank 8 GiB virtual target disk.
# Lets you test `forge-installer` end-to-end onto /dev/sda safely without touching Mint!
set -euo pipefail

ISO="/var/tmp/forge-build/forge-os-1.0.iso"
SIM_DISK="/tmp/forge-sim/blank-target-8g.img"
mkdir -p "$(dirname "$SIM_DISK")"
[ -f "$SIM_DISK" ] || truncate -s 8G "$SIM_DISK"

ACCEL="-cpu Westmere"
[ -r /dev/kvm ] && [ -w /dev/kvm ] && ACCEL="-enable-kvm -cpu host"

UEFI_ARGS=""
for p in /usr/share/OVMF/OVMF_CODE_4M.fd /usr/share/OVMF/OVMF_CODE.fd /usr/share/ovmf/OVMF.fd; do
    if [ -f "$p" ]; then
        UEFI_ARGS="-bios $p"
        break
    fi
done

echo "Booting Forge OS ISO with blank 8 GiB target disk (/dev/sda)..."
echo "Inside Forge OS: Right-click desktop -> Install Forge OS to Disk -> pick /dev/sda"
exec qemu-system-x86_64 \
    $ACCEL \
    $UEFI_ARGS \
    -smp 2 \
    -m 3072 \
    -drive file="$SIM_DISK",format=raw,if=none,id=hd0 \
    -device ahci,id=ahci \
    -device ide-hd,drive=hd0,bus=ahci.0 \
    -cdrom "$ISO" \
    -boot d \
    -netdev user,id=net0 \
    -device virtio-net-pci,netdev=net0 \
    -vga virtio \
    -display gtk,gl=off
