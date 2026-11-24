#!/usr/bin/env bash
# Safely flash Forge OS 1.0 hybrid ISO onto a removable USB drive (never touches internal disks!)
set -euo pipefail

ISO="/var/tmp/forge-build/forge-os-1.0.iso"
[ -f "$ISO" ] || { echo "Missing $ISO. Run 'make iso' first."; exit 1; }

echo "=== Removable USB Drives Detected ==="
lsblk -d -p -o NAME,SIZE,MODEL,TRAN,RM | awk 'NR==1 || ($4=="usb" && $5=="1")'
echo ""
read -r -p "Enter USB device to ERASE & flash (e.g. /dev/sdb): " USB
[ -b "$USB" ] || { echo "ERROR: $USB is not a valid block device."; exit 1; }

TRAN="$(lsblk -d -no TRAN "$USB" 2>/dev/null || true)"
RM="$(lsblk -d -no RM "$USB" 2>/dev/null || true)"
if [ "$TRAN" != "usb" ] || [ "$RM" != "1" ]; then
    echo "REFUSED: $USB does not look like a removable USB drive (TRAN=$TRAN, RM=$RM)."
    exit 1
fi

echo "WARNING: ALL DATA ON $USB WILL BE ERASED!"
read -r -p "Type YES to flash $ISO -> $USB: " ANS
[ "$ANS" = "YES" ] || { echo "Cancelled."; exit 0; }

sudo umount "${USB}"?* 2>/dev/null || true
sudo dd if="$ISO" of="$USB" bs=4M status=progress conv=fsync
sync
echo "USB ready! Boot from $USB on any UEFI or BIOS PC."
