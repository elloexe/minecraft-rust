use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use crate::mesher::mesh_column;
use crate::net::buf::{Reader, Writer, pack_position, unpack_position};
use crate::net::nbt;
use crate::registry::{self, Kind};
use crate::world::{Column, World};
use crate::worldgen::{Generator, SP_SECTIONS};

fn sp_registry() -> registry::Registry {
    registry::build_singleplayer(&|_| false).0
}

#[test]
fn varints_round_trip() {
    for v in [0, 1, 127, 128, 255, 25565, 2097151, i32::MAX, -1, i32::MIN] {
        let mut w = Writer::default();
        w.varint(v);
        let b = w.done();
        assert_eq!(Reader::new(&b).varint().unwrap(), v);
        assert!(b.len() <= 5);
    }
    // Known encodings
    let mut w = Writer::default();
    w.varint(25565);
    assert_eq!(w.done(), vec![0xdd, 0xc7, 0x01]);
    let mut w = Writer::default();
    w.varint(-1);
    assert_eq!(w.done(), vec![0xff, 0xff, 0xff, 0xff, 0x0f]);
}

#[test]
fn positions_round_trip() {
    for (x, y, z) in [(0, 0, 0), (-1, -64, -1), (33554431, 2047, -33554432), (18357644, 831, -20882616), (-5, 319, 7)] {
        assert_eq!(unpack_position(pack_position(x, y, z)), (x, y, z));
    }
}

#[test]
fn handshake_bytes() {
    let p = Writer::packet(0x00).varint(775).string("localhost").u16(25565).varint(2).done();
    let mut r = Reader::new(&p);
    assert_eq!(r.varint().unwrap(), 0);
    assert_eq!(r.varint().unwrap(), 775);
    assert_eq!(r.string().unwrap(), "localhost");
    assert_eq!(r.u16().unwrap(), 25565);
    assert_eq!(r.varint().unwrap(), 2);
    assert_eq!(r.remaining(), 0);
}

#[test]
fn nbt_text_components() {
    // {"translate":"chat.type.text","with":[{"text":"Alex"},"hello"]}
    let mut w = Writer::default();
    w.u8(10);
    w.u8(8).u16(9).raw(b"translate").u16(14).raw(b"chat.type.text");
    w.u8(9).u16(4).raw(b"with");
    w.u8(10).i32(1);
    w.u8(8).u16(4).raw(b"text").u16(4).raw(b"Alex");
    w.u8(0);
    w.u8(0);
    let b = w.done();
    let mut r = Reader::new(&b);
    assert_eq!(nbt::read_text(&mut r).unwrap(), "<Alex> ");
    assert_eq!(r.remaining(), 0);

    // Bare string root
    let mut w = Writer::default();
    w.u8(8).u16(5).raw(b"Hello");
    let b = w.done();
    assert_eq!(nbt::read_text(&mut Reader::new(&b)).unwrap(), "Hello");
}

#[test]
fn worldgen_is_deterministic_and_plausible() {
    let reg = sp_registry();
    let g = Generator::new(1234, &reg);
    let a = g.generate(3, -7);
    let b = g.generate(3, -7);
    for y in 0..256 {
        for z in 0..16 {
            for x in 0..16 {
                assert_eq!(a.get(x, y, z), b.get(x, y, z));
            }
        }
    }
    let bedrock = reg.id_of("bedrock").unwrap();
    assert_eq!(a.get(0, 0, 0), bedrock);
    // Every column has a surface somewhere between y=4 and y=240.
    for z in 0..16 {
        for x in 0..16 {
            let top = (0..256).rev().find(|&y| a.get(x, y, z) != 0).unwrap();
            assert!((4..=250).contains(&top), "surface at {top}");
        }
    }
}

#[test]
fn skylight_is_full_above_ground_and_dark_deep_down() {
    let reg = sp_registry();
    let g = Generator::new(99, &reg);
    let mut c = g.generate(0, 0);
    c.compute_skylight(&reg);
    assert_eq!(c.light(5, 250, 5) >> 4, 15);
    assert_eq!(c.light(5, 2, 5) >> 4, 0);
}

#[test]
fn mesher_culls_hidden_faces() {
    let reg = sp_registry();
    let stone = reg.id_of("stone").unwrap();
    // A single block: 6 faces * 4 vertices
    let mut c = Column::new(1);
    c.set(5, 5, 5, stone);
    let m = mesh_column(&c, [None, None, None, None], &reg);
    assert_eq!(m.opaque.len(), 24);
    // Two touching blocks share a hidden face: 10 faces
    c.set(6, 5, 5, stone);
    let m = mesh_column(&c, [None, None, None, None], &reg);
    assert_eq!(m.opaque.len(), 40);
    // A solid 3x3x3 cube shows only its outside: 6 * 9 faces
    let mut c = Column::new(1);
    for y in 1..4 {
        for z in 1..4 {
            for x in 1..4 {
                c.set(x, y, z, stone);
            }
        }
    }
    let m = mesh_column(&c, [None, None, None, None], &reg);
    assert_eq!(m.opaque.len(), 54 * 4);
    // Water next to water hides the shared face, and goes in the water mesh.
    let water = reg.id_of("water").unwrap();
    let mut c = Column::new(1);
    c.set(1, 1, 1, water);
    c.set(2, 1, 1, water);
    let m = mesh_column(&c, [None, None, None, None], &reg);
    assert_eq!(m.opaque.len(), 0);
    assert_eq!(m.water.len(), 10 * 4);
}

#[test]
fn mesher_vertex_packing() {
    let reg = sp_registry();
    let stone = reg.id_of("stone").unwrap();
    let mut c = Column::new(16);
    c.set(15, 200, 15, stone);
    let m = mesh_column(&c, [None, None, None, None], &reg);
    for v in &m.opaque {
        let x = v[0] & 31;
        let z = (v[0] >> 5) & 31;
        let y = ((v[0] >> 10) & 8191) as f32 / 16.0;
        assert!(x == 15 || x == 16);
        assert!(z == 15 || z == 16);
        assert!(y == 200.0 || y == 201.0);
        let face = (v[0] >> 23) & 7;
        assert!(face < 6);
    }
}

#[test]
fn ambient_occlusion_darkens_corners() {
    let reg = sp_registry();
    let stone = reg.id_of("stone").unwrap();
    let mut c = Column::new(1);
    // floor block with a wall block next to it, one up
    c.set(5, 5, 5, stone);
    c.set(6, 6, 5, stone);
    let m = mesh_column(&c, [None, None, None, None], &reg);
    // top face of (5,5,5): two corners touch the wall at x=6
    let top: Vec<_> = m
        .opaque
        .iter()
        .filter(|v| (v[0] >> 23) & 7 == 2 && ((v[0] >> 10) & 8191) == 6 * 16 && (v[0] & 31) <= 6 && (v[0] & 31) >= 5)
        .collect();
    let dark = top.iter().filter(|v| (v[0] >> 26) & 3 < 3).count();
    assert!(dark >= 2, "expected occluded corners, got {dark}");
}

#[test]
fn raycast_hits_the_right_face() {
    let reg = Arc::new(sp_registry());
    let mut w = World::new(reg.clone(), 0, 1);
    w.insert(0, 0, Column::new(1));
    let stone = reg.id_of("stone").unwrap();
    w.set(5, 5, 8, stone, false);
    let o = glam::Vec3::new(5.5, 5.5, 2.5);
    let d = glam::Vec3::new(0.0, 0.0, 1.0);
    let h = w.raycast(o, d, 10.0).expect("hit");
    assert_eq!((h.block.x, h.block.y, h.block.z), (5, 5, 8));
    assert_eq!((h.before.x, h.before.y, h.before.z), (5, 5, 7));
    assert_eq!(h.face, 2); // entered through the north (-Z) face
    // looking down onto it
    let h = w.raycast(glam::Vec3::new(5.5, 9.2, 8.5), glam::Vec3::new(0.0, -1.0, 0.0), 10.0).unwrap();
    assert_eq!(h.face, 1);
    assert_eq!((h.before.x, h.before.y, h.before.z), (5, 6, 8));
}

#[test]
fn classify_vanilla_style_names() {
    let have = |n: &str| {
        ["oak_planks", "oak_log", "oak_log_top", "grass_block_top", "stone_bricks", "white_wool", "tall_grass_top", "tall_grass_bottom"]
            .contains(&n)
    };
    let mut alloc = registry::TileAlloc::new();
    let mut props = HashMap::new();
    let b = registry::classify("oak_slab", &props, &have, &mut alloc);
    assert_eq!(b.kind, Kind::Solid);
    assert_eq!((b.y0, b.y1), (0, 8));
    assert_eq!(alloc.names[b.tex[2] as usize], "oak_planks");
    let b = registry::classify("stone_brick_stairs", &props, &have, &mut alloc);
    assert_eq!(alloc.names[b.tex[2] as usize], "stone_bricks");
    let b = registry::classify("white_carpet", &props, &have, &mut alloc);
    assert_eq!(b.y1, 1);
    assert_eq!(alloc.names[b.tex[0] as usize], "white_wool");
    props.insert("half".to_string(), "upper".to_string());
    let b = registry::classify("tall_grass", &props, &have, &mut alloc);
    assert_eq!(b.kind, Kind::Cross);
    assert_eq!(alloc.names[b.tex[0] as usize], "tall_grass_top");
    props.clear();
    props.insert("axis".to_string(), "y".to_string());
    let b = registry::classify("oak_log", &props, &have, &mut alloc);
    assert_eq!(alloc.names[b.tex[0] as usize], "oak_log_top");
    assert_eq!(alloc.names[b.tex[2] as usize], "oak_log");
    for air in ["air", "cave_air", "void_air"] {
        assert_eq!(registry::classify(air, &props, &have, &mut alloc).kind, Kind::Air);
    }
    assert_eq!(registry::classify("water", &props, &have, &mut alloc).kind, Kind::Water);
    assert_eq!(registry::classify("oak_leaves", &props, &have, &mut alloc).kind, Kind::Cutout);
}

#[test]
fn performance_generate_and_mesh() {
    let reg = sp_registry();
    let g = Generator::new(7, &reg);
    let t = Instant::now();
    let mut cols = HashMap::new();
    for cz in -4..4 {
        for cx in -4..4 {
            let mut c = g.generate(cx, cz);
            c.compute_skylight(&reg);
            cols.insert((cx, cz), c);
        }
    }
    let gen_ms = t.elapsed().as_secs_f64() * 1000.0 / 64.0;
    let t = Instant::now();
    let mut quads = 0;
    for cz in -3..3 {
        for cx in -3..3 {
            let n = [cols.get(&(cx - 1, cz)), cols.get(&(cx + 1, cz)), cols.get(&(cx, cz - 1)), cols.get(&(cx, cz + 1))];
            let m = mesh_column(&cols[&(cx, cz)], n, &reg);
            quads += (m.opaque.len() + m.water.len()) / 4;
        }
    }
    let mesh_ms = t.elapsed().as_secs_f64() * 1000.0 / 36.0;
    println!("PERF: generate+light {gen_ms:.2} ms/column, mesh {mesh_ms:.2} ms/column, {} quads/column avg", quads / 36);
    assert!(quads > 0);
    let _ = SP_SECTIONS;
}
