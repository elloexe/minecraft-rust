//! Java Edition network client, written from the community protocol documentation.
//! Packet ids target protocol 26.1.2.

pub mod auth;
pub mod buf;
pub mod chunk;
pub mod crypto;
pub mod nbt;

use std::io::{BufReader, Read, Write};
use std::net::TcpStream;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::Duration;

use anyhow::{Context, anyhow, bail};
use flate2::Compression;
use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;

use buf::{Reader, Writer};
use crypto::Cfb8;

use crate::world::Column;

pub const PROTOCOL: i32 = 775;
pub const GAME_VERSION: &str = "26.1.2";

/// Packet ids for protocol 26.1.2.
pub mod id {
    pub mod login {
        pub const CB_DISCONNECT: i32 = 0x00;
        pub const CB_ENCRYPTION: i32 = 0x01;
        pub const CB_SUCCESS: i32 = 0x02;
        pub const CB_COMPRESSION: i32 = 0x03;
        pub const CB_PLUGIN_REQUEST: i32 = 0x04;
        pub const CB_COOKIE_REQUEST: i32 = 0x05;
        pub const SB_HELLO: i32 = 0x00;
        pub const SB_KEY: i32 = 0x01;
        pub const SB_PLUGIN_RESPONSE: i32 = 0x02;
        pub const SB_ACK: i32 = 0x03;
        pub const SB_COOKIE_RESPONSE: i32 = 0x04;
    }
    pub mod config {
        pub const CB_COOKIE_REQUEST: i32 = 0x00;
        pub const CB_DISCONNECT: i32 = 0x02;
        pub const CB_FINISH: i32 = 0x03;
        pub const CB_KEEP_ALIVE: i32 = 0x04;
        pub const CB_PING: i32 = 0x05;
        pub const CB_REGISTRY_DATA: i32 = 0x07;
        pub const CB_RESOURCE_PACK_PUSH: i32 = 0x09;
        pub const CB_TRANSFER: i32 = 0x0B;
        pub const CB_KNOWN_PACKS: i32 = 0x0E;
        pub const CB_CODE_OF_CONDUCT: i32 = 0x13;
        pub const SB_CLIENT_INFO: i32 = 0x00;
        pub const SB_COOKIE_RESPONSE: i32 = 0x01;
        pub const SB_FINISH: i32 = 0x03;
        pub const SB_KEEP_ALIVE: i32 = 0x04;
        pub const SB_PONG: i32 = 0x05;
        pub const SB_RESOURCE_PACK: i32 = 0x06;
        pub const SB_KNOWN_PACKS: i32 = 0x07;
        pub const SB_ACCEPT_CONDUCT: i32 = 0x09;
    }
    pub mod play {
        pub const CB_BLOCK_UPDATE: i32 = 0x08;
        pub const CB_CHUNK_BATCH_FINISHED: i32 = 0x0B;
        pub const CB_COOKIE_REQUEST: i32 = 0x15;
        pub const CB_DISCONNECT: i32 = 0x20;
        pub const CB_DISGUISED_CHAT: i32 = 0x21;
        pub const CB_FORGET_CHUNK: i32 = 0x25;
        pub const CB_GAME_EVENT: i32 = 0x26;
        pub const CB_KEEP_ALIVE: i32 = 0x2C;
        pub const CB_CHUNK: i32 = 0x2D;
        pub const CB_LIGHT: i32 = 0x30;
        pub const CB_LOGIN: i32 = 0x31;
        pub const CB_PING: i32 = 0x3D;
        pub const CB_ABILITIES: i32 = 0x40;
        pub const CB_PLAYER_CHAT: i32 = 0x41;
        pub const CB_COMBAT_KILL: i32 = 0x44;
        pub const CB_PLAYER_POSITION: i32 = 0x48;
        pub const CB_RESOURCE_PACK_PUSH: i32 = 0x51;
        pub const CB_RESPAWN: i32 = 0x52;
        pub const CB_SECTION_BLOCKS: i32 = 0x54;
        pub const CB_SET_HEALTH: i32 = 0x68;
        pub const CB_SET_HELD_SLOT: i32 = 0x69;
        pub const CB_START_CONFIG: i32 = 0x76;
        pub const CB_SYSTEM_CHAT: i32 = 0x79;

        pub const SB_CONFIRM_TELEPORT: i32 = 0x00;
        pub const SB_CHAT_COMMAND: i32 = 0x07;
        pub const SB_CHAT: i32 = 0x09;
        pub const SB_CHUNK_BATCH_RECEIVED: i32 = 0x0B;
        pub const SB_CLIENT_STATUS: i32 = 0x0C;
        pub const SB_TICK_END: i32 = 0x0D;
        pub const SB_ACK_CONFIG: i32 = 0x10;
        pub const SB_COOKIE_RESPONSE: i32 = 0x15;
        pub const SB_KEEP_ALIVE: i32 = 0x1C;
        pub const SB_POS: i32 = 0x1E;
        pub const SB_POS_ROT: i32 = 0x1F;
        pub const SB_ABILITIES: i32 = 0x28;
        pub const SB_PLAYER_ACTION: i32 = 0x29;
        pub const SB_PLAYER_LOADED: i32 = 0x2C;
        pub const SB_PONG: i32 = 0x2D;
        pub const SB_RESOURCE_PACK: i32 = 0x31;
        pub const SB_SET_HELD: i32 = 0x35;
        pub const SB_SWING: i32 = 0x3F;
        pub const SB_USE_ITEM_ON: i32 = 0x42;
    }
}

// ---------------------------------------------------------------------------
// Framing

const MAX_PACKET: usize = 8 * 1024 * 1024;

struct PacketReader {
    r: BufReader<TcpStream>,
    dec: Option<Cfb8>,
    threshold: i32,
}

impl PacketReader {
    fn byte(&mut self) -> anyhow::Result<u8> {
        let mut b = [0u8; 1];
        self.r.read_exact(&mut b)?;
        if let Some(d) = &mut self.dec {
            d.decrypt(&mut b);
        }
        Ok(b[0])
    }

    fn read(&mut self) -> anyhow::Result<Vec<u8>> {
        let mut len: u32 = 0;
        for i in 0..5 {
            let b = self.byte()?;
            len |= ((b & 0x7f) as u32) << (7 * i);
            if b & 0x80 == 0 {
                break;
            }
            if i == 4 {
                bail!("packet length varint too long");
            }
        }
        let len = len as usize;
        if len > MAX_PACKET {
            bail!("packet too large ({len} bytes)");
        }
        let mut body = vec![0u8; len];
        self.r.read_exact(&mut body)?;
        if let Some(d) = &mut self.dec {
            d.decrypt(&mut body);
        }
        if self.threshold >= 0 {
            let mut rd = Reader::new(&body);
            let data_len = rd.len()?;
            let rest = rd.rest();
            if data_len == 0 {
                return Ok(rest.to_vec());
            }
            if data_len > MAX_PACKET {
                bail!("decompressed packet too large");
            }
            let mut out = Vec::with_capacity(data_len);
            ZlibDecoder::new(rest).read_to_end(&mut out)?;
            return Ok(out);
        }
        Ok(body)
    }
}

pub enum WriterCmd {
    Packet(Vec<u8>),
    Encrypt([u8; 16]),
    Threshold(i32),
}

fn frame(packet: &[u8], threshold: i32) -> Vec<u8> {
    let mut body = Vec::with_capacity(packet.len() + 8);
    if threshold >= 0 {
        if packet.len() >= threshold as usize {
            buf::write_varint(&mut body, packet.len() as i32);
            let mut z = ZlibEncoder::new(body, Compression::fast());
            z.write_all(packet).expect("zlib");
            body = z.finish().expect("zlib");
        } else {
            body.push(0);
            body.extend_from_slice(packet);
        }
    } else {
        body.extend_from_slice(packet);
    }
    let mut out = Vec::with_capacity(body.len() + 5);
    buf::write_varint(&mut out, body.len() as i32);
    out.extend_from_slice(&body);
    out
}

fn spawn_writer(mut stream: TcpStream) -> Sender<WriterCmd> {
    let (tx, rx) = channel::<WriterCmd>();
    std::thread::Builder::new()
        .name("claudium-net-write".into())
        .spawn(move || {
            let mut enc: Option<Cfb8> = None;
            let mut threshold = -1;
            while let Ok(cmd) = rx.recv() {
                match cmd {
                    WriterCmd::Encrypt(k) => enc = Some(Cfb8::new(&k)),
                    WriterCmd::Threshold(t) => threshold = t,
                    WriterCmd::Packet(p) => {
                        let mut data = frame(&p, threshold);
                        if let Some(e) = &mut enc {
                            e.encrypt(&mut data);
                        }
                        if stream.write_all(&data).is_err() {
                            return;
                        }
                    }
                }
            }
            let _ = stream.shutdown(std::net::Shutdown::Both);
        })
        .expect("spawn writer");
    tx
}

// ---------------------------------------------------------------------------
// Server list ping

pub struct ServerStatus {
    pub protocol: i32,
    pub version: String,
    pub motd: String,
    pub online: i64,
    pub max: i64,
    pub latency_ms: u128,
}

pub fn parse_address(addr: &str) -> (String, u16) {
    if let Some(rest) = addr.strip_prefix('[') {
        // [ipv6]:port
        if let Some((h, p)) = rest.split_once(']') {
            let port = p.strip_prefix(':').and_then(|p| p.parse().ok()).unwrap_or(25565);
            return (h.to_string(), port);
        }
    }
    match addr.rsplit_once(':') {
        Some((h, p)) if !h.contains(':') => match p.parse::<u16>() {
            Ok(port) => (h.to_string(), port),
            Err(_) => (addr.to_string(), 25565),
        },
        _ => (addr.to_string(), 25565),
    }
}

fn open(host: &str, port: u16) -> anyhow::Result<TcpStream> {
    use std::net::ToSocketAddrs;
    let addr = (host, port).to_socket_addrs()?.next().ok_or_else(|| anyhow!("could not resolve {host}"))?;
    let s = TcpStream::connect_timeout(&addr, Duration::from_secs(10)).with_context(|| format!("connecting to {host}:{port}"))?;
    s.set_nodelay(true)?;
    Ok(s)
}

fn handshake(host: &str, port: u16, protocol: i32, next: i32) -> Vec<u8> {
    Writer::packet(0x00).varint(protocol).string(host).u16(port).varint(next).done()
}

pub fn ping(addr: &str) -> anyhow::Result<ServerStatus> {
    let (host, port) = parse_address(addr);
    let stream = open(&host, port)?;
    stream.set_read_timeout(Some(Duration::from_secs(10)))?;
    let mut w = stream.try_clone()?;
    let mut r = PacketReader { r: BufReader::new(stream), dec: None, threshold: -1 };
    w.write_all(&frame(&handshake(&host, port, PROTOCOL, 1), -1))?;
    w.write_all(&frame(&Writer::packet(0x00).done(), -1))?;
    let p = r.read()?;
    let mut rd = Reader::new(&p);
    if rd.varint()? != 0x00 {
        bail!("unexpected status response");
    }
    let json: serde_json::Value = serde_json::from_str(&rd.string()?)?;
    let start = std::time::Instant::now();
    w.write_all(&frame(&Writer::packet(0x01).i64(42).done(), -1))?;
    let _ = r.read();
    let latency_ms = start.elapsed().as_millis();
    let motd = match &json["description"] {
        serde_json::Value::String(s) => s.clone(),
        v => {
            let mut s = v["text"].as_str().unwrap_or("").to_string();
            if let Some(extra) = v["extra"].as_array() {
                for e in extra {
                    s.push_str(e["text"].as_str().or(e.as_str()).unwrap_or(""));
                }
            }
            s
        }
    };
    Ok(ServerStatus {
        protocol: json["version"]["protocol"].as_i64().unwrap_or(-1) as i32,
        version: json["version"]["name"].as_str().unwrap_or("?").to_string(),
        motd,
        online: json["players"]["online"].as_i64().unwrap_or(0),
        max: json["players"]["max"].as_i64().unwrap_or(0),
        latency_ms,
    })
}

// ---------------------------------------------------------------------------
// Session

pub enum NetEvent {
    Info(String),
    Chat(String),
    Joined { dimension: String, min_y: i32, sections: usize, gamemode: u8 },
    Respawned { dimension: String, min_y: i32, sections: usize, gamemode: u8 },
    Chunk { x: i32, z: i32, column: Box<Column> },
    Unload { x: i32, z: i32 },
    Block { x: i32, y: i32, z: i32, state: u16 },
    Light { x: i32, z: i32, light: Box<chunk::LightData> },
    Teleport { x: f64, y: f64, z: f64, yaw: f32, pitch: f32, flags: i32 },
    GameMode(u8),
    Abilities { flying: bool, allow_flying: bool, instabuild: bool },
    Health(f32),
    HeldSlot(u8),
    Disconnected(String),
}

pub struct NetHandle {
    pub events: Receiver<NetEvent>,
    out: Sender<WriterCmd>,
    pub in_play: Arc<AtomicBool>,
    pub sequence: i32,
}

impl NetHandle {
    pub fn send(&self, packet: Vec<u8>) {
        let _ = self.out.send(WriterCmd::Packet(packet));
    }
    pub fn playing(&self) -> bool {
        self.in_play.load(Ordering::Relaxed)
    }
    pub fn next_sequence(&mut self) -> i32 {
        self.sequence += 1;
        self.sequence
    }
}

pub struct ConnectOptions {
    pub address: String,
    pub username: String,
    pub profile: Option<auth::Profile>,
    pub protocol: i32,
    pub view_distance: u8,
    pub accept_code_of_conduct: bool,
}

#[derive(Clone, Copy)]
struct DimInfo {
    min_y: i32,
    height: i32,
}

struct Session {
    out: Sender<WriterCmd>,
    events: Sender<NetEvent>,
    in_play: Arc<AtomicBool>,
    dim_types: Vec<Option<DimInfo>>,
    sections: usize,
    layout: chunk::Layout,
    opts_view: u8,
    accept_conduct: bool,
}

impl Session {
    fn send(&self, p: Vec<u8>) {
        let _ = self.out.send(WriterCmd::Packet(p));
    }
    fn event(&self, e: NetEvent) {
        let _ = self.events.send(e);
    }

    fn client_info(&self, id: i32) -> Vec<u8> {
        Writer::packet(id)
            .string("en_us")
            .u8(self.opts_view)
            .varint(0) // chat enabled
            .bool(true)
            .u8(0x7f) // all skin layers
            .varint(1) // right handed
            .bool(false)
            .bool(true)
            .varint(0) // all particles
            .done()
    }

    /// Handles one configuration-state packet. Returns true when configuration is finished.
    fn config_packet(&mut self, pid: i32, r: &mut Reader) -> anyhow::Result<bool> {
        use id::config::*;
        match pid {
            CB_FINISH => {
                self.send(Writer::packet(SB_FINISH).done());
                self.in_play.store(true, Ordering::Relaxed);
                return Ok(true);
            }
            CB_KEEP_ALIVE => {
                let v = r.i64()?;
                self.send(Writer::packet(SB_KEEP_ALIVE).i64(v).done());
            }
            CB_PING => {
                let v = r.i32()?;
                self.send(Writer::packet(SB_PONG).i32(v).done());
            }
            CB_KNOWN_PACKS => {
                // Echo the packs back: we read the same vanilla data the server has.
                let n = r.len()?;
                let mut w = Writer::packet(SB_KNOWN_PACKS);
                w.varint(n as i32);
                for _ in 0..n {
                    let (ns, pid, ver) = (r.string()?, r.string()?, r.string()?);
                    w.string(&ns).string(&pid).string(&ver);
                }
                self.send(w.done());
            }
            CB_REGISTRY_DATA => {
                let reg = r.string()?;
                let n = r.len()?;
                let dims = reg == "minecraft:dimension_type";
                if dims {
                    self.dim_types.clear();
                }
                for _ in 0..n {
                    let _entry = r.string()?;
                    let data = if r.bool()? { Some(nbt::read(r)?) } else { None };
                    if dims {
                        let info = data.as_ref().and_then(|d| {
                            let num = |k: &str| match d.get(k) {
                                Some(nbt::Tag::Int(v)) => Some(*v),
                                _ => None,
                            };
                            Some(DimInfo { min_y: num("min_y")?, height: num("height")? })
                        });
                        self.dim_types.push(info);
                    }
                }
            }
            CB_COOKIE_REQUEST => {
                let key = r.string()?;
                self.send(Writer::packet(SB_COOKIE_RESPONSE).string(&key).bool(false).done());
            }
            CB_RESOURCE_PACK_PUSH => {
                let uuid = r.uuid()?;
                // We don't apply server resource packs yet; report it as loaded so the server continues.
                self.send(Writer::packet(SB_RESOURCE_PACK).uuid(uuid).varint(3).done());
                self.send(Writer::packet(SB_RESOURCE_PACK).uuid(uuid).varint(0).done());
                self.event(NetEvent::Info("Server sent a resource pack (not applied yet)".into()));
            }
            CB_CODE_OF_CONDUCT => {
                let text = r.string()?;
                self.event(NetEvent::Chat(format!("[Server code of conduct]\n{text}")));
                if self.accept_conduct {
                    self.send(Writer::packet(SB_ACCEPT_CONDUCT).done());
                } else {
                    bail!(
                        "this server requires accepting its code of conduct (printed above); reconnect with --accept-conduct to accept it"
                    );
                }
            }
            CB_TRANSFER => {
                let host = r.string()?;
                let port = r.varint()?;
                bail!("server asked to transfer to {host}:{port} (not supported yet)");
            }
            CB_DISCONNECT => {
                let reason = nbt::read_text(r)?;
                bail!("{reason}");
            }
            _ => {}
        }
        Ok(false)
    }

    fn spawn_info(&mut self, r: &mut Reader) -> anyhow::Result<(String, i32, usize, u8)> {
        let dim_type = r.varint()?;
        let dim = r.string()?;
        let _seed = r.i64()?;
        let gamemode = r.u8()?;
        let _prev = r.i8()?;
        let _debug = r.bool()?;
        let _flat = r.bool()?;
        if r.bool()? {
            r.string()?;
            r.i64()?;
        }
        let _portal = r.varint()?;
        let _sea = r.varint()?;
        let info = self.dim_types.get(dim_type as usize).copied().flatten().unwrap_or(match dim.as_str() {
            "minecraft:the_nether" | "minecraft:the_end" => DimInfo { min_y: 0, height: 256 },
            _ => DimInfo { min_y: -64, height: 384 },
        });
        self.sections = (info.height / 16).max(1) as usize;
        Ok((dim, info.min_y, self.sections, gamemode))
    }

    /// Handles one play-state packet. Returns true if the server switched us back to configuration.
    fn play_packet(&mut self, pid: i32, r: &mut Reader) -> anyhow::Result<bool> {
        use id::play::*;
        match pid {
            CB_KEEP_ALIVE => {
                let v = r.i64()?;
                self.send(Writer::packet(SB_KEEP_ALIVE).i64(v).done());
            }
            CB_PING => {
                let v = r.i32()?;
                self.send(Writer::packet(SB_PONG).i32(v).done());
            }
            CB_LOGIN => {
                let _eid = r.i32()?;
                let _hardcore = r.bool()?;
                let n = r.len()?;
                for _ in 0..n {
                    r.string()?;
                }
                let _max = r.varint()?;
                let _view = r.varint()?;
                let _sim = r.varint()?;
                let _reduced = r.bool()?;
                let _death_screen = r.bool()?;
                let _limited = r.bool()?;
                let (dimension, min_y, sections, gamemode) = self.spawn_info(r)?;
                self.event(NetEvent::Joined { dimension, min_y, sections, gamemode });
            }
            CB_RESPAWN => {
                let (dimension, min_y, sections, gamemode) = self.spawn_info(r)?;
                self.event(NetEvent::Respawned { dimension, min_y, sections, gamemode });
            }
            CB_CHUNK => {
                let (x, z, column) = chunk::read_chunk(r, self.sections, &mut self.layout)?;
                self.event(NetEvent::Chunk { x, z, column: Box::new(column) });
            }
            CB_CHUNK_BATCH_FINISHED => {
                self.send(Writer::packet(SB_CHUNK_BATCH_RECEIVED).f32(25.0).done());
            }
            CB_LIGHT => {
                let x = r.varint()?;
                let z = r.varint()?;
                let light = chunk::read_light(r)?;
                self.event(NetEvent::Light { x, z, light: Box::new(light) });
            }
            CB_FORGET_CHUNK => {
                let v = r.i64()?;
                self.event(NetEvent::Unload { x: v as i32, z: (v >> 32) as i32 });
            }
            CB_BLOCK_UPDATE => {
                let (x, y, z) = r.position()?;
                let state = r.varint()? as u16;
                self.event(NetEvent::Block { x, y, z, state });
            }
            CB_SECTION_BLOCKS => {
                let s = r.i64()?;
                let (sx, sy, sz) = ((s >> 42) as i32, ((s << 44) >> 44) as i32, ((s << 22) >> 42) as i32);
                let n = r.len()?;
                for _ in 0..n {
                    let e = r.varlong()?;
                    let state = (e >> 12) as u16;
                    let p = (e & 0xfff) as i32;
                    self.event(NetEvent::Block { x: sx * 16 + (p >> 8 & 15), y: sy * 16 + (p & 15), z: sz * 16 + (p >> 4 & 15), state });
                }
            }
            CB_PLAYER_POSITION => {
                let tid = r.varint()?;
                let (x, y, z) = (r.f64()?, r.f64()?, r.f64()?);
                let (_vx, _vy, _vz) = (r.f64()?, r.f64()?, r.f64()?);
                let (yaw, pitch) = (r.f32()?, r.f32()?);
                let flags = r.i32()?;
                self.send(Writer::packet(SB_CONFIRM_TELEPORT).varint(tid).done());
                self.event(NetEvent::Teleport { x, y, z, yaw, pitch, flags });
            }
            CB_GAME_EVENT => {
                let ev = r.u8()?;
                let param = r.f32()?;
                if ev == 3 {
                    self.event(NetEvent::GameMode(param as u8));
                }
            }
            CB_ABILITIES => {
                let f = r.u8()?;
                self.event(NetEvent::Abilities { flying: f & 2 != 0, allow_flying: f & 4 != 0, instabuild: f & 8 != 0 });
            }
            CB_SET_HEALTH => {
                let h = r.f32()?;
                self.event(NetEvent::Health(h));
            }
            CB_SET_HELD_SLOT => {
                let s = r.varint()?;
                self.event(NetEvent::HeldSlot(s.clamp(0, 8) as u8));
            }
            CB_SYSTEM_CHAT => {
                let msg = nbt::read_text(r)?;
                let overlay = r.bool().unwrap_or(false);
                if !overlay {
                    self.event(NetEvent::Chat(msg));
                }
            }
            CB_PLAYER_CHAT => {
                if let Ok(msg) = read_player_chat(r) {
                    self.event(NetEvent::Chat(msg));
                }
            }
            CB_DISGUISED_CHAT => {
                let msg = nbt::read_text(r)?;
                self.event(NetEvent::Chat(msg));
            }
            CB_COMBAT_KILL => {
                let _id = r.varint()?;
                let msg = nbt::read_text(r)?;
                self.event(NetEvent::Chat(format!("You died: {msg}")));
            }
            CB_COOKIE_REQUEST => {
                let key = r.string()?;
                self.send(Writer::packet(SB_COOKIE_RESPONSE).string(&key).bool(false).done());
            }
            CB_RESOURCE_PACK_PUSH => {
                let uuid = r.uuid()?;
                self.send(Writer::packet(SB_RESOURCE_PACK).uuid(uuid).varint(3).done());
                self.send(Writer::packet(SB_RESOURCE_PACK).uuid(uuid).varint(0).done());
            }
            CB_START_CONFIG => {
                self.in_play.store(false, Ordering::Relaxed);
                self.send(Writer::packet(SB_ACK_CONFIG).done());
                return Ok(true);
            }
            CB_DISCONNECT => {
                let reason = nbt::read_text(r)?;
                bail!("{reason}");
            }
            _ => {}
        }
        Ok(false)
    }
}

fn read_player_chat(r: &mut Reader) -> anyhow::Result<String> {
    let _global = r.varint()?;
    let _sender = r.uuid()?;
    let _index = r.varint()?;
    if r.bool()? {
        r.bytes(256)?;
    }
    let message = r.string()?;
    let _ts = r.i64()?;
    let _salt = r.i64()?;
    let n = r.len()?;
    for _ in 0..n {
        if r.varint()? == 0 {
            r.bytes(256)?;
        }
    }
    let unsigned = if r.bool()? { Some(nbt::read_text(r)?) } else { None };
    if r.varint()? == 2 {
        r.long_array()?;
    }
    let chat_type = r.varint()?;
    let shown = unsigned.unwrap_or(message);
    if chat_type == 0 {
        return Ok(shown); // inline chat type: don't try to parse further
    }
    let name = nbt::read_text(r)?;
    Ok(format!("<{name}> {shown}"))
}

/// Connects to a server on a background thread and returns a handle for events and outgoing packets.
pub fn connect(opts: ConnectOptions) -> anyhow::Result<NetHandle> {
    let (host, port) = parse_address(&opts.address);
    let stream = open(&host, port)?;
    let out = spawn_writer(stream.try_clone()?);
    let (etx, erx) = channel();
    let in_play = Arc::new(AtomicBool::new(false));
    let handle = NetHandle { events: erx, out: out.clone(), in_play: in_play.clone(), sequence: 0 };

    std::thread::Builder::new().name("claudium-net-read".into()).spawn(move || {
        let mut session = Session {
            out: out.clone(),
            events: etx.clone(),
            in_play,
            dim_types: vec![],
            sections: 24,
            layout: chunk::LAYOUTS[0],
            opts_view: opts.view_distance,
            accept_conduct: opts.accept_code_of_conduct,
        };
        let mut reader = PacketReader { r: BufReader::with_capacity(1 << 16, stream), dec: None, threshold: -1 };
        if let Err(e) = run_session(&mut session, &mut reader, &opts, &host, port) {
            let _ = etx.send(NetEvent::Disconnected(e.to_string()));
        } else {
            let _ = etx.send(NetEvent::Disconnected("Connection closed".into()));
        }
    })?;
    Ok(handle)
}

fn run_session(s: &mut Session, reader: &mut PacketReader, opts: &ConnectOptions, host: &str, port: u16) -> anyhow::Result<()> {
    use id::login::*;
    let (name, uuid) = match &opts.profile {
        Some(p) => (p.name.clone(), p.uuid),
        None => (opts.username.clone(), 0u128),
    };
    s.send(handshake(host, port, opts.protocol, 2));
    s.send(Writer::packet(SB_HELLO).string(&name).uuid(uuid).done());
    s.event(NetEvent::Info(format!("Logging in as {name}...")));

    // Login
    loop {
        let p = reader.read()?;
        let mut r = Reader::new(&p);
        let pid = r.varint()?;
        match pid {
            CB_DISCONNECT => bail!("{}", nbt::json_text(&r.string()?)),
            CB_ENCRYPTION => {
                let server_id = r.string()?;
                let public_key = r.byte_array()?.to_vec();
                let token = r.byte_array()?.to_vec();
                let should_auth = r.bool().unwrap_or(true);
                let mut secret = [0u8; 16];
                rand::RngCore::fill_bytes(&mut rand::thread_rng(), &mut secret);
                if should_auth {
                    let profile =
                        opts.profile.as_ref().ok_or_else(|| anyhow!("this server is in online mode: sign in with --login first"))?;
                    let hash = auth::server_hash(&server_id, &secret, &public_key);
                    auth::join_server(profile, &hash)?;
                }
                use rsa::pkcs8::DecodePublicKey;
                let key = rsa::RsaPublicKey::from_public_key_der(&public_key).context("server public key")?;
                let mut rng = rand::thread_rng();
                let enc_secret = key.encrypt(&mut rng, rsa::Pkcs1v15Encrypt, &secret)?;
                let enc_token = key.encrypt(&mut rng, rsa::Pkcs1v15Encrypt, &token)?;
                s.send(Writer::packet(SB_KEY).byte_array(&enc_secret).byte_array(&enc_token).done());
                let _ = s.out.send(WriterCmd::Encrypt(secret));
                reader.dec = Some(Cfb8::new(&secret));
            }
            CB_COMPRESSION => {
                let t = r.varint()?;
                reader.threshold = t;
                let _ = s.out.send(WriterCmd::Threshold(t));
            }
            CB_SUCCESS => {
                let _uuid = r.uuid()?;
                let name = r.string()?;
                s.send(Writer::packet(SB_ACK).done());
                s.event(NetEvent::Info(format!("Logged in as {name}")));
                break;
            }
            CB_PLUGIN_REQUEST => {
                let tx = r.varint()?;
                s.send(Writer::packet(SB_PLUGIN_RESPONSE).varint(tx).bool(false).done());
            }
            CB_COOKIE_REQUEST => {
                let key = r.string()?;
                s.send(Writer::packet(SB_COOKIE_RESPONSE).string(&key).bool(false).done());
            }
            other => bail!("unexpected login packet 0x{other:02x}"),
        }
    }

    // Configuration, then play (the server may send us back to configuration at any time).
    loop {
        s.send(s.client_info(id::config::SB_CLIENT_INFO));
        loop {
            let p = reader.read()?;
            let mut r = Reader::new(&p);
            let pid = r.varint()?;
            if s.config_packet(pid, &mut r)? {
                break;
            }
        }
        loop {
            let p = reader.read()?;
            let mut r = Reader::new(&p);
            let pid = r.varint()?;
            match s.play_packet(pid, &mut r) {
                Ok(true) => break,
                Ok(false) => {}
                Err(e) if e.downcast_ref::<buf::DecodeError>().is_some() => {
                    // A packet we misunderstood: report it but keep the connection.
                    s.event(NetEvent::Info(format!("could not read packet 0x{pid:02x}: {e}")));
                }
                Err(e) => return Err(e),
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Outgoing play packets

pub mod out {
    use super::buf::Writer;
    use super::id::play::*;

    pub fn position(x: f64, y: f64, z: f64, yaw: f32, pitch: f32, on_ground: bool, horizontal: bool) -> Vec<u8> {
        let flags = on_ground as u8 | ((horizontal as u8) << 1);
        Writer::packet(SB_POS_ROT).f64(x).f64(y).f64(z).f32(yaw).f32(pitch).u8(flags).done()
    }
    pub fn tick_end() -> Vec<u8> {
        Writer::packet(SB_TICK_END).done()
    }
    pub fn player_loaded() -> Vec<u8> {
        Writer::packet(SB_PLAYER_LOADED).done()
    }
    /// status: 0 start digging, 1 cancel, 2 finish.
    pub fn dig(status: i32, x: i32, y: i32, z: i32, face: u8, seq: i32) -> Vec<u8> {
        Writer::packet(SB_PLAYER_ACTION).varint(status).position(x, y, z).u8(face).varint(seq).done()
    }
    pub fn use_item_on(x: i32, y: i32, z: i32, face: u8, cursor: [f32; 3], seq: i32) -> Vec<u8> {
        Writer::packet(SB_USE_ITEM_ON)
            .varint(0)
            .position(x, y, z)
            .varint(face as i32)
            .f32(cursor[0])
            .f32(cursor[1])
            .f32(cursor[2])
            .bool(false)
            .bool(false)
            .varint(seq)
            .done()
    }
    pub fn swing() -> Vec<u8> {
        Writer::packet(SB_SWING).varint(0).done()
    }
    pub fn held_slot(slot: u8) -> Vec<u8> {
        Writer::packet(SB_SET_HELD).i16(slot as i16).done()
    }
    pub fn abilities(flying: bool) -> Vec<u8> {
        Writer::packet(SB_ABILITIES).u8(if flying { 0x02 } else { 0 }).done()
    }
    pub fn respawn() -> Vec<u8> {
        Writer::packet(SB_CLIENT_STATUS).varint(0).done()
    }
    pub fn chat(msg: &str) -> Vec<u8> {
        if let Some(cmd) = msg.strip_prefix('/') {
            return Writer::packet(SB_CHAT_COMMAND).string(cmd).done();
        }
        let ts = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0);
        // Unsigned chat: accepted by servers that don't enforce secure chat.
        Writer::packet(SB_CHAT).string(msg).i64(ts).i64(0).bool(false).varint(0).raw(&[0, 0, 0]).done()
    }
}
