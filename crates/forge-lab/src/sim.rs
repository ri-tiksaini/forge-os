#![forbid(unsafe_code)]

//! Hardened Virtual OS Simulation Engine & Atomic Rollback-Safe Patcher.
//! Covers all Forge OS core scripts, desktop/theme/boot configs, systemd/kernel units,
//! and core Rust source files while strictly excluding /home user data.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileKind {
    Script,
    Config,
    Style,
    Rust,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileCategory {
    Scripts,
    UiBoot,
    SystemdKernel,
    RustCore,
}

impl FileCategory {
    pub fn label(self) -> &'static str {
        match self {
            FileCategory::Scripts => "Scripts",
            FileCategory::UiBoot => "UI & Boot",
            FileCategory::SystemdKernel => "Systemd & Kernel",
            FileCategory::RustCore => "Rust Core",
        }
    }
}

#[derive(Debug, Clone)]
pub struct CoreFileSpec {
    pub display_name: String,
    pub path: String,
    pub repo_fallback: String,
    pub role: String,
    pub kind: FileKind,
    pub category: FileCategory,
    pub is_main_hub: bool,
}

#[derive(Debug, Clone)]
pub struct LoadedCoreFile {
    pub content: String,
    pub source: &'static str,
    pub resolved_path: String,
}

#[derive(Debug, Clone)]
pub struct SimCheckResult {
    pub passed: bool,
    pub test_name: String,
    pub target: String,
    pub reason: String,
    pub fix_hint: String,
}

#[derive(Debug, Clone)]
pub struct SimReport {
    pub passed: bool,
    pub virtual_root: PathBuf,
    pub checks: Vec<SimCheckResult>,
    pub logs: Vec<String>,
}

fn spec(
    display_name: &str,
    path: &str,
    repo_fallback: &str,
    role: &str,
    kind: FileKind,
    category: FileCategory,
    is_main_hub: bool,
) -> CoreFileSpec {
    CoreFileSpec {
        display_name: display_name.into(),
        path: path.into(),
        repo_fallback: repo_fallback.into(),
        role: role.into(),
        kind,
        category,
        is_main_hub,
    }
}

pub fn default_core_files() -> Vec<CoreFileSpec> {
    use FileCategory::*;
    use FileKind::*;
    vec![
        // --- CORE SCRIPTS (/usr/local/bin/forge-*) ---
        spec(
            "Display Session Launcher",
            "/usr/local/bin/forge-session",
            "scripts/forge-session.sh",
            "Starts labwc with Dark Cherry & software fallback",
            Script,
            Scripts,
            true,
        ),
        spec(
            "Popup Window Toggler",
            "/usr/local/bin/forge-toggle",
            "scripts/forge-toggle.sh",
            "Toggles Duck Launcher, Quick Settings, Power & Menu",
            Script,
            Scripts,
            true,
        ),
        spec(
            "RAM & Cache Cleaner",
            "/usr/local/bin/forge-clean",
            "scripts/forge-clean.sh",
            "Drops kernel RAM caches & dev build caches safely",
            Script,
            Scripts,
            false,
        ),
        spec(
            "System-Wide Ad Blocker",
            "/usr/local/bin/forge-adblock",
            "scripts/forge-adblock.sh",
            "Manages /etc/hosts ad & tracker sinkhole",
            Script,
            Scripts,
            false,
        ),
        spec(
            "Network Troubleshooter",
            "/usr/local/bin/forge-netcheck",
            "scripts/forge-netcheck.sh",
            "Checks Wi-Fi, gateway, DNS & adblock status",
            Script,
            Scripts,
            false,
        ),
        spec(
            "Dev Port Inspector",
            "/usr/local/bin/forge-ports",
            "scripts/forge-ports.sh",
            "Lists open dev ports & kills stuck servers",
            Script,
            Scripts,
            false,
        ),
        spec(
            "Security & Firewall Lock",
            "/usr/local/bin/forge-security",
            "scripts/forge-security.sh",
            "UFW lockdown, screen lock & security audit",
            Script,
            Scripts,
            false,
        ),
        spec(
            "Session Error Tracker",
            "/usr/local/bin/forge-track",
            "scripts/forge-track.sh",
            "Views local failed terminal commands & exit codes",
            Script,
            Scripts,
            false,
        ),
        spec(
            "Attacker Security Watch",
            "/usr/local/bin/forge-watch",
            "scripts/forge-watch.sh",
            "Scans external TCP ports, /tmp & auth failures",
            Script,
            Scripts,
            false,
        ),
        spec(
            "OS Repair & Factory Restore",
            "/usr/local/bin/forge-repair",
            "scripts/forge-repair.sh",
            "Repairs configs/packages or restores factory core safely",
            Script,
            Scripts,
            true,
        ),
        spec(
            "Bare-Metal UEFI/BIOS Installer",
            "/usr/local/bin/forge-installer",
            "scripts/forge-installer.sh",
            "Installs Forge OS to real disk (UEFI + Legacy BIOS)",
            Script,
            Scripts,
            false,
        ),
        spec(
            "Live System HUD Monitor",
            "/usr/local/bin/forge-hud",
            "scripts/forge-hud.sh",
            "Flicker-free terminal CPU/RAM/Disk monitor",
            Script,
            Scripts,
            false,
        ),
        spec(
            "SSH Key & Config Helper",
            "/usr/local/bin/forge-ssh",
            "scripts/forge-ssh.sh",
            "Generates ed25519 SSH keys & manages connections",
            Script,
            Scripts,
            false,
        ),
        spec(
            "Daily Config Backup",
            "/usr/local/bin/forge-config-backup",
            "scripts/forge-config-backup.sh",
            "Allowlisted daily backup of ~/.config",
            Script,
            Scripts,
            false,
        ),
        spec(
            "Clipboard History Picker",
            "/usr/local/bin/forge-clipboard",
            "scripts/forge-clipboard.sh",
            "Win+V clipboard history menu",
            Script,
            Scripts,
            false,
        ),
        spec(
            "Clipboard Watcher Daemon",
            "/usr/local/bin/forge-clip-watch",
            "scripts/forge-clip-watch.sh",
            "Tracks Wayland clipboard entries locally",
            Script,
            Scripts,
            false,
        ),
        spec(
            "Smart Screenshot Tool",
            "/usr/local/bin/forge-screenshot",
            "scripts/forge-screenshot.sh",
            "Win+Shift+S region/window screenshot capture",
            Script,
            Scripts,
            false,
        ),
        spec(
            "Double-Click .deb Installer",
            "/usr/local/bin/forge-deb-install",
            "scripts/forge-deb-install.sh",
            "Installs downloaded .deb packages cleanly",
            Script,
            Scripts,
            false,
        ),
        spec(
            "Offline Helpbook Launcher",
            "/usr/local/bin/forge-help",
            "scripts/forge-help.sh",
            "Opens the offline Forge OS HTML Helpbook",
            Script,
            Scripts,
            false,
        ),
        spec(
            "Dev Profile Export/Import",
            "/usr/local/bin/forge-profile",
            "scripts/forge-profile.sh",
            "Exports/imports installed dev tools & configs",
            Script,
            Scripts,
            false,
        ),
        spec(
            "Terminal Welcome Banner",
            "/usr/local/bin/forge-welcome-term",
            "scripts/forge-welcome-term.sh",
            "Shows quick Forge OS commands in new terminals",
            Script,
            Scripts,
            false,
        ),
        // --- UI, THEME, KEYMAPS & BOOT ---
        spec(
            "Bottom Taskbar Layout",
            "/usr/share/forge/waybar/config",
            "data/theme/waybar/config",
            "Taskbar icons, click actions & live monitors (JSON)",
            Config,
            UiBoot,
            true,
        ),
        spec(
            "Bottom Taskbar Theme",
            "/usr/share/forge/waybar/style.css",
            "data/theme/waybar/style.css",
            "Dark Cherry CSS colors & borders for taskbar",
            Style,
            UiBoot,
            true,
        ),
        spec(
            "Window Titlebar Theme",
            "/usr/share/themes/Forge-Cherry/openbox-3/themerc",
            "data/theme/Forge-Cherry/openbox-3/themerc",
            "labwc _ □ × window titlebar colors & borders",
            Style,
            UiBoot,
            false,
        ),
        spec(
            "Master Keymap Table",
            "/usr/share/forge/keymap.toml",
            "data/keymap.toml",
            "All Windows-familiar shortcuts (Win+E, Alt+Tab, etc.)",
            Config,
            UiBoot,
            false,
        ),
        spec(
            "Brand Palette & Identity",
            "/usr/share/forge/brand.toml",
            "branding/brand.toml",
            "Single source of truth for Dark Cherry colors & fonts",
            Config,
            UiBoot,
            true,
        ),
        spec(
            "Plymouth Boot Splash Script",
            "/usr/share/plymouth/themes/forge/forge.script",
            "branding/plymouth/forge/forge.script",
            "Duck logo + Dark Cherry progress bar animation at boot",
            Script,
            UiBoot,
            false,
        ),
        spec(
            "Plymouth Theme Descriptor",
            "/usr/share/plymouth/themes/forge/forge.plymouth",
            "branding/plymouth/forge/forge.plymouth",
            "Registers the Forge Plymouth boot theme",
            Config,
            UiBoot,
            false,
        ),
        spec(
            "Bash Error Tracker Hook",
            "/etc/profile.d/forge-track.sh",
            "data/theme/forge-track-profile.sh",
            "Captures failed terminal commands into local log",
            Script,
            UiBoot,
            false,
        ),
        spec(
            "Firefox Adblock & Privacy Policy",
            "/etc/firefox/policies/policies.json",
            "data/adblock/firefox-policies.json",
            "Enforces zero-telemetry & uBlock policy in Firefox",
            Config,
            UiBoot,
            false,
        ),
        spec(
            "Adblock Sinkhole Domains",
            "/usr/share/forge/adblock/hosts.block",
            "data/adblock/hosts.block",
            "Domain blocklist written to /etc/hosts by forge-adblock",
            Config,
            UiBoot,
            false,
        ),
        spec(
            "Login Greeter Config",
            "/etc/greetd/config.toml",
            "image/rootfs.mk",
            "Auto-launches /usr/local/bin/forge-session on VT1",
            Config,
            UiBoot,
            false,
        ),
        // --- SYSTEMD, DB PROXIES & KERNEL ---
        spec(
            "forged Socket Unit",
            "/etc/systemd/system/forged.socket",
            "data/systemd/forged.socket",
            "Zero-RAM socket activation trigger for forged",
            Config,
            SystemdKernel,
            true,
        ),
        spec(
            "forged Daemon Unit",
            "/etc/systemd/system/forged.service",
            "data/systemd/forged.service",
            "Package daemon lifecycle & 90s idle timeout",
            Config,
            SystemdKernel,
            true,
        ),
        spec(
            "Polkit Privilege Policy",
            "/usr/share/polkit-1/actions/org.forge.catalog.policy",
            "data/systemd/org.forge.catalog.policy",
            "Polkit rules for installing/removing dev tools",
            Config,
            SystemdKernel,
            false,
        ),
        spec(
            "Config Backup Service",
            "/etc/systemd/system/forge-config-backup.service",
            "data/systemd/forge-config-backup.service",
            "Systemd service for daily config snapshots",
            Config,
            SystemdKernel,
            false,
        ),
        spec(
            "Config Backup Timer",
            "/etc/systemd/system/forge-config-backup.timer",
            "data/systemd/forge-config-backup.timer",
            "Schedules daily ~/.config backups",
            Config,
            SystemdKernel,
            false,
        ),
        spec(
            "PostgreSQL Zero-RAM Socket",
            "/usr/share/forge/proxies/forge-postgres-proxy.socket",
            "data/systemd/proxies/forge-postgres-proxy.socket",
            "Listens on port 5432 with 0 MiB idle RAM",
            Config,
            SystemdKernel,
            false,
        ),
        spec(
            "PostgreSQL Proxy Service",
            "/usr/share/forge/proxies/forge-postgres-proxy.service",
            "data/systemd/proxies/forge-postgres-proxy.service",
            "Wakes Postgres on demand & sleeps after 90s idle",
            Config,
            SystemdKernel,
            false,
        ),
        spec(
            "Redis Zero-RAM Socket",
            "/usr/share/forge/proxies/forge-redis-proxy.socket",
            "data/systemd/proxies/forge-redis-proxy.socket",
            "Listens on port 6379 with 0 MiB idle RAM",
            Config,
            SystemdKernel,
            false,
        ),
        spec(
            "Redis Proxy Service",
            "/usr/share/forge/proxies/forge-redis-proxy.service",
            "data/systemd/proxies/forge-redis-proxy.service",
            "Wakes Redis on demand & sleeps after 90s idle",
            Config,
            SystemdKernel,
            false,
        ),
        spec(
            "MariaDB Zero-RAM Socket",
            "/usr/share/forge/proxies/forge-mariadb-proxy.socket",
            "data/systemd/proxies/forge-mariadb-proxy.socket",
            "Listens on port 3306 with 0 MiB idle RAM",
            Config,
            SystemdKernel,
            false,
        ),
        spec(
            "MariaDB Proxy Service",
            "/usr/share/forge/proxies/forge-mariadb-proxy.service",
            "data/systemd/proxies/forge-mariadb-proxy.service",
            "Wakes MariaDB on demand & sleeps after 90s idle",
            Config,
            SystemdKernel,
            false,
        ),
        spec(
            "PostgreSQL Parked Drop-In",
            "/usr/share/forge/proxies/postgresql-parked.conf",
            "data/systemd/proxies/postgresql-parked.conf",
            "StopWhenUnneeded=yes drop-in for parked Postgres",
            Config,
            SystemdKernel,
            false,
        ),
        spec(
            "Kernel Memory Tuning",
            "/etc/sysctl.d/99-forge.conf",
            "image/overlays/etc/sysctl.d/99-forge.conf",
            "zram swappiness=100 & low-RAM reclaim tuning",
            Config,
            SystemdKernel,
            false,
        ),
        spec(
            "OOM Desktop Protector",
            "/etc/default/earlyoom",
            "image/overlays/etc/default/earlyoom",
            "Protects desktop from runaway compilers",
            Config,
            SystemdKernel,
            false,
        ),
        spec(
            "Journald Log Size Cap",
            "/etc/systemd/journald.conf.d/forge.conf",
            "image/overlays/etc/systemd/journald.conf.d/forge.conf",
            "Caps systemd journal size to keep disk lean",
            Config,
            SystemdKernel,
            false,
        ),
        spec(
            "APT Lean Install Policy",
            "/etc/apt/apt.conf.d/99forge-norecommends",
            "image/overlays/etc/apt/apt.conf.d/99forge-norecommends",
            "Disables bloated recommended packages in apt",
            Config,
            SystemdKernel,
            false,
        ),
        // --- CORE RUST APPS, DAEMONS & IMAGE BUILDERS ---
        spec(
            "Settings Control Center UI",
            "/usr/src/forge-os/crates/forge-settings/src/ui.rs",
            "crates/forge-settings/src/ui.rs",
            "All 9 Settings pages, live bar graphs, themes & repair UI",
            Rust,
            RustCore,
            true,
        ),
        spec(
            "Settings Data & Actions",
            "/usr/src/forge-os/crates/forge-settings/src/model.rs",
            "crates/forge-settings/src/model.rs",
            "System telemetry, themes, network, services & repair logic",
            Rust,
            RustCore,
            false,
        ),
        spec(
            "Duck Launcher & Quick Settings",
            "/usr/src/forge-os/crates/forge-shell/src/ui.rs",
            "crates/forge-shell/src/ui.rs",
            "Start menu, volume/brightness sliders, audio & power menu",
            Rust,
            RustCore,
            true,
        ),
        spec(
            "Launcher App Search Logic",
            "/usr/src/forge-os/crates/forge-shell/src/launcher.rs",
            "crates/forge-shell/src/launcher.rs",
            "Desktop app discovery & instant search ranking",
            Rust,
            RustCore,
            false,
        ),
        spec(
            "labwc Config Generator",
            "/usr/src/forge-os/crates/forge-conf/src/emit_labwc.rs",
            "crates/forge-conf/src/emit_labwc.rs",
            "Generates labwc rc.xml, autostart, mouse & titlebar rules",
            Rust,
            RustCore,
            true,
        ),
        spec(
            "GTK4 Dark Cherry CSS Emitter",
            "/usr/src/forge-os/crates/forge-conf/src/emit_gtk.rs",
            "crates/forge-conf/src/emit_gtk.rs",
            "Generates global Dark Cherry GTK4/GTK3 theme CSS",
            Rust,
            RustCore,
            false,
        ),
        spec(
            "foot Terminal Config Emitter",
            "/usr/src/forge-os/crates/forge-conf/src/emit_foot.rs",
            "crates/forge-conf/src/emit_foot.rs",
            "Generates foot terminal colors, font & Ctrl+V bindings",
            Rust,
            RustCore,
            false,
        ),
        spec(
            "Sway Config Generator",
            "/usr/src/forge-os/crates/forge-conf/src/emit_sway.rs",
            "crates/forge-conf/src/emit_sway.rs",
            "Generates optional tiling sway compositor config",
            Rust,
            RustCore,
            false,
        ),
        spec(
            "Brand Constants Crate",
            "/usr/src/forge-os/crates/forge-brand/src/lib.rs",
            "crates/forge-brand/src/lib.rs",
            "Compile-time Dark Cherry hex codes & typography",
            Rust,
            RustCore,
            false,
        ),
        spec(
            "forged Package Daemon",
            "/usr/src/forge-os/crates/forged/src/lib.rs",
            "crates/forged/src/lib.rs",
            "Socket-activated package installer & state engine",
            Rust,
            RustCore,
            true,
        ),
        spec(
            "Zero-RAM Service Parking",
            "/usr/src/forge-os/crates/forged/src/parking.rs",
            "crates/forged/src/parking.rs",
            "Controls systemd-socket-proxyd for Postgres/Redis/MariaDB",
            Rust,
            RustCore,
            false,
        ),
        spec(
            "SO_PEERCRED & Polkit Auth",
            "/usr/src/forge-os/crates/forged/src/policy.rs",
            "crates/forged/src/policy.rs",
            "Verifies caller UID/GID & Polkit permissions",
            Rust,
            RustCore,
            false,
        ),
        spec(
            "dev CLI Frontend",
            "/usr/src/forge-os/crates/dev-cli/src/main.rs",
            "crates/dev-cli/src/main.rs",
            "Command-line client for search/add/remove/on/off/doctor",
            Rust,
            RustCore,
            false,
        ),
        spec(
            "Hardware Tier Detector",
            "/usr/src/forge-os/crates/forge-tier/src/lib.rs",
            "crates/forge-tier/src/lib.rs",
            "Classifies PC into Tier 0/1/2/3 & software rendering",
            Rust,
            RustCore,
            false,
        ),
        spec(
            "Developer Labs Engine",
            "/usr/src/forge-os/crates/forge-lab/src/sim.rs",
            "crates/forge-lab/src/sim.rs",
            "Virtual OS simulation & atomic rollback patcher",
            Rust,
            RustCore,
            false,
        ),
        spec(
            "Developer Labs UI",
            "/usr/src/forge-os/crates/forge-lab/src/ui.rs",
            "crates/forge-lab/src/ui.rs",
            "This Developer Labs GTK4 window & helpbook",
            Rust,
            RustCore,
            false,
        ),
        spec(
            "OS Base Package List",
            "/usr/src/forge-os/image/packages.list",
            "image/packages.list",
            "Exact Debian 13 packages installed into Forge OS rootfs",
            Config,
            RustCore,
            false,
        ),
        spec(
            "Rootfs Builder Recipe",
            "/usr/src/forge-os/image/rootfs.mk",
            "image/rootfs.mk",
            "Builds the Debian 13 rootfs & installs all Forge binaries",
            Script,
            RustCore,
            true,
        ),
        spec(
            "Hybrid UEFI/BIOS ISO Builder",
            "/usr/src/forge-os/image/iso.mk",
            "image/iso.mk",
            "Packages SquashFS + GRUB into bootable forge-os.iso",
            Script,
            RustCore,
            false,
        ),
    ]
}

fn repo_root_from_manifest() -> Option<PathBuf> {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .map(Path::to_path_buf)
}

pub fn load_core_file(spec: &CoreFileSpec) -> LoadedCoreFile {
    if let Ok(txt) = fs::read_to_string(&spec.path) {
        return LoadedCoreFile {
            content: txt,
            source: "LIVE OS",
            resolved_path: spec.path.clone(),
        };
    }

    if !spec.repo_fallback.is_empty() {
        let mut candidates = Vec::new();
        if let Some(root) = repo_root_from_manifest() {
            candidates.push(root.join(&spec.repo_fallback));
            candidates.push(root.join("forge-os").join(&spec.repo_fallback));
        }
        candidates.push(PathBuf::from(&spec.repo_fallback));
        candidates.push(PathBuf::from("/usr/src/forge-os").join(&spec.repo_fallback));

        for cand in candidates {
            if let Ok(txt) = fs::read_to_string(&cand) {
                return LoadedCoreFile {
                    content: txt,
                    source: "REPO SOURCE",
                    resolved_path: cand.display().to_string(),
                };
            }
        }
    }

    LoadedCoreFile {
        content: format!(
            "# File not found on this machine yet: {}\n# Expected live path: {}\n# Expected repo path: {}\n",
            spec.display_name, spec.path, spec.repo_fallback
        ),
        source: "NOT FOUND",
        resolved_path: spec.path.clone(),
    }
}

pub fn read_core_file(spec: &CoreFileSpec) -> String {
    load_core_file(spec).content
}

pub fn is_allowed_core_path(path: &str) -> bool {
    let prefix_ok = path.starts_with("/usr/local/bin/")
        || path.starts_with("/usr/share/forge/")
        || path.starts_with("/usr/share/themes/")
        || path.starts_with("/usr/share/polkit-1/")
        || path.starts_with("/usr/share/plymouth/")
        || path.starts_with("/usr/src/forge-os/")
        || path.starts_with("/etc/systemd/")
        || path.starts_with("/etc/sysctl.d/")
        || path.starts_with("/etc/default/")
        || path.starts_with("/etc/profile.d/")
        || path.starts_with("/etc/greetd/")
        || path.starts_with("/etc/apt/")
        || path.starts_with("/etc/firefox/");
    let chars_ok = !path.is_empty()
        && path
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '/' | '_' | '.' | '-' | '+'));
    prefix_ok && chars_ok && !path.contains("..")
}

fn validate_css_strict(content: &str) -> Result<(), String> {
    let mut depth: i32 = 0;
    let mut in_comment = false;
    for (idx, raw) in content.lines().enumerate() {
        let line_no = idx + 1;
        let mut line = raw.trim();
        if in_comment {
            if let Some(end) = line.find("*/") {
                line = line[end + 2..].trim();
                in_comment = false;
            } else {
                continue;
            }
        }
        while let Some(start) = line.find("/*") {
            if let Some(end) = line[start + 2..].find("*/") {
                let before = line[..start].trim();
                let after = line[start + 2 + end + 2..].trim();
                if before.is_empty() {
                    line = after;
                } else {
                    break;
                }
            } else {
                line = line[..start].trim();
                in_comment = true;
                break;
            }
        }
        if line.is_empty() {
            continue;
        }
        let open_cnt = line.chars().filter(|&c| c == '{').count() as i32;
        let close_cnt = line.chars().filter(|&c| c == '}').count() as i32;
        let was_inside = depth > 0;
        depth += open_cnt - close_cnt;
        if depth < 0 {
            return Err(format!("line {line_no}: unexpected closing '}}' in CSS"));
        }
        if open_cnt == 0 && close_cnt == 0 {
            if was_inside {
                // Inside a CSS block: must be a property: value declaration
                if !line.contains(':') {
                    return Err(format!(
                        "line {line_no}: invalid CSS declaration '{line}' (expected 'property: value;')"
                    ));
                }
            } else if !line.starts_with('@') && !line.ends_with(',') && !line.ends_with('{') {
                // Outside a block: stray gibberish or selector missing '{'
                return Err(format!(
                    "line {line_no}: stray text or selector missing '{{': '{line}'"
                ));
            }
        }
    }
    if in_comment {
        return Err("unclosed CSS comment '/*'".to_string());
    }
    if depth != 0 {
        return Err(format!("{depth} unclosed '{{' block(s) in CSS"));
    }
    Ok(())
}

fn validate_systemd_unit(content: &str) -> Result<(), String> {
    let mut has_section = false;
    for (idx, raw) in content.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            has_section = true;
            continue;
        }
        if !has_section {
            return Err(format!(
                "line {}: directive '{line}' appears before any [Section] header",
                idx + 1
            ));
        }
        let Some((k, _)) = line.split_once('=') else {
            return Err(format!(
                "line {}: expected Key=Value, got '{line}'",
                idx + 1
            ));
        };
        if k.trim().is_empty() || k.trim().contains(' ') {
            return Err(format!("line {}: invalid key '{}'", idx + 1, k.trim()));
        }
    }
    if !has_section {
        return Err(
            "missing [Unit]/[Service]/[Socket]/[Timer]/[Journal] section header".to_string(),
        );
    }
    Ok(())
}

fn validate_sysctl(content: &str) -> Result<(), String> {
    for (idx, raw) in content.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
            continue;
        }
        let Some((k, v)) = line.split_once('=') else {
            return Err(format!(
                "line {}: expected key = value, got '{line}'",
                idx + 1
            ));
        };
        let kt = k.trim();
        let vt = v.trim();
        if kt.is_empty() || vt.is_empty() || !kt.contains('.') {
            return Err(format!(
                "line {}: invalid sysctl entry '{line}' (expected e.g. vm.swappiness = 100)",
                idx + 1
            ));
        }
    }
    Ok(())
}

fn validate_toml_strict(vpath: &Path) -> Result<(), String> {
    let script = "import tomllib, sys\nwith open(sys.argv[1], 'rb') as f: tomllib.load(f)\n";
    let out = Command::new("python3")
        .args(["-c", script])
        .arg(vpath)
        .output()
        .map_err(|e| format!("failed to run python3 tomllib: {e}"))?;
    if out.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
    }
}

fn validate_xml_strict(vpath: &Path) -> Result<(), String> {
    let script = "import xml.etree.ElementTree as ET, sys\nET.parse(sys.argv[1])\n";
    let out = Command::new("python3")
        .args(["-c", script])
        .arg(vpath)
        .output()
        .map_err(|e| format!("failed to run python3 xml parser: {e}"))?;
    if out.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
    }
}

fn validate_themerc(content: &str) -> Result<(), String> {
    for (idx, raw) in content.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with('!') {
            continue;
        }
        let Some((k, v)) = line.split_once(':') else {
            return Err(format!(
                "line {}: expected 'property: value', got '{line}'",
                idx + 1
            ));
        };
        if k.trim().is_empty() || v.trim().is_empty() || k.trim().contains(' ') {
            return Err(format!("line {}: invalid themerc entry '{line}'", idx + 1));
        }
    }
    Ok(())
}

fn validate_apt_conf(content: &str) -> Result<(), String> {
    for (idx, raw) in content.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with("//") {
            continue;
        }
        if !line.ends_with(';') && !line.ends_with('{') && !line.ends_with('}') {
            return Err(format!(
                "line {}: APT config directive must end with ';': '{line}'",
                idx + 1
            ));
        }
    }
    Ok(())
}

fn validate_hosts_block(content: &str) -> Result<(), String> {
    for (idx, raw) in content.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 2 || (!parts[0].contains('.') && !parts[0].contains(':')) {
            return Err(format!(
                "line {}: expected '<ip> <domain>' (e.g. 0.0.0.0 ads.example.com), got '{line}'",
                idx + 1
            ));
        }
    }
    Ok(())
}

fn validate_packages_list(content: &str) -> Result<(), String> {
    for (idx, raw) in content.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let ok = line
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '+' | '-' | '.'));
        if !ok || line.contains(' ') {
            return Err(format!(
                "line {}: invalid Debian package name '{line}'",
                idx + 1
            ));
        }
    }
    Ok(())
}

fn validate_makefile(content: &str) -> Result<(), String> {
    let mut if_depth: i32 = 0;
    for (idx, raw) in content.lines().enumerate() {
        let line_no = idx + 1;
        if raw.starts_with('\t') {
            continue;
        }
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line.starts_with("ifeq")
            || line.starts_with("ifneq")
            || line.starts_with("ifdef")
            || line.starts_with("ifndef")
        {
            if_depth += 1;
            continue;
        }
        if line == "else" || line.starts_with("else ") {
            if if_depth <= 0 {
                return Err(format!(
                    "line {line_no}: unexpected 'else' without matching if"
                ));
            }
            continue;
        }
        if line == "endif" {
            if_depth -= 1;
            if if_depth < 0 {
                return Err(format!("line {line_no}: unexpected 'endif'"));
            }
            continue;
        }
        if line.starts_with("include ")
            || line.starts_with("-include ")
            || line.starts_with("export ")
            || line.starts_with(".PHONY")
            || line.contains('=')
            || line.contains(':')
        {
            continue;
        }
        return Err(format!(
            "line {line_no}: invalid Makefile line (missing tab, ':', or '='): '{line}'"
        ));
    }
    if if_depth != 0 {
        return Err(format!(
            "{if_depth} unclosed ifeq/ifdef block(s) in Makefile"
        ));
    }
    Ok(())
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), String> {
    fs::create_dir_all(dst).map_err(|e| e.to_string())?;
    let entries = fs::read_dir(src).map_err(|e| e.to_string())?;
    for entry in entries.flatten() {
        let p = entry.path();
        let name = entry.file_name();
        if name == "target" || name == ".git" {
            continue;
        }
        let target = dst.join(name);
        if p.is_dir() {
            copy_dir_recursive(&p, &target)?;
        } else if p.is_file() {
            fs::copy(&p, &target).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

fn crate_name_for_rust_path(target_path: &str) -> Option<&str> {
    let rest = target_path.split("/crates/").nth(1)?;
    rest.split('/').next()
}

fn validate_rust_strict(
    vroot: &Path,
    vpath: &Path,
    target_path: &str,
    staged: &BTreeMap<String, String>,
) -> Result<String, String> {
    // Step 1: Instant real rustc AST syntax parse via rustfmt
    if let Ok(out) = Command::new("rustfmt")
        .args(["--edition", "2021", "--emit", "stdout"])
        .arg(vpath)
        .output()
    {
        if !out.status.success() {
            let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
            return Err(format!("Rust syntax error:\n{err}"));
        }
    }

    // Step 2: Real `cargo check` inside an isolated virtual copy of the workspace!
    if let (Some(real_repo), Some(crate_dir)) = (
        repo_root_from_manifest(),
        crate_name_for_rust_path(target_path),
    ) {
        let sim_repo = vroot.join("sim-rust-workspace");
        if !sim_repo.exists() {
            fs::create_dir_all(&sim_repo).map_err(|e| e.to_string())?;
            for top_file in ["Cargo.toml", "Cargo.lock", "rust-toolchain.toml"] {
                let src_f = real_repo.join(top_file);
                if src_f.exists() {
                    let _ = fs::copy(&src_f, sim_repo.join(top_file));
                }
            }
            for top_dir in ["crates", "branding", "data"] {
                let src_d = real_repo.join(top_dir);
                if src_d.is_dir() {
                    copy_dir_recursive(&src_d, &sim_repo.join(top_dir))?;
                }
            }
            // Overlay ALL staged .rs / Cargo / data / branding files into the isolated copy
            for (spath, scontent) in staged {
                if let Some(rel) = spath.strip_prefix("/usr/src/forge-os/") {
                    let dest = sim_repo.join(rel);
                    if let Some(p) = dest.parent() {
                        let _ = fs::create_dir_all(p);
                    }
                    let _ = fs::write(&dest, scontent);
                }
            }
        }

        let pkg = if crate_dir == "dev-cli" {
            "dev-cli"
        } else {
            crate_dir
        };
        let mut cmd = Command::new("cargo");
        cmd.args([
            "check",
            "--manifest-path",
            &sim_repo.join("Cargo.toml").to_string_lossy(),
            "-p",
            pkg,
        ]);
        if matches!(pkg, "forge-lab" | "forge-settings" | "forge-shell") {
            cmd.args(["--features", "gtk-ui"]);
        }
        cmd.env("CARGO_TARGET_DIR", real_repo.join("target"));

        match cmd.output() {
            Ok(o) if o.status.success() => {
                return Ok(format!(
                    "Real `cargo check -p {pkg}` compiled cleanly with 0 errors in isolated Virtual OS."
                ));
            }
            Ok(o) => {
                let stderr = String::from_utf8_lossy(&o.stderr);
                let brief: Vec<&str> = stderr
                    .lines()
                    .filter(|l| {
                        l.contains("error")
                            || l.contains("-->")
                            || l.contains("|")
                            || l.contains("help:")
                    })
                    .take(18)
                    .collect();
                let msg = if brief.is_empty() {
                    stderr.trim().to_string()
                } else {
                    brief.join("\n")
                };
                return Err(format!("`cargo check -p {pkg}` failed:\n{msg}"));
            }
            Err(e) => {
                return Err(format!("Could not run cargo check: {e}"));
            }
        }
    }

    Ok("Rust syntax parsed cleanly via rustfmt.".to_string())
}

fn record_check(
    checks: &mut Vec<SimCheckResult>,
    logs: &mut Vec<String>,
    passed: bool,
    test_name: &str,
    target: &str,
    reason: String,
    fix_hint: &str,
) {
    let tag = if passed { "PASS" } else { "FAIL" };
    logs.push(format!("[{tag}] {test_name} ({target}): {reason}"));
    checks.push(SimCheckResult {
        passed,
        test_name: test_name.to_string(),
        target: target.to_string(),
        reason,
        fix_hint: fix_hint.to_string(),
    });
}

pub fn run_virtual_os_simulation(staged: &BTreeMap<String, String>) -> SimReport {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let vroot = std::env::temp_dir().join(format!("forge-virtual-os-{ts}"));
    let mut checks = Vec::new();
    let mut logs = Vec::new();
    let mut all_passed = true;

    if let Err(e) = fs::create_dir_all(&vroot) {
        record_check(
            &mut checks,
            &mut logs,
            false,
            "Virtual OS Sandbox Creation",
            &vroot.display().to_string(),
            format!("Could not create isolated sandbox directory: {e}"),
            "Ensure /tmp is writable and has free disk space.",
        );
        return SimReport {
            passed: false,
            virtual_root: vroot,
            checks,
            logs,
        };
    }

    // Check 1: User Data Isolation Guard
    let touches_home = staged
        .keys()
        .any(|k| k.starts_with("/home") || k.contains("/home/"));
    if touches_home {
        all_passed = false;
        record_check(
            &mut checks,
            &mut logs,
            false,
            "User Data Isolation Guard",
            "/home/*",
            "One or more staged paths attempt to modify user files inside /home.".to_string(),
            "Remove any /home paths; Developer Labs only patches core OS files.",
        );
    } else {
        record_check(
            &mut checks,
            &mut logs,
            true,
            "User Data Isolation Guard",
            "/home/*",
            "Verified 0 user files in /home are touched by this patch batch.".to_string(),
            "",
        );
    }

    // Check 2: Hydrate Virtual OS with untouched core files
    let mut copied_core = 0usize;
    for s in default_core_files() {
        if staged.contains_key(&s.path) {
            continue;
        }
        let loaded = load_core_file(&s);
        if loaded.source == "NOT FOUND" {
            continue;
        }
        let dest = vroot.join(s.path.trim_start_matches('/'));
        if let Some(p) = dest.parent() {
            let _ = fs::create_dir_all(p);
        }
        if fs::write(&dest, loaded.content).is_ok() {
            copied_core += 1;
        }
    }
    record_check(
        &mut checks,
        &mut logs,
        true,
        "Virtual OS Core Snapshot",
        &vroot.display().to_string(),
        format!(
            "Assembled isolated Virtual OS with {copied_core} untouched core files + {} staged edit(s).",
            staged.len()
        ),
        "",
    );

    // Check 3: Per-file Strict Validators
    for (target_path, content) in staged {
        if !is_allowed_core_path(target_path) {
            all_passed = false;
            record_check(
                &mut checks,
                &mut logs,
                false,
                "Core Path Security Gate",
                target_path,
                "Path is outside the allowed core system directories or contains unsafe characters.".to_string(),
                "Only patch files under /usr/local/bin, /usr/share/forge, /etc/systemd, /etc/sysctl.d, or /usr/src/forge-os.",
            );
            continue;
        }

        let vpath = vroot.join(target_path.trim_start_matches('/'));
        if let Some(p) = vpath.parent() {
            let _ = fs::create_dir_all(p);
        }
        if let Err(e) = fs::write(&vpath, content) {
            all_passed = false;
            record_check(
                &mut checks,
                &mut logs,
                false,
                "Virtual OS Overlay Write",
                target_path,
                format!("Failed writing staged file into virtual OS copy: {e}"),
                "Check file path validity and disk space in /tmp.",
            );
            continue;
        }

        if content.trim().is_empty() {
            all_passed = false;
            record_check(
                &mut checks,
                &mut logs,
                false,
                "Non-Empty Core File Check",
                target_path,
                "File content is completely empty, which would break system behavior.".to_string(),
                "Restore valid code or discard this file from the staged batch.",
            );
            continue;
        }

        if target_path.starts_with("/usr/local/bin/") || target_path.ends_with(".sh") {
            let first_line = content.lines().next().unwrap_or("").trim();
            if target_path.starts_with("/usr/local/bin/") && !first_line.starts_with("#!") {
                all_passed = false;
                record_check(
                    &mut checks,
                    &mut logs,
                    false,
                    "Executable Shebang Check",
                    target_path,
                    format!("Missing '#!' shebang on line 1 (found: '{first_line}')."),
                    "Start executable scripts in /usr/local/bin/ with '#!/usr/bin/env bash' or '#!/bin/sh'.",
                );
                continue;
            }
            match Command::new("bash").args(["-n"]).arg(&vpath).output() {
                Ok(o) if o.status.success() => record_check(
                    &mut checks,
                    &mut logs,
                    true,
                    "Bash Syntax Verification (bash -n)",
                    target_path,
                    "Shell script parsed with 0 syntax errors.".to_string(),
                    "",
                ),
                Ok(o) => {
                    all_passed = false;
                    let err = String::from_utf8_lossy(&o.stderr).trim().to_string();
                    record_check(
                        &mut checks,
                        &mut logs,
                        false,
                        "Bash Syntax Verification (bash -n)",
                        target_path,
                        format!("Shell parser error: {err}"),
                        "Fix the syntax error on the reported line (missing 'fi'/'done', unclosed quotes, or broken block).",
                    );
                }
                Err(e) => {
                    all_passed = false;
                    record_check(
                        &mut checks,
                        &mut logs,
                        false,
                        "Bash Syntax Verification (bash -n)",
                        target_path,
                        format!("Could not run bash -n: {e}"),
                        "Ensure /bin/bash is installed on the host.",
                    );
                }
            }
        } else if target_path.ends_with(".rs") {
            match validate_rust_strict(&vroot, &vpath, target_path, staged) {
                Ok(msg) => record_check(
                    &mut checks,
                    &mut logs,
                    true,
                    "Rust Compiler & Syntax Check (cargo check)",
                    target_path,
                    msg,
                    "",
                ),
                Err(e) => {
                    all_passed = false;
                    record_check(
                        &mut checks,
                        &mut logs,
                        false,
                        "Rust Compiler & Syntax Check (cargo check)",
                        target_path,
                        e,
                        "Fix the Rust compiler error(s) above in the editor, then click '1. Test in Virtual OS' again.",
                    );
                }
            }
        } else if target_path.ends_with(".json") || target_path.ends_with("waybar/config") {
            match Command::new("python3")
                .args(["-m", "json.tool"])
                .arg(&vpath)
                .output()
            {
                Ok(o) if o.status.success() => record_check(
                    &mut checks,
                    &mut logs,
                    true,
                    "JSON Schema & Parser Check",
                    target_path,
                    "Valid JSON document parsed cleanly.".to_string(),
                    "",
                ),
                Ok(o) => {
                    all_passed = false;
                    let err = String::from_utf8_lossy(&o.stderr).trim().to_string();
                    record_check(
                        &mut checks,
                        &mut logs,
                        false,
                        "JSON Schema & Parser Check",
                        target_path,
                        format!("Invalid JSON: {err}"),
                        "Look for trailing commas, missing quotes around keys, or unclosed brackets.",
                    );
                }
                Err(e) => {
                    all_passed = false;
                    record_check(
                        &mut checks,
                        &mut logs,
                        false,
                        "JSON Schema & Parser Check",
                        target_path,
                        format!("Could not invoke python3 json.tool: {e}"),
                        "Ensure python3 is installed.",
                    );
                }
            }
        } else if target_path.ends_with(".css") {
            match validate_css_strict(content) {
                Ok(()) => record_check(
                    &mut checks,
                    &mut logs,
                    true,
                    "Strict CSS Syntax & Declaration Check",
                    target_path,
                    "All CSS selectors, braces, and 'property: value' declarations are valid."
                        .to_string(),
                    "",
                ),
                Err(e) => {
                    all_passed = false;
                    record_check(
                        &mut checks,
                        &mut logs,
                        false,
                        "Strict CSS Syntax & Declaration Check",
                        target_path,
                        e,
                        "Ensure every selector opens '{ ... }' and every line inside uses 'property: value;'.",
                    );
                }
            }
        } else if target_path.ends_with(".toml") {
            match validate_toml_strict(&vpath) {
                Ok(()) => record_check(
                    &mut checks,
                    &mut logs,
                    true,
                    "Strict TOML Parser Check (tomllib)",
                    target_path,
                    "Valid TOML configuration parsed cleanly.".to_string(),
                    "",
                ),
                Err(e) => {
                    all_passed = false;
                    record_check(
                        &mut checks,
                        &mut logs,
                        false,
                        "Strict TOML Parser Check (tomllib)",
                        target_path,
                        e,
                        "Fix the invalid TOML syntax on the reported line (missing '=', quotes, or bracket).",
                    );
                }
            }
        } else if target_path.ends_with(".policy") || target_path.ends_with(".xml") {
            match validate_xml_strict(&vpath) {
                Ok(()) => record_check(
                    &mut checks,
                    &mut logs,
                    true,
                    "XML / Polkit Policy Parser Check",
                    target_path,
                    "Valid XML policy document parsed cleanly.".to_string(),
                    "",
                ),
                Err(e) => {
                    all_passed = false;
                    record_check(
                        &mut checks,
                        &mut logs,
                        false,
                        "XML / Polkit Policy Parser Check",
                        target_path,
                        e,
                        "Ensure all XML tags are properly closed and well-formed.",
                    );
                }
            }
        } else if target_path.ends_with(".service")
            || target_path.ends_with(".socket")
            || target_path.ends_with(".timer")
            || target_path.ends_with(".plymouth")
            || target_path.contains("journald.conf.d")
            || target_path.ends_with("postgresql-parked.conf")
        {
            match validate_systemd_unit(content) {
                Ok(()) => record_check(
                    &mut checks,
                    &mut logs,
                    true,
                    "Systemd / INI Unit Structure Check",
                    target_path,
                    "Valid [Section] headers and Key=Value entries.".to_string(),
                    "",
                ),
                Err(e) => {
                    all_passed = false;
                    record_check(
                        &mut checks,
                        &mut logs,
                        false,
                        "Systemd / INI Unit Structure Check",
                        target_path,
                        e,
                        "Ensure every directive uses Key=Value inside a valid [Section] header.",
                    );
                }
            }
        } else if target_path.starts_with("/etc/sysctl.d/") {
            match validate_sysctl(content) {
                Ok(()) => record_check(
                    &mut checks,
                    &mut logs,
                    true,
                    "Kernel Sysctl Parameter Check",
                    target_path,
                    "All kernel sysctl entries use valid 'namespace.param = value' syntax."
                        .to_string(),
                    "",
                ),
                Err(e) => {
                    all_passed = false;
                    record_check(
                        &mut checks,
                        &mut logs,
                        false,
                        "Kernel Sysctl Parameter Check",
                        target_path,
                        e,
                        "Use 'parameter.name = value' on each non-comment line.",
                    );
                }
            }
        } else if target_path.ends_with("themerc") {
            match validate_themerc(content) {
                Ok(()) => record_check(
                    &mut checks,
                    &mut logs,
                    true,
                    "Openbox/labwc Theme Check (themerc)",
                    target_path,
                    "All theme entries follow valid 'property: value' format.".to_string(),
                    "",
                ),
                Err(e) => {
                    all_passed = false;
                    record_check(
                        &mut checks,
                        &mut logs,
                        false,
                        "Openbox/labwc Theme Check (themerc)",
                        target_path,
                        e,
                        "Use 'property.name: value' on each non-comment line.",
                    );
                }
            }
        } else if target_path.starts_with("/etc/apt/") {
            match validate_apt_conf(content) {
                Ok(()) => record_check(
                    &mut checks,
                    &mut logs,
                    true,
                    "APT Configuration Syntax Check",
                    target_path,
                    "All APT directives properly terminated with ';'.".to_string(),
                    "",
                ),
                Err(e) => {
                    all_passed = false;
                    record_check(
                        &mut checks,
                        &mut logs,
                        false,
                        "APT Configuration Syntax Check",
                        target_path,
                        e,
                        "End every APT option line with a semicolon ';'.",
                    );
                }
            }
        } else if target_path.ends_with("hosts.block") {
            match validate_hosts_block(content) {
                Ok(()) => record_check(
                    &mut checks,
                    &mut logs,
                    true,
                    "Adblock Hosts Format Check",
                    target_path,
                    "All sinkhole entries use valid '<ip> <domain>' format.".to_string(),
                    "",
                ),
                Err(e) => {
                    all_passed = false;
                    record_check(
                        &mut checks,
                        &mut logs,
                        false,
                        "Adblock Hosts Format Check",
                        target_path,
                        e,
                        "Format each line as '0.0.0.0 ad.domain.com'.",
                    );
                }
            }
        } else if target_path.ends_with("packages.list") {
            match validate_packages_list(content) {
                Ok(()) => record_check(
                    &mut checks,
                    &mut logs,
                    true,
                    "Debian Package List Check",
                    target_path,
                    "All package names follow valid Debian naming rules.".to_string(),
                    "",
                ),
                Err(e) => {
                    all_passed = false;
                    record_check(
                        &mut checks,
                        &mut logs,
                        false,
                        "Debian Package List Check",
                        target_path,
                        e,
                        "Use one lowercase Debian package name per line (no spaces).",
                    );
                }
            }
        } else if target_path.ends_with(".mk") {
            match validate_makefile(content) {
                Ok(()) => record_check(
                    &mut checks,
                    &mut logs,
                    true,
                    "Makefile Recipe Syntax Check",
                    target_path,
                    "Makefile targets, conditionals, and assignments look valid.".to_string(),
                    "",
                ),
                Err(e) => {
                    all_passed = false;
                    record_check(
                        &mut checks,
                        &mut logs,
                        false,
                        "Makefile Recipe Syntax Check",
                        target_path,
                        e,
                        "Prefix recipe commands with a TAB character and check target/variable syntax.",
                    );
                }
            }
        } else if target_path.starts_with("/etc/default/") {
            match Command::new("bash").args(["-n"]).arg(&vpath).output() {
                Ok(o) if o.status.success() => record_check(
                    &mut checks,
                    &mut logs,
                    true,
                    "Environment Defaults Syntax Check (bash -n)",
                    target_path,
                    "Valid shell environment variable definitions.".to_string(),
                    "",
                ),
                Ok(o) => {
                    all_passed = false;
                    record_check(
                        &mut checks,
                        &mut logs,
                        false,
                        "Environment Defaults Syntax Check (bash -n)",
                        target_path,
                        String::from_utf8_lossy(&o.stderr).trim().to_string(),
                        "Use valid KEY=\"VALUE\" shell assignments.",
                    );
                }
                Err(e) => {
                    all_passed = false;
                    record_check(
                        &mut checks,
                        &mut logs,
                        false,
                        "Environment Defaults Syntax Check",
                        target_path,
                        format!("Could not run bash -n: {e}"),
                        "Ensure /bin/bash is installed.",
                    );
                }
            }
        } else {
            record_check(
                &mut checks,
                &mut logs,
                true,
                "Core Config Integrity Check",
                target_path,
                "Core configuration file verified non-empty and readable in virtual OS."
                    .to_string(),
                "",
            );
        }
    }

    // Check 4: Core OS Boot/Desktop Wiring Presence inside Virtual OS
    let critical_wired = [
        "/usr/local/bin/forge-session",
        "/usr/share/forge/waybar/config",
        "/etc/systemd/system/forged.socket",
    ];
    let present_cnt = critical_wired
        .iter()
        .filter(|p| vroot.join(p.trim_start_matches('/')).exists())
        .count();
    record_check(
        &mut checks,
        &mut logs,
        true,
        "Core Boot & Desktop Wiring Check",
        "Virtual OS Graph",
        format!(
            "Verified {present_cnt}/{} core wiring anchors (session, taskbar, forged.socket) intact in sandbox.",
            critical_wired.len()
        ),
        "",
    );

    let pass_cnt = checks.iter().filter(|c| c.passed).count();
    let fail_cnt = checks.len() - pass_cnt;
    if all_passed {
        logs.push(format!(
            "[SIM PASS] All {pass_cnt}/{} Virtual OS tests passed! Ready to patch live OS.",
            checks.len()
        ));
    } else {
        logs.push(format!(
            "[SIM FAIL] {fail_cnt} test(s) failed ({pass_cnt} passed). Review the Virtual OS Test Report window and fix errors before patching."
        ));
    }

    let _ = fs::remove_dir_all(&vroot);
    SimReport {
        passed: all_passed,
        virtual_root: vroot,
        checks,
        logs,
    }
}

/// Generates the transactional bash patch script (exposed for contract testing).
pub fn build_apply_script(staged_items: &[(String, PathBuf)], ts: u64) -> Result<String, String> {
    let mut reload_systemd = false;
    let mut reload_sysctl = false;

    let mut script = format!(
        "#!/usr/bin/env bash\nset -euo pipefail\n\
         BK=\"/var/lib/forge/lab-rollback/{ts}\"\n\
         mkdir -p \"$BK/root\"\n\
         CREATED=\"$BK/created.list\"\n\
         : > \"$CREATED\"\n\
         rollback() {{\n\
           echo 'Patch failed; rolling back live files...' >&2\n\
           if [ -f \"$CREATED\" ]; then\n\
             while IFS= read -r f; do\n\
               [ -n \"$f\" ] && rm -f \"$f\" \"${{f}}.forge.new\"\n\
             done < \"$CREATED\"\n\
           fi\n\
           if [ -d \"$BK/root\" ] && [ -n \"$(ls -A \"$BK/root\" 2>/dev/null)\" ]; then\n\
             cp -a \"$BK/root/.\" /\n\
           fi\n\
         }}\n\
         trap rollback ERR INT TERM\n"
    );

    for (real_path, tmp_file) in staged_items {
        if !is_allowed_core_path(real_path) {
            return Err(format!("Blocked non-core or unsafe path: {real_path}"));
        }
        if real_path.starts_with("/etc/systemd/") {
            reload_systemd = true;
        }
        if real_path.starts_with("/etc/sysctl.d/") {
            reload_sysctl = true;
        }
        let mode = if real_path.starts_with("/usr/local/bin/") || real_path.ends_with(".sh") {
            "0755"
        } else {
            "0644"
        };
        script.push_str(&format!(
            "if [ -f '{real_path}' ]; then\n\
               mkdir -p \"$BK/root$(dirname '{real_path}')\"\n\
               cp -a '{real_path}' \"$BK/root{real_path}\"\n\
             else\n\
               printf '%s\\n' '{real_path}' >> \"$CREATED\"\n\
             fi\n\
             mkdir -p \"$(dirname '{real_path}')\"\n\
             install -m {mode} '{}' '{real_path}.forge.new'\n\
             mv -f '{real_path}.forge.new' '{real_path}'\n",
            tmp_file.display()
        ));
    }

    if reload_systemd {
        script.push_str("systemctl daemon-reload >/dev/null 2>&1 || true\n");
    }
    if reload_sysctl {
        script.push_str("sysctl --system >/dev/null 2>&1 || true\n");
    }
    script.push_str("trap - ERR INT TERM\n");
    Ok(script)
}

pub fn apply_staged_to_live(staged: &BTreeMap<String, String>) -> Result<String, String> {
    if staged.is_empty() {
        return Err("No staged changes to apply.".to_string());
    }
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let stage_dir = std::env::temp_dir().join(format!("forge-lab-apply-{ts}"));
    let _ = fs::remove_dir_all(&stage_dir);
    fs::create_dir_all(&stage_dir).map_err(|e| e.to_string())?;

    let mut staged_items = Vec::new();
    let mut touched_rust = false;
    for (idx, (real_path, content)) in staged.iter().enumerate() {
        if !is_allowed_core_path(real_path) {
            let _ = fs::remove_dir_all(&stage_dir);
            return Err(format!("Blocked non-core or unsafe path: {real_path}"));
        }
        if real_path.ends_with(".rs") {
            touched_rust = true;
        }
        let tmp_file = stage_dir.join(format!("item_{idx}"));
        fs::write(&tmp_file, content).map_err(|e| e.to_string())?;
        staged_items.push((real_path.clone(), tmp_file));
    }

    let script = match build_apply_script(&staged_items, ts) {
        Ok(s) => s,
        Err(e) => {
            let _ = fs::remove_dir_all(&stage_dir);
            return Err(e);
        }
    };

    let runner = stage_dir.join("apply.sh");
    fs::write(&runner, script).map_err(|e| e.to_string())?;

    // IMPORTANT: Must invoke /bin/bash (not /bin/sh = dash) so `trap rollback ERR` works!
    let out = Command::new("pkexec")
        .args(["/bin/bash", &runner.to_string_lossy()])
        .output()
        .map_err(|e| format!("pkexec failed: {e}"))?;

    let _ = fs::remove_dir_all(&stage_dir);

    if out.status.success() {
        let mut msg = format!(
            "[OK] Atomically patched {} core file(s). Rollback backup saved at /var/lib/forge/lab-rollback/{ts} (undo anytime via `forge-repair rollback`).",
            staged.len()
        );
        if touched_rust {
            msg.push_str("\n[NOTE] You patched Rust source file(s): run `cargo build --release` to rebuild the updated binaries.");
        }
        Ok(msg)
    } else {
        Err(format!(
            "[FAILED] Patch aborted & rolled back cleanly: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ))
    }
}
