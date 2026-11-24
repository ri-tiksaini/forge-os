#![forbid(unsafe_code)]

use std::env;
use std::fs;
use std::path::PathBuf;
use std::process;

const DEFAULT_KEYMAP: &str = include_str!("../../../data/keymap.toml");

fn main() {
    let args: Vec<String> = env::args().collect();
    let mut out_dir: Option<PathBuf> = None;
    let mut keymap_path: Option<PathBuf> = None;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--out" if i + 1 < args.len() => {
                out_dir = Some(PathBuf::from(&args[i + 1]));
                i += 2;
            }
            "--keymap" if i + 1 < args.len() => {
                keymap_path = Some(PathBuf::from(&args[i + 1]));
                i += 2;
            }
            _ => i += 1,
        }
    }

    let target = out_dir.unwrap_or_else(|| {
        let home = env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
        PathBuf::from(home).join(".config")
    });

    let raw = match keymap_path {
        Some(p) => fs::read_to_string(&p).unwrap_or_else(|e| {
            eprintln!("failed to read {}: {e}", p.display());
            process::exit(1);
        }),
        None => DEFAULT_KEYMAP.to_string(),
    };

    let km = forge_conf::keymap::Keymap::parse_toml(&raw).unwrap_or_else(|e| {
        eprintln!("keymap parse error: {e}");
        process::exit(1);
    });

    if let Err(e) = forge_conf::write_all(&km, &target) {
        eprintln!("failed to write configs: {e}");
        process::exit(1);
    }

    println!(
        "Generated labwc, sway, and gtk-4.0 configs in {}",
        target.display()
    );
}
