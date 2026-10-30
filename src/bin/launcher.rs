//! Claudium Launcher — a small desktop launcher in the spirit of the Minecraft
//! launcher (sidebar + a Play panel), themed in Claude's warm palette. It collects
//! options and starts the `claudium` game binary; it shares no code with the game.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::PathBuf;
use std::process::Command;

use eframe::egui;
use egui::{Color32, RichText, Rounding, Stroke, vec2};

// --- Claude theme -----------------------------------------------------------

const CREAM: Color32 = Color32::from_rgb(244, 242, 236);
const CARD: Color32 = Color32::from_rgb(252, 251, 248);
const INK: Color32 = Color32::from_rgb(43, 42, 39);
const MUTED: Color32 = Color32::from_rgb(124, 120, 112);
const ACCENT: Color32 = Color32::from_rgb(217, 119, 87); // Claude coral
const ACCENT_DK: Color32 = Color32::from_rgb(197, 103, 74);
const SIDEBAR: Color32 = Color32::from_rgb(48, 48, 44);
const SIDEBAR_TX: Color32 = Color32::from_rgb(234, 230, 222);

fn theme(ctx: &egui::Context) {
    let mut v = egui::Visuals::light();
    v.panel_fill = CREAM;
    v.window_fill = CREAM;
    v.extreme_bg_color = Color32::from_rgb(255, 255, 253);
    v.override_text_color = Some(INK);
    v.selection.bg_fill = ACCENT.linear_multiply(0.35);
    v.selection.stroke = Stroke::new(1.0, ACCENT);
    let r = Rounding::same(8.0);
    for w in [&mut v.widgets.inactive, &mut v.widgets.hovered, &mut v.widgets.active, &mut v.widgets.open] {
        w.rounding = r;
    }
    v.widgets.inactive.bg_fill = CARD;
    v.widgets.inactive.weak_bg_fill = CARD;
    v.widgets.hovered.bg_fill = Color32::from_rgb(246, 238, 232);
    v.widgets.hovered.weak_bg_fill = Color32::from_rgb(246, 238, 232);
    ctx.set_visuals(v);

    let mut style = (*ctx.style()).clone();
    style.spacing.item_spacing = vec2(10.0, 10.0);
    style.spacing.button_padding = vec2(12.0, 8.0);
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
    version: String,
    login: bool,
    assets: String,
    status: String,
}

impl Default for Launcher {
    fn default() -> Self {
        Launcher {
            view: View::Play,
            mode: Mode::Single,
            world: "world".into(),
            seed: String::new(),
            server: String::new(),
            name: "Player".into(),
            version: "26.3".into(),
            login: false,
            assets: String::new(),
            status: String::new(),
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
    PathBuf::from(name) // fall back to PATH
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
            cmd.args(["--version", self.version.trim()]);
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
            Ok(_) => self.status = if server { "Claudium server started.".into() } else { "Launched Claudium — have fun!".into() },
            Err(e) => self.status = format!("Could not start {}: {e}", game_binary().display()),
        }
    }

    fn sidebar(&mut self, ui: &mut egui::Ui) {
        ui.add_space(22.0);
        ui.vertical_centered(|ui| {
            ui.label(RichText::new("CLAUDIUM").size(26.0).strong().color(SIDEBAR_TX));
            ui.label(RichText::new("Rust Edition").size(13.0).color(ACCENT));
        });
        ui.add_space(26.0);

        let mut nav = |ui: &mut egui::Ui, view: View, label: &str| {
            let selected = self.view == view;
            let color = if selected { ACCENT } else { SIDEBAR_TX };
            let text = RichText::new(label).size(16.0).color(color);
            if ui.add_sized(vec2(ui.available_width(), 34.0), egui::SelectableLabel::new(selected, text)).clicked() {
                self.view = view;
            }
        };
        nav(ui, View::Play, "  Play");
        nav(ui, View::About, "  About");

        ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
            ui.add_space(14.0);
            ui.label(RichText::new("not affiliated with Mojang").size(10.0).color(MUTED));
            ui.label(RichText::new(concat!("v", env!("CARGO_PKG_VERSION"))).size(11.0).color(SIDEBAR_TX.linear_multiply(0.7)));
        });
    }

    fn play_view(&mut self, ui: &mut egui::Ui) {
        ui.add_space(18.0);
        ui.label(RichText::new("Play").size(30.0).strong());
        ui.label(RichText::new("Start a local world, host a server, or join one.").color(MUTED));
        ui.add_space(16.0);

        egui::Frame::none().fill(CARD).rounding(Rounding::same(12.0)).inner_margin(egui::Margin::same(18.0)).show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.mode, Mode::Single, RichText::new("Singleplayer").size(15.0));
                ui.selectable_value(&mut self.mode, Mode::Multi, RichText::new("Multiplayer").size(15.0));
            });
            ui.add_space(12.0);

            egui::Grid::new("form").num_columns(2).spacing(vec2(14.0, 12.0)).show(ui, |ui| {
                ui.label("Version");
                egui::ComboBox::from_id_salt("version").selected_text(&self.version).show_ui(ui, |ui| {
                    for v in ["26.3", "26.1.2"] {
                        ui.selectable_value(&mut self.version, v.to_string(), v);
                    }
                });
                ui.end_row();

                match self.mode {
                    Mode::Single => {
                        ui.label("World name");
                        ui.text_edit_singleline(&mut self.world);
                        ui.end_row();
                        ui.label("Seed (optional)");
                        ui.text_edit_singleline(&mut self.seed);
                        ui.end_row();
                    }
                    Mode::Multi => {
                        ui.label("Server address");
                        ui.text_edit_singleline(&mut self.server);
                        ui.end_row();
                        ui.label("Username");
                        ui.text_edit_singleline(&mut self.name);
                        ui.end_row();
                        ui.label("");
                        ui.checkbox(&mut self.login, "Sign in with Microsoft (online-mode servers)");
                        ui.end_row();
                    }
                }

                ui.label("Textures jar (optional)");
                ui.text_edit_singleline(&mut self.assets);
                ui.end_row();
            });
        });

        ui.add_space(18.0);
        ui.horizontal(|ui| {
            let play =
                egui::Button::new(RichText::new("PLAY").size(20.0).strong().color(Color32::WHITE)).fill(ACCENT).min_size(vec2(170.0, 50.0));
            if ui.add(play).on_hover_text("Launch the game").clicked() {
                self.launch(false);
            }
            ui.add_space(8.0);
            let host = egui::Button::new(RichText::new("Host server").size(15.0).color(INK))
                .fill(Color32::from_rgb(236, 230, 222))
                .min_size(vec2(130.0, 50.0));
            if ui.add(host).on_hover_text("Run a Claudium server with this seed").clicked() {
                self.launch(true);
            }
        });

        if !self.status.is_empty() {
            ui.add_space(12.0);
            ui.label(RichText::new(&self.status).color(ACCENT_DK));
        }
    }

    fn about_view(&mut self, ui: &mut egui::Ui) {
        ui.add_space(18.0);
        ui.label(RichText::new("About Claudium").size(30.0).strong());
        ui.add_space(10.0);
        ui.label("An open-source Minecraft client written from scratch in Rust.");
        ui.add_space(6.0);
        ui.label(RichText::new("• Play local worlds, host a Claudium server, or join Java Edition servers.").color(MUTED));
        ui.label(RichText::new("• Loads textures from your own installed copy of the game at runtime.").color(MUTED));
        ui.label(RichText::new("• Targets the Minecraft 26.x line (26.1.2 – 26.3).").color(MUTED));
        ui.add_space(14.0);
        ui.label(RichText::new("Not affiliated with, endorsed by, or associated with Mojang or Microsoft.\n\"Minecraft\" is a trademark of Mojang Synergies AB. You must own the game to use its files.").color(MUTED));
    }
}

impl eframe::App for Launcher {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::SidePanel::left("nav")
            .exact_width(200.0)
            .resizable(false)
            .frame(egui::Frame::none().fill(SIDEBAR).inner_margin(egui::Margin::same(14.0)))
            .show(ctx, |ui| self.sidebar(ui));

        egui::CentralPanel::default().frame(egui::Frame::none().fill(CREAM).inner_margin(egui::Margin::same(28.0))).show(ctx, |ui| {
            match self.view {
                View::Play => self.play_view(ui),
                View::About => self.about_view(ui),
            }
        });
    }
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([900.0, 580.0])
            .with_min_inner_size([720.0, 480.0])
            .with_title("Claudium Launcher"),
        ..Default::default()
    };
    eframe::run_native(
        "Claudium Launcher",
        options,
        Box::new(|cc| {
            theme(&cc.egui_ctx);
            Ok(Box::new(Launcher::default()))
        }),
    )
}
