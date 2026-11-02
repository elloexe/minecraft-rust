# Claudium's built-in mods

The point of Claudium is lots of useful mods, in Rust, inside the game: nothing to install,
no per-player build, and they ship in the same EXE. This page is the list we are working
through, and where each one stands.

## Rules for every mod

- **Off by default.** A fresh install plays exactly like vanilla 26.3, and turning a mod off
  restores vanilla completely. That is what keeps the parity harnesses meaningful.
- **Our own code.** Each is our implementation of what a well-known mod *does*, written from
  how the feature behaves in game, never from the mod's source (most are All Rights Reserved
  or LGPL). Names describe the feature, not the original mod.
- **Switched on in the game**, under Options → Claudium Settings → Extras, grouped by
  category. Settings are stored in `claudium-extras.txt` (`builtinMods:` lists the switch-only
  mods that are on).
- **Client-side first.** Mods that change only what one player sees work on every server,
  vanilla ones included. Gameplay mods (marked *world*) change the rules, so they only apply
  in worlds the player hosts: singleplayer and LAN.

## Where the code lives

| Part | Code |
|---|---|
| Settings, the Extras page's rows, the switch-only list (`extras::Mod`) | `crates/claudium-client-render/src/extras.rs` |
| The mods in the game: zoom, minimap, HUD lines, keystrokes, alerts | `crates/claudium-client/src/game/builtin_mods.rs` |
| Hook-up (one line each: tick, HUD, zoom, minimap) | `game.rs`, `game/remote.rs`, `game/extras.rs` |

A native mod (docs/native-mods.md) that provides a zoom or a minimap still wins over the
built-in one.

## The list

**Done** is in the game now. **Inherited** came with Horizon already. **Planned** is the
queue, roughly in the order we expect to build it.

| # | Mod | Kind | Like | Status |
|---|---|---|---|---|
| 1 | Zoom (hold a key, smooth, slower turning) | client | OptiFine zoom, Zoomify | Done |
| 2 | FPS counter (with min and 1% low) | client | many HUD mods | Done |
| 3 | Keystrokes with clicks per second | client | Keystrokes | Done |
| 4 | Minimap (round or square, turning or north-up) | client | Xaero's Minimap, JourneyMap | Done |
| 5 | Coordinates and facing | client | InfoHUD, BetterF3 | Done |
| 6 | Biome display | client | InfoHUD | Done |
| 7 | Light level display (red at 0) | client | InfoHUD, Light Level Overlay | Done |
| 8 | Clock (day and time of day) | client | InfoHUD | Done |
| 9 | Speedometer | client | Speedometer mods | Done |
| 10 | Armor and held item durability HUD | client | Armor HUD, Durability Viewer | Done |
| 11 | Status effect timers | client | Effect timers | Done |
| 12 | Death coordinates in chat | client | Death Coordinates | Done |
| 13 | Low durability alert | client | Durability warnings | Done |
| 14 | Fullbright | client | Fullbright | Inherited |
| 15 | Dynamic lights | client | LambDynamicLights | Inherited |
| 16 | Mouse Tweaks-style inventory controls | client | Mouse Tweaks | Inherited |
| 17 | Inventory and chest sorting | client | Inventory Sorter | Inherited |
| 18 | Auto-reconnect | client | AutoReconnect | Inherited |
| 19 | Own name tag in third person | client | — | Inherited |
| 20 | Item and recipe viewer: search, recipes, uses, station tabs, fill the grid | client | JEI, REI, EMI | Done |
| 21 | Waypoints, with a death waypoint | client | Xaero's, JourneyMap | Planned |
| 22 | Saturation on the hunger bar, food preview, food values in tooltips | client | AppleSkin | Done |
| 23 | Shulker box contents in its tooltip | client | Shulker Box Tooltip | Planned: tooltip hook (API 12) |
| 24 | Exact durability in tooltips | client | Durability tooltips | Planned: tooltip hook |
| 25 | Enchantment descriptions in tooltips | client | Enchantment Descriptions | Planned: tooltip hook |
| 26 | Chat timestamps | client | Chat timestamps | Planned |
| 26b | Player faces before their chat messages | client | Chat Heads | Done |
| 27 | Keep chat history across worlds | client | Chat history mods | Planned |
| 28 | Item pickup notifications | client | Item pickup notifier | Planned |
| 29 | Ping and server TPS in the HUD | client | Ping display | Planned |
| 30 | Freelook (look around without turning) | client | Freelook, Perspective | Planned |
| 31 | Spawn-proofing light overlay | client | Light Level Overlay | Planned: needs a world overlay |
| 32 | Block outline colour and thickness | client | Block outline mods | Planned |
| 33 | Hotbar swap rows | client | Inventory Profiles | Planned |
| 34 | What you look at: block or mob, tool, mod, health | client | Jade, WTHIT | Done |
| 35 | Screenshot to clipboard | client | Screenshot tools | Planned |
| 36 | Vein mining: sneak-mine whole ore veins and trees, 64 blocks | world | VeinMiner | Done |
| 37 | Tree felling | world | TreeCapitator, FallingTree | Planned: block destroy hook |
| 38 | Graves for your items on death | world | Gravestones, Corpse | Planned |
| 39 | One player sleeping skips the night | world | One Player Sleep | Planned |
| 40 | Right-click to harvest and replant crops | world | Right Click Harvest | Planned: item use hook (API 13) |
| 41 | Auto-replant saplings and crops | world | Replanting mods | Planned |
| 42 | Fast leaf decay | world | Fast Leaf Decay | Planned |
| 43 | Double doors open together | world | Couplings, Quark | Planned |
| 44 | Sit on stairs and slabs | world | Quark, Sit | Planned |
| 45 | Waystones (teleport between placed stones) | world | Waystones | Planned: mod block entities (API 15+) |
| 46 | Backpacks | world | Sophisticated Backpacks | Planned: mod containers (API 16+) |
| 47 | Elevators (jump or sneak to move floors) | world | Elevator mods | Planned |
| 48 | Mob stacking for farms | world | Mob stacking mods | Planned |
| 49 | Villager trade cycling without breaking blocks | world | Easy Villagers | Planned |
| 50 | Silk Touch picks up spawners | world | Silk Spawners | Planned |

Counted honestly: 18 are new in Claudium, 6 came with Horizon, 27 are queued. Of the 17,
the four real mod ports are the item viewer (JEI), block info (Jade), food info (AppleSkin)
vein mining (VeinMiner) and chat heads (Chat Heads); the other 13 are smaller HUD and quality-of-life features.

## Claudium's own build starts with ten on

The game's defaults stay vanilla. The Windows build the launcher ships (`launcher/src/bundle.rs`,
`starter_mods`) switches ten on the first time it runs, only while `claudium-extras.txt` has no
`builtinMods` line (the game writes it the first time it saves), so a player's own choices are
never undone: item viewer, block info, food info, vein mining, chat heads, armor HUD, zoom, the
FPS counter, keystrokes and the minimap.
