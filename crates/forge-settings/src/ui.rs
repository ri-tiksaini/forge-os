#![forbid(unsafe_code)]

//! Sleek Forge OS Control Panel:
//! - Live System Monitor with 30-second Bar Graph + Task Manager (End Task)
//! - Date & Time + Timezone + NTP Settings
//! - Network & Wi-Fi + System-Wide Ad Blocker
//! - Keyboard, Mouse, Display & Firmware
//! - Software Store, Parked Services, Projects, Multi-Theme Appearance, About & Repair

use std::cell::RefCell;
use std::collections::VecDeque;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::rc::Rc;
use std::time::Duration;

use gtk4::gdk;
use gtk4::glib;
use gtk4::prelude::*;
use gtk4::{
    Align, Application, ApplicationWindow, Box as GtkBox, Button, CssProvider, DrawingArea, Entry,
    Label, ListBox, Orientation, ProgressBar, ScrolledWindow, SearchEntry, SelectionMode, Stack,
    StackSidebar, Switch,
};

use forge_catalog::CatalogIndex;
use forge_shell::pill::{CpuPrev, NetPrev, PillSnapshot};
use forge_tier::TierReport;

use crate::model::{create_project, parked_services, query_store};

#[derive(Clone, Copy)]
struct ThemePreset {
    name: &'static str,
    accent: &'static str,
    accent_hi: &'static str,
    bg: &'static str,
    rgb: (f64, f64, f64),
    rgb_hi: (f64, f64, f64),
}

const PRESETS: &[ThemePreset] = &[
    ThemePreset {
        name: "Dark Cherry",
        accent: "#9B1126",
        accent_hi: "#C42B42",
        bg: "#070709",
        rgb: (0.608, 0.067, 0.149),
        rgb_hi: (0.769, 0.169, 0.259),
    },
    ThemePreset {
        name: "Midnight Violet",
        accent: "#7C3AED",
        accent_hi: "#A78BFA",
        bg: "#09090F",
        rgb: (0.486, 0.227, 0.929),
        rgb_hi: (0.655, 0.545, 0.980),
    },
    ThemePreset {
        name: "Cyber Emerald",
        accent: "#059669",
        accent_hi: "#34D399",
        bg: "#060B09",
        rgb: (0.020, 0.588, 0.412),
        rgb_hi: (0.204, 0.827, 0.600),
    },
    ThemePreset {
        name: "Arctic Cyan",
        accent: "#0891B2",
        accent_hi: "#22D3EE",
        bg: "#070B0F",
        rgb: (0.031, 0.569, 0.698),
        rgb_hi: (0.133, 0.827, 0.933),
    },
    ThemePreset {
        name: "Obsidian Gold",
        accent: "#D97706",
        accent_hi: "#FBBF24",
        bg: "#090806",
        rgb: (0.851, 0.467, 0.024),
        rgb_hi: (0.984, 0.749, 0.141),
    },
];

fn css_for(preset: ThemePreset) -> String {
    let bg = preset.bg;
    let fg = forge_brand::FG;
    let accent = preset.accent;
    let accent_hi = preset.accent_hi;
    let ui = forge_brand::FONT_UI;
    let mono = forge_brand::FONT_MONO;
    format!(
        "window {{ background-color: {bg}; color: {fg}; font-family: '{ui}'; }}\n\
         .forge-sidebar {{ background-color: #0D0D11; border-right: 2px solid {accent}; min-width: 205px; padding: 8px 6px; }}\n\
         stacksidebar list {{ background-color: transparent; }}\n\
         stacksidebar row {{ border-radius: 8px; margin: 2px 4px; padding: 8px 10px; color: #A1A1AA; }}\n\
         stacksidebar row:hover {{ background-color: #18141A; color: {fg}; }}\n\
         stacksidebar row:selected {{ background-color: #1E1419; border-left: 3px solid {accent}; color: {fg}; font-weight: 700; }}\n\
         .forge-page {{ padding: 20px; background-color: {bg}; }}\n\
         .forge-h1 {{ font-size: 20px; font-weight: 700; color: {fg}; }}\n\
         .forge-kpi {{ font-size: 22px; font-weight: 800; color: {accent_hi}; font-family: '{mono}'; }}\n\
         .forge-muted {{ font-size: 13px; color: #A1A1AA; }}\n\
         .forge-mono {{ font-family: '{mono}'; font-size: 12px; color: #D4D4D8; }}\n\
         .forge-card {{ background-color: #121217; border: 1px solid {accent}; border-radius: 12px; padding: 14px; }}\n\
         button {{ background-color: #16161D; color: {fg}; border: 1px solid {accent}; border-radius: 8px; padding: 7px 14px; }}\n\
         button:hover {{ background-color: #20141A; border-color: {accent_hi}; }}\n\
         .forge-accent {{ background-color: {accent}; color: #FFFFFF; font-weight: 700; border: 1px solid {accent_hi}; border-radius: 10px; padding: 7px 14px; }}\n\
         entry, searchentry {{ background-color: #121217; color: {fg}; border: 1px solid {accent}; border-radius: 8px; padding: 6px 10px; }}\n\
         entry:focus-within, searchentry:focus-within {{ border-color: {accent_hi}; }}\n\
         progressbar trough {{ background-color: #1E161A; border-radius: 6px; min-height: 8px; }}\n\
         progressbar progress {{ background-color: {accent}; border-radius: 6px; }}\n\
         switch {{ background-color: #1E161A; border: 1px solid {accent}; border-radius: 14px; }}\n\
         switch:checked {{ background-color: {accent}; border-color: {accent_hi}; }}\n\
         scrollbar slider {{ background-color: {accent}; border-radius: 6px; min-width: 6px; }}\n\
         scrollbar slider:hover {{ background-color: {accent_hi}; }}\n"
    )
}

pub fn run(index: CatalogIndex, tier: TierReport) {
    let app = Application::builder()
        .application_id("org.forge.settings")
        .build();

    let index_rc = Rc::new(index);
    let tier_rc = Rc::new(tier);

    app.connect_activate(move |app| {
        let initial_preset = load_saved_preset();
        let provider = CssProvider::new();
        provider.load_from_data(&css_for(initial_preset));
        if let Some(display) = gdk::Display::default() {
            gtk4::style_context_add_provider_for_display(
                &display,
                &provider,
                gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
            );
        }

        let active_rgb = Rc::new(RefCell::new((initial_preset.rgb, initial_preset.rgb_hi)));

        let title = format!("{} Control Center", forge_brand::NAME);
        let window = ApplicationWindow::builder()
            .application(app)
            .title(&title)
            .default_width(900)
            .default_height(580)
            .build();

        let root = GtkBox::new(Orientation::Horizontal, 0);
        let stack = Stack::new();
        stack.set_hexpand(true);
        stack.set_vexpand(true);

        let sidebar = StackSidebar::new();
        sidebar.set_stack(&stack);
        sidebar.add_css_class("forge-sidebar");

        let tier_lbl = format!("T{}", tier_rc.tier.as_u8());
        stack.add_titled(
            &page_monitor(tier_lbl, active_rgb.clone()),
            Some("monitor"),
            "System Monitor",
        );
        stack.add_titled(&page_datetime(), Some("datetime"), "Date & Time");
        stack.add_titled(&page_network(), Some("network"), "Network & Wi-Fi");
        stack.add_titled(
            &page_input_display(&tier_rc),
            Some("devices"),
            "Devices & Display",
        );
        stack.add_titled(
            &page_software(index_rc.clone(), tier_rc.tier.as_u8()),
            Some("software"),
            "Software Store",
        );
        stack.add_titled(
            &page_services(index_rc.clone()),
            Some("services"),
            "Parked Services",
        );
        stack.add_titled(&page_projects(), Some("projects"), "Projects");
        stack.add_titled(
            &page_appearance(provider.clone(), active_rgb),
            Some("appearance"),
            "Appearance & Themes",
        );
        stack.add_titled(&page_about(), Some("about"), "About & Repair");

        let main_scroll = ScrolledWindow::builder()
            .hexpand(true)
            .vexpand(true)
            .child(&stack)
            .build();
        root.append(&sidebar);
        root.append(&main_scroll);
        window.set_child(Some(&root));
        window.present();
    });

    app.run_with_args::<&str>(&[]);
}

fn page_header(title: &str, subtitle: &str) -> GtkBox {
    let b = GtkBox::new(Orientation::Vertical, 4);
    let h = Label::new(Some(title));
    h.add_css_class("forge-h1");
    h.set_halign(Align::Start);
    let s = Label::new(Some(subtitle));
    s.add_css_class("forge-muted");
    s.set_halign(Align::Start);
    b.append(&h);
    b.append(&s);
    b
}

fn page_monitor(
    tier_label: String,
    active_rgb: Rc<RefCell<((f64, f64, f64), (f64, f64, f64))>>,
) -> GtkBox {
    let page = GtkBox::new(Orientation::Vertical, 14);
    page.add_css_class("forge-page");
    page.append(&page_header(
        "System Monitor",
        "Real-time 30-second bar graph telemetry + Windows-style Task Manager.",
    ));

    // Top 3 KPI summary cards
    let kpi_row = GtkBox::new(Orientation::Horizontal, 10);
    let (cpu_card, cpu_val, cpu_bar) = make_kpi_card("CPU");
    let (ram_card, ram_val, ram_bar) = make_kpi_card("RAM");
    let (disk_card, disk_val, disk_bar) = make_kpi_card("STORAGE /");
    kpi_row.append(&cpu_card);
    kpi_row.append(&ram_card);
    kpi_row.append(&disk_card);
    page.append(&kpi_row);

    // Live 30-second Bar Graph (CPU + RAM side-by-side bars)
    let history: Rc<RefCell<VecDeque<(u8, u8)>>> =
        Rc::new(RefCell::new(VecDeque::from(vec![(0u8, 0u8); 28])));
    let graph_card = GtkBox::new(Orientation::Vertical, 6);
    graph_card.add_css_class("forge-card");

    let legend = Label::new(Some(
        "Live 30s Telemetry Bar Graph  (Left bar: CPU %   |   Right bar: RAM %)",
    ));
    legend.add_css_class("forge-mono");
    legend.set_halign(Align::Start);
    graph_card.append(&legend);

    let area = DrawingArea::new();
    area.set_content_height(145);
    area.set_hexpand(true);
    let hist_draw = history.clone();
    let rgb_draw = active_rgb;
    area.set_draw_func(move |_, cr, w, h| {
        let wf = f64::from(w);
        let hf = f64::from(h);
        // Background
        cr.set_source_rgb(0.051, 0.051, 0.071);
        let _ = cr.paint();

        // Subtle horizontal gridlines (25%, 50%, 75%)
        cr.set_source_rgba(1.0, 1.0, 1.0, 0.08);
        cr.set_line_width(1.0);
        for pct in [0.25, 0.50, 0.75] {
            let y = hf * (1.0 - pct);
            cr.move_to(0.0, y);
            cr.line_to(wf, y);
            let _ = cr.stroke();
        }

        let samples = hist_draw.borrow();
        let n = samples.len().max(1) as f64;
        let slot_w = wf / n;
        let bar_w = (slot_w * 0.36).max(3.0);
        let ((r1, g1, b1), (r2, g2, b2)) = *rgb_draw.borrow();

        for (i, &(cpu, ram)) in samples.iter().enumerate() {
            let x0 = (i as f64) * slot_w + slot_w * 0.12;
            let cpu_h = (f64::from(cpu) / 100.0) * (hf - 10.0);
            let ram_h = (f64::from(ram) / 100.0) * (hf - 10.0);

            cr.set_source_rgb(r1, g1, b1);
            cr.rectangle(x0, hf - cpu_h.max(2.0), bar_w, cpu_h.max(2.0));
            let _ = cr.fill();

            cr.set_source_rgb(r2, g2, b2);
            cr.rectangle(x0 + bar_w + 2.0, hf - ram_h.max(2.0), bar_w, ram_h.max(2.0));
            let _ = cr.fill();
        }
    });
    graph_card.append(&area);

    let extra_lbl = Label::new(Some("GPU / Network / Battery / Sound"));
    extra_lbl.add_css_class("forge-mono");
    extra_lbl.set_halign(Align::Start);
    graph_card.append(&extra_lbl);
    page.append(&graph_card);

    let cpu_prev = Rc::new(RefCell::new(CpuPrev::default()));
    let net_prev = Rc::new(RefCell::new(NetPrev::default()));
    let tier_c = tier_label.clone();
    let hist_tick = history;
    let area_c = area;
    glib::timeout_add_local(Duration::from_secs(1), move || {
        let s = PillSnapshot::sample_live(
            &tier_c,
            &mut cpu_prev.borrow_mut(),
            &mut net_prev.borrow_mut(),
        );
        cpu_val.set_text(&format!("{}%", s.cpu_percent));
        cpu_bar.set_fraction(f64::from(s.cpu_percent) / 100.0);
        ram_val.set_text(&format!("{}% ({}M)", s.ram_percent, s.ram_used_mib));
        ram_bar.set_fraction(f64::from(s.ram_percent) / 100.0);
        disk_val.set_text(&format!("{}%", s.disk_percent));
        disk_bar.set_fraction(f64::from(s.disk_percent) / 100.0);

        let bat = s
            .battery_percent
            .map(|b| format!("{b}%"))
            .unwrap_or_else(|| "AC".to_string());
        let vol = s
            .volume_percent
            .map(|v| format!("{v}%"))
            .unwrap_or_else(|| "--".to_string());
        extra_lbl.set_text(&format!(
            "GPU: {}   |   Net: {}K↓ / {}K↑   |   Power: {}   |   Sound: {}",
            s.gpu_label, s.net_rx_kbps, s.net_tx_kbps, bat, vol
        ));

        {
            let mut h = hist_tick.borrow_mut();
            if h.len() >= 28 {
                h.pop_front();
            }
            h.push_back((s.cpu_percent, s.ram_percent));
        }
        area_c.queue_draw();
        glib::ControlFlow::Continue
    });

    // Task Manager section
    let proc_hdr = GtkBox::new(Orientation::Horizontal, 8);
    let proc_title = Label::new(Some("Running Processes (Task Manager)"));
    proc_title.add_css_class("forge-h1");
    proc_title.set_hexpand(true);
    proc_title.set_halign(Align::Start);
    let refresh_btn = Button::with_label("Refresh");
    refresh_btn.add_css_class("forge-accent");
    proc_hdr.append(&proc_title);
    proc_hdr.append(&refresh_btn);
    page.append(&proc_hdr);

    let proc_list = ListBox::new();
    proc_list.set_selection_mode(SelectionMode::None);
    page.append(&proc_list);

    let proc_status = Label::new(Some(""));
    proc_status.add_css_class("forge-mono");
    proc_status.set_halign(Align::Start);
    page.append(&proc_status);

    fill_processes(&proc_list, &proc_status);
    let pl_c = proc_list.clone();
    let ps_c = proc_status.clone();
    refresh_btn.connect_clicked(move |_| fill_processes(&pl_c, &ps_c));

    page
}

fn make_kpi_card(label: &str) -> (GtkBox, Label, ProgressBar) {
    let card = GtkBox::new(Orientation::Vertical, 6);
    card.add_css_class("forge-card");
    card.set_hexpand(true);
    let title = Label::new(Some(label));
    title.add_css_class("forge-muted");
    title.set_halign(Align::Start);
    let val = Label::new(Some("--%"));
    val.add_css_class("forge-kpi");
    val.set_halign(Align::Start);
    let bar = ProgressBar::new();
    card.append(&title);
    card.append(&val);
    card.append(&bar);
    (card, val, bar)
}

fn fill_processes(list: &ListBox, status: &Label) {
    while let Some(child) = list.first_child() {
        list.remove(&child);
    }
    let Ok(out) = Command::new("ps")
        .args(["-eo", "pid,pcpu,pmem,comm", "--sort=-pmem"])
        .output()
    else {
        return;
    };
    let text = String::from_utf8_lossy(&out.stdout);
    for line in text.lines().skip(1).take(10) {
        let cols: Vec<&str> = line.split_whitespace().collect();
        if cols.len() < 4 {
            continue;
        }
        let pid = cols[0].to_string();
        let cpu = cols[1];
        let mem = cols[2];
        let comm = cols[3];

        let row = GtkBox::new(Orientation::Horizontal, 10);
        row.add_css_class("forge-card");
        row.set_margin_bottom(4);

        let info = Label::new(Some(&format!(
            "PID {pid:<6}  |  {comm:<18}  |  CPU {cpu:>4}%  |  RAM {mem:>4}%"
        )));
        info.add_css_class("forge-mono");
        info.set_hexpand(true);
        info.set_halign(Align::Start);

        let end_btn = Button::with_label("End Task");
        end_btn.add_css_class("forge-accent");
        let pid_c = pid.clone();
        let comm_s = comm.to_string();
        let st_c = status.clone();
        end_btn.connect_clicked(move |_| {
            let _ = Command::new("kill").args(["-TERM", &pid_c]).status();
            st_c.set_text(&format!("Sent SIGTERM to {comm_s} (PID {pid_c})"));
        });

        row.append(&info);
        row.append(&end_btn);
        list.append(&row);
    }
}

fn read_clock_string() -> String {
    if let Ok(out) = Command::new("date")
        .arg("+%Y-%m-%d  %H:%M:%S  (%Z)")
        .output()
    {
        if out.status.success() {
            return String::from_utf8_lossy(&out.stdout).trim().to_string();
        }
    }
    "Clock unavailable".to_string()
}

fn read_ntp_enabled() -> bool {
    Command::new("timedatectl")
        .args(["show", "-p", "NTP", "--value"])
        .output()
        .ok()
        .is_some_and(|o| {
            String::from_utf8_lossy(&o.stdout)
                .trim()
                .eq_ignore_ascii_case("yes")
        })
}

fn run_privileged_timedate(args: &[&str]) -> Result<(), String> {
    if let Ok(o) = Command::new("pkexec")
        .arg("timedatectl")
        .args(args)
        .output()
    {
        if o.status.success() {
            return Ok(());
        }
        let err = String::from_utf8_lossy(&o.stderr).trim().to_string();
        return Err(if err.is_empty() {
            "Authentication cancelled or timedatectl rejected the change.".to_string()
        } else {
            err
        });
    }
    Err("pkexec is not available on this host.".to_string())
}

fn page_datetime() -> GtkBox {
    let page = GtkBox::new(Orientation::Vertical, 12);
    page.add_css_class("forge-page");
    page.append(&page_header(
        "Date & Time",
        "Configure timezone, automatic NTP sync, or set system date/time manually.",
    ));

    let card = GtkBox::new(Orientation::Vertical, 10);
    card.add_css_class("forge-card");

    let current_lbl = Label::new(Some(&read_clock_string()));
    current_lbl.add_css_class("forge-kpi");
    current_lbl.set_halign(Align::Start);
    card.append(&current_lbl);

    let status_lbl = Label::new(Some(""));
    status_lbl.add_css_class("forge-mono");
    status_lbl.set_wrap(true);
    status_lbl.set_halign(Align::Start);

    // Live 1 Hz clock readout
    let cur_c = current_lbl.clone();
    glib::timeout_add_local(Duration::from_secs(1), move || {
        cur_c.set_text(&read_clock_string());
        glib::ControlFlow::Continue
    });

    // Automatic NTP Toggle
    let ntp_row = GtkBox::new(Orientation::Horizontal, 8);
    let ntp_lbl = Label::new(Some("Automatic Network Time Sync (NTP)"));
    ntp_lbl.set_hexpand(true);
    ntp_lbl.set_halign(Align::Start);
    let ntp_sw = Switch::new();
    ntp_sw.set_active(read_ntp_enabled());
    let st_ntp = status_lbl.clone();
    let cur_ntp = current_lbl.clone();
    ntp_sw.connect_state_set(move |_, active| {
        let arg = if active { "true" } else { "false" };
        match run_privileged_timedate(&["set-ntp", arg]) {
            Ok(()) => {
                cur_ntp.set_text(&read_clock_string());
                st_ntp.set_text(&format!("Automatic NTP time sync set to {arg}."));
                glib::Propagation::Proceed
            }
            Err(e) => {
                st_ntp.set_text(&format!("NTP change failed: {e}"));
                glib::Propagation::Stop
            }
        }
    });
    ntp_row.append(&ntp_lbl);
    ntp_row.append(&ntp_sw);
    card.append(&ntp_row);

    // Quick Timezone Presets
    let tz_title = Label::new(Some("Quick Timezone"));
    tz_title.add_css_class("forge-muted");
    tz_title.set_halign(Align::Start);
    card.append(&tz_title);

    let tz_row = GtkBox::new(Orientation::Horizontal, 8);
    for tz in ["Asia/Kolkata", "UTC", "America/New_York", "Europe/London"] {
        let b = Button::with_label(tz);
        let tz_s = tz.to_string();
        let st_tz = status_lbl.clone();
        let cur_tz = current_lbl.clone();
        b.connect_clicked(
            move |_| match run_privileged_timedate(&["set-timezone", &tz_s]) {
                Ok(()) => {
                    cur_tz.set_text(&read_clock_string());
                    st_tz.set_text(&format!("Timezone updated to {tz_s}."));
                }
                Err(e) => st_tz.set_text(&format!("Timezone update failed: {e}")),
            },
        );
        tz_row.append(&b);
    }
    card.append(&tz_row);

    // Manual Date & Time Input
    let manual_entry = Entry::new();
    manual_entry.set_placeholder_text(Some("YYYY-MM-DD HH:MM:SS (e.g. 2026-11-24 20:30:00)"));
    let set_btn = Button::with_label("Set Date & Time Manually");
    set_btn.add_css_class("forge-accent");
    let ent_c = manual_entry.clone();
    let st_man = status_lbl.clone();
    let cur_man = current_lbl;
    let ntp_man = ntp_sw;
    set_btn.connect_clicked(move |_| {
        let raw = ent_c.text();
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            st_man.set_text("Enter date/time as YYYY-MM-DD HH:MM:SS (or YYYY-MM-DD HH:MM).");
            return;
        }
        let normalized =
            if trimmed.len() == 16 && trimmed.chars().filter(|&c| c == ':').count() == 1 {
                format!("{trimmed}:00")
            } else {
                trimmed.to_string()
            };

        // Single pkexec call so user only enters password once!
        let out = Command::new("pkexec")
            .args([
                "/bin/bash",
                "-c",
                "timedatectl set-ntp false && timedatectl set-time \"$1\"",
                "_",
                &normalized,
            ])
            .output();
        match out {
            Ok(o) if o.status.success() => {
                ntp_man.set_active(false);
                cur_man.set_text(&read_clock_string());
                st_man.set_text(&format!("System clock updated to {normalized}."));
            }
            Ok(o) => {
                let err = String::from_utf8_lossy(&o.stderr).trim().to_string();
                st_man.set_text(&format!("Could not set date/time: {err}"));
            }
            Err(e) => st_man.set_text(&format!("pkexec failed: {e}")),
        }
    });

    card.append(&manual_entry);
    card.append(&set_btn);
    card.append(&status_lbl);
    page.append(&card);
    page
}

fn page_network() -> GtkBox {
    let page = GtkBox::new(Orientation::Vertical, 12);
    page.add_css_class("forge-page");
    page.append(&page_header(
        "Network, Wireless & Ad Blocker",
        "Manage Wi-Fi, Bluetooth, and system-wide ad/tracker blocking.",
    ));

    let card = GtkBox::new(Orientation::Vertical, 10);
    card.add_css_class("forge-card");

    for (title, on_cmd, off_cmd, default_on) in [
        (
            "Wi-Fi Radio",
            "nmcli radio wifi on",
            "nmcli radio wifi off",
            true,
        ),
        (
            "Bluetooth Radio",
            "bluetoothctl power on",
            "bluetoothctl power off",
            false,
        ),
        (
            "System-Wide Ad & Tracker Blocker",
            "sudo /usr/local/bin/forge-adblock on",
            "sudo /usr/local/bin/forge-adblock off",
            true,
        ),
        (
            "Security Firewall Lockdown (UFW)",
            "sudo /usr/local/bin/forge-security lockdown",
            "sudo /usr/local/bin/forge-security unlock",
            false,
        ),
    ] {
        let row = GtkBox::new(Orientation::Horizontal, 8);
        let lbl = Label::new(Some(title));
        lbl.set_hexpand(true);
        lbl.set_halign(Align::Start);
        let sw = Switch::new();
        sw.set_active(default_on);
        let on_s = on_cmd.to_string();
        let off_s = off_cmd.to_string();
        sw.connect_state_set(move |_, active| {
            let _ = Command::new("sh")
                .args(["-c", if active { &on_s } else { &off_s }])
                .spawn();
            glib::Propagation::Proceed
        });
        row.append(&lbl);
        row.append(&sw);
        card.append(&row);
    }

    let status_lbl = Label::new(Some("Click Scan to list nearby Wi-Fi networks."));
    status_lbl.add_css_class("forge-mono");
    status_lbl.set_halign(Align::Start);

    let diag_row = GtkBox::new(Orientation::Horizontal, 8);
    let netcheck_btn = Button::with_label("Troubleshoot Network");
    let st_diag = status_lbl.clone();
    netcheck_btn.connect_clicked(move |_| match Command::new("forge-netcheck").output() {
        Ok(o) => st_diag.set_text(String::from_utf8_lossy(&o.stdout).trim()),
        Err(_) => st_diag.set_text("Run in terminal: forge-netcheck"),
    });
    let ports_btn = Button::with_label("Inspect Open Ports");
    let st_ports = status_lbl.clone();
    ports_btn.connect_clicked(move |_| match Command::new("forge-ports").output() {
        Ok(o) => st_ports.set_text(String::from_utf8_lossy(&o.stdout).trim()),
        Err(_) => st_ports.set_text("Run in terminal: forge-ports"),
    });
    diag_row.append(&netcheck_btn);
    diag_row.append(&ports_btn);
    card.append(&diag_row);

    let scan_btn = Button::with_label("Scan Wi-Fi Networks");
    scan_btn.add_css_class("forge-accent");
    let st_c = status_lbl.clone();
    scan_btn.connect_clicked(move |_| {
        let out = Command::new("nmcli")
            .args(["-t", "-f", "SSID,SIGNAL,SECURITY", "dev", "wifi", "list"])
            .output();
        match out {
            Ok(o) if o.status.success() => {
                let txt = String::from_utf8_lossy(&o.stdout);
                st_c.set_text(if txt.trim().is_empty() {
                    "No Wi-Fi adapters or networks detected."
                } else {
                    txt.trim()
                });
            }
            _ => st_c.set_text("NetworkManager unavailable in this preview environment."),
        }
    });

    let ssid_entry = Entry::new();
    ssid_entry.set_placeholder_text(Some("Wi-Fi network name (SSID)"));
    let pw_entry = Entry::new();
    pw_entry.set_visibility(false);
    pw_entry.set_placeholder_text(Some("Wi-Fi password"));
    let connect_btn = Button::with_label("Connect to Wi-Fi");
    connect_btn.add_css_class("forge-accent");

    let ssid_c = ssid_entry.clone();
    let pw_c = pw_entry.clone();
    let st_conn = status_lbl.clone();
    connect_btn.connect_clicked(move |_| {
        let ssid = ssid_c.text();
        let pw = pw_c.text();
        if ssid.trim().is_empty() {
            st_conn.set_text("Enter an SSID first.");
            return;
        }
        let out = Command::new("nmcli")
            .args([
                "dev",
                "wifi",
                "connect",
                ssid.trim(),
                "password",
                pw.as_str(),
            ])
            .output();
        match out {
            Ok(o) if o.status.success() => st_conn.set_text("Connected to Wi-Fi."),
            Ok(o) => st_conn.set_text(String::from_utf8_lossy(&o.stderr).trim()),
            Err(e) => st_conn.set_text(&format!("nmcli error: {e}")),
        }
    });

    card.append(&scan_btn);
    card.append(&ssid_entry);
    card.append(&pw_entry);
    card.append(&connect_btn);
    card.append(&status_lbl);
    page.append(&card);
    page
}

fn page_input_display(tier: &TierReport) -> GtkBox {
    let page = GtkBox::new(Orientation::Vertical, 12);
    page.add_css_class("forge-page");
    page.append(&page_header(
        "Keyboard, Mouse, Display & Firmware",
        "Windows-familiar navigation + hardware tier diagnostics.",
    ));

    let card = GtkBox::new(Orientation::Vertical, 6);
    card.add_css_class("forge-card");
    for line in [
        "Keyboard Shortcuts : Windows-familiar (Win = Apps, Alt+Tab, Alt+F4, Win+E, Win+V)",
        "Terminal Paste/Copy: Ctrl+V Paste | Ctrl+Shift+C / Ctrl+Insert Copy | Ctrl+C Interrupt",
        &format!("Detected Tier      : {} ({}/100)", tier.tier, tier.score),
        &format!("Compositor         : {}", tier.tier.compositor()),
        &format!("CPU                : {}", tier.raw.cpu_model),
        &format!("GPU Driver         : {}", tier.raw.gpu_driver),
    ] {
        let l = Label::new(Some(line));
        l.add_css_class("forge-mono");
        l.set_halign(Align::Start);
        card.append(&l);
    }

    let regen = Button::with_label("Apply Keyboard & Display Config (forge-conf)");
    regen.add_css_class("forge-accent");
    regen.connect_clicked(|_| {
        let _ = Command::new("forge-conf").spawn();
    });

    let fw_status = Label::new(Some(""));
    fw_status.add_css_class("forge-mono");
    fw_status.set_halign(Align::Start);

    let fw_btn = Button::with_label("Check Laptop Firmware & BIOS Updates (fwupd)");
    let fw_c = fw_status.clone();
    fw_btn.connect_clicked(
        move |_| match Command::new("fwupdmgr").arg("get-updates").output() {
            Ok(o) => {
                let txt = String::from_utf8_lossy(&o.stdout);
                let err = String::from_utf8_lossy(&o.stderr);
                fw_c.set_text(if !txt.trim().is_empty() {
                    txt.trim()
                } else if !err.trim().is_empty() {
                    err.trim()
                } else {
                    "Firmware is up to date."
                });
            }
            Err(_) => fw_c.set_text("fwupd not installed or no supported firmware devices."),
        },
    );

    page.append(&card);
    page.append(&regen);
    page.append(&fw_btn);
    page.append(&fw_status);
    page
}

fn page_software(index: Rc<CatalogIndex>, tier: u8) -> GtkBox {
    let page = GtkBox::new(Orientation::Vertical, 12);
    page.add_css_class("forge-page");
    page.append(&page_header(
        "Software Store",
        "See disk and RAM cost before installing. Nothing auto-starts without your consent.",
    ));

    let search = SearchEntry::new();
    search.set_placeholder_text(Some("Search editors, databases, languages, stacks..."));
    page.append(&search);

    let list = ListBox::new();
    list.set_selection_mode(SelectionMode::None);
    let scroller = ScrolledWindow::builder().vexpand(true).child(&list).build();
    page.append(&scroller);

    let status = Label::new(Some("Tip: every button here maps 1:1 to `dev add <id>`."));
    status.add_css_class("forge-mono");
    status.set_halign(Align::Start);
    page.append(&status);

    fill_store(&list, &index, "", tier, &status);

    let list_c = list.clone();
    let idx_c = index.clone();
    let status_c = status.clone();
    search.connect_search_changed(move |entry| {
        fill_store(&list_c, &idx_c, &entry.text(), tier, &status_c);
    });

    page
}

fn fill_store(list: &ListBox, index: &CatalogIndex, query: &str, tier: u8, status: &Label) {
    while let Some(child) = list.first_child() {
        list.remove(&child);
    }
    for item in query_store(index, query, tier) {
        let card = GtkBox::new(Orientation::Horizontal, 12);
        card.add_css_class("forge-card");
        card.set_margin_bottom(8);

        let info = GtkBox::new(Orientation::Vertical, 4);
        info.set_hexpand(true);

        let name = Label::new(Some(&item.name));
        name.set_halign(Align::Start);
        let cost = format!(
            "{}  |  Disk: {} MB  |  Idle RAM: {} MB  |  Active RAM: {} MB",
            item.category, item.disk_mb, item.ram_idle_mb, item.ram_active_mb
        );
        let meta = Label::new(Some(&cost));
        meta.add_css_class("forge-mono");
        meta.set_halign(Align::Start);

        let note = Label::new(Some(&item.beginner_note));
        note.add_css_class("forge-muted");
        note.set_halign(Align::Start);

        info.append(&name);
        info.append(&meta);
        info.append(&note);

        let btn = Button::with_label("Install");
        btn.add_css_class("forge-accent");
        btn.set_valign(Align::Center);
        let id = item.id.clone();
        let status_b = status.clone();
        btn.connect_clicked(move |_| {
            let cmd = format!("dev add {id} --yes");
            status_b.set_text(&format!("Running: {cmd}"));
            let _ = Command::new("sh").args(["-c", &cmd]).spawn();
        });

        card.append(&info);
        card.append(&btn);
        list.append(&card);
    }
}

fn page_services(index: Rc<CatalogIndex>) -> GtkBox {
    let page = GtkBox::new(Orientation::Vertical, 12);
    page.add_css_class("forge-page");
    page.append(&page_header(
        "Parked Services",
        "Off means installed but silent (0 MB RAM until a project connects).",
    ));

    let status = Label::new(Some(""));
    status.add_css_class("forge-mono");
    status.set_halign(Align::Start);

    for svc in parked_services(&index) {
        let card = GtkBox::new(Orientation::Horizontal, 12);
        card.add_css_class("forge-card");

        let info = GtkBox::new(Orientation::Vertical, 4);
        info.set_hexpand(true);
        let title = Label::new(Some(&svc.name));
        title.set_halign(Align::Start);
        let detail = format!(
            "On-demand  |  Idle: 0 MB  |  Active: {} MB  |  CLI: dev on/off {}",
            svc.ram_active_mb, svc.id
        );
        let meta = Label::new(Some(&detail));
        meta.add_css_class("forge-mono");
        meta.set_halign(Align::Start);
        info.append(&title);
        info.append(&meta);

        let on_btn = Button::with_label("Start");
        let off_btn = Button::with_label("Park");
        let id_on = svc.id.clone();
        let st_on = status.clone();
        on_btn.connect_clicked(move |_| {
            st_on.set_text(&format!("Executed: dev on {id_on}"));
            let _ = Command::new("dev").args(["on", &id_on]).spawn();
        });
        let id_off = svc.id.clone();
        let st_off = status.clone();
        off_btn.connect_clicked(move |_| {
            st_off.set_text(&format!("Executed: dev off {id_off}"));
            let _ = Command::new("dev").args(["off", &id_off]).spawn();
        });

        card.append(&info);
        card.append(&on_btn);
        card.append(&off_btn);
        page.append(&card);
    }

    page.append(&status);
    page
}

fn page_projects() -> GtkBox {
    let page = GtkBox::new(Orientation::Vertical, 12);
    page.add_css_class("forge-page");
    page.append(&page_header(
        "New Project Wizard",
        "Creates a clean starter folder in ~/Projects with zero global clutter.",
    ));

    let name_entry = Entry::new();
    name_entry.set_placeholder_text(Some("my-web-app"));

    let row = GtkBox::new(Orientation::Horizontal, 8);
    let web_btn = Button::with_label("Create Web Project");
    web_btn.add_css_class("forge-accent");
    let rust_btn = Button::with_label("Create Rust Project");
    row.append(&web_btn);
    row.append(&rust_btn);

    let feedback = Label::new(Some(""));
    feedback.add_css_class("forge-mono");
    feedback.set_halign(Align::Start);

    for (btn, stack) in [(web_btn, "web"), (rust_btn, "rust")] {
        let entry_c = name_entry.clone();
        let fb_c = feedback.clone();
        btn.connect_clicked(move |_| {
            let home = env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
            let base = PathBuf::from(home).join("Projects");
            match create_project(&base, stack, &entry_c.text()) {
                Ok(dir) => fb_c.set_text(&format!("Created: {}", dir.display())),
                Err(e) => fb_c.set_text(&format!("Error: {e}")),
            }
        });
    }

    page.append(&name_entry);
    page.append(&row);
    page.append(&feedback);
    page
}

fn page_appearance(
    provider: CssProvider,
    active_rgb: Rc<RefCell<((f64, f64, f64), (f64, f64, f64))>>,
) -> GtkBox {
    let page = GtkBox::new(Orientation::Vertical, 12);
    page.add_css_class("forge-page");
    page.append(&page_header(
        "Appearance & Color Themes",
        "Switch between curated Forge OS color combinations instantly.",
    ));

    let cur_p = load_saved_preset();
    let status = Label::new(Some(&format!(
        "Active OS theme: {} ({} on {})",
        cur_p.name, cur_p.accent, cur_p.bg
    )));
    status.add_css_class("forge-mono");
    status.set_halign(Align::Start);

    for &preset in PRESETS {
        let card = GtkBox::new(Orientation::Horizontal, 12);
        card.add_css_class("forge-card");

        let info = GtkBox::new(Orientation::Vertical, 2);
        info.set_hexpand(true);
        let name = Label::new(Some(preset.name));
        name.set_halign(Align::Start);
        let hex = Label::new(Some(&format!(
            "Accent {}  |  Highlight {}  |  Background {}",
            preset.accent, preset.accent_hi, preset.bg
        )));
        hex.add_css_class("forge-mono");
        hex.set_halign(Align::Start);
        info.append(&name);
        info.append(&hex);

        let apply_btn = Button::with_label("Apply Theme");
        apply_btn.add_css_class("forge-accent");
        let prov_c = provider.clone();
        let rgb_c = active_rgb.clone();
        let st_c = status.clone();
        apply_btn.connect_clicked(move |_| {
            prov_c.load_from_data(&css_for(preset));
            *rgb_c.borrow_mut() = (preset.rgb, preset.rgb_hi);
            save_user_gtk_theme(preset);
            st_c.set_text(&format!(
                "Applied OS-wide: {} ({} on {}) — GTK4, GTK3, Waybar, labwc, foot & all Forge apps updated.",
                preset.name, preset.accent, preset.bg
            ));
        });

        card.append(&info);
        card.append(&apply_btn);
        page.append(&card);
    }

    page.append(&status);
    page
}

fn load_saved_preset() -> ThemePreset {
    let home = env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    let env_file = PathBuf::from(home).join(".config/forge/active-theme.env");
    if let Ok(txt) = fs::read_to_string(env_file) {
        for line in txt.lines() {
            if let Some(name) = line.strip_prefix("THEME_NAME=") {
                if let Some(&p) = PRESETS.iter().find(|p| p.name == name.trim()) {
                    return p;
                }
            }
        }
    }
    PRESETS[0]
}

fn save_user_gtk_theme(preset: ThemePreset) {
    let home = env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    let home_p = PathBuf::from(&home);

    let forge_dir = home_p.join(".config/forge");
    let _ = fs::create_dir_all(&forge_dir);
    let env_txt = format!(
        "THEME_NAME={}\nACCENT={}\nACCENT_HI={}\nBG={}\n",
        preset.name, preset.accent, preset.accent_hi, preset.bg
    );
    let _ = fs::write(forge_dir.join("active-theme.env"), env_txt);

    let css = format!(
        "/* Forge OS Global Theme: {} */\n\
         @define-color window_bg_color {};\n\
         @define-color window_fg_color {};\n\
         @define-color view_bg_color {};\n\
         @define-color view_fg_color {};\n\
         @define-color headerbar_bg_color {};\n\
         @define-color headerbar_fg_color {};\n\
         @define-color accent_bg_color {};\n\
         @define-color accent_color {};\n\
         @define-color accent_fg_color #FFFFFF;\n\
         button {{ border: 1px solid {}; border-radius: 8px; }}\n\
         button:hover, button:focus {{ border-color: {}; }}\n\
         entry, searchentry, spinbutton {{ border: 1px solid {}; border-radius: 8px; }}\n\
         entry:focus-within, searchentry:focus-within {{ border-color: {}; }}\n\
         scale highlight, progressbar progress {{ background-color: {}; }}\n\
         scale slider {{ border: 2px solid {}; }}\n\
         switch {{ border: 1px solid {}; }}\n\
         switch:checked {{ background-color: {}; border-color: {}; }}\n\
         scrollbar slider:hover {{ background-color: {}; }}\n",
        preset.name,
        preset.bg,
        forge_brand::FG,
        preset.bg,
        forge_brand::FG,
        preset.bg,
        forge_brand::FG,
        preset.accent,
        preset.accent_hi,
        preset.accent,
        preset.accent_hi,
        preset.accent,
        preset.accent_hi,
        preset.accent,
        preset.accent,
        preset.accent,
        preset.accent,
        preset.accent_hi,
        preset.accent,
    );
    for sub in [".config/gtk-4.0", ".config/gtk-3.0"] {
        let dir = home_p.join(sub);
        let _ = fs::create_dir_all(&dir);
        let _ = fs::write(dir.join("gtk.css"), &css);
    }

    let script_candidates = [
        "/usr/local/bin/forge-apply-theme".to_string(),
        format!(
            "{}/../../scripts/forge-apply-theme.sh",
            env!("CARGO_MANIFEST_DIR")
        ),
    ];
    for cand in script_candidates {
        if PathBuf::from(&cand).exists() {
            let _ = Command::new("bash")
                .args([
                    &cand,
                    preset.name,
                    preset.accent,
                    preset.accent_hi,
                    preset.bg,
                ])
                .spawn();
            break;
        }
    }
}

fn page_about() -> GtkBox {
    let page = GtkBox::new(Orientation::Vertical, 12);
    page.add_css_class("forge-page");
    page.append(&page_header(
        forge_brand::PRETTY_NAME,
        "Honest upstream lineage (N6) with single-click recovery.",
    ));

    let card = GtkBox::new(Orientation::Vertical, 6);
    card.add_css_class("forge-card");
    for line in [
        format!(
            "Product      : {} {}",
            forge_brand::NAME,
            forge_brand::VERSION
        ),
        format!("Codename     : {}", forge_brand::CODENAME),
        format!(
            "Upstream     : {} ({})",
            forge_brand::ID_LIKE,
            forge_brand::UPSTREAM_DOC
        ),
        format!("Home         : {}", forge_brand::HOME_URL),
    ] {
        let l = Label::new(Some(&line));
        l.add_css_class("forge-mono");
        l.set_halign(Align::Start);
        card.append(&l);
    }

    let track_btn = Button::with_label("View Recent Command Errors (forge-track)");
    let watch_btn = Button::with_label("Scan for Background Attackers (forge-watch)");
    let sec_btn = Button::with_label("Run Security & Firewall Audit");
    let sec_lbl = Label::new(Some(""));
    sec_lbl.add_css_class("forge-mono");
    sec_lbl.set_halign(Align::Start);
    let tr_c = sec_lbl.clone();
    track_btn.connect_clicked(move |_| match Command::new("forge-track").output() {
        Ok(o) => tr_c.set_text(String::from_utf8_lossy(&o.stdout).trim()),
        Err(_) => tr_c.set_text("Run in terminal: forge-track"),
    });
    let wa_c = sec_lbl.clone();
    watch_btn.connect_clicked(move |_| match Command::new("forge-watch").output() {
        Ok(o) => wa_c.set_text(String::from_utf8_lossy(&o.stdout).trim()),
        Err(_) => wa_c.set_text("Run in terminal: forge-watch"),
    });
    let sec_c = sec_lbl.clone();
    sec_btn.connect_clicked(
        move |_| match Command::new("forge-security").arg("status").output() {
            Ok(o) => sec_c.set_text(String::from_utf8_lossy(&o.stdout).trim()),
            Err(_) => sec_c.set_text("Run in terminal: forge-security status"),
        },
    );

    let factory_btn =
        Button::with_label("Restore Core OS to Factory Defaults (Keeps User Files Safe)");
    factory_btn.add_css_class("forge-accent");
    let fac_c = sec_lbl.clone();
    factory_btn.connect_clicked(move |_| {
        match Command::new("pkexec")
            .args(["/usr/local/bin/forge-repair", "factory"])
            .output()
        {
            Ok(o) if o.status.success() => {
                fac_c.set_text("Core OS restored to factory state! User files untouched.")
            }
            Ok(o) => fac_c.set_text(String::from_utf8_lossy(&o.stderr).trim()),
            Err(_) => fac_c.set_text("Run in terminal: sudo forge-repair factory"),
        }
    });

    let repair_btn = Button::with_label("Repair System Configs");
    repair_btn.add_css_class("forge-accent");
    repair_btn.connect_clicked(|_| {
        let _ = Command::new("forge-repair").arg("configs").spawn();
    });

    let export_btn = Button::with_label("Export My Forge Profile (Backup Apps & Dotfiles)");
    let export_lbl = Label::new(Some(""));
    export_lbl.add_css_class("forge-mono");
    export_lbl.set_halign(Align::Start);
    let exp_c = export_lbl.clone();
    export_btn.connect_clicked(move |_| {
        match Command::new("forge-profile").arg("export").output() {
            Ok(o) if o.status.success() => {
                exp_c.set_text(String::from_utf8_lossy(&o.stdout).trim());
            }
            _ => exp_c.set_text("Export ready via CLI: forge-profile export"),
        }
    });

    page.append(&card);
    page.append(&track_btn);
    page.append(&watch_btn);
    page.append(&sec_btn);
    page.append(&sec_lbl);
    page.append(&factory_btn);
    page.append(&repair_btn);
    page.append(&export_btn);
    page.append(&export_lbl);
    page
}
