//! Claudium's built-in mods in the game (docs/claudium/mods.md): zoom, the FPS counter,
//! keystrokes and the minimap on the extras' settings, and the switch-only mods of
//! `extras::Mod` (HUD lines in the info corner, and alerts in chat). Every one is off by
//! default and does nothing while off, so the game stays vanilla 26.3 until the player turns
//! one on. A native mod's zoom or minimap wins over the built-in one.
//!
//! These are Claudium's own implementations of what well-known mods do, never their code.
use super::{Game, Screen, native_hooks::NativeMinimap, native_hooks::NativeZoom};
use claudium_client::{
    extras::{Corner, Mod},
    extras_hud::{self, Corners},
    hud::{HudBatch, rgba},
    items::ItemStack,
    keybinds,
};

/// What the built-in mods remember between ticks.
#[derive(Default)]
pub(super) struct BuiltinState {
    /// The player's position last tick, for the speedometer.
    last: Option<[f64; 3]>,
    /// Blocks per second, eased over a few ticks.
    speed: f32,
    /// The death screen was up last tick.
    dead: bool,
    /// The low-durability alert already given: main hand, off hand, head, chest, legs, feet.
    warned: [bool; 6],
}

/// Durability at or under this share of the most warns (`Mod::LowDurability`).
const LOW_SHARE: u32 = 10;

impl Game<'_> {
    /// The zoom key held while zoom is on: the factor, smooth or instant, and mouse turning.
    pub(super) fn builtin_zoom(&self) -> Option<NativeZoom> {
        let e = &self.settings.extras;
        if !e.zoom || !self.native_playing() || self.scoping() || !self.key_held(&e.zoom_key) {
            return None;
        }
        let factor = f32::from(e.zoom_level.clamp(2, 10));
        Some(NativeZoom {
            factor,
            smooth: e.zoom_smooth,
            turn: if e.zoom_sensitivity {
                1.0 / factor
            } else {
                1.0
            },
        })
    }

    /// The minimap while it is on.
    pub(super) fn builtin_minimap(&self) -> Option<NativeMinimap> {
        let e = &self.settings.extras;
        e.minimap.then(|| NativeMinimap {
            corner: e.minimap_corner,
            size: f32::from(e.minimap_size),
            blocks_per_pixel: e.minimap_zoom,
            rotate: e.minimap_rotate,
            round: e.minimap_round,
        })
    }

    /// Whether the key bound as `name` (`key.keyboard.c`, `key.mouse.left`) is held.
    fn key_held(&self, name: &str) -> bool {
        match name {
            "key.mouse.left" => self.extras.mouse[0],
            "key.mouse.right" => self.extras.mouse[1],
            _ => self
                .keys
                .iter()
                .any(|&k| keybinds::key_name(k) == Some(name)),
        }
    }

    /// Once per client tick: the speedometer's sample and the alerts.
    pub(super) fn builtin_tick(&mut self) {
        let e = &self.settings.extras;
        let (speedometer, death, durability) = (
            e.on(Mod::Speedometer),
            e.on(Mod::DeathCoordinates),
            e.on(Mod::LowDurability),
        );
        if speedometer {
            let now = self
                .player
                .as_ref()
                .map(|p| [p.position.x, p.position.y, p.position.z]);
            let state = &mut self.extras.builtin;
            if let (Some(a), Some(b)) = (state.last, now) {
                let d =
                    ((b[0] - a[0]).powi(2) + (b[1] - a[1]).powi(2) + (b[2] - a[2]).powi(2)).sqrt();
                // A teleport is not speed.
                let per_second = if d > 10.0 { 0.0 } else { d as f32 * 20.0 };
                state.speed += (per_second - state.speed) * 0.25;
            }
            state.last = now;
        }
        let dead = self.screen == Screen::Dead;
        let died = dead && !self.extras.builtin.dead;
        self.extras.builtin.dead = dead;
        if death
            && died
            && let Some(p) = self.player.as_ref()
        {
            let line = death_line(
                [p.position.x, p.position.y, p.position.z],
                &dimension_name(self.dim.id()),
            );
            self.native_chat(&line);
        }
        if durability {
            self.durability_alerts();
        }
    }

    /// Warns once when a used tool or worn armor piece drops to `LOW_SHARE`% durability.
    fn durability_alerts(&mut self) {
        let worn: [Option<ItemStack>; 6] = [
            self.stack_in(self.selected).cloned(),
            self.offhand.clone(),
            self.armor[0].clone(),
            self.armor[1].clone(),
            self.armor[2].clone(),
            self.armor[3].clone(),
        ];
        let items = self.res.items();
        let mut lines = Vec::new();
        for (i, stack) in worn.iter().enumerate() {
            let low = stack.as_ref().and_then(|s| {
                let max = items.get(&s.item).map_or(0, |info| info.max_damage);
                let left = max.saturating_sub(s.damage);
                is_low(left, max).then(|| durability_warning(&item_name(&s.item), left, max))
            });
            match low {
                Some(line) if !self.extras.builtin.warned[i] => {
                    self.extras.builtin.warned[i] = true;
                    lines.push(line);
                }
                Some(_) => {}
                None => self.extras.builtin.warned[i] = false,
            }
        }
        for line in lines {
            self.native_chat(&line);
        }
    }

    /// The built-in mods' HUD: keystrokes, the FPS counter, and the info corner's lines.
    pub(super) fn builtin_hud(&mut self, batch: &mut HudBatch, corners: &mut Corners) {
        if self.player.is_none() || self.title.is_some() {
            return;
        }
        let e = self.settings.extras.clone();
        let debug = self.debug;
        let visible = |corner: Corner| !(debug && corner == Corner::TopLeft);
        if e.fps && visible(Corner::TopLeft) {
            let (fps, min, low) = self.frame_reading();
            let line = if e.fps_lows {
                format!("{fps} FPS (min {min}, 1% {low})")
            } else {
                format!("{fps} FPS")
            };
            extras_hud::draw_lines(
                batch,
                &self.gui.atlas,
                corners,
                Corner::TopLeft,
                &[(&line, 0xFFFFFF)],
            );
        }
        if e.keystrokes && visible(e.keystrokes_corner) {
            self.draw_keystrokes(batch, corners, e.keystrokes_corner, e.keystrokes_cps);
        }
        if !visible(e.info_corner) {
            return;
        }
        let lines = self.info_lines();
        let refs: Vec<(&str, u32)> = lines.iter().map(|(t, c)| (t.as_str(), *c)).collect();
        extras_hud::draw_lines(batch, &self.gui.atlas, corners, e.info_corner, &refs);
    }

    /// The info corner's lines, from the HUD mods that are on.
    fn info_lines(&self) -> Vec<(String, u32)> {
        let e = &self.settings.extras;
        let Some(p) = self.player.as_ref() else {
            return Vec::new();
        };
        let (x, y, z) = (p.position.x, p.position.y, p.position.z);
        let block = [x.floor() as i32, y.floor() as i32, z.floor() as i32];
        let mut lines = Vec::new();
        const WHITE: u32 = 0xFFFFFF;
        if e.on(Mod::Coordinates) {
            lines.push((format!("XYZ: {x:.1} / {y:.1} / {z:.1}"), WHITE));
            lines.push((format!("Facing: {}", facing(self.yaw)), WHITE));
        }
        if e.on(Mod::Biome) {
            let biome = self
                .world
                .chunks
                .get(&(block[0] >> 4, block[2] >> 4))
                .map(|c| {
                    self.res
                        .biomes()
                        .get(c.biome(block[0], block[1], block[2]))
                        .name
                        .clone()
                });
            if let Some(name) = biome {
                lines.push((format!("Biome: {}", pretty_id(&name)), WHITE));
            }
        }
        if e.on(Mod::LightLevel) {
            let light = self.world.light(block[0], block[1], block[2]);
            let (blocks, sky) = (light & 15, light >> 4);
            // Mobs spawn in the dark: block light 0.
            let color = if blocks == 0 { 0xFF5555 } else { WHITE };
            lines.push((format!("Light: {blocks} block, {sky} sky"), color));
        }
        if e.on(Mod::Clock) {
            lines.push((clock(self.world.day_time), WHITE));
        }
        if e.on(Mod::Speedometer) {
            lines.push((
                format!("Speed: {:.1} blocks/s", self.extras.builtin.speed),
                WHITE,
            ));
        }
        if e.on(Mod::ArmorHud) {
            let items = self.res.items();
            let shown = [
                self.stack_in(self.selected),
                self.offhand.as_ref(),
                self.armor[0].as_ref(),
                self.armor[1].as_ref(),
                self.armor[2].as_ref(),
                self.armor[3].as_ref(),
            ];
            for stack in shown.into_iter().flatten() {
                let max = items.get(&stack.item).map_or(0, |i| i.max_damage);
                lines.push(armor_line(stack, max));
            }
        }
        if e.on(Mod::EffectTimers) {
            for effect in &self.potions.effects.list {
                lines.push((
                    effect_line(effect.id, effect.amplifier, effect.duration),
                    WHITE,
                ));
            }
        }
        lines
    }

    /// Keystrokes: the movement keys, jump and the mouse buttons, lit while held.
    fn draw_keystrokes(
        &mut self,
        batch: &mut HudBatch,
        corners: &mut Corners,
        corner: Corner,
        cps: bool,
    ) {
        const KEY: f32 = 20.0;
        const GAP: f32 = 2.0;
        let width = KEY * 3.0 + GAP * 2.0;
        let height = KEY * 3.0 + GAP * 3.0 + 12.0;
        let (x, y) = corners.place(corner, width, height);
        let ((left_cps, right_cps), _) = self.mouse_reading();
        let binds = &self.settings.keys;
        let held = |name: &str| self.key_held(binds.get(name));
        let label = |name: &str| {
            let full = keybinds::display_name(binds.get(name)).plain();
            if full.chars().count() <= 3 {
                full
            } else {
                full.chars().take(1).collect()
            }
        };
        let mut keys = vec![
            (
                x + KEY + GAP,
                y,
                KEY,
                KEY,
                label("key.forward"),
                held("key.forward"),
            ),
            (
                x,
                y + KEY + GAP,
                KEY,
                KEY,
                label("key.left"),
                held("key.left"),
            ),
            (
                x + KEY + GAP,
                y + KEY + GAP,
                KEY,
                KEY,
                label("key.back"),
                held("key.back"),
            ),
            (
                x + (KEY + GAP) * 2.0,
                y + KEY + GAP,
                KEY,
                KEY,
                label("key.right"),
                held("key.right"),
            ),
        ];
        let half = (width - GAP) / 2.0;
        let mouse_y = y + (KEY + GAP) * 2.0;
        let mouse = |name: &str, n: u32| {
            if cps {
                format!("{name} {n}")
            } else {
                name.to_string()
            }
        };
        keys.push((
            x,
            mouse_y,
            half,
            KEY,
            mouse("LMB", left_cps),
            held("key.attack"),
        ));
        keys.push((
            x + half + GAP,
            mouse_y,
            half,
            KEY,
            mouse("RMB", right_cps),
            held("key.use"),
        ));
        keys.push((
            x,
            mouse_y + KEY + GAP,
            width,
            12.0,
            "Jump".to_string(),
            held("key.jump"),
        ));
        let atlas = &self.gui.atlas;
        for (kx, ky, w, h, text, on) in keys {
            let (bg, fg) = if on {
                (rgba(255, 255, 255, 0xB0), rgba(0, 0, 0, 255))
            } else {
                (rgba(0, 0, 0, 0x78), rgba(255, 255, 255, 255))
            };
            batch.fill(atlas, kx, ky, w, h, bg);
            let tw = HudBatch::text_width(atlas, &text);
            batch.text(
                atlas,
                &text,
                kx + (w - tw) / 2.0,
                ky + (h - 8.0) / 2.0,
                fg,
                !on,
            );
        }
    }
}

/// The compass point the player faces, with the axis it runs along (yaw 0 faces south).
fn facing(yaw: f32) -> &'static str {
    match ((yaw / 90.0).round() as i32).rem_euclid(4) {
        0 => "south (+Z)",
        1 => "west (-X)",
        2 => "north (-Z)",
        _ => "east (+X)",
    }
}

/// `minecraft:dark_oak_forest` as `Dark Oak Forest`.
fn pretty_id(id: &str) -> String {
    let name = id.rsplit(':').next().unwrap_or(id);
    name.split('_')
        .filter(|w| !w.is_empty())
        .map(|w| {
            let mut c = w.chars();
            c.next()
                .map_or(String::new(), |f| f.to_uppercase().chain(c).collect())
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn item_name(id: &str) -> String {
    pretty_id(id)
}

fn dimension_name(id: &str) -> String {
    match id {
        "minecraft:overworld" => "the Overworld".into(),
        "minecraft:the_nether" => "the Nether".into(),
        "minecraft:the_end" => "the End".into(),
        other => pretty_id(other),
    }
}

/// The world's day and time of day: day 1 starts at tick 0, which is 06:00.
fn clock(day_time: i64) -> String {
    let day = day_time.div_euclid(24000) + 1;
    let t = (day_time + 6000).rem_euclid(24000);
    format!("Day {day}, {:02}:{:02}", t / 1000, t % 1000 * 60 / 1000)
}

fn death_line(at: [f64; 3], dimension: &str) -> String {
    format!(
        "You died at X {}, Y {}, Z {} in {dimension}.",
        at[0].floor() as i64,
        at[1].floor() as i64,
        at[2].floor() as i64
    )
}

/// Whether `left` of `max` durability is low enough to warn (and not broken or unbreakable).
fn is_low(left: u32, max: u32) -> bool {
    max > 0 && left > 0 && left * 100 <= max * LOW_SHARE
}

fn durability_warning(name: &str, left: u32, max: u32) -> String {
    format!("Your {name} is about to break: {left}/{max} durability left.")
}

/// An armor HUD line: the item, its durability left (green, yellow, red) or its count.
fn armor_line(stack: &ItemStack, max: u32) -> (String, u32) {
    let name = item_name(&stack.item);
    if max == 0 {
        let text = if stack.count > 1 {
            format!("{name} x{}", stack.count)
        } else {
            name
        };
        return (text, 0xFFFFFF);
    }
    let left = max.saturating_sub(stack.damage);
    let color = match left * 100 / max {
        51.. => 0x55FF55,
        26..=50 => 0xFFFF55,
        _ => 0xFF5555,
    };
    (format!("{name} {left}/{max}"), color)
}

/// `Speed II 1:05`, as the inventory shows an effect; `**:**` when infinite (vanilla's).
fn effect_line(id: &str, amplifier: i32, duration: i32) -> String {
    let level = match amplifier {
        0 => String::new(),
        n => format!(" {}", roman(n + 1)),
    };
    let time = if duration < 0 {
        "**:**".to_string()
    } else {
        let seconds = duration / 20;
        format!("{}:{:02}", seconds / 60, seconds % 60)
    };
    format!("{}{level} {time}", pretty_id(id))
}

fn roman(n: i32) -> String {
    const R: [(i32, &str); 9] = [
        (100, "C"),
        (90, "XC"),
        (50, "L"),
        (40, "XL"),
        (10, "X"),
        (9, "IX"),
        (5, "V"),
        (4, "IV"),
        (1, "I"),
    ];
    if !(1..=255).contains(&n) {
        return n.to_string();
    }
    let mut n = n;
    let mut out = String::new();
    for (v, s) in R {
        while n >= v {
            out.push_str(s);
            n -= v;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn facing_follows_vanillas_yaw() {
        assert_eq!(facing(0.0), "south (+Z)");
        assert_eq!(facing(90.0), "west (-X)");
        assert_eq!(facing(180.0), "north (-Z)");
        assert_eq!(facing(-90.0), "east (+X)");
        assert_eq!(facing(-180.0), "north (-Z)");
        assert_eq!(facing(359.0), "south (+Z)");
    }

    #[test]
    fn names_read_like_the_game() {
        assert_eq!(pretty_id("minecraft:dark_oak_forest"), "Dark Oak Forest");
        assert_eq!(pretty_id("diamond_pickaxe"), "Diamond Pickaxe");
        assert_eq!(dimension_name("minecraft:the_nether"), "the Nether");
    }

    #[test]
    fn the_clock_starts_at_six_on_day_one() {
        assert_eq!(clock(0), "Day 1, 06:00");
        assert_eq!(clock(6000), "Day 1, 12:00");
        assert_eq!(clock(18000), "Day 1, 00:00");
        assert_eq!(clock(24000 + 13500), "Day 2, 19:30");
    }

    #[test]
    fn durability_warns_at_ten_percent() {
        assert!(!is_low(200, 1561));
        assert!(is_low(156, 1561));
        assert!(!is_low(0, 1561), "broken: nothing to warn about");
        assert!(!is_low(0, 0), "unbreakable");
        assert_eq!(
            durability_warning("Diamond Pickaxe", 12, 1561),
            "Your Diamond Pickaxe is about to break: 12/1561 durability left."
        );
    }

    #[test]
    fn effects_show_level_and_time() {
        assert_eq!(effect_line("speed", 1, 20 * 65), "Speed II 1:05");
        assert_eq!(effect_line("night_vision", 0, -1), "Night Vision **:**");
        assert_eq!(roman(4), "IV");
        assert_eq!(roman(10), "X");
    }

    #[test]
    fn the_death_line_rounds_down() {
        assert_eq!(
            death_line([-12.7, 64.0, 3.2], "the Overworld"),
            "You died at X -13, Y 64, Z 3 in the Overworld."
        );
    }

    #[test]
    fn armor_lines_colour_by_durability_left() {
        let stack = |damage| ItemStack {
            item: "minecraft:iron_helmet".into(),
            count: 1,
            damage,
            extra: None,
        };
        assert_eq!(
            armor_line(&stack(0), 165),
            ("Iron Helmet 165/165".into(), 0x55FF55)
        );
        assert_eq!(armor_line(&stack(100), 165).1, 0xFFFF55);
        assert_eq!(armor_line(&stack(150), 165).1, 0xFF5555);
    }
}
