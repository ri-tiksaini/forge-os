# Forge OS 1.0

> *"Everything you need is already here. Everything you don't need is one word away — and never runs until you ask."*

**Forge OS 1.0** is a lightweight, developer-first Linux distribution built on **Debian 13 ("trixie")** and **Linux 6.12 LTS** with a custom **Dark Cherry & Obsidian** desktop, zero telemetry, zero-RAM parked databases, and a built-in **Developer Labs** sandbox for modifying and testing core OS files safely.

## Highlights

- **Ultra-Lean Base**: Boots in ~11 seconds with ~66 MiB idle PSS; supports legacy/low-RAM hardware via automatic Tier 0 software rendering (`WLR_RENDERER=pixman`, `GSK_RENDERER=cairo`).
- **Windows-Familiar Wayland Desktop**: `labwc` stacking window manager (`_ □ ×` titlebars, mouse drag/resize) + bottom `waybar` taskbar, Duck Start Launcher (`Win`), Files (`Win+E`), Settings (`Win+I`), Code Editor (`Win+N`), Quick Settings (`Win+A`), Clipboard History (`Win+V`), and Smart Screenshots (`Win+Shift+S`).
- **Zero-RAM Package & Service Engine**: Socket-activated `forged` daemon and `dev` CLI (`search`, `add`, `remove`, `list`, `on/off`, `doctor`) with `systemd-socket-proxyd` zero-RAM parking for PostgreSQL, Redis, and MariaDB.
- **Whole-OS Theme Engine**: 5 switchable color themes in Settings (`forge-apply-theme`) that update GTK4, GTK3, Waybar, `labwc`, `foot`, and all Forge apps together.
- **Developer Labs (`forge-lab`)**: Authenticated core-system workbench with a Master Wiring Helpbook, 65+ core files, Virtual OS simulation (real `cargo check`, `bash -n`, `tomllib`, JSON, CSS, XML, systemd checks), atomic rollback-safe live patching, and non-destructive **Factory Core Restore** (`forge-repair factory`, never touches `/home`).
- **Built-In Security & Troubleshooting**: System-wide `/etc/hosts` + Firefox policy ad blocker (`forge-adblock`), local failed-command tracker (`forge-track`), background intrusion scanner (`forge-watch`), firewall/lockdown (`forge-security`), network & port diagnostics (`forge-netcheck`, `forge-ports`), and 1-click RAM cleaner (`forge-clean`).

## Repository Layout

```text
forge-os/
├── branding/        # Brand palette (brand.toml), Solid Black Duck SVG, wallpaper, Plymouth boot splash
├── crates/          # Rust workspace (11 crates: forge-brand, forge-tier, forge-conf, forge-proto,
│                    #   forge-catalog, forge-txn, forged, dev-cli, forge-shell, forge-settings, forge-lab)
├── data/            # Curated 30-entry dev catalog, keymaps, systemd units, DB proxies, Waybar & adblock data
├── docs/            # Offline HTML Helpbook & architecture decision records
├── image/           # Debian 13 rootfs builder (rootfs.mk), package list, hybrid UEFI+BIOS ISO builder (iso.mk)
├── scripts/         # Core /usr/local/bin/forge-* scripts, bare-metal installer, USB flasher, theme applier
├── tests/           # Unit tests, allowlisted backup tests, and multi-device QEMU simulation scripts
└── Makefile         # One-command build, test, preview, ISO, QEMU, and USB targets
```

---

## Building Forge OS on Another Linux PC

These steps work on any **Debian 13 / Ubuntu 24.04+ / Linux Mint 22+** x86_64 machine with at least **15 GB free disk space** (for `/var/tmp/forge-build` and `target/`).

### 1. Install Host Dependencies

```bash
sudo apt-get update
sudo apt-get install -y \
  build-essential pkg-config git curl make \
  libgtk-4-dev libglib2.0-dev libcairo2-dev libpango1.0-dev libgdk-pixbuf-2.0-dev \
  debootstrap squashfs-tools dosfstools mtools xorriso \
  grub-pc-bin grub-efi-amd64-bin grub-common \
  qemu-system-x86 ovmf python3
```

### 2. Install Rust `1.88.0`

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source "$HOME/.cargo/env"
rustup toolchain install 1.88.0
rustup default 1.88.0
rustup component add rustfmt clippy
```

### 3. Clone This Repository

```bash
git clone https://github.com/<YOUR_USERNAME>/forge-os.git
cd forge-os
```

### 4. Verify Code & Preview Apps Locally (Optional)

Run the full test suite (formatting, `clippy -D warnings`, all 11 crates, brand & backup checks):

```bash
make test-all
```

Preview individual GTK4 desktop apps directly on your host desktop:

```bash
make preview-shell      # Duck Start Launcher
make preview-settings   # 9-page Control Center
make preview-lab        # Developer Labs
```

---

## Building the Bootable Hybrid ISO (UEFI + Legacy BIOS)

> **Note:** Build staging automatically uses `/var/tmp/forge-build` (which supports device nodes and executables even if your repo sits on an external or `noexec` drive).

```bash
make iso
```

What `make iso` runs automatically:
1. `make build-all` — Compiles all Rust binaries in `--release` mode.
2. `make rootfs` — Bootstraps a clean 2-stage Debian 13 `trixie` rootfs, installs all Forge binaries/scripts/themes/Plymouth splash, and snapshots `/var/lib/forge/factory-core`.
3. `image/iso.mk` — Compresses the rootfs with `zstd` SquashFS and builds a hybrid UEFI + Legacy BIOS bootable ISO via `grub-mkrescue`.

**Output ISO path:**
```text
/var/tmp/forge-build/forge-os-1.0.iso
```

---

## Testing in QEMU (Before Flashing Hardware)

Boot the generated live ISO in QEMU:

```bash
make vm-iso
```

Or boot the raw Tier 0 ext4 disk image:

```bash
make vm-tier0
```

Run the multi-device hardware profile simulation (Legacy BIOS, UEFI, low-RAM Tier 0):

```bash
make test-devices
```

*(Default live session login if prompted: user `forge`, password `forge`.)*

---

## Flashing to a Real USB Drive & Bare-Metal Install

### Flash USB (Interactive Safe Flasher)

Plug in a USB drive (4 GB+ recommended) and run:

```bash
make usb
```

Or flash manually with `dd` (replace `/dev/sdX` with your actual USB device from `lsblk` — **double-check the device name**):

```bash
sudo dd if=/var/tmp/forge-build/forge-os-1.0.iso of=/dev/sdX bs=4M status=progress oflag=sync
```

### Install to Bare-Metal PC / Laptop

1. Boot the target PC from the USB stick (works on both **UEFI** and **Legacy BIOS** machines).
2. Once the Forge OS desktop loads, open a terminal and run:
   ```bash
   sudo forge-installer
   ```
3. Follow the prompts to pick your target NVMe/SATA disk and reboot into your installed Forge OS 1.0 system.

---

## Recovery Commands Inside Forge OS

- `forge-repair configs` — Regenerates `labwc`, `foot`, and GTK4/GTK3 configs.
- `forge-repair rollback` — Undoes the latest Developer Labs live patch snapshot (never touches `/home`).
- `forge-repair factory` — Restores all core OS files from `/var/lib/forge/factory-core` while leaving `/home` untouched.
- `make clean` — Removes local `target/` and `/var/tmp/forge-build`.
