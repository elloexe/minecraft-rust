//! Chunk columns and the loaded world.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use crate::registry::Registry;

pub const SECTION: usize = 4096;

#[inline]
pub fn idx(x: usize, y: usize, z: usize) -> usize {
    (y << 8) | (z << 4) | x
}

/// A 16-wide column of 16x16x16 sections. Empty sections are `None` (all air).
/// Light is stored per block as `sky << 4 | block`.
#[derive(Clone)]
pub struct Column {
    pub sections: Vec<Option<Box<[u16; SECTION]>>>,
    pub light: Vec<Option<Box<[u8; SECTION]>>>,
}

impl Column {
    pub fn new(sections: usize) -> Self {
        Column { sections: vec![None; sections], light: vec![None; sections] }
    }

    #[inline]
    pub fn get(&self, x: usize, y: usize, z: usize) -> u16 {
        match self.sections.get(y >> 4) {
            Some(Some(s)) => s[idx(x, y & 15, z)],
            _ => 0,
        }
    }

    pub fn set(&mut self, x: usize, y: usize, z: usize, id: u16) {
        let si = y >> 4;
        if si >= self.sections.len() {
            return;
        }
        let s = self.sections[si].get_or_insert_with(|| Box::new([0; SECTION]));
        s[idx(x, y & 15, z)] = id;
    }

    /// Light at a block; sections without data count as full skylight.
    #[inline]
    pub fn light(&self, x: usize, y: usize, z: usize) -> u8 {
        match self.light.get(y >> 4) {
            Some(Some(l)) => l[idx(x, y & 15, z)],
            Some(None) => 0xF0,
            None => 0xF0,
        }
    }

    pub fn height(&self) -> usize {
        self.sections.len() * 16
    }

    /// Simple skylight for singleplayer: sunlight falls straight down until it
    /// meets an opaque block, then spreads sideways inside the column.
    pub fn compute_skylight(&mut self, reg: &Registry) {
        let h = self.height();
        let n = self.sections.len();
        let mut sky = vec![0u8; 256 * h];
        let at = |x: usize, y: usize, z: usize| (y * 16 + z) * 16 + x;
        let mut queue: Vec<(usize, usize, usize)> = Vec::new();
        for z in 0..16 {
            for x in 0..16 {
                let mut level = 15u8;
                for y in (0..h).rev() {
                    let b = reg.get(self.get(x, y, z));
                    if b.occludes() {
                        break;
                    }
                    if b.kind == crate::registry::Kind::Water {
                        level = level.saturating_sub(2);
                    }
                    sky[at(x, y, z)] = level;
                    if level > 1 {
                        queue.push((x, y, z));
                    }
                }
            }
        }
        // Flood fill to light overhangs and cave mouths.
        while let Some((x, y, z)) = queue.pop() {
            let l = sky[at(x, y, z)];
            if l <= 1 {
                continue;
            }
            let nb: [(i32, i32, i32); 6] = [(1, 0, 0), (-1, 0, 0), (0, 1, 0), (0, -1, 0), (0, 0, 1), (0, 0, -1)];
            for (dx, dy, dz) in nb {
                let (nx, ny, nz) = (x as i32 + dx, y as i32 + dy, z as i32 + dz);
                if nx < 0 || nz < 0 || ny < 0 || nx > 15 || nz > 15 || ny >= h as i32 {
                    continue;
                }
                let (nx, ny, nz) = (nx as usize, ny as usize, nz as usize);
                if reg.get(self.get(nx, ny, nz)).occludes() {
                    continue;
                }
                let i = at(nx, ny, nz);
                if sky[i] + 1 < l {
                    sky[i] = l - 1;
                    queue.push((nx, ny, nz));
                }
            }
        }
        for si in 0..n {
            let mut l = Box::new([0u8; SECTION]);
            let mut all_full = true;
            for y in 0..16 {
                for z in 0..16 {
                    for x in 0..16 {
                        let v = sky[at(x, si * 16 + y, z)];
                        if v != 15 {
                            all_full = false;
                        }
                        l[idx(x, y, z)] = v << 4;
                    }
                }
            }
            self.light[si] = if all_full { None } else { Some(l) };
        }
    }
}

pub type ColumnRef = Arc<Column>;

pub struct World {
    pub columns: HashMap<(i32, i32), ColumnRef>,
    pub min_y: i32,
    pub sections: usize,
    pub registry: Arc<Registry>,
    /// Columns whose mesh needs rebuilding.
    pub dirty: HashSet<(i32, i32)>,
    /// Columns edited by the player (saved in singleplayer).
    pub modified: HashSet<(i32, i32)>,
}

impl World {
    pub fn new(registry: Arc<Registry>, min_y: i32, sections: usize) -> Self {
        World { columns: HashMap::new(), min_y, sections, registry, dirty: HashSet::new(), modified: HashSet::new() }
    }

    pub fn height(&self) -> i32 {
        self.sections as i32 * 16
    }

    pub fn insert(&mut self, cx: i32, cz: i32, col: Column) {
        self.columns.insert((cx, cz), Arc::new(col));
        self.dirty.insert((cx, cz));
        for (dx, dz) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
            if self.columns.contains_key(&(cx + dx, cz + dz)) {
                self.dirty.insert((cx + dx, cz + dz));
            }
        }
    }

    pub fn remove(&mut self, cx: i32, cz: i32) -> Option<ColumnRef> {
        self.dirty.remove(&(cx, cz));
        self.columns.remove(&(cx, cz))
    }

    pub fn clear(&mut self) {
        self.columns.clear();
        self.dirty.clear();
    }

    #[inline]
    pub fn get(&self, x: i32, y: i32, z: i32) -> u16 {
        let ly = y - self.min_y;
        if ly < 0 || ly >= self.height() {
            return 0;
        }
        match self.columns.get(&(x >> 4, z >> 4)) {
            Some(c) => c.get((x & 15) as usize, ly as usize, (z & 15) as usize),
            None => 0,
        }
    }

    pub fn is_loaded(&self, x: i32, z: i32) -> bool {
        self.columns.contains_key(&(x >> 4, z >> 4))
    }

    /// Changes a block and marks affected meshes dirty. Returns false if the column isn't loaded.
    pub fn set(&mut self, x: i32, y: i32, z: i32, id: u16, recompute_light: bool) -> bool {
        let ly = y - self.min_y;
        if ly < 0 || ly >= self.height() {
            return false;
        }
        let key = (x >> 4, z >> 4);
        let reg = self.registry.clone();
        let Some(col) = self.columns.get_mut(&key) else { return false };
        let col = Arc::make_mut(col);
        let (lx, lz) = ((x & 15) as usize, (z & 15) as usize);
        col.set(lx, ly as usize, lz, id);
        if recompute_light {
            col.compute_skylight(&reg);
        }
        self.dirty.insert(key);
        if lx == 0 {
            self.dirty.insert((key.0 - 1, key.1));
        }
        if lx == 15 {
            self.dirty.insert((key.0 + 1, key.1));
        }
        if lz == 0 {
            self.dirty.insert((key.0, key.1 - 1));
        }
        if lz == 15 {
            self.dirty.insert((key.0, key.1 + 1));
        }
        self.dirty.retain(|k| self.columns.contains_key(k));
        true
    }

    /// Steps through blocks along a ray (DDA). Returns the hit block and the empty
    /// block in front of it (where a new block would go).
    pub fn raycast(&self, origin: glam::Vec3, dir: glam::Vec3, max: f32) -> Option<Hit> {
        let mut pos = origin.floor().as_ivec3();
        let step = dir.signum().as_ivec3();
        let inv = glam::Vec3::new(1.0 / dir.x.abs().max(1e-9), 1.0 / dir.y.abs().max(1e-9), 1.0 / dir.z.abs().max(1e-9));
        let frac = origin - origin.floor();
        let mut tmax = glam::Vec3::new(
            if dir.x > 0.0 { 1.0 - frac.x } else { frac.x } * inv.x,
            if dir.y > 0.0 { 1.0 - frac.y } else { frac.y } * inv.y,
            if dir.z > 0.0 { 1.0 - frac.z } else { frac.z } * inv.z,
        );
        let mut prev = pos;
        let mut t = 0.0;
        let mut face = 0u8;
        while t <= max {
            let id = self.get(pos.x, pos.y, pos.z);
            let info = self.registry.get(id);
            if !info.is_air() && info.kind != crate::registry::Kind::Water {
                return Some(Hit { block: pos, before: prev, id, face, dist: t });
            }
            prev = pos;
            if tmax.x < tmax.y && tmax.x < tmax.z {
                pos.x += step.x;
                t = tmax.x;
                tmax.x += inv.x;
                face = if step.x > 0 { 4 } else { 5 };
            } else if tmax.y < tmax.z {
                pos.y += step.y;
                t = tmax.y;
                tmax.y += inv.y;
                face = if step.y > 0 { 0 } else { 1 };
            } else {
                pos.z += step.z;
                t = tmax.z;
                tmax.z += inv.z;
                face = if step.z > 0 { 2 } else { 3 };
            }
        }
        None
    }
}

pub struct Hit {
    pub block: glam::IVec3,
    pub before: glam::IVec3,
    pub id: u16,
    /// Face hit, in protocol order: 0 bottom, 1 top, 2 north, 3 south, 4 west, 5 east.
    pub face: u8,
    pub dist: f32,
}
