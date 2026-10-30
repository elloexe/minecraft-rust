//! Claudium's own Minecraft server.
//!
//! This is the "like Horizon" piece: Claudium can host the world it generates and
//! serve it over the same protocol its client speaks, so both ends are ours and the
//! version can never mismatch. It speaks offline mode (no encryption, no compression)
//! and is deliberately small — enough to walk around, stream chunks, break and place
//! blocks, and chat. It serves Claudium clients (connect with `--connect`).

use std::collections::{HashMap, HashSet};
use std::io::{self, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::mpsc::{Sender, channel};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

type SResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

use crate::net::buf::{Reader, Writer};
use crate::net::{chunk, id};
use crate::registry::{self, Registry};
use crate::world::Column;
use crate::worldgen::{self, Generator};

const SECTIONS: usize = worldgen::SP_SECTIONS;
const SEA: i32 = worldgen::SEA_LEVEL;

/// The world the server owns: generated lazily, shared between connected players.
struct ServerWorld {
    generator: Generator,
    columns: HashMap<(i32, i32), Column>,
}

impl ServerWorld {
    fn column(&mut self, cx: i32, cz: i32) -> &Column {
        if !self.columns.contains_key(&(cx, cz)) {
            let c = self.generator.generate(cx, cz);
            self.columns.insert((cx, cz), c);
        }
        self.columns.get(&(cx, cz)).unwrap()
    }
    fn set_block(&mut self, x: i32, y: i32, z: i32, id: u16) {
        if y < 0 || y >= (SECTIONS * 16) as i32 {
            return;
        }
        let key = (x >> 4, z >> 4);
        if !self.columns.contains_key(&key) {
            let c = self.generator.generate(key.0, key.1);
            self.columns.insert(key, c);
        }
        if let Some(col) = self.columns.get_mut(&key) {
            col.set((x & 15) as usize, y as usize, (z & 15) as usize, id);
        }
    }
}

struct Client {
    out: Sender<Vec<u8>>,
    name: String,
}

struct Shared {
    reg: Arc<Registry>,
    stone: u16,
    world: Mutex<ServerWorld>,
    clients: Mutex<HashMap<u32, Client>>,
    next_id: AtomicU32,
}

impl Shared {
    fn new(seed: u32) -> Arc<Self> {
        let reg = Arc::new(registry::build_singleplayer(&|_| false).0);
        let generator = Generator::new(seed, &reg);
        let stone = reg.id_of("stone").unwrap_or(1);
        Arc::new(Shared {
            reg,
            stone,
            world: Mutex::new(ServerWorld { generator, columns: HashMap::new() }),
            clients: Mutex::new(HashMap::new()),
            next_id: AtomicU32::new(1),
        })
    }

    fn broadcast(&self, packet: Vec<u8>) {
        let clients = self.clients.lock().unwrap();
        for c in clients.values() {
            let _ = c.out.send(packet.clone());
        }
    }

    fn spawn_y(&self) -> f64 {
        let (h, _, _) = self.world.lock().unwrap().generator.sample(0, 0);
        h.max(SEA) as f64 + 2.0
    }
}

// --- plain framing (offline: no compression, no encryption) ----------------

fn read_packet(stream: &mut TcpStream) -> io::Result<Vec<u8>> {
    let mut len = 0u32;
    for i in 0..5 {
        let mut b = [0u8; 1];
        stream.read_exact(&mut b)?;
        len |= ((b[0] & 0x7f) as u32) << (7 * i);
        if b[0] & 0x80 == 0 {
            break;
        }
    }
    if len > 8 * 1024 * 1024 {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "packet too large"));
    }
    let mut body = vec![0u8; len as usize];
    stream.read_exact(&mut body)?;
    Ok(body)
}

fn framed(packet: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(packet.len() + 5);
    crate::net::buf::write_varint(&mut out, packet.len() as i32);
    out.extend_from_slice(packet);
    out
}

fn nbt_text(s: &str) -> Vec<u8> {
    // A nameless network-NBT string tag (tag id 8).
    let mut v = vec![8u8];
    v.extend_from_slice(&(s.len() as u16).to_be_bytes());
    v.extend_from_slice(s.as_bytes());
    v
}

fn dimension_nbt() -> Vec<u8> {
    // Compound { min_y: 0, height: SECTIONS*16 } — the fields Claudium's client reads.
    let mut v = vec![10u8]; // compound (nameless root)
    let mut int_field = |v: &mut Vec<u8>, name: &str, val: i32| {
        v.push(3); // TAG_Int
        v.extend_from_slice(&(name.len() as u16).to_be_bytes());
        v.extend_from_slice(name.as_bytes());
        v.extend_from_slice(&val.to_be_bytes());
    };
    int_field(&mut v, "min_y", 0);
    int_field(&mut v, "height", (SECTIONS * 16) as i32);
    int_field(&mut v, "logical_height", (SECTIONS * 16) as i32);
    v.push(0); // end
    v
}

// --- connection handling ----------------------------------------------------

struct PlayerConn {
    pos: [f64; 3],
    loaded: HashSet<(i32, i32)>,
    view: i32,
    teleport_id: i32,
    last_keep_alive: Instant,
}

fn handle(mut stream: TcpStream, shared: Arc<Shared>) {
    stream.set_nodelay(true).ok();
    if let Err(e) = handle_inner(&mut stream, &shared) {
        let _ = e; // client disconnects are routine
    }
}

fn handle_inner(stream: &mut TcpStream, shared: &Arc<Shared>) -> SResult<()> {
    // Handshake
    let hs = read_packet(stream)?;
    let mut r = Reader::new(&hs);
    if r.varint()? != 0x00 {
        return Ok(());
    }
    let _proto = r.varint()?;
    let _addr = r.string()?;
    let _port = r.u16()?;
    let next = r.varint()?;

    if next == 1 {
        return status(stream, shared);
    }
    if next != 2 {
        return Ok(());
    }

    // Login (offline: no encryption, no compression)
    let p = read_packet(stream)?;
    let mut r = Reader::new(&p);
    if r.varint()? != id::login::SB_HELLO {
        return Ok(());
    }
    let name = r.string()?;
    let uuid = r.uuid().unwrap_or(0);
    let eid = shared.next_id.fetch_add(1, Ordering::Relaxed);
    let success = Writer::packet(id::login::CB_SUCCESS).uuid(uuid).string(&name).varint(0).done();
    stream.write_all(&framed(&success))?;
    // Login Acknowledged
    let p = read_packet(stream)?;
    if Reader::new(&p).varint()? != id::login::SB_ACK {
        return Ok(());
    }

    // Configuration: send registries + finish, wait for the client's finish.
    let reg_data = Writer::packet(id::config::CB_REGISTRY_DATA)
        .string("minecraft:dimension_type")
        .varint(1)
        .string("minecraft:overworld")
        .bool(true)
        .raw(&dimension_nbt())
        .done();
    stream.write_all(&framed(&reg_data))?;
    stream.write_all(&framed(&Writer::packet(id::config::CB_FINISH).done()))?;
    loop {
        let p = read_packet(stream)?;
        let pid = Reader::new(&p).varint()?;
        if pid == id::config::SB_FINISH {
            break;
        }
    }

    // Set up a writer thread so broadcasts from other players can reach this socket.
    let (tx, rx) = channel::<Vec<u8>>();
    let mut write_half = stream.try_clone()?;
    let writer = std::thread::spawn(move || {
        while let Ok(pkt) = rx.recv() {
            if write_half.write_all(&framed(&pkt)).is_err() {
                break;
            }
        }
    });
    shared.clients.lock().unwrap().insert(eid, Client { out: tx.clone(), name: name.clone() });
    shared.broadcast(Writer::packet(id::play::CB_SYSTEM_CHAT).raw(&nbt_text(&format!("{name} joined the game"))).bool(false).done());
    println!("{name} connected (id {eid})");

    let result = play(stream, shared, &tx, eid, &name);

    shared.clients.lock().unwrap().remove(&eid);
    shared.broadcast(Writer::packet(id::play::CB_SYSTEM_CHAT).raw(&nbt_text(&format!("{name} left the game"))).bool(false).done());
    println!("{name} disconnected");
    drop(tx);
    let _ = writer.join();
    result
}

fn status(stream: &mut TcpStream, shared: &Arc<Shared>) -> SResult<()> {
    let online = shared.clients.lock().unwrap().len();
    let json = format!(
        "{{\"version\":{{\"name\":\"Claudium {}\",\"protocol\":{}}},\"players\":{{\"online\":{},\"max\":100}},\"description\":{{\"text\":\"Claudium server\"}}}}",
        crate::net::DEFAULT_VERSION,
        crate::net::PROTOCOL,
        online
    );
    // Status Request then Ping.
    let _ = read_packet(stream)?;
    stream.write_all(&framed(&Writer::packet(0x00).string(&json).done()))?;
    if let Ok(p) = read_packet(stream) {
        let mut r = Reader::new(&p);
        if r.varint().ok() == Some(0x01) {
            if let Ok(n) = r.i64() {
                stream.write_all(&framed(&Writer::packet(0x01).i64(n).done()))?;
            }
        }
    }
    Ok(())
}

fn play(stream: &mut TcpStream, shared: &Arc<Shared>, tx: &Sender<Vec<u8>>, eid: u32, name: &str) -> SResult<()> {
    let spawn = [0.5, shared.spawn_y(), 0.5];
    let mut pc = PlayerConn { pos: spawn, loaded: HashSet::new(), view: 8, teleport_id: 1, last_keep_alive: Instant::now() };

    // Join game
    let login = Writer::packet(id::play::CB_LOGIN)
        .i32(eid as i32)
        .bool(false)
        .varint(1)
        .string("minecraft:overworld")
        .varint(100)
        .varint(pc.view)
        .varint(pc.view)
        .bool(false)
        .bool(true)
        .bool(false)
        // spawn info
        .varint(0)
        .string("minecraft:overworld")
        .i64(0)
        .u8(1) // creative
        .u8(0xFF) // previous game mode = -1
        .bool(false)
        .bool(false)
        .bool(false)
        .varint(0)
        .varint(SEA)
        .bool(false)
        .done();
    tx.send(login).ok();

    stream_chunks(shared, tx, &mut pc);
    // Teleport the player to spawn so the client confirms and starts ticking.
    tx.send(teleport(&mut pc, spawn)).ok();

    stream.set_read_timeout(Some(Duration::from_millis(500)))?;
    loop {
        match read_packet(stream) {
            Ok(p) => {
                let mut r = Reader::new(&p);
                let pid = match r.varint() {
                    Ok(v) => v,
                    Err(_) => continue,
                };
                handle_play_packet(shared, tx, &mut pc, name, pid, &mut r);
            }
            Err(e) if e.kind() == io::ErrorKind::WouldBlock || e.kind() == io::ErrorKind::TimedOut => {}
            Err(_) => return Ok(()),
        }
        if pc.last_keep_alive.elapsed() > Duration::from_secs(10) {
            pc.last_keep_alive = Instant::now();
            tx.send(Writer::packet(id::play::CB_KEEP_ALIVE).i64(42).done()).ok();
        }
        stream_chunks(shared, tx, &mut pc);
    }
}

fn teleport(pc: &mut PlayerConn, pos: [f64; 3]) -> Vec<u8> {
    pc.teleport_id += 1;
    Writer::packet(id::play::CB_PLAYER_POSITION)
        .varint(pc.teleport_id)
        .f64(pos[0])
        .f64(pos[1])
        .f64(pos[2])
        .f64(0.0)
        .f64(0.0)
        .f64(0.0)
        .f32(0.0)
        .f32(0.0)
        .i32(0)
        .done()
}

fn stream_chunks(shared: &Arc<Shared>, tx: &Sender<Vec<u8>>, pc: &mut PlayerConn) {
    let (pcx, pcz) = (pc.pos[0].floor() as i32 >> 4, pc.pos[2].floor() as i32 >> 4);
    let mut sent = 0;
    for ring in 0..=pc.view {
        for dz in -ring..=ring {
            for dx in -ring..=ring {
                if dx.abs() != ring && dz.abs() != ring {
                    continue;
                }
                let key = (pcx + dx, pcz + dz);
                if pc.loaded.contains(&key) {
                    continue;
                }
                let body = {
                    let mut w = shared.world.lock().unwrap();
                    chunk::encode_chunk_body(key.0, key.1, w.column(key.0, key.1))
                };
                tx.send(Writer::packet(id::play::CB_CHUNK).raw(&body).done()).ok();
                pc.loaded.insert(key);
                sent += 1;
                if sent >= 8 {
                    return; // spread chunk work across wake-ups
                }
            }
        }
    }
}

fn handle_play_packet(shared: &Arc<Shared>, _tx: &Sender<Vec<u8>>, pc: &mut PlayerConn, name: &str, pid: i32, r: &mut Reader) {
    use id::play::*;
    // All reads are fallible; a short packet just means we ignore it.
    let _ = (|| -> crate::net::buf::DResult<()> {
        match pid {
            SB_POS | SB_POS_ROT => {
                pc.pos = [r.f64()?, r.f64()?, r.f64()?];
            }
            SB_PLAYER_ACTION => {
                let status = r.varint()?;
                let (x, y, z) = r.position()?;
                // 0 = start (creative instant break), 2 = finish digging.
                if status == 0 || status == 2 {
                    shared.world.lock().unwrap().set_block(x, y, z, 0);
                    shared.broadcast(block_update(x, y, z, 0));
                }
            }
            SB_USE_ITEM_ON => {
                let _hand = r.varint()?;
                let (x, y, z) = r.position()?;
                let face = r.varint()?;
                let (px, py, pz) = offset(x, y, z, face);
                // Placement isn't inventory-aware yet, so new blocks are stone.
                shared.world.lock().unwrap().set_block(px, py, pz, shared.stone);
                shared.broadcast(block_update(px, py, pz, shared.stone));
            }
            SB_CHAT => {
                let msg = r.string()?;
                let line = format!("<{name}> {msg}");
                println!("{line}");
                shared.broadcast(Writer::packet(CB_SYSTEM_CHAT).raw(&nbt_text(&line)).bool(false).done());
            }
            _ => {}
        }
        Ok(())
    })();
}

fn block_update(x: i32, y: i32, z: i32, state: u16) -> Vec<u8> {
    Writer::packet(id::play::CB_BLOCK_UPDATE).position(x, y, z).varint(state as i32).done()
}

/// The block adjacent to `face` of (x,y,z) — where a placed block goes.
fn offset(x: i32, y: i32, z: i32, face: i32) -> (i32, i32, i32) {
    match face {
        0 => (x, y - 1, z),
        1 => (x, y + 1, z),
        2 => (x, y, z - 1),
        3 => (x, y, z + 1),
        4 => (x - 1, y, z),
        5 => (x + 1, y, z),
        _ => (x, y, z),
    }
}

/// Runs the server on `listener` forever.
pub fn serve(listener: TcpListener, seed: u32) {
    let shared = Shared::new(seed);
    for stream in listener.incoming() {
        match stream {
            Ok(s) => {
                let shared = shared.clone();
                std::thread::spawn(move || handle(s, shared));
            }
            Err(e) => eprintln!("accept error: {e}"),
        }
    }
}

/// Binds and runs a dedicated server (for `claudium --server`).
pub fn run(port: u16, seed: u32) -> SResult<()> {
    let listener = TcpListener::bind(("0.0.0.0", port))?;
    println!("Claudium server listening on 0.0.0.0:{} (seed {seed})", listener.local_addr()?.port());
    println!("Connect another Claudium with:  claudium --connect <this-ip>");
    serve(listener, seed);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::buf::Reader;

    fn send(stream: &mut TcpStream, pkt: Vec<u8>) {
        stream.write_all(&framed(&pkt)).unwrap();
    }

    #[test]
    fn client_can_log_in_receive_chunks_and_break_a_block() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        std::thread::spawn(move || serve(listener, 1234));

        let mut s = TcpStream::connect(("127.0.0.1", port)).unwrap();
        s.set_read_timeout(Some(Duration::from_secs(5))).unwrap();

        // Handshake (next state 2 = login) + Login Start
        send(&mut s, Writer::packet(0x00).varint(crate::net::PROTOCOL).string("localhost").u16(port).varint(2).done());
        send(&mut s, Writer::packet(id::login::SB_HELLO).string("Tester").uuid(0).done());

        // Login Success -> acknowledge
        let p = read_packet(&mut s).unwrap();
        assert_eq!(Reader::new(&p).varint().unwrap(), id::login::CB_SUCCESS);
        send(&mut s, Writer::packet(id::login::SB_ACK).done());

        // Configuration: client info, then read until Finish, then ack Finish.
        send(
            &mut s,
            Writer::packet(id::config::SB_CLIENT_INFO)
                .string("en_us")
                .u8(8)
                .varint(0)
                .bool(true)
                .u8(0x7f)
                .varint(1)
                .bool(false)
                .bool(true)
                .varint(0)
                .done(),
        );
        loop {
            let p = read_packet(&mut s).unwrap();
            if Reader::new(&p).varint().unwrap() == id::config::CB_FINISH {
                break;
            }
        }
        send(&mut s, Writer::packet(id::config::SB_FINISH).done());

        // Play: expect a Login (join) packet and at least one chunk.
        let mut got_login = false;
        let mut got_chunk = false;
        let mut teleport_id = None;
        for _ in 0..400 {
            let p = read_packet(&mut s).unwrap();
            let mut r = Reader::new(&p);
            match r.varint().unwrap() {
                x if x == id::play::CB_LOGIN => got_login = true,
                x if x == id::play::CB_CHUNK => got_chunk = true,
                x if x == id::play::CB_PLAYER_POSITION => {
                    teleport_id = Some(r.varint().unwrap());
                }
                _ => {}
            }
            if got_login && got_chunk && teleport_id.is_some() {
                break;
            }
        }
        assert!(got_login, "no join packet");
        assert!(got_chunk, "no chunk packet");
        let tid = teleport_id.expect("no teleport");
        send(&mut s, Writer::packet(id::play::SB_CONFIRM_TELEPORT).varint(tid).done());

        // Break the block at (0, SEA, 0): finish digging, expect a Block Update back.
        send(&mut s, Writer::packet(id::play::SB_PLAYER_ACTION).varint(2).position(0, SEA, 0).u8(1).varint(1).done());
        let mut got_block_update = false;
        for _ in 0..400 {
            let p = read_packet(&mut s).unwrap();
            let mut r = Reader::new(&p);
            if r.varint().unwrap() == id::play::CB_BLOCK_UPDATE {
                let (x, y, z) = r.position().unwrap();
                let state = r.varint().unwrap();
                if (x, y, z) == (0, SEA, 0) {
                    assert_eq!(state, 0, "broken block should become air");
                    got_block_update = true;
                    break;
                }
            }
        }
        assert!(got_block_update, "server did not confirm the block break");
    }
}
