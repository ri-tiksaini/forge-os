# Forge OS M6 Bootable Hybrid Live ISO Builder (squashfs + grub-mkrescue / xorriso)
BUILD   := /var/tmp/forge-build
ROOTFS  := $(BUILD)/rootfs
ISO_DIR := $(BUILD)/iso_stage
ISO_OUT := $(BUILD)/forge-os-1.0.iso

.PHONY: iso

iso:
	@if [ "$$(id -u)" -ne 0 ]; then echo "ERROR: run with sudo"; exit 1; fi
	@command -v mksquashfs >/dev/null || { echo "Install squashfs-tools"; exit 1; }
	@command -v grub-mkrescue >/dev/null || { echo "Install grub-pc-bin grub-efi-amd64-bin grub-common mtools xorriso"; exit 1; }
	@rm -rf $(ISO_DIR)
	@mkdir -p $(ISO_DIR)/live $(ISO_DIR)/boot/grub
	@echo "[M6 1/3] Compressing rootfs into SquashFS..."
	mksquashfs $(ROOTFS) $(ISO_DIR)/live/filesystem.squashfs -comp zstd -Xcompression-level 9 -noappend -e boot
	@echo "[M6 2/3] Staging kernel, initrd (with Forge Plymouth splash), and GRUB menu..."
	cp -L $(ROOTFS)/boot/vmlinuz-* $(ISO_DIR)/live/vmlinuz
	cp -L $(ROOTFS)/boot/initrd.img-* $(ISO_DIR)/live/initrd.img
	printf 'set timeout=3\nset default=0\nset gfxmode=1280x720,1024x768,auto\nset gfxpayload=keep\n\nmenuentry "Forge OS 1.0 (Live Desktop)" {\n    linux /live/vmlinuz boot=live quiet splash loglevel=3 vt.global_cursor_default=0\n    initrd /live/initrd.img\n}\n\nmenuentry "Forge OS 1.0 - Rescue (No Compositor)" {\n    linux /live/vmlinuz boot=live systemd.unit=multi-user.target nomodeset\n    initrd /live/initrd.img\n}\n' > $(ISO_DIR)/boot/grub/grub.cfg
	@echo "[M6 3/3] Creating hybrid UEFI+BIOS ISO: $(ISO_OUT)..."
	grub-mkrescue -o $(ISO_OUT) $(ISO_DIR)
	@ls -lh $(ISO_OUT)
