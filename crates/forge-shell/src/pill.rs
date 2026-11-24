#![forbid(unsafe_code)]

//! Live system telemetry sampler for Taskbar & Control Center (ADR 0002).
//! Samples CPU %, RAM %, GPU driver, Storage %, Network RX/TX speed, Battery %, Sound %.

use std::fs;
use std::process::Command;

#[derive(Debug, Clone, Default)]
pub struct CpuPrev {
    pub total: u64,
    pub idle: u64,
}

#[derive(Debug, Clone, Default)]
pub struct NetPrev {
    pub rx_bytes: u64,
    pub tx_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PillSnapshot {
    pub cpu_percent: u8,
    pub ram_percent: u8,
    pub ram_used_mib: u64,
    pub ram_total_mib: u64,
    pub gpu_label: String,
    pub disk_percent: u8,
    pub net_rx_kbps: u64,
    pub net_tx_kbps: u64,
    pub battery_percent: Option<u8>,
    pub volume_percent: Option<u8>,
    pub tier_label: String,
    pub clock_hhmm: String,
}

impl PillSnapshot {
    pub fn sample(tier_label: &str) -> Self {
        let mut cpu_prev = CpuPrev::default();
        let mut net_prev = NetPrev::default();
        Self::sample_live(tier_label, &mut cpu_prev, &mut net_prev)
    }

    pub fn sample_live(tier_label: &str, cpu_prev: &mut CpuPrev, net_prev: &mut NetPrev) -> Self {
        let cpu_percent = read_cpu_percent(cpu_prev);
        let (ram_used_mib, ram_total_mib, ram_percent) = read_ram_usage();
        let (net_rx_kbps, net_tx_kbps) = read_net_kbps(net_prev);
        let gpu_label = read_gpu_label();
        let disk_percent = read_disk_percent();
        let battery_percent = read_battery_percent();
        let volume_percent = read_volume_percent();
        let clock_hhmm = read_clock();

        Self {
            cpu_percent,
            ram_percent,
            ram_used_mib,
            ram_total_mib,
            gpu_label,
            disk_percent,
            net_rx_kbps,
            net_tx_kbps,
            battery_percent,
            volume_percent,
            tier_label: tier_label.to_string(),
            clock_hhmm,
        }
    }

    pub fn format_compact(&self) -> String {
        let bat = self
            .battery_percent
            .map(|b| format!("{b}%"))
            .unwrap_or_else(|| "AC".to_string());
        format!(
            "CPU {}%  |  RAM {}%  |  NET {}K  |  {}  |  {}",
            self.cpu_percent, self.ram_percent, self.net_rx_kbps, bat, self.clock_hhmm
        )
    }

    pub fn format_detailed(&self) -> String {
        let vol = self
            .volume_percent
            .map(|v| format!("{v}%"))
            .unwrap_or_else(|| "--".to_string());
        format!(
            "GPU: {}  |  Disk: {}%  |  Net: {}K/{}K  |  Sound: {}",
            self.gpu_label, self.disk_percent, self.net_rx_kbps, self.net_tx_kbps, vol
        )
    }
}

fn read_cpu_percent(prev: &mut CpuPrev) -> u8 {
    let Ok(text) = fs::read_to_string("/proc/stat") else {
        return 0;
    };
    let Some(line) = text.lines().next() else {
        return 0;
    };
    let nums: Vec<u64> = line
        .split_whitespace()
        .skip(1)
        .filter_map(|s| s.parse::<u64>().ok())
        .collect();
    if nums.len() < 4 {
        return 0;
    }
    let idle = nums[3] + nums.get(4).copied().unwrap_or(0);
    let total: u64 = nums.iter().sum();

    let d_total = total.saturating_sub(prev.total);
    let d_idle = idle.saturating_sub(prev.idle);
    prev.total = total;
    prev.idle = idle;

    if d_total == 0 {
        return 0;
    }
    let busy = d_total.saturating_sub(d_idle);
    ((busy * 100) / d_total).min(100) as u8
}

fn read_ram_usage() -> (u64, u64, u8) {
    let Ok(text) = fs::read_to_string("/proc/meminfo") else {
        return (0, 0, 0);
    };
    let mut total_kb: u64 = 0;
    let mut avail_kb: u64 = 0;
    for line in text.lines() {
        if line.starts_with("MemTotal:") {
            total_kb = line
                .split_whitespace()
                .nth(1)
                .and_then(|n| n.parse().ok())
                .unwrap_or(0);
        } else if line.starts_with("MemAvailable:") {
            avail_kb = line
                .split_whitespace()
                .nth(1)
                .and_then(|n| n.parse().ok())
                .unwrap_or(0);
        }
    }
    if total_kb == 0 {
        return (0, 0, 0);
    }
    let used_kb = total_kb.saturating_sub(avail_kb);
    let pct = ((used_kb * 100) / total_kb).min(100) as u8;
    (used_kb / 1024, total_kb / 1024, pct)
}

fn read_net_kbps(prev: &mut NetPrev) -> (u64, u64) {
    let Ok(text) = fs::read_to_string("/proc/net/dev") else {
        return (0, 0);
    };
    let mut rx_sum: u64 = 0;
    let mut tx_sum: u64 = 0;
    for line in text.lines().skip(2) {
        let Some((iface, rest)) = line.split_once(':') else {
            continue;
        };
        if iface.trim() == "lo" {
            continue;
        }
        let cols: Vec<u64> = rest
            .split_whitespace()
            .filter_map(|s| s.parse::<u64>().ok())
            .collect();
        if cols.len() >= 9 {
            rx_sum = rx_sum.saturating_add(cols[0]);
            tx_sum = tx_sum.saturating_add(cols[8]);
        }
    }
    let rx_kbps = if prev.rx_bytes == 0 {
        0
    } else {
        rx_sum.saturating_sub(prev.rx_bytes) / 1024
    };
    let tx_kbps = if prev.tx_bytes == 0 {
        0
    } else {
        tx_sum.saturating_sub(prev.tx_bytes) / 1024
    };
    prev.rx_bytes = rx_sum;
    prev.tx_bytes = tx_sum;
    (rx_kbps, tx_kbps)
}

fn read_gpu_label() -> String {
    if let Ok(entries) = fs::read_dir("/sys/class/drm") {
        for e in entries.flatten() {
            let name = e.file_name().to_string_lossy().to_string();
            if name.starts_with("card") && !name.contains('-') {
                if let Ok(link) = fs::read_link(e.path().join("device/driver")) {
                    if let Some(drv) = link.file_name() {
                        return drv.to_string_lossy().to_string();
                    }
                }
            }
        }
    }
    "sw".to_string()
}

fn read_disk_percent() -> u8 {
    if let Ok(out) = Command::new("df").args(["-P", "/"]).output() {
        if out.status.success() {
            let txt = String::from_utf8_lossy(&out.stdout);
            if let Some(line) = txt.lines().nth(1) {
                for tok in line.split_whitespace() {
                    if let Some(pct) = tok.strip_suffix('%') {
                        if let Ok(n) = pct.parse::<u8>() {
                            return n;
                        }
                    }
                }
            }
        }
    }
    0
}

fn read_battery_percent() -> Option<u8> {
    let Ok(entries) = fs::read_dir("/sys/class/power_supply") else {
        return None;
    };
    for e in entries.flatten() {
        let name = e.file_name().to_string_lossy().to_string();
        if name.starts_with("BAT") {
            if let Ok(cap) = fs::read_to_string(e.path().join("capacity")) {
                if let Ok(n) = cap.trim().parse::<u8>() {
                    return Some(n.min(100));
                }
            }
        }
    }
    None
}

fn read_volume_percent() -> Option<u8> {
    let out = Command::new("wpctl")
        .args(["get-volume", "@DEFAULT_AUDIO_SINK@"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8_lossy(&out.stdout);
    let val_str = s.split_whitespace().nth(1)?;
    let f: f32 = val_str.parse().ok()?;
    Some((f * 100.0).round().clamp(0.0, 150.0) as u8)
}

fn read_clock() -> String {
    if let Ok(out) = Command::new("date").arg("+%H:%M").output() {
        if out.status.success() {
            return String::from_utf8_lossy(&out.stdout).trim().to_string();
        }
    }
    "00:00".to_string()
}
