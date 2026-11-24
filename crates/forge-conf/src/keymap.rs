#![forbid(unsafe_code)]

use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    Launcher,
    Terminal,
    Files,
    Browser,
    CloseWindow,
    Fullscreen,
    FloatToggle,
    TileToggle,
    NextWindow,
    FocusLeft,
    FocusRight,
    Screenshot,
    Lock,
    Logout,
    Workspace(u8),
    MoveToWorkspace(u8),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bind {
    pub mods: Vec<String>,
    pub key: String,
    pub action: Action,
}

#[derive(Debug, Clone)]
pub struct Keymap {
    pub binds: Vec<Bind>,
}

impl Keymap {
    pub fn parse_toml(content: &str) -> Result<Self, String> {
        let mut map = BTreeMap::new();
        for (i, raw) in content.lines().enumerate() {
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let (k, v) = line
                .split_once('=')
                .ok_or_else(|| format!("keymap.toml:{}: expected key = value", i + 1))?;
            let val = v
                .trim()
                .strip_prefix('"')
                .and_then(|s| s.strip_suffix('"'))
                .ok_or_else(|| format!("keymap.toml:{}: value must be quoted", i + 1))?;
            map.insert(k.trim().to_string(), val.to_string());
        }

        let specs: [(&str, Action); 14] = [
            ("launcher", Action::Launcher),
            ("terminal", Action::Terminal),
            ("files", Action::Files),
            ("browser", Action::Browser),
            ("close", Action::CloseWindow),
            ("fullscreen", Action::Fullscreen),
            ("float_toggle", Action::FloatToggle),
            ("tile_toggle", Action::TileToggle),
            ("next_window", Action::NextWindow),
            ("focus_left", Action::FocusLeft),
            ("focus_right", Action::FocusRight),
            ("screenshot", Action::Screenshot),
            ("lock", Action::Lock),
            ("logout", Action::Logout),
        ];

        let mut binds = Vec::new();
        for (name, action) in specs {
            let combo = map
                .get(name)
                .ok_or_else(|| format!("missing key `{name}` in keymap.toml"))?;
            let (mods, key) = split_combo(combo)?;
            binds.push(Bind { mods, key, action });
        }

        for ws in 1..=9u8 {
            binds.push(Bind {
                mods: vec!["Super".to_string()],
                key: ws.to_string(),
                action: Action::Workspace(ws),
            });
            binds.push(Bind {
                mods: vec!["Super".to_string(), "Shift".to_string()],
                key: ws.to_string(),
                action: Action::MoveToWorkspace(ws),
            });
        }

        Ok(Self { binds })
    }
}

fn split_combo(combo: &str) -> Result<(Vec<String>, String), String> {
    let parts: Vec<&str> = combo.split('+').map(str::trim).collect();
    if parts.is_empty() || parts.iter().any(|p| p.is_empty()) {
        return Err(format!("invalid key combo: {combo}"));
    }
    let key = (*parts.last().expect("non-empty")).to_string();
    let mods = parts[..parts.len() - 1]
        .iter()
        .map(|s| (*s).to_string())
        .collect();
    Ok((mods, key))
}
