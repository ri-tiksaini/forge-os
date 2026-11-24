#![forbid(unsafe_code)]

use std::collections::HashSet;
use std::path::Path;
use std::time::Instant;

use forge_catalog::{load_dir, CatalogIndex};
use forge_shell::launcher::{LauncherEngine, LauncherState};
use forge_shell::notify::NotificationQueue;
use forge_shell::pill::PillSnapshot;

#[test]
fn launcher_state_machine_and_progressive_cli_disclosure() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/catalog");
    let index = CatalogIndex::build(load_dir(&dir).expect("catalog"));
    let mut engine = LauncherEngine::new(index, 0, HashSet::new());

    assert_eq!(engine.state, LauncherState::Hidden);
    engine.open();
    assert_eq!(engine.state, LauncherState::Ready);

    let rows = engine.query_rows("postg");
    let top = rows.first().expect("expected postgres result");
    assert_eq!(top.title, "PostgreSQL");
    assert_eq!(top.cli_hint, "dev add postgres");

    engine.close();
    assert_eq!(engine.state, LauncherState::Hidden);
}

#[test]
fn launcher_1000_keystroke_latency_gate_under_8ms_p95() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/catalog");
    let index = CatalogIndex::build(load_dir(&dir).expect("catalog"));
    let mut engine = LauncherEngine::new(index, 0, HashSet::new());
    engine.open();

    let queries = [
        "p", "po", "pos", "post", "postg", "neo", "web", "red", "term", "rust",
    ];
    let mut samples_us = Vec::with_capacity(1000);
    for i in 0..1000 {
        let q = queries[i % queries.len()];
        let t0 = Instant::now();
        let rows = engine.query_rows(q);
        let elapsed_us = t0.elapsed().as_micros();
        assert!(!rows.is_empty());
        samples_us.push(elapsed_us);
    }
    samples_us.sort_unstable();
    let p95_us = samples_us[949];
    assert!(
        p95_us <= 8_000,
        "launcher p95 query latency {p95_us}us exceeded 8000us (8ms) budget"
    );
}

#[test]
fn pill_snapshot_and_notification_queue_work() {
    let snap = PillSnapshot::sample("T0");
    assert!(snap.format_compact().contains("CPU"));
    assert!(snap.format_compact().contains("RAM"));

    let mut q = NotificationQueue::default();
    let id = q.push("PostgreSQL", "Parked at 0 MB RAM", 60);
    assert_eq!(id, 1);
    assert_eq!(q.active().len(), 1);
    q.prune_expired();
    assert_eq!(q.active().len(), 1);
}
