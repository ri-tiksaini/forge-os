#![deny(unsafe_op_in_unsafe_fn)]

//! Socket-activated Forge OS catalog & transaction daemon (Blueprint 6.5.5).

mod ffi;
mod parking;
mod policy;

use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use forge_catalog::{Activation, CatalogIndex};
use forge_proto::{
    escape_json, extract_bool, extract_str, extract_str_array, extract_u64, Request, Response,
    RpcError, RpcErrorCode,
};
use forge_txn::{
    apply_with_journal, plan, rollback_steps, scan_incomplete_journals, StepExecutor,
    SystemExecutor, TxnPlan,
};

pub struct DaemonConfig {
    pub socket_path: PathBuf,
    pub catalog_dir: PathBuf,
    pub journal_dir: PathBuf,
    pub idle_timeout_secs: u64,
    pub dry_run: bool,
}

struct DryRunExecutor;

impl StepExecutor for DryRunExecutor {
    fn run(&mut self, argv: &[String]) -> Result<String, String> {
        Ok(format!("dry-run: {}", argv.join(" ")))
    }
}

pub struct DaemonState {
    pub catalog: CatalogIndex,
    pub installed: HashSet<String>,
    pub active_services: HashSet<String>,
    pub plans: HashMap<String, TxnPlan>,
    pub applied: HashMap<String, (TxnPlan, usize)>,
    pub journal_dir: PathBuf,
    pub dry_run: bool,
    pub device_tier: u8,
}

impl DaemonState {
    pub fn load(cfg: &DaemonConfig) -> Result<Self, String> {
        policy::verify_catalog_integrity(&cfg.catalog_dir)?;
        let entries = forge_catalog::load_dir(&cfg.catalog_dir)?;
        let catalog = CatalogIndex::build(entries);
        let tier_report = forge_tier::detect();
        let installed = if cfg.dry_run {
            HashSet::new()
        } else {
            parking::detect_installed_apt(&catalog)
        };
        Ok(Self {
            catalog,
            installed,
            active_services: HashSet::new(),
            plans: HashMap::new(),
            applied: HashMap::new(),
            journal_dir: cfg.journal_dir.clone(),
            dry_run: cfg.dry_run,
            device_tier: tier_report.tier.as_u8(),
        })
    }

    pub fn handle(&mut self, req: Request) -> Response {
        match req.method.as_str() {
            "catalog.search" => self.handle_search(req.id, &req.params_json),
            "catalog.get" => self.handle_get(req.id, &req.params_json),
            "catalog.cost" => self.handle_cost(req.id, &req.params_json),
            "txn.plan" => self.handle_plan(req.id, &req.params_json),
            "txn.apply" => self.handle_apply(req.id, &req.params_json),
            "txn.rollback" => self.handle_rollback(req.id, &req.params_json),
            "system.state" => self.handle_state(req.id),
            "service.enable" => self.handle_service(req.id, &req.params_json, true),
            "service.disable" => self.handle_service(req.id, &req.params_json, false),
            other => Response::Err {
                id: req.id,
                error: RpcError::new(
                    RpcErrorCode::MethodNotFound,
                    format!("unknown method: {other}"),
                ),
            },
        }
    }

    fn handle_search(&self, id: u64, params: &str) -> Response {
        let query = extract_str(params, "query").unwrap_or_default();
        let limit = extract_u64(params, "limit").unwrap_or(12) as usize;
        let hits = self
            .catalog
            .search(&query, limit, self.device_tier, &self.installed);

        let items: Vec<String> = hits
            .iter()
            .filter_map(|h| {
                let e = self.catalog.get(&h.id)?;
                let status = if self.installed.contains(&e.id) {
                    if e.activation == Activation::OnDemand
                        && !self.active_services.contains(&e.id)
                    {
                        "parked"
                    } else {
                        "installed"
                    }
                } else {
                    "available"
                };
                Some(format!(
                    "{{\"id\":\"{}\",\"name\":\"{}\",\"category\":\"{}\",\"disk_mb\":{},\"ram_idle_mb\":{},\"ram_active_mb\":{},\"status\":\"{}\",\"score\":{}}}",
                    escape_json(&e.id),
                    escape_json(&e.name),
                    escape_json(&e.category),
                    e.disk_mb,
                    e.ram_idle_mb,
                    e.ram_active_mb,
                    status,
                    h.score
                ))
            })
            .collect();

        Response::Ok {
            id,
            result_json: format!("[{}]", items.join(",")),
        }
    }

    fn handle_get(&self, id: u64, params: &str) -> Response {
        let Some(entry_id) = extract_str(params, "id") else {
            return Response::Err {
                id,
                error: RpcError::new(RpcErrorCode::InvalidParams, "missing id"),
            };
        };
        let Some(e) = self.catalog.get(&entry_id) else {
            return Response::Err {
                id,
                error: RpcError::new(RpcErrorCode::NotFound, format!("not found: {entry_id}")),
            };
        };
        let json = format!(
            "{{\"id\":\"{}\",\"name\":\"{}\",\"summary\":\"{}\",\"backend\":\"{}\",\"activation\":\"{}\",\"disk_mb\":{},\"ram_idle_mb\":{},\"ram_active_mb\":{},\"beginner_note\":\"{}\"}}",
            escape_json(&e.id),
            escape_json(&e.name),
            escape_json(&e.summary),
            e.backend.as_str(),
            e.activation.as_str(),
            e.disk_mb,
            e.ram_idle_mb,
            e.ram_active_mb,
            escape_json(&e.beginner_note)
        );
        Response::Ok {
            id,
            result_json: json,
        }
    }

    fn handle_cost(&self, id: u64, params: &str) -> Response {
        let ids = extract_str_array(params, "ids");
        match plan(&ids, false, &self.catalog, &self.installed) {
            Ok(p) => Response::Ok {
                id,
                result_json: format!(
                    "{{\"disk_mb\":{},\"peak_ram_mb\":{},\"step_count\":{}}}",
                    p.disk_mb,
                    p.peak_ram_mb,
                    p.steps.len()
                ),
            },
            Err(msg) => Response::Err {
                id,
                error: RpcError::new(RpcErrorCode::NotFound, msg),
            },
        }
    }

    fn handle_plan(&mut self, id: u64, params: &str) -> Response {
        let ids = extract_str_array(params, "ids");
        let remove = extract_bool(params, "remove").unwrap_or(false);
        match plan(&ids, remove, &self.catalog, &self.installed) {
            Ok(p) => {
                let res = format!(
                    "{{\"token\":\"{}\",\"disk_mb\":{},\"peak_ram_mb\":{},\"step_count\":{}}}",
                    escape_json(&p.token),
                    p.disk_mb,
                    p.peak_ram_mb,
                    p.steps.len()
                );
                self.plans.insert(p.token.clone(), p);
                Response::Ok {
                    id,
                    result_json: res,
                }
            }
            Err(msg) => Response::Err {
                id,
                error: RpcError::new(RpcErrorCode::NotFound, msg),
            },
        }
    }

    fn handle_apply(&mut self, id: u64, params: &str) -> Response {
        let Some(token) = extract_str(params, "token") else {
            return Response::Err {
                id,
                error: RpcError::new(RpcErrorCode::InvalidParams, "missing token"),
            };
        };
        let Some(p) = self.plans.remove(&token) else {
            return Response::Err {
                id,
                error: RpcError::new(RpcErrorCode::NotFound, format!("unknown token: {token}")),
            };
        };

        let result = if self.dry_run {
            let mut exec = DryRunExecutor;
            apply_with_journal(&p, &self.journal_dir, &mut exec)
        } else {
            let mut exec = SystemExecutor;
            apply_with_journal(&p, &self.journal_dir, &mut exec)
        };

        match result {
            Ok(completed) => {
                for step in &p.steps {
                    for eid in &step.entry_ids {
                        if p.remove {
                            self.installed.remove(eid);
                            self.active_services.remove(eid);
                        } else {
                            self.installed.insert(eid.clone());
                            if !self.dry_run {
                                match eid.as_str() {
                                    "postgres" => {
                                        let _ = parking::park_postgres();
                                    }
                                    "redis" => {
                                        let _ = parking::park_redis();
                                    }
                                    "mariadb" => {
                                        let _ = parking::park_mariadb();
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                }
                self.applied.insert(token.clone(), (p, completed));
                Response::Ok {
                    id,
                    result_json: format!(
                        "{{\"token\":\"{}\",\"status\":\"completed\",\"steps\":{completed}}}",
                        escape_json(&token)
                    ),
                }
            }
            Err((completed, err)) => {
                self.applied.insert(token.clone(), (p, completed));
                Response::Err {
                    id,
                    error: RpcError::new(RpcErrorCode::BackendFailed, err),
                }
            }
        }
    }

    fn handle_rollback(&mut self, id: u64, params: &str) -> Response {
        let Some(token) = extract_str(params, "token") else {
            return Response::Err {
                id,
                error: RpcError::new(RpcErrorCode::InvalidParams, "missing token"),
            };
        };
        let Some((p, completed)) = self.applied.remove(&token) else {
            return Response::Err {
                id,
                error: RpcError::new(
                    RpcErrorCode::RollbackUnavailable,
                    format!("no rollback record for {token}"),
                ),
            };
        };

        let res = if self.dry_run {
            let mut exec = DryRunExecutor;
            rollback_steps(&p, completed, &self.journal_dir, &mut exec)
        } else {
            let mut exec = SystemExecutor;
            rollback_steps(&p, completed, &self.journal_dir, &mut exec)
        };

        match res {
            Ok(()) => {
                for step in &p.steps {
                    for eid in &step.entry_ids {
                        if p.remove {
                            self.installed.insert(eid.clone());
                        } else {
                            self.installed.remove(eid);
                        }
                    }
                }
                Response::Ok {
                    id,
                    result_json: format!(
                        "{{\"token\":\"{}\",\"status\":\"rolled_back\"}}",
                        escape_json(&token)
                    ),
                }
            }
            Err(err) => Response::Err {
                id,
                error: RpcError::new(RpcErrorCode::BackendFailed, err),
            },
        }
    }

    fn handle_state(&self, id: u64) -> Response {
        let mut installed: Vec<&String> = self.installed.iter().collect();
        installed.sort();
        let inst_json: Vec<String> = installed
            .iter()
            .map(|s| format!("\"{}\"", escape_json(s)))
            .collect();

        let parked: Vec<String> = installed
            .iter()
            .filter(|s| {
                self.catalog
                    .get(s)
                    .is_some_and(|e| e.activation == Activation::OnDemand)
                    && !self.active_services.contains(s.as_str())
            })
            .map(|s| format!("\"{}\"", escape_json(s)))
            .collect();

        let incomplete = scan_incomplete_journals(&self.journal_dir);
        let inc_json: Vec<String> = incomplete
            .iter()
            .map(|s| format!("\"{}\"", escape_json(s)))
            .collect();

        Response::Ok {
            id,
            result_json: format!(
                "{{\"tier\":{},\"installed\":[{}],\"parked\":[{}],\"incomplete_journals\":[{}]}}",
                self.device_tier,
                inst_json.join(","),
                parked.join(","),
                inc_json.join(",")
            ),
        }
    }

    fn handle_service(&mut self, id: u64, params: &str, enable: bool) -> Response {
        let Some(svc_id) = extract_str(params, "id") else {
            return Response::Err {
                id,
                error: RpcError::new(RpcErrorCode::InvalidParams, "missing id"),
            };
        };
        if self.catalog.get(&svc_id).is_none() {
            return Response::Err {
                id,
                error: RpcError::new(RpcErrorCode::NotFound, format!("unknown service: {svc_id}")),
            };
        }
        if enable {
            self.active_services.insert(svc_id.clone());
        } else {
            self.active_services.remove(&svc_id);
        }
        Response::Ok {
            id,
            result_json: format!(
                "{{\"id\":\"{}\",\"active\":{}}}",
                escape_json(&svc_id),
                enable
            ),
        }
    }
}

pub fn serve(cfg: DaemonConfig) -> Result<(), String> {
    let mut state = DaemonState::load(&cfg)?;
    let (listener, systemd_activated) = match ffi::take_systemd_socket() {
        Some(l) => (l, true),
        None => {
            if let Some(parent) = cfg.socket_path.parent() {
                fs::create_dir_all(parent).map_err(|e| format!("create socket dir: {e}"))?;
            }
            if Path::new(&cfg.socket_path).exists() {
                let _ = fs::remove_file(&cfg.socket_path);
            }
            let l = UnixListener::bind(&cfg.socket_path)
                .map_err(|e| format!("bind {}: {e}", cfg.socket_path.display()))?;
            (l, false)
        }
    };
    listener
        .set_nonblocking(true)
        .map_err(|e| format!("nonblocking: {e}"))?;
    let idle_limit = Duration::from_secs(cfg.idle_timeout_secs);
    let mut last_activity = Instant::now();
    loop {
        match listener.accept() {
            Ok((stream, _)) => {
                last_activity = Instant::now();
                let _ = handle_client(stream, &mut state);
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                if last_activity.elapsed() >= idle_limit {
                    break;
                }
                std::thread::sleep(Duration::from_millis(25));
            }
            Err(e) => return Err(format!("accept error: {e}")),
        }
    }
    if !systemd_activated {
        let _ = fs::remove_file(&cfg.socket_path);
    }
    Ok(())
}

fn handle_client(stream: UnixStream, state: &mut DaemonState) -> Result<(), String> {
    let peer = policy::peer_cred(&stream).ok();
    stream.set_nonblocking(false).map_err(|e| e.to_string())?;
    let mut writer = stream.try_clone().map_err(|e| e.to_string())?;
    let reader = BufReader::new(stream);

    for line in reader.lines() {
        let line = line.map_err(|e| e.to_string())?;
        if line.trim().is_empty() {
            continue;
        }
        let resp = match Request::parse_line(&line) {
            Ok(req) => {
                let is_mut = matches!(
                    req.method.as_str(),
                    "txn.apply" | "txn.rollback" | "service.enable" | "service.disable"
                );
                if is_mut && !state.dry_run {
                    if let Some(ref p) = peer {
                        if let Err(msg) = policy::authorize_mutation(p) {
                            let err = Response::Err {
                                id: req.id,
                                error: RpcError::new(RpcErrorCode::PermissionDenied, msg),
                            };
                            writeln!(writer, "{}", err.to_line()).map_err(|e| e.to_string())?;
                            writer.flush().map_err(|e| e.to_string())?;
                            continue;
                        }
                    }
                }
                state.handle(req)
            }
            Err(err) => Response::Err { id: 0, error: err },
        };
        writeln!(writer, "{}", resp.to_line()).map_err(|e| e.to_string())?;
        writer.flush().map_err(|e| e.to_string())?;
    }
    Ok(())
}
