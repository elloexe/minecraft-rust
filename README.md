# Claudium

A block-game client written from scratch in Rust. Play a local world, or join
Java Edition servers (protocol 26.1.2). Claudium contains no game code and ships
no game art: it reads textures from **your own installed copy** of the game at
runtime, and falls back to its own procedurally drawn textures if none is found.

## Downloads

Every push is compiled by GitHub Actions for Windows, macOS and Linux. Grab a
build from the **Actions** tab → latest run → Artifacts (`claudium.exe` for
Windows). No Rust toolchain needed to just play.

## Build and run

You need Rust 1.85 or newer (`rustup update`).

```sh
cargo build --release
./target/release/claudium                       # title screen -> local world
./target/release/claudium --connect localhost   # title screen -> join a server (offline mode)
./target/release/claudium --ping mc.example.com # server status, no window
./target/release/claudium --help                # all options
```

Claudium opens on its **title screen** (its own "Rust Edition" home screen).
Click **Singleplayer** (or **Join**, when launched with `--connect`) to enter,
or press Enter. Press Esc in-game to free the mouse.

On Linux you may need `libxkbcommon-dev`, `libwayland-dev` and `libxcb` dev packages.

## Using your game's textures

Claudium looks for `~/.minecraft/versions/26.1.2/26.1.2.jar` (or `%APPDATA%\.minecraft`
on Windows, `~/Library/Application Support/minecraft` on macOS). Point it somewhere
else with `--assets path/to/game.jar` or a resource-pack `.zip`.

## Joining servers

Servers identify blocks by numeric state ids that change every release. To show
them correctly, Claudium needs `blocks.json` from Mojang's official data generator,
run on the server jar (free download from minecraft.net):

```sh
java -DbundlerMainClass=net.minecraft.data.Main -jar server.jar --reports
./target/release/claudium --connect myserver.net --blocks generated/reports/blocks.json
```

Or copy it to `~/.claudium/reports/26.1.2/blocks.json` once. Without it, terrain
still loads with the right shape but every block type gets a flat colour.

**Offline-mode / LAN servers** work with just `--name YourName`.

**Online-mode servers** need a Microsoft account sign-in:

```sh
CLAUDIUM_CLIENT_ID=<your-azure-app-id> ./target/release/claudium --connect play.example.net --login
```

Mojang requires third-party clients to use their own Azure app registration that has
been approved for the game services API, so you need to register one (Azure portal →
App registrations → personal Microsoft accounts, enable public client flows) and
request access. The sign-in token is cached in `~/.claudium/auth.json`;
`--logout` removes it.

Chat: type into the terminal Claudium was started from. Lines starting with `/`
are sent as commands. Incoming chat prints there too.

## Controls

| Key | Action |
|---|---|
| WASD | move |
| Space / Shift | jump, sneak (up/down while flying) |
| Ctrl | sprint |
| F | toggle flying (local world, or when the server allows it) |
| Left / right click | break / place |
| Middle click | pick block (local world) |
| 1-9, mouse wheel | hotbar |
| Esc | release the mouse |

## How it's built

| File | What it does |
|---|---|
| `src/render.rs`, `src/shaders.wgsl` | wgpu renderer: camera-relative coordinates, frustum culling, front-to-back opaque pass, sorted water pass |
| `src/mesher.rs` | turns chunk columns into 8-byte packed vertices with face culling, ambient occlusion and light, on worker threads |
| `src/world.rs` | chunk storage (sections are skipped when empty), skylight, ray casting |
| `src/worldgen.rs` | local-world terrain: continents, mountains, caves, ores, trees |
| `src/net/` | protocol client: framing, zlib compression, AES/CFB-8 encryption, login, configuration, play, chunk and light decoding, Microsoft sign-in |
| `src/registry.rs`, `src/assets.rs` | block appearance rules, texture loading, atlas building |

Measured on a 2-core cloud VM (release build): generating a column takes about 2.4 ms
and meshing it about 0.7 ms, so a 12-chunk view loads in well under a second per core.

## Status

Works: original title screen ("Rust Edition" home screen), rendering, local
worlds with saving, walking/flying physics, building,
server status ping, offline and online login, encryption and compression, chunk
streaming, light, block updates, teleports, keep-alive, chat, creative building,
survival mining (click once, the block breaks after its normal time).

Not yet: other players, mobs and items (entities aren't drawn), inventory and
crafting screens, in-game chat box, sounds, non-cube shapes beyond slabs, carpets
and snow layers (stairs, fences and doors render as cubes), biome colours,
day/night, server resource packs, SRV DNS records, and versions other than 26.1.2.

## Running the tests

```sh
cargo test
```
