# Changes on top of Horizon

Everything in this repository that is not upstream Horizon, newest first. Keep
this current: it is what tells us, and anyone reading the project, which parts
are ours.

## Unreleased

### CI

- Removed nine Python tests that asserted on upstream machinery this fork does
  not have: the issue-tracker command bots (`test_claim_command`,
  `test_triage_command`, `test_overlap_note`, `test_pr_labels`), the nightly
  release channel (`test_nightly`, `test_nightly_set`), the README parity table
  (`test_parity`, `test_parity_gaps`) and roadmap generation (`test_roadmap`).
  Their workflows, README section and ROADMAP were already removed or moved, so
  the tests failed on main's `verify` job. The ~430 kernel parity tests and the
  parity-harness runner tests are untouched. The orphaned tool scripts under
  `tools/ci/` and `tools/` stay for now (some are shared by `test_release_train`).

### Full debrand

- Removed every player-facing and in-code "Horizon": ~87,000 tokens across 2,495
  files, then every embedded and all-caps form, then 62 directory and file names.
  All 30 crates are now `claudium-*` (lib paths `claudium_*`), env vars are
  `CLAUDIUM_*`, the mod namespace and datapack fixtures are `claudium*`, and the
  upstream service URLs (registry, mixins, telemetry domain, keychain service)
  are repointed into this project's namespace so nothing calls upstream.
- "Horizontal" and the like are untouched (the rename guarded against `*tal`).
- Lockfiles kept their pinned third-party versions: only our own crate names were
  renamed inside them, so no transitive dependency drifted.
- The one deliberate exception in code: `claudium-mods` also reads the legacy
  `horizon-mod.toml` manifest name (`LEGACY_MANIFEST`), because the signed sample
  fixture `coords_hud-0.1.0.hmod` embeds it and its signing key was deleted
  upstream, so its bytes can't change without losing the registry's
  signature-verification test coverage. New mods are always written
  `claudium-mod.toml`; the two unsigned fixtures were repacked to the new name.
- "Horizon" now remains only in the four credit/reference locations: `LICENSE`,
  `NOTICE.md`, `docs/upstream/` (archived upstream docs) and `docs/claudium/`
  (this derivation record) — the inspiration credit.

### Debranding

- Renamed every player-facing "Horizon" to "Claudium": 684 string literals across
  both workspaces, all seven language files, the window and title bar, the Linux
  desktop entries and Wayland app id, the macOS bundle identity, and the icon
  files (`packaging/icons/claudium.*`, `claudium-64.png`).
- The data folders a player sees are now `~/.local/share/claudium` and
  `claudium-launcher`, and the installed binary is `claudium-launcher`.
- The `minecraft:brand` our client and server send is now `claudium`
  (`horizon_protocol::BRAND`). `is_native_brand` still accepts `horizon`, so the
  extra channels (quiet commands, content identity) keep working both ways with
  upstream Horizon. Vanilla ignores the brand, so joining vanilla servers is
  unaffected.
- The server panel's POST guard header is `X-Claudium`, and a shortcut's macOS
  plist key is `ClaudiumInstance`.
- Removed the official-server injection from both the launcher and the game: it
  put *upstream's* server (`horizon.bunchy7s.dev`) at the top of the player's
  list, which a fork must not ship under its own name. Nothing is added to the
  server list that the player did not add.

What deliberately keeps the old name: crate names and Rust paths
(`horizon_client::…`, which native mod hook targets are pinned to), `HORIZON_*`
environment variables, the `horizon` mod namespace (renaming it would break mod
content), the `SourceId::Horizon` registry and `Item::Horizon` settings variants,
code comments, and `LICENSE`/`NOTICE.md`, where the copyright notice is required.

### Launcher

- Renamed the shipped binary to `claudium-launcher` and the window, title bar
  and sidebar wordmark to Claudium. The Cargo package keeps upstream's name so
  the library target stays `horizon_launcher`, which the whole crate imports by
  that path; only the binary is renamed.
- Added `.github/workflows/windows-exe.yml`: builds `Claudium-Launcher.exe` and
  `Claudium.exe` (plus `claudium-ctl.exe`) on Windows runners, uploads them as
  artifacts on every run and attaches them to the release on a `v*` tag. Neither
  can be cross-compiled from Linux: wgpu and GPUI draw through DirectX there,
  and the icon is embedded by a Windows-only build script.

- Rebuilt the Play row as the Minecraft launcher's play bar: a full-width band
  under the hero with its own fill and a rule along the top, the version picker
  on the left, the one big green PLAY in the middle of the window and the
  secondary actions on the right. PLAY is set in capitals and 320px wide.

- Moved the account to the top of the sidebar, where the Minecraft launcher has
  it: face, name and how you are signed in, opening the same account menu. The
  top bar now carries nothing but the page tabs, and Settings moved to the foot
  of the sidebar.
- Removed upstream's sponsor advertising: the sidebar card, the Servers page
  banner, the timer that counted how long it had been on screen, `partner.rs`,
  its three images and its strings in all seven languages. Horizon's own
  community links (Sponsor, Discord, Website) went with them.

- Repainted the launcher in the Minecraft launcher's colours: near-black sidebar,
  dark neutral page, Minecraft green for the one main action, white text at three
  weights (`horizon-theme`'s `night::VANILLA` and `VANILLA_LIGHT`). White on the
  green face is 4.6:1; `accent_hi` is the lighter green that stays readable as
  text on both the page and the sidebar.
- Replaced Horizon's painted night sky with a flat surface, as the real launcher
  has (`launcher/src/sky.rs`, `FLAT`). This skips the bands, stars, clouds, sun
  and skylines, their baked sprites and the window wake-ups that animated them,
  so the launcher now stands completely still at rest. Set `FLAT` to `false` to
  get the sky back; the module still implements it in full.

### Base

- Copied Horizon `c995c9b` into this repository as the base, kept its MIT licence
  and recorded the derivation in `NOTICE.md`. Removed upstream workflows that
  depend on their runners and signing keys (see `upstream.md`).
- Removed the previous from-scratch Claudium client (~7.5k lines: wgpu renderer,
  world generation, Java Edition protocol client, egui launcher). It is still on
  the `main` branch if anything there is worth recovering.

### Notes

- [docs/claudium/ideas.md](ideas.md) lists features worth building, each chosen
  because the engine already has the hard part.

- [docs/claudium/roadmap.md](roadmap.md) records what is already done upstream
  in mod porting and performance, what is genuinely open, and what each open
  item is blocked on. Read it before planning work in either area.

## Known gaps

- **Sign-in uses the public launcher id by default** (browser sign-in, see the
  2026-11-01 entry). Set the repository variable `CLAUDIUM_MSA_CLIENT_ID` to use
  a registered Azure app instead (`docs/microsoft-account.md`).
- **Online-mode servers need that sign-in too.** Until it is configured,
  Claudium joins offline-mode and LAN servers only. A Claudium *server* hosts
  vanilla 26.3 clients either way, online mode included.

- **The launcher still installs upstream's game.** `backend::REPO` points at
  `horizon-rust/horizon`, so Play downloads Horizon's released builds. Pointing
  it at this repository needs us to publish game builds of our own; upstream's
  release workflows were removed because they need their signing keys and their
  self-hosted macOS runner.
- The window icon is still Horizon's (`packaging/icons/horizon.ico`).
- The look has not been seen running. It is checked by `cargo fmt`, clippy and
  the launcher's 296 tests, but GPUI needs a GPU and no CI here opens a window.

## The bundle beside the root, and only for the prepared root (2026-11-03)

A player's log on Windows: the loading screen in 1.3 s, then 59 s before the bundle was even
opened, a second bundle of a 17,584-file overlay (a resource pack or content mod is on), and
item icons missing in game.

- The bundle is written beside the prepared root (`<root>.claudium-bundle.bin`), not in it.
  A resource pack's overlay records the prepared root's directories as dependencies
  (`packs/cache.rs`), so writing into the root made every overlay rebuild, which on Windows
  copies every file again. One left inside an old root is removed.
- Only the prepared root is bundled. An overlay is rebuilt under its own rules and rewrites
  merged files in place, which the bundle's directory times can't see: a stale overlay bundle
  is the likeliest cause of the missing icons (not reproduced on Linux, where overlays are
  symlinks).
- The startup loader logs how long the resource-pack and content-mod overlays took.

## Built-in mods on the Mods page; a switch to turn the bundle off (2026-11-03)

- Options → Claudium Settings → **Mods** lists the built-in mods with on/off buttons
  (`extras::builtin_mod_items`), under mods installed from files, or alone when there are
  none. Before, the page said no mods were installed and the switches were only on Extras.
- `CLAUDIUM_NO_BUNDLE` (any value) makes `vfs::mount` refuse, so the game reads the loose
  files as before the bundle: a way out while a player reports missing textures on Windows
  that do not reproduce on Linux (item icons blank, with and without the bundle and the ten
  starter mods, all drew there).

## Chat Heads, and ten mods on in our build (2026-11-02)

- **Chat heads** (`Mod::ChatHeads`): a chat line remembers its sender (`<Name> ...`,
  `chat::sender_of`), and while the mod is on the game hands the chat each listed player's
  face skin (`Chat::heads`, from the player list, downloaded skins through `face_skin`); the
  line draws the face (8 px, with the hat layer) and starts 10 px later, and clicks and hovers
  follow it.
- The launcher's bundled build switches ten mods on once (`bundle::starter_mods`), never
  over a saved choice. The game's own defaults are unchanged.

## Fast start on Windows: the resources as one bundle (2026-11-02)

The game opened about 26,000 small files on every launch before the title screen, and about
16,000 more entering a world (the world's server reads the recipes, loot tables, tags and
worldgen data again). That is a few seconds on Linux, but on Windows each open is slow and
passes the antivirus: a ~30 second black window at every start, and a slow world load.

- `claudium_core::vfs`: the prepared root's small files (`assets/`, `data/`, `worldgen/`;
  not the sounds or anything over 1 MB) packed into `claudium-bundle.bin`, about 17 MB, read
  into memory in one go. It mirrors the `std::fs` calls the loaders use (`read`,
  `read_to_string`, `read_dir`), and everything else, including every write, goes to `std::fs`.
  A file missing from a directory the bundle holds whole is answered as missing without asking
  the disk (optional `.mcmeta` files are looked for a lot).
- Kept current: keyed on `claudium-resources.json` (every preparation writes it) and on the
  modification times of the directories it lists, so a file added after it (sounds, fonts,
  languages) makes a new one. A missing, stale or damaged bundle only means reading the files
  one by one, as before.
- Made at the end of preparing (`claudium-prepare`), else by the game on its first start.
  The game mounts it in `load_resources` and logs how long it took.
- Every `fs::read`, `fs::read_to_string` and `fs::read_dir` of the loader crates
  (claudium-items, -world, -client-render, -client-core, -client) goes through it, plus the
  PNG decoders and header reads that opened files directly.
- Measured on Linux (llvmpipe), start to a loaded world: **41,738 file opens before, 121
  after**; the bundle loads in 20 to 30 ms. Not yet measured on Windows.

## Built-in mods, batch 2: JEI, Jade, AppleSkin, VeinMiner (2026-11-01)

Our own Rust versions of four of Modrinth's most used mods, switches under Extras → Mods,
off by default.

- **Item viewer** (`game/builtin_jei.rs`): drives the item panel upstream built for a
  recipe-viewer native mod. Search by name, `@mod`, `#tag`; click for recipes, right-click
  or U for uses, R for recipes; tabs per station; the + button fills the crafting grid
  (shift for as many as fit) through the recipe book's placement.
- **Block info**: the block or mob under the crosshair at the top of the screen, with its
  icon, the tool its `mineable` tag asks for and whether the held item harvests it, the mod
  it comes from, and a mob's health (single player mobs).
- **Food info**: saturation as a gold frame over the hunger shanks, the shanks the held food
  would fill, and "Restores N hunger, S saturation" in food tooltips. Drawn over the HUD; the
  vanilla HUD layout is untouched.
- **Vein mining**: sneaking with a tool, breaking an ore, log or stem breaks the connected
  vein (26 neighbours, up to 64 blocks), each through `break_block_as_player`, so tool wear,
  drops, statistics and every rule of a normal break apply. Stops if the tool breaks. In
  worlds the player hosts; the world's server reads the switch when the world opens.

## Built-in mods, batch 1 (2026-11-01)

Mods are now built into the game instead of being native mods each player has to build
(20 to 30 minutes on first install). They are switches on the Extras page, all off by
default, so a fresh install still plays as vanilla 26.3. [mods.md](mods.md) is the list of
50 and where each stands.

- New: zoom, FPS counter, keystrokes with clicks per second and the minimap, which upstream
  had moved out to its own mods repository (`horizon-rust/horizon-mods`, not in this copy).
  The game already drew them for native mods; the built-in ones feed the same paths, and a
  native mod's zoom or minimap still wins.
- New switch-only mods (`extras::Mod`): coordinates and facing, biome, light level, clock,
  speedometer, armor HUD, effect timers (info corner), death coordinates and a low-durability
  alert (chat).
- `crates/claudium-client/src/game/builtin_mods.rs` holds them; `game.rs`, `game/remote.rs`
  and `game/extras.rs` each gained one line to call it.

## The Rust launcher is the launcher again; Electron removed (2026-11-01)

The Electron launcher took about 15 minutes to start the game on every Play: it
ran `claudium-launcher prepare` each time, and the CLI's `prepare` always does
the whole preparation (data generators, exporters, assets) without asking whether
the version is prepared already. The Rust launcher in `launcher/` (upstream's,
already repainted in the Minecraft launcher's colours) prepares once and then
starts the game directly, as Horizon does. It is now the only launcher.

- Removed `launcher-electron/` and `.github/workflows/electron-launcher.yml`.
- `launcher/src/bundle.rs`: on start, the launcher adopts the game shipped beside
  it (`game/`: client, tools, `research-lock.json`, precompiled exporters) as an
  installed version, so nothing is downloaded from a release. A newer bundle is
  copied over the installed one without touching what was prepared (a stamp of
  the client's size, mtime and pins). A Java runtime in `runtime/` becomes
  `JAVA_HOME` when the player has none set. A version the Electron launcher
  prepared in `%APPDATA%\Claudium` is moved over, so nobody prepares twice.
- The window starts preparing the selected version on its own, once per run,
  when it is installed and not prepared, so the one-time step is usually done
  before Play is pressed. Play pressed while it runs starts the game when it ends.
- Sign-in with no Azure app: the public launcher id has no device-code flow, so
  the window now signs in through the browser (`SignInState::Browser`): it opens
  Microsoft's page, the player copies the blank page's address it lands on and
  presses Continue, which reads it from the clipboard (`SignIn::submit`). Both the
  account settings row and the owner gate have it; strings in all seven languages.
  A configured `CLAUDIUM_MSA_CLIENT_ID` still uses the device code.
- On Windows the public id signs in in a window of the launcher's own
  (`launcher/src/account_window.rs`, WebView2 through `wry`/`tao`, in a child
  process `claudium-launcher --sign-in-window URL` so its event loop never meets
  GPUI's). It reads the code from the redirect to `oauth20_desktop.srf` before
  the page loads: Microsoft now strips it from that page's address
  (`?removed=true`), so the copy-the-address flow above cannot work in a normal
  browser. That flow stays only where there is no window (macOS, Linux).
- `windows-exe.yml` builds it all in one job: `Claudium-Setup.exe` (NSIS,
  `packaging/windows/installer.nsi`, per user, Start menu and desktop shortcuts)
  and the same folder unpacked (`Claudium-Launcher.exe`, `game/`, `runtime/`).

The prepared Minecraft resources are still made on the player's machine from
their own copy, the first time only: they are Mojang's files and are never
shipped, as upstream never ships them.

## Play the bundled game, no release needed (2026-10-31)

The Electron launcher now installs the game from the copy bundled inside its own
installer, instead of downloading a published release. This fixes "no version is
installed yet" on a private fork that has no releases.

- `backend::install_from_dir(tag, src)` copies a local game folder into the
  launcher's `versions/<tag>` and marks it installed, the same end state as a
  downloaded release (idempotent: an already-installed version keeps its prepared
  resources). `claudium-launcher install <tag> --from <dir>` exposes it.
- `launcher-electron/game.js` Play now: install from the bundled `game/` folder →
  prepare from the player's own Minecraft → play. No network, no release.
- `.github/workflows/electron-launcher.yml` builds the engine *and* the game
  (`claudium-client`, `claudium-ctl`, `claudium-prepare` + `research-lock.json`)
  on Windows, bundles both via electron-builder `extraResources`, and uploads a
  single installer artifact (the confusing internal `claudium-engine-exe` is gone).

Still needs, on the player's machine: Minecraft: Java Edition installed, and Java
available for `prepare` (the bundle ships no precompiled exporters yet, so prepare
needs a JDK). Untestable in CI/cloud: no GPU, no Minecraft.

## Bundle a JDK so preparing needs no Java install (2026-10-31)

First Play failed on a machine with no JDK: preparing the game compiles the
exporters (needs `javac`) and runs data generators (needs a Java runtime), and
the launcher's fallback — downloading Mojang's Java — ships only a runtime (no
`javac`) and timed out besides. Now the Windows installer bundles a full Temurin
JDK (the `java_major` research-lock.json pins), and `launcher-electron/game.js`
sets `JAVA_HOME`/`PATH` to it on every engine call, so prepare just runs — no Java
install, no Mojang download. Adds ~180 MB to the installer; fine for private use.
