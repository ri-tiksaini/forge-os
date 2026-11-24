#![forbid(unsafe_code)]

use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    let report = forge_tier::detect();

    if args.iter().any(|a| a == "--json") {
        println!("{}", report.to_json());
        return;
    }

    println!("Forge OS Hardware Tier Report");
    println!("  Tier       : {}", report.tier);
    println!("  Compositor : {}", report.tier.compositor());
    println!("  Score      : {}/100", report.score);
    println!("  RAM        : {} MiB", report.raw.mem_total_kb / 1024);
    println!(
        "  Disk       : {} ({})",
        if report.raw.disk_rotational {
            "Rotational HDD"
        } else {
            "SSD/NVMe"
        },
        report.raw.disk_model
    );
    println!(
        "  GPU        : {} (DRM={})",
        if report.raw.gpu_driver.is_empty() {
            "none"
        } else {
            &report.raw.gpu_driver
        },
        report.raw.has_drm_driver
    );
    println!("  Reasons    :");
    for r in &report.reasons {
        println!("    - [{}] {}", r.code, r.message);
    }
}
