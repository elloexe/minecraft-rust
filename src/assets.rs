//! Texture loading. Claudium ships no game art: it reads textures from the
//! player's own installed game jar (or a resource pack zip) at runtime, and falls
//! back to its own procedurally generated textures when none is found.

use std::collections::HashMap;
use std::io::Read;
use std::path::{Path, PathBuf};

pub const TILE: usize = 16;

pub struct TextureSource {
    images: HashMap<String, Vec<u8>>, // name -> raw png bytes (lazy decode)
    pub origin: Option<PathBuf>,
}

impl TextureSource {
    pub fn empty() -> Self {
        TextureSource { images: HashMap::new(), origin: None }
    }

    /// Reads every block texture from a game jar or resource-pack zip.
    pub fn from_zip(path: &Path) -> anyhow::Result<Self> {
        let f = std::fs::File::open(path)?;
        let mut zip = zip::ZipArchive::new(std::io::BufReader::new(f))?;
        let mut images = HashMap::new();
        let prefix = "assets/minecraft/textures/block/";
        for i in 0..zip.len() {
            let mut entry = zip.by_index(i)?;
            let name = entry.name().to_string();
            if let Some(rest) = name.strip_prefix(prefix) {
                if let Some(stem) = rest.strip_suffix(".png") {
                    if stem.contains('/') {
                        continue;
                    }
                    let mut buf = Vec::with_capacity(entry.size() as usize);
                    entry.read_to_end(&mut buf)?;
                    images.insert(stem.to_string(), buf);
                }
            }
        }
        if images.is_empty() {
            anyhow::bail!("no block textures found in {}", path.display());
        }
        Ok(TextureSource { images, origin: Some(path.to_path_buf()) })
    }

    pub fn has(&self, name: &str) -> bool {
        self.images.contains_key(name)
    }

    pub fn count(&self) -> usize {
        self.images.len()
    }

    /// Decodes a texture and resamples its first frame to 16x16 RGBA.
    fn decode(&self, name: &str) -> Option<Vec<u8>> {
        let bytes = self.images.get(name)?;
        let mut dec = png::Decoder::new(std::io::Cursor::new(bytes));
        dec.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
        let mut reader = dec.read_info().ok()?;
        let mut buf = vec![0; reader.output_buffer_size()];
        let info = reader.next_frame(&mut buf).ok()?;
        let (w, h) = (info.width as usize, info.height as usize);
        let channels = match info.color_type {
            png::ColorType::Rgba => 4,
            png::ColorType::Rgb => 3,
            png::ColorType::GrayscaleAlpha => 2,
            png::ColorType::Grayscale => 1,
            _ => return None,
        };
        // Animated textures are vertical strips; use the first square frame.
        let size = w.min(h);
        let mut out = vec![0u8; TILE * TILE * 4];
        for y in 0..TILE {
            for x in 0..TILE {
                let sx = x * size / TILE;
                let sy = y * size / TILE;
                let i = (sy * w + sx) * channels;
                let px = &buf[i..i + channels];
                let rgba = match channels {
                    4 => [px[0], px[1], px[2], px[3]],
                    3 => [px[0], px[1], px[2], 255],
                    2 => [px[0], px[0], px[0], px[1]],
                    _ => [px[0], px[0], px[0], 255],
                };
                out[(y * TILE + x) * 4..][..4].copy_from_slice(&rgba);
            }
        }
        Some(out)
    }
}

/// Finds the player's installed game jar for a version (e.g. "26.1.2").
pub fn find_game_jar(version: &str) -> Option<PathBuf> {
    let mut roots = vec![];
    if let Ok(appdata) = std::env::var("APPDATA") {
        roots.push(PathBuf::from(appdata).join(".minecraft"));
    }
    if let Ok(home) = std::env::var("HOME") {
        roots.push(PathBuf::from(&home).join(".minecraft"));
        roots.push(PathBuf::from(&home).join("Library/Application Support/minecraft"));
        roots.push(PathBuf::from(&home).join(".var/app/com.mojang.Minecraft/.minecraft"));
    }
    for r in &roots {
        let p = r.join("versions").join(version).join(format!("{version}.jar"));
        if p.exists() {
            return Some(p);
        }
    }
    // Otherwise take the newest installed release-looking version.
    for r in &roots {
        let Ok(dir) = std::fs::read_dir(r.join("versions")) else { continue };
        let mut found: Vec<PathBuf> = dir
            .flatten()
            .map(|e| {
                let n = e.file_name().to_string_lossy().to_string();
                e.path().join(format!("{n}.jar"))
            })
            .filter(|p| p.exists())
            .collect();
        found.sort();
        if let Some(p) = found.pop() {
            return Some(p);
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Procedural fallback textures (Claudium's own art)

fn hash(x: u32, y: u32, seed: u32) -> u32 {
    let mut h = x.wrapping_mul(374761393) ^ y.wrapping_mul(668265263) ^ seed.wrapping_mul(2246822519);
    h = (h ^ (h >> 13)).wrapping_mul(1274126177);
    h ^ (h >> 16)
}

fn noise01(x: usize, y: usize, seed: u32) -> f32 {
    (hash(x as u32, y as u32, seed) & 0xffff) as f32 / 65535.0
}

fn shade(c: [u8; 3], f: f32) -> [u8; 4] {
    [(c[0] as f32 * f).clamp(0.0, 255.0) as u8, (c[1] as f32 * f).clamp(0.0, 255.0) as u8, (c[2] as f32 * f).clamp(0.0, 255.0) as u8, 255]
}

fn name_seed(name: &str) -> u32 {
    name.bytes().fold(2166136261u32, |h, b| (h ^ b as u32).wrapping_mul(16777619))
}

/// Draws a 16x16 tile for `name` without any game files.
pub fn procedural(name: &str) -> Vec<u8> {
    let seed = name_seed(name);
    let mut px = vec![0u8; TILE * TILE * 4];
    let mut put = |x: usize, y: usize, c: [u8; 4]| px[(y * TILE + x) * 4..][..4].copy_from_slice(&c);

    let speckled = |base: [u8; 3], amount: f32| move |x: usize, y: usize| shade(base, 1.0 - amount + noise01(x, y, seed) * amount * 2.0);

    if let Some(h) = name.strip_prefix("#color") {
        let h: u32 = h.parse().unwrap_or(0);
        let c = [(h * 97 % 200 + 40) as u8, (h * 57 % 200 + 40) as u8, (h * 151 % 200 + 40) as u8];
        let f = speckled(c, 0.12);
        for y in 0..TILE {
            for x in 0..TILE {
                put(x, y, f(x, y));
            }
        }
        return px;
    }

    let ore = |base_ore: [u8; 3]| {
        move |x: usize, y: usize| {
            let n = noise01(x / 2, y / 2, seed ^ 99);
            if n > 0.78 {
                shade(base_ore, 0.8 + noise01(x, y, seed) * 0.4)
            } else {
                shade([125, 125, 125], 0.85 + noise01(x, y, seed) * 0.3)
            }
        }
    };

    for y in 0..TILE {
        for x in 0..TILE {
            let c = match name {
                "stone" | "missing" => speckled([125, 125, 125], 0.15)(x, y),
                "cobblestone" => {
                    let cell = hash((x / 4) as u32 + (y / 3) as u32 * 7, 0, seed);
                    let edge = x % 4 == 0 || (y + (x / 4) % 2) % 3 == 0;
                    shade([120, 120, 120], if edge { 0.6 } else { 0.85 + (cell & 0xff) as f32 / 900.0 })
                }
                "dirt" => speckled([134, 96, 67], 0.2)(x, y),
                "grass_block_top" | "short_grass_tint" => shade([150, 150, 150], 0.85 + noise01(x, y, seed) * 0.3),
                "grass_block_side" => {
                    let fringe = 3 + (hash(x as u32, 0, seed) % 3) as usize;
                    if y < fringe { shade([100, 160, 60], 0.85 + noise01(x, y, seed) * 0.3) } else { speckled([134, 96, 67], 0.2)(x, y) }
                }
                "grass_block_snow" => {
                    let fringe = 3 + (hash(x as u32, 0, seed) % 3) as usize;
                    if y < fringe { shade([245, 250, 255], 0.95) } else { speckled([134, 96, 67], 0.2)(x, y) }
                }
                "sand" => speckled([219, 207, 163], 0.08)(x, y),
                "sandstone" | "sandstone_top" | "sandstone_bottom" => {
                    let band = if y % 5 == 0 { 0.88 } else { 1.0 };
                    shade([216, 203, 155], band * (0.95 + noise01(x, y, seed) * 0.08))
                }
                "gravel" => speckled([136, 126, 126], 0.3)(x, y),
                "clay" => speckled([160, 166, 179], 0.06)(x, y),
                "snow" | "snow_block" => speckled([245, 250, 255], 0.03)(x, y),
                "bedrock" => speckled([85, 85, 85], 0.6)(x, y),
                "water_still" => shade([180, 190, 230], 0.9 + noise01(x / 2, y, seed) * 0.15),
                "lava_still" => shade([230, 110, 20], 0.85 + noise01(x / 2, y / 2, seed) * 0.3),
                "oak_log" | "spruce_log" => {
                    let base = if name == "oak_log" { [109, 85, 50] } else { [58, 37, 16] };
                    let stripe = if (x + (hash(0, y as u32 / 4, seed) % 2) as usize) % 3 == 0 { 0.75 } else { 1.0 };
                    shade(base, stripe * (0.9 + noise01(x, y, seed) * 0.15))
                }
                "oak_log_top" | "spruce_log_top" => {
                    let dx = x as f32 - 7.5;
                    let dy = y as f32 - 7.5;
                    let r = (dx * dx + dy * dy).sqrt();
                    if r > 7.0 { shade([109, 85, 50], 0.8) } else { shade([176, 143, 90], if (r as usize) % 2 == 0 { 0.9 } else { 1.0 }) }
                }
                "oak_leaves" | "spruce_leaves" => {
                    if noise01(x, y, seed) < 0.18 {
                        [0, 0, 0, 0]
                    } else {
                        shade([150, 150, 150], 0.7 + noise01(x, y, seed ^ 5) * 0.4)
                    }
                }
                "oak_planks" => {
                    let row = y / 4;
                    let seam = y % 4 == 3 || (x + row * 5) % 16 == 0;
                    shade([162, 130, 78], if seam { 0.7 } else { 0.92 + noise01(x, y, seed) * 0.1 })
                }
                "bricks" => {
                    let row = y / 4;
                    let mortar = y % 4 == 3 || (x + if row % 2 == 0 { 0 } else { 4 }) % 8 == 7;
                    if mortar { shade([180, 170, 160], 0.9) } else { shade([150, 75, 60], 0.9 + noise01(x, y, seed) * 0.15) }
                }
                "glass" => {
                    let edge = x == 0 || y == 0 || x == 15 || y == 15;
                    let glint = (x == y + 3 || x == y + 4) && x > 4 && x < 12;
                    if edge || glint { [220, 235, 240, 255] } else { [0, 0, 0, 0] }
                }
                "cactus_side" | "cactus_top" | "cactus_bottom" => {
                    let rib = x % 4 == 1;
                    shade([85, 127, 43], if rib { 0.8 } else { 1.0 })
                }
                "coal_ore" => ore([40, 40, 40])(x, y),
                "iron_ore" => ore([216, 175, 147])(x, y),
                "diamond_ore" => ore([93, 236, 245])(x, y),
                "short_grass" | "fern" | "tall_grass_bottom" | "tall_grass_top" => {
                    let blade = hash(x as u32, 1, seed) % 16;
                    if (15 - y) as u32 <= blade && x % 2 == (y / 5) % 2 {
                        shade([150, 150, 150], 0.8 + noise01(x, y, seed) * 0.4)
                    } else {
                        [0, 0, 0, 0]
                    }
                }
                "poppy" | "dandelion" => {
                    let petal = if name == "poppy" { [200, 30, 30] } else { [240, 210, 30] };
                    let stem = x == 7 && y > 7;
                    let dx = x as i32 - 7;
                    let dy = y as i32 - 5;
                    if dx * dx + dy * dy <= 6 {
                        shade(petal, 0.9 + noise01(x, y, seed) * 0.2)
                    } else if stem {
                        shade([60, 130, 40], 1.0)
                    } else {
                        [0, 0, 0, 0]
                    }
                }
                _ => speckled([120 + (seed % 100) as u8, 110 + (seed / 7 % 100) as u8, 100 + (seed / 13 % 100) as u8], 0.12)(x, y),
            };
            put(x, y, c);
        }
    }
    px
}

/// Builds the RGBA atlas: tiles laid out left-to-right, top-to-bottom.
pub fn build_atlas(tiles: &[String], src: &TextureSource) -> (Vec<u8>, u32, u32, usize) {
    let per_row = 64usize;
    let rows = tiles.len().div_ceil(per_row).max(1);
    let w = per_row * TILE;
    let h = (rows.next_power_of_two()) * TILE;
    let mut atlas = vec![0u8; w * h * 4];
    let mut from_game = 0;
    for (i, name) in tiles.iter().enumerate() {
        let px = match src.decode(name) {
            Some(p) => {
                from_game += 1;
                p
            }
            None => procedural(name),
        };
        let tx = (i % per_row) * TILE;
        let ty = (i / per_row) * TILE;
        for y in 0..TILE {
            let dst = ((ty + y) * w + tx) * 4;
            atlas[dst..dst + TILE * 4].copy_from_slice(&px[y * TILE * 4..(y + 1) * TILE * 4]);
        }
    }
    (atlas, w as u32, h as u32, from_game)
}
