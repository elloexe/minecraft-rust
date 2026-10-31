# Launcher commands

<!-- Generated from launcher/src/cli.rs by cli_tools::markdown(); a test fails when this file is out of date. Update it with: CLAUDIUM_WRITE_DOCS=1 cargo test --manifest-path launcher/Cargo.toml the_launcher_reference -->

Every command of `claudium-launcher`, and the tool `claudium-launcher mcp` offers for it. A tool's arguments are the command's, in snake_case. A tool runs the command with `--json` and returns what it printed. The tools that change the player's things for good need `confirm: true`. The tools that send something or reach other people say "only when the player asks". The game's own tools, `game_*`, are in [claudium-ctl-commands.md](claudium-ctl-commands.md). With No AI on, `mcp` and `agent-config` are hidden and refuse; every other command stays. See [agent-control.md](agent-control.md#no-ai).

## `claudium-launcher versions`

List Claudium releases, or the installed versions.

    claudium-launcher versions

- `--installed`: Only the versions installed here (no network)

MCP: `versions`.

## `claudium-launcher install`

Download and install a release (a tag such as v2026.10.13, or "latest").

    claudium-launcher install <TAG>

- `TAG`

MCP: `install`.

## `claudium-launcher remove`

Remove an installed version.

    claudium-launcher remove <TAG>

- `TAG`

MCP: `remove`, needs `confirm: true`.

## `claudium-launcher prepare`

Prepare a version's resources from your own Minecraft install, or from Mojang.

    claudium-launcher prepare

- `--minecraft`: The game folder of the official launcher, Prism Launcher, MultiMC, the Modrinth App, ATLauncher or GDLauncher (default: the launcher's, then every launcher's default)
- `--download`: Download Minecraft from Mojang for the signed-in account that owns it, instead of using an install

MCP: `prepare`.

## `claudium-launcher play`

Start the client (needs a signed-in Microsoft account that owns Minecraft: Java Edition).

    claudium-launcher play

- `--instance`: The instance to play (its name or id; default: the selected one). A Java instance starts its Minecraft: Java Edition, downloaded with its Java the first time
- `--world`: Open or create this world (a folder under saves) at once
- `--creative`: Creative mode for a new world
- `--join`: Join a friend's world with its invite code
- `--server`: Join this server at once: an address, or a server of the list by name or position
- `--arg`: An extra argument for the game, passed through as-is (repeatable), for example --server-thread

MCP: `play`, then the `game_*` tools.

## `claudium-launcher server`

Run a dedicated server here; type commands into its console.

    claudium-launcher server

- `--dir`: The server's folder (default: the one the launcher uses)
- `--accept-eula`: Agree to the Minecraft EULA (https://aka.ms/MinecraftEULA)

MCP: `server`, `server_command`, `server_log`, `server_stop`.

## `claudium-launcher native-build`

Native mods (experimental, docs/native-mods.md): compile mixin bundles on the Claudium revision they pin, with the launcher's own Rust, as the version the "Native mods" instance plays. Not sandboxed: the mods' code runs with your rights.

    claudium-launcher native-build [FOLDER]

- `FOLDER`: A mod folder (claudium-mixin.toml, or an unpacked .hnmod), or a folder of them (default: the native mods installed and turned on in the instance)
- `--instance`: The instance whose installed native mods to build, and which then plays the build (default: the selected one; the game's own home builds into the Native mods instance)
- `--trust-native-code`: You read the mods' source and trust it with full access to this computer
- `--allow-version-mismatch`: Build mods made for different Claudium versions together anyway, on the newest of them

MCP: none: the player's switch.

## `claudium-launcher notes`

Show a release's patch notes (default: the newest).

    claudium-launcher notes [TAG]

- `TAG`

MCP: `notes`.

## `claudium-launcher links`

Show the Discord, sponsor and website links.

    claudium-launcher links

MCP: `links`.

## `claudium-launcher account list`

The signed-in accounts and which one plays.

    claudium-launcher account list

MCP: `account_list`.

## `claudium-launcher account sign-in`

Sign in with a Microsoft account in your browser (device code). Play needs one that owns Minecraft: Java Edition.

    claudium-launcher account sign-in

MCP: `account_sign_in`.

## `claudium-launcher account sign-out`

Sign out: the account's tokens leave the keychain.

    claudium-launcher account sign-out <WHO>

- `WHO`: Its name or UUID

MCP: `account_sign_out`, needs `confirm: true`.

## `claudium-launcher account select`

Play as this account, or as one of your offline names.

    claudium-launcher account select <WHO>

- `WHO`

MCP: `account_select`.

## `claudium-launcher account offline`

Play offline (LAN, offline-mode servers) beside a signed-in account that owns the game: as NAME (kept to switch back to; 3 to 16 letters, digits or _), or as the name the game made up.

    claudium-launcher account offline [NAME]

- `NAME`

MCP: `account_offline`.

## `claudium-launcher account forget-offline`

Forget one of your offline names; its skin and cape go with it.

    claudium-launcher account forget-offline <NAME>

- `NAME`

MCP: `account_forget_offline`.

## `claudium-launcher account skin`

Show or set an offline name's skin and cape: a 64x64 (or 64x32) skin PNG, a 64x32 cape PNG, the model. Kept by the launcher and given to the game when that name plays.

    claudium-launcher account skin <NAME> [FILE]

- `NAME`
- `FILE`: The skin's PNG
- `--model`: classic or slim arms
- `--cape`: The cape's PNG
- `--no-skin`: Back to the default skin
- `--no-cape`: Take the cape off

MCP: `account_skin`.

## `claudium-launcher settings`

Show the launcher's settings, or set one (the Settings page): selected-version TAG, minecraft-dir PATH, server-dir PATH ("default" resets a folder), install-new-versions on|off, prune-old-versions on|off (delete old versions after an install, keeping the selected one, the ones instances use and the newest few), keep-versions N (how many of the newest to keep; "default" is 2), opening-animation on|off, proxy http://host:port ("default" removes it), while-playing stay|minimize|hide (what the window does while the game runs), language system|en|fr|ru|de|zh|es|uk|pl (the window's language; system follows the computer's), accent violet|blue|teal|green|amber|red|pink, background PATH (a picture behind Home; "default" shows Claudium's landscape).

    claudium-launcher settings [KEY] [VALUE]

- `KEY` (selected-version, minecraft-dir, server-dir, install-new-versions, prune-old-versions, keep-versions, opening-animation, proxy, while-playing, language, accent, background)
- `VALUE`: The new value (with KEY)

MCP: `settings`.

## `claudium-launcher game-args`

Show or set the extra arguments the game starts with (for example --server-thread); quotes group a word with spaces in it.

    claudium-launcher game-args [ARGS]

- `ARGS`: Omit to show the current arguments

MCP: `game_args`.

## `claudium-launcher share-log`

Upload an instance's latest.log to mclo.gs (a paste service for Minecraft logs) and print its link, for a bug report: only when the player asks. The home folder, the user name and the account names are taken out first; the link expires after 90 days unread.

    claudium-launcher share-log

- `--instance`: The instance (default: the selected one)

MCP: `share_log`, only when the player asks.

## `claudium-launcher unshare-log`

Delete a log shared with share-log: the "<id>:<token>" it printed.

    claudium-launcher unshare-log <SHARED>

- `SHARED`

MCP: `unshare_log`, needs `confirm: true`.

## `claudium-launcher logs`

Print the end of the game's log (logs/latest.log in the game folder), or its newest crash report.

    claudium-launcher logs

- `--lines`: How many lines of latest.log
- `--crash`: The newest crash report instead

MCP: `logs`.

## `claudium-launcher update`

Look for a newer Claudium release and launcher, and install them.

    claudium-launcher update

- `--check`: Only report what is available

MCP: `update`.

## `claudium-launcher desktop-entry`

Linux: put Claudium in the applications menu (an entry and the icon, under $XDG_DATA_HOME), or take it out with --remove.

    claudium-launcher desktop-entry

- `--remove`: Remove the entry and the icon, and stop adding them back

MCP: `desktop_entry`.

## `claudium-launcher data status`

What is allowed, the install ID and how many items wait (the default).

    claudium-launcher data status

MCP: `data_status`.

## `claudium-launcher data allow`

Turn kinds on: performance, crashes, bugs, benchmarks, or all.

    claudium-launcher data allow <KINDS>...

- `KINDS`

MCP: `data_allow`, only when the player asks.

## `claudium-launcher data deny`

Turn kinds off: performance, crashes, bugs, benchmarks, or all.

    claudium-launcher data deny <KINDS>...

- `KINDS`

MCP: `data_deny`.

## `claudium-launcher data show`

Print exactly what would be sent.

    claudium-launcher data show

MCP: `data_show`.

## `claudium-launcher data delete`

Delete everything sent from here, and what waits, and take a new install ID.

    claudium-launcher data delete

MCP: `data_delete`, needs `confirm: true`.

## `claudium-launcher data send`

Send what is waiting now.

    claudium-launcher data send

MCP: `data_send`, only when the player asks.

## `claudium-launcher data bug`

Send a bug report you write; it is printed first, as it would leave.

    claudium-launcher data bug <TEXT>

- `TEXT`
- `--log`: Add the end of the game's log (scrubbed, without chat)
- `--yes`: Send without asking

MCP: `data_bug`, only when the player asks.

## `claudium-launcher data benchmark`

Upload a benchmark report (docs/benchmark-report.md); it is printed first.

    claudium-launcher data benchmark <FILE>

- `FILE`
- `--yes`

MCP: `data_benchmark`, only when the player asks.

## `claudium-launcher instance list`

The instances, each with its game folder (* = the one Play starts).

    claudium-launcher instance list

MCP: `instance_list`.

## `claudium-launcher instance create`

Make an instance: empty but for your options and name, or a copy of another one's worlds, options and packs.

    claudium-launcher instance create <NAME>

- `NAME`
- `--copy`: The instance to copy (its name or id)
- `--minecraft`: Make a Minecraft: Java Edition instance of this version (1.21.4, latest or latest-snapshot) instead of a Claudium one
- `--loader`: With this mod loader: fabric or quilt
- `--loader-version`: The loader's version (default: its newest stable one)

MCP: `instance_create`.

## `claudium-launcher instance versions`

The versions of Minecraft: Java Edition a Java instance can play, the newest first.

    claudium-launcher instance versions

- `--snapshots`: Snapshots too
- `--old`: Old betas and alphas too

MCP: `instance_versions`.

## `claudium-launcher instance java`

A Java instance's Java: its own Java home or the automatic pick, memory and JVM arguments.

    claudium-launcher instance java <INSTANCE>

- `INSTANCE`
- `--home`: A Java home to run it on
- `--auto`: Go back to the Java the version needs, picked or downloaded by the launcher
- `--memory`: The most memory the game may use, in MiB
- `--args`: Extra JVM arguments (one string; "" clears them)
- `--window`: The game window's size, as 1280x720; "default" goes back to the game's own

MCP: `instance_java`.

## `claudium-launcher instance loader`

A Java instance's mod loader: fabric or quilt, or none. --list shows its versions.

    claudium-launcher instance loader <INSTANCE> <LOADER>

- `INSTANCE`
- `LOADER`
- `--list`: List the loader's versions for the instance's Minecraft instead

MCP: `instance_loader`.

## `claudium-launcher instance rename`

Rename an instance (its folder stays).

    claudium-launcher instance rename <INSTANCE> <NAME>

- `INSTANCE`
- `NAME`

MCP: `instance_rename`.

## `claudium-launcher instance delete`

Delete an instance: its folder (worlds, options, packs) goes to the Trash, where you can get it back. Claudium's own instance stays.

    claudium-launcher instance delete <INSTANCE>

- `INSTANCE`

MCP: `instance_delete`.

## `claudium-launcher instance icon`

Set an instance's icon: a block or item name from your prepared files (grass_block), a colour (#3fa7d6), or none.

    claudium-launcher instance icon <INSTANCE> <ICON>

- `INSTANCE`
- `ICON`

MCP: `instance_icon`.

## `claudium-launcher instance group`

Put an instance in a sidebar group, or take it out with no group.

    claudium-launcher instance group <INSTANCE> [GROUP]

- `INSTANCE`
- `GROUP`

MCP: `instance_group`.

## `claudium-launcher instance select`

Make Play start this instance.

    claudium-launcher instance select <INSTANCE>

- `INSTANCE`

MCP: `instance_select`.

## `claudium-launcher instance version`

Keep an instance on one installed Claudium version, or "latest" to follow updates again.

    claudium-launcher instance version <INSTANCE> <VERSION>

- `INSTANCE`

MCP: `instance_version`.

## `claudium-launcher instance export`

Export an instance (or a port) to share: its settings, packs, Claudium mods and the worlds named, as a .mrpack (resource packs from Modrinth listed by download) or a zip. Never logs, screenshots or account data; worlds go without their per-player files.

    claudium-launcher instance export [INSTANCE]

- `INSTANCE`: The instance (default: the selected one)
- `--format` (mrpack, zip)
- `--world`: A world to put in it (its name or folder); repeat for more
- `--out`: Where to write it (default: "<name>.mrpack" in the current folder)

MCP: `instance_export`.

## `claudium-launcher instance launch`

Show or set what runs around an instance's game: a shell command before it (the game starts only if it succeeds), a wrapper it runs through (`nice -n 5`), environment variables ("NAME=value NAME2=value") and a shell command after it exits. An empty value clears one. The game and the commands also get INST_NAME, INST_ID, INST_DIR, INST_MC_DIR and, for a Java instance, INST_JAVA. On Linux, also GameMode, MangoHud and the dedicated graphics card, when the computer has them.

    claudium-launcher instance launch <INSTANCE>

- `INSTANCE`
- `--pre-launch`
- `--wrapper`
- `--post-exit`
- `--env`
- `--gamemode`: Linux: run it through GameMode (gamemoderun), on or off
- `--mangohud`: Linux: MangoHud's overlay, on or off
- `--dedicated-gpu`: Linux: the dedicated graphics card (DRI_PRIME, or NVIDIA's offload), on or off

MCP: `instance_launch`, needs `confirm: true`.

## `claudium-launcher instance shortcut`

Put a shortcut on the desktop that plays an instance (`play --instance`): a .desktop file (also in the applications menu) on Linux, an app on macOS, a .lnk on Windows.

    claudium-launcher instance shortcut [INSTANCE]

- `INSTANCE`: The instance (default: the selected one)
- `--remove`: Move the instance's shortcuts to the trash instead

MCP: `instance_shortcut`.

## `claudium-launcher instance import`

Import what a file is, as dropping it on the window does: a shared instance (a .mrpack without mod loaders, such as `instance export` writes, or a game folder zip) becomes a new instance; a Java modpack goes to the Modpacks import; a resource or shader pack into the instance (default: the selected one).

    claudium-launcher instance import <FILE>

- `FILE`
- `--instance`: The instance a pack goes into

MCP: `instance_import`.

## `claudium-launcher instance others`

List the instances other launchers keep (Prism Launcher, MultiMC, the official launcher, the CurseForge app, the Modrinth App), read without changing anything there.

    claudium-launcher instance others

- `--folder`: Look in this folder instead: a launcher's folder, its instances/ or one instance

MCP: `instance_others`.

## `claudium-launcher instance copy-from`

Copy another launcher's instance into a new Java instance: its Minecraft version and its game folder (worlds, mods, packs, options); the other launcher's copy stays as it is.

    claudium-launcher instance copy-from <NAME>

- `NAME`: Its name, as `instance others` lists it
- `--folder`: Look in this folder instead: a launcher's folder, its instances/ or one instance
- `--as`: The new instance's name (default: the same)
- `--minecraft`: The Minecraft version it plays, when its launcher doesn't say (the CurseForge app, the Modrinth App)

MCP: `instance_copy_from`.

## `claudium-launcher worlds list`

The worlds: name, mode, version, last played and size (the default).

    claudium-launcher worlds list

- `--instance`: The instance (default: the selected one)

MCP: `worlds_list`.

## `claudium-launcher worlds backup`

Zip a world into the instance's backups/ folder, as the game's Make Backup does.

    claudium-launcher worlds backup <WORLD>

- `WORLD`: Its folder or name
- `--instance`

MCP: `worlds_backup`.

## `claudium-launcher worlds copy`

Copy a world beside it, as "<name> (1)".

    claudium-launcher worlds copy <WORLD>

- `WORLD`
- `--instance`

MCP: `worlds_copy`.

## `claudium-launcher worlds delete`

Move a world to the system's trash.

    claudium-launcher worlds delete <WORLD>

- `WORLD`
- `--instance`

MCP: `worlds_delete`, needs `confirm: true`.

## `claudium-launcher worlds datapack-list`

A world's data packs, and which are on.

    claudium-launcher worlds datapack-list <WORLD>

- `WORLD`
- `--instance`

MCP: `worlds_datapack_list`.

## `claudium-launcher worlds datapack-add`

Copy a data pack zip into a world (the game turns it on when the world opens).

    claudium-launcher worlds datapack-add <WORLD> <FILE>

- `WORLD`
- `FILE`
- `--instance`

MCP: `worlds_datapack_add`.

## `claudium-launcher worlds datapack-remove`

Move a world's data pack to the system's trash.

    claudium-launcher worlds datapack-remove <WORLD> <PACK>

- `WORLD`
- `PACK`: Its file name, with or without .zip
- `--instance`

MCP: `worlds_datapack_remove`, needs `confirm: true`.

## `claudium-launcher packs list`

The resource and shader packs, and which are on (the default).

    claudium-launcher packs list

- `--kind` (resource, shader): Only this kind
- `--instance`: The instance (default: the selected one)

MCP: `packs_list`.

## `claudium-launcher packs add`

Copy a pack file in: a resource pack .zip, a shader pack .zip or a .hshader.

    claudium-launcher packs add <FILE>

- `FILE`
- `--instance`

MCP: `packs_add`.

## `claudium-launcher packs on`

Turn a pack on (a resource pack goes on top; a shader pack replaces the one that was on).

    claudium-launcher packs on <PACK>

- `PACK`: Its file name or title
- `--kind` (resource, shader)
- `--instance`

MCP: `packs_on`.

## `claudium-launcher packs off`

Turn a pack off.

    claudium-launcher packs off <PACK>

- `PACK`
- `--kind` (resource, shader)
- `--instance`

MCP: `packs_off`.

## `claudium-launcher packs remove`

Turn a pack off and move it to the system's trash.

    claudium-launcher packs remove <PACK>

- `PACK`
- `--kind` (resource, shader)
- `--instance`

MCP: `packs_remove`, needs `confirm: true`.

## `claudium-launcher java list`

The Java runtimes Claudium downloaded from Mojang and the ones installed on this computer.

    claudium-launcher java list

MCP: `java_list`.

## `claudium-launcher java install`

Download Mojang's Java runtime: a major (8, 16, 17, 21, 25) or a component name from a version's JSON (java-runtime-delta).

    claudium-launcher java install <JAVA>

- `JAVA`

MCP: `java_install`.

## `claudium-launcher screenshots list`

The screenshots, the newest first, with their files (the default).

    claudium-launcher screenshots list

- `--instance`: The instance (default: the selected one)

MCP: `screenshots_list`.

## `claudium-launcher screenshots delete`

Move a screenshot to the system's trash.

    claudium-launcher screenshots delete <SCREENSHOT>

- `SCREENSHOT`: Its file name, with or without .png
- `--instance`

MCP: `screenshots_delete`, needs `confirm: true`.

## `claudium-launcher multiplayer list`

The servers of the game's Multiplayer screen (the default).

    claudium-launcher multiplayer list

- `--instance`: The instance (default: the selected one)

MCP: `multiplayer_list`.

## `claudium-launcher multiplayer add`

Add a server to the list.

    claudium-launcher multiplayer add <ADDRESS>

- `ADDRESS`: Its address, as in the game's Add Server (host or host:port)
- `--name`: Its name in the list (default: Minecraft Server)
- `--instance`

MCP: `multiplayer_add`.

## `claudium-launcher multiplayer remove`

Remove a server from the list: its position from 1, name or address.

    claudium-launcher multiplayer remove <SERVER>

- `SERVER`
- `--instance`

MCP: `multiplayer_remove`, needs `confirm: true`.

## `claudium-launcher modpack list`

The imported packs and their coverage (the default).

    claudium-launcher modpack list

MCP: `modpack_list`.

## `claudium-launcher modpack import`

Import a .mrpack, a CurseForge zip or an instance folder, and write its coverage report.

    claudium-launcher modpack import <PATH>

- `PATH`

MCP: `modpack_import`.

## `claudium-launcher modpack port`

Get the jars of what this computer can port and carry them over.

    claudium-launcher modpack port <PACK>

- `PACK`: The pack's folder name, as list shows it
- `--from`: A folder with your own jars (an installed instance, a downloads folder)

MCP: `modpack_port`.

## `claudium-launcher modpack play`

Play a ported pack as its own profile.

    claudium-launcher modpack play <PACK>

- `PACK`

MCP: `modpack_play`.

## `claudium-launcher modpack remove`

Remove an imported pack: its jars, ports and the profile's saves.

    claudium-launcher modpack remove <PACK>

- `PACK`

MCP: `modpack_remove`, needs `confirm: true`.

## `claudium-launcher mods list`

What the browser installed in an instance (the default).

    claudium-launcher mods list

- `--instance`: The instance (default: the selected one)

MCP: `mods_list`.

## `claudium-launcher mods search`

Search for packs Claudium loads, or for what a Java instance runs (made for its Minecraft version and loader).

    claudium-launcher mods search [TEXT]...

- `TEXT`
- `--kind` (mod, resource, modpack, shader, datapack)
- `--everything`: Also what was made for other versions or runs only partly, each marked
- `--source` (modrinth, curseforge, claudium)
- `--instance`: The instance (default: the selected one)

MCP: `mods_search`.

## `claudium-launcher mods check`

Say how well a Java mod runs in Claudium: its jar is downloaded (checksum checked), carried over and read; the answer is kept.

    claudium-launcher mods check <PROJECT>

- `PROJECT`
- `--everything`: Allow a version made for another Minecraft version (1.21 and later)
- `--source` (modrinth, curseforge, claudium)

MCP: `mods_check`.

## `claudium-launcher mods install`

Install a pack and what it needs into an instance, by its id or slug (checksums checked); a Java mod is ported into claudium-mods and turned on (a Java instance gets the jar made for its version and loader in mods/); a modpack is imported as an instance of its own; a data pack goes into the world `--world` names.

    claudium-launcher mods install <PROJECT>

- `PROJECT`
- `--world`: Install the project's data pack into this world of the instance (its name or folder)
- `--instance`: The instance (default: the selected one)
- `--everything`: Allow a version made for another Minecraft version
- `--source` (modrinth, curseforge, claudium)

MCP: `mods_install`.

## `claudium-launcher mods update`

Update what the browser installed (or one pack).

    claudium-launcher mods update [PROJECT]

- `PROJECT`
- `--check`: Only say what is newer
- `--instance`: The instance (default: the selected one)

MCP: `mods_update`.

## `claudium-launcher mods add-github`

Install a native mod from a GitHub link (docs/native-mods.md, "Install from GitHub"): https://github.com/OWNER/REPO (its latest release, else its default branch), .../releases/tag/TAG, .../tree/REF or .../commit/SHA. Resolved to an exact release file or commit, checked like a registry mod. NOT reviewed by the Claudium registry.

    claudium-launcher mods add-github <URL>

- `URL`
- `--mod`: The mod to install when the link holds several
- `--all`: Install every mod the link holds
- `--instance`: The instance (default: the selected one)
- `--trust-native-code`: You read the mod's source (the link printed) and trust it with full access to this computer

MCP: none: the player's switch.

## `claudium-launcher mods update-github`

Look for newer versions of the native mods installed from GitHub (the latest release, or the default branch's head for a mod installed from a commit), and install them.

    claudium-launcher mods update-github [ID]

- `ID`: One mod (default: all of them)
- `--check`: Only say what is newer
- `--instance`: The instance (default: the selected one)
- `--trust-native-code`: You read the new source and trust it with full access to this computer

MCP: none: the player's switch.

## `claudium-launcher mods remove`

Remove a pack the browser installed.

    claudium-launcher mods remove <PROJECT>

- `PROJECT`
- `--instance`: The instance (default: the selected one)

MCP: `mods_remove`, needs `confirm: true`.

## `claudium-launcher mods enable`

Turn on a mod the browser installed (claudium-mods.txt, as the game's Options > Mods does; in a Java instance, its file loses `.disabled`).

    claudium-launcher mods enable <PROJECT>

- `PROJECT`
- `--instance`: The instance (default: the selected one)

MCP: `mods_enable`.

## `claudium-launcher mods disable`

Turn off a mod the browser installed; it stays installed (in a Java instance, as `<file>.disabled`).

    claudium-launcher mods disable <PROJECT>

- `PROJECT`
- `--instance`: The instance (default: the selected one)

MCP: `mods_disable`.

## `claudium-launcher rust status`

Whether it is installed, its version and whether this computer can link with it.

    claudium-launcher rust status

MCP: `rust_status`.

## `claudium-launcher rust install`

Download the pinned toolchain from static.rust-lang.org (once), checked by SHA-256.

    claudium-launcher rust install

MCP: `rust_install`.

## `claudium-launcher rust remove`

Delete the toolchain and the native builds' Cargo home.

    claudium-launcher rust remove

MCP: `rust_remove`.

## `claudium-launcher servers list`

The servers, their folder and whether they run.

    claudium-launcher servers list

MCP: `servers_list`.

## `claudium-launcher servers create`

Create a server; with no options it is a new world, survival, normal, 10 players, friends only.

    claudium-launcher servers create

- `--name`
- `--seed`: The new world's seed (default: random)
- `--copy-world`: Start from a copy of this world folder (one of your saves)
- `--mode` (survival, creative, adventure, spectator)
- `--difficulty` (peaceful, easy, normal, hard)
- `--max-players`
- `--open`: Let anyone in (no whitelist)
- `--check-accounts`: Check players' Minecraft accounts (they must sign in with Microsoft)
- `--player`: Who may join (repeatable); the first becomes an operator
- `--accept-eula`: Agree to the Minecraft EULA (https://aka.ms/MinecraftEULA)

MCP: `servers_create`, only when the player asks.

## `claudium-launcher servers set-up`

Copy an installed, prepared version's programs and game files for the servers (again).

    claudium-launcher servers set-up

MCP: `servers_set_up`.

## `claudium-launcher servers run`

Run a claudium-server command on a server, for example: servers run "My server" -- status.

    claudium-launcher servers run <SERVER> -- <ARGS>...

- `SERVER`: The server's name or folder
- `ARGS`

MCP: `servers_run`.

## `claudium-launcher servers forget`

Take a server off the list (its folder stays).

    claudium-launcher servers forget <SERVER>

- `SERVER`

MCP: `servers_forget`, needs `confirm: true`.

## `claudium-launcher mcp`

Serve the Model Context Protocol on stdin and stdout, for AI agents (refused while No AI is on).

    claudium-launcher mcp

MCP: none: this command is the MCP server.

## `claudium-launcher agent-config`

How to connect an AI agent (Claude Code, Codex, Cursor, any MCP client) to `mcp`: the config to paste and the command that adds it (refused while No AI is on).

    claudium-launcher agent-config

- `--client` (claude-code, codex, cursor, generic): Only this client
- `--test`: Start the MCP server as a client would and say whether it answered

MCP: none: it prints how to connect a client to `mcp`.

## `claudium-launcher no-ai`

Show or set No AI: on hides and turns off every AI feature (the companion, agent control, claudium-ctl and both MCP servers); off, the default, makes them available.

    claudium-launcher no-ai [STATE]

- `STATE` (on, off): Omit to show the current state

MCP: none: the player's switch.

## `claudium-launcher benchmark`

Measure the launcher: starts (the first is the cold one), page switching, idle CPU and memory, the release list fetch; writes a benchmark report to benchmarks/ in its data folder.

    claudium-launcher benchmark

- `--repeats`: How many times to start it

MCP: `benchmark`.
