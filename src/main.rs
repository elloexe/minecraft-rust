//! Claudium: a fast block-game client written from scratch in Rust.
//!
//! Play a local world, or join Java Edition servers. Claudium contains no game
//! code or art; it can read textures from your own installed copy of the game.

mod assets;
mod menu;
mod mesher;
mod net;
mod player;
mod registry;
mod render;
mod save;
mod world;
mod worldgen;

#[cfg(test)]
mod tests;

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, channel};
use std::time::Instant;

use glam::{DVec3, IVec3};
use winit::application::ApplicationHandler;
use winit::event::{DeviceEvent, DeviceId, ElementState, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{CursorGrabMode, Window, WindowId};

use mesher::{Job, JobResult, Pool};
use net::{NetEvent, NetHandle};
use player::{MoveInput, Player};
use registry::{Kind, Registry};
use render::{Camera, FrameParams, HudVertex, Renderer};
use world::World;

pub fn data_dir() -> PathBuf {
    if let Ok(d) = std::env::var("CLAUDIUM_HOME") {
        return PathBuf::from(d);
    }
    if let Ok(a) = std::env::var("APPDATA") {
        return PathBuf::from(a).join("claudium");
    }
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    PathBuf::from(home).join(".claudium")
}

// ---------------------------------------------------------------------------
// Command line

#[derive(Clone)]
struct Args {
    connect: Option<String>,
    ping: Option<String>,
    name: String,
    login: bool,
    logout: bool,
    client_id: Option<String>,
    world: String,
    seed: u32,
    assets: Option<PathBuf>,
    blocks: Option<PathBuf>,
    version: String,
    render_distance: i32,
    vsync: bool,
    fov: f32,
    protocol: Option<i32>,
    accept_conduct: bool,
}

const HELP: &str = "\
Claudium - a block-game client written in Rust

USAGE:
  claudium                              play the local world
  claudium --connect HOST[:PORT]        join a server
  claudium --ping HOST[:PORT]           show a server's status

OPTIONS:
  --name NAME             username for offline-mode servers (default: Player)
  --login                 sign in with a Microsoft account (for online-mode servers)
  --client-id ID          Azure app client id for sign-in (or CLAUDIUM_CLIENT_ID)
  --logout                forget the saved sign-in
  --world NAME            local world name (default: world)
  --seed N                seed for a new local world
  --assets PATH           game jar or resource-pack zip to read textures from
  --version V             game version to look for in your launcher folder (default: 26.1.2)
  --blocks PATH           blocks.json from the data generator (needed to show server blocks correctly)
  --render-distance N     in chunks (default: 12)
  --fov DEGREES           field of view (default: 70)
  --no-vsync              uncapped frame rate
  --protocol N            override the protocol number sent to servers
  --accept-conduct        accept a server's code of conduct automatically

CONTROLS:
  WASD move, Space jump/up, Shift sneak/down, Ctrl sprint, F toggle flying
  Left click break, right click place, middle click pick, 1-9 / wheel select
  Esc release the mouse. Type chat or /commands in the terminal.
";

fn parse_args() -> Args {
    let mut a = Args {
        connect: None,
        ping: None,
        name: "Player".into(),
        login: false,
        logout: false,
        client_id: std::env::var("CLAUDIUM_CLIENT_ID").ok(),
        world: "world".into(),
        seed: 0,
        assets: None,
        blocks: None,
        version: net::GAME_VERSION.into(),
        render_distance: 12,
        vsync: true,
        fov: 70.0,
        protocol: None,
        accept_conduct: false,
    };
    let mut seed_set = false;
    let mut it = std::env::args().skip(1);
    while let Some(arg) = it.next() {
        let mut val = || {
            it.next().unwrap_or_else(|| {
                eprintln!("{arg} needs a value");
                std::process::exit(2)
            })
        };
        match arg.as_str() {
            "--connect" | "-c" => a.connect = Some(val()),
            "--ping" => a.ping = Some(val()),
            "--name" | "-n" => a.name = val(),
            "--login" => a.login = true,
            "--logout" => a.logout = true,
            "--client-id" => a.client_id = Some(val()),
            "--world" => a.world = val(),
            "--seed" => {
                let v = val();
                a.seed = v.parse().unwrap_or_else(|_| v.bytes().fold(7u32, |h, b| h.wrapping_mul(31).wrapping_add(b as u32)));
                seed_set = true;
            }
            "--assets" => a.assets = Some(val().into()),
            "--blocks" => a.blocks = Some(val().into()),
            "--version" => a.version = val(),
            "--render-distance" => a.render_distance = val().parse().unwrap_or(12).clamp(2, 48),
            "--fov" => a.fov = val().parse().unwrap_or(70.0),
            "--no-vsync" => a.vsync = false,
            "--protocol" => a.protocol = val().parse().ok(),
            "--accept-conduct" => a.accept_conduct = true,
            "--help" | "-h" => {
                print!("{HELP}");
                std::process::exit(0);
            }
            other => {
                eprintln!("unknown option {other}\n\n{HELP}");
                std::process::exit(2);
            }
        }
    }
    if !seed_set {
        a.seed = 0x5eed_c1a0;
    }
    a
}

// ---------------------------------------------------------------------------

struct Singleplayer {
    save: save::WorldSave,
    pending: HashSet<(i32, i32)>,
}

struct Multiplayer {
    net: NetHandle,
    joined: bool,
    loaded_sent: bool,
    gamemode: u8,
    last_send: Instant,
    dimension: String,
    dig_started: Option<(IVec3, Instant)>,
}

enum Mode {
    Single(Singleplayer),
    Multi(Multiplayer),
}

#[derive(Default)]
struct Input {
    keys: HashSet<KeyCode>,
    left: bool,
    right: bool,
    middle_clicked: bool,
    left_clicked: bool,
    right_clicked: bool,
    grabbed: bool,
    mouse_delta: (f64, f64),
    scroll: f32,
}

struct Game {
    window: Arc<Window>,
    renderer: Renderer,
    world: World,
    player: Player,
    pool: Pool,
    mode: Mode,
    input: Input,
    args: Args,
    versions: HashMap<(i32, i32), u64>,
    next_version: u64,
    hotbar: [u16; 9],
    slot: usize,
    last_frame: Instant,
    last_title: Instant,
    frames: u32,
    fps: f32,
    action_cooldown: f32,
    console: Receiver<String>,
    tile_of_block: Vec<u16>,
    last_save: Instant,
    screen: Screen,
    title: menu::TitleScreen,
    cursor: (f32, f32),
    started: Instant,
}

/// Which screen is showing. The title screen sits in front of the game; entering
/// the world does not change any gameplay, world, or network code.
#[derive(PartialEq)]
enum Screen {
    Title,
    Playing,
}

fn console_reader() -> Receiver<String> {
    let (tx, rx) = channel();
    std::thread::Builder::new()
        .name("claudium-console".into())
        .spawn(move || {
            let stdin = std::io::stdin();
            let mut line = String::new();
            while stdin.read_line(&mut line).map(|n| n > 0).unwrap_or(false) {
                let l = line.trim().to_string();
                if !l.is_empty() && tx.send(l).is_err() {
                    return;
                }
                line.clear();
            }
        })
        .ok();
    rx
}

fn find_blocks_report(args: &Args) -> Option<PathBuf> {
    if let Some(p) = &args.blocks {
        return Some(p.clone());
    }
    let cands = [
        data_dir().join("reports").join(&args.version).join("blocks.json"),
        data_dir().join("blocks.json"),
        PathBuf::from("generated/reports/blocks.json"),
    ];
    cands.into_iter().find(|p| p.exists())
}

impl Game {
    fn new(event_loop: &ActiveEventLoop, args: Args) -> anyhow::Result<Self> {
        // Textures from the player's own game install, if available.
        let jar = args.assets.clone().or_else(|| assets::find_game_jar(&args.version));
        let textures = match &jar {
            Some(p) => match assets::TextureSource::from_zip(p) {
                Ok(t) => {
                    println!("Using {} textures from {}", t.count(), p.display());
                    t
                }
                Err(e) => {
                    eprintln!("Could not read textures from {}: {e}", p.display());
                    assets::TextureSource::empty()
                }
            },
            None => {
                println!("No game install found; using Claudium's built-in textures (pass --assets to use yours)");
                assets::TextureSource::empty()
            }
        };
        let has_tex = |n: &str| textures.has(n);

        // Block registry
        let multiplayer = args.connect.is_some();
        let (registry, tiles) = if multiplayer {
            match find_blocks_report(&args) {
                Some(p) => {
                    let text = std::fs::read_to_string(&p)?;
                    let json: serde_json::Value = serde_json::from_str(&text)?;
                    let (r, t) = registry::build_from_report(&json, &has_tex)?;
                    println!("Loaded {} block states from {}", r.blocks.len(), p.display());
                    (r, t)
                }
                None => {
                    println!(
                        "No blocks.json found: server blocks will show as coloured cubes.\n\
                         Generate it from your server jar with the official data generator:\n  \
                         java -DbundlerMainClass=net.minecraft.data.Main -jar server.jar --reports\n\
                         then pass --blocks generated/reports/blocks.json"
                    );
                    registry::build_unknown_states()
                }
            }
        } else {
            registry::build_singleplayer(&has_tex)
        };
        let registry = Arc::new(registry);
        let (atlas, aw, ah, from_game) = assets::build_atlas(&tiles, &textures);
        println!("Texture atlas: {} tiles ({} from your game files)", tiles.len(), from_game);

        let window = Arc::new(event_loop.create_window(
            Window::default_attributes().with_title("Claudium").with_inner_size(winit::dpi::LogicalSize::new(1280.0, 720.0)),
        )?);
        let renderer = pollster::block_on(Renderer::new(window.clone(), &atlas, aw, ah, args.vsync))?;
        println!("GPU: {}", renderer.adapter_name);

        let tile_of_block: Vec<u16> = registry.blocks.iter().map(|b| b.tex[2]).collect();

        let (mode, world, player, generator) = if let Some(addr) = &args.connect {
            let profile = if args.login {
                let cid = args.client_id.clone().ok_or_else(|| {
                    anyhow::anyhow!("--login needs --client-id or CLAUDIUM_CLIENT_ID (an Azure app approved for the game services API)")
                })?;
                Some(net::auth::login(&cid)?)
            } else {
                None
            };
            let mut protocol = args.protocol.unwrap_or(net::PROTOCOL);
            if args.protocol.is_none() {
                match net::ping(addr) {
                    Ok(st) => {
                        println!("{} - {} ({} / {} players, {} ms)", addr, st.version, st.online, st.max, st.latency_ms);
                        println!("{}", st.motd);
                        if st.protocol != net::PROTOCOL && st.protocol > 0 {
                            println!(
                                "Note: server uses protocol {} but Claudium's packet ids target {} ({}). Trying anyway.",
                                st.protocol,
                                net::PROTOCOL,
                                net::GAME_VERSION
                            );
                            protocol = st.protocol;
                        }
                    }
                    Err(e) => println!("Status ping failed ({e}); connecting anyway"),
                }
            }
            let net = net::connect(net::ConnectOptions {
                address: addr.clone(),
                username: args.name.clone(),
                profile,
                protocol,
                view_distance: args.render_distance as u8,
                accept_code_of_conduct: args.accept_conduct,
            })?;
            let mp = Multiplayer {
                net,
                joined: false,
                loaded_sent: false,
                gamemode: 0,
                last_send: Instant::now(),
                dimension: String::new(),
                dig_started: None,
            };
            let mut p = Player::new(DVec3::new(0.5, 100.0, 0.5));
            p.can_fly = false;
            (Mode::Multi(mp), World::new(registry.clone(), -64, 24), p, None)
        } else {
            let save = save::WorldSave::open(data_dir().join("worlds").join(&args.world))?;
            println!("World saved in {}", save.dir.display());
            let generator = Arc::new(worldgen::Generator::new(args.seed, &registry));
            let mut p = match save.load_player() {
                Some((pos, yaw, pitch, flying)) => {
                    let mut p = Player::new(DVec3::from_array(pos));
                    p.yaw = yaw;
                    p.pitch = pitch;
                    p.flying = flying;
                    p
                }
                None => {
                    let (h, _, _) = generator.sample(0, 0);
                    Player::new(DVec3::new(0.5, h.max(worldgen::SEA_LEVEL) as f64 + 2.0, 0.5))
                }
            };
            p.can_fly = true;
            let world = World::new(registry.clone(), 0, worldgen::SP_SECTIONS);
            (Mode::Single(Singleplayer { save, pending: HashSet::new() }), world, p, Some(generator))
        };

        let pool = Pool::new(registry.clone(), generator);
        let hotbar_names = ["stone", "dirt", "grass_block", "oak_planks", "cobblestone", "oak_log", "glass", "sand", "bricks"];
        let mut hotbar = [1u16; 9];
        for (i, n) in hotbar_names.iter().enumerate() {
            hotbar[i] = registry.id_of(n).unwrap_or(1);
        }

        // Title-screen buttons depend on how Claudium was launched.
        let mut entries = Vec::new();
        match &mode {
            Mode::Multi(_) => {
                entries.push((format!("JOIN {}", args.connect.clone().unwrap_or_default().to_uppercase()), menu::Action::JoinServer))
            }
            Mode::Single(_) => entries.push(("SINGLEPLAYER".to_string(), menu::Action::Play)),
        }
        entries.push(("QUIT".to_string(), menu::Action::Quit));
        let splashes = ["MADE IN RUST!", "100% FERROUS!", "NOW WITH CHUNKS!", "BLAZINGLY FAST!", "NO JAVA REQUIRED!", "OXIDIZED!"];
        let splash = splashes[(args.seed as usize) % splashes.len()].to_string();
        let title = menu::TitleScreen {
            entries,
            status: String::new(),
            splash,
            version: format!("CLAUDIUM {} - RUST EDITION", env!("CARGO_PKG_VERSION")),
        };

        Ok(Game {
            window,
            renderer,
            world,
            player,
            pool,
            mode,
            input: Input::default(),
            args,
            versions: HashMap::new(),
            next_version: 1,
            hotbar,
            slot: 0,
            last_frame: Instant::now(),
            last_title: Instant::now(),
            frames: 0,
            fps: 0.0,
            action_cooldown: 0.0,
            console: console_reader(),
            tile_of_block,
            last_save: Instant::now(),
            screen: Screen::Title,
            title,
            cursor: (0.0, 0.0),
            started: Instant::now(),
        })
    }

    fn grab(&mut self, on: bool) {
        if on {
            let ok = self.window.set_cursor_grab(CursorGrabMode::Locked).or_else(|_| self.window.set_cursor_grab(CursorGrabMode::Confined));
            if ok.is_ok() {
                self.window.set_cursor_visible(false);
                self.input.grabbed = true;
            }
        } else {
            let _ = self.window.set_cursor_grab(CursorGrabMode::None);
            self.window.set_cursor_visible(true);
            self.input.grabbed = false;
        }
    }

    fn chunk_pos(&self) -> (i32, i32) {
        ((self.player.pos.x.floor() as i32) >> 4, (self.player.pos.z.floor() as i32) >> 4)
    }

    fn select_slot(&mut self, s: usize) {
        self.slot = s % 9;
        if let Mode::Multi(mp) = &self.mode {
            if mp.net.playing() {
                mp.net.send(net::out::held_slot(self.slot as u8));
            }
        }
    }

    // --- per-frame work ---------------------------------------------------

    fn handle_job_results(&mut self) {
        let mut budget = 64;
        while budget > 0 {
            let Ok(res) = self.pool.results.try_recv() else { break };
            budget -= 1;
            self.pool.in_flight = self.pool.in_flight.saturating_sub(1);
            match res {
                JobResult::Mesh { key, version, data } => {
                    if self.versions.get(&key) == Some(&version) && self.world.columns.contains_key(&key) {
                        self.renderer.upload_mesh(key, self.world.min_y, &data);
                    }
                }
                JobResult::Generated { key, column } => {
                    if let Mode::Single(sp) = &mut self.mode {
                        sp.pending.remove(&key);
                        let (pcx, pcz) = ((self.player.pos.x.floor() as i32) >> 4, (self.player.pos.z.floor() as i32) >> 4);
                        let r = self.args.render_distance + 2;
                        if (key.0 - pcx).abs() <= r && (key.1 - pcz).abs() <= r {
                            self.world.insert(key.0, key.1, column);
                        }
                    }
                }
            }
        }
    }

    fn schedule_meshes(&mut self) {
        let cap = self.pool.threads * 3;
        if self.pool.in_flight >= cap || self.world.dirty.is_empty() {
            return;
        }
        let (pcx, pcz) = self.chunk_pos();
        let mut dirty: Vec<(i32, i32)> = self.world.dirty.iter().copied().collect();
        dirty.sort_by_key(|(x, z)| (x - pcx) * (x - pcx) + (z - pcz) * (z - pcz));
        for key in dirty {
            if self.pool.in_flight >= cap {
                break;
            }
            self.world.dirty.remove(&key);
            let Some(center) = self.world.columns.get(&key).cloned() else { continue };
            let n = |dx: i32, dz: i32| self.world.columns.get(&(key.0 + dx, key.1 + dz)).cloned();
            let neighbours = [n(-1, 0), n(1, 0), n(0, -1), n(0, 1)];
            let version = self.next_version;
            self.next_version += 1;
            self.versions.insert(key, version);
            self.pool.submit(Job::Mesh { key, version, center, neighbours });
        }
    }

    fn unload(&mut self, key: (i32, i32)) {
        if let Some(col) = self.world.remove(key.0, key.1) {
            if let Mode::Single(sp) = &self.mode {
                if self.world.modified.remove(&key) {
                    if let Err(e) = sp.save.save_column(key.0, key.1, &col, &self.world.registry) {
                        eprintln!("save failed: {e}");
                    }
                }
            }
        }
        self.renderer.remove_mesh(key);
        self.versions.remove(&key);
    }

    fn update_singleplayer(&mut self) {
        let (pcx, pcz) = self.chunk_pos();
        let r = self.args.render_distance;
        let Mode::Single(sp) = &mut self.mode else { return };
        // Load columns in rings around the player.
        let cap = self.pool.threads * 3;
        'outer: for ring in 0..=r {
            for dz in -ring..=ring {
                for dx in -ring..=ring {
                    if dx.abs() != ring && dz.abs() != ring {
                        continue;
                    }
                    if dx * dx + dz * dz > r * r + 2 {
                        continue;
                    }
                    if self.pool.in_flight >= cap {
                        break 'outer;
                    }
                    let key = (pcx + dx, pcz + dz);
                    if self.world.columns.contains_key(&key) || sp.pending.contains(&key) {
                        continue;
                    }
                    if let Some(mut col) = sp.save.load_column(key.0, key.1, &self.world.registry) {
                        col.compute_skylight(&self.world.registry);
                        self.world.insert(key.0, key.1, col);
                        continue;
                    }
                    sp.pending.insert(key);
                    self.pool.submit(Job::Generate { key });
                }
            }
        }
        // Unload far columns.
        let far: Vec<(i32, i32)> =
            self.world.columns.keys().copied().filter(|(x, z)| (x - pcx).abs() > r + 3 || (z - pcz).abs() > r + 3).collect();
        for k in far {
            self.unload(k);
        }
        if self.last_save.elapsed().as_secs() >= 30 {
            self.save_all();
        }
    }

    fn save_all(&mut self) {
        self.last_save = Instant::now();
        let Mode::Single(sp) = &self.mode else { return };
        for key in self.world.modified.drain().collect::<Vec<_>>() {
            if let Some(col) = self.world.columns.get(&key) {
                if let Err(e) = sp.save.save_column(key.0, key.1, col, &self.world.registry) {
                    eprintln!("save failed: {e}");
                }
            }
        }
        let p = &self.player;
        sp.save.save_player(p.pos.to_array(), p.yaw, p.pitch, p.flying);
    }

    fn update_multiplayer(&mut self) {
        let Mode::Multi(mp) = &mut self.mode else { return };
        let mut events = vec![];
        while let Ok(e) = mp.net.events.try_recv() {
            events.push(e);
            if events.len() > 512 {
                break;
            }
        }
        for e in events {
            match e {
                NetEvent::Info(s) => println!("[net] {s}"),
                NetEvent::Chat(s) => println!("{s}"),
                NetEvent::Joined { dimension, min_y, sections, gamemode }
                | NetEvent::Respawned { dimension, min_y, sections, gamemode } => {
                    let Mode::Multi(mp) = &mut self.mode else { return };
                    let changed = !mp.joined || dimension != mp.dimension || self.world.min_y != min_y || self.world.sections != sections;
                    mp.joined = true;
                    mp.gamemode = gamemode;
                    mp.dimension = dimension.clone();
                    self.player.can_fly = gamemode == 1 || gamemode == 3;
                    if changed {
                        println!("Entered {dimension}");
                        self.world.clear();
                        self.renderer.clear_meshes();
                        self.versions.clear();
                        self.world.min_y = min_y;
                        self.world.sections = sections;
                    }
                }
                NetEvent::Chunk { x, z, column } => {
                    if column.sections.len() == self.world.sections {
                        self.world.insert(x, z, *column);
                    }
                }
                NetEvent::Unload { x, z } => self.unload((x, z)),
                NetEvent::Block { x, y, z, state } => {
                    self.world.set(x, y, z, state, false);
                }
                NetEvent::Light { x, z, light } => {
                    if let Some(col) = self.world.columns.get_mut(&(x, z)) {
                        net::chunk::apply_light(Arc::make_mut(col), &light);
                        self.world.dirty.insert((x, z));
                    }
                }
                NetEvent::Teleport { x, y, z, yaw, pitch, flags } => {
                    let p = &mut self.player;
                    p.pos.x = if flags & 1 != 0 { p.pos.x + x } else { x };
                    p.pos.y = if flags & 2 != 0 { p.pos.y + y } else { y };
                    p.pos.z = if flags & 4 != 0 { p.pos.z + z } else { z };
                    p.yaw = if flags & 8 != 0 { p.yaw + yaw.to_radians() } else { yaw.to_radians() };
                    p.pitch = if flags & 16 != 0 { p.pitch + pitch.to_radians() } else { pitch.to_radians() };
                    p.vel = DVec3::ZERO;
                    let Mode::Multi(mp) = &mut self.mode else { return };
                    mp.net.send(net::out::position(
                        p.pos.x,
                        p.pos.y,
                        p.pos.z,
                        p.yaw.to_degrees(),
                        p.pitch.to_degrees(),
                        p.on_ground,
                        false,
                    ));
                    if !mp.loaded_sent {
                        mp.loaded_sent = true;
                        mp.net.send(net::out::player_loaded());
                    }
                }
                NetEvent::GameMode(g) => {
                    let Mode::Multi(mp) = &mut self.mode else { return };
                    mp.gamemode = g;
                    println!("Game mode is now {}", ["survival", "creative", "adventure", "spectator"].get(g as usize).unwrap_or(&"?"));
                }
                NetEvent::Abilities { flying, allow_flying, .. } => {
                    self.player.can_fly = allow_flying;
                    self.player.flying = flying && allow_flying;
                }
                NetEvent::Health(h) => {
                    if h <= 0.0 {
                        println!("You died. Respawning...");
                        let Mode::Multi(mp) = &mut self.mode else { return };
                        mp.net.send(net::out::respawn());
                    }
                }
                NetEvent::HeldSlot(s) => self.slot = s as usize,
                NetEvent::Disconnected(reason) => {
                    println!("Disconnected: {reason}");
                    std::process::exit(0);
                }
            }
        }

        // Movement updates at 20 per second.
        let Mode::Multi(mp) = &mut self.mode else { return };
        if mp.net.playing() && mp.loaded_sent && mp.last_send.elapsed().as_millis() >= 50 {
            mp.last_send = Instant::now();
            let p = &self.player;
            mp.net.send(net::out::position(
                p.pos.x,
                p.pos.y,
                p.pos.z,
                p.yaw.to_degrees(),
                p.pitch.to_degrees(),
                p.on_ground,
                p.horizontal_collision,
            ));
            mp.net.send(net::out::tick_end());
        }
    }

    fn handle_console(&mut self) {
        while let Ok(line) = self.console.try_recv() {
            if let Mode::Multi(mp) = &self.mode {
                mp.net.send(net::out::chat(&line));
                continue;
            }
            {
                {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    match parts.as_slice() {
                        ["/tp", x, y, z] => {
                            if let (Ok(x), Ok(y), Ok(z)) = (x.parse(), y.parse(), z.parse()) {
                                self.player.pos = DVec3::new(x, y, z);
                                self.player.vel = DVec3::ZERO;
                            }
                        }
                        ["/save"] => {
                            self.save_all();
                            println!("Saved.");
                        }
                        ["/seed"] => println!("Seed: {}", self.args.seed),
                        ["/pos"] => println!("{:.1} {:.1} {:.1}", self.player.pos.x, self.player.pos.y, self.player.pos.z),
                        _ => println!("Commands: /tp x y z, /pos, /seed, /save"),
                    }
                }
            }
        }
    }

    fn interact(&mut self, dt: f32) {
        self.action_cooldown -= dt;
        let eye = self.player.eye();
        let cam = Camera { eye, yaw: self.player.yaw, pitch: self.player.pitch, fov_y: 0.0 };
        let dir = cam.forward();
        let Some(hit) = self.world.raycast(eye.as_vec3(), dir, 5.0) else {
            self.input.left_clicked = false;
            self.input.right_clicked = false;
            self.input.middle_clicked = false;
            return;
        };

        if self.input.middle_clicked {
            if matches!(self.mode, Mode::Single(_)) {
                if let Some(i) = self.hotbar.iter().position(|&b| b == hit.id) {
                    self.slot = i;
                } else {
                    self.hotbar[self.slot] = hit.id;
                }
            }
        }

        let want_break = self.input.left_clicked || (self.input.left && self.action_cooldown <= 0.0);
        let want_place = self.input.right_clicked || (self.input.right && self.action_cooldown <= 0.0);
        let b = hit.block;
        match &mut self.mode {
            Mode::Single(_) => {
                if want_break {
                    self.world.set(b.x, b.y, b.z, 0, true);
                    self.world.modified.insert((b.x >> 4, b.z >> 4));
                    self.action_cooldown = 0.22;
                } else if want_place {
                    let p = hit.before;
                    let id = self.hotbar[self.slot];
                    let solid = self.world.registry.get(id).collides();
                    let target = self.world.registry.get(self.world.get(p.x, p.y, p.z));
                    if (target.is_air() || target.kind == Kind::Water || target.kind == Kind::Cross)
                        && !(solid && self.player.intersects_block(p))
                    {
                        self.world.set(p.x, p.y, p.z, id, true);
                        self.world.modified.insert((p.x >> 4, p.z >> 4));
                    }
                    self.action_cooldown = 0.22;
                }
            }
            Mode::Multi(mp) => {
                if !mp.net.playing() {
                    return;
                }
                if want_break && self.input.left_clicked {
                    let seq = mp.net.next_sequence();
                    mp.net.send(net::out::dig(0, b.x, b.y, b.z, hit.face, seq));
                    if mp.gamemode == 1 {
                        // Creative: instant, predict it locally.
                        self.world.set(b.x, b.y, b.z, 0, false);
                    } else {
                        // Survival: the server finishes the break once enough time has passed.
                        let seq = mp.net.next_sequence();
                        mp.net.send(net::out::dig(2, b.x, b.y, b.z, hit.face, seq));
                        mp.dig_started = Some((b, Instant::now()));
                    }
                    mp.net.send(net::out::swing());
                    self.action_cooldown = 0.25;
                } else if want_break && mp.gamemode == 1 {
                    let seq = mp.net.next_sequence();
                    mp.net.send(net::out::dig(0, b.x, b.y, b.z, hit.face, seq));
                    self.world.set(b.x, b.y, b.z, 0, false);
                    mp.net.send(net::out::swing());
                    self.action_cooldown = 0.25;
                } else if want_place {
                    let seq = mp.net.next_sequence();
                    mp.net.send(net::out::use_item_on(b.x, b.y, b.z, hit.face, [0.5, 0.5, 0.5], seq));
                    mp.net.send(net::out::swing());
                    self.action_cooldown = 0.25;
                }
            }
        }
        self.input.left_clicked = false;
        self.input.right_clicked = false;
        self.input.middle_clicked = false;
    }

    fn hud(&self) -> Vec<HudVertex> {
        let (w, h) = (self.window.inner_size().width.max(1) as f32, self.window.inner_size().height.max(1) as f32);
        let scale = (h / 360.0).floor().max(1.0);
        let mut v: Vec<HudVertex> = Vec::with_capacity(512);
        let mut rect = |x: f32, y: f32, rw: f32, rh: f32, color: [f32; 4], uv: Option<[f32; 4]>| {
            // x, y in pixels from the top-left
            let x0 = x / w * 2.0 - 1.0;
            let x1 = (x + rw) / w * 2.0 - 1.0;
            let y0 = 1.0 - y / h * 2.0;
            let y1 = 1.0 - (y + rh) / h * 2.0;
            let [u0, v0, u1, v1] = uv.unwrap_or([-1.0, -1.0, -1.0, -1.0]);
            let p = |px: f32, py: f32, u: f32, vv: f32| HudVertex { pos: [px, py], uv: [u, vv], color };
            v.extend_from_slice(&[
                p(x0, y0, u0, v0),
                p(x1, y0, u1, v0),
                p(x1, y1, u1, v1),
                p(x0, y0, u0, v0),
                p(x1, y1, u1, v1),
                p(x0, y1, u0, v1),
            ]);
        };
        // Crosshair
        let (cx, cy) = (w / 2.0, h / 2.0);
        let (len, th) = (7.0 * scale, scale.max(1.0) * 1.0);
        rect(cx - len, cy - th / 2.0, len * 2.0, th, [1.0, 1.0, 1.0, 0.85], None);
        rect(cx - th / 2.0, cy - len, th, len * 2.0, [1.0, 1.0, 1.0, 0.85], None);
        // Hotbar
        let slot = 20.0 * scale;
        let total = slot * 9.0;
        let x0 = cx - total / 2.0;
        let y0 = h - slot - 4.0 * scale;
        rect(x0 - 2.0 * scale, y0 - 2.0 * scale, total + 4.0 * scale, slot + 4.0 * scale, [0.0, 0.0, 0.0, 0.45], None);
        for i in 0..9 {
            let sx = x0 + i as f32 * slot;
            if i == self.slot {
                rect(sx - scale, y0 - scale, slot + 2.0 * scale, slot + 2.0 * scale, [1.0, 1.0, 1.0, 0.9], None);
                rect(sx + scale, y0 + scale, slot - 2.0 * scale, slot - 2.0 * scale, [0.15, 0.15, 0.15, 0.9], None);
            }
            if let Mode::Single(_) = self.mode {
                let id = self.hotbar[i] as usize;
                if let Some(&tile) = self.tile_of_block.get(id) {
                    let pad = 3.0 * scale;
                    rect(sx + pad, y0 + pad, slot - pad * 2.0, slot - pad * 2.0, [1.0, 1.0, 1.0, 1.0], Some(self.renderer.tile_uv(tile)));
                }
            }
        }
        v
    }

    /// Draws the title screen and returns an action if a button was clicked this frame.
    fn render_title(&mut self) {
        // Keep draining network/world work so the world is ready the moment you press Play.
        self.handle_job_results();
        if matches!(self.mode, Mode::Single(_)) {
            self.update_singleplayer();
        } else {
            self.update_multiplayer();
        }
        self.schedule_meshes();
        let (w, h) = self.renderer.size();
        let canvas = self.title.build(w, h, self.cursor, self.started.elapsed().as_secs_f32());
        self.renderer.render_menu(&canvas.verts, [0.10, 0.12, 0.18]);
    }

    fn menu_click(&mut self, event_loop: &ActiveEventLoop) {
        let (w, h) = self.renderer.size();
        let canvas = self.title.build(w, h, self.cursor, self.started.elapsed().as_secs_f32());
        match canvas.hit(self.cursor.0, self.cursor.1) {
            Some(menu::Action::Play) | Some(menu::Action::JoinServer) => {
                self.screen = Screen::Playing;
                self.last_frame = Instant::now();
                self.grab(true);
            }
            Some(menu::Action::Quit) => {
                self.save_all();
                event_loop.exit();
            }
            None => {}
        }
    }

    fn frame(&mut self) {
        if self.screen == Screen::Title {
            self.render_title();
            return;
        }
        let now = Instant::now();
        let dt = (now - self.last_frame).as_secs_f32().min(0.25);
        self.last_frame = now;
        self.frames += 1;

        self.handle_job_results();
        if matches!(self.mode, Mode::Single(_)) {
            self.update_singleplayer();
        } else {
            self.update_multiplayer();
        }
        self.handle_console();
        self.schedule_meshes();

        // Look
        if self.input.grabbed {
            let sens = 0.0025;
            self.player.yaw += self.input.mouse_delta.0 as f32 * sens;
            self.player.pitch = (self.player.pitch + self.input.mouse_delta.1 as f32 * sens).clamp(-1.5607, 1.5607);
        }
        self.input.mouse_delta = (0.0, 0.0);
        if self.input.scroll != 0.0 {
            let s = (self.slot as i32 - self.input.scroll.signum() as i32).rem_euclid(9) as usize;
            self.select_slot(s);
            self.input.scroll = 0.0;
        }

        // Move (only once the ground under us has loaded)
        let k = |c: KeyCode| self.input.keys.contains(&c) && self.input.grabbed;
        let mv = MoveInput {
            forward: (k(KeyCode::KeyW) as i32 - k(KeyCode::KeyS) as i32) as f32,
            strafe: (k(KeyCode::KeyD) as i32 - k(KeyCode::KeyA) as i32) as f32,
            jump: k(KeyCode::Space),
            sneak: k(KeyCode::ShiftLeft),
            sprint: k(KeyCode::ControlLeft),
        };
        let ready = match &self.mode {
            Mode::Single(_) => true,
            Mode::Multi(mp) => mp.loaded_sent,
        };
        let (px, pz) = (self.player.pos.x.floor() as i32, self.player.pos.z.floor() as i32);
        if ready && self.world.is_loaded(px, pz) {
            self.player.update(&self.world, mv, dt as f64);
        }
        if self.player.pos.y < self.world.min_y as f64 - 64.0 {
            self.player.pos.y = (self.world.min_y + self.world.height()) as f64;
            self.player.vel = DVec3::ZERO;
        }

        self.interact(dt);

        // Render
        let eye = self.player.eye();
        let cam = Camera { eye, yaw: self.player.yaw, pitch: self.player.pitch, fov_y: self.args.fov.to_radians() };
        let outline = self.world.raycast(eye.as_vec3(), cam.forward(), 5.0).map(|h| h.block);
        let head = self.world.registry.get(self.world.get(eye.x.floor() as i32, eye.y.floor() as i32, eye.z.floor() as i32));
        let underwater = head.kind == Kind::Water;
        let dim = match &self.mode {
            Mode::Multi(mp) => mp.dimension.as_str(),
            _ => "",
        };
        let sky = if underwater {
            [0.05, 0.12, 0.35]
        } else if dim.contains("nether") {
            [0.2, 0.03, 0.03]
        } else if dim.contains("the_end") {
            [0.08, 0.06, 0.1]
        } else {
            [0.53, 0.71, 0.95]
        };
        let fog_end = if underwater { 24.0 } else { (self.args.render_distance as f32 - 0.5) * 16.0 };
        let hud = self.hud();
        let params = FrameParams { sky, fog_end, daylight: 1.0, outline, hud: &hud };
        self.renderer.render(&cam, self.world.height(), &params);

        if self.last_title.elapsed().as_secs_f32() >= 0.5 {
            self.fps = self.frames as f32 / self.last_title.elapsed().as_secs_f32();
            self.frames = 0;
            self.last_title = Instant::now();
            let p = self.player.pos;
            let block_name = &self.world.registry.get(self.hotbar[self.slot]).name;
            let mode = match &self.mode {
                Mode::Single(_) => format!("{} | holding {}", self.args.world, block_name),
                Mode::Multi(mp) => {
                    format!("{} | {}", self.args.connect.clone().unwrap_or_default(), if mp.joined { "online" } else { "connecting" })
                }
            };
            self.window.set_title(&format!(
                "Claudium | {:.0} fps | {:.1} {:.1} {:.1} | {} chunks ({} drawn){} | {}",
                self.fps,
                p.x,
                p.y,
                p.z,
                self.renderer.mesh_count(),
                self.renderer.drawn_last_frame,
                if self.player.flying { " | flying" } else { "" },
                mode
            ));
        }
    }
}

// ---------------------------------------------------------------------------

struct App {
    args: Args,
    game: Option<Game>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.game.is_some() {
            return;
        }
        match Game::new(event_loop, self.args.clone()) {
            Ok(g) => {
                // Start on the title screen with the cursor free; the mouse is grabbed on Play.
                self.game = Some(g);
            }
            Err(e) => {
                eprintln!("Could not start: {e:#}");
                event_loop.exit();
            }
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        let Some(g) = self.game.as_mut() else { return };
        // Title screen: a small, self-contained event handler that never feeds game input.
        if g.screen == Screen::Title {
            match event {
                WindowEvent::CloseRequested => {
                    g.save_all();
                    event_loop.exit();
                }
                WindowEvent::Resized(size) => g.renderer.resize(size.width, size.height),
                WindowEvent::CursorMoved { position, .. } => g.cursor = (position.x as f32, position.y as f32),
                WindowEvent::MouseInput { state: ElementState::Pressed, button: MouseButton::Left, .. } => g.menu_click(event_loop),
                WindowEvent::KeyboardInput { event, .. } => {
                    if event.state == ElementState::Pressed && !event.repeat {
                        if event.physical_key == PhysicalKey::Code(KeyCode::Escape) {
                            g.save_all();
                            event_loop.exit();
                        } else if matches!(event.physical_key, PhysicalKey::Code(KeyCode::Enter) | PhysicalKey::Code(KeyCode::NumpadEnter))
                        {
                            // Enter activates the first (primary) button.
                            if let Some(&(_, action)) = g.title.entries.first() {
                                match action {
                                    menu::Action::Play | menu::Action::JoinServer => {
                                        g.screen = Screen::Playing;
                                        g.last_frame = Instant::now();
                                        g.grab(true);
                                    }
                                    menu::Action::Quit => event_loop.exit(),
                                }
                            }
                        }
                    }
                }
                WindowEvent::RedrawRequested => g.frame(),
                _ => {}
            }
            return;
        }
        match event {
            WindowEvent::CloseRequested => {
                g.save_all();
                event_loop.exit();
            }
            WindowEvent::Resized(size) => g.renderer.resize(size.width, size.height),
            WindowEvent::Focused(false) => g.grab(false),
            WindowEvent::KeyboardInput { event, .. } => {
                let PhysicalKey::Code(code) = event.physical_key else { return };
                let down = event.state == ElementState::Pressed;
                if down {
                    g.input.keys.insert(code);
                } else {
                    g.input.keys.remove(&code);
                }
                if down && !event.repeat {
                    match code {
                        KeyCode::Escape => g.grab(false),
                        KeyCode::KeyF if g.player.can_fly => {
                            g.player.flying = !g.player.flying;
                            if let Mode::Multi(mp) = &g.mode {
                                mp.net.send(net::out::abilities(g.player.flying));
                            }
                        }
                        KeyCode::Digit1 => g.select_slot(0),
                        KeyCode::Digit2 => g.select_slot(1),
                        KeyCode::Digit3 => g.select_slot(2),
                        KeyCode::Digit4 => g.select_slot(3),
                        KeyCode::Digit5 => g.select_slot(4),
                        KeyCode::Digit6 => g.select_slot(5),
                        KeyCode::Digit7 => g.select_slot(6),
                        KeyCode::Digit8 => g.select_slot(7),
                        KeyCode::Digit9 => g.select_slot(8),
                        _ => {}
                    }
                }
            }
            WindowEvent::MouseInput { state, button, .. } => {
                let down = state == ElementState::Pressed;
                if !g.input.grabbed {
                    if down {
                        g.grab(true);
                    }
                    return;
                }
                match button {
                    MouseButton::Left => {
                        g.input.left = down;
                        g.input.left_clicked |= down;
                    }
                    MouseButton::Right => {
                        g.input.right = down;
                        g.input.right_clicked |= down;
                    }
                    MouseButton::Middle => g.input.middle_clicked |= down,
                    _ => {}
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                if g.input.grabbed {
                    g.input.scroll += match delta {
                        MouseScrollDelta::LineDelta(_, y) => y,
                        MouseScrollDelta::PixelDelta(p) => (p.y / 40.0) as f32,
                    };
                }
            }
            WindowEvent::RedrawRequested => g.frame(),
            _ => {}
        }
    }

    fn device_event(&mut self, _el: &ActiveEventLoop, _id: DeviceId, event: DeviceEvent) {
        if let (Some(g), DeviceEvent::MouseMotion { delta }) = (self.game.as_mut(), event) {
            if g.input.grabbed {
                g.input.mouse_delta.0 += delta.0;
                g.input.mouse_delta.1 += delta.1;
            }
        }
    }

    fn about_to_wait(&mut self, _el: &ActiveEventLoop) {
        if let Some(g) = &self.game {
            g.window.request_redraw();
        }
    }
}

fn main() {
    let args = parse_args();

    if args.logout {
        let _ = std::fs::remove_file(data_dir().join("auth.json"));
        println!("Signed out.");
        return;
    }
    if let Some(addr) = &args.ping {
        match net::ping(addr) {
            Ok(s) => {
                println!("{addr}: {} (protocol {}), {}/{} players, {} ms", s.version, s.protocol, s.online, s.max, s.latency_ms);
                println!("{}", s.motd);
            }
            Err(e) => {
                eprintln!("ping failed: {e:#}");
                std::process::exit(1);
            }
        }
        return;
    }

    let event_loop = EventLoop::new().expect("event loop");
    event_loop.set_control_flow(ControlFlow::Poll);
    let mut app = App { args, game: None };
    if let Err(e) = event_loop.run_app(&mut app) {
        eprintln!("{e}");
    }
}
