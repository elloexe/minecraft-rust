<div align="center">

# Claudium

**An open-source Minecraft client, written from scratch in Rust.**

Play a local world or join **Minecraft: Java Edition** servers — with a native
voxel engine, threaded world generation, and a GPU renderer built on `wgpu`.

[![build](https://github.com/elloexe/minecraft-rust/actions/workflows/ci.yml/badge.svg)](https://github.com/elloexe/minecraft-rust/actions/workflows/ci.yml)
![rust](https://img.shields.io/badge/rust-1.85%2B-orange)
![license](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue)
![versions](https://img.shields.io/badge/Minecraft-26.1.2%20–%2026.3-brightgreen)

</div>

---

Claudium is a clean-room **reimplementation of Minecraft** in Rust. It ships **no
Minecraft code and no Minecraft art** — it reads textures and the title logo from
*your own installed copy of the game* at runtime, and draws its own procedural
textures when it can't find them. It talks to vanilla servers using the publicly
documented Java Edition network protocol.

> ⚠️ **Not affiliated with, endorsed by, or associated with Mojang or Microsoft.**
> "Minecraft" is a trademark of Mojang Synergies AB. You must own a copy of the
> game to use its assets and to sign in to online-mode servers.

## ✨ Features

- 🌍 **Singleplayer** — original terrain generation (continents, mountains, caves,
  ores, biomes, trees), block breaking and placing, flying, and world saving.
- 🛰️ **Multiplayer** — join Java Edition servers: full login, zlib compression,
  AES encryption, chunk & lighting streaming, block updates, chat, and
  Microsoft-account sign-in for online-mode servers.
- 🎨 **Your textures** — loads block textures and the title logo straight from your
  Minecraft install (or any resource pack), with procedural fallbacks.
- 🏠 **Title screen** — a familiar home screen that shows the game's own logo over a
  **"Rust Edition"** line.
- ⚡ **Fast** — camera-relative rendering, frustum culling, ambient occlusion, and
  chunk meshing spread across worker threads. ~2.4 ms to generate a chunk and
  ~0.7 ms to mesh it, per core.
- 🧩 **Versions** — targets the **Minecraft 26.x** line (26.1.2 – 26.3); the client
  adopts the server's advertised protocol automatically.

## 📦 Download

Every push is built for **Windows, macOS and Linux** by GitHub Actions — no Rust
toolchain needed to just play:

**[→ Actions tab](https://github.com/elloexe/minecraft-rust/actions)** → latest run → **Artifacts** → `claudium.exe`

## 🔨 Build from source

Requires Rust 1.85+ (`rustup update`).

```sh
cargo build --release
./target/release/claudium                       # title screen → local world
./target/release/claudium --connect localhost   # title screen → join a server
./target/release/claudium --ping mc.example.com # server status, no window
./target/release/claudium --help                # all options
```

<details>
<summary>Linux build dependencies</summary>

```sh
sudo apt-get install -y libxkbcommon-dev libwayland-dev libxcb1-dev \
  libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev
```
</details>

## 🎨 Using your Minecraft textures

Claudium looks for your install automatically:

| OS | Location |
|----|----------|
| Windows | `%APPDATA%\.minecraft\versions\<ver>\<ver>.jar` |
| Linux | `~/.minecraft/versions/<ver>/<ver>.jar` |
| macOS | `~/Library/Application Support/minecraft/versions/<ver>/<ver>.jar` |

Pick a version with `--version 26.3`, or point at any jar / resource-pack zip with
`--assets path/to/file.zip`. Nothing from the game is copied into Claudium — it is
read from your machine at runtime, the same way a resource pack is.

## 🛰️ Joining servers

**Offline-mode / LAN servers** need only a name:

```sh
claudium --connect 192.168.1.10 --name Steve
```

**Online-mode servers** need a Microsoft sign-in:

```sh
CLAUDIUM_CLIENT_ID=<azure-app-id> claudium --connect play.example.net --login
```

Sign-in uses an Azure application id (`--client-id` or `CLAUDIUM_CLIENT_ID`); the
refresh token is cached in `~/.claudium/auth.json` and cleared with `--logout`.

To show server blocks with the correct textures, generate `blocks.json` once with
Mojang's official data generator and pass `--blocks`:

```sh
java -DbundlerMainClass=net.minecraft.data.Main -jar server.jar --reports
claudium --connect myserver.net --blocks generated/reports/blocks.json
```

Without it, terrain still loads with the right shape, but block types show as
flat colours.

## 🎮 Controls

| Input | Action |
|-------|--------|
| `WASD` | move |
| `Space` / `Shift` | jump / sneak — up / down while flying |
| `Ctrl` | sprint |
| `F` | toggle flying (local world, or when the server allows it) |
| Left / right click | break / place |
| Middle click | pick block (local world) |
| `1`–`9`, scroll | select hotbar slot |
| `Esc` | release the mouse |
| terminal | type to chat; lines starting with `/` are commands |

## 🧱 How it works

| Module | Responsibility |
|--------|----------------|
| `render.rs`, `shaders.wgsl` | `wgpu` renderer: camera-relative coords, frustum culling, opaque + water passes |
| `mesher.rs` | chunk → packed 8-byte vertices, face culling, ambient occlusion, on worker threads |
| `world.rs` | chunk storage, skylight, ray casting |
| `worldgen.rs` | singleplayer terrain, caves, ores, trees |
| `net/` | protocol client: framing, compression, encryption, login/config/play, chunk + light decode, Microsoft sign-in |
| `registry.rs`, `assets.rs` | block appearance rules, texture + logo loading, atlas building |
| `menu.rs` | title screen and its built-in pixel font |

## ✅ Status

**Working:** title screen, rendering, local worlds with saving, walking/flying
physics, building, server status ping, offline & online login, encryption and
compression, chunk streaming, lighting, block updates, teleports, keep-alive,
chat, creative building, survival mining.

**Not yet:** other players, mobs and items (entities aren't drawn), inventory and
crafting screens, in-game chat box, sounds, non-cube block shapes beyond slabs /
carpets / snow, biome tint from server data, day/night, server resource packs.

## 🧪 Tests

```sh
cargo test
```

Covers protocol encoding, chunk decoding across layout variants, world
generation, meshing, lighting, ray casting and the menu.

## 📄 License

Dual-licensed under **MIT** or **Apache-2.0**, at your option.

Claudium is an independent, fan-made project. It is **not affiliated with Mojang
or Microsoft**, contains none of their code or assets, and requires you to own
Minecraft to use its files.
