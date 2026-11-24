#![forbid(unsafe_code)]

//! Deterministic hardware tier detection (Blueprint 5.7 & 6.3).

use std::env;
use std::fmt;
use std::fs;
use std::path::Path;

const GIB_KB: u64 = 1024 * 1024;
const MIN_T1_RAM_KB: u64 = 3_670_016; // 3.5 GiB in KiB

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Tier {
    Tier0 = 0,
    Tier1 = 1,
    Tier2 = 2,
}

impl Tier {
    pub fn as_u8(self) -> u8 {
        self as u8
    }

    pub fn label(self) -> &'static str {
        match self {
            Tier::Tier0 => "Tier 0 (Legacy)",
            Tier::Tier1 => "Tier 1 (Standard)",
            Tier::Tier2 => "Tier 2 (Modern)",
        }
    }

    pub fn compositor(self) -> &'static str {
        match self {
            Tier::Tier0 => "labwc",
            Tier::Tier1 | Tier::Tier2 => "sway",
        }
    }
}

impl fmt::Display for Tier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reason {
    pub code: &'static str,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Probes {
    pub mem_total_kb: u64,
    pub cpu_model: String,
    pub cpu_cores: u32,
    pub has_sse4_2: bool,
    pub has_avx2: bool,
    pub cpu_year: u16,
    pub disk_rotational: bool,
    pub disk_model: String,
    pub gpu_vendor: String,
    pub gpu_driver: String,
    pub gles_major: u8,
    pub gles_minor: u8,
    pub is_software_rasterizer: bool,
    pub has_drm_driver: bool,
    pub display_count: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TierReport {
    pub tier: Tier,
    pub score: u8,
    pub reasons: Vec<Reason>,
    pub raw: Probes,
    pub override_active: bool,
    pub nomodeset_recommended: bool,
}

impl TierReport {
    pub fn to_json(&self) -> String {
        let reasons_json: Vec<String> = self
            .reasons
            .iter()
            .map(|r| {
                format!(
                    "{{\"code\":\"{}\",\"message\":\"{}\"}}",
                    escape_json(r.code),
                    escape_json(&r.message)
                )
            })
            .collect();
        format!(
            concat!(
                "{{\n",
                "  \"tier\": {},\n",
                "  \"label\": \"{}\",\n",
                "  \"compositor\": \"{}\",\n",
                "  \"score\": {},\n",
                "  \"override_active\": {},\n",
                "  \"nomodeset_recommended\": {},\n",
                "  \"mem_total_mib\": {},\n",
                "  \"cpu_model\": \"{}\",\n",
                "  \"cpu_year\": {},\n",
                "  \"disk_rotational\": {},\n",
                "  \"gpu_driver\": \"{}\",\n",
                "  \"reasons\": [{}]\n",
                "}}"
            ),
            self.tier.as_u8(),
            escape_json(self.tier.label()),
            self.tier.compositor(),
            self.score,
            self.override_active,
            self.nomodeset_recommended,
            self.raw.mem_total_kb / 1024,
            escape_json(&self.raw.cpu_model),
            self.raw.cpu_year,
            self.raw.disk_rotational,
            escape_json(&self.raw.gpu_driver),
            reasons_json.join(", ")
        )
    }
}

fn escape_json(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
}

/// Pure deterministic scoring + hard overrides (Blueprint 5.7).
pub fn detect_from(p: &Probes) -> TierReport {
    let mut reasons = Vec::new();

    // Normalized sub-scores in 0..=100 (integer arithmetic only)
    let f_mem: u32 = match p.mem_total_kb {
        kb if kb < 4 * GIB_KB => 15,
        kb if kb < 6 * GIB_KB => 30,
        kb if kb < 12 * GIB_KB => 55,
        kb if kb < 16 * GIB_KB => 75,
        _ => 100,
    };

    let f_cpu: u32 = match p.cpu_year {
        y if y <= 2012 => 15,
        y if y <= 2016 => 45,
        y if y <= 2020 => 75,
        _ => 100,
    };

    let f_disk: u32 = if p.disk_rotational { 10 } else { 95 };

    let f_gpu: u32 = if !p.has_drm_driver || p.is_software_rasterizer {
        0
    } else if p.gles_major < 3 {
        20
    } else if p.gles_major == 3 && p.gles_minor == 0 {
        50
    } else if p.gles_major == 3 && p.gles_minor == 1 {
        65
    } else {
        100
    };

    // Weights: w_mem=35, w_cpu=25, w_disk=20, w_gpu=20
    let weighted = (35 * f_mem + 25 * f_cpu + 20 * f_disk + 20 * f_gpu) / 100;
    let score = weighted.min(100) as u8;

    let mut tier = if score < 35 {
        Tier::Tier0
    } else if score <= 70 {
        Tier::Tier1
    } else {
        Tier::Tier2
    };

    reasons.push(Reason {
        code: "SCORE",
        message: format!(
            "Hardware score {score}/100 (RAM {} MiB, CPU ~{}, {}, GPU {}).",
            p.mem_total_kb / 1024,
            p.cpu_year,
            if p.disk_rotational { "HDD" } else { "SSD/NVMe" },
            if p.gpu_driver.is_empty() {
                "none"
            } else {
                &p.gpu_driver
            }
        ),
    });

    let mut nomodeset_recommended = false;

    // Hard overrides (applied after scoring; they always win)
    if p.mem_total_kb < MIN_T1_RAM_KB {
        tier = Tier::Tier0;
        reasons.push(Reason {
            code: "OVERRIDE_LOW_RAM",
            message: format!(
                "RAM ({} MiB) is below 3.5 GiB; forced to Tier 0.",
                p.mem_total_kb / 1024
            ),
        });
    }

    if p.disk_rotational && tier == Tier::Tier2 {
        tier = Tier::Tier1;
        reasons.push(Reason {
            code: "OVERRIDE_ROTATIONAL_DISK",
            message: "Rotational HDD detected; capped at Tier 1.".to_string(),
        });
    }

    if p.gles_major < 3 || p.is_software_rasterizer {
        tier = Tier::Tier0;
        reasons.push(Reason {
            code: "OVERRIDE_SW_OR_OLD_GPU",
            message: "Software rasterizer or GLES < 3.0 detected; forced to Tier 0.".to_string(),
        });
    }

    if !p.has_drm_driver {
        tier = Tier::Tier0;
        nomodeset_recommended = true;
        reasons.push(Reason {
            code: "OVERRIDE_NO_DRM",
            message: "No working DRM driver detected; forced to Tier 0 (nomodeset).".to_string(),
        });
    }

    TierReport {
        tier,
        score,
        reasons,
        raw: p.clone(),
        override_active: false,
        nomodeset_recommended,
    }
}

/// Reads live Linux `/proc` and `/sys` probes, applies optional `FORGE_TIER` override,
/// and best-effort caches the report to `/run/forge/tier.json`.
pub fn detect() -> TierReport {
    let probes = read_system_probes();
    let mut report = detect_from(&probes);

    if let Ok(val) = env::var("FORGE_TIER") {
        let parsed = match val.trim() {
            "0" => Some(Tier::Tier0),
            "1" => Some(Tier::Tier1),
            "2" => Some(Tier::Tier2),
            _ => None,
        };
        if let Some(forced) = parsed {
            eprintln!("WARN: FORGE_TIER={val} override active; forcing {forced}");
            report.tier = forced;
            report.override_active = true;
            report.reasons.push(Reason {
                code: "ENV_OVERRIDE",
                message: format!("FORGE_TIER={val} environment override forced {forced}."),
            });
        }
    }

    if Path::new("/run/forge").exists() || fs::create_dir_all("/run/forge").is_ok() {
        let _ = fs::write("/run/forge/tier.json", report.to_json());
    }

    report
}

pub fn estimate_cpu_year(model: &str, has_sse4_2: bool, has_avx2: bool) -> u16 {
    let m = model.to_ascii_lowercase();
    if m.contains("core(tm)2") || m.contains("pentium(r) dual") || m.contains("celeron(r) m") {
        return 2008;
    }
    if m.contains("i5-2") || m.contains("i7-2") || m.contains("i3-2") || m.contains("sandy") {
        return 2011;
    }
    if m.contains("i5-3") || m.contains("i7-3") || m.contains("i3-3") || m.contains("ivy") {
        return 2012;
    }
    if m.contains("i5-4") || m.contains("i7-4") || m.contains("i5-5") || m.contains("i7-5") {
        return 2014;
    }
    if m.contains("i5-8")
        || m.contains("i7-8")
        || m.contains("ryzen 5 2")
        || m.contains("ryzen 7 2")
    {
        return 2018;
    }
    if m.contains("12th gen")
        || m.contains("13th gen")
        || m.contains("14th gen")
        || m.contains("ultra")
        || m.contains("ryzen 5 7")
        || m.contains("ryzen 7 7")
        || m.contains("ryzen 9 7")
        || m.contains("ryzen 7 8")
    {
        return 2023;
    }
    if has_avx2 {
        2019
    } else if has_sse4_2 {
        2012
    } else {
        2008
    }
}

fn read_system_probes() -> Probes {
    let mem_total_kb = fs::read_to_string("/proc/meminfo")
        .ok()
        .and_then(|s| {
            s.lines()
                .find(|l| l.starts_with("MemTotal:"))
                .and_then(|l| l.split_whitespace().nth(1))
                .and_then(|n| n.parse::<u64>().ok())
        })
        .unwrap_or(2 * GIB_KB);

    let cpuinfo = fs::read_to_string("/proc/cpuinfo").unwrap_or_default();
    let cpu_model = cpuinfo
        .lines()
        .find(|l| l.starts_with("model name"))
        .and_then(|l| l.split_once(':'))
        .map(|(_, v)| v.trim().to_string())
        .unwrap_or_else(|| "Unknown CPU".to_string());

    let cpu_cores = cpuinfo
        .lines()
        .filter(|l| l.starts_with("processor"))
        .count()
        .max(1) as u32;

    let flags_line = cpuinfo
        .lines()
        .find(|l| l.starts_with("flags"))
        .unwrap_or_default();
    let has_sse4_2 = flags_line.split_whitespace().any(|f| f == "sse4_2");
    let has_avx2 = flags_line.split_whitespace().any(|f| f == "avx2");
    let cpu_year = estimate_cpu_year(&cpu_model, has_sse4_2, has_avx2);

    let (disk_rotational, disk_model) = probe_primary_disk();
    let (gpu_vendor, gpu_driver, has_drm_driver, display_count) = probe_drm();

    let is_software_rasterizer = !has_drm_driver
        || gpu_driver == "vgem"
        || gpu_driver == "simpledrm"
        || env::var("WLR_RENDERER").is_ok_and(|v| v == "pixman");

    let (gles_major, gles_minor) = if is_software_rasterizer {
        (2, 0)
    } else if gpu_driver == "virtio-pci" || gpu_driver == "virtio_gpu" {
        (3, 0)
    } else {
        (3, 2)
    };

    Probes {
        mem_total_kb,
        cpu_model,
        cpu_cores,
        has_sse4_2,
        has_avx2,
        cpu_year,
        disk_rotational,
        disk_model,
        gpu_vendor,
        gpu_driver,
        gles_major,
        gles_minor,
        is_software_rasterizer,
        has_drm_driver,
        display_count,
    }
}

fn probe_primary_disk() -> (bool, String) {
    let Ok(entries) = fs::read_dir("/sys/block") else {
        return (true, "unknown".to_string());
    };
    let mut found_any = false;
    let mut all_rotational = true;
    let mut model = String::from("unknown");

    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with("loop")
            || name.starts_with("ram")
            || name.starts_with("zram")
            || name.starts_with("sr")
        {
            continue;
        }
        let rot_path = entry.path().join("queue/rotational");
        if let Ok(val) = fs::read_to_string(&rot_path) {
            found_any = true;
            if val.trim() == "0" {
                all_rotational = false;
            }
            if let Ok(m) = fs::read_to_string(entry.path().join("device/model")) {
                if !m.trim().is_empty() {
                    model = m.trim().to_string();
                }
            }
        }
    }

    (if found_any { all_rotational } else { true }, model)
}

fn probe_drm() -> (String, String, bool, u8) {
    let Ok(entries) = fs::read_dir("/sys/class/drm") else {
        return (String::new(), String::new(), false, 0);
    };
    let mut vendor = String::new();
    let mut driver = String::new();
    let mut has_card = false;
    let mut displays: u8 = 0;

    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with("card") && !name.contains('-') {
            has_card = true;
            let dev = entry.path().join("device");
            if vendor.is_empty() {
                if let Ok(v) = fs::read_to_string(dev.join("vendor")) {
                    vendor = v.trim().to_string();
                }
            }
            if driver.is_empty() {
                if let Ok(link) = fs::read_link(dev.join("driver")) {
                    if let Some(d) = link.file_name() {
                        driver = d.to_string_lossy().to_string();
                    }
                }
            }
        } else if name.starts_with("card") && name.contains('-') {
            if let Ok(status) = fs::read_to_string(entry.path().join("status")) {
                if status.trim() == "connected" {
                    displays = displays.saturating_add(1);
                }
            }
        }
    }

    (vendor, driver, has_card, displays)
}
