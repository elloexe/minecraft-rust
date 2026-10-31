<div align="center">

# Claudium

**Minecraft: Java Edition, in Rust — built on [Claudium](https://github.com/elloexe/minecraft-rust).**

![rust](https://img.shields.io/badge/rust-1.85%2B-orange)
![license](https://img.shields.io/badge/license-MIT-blue)
![minecraft](https://img.shields.io/badge/Minecraft-Java%2026.3-5D9C3F)

</div>

---

Claudium is a native Rust Minecraft client: it reads vanilla worlds, follows
vanilla's rules and generates vanilla's terrain, with a `wgpu` renderer and no
Java in the loop.

It is a **derivative of Claudium**, an independent Rust reimplementation of
Minecraft: Java Edition 26.3 that is checked against the real game by more than
290 differential test harnesses. Claudium is MIT licensed and did the hard part;
Claudium is where we modify it. See [NOTICE.md](NOTICE.md) for the attribution
and [docs/claudium/changes.md](docs/claudium/changes.md) for what we have changed.

> ⚠️ Not an official Minecraft product. Not approved by or associated with Mojang
> or Microsoft. Claudium contains no Minecraft code, assets or data — you need
> your own legally obtained copy of Minecraft: Java Edition 26.3.

## What you get from the base

- **Singleplayer** with vanilla world generation, mobs, villagers, raids,
  redstone, every command, the HUD and the recipe book, on saves in vanilla's
  own format. Worlds from 1.8 onward are upgraded as vanilla does.
- **Distant terrain** — LOD rendering at 64, 128, 256, 512 or **1024 chunks**,
  with a separate full-detail radius for nearby chunks.
- **Multiplayer** — join servers, including online-mode ones; Open to LAN;
  invite friends with a code.
- **Resource packs and data packs**, including data-driven world generation.
- **Mods** — content mods add blocks and items; native mods change the game's own
  Rust source before it is compiled.

## Build

You need Rust (stable), Python 3.10+, and on Linux the ALSA and udev headers:

```sh
sudo apt-get install -y libasound2-dev libudev-dev
```

The game never downloads Minecraft. It prepares its resources once from your own
copy, then runs:

```sh
cargo run --release -p claudium-prepare -- --prepare --server-jar ~/Downloads/server.jar
cargo run --release -p claudium-client
```

The launcher is a separate Cargo workspace:

```sh
cd launcher && cargo run
```

## Layout

30 crates; the ones you will touch most:

| Crate | Owns |
|---|---|
| `claudium-client` | The game: gameplay, networking, screens, and the dedicated server |
| `claudium-client-render` | The `wgpu` renderer, meshing, LOD / distant terrain |
| `claudium-world` | Chunks, world generation, lighting |
| `claudium-items` | Blocks and items beneath gameplay |
| `claudium-protocol` | The Java Edition network protocol |
| `claudium-command` | Command parsing (Brigadier) |

`docs/architecture.md` maps features to files. `tools/` holds the parity
harnesses that check behaviour against the real game.

## Contributing to this repository

Read [CLAUDE.md](CLAUDE.md) first — it covers the one rule that matters here
(vanilla is the specification) and how we track our changes against upstream.

Upstream's own contributor docs are kept in [docs/upstream/](docs/upstream/) for
reference. They describe *Claudium's* issue tracker and review process, not ours.

## License

MIT, inherited from Claudium. [LICENSE](LICENSE) keeps their copyright notice, as
the licence requires; our changes are MIT on the same terms.
