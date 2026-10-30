//! Singleplayer world saving: one compressed file per edited column.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use flate2::Compression;
use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;

use crate::registry::Registry;
use crate::world::{Column, SECTION};

const MAGIC: &[u8; 4] = b"CLDM";

pub struct WorldSave {
    pub dir: PathBuf,
}

impl WorldSave {
    pub fn open(dir: PathBuf) -> std::io::Result<Self> {
        std::fs::create_dir_all(&dir)?;
        Ok(WorldSave { dir })
    }

    fn path(&self, cx: i32, cz: i32) -> PathBuf {
        self.dir.join(format!("c.{cx}.{cz}.bin"))
    }

    pub fn save_column(&self, cx: i32, cz: i32, col: &Column, reg: &Registry) -> std::io::Result<()> {
        let mut raw = Vec::with_capacity(64 * 1024);
        raw.extend_from_slice(MAGIC);
        raw.push(1);
        // Name table so saves survive changes to the block list.
        raw.extend_from_slice(&(reg.blocks.len() as u16).to_le_bytes());
        for b in &reg.blocks {
            raw.push(b.name.len() as u8);
            raw.extend_from_slice(b.name.as_bytes());
        }
        raw.push(col.sections.len() as u8);
        for s in &col.sections {
            match s {
                None => raw.push(0),
                Some(s) => {
                    raw.push(1);
                    for v in s.iter() {
                        raw.extend_from_slice(&v.to_le_bytes());
                    }
                }
            }
        }
        let mut z = ZlibEncoder::new(Vec::new(), Compression::fast());
        z.write_all(&raw)?;
        let data = z.finish()?;
        let tmp = self.path(cx, cz).with_extension("tmp");
        std::fs::write(&tmp, data)?;
        std::fs::rename(tmp, self.path(cx, cz))
    }

    pub fn load_column(&self, cx: i32, cz: i32, reg: &Registry) -> Option<Column> {
        load_file(&self.path(cx, cz), reg)
    }

    pub fn save_player(&self, pos: [f64; 3], yaw: f32, pitch: f32, flying: bool) {
        let _ = std::fs::write(self.dir.join("player.txt"), format!("{} {} {} {} {} {}", pos[0], pos[1], pos[2], yaw, pitch, flying));
    }

    pub fn load_player(&self) -> Option<([f64; 3], f32, f32, bool)> {
        let s = std::fs::read_to_string(self.dir.join("player.txt")).ok()?;
        let p: Vec<&str> = s.split_whitespace().collect();
        Some((
            [p.first()?.parse().ok()?, p.get(1)?.parse().ok()?, p.get(2)?.parse().ok()?],
            p.get(3)?.parse().ok()?,
            p.get(4)?.parse().ok()?,
            p.get(5)?.parse().ok()?,
        ))
    }
}

fn load_file(path: &Path, reg: &Registry) -> Option<Column> {
    let data = std::fs::read(path).ok()?;
    let mut raw = Vec::new();
    ZlibDecoder::new(&data[..]).read_to_end(&mut raw).ok()?;
    let mut p = 0usize;
    let mut take = |n: usize| -> Option<&[u8]> {
        let s = raw.get(p..p + n)?;
        p += n;
        Some(s)
    };
    if take(4)? != MAGIC || take(1)?[0] != 1 {
        return None;
    }
    let names = u16::from_le_bytes(take(2)?.try_into().ok()?) as usize;
    let mut remap = Vec::with_capacity(names);
    for _ in 0..names {
        let n = take(1)?[0] as usize;
        let name = String::from_utf8_lossy(take(n)?).into_owned();
        remap.push(reg.id_of(&name).unwrap_or(0));
    }
    let count = take(1)?[0] as usize;
    let mut col = Column::new(count);
    for si in 0..count {
        if take(1)?[0] == 1 {
            let bytes = take(SECTION * 2)?;
            let mut s = Box::new([0u16; SECTION]);
            for i in 0..SECTION {
                let v = u16::from_le_bytes([bytes[i * 2], bytes[i * 2 + 1]]) as usize;
                s[i] = *remap.get(v).unwrap_or(&0);
            }
            col.sections[si] = Some(s);
        }
    }
    Some(col)
}
