//! Decoding chunk and light data sent by the server (std only).
//!
//! The section layout has shifted between releases (a length prefix on packed
//! arrays was removed, a fluid count was added), so the decoder tries each known
//! layout and keeps the one that consumes the data exactly.

use super::buf::{DResult, DecodeError, Reader, Writer};
use super::nbt;
use crate::world::{Column, SECTION};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Layout {
    pub fluid_count: bool,
    pub length_prefix: bool,
}

pub const LAYOUTS: [Layout; 4] = [
    Layout { fluid_count: true, length_prefix: false },
    Layout { fluid_count: false, length_prefix: false },
    Layout { fluid_count: false, length_prefix: true },
    Layout { fluid_count: true, length_prefix: true },
];

fn read_longs(r: &mut Reader, entries: usize, bits: usize, layout: Layout) -> DResult<Vec<u64>> {
    let per_long = 64 / bits;
    let expected = entries.div_ceil(per_long);
    let n = if layout.length_prefix { r.len()? } else { expected };
    if n != expected {
        return Err(DecodeError(format!("packed array has {n} longs, expected {expected}")));
    }
    (0..n).map(|_| r.i64().map(|v| v as u64)).collect()
}

/// Reads a paletted container of `entries` values into `out` (None = skip values).
fn paletted(r: &mut Reader, entries: usize, max_indirect: u8, layout: Layout, out: Option<&mut [u16]>) -> DResult<()> {
    let bits = r.u8()?;
    if bits == 0 {
        let v = r.varint()?;
        if layout.length_prefix {
            let n = r.len()?;
            r.bytes(n * 8)?;
        }
        if let Some(out) = out {
            out.fill(v as u16);
        }
        return Ok(());
    }
    if bits > 32 {
        return Err(DecodeError(format!("bad bits per entry {bits}")));
    }
    let palette: Option<Vec<i32>> = if bits <= max_indirect {
        let n = r.len()?;
        if n > 4096 {
            return Err(DecodeError("palette too large".into()));
        }
        Some((0..n).map(|_| r.varint()).collect::<DResult<_>>()?)
    } else {
        None
    };
    let longs = read_longs(r, entries, bits as usize, layout)?;
    if let Some(out) = out {
        let b = bits as usize;
        let per_long = 64 / b;
        let mask = (1u64 << b) - 1;
        for (i, slot) in out.iter_mut().enumerate().take(entries) {
            let l = longs[i / per_long];
            let v = ((l >> ((i % per_long) * b)) & mask) as usize;
            *slot = match &palette {
                Some(p) => *p.get(v).unwrap_or(&0) as u16,
                None => v as u16,
            };
        }
    }
    Ok(())
}

fn sections(data: &[u8], count: usize, layout: Layout) -> DResult<Vec<Option<Box<[u16; SECTION]>>>> {
    let mut r = Reader::new(data);
    let mut out = Vec::with_capacity(count);
    for _ in 0..count {
        let non_air = r.i16()?;
        if layout.fluid_count {
            r.i16()?;
        }
        let mut blocks = Box::new([0u16; SECTION]);
        paletted(&mut r, SECTION, 8, layout, Some(&mut blocks[..]))?;
        paletted(&mut r, 64, 3, layout, None)?;
        let empty = non_air == 0 && blocks.iter().all(|&b| b == 0);
        out.push(if empty { None } else { Some(blocks) });
    }
    if r.remaining() != 0 {
        return Err(DecodeError(format!("{} bytes left after sections", r.remaining())));
    }
    Ok(out)
}

/// Decodes the section blob of a chunk, trying `preferred` first.
pub fn decode_sections(data: &[u8], count: usize, preferred: &mut Layout) -> DResult<Vec<Option<Box<[u16; SECTION]>>>> {
    if let Ok(s) = sections(data, count, *preferred) {
        return Ok(s);
    }
    for l in LAYOUTS {
        if l == *preferred {
            continue;
        }
        if let Ok(s) = sections(data, count, l) {
            *preferred = l;
            return Ok(s);
        }
    }
    Err(DecodeError("chunk sections match no known layout".into()))
}

pub struct LightData {
    pub sky: Vec<(usize, Box<[u8; 2048]>)>,
    pub block: Vec<(usize, Box<[u8; 2048]>)>,
    pub empty_sky: Vec<usize>,
    pub empty_block: Vec<usize>,
}

fn bitset(r: &mut Reader) -> DResult<Vec<usize>> {
    let longs = r.long_array()?;
    let mut v = vec![];
    for (i, l) in longs.iter().enumerate() {
        for b in 0..64 {
            if l >> b & 1 == 1 {
                v.push(i * 64 + b);
            }
        }
    }
    Ok(v)
}

pub fn read_light(r: &mut Reader) -> DResult<LightData> {
    let sky_mask = bitset(r)?;
    let block_mask = bitset(r)?;
    let empty_sky = bitset(r)?;
    let empty_block = bitset(r)?;
    let mut arrays = |mask: &[usize]| -> DResult<Vec<(usize, Box<[u8; 2048]>)>> {
        let n = r.len()?;
        let mut out = vec![];
        for i in 0..n {
            let len = r.len()?;
            let b = r.bytes(len)?;
            if len == 2048 {
                if let Some(&sec) = mask.get(i) {
                    out.push((sec, Box::new(b.try_into().unwrap())));
                }
            }
        }
        Ok(out)
    };
    let sky = arrays(&sky_mask)?;
    let block = arrays(&block_mask)?;
    Ok(LightData { sky, block, empty_sky, empty_block })
}

/// Writes light into a column. Light section `i` covers block section `i - 1`.
pub fn apply_light(col: &mut Column, light: &LightData) {
    let n = col.sections.len();
    let mut set = |sec: usize, data: Option<&[u8; 2048]>, sky: bool| {
        if sec == 0 || sec > n {
            return;
        }
        let si = sec - 1;
        let l = col.light[si].get_or_insert_with(|| Box::new([0xF0; SECTION]));
        for i in 0..SECTION {
            let v = match data {
                Some(d) => (d[i >> 1] >> ((i & 1) * 4)) & 15,
                None => 0,
            };
            l[i] = if sky { (l[i] & 0x0F) | (v << 4) } else { (l[i] & 0xF0) | v };
        }
    };
    for (s, d) in &light.sky {
        set(*s, Some(d), true);
    }
    for (s, d) in &light.block {
        set(*s, Some(d), false);
    }
    for s in &light.empty_sky {
        set(*s, None, true);
    }
    for s in &light.empty_block {
        set(*s, None, false);
    }
}

// ---------------------------------------------------------------------------
// Encoding (used by Claudium's own server). Produces the 26.x layout the decoder
// above reads as LAYOUTS[0]: a fluid count per section and no length prefix.

/// Encodes one paletted container: single-valued when uniform, otherwise direct.
fn encode_container(w: &mut Writer, values: &[u16], direct_bits: u8) {
    let first = values[0];
    if values.iter().all(|&v| v == first) {
        w.u8(0);
        w.varint(first as i32);
        return;
    }
    let bits = direct_bits;
    w.u8(bits);
    let per = 64 / bits as usize;
    let mut longs = vec![0u64; values.len().div_ceil(per)];
    for (i, &v) in values.iter().enumerate() {
        longs[i / per] |= (v as u64) << ((i % per) * bits as usize);
    }
    for l in longs {
        w.i64(l as i64);
    }
}

fn encode_section(w: &mut Writer, blocks: &[u16; SECTION]) {
    let non_air = blocks.iter().filter(|&&v| v != 0).count() as i16;
    w.i16(non_air);
    w.i16(0); // fluid count
    encode_container(w, &blocks[..], 15); // block states, direct = 15 bits
    encode_container(w, &[0u16; 64], 4); // biomes: single-valued (plains)
}

/// Body of a "Level Chunk With Light" packet for a column (without the packet id,
/// which the caller prepends). Kept id-free so this module has no protocol-table deps.
pub fn encode_chunk_body(cx: i32, cz: i32, col: &Column) -> Vec<u8> {
    let mut w = Writer::default();
    w.i32(cx).i32(cz);
    w.varint(0); // heightmaps: empty map

    let empty = Box::new([0u16; SECTION]);
    let mut blob = Writer::default();
    for sec in &col.sections {
        let blocks = sec.as_deref().unwrap_or(&empty);
        encode_section(&mut blob, blocks);
    }
    w.byte_array(&blob.buf);

    w.varint(0); // block entities

    // Light: send full-bright by marking every section empty-sky (no block light).
    // Four bitsets then two array lists; all empty means the client keeps full sky light.
    for _ in 0..6 {
        w.varint(0);
    }
    w.done()
}

/// Parses "Level Chunk With Light". Returns chunk coordinates and the column.
pub fn read_chunk(r: &mut Reader, section_count: usize, layout: &mut Layout) -> DResult<(i32, i32, Column)> {
    let cx = r.i32()?;
    let cz = r.i32()?;
    // Heightmaps: we compute our own, so skip them. Newer servers send a map of
    // (type, long array); older ones sent an NBT compound.
    let save = r.pos;
    let parsed_map = (|| -> DResult<()> {
        let n = r.len()?;
        if n > 16 {
            return Err(DecodeError("too many heightmaps".into()));
        }
        for _ in 0..n {
            r.varint()?;
            let longs = r.len()?;
            r.bytes(longs * 8)?;
        }
        Ok(())
    })();
    if parsed_map.is_err() {
        r.pos = save;
        nbt::read(r)?;
    }
    let data = r.byte_array()?;
    let secs = decode_sections(data, section_count, layout)?;
    let mut col = Column::new(section_count);
    col.sections = secs;
    // Block entities
    let n = r.len()?;
    for _ in 0..n {
        r.u8()?;
        r.i16()?;
        r.varint()?;
        nbt::read(r)?;
    }
    let light = read_light(r)?;
    apply_light(&mut col, &light);
    Ok((cx, cz, col))
}

// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::buf::Writer;

    fn encode_container(w: &mut Writer, values: &[u16], bits: u8, palette: Option<&[u16]>, layout: Layout) {
        w.u8(bits);
        if bits == 0 {
            w.varint(values[0] as i32);
            if layout.length_prefix {
                w.varint(0);
            }
            return;
        }
        let idx: Vec<u64> = match palette {
            Some(p) => {
                w.varint(p.len() as i32);
                for v in p {
                    w.varint(*v as i32);
                }
                values.iter().map(|v| p.iter().position(|x| x == v).unwrap() as u64).collect()
            }
            None => values.iter().map(|&v| v as u64).collect(),
        };
        let per = 64 / bits as usize;
        let longs: Vec<u64> =
            idx.chunks(per).map(|c| c.iter().enumerate().fold(0u64, |acc, (i, v)| acc | (v << (i * bits as usize)))).collect();
        if layout.length_prefix {
            w.varint(longs.len() as i32);
        }
        for l in longs {
            w.i64(l as i64);
        }
    }

    fn sample_blob(layout: Layout) -> (Vec<u8>, Vec<u16>) {
        let mut w = Writer::default();
        // Section 0: mix of 3 states via a 4-bit palette
        let vals: Vec<u16> = (0..4096).map(|i| [1u16, 9, 1, 2000][(i * 7 / 3) % 4]).collect();
        w.i16(4096);
        if layout.fluid_count {
            w.i16(0);
        }
        encode_container(&mut w, &vals, 4, Some(&[1, 9, 2000]), layout);
        encode_container(&mut w, &[0; 64], 0, None, layout);
        // Section 1: all air
        w.i16(0);
        if layout.fluid_count {
            w.i16(0);
        }
        encode_container(&mut w, &[0; 4096], 0, None, layout);
        encode_container(&mut w, &[0; 64], 0, None, layout);
        // Section 2: direct 15-bit values
        let direct: Vec<u16> = (0..4096).map(|i| (i * 13 % 30000) as u16).collect();
        w.i16(4000);
        if layout.fluid_count {
            w.i16(3);
        }
        encode_container(&mut w, &direct, 15, None, layout);
        encode_container(&mut w, &[5; 64], 0, None, layout);
        let mut expected = vals;
        expected.extend(direct);
        (w.done(), expected)
    }

    #[test]
    fn decodes_every_layout() {
        for l in LAYOUTS {
            let (blob, expected) = sample_blob(l);
            let mut pref = LAYOUTS[0];
            let secs = decode_sections(&blob, 3, &mut pref).expect("decode");
            assert_eq!(pref, l, "picked the wrong layout");
            assert!(secs[1].is_none());
            let s0 = secs[0].as_ref().unwrap();
            let s2 = secs[2].as_ref().unwrap();
            assert_eq!(&s0[..], &expected[..4096]);
            assert_eq!(&s2[..], &expected[4096..]);
        }
    }

    #[test]
    fn encode_decode_round_trip() {
        // A column with a uniform section, an empty section and a varied section.
        let mut col = Column::new(3);
        for i in 0..SECTION {
            col.set(i & 15, i >> 8, (i >> 4) & 15, 7); // section 0: all id 7
        }
        // section 1 left empty (None)
        col.set(3, 32, 4, 42); // section 2: a couple of blocks
        col.set(9, 40, 1, 999);
        let body = encode_chunk_body(5, -3, &col);
        let mut layout = LAYOUTS[0];
        let (cx, cz, out) = read_chunk(&mut Reader::new(&body), 3, &mut layout).expect("decode");
        assert_eq!((cx, cz), (5, -3));
        for y in 0..48 {
            for z in 0..16 {
                for x in 0..16 {
                    assert_eq!(out.get(x, y, z), col.get(x, y, z), "mismatch at {x},{y},{z}");
                }
            }
        }
        assert!(out.sections[1].is_none(), "empty section should stay empty");
    }

    #[test]
    fn light_nibbles() {
        let mut col = Column::new(2);
        let mut d = Box::new([0u8; 2048]);
        d[0] = 0xA5; // block 0 -> 5, block 1 -> 10
        let ld = LightData { sky: vec![(1, d)], block: vec![], empty_sky: vec![], empty_block: vec![2] };
        apply_light(&mut col, &ld);
        assert_eq!(col.light(0, 0, 0) >> 4, 5);
        assert_eq!(col.light(1, 0, 0) >> 4, 10);
        assert_eq!(col.light(0, 16, 0) & 15, 0);
    }
}
