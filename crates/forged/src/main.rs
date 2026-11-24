#![forbid(unsafe_code)]

use std::env;
use std::path::PathBuf;
use std::process;

use forged::{serve, DaemonConfig};

fn main() {
    let args: Vec<String> = env::args().collect();
    let mut socket_path = PathBuf::from(forge_proto::DEFAULT_SOCKET_PATH);
    let mut catalog_dir = PathBuf::from("/usr/share/forge/catalog");
    let mut journal_dir = PathBuf::from("/var/lib/forge/journal");
    let mut idle_timeout_secs: u64 = 90;
    let mut dry_run = false;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--socket" if i + 1 < args.len() => {
                socket_path = PathBuf::from(&args[i + 1]);
                i += 2;
            }
            "--catalog-dir" if i + 1 < args.len() => {
                catalog_dir = PathBuf::from(&args[i + 1]);
                i += 2;
            }
            "--journal-dir" if i + 1 < args.len() => {
                journal_dir = PathBuf::from(&args[i + 1]);
                i += 2;
            }
            "--idle-timeout" if i + 1 < args.len() => {
                idle_timeout_secs = args[i + 1].parse().unwrap_or(90);
                i += 2;
            }
            "--dry-run" => {
                dry_run = true;
                i += 1;
            }
            _ => i += 1,
        }
    }

    let cfg = DaemonConfig {
        socket_path,
        catalog_dir,
        journal_dir,
        idle_timeout_secs,
        dry_run,
    };

    if let Err(e) = serve(cfg) {
        eprintln!("forged error: {e}");
        process::exit(1);
    }
}
