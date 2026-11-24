#![forbid(unsafe_code)]

use std::collections::HashSet;
use std::env;
use std::path::PathBuf;
use std::process;

use forge_catalog::CatalogIndex;
use forge_shell::launcher::LauncherEngine;
use forge_shell::pill::PillSnapshot;

fn main() {
    let args: Vec<String> = env::args().collect();
    let mut mode = String::from("launcher");
    let mut catalog_dir = PathBuf::from("/usr/share/forge/catalog");

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--mode" if i + 1 < args.len() => {
                mode = args[i + 1].clone();
                i += 2;
            }
            "--catalog-dir" if i + 1 < args.len() => {
                catalog_dir = PathBuf::from(&args[i + 1]);
                i += 2;
            }
            _ => i += 1,
        }
    }

    let report = forge_tier::detect();

    if mode == "welcome" {
        #[cfg(feature = "gtk-ui")]
        {
            forge_shell::ui::run_welcome();
            return;
        }
        #[cfg(not(feature = "gtk-ui"))]
        {
            println!("{} ({})", forge_brand::PRETTY_NAME, forge_brand::HOME_URL);
            return;
        }
    }

    if mode == "power" {
        #[cfg(feature = "gtk-ui")]
        {
            forge_shell::ui::run_power();
            return;
        }
        #[cfg(not(feature = "gtk-ui"))]
        {
            println!("Power menu requires gtk-ui");
            return;
        }
    }

    if mode == "quick" {
        #[cfg(feature = "gtk-ui")]
        {
            forge_shell::ui::run_quick_settings();
            return;
        }
        #[cfg(not(feature = "gtk-ui"))]
        {
            println!("Quick settings require gtk-ui");
            return;
        }
    }

    if mode == "menu" {
        #[cfg(feature = "gtk-ui")]
        {
            forge_shell::ui::run_context_menu();
            return;
        }
        #[cfg(not(feature = "gtk-ui"))]
        {
            println!("Context menu requires gtk-ui");
            return;
        }
    }

    if mode == "pill" || mode == "taskbar" {
        let label = format!("T{}", report.tier.as_u8());
        #[cfg(feature = "gtk-ui")]
        {
            forge_shell::ui::run_taskbar(label);
            return;
        }
        #[cfg(not(feature = "gtk-ui"))]
        {
            let snap = PillSnapshot::sample(&label);
            println!("{}", snap.format_compact());
            return;
        }
    }

    let entries = forge_catalog::load_dir(&catalog_dir).unwrap_or_else(|e| {
        eprintln!(
            "forge-shell: failed to load catalog {}: {e}",
            catalog_dir.display()
        );
        process::exit(1);
    });
    let index = CatalogIndex::build(entries);
    let mut engine = LauncherEngine::new(index, report.tier.as_u8(), HashSet::new());
    engine.open();

    #[cfg(feature = "gtk-ui")]
    {
        forge_shell::ui::run_launcher(engine);
    }

    #[cfg(not(feature = "gtk-ui"))]
    {
        let rows = engine.query_rows("");
        println!(
            "{} Launcher (headless mode — build with --features gtk-ui for GUI)",
            forge_brand::NAME
        );
        for r in rows {
            println!(
                "  > {:<24} {:<18} [{}]",
                r.title, r.subtitle_right, r.cli_hint
            );
        }
    }
}
