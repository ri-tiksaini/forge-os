#![forbid(unsafe_code)]

//! Installed-state discovery from dpkg & systemd-socket-proxyd parking (Blueprint 5.5 & 5.6).

use std::collections::HashSet;
use std::fs;
use std::path::Path;
use std::process::Command;

use forge_catalog::{Backend, CatalogIndex};

/// Rebuilds the installed catalog ID set from dpkg on daemon startup (Blueprint 5.5).
pub fn detect_installed_apt(catalog: &CatalogIndex) -> HashSet<String> {
    let mut installed_pkgs = HashSet::new();
    if let Ok(out) = Command::new("dpkg-query")
        .args(["-W", "-f=${Status}\t${Package}\n"])
        .output()
    {
        if out.status.success() {
            let text = String::from_utf8_lossy(&out.stdout);
            for line in text.lines() {
                if let Some((status, pkg)) = line.split_once('\t') {
                    if status.contains("install ok installed") {
                        installed_pkgs.insert(pkg.trim().to_string());
                    }
                }
            }
        }
    }

    let mut installed_ids = HashSet::new();
    for entry in catalog.entries() {
        if entry.backend == Backend::Apt
            && !entry.packages.is_empty()
            && entry.packages.iter().all(|p| installed_pkgs.contains(p))
        {
            installed_ids.insert(entry.id.clone());
        }
    }
    installed_ids
}

/// Parks PostgreSQL behind `systemd-socket-proxyd` on 127.0.0.1:5432 -> 127.0.0.1:5433 (Blueprint 5.6).
pub fn park_postgres() -> Result<(), String> {
    // 1. Move PostgreSQL cluster to internal port 5433
    if let Ok(entries) = fs::read_dir("/etc/postgresql") {
        for ver in entries.flatten() {
            let conf = ver.path().join("main/postgresql.conf");
            if let Ok(text) = fs::read_to_string(&conf) {
                let updated = if text.contains("port = 5432") {
                    text.replace("port = 5432", "port = 5433")
                } else if !text.contains("port = 5433") {
                    format!("{text}\nport = 5433\n")
                } else {
                    text
                };
                fs::write(&conf, updated).map_err(|e| format!("write {}: {e}", conf.display()))?;
            }
        }
    }

    // 2. Install systemd drop-in & proxy units from /usr/share/forge/proxies
    let proxy_src = Path::new("/usr/share/forge/proxies");
    let dropin_dir = Path::new("/etc/systemd/system/postgresql.service.d");
    fs::create_dir_all(dropin_dir).map_err(|e| e.to_string())?;
    fs::copy(
        proxy_src.join("postgresql-parked.conf"),
        dropin_dir.join("forge-parked.conf"),
    )
    .map_err(|e| format!("copy drop-in: {e}"))?;
    fs::copy(
        proxy_src.join("forge-postgres-proxy.socket"),
        "/etc/systemd/system/forge-postgres-proxy.socket",
    )
    .map_err(|e| format!("copy proxy socket: {e}"))?;
    fs::copy(
        proxy_src.join("forge-postgres-proxy.service"),
        "/etc/systemd/system/forge-postgres-proxy.service",
    )
    .map_err(|e| format!("copy proxy service: {e}"))?;

    // 3. Reload systemd, disable & stop postgresql (0 MiB idle), enable proxy socket
    run_systemctl(&["daemon-reload"])?;
    let _ = run_systemctl(&["disable", "--now", "postgresql.service"]);
    run_systemctl(&["enable", "--now", "forge-postgres-proxy.socket"])?;
    Ok(())
}

fn run_systemctl(args: &[&str]) -> Result<(), String> {
    let out = Command::new("systemctl")
        .args(args)
        .output()
        .map_err(|e| format!("systemctl {args:?}: {e}"))?;
    if out.status.success() {
        Ok(())
    } else {
        Err(format!(
            "systemctl {args:?} failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ))
    }
}

/// Parks Redis behind `systemd-socket-proxyd` on 127.0.0.1:6379 -> 127.0.0.1:6380.
pub fn park_redis() -> Result<(), String> {
    let conf = Path::new("/etc/redis/redis.conf");
    if let Ok(text) = fs::read_to_string(conf) {
        let updated = text.replace("\nport 6379", "\nport 6380");
        fs::write(conf, updated).map_err(|e| format!("write redis.conf: {e}"))?;
    }
    install_proxy_pair("redis-server.service", "forge-redis-proxy")
}

/// Parks MariaDB behind `systemd-socket-proxyd` on 127.0.0.1:3306 -> 127.0.0.1:3307.
pub fn park_mariadb() -> Result<(), String> {
    let conf_dir = Path::new("/etc/mysql/mariadb.conf.d");
    if conf_dir.exists() {
        fs::write(
            conf_dir.join("99-forge-parked.cnf"),
            "[mysqld]\nport = 3307\n",
        )
        .map_err(|e| format!("write mariadb drop-in: {e}"))?;
    }
    install_proxy_pair("mariadb.service", "forge-mariadb-proxy")
}

fn install_proxy_pair(upstream_svc: &str, proxy_name: &str) -> Result<(), String> {
    let proxy_src = Path::new("/usr/share/forge/proxies");
    let dropin_dir = Path::new("/etc/systemd/system").join(format!("{upstream_svc}.d"));
    fs::create_dir_all(&dropin_dir).map_err(|e| e.to_string())?;
    fs::write(
        dropin_dir.join("forge-parked.conf"),
        "[Unit]\nStopWhenUnneeded=yes\n",
    )
    .map_err(|e| e.to_string())?;

    for ext in ["socket", "service"] {
        let fname = format!("{proxy_name}.{ext}");
        fs::copy(
            proxy_src.join(&fname),
            Path::new("/etc/systemd/system").join(&fname),
        )
        .map_err(|e| format!("copy {fname}: {e}"))?;
    }

    run_systemctl(&["daemon-reload"])?;
    let _ = run_systemctl(&["disable", "--now", upstream_svc]);
    let sock = format!("{proxy_name}.socket");
    run_systemctl(&["enable", "--now", &sock])?;
    Ok(())
}
