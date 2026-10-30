//! Player movement and collision.

use glam::{DVec3, Vec3};

use crate::world::World;

pub const WIDTH: f64 = 0.6;
pub const HEIGHT: f64 = 1.8;
pub const EYE: f64 = 1.62;

#[derive(Default, Clone, Copy)]
pub struct MoveInput {
    pub forward: f32,
    pub strafe: f32,
    pub jump: bool,
    pub sneak: bool,
    pub sprint: bool,
}

pub struct Player {
    pub pos: DVec3, // feet
    pub vel: DVec3,
    pub yaw: f32,
    pub pitch: f32,
    pub on_ground: bool,
    pub flying: bool,
    pub can_fly: bool,
    pub horizontal_collision: bool,
    pub in_water: bool,
}

impl Player {
    pub fn new(pos: DVec3) -> Self {
        Player {
            pos,
            vel: DVec3::ZERO,
            yaw: 0.0,
            pitch: 0.0,
            on_ground: false,
            flying: false,
            can_fly: true,
            horizontal_collision: false,
            in_water: false,
        }
    }

    pub fn eye(&self) -> DVec3 {
        self.pos + DVec3::new(0.0, EYE, 0.0)
    }

    fn solid_at(world: &World, x: i32, y: i32, z: i32) -> Option<(f64, f64)> {
        if !world.is_loaded(x, z) {
            return Some((0.0, 1.0)); // unloaded terrain acts as a wall
        }
        let b = world.registry.get(world.get(x, y, z));
        b.collides().then(|| (b.y0 as f64 / 16.0, b.y1 as f64 / 16.0))
    }

    /// Moves along one axis, stopping at the first block hit. Returns true on collision.
    fn sweep(&mut self, world: &World, axis: usize, delta: f64) -> bool {
        if delta == 0.0 {
            return false;
        }
        let mut p = self.pos;
        p[axis] += delta;
        let half = WIDTH / 2.0;
        let lo = DVec3::new(p.x - half, p.y, p.z - half);
        let hi = DVec3::new(p.x + half, p.y + HEIGHT, p.z + half);
        let eps = 1e-7;
        let mut hit = false;
        let mut limit = delta;
        for bx in (lo.x + eps).floor() as i32..=(hi.x - eps).floor() as i32 {
            for by in (lo.y + eps).floor() as i32 - 1..=(hi.y - eps).floor() as i32 {
                for bz in (lo.z + eps).floor() as i32..=(hi.z - eps).floor() as i32 {
                    let Some((y0, y1)) = Self::solid_at(world, bx, by, bz) else { continue };
                    let blo = DVec3::new(bx as f64, by as f64 + y0, bz as f64);
                    let bhi = DVec3::new(bx as f64 + 1.0, by as f64 + y1, bz as f64 + 1.0);
                    if hi.x <= blo.x + eps
                        || lo.x >= bhi.x - eps
                        || hi.y <= blo.y + eps
                        || lo.y >= bhi.y - eps
                        || hi.z <= blo.z + eps
                        || lo.z >= bhi.z - eps
                    {
                        continue;
                    }
                    hit = true;
                    let cur_lo = match axis {
                        0 => self.pos.x - half,
                        1 => self.pos.y,
                        _ => self.pos.z - half,
                    };
                    let size = if axis == 1 { HEIGHT } else { WIDTH };
                    let allowed = if delta > 0.0 { blo[axis] - (cur_lo + size) } else { bhi[axis] - cur_lo };
                    if delta > 0.0 {
                        limit = limit.min(allowed.max(0.0));
                    } else {
                        limit = limit.max(allowed.min(0.0));
                    }
                }
            }
        }
        self.pos[axis] += if hit { limit } else { delta };
        hit
    }

    pub fn update(&mut self, world: &World, input: MoveInput, dt: f64) {
        let dt = dt.min(0.1);
        let feet = self.pos.floor();
        let water_id = world.get(feet.x as i32, (self.pos.y + 0.4).floor() as i32, feet.z as i32);
        self.in_water = world.registry.get(water_id).kind == crate::registry::Kind::Water;

        let (sy, cy) = (self.yaw as f64).sin_cos();
        let fwd = DVec3::new(-sy, 0.0, cy);
        let right = DVec3::new(-cy, 0.0, -sy);
        let mut wish = fwd * input.forward as f64 + right * input.strafe as f64;
        if wish.length_squared() > 1.0 {
            wish = wish.normalize();
        }

        if self.flying {
            let speed = if input.sprint { 21.6 } else { 10.9 };
            let target = wish * speed;
            let vy = (input.jump as i32 - input.sneak as i32) as f64 * 7.5;
            self.vel.x += (target.x - self.vel.x) * (dt * 10.0).min(1.0);
            self.vel.z += (target.z - self.vel.z) * (dt * 10.0).min(1.0);
            self.vel.y += (vy - self.vel.y) * (dt * 10.0).min(1.0);
        } else {
            let speed = if input.sneak {
                1.31
            } else if input.sprint {
                5.61
            } else {
                4.317
            };
            let speed = if self.in_water { speed * 0.5 } else { speed };
            let accel = if self.on_ground {
                20.0
            } else if self.in_water {
                6.0
            } else {
                4.0
            };
            let target = wish * speed;
            self.vel.x += (target.x - self.vel.x) * (dt * accel).min(1.0);
            self.vel.z += (target.z - self.vel.z) * (dt * accel).min(1.0);
            if self.in_water {
                self.vel.y -= 6.0 * dt;
                self.vel.y *= 1.0 - (dt * 3.0).min(1.0);
                if input.jump {
                    self.vel.y = self.vel.y.max(3.5);
                }
            } else {
                self.vel.y -= 32.0 * dt;
                self.vel.y = self.vel.y.max(-78.4);
                if input.jump && self.on_ground {
                    self.vel.y = 9.0;
                }
            }
        }

        let d = self.vel * dt;
        // Sneaking keeps you from walking off edges.
        let before = self.pos;
        let hit_y = self.sweep(world, 1, d.y);
        let hx = self.sweep(world, 0, d.x);
        let hz = self.sweep(world, 2, d.z);
        self.horizontal_collision = hx || hz;
        self.on_ground = hit_y && d.y < 0.0;
        if hit_y {
            self.vel.y = 0.0;
        }
        if hx {
            self.vel.x = 0.0;
        }
        if hz {
            self.vel.z = 0.0;
        }
        if input.sneak && self.on_ground && !self.flying {
            let below = |p: DVec3| -> bool {
                let h = WIDTH / 2.0 - 0.05;
                [(-h, -h), (h, -h), (-h, h), (h, h)].iter().any(|(ox, oz)| {
                    Self::solid_at(world, (p.x + ox).floor() as i32, (p.y - 0.5).floor() as i32, (p.z + oz).floor() as i32).is_some()
                })
            };
            if !below(self.pos) {
                self.pos.x = before.x;
                self.pos.z = before.z;
                self.vel.x = 0.0;
                self.vel.z = 0.0;
            }
        }
        if self.on_ground && self.flying {
            self.flying = false;
        }
    }

    /// True if a block at `b` would overlap the player.
    pub fn intersects_block(&self, b: glam::IVec3) -> bool {
        let half = WIDTH / 2.0;
        let lo = Vec3::new((self.pos.x - half) as f32, self.pos.y as f32, (self.pos.z - half) as f32);
        let hi = Vec3::new((self.pos.x + half) as f32, (self.pos.y + HEIGHT) as f32, (self.pos.z + half) as f32);
        let bl = b.as_vec3();
        let bh = bl + Vec3::ONE;
        hi.x > bl.x && lo.x < bh.x && hi.y > bl.y && lo.y < bh.y && hi.z > bl.z && lo.z < bh.z
    }
}
