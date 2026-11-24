#![forbid(unsafe_code)]

//! Sleek Cherry-Red & Obsidian-Black GTK4 Shell:
//! - `welcome`: First-screen Forge OS Solid Black Duck logo window
//! - `taskbar`: Bottom Windows-familiar bar with Duck Start Button, pinned app icons, live telemetry & Control Center
//! - `launcher`: Animated radial/staggered App Burst Grid + live catalog search

use std::cell::RefCell;
use std::process::Command;
use std::rc::Rc;
use std::time::{Duration, Instant};

use gtk4::gdk;
use gtk4::glib;
use gtk4::prelude::*;
use gtk4::{
    Align, Application, ApplicationWindow, Box as GtkBox, Button, CssProvider, EventControllerKey,
    FlowBox, Image, Label, ListBox, ListBoxRow, Orientation, Popover, Scale, ScrolledWindow,
    SearchEntry, SelectionMode, Separator, Switch,
};
#[cfg(feature = "layer-shell")]
use gtk4_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};

use crate::launcher::{LauncherEngine, LauncherRow};
use crate::pill::{CpuPrev, NetPrev, PillSnapshot};

const DUCK_SVG: &str = include_str!("../../../branding/logo.svg");
const MONITOR_SVG: &str = include_str!("../../../data/theme/icons/forge-monitor.svg");
const CLEAN_SVG: &str = include_str!("../../../data/theme/icons/forge-clean.svg");
const TERM_SVG: &str = include_str!("../../../data/theme/icons/forge-terminal.svg");
const EDITOR_SVG: &str = include_str!("../../../data/theme/icons/forge-editor.svg");
const FILES_SVG: &str = include_str!("../../../data/theme/icons/forge-files.svg");
const BROWSER_SVG: &str = include_str!("../../../data/theme/icons/forge-browser.svg");
const SETTINGS_SVG: &str = include_str!("../../../data/theme/icons/forge-settings.svg");
const HELP_SVG: &str = include_str!("../../../data/theme/icons/forge-help.svg");
const LAB_SVG: &str = include_str!("../../../data/theme/icons/forge-lab.svg");

fn load_active_theme() -> (String, String, String) {
    let mut bg = forge_brand::BG.to_string();
    let mut accent = forge_brand::ACCENT.to_string();
    let mut accent_hi = "#C42B42".to_string();
    if let Ok(home) = std::env::var("HOME") {
        let p = std::path::PathBuf::from(home).join(".config/forge/active-theme.env");
        if let Ok(txt) = std::fs::read_to_string(p) {
            for line in txt.lines() {
                if let Some(v) = line.strip_prefix("BG=") {
                    bg = v.trim().to_string();
                } else if let Some(v) = line.strip_prefix("ACCENT=") {
                    accent = v.trim().to_string();
                } else if let Some(v) = line.strip_prefix("ACCENT_HI=") {
                    accent_hi = v.trim().to_string();
                }
            }
        }
    }
    (bg, accent, accent_hi)
}

fn build_css() -> String {
    let (bg, accent, accent_hi) = load_active_theme();
    let fg = forge_brand::FG;
    let ui = forge_brand::FONT_UI;
    let mono = forge_brand::FONT_MONO;
    format!(
        "window {{ background-color: {bg}; color: {fg}; font-family: '{ui}'; }}\n\
         button {{ border: 1px solid {accent}; border-radius: 10px; }}\n\
         button:hover {{ border-color: {accent_hi}; }}\n\
         .forge-bar {{ background-color: rgba(9, 9, 11, 0.94); border-top: 2px solid {accent}; padding: 6px 14px; }}\n\
         .forge-duck-btn {{ background-color: {accent}; border-radius: 12px; padding: 4px 10px; border: 1px solid {accent_hi}; }}\n\
         .forge-duck-btn:hover {{ background-color: {accent_hi}; }}\n\
         .forge-pin-btn {{ background-color: #16161C; border: 1px solid {accent}; border-radius: 10px; padding: 6px 10px; color: {fg}; }}\n\
         .forge-pin-btn:hover {{ background-color: #261218; border-color: {accent_hi}; }}\n\
         .forge-tray-btn {{ background-color: #141419; border: 1px solid {accent}; border-radius: 10px; padding: 6px 12px; font-family: '{mono}'; font-size: 11px; color: {fg}; }}\n\
         .forge-tray-btn:hover {{ border-color: {accent_hi}; }}\n\
         .forge-launcher {{ background-color: {bg}; border: 2px solid {accent}; border-radius: 18px; padding: 18px; }}\n\
         .forge-search {{ background-color: #17171E; color: {fg}; border: 1px solid {accent}; border-radius: 12px; font-size: 15px; min-height: 44px; padding: 4px 12px; }}\n\
         .forge-tile {{ background-color: #16161D; border: 1px solid {accent}; border-radius: 14px; padding: 12px; min-width: 110px; min-height: 92px; }}\n\
         .forge-tile:hover {{ background-color: #240B10; border-color: {accent_hi}; }}\n\
         .forge-row {{ padding: 6px 10px; border-radius: 8px; }}\n\
         list row:selected {{ background-color: #240B10; border-left: 3px solid {accent}; }}\n\
         .forge-title {{ font-size: 14px; font-weight: 700; color: {fg}; }}\n\
         .forge-meta {{ font-size: 12px; color: #A1A1AA; }}\n\
         .forge-cli {{ font-family: '{mono}'; font-size: 11px; color: {accent_hi}; }}\n\
         .forge-card {{ background-color: #141419; border: 1px solid {accent}; border-radius: 14px; padding: 14px; }}\n\
         .forge-accent-btn {{ background-color: {accent}; color: #FFFFFF; font-weight: 700; border-radius: 10px; padding: 8px 16px; border: 1px solid {accent_hi}; }}\n\
         .forge-welcome {{ background-color: {bg}; border: 2px solid {accent}; border-radius: 20px; padding: 28px; }}\n\
         scale trough {{ background-color: #22161A; border-radius: 6px; min-height: 8px; }}\n\
         scale highlight {{ background-color: {accent}; border-radius: 6px; }}\n\
         scale slider {{ background-color: #ECECEF; border: 2px solid {accent}; border-radius: 50%; min-width: 18px; min-height: 18px; }}\n\
         switch {{ background-color: #22161A; border: 1px solid {accent}; border-radius: 14px; }}\n\
         switch:checked {{ background-color: {accent}; border-color: {accent_hi}; }}\n"
    )
}

fn load_css() {
    let provider = CssProvider::new();
    provider.load_from_data(&build_css());
    if let Some(display) = gdk::Display::default() {
        gtk4::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }
}

fn svg_image(svg: &'static str, size: i32, fallback_icon: &str) -> Image {
    let bytes = glib::Bytes::from_static(svg.as_bytes());
    let stream = gtk4::gio::MemoryInputStream::from_bytes(&bytes);
    if let Ok(pixbuf) = gtk4::gdk_pixbuf::Pixbuf::from_stream_at_scale(
        &stream,
        size,
        size,
        true,
        gtk4::gio::Cancellable::NONE,
    ) {
        let tex = gdk::Texture::for_pixbuf(&pixbuf);
        let img = Image::from_paintable(Some(&tex));
        img.set_pixel_size(size);
        return img;
    }
    Image::from_icon_name(fallback_icon)
}

fn duck_image(size: i32) -> Image {
    svg_image(DUCK_SVG, size, "start-here-symbolic")
}

/// First-screen Forge OS Logo Window (shows our solid black duck logo + quick launch buttons).
pub fn run_welcome() {
    let app = Application::builder()
        .application_id("org.forge.shell.welcome")
        .build();

    app.connect_activate(|app| {
        load_css();
        let window = ApplicationWindow::builder()
            .application(app)
            .title(forge_brand::NAME)
            .default_width(420)
            .default_height(320)
            .decorated(false)
            .build();

        let root = GtkBox::new(Orientation::Vertical, 14);
        root.add_css_class("forge-welcome");
        root.set_halign(Align::Center);
        root.set_valign(Align::Center);

        let logo = duck_image(96);
        let title = Label::new(Some(forge_brand::PRETTY_NAME));
        title.add_css_class("forge-title");
        let sub = Label::new(Some(
            "Click the Duck button below (or press Super) to burst open apps.",
        ));
        sub.add_css_class("forge-meta");

        let btn_row = GtkBox::new(Orientation::Horizontal, 10);
        btn_row.set_halign(Align::Center);
        let open_btn = Button::with_label("Open Apps");
        open_btn.add_css_class("forge-accent-btn");
        open_btn.connect_clicked(|_| spawn_command("forge-shell --mode launcher"));

        let settings_btn = Button::with_label("Settings");
        settings_btn.add_css_class("forge-pin-btn");
        settings_btn.connect_clicked(|_| spawn_command("forge-settings"));

        let close_btn = Button::with_label("Dismiss");
        close_btn.add_css_class("forge-pin-btn");
        let win_c = window.clone();
        close_btn.connect_clicked(move |_| win_c.close());

        btn_row.append(&open_btn);
        btn_row.append(&settings_btn);
        btn_row.append(&close_btn);

        root.append(&logo);
        root.append(&title);
        root.append(&sub);
        root.append(&btn_row);
        window.set_child(Some(&root));
        window.present();
    });

    app.run_with_args::<&str>(&[]);
}

/// Bottom Windows-familiar Taskbar with Duck Start Button, App Icons, Live Telemetry & Control Center.
pub fn run_taskbar(tier_label: String) {
    let app = Application::builder()
        .application_id("org.forge.shell.taskbar")
        .build();

    app.connect_activate(move |app| {
        load_css();
        let window = ApplicationWindow::builder()
            .application(app)
            .title(forge_brand::NAME)
            .default_width(980)
            .default_height(48)
            .decorated(false)
            .build();

        #[cfg(feature = "layer-shell")]
        if gtk4_layer_shell::is_supported() {
            window.init_layer_shell();
            window.set_layer(Layer::Top);
            window.set_anchor(Edge::Bottom, true);
            window.set_anchor(Edge::Left, true);
            window.set_anchor(Edge::Right, true);
            window.set_exclusive_zone(48);
        }

        let bar = GtkBox::new(Orientation::Horizontal, 10);
        bar.add_css_class("forge-bar");

        // 1. Duck Start Button
        let duck_btn = Button::new();
        duck_btn.add_css_class("forge-duck-btn");
        duck_btn.set_tooltip_text(Some("Forge Apps (Win / Super)"));
        let duck_box = GtkBox::new(Orientation::Horizontal, 6);
        duck_box.append(&duck_image(26));
        let duck_lbl = Label::new(Some("Apps"));
        duck_box.append(&duck_lbl);
        duck_btn.set_child(Some(&duck_box));
        duck_btn.connect_clicked(|_| spawn_command("forge-shell --mode launcher"));
        bar.append(&duck_btn);

        // 2. Pinned App Icons (Terminal, Files, Browser, Settings)
        let pins = [
            ("utilities-terminal", "Terminal", "foot"),
            ("system-file-manager", "Files", "thunar"),
            ("web-browser", "Browser", "firefox-esr"),
            ("preferences-system", "Settings", "forge-settings"),
        ];
        for (icon_name, tooltip, cmd) in pins {
            let b = Button::from_icon_name(icon_name);
            b.add_css_class("forge-pin-btn");
            b.set_tooltip_text(Some(tooltip));
            let c = cmd.to_string();
            b.connect_clicked(move |_| spawn_command(&c));
            bar.append(&b);
        }

        let spacer = GtkBox::new(Orientation::Horizontal, 0);
        spacer.set_hexpand(true);
        bar.append(&spacer);

        // 3. Live System Telemetry + Control Center Popover
        let cpu_prev = Rc::new(RefCell::new(CpuPrev::default()));
        let net_prev = Rc::new(RefCell::new(NetPrev::default()));
        let snap = PillSnapshot::sample_live(
            &tier_label,
            &mut cpu_prev.borrow_mut(),
            &mut net_prev.borrow_mut(),
        );
        let tray_btn = Button::with_label(&snap.format_compact());
        tray_btn.add_css_class("forge-tray-btn");
        tray_btn.set_tooltip_text(Some(
            "Click for Quick Control Center (Wi-Fi, Bluetooth, Sound, Settings)",
        ));

        let popover = build_control_center(&tray_btn);
        tray_btn.connect_clicked(move |_| popover.popup());
        bar.append(&tray_btn);

        let tray_c = tray_btn.clone();
        let tier_c = tier_label.clone();
        let cpu_c = cpu_prev.clone();
        let net_c = net_prev.clone();
        glib::timeout_add_local(Duration::from_secs(1), move || {
            let s = PillSnapshot::sample_live(
                &tier_c,
                &mut cpu_c.borrow_mut(),
                &mut net_c.borrow_mut(),
            );
            tray_c.set_label(&s.format_compact());
            glib::ControlFlow::Continue
        });

        window.set_child(Some(&bar));
        window.present();
    });

    app.run_with_args::<&str>(&[]);
}

fn build_control_center(parent: &Button) -> Popover {
    let pop = Popover::new();
    pop.set_parent(parent);

    let box_root = GtkBox::new(Orientation::Vertical, 10);
    box_root.add_css_class("forge-card");
    box_root.set_width_request(300);

    let title = Label::new(Some("Quick Control Center"));
    title.add_css_class("forge-title");
    title.set_halign(Align::Start);
    box_root.append(&title);

    let sys_detail = Label::new(Some(&PillSnapshot::sample("T0").format_detailed()));
    sys_detail.add_css_class("forge-meta");
    sys_detail.set_halign(Align::Start);
    box_root.append(&sys_detail);

    // Hardware toggles: Wi-Fi, Bluetooth, Dark/Cherry Theme
    for (label_txt, on_cmd, off_cmd) in [
        ("Wi-Fi Radio", "nmcli radio wifi on", "nmcli radio wifi off"),
        (
            "Bluetooth",
            "bluetoothctl power on",
            "bluetoothctl power off",
        ),
    ] {
        let row = GtkBox::new(Orientation::Horizontal, 8);
        let lbl = Label::new(Some(label_txt));
        lbl.set_hexpand(true);
        lbl.set_halign(Align::Start);
        let sw = Switch::new();
        sw.set_active(label_txt == "Wi-Fi Radio");
        let on_s = on_cmd.to_string();
        let off_s = off_cmd.to_string();
        sw.connect_state_set(move |_, active| {
            spawn_command(if active { &on_s } else { &off_s });
            glib::Propagation::Proceed
        });
        row.append(&lbl);
        row.append(&sw);
        box_root.append(&row);
    }

    // Sound quick controls
    let vol_row = GtkBox::new(Orientation::Horizontal, 6);
    let vol_lbl = Label::new(Some("Sound"));
    vol_lbl.set_hexpand(true);
    vol_lbl.set_halign(Align::Start);
    let v_down = Button::with_label("-");
    v_down.connect_clicked(|_| spawn_command("wpctl set-volume @DEFAULT_AUDIO_SINK@ 5%-"));
    let v_mute = Button::with_label("Mute");
    v_mute.connect_clicked(|_| spawn_command("wpctl set-mute @DEFAULT_AUDIO_SINK@ toggle"));
    let v_up = Button::with_label("+");
    v_up.connect_clicked(|_| spawn_command("wpctl set-volume @DEFAULT_AUDIO_SINK@ 5%+"));
    vol_row.append(&vol_lbl);
    vol_row.append(&v_down);
    vol_row.append(&v_mute);
    vol_row.append(&v_up);
    box_root.append(&vol_row);

    box_root.append(&Separator::new(Orientation::Horizontal));

    // Direct jump buttons to Settings pages
    let settings_btn =
        Button::with_label("Open Full Settings (Network, Display, Keyboard, Monitor)");
    settings_btn.add_css_class("forge-accent-btn");
    let pop_c = pop.clone();
    settings_btn.connect_clicked(move |_| {
        pop_c.popdown();
        spawn_command("forge-settings");
    });
    box_root.append(&settings_btn);

    pop.set_child(Some(&box_root));
    pop
}

/// Animated App Burst Launcher (clicks on Duck button smoothly burst tiles into an organized grid).
pub fn run_launcher(engine: LauncherEngine) {
    let app = Application::builder()
        .application_id("org.forge.shell.launcher")
        .build();

    let engine_rc = Rc::new(RefCell::new(engine));
    let rows_rc: Rc<RefCell<Vec<LauncherRow>>> = Rc::new(RefCell::new(Vec::new()));

    app.connect_activate(move |app| {
        load_css();

        let window = ApplicationWindow::builder()
            .application(app)
            .title(forge_brand::NAME)
            .default_width(720)
            .default_height(540)
            .decorated(false)
            .build();

        #[cfg(feature = "layer-shell")]
        if gtk4_layer_shell::is_supported() {
            window.init_layer_shell();
            window.set_layer(Layer::Overlay);
            window.set_keyboard_mode(KeyboardMode::Exclusive);
            window.set_anchor(Edge::Bottom, true);
            window.set_margin(Edge::Bottom, 64);
        }

        let root = GtkBox::new(Orientation::Vertical, 12);
        root.add_css_class("forge-launcher");

        // Header with Solid Black Duck + Search
        let header = GtkBox::new(Orientation::Horizontal, 10);
        header.append(&duck_image(36));
        let search = SearchEntry::new();
        search.add_css_class("forge-search");
        search.set_hexpand(true);
        search.set_placeholder_text(Some("Search apps, tools, settings..."));
        header.append(&search);
        root.append(&header);

        // Quick App Burst Grid (Terminal, Files, Browser, Settings, Software, Monitor)
        let grid = FlowBox::new();
        grid.set_max_children_per_line(4);
        grid.set_min_children_per_line(3);
        grid.set_row_spacing(10);
        grid.set_column_spacing(10);
        grid.set_selection_mode(SelectionMode::None);

        let quick_apps = [
            ("utilities-terminal", "Terminal", "foot"),
            ("accessories-text-editor", "Code Editor", "mousepad"),
            ("system-file-manager", "Files", "thunar"),
            ("web-browser", "Browser", "firefox-esr"),
            ("preferences-system", "Settings", "forge-settings"),
            (
                "forge-monitor",
                "Monitor",
                "foot -e /usr/local/bin/forge-hud",
            ),
            (
                "forge-clean",
                "Clean RAM",
                "foot -e sh -c 'sudo /usr/local/bin/forge-clean; sleep 2'",
            ),
            ("help-browser", "Helpbook", "forge-help"),
            ("forge-lab", "Developer Labs", "forge-lab"),
        ];

        let mut tile_widgets: Vec<Button> = Vec::new();
        for (icon_name, label_txt, cmd) in quick_apps {
            let tile = Button::new();
            tile.add_css_class("forge-tile");
            let vbox = GtkBox::new(Orientation::Vertical, 6);
            vbox.set_halign(Align::Center);
            vbox.set_valign(Align::Center);
            let ic = match icon_name {
                "utilities-terminal" => svg_image(TERM_SVG, 36, "utilities-terminal"),
                "accessories-text-editor" => svg_image(EDITOR_SVG, 36, "accessories-text-editor"),
                "system-file-manager" => svg_image(FILES_SVG, 36, "system-file-manager"),
                "web-browser" => svg_image(BROWSER_SVG, 36, "web-browser"),
                "preferences-system" => svg_image(SETTINGS_SVG, 36, "preferences-system"),
                "forge-monitor" => svg_image(MONITOR_SVG, 36, "utilities-system-monitor"),
                "forge-clean" => svg_image(CLEAN_SVG, 36, "edit-clear-all"),
                "help-browser" => svg_image(HELP_SVG, 36, "help-browser"),
                "forge-lab" => svg_image(LAB_SVG, 36, "applications-engineering"),
                other => {
                    let i = Image::from_icon_name(other);
                    i.set_pixel_size(34);
                    i
                }
            };
            let lbl = Label::new(Some(label_txt));
            lbl.add_css_class("forge-title");
            vbox.append(&ic);
            vbox.append(&lbl);
            tile.set_child(Some(&vbox));

            let c = cmd.to_string();
            let win_b = window.clone();
            tile.connect_clicked(move |_| {
                spawn_command(&c);
                win_b.close();
            });
            tile.set_opacity(0.0);
            grid.insert(&tile, -1);
            tile_widgets.push(tile);
        }
        root.append(&grid);

        // Staggered burst-and-settle animation over 320ms
        let start = Instant::now();
        grid.add_tick_callback(move |_, _| {
            let elapsed = start.elapsed().as_secs_f64();
            let mut all_done = true;
            for (idx, tile) in tile_widgets.iter().enumerate() {
                let delay = (idx as f64) * 0.035;
                let t = ((elapsed - delay) / 0.24).clamp(0.0, 1.0);
                // Cubic ease-out
                let eased = 1.0 - (1.0 - t).powi(3);
                tile.set_opacity(eased);
                let top_pad = ((1.0 - eased) * 16.0) as i32;
                tile.set_margin_top(top_pad);
                tile.set_margin_bottom(16 - top_pad);
                if t < 1.0 {
                    all_done = false;
                }
            }
            if all_done {
                glib::ControlFlow::Break
            } else {
                glib::ControlFlow::Continue
            }
        });

        let list = ListBox::new();
        list.set_selection_mode(SelectionMode::Single);
        let scroller = ScrolledWindow::builder()
            .vexpand(true)
            .min_content_height(210)
            .child(&list)
            .build();
        root.append(&scroller);

        let footer = Label::new(Some(
            "Click any app or type to search catalog  |  Enter run  |  Esc close",
        ));
        footer.add_css_class("forge-meta");
        footer.set_halign(Align::Start);
        root.append(&footer);

        window.set_child(Some(&root));
        populate_list(&list, &engine_rc, &rows_rc, "");

        let list_c = list.clone();
        let eng_c = engine_rc.clone();
        let rows_c = rows_rc.clone();
        search.connect_search_changed(move |entry| {
            populate_list(&list_c, &eng_c, &rows_c, &entry.text());
        });

        let win_c = window.clone();
        let list_enter = list.clone();
        let rows_enter = rows_rc.clone();
        search.connect_activate(move |_| {
            let idx = list_enter
                .selected_row()
                .map(|r| r.index() as usize)
                .unwrap_or(0);
            if let Some(item) = rows_enter.borrow().get(idx) {
                spawn_command(&item.exec_cmd);
                win_c.close();
            }
        });

        let win_row = window.clone();
        let rows_row = rows_rc.clone();
        list.connect_row_activated(move |_, row| {
            if let Some(item) = rows_row.borrow().get(row.index() as usize) {
                spawn_command(&item.exec_cmd);
                win_row.close();
            }
        });

        let key_ctrl = EventControllerKey::new();
        let win_esc = window.clone();
        key_ctrl.connect_key_pressed(move |_, key, _, _| {
            if key == gdk::Key::Escape {
                win_esc.close();
                return glib::Propagation::Stop;
            }
            glib::Propagation::Proceed
        });
        window.add_controller(key_ctrl);

        // Auto-close when clicking anywhere outside (Windows Start behavior)
        window.connect_is_active_notify(|win| {
            if !win.is_active() {
                win.close();
            }
        });

        window.present();
        search.grab_focus();
    });

    app.run_with_args::<&str>(&[]);
}

fn populate_list(
    list: &ListBox,
    engine: &Rc<RefCell<LauncherEngine>>,
    rows_store: &Rc<RefCell<Vec<LauncherRow>>>,
    query: &str,
) {
    while let Some(child) = list.first_child() {
        list.remove(&child);
    }
    let rows = engine.borrow_mut().query_rows(query);
    if rows.is_empty() {
        let empty = Label::new(Some(
            "Nothing here yet. Press Enter to search the software catalog.",
        ));
        empty.add_css_class("forge-meta");
        list.append(&empty);
    } else {
        for item in &rows {
            let vbox = GtkBox::new(Orientation::Vertical, 2);
            vbox.add_css_class("forge-row");

            let top = GtkBox::new(Orientation::Horizontal, 8);
            let title = Label::new(Some(&item.title));
            title.add_css_class("forge-title");
            title.set_halign(Align::Start);
            title.set_hexpand(true);

            let meta = Label::new(Some(&item.subtitle_right));
            meta.add_css_class("forge-meta");
            meta.set_halign(Align::End);

            top.append(&title);
            top.append(&meta);

            let cli = Label::new(Some(&item.cli_hint));
            cli.add_css_class("forge-cli");
            cli.set_halign(Align::Start);

            vbox.append(&top);
            vbox.append(&cli);

            let row = ListBoxRow::new();
            row.set_child(Some(&vbox));
            list.append(&row);
        }
        if let Some(first) = list.row_at_index(0) {
            list.select_row(Some(&first));
        }
    }
    *rows_store.borrow_mut() = rows;
}

fn spawn_command(cmd: &str) {
    let _ = Command::new("sh").args(["-c", cmd]).spawn();
}

/// Power Management Popup (Lock, Suspend, Reboot, Shutdown, Logout) — closes on Esc or click outside.
pub fn run_power() {
    let app = Application::builder()
        .application_id("org.forge.shell.power")
        .build();

    app.connect_activate(|app| {
        load_css();
        let window = ApplicationWindow::builder()
            .application(app)
            .title("Power")
            .default_width(340)
            .default_height(260)
            .decorated(false)
            .build();

        let root = GtkBox::new(Orientation::Vertical, 10);
        root.add_css_class("forge-launcher");

        let title = Label::new(Some("Power Management"));
        title.add_css_class("forge-title");
        title.set_halign(Align::Start);
        root.append(&title);

        let items = [
            ("Lock Screen", "swaylock -f -c 070709", false),
            ("Suspend", "systemctl suspend", false),
            ("Log Out", "swaymsg exit", false),
            ("Reboot", "systemctl reboot", true),
            ("Shut Down", "systemctl poweroff", true),
        ];
        for (lbl, cmd, danger) in items {
            let btn = Button::with_label(lbl);
            btn.add_css_class(if danger {
                "forge-accent-btn"
            } else {
                "forge-pin-btn"
            });
            let c = cmd.to_string();
            let w = window.clone();
            btn.connect_clicked(move |_| {
                spawn_command(&c);
                w.close();
            });
            root.append(&btn);
        }

        let key_ctrl = EventControllerKey::new();
        let w_esc = window.clone();
        key_ctrl.connect_key_pressed(move |_, key, _, _| {
            if key == gdk::Key::Escape {
                w_esc.close();
                return glib::Propagation::Stop;
            }
            glib::Propagation::Proceed
        });
        window.add_controller(key_ctrl);
        window.connect_is_active_notify(|win| {
            if !win.is_active() {
                win.close();
            }
        });

        window.set_child(Some(&root));
        window.present();
    });

    app.run_with_args::<&str>(&[]);
}

/// Windows-style Quick Settings Popup: Volume slider, Brightness slider, Wi-Fi & Bluetooth toggles.
pub fn run_quick_settings() {
    let app = Application::builder()
        .application_id("org.forge.shell.quick")
        .build();

    app.connect_activate(|app| {
        load_css();
        let window = ApplicationWindow::builder()
            .application(app)
            .title("Quick Settings")
            .default_width(380)
            .default_height(320)
            .decorated(false)
            .build();

        let root = GtkBox::new(Orientation::Vertical, 12);
        root.add_css_class("forge-launcher");

        let title = Label::new(Some("Quick Settings"));
        title.add_css_class("forge-title");
        title.set_halign(Align::Start);
        root.append(&title);

        // Volume slider (PipeWire / wpctl)
        let vol_lbl = Label::new(Some("Volume"));
        vol_lbl.add_css_class("forge-meta");
        vol_lbl.set_halign(Align::Start);
        let vol_scale = Scale::with_range(Orientation::Horizontal, 0.0, 100.0, 5.0);
        vol_scale.set_value(75.0);
        vol_scale.set_draw_value(true);
        vol_scale.connect_value_changed(|s| {
            let pct = s.value() as u32;
            spawn_command(&format!("wpctl set-volume @DEFAULT_AUDIO_SINK@ {pct}%"));
        });
        root.append(&vol_lbl);
        root.append(&vol_scale);

        // 1-click Audio Output Switcher (speakers vs headphones via wpctl)
        let sink_row = GtkBox::new(Orientation::Horizontal, 6);
        let sink_lbl = Label::new(Some("Audio Output"));
        sink_lbl.add_css_class("forge-meta");
        sink_lbl.set_hexpand(true);
        sink_lbl.set_halign(Align::Start);
        let cycle_sink = Button::with_label("Switch Speaker / Headphones");
        cycle_sink.add_css_class("forge-pin-btn");
        cycle_sink.connect_clicked(|_| {
            spawn_command(
                r#"IDS=$(wpctl status | awk '/Sinks:/,/Sources:/' | grep -oE '[0-9]+\.' | tr -d .); NEXT=$(echo "$IDS" | head -n 2 | tail -n 1); [ -n "$NEXT" ] && wpctl set-default "$NEXT""#,
            );
        });
        sink_row.append(&sink_lbl);
        sink_row.append(&cycle_sink);
        root.append(&sink_row);

        // Brightness slider (brightnessctl)
        let br_lbl = Label::new(Some("Brightness"));
        br_lbl.add_css_class("forge-meta");
        br_lbl.set_halign(Align::Start);
        let br_scale = Scale::with_range(Orientation::Horizontal, 10.0, 100.0, 5.0);
        br_scale.set_value(80.0);
        br_scale.set_draw_value(true);
        br_scale.connect_value_changed(|s| {
            let pct = s.value() as u32;
            spawn_command(&format!("brightnessctl set {pct}%"));
        });
        root.append(&br_lbl);
        root.append(&br_scale);

        root.append(&Separator::new(Orientation::Horizontal));

        for (lbl_txt, on_cmd, off_cmd, default_on) in [
            ("Wi-Fi Radio", "nmcli radio wifi on", "nmcli radio wifi off", true),
            ("Bluetooth", "bluetoothctl power on", "bluetoothctl power off", false),
            ("Ad Blocker (System)", "sudo /usr/local/bin/forge-adblock on", "sudo /usr/local/bin/forge-adblock off", true),
        ] {
            let row = GtkBox::new(Orientation::Horizontal, 8);
            let lbl = Label::new(Some(lbl_txt));
            lbl.set_hexpand(true);
            lbl.set_halign(Align::Start);
            let sw = Switch::new();
            sw.set_active(default_on);
            let on_s = on_cmd.to_string();
            let off_s = off_cmd.to_string();
            sw.connect_state_set(move |_, active| {
                spawn_command(if active { &on_s } else { &off_s });
                glib::Propagation::Proceed
            });
            row.append(&lbl);
            row.append(&sw);
            root.append(&row);
        }

        let clean_btn = Button::with_label("Clean RAM & Dev Caches");
        clean_btn.add_css_class("forge-pin-btn");
        let w_cln = window.clone();
        clean_btn.connect_clicked(move |_| {
            spawn_command("foot -e sh -c 'sudo /usr/local/bin/forge-clean; sleep 2'");
            w_cln.close();
        });
        root.append(&clean_btn);

        let open_all = Button::with_label("Open Full Settings");
        open_all.add_css_class("forge-accent-btn");
        let w_set = window.clone();
        open_all.connect_clicked(move |_| {
            spawn_command("forge-settings");
            w_set.close();
        });
        root.append(&open_all);

        let key_ctrl = EventControllerKey::new();
        let w_esc = window.clone();
        key_ctrl.connect_key_pressed(move |_, key, _, _| {
            if key == gdk::Key::Escape {
                w_esc.close();
                return glib::Propagation::Stop;
            }
            glib::Propagation::Proceed
        });
        window.add_controller(key_ctrl);
        window.connect_is_active_notify(|win| {
            if !win.is_active() {
                win.close();
            }
        });

        window.set_child(Some(&root));
        window.present();
    });

    app.run_with_args::<&str>(&[]);
}

/// Mouse Right-Click Context Menu Popup (closes on Esc or click outside).
pub fn run_context_menu() {
    let app = Application::builder()
        .application_id("org.forge.shell.menu")
        .build();

    app.connect_activate(|app| {
        load_css();
        let window = ApplicationWindow::builder()
            .application(app)
            .title("Menu")
            .default_width(260)
            .default_height(280)
            .decorated(false)
            .build();

        let root = GtkBox::new(Orientation::Vertical, 6);
        root.add_css_class("forge-launcher");

        let actions = [
            ("New Terminal", "foot"),
            ("New Code File (Mousepad)", "mousepad"),
            ("Open File Manager", "thunar"),
            (
                "Quick Settings (Sound/Brightness)",
                "forge-shell --mode quick",
            ),
            (
                "Clean RAM & Dev Caches",
                "foot -e sh -c 'sudo /usr/local/bin/forge-clean; sleep 2'",
            ),
            ("System Monitor (HUD)", "foot -e /usr/local/bin/forge-hud"),
            ("Clipboard History (Win+V)", "forge-clipboard"),
            ("Forge OS Helpbook", "forge-help"),
            (
                "Install Forge OS to Disk",
                "foot -e sudo /usr/local/bin/forge-installer",
            ),
            ("Developer Labs (System Internals)", "forge-lab"),
            ("System Settings", "forge-settings"),
            ("Power Menu", "forge-shell --mode power"),
        ];
        for (lbl, cmd) in actions {
            let btn = Button::with_label(lbl);
            btn.add_css_class("forge-pin-btn");
            let c = cmd.to_string();
            let w = window.clone();
            btn.connect_clicked(move |_| {
                spawn_command(&c);
                w.close();
            });
            root.append(&btn);
        }

        let key_ctrl = EventControllerKey::new();
        let w_esc = window.clone();
        key_ctrl.connect_key_pressed(move |_, key, _, _| {
            if key == gdk::Key::Escape {
                w_esc.close();
                return glib::Propagation::Stop;
            }
            glib::Propagation::Proceed
        });
        window.add_controller(key_ctrl);
        window.connect_is_active_notify(|win| {
            if !win.is_active() {
                win.close();
            }
        });

        window.set_child(Some(&root));
        window.present();
    });

    app.run_with_args::<&str>(&[]);
}
