//! Claudium Launcher — a desktop launcher in the Minecraft-launcher layout (icon
//! rail, hero banner, and a bottom PLAY bar), rebuilt dark with an orange accent and
//! Claudium branding (no Mojang art). It starts the `claudium` game binary.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::PathBuf;
use std::process::Command;

use eframe::egui;
use egui::{Align2, Color32, FontId, Pos2, Rect, RichText, Rounding, Sense, Stroke, TextStyle, pos2, vec2};

// --- dark theme, orange accent ----------------------------------------------

const BG: Color32 = Color32::from_rgb(27, 28, 31);
const PANEL: Color32 = Color32::from_rgb(38, 40, 44);
const PANEL2: Color32 = Color32::from_rgb(46, 48, 53);
const RAIL: Color32 = Color32::from_rgb(21, 22, 25);
const FG: Color32 = Color32::from_rgb(236, 236, 238);
const MUTED: Color32 = Color32::from_rgb(154, 156, 163);
const LINE: Color32 = Color32::from_rgb(54, 56, 61);
const FIELD: Color32 = Color32::from_rgb(28, 29, 32);
const ACCENT: Color32 = Color32::from_rgb(232, 129, 46);
const ACCENT_HI: Color32 = Color32::from_rgb(244, 154, 76);
const ACCENT_LO: Color32 = Color32::from_rgb(181, 97, 28);
const ON_ACCENT: Color32 = Color32::from_rgb(26, 18, 5);

fn setup_style(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();
    use egui::FontFamily::Proportional;
    style.text_styles = [
        (TextStyle::Heading, FontId::new(26.0, Proportional)),
        (TextStyle::Body, FontId::new(15.0, Proportional)),
        (TextStyle::Button, FontId::new(15.0, Proportional)),
        (TextStyle::Small, FontId::new(12.0, Proportional)),
        (TextStyle::Monospace, FontId::new(13.0, egui::FontFamily::Monospace)),
    ]
    .into();

    let v = &mut style.visuals;
    v.dark_mode = true;
    v.panel_fill = BG;
    v.window_fill = PANEL;
    v.extreme_bg_color = FIELD;
    v.override_text_color = Some(FG);
    v.selection.bg_fill = ACCENT.linear_multiply(0.35);
    v.selection.stroke = Stroke::new(1.0, ACCENT);
    let r8 = Rounding::same(8.0);
    for w in [&mut v.widgets.inactive, &mut v.widgets.hovered, &mut v.widgets.active, &mut v.widgets.open] {
        w.rounding = r8;
        w.bg_fill = FIELD;
        w.weak_bg_fill = FIELD;
        w.bg_stroke = Stroke::new(1.0, LINE);
        w.fg_stroke = Stroke::new(1.0, FG);
    }
    v.widgets.hovered.bg_stroke = Stroke::new(1.2, ACCENT);
    v.widgets.active.bg_stroke = Stroke::new(1.4, ACCENT);

    style.spacing.item_spacing = vec2(12.0, 12.0);
    style.spacing.button_padding = vec2(14.0, 9.0);
    style.spacing.interact_size.y = 20.0;
    ctx.set_style(style);
}

#[derive(PartialEq, Clone, Copy)]
enum Tab {
    Play,
    Worlds,
    Servers,
    News,
    Settings,
}
#[derive(PartialEq, Clone, Copy)]
enum Mode {
    Single,
    Multi,
}

const VERSIONS: [&str; 2] = ["26.3", "26.1.2"];

struct Launcher {
    tab: Tab,
    mode: Mode,
    world: String,
    seed: String,
    server: String,
    name: String,
    version: usize,
    login: bool,
    assets: String,
    status: String,
}

impl Default for Launcher {
    fn default() -> Self {
        Launcher {
            tab: Tab::Play,
            mode: Mode::Single,
            world: "New World".into(),
            seed: String::new(),
            server: String::new(),
            name: "Player".into(),
            version: 0,
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
        self.status = match cmd.spawn() {
            Ok(_) if server => "Claudium server started on port 25565.".into(),
            Ok(_) => "Launching Claudium…".into(),
            Err(e) => format!("Couldn't start {}: {e}", game_binary().display()),
        };
    }
}

fn voxel_cube(painter: &egui::Painter, rect: Rect) {
    let p = |x: f32, y: f32| pos2(rect.min.x + x / 40.0 * rect.width(), rect.min.y + y / 40.0 * rect.height());
    let n = Stroke::NONE;
    painter.add(egui::Shape::convex_polygon(vec![p(20.0, 4.0), p(35.0, 12.5), p(20.0, 21.0), p(5.0, 12.5)], ACCENT_HI, n));
    painter.add(egui::Shape::convex_polygon(vec![p(5.0, 13.0), p(20.0, 21.5), p(20.0, 37.0), p(5.0, 28.5)], ACCENT, n));
    painter.add(egui::Shape::convex_polygon(vec![p(35.0, 13.0), p(20.0, 21.5), p(20.0, 37.0), p(35.0, 28.5)], ACCENT_LO, n));
}

fn tab_icon(painter: &egui::Painter, c: Pos2, tab: Tab, color: Color32) {
    let s = Stroke::new(1.8, color);
    let r = Rounding::same(2.0);
    match tab {
        Tab::Play => {
            painter.add(egui::Shape::convex_polygon(
                vec![pos2(c.x - 5.0, c.y - 6.0), pos2(c.x + 6.0, c.y), pos2(c.x - 5.0, c.y + 6.0)],
                color,
                Stroke::NONE,
            ));
        }
        Tab::Worlds => {
            painter.rect(Rect::from_center_size(c + vec2(0.0, 1.0), vec2(16.0, 12.0)), r, Color32::TRANSPARENT, s);
            painter.line_segment([pos2(c.x - 8.0, c.y - 5.0), pos2(c.x - 2.0, c.y - 5.0)], s);
        }
        Tab::Servers => {
            painter.rect(Rect::from_center_size(c + vec2(0.0, -4.0), vec2(16.0, 7.0)), r, Color32::TRANSPARENT, s);
            painter.rect(Rect::from_center_size(c + vec2(0.0, 5.0), vec2(16.0, 7.0)), r, Color32::TRANSPARENT, s);
        }
        Tab::News => {
            painter.rect(Rect::from_center_size(c, vec2(14.0, 16.0)), r, Color32::TRANSPARENT, s);
            for dy in [-4.0, 0.0, 4.0] {
                painter.line_segment([pos2(c.x - 4.0, c.y + dy), pos2(c.x + 4.0, c.y + dy)], s);
            }
        }
        Tab::Settings => {
            painter.circle(c, 6.0, Color32::TRANSPARENT, s);
            painter.circle_filled(c, 2.0, color);
        }
    }
}

fn rail(ui: &mut egui::Ui, app: &mut Launcher) {
    ui.vertical_centered(|ui| {
        ui.add_space(16.0);
        let (rect, _) = ui.allocate_exact_size(vec2(40.0, 40.0), Sense::hover());
        voxel_cube(ui.painter(), rect);
        ui.add_space(12.0);

        for (tab, label) in
            [(Tab::Play, "Play"), (Tab::Worlds, "Worlds"), (Tab::Servers, "Servers"), (Tab::News, "News"), (Tab::Settings, "Settings")]
        {
            let active = app.tab == tab;
            let (rect, resp) = ui.allocate_exact_size(vec2(56.0, 54.0), Sense::click());
            let hover = resp.hovered();
            if active || hover {
                ui.painter().rect_filled(
                    rect.shrink2(vec2(2.0, 1.0)),
                    Rounding::same(12.0),
                    Color32::from_rgba_unmultiplied(255, 255, 255, if active { 20 } else { 12 }),
                );
            }
            if active {
                ui.painter().rect_filled(
                    Rect::from_min_size(pos2(rect.left(), rect.center().y - 15.0), vec2(3.0, 30.0)),
                    Rounding::same(2.0),
                    ACCENT,
                );
            }
            let color = if active || hover { FG } else { MUTED };
            tab_icon(ui.painter(), rect.center() - vec2(0.0, 7.0), tab, color);
            ui.painter().text(rect.center() + vec2(0.0, 15.0), Align2::CENTER_CENTER, label, FontId::proportional(9.5), color);
            if resp.clicked() {
                app.tab = tab;
            }
        }
    });

    ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
        ui.add_space(14.0);
        let (rect, _) = ui.allocate_exact_size(vec2(40.0, 40.0), Sense::hover());
        ui.painter().rect_filled(rect, Rounding::same(9.0), ACCENT_LO);
        ui.painter().text(rect.center(), Align2::CENTER_CENTER, "P", FontId::proportional(18.0), FG);
    });
}

fn hero(ui: &mut egui::Ui) {
    let rect = ui.max_rect();
    let p = ui.painter();
    // vertical gradient panel
    let mut mesh = egui::Mesh::default();
    for (pos, col) in [(rect.left_top(), PANEL2), (rect.right_top(), PANEL2), (rect.right_bottom(), BG), (rect.left_bottom(), BG)] {
        mesh.colored_vertex(pos, col);
    }
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(0, 2, 3);
    p.add(egui::Shape::mesh(mesh));
    // warm glow, top-right
    p.circle_filled(pos2(rect.right() - 40.0, rect.top() - 20.0), 150.0, Color32::from_rgba_unmultiplied(232, 129, 46, 28));
    // cube + wordmark
    voxel_cube(p, Rect::from_min_size(pos2(rect.right() - 128.0, rect.top() + 26.0), vec2(92.0, 92.0)));
    p.text(pos2(rect.left() + 28.0, rect.bottom() - 54.0), Align2::LEFT_TOP, "CLAUDIUM", FontId::proportional(46.0), FG);
    p.text(pos2(rect.left() + 30.0, rect.bottom() - 20.0), Align2::LEFT_TOP, "R U S T   E D I T I O N", FontId::proportional(13.0), ACCENT);
}

fn seg(ui: &mut egui::Ui, mode: &mut Mode) {
    egui::Frame::none()
        .fill(PANEL)
        .stroke(Stroke::new(1.0, LINE))
        .rounding(Rounding::same(10.0))
        .inner_margin(egui::Margin::same(4.0))
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            ui.horizontal(|ui| {
                for (m, label) in [(Mode::Single, "Singleplayer"), (Mode::Multi, "Multiplayer")] {
                    let on = *mode == m;
                    let text = RichText::new(label).size(14.0).color(if on { ON_ACCENT } else { MUTED });
                    let btn = egui::Button::new(text)
                        .fill(if on { ACCENT } else { Color32::TRANSPARENT })
                        .stroke(Stroke::NONE)
                        .rounding(Rounding::same(7.0))
                        .min_size(vec2(0.0, 30.0));
                    if ui.add(btn).clicked() {
                        *mode = m;
                    }
                }
            });
        });
}

fn field(ui: &mut egui::Ui, label: &str, value: &mut String, hint: &str) {
    ui.label(RichText::new(label).size(13.5).color(MUTED));
    ui.add(egui::TextEdit::singleline(value).hint_text(hint).margin(egui::Margin::symmetric(12.0, 8.0)).desired_width(f32::INFINITY));
    ui.end_row();
}

impl Launcher {
    fn body(&mut self, ui: &mut egui::Ui) {
        if self.tab != Tab::Play {
            ui.add_space(60.0);
            ui.vertical_centered(|ui| {
                let name = match self.tab {
                    Tab::Worlds => "Your worlds",
                    Tab::Servers => "Servers",
                    Tab::News => "News",
                    _ => "Settings",
                };
                ui.label(RichText::new(name).size(20.0).color(FG));
                ui.add_space(6.0);
                ui.label(RichText::new("Coming soon.").color(MUTED));
            });
            return;
        }

        seg(ui, &mut self.mode);
        ui.add_space(16.0);
        egui::Grid::new("form").num_columns(2).spacing(vec2(14.0, 12.0)).min_col_width(100.0).max_col_width(440.0).show(ui, |ui| {
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
        ui.add_space(10.0);
        ui.label(RichText::new("Textures and the title logo load from your own Minecraft install at startup.").size(13.0).color(MUTED));
    }

    fn playbar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            let play = egui::Button::new(RichText::new("PLAY").size(20.0).color(ON_ACCENT))
                .fill(ACCENT)
                .rounding(Rounding::same(9.0))
                .min_size(vec2(230.0, 52.0));
            if ui.add(play).clicked() {
                self.launch(false);
            }
            ui.add_space(14.0);
            ui.vertical(|ui| {
                ui.add_space(4.0);
                ui.label(RichText::new("CLAUDIUM VERSION").size(11.0).color(MUTED));
                egui::ComboBox::from_id_salt("ver")
                    .selected_text(RichText::new(format!("Latest release · {}", VERSIONS[self.version])).size(15.0))
                    .show_ui(ui, |ui| {
                        for (i, v) in VERSIONS.iter().enumerate() {
                            ui.selectable_value(&mut self.version, i, *v);
                        }
                    });
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let host = egui::Button::new(RichText::new("Host server").size(14.0).color(FG))
                    .fill(PANEL)
                    .stroke(Stroke::new(1.0, LINE))
                    .rounding(Rounding::same(8.0))
                    .min_size(vec2(0.0, 40.0));
                if ui.add(host).clicked() {
                    self.launch(true);
                }
                if !self.status.is_empty() {
                    ui.add_space(12.0);
                    ui.label(RichText::new(&self.status).size(13.0).color(ACCENT_HI));
                }
            });
        });
    }
}

impl eframe::App for Launcher {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::SidePanel::left("rail")
            .exact_width(76.0)
            .resizable(false)
            .show_separator_line(false)
            .frame(egui::Frame::none().fill(RAIL))
            .show(ctx, |ui| rail(ui, self));

        egui::TopBottomPanel::top("hero")
            .exact_height(184.0)
            .resizable(false)
            .show_separator_line(false)
            .frame(egui::Frame::none().fill(BG))
            .show(ctx, |ui| hero(ui));

        egui::TopBottomPanel::bottom("playbar")
            .exact_height(86.0)
            .resizable(false)
            .show_separator_line(false)
            .frame(egui::Frame::none().fill(PANEL).inner_margin(egui::Margin::symmetric(26.0, 16.0)))
            .show(ctx, |ui| self.playbar(ui));

        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(BG).inner_margin(egui::Margin { left: 26.0, right: 26.0, top: 22.0, bottom: 22.0 }))
            .show(ctx, |ui| self.body(ui));
    }
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([980.0, 620.0])
            .with_min_inner_size([820.0, 560.0])
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
