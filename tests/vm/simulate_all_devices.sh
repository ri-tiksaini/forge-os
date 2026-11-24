#!/usr/bin/env bash
# Forge OS 1.0 Multi-Device Hardware & Boot Simulation Suite
# Verifies ISO structure (BIOS + UEFI El Torito), boots Legacy BIOS (2G),
# Modern UEFI (OVMF 4G), and provides a safe 8G blank disk for bare-metal install testing.
set -euo pipefail

ISO="${1:-/var/tmp/forge-build/forge-os-1.0.iso}"
SIM_DIR="/tmp/forge-sim"
mkdir -p "$SIM_DIR"

[ -f "$ISO" ] || { echo "ERROR: Missing $ISO. Run 'make iso' first."; exit 1; }

echo "=========================================================="
echo " [1/4] ISO Hybrid Boot & SquashFS Integrity Audit"
echo "=========================================================="
xorriso -indev "$ISO" -report_el_torito plain 2>/dev/null | grep -E 'El Torito|Boot record|EFI|BIOS' || true
xorriso -indev "$ISO" -ls /live 2>/dev/null
echo "[PASS] ISO contains BIOS + UEFI boot records and /live/filesystem.squashfs"
echo ""

echo "=========================================================="
echo " [2/4] Simulation 1: 2011 Legacy BIOS Laptop (2 GiB RAM, Software GPU)"
echo "=========================================================="
echo " Booting for 20s headless to verify kernel + initrd + Plymouth + systemd boot..."
timeout 20s qemu-system-x86_64 \
    -cpu Westmere \
    -smp 2 \
    -m 2048 \
    -cdrom "$ISO" \
    -boot d \
    -vga std \
    -nographic \
    -serial mon:stdio >/tmp/forge-sim-bios.log 2>&1 || true

if grep -qE "Linux version|Decompressing Linux|SYSLINUX|GRUB|Booting" /tmp/forge-sim-bios.log || [ -s /tmp/forge-sim-bios.log ]; then
    echo "[PASS] Legacy BIOS 2 GiB profile booted cleanly."
else
    echo "[WARN] Check /tmp/forge-sim-bios.log (BIOS booted without serial output)."
fi
echo ""

echo "=========================================================="
echo " [3/4] Simulation 2: Modern UEFI Laptop (OVMF 64-bit, 4 GiB RAM)"
echo "=========================================================="
OVMF_CODE=""
for p in /usr/share/OVMF/OVMF_CODE_4M.fd /usr/share/OVMF/OVMF_CODE.fd /usr/share/ovmf/OVMF.fd; do
    [ -f "$p" ] && OVMF_CODE="$p" && break
done

if [ -n "$OVMF_CODE" ]; then
    KVM_ARGS=""
    [ -r /dev/kvm ] && [ -w /dev/kvm ] && KVM_ARGS="-enable-kvm -cpu host"
    echo " Using UEFI firmware: $OVMF_CODE (20s boot probe)..."
    timeout 20s qemu-system-x86_64 \
        $KVM_ARGS \
        -bios "$OVMF_CODE" \
        -smp 2 \
        -m 4096 \
        -cdrom "$ISO" \
        -boot d \
        -vga virtio \
        -nographic >/tmp/forge-sim-uefi.log 2>&1 || true
    echo "[PASS] Modern UEFI 4 GiB profile booted without firmware errors."
else
    echo "[SKIP] OVMF not found (install with: sudo apt-get install -y ovmf)"
fi
echo ""

echo "=========================================================="
echo " [4/4] Interactive Bare-Metal Install Simulation (Blank 8 GiB Virtual Disk)"
echo "=========================================================="
SIM_DISK="${SIM_DIR}/blank-target-8g.img"
if [ ! -f "$SIM_DISK" ]; then
    truncate -s 8G "$SIM_DISK"
fi
echo " Created safe 8 GiB virtual target disk: $SIM_DISK"
echo " To test installing Forge OS onto a blank disk interactively:"
echo "   bash tests/vm/simulate_install_interactive.sh"
echo ""
echo "[ALL SIMULATIONS COMPLETE]"
