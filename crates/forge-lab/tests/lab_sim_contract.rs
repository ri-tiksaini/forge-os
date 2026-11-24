#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::process::Command;

use forge_lab::sim::{
    build_apply_script, default_core_files, is_allowed_core_path, run_virtual_os_simulation,
};

#[test]
fn developer_labs_never_allows_user_home_or_shell_metachars() {
    assert!(is_allowed_core_path("/usr/local/bin/my-tool"));
    assert!(is_allowed_core_path("/usr/share/forge/waybar/config"));
    assert!(is_allowed_core_path(
        "/usr/src/forge-os/crates/forge-settings/src/ui.rs"
    ));
    assert!(!is_allowed_core_path("/home/forge/secret.txt"));
    assert!(!is_allowed_core_path("/etc/../home/forge/x"));
    assert!(!is_allowed_core_path("/usr/local/bin/bad;rm -rf /"));
    assert!(!is_allowed_core_path("/usr/local/bin/bad$(id)"));
}

#[test]
fn apply_script_uses_bash_with_valid_err_trap_and_created_file_cleanup() {
    let items = vec![
        (
            "/usr/local/bin/custom-tool".to_string(),
            PathBuf::from("/tmp/item_0"),
        ),
        (
            "/etc/sysctl.d/99-forge.conf".to_string(),
            PathBuf::from("/tmp/item_1"),
        ),
    ];
    let script = build_apply_script(&items, 12345).expect("script should generate");
    assert!(script.starts_with("#!/usr/bin/env bash"));
    assert!(script.contains("trap rollback ERR INT TERM"));
    assert!(script.contains("CREATED=\"$BK/created.list\""));
    assert!(script.contains("sysctl --system"));

    let tmp = std::env::temp_dir().join("forge-lab-test-apply.sh");
    std::fs::write(&tmp, &script).unwrap();
    let out = Command::new("bash")
        .args(["-n"])
        .arg(&tmp)
        .output()
        .unwrap();
    let _ = std::fs::remove_file(&tmp);
    assert!(
        out.status.success(),
        "generated apply script failed bash -n: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn full_core_catalog_covers_scripts_ui_systemd_and_rust_with_main_hubs() {
    let files = default_core_files();
    assert!(
        files.len() >= 55,
        "expected full core catalog, got {}",
        files.len()
    );
    let hubs = files.iter().filter(|f| f.is_main_hub).count();
    assert!(
        hubs >= 10,
        "expected highlighted main hub files, got {hubs}"
    );
}

#[test]
fn virtual_os_simulation_strictly_catches_errors_across_file_types() {
    let mut bad_bash = BTreeMap::new();
    bad_bash.insert(
        "/usr/local/bin/broken-tool".to_string(),
        "#!/bin/bash\nif [ 1 -eq 1 ]; then\n".to_string(),
    );
    let r = run_virtual_os_simulation(&bad_bash);
    assert!(!r.passed);
    let f = r
        .checks
        .iter()
        .find(|c| !c.passed)
        .expect("expected failed bash check");
    assert!(!f.reason.is_empty() && !f.fix_hint.is_empty());

    let mut bad_css = BTreeMap::new();
    bad_css.insert(
        "/usr/share/forge/waybar/style.css".to_string(),
        "window {\n  broken_gibberish_without_colon\n}\n".to_string(),
    );
    assert!(!run_virtual_os_simulation(&bad_css).passed);

    let mut bad_toml = BTreeMap::new();
    bad_toml.insert(
        "/usr/share/forge/brand.toml".to_string(),
        "[brand\nname = 123 = 456\n".to_string(),
    );
    assert!(!run_virtual_os_simulation(&bad_toml).passed);

    let mut good = BTreeMap::new();
    good.insert(
        "/usr/local/bin/custom-tool".to_string(),
        "#!/usr/bin/env bash\necho ok\n".to_string(),
    );
    good.insert(
        "/usr/share/forge/waybar/config".to_string(),
        "{\"layer\":\"top\",\"position\":\"bottom\"}\n".to_string(),
    );
    good.insert(
        "/usr/share/forge/waybar/style.css".to_string(),
        "window {\n  color: #ffffff;\n}\n".to_string(),
    );
    let r_good = run_virtual_os_simulation(&good);
    assert!(r_good.passed, "logs: {:?}", r_good.logs);
    assert!(r_good.checks.iter().all(|c| c.passed));
}
