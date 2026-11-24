#!/usr/bin/env bash
# Boot the generated Forge OS 1.0 hybrid live ISO in QEMU (Blueprint 8.4)
set -euo pipefail

ISO="/var/tmp/forge-build/forge-os-1.0.iso"
if [ ! -f "$ISO" ]; then
    echo "Missing $ISO. Run 'make iso' first."
    exit 1
fi

if [ -r /dev/kvm ] && [ -w /dev/kvm ]; then
    ACCEL_ARGS="-enable-kvm -cpu host"
else
    ACCEL_ARGS="-cpu Westmere"
fi

echo "Booting Forge OS 1.0 Live ISO ($ISO)..."
exec qemu-system-x86_64     $ACCEL_ARGS     -smp 2     -m 2048     -cdrom "$ISO"     -boot d     -netdev user,id=net0     -device virtio-net-pci,netdev=net0     -vga virtio     -display gtk,gl=off
