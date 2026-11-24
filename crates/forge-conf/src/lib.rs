#![forbid(unsafe_code)]

//! Unified compositor, terminal & GTK4 config generator (Blueprint 6.4 + ADR 0002).

pub mod emit_foot;
pub mod emit_gtk;
pub mod emit_labwc;
pub mod emit_sway;
pub mod keymap;

use std::fs;
use std::path::Path;

use keymap::Keymap;

pub fn write_all(keymap: &Keymap, config_root: &Path) -> Result<(), String> {
    let labwc_dir = config_root.join("labwc");
    let sway_dir = config_root.join("sway");
    let gtk4_dir = config_root.join("gtk-4.0");
    let gtk3_dir = config_root.join("gtk-3.0");
    let foot_dir = config_root.join("foot");

    for d in [&labwc_dir, &sway_dir, &gtk4_dir, &gtk3_dir, &foot_dir] {
        fs::create_dir_all(d).map_err(|e| e.to_string())?;
    }

    fs::write(labwc_dir.join("rc.xml"), emit_labwc::emit_rc_xml(keymap))
        .map_err(|e| e.to_string())?;
    fs::write(labwc_dir.join("autostart"), emit_labwc::emit_autostart())
        .map_err(|e| e.to_string())?;
    fs::write(sway_dir.join("config"), emit_sway::emit_config(keymap))
        .map_err(|e| e.to_string())?;
    fs::write(gtk4_dir.join("gtk.css"), emit_gtk::emit_gtk_css()).map_err(|e| e.to_string())?;
    fs::write(
        gtk4_dir.join("settings.ini"),
        emit_gtk::emit_gtk_settings_ini(),
    )
    .map_err(|e| e.to_string())?;
    fs::write(
        gtk3_dir.join("settings.ini"),
        emit_gtk::emit_gtk_settings_ini(),
    )
    .map_err(|e| e.to_string())?;
    fs::write(foot_dir.join("foot.ini"), emit_foot::emit_foot_ini()).map_err(|e| e.to_string())?;
    Ok(())
}
