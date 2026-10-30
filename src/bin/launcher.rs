//! Claudium Launcher — a small desktop launcher in the spirit of the Minecraft
//! launcher (a brand rail beside a Play panel), themed in Claude's warm palette.
//! It collects options and starts the `claudium` game binary; no shared game code.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::PathBuf;
use std::process::Command;

use eframe::egui;
use egui::{Color32, FontId, Rect, RichText, Rounding, Sense, Stroke, TextStyle, pos2, vec2};

// --- Claude theme (burnt-sienna accent so it isn't the exact default coral) --

const PAPER: Color32 = Color32::from_rgb(241, 237, 228);
const CARD: Color32 = Color32::from_rgb(251, 250, 246);
const INK: Color32 = Color32::from_rgb(32, 29, 24);
const MUTED: Color32 = Color32::from_rgb(133, 125, 110);
const LINE: Color32 = Color32::from_rgb(227, 222, 210);
const ACCENT: Color32 = Color32::from_rgb(199, 95, 56);
const ACCENT_PRESS: Color32 = Color32::from_rgb(173, 78, 43);
const RAIL: Color32 = Color32::from_rgb(40, 36, 30);
const RAIL_FG: Color32 = Color32::from_rgb(236, 230, 219);
const RAIL_DIM: Color32 = Color32::from_rgb(154, 145, 131);
const SAGE: Color32 = Color32::from_rgb(94, 110, 79);

fn setup_style(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();
    use egui::FontFamily::Proportional;
    style.text_styles = [
        (TextStyle::Heading, FontId::new(30.0, Proportional)),
        (TextStyle::Body, FontId::new(15.5, Proportional)),
        (TextStyle::Button, FontId::new(15.5, Proportional)),
        (TextStyle::Small, FontId::new(12.5, Proportional)),
        (TextStyle::Monospace, FontId::new(13.0, egui::FontFamily::Monospace)),
    ]
    .into();

    let v = &mut style.visuals;
    v.panel_fill = PAPER;
    v.window_fill = CARD;
    v.extreme_bg_color = PAPER; // text-field background
    v.override_text_color = Some(INK);
    v.selection.bg_fill = ACCENT.linear_multiply(0.30);
    v.selection.stroke = Stroke::new(1.0, ACCENT);
    let r9 = Rounding::same(9.0);
    for w in [&mut v.widgets.inactive, &mut v.widgets.hovered, &mut v.widgets.active, &mut v.widgets.open] {
        w.rounding = r9;
        w.bg_fill = PAPER;
        w.weak_bg_fill = PAPER;
        w.bg_stroke = Stroke::new(1.0, LINE);
        w.fg_stroke = Stroke::new(1.0, INK);
    }
    v.widgets.hovered.bg_stroke = Stroke::new(1.2, ACCENT);
    v.widgets.active.bg_stroke = Stroke::new(1.4, ACCENT);

    style.spacing.item_spacing = vec2(12.0, 12.0);
    style.spacing.button_padding = vec2(14.0, 9.0);
    style.spacing.interact_size.y = 20.0;
    ctx.set_style(style);
}

#[derive(PartialEq, Clone, Copy)]
enum Mode {
    Single,
    Multi,
}
#[derive(PartialEq, Clone, Copy)]
enum View {
    Play,
    About,
}

struct Launcher {
    view: View,
    mode: Mode,
    world: String,
    seed: String,
    server: String,
    name: String,
    version: usize, // index into VERSIONS
    login: bool,
    assets: String,
    status: String,
    status_good: bool,
}

const VERSIONS: [&str; 2] = ["26.3", "26.1.2"];

impl Default for Launcher {
    fn default() -> Self {
        Launcher {
            view: View::Play,
            mode: Mode::Single,
            world: "New World".into(),
            seed: String::new(),
            server: String::new(),
            name: "Player".into(),
            version: 0,
            login: false,
            assets: String::new(),
            status: "Ready — uses your installed game's textures when it starts.".into(),
            status_good: true,
        }
    }
}

fn game_binary() -> PathBuf {
    let name = if cfg!(windows) { "claudium.exe" } else { "claudium" };
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let p = dir.join(name);
            if p.exists() {
                return p;
            }
        }
    }
    PathBuf::from(name)
}

impl Launcher {
    fn launch(&mut self, server: bool) {
        let mut cmd = Command::new(game_binary());
        if server {
            cmd.arg("--server");
            if !self.seed.trim().is_empty() {
                cmd.args(["--seed", self.seed.trim()]);
            }
        } else {
            cmd.args(["--version", VERSIONS[self.version]]);
            match self.mode {
                Mode::Single => {
                    cmd.args(["--world", self.world.trim()]);
                    if !self.seed.trim().is_empty() {
                        cmd.args(["--seed", self.seed.trim()]);
                    }
                }
                Mode::Multi => {
                    cmd.args(["--connect", self.server.trim()]);
                    cmd.args(["--name", self.name.trim()]);
                    if self.login {
                        cmd.arg("--login");
                    }
                }
            }
            if !self.assets.trim().is_empty() {
                cmd.args(["--assets", self.assets.trim()]);
            }
        }
        match cmd.spawn() {
            Ok(_) => {
                self.status =
                    if server { "Claudium server started on port 25565.".into() } else { "Launched Claudium — have fun!".into() };
                self.status_good = true;
            }
            Err(e) => {
                self.status = format!("Couldn't start {}: {e}", game_binary().display());
                self.status_good = false;
            }
        }
    }
}

/// Draws the isometric voxel mark in `rect`.
fn voxel_cube(painter: &egui::Painter, rect: Rect) {
    let p = |x: f32, y: f32| pos2(rect.min.x + x / 40.0 * rect.width(), rect.min.y + y / 40.0 * rect.height());
    let top = vec![p(20.0, 4.0), p(35.0, 12.5), p(20.0, 21.0), p(5.0, 12.5)];
    let left = vec![p(5.0, 13.0), p(20.0, 21.5), p(20.0, 37.0), p(5.0, 28.5)];
    let right = vec![p(35.0, 13.0), p(20.0, 21.5), p(20.0, 37.0), p(35.0, 28.5)];
    let n = Stroke::NONE;
    painter.add(egui::Shape::convex_polygon(top, Color32::from_rgb(230, 138, 94), n));
    painter.add(egui::Shape::convex_polygon(left, ACCENT, n));
    painter.add(egui::Shape::convex_polygon(right, Color32::from_rgb(169, 75, 41), n));
}

fn rail_nav(ui: &mut egui::Ui, current: &mut View, view: View, label: &str) {
    let selected = *current == view;
    let (fg, bg) = if selected { (RAIL_FG, Color32::from_rgba_unmultiplied(255, 255, 255, 18)) } else { (RAIL_DIM, Color32::TRANSPARENT) };
    let frame = egui::Frame::none().fill(bg).rounding(Rounding::same(10.0)).inner_margin(egui::Margin::symmetric(13.0, 10.0));
    let r = frame
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                let (dot, _) = ui.allocate_exact_size(vec2(7.0, 7.0), Sense::hover());
                ui.painter().rect_filled(dot, Rounding::same(2.0), if selected { ACCENT } else { RAIL_DIM });
                ui.add_space(4.0);
                ui.label(RichText::new(label).size(15.0).strong().color(fg));
            });
        })
        .response;
    if ui.interact(r.rect, r.id.with(label), Sense::click()).clicked() {
        *current = view;
    }
}

fn segmented(ui: &mut egui::Ui, mode: &mut Mode) {
    egui::Frame::none()
        .fill(PAPER)
        .stroke(Stroke::new(1.0, LINE))
        .rounding(Rounding::same(11.0))
        .inner_margin(egui::Margin::same(4.0))
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            ui.horizontal(|ui| {
                for (m, label) in [(Mode::Single, "Singleplayer"), (Mode::Multi, "Multiplayer")] {
                    let on = *mode == m;
                    let text = RichText::new(label).size(14.0).strong().color(if on { INK } else { MUTED });
                    let btn = egui::Button::new(text)
                        .fill(if on { CARD } else { Color32::TRANSPARENT })
                        .stroke(if on { Stroke::new(1.0, LINE) } else { Stroke::NONE })
                        .rounding(Rounding::same(8.0))
                        .min_size(vec2(0.0, 30.0));
                    if ui.add(btn).clicked() {
                        *mode = m;
                    }
                }
            });
        });
}

fn field(ui: &mut egui::Ui, label: &str, value: &mut String, hint: &str) {
    ui.label(RichText::new(label).size(14.0).strong().color(MUTED));
    ui.add(egui::TextEdit::singleline(value).hint_text(hint).margin(egui::Margin::symmetric(12.0, 9.0)).desired_width(f32::INFINITY));
    ui.end_row();
}

impl Launcher {
    fn play_view(&mut self, ui: &mut egui::Ui) {
        ui.add_space(4.0);
        ui.label(RichText::new("Play").heading().strong());
        ui.label(RichText::new("Start a local world, host a server, or join one.").color(MUTED));
        ui.add_space(10.0);

        egui::Frame::none()
            .fill(CARD)
            .stroke(Stroke::new(1.0, LINE))
            .rounding(Rounding::same(14.0))
            .inner_margin(egui::Margin::same(20.0))
            .show(ui, |ui| {
                segmented(ui, &mut self.mode);
                ui.add_space(16.0);
                egui::Grid::new("form").num_columns(2).spacing(vec2(16.0, 14.0)).min_col_width(108.0).show(ui, |ui| {
                    ui.label(RichText::new("Version").size(14.0).strong().color(MUTED));
                    egui::ComboBox::from_id_salt("ver").selected_text(VERSIONS[self.version]).width(ui.available_width()).show_ui(
                        ui,
                        |ui| {
                            for (i, v) in VERSIONS.iter().enumerate() {
                                ui.selectable_value(&mut self.version, i, *v);
                            }
                        },
                    );
                    ui.end_row();

                    match self.mode {
                        Mode::Single => {
                            field(ui, "World", &mut self.world, "New World");
                            field(ui, "Seed", &mut self.seed, "random");
                        }
                        Mode::Multi => {
                            field(ui, "Server", &mut self.server, "play.example.net");
                            field(ui, "Username", &mut self.name, "Player");
                            ui.label("");
                            ui.checkbox(&mut self.login, RichText::new("Sign in with Microsoft").size(14.0));
                            ui.end_row();
                        }
                    }
                    field(ui, "Textures", &mut self.assets, "from your Minecraft install");
                });
            });

        ui.add_space(14.0);
        ui.horizontal(|ui| {
            let total = ui.available_width();
            let play = egui::Button::new(RichText::new("Play  ▸").size(20.0).strong().color(Color32::WHITE))
                .fill(ACCENT)
                .rounding(Rounding::same(13.0))
                .min_size(vec2(total - 150.0, 56.0));
            if ui.add(play).clicked() {
                self.launch(false);
            }
            let host = egui::Button::new(RichText::new("Host server").size(15.0).color(INK))
                .fill(CARD)
                .stroke(Stroke::new(1.0, LINE))
                .rounding(Rounding::same(13.0))
                .min_size(vec2(134.0, 56.0));
            if ui.add(host).clicked() {
                self.launch(true);
            }
        });
        ui.add_space(8.0);
        ui.label(RichText::new(&self.status).size(13.5).strong().color(if self.status_good { SAGE } else { ACCENT_PRESS }));
    }

    fn about_view(&mut self, ui: &mut egui::Ui) {
        ui.add_space(4.0);
        ui.label(RichText::new("About Claudium").heading().strong());
        ui.add_space(10.0);
        ui.label(RichText::new("An open-source Minecraft client written from scratch in Rust.").size(16.0));
        ui.add_space(8.0);
        for line in [
            "Play local worlds, host a Claudium server, or join Java Edition servers. Block textures and the title logo load from your own installed copy of the game at runtime — nothing from Mojang ships inside Claudium.",
            "Targets the Minecraft 26.x line (26.1.2 through 26.3).",
        ] {
            ui.label(RichText::new(line).color(MUTED));
            ui.add_space(6.0);
        }
        ui.add_space(8.0);
        ui.label(RichText::new("Not affiliated with, endorsed by, or associated with Mojang or Microsoft. \"Minecraft\" is a trademark of Mojang Synergies AB. You must own the game to use its files.").size(13.0).color(MUTED));
    }
}

impl eframe::App for Launcher {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::SidePanel::left("rail")
            .exact_width(216.0)
            .resizable(false)
            .frame(egui::Frame::none().fill(RAIL).inner_margin(egui::Margin { left: 18.0, right: 18.0, top: 26.0, bottom: 18.0 }))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    let (rect, _) = ui.allocate_exact_size(vec2(32.0, 32.0), Sense::hover());
                    voxel_cube(ui.painter(), rect);
                    ui.add_space(6.0);
                    ui.label(RichText::new("Claudium").size(22.0).strong().color(RAIL_FG));
                });
                ui.add_space(2.0);
                ui.horizontal(|ui| {
                    ui.add_space(46.0);
                    ui.label(RichText::new("RUST EDITION").size(11.5).strong().color(ACCENT));
                });
                ui.add_space(20.0);
                rail_nav(ui, &mut self.view, View::Play, "Play");
                rail_nav(ui, &mut self.view, View::About, "About");

                ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
                    ui.add_space(2.0);
                    ui.label(RichText::new("Not affiliated with Mojang").size(10.5).color(RAIL_DIM));
                    ui.label(
                        RichText::new(concat!("v", env!("CARGO_PKG_VERSION"), "  ·  26.1.2–26.3"))
                            .size(12.0)
                            .color(RAIL_FG.linear_multiply(0.72)),
                    );
                });
            });

        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(PAPER).inner_margin(egui::Margin { left: 32.0, right: 28.0, top: 30.0, bottom: 28.0 }))
            .show(ctx, |ui| match self.view {
                View::Play => self.play_view(ui),
                View::About => self.about_view(ui),
            });
    }
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([940.0, 600.0])
            .with_min_inner_size([760.0, 520.0])
            .with_title("Claudium Launcher"),
        ..Default::default()
    };
    eframe::run_native(
        "Claudium Launcher",
        options,
        Box::new(|cc| {
            setup_style(&cc.egui_ctx);
            Ok(Box::new(Launcher::default()))
        }),
    )
}
