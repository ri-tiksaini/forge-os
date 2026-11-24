#![forbid(unsafe_code)]

//! Headless view models & project scaffold helper for `forge-settings` (Blueprint 6.8).

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use forge_catalog::{Activation, CatalogEntry, CatalogIndex};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoreItem {
    pub id: String,
    pub name: String,
    pub category: String,
    pub summary: String,
    pub disk_mb: u32,
    pub ram_idle_mb: u32,
    pub ram_active_mb: u32,
    pub beginner_note: String,
    pub is_parked_capable: bool,
}

impl From<&CatalogEntry> for StoreItem {
    fn from(e: &CatalogEntry) -> Self {
        Self {
            id: e.id.clone(),
            name: e.name.clone(),
            category: e.category.clone(),
            summary: e.summary.clone(),
            disk_mb: e.disk_mb,
            ram_idle_mb: e.ram_idle_mb,
            ram_active_mb: e.ram_active_mb,
            beginner_note: e.beginner_note.clone(),
            is_parked_capable: e.activation == Activation::OnDemand,
        }
    }
}

pub fn query_store(index: &CatalogIndex, query: &str, tier: u8) -> Vec<StoreItem> {
    let installed = HashSet::new();
    index
        .search(query, 24, tier, &installed)
        .into_iter()
        .filter_map(|h| index.get(&h.id).map(StoreItem::from))
        .collect()
}

pub fn parked_services(index: &CatalogIndex) -> Vec<StoreItem> {
    index
        .entries()
        .iter()
        .filter(|e| e.activation == Activation::OnDemand)
        .map(StoreItem::from)
        .collect()
}

/// Creates a clean starter project directory for the Projects wizard (Blueprint 1.4 F4 & 6.8).
pub fn create_project(base_dir: &Path, stack: &str, name: &str) -> Result<PathBuf, String> {
    let clean = name.trim();
    if clean.is_empty() || clean.contains('/') || clean.contains('\\') || clean.contains("..") {
        return Err("project name must be a single valid folder name".to_string());
    }
    let dir = base_dir.join(clean);
    fs::create_dir_all(&dir).map_err(|e| format!("create {}: {e}", dir.display()))?;

    let readme = format!(
        "# {clean}\n\nCreated with {} ({stack} stack).\n",
        forge_brand::NAME
    );
    fs::write(dir.join("README.md"), readme).map_err(|e| e.to_string())?;

    match stack {
        "web" => {
            let pkg = format!(
                "{{\n  \"name\": \"{clean}\",\n  \"version\": \"0.1.0\",\n  \"scripts\": {{ \"start\": \"node index.js\" }}\n}}\n"
            );
            fs::write(dir.join("package.json"), pkg).map_err(|e| e.to_string())?;
            fs::write(
                dir.join("index.js"),
                "console.log('Hello from Forge OS Web Project!');\n",
            )
            .map_err(|e| e.to_string())?;
        }
        "rust" => {
            fs::create_dir_all(dir.join("src")).map_err(|e| e.to_string())?;
            let cargo =
                format!("[package]\nname = \"{clean}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n");
            fs::write(dir.join("Cargo.toml"), cargo).map_err(|e| e.to_string())?;
            fs::write(
                dir.join("src/main.rs"),
                "fn main() {\n    println!(\"Hello from Forge OS Rust Project!\");\n}\n",
            )
            .map_err(|e| e.to_string())?;
        }
        _ => {
            fs::write(
                dir.join("main.py"),
                "print('Hello from Forge OS Project!')\n",
            )
            .map_err(|e| e.to_string())?;
        }
    }

    Ok(dir)
}
