#![forbid(unsafe_code)]

use forge_conf::emit_foot::emit_foot_ini;
use forge_conf::emit_gtk::emit_gtk_css;
use forge_conf::emit_labwc::{emit_autostart, emit_rc_xml};
use forge_conf::emit_sway::emit_config;
use forge_conf::keymap::Keymap;

const KEYMAP_SRC: &str = include_str!("../../../data/keymap.toml");

#[test]
fn bind_parity_across_labwc_and_sway() {
    let km = Keymap::parse_toml(KEYMAP_SRC).expect("valid keymap.toml");
    let labwc = emit_rc_xml(&km);
    let sway = emit_config(&km);

    let shared_commands = [
        "forge-settings",
        "foot",
        "thunar",
        "firefox-esr",
        "swaylock",
    ];
    for cmd in shared_commands {
        assert!(labwc.contains(cmd), "labwc missing command {cmd}");
        assert!(sway.contains(cmd), "sway missing command {cmd}");
    }

    assert!(labwc.contains("forge-toggle launcher"));
    assert!(labwc.contains("A-Tab") && sway.contains("Mod1+Tab"));
    assert!(labwc.contains("A-F4") && sway.contains("Mod1+F4"));
    assert!(labwc.contains("iconify,maximize,close"));

    for ws in 1..=9 {
        assert!(labwc.contains(&format!("GoToDesktop\" to=\"{ws}\"")));
        assert!(sway.contains(&format!("workspace number {ws}")));
    }
}

#[test]
fn output_is_deterministic_and_cherry_branded() {
    let km = Keymap::parse_toml(KEYMAP_SRC).expect("valid keymap.toml");
    assert_eq!(emit_rc_xml(&km), emit_rc_xml(&km));
    assert_eq!(emit_config(&km), emit_config(&km));

    let css = emit_gtk_css();
    assert!(css.contains(forge_brand::ACCENT));
    assert!(css.contains(forge_brand::BG));

    let foot = emit_foot_ini();
    assert!(foot.contains("Control+v"));
    assert!(foot.contains("9B1126"));

    let autostart = emit_autostart();
    assert!(autostart.contains("waybar"));
}
