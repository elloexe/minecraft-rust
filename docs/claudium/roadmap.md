# Where Claudium can actually beat Horizon

Findings from reading the inherited codebase, 2026-10-31. The point of this page
is to stop us re-deriving the same conclusions: what is already done upstream,
what is genuinely open, and what each open thing is blocked on.

## Porting more Java mods

Upstream is much further along here than it looks from outside.
`crates/horizon-port-pack` is a 9,357-line Java→Horizon pipeline, designed
around All the Mods 10 (495 jars) and documented in [docs/modpacks.md](../modpacks.md).

What already works, measured upstream on 2026-10-22 over ATM10's 42 data-tier
mods: 687 structures defined, 190,550 block states and 12,931 items loading,
1,560 structure processors carried.

What does not:

| Gap | Size | Blocked on |
|---|---|---|
| **Code mods** | 333 of 496 ATM10 mods | A native port per mod. This is the real wall: machines, magic, mobs, tools and storage are the pack's gameplay. |
| **Dropped processors** | 605 | Each processor type written in Java has to be reimplemented as data the game runs. |
| **Structures that never generate** | 27 types | Their type is written in Java (Better Mineshafts, Repurposed Structures' mansions and monuments). |

The dropped processors concentrate in two families, and both have a catch:

- **Formations (227).** Its block-ageing and biome-swap tables are All Rights
  Reserved. Not portable into an MIT repository.
- **Repurposed Structures (182).** Its mansion and monument layout code is
  ~3,000 lines of LGPL. Upstream has this open as a licence question.
- **YUNG's mods (182** across Better Dungeons, Jungle Temples, Strongholds,
  Desert Temples, Fortresses, Witch Huts**).** The most promising target. There
  is already a partial carry in `crates/horizon-port-pack/src/yung.rs`.

Porting YUNG's processors needs a clean-room specification of what each
processor does, written from observing the mod rather than reading its source.
Upstream keeps those specs in a `cleanroom/` folder that is **not committed**
(the repo refuses decompiled and derived source — `tools/check_public_tree.py`),
so `spec-yungsapi.md`, referenced from `yung.rs`, is not in this copy.

**To unblock:** a machine with the mod jars and a Minecraft 26.3 install, plus
the clean-room spec step. None of that can happen in a cloud container with no
game files.

## A mod translator built into Claudium

This already exists twice over, which is worth knowing before building a third:

1. **Upstream's own pipeline** is exactly this idea, and is further along than a
   fresh attempt would get: import → inventory → classify → dependency order →
   carry-over or per-mod port → per-mod harness against the Java mod running in
   a dev-only NeoForge instance → coverage report. The launcher's **Import
   modpack** page is its front end.
2. **`Ropesehypnotize/smelt-minecraft-mods-to-rust-converter`** is a third-party
   tool with the same goal: "drop a .jar, get a Rust mod back".

The honest limit, which neither tool escapes: **data and assets translate
mechanically; code does not.** A Fabric or NeoForge jar is JVM bytecode calling
Minecraft's Java classes, and Horizon has none of those classes. Everything
mechanical — models, textures, recipes, loot tables, structures, worldgen JSON,
block and item registration — is already carried. What is left is behaviour, and
translating behaviour means understanding it. Upstream's answer is one AI agent
per mod with a differential harness proving the port matches, not a decompiler.

So the useful thing to build is not a translator. It is **more carry coverage**
(processor types, structure types) and **a wider native hook API**, so that each
agent port is smaller.

## Performance

I went looking for a win here and did not find an honest one. What I checked:

| Lever | State |
|---|---|
| `PackedStorage` index by multiply-and-shift instead of division | **Done** — `MAGIC` table and `locate()` in `crates/horizon-core/src/storage.rs` |
| `BlockSection::get`/`set` palette probes | **Done** — one probe, bounds checks hoisted |
| `PalettedContainer::unpack` per-cell dispatch | **Done** — hoisted out of the loop |
| Lighting worker result packaging | **Done** — 1.23–1.44x, `docs/performance/lighting-worker-results.md` |
| `light_chunk` flood fill | Already packs cells to avoid the large table and bounds work by highest occupied section |
| Release profile | Already `lto = "thin"`, `codegen-units = 1` |

Upstream measures with `tools/perf_bench.py` against a real vanilla 26.3 client
on the same machine, and every optimisation ships with before/after numbers.
Their microbenchmark convention is an `#[ignore]`d test inside the crate run
with `--profile fast`, comparing baseline and optimised with `black_box` and
medians — that part does run headless and is the right pattern for us.

**What is actually still open** (upstream's own roadmap):

- Far terrain at 48–64 chunks.
- CPU-bound block entity fields.
- The first seconds of a fresh world deliver no chunk.
- Fluids remesh for about a minute after loading.
- Server tick with 25–50 players near spawn, where vanilla is still ahead.

The last two are the best targets: both are described as unprofiled, and both
sound like bugs rather than tuning. Both need the game running to find.

**To unblock:** a machine with a GPU and a Minecraft 26.3 install. This
container has neither — no Vulkan driver is installable here, so GPUI and wgpu
cannot open a window, and `horizon-prepare` has no game files to prepare from.

## Native mods

The hook API is already at **API 20 with 34 hook points**
(`crates/horizon-client/src/game/native_hooks.rs`, registered in
`native-hooks.json`): frame, tick, world lifecycle, server tick, block and
neighbour changes, HUD, keys, commands, menus, screens, tooltips, item use,
block place and destroy, block entities with containers and menus.

Two things to know before planning mod work:

- Horizon's own mods (Zoom, FPS counter, Keystrokes, Minimap) live in a
  **separate repository**, `horizon-rust/horizon-mods`, under `native/<name>/`.
  They are not in this copy.
- The launcher builds a native mod **against Horizon's source fetched from
  GitHub at a pinned revision**, never a local checkout. So a native mod built
  today gets upstream's game, not ours. Shipping our own native mods means
  teaching `launcher/src/native_mods.rs` to build from our source.

## The repository going private

Two consequences worth deciding on before flipping the switch:

- **Release downloads break.** A private repository's GitHub releases are not
  publicly downloadable. `backend::REPO` is what the launcher fetches game
  builds from; if it ever points here, the launcher needs a token it cannot
  ship. Whatever hosts the game builds has to stay reachable.
- **Native mod builds break** for the same reason, if we ever point them at our
  source.

MIT itself is no obstacle: it permits private modified copies. `LICENSE` and
`NOTICE.md` simply have to stay in the tree.
