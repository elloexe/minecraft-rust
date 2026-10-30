//! Claudium's title screen (the "home screen"). Original artwork: a blocky
//! wordmark drawn with a built-in 5x7 pixel font, a stack of buttons, and a
//! "Rust Edition" line. It draws through the renderer's HUD pipeline, so it
//! shares nothing with and changes nothing about the in-game rendering.

use crate::render::HudVertex;

pub const GW: usize = 5;
pub const GH: usize = 7;

/// 5x7 glyph, one byte per row, bit 4 (0x10) is the leftmost column.
fn glyph(c: char) -> [u8; 7] {
    match c.to_ascii_uppercase() {
        'A' => [0b01110, 0b10001, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001],
        'B' => [0b11110, 0b10001, 0b10001, 0b11110, 0b10001, 0b10001, 0b11110],
        'C' => [0b01110, 0b10001, 0b10000, 0b10000, 0b10000, 0b10001, 0b01110],
        'D' => [0b11110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b11110],
        'E' => [0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b11111],
        'F' => [0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b10000],
        'G' => [0b01110, 0b10001, 0b10000, 0b10111, 0b10001, 0b10001, 0b01111],
        'H' => [0b10001, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001],
        'I' => [0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b11111],
        'J' => [0b00111, 0b00010, 0b00010, 0b00010, 0b10010, 0b10010, 0b01100],
        'K' => [0b10001, 0b10010, 0b10100, 0b11000, 0b10100, 0b10010, 0b10001],
        'L' => [0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b11111],
        'M' => [0b10001, 0b11011, 0b10101, 0b10101, 0b10001, 0b10001, 0b10001],
        'N' => [0b10001, 0b10001, 0b11001, 0b10101, 0b10011, 0b10001, 0b10001],
        'O' => [0b01110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110],
        'P' => [0b11110, 0b10001, 0b10001, 0b11110, 0b10000, 0b10000, 0b10000],
        'Q' => [0b01110, 0b10001, 0b10001, 0b10001, 0b10101, 0b10010, 0b01101],
        'R' => [0b11110, 0b10001, 0b10001, 0b11110, 0b10100, 0b10010, 0b10001],
        'S' => [0b01111, 0b10000, 0b10000, 0b01110, 0b00001, 0b00001, 0b11110],
        'T' => [0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100],
        'U' => [0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110],
        'V' => [0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01010, 0b00100],
        'W' => [0b10001, 0b10001, 0b10001, 0b10101, 0b10101, 0b11011, 0b10001],
        'X' => [0b10001, 0b10001, 0b01010, 0b00100, 0b01010, 0b10001, 0b10001],
        'Y' => [0b10001, 0b10001, 0b01010, 0b00100, 0b00100, 0b00100, 0b00100],
        'Z' => [0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b10000, 0b11111],
        '0' => [0b01110, 0b10001, 0b10011, 0b10101, 0b11001, 0b10001, 0b01110],
        '1' => [0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110],
        '2' => [0b01110, 0b10001, 0b00001, 0b00010, 0b00100, 0b01000, 0b11111],
        '3' => [0b11111, 0b00010, 0b00100, 0b00010, 0b00001, 0b10001, 0b01110],
        '4' => [0b00010, 0b00110, 0b01010, 0b10010, 0b11111, 0b00010, 0b00010],
        '5' => [0b11111, 0b10000, 0b11110, 0b00001, 0b00001, 0b10001, 0b01110],
        '6' => [0b00110, 0b01000, 0b10000, 0b11110, 0b10001, 0b10001, 0b01110],
        '7' => [0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b01000, 0b01000],
        '8' => [0b01110, 0b10001, 0b10001, 0b01110, 0b10001, 0b10001, 0b01110],
        '9' => [0b01110, 0b10001, 0b10001, 0b01111, 0b00001, 0b00010, 0b01100],
        '-' => [0, 0, 0, 0b11111, 0, 0, 0],
        '.' => [0, 0, 0, 0, 0, 0b00110, 0b00110],
        ':' => [0, 0b00110, 0b00110, 0, 0b00110, 0b00110, 0],
        '/' => [0b00001, 0b00010, 0b00010, 0b00100, 0b01000, 0b01000, 0b10000],
        '!' => [0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0, 0b00100],
        '\'' => [0b00100, 0b00100, 0b00100, 0, 0, 0, 0],
        _ => [0; 7], // space and unknowns
    }
}

/// Width in pixels of a string drawn at the given per-pixel scale (1px gap between glyphs).
pub fn text_width(s: &str, scale: f32) -> f32 {
    if s.is_empty() {
        return 0.0;
    }
    (s.chars().count() as f32 * (GW as f32 + 1.0) - 1.0) * scale
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Action {
    Play,
    JoinServer,
    Quit,
}

pub struct Button {
    pub label: String,
    pub action: Action,
    rect: [f32; 4], // x, y, w, h in pixels
}

/// Builds the title screen geometry and button hitboxes for the current window size.
pub struct Canvas {
    pub verts: Vec<HudVertex>,
    pub buttons: Vec<Button>,
    w: f32,
    h: f32,
}

impl Canvas {
    pub fn new(w: f32, h: f32) -> Self {
        Canvas { verts: Vec::with_capacity(4096), buttons: Vec::new(), w, h }
    }

    fn quad(&mut self, x: f32, y: f32, rw: f32, rh: f32, color: [f32; 4]) {
        let x0 = x / self.w * 2.0 - 1.0;
        let x1 = (x + rw) / self.w * 2.0 - 1.0;
        let y0 = 1.0 - y / self.h * 2.0;
        let y1 = 1.0 - (y + rh) / self.h * 2.0;
        let p = |px: f32, py: f32| HudVertex { pos: [px, py], uv: [-1.0, -1.0], color };
        self.verts.extend_from_slice(&[p(x0, y0), p(x1, y0), p(x1, y1), p(x0, y0), p(x1, y1), p(x0, y1)]);
    }

    fn quad_shaded(&mut self, x: f32, y: f32, rw: f32, rh: f32, top: [f32; 4], bot: [f32; 4]) {
        let x0 = x / self.w * 2.0 - 1.0;
        let x1 = (x + rw) / self.w * 2.0 - 1.0;
        let y0 = 1.0 - y / self.h * 2.0;
        let y1 = 1.0 - (y + rh) / self.h * 2.0;
        let p = |px: f32, py: f32, c: [f32; 4]| HudVertex { pos: [px, py], uv: [-1.0, -1.0], color: c };
        self.verts.extend_from_slice(&[p(x0, y0, top), p(x1, y0, top), p(x1, y1, bot), p(x0, y0, top), p(x1, y1, bot), p(x0, y1, bot)]);
    }

    /// Draws a string; returns its pixel width.
    pub fn text(&mut self, x: f32, y: f32, scale: f32, color: [f32; 4], s: &str) -> f32 {
        let mut cx = x;
        for c in s.chars() {
            let g = glyph(c);
            for (row, bits) in g.iter().enumerate() {
                for col in 0..GW {
                    if bits & (0x10 >> col) != 0 {
                        self.quad(cx + col as f32 * scale, y + row as f32 * scale, scale, scale, color);
                    }
                }
            }
            cx += (GW as f32 + 1.0) * scale;
        }
        cx - x - scale
    }

    fn text_centered(&mut self, cx: f32, y: f32, scale: f32, color: [f32; 4], s: &str) {
        self.text(cx - text_width(s, scale) / 2.0, y, scale, color, s);
    }

    /// Draws a string with a dark drop shadow (one pixel-cell down-right), like a classic game menu.
    fn text_shadow(&mut self, x: f32, y: f32, scale: f32, color: [f32; 4], s: &str) {
        self.text(x + scale, y + scale, scale, [0.0, 0.0, 0.0, color[3] * 0.6], s);
        self.text(x, y, scale, color, s);
    }

    fn button(&mut self, cx: f32, y: f32, bw: f32, bh: f32, label: &str, action: Action, hovered: bool) {
        let x = cx - bw / 2.0;
        // Border, then face, then label.
        self.quad(x - 2.0, y - 2.0, bw + 4.0, bh + 4.0, [0.0, 0.0, 0.0, 0.85]);
        let (top, bot) =
            if hovered { ([0.42, 0.52, 0.42, 1.0], [0.26, 0.34, 0.26, 1.0]) } else { ([0.30, 0.30, 0.33, 1.0], [0.18, 0.18, 0.20, 1.0]) };
        self.quad_shaded(x, y, bw, bh, top, bot);
        self.quad(x, y, bw, 1.5, [1.0, 1.0, 1.0, 0.18]);
        let scale = (bh / 11.0).floor().max(2.0);
        let ty = y + (bh - GH as f32 * scale) / 2.0;
        self.text_centered(cx, ty, scale, [0.94, 0.94, 0.94, 1.0], label);
        self.buttons.push(Button { label: label.to_string(), action, rect: [x, y, bw, bh] });
    }

    pub fn hit(&self, mx: f32, my: f32) -> Option<Action> {
        self.buttons
            .iter()
            .find(|b| mx >= b.rect[0] && mx <= b.rect[0] + b.rect[2] && my >= b.rect[1] && my <= b.rect[1] + b.rect[3])
            .map(|b| b.action)
    }

    pub fn hovered_at(&self, mx: f32, my: f32) -> Option<usize> {
        self.buttons.iter().position(|b| mx >= b.rect[0] && mx <= b.rect[0] + b.rect[2] && my >= b.rect[1] && my <= b.rect[1] + b.rect[3])
    }
}

pub struct TitleScreen {
    /// Buttons to offer, top to bottom.
    pub entries: Vec<(String, Action)>,
    pub status: String,
    pub splash: String,
    pub version: String,
}

impl TitleScreen {
    /// Builds the full title screen. `mouse` is the cursor in physical pixels; `time`
    /// drives the splash pulse.
    pub fn build(&self, w: f32, h: f32, mouse: (f32, f32), time: f32) -> Canvas {
        let mut c = Canvas::new(w, h);
        let s = (h / 240.0).max(1.0);

        // Background: deep sky gradient with a darker ground band, drawn as big quads.
        c.quad_shaded(0.0, 0.0, w, h * 0.72, [0.18, 0.26, 0.42, 1.0], [0.37, 0.52, 0.70, 1.0]);
        c.quad_shaded(0.0, h * 0.72, w, h * 0.28, [0.20, 0.15, 0.11, 1.0], [0.10, 0.08, 0.06, 1.0]);
        // A row of blocky "hills" along the horizon (original decoration, not any game's art).
        let tile = (14.0 * s).max(10.0);
        let cols = (w / tile).ceil() as i32 + 1;
        for i in 0..cols {
            let hh = ((i as f32 * 1.7).sin() * 0.5 + 0.5) * tile * 2.0 + tile;
            let x = i as f32 * tile;
            let g = 0.16 + ((i as f32 * 0.9).cos() * 0.5 + 0.5) * 0.06;
            c.quad(x, h * 0.72 - hh, tile + 1.0, hh, [g * 0.6, g, g * 0.5, 1.0]);
        }

        // Wordmark
        let logo_scale = (w / 90.0).min(h / 46.0).floor().max(3.0);
        let title = "CLAUDIUM";
        let tw = text_width(title, logo_scale);
        let ty = h * 0.14;
        c.text_shadow(w / 2.0 - tw / 2.0, ty, logo_scale, [0.86, 0.90, 0.96, 1.0], title);

        // "Rust Edition" line under the wordmark.
        let sub_scale = (logo_scale * 0.42).floor().max(2.0);
        c.text_centered(w / 2.0, ty + GH as f32 * logo_scale + 6.0 * s, sub_scale, [0.78, 0.82, 0.90, 1.0], "RUST EDITION");

        // Yellow splash to the side, gently pulsing, like a classic title splash.
        let pulse = 1.0 + (time * 3.0).sin() * 0.06;
        let splash_scale = (sub_scale as f32 * pulse).max(2.0);
        c.text_shadow(
            w / 2.0 + tw / 2.0 - text_width(&self.splash, splash_scale) * 0.3,
            ty + GH as f32 * logo_scale + 2.0 * s,
            splash_scale,
            [1.0, 0.92, 0.25, 1.0],
            &self.splash,
        );

        // Buttons, centered and stacked.
        let bw = (w * 0.42).clamp(180.0, 420.0);
        let bh = (26.0 * s).clamp(26.0, 52.0);
        let gap = 10.0 * s;
        let total = self.entries.len() as f32 * (bh + gap) - gap;
        let mut by = (h * 0.52).max(ty + GH as f32 * logo_scale + 40.0 * s);
        let _ = total;
        for (label, action) in &self.entries {
            let hovered = mouse.0 >= w / 2.0 - bw / 2.0 && mouse.0 <= w / 2.0 + bw / 2.0 && mouse.1 >= by && mouse.1 <= by + bh;
            c.button(w / 2.0, by, bw, bh, label, *action, hovered);
            by += bh + gap;
        }

        // Status line (e.g. "Connecting..."), version, and an honest disclaimer.
        if !self.status.is_empty() {
            c.text_centered(w / 2.0, by + 4.0 * s, (2.0 * s).max(2.0), [0.85, 0.9, 1.0, 1.0], &self.status);
        }
        let foot = (2.0 * s).max(2.0);
        c.text_shadow(6.0 * s, h - GH as f32 * foot - 6.0 * s, foot, [0.8, 0.8, 0.85, 1.0], &self.version);
        let disc = "UNOFFICIAL - NOT MADE BY OR AFFILIATED WITH MOJANG";
        c.text_shadow(w - text_width(disc, foot) - 6.0 * s, h - GH as f32 * foot - 6.0 * s, foot, [0.7, 0.7, 0.72, 1.0], disc);
        c
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_menu_char_has_a_glyph() {
        let used = "CLAUDIUM RUST EDITION SINGLEPLAYER JOIN SERVER QUIT MADE IN ! 0123456789 UNOFFICIAL-NOTABYMJGHFDW.:/'";
        for c in used.chars() {
            let g = glyph(c);
            if c != ' ' {
                assert!(g.iter().any(|&r| r != 0), "no glyph for {c:?}");
            }
        }
    }

    #[test]
    fn buttons_are_hittable() {
        let ts = TitleScreen {
            entries: vec![("SINGLEPLAYER".into(), Action::Play), ("QUIT".into(), Action::Quit)],
            status: String::new(),
            splash: "MADE IN RUST!".into(),
            version: "CLAUDIUM 0.1.0".into(),
        };
        let c = ts.build(1280.0, 720.0, (0.0, 0.0), 0.0);
        assert_eq!(c.buttons.len(), 2);
        // The center of the first button should hit Play.
        let b = &c.buttons[0];
        let (mx, my) = (b.rect[0] + b.rect[2] / 2.0, b.rect[1] + b.rect[3] / 2.0);
        assert_eq!(c.hit(mx, my), Some(Action::Play));
        assert_eq!(c.hit(5.0, 5.0), None);
        assert!(!c.verts.is_empty());
    }
}
