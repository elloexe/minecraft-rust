//! Turns chunk columns into GPU vertices on worker threads.
//!
//! Each vertex is 8 bytes:
//!   a: x:5 | z:5 | y:13 (sixteenths) | face:3 | ao:2 | u:1 | v:1
//!   b: tile:16 | sky:4 | block:4 | tint:2

use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex};

use crate::registry::{Kind, Registry};
use crate::world::{Column, ColumnRef};
use crate::worldgen::Generator;

pub type Vertex = [u32; 2];

const FACES: [([i32; 3], [[u8; 3]; 4], [[u8; 2]; 4]); 6] = [
    ([1, 0, 0], [[1, 0, 0], [1, 1, 0], [1, 1, 1], [1, 0, 1]], [[1, 1], [1, 0], [0, 0], [0, 1]]),
    ([-1, 0, 0], [[0, 0, 0], [0, 0, 1], [0, 1, 1], [0, 1, 0]], [[0, 1], [1, 1], [1, 0], [0, 0]]),
    ([0, 1, 0], [[0, 1, 0], [0, 1, 1], [1, 1, 1], [1, 1, 0]], [[0, 0], [0, 1], [1, 1], [1, 0]]),
    ([0, -1, 0], [[0, 0, 0], [1, 0, 0], [1, 0, 1], [0, 0, 1]], [[0, 0], [1, 0], [1, 1], [0, 1]]),
    ([0, 0, 1], [[0, 0, 1], [1, 0, 1], [1, 1, 1], [0, 1, 1]], [[0, 1], [1, 1], [1, 0], [0, 0]]),
    ([0, 0, -1], [[1, 0, 0], [0, 0, 0], [0, 1, 0], [1, 1, 0]], [[0, 1], [1, 1], [1, 0], [0, 0]]),
];

#[inline]
fn pack(x: u32, y16: u32, z: u32, face: u32, ao: u32, u: u32, v: u32, tile: u32, light: u8, tint: u32) -> Vertex {
    let a = x | (z << 5) | (y16 << 10) | (face << 23) | (ao << 26) | (u << 28) | (v << 29);
    let b = tile | ((light as u32 >> 4) << 16) | ((light as u32 & 15) << 20) | (tint << 24);
    [a, b]
}

struct Neighbourhood<'a> {
    c: &'a Column,
    n: [Option<&'a Column>; 4], // -x, +x, -z, +z
    h: i32,
}

impl Neighbourhood<'_> {
    /// Block id, or None outside the loaded area / world (treated as solid below, air above).
    #[inline]
    fn id(&self, x: i32, y: i32, z: i32) -> Option<u16> {
        if y < 0 {
            return None;
        }
        if y >= self.h {
            return Some(0);
        }
        let col = if x < 0 {
            self.n[0]?
        } else if x > 15 {
            self.n[1]?
        } else if z < 0 {
            self.n[2]?
        } else if z > 15 {
            self.n[3]?
        } else {
            self.c
        };
        Some(col.get((x & 15) as usize, y as usize, (z & 15) as usize))
    }
    #[inline]
    fn light(&self, x: i32, y: i32, z: i32) -> u8 {
        if y < 0 {
            return 0;
        }
        if y >= self.h {
            return 0xF0;
        }
        let col = if x < 0 {
            self.n[0]
        } else if x > 15 {
            self.n[1]
        } else if z < 0 {
            self.n[2]
        } else if z > 15 {
            self.n[3]
        } else {
            Some(self.c)
        };
        match col {
            Some(c) => c.light((x & 15) as usize, y as usize, (z & 15) as usize),
            None => 0xF0,
        }
    }
}

pub struct MeshData {
    pub opaque: Vec<Vertex>,
    pub water: Vec<Vertex>,
}

pub fn mesh_column(center: &Column, neighbours: [Option<&Column>; 4], reg: &Registry) -> MeshData {
    let nb = Neighbourhood { c: center, n: neighbours, h: center.height() as i32 };
    let mut opaque = Vec::new();
    let mut water = Vec::new();
    let occl = |x: i32, y: i32, z: i32| -> bool {
        match nb.id(x, y, z) {
            Some(id) => reg.get(id).occludes(),
            None => y < 0,
        }
    };

    for (si, sec) in center.sections.iter().enumerate() {
        let Some(sec) = sec else { continue };
        for ly in 0..16 {
            let y = (si * 16 + ly) as i32;
            for z in 0..16i32 {
                for x in 0..16i32 {
                    let id = sec[crate::world::idx(x as usize, ly, z as usize)];
                    if id == 0 {
                        continue;
                    }
                    let b = reg.get(id);
                    match b.kind {
                        Kind::Air | Kind::Invisible => continue,
                        Kind::Cross => {
                            let light = nb.light(x, y, z);
                            let tile = b.tex[0] as u32;
                            let tint = b.tint[0] as u32;
                            let quads: [[(u32, u32); 2]; 2] = [[(0, 0), (1, 1)], [(1, 0), (0, 1)]];
                            for q in quads {
                                let (a, c) = (q[0], q[1]);
                                let yb = (y as u32) * 16;
                                let corners =
                                    [(a.0, yb, a.1, 0, 1), (c.0, yb, c.1, 1, 1), (c.0, yb + 16, c.1, 1, 0), (a.0, yb + 16, a.1, 0, 0)];
                                // Both windings so it's visible from either side.
                                for order in [[0usize, 1, 2, 3], [1, 0, 3, 2]] {
                                    for i in order {
                                        let (vx, vy, vz, u, v) = corners[i];
                                        opaque.push(pack(x as u32 + vx, vy, z as u32 + vz, 6, 3, u, v, tile, light, tint));
                                    }
                                }
                            }
                        }
                        _ => {
                            let is_water = b.kind == Kind::Water;
                            let above = nb.id(x, y + 1, z).unwrap_or(0);
                            let water_top_low = is_water && reg.get(above).kind != Kind::Water;
                            let (y0, y1) = if is_water { (0u32, if water_top_low { 14 } else { 16 }) } else { (b.y0 as u32, b.y1 as u32) };
                            for (f, (n, corners, uvs)) in FACES.iter().enumerate() {
                                let (nx, ny, nz) = (x + n[0], y + n[1], z + n[2]);
                                let nid = match nb.id(nx, ny, nz) {
                                    Some(i) => i,
                                    None => {
                                        if ny < 0 {
                                            continue;
                                        }
                                        0 // unloaded neighbour column: draw the face
                                    }
                                };
                                let ninfo = reg.get(nid);
                                let open_top = f == 2 && y1 < 16;
                                let open_bottom = f == 3 && y0 > 0;
                                if !(open_top || open_bottom) {
                                    if ninfo.occludes() {
                                        continue;
                                    }
                                    if b.cull_same && nid == id {
                                        continue;
                                    }
                                    if is_water && ninfo.kind == Kind::Water {
                                        continue;
                                    }
                                }
                                let light = nb.light(nx, ny, nz).max(if open_top || open_bottom { nb.light(x, y, z) } else { 0 });
                                let tex_i = match f {
                                    2 => 0,
                                    3 => 1,
                                    _ => 2,
                                };
                                let tile = b.tex[tex_i] as u32;
                                let tint = b.tint[tex_i] as u32;
                                let full = y0 == 0 && y1 == 16 && !is_water;
                                // Ambient occlusion from the three blocks around each corner.
                                let mut ao = [3u32; 4];
                                if full {
                                    let axis = if n[0] != 0 {
                                        0
                                    } else if n[1] != 0 {
                                        1
                                    } else {
                                        2
                                    };
                                    let (t1, t2) = match axis {
                                        0 => (1, 2),
                                        1 => (0, 2),
                                        _ => (0, 1),
                                    };
                                    for (ci, c) in corners.iter().enumerate() {
                                        let mut d1 = [0i32; 3];
                                        let mut d2 = [0i32; 3];
                                        d1[t1] = if c[t1] == 1 { 1 } else { -1 };
                                        d2[t2] = if c[t2] == 1 { 1 } else { -1 };
                                        let s1 = occl(nx + d1[0], ny + d1[1], nz + d1[2]);
                                        let s2 = occl(nx + d2[0], ny + d2[1], nz + d2[2]);
                                        let cc = occl(nx + d1[0] + d2[0], ny + d1[1] + d2[1], nz + d1[2] + d2[2]);
                                        ao[ci] = if s1 && s2 { 0 } else { 3 - (s1 as u32 + s2 as u32 + cc as u32) };
                                    }
                                }
                                let mut order = [0usize, 1, 2, 3];
                                if ao[0] + ao[2] < ao[1] + ao[3] {
                                    order = [1, 2, 3, 0];
                                }
                                let out = if is_water { &mut water } else { &mut opaque };
                                for &i in &order {
                                    let c = corners[i];
                                    let vy = y as u32 * 16 + if c[1] == 1 { y1 } else { y0 };
                                    out.push(pack(
                                        x as u32 + c[0] as u32,
                                        vy,
                                        z as u32 + c[2] as u32,
                                        f as u32,
                                        ao[i],
                                        uvs[i][0] as u32,
                                        uvs[i][1] as u32,
                                        tile,
                                        light,
                                        tint,
                                    ));
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    MeshData { opaque, water }
}

// ---------------------------------------------------------------------------
// Worker pool

pub enum Job {
    Mesh { key: (i32, i32), version: u64, center: ColumnRef, neighbours: [Option<ColumnRef>; 4] },
    Generate { key: (i32, i32) },
}

pub enum JobResult {
    Mesh { key: (i32, i32), version: u64, data: MeshData },
    Generated { key: (i32, i32), column: Column },
}

pub struct Pool {
    pub jobs: Sender<Job>,
    pub results: Receiver<JobResult>,
    pub in_flight: usize,
    pub threads: usize,
}

impl Pool {
    pub fn new(reg: Arc<Registry>, generator: Option<Arc<Generator>>) -> Self {
        let (jtx, jrx) = channel::<Job>();
        let (rtx, rrx) = channel::<JobResult>();
        let jrx = Arc::new(Mutex::new(jrx));
        let threads = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(2).saturating_sub(1).clamp(1, 8);
        for i in 0..threads {
            let jrx = jrx.clone();
            let rtx = rtx.clone();
            let reg = reg.clone();
            let generator = generator.clone();
            std::thread::Builder::new()
                .name(format!("claudium-worker-{i}"))
                .spawn(move || {
                    loop {
                        let job = { jrx.lock().unwrap().recv() };
                        let Ok(job) = job else { return };
                        let res = match job {
                            Job::Mesh { key, version, center, neighbours } => {
                                let n = [
                                    neighbours[0].as_deref(),
                                    neighbours[1].as_deref(),
                                    neighbours[2].as_deref(),
                                    neighbours[3].as_deref(),
                                ];
                                JobResult::Mesh { key, version, data: mesh_column(&center, n, &reg) }
                            }
                            Job::Generate { key } => {
                                let g = generator.as_ref().expect("generator");
                                let mut column = g.generate(key.0, key.1);
                                column.compute_skylight(&reg);
                                JobResult::Generated { key, column }
                            }
                        };
                        if rtx.send(res).is_err() {
                            return;
                        }
                    }
                })
                .expect("spawn worker");
        }
        Pool { jobs: jtx, results: rrx, in_flight: 0, threads }
    }

    pub fn submit(&mut self, job: Job) {
        self.in_flight += 1;
        let _ = self.jobs.send(job);
    }
}
