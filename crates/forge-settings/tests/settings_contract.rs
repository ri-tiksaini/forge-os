#![forbid(unsafe_code)]

use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use forge_catalog::{load_dir, CatalogIndex};
use forge_settings::model::{create_project, parked_services, query_store};

#[test]
fn store_and_parked_services_views_reflect_catalog() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/catalog");
    let index = CatalogIndex::build(load_dir(&dir).expect("catalog"));

    let items = query_store(&index, "postg", 0);
    assert_eq!(items.first().map(|i| i.id.as_str()), Some("postgres"));
    assert_eq!(items[0].ram_idle_mb, 0);

    let parked = parked_services(&index);
    assert!(parked.iter().any(|s| s.id == "postgres"));
    assert!(parked.iter().any(|s| s.id == "redis"));
}

#[test]
fn project_wizard_scaffolds_web_and_rejects_traversal() {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let base = std::env::temp_dir().join(format!("forge-proj-test-{ts}"));

    assert!(create_project(&base, "web", "../escape").is_err());

    let proj = create_project(&base, "web", "hello-web").expect("create project");
    assert!(proj.join("README.md").exists());
    assert!(proj.join("package.json").exists());
    assert!(proj.join("index.js").exists());

    let _ = std::fs::remove_dir_all(&base);
}
