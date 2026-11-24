#![forbid(unsafe_code)]

//! Launcher state machine, merged providers, and progressive CLI disclosure (Blueprint 6.7 & 7.4).

use std::collections::HashSet;

use forge_catalog::CatalogIndex;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LauncherState {
    Hidden,
    Opening,
    Ready,
    Querying,
    Executing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderKind {
    App,
    Catalog,
    Action,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LauncherRow {
    pub provider: ProviderKind,
    pub id: String,
    pub title: String,
    pub subtitle_right: String,
    pub cli_hint: String, // Line 2 dim monospace teaching trick (Blueprint 6.7)
    pub exec_cmd: String,
    pub score: i32,
}

pub struct LauncherEngine {
    pub state: LauncherState,
    pub query: String,
    pub selected: usize,
    catalog: CatalogIndex,
    device_tier: u8,
    installed: HashSet<String>,
    actions: Vec<LauncherRow>,
}

impl LauncherEngine {
    pub fn new(catalog: CatalogIndex, device_tier: u8, installed: HashSet<String>) -> Self {
        let actions = vec![
            LauncherRow {
                provider: ProviderKind::Action,
                id: "action:terminal".into(),
                title: "Terminal (foot)".into(),
                subtitle_right: "App".into(),
                cli_hint: "foot".into(),
                exec_cmd: "foot".into(),
                score: 40,
            },
            LauncherRow {
                provider: ProviderKind::Action,
                id: "action:files".into(),
                title: "Files (Thunar)".into(),
                subtitle_right: "App".into(),
                cli_hint: "thunar".into(),
                exec_cmd: "thunar".into(),
                score: 38,
            },
            LauncherRow {
                provider: ProviderKind::Action,
                id: "action:browser".into(),
                title: "Web Browser (Firefox ESR)".into(),
                subtitle_right: "App".into(),
                cli_hint: "firefox-esr".into(),
                exec_cmd: "firefox-esr".into(),
                score: 38,
            },
            LauncherRow {
                provider: ProviderKind::Action,
                id: "action:doctor".into(),
                title: "System Tier & Health Report".into(),
                subtitle_right: "Action".into(),
                cli_hint: "dev doctor".into(),
                exec_cmd: "foot -e sh -c 'dev doctor; read -p \"Press Enter...\" _'".into(),
                score: 35,
            },
            LauncherRow {
                provider: ProviderKind::Action,
                id: "action:lock".into(),
                title: "Lock Screen".into(),
                subtitle_right: "Action".into(),
                cli_hint: "swaylock -f -c 0E1013".into(),
                exec_cmd: "swaylock -f -c 0E1013".into(),
                score: 30,
            },
        ];

        Self {
            state: LauncherState::Hidden,
            query: String::with_capacity(256),
            selected: 0,
            catalog,
            device_tier,
            installed,
            actions,
        }
    }

    pub fn open(&mut self) {
        self.state = LauncherState::Opening;
        self.query.clear();
        self.selected = 0;
        self.state = LauncherState::Ready;
    }

    pub fn close(&mut self) {
        self.state = LauncherState::Hidden;
        self.query.clear();
        self.selected = 0;
    }

    /// Queries merged providers and returns at most 12 ranked rows (Blueprint 6.7 & 7.4).
    pub fn query_rows(&mut self, input: &str) -> Vec<LauncherRow> {
        self.state = LauncherState::Querying;
        self.query.clear();
        self.query.push_str(input);
        self.selected = 0;

        let q_lower = input.trim().to_ascii_lowercase();
        let mut rows: Vec<LauncherRow> = Vec::with_capacity(16);

        // 1. Catalog provider (<= 7 rows)
        for hit in self
            .catalog
            .search(input, 7, self.device_tier, &self.installed)
        {
            if let Some(e) = self.catalog.get(&hit.id) {
                let is_inst = self.installed.contains(&e.id);
                let cli_hint = if is_inst {
                    format!("dev remove {}", e.id)
                } else {
                    format!("dev add {}", e.id)
                };
                let exec_cmd =
                    format!("foot -e sh -c '{cli_hint} --yes; read -p \"Done. Press Enter...\" _'");
                let subtitle_right = format!("{} - {} MB", capitalize(&e.category), e.disk_mb);
                rows.push(LauncherRow {
                    provider: ProviderKind::Catalog,
                    id: format!("catalog:{}", e.id),
                    title: e.name.clone(),
                    subtitle_right,
                    cli_hint,
                    exec_cmd,
                    score: hit.score,
                });
            }
        }

        // 2. Built-in Apps & Actions provider (<= 5 rows)
        let mut action_count = 0usize;
        for act in &self.actions {
            if q_lower.is_empty()
                || act.title.to_ascii_lowercase().contains(&q_lower)
                || act.cli_hint.to_ascii_lowercase().contains(&q_lower)
            {
                let bonus = if !q_lower.is_empty()
                    && act.title.to_ascii_lowercase().starts_with(&q_lower)
                {
                    65
                } else {
                    0
                };
                let mut cloned = act.clone();
                cloned.score += bonus;
                rows.push(cloned);
                action_count += 1;
                if action_count >= 5 {
                    break;
                }
            }
        }

        rows.sort_by(|a, b| b.score.cmp(&a.score).then_with(|| a.id.cmp(&b.id)));
        rows.truncate(12);
        self.state = LauncherState::Ready;
        rows
    }
}

fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) => format!("{}{}", c.to_ascii_uppercase(), chars.as_str()),
        None => String::new(),
    }
}
