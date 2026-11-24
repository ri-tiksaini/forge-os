#![forbid(unsafe_code)]

//! Standalone Forge OS Developer Labs (Full Core System Edition):
//! - Separate Master Wiring Helpbook popup window with search/filter & ★ MAIN hub highlights
//! - Dedicated Virtual OS Test & Error Report popup window showing every PASS/FAIL test + exact reason & fix hint
//! - Category filter tabs (All / Scripts / UI / Systemd / Rust) + instant search
//! - Custom Dark Cherry file-type SVG icons + human names + real code loaded via live OS or repo root

use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::process::Command;
use std::rc::Rc;

use gtk4::gdk;
use gtk4::glib;
use gtk4::prelude::*;
use gtk4::{
    Align, Application, ApplicationWindow, Box as GtkBox, Button, CssProvider, Entry, Image, Label,
    ListBox, ListBoxRow, Orientation, Paned, ScrolledWindow, SelectionMode, Stack, TextView,
    Window,
};

use crate::sim::{
    apply_staged_to_live, default_core_files, load_core_file, run_virtual_os_simulation,
    CoreFileSpec, FileCategory, FileKind, SimReport,
};

const MASTER_ARCH: &str = include_str!("../../../data/theme/master-architecture.txt");
const SCRIPT_SVG: &str = include_str!("../../../data/theme/icons/lab-script.svg");
const CONFIG_SVG: &str = include_str!("../../../data/theme/icons/lab-config.svg");
const STYLE_SVG: &str = include_str!("../../../data/theme/icons/lab-style.svg");
const RUST_SVG: &str = include_str!("../../../data/theme/icons/lab-rust.svg");
const HELP_SVG: &str = include_str!("../../../data/theme/icons/forge-help.svg");

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

fn css() -> String {
    let (bg, accent, accent_hi) = load_active_theme();
    let fg = forge_brand::FG;
    let ui = forge_brand::FONT_UI;
    let mono = forge_brand::FONT_MONO;
    format!(
        "window {{ background-color: {bg}; color: {fg}; font-family: '{ui}'; }}\n\
         .lab-warn {{ background-color: #16090C; border: 2px solid {accent}; border-radius: 16px; padding: 28px; }}\n\
         .lab-h1 {{ font-size: 19px; font-weight: 800; color: {fg}; }}\n\
         .lab-section {{ font-size: 12px; font-weight: 700; color: {accent_hi}; letter-spacing: 0.5px; }}\n\
         .lab-name {{ font-size: 13px; font-weight: 700; color: {fg}; }}\n\
         .lab-main-name {{ font-size: 13px; font-weight: 800; color: {accent_hi}; }}\n\
         .lab-mono {{ font-family: '{mono}'; font-size: 12px; color: #D4D4D8; }}\n\
         .lab-muted {{ font-size: 11px; color: #9A9AA6; }}\n\
         .lab-staged-box {{ background-color: #121217; border: 1px solid {accent}; border-radius: 10px; padding: 10px; }}\n\
         .sim-banner-pass {{ background-color: #0D2218; border: 1px solid #1E7E4A; border-radius: 10px; padding: 12px; }}\n\
         .sim-banner-fail {{ background-color: #2A0B10; border: 1px solid #E53955; border-radius: 10px; padding: 12px; }}\n\
         .sim-card-pass {{ background-color: #101614; border-left: 4px solid #27AE60; border-radius: 8px; padding: 10px; }}\n\
         .sim-card-fail {{ background-color: #1C0D10; border-left: 4px solid #E53955; border-radius: 8px; padding: 10px; }}\n\
         .badge-pass {{ color: #4ADE80; font-weight: 800; font-size: 12px; }}\n\
         .badge-fail {{ color: #FF5C77; font-weight: 800; font-size: 12px; }}\n\
         .sim-hint {{ color: #F5B041; font-size: 11px; font-family: '{mono}'; }}\n\
         button {{ background-color: #16161D; color: {fg}; border: 1px solid {accent}; border-radius: 8px; padding: 5px 10px; }}\n\
         button:hover {{ background-color: #240B10; border-color: {accent_hi}; }}\n\
         .lab-tab {{ font-size: 11px; padding: 4px 8px; }}\n\
         .lab-tab-active {{ background-color: #2A0D14; border-color: {accent_hi}; color: #FFFFFF; font-weight: 700; }}\n\
         .lab-accent {{ background-color: {accent}; color: #FFFFFF; font-weight: 700; border: 1px solid {accent_hi}; border-radius: 8px; padding: 6px 14px; }}\n\
         .lab-accent:hover {{ background-color: {accent_hi}; }}\n\
         entry {{ background-color: #121217; color: {fg}; border: 1px solid {accent}; border-radius: 8px; padding: 5px 10px; }}\n\
         entry:focus-within {{ border-color: {accent_hi}; }}\n\
         textview {{ font-family: '{mono}'; font-size: 12px; background-color: #0B0B10; color: {fg}; padding: 10px; }}\n\
         list {{ background-color: #0D0D12; }}\n\
         list row {{ padding: 6px 8px; border-radius: 8px; margin: 2px 4px; }}\n\
         list row:selected {{ background-color: #240B10; border-left: 3px solid {accent}; }}\n"
    )
}

fn svg_icon(svg: &'static str, size: i32) -> Image {
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
    Image::from_icon_name("text-x-generic")
}

fn open_helpbook_window(parent: &ApplicationWindow) {
    let win = Window::builder()
        .transient_for(parent)
        .title("Forge OS — Master Architecture, Complete File Tree & Wiring Helpbook")
        .default_width(980)
        .default_height(700)
        .build();

    let outer = GtkBox::new(Orientation::Vertical, 8);
    outer.set_margin_top(10);
    outer.set_margin_bottom(10);
    outer.set_margin_start(10);
    outer.set_margin_end(10);

    let top = GtkBox::new(Orientation::Horizontal, 8);
    let search = Entry::new();
    search.set_placeholder_text(Some(
        "Filter guide (e.g. ★ MAIN, waybar, forged, theme, labwc, repair, plymouth)...",
    ));
    search.set_hexpand(true);
    let main_only_btn = Button::with_label("★ Main Hubs");
    let reset_btn = Button::with_label("Show Full Guide");
    top.append(&search);
    top.append(&main_only_btn);
    top.append(&reset_btn);
    outer.append(&top);

    let tv = TextView::new();
    tv.set_editable(false);
    tv.buffer().set_text(MASTER_ARCH);
    let scroller = ScrolledWindow::builder()
        .hexpand(true)
        .vexpand(true)
        .child(&tv)
        .build();
    outer.append(&scroller);

    let tv_s = tv.clone();
    search.connect_changed(move |entry| {
        let q = entry.text().trim().to_lowercase();
        if q.is_empty() {
            tv_s.buffer().set_text(MASTER_ARCH);
            return;
        }
        let mut matched = Vec::new();
        for line in MASTER_ARCH.lines() {
            if line.to_lowercase().contains(&q)
                || line.starts_with("===")
                || line.starts_with("---")
            {
                matched.push(line);
            }
        }
        if matched.is_empty() {
            tv_s.buffer()
                .set_text("No matching wiring sections found. Click 'Show Full Guide' to reset.");
        } else {
            tv_s.buffer().set_text(&matched.join("\n"));
        }
    });

    let se_m = search.clone();
    main_only_btn.connect_clicked(move |_| {
        se_m.set_text("★");
    });

    let se_r = search;
    reset_btn.connect_clicked(move |_| {
        se_r.set_text("");
    });

    win.set_child(Some(&outer));
    win.present();
}

fn open_sim_report_window(parent: &ApplicationWindow, report: &SimReport) {
    let win = Window::builder()
        .transient_for(parent)
        .title("Forge OS — Virtual OS Simulation Test & Error Report")
        .default_width(820)
        .default_height(580)
        .build();

    let outer = GtkBox::new(Orientation::Vertical, 10);
    outer.set_margin_top(12);
    outer.set_margin_bottom(12);
    outer.set_margin_start(12);
    outer.set_margin_end(12);

    let pass_cnt = report.checks.iter().filter(|c| c.passed).count();
    let fail_cnt = report.checks.len().saturating_sub(pass_cnt);

    // Summary Banner
    let banner = GtkBox::new(Orientation::Vertical, 4);
    banner.add_css_class(if report.passed {
        "sim-banner-pass"
    } else {
        "sim-banner-fail"
    });
    let headline = if report.passed {
        format!(
            "VIRTUAL OS SIMULATION PASSED  —  {pass_cnt}/{} Tests Passed",
            report.checks.len()
        )
    } else {
        format!("VIRTUAL OS SIMULATION FAILED  —  {fail_cnt} Failed, {pass_cnt} Passed")
    };
    let h_lbl = Label::new(Some(&headline));
    h_lbl.add_css_class("lab-h1");
    h_lbl.set_halign(Align::Start);

    let sub = if report.passed {
        "Every syntax, security, and wiring check passed in the isolated Virtual OS. You can safely click '2. Patch Live OS'."
    } else {
        "Live OS patching is blocked until every [FAIL] test below is fixed and re-tested."
    };
    let sub_lbl = Label::new(Some(sub));
    sub_lbl.add_css_class("lab-mono");
    sub_lbl.set_wrap(true);
    sub_lbl.set_halign(Align::Start);
    banner.append(&h_lbl);
    banner.append(&sub_lbl);
    outer.append(&banner);

    // Scrollable Individual Test Cards
    let cards_box = GtkBox::new(Orientation::Vertical, 8);
    for chk in &report.checks {
        let card = GtkBox::new(Orientation::Vertical, 4);
        card.add_css_class(if chk.passed {
            "sim-card-pass"
        } else {
            "sim-card-fail"
        });

        let top_row = GtkBox::new(Orientation::Horizontal, 8);
        let badge = Label::new(Some(if chk.passed { "[PASS]" } else { "[FAIL]" }));
        badge.add_css_class(if chk.passed {
            "badge-pass"
        } else {
            "badge-fail"
        });
        let name_lbl = Label::new(Some(&chk.test_name));
        name_lbl.add_css_class("lab-name");
        name_lbl.set_halign(Align::Start);
        let tgt_lbl = Label::new(Some(&format!("({})", chk.target)));
        tgt_lbl.add_css_class("lab-muted");
        tgt_lbl.set_halign(Align::Start);
        top_row.append(&badge);
        top_row.append(&name_lbl);
        top_row.append(&tgt_lbl);
        card.append(&top_row);

        let reason_lbl = Label::new(Some(&format!("Reason: {}", chk.reason)));
        reason_lbl.add_css_class("lab-mono");
        reason_lbl.set_wrap(true);
        reason_lbl.set_halign(Align::Start);
        card.append(&reason_lbl);

        if !chk.passed && !chk.fix_hint.is_empty() {
            let hint_lbl = Label::new(Some(&format!("How to fix: {}", chk.fix_hint)));
            hint_lbl.add_css_class("sim-hint");
            hint_lbl.set_wrap(true);
            hint_lbl.set_halign(Align::Start);
            card.append(&hint_lbl);
        }

        cards_box.append(&card);
    }

    let scroller = ScrolledWindow::builder()
        .hexpand(true)
        .vexpand(true)
        .child(&cards_box)
        .build();
    outer.append(&scroller);

    let close_btn = Button::with_label("Close Report");
    close_btn.set_halign(Align::End);
    let win_c = win.clone();
    close_btn.connect_clicked(move |_| win_c.close());
    outer.append(&close_btn);

    win.set_child(Some(&outer));
    win.present();
}

pub fn run() {
    let app = Application::builder()
        .application_id("org.forge.lab")
        .build();

    app.connect_activate(|app| {
        let provider = CssProvider::new();
        provider.load_from_data(&css());
        if let Some(display) = gdk::Display::default() {
            gtk4::style_context_add_provider_for_display(
                &display,
                &provider,
                gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
            );
        }

        let window = ApplicationWindow::builder()
            .application(app)
            .title("Forge OS — Developer Labs")
            .default_width(1200)
            .default_height(720)
            .build();

        let root_stack = Stack::new();
        let staged: Rc<RefCell<BTreeMap<String, String>>> = Rc::new(RefCell::new(BTreeMap::new()));
        let sim_passed: Rc<RefCell<bool>> = Rc::new(RefCell::new(false));
        let last_report: Rc<RefCell<Option<SimReport>>> = Rc::new(RefCell::new(None));

        let gate = build_auth_gate(root_stack.clone(), window.clone());
        let workspace = build_lab_workspace(window.clone(), staged, sim_passed, last_report);

        root_stack.add_named(&gate, Some("gate"));
        root_stack.add_named(&workspace, Some("workspace"));
        root_stack.set_visible_child_name("gate");

        window.set_child(Some(&root_stack));
        window.present();
    });

    app.run_with_args::<&str>(&[]);
}

fn build_auth_gate(stack: Stack, win: ApplicationWindow) -> GtkBox {
    let outer = GtkBox::new(Orientation::Vertical, 16);
    outer.set_halign(Align::Center);
    outer.set_valign(Align::Center);

    let card = GtkBox::new(Orientation::Vertical, 14);
    card.add_css_class("lab-warn");
    card.set_width_request(560);

    let title = Label::new(Some("DEVELOPER LABS — SYSTEM INTERNALS WARNING"));
    title.add_css_class("lab-h1");
    title.set_halign(Align::Start);

    let msg = Label::new(Some(
        "You are about to open ALL Core System Files (scripts, UI/theme, systemd, kernel & Rust source).\n\n\
         • User data in /home is strictly excluded and kept safe.\n\
         • Multi-file edits are tested inside an isolated Virtual OS copy first.\n\
         • Settings -> About & Repair includes 1-click Factory Core Restore if needed.",
    ));
    msg.set_wrap(true);
    msg.add_css_class("lab-mono");
    msg.set_halign(Align::Start);

    let status = Label::new(Some(""));
    status.add_css_class("lab-mono");
    status.set_halign(Align::Start);

    let btn_row = GtkBox::new(Orientation::Horizontal, 10);
    let auth_btn = Button::with_label("Authenticate & Enter Developer Labs");
    auth_btn.add_css_class("lab-accent");
    let cancel_btn = Button::with_label("Cancel");

    let st_c = status.clone();
    let stack_c = stack;
    auth_btn.connect_clicked(move |_| {
        let ok = Command::new("pkexec")
            .args(["true"])
            .status()
            .is_ok_and(|s| s.success())
            || Command::new("sudo")
                .args(["-n", "true"])
                .status()
                .is_ok_and(|s| s.success());
        if ok {
            stack_c.set_visible_child_name("workspace");
        } else {
            st_c.set_text("Authentication cancelled or failed.");
        }
    });

    cancel_btn.connect_clicked(move |_| win.close());

    btn_row.append(&auth_btn);
    btn_row.append(&cancel_btn);
    card.append(&title);
    card.append(&msg);
    card.append(&btn_row);
    card.append(&status);
    outer.append(&card);
    outer
}

fn append_file_row(list: &ListBox, spec: &CoreFileSpec, is_staged: bool) {
    let row_box = GtkBox::new(Orientation::Horizontal, 10);
    let svg = match spec.kind {
        FileKind::Script => SCRIPT_SVG,
        FileKind::Config => CONFIG_SVG,
        FileKind::Style => STYLE_SVG,
        FileKind::Rust => RUST_SVG,
    };
    row_box.append(&svg_icon(svg, 26));

    let text_box = GtkBox::new(Orientation::Vertical, 1);
    let prefix = if is_staged {
        "● "
    } else if spec.is_main_hub {
        "★ "
    } else {
        ""
    };
    let name_lbl = Label::new(Some(&format!("{prefix}{}", spec.display_name)));
    if spec.is_main_hub {
        name_lbl.add_css_class("lab-main-name");
    } else {
        name_lbl.add_css_class("lab-name");
    }
    name_lbl.set_halign(Align::Start);

    let short = spec.path.rsplit('/').next().unwrap_or(&spec.path);
    let hub_tag = if spec.is_main_hub { " · MAIN" } else { "" };
    let sub_lbl = Label::new(Some(&format!(
        "{short}  ·  {}{hub_tag}",
        spec.category.label()
    )));
    sub_lbl.add_css_class("lab-muted");
    sub_lbl.set_halign(Align::Start);

    text_box.append(&name_lbl);
    text_box.append(&sub_lbl);
    row_box.append(&text_box);

    let row = ListBoxRow::new();
    row.set_child(Some(&row_box));
    list.append(&row);
}

fn repopulate_sidebar(
    list: &ListBox,
    items: &[CoreFileSpec],
    visible_indices: &Rc<RefCell<Vec<usize>>>,
    staged: &BTreeMap<String, String>,
    category_filter: Option<FileCategory>,
    query: &str,
    count_lbl: &Label,
) {
    while let Some(child) = list.first_child() {
        list.remove(&child);
    }
    let q = query.trim().to_lowercase();
    let mut new_vis = Vec::new();

    for (idx, spec) in items.iter().enumerate() {
        if let Some(cat) = category_filter {
            if spec.category != cat {
                continue;
            }
        }
        if !q.is_empty() {
            let hay = format!(
                "{} {} {} {} {}",
                spec.display_name,
                spec.path,
                spec.repo_fallback,
                spec.role,
                if spec.is_main_hub { "main star" } else { "" }
            )
            .to_lowercase();
            if !hay.contains(&q) {
                continue;
            }
        }
        new_vis.push(idx);
        append_file_row(list, spec, staged.contains_key(&spec.path));
    }

    let vis_len = new_vis.len();
    *visible_indices.borrow_mut() = new_vis;

    count_lbl.set_text(&format!(
        "Showing {vis_len} of {} core files (★ = Main Hub)",
        items.len()
    ));
    if let Some(r0) = list.row_at_index(0) {
        list.select_row(Some(&r0));
    }
}

fn load_spec_into_editor(
    spec: &CoreFileSpec,
    staged: &BTreeMap<String, String>,
    active_lbl: &Label,
    source_lbl: &Label,
    code_view: &TextView,
    loading_guard: &Cell<bool>,
) {
    loading_guard.set(true);
    let star = if spec.is_main_hub { "★ " } else { "" };
    if let Some(draft) = staged.get(&spec.path) {
        let lines = draft.lines().count();
        active_lbl.set_text(&format!(
            "{star}{}  [STAGED]  —  {}",
            spec.display_name, spec.role
        ));
        source_lbl.set_text(&format!(
            "Target: {}  |  Source: STAGED DRAFT  |  {lines} lines",
            spec.path
        ));
        code_view.buffer().set_text(draft);
    } else {
        let loaded = load_core_file(spec);
        let lines = loaded.content.lines().count();
        active_lbl.set_text(&format!("{star}{}  —  {}", spec.display_name, spec.role));
        source_lbl.set_text(&format!(
            "Target: {}  |  Loaded from: {} ({})  |  {lines} lines",
            spec.path, loaded.source, loaded.resolved_path
        ));
        code_view.buffer().set_text(&loaded.content);
    }
    loading_guard.set(false);
}

fn build_lab_workspace(
    win: ApplicationWindow,
    staged: Rc<RefCell<BTreeMap<String, String>>>,
    sim_passed: Rc<RefCell<bool>>,
    last_report: Rc<RefCell<Option<SimReport>>>,
) -> GtkBox {
    let root = GtkBox::new(Orientation::Vertical, 8);
    root.set_margin_top(10);
    root.set_margin_bottom(10);
    root.set_margin_start(10);
    root.set_margin_end(10);

    // Top Toolbar
    let toolbar = GtkBox::new(Orientation::Horizontal, 8);

    let help_btn = Button::new();
    let help_inner = GtkBox::new(Orientation::Horizontal, 6);
    help_inner.append(&svg_icon(HELP_SVG, 20));
    help_inner.append(&Label::new(Some("Master Wiring Guide")));
    help_btn.set_child(Some(&help_inner));
    let win_h = win.clone();
    help_btn.connect_clicked(move |_| open_helpbook_window(&win_h));

    let new_name = Entry::new();
    new_name.set_placeholder_text(Some("new-tool-name"));
    new_name.set_hexpand(true);
    let create_btn = Button::with_label("+ Create Tool");
    let sim_btn = Button::with_label("1. Test in Virtual OS");
    sim_btn.add_css_class("lab-accent");
    let patch_btn = Button::with_label("2. Patch Live OS");
    let reboot_btn = Button::with_label("Restart OS");

    toolbar.append(&help_btn);
    toolbar.append(&new_name);
    toolbar.append(&create_btn);
    toolbar.append(&sim_btn);
    toolbar.append(&patch_btn);
    toolbar.append(&reboot_btn);
    root.append(&toolbar);

    // Left Sidebar
    let file_items: Rc<RefCell<Vec<CoreFileSpec>>> = Rc::new(RefCell::new(default_core_files()));
    let visible_indices: Rc<RefCell<Vec<usize>>> = Rc::new(RefCell::new(Vec::new()));
    let active_cat: Rc<RefCell<Option<FileCategory>>> = Rc::new(RefCell::new(None));
    let active_idx: Rc<RefCell<usize>> = Rc::new(RefCell::new(0usize));

    let left_box = GtkBox::new(Orientation::Vertical, 6);

    let search_entry = Entry::new();
    search_entry.set_placeholder_text(Some("Filter 65+ core files (or type 'main')..."));
    left_box.append(&search_entry);

    let cat_row = GtkBox::new(Orientation::Horizontal, 4);
    let btn_all = Button::with_label("All");
    let btn_scripts = Button::with_label("Scripts");
    let btn_ui = Button::with_label("UI");
    let btn_sys = Button::with_label("Systemd");
    let btn_rust = Button::with_label("Rust");
    for b in [&btn_all, &btn_scripts, &btn_ui, &btn_sys, &btn_rust] {
        b.add_css_class("lab-tab");
        cat_row.append(b);
    }
    btn_all.add_css_class("lab-tab-active");
    left_box.append(&cat_row);

    let count_lbl = Label::new(Some(""));
    count_lbl.add_css_class("lab-muted");
    count_lbl.set_halign(Align::Start);
    left_box.append(&count_lbl);

    let file_list = ListBox::new();
    file_list.set_selection_mode(SelectionMode::Single);
    let left_scroll = ScrolledWindow::builder()
        .min_content_width(305)
        .vexpand(true)
        .child(&file_list)
        .build();
    left_box.append(&left_scroll);

    // Right Pane
    let active_lbl = Label::new(Some(""));
    active_lbl.add_css_class("lab-section");
    active_lbl.set_halign(Align::Start);

    let source_lbl = Label::new(Some(""));
    source_lbl.add_css_class("lab-muted");
    source_lbl.set_halign(Align::Start);

    let loading_guard: Rc<Cell<bool>> = Rc::new(Cell::new(false));
    let code_view = TextView::new();
    {
        let lg_ch = loading_guard.clone();
        let sp_ch = sim_passed.clone();
        code_view.buffer().connect_changed(move |_| {
            if !lg_ch.get() {
                *sp_ch.borrow_mut() = false;
            }
        });
    }
    let code_scroll = ScrolledWindow::builder()
        .min_content_height(330)
        .vexpand(true)
        .child(&code_view)
        .build();

    let edit_bar = GtkBox::new(Orientation::Horizontal, 8);
    let stage_btn = Button::with_label("Save & Stage This File");
    stage_btn.add_css_class("lab-accent");
    let next_btn = Button::with_label("Next File →");
    let report_btn = Button::with_label("View Test Report");
    let clear_btn = Button::with_label("Clear Staged");
    edit_bar.append(&stage_btn);
    edit_bar.append(&next_btn);
    edit_bar.append(&report_btn);
    edit_bar.append(&clear_btn);

    let staged_card = GtkBox::new(Orientation::Vertical, 4);
    staged_card.add_css_class("lab-staged-box");
    let staged_hdr = Label::new(Some("STAGED CHANGES READY FOR VIRTUAL OS SIMULATION (0)"));
    staged_hdr.add_css_class("lab-section");
    staged_hdr.set_halign(Align::Start);
    let staged_list_lbl = Label::new(Some(
        "No files edited yet. Edit any file above and click 'Save & Stage This File'.",
    ));
    staged_list_lbl.set_wrap(true);
    staged_list_lbl.add_css_class("lab-mono");
    staged_list_lbl.set_halign(Align::Start);
    staged_card.append(&staged_hdr);
    staged_card.append(&staged_list_lbl);

    let sim_console = TextView::new();
    sim_console.set_editable(false);
    sim_console.buffer().set_text(
        "[READY] Click any file (★ = Main Hub), edit code, click 'Save & Stage This File', then '1. Test in Virtual OS' for the full Pass/Fail report.\n",
    );
    let sim_scroll = ScrolledWindow::builder()
        .min_content_height(110)
        .child(&sim_console)
        .build();

    let right_box = GtkBox::new(Orientation::Vertical, 6);
    right_box.append(&active_lbl);
    right_box.append(&source_lbl);
    right_box.append(&code_scroll);
    right_box.append(&edit_bar);
    right_box.append(&staged_card);
    right_box.append(&sim_scroll);

    let paned = Paned::new(Orientation::Horizontal);
    paned.set_start_child(Some(&left_box));
    paned.set_end_child(Some(&right_box));
    paned.set_position(325);
    paned.set_vexpand(true);
    root.append(&paned);

    // Wire selection handler BEFORE initial populate
    let items_sel = file_items.clone();
    let vis_sel = visible_indices.clone();
    let idx_sel = active_idx.clone();
    let al_sel = active_lbl.clone();
    let sl_sel = source_lbl.clone();
    let cv_sel = code_view.clone();
    let st_sel = staged.clone();
    let lg_sel = loading_guard.clone();
    file_list.connect_row_selected(move |_, row| {
        let Some(r) = row else { return };
        let row_i = r.index() as usize;
        let Some(&real_idx) = vis_sel.borrow().get(row_i) else {
            return;
        };
        if let Some(spec) = items_sel.borrow().get(real_idx) {
            *idx_sel.borrow_mut() = real_idx;
            load_spec_into_editor(spec, &st_sel.borrow(), &al_sel, &sl_sel, &cv_sel, &lg_sel);
        }
    });

    repopulate_sidebar(
        &file_list,
        &file_items.borrow(),
        &visible_indices,
        &staged.borrow(),
        None,
        "",
        &count_lbl,
    );

    // Category Tabs
    let tab_buttons = vec![
        (btn_all, None),
        (btn_scripts, Some(FileCategory::Scripts)),
        (btn_ui, Some(FileCategory::UiBoot)),
        (btn_sys, Some(FileCategory::SystemdKernel)),
        (btn_rust, Some(FileCategory::RustCore)),
    ];
    let all_tab_refs: Vec<Button> = tab_buttons.iter().map(|(b, _)| b.clone()).collect();
    for (btn, cat_opt) in tab_buttons {
        let all_b = all_tab_refs.clone();
        let ac_c = active_cat.clone();
        let fl_c = file_list.clone();
        let fi_c = file_items.clone();
        let vi_c = visible_indices.clone();
        let st_c = staged.clone();
        let se_c = search_entry.clone();
        let cl_c = count_lbl.clone();
        let b_self = btn.clone();
        btn.connect_clicked(move |_| {
            for b in &all_b {
                b.remove_css_class("lab-tab-active");
            }
            b_self.add_css_class("lab-tab-active");
            *ac_c.borrow_mut() = cat_opt;
            repopulate_sidebar(
                &fl_c,
                &fi_c.borrow(),
                &vi_c,
                &st_c.borrow(),
                cat_opt,
                &se_c.text(),
                &cl_c,
            );
        });
    }

    // Search Box
    let ac_s = active_cat.clone();
    let fl_s = file_list.clone();
    let fi_s = file_items.clone();
    let vi_s = visible_indices.clone();
    let st_s = staged.clone();
    let cl_s = count_lbl.clone();
    search_entry.connect_changed(move |entry| {
        repopulate_sidebar(
            &fl_s,
            &fi_s.borrow(),
            &vi_s,
            &st_s.borrow(),
            *ac_s.borrow(),
            &entry.text(),
            &cl_s,
        );
    });

    // Next File button
    let fl_next = file_list.clone();
    let vi_next = visible_indices.clone();
    next_btn.connect_clicked(move |_| {
        let len = vi_next.borrow().len();
        if len == 0 {
            return;
        }
        let cur = fl_next
            .selected_row()
            .map(|r| r.index() as usize)
            .unwrap_or(0);
        let next_i = (cur + 1) % len;
        if let Some(r) = fl_next.row_at_index(next_i as i32) {
            fl_next.select_row(Some(&r));
        }
    });

    // View Last Test Report button
    let win_rep = win.clone();
    let lr_view = last_report.clone();
    let con_rep = sim_console.clone();
    report_btn.connect_clicked(move |_| {
        if let Some(ref rep) = *lr_view.borrow() {
            open_sim_report_window(&win_rep, rep);
        } else {
            con_rep
                .buffer()
                .set_text("[INFO] No Virtual OS test run yet. Stage an edit and click '1. Test in Virtual OS'.");
        }
    });

    // Create New Tool
    let nn_c = new_name;
    let fi_new = file_items.clone();
    let fl_new = file_list.clone();
    let vi_new = visible_indices.clone();
    let st_new = staged.clone();
    let ac_new = active_cat;
    let se_new = search_entry;
    let cl_new = count_lbl;
    let cv_new = code_view.clone();
    let lg_new = loading_guard;
    let con_new = sim_console.clone();
    create_btn.connect_clicked(move |_| {
        let clean: String = nn_c
            .text()
            .trim()
            .chars()
            .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
            .collect();
        if clean.is_empty() {
            con_new
                .buffer()
                .set_text("[ERROR] Enter a valid tool name (letters, numbers, - or _).");
            return;
        }
        let spec = CoreFileSpec {
            display_name: format!("Custom: {clean}"),
            path: format!("/usr/local/bin/{clean}"),
            repo_fallback: String::new(),
            role: "User-created core system tool".into(),
            kind: FileKind::Script,
            category: FileCategory::Scripts,
            is_main_hub: false,
        };
        fi_new.borrow_mut().push(spec.clone());
        *ac_new.borrow_mut() = None;
        se_new.set_text("");
        repopulate_sidebar(
            &fl_new,
            &fi_new.borrow(),
            &vi_new,
            &st_new.borrow(),
            None,
            "",
            &cl_new,
        );
        let last_row = (vi_new.borrow().len().saturating_sub(1)) as i32;
        if let Some(r) = fl_new.row_at_index(last_row) {
            fl_new.select_row(Some(&r));
        }
        lg_new.set(true);
        cv_new.buffer().set_text(&format!(
            "#!/usr/bin/env bash\n# Custom Forge OS Tool: {clean}\nset -euo pipefail\necho \"Running {clean}\"\n"
        ));
        lg_new.set(false);
        con_new.buffer().set_text(&format!(
            "[NEW] Created {}. Edit code and click 'Save & Stage This File'.",
            spec.path
        ));
    });

    // Save & Stage current file into batch
    let fi_stg = file_items.clone();
    let idx_stg = active_idx.clone();
    let cv_stg = code_view.clone();
    let st_stg = staged.clone();
    let sh_stg = staged_hdr.clone();
    let sl_stg = staged_list_lbl.clone();
    let sp_stg = sim_passed.clone();
    let con_stg = sim_console.clone();
    let al_stg = active_lbl.clone();
    let src_stg = source_lbl.clone();
    stage_btn.connect_clicked(move |_| {
        let idx = *idx_stg.borrow();
        let Some(spec) = fi_stg.borrow().get(idx).cloned() else {
            return;
        };
        let buf = cv_stg.buffer();
        let text = buf.text(&buf.start_iter(), &buf.end_iter(), false).to_string();
        let lines_cnt = text.lines().count();
        st_stg.borrow_mut().insert(spec.path.clone(), text);
        *sp_stg.borrow_mut() = false;

        let star = if spec.is_main_hub { "★ " } else { "" };
        al_stg.set_text(&format!("{star}{}  [STAGED]  —  {}", spec.display_name, spec.role));
        src_stg.set_text(&format!(
            "Target: {}  |  Source: STAGED DRAFT  |  {lines_cnt} lines",
            spec.path
        ));

        let map = st_stg.borrow();
        sh_stg.set_text(&format!(
            "STAGED CHANGES READY FOR VIRTUAL OS SIMULATION ({})",
            map.len()
        ));
        let lines: Vec<String> = map.keys().map(|k| format!("  ✓  {k}")).collect();
        sl_stg.set_text(&lines.join("\n"));
        con_stg.buffer().set_text(&format!(
            "[SAVED TO BATCH] {} ({}). Click 'Next File →' to edit more or '1. Test in Virtual OS'.",
            spec.display_name, spec.path
        ));
    });

    // Clear Staged
    let st_clr = staged.clone();
    let sh_clr = staged_hdr.clone();
    let sl_clr = staged_list_lbl.clone();
    let sp_clr = sim_passed.clone();
    let lr_clr = last_report.clone();
    let con_clr = sim_console.clone();
    clear_btn.connect_clicked(move |_| {
        st_clr.borrow_mut().clear();
        *sp_clr.borrow_mut() = false;
        *lr_clr.borrow_mut() = None;
        sh_clr.set_text("STAGED CHANGES READY FOR VIRTUAL OS SIMULATION (0)");
        sl_clr.set_text("No files edited yet.");
        con_clr
            .buffer()
            .set_text("[CLEARED] Discarded all staged edits.");
    });

    // Run Virtual OS Simulation + Open Detailed Pass/Fail Report Window
    // Auto-syncs the currently open editor buffer if edited so you never test stale code!
    let fi_sim = file_items;
    let idx_sim = active_idx;
    let cv_sim = code_view;
    let st_sim = staged.clone();
    let sh_sim = staged_hdr.clone();
    let sl_sim = staged_list_lbl.clone();
    let al_sim = active_lbl;
    let src_sim = source_lbl;
    let sp_sim = sim_passed.clone();
    let lr_sim = last_report;
    let con_sim = sim_console.clone();
    let win_sim = win;
    sim_btn.connect_clicked(move |_| {
        let cur_idx = *idx_sim.borrow();
        if let Some(spec) = fi_sim.borrow().get(cur_idx).cloned() {
            let buf = cv_sim.buffer();
            let current_text = buf.text(&buf.start_iter(), &buf.end_iter(), false).to_string();
            let orig_text = load_core_file(&spec).content;
            let already_staged = st_sim.borrow().contains_key(&spec.path);
            if already_staged || current_text != orig_text {
                let lines_cnt = current_text.lines().count();
                st_sim.borrow_mut().insert(spec.path.clone(), current_text);
                let star = if spec.is_main_hub { "★ " } else { "" };
                al_sim.set_text(&format!("{star}{}  [STAGED]  —  {}", spec.display_name, spec.role));
                src_sim.set_text(&format!(
                    "Target: {}  |  Source: STAGED DRAFT  |  {lines_cnt} lines",
                    spec.path
                ));
                let map_ref = st_sim.borrow();
                sh_sim.set_text(&format!(
                    "STAGED CHANGES READY FOR VIRTUAL OS SIMULATION ({})",
                    map_ref.len()
                ));
                let lines: Vec<String> = map_ref.keys().map(|k| format!("  ✓  {k}")).collect();
                sl_sim.set_text(&lines.join("\n"));
            }
        }

        let snapshot = st_sim.borrow().clone();
        if snapshot.is_empty() {
            con_sim
                .buffer()
                .set_text("[INFO] Edit any file in the editor (or click 'Save & Stage This File') before running Virtual OS Simulation.");
            return;
        }
        let report = run_virtual_os_simulation(&snapshot);
        *sp_sim.borrow_mut() = report.passed;
        con_sim.buffer().set_text(&report.logs.join("\n"));
        open_sim_report_window(&win_sim, &report);
        *lr_sim.borrow_mut() = Some(report);
    });

    // Patch Live OS
    let st_pat = staged;
    let sp_pat = sim_passed;
    let sh_pat = staged_hdr;
    let sl_pat = staged_list_lbl;
    let con_pat = sim_console;
    patch_btn.connect_clicked(move |_| {
        if !*sp_pat.borrow() {
            con_pat
                .buffer()
                .set_text("[BLOCKED] Code was edited or not tested yet. Run '1. Test in Virtual OS' and pass every check first.");
            return;
        }
        let snapshot = st_pat.borrow().clone();
        match apply_staged_to_live(&snapshot) {
            Ok(msg) => {
                st_pat.borrow_mut().clear();
                *sp_pat.borrow_mut() = false;
                sh_pat.set_text("STAGED CHANGES READY FOR VIRTUAL OS SIMULATION (0)");
                sl_pat.set_text("All staged changes were patched to the live OS.");
                con_pat.buffer().set_text(&msg);
            }
            Err(e) => con_pat.buffer().set_text(&e),
        }
    });

    reboot_btn.connect_clicked(|_| {
        let _ = Command::new("systemctl").arg("reboot").spawn();
    });

    root
}
