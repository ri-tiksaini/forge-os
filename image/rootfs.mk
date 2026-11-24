# Forge OS M0-M6 2-Stage Rootfs & Tier 0 QEMU Image Builder (Debian 13 trixie)
SUITE    := trixie
MIRROR   := http://deb.debian.org/debian
KEYRING  := /usr/share/keyrings/debian-archive-keyring.gpg
BUILD    := build
ROOTFS   := $(BUILD)/rootfs
IMG      := $(BUILD)/forge-m0-tier0.img
IMG_SIZE := 6G

PKGS := $(shell grep -v '^\#' image/packages.list | grep -v '^$$' | tr '\n' ' ')

MASKED_UNITS := \
	cups-browsed.service \
	avahi-daemon.service \
	ModemManager.service \
	apt-daily.timer \
	apt-daily-upgrade.timer \
	man-db.timer

.PHONY: all rootfs configure image clean

all: rootfs configure image

rootfs:
	@if [ "$$(id -u)" -ne 0 ]; then echo "ERROR: run with sudo"; exit 1; fi
	@if [ ! -f "$(KEYRING)" ]; then echo "ERROR: install debian-archive-keyring"; exit 1; fi
	mkdir -p $(BUILD)
	@if [ ! -f "$(ROOTFS)/.m0-stage2-done" ]; then \
		rm -rf $(ROOTFS); \
		echo "[Stage 1] Minimal debootstrap (trixie)..."; \
		debootstrap --variant=minbase --components=main,non-free-firmware \
			--keyring=$(KEYRING) --include=ca-certificates,apt-utils \
			$(SUITE) $(ROOTFS) $(MIRROR); \
		echo "[Stage 2] Batched apt-get install inside chroot..."; \
		printf 'deb %s %s main non-free-firmware\n' "$(MIRROR)" "$(SUITE)" > $(ROOTFS)/etc/apt/sources.list; \
		cp -a image/overlays/. $(ROOTFS)/; \
		printf '#!/bin/sh\nexit 101\n' > $(ROOTFS)/usr/sbin/policy-rc.d; \
		chmod +x $(ROOTFS)/usr/sbin/policy-rc.d; \
		( \
			trap 'umount -lf $(ROOTFS)/dev/pts $(ROOTFS)/dev $(ROOTFS)/proc $(ROOTFS)/sys 2>/dev/null || true; rm -f $(ROOTFS)/usr/sbin/policy-rc.d' EXIT; \
			mount --bind /dev $(ROOTFS)/dev; \
			mount --bind /dev/pts $(ROOTFS)/dev/pts; \
			mount -t proc proc $(ROOTFS)/proc; \
			mount -t sysfs sys $(ROOTFS)/sys; \
			DEBIAN_FRONTEND=noninteractive chroot $(ROOTFS) apt-get update; \
			DEBIAN_FRONTEND=noninteractive chroot $(ROOTFS) apt-get install -y --no-install-recommends $(PKGS) live-boot; \
			DEBIAN_FRONTEND=noninteractive chroot $(ROOTFS) apt-get clean; \
		); \
		touch $(ROOTFS)/.m0-stage2-done; \
	else \
		echo "Rootfs present; ensuring live-boot, libgtk-4-1 & papirus-icon-theme are installed..."; \
		printf '#!/bin/sh\nexit 101\n' > $(ROOTFS)/usr/sbin/policy-rc.d; \
		chmod +x $(ROOTFS)/usr/sbin/policy-rc.d; \
		( \
			trap 'umount -lf $(ROOTFS)/dev/pts $(ROOTFS)/dev $(ROOTFS)/proc $(ROOTFS)/sys 2>/dev/null || true; rm -f $(ROOTFS)/usr/sbin/policy-rc.d' EXIT; \
			mount --bind /dev $(ROOTFS)/dev; \
			mount --bind /dev/pts $(ROOTFS)/dev/pts; \
			mount -t proc proc $(ROOTFS)/proc; \
			mount -t sysfs sys $(ROOTFS)/sys; \
			DEBIAN_FRONTEND=noninteractive chroot $(ROOTFS) apt-get update; \
			DEBIAN_FRONTEND=noninteractive chroot $(ROOTFS) apt-get install -y --no-install-recommends iproute2 live-boot libgtk-4-1 papirus-icon-theme mousepad librsvg2-common hicolor-icon-theme shared-mime-info desktop-file-utils fonts-font-awesome libgtk-3-bin libgdk-pixbuf2.0-bin gtk-update-icon-cache fonts-noto-color-emoji plymouth-themes parted rsync squashfs-tools libnotify-bin fwupd grub-efi-amd64-bin grub-efi-amd64-signed shim-signed grub2-common os-prober ufw swayidle openssh-client udisks2 gvfs-backends gvfs-fuse mtp-tools usbmuxd exfatprogs ntfs-3g cups cups-filters printer-driver-escpr bluez; \
			DEBIAN_FRONTEND=noninteractive chroot $(ROOTFS) apt-get clean; \
		); \
	fi

configure: rootfs
	@echo "[M5] Installing Forge OS binaries, Cherry theme, wallpaper, safety net & session..."
	cp -a image/overlays/. $(ROOTFS)/
	install -m 0644 target/brand/os-release $(ROOTFS)/etc/os-release
	install -m 0644 NOTICE $(ROOTFS)/usr/share/doc/FORGE-NOTICE
	echo "forge" > $(ROOTFS)/etc/hostname
	printf '127.0.0.1\tlocalhost\n127.0.1.1\tforge\n' > $(ROOTFS)/etc/hosts
	printf 'Forge OS 1.0 \\n \\l\n\n' > $(ROOTFS)/etc/issue
	: > $(ROOTFS)/etc/motd
	mkdir -p $(ROOTFS)/etc/systemd/system $(ROOTFS)/etc/systemd/system/sockets.target.wants $(ROOTFS)/etc/systemd/system/timers.target.wants
	for u in $(MASKED_UNITS); do \
		ln -sf /dev/null $(ROOTFS)/etc/systemd/system/$$u; \
	done
	mkdir -p $(ROOTFS)/etc/NetworkManager/conf.d
	printf '[device]\nwifi.backend=iwd\n' > $(ROOTFS)/etc/NetworkManager/conf.d/wifi_backend.conf
	install -m 0755 bench/idle_ram.sh $(ROOTFS)/usr/local/bin/forge-idle-ram
	install -m 0755 tests/vm/verify_m0_guest.sh $(ROOTFS)/usr/local/bin/forge-verify-m0
	install -m 0755 scripts/forge-config-backup.sh $(ROOTFS)/usr/local/bin/forge-config-backup
	install -m 0755 scripts/forge-repair.sh $(ROOTFS)/usr/local/bin/forge-repair
	install -m 0755 scripts/forge-session.sh $(ROOTFS)/usr/local/bin/forge-session
	install -m 0755 scripts/forge-toggle.sh $(ROOTFS)/usr/local/bin/forge-toggle
	install -m 0755 scripts/forge-clean.sh $(ROOTFS)/usr/local/bin/forge-clean
	install -m 0755 scripts/forge-help.sh $(ROOTFS)/usr/local/bin/forge-help
	install -m 0755 scripts/forge-screenshot.sh $(ROOTFS)/usr/local/bin/forge-screenshot
	install -m 0755 scripts/forge-clip-watch.sh $(ROOTFS)/usr/local/bin/forge-clip-watch
	install -m 0755 scripts/forge-clipboard.sh $(ROOTFS)/usr/local/bin/forge-clipboard
	install -m 0755 scripts/forge-installer.sh $(ROOTFS)/usr/local/bin/forge-installer
	install -m 0755 scripts/forge-deb-install.sh $(ROOTFS)/usr/local/bin/forge-deb-install
	install -m 0755 scripts/forge-profile.sh $(ROOTFS)/usr/local/bin/forge-profile
	install -m 0755 scripts/forge-adblock.sh $(ROOTFS)/usr/local/bin/forge-adblock
	install -m 0755 scripts/forge-netcheck.sh $(ROOTFS)/usr/local/bin/forge-netcheck
	install -m 0755 scripts/forge-ports.sh $(ROOTFS)/usr/local/bin/forge-ports
	install -m 0755 scripts/forge-ssh.sh $(ROOTFS)/usr/local/bin/forge-ssh
	install -m 0755 scripts/forge-security.sh $(ROOTFS)/usr/local/bin/forge-security
	install -m 0755 scripts/forge-track.sh $(ROOTFS)/usr/local/bin/forge-track
	install -m 0755 scripts/forge-watch.sh $(ROOTFS)/usr/local/bin/forge-watch
	install -m 0755 scripts/forge-lab.sh $(ROOTFS)/usr/local/bin/forge-lab
	install -m 0644 data/theme/system-map.json $(ROOTFS)/usr/share/forge/system-map.json
	mkdir -p $(ROOTFS)/etc/profile.d
	install -m 0644 data/theme/forge-track-profile.sh $(ROOTFS)/etc/profile.d/forge-track.sh
	printf 'forge ALL=(root) NOPASSWD: /usr/local/bin/forge-security, /usr/sbin/ufw, /usr/bin/timedatectl\n' > $(ROOTFS)/etc/sudoers.d/forge-tools
	chmod 0440 $(ROOTFS)/etc/sudoers.d/forge-tools
	mkdir -p $(ROOTFS)/usr/share/forge/adblock $(ROOTFS)/etc/firefox-esr/policies
	install -m 0644 data/adblock/hosts.block $(ROOTFS)/usr/share/forge/adblock/hosts.block
	install -m 0644 data/adblock/firefox-policies.json $(ROOTFS)/etc/firefox-esr/policies/policies.json
	printf 'forge ALL=(root) NOPASSWD: /usr/local/bin/forge-adblock\n' > $(ROOTFS)/etc/sudoers.d/forge-adblock
	chmod 0440 $(ROOTFS)/etc/sudoers.d/forge-adblock
	# Enable baseline ad/tracker blocklist in image /etc/hosts
	printf '\n# BEGIN FORGE ADBLOCK\n' >> $(ROOTFS)/etc/hosts
	cat data/adblock/hosts.block >> $(ROOTFS)/etc/hosts
	printf '# END FORGE ADBLOCK\n' >> $(ROOTFS)/etc/hosts
	mkdir -p $(ROOTFS)/usr/share/applications
	install -m 0644 data/theme/forge-deb-install.desktop $(ROOTFS)/usr/share/applications/forge-deb-install.desktop
	chroot $(ROOTFS) /bin/sh -c 'update-desktop-database /usr/share/applications || true'
	# Bundle mise static binary if not yet present in rootfs (Blueprint 6.2)
	@if [ ! -x "$(ROOTFS)/usr/local/bin/mise" ]; then curl -fsSL https://mise.jdx.dev/mise-latest-linux-x64 -o $(ROOTFS)/usr/local/bin/mise && chmod 0755 $(ROOTFS)/usr/local/bin/mise || true; fi
	mkdir -p $(ROOTFS)/usr/share/forge/docs
	install -m 0644 docs/user/helpbook.html $(ROOTFS)/usr/share/forge/docs/helpbook.html
	mkdir -p $(ROOTFS)/etc/sudoers.d $(ROOTFS)/usr/share/icons/hicolor/scalable/apps
	printf 'forge ALL=(root) NOPASSWD: /usr/local/bin/forge-clean\n' > $(ROOTFS)/etc/sudoers.d/forge-clean
	chmod 0440 $(ROOTFS)/etc/sudoers.d/forge-clean
	install -m 0644 data/theme/icons/forge-monitor.svg $(ROOTFS)/usr/share/icons/hicolor/scalable/apps/forge-monitor.svg
	install -m 0644 data/theme/icons/forge-clean.svg $(ROOTFS)/usr/share/icons/hicolor/scalable/apps/forge-clean.svg
	install -m 0755 scripts/forge-apply-theme.sh $(ROOTFS)/usr/local/bin/forge-apply-theme
	install -m 0755 scripts/forge-hud.sh $(ROOTFS)/usr/local/bin/forge-hud
	install -m 0755 scripts/forge-welcome-term.sh $(ROOTFS)/usr/local/bin/forge-welcome-term
	mkdir -p $(ROOTFS)/usr/share/forge/waybar
	install -m 0644 data/theme/waybar/config $(ROOTFS)/usr/share/forge/waybar/config
	install -m 0644 data/theme/waybar/style.css $(ROOTFS)/usr/share/forge/waybar/style.css
	# Global GTK3/GTK4 icon theme settings + rebuild gdk-pixbuf SVG loader & icon caches
	mkdir -p $(ROOTFS)/etc/gtk-3.0 $(ROOTFS)/etc/gtk-4.0
	printf '[Settings]\ngtk-application-prefer-dark-theme=1\ngtk-icon-theme-name=Papirus-Dark\n' > $(ROOTFS)/etc/gtk-3.0/settings.ini
	printf '[Settings]\ngtk-application-prefer-dark-theme=1\ngtk-icon-theme-name=Papirus-Dark\n' > $(ROOTFS)/etc/gtk-4.0/settings.ini
	chroot $(ROOTFS) /bin/sh -c '/usr/lib/x86_64-linux-gnu/gdk-pixbuf-2.0/gdk-pixbuf-query-loaders --update-cache || true; update-mime-database /usr/share/mime || true; gtk-update-icon-cache -f -t /usr/share/icons/Papirus-Dark || true; gtk-update-icon-cache -f -t /usr/share/icons/Papirus || true; gtk-update-icon-cache -f -t /usr/share/icons/Adwaita || true; fc-cache -f || true'
	install -m 0755 target/release/forge-tier $(ROOTFS)/usr/local/bin/forge-tier
	install -m 0755 target/release/forge-conf $(ROOTFS)/usr/local/bin/forge-conf
	install -m 0755 target/release/forged $(ROOTFS)/usr/local/bin/forged
	install -m 0755 target/release/dev $(ROOTFS)/usr/local/bin/dev
	install -m 0755 target/release/forge-shell $(ROOTFS)/usr/local/bin/forge-shell
	install -m 0755 target/release/forge-settings $(ROOTFS)/usr/local/bin/forge-settings
	install -m 0755 target/release/forge-lab $(ROOTFS)/usr/local/bin/forge-lab
	mkdir -p $(ROOTFS)/usr/share/forge/catalog $(ROOTFS)/usr/share/forge/proxies $(ROOTFS)/usr/share/forge/wallpaper $(ROOTFS)/usr/share/themes
	cp -a data/catalog/. $(ROOTFS)/usr/share/forge/catalog/
	cp -a data/systemd/proxies/. $(ROOTFS)/usr/share/forge/proxies/
	install -m 0644 branding/wallpaper/forge-cherry.png $(ROOTFS)/usr/share/forge/wallpaper/forge-cherry.png
	cp -a data/theme/Forge-Cherry $(ROOTFS)/usr/share/themes/
	install -m 0644 data/systemd/forged.socket $(ROOTFS)/etc/systemd/system/forged.socket
	install -m 0644 data/systemd/forged.service $(ROOTFS)/etc/systemd/system/forged.service
	mkdir -p $(ROOTFS)/usr/share/polkit-1/actions
	install -m 0644 data/systemd/org.forge.catalog.policy $(ROOTFS)/usr/share/polkit-1/actions/org.forge.catalog.policy
	(cd $(ROOTFS)/usr/share/forge/catalog && sha256sum */*.toml > catalog.sha256)
	ln -sf /etc/systemd/system/forged.socket $(ROOTFS)/etc/systemd/system/sockets.target.wants/forged.socket
	# Keep Bluetooth & CUPS disabled at boot (0 MB idle); enable cups.socket for on-demand printing
	chroot $(ROOTFS) /bin/sh -c 'systemctl disable bluetooth.service cups.service 2>/dev/null || true; systemctl enable cups.socket 2>/dev/null || true'
	install -m 0644 data/systemd/forge-config-backup.service $(ROOTFS)/etc/systemd/system/forge-config-backup.service
	install -m 0644 data/systemd/forge-config-backup.timer $(ROOTFS)/etc/systemd/system/forge-config-backup.timer
	ln -sf /etc/systemd/system/forge-config-backup.timer $(ROOTFS)/etc/systemd/system/timers.target.wants/forge-config-backup.timer
	mkdir -p $(ROOTFS)/etc/greetd
	printf '[terminal]\nvt = 1\n\n[default_session]\ncommand = "/usr/local/bin/forge-session"\nuser = "forge"\n' > $(ROOTFS)/etc/greetd/config.toml
	chroot $(ROOTFS) /bin/sh -c 'id -u forge >/dev/null 2>&1 || (useradd -m -G sudo,video,render,input -s /bin/bash forge && echo "forge:forge" | chpasswd)'
	target/release/forge-conf --out $(ROOTFS)/home/forge/.config
	chmod +x $(ROOTFS)/home/forge/.config/labwc/autostart
	chroot $(ROOTFS) chown -R forge:forge /home/forge/.config
	# Install Forge OS Plymouth boot splash (Duck logo + FORGE OS + Dark Cherry loading bar)
	mkdir -p $(ROOTFS)/usr/share/plymouth/themes/forge
	cp -a branding/plymouth/forge/. $(ROOTFS)/usr/share/plymouth/themes/forge/
	chroot $(ROOTFS) /bin/sh -c 'plymouth-set-default-theme -R forge || true'
	# Snapshot pristine core OS files for non-destructive Factory Core Restore
	mkdir -p $(ROOTFS)/var/lib/forge/factory-core/usr/local/bin $(ROOTFS)/var/lib/forge/factory-core/usr/share/forge $(ROOTFS)/var/lib/forge/factory-core/etc/sysctl.d $(ROOTFS)/var/lib/forge/factory-core/etc/default
	cp -a $(ROOTFS)/usr/local/bin/forge-* $(ROOTFS)/var/lib/forge/factory-core/usr/local/bin/ 2>/dev/null || true
	cp -a $(ROOTFS)/usr/share/forge/. $(ROOTFS)/var/lib/forge/factory-core/usr/share/forge/
	cp -a $(ROOTFS)/etc/sysctl.d/99-forge.conf $(ROOTFS)/var/lib/forge/factory-core/etc/sysctl.d/ 2>/dev/null || true
	cp -a $(ROOTFS)/etc/default/earlyoom $(ROOTFS)/var/lib/forge/factory-core/etc/default/ 2>/dev/null || true
	cp -L $(ROOTFS)/boot/vmlinuz-* $(BUILD)/vmlinuz
	cp -L $(ROOTFS)/boot/initrd.img-* $(BUILD)/initrd.img

image: configure
	@echo "[M5] Building ext4 disk image $(IMG) ($(IMG_SIZE))..."
	rm -f $(IMG)
	truncate -s $(IMG_SIZE) $(IMG)
	mkfs.ext4 -F -L FORGE_ROOT -d $(ROOTFS) $(IMG)
	@echo "[M5] Ready: $(IMG)"

clean:
	rm -rf $(BUILD)
