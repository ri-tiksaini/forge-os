#![forbid(unsafe_code)]

//! Journaled, batched, recoverable transaction engine (Blueprint 5.5 & 6.5.3).

use std::collections::{BTreeSet, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use forge_catalog::{Backend, CatalogEntry, CatalogIndex};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TxnStep {
    pub backend: Backend,
    pub entry_ids: Vec<String>,
    pub argv: Vec<String>,
    pub undo_argv: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TxnPlan {
    pub token: String,
    pub remove: bool,
    pub requested: Vec<String>,
    pub steps: Vec<TxnStep>,
    pub disk_mb: u32,
    pub peak_ram_mb: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JournalState {
    Planned,
    InProgress,
    Completed,
    Failed,
    RolledBack,
}

impl JournalState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Planned => "PLANNED",
            Self::InProgress => "IN_PROGRESS",
            Self::Completed => "COMPLETED",
            Self::Failed => "FAILED",
            Self::RolledBack => "ROLLED_BACK",
        }
    }
}

pub trait StepExecutor {
    fn run(&mut self, argv: &[String]) -> Result<String, String>;
}

/// Real system executor running bulk operations under `ionice -c 2 -n 7` (Blueprint 5.9).
pub struct SystemExecutor;

impl StepExecutor for SystemExecutor {
    fn run(&mut self, argv: &[String]) -> Result<String, String> {
        let Some((prog, args)) = argv.split_first() else {
            return Ok(String::new());
        };
        let out = Command::new("ionice")
            .args(["-c", "2", "-n", "7", prog])
            .args(args)
            .output()
            .map_err(|e| format!("failed to spawn {prog}: {e}"))?;
        if out.status.success() {
            Ok(String::from_utf8_lossy(&out.stdout).to_string())
        } else {
            Err(format!(
                "{prog} exited {}: {}",
                out.status,
                String::from_utf8_lossy(&out.stderr).trim()
            ))
        }
    }
}

/// Pure install/remove planner: resolves transitive DAG deps and batches apt into ONE call.
pub fn plan(
    requested_ids: &[String],
    remove: bool,
    catalog: &CatalogIndex,
    installed: &HashSet<String>,
) -> Result<TxnPlan, String> {
    let mut ordered: Vec<&CatalogEntry> = Vec::new();
    let mut visited: HashSet<String> = HashSet::new();

    for id in requested_ids {
        resolve_dfs(id, catalog, &mut visited, &mut ordered)?;
    }

    let targets: Vec<&CatalogEntry> = if remove {
        ordered
            .into_iter()
            .rev()
            .filter(|e| requested_ids.contains(&e.id) && installed.contains(&e.id))
            .collect()
    } else {
        ordered
            .into_iter()
            .filter(|e| !installed.contains(&e.id))
            .collect()
    };

    let mut apt_pkgs: BTreeSet<String> = BTreeSet::new();
    let mut apt_ids: Vec<String> = Vec::new();
    let mut steps: Vec<TxnStep> = Vec::new();
    let mut disk_mb: u32 = 0;
    let mut peak_ram_mb: u32 = 0;

    for e in targets {
        disk_mb = disk_mb.saturating_add(e.disk_mb);
        peak_ram_mb = peak_ram_mb.max(e.ram_active_mb);

        match e.backend {
            Backend::Apt => {
                apt_ids.push(e.id.clone());
                for p in &e.packages {
                    apt_pkgs.insert(p.clone());
                }
            }
            Backend::Flatpak => {
                for r in &e.packages {
                    let (argv, undo_argv) = if remove {
                        (
                            vec![
                                "flatpak".into(),
                                "uninstall".into(),
                                "--system".into(),
                                "-y".into(),
                                r.clone(),
                            ],
                            vec![
                                "flatpak".into(),
                                "install".into(),
                                "--system".into(),
                                "-y".into(),
                                "--noninteractive".into(),
                                r.clone(),
                            ],
                        )
                    } else {
                        (
                            vec![
                                "flatpak".into(),
                                "install".into(),
                                "--system".into(),
                                "-y".into(),
                                "--noninteractive".into(),
                                r.clone(),
                            ],
                            vec![
                                "flatpak".into(),
                                "uninstall".into(),
                                "--system".into(),
                                "-y".into(),
                                r.clone(),
                            ],
                        )
                    };
                    steps.push(TxnStep {
                        backend: Backend::Flatpak,
                        entry_ids: vec![e.id.clone()],
                        argv,
                        undo_argv,
                    });
                }
            }
            Backend::Mise => {
                for tool in &e.packages {
                    let (argv, undo_argv) = if remove {
                        (
                            vec!["mise".into(), "uninstall".into(), tool.clone()],
                            vec!["mise".into(), "use".into(), "-g".into(), tool.clone()],
                        )
                    } else {
                        (
                            vec!["mise".into(), "use".into(), "-g".into(), tool.clone()],
                            vec!["mise".into(), "uninstall".into(), tool.clone()],
                        )
                    };
                    steps.push(TxnStep {
                        backend: Backend::Mise,
                        entry_ids: vec![e.id.clone()],
                        argv,
                        undo_argv,
                    });
                }
            }
            Backend::Oci => {
                for img in &e.packages {
                    steps.push(TxnStep {
                        backend: Backend::Oci,
                        entry_ids: vec![e.id.clone()],
                        argv: vec!["podman".into(), "pull".into(), img.clone()],
                        undo_argv: vec!["podman".into(), "rmi".into(), img.clone()],
                    });
                }
            }
            Backend::Stack => {}
        }
    }

    // Batch ALL apt packages into ONE single apt-get call (Blueprint 5.5)
    if !apt_pkgs.is_empty() {
        let pkg_list: Vec<String> = apt_pkgs.into_iter().collect();
        let (mut argv, mut undo_argv) = if remove {
            (
                vec!["apt-get".into(), "-y".into(), "purge".into()],
                vec![
                    "apt-get".into(),
                    "-y".into(),
                    "--no-install-recommends".into(),
                    "install".into(),
                ],
            )
        } else {
            (
                vec![
                    "apt-get".into(),
                    "-y".into(),
                    "--no-install-recommends".into(),
                    "install".into(),
                ],
                vec!["apt-get".into(), "-y".into(), "purge".into()],
            )
        };
        argv.extend(pkg_list.clone());
        undo_argv.extend(pkg_list);
        steps.insert(
            0,
            TxnStep {
                backend: Backend::Apt,
                entry_ids: apt_ids,
                argv,
                undo_argv,
            },
        );
    }

    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);

    Ok(TxnPlan {
        token: format!("txn-{nanos}"),
        remove,
        requested: requested_ids.to_vec(),
        steps,
        disk_mb,
        peak_ram_mb,
    })
}

fn resolve_dfs<'a>(
    id: &str,
    catalog: &'a CatalogIndex,
    visited: &mut HashSet<String>,
    ordered: &mut Vec<&'a CatalogEntry>,
) -> Result<(), String> {
    if visited.contains(id) {
        return Ok(());
    }
    let entry = catalog
        .get(id)
        .ok_or_else(|| format!("catalog entry not found: `{id}`"))?;
    visited.insert(entry.id.clone());
    for dep in &entry.depends {
        resolve_dfs(dep, catalog, visited, ordered)?;
    }
    ordered.push(entry);
    Ok(())
}

pub fn apply_with_journal(
    plan: &TxnPlan,
    journal_dir: &Path,
    exec: &mut dyn StepExecutor,
) -> Result<usize, (usize, String)> {
    fs::create_dir_all(journal_dir).map_err(|e| (0, e.to_string()))?;
    let jpath = journal_path(journal_dir, &plan.token);
    write_journal(&jpath, plan, JournalState::InProgress, 0).map_err(|e| (0, e))?;

    let mut completed = 0usize;
    for step in &plan.steps {
        if let Err(err) = exec.run(&step.argv) {
            let _ = write_journal(&jpath, plan, JournalState::Failed, completed);
            return Err((completed, err));
        }
        completed += 1;
        let _ = write_journal(&jpath, plan, JournalState::InProgress, completed);
    }

    let _ = write_journal(&jpath, plan, JournalState::Completed, completed);
    Ok(completed)
}

pub fn rollback_steps(
    plan: &TxnPlan,
    completed_count: usize,
    journal_dir: &Path,
    exec: &mut dyn StepExecutor,
) -> Result<(), String> {
    let jpath = journal_path(journal_dir, &plan.token);
    for step in plan.steps[..completed_count].iter().rev() {
        exec.run(&step.undo_argv)?;
    }
    write_journal(&jpath, plan, JournalState::RolledBack, 0)?;
    Ok(())
}

pub fn scan_incomplete_journals(journal_dir: &Path) -> Vec<String> {
    let Ok(read) = fs::read_dir(journal_dir) else {
        return Vec::new();
    };
    let mut incomplete = Vec::new();
    for entry in read.flatten() {
        if let Ok(text) = fs::read_to_string(entry.path()) {
            if text.contains("\"state\":\"IN_PROGRESS\"") || text.contains("\"state\":\"FAILED\"") {
                if let Some(stem) = entry.path().file_stem() {
                    incomplete.push(stem.to_string_lossy().to_string());
                }
            }
        }
    }
    incomplete.sort();
    incomplete
}

fn journal_path(dir: &Path, token: &str) -> PathBuf {
    dir.join(format!("{token}.json"))
}

fn write_journal(
    path: &Path,
    plan: &TxnPlan,
    state: JournalState,
    completed: usize,
) -> Result<(), String> {
    let body = format!(
        "{{\"token\":\"{}\",\"state\":\"{}\",\"completed\":{},\"total\":{},\"disk_mb\":{}}}\n",
        plan.token,
        state.as_str(),
        completed,
        plan.steps.len(),
        plan.disk_mb
    );
    fs::write(path, body).map_err(|e| format!("write journal {}: {e}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use forge_catalog::load_dir;

    struct RecordingExecutor {
        ran: Vec<Vec<String>>,
        fail_on: Option<usize>,
    }

    impl StepExecutor for RecordingExecutor {
        fn run(&mut self, argv: &[String]) -> Result<String, String> {
            let idx = self.ran.len();
            self.ran.push(argv.to_vec());
            if self.fail_on == Some(idx) {
                return Err("simulated failure".to_string());
            }
            Ok("ok".to_string())
        }
    }

    #[test]
    fn stack_plan_resolves_deps_and_batches_apt_into_one_call() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/catalog");
        let index = CatalogIndex::build(load_dir(&dir).expect("catalog"));
        let installed = HashSet::new();

        // Requesting web (node + postgres) + neovim must batch postgres & neovim into ONE apt-get
        let p = plan(
            &["web".to_string(), "neovim".to_string()],
            false,
            &index,
            &installed,
        )
        .expect("plan");

        let apt_steps: Vec<_> = p
            .steps
            .iter()
            .filter(|s| s.backend == Backend::Apt)
            .collect();
        assert_eq!(
            apt_steps.len(),
            1,
            "apt packages must be batched into ONE call"
        );
        assert!(apt_steps[0].argv.contains(&"postgresql".to_string()));
        assert!(apt_steps[0].argv.contains(&"neovim".to_string()));
        assert!(p.disk_mb >= 320 + 180 + 38);
    }

    #[test]
    fn failed_apply_writes_journal_and_rollback_replays_undo() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/catalog");
        let index = CatalogIndex::build(load_dir(&dir).expect("catalog"));
        let p = plan(&["web".to_string()], false, &index, &HashSet::new()).expect("plan");
        assert!(p.steps.len() >= 2);

        let jdir = std::env::temp_dir().join(format!("forge-txn-test-{}", p.token));
        let mut exec = RecordingExecutor {
            ran: Vec::new(),
            fail_on: Some(1),
        };
        let err = apply_with_journal(&p, &jdir, &mut exec).unwrap_err();
        assert_eq!(err.0, 1);
        assert_eq!(scan_incomplete_journals(&jdir), vec![p.token.clone()]);

        let mut undo_exec = RecordingExecutor {
            ran: Vec::new(),
            fail_on: None,
        };
        rollback_steps(&p, 1, &jdir, &mut undo_exec).expect("rollback");
        assert_eq!(undo_exec.ran.len(), 1);
        assert!(scan_incomplete_journals(&jdir).is_empty());
        let _ = fs::remove_dir_all(&jdir);
    }
}
