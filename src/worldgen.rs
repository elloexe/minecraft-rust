//! Claudium's own singleplayer terrain generator.

use crate::registry::Registry;
use crate::world::Column;

#[inline]
fn hash3(x: i32, y: i32, z: i32, seed: u32) -> u32 {
    let mut h = (x as u32).wrapping_mul(0x8da6b343)
        ^ (y as u32).wrapping_mul(0xd8163841)
        ^ (z as u32).wrapping_mul(0xcb1ab31f)
        ^ seed.wrapping_mul(0x9e3779b9);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2c1b3c6d);
    h ^= h >> 12;
    h = h.wrapping_mul(0x297a2d39);
    h ^ (h >> 15)
}

#[inline]
fn grad(h: u32, x: f32, y: f32, z: f32) -> f32 {
    match h & 15 {
        0 => x + y,
        1 => -x + y,
        2 => x - y,
        3 => -x - y,
        4 => x + z,
        5 => -x + z,
        6 => x - z,
        7 => -x - z,
        8 => y + z,
        9 => -y + z,
        10 => y - z,
        11 => -y - z,
        12 => x + y,
        13 => -x + y,
        14 => -y + z,
        _ => -y - z,
    }
}

#[inline]
fn fade(t: f32) -> f32 {
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}

/// Gradient noise in roughly -1..1.
pub fn noise3(x: f32, y: f32, z: f32, seed: u32) -> f32 {
    let (x0, y0, z0) = (x.floor(), y.floor(), z.floor());
    let (xi, yi, zi) = (x0 as i32, y0 as i32, z0 as i32);
    let (fx, fy, fz) = (x - x0, y - y0, z - z0);
    let (u, v, w) = (fade(fx), fade(fy), fade(fz));
    let g = |dx: i32, dy: i32, dz: i32| grad(hash3(xi + dx, yi + dy, zi + dz, seed), fx - dx as f32, fy - dy as f32, fz - dz as f32);
    let lerp = |a: f32, b: f32, t: f32| a + (b - a) * t;
    lerp(
        lerp(lerp(g(0, 0, 0), g(1, 0, 0), u), lerp(g(0, 1, 0), g(1, 1, 0), u), v),
        lerp(lerp(g(0, 0, 1), g(1, 0, 1), u), lerp(g(0, 1, 1), g(1, 1, 1), u), v),
        w,
    )
}

pub fn fbm2(x: f32, z: f32, octaves: u32, seed: u32) -> f32 {
    let (mut sum, mut amp, mut freq, mut norm) = (0.0, 1.0, 1.0, 0.0);
    for o in 0..octaves {
        sum += noise3(x * freq, 0.5, z * freq, seed.wrapping_add(o * 131)) * amp;
        norm += amp;
        amp *= 0.5;
        freq *= 2.0;
    }
    sum / norm
}

pub struct Generator {
    pub seed: u32,
    ids: Ids,
}

struct Ids {
    stone: u16,
    dirt: u16,
    grass: u16,
    sand: u16,
    water: u16,
    log: u16,
    leaves: u16,
    bedrock: u16,
    gravel: u16,
    snow: u16,
    tallgrass: u16,
    poppy: u16,
    dandelion: u16,
    sandstone: u16,
    coal: u16,
    iron: u16,
    spruce_log: u16,
    spruce_leaves: u16,
    cactus: u16,
    clay: u16,
    diamond: u16,
}

pub const SEA_LEVEL: i32 = 62;
pub const SP_SECTIONS: usize = 16; // 256 blocks tall, y = 0..256

impl Generator {
    pub fn new(seed: u32, reg: &Registry) -> Self {
        let id = |n: &str| reg.id_of(n).unwrap_or(1);
        Generator {
            seed,
            ids: Ids {
                stone: id("stone"),
                dirt: id("dirt"),
                grass: id("grass_block"),
                sand: id("sand"),
                water: id("water"),
                log: id("oak_log"),
                leaves: id("oak_leaves"),
                bedrock: id("bedrock"),
                gravel: id("gravel"),
                snow: id("snow_block"),
                tallgrass: id("short_grass"),
                poppy: id("poppy"),
                dandelion: id("dandelion"),
                sandstone: id("sandstone"),
                coal: id("coal_ore"),
                iron: id("iron_ore"),
                spruce_log: id("spruce_log"),
                spruce_leaves: id("spruce_leaves"),
                cactus: id("cactus"),
                clay: id("clay"),
                diamond: id("diamond_ore"),
            },
        }
    }

    /// Terrain height and climate at a world column.
    pub fn sample(&self, x: i32, z: i32) -> (i32, f32, f32) {
        let (fx, fz) = (x as f32, z as f32);
        let s = self.seed;
        let continent = fbm2(fx * 0.0016, fz * 0.0016, 4, s);
        let hills = fbm2(fx * 0.008, fz * 0.008, 5, s ^ 0x55);
        let ridge = 1.0 - fbm2(fx * 0.004, fz * 0.004, 4, s ^ 0x99).abs() * 2.0;
        let mountain = ((continent - 0.1) * 3.0).clamp(0.0, 1.0);
        let mut h = 64.0 + continent * 40.0 + hills * 10.0 + mountain * ridge.max(0.0).powi(2) * 70.0;
        if continent < -0.15 {
            h -= (-0.15 - continent) * 60.0; // oceans
        }
        let temp = fbm2(fx * 0.0025, fz * 0.0025, 3, s ^ 0x1234) + (h - 80.0).max(0.0) * -0.01;
        let wet = fbm2(fx * 0.003, fz * 0.003, 3, s ^ 0x4321);
        (h.clamp(4.0, 240.0) as i32, temp, wet)
    }

    fn is_cave(&self, x: i32, y: i32, z: i32, surface: i32) -> bool {
        if y < 6 || y > surface - 3 {
            return false;
        }
        let (fx, fy, fz) = (x as f32, y as f32 * 1.6, z as f32);
        let a = noise3(fx * 0.03, fy * 0.03, fz * 0.03, self.seed ^ 0xA);
        let b = noise3(fx * 0.03, fy * 0.03, fz * 0.03, self.seed ^ 0xB);
        a * a + b * b < 0.004
    }

    pub fn generate(&self, cx: i32, cz: i32) -> Column {
        let ids = &self.ids;
        let mut col = Column::new(SP_SECTIONS);
        let mut surface = [[0i32; 16]; 16];
        for lz in 0..16 {
            for lx in 0..16 {
                let (x, z) = (cx * 16 + lx as i32, cz * 16 + lz as i32);
                let (h, temp, _wet) = self.sample(x, z);
                surface[lz][lx] = h;
                let desert = temp > 0.25;
                let snowy = temp < -0.3;
                let beach = h <= SEA_LEVEL + 1 && h >= SEA_LEVEL - 3;
                for y in 0..=h.max(SEA_LEVEL) {
                    let id = if y == 0 || (y < 4 && hash3(x, y, z, self.seed) % 3 == 0) {
                        ids.bedrock
                    } else if y > h {
                        ids.water
                    } else if self.is_cave(x, y, z, h) {
                        0
                    } else if y == h {
                        if h < SEA_LEVEL - 3 {
                            if hash3(x, 1, z, self.seed) % 4 == 0 { ids.clay } else { ids.gravel }
                        } else if desert || beach {
                            ids.sand
                        } else if snowy || h > 150 {
                            ids.snow
                        } else if h < SEA_LEVEL {
                            ids.dirt
                        } else {
                            ids.grass
                        }
                    } else if y > h - 4 {
                        if desert || beach { if y > h - 3 { ids.sand } else { ids.sandstone } } else { ids.dirt }
                    } else {
                        let r = hash3(x, y, z, self.seed ^ 0x77) % 1000;
                        if r < 9 && y < 120 {
                            ids.coal
                        } else if r < 14 && y < 64 {
                            ids.iron
                        } else if r < 15 && y < 16 {
                            ids.diamond
                        } else {
                            ids.stone
                        }
                    };
                    if id != 0 {
                        col.set(lx, y as usize, lz, id);
                    }
                }
            }
        }

        // Decorations: trees may reach into this chunk from neighbours, so look 3 blocks out.
        for wz in cz * 16 - 3..cz * 16 + 19 {
            for wx in cx * 16 - 3..cx * 16 + 19 {
                let r = hash3(wx, 7, wz, self.seed ^ 0x7ee);
                let inside = |x: i32, z: i32| x >= cx * 16 && x < cx * 16 + 16 && z >= cz * 16 && z < cz * 16 + 16;
                let (h, temp, wet) = self.sample(wx, wz);
                if h <= SEA_LEVEL + 1 || h > 150 {
                    continue;
                }
                let on_grass = inside(wx, wz) && {
                    let (lx, lz) = ((wx - cx * 16) as usize, (wz - cz * 16) as usize);
                    col.get(lx, h as usize, lz) == ids.grass
                };
                let desert = temp > 0.25;
                let snowy = temp < -0.3;
                let density = if desert { 0 } else { 6 + ((wet + 0.5) * 20.0).max(0.0) as u32 };
                let mut put = |x: i32, y: i32, z: i32, id: u16, only_air: bool| {
                    if inside(x, z) && (0..256).contains(&y) {
                        let (lx, lz) = ((x - cx * 16) as usize, (z - cz * 16) as usize);
                        if !only_air || col.get(lx, y as usize, lz) == 0 {
                            col.set(lx, y as usize, lz, id);
                        }
                    }
                };
                if desert {
                    if r % 400 == 0 {
                        let tall = 1 + (r >> 9) % 3;
                        for dy in 1..=tall as i32 {
                            put(wx, h + dy, wz, ids.cactus, true);
                        }
                    }
                    continue;
                }
                if r % 1000 < density {
                    // A tree
                    let spruce = snowy || (r >> 10) % 5 == 0;
                    let trunk = 4 + ((r >> 12) % 3) as i32 + if spruce { 2 } else { 0 };
                    let (log, leaves) = if spruce { (ids.spruce_log, ids.spruce_leaves) } else { (ids.log, ids.leaves) };
                    for dy in 1..=trunk {
                        put(wx, h + dy, wz, log, false);
                    }
                    if inside(wx, wz) {
                        put(wx, h, wz, ids.dirt, false);
                    }
                    let top = h + trunk;
                    if spruce {
                        for dy in -4i32..=1 {
                            let rad: i32 = if dy == 1 {
                                0
                            } else if dy == 0 {
                                1
                            } else {
                                2 - (dy + 4) % 2
                            };
                            for dz in -rad..=rad {
                                for dx in -rad..=rad {
                                    if dx.abs() + dz.abs() <= rad + 1 {
                                        put(wx + dx, top + dy, wz + dz, leaves, true);
                                    }
                                }
                            }
                        }
                    } else {
                        for dy in -2i32..=1 {
                            let rad: i32 = if dy >= 0 { 1 } else { 2 };
                            for dz in -rad..=rad {
                                for dx in -rad..=rad {
                                    let corner = dx.abs() == rad && dz.abs() == rad;
                                    if corner && hash3(wx + dx, top + dy, wz + dz, self.seed) % 2 == 0 {
                                        continue;
                                    }
                                    put(wx + dx, top + dy, wz + dz, leaves, true);
                                }
                            }
                        }
                    }
                } else if inside(wx, wz) && !snowy {
                    let roll = r % 100;
                    let plant = if roll < 22 {
                        ids.tallgrass
                    } else if roll < 24 {
                        ids.poppy
                    } else if roll < 26 {
                        ids.dandelion
                    } else {
                        0
                    };
                    if plant != 0 && on_grass {
                        put(wx, h + 1, wz, plant, true);
                    }
                }
            }
        }
        col
    }
}
