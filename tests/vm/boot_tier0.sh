#!/usr/bin/env bash
set -euo pipefail

IMG="build/forge-m0-tier0.img"
KERNEL="build/vmlinuz"
INITRD="build/initrd.img"

for f in "$IMG" "$KERNEL" "$INITRD"; do
    if [ ! -f "$f" ]; then
        echo "Missing $f. Run 'make rootfs' first."
        exit 1
    fi
done

if [ -r /dev/kvm ] && [ -w /dev/kvm ]; then
    ACCEL_ARGS="-enable-kvm -cpu host"
else
    echo "WARN: /dev/kvm not accessible; booting with slower TCG emulation."
    ACCEL_ARGS="-cpu Westmere"
fi

echo "Booting Forge OS M0 (Tier 0 profile: 2 GiB RAM, 5400 RPM SATA HDD)..."
echo "Login: forge / forge  |  Inside VM run: sudo forge-verify-m0"
echo "Exit QEMU: Ctrl-A then X"

exec qemu-system-x86_64 \
    $ACCEL_ARGS \
    -smp 2 \
    -m 2048 \
    -kernel "$KERNEL" \
    -initrd "$INITRD" \
    -append "root=/dev/sda rw console=ttyS0 loglevel=3" \
    -drive file="$IMG",format=raw,if=none,id=hd0 \
    -device ahci,id=ahci \
    -device ide-hd,drive=hd0,bus=ahci.0,rotation_rate=5400 \
    -netdev user,id=net0 \
    -device virtio-net-pci,netdev=net0 \
    -vga virtio \
    -display gtk,gl=off \
    -serial mon:stdio
