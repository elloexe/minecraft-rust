//! Minimal network-NBT reader: enough to skip tags and turn text components into plain text.

use super::buf::{DResult, DecodeError, Reader};

#[derive(Debug, Clone)]
pub enum Tag {
    End,
    Byte(i8),
    Short(i16),
    Int(i32),
    Long(i64),
    Float(f32),
    Double(f64),
    ByteArray(usize),
    String(String),
    List(Vec<Tag>),
    Compound(Vec<(String, Tag)>),
    IntArray(usize),
    LongArray(usize),
}

impl Tag {
    pub fn get(&self, key: &str) -> Option<&Tag> {
        match self {
            Tag::Compound(v) => v.iter().find(|(k, _)| k == key).map(|(_, t)| t),
            _ => None,
        }
    }
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Tag::String(s) => Some(s),
            _ => None,
        }
    }
}

fn nbt_string(r: &mut Reader) -> DResult<String> {
    let n = r.u16()? as usize;
    let b = r.bytes(n)?;
    // Modified UTF-8; plain text decodes fine for display purposes.
    Ok(String::from_utf8_lossy(b).into_owned())
}

fn payload(r: &mut Reader, ty: u8, depth: u32) -> DResult<Tag> {
    if depth > 512 {
        return Err(DecodeError("nbt too deep".into()));
    }
    Ok(match ty {
        0 => Tag::End,
        1 => Tag::Byte(r.i8()?),
        2 => Tag::Short(r.i16()?),
        3 => Tag::Int(r.i32()?),
        4 => Tag::Long(r.i64()?),
        5 => Tag::Float(r.f32()?),
        6 => Tag::Double(r.f64()?),
        7 => {
            let n = r.i32()?.max(0) as usize;
            r.bytes(n)?;
            Tag::ByteArray(n)
        }
        8 => Tag::String(nbt_string(r)?),
        9 => {
            let et = r.u8()?;
            let n = r.i32()?.max(0) as usize;
            let mut v = Vec::with_capacity(n.min(1024));
            for _ in 0..n {
                v.push(payload(r, et, depth + 1)?);
            }
            Tag::List(v)
        }
        10 => {
            let mut v = Vec::new();
            loop {
                let t = r.u8()?;
                if t == 0 {
                    break;
                }
                let name = nbt_string(r)?;
                v.push((name, payload(r, t, depth + 1)?));
            }
            Tag::Compound(v)
        }
        11 => {
            let n = r.i32()?.max(0) as usize;
            r.bytes(n * 4)?;
            Tag::IntArray(n)
        }
        12 => {
            let n = r.i32()?.max(0) as usize;
            r.bytes(n * 8)?;
            Tag::LongArray(n)
        }
        _ => return Err(DecodeError(format!("bad nbt tag {ty}"))),
    })
}

/// Reads a nameless root tag as sent over the network.
pub fn read(r: &mut Reader) -> DResult<Tag> {
    let ty = r.u8()?;
    payload(r, ty, 0)
}

fn translate(key: &str, args: &[String]) -> String {
    let a = |i: usize| args.get(i).cloned().unwrap_or_default();
    match key {
        "chat.type.text" => format!("<{}> {}", a(0), a(1)),
        "chat.type.announcement" => format!("[{}] {}", a(0), a(1)),
        "chat.type.emote" => format!("* {} {}", a(0), a(1)),
        "multiplayer.player.joined" => format!("{} joined the game", a(0)),
        "multiplayer.player.left" => format!("{} left the game", a(0)),
        "multiplayer.disconnect.kicked" => "Kicked by an operator".into(),
        "multiplayer.disconnect.server_shutdown" => "Server closed".into(),
        _ if args.is_empty() => key.to_string(),
        _ => format!("{key} [{}]", args.join(", ")),
    }
}

/// Flattens a text component into plain text.
pub fn text(tag: &Tag) -> String {
    match tag {
        Tag::String(s) => s.clone(),
        Tag::List(items) => items.iter().map(text).collect(),
        Tag::Compound(_) => {
            let mut out = String::new();
            if let Some(t) = tag.get("text").and_then(|t| t.as_str()) {
                out.push_str(t);
            } else if let Some(t) = tag.get("").and_then(|t| t.as_str()) {
                out.push_str(t);
            } else if let Some(k) = tag.get("translate").and_then(|t| t.as_str()) {
                let args: Vec<String> = match tag.get("with") {
                    Some(Tag::List(w)) => w.iter().map(text).collect(),
                    _ => vec![],
                };
                let fallback = tag.get("fallback").and_then(|t| t.as_str());
                let tr = translate(k, &args);
                out.push_str(if tr == k { fallback.unwrap_or(&tr) } else { &tr });
            }
            if let Some(Tag::List(extra)) = tag.get("extra") {
                for e in extra {
                    out.push_str(&text(e));
                }
            }
            out
        }
        Tag::Int(i) => i.to_string(),
        _ => String::new(),
    }
}

/// Reads a text component encoded as network NBT and returns plain text.
pub fn read_text(r: &mut Reader) -> DResult<String> {
    Ok(text(&read(r)?))
}

/// Plain text from a JSON text component (used during login).
pub fn json_text(s: &str) -> String {
    s.to_string()
}
