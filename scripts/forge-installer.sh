#!/usr/bin/env bash
# Forge OS 1.0 Hardened Bare-Metal Installer (UEFI + Secure Boot + Legacy BIOS)
set -euo pipefail

RED=$'\033[38;2;155;17;38m'
WHITE=$'\033[38;2;244;244;245m'
RST=$'\033[0m'

if [ "$(id -u)" -ne 0 ]; then
    exec sudo "$0" "$@"
fi

clear
echo "${RED}======================================================${RST}"
echo "${RED}   FORGE OS 1.0  //  BARE-METAL SYSTEM INSTALLER      ${RST}"
echo "${RED}======================================================${RST}"
echo ""
if [ -d /sys/firmware/efi ]; then
    echo "Boot Mode: UEFI (64-bit)"
else
    echo "Boot Mode: Legacy BIOS"
fi
echo ""
echo "${WHITE}Available Disks:${RST}"
lsblk -d -p -o NAME,SIZE,MODEL,TRAN | grep -v "loop" | grep -v "sr0"
echo ""
read -r -p "Enter target disk to ERASE & install to (e.g. /dev/sda or /dev/nvme0n1): " DISK
[ -b "$DISK" ] || { echo "ERROR: $DISK is not a valid block device."; exit 1; }

read -r -p "New username (default: forge): " NEWUSER
NEWUSER="${NEWUSER:-forge}"
read -r -s -p "Password for ${NEWUSER}: " NEWPASS
echo ""
read -r -p "Hostname (default: forge): " NEWHOST
NEWHOST="${NEWHOST:-forge}"

echo ""
echo "${RED}WARNING: ALL DATA ON ${DISK} WILL BE PERMANENTLY ERASED!${RST}"
read -r -p "Type YES to confirm: " CONFIRM
[ "$CONFIRM" = "YES" ] || { echo "Cancelled. Nothing was modified."; exit 0; }

echo "[1/6] Partitioning ${DISK} (GPT: BIOS-Boot + EFI System + Root ext4)..."
wipefs -af "$DISK" >/dev/null 2>&1 || true
parted -s "$DISK" mklabel gpt
parted -s "$DISK" mkpart bios 1MiB 3MiB set 1 bios_grub on
parted -s "$DISK" mkpart ESP fat32 3MiB 515MiB set 2 esp on
parted -s "$DISK" mkpart root ext4 515MiB 100%
partprobe "$DISK" || true
udevadm settle || sleep 2

PART_PREFIX="$DISK"
[[ "$DISK" == *"nvme"* || "$DISK" == *"mmcblk"* ]] && PART_PREFIX="${DISK}p"
EFI_PART="${PART_PREFIX}2"
ROOT_PART="${PART_PREFIX}3"

echo "[2/6] Formatting ${EFI_PART} (FAT32) and ${ROOT_PART} (ext4)..."
mkfs.vfat -F32 -n FORGE_EFI "$EFI_PART"
mkfs.ext4 -F -L FORGE_ROOT "$ROOT_PART"

MNT="/mnt/forge-install"
mkdir -p "$MNT"
mount "$ROOT_PART" "$MNT"
mkdir -p "$MNT/boot/efi"
mount "$EFI_PART" "$MNT/boot/efi"

echo "[3/6] Extracting Forge OS system files..."
SQUASH="/run/live/medium/live/filesystem.squashfs"
if [ -f "$SQUASH" ]; then
    unsquashfs -f -d "$MNT" "$SQUASH"
    cp -a /run/live/medium/live/vmlinuz "$MNT/boot/vmlinuz-$(uname -r)" 2>/dev/null || true
else
    rsync -aAX --exclude={"/dev/*","/proc/*","/sys/*","/tmp/*","/run/*","/mnt/*","/media/*","/lost+found"} / "$MNT/"
fi

echo "[4/6] Configuring fstab, hostname, and user ${NEWUSER}..."
ROOT_UUID="$(blkid -s UUID -o value "$ROOT_PART")"
EFI_UUID="$(blkid -s UUID -o value "$EFI_PART")"
cat <<FSTAB > "$MNT/etc/fstab"
UUID=${ROOT_UUID}  /          ext4  noatime,lazytime,commit=30,errors=remount-ro  0 1
UUID=${EFI_UUID}   /boot/efi  vfat  umask=0077                                    0 2
FSTAB

echo "$NEWHOST" > "$MNT/etc/hostname"
printf '127.0.0.1\tlocalhost\n127.0.1.1\t%s\n' "$NEWHOST" > "$MNT/etc/hosts"
chroot "$MNT" /bin/sh -c "id -u '${NEWUSER}' >/dev/null 2>&1 || useradd -m -G sudo,video,render,input -s /bin/bash '${NEWUSER}'; echo '${NEWUSER}:${NEWPASS}' | chpasswd"

echo "[5/6] Removing live-boot hooks & regenerating initramfs for bare metal..."
for d in /dev /dev/pts /proc /sys; do mount --bind "$d" "$MNT$d"; done
if [ -d /sys/firmware/efi/efivars ]; then
    mount --bind /sys/firmware/efi/efivars "$MNT/sys/firmware/efi/efivars" || true
fi

chroot "$MNT" /bin/sh -c '
    DEBIAN_FRONTEND=noninteractive apt-get purge -y live-boot live-boot-initramfs-tools >/dev/null 2>&1 || true
    update-initramfs -u -k all
'

echo "[6/6] Installing GRUB bootloader (UEFI + BIOS)..."
if [ -d /sys/firmware/efi ]; then
    chroot "$MNT" grub-install --target=x86_64-efi --efi-directory=/boot/efi --bootloader-id=forge --recheck
else
    chroot "$MNT" grub-install --target=i386-pc --recheck "$DISK"
fi
chroot "$MNT" update-grub

umount -lf "$MNT/sys/firmware/efi/efivars" 2>/dev/null || true
for d in /sys /proc /dev/pts /dev; do umount -lf "$MNT$d" 2>/dev/null || true; done
umount -lf "$MNT/boot/efi" "$MNT" 2>/dev/null || true

echo ""
echo "${WHITE}Forge OS 1.0 bare-metal installation complete! Remove the USB and reboot.${RST}"
read -r -p "Press Enter to close..." _
