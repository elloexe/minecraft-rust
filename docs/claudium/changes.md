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

- **The launcher cannot start the game without an Azure app id.** `client_id()`
  reads `HORIZON_MSA_CLIENT_ID` from the environment or the build; with none,
  Play says so and refuses. Upstream bakes theirs in from a CI variable we do
  not have. Register one (`docs/microsoft-account.md`) and set the repository
  variable `CLAUDIUM_MSA_CLIENT_ID`, which `windows-exe.yml` passes to the
  build.
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
