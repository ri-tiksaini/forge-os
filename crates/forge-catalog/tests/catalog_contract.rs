#![forbid(unsafe_code)]

use std::collections::HashSet;
use std::path::Path;
use std::time::Instant;

use forge_catalog::{
    load_dir, validate_catalog, Activation, Backend, CatalogEntry, CatalogIndex, ParkingMode,
};

#[test]
fn starter_catalog_validates_and_searches() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/catalog");
    let entries = load_dir(&dir).expect("starter catalog must validate cleanly");
    assert!(entries.len() >= 8);

    let index = CatalogIndex::build(entries);
    let installed = HashSet::new();

    // Exact + prefix search
    let hits = index.search("postg", 12, 0, &installed);
    assert_eq!(hits.first().map(|h| h.id.as_str()), Some("postgres"));

    // Typo-tolerant fuzzy fallback (distance <= 2)
    let fuzzy = index.search("postgrs", 12, 0, &installed);
    assert_eq!(fuzzy.first().map(|h| h.id.as_str()), Some("postgres"));

    // O(log n) lookup by id
    let pg = index.get("postgres").expect("postgres entry");
    assert_eq!(pg.ram_idle_mb, 0);
    assert_eq!(pg.activation, Activation::OnDemand);
    assert_eq!(pg.parking_mode, ParkingMode::SocketProxy);
}

#[test]
fn dag_validator_rejects_cycles() {
    let mut a = sample_entry("alpha");
    a.depends = vec!["beta".to_string()];
    let mut b = sample_entry("beta");
    b.depends = vec!["alpha".to_string()];

    let err = validate_catalog(&[a, b]).unwrap_err();
    assert!(err.contains("cycle"), "unexpected error: {err}");
}

#[test]
fn search_5000_entries_meets_20ms_tier0_budget_and_is_deterministic() {
    let mut entries = Vec::with_capacity(5000);
    for i in 0..4999 {
        entries.push(sample_entry(&format!("pkg-{i:04}")));
    }
    let mut pg = sample_entry("postgres");
    pg.name = "PostgreSQL".to_string();
    pg.keywords = vec!["sql".to_string(), "database".to_string()];
    entries.push(pg);

    let index = CatalogIndex::build(entries);
    let installed = HashSet::new();

    let t0 = Instant::now();
    let cold = index.search("postg", 12, 0, &installed);
    let cold_ms = t0.elapsed().as_millis();

    let t1 = Instant::now();
    let warm = index.search("postg", 12, 0, &installed);
    let warm_ms = t1.elapsed().as_millis();

    assert_eq!(cold, warm, "search results must be deterministic");
    assert_eq!(cold.first().map(|h| h.id.as_str()), Some("postgres"));
    assert!(
        cold_ms <= 20 && warm_ms <= 20,
        "search exceeded 20 ms budget: cold={cold_ms}ms warm={warm_ms}ms"
    );
}

fn sample_entry(id: &str) -> CatalogEntry {
    CatalogEntry {
        id: id.to_string(),
        name: format!("Tool {id}"),
        category: "tool".to_string(),
        summary: "Synthetic test entry".to_string(),
        keywords: vec!["dev".to_string(), "cli".to_string()],
        backend: Backend::Apt,
        packages: vec![id.to_string()],
        activation: Activation::Manual,
        parking_mode: ParkingMode::None,
        autostart: false,
        disk_mb: 25,
        ram_idle_mb: 0,
        ram_active_mb: 32,
        tier_min: 0,
        license: "MIT".to_string(),
        popularity: 50,
        depends: Vec::new(),
        conflicts: Vec::new(),
        gui_entry: false,
        cli_entry: vec![id.to_string()],
        beginner_note: "Test".to_string(),
        heavy_warning: String::new(),
    }
}
