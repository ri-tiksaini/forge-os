#![forbid(unsafe_code)]

use std::collections::{BTreeMap, VecDeque};
use std::fs;
use std::path::Path;

use crate::model::{Activation, Backend, CatalogEntry, ParkingMode};

pub fn load_dir(dir: &Path) -> Result<Vec<CatalogEntry>, String> {
    let mut entries = Vec::new();
    collect_toml(dir, &mut entries)?;
    entries.sort_by(|a, b| a.id.cmp(&b.id));
    validate_catalog(&entries)?;
    Ok(entries)
}

fn collect_toml(dir: &Path, out: &mut Vec<CatalogEntry>) -> Result<(), String> {
    let read = fs::read_dir(dir).map_err(|e| format!("read {}: {e}", dir.display()))?;
    let mut paths: Vec<_> = read.flatten().map(|e| e.path()).collect();
    paths.sort();

    for path in paths {
        if path.is_dir() {
            collect_toml(&path, out)?;
        } else if path.extension().is_some_and(|e| e == "toml") {
            let text =
                fs::read_to_string(&path).map_err(|e| format!("read {}: {e}", path.display()))?;
            let entry = parse_entry_toml(&text).map_err(|e| format!("{}: {e}", path.display()))?;
            out.push(entry);
        }
    }
    Ok(())
}

pub fn parse_entry_toml(content: &str) -> Result<CatalogEntry, String> {
    let mut map = BTreeMap::new();
    for (idx, raw) in content.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (k, v) = line
            .split_once('=')
            .ok_or_else(|| format!("line {}: expected key = value", idx + 1))?;
        map.insert(k.trim().to_string(), v.trim().to_string());
    }

    let get_str = |k: &str| -> Result<String, String> {
        let raw = map.get(k).ok_or_else(|| format!("missing key `{k}`"))?;
        unquote(raw).ok_or_else(|| format!("key `{k}` must be a string"))
    };
    let get_u32 = |k: &str| -> Result<u32, String> {
        let raw = map.get(k).ok_or_else(|| format!("missing key `{k}`"))?;
        raw.parse::<u32>()
            .map_err(|_| format!("key `{k}` must be u32"))
    };
    let get_bool = |k: &str| -> Result<bool, String> {
        let raw = map.get(k).ok_or_else(|| format!("missing key `{k}`"))?;
        match raw.as_str() {
            "true" => Ok(true),
            "false" => Ok(false),
            _ => Err(format!("key `{k}` must be bool")),
        }
    };
    let get_arr = |k: &str| -> Result<Vec<String>, String> {
        let raw = map.get(k).ok_or_else(|| format!("missing key `{k}`"))?;
        parse_str_list(raw).ok_or_else(|| format!("key `{k}` must be string array"))
    };

    Ok(CatalogEntry {
        id: get_str("id")?,
        name: get_str("name")?,
        category: get_str("category")?,
        summary: get_str("summary")?,
        keywords: get_arr("keywords")?,
        backend: Backend::parse(&get_str("backend")?)?,
        packages: get_arr("packages")?,
        activation: Activation::parse(&get_str("activation")?)?,
        parking_mode: ParkingMode::parse(&get_str("parking_mode")?)?,
        autostart: get_bool("autostart")?,
        disk_mb: get_u32("disk_mb")?,
        ram_idle_mb: get_u32("ram_idle_mb")?,
        ram_active_mb: get_u32("ram_active_mb")?,
        tier_min: get_u32("tier_min")? as u8,
        license: get_str("license")?,
        popularity: get_u32("popularity")?.min(100) as u16,
        depends: get_arr("depends")?,
        conflicts: get_arr("conflicts")?,
        gui_entry: get_bool("gui_entry")?,
        cli_entry: get_arr("cli_entry")?,
        beginner_note: get_str("beginner_note")?,
        heavy_warning: get_str("heavy_warning")?,
    })
}

/// Validates Blueprint 6.5.1 schema rules & DAG acyclicity via Kahn's algorithm (CI Gate G3).
pub fn validate_catalog(entries: &[CatalogEntry]) -> Result<(), String> {
    let mut id_to_idx: BTreeMap<&str, usize> = BTreeMap::new();
    for (idx, e) in entries.iter().enumerate() {
        if id_to_idx.insert(e.id.as_str(), idx).is_some() {
            return Err(format!("duplicate catalog id `{}`", e.id));
        }
        if e.autostart && e.category != "core" {
            return Err(format!(
                "entry `{}` violates N2: autostart=true forbidden outside category=core",
                e.id
            ));
        }
        if e.ram_idle_mb > 0 && e.activation != Activation::Resident {
            return Err(format!(
                "entry `{}` has ram_idle_mb > 0 but activation is not resident",
                e.id
            ));
        }
        if e.activation == Activation::OnDemand && e.parking_mode == ParkingMode::None {
            return Err(format!(
                "entry `{}` is on-demand but parking_mode is none",
                e.id
            ));
        }
        if !e.gui_entry && e.cli_entry.is_empty() {
            return Err(format!(
                "entry `{}` must expose gui_entry or cli_entry for launcher discovery",
                e.id
            ));
        }
    }

    // Kahn's algorithm for DAG acyclicity + dependency existence
    let n = entries.len();
    let mut indegree = vec![0usize; n];
    let mut adj: Vec<Vec<usize>> = vec![Vec::new(); n];

    for (u, e) in entries.iter().enumerate() {
        for dep in &e.depends {
            let Some(&v) = id_to_idx.get(dep.as_str()) else {
                return Err(format!("entry `{}` depends on unknown id `{dep}`", e.id));
            };
            adj[v].push(u);
            indegree[u] += 1;
        }
    }

    let mut q = VecDeque::new();
    for (i, &deg) in indegree.iter().enumerate() {
        if deg == 0 {
            q.push_back(i);
        }
    }

    let mut visited = 0usize;
    while let Some(v) = q.pop_front() {
        visited += 1;
        for &next in &adj[v] {
            indegree[next] -= 1;
            if indegree[next] == 0 {
                q.push_back(next);
            }
        }
    }

    if visited != n {
        return Err("catalog dependency graph contains a cycle".to_string());
    }
    Ok(())
}

fn unquote(s: &str) -> Option<String> {
    let inner = s.strip_prefix('"')?.strip_suffix('"')?;
    Some(inner.replace("\\\"", "\"").replace("\\\\", "\\"))
}

fn parse_str_list(s: &str) -> Option<Vec<String>> {
    let inner = s.strip_prefix('[')?.strip_suffix(']')?.trim();
    if inner.is_empty() {
        return Some(Vec::new());
    }
    let mut out = Vec::new();
    for part in inner.split(',') {
        let item = unquote(part.trim())?;
        out.push(item);
    }
    Some(out)
}
