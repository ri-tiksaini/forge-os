#![forbid(unsafe_code)]

use std::env;
use std::path::PathBuf;
use std::process;

use forge_catalog::CatalogIndex;

fn main() {
    let args: Vec<String> = env::args().collect();
    let mut catalog_dir = PathBuf::from("/usr/share/forge/catalog");

    let mut i = 1;
    while i < args.len() {
        if args[i] == "--catalog-dir" && i + 1 < args.len() {
            catalog_dir = PathBuf::from(&args[i + 1]);
            i += 2;
        } else {
            i += 1;
        }
    }

    let entries = forge_catalog::load_dir(&catalog_dir).unwrap_or_else(|e| {
        eprintln!(
            "forge-settings: failed to load {}: {e}",
            catalog_dir.display()
        );
        process::exit(1);
    });
    let index = CatalogIndex::build(entries);
    let tier = forge_tier::detect();

    #[cfg(feature = "gtk-ui")]
    {
        forge_settings::ui::run(index, tier);
    }

    #[cfg(not(feature = "gtk-ui"))]
    {
        println!(
            "{} Settings (headless — build with --features gtk-ui)",
            forge_brand::NAME
        );
        println!(
            "Tier: {} | Catalog entries: {}",
            tier.tier,
            index.entries().len()
        );
    }
}
