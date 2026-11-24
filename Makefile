.PHONY: help host-check test-all build-all preview-shell preview-settings rootfs vm-tier0 iso vm-iso clean

help:
	@echo "Forge OS M0-M6 Targets (Linux Mint host):"
	@echo "  make test-all          Run all CI gates (fmt, clippy -D warnings, all 10 crates, brand)"
	@echo "  make build-all         Build all release binaries (M1-M5)"
	@echo "  make preview-shell     Preview forge-shell launcher on Linux Mint"
	@echo "  make preview-settings  Preview forge-settings 6-page app on Linux Mint"
	@echo "  make rootfs            Rebuild full Forge OS Tier 0 disk image (sudo)"
	@echo "  make vm-tier0          Boot full Forge OS desktop in QEMU (Tier 0 profile)"
	@echo "  make iso               Build bootable hybrid UEFI+BIOS live ISO (forge-os-1.0.iso)"
	@echo "  make vm-iso            Boot forge-os-1.0.iso in QEMU to verify live USB boot"

host-check:
	@bash scripts/check_mint_host.sh

test-all:
	cargo fmt --all -- --check
	cargo clippy --workspace --all-targets -- -D warnings
	cargo test --workspace
	@bash tests/unit/test_brand_acceptance.sh
	@bash tests/unit/test_backup_allowlist.sh

build-all:
	cargo build --release -p forge-tier -p forge-conf -p forged -p dev-cli
	cargo build --release -p forge-shell --features gtk-ui
	cargo build --release -p forge-settings --features gtk-ui
	cargo build --release -p forge-lab --features gtk-ui

preview-shell:
	cargo run -p forge-shell --features gtk-ui -- --mode launcher --catalog-dir data/catalog

preview-lab:
	cargo run -p forge-lab --features gtk-ui

preview-settings:
	cargo run -p forge-settings --features gtk-ui -- --catalog-dir data/catalog

rootfs: build-all
	sudo $(MAKE) -f image/rootfs.mk BUILD=/var/tmp/forge-build all
	sudo chown "$$USER:$$USER" /var/tmp/forge-build/forge-m0-tier0.img /var/tmp/forge-build/vmlinuz /var/tmp/forge-build/initrd.img

vm-tier0:
	@bash tests/vm/boot_tier0.sh

iso: rootfs
	sudo $(MAKE) -f image/iso.mk BUILD=/var/tmp/forge-build iso
	sudo chown "$$USER:$$USER" /var/tmp/forge-build/forge-os-1.0.iso

vm-iso:
	@bash tests/vm/boot_iso.sh

usb:
	@bash scripts/flash-usb.sh

clean:
	cargo clean
	sudo rm -rf /var/tmp/forge-build

test-devices:
	@bash tests/vm/simulate_all_devices.sh

sim-install:
	@bash tests/vm/simulate_install_interactive.sh
