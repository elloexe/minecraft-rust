# Claudium launcher (Electron)

A fast, native-looking launcher for Claudium (Minecraft: Java Edition, in Rust).
It signs you in with Microsoft in an embedded window — one click, no code to
paste — and launches the Rust game, which starts in about a second and a half.

- **No local web server** (unlike the old client): the window loads the page
  directly, so it opens instantly.
- **Sign-in** uses the public launcher app id by default, so it works with no
  Azure setup. Override it with the `CLAUDIUM_MSA_CLIENT_ID` environment
  variable or repository variable.
- **Play** writes the signed-in account to a session file and runs
  `claudium-client`. Point Settings at the game binary (or drop it next to the
  launcher / in the launcher's config folder).

## Develop

```sh
cd launcher-electron
npm install
npm start
```

## Build the Windows installer

```sh
npm run dist:win     # dist/Claudium-Launcher-Setup.exe
```

CI builds it on every push that touches this folder
(`.github/workflows/electron-launcher.yml`); a `v*` tag attaches it to the
release.

## Files

| File | Does |
|---|---|
| `main.js` | Electron main: window, IPC, the embedded Microsoft sign-in window, launching the game |
| `auth.js` | Microsoft → Xbox → Minecraft token exchange (the login.live.com flow) |
| `game.js` | Finds `claudium-client`, writes the session file, spawns it |
| `preload.js` | The locked-down bridge the page talks to |
| `renderer/` | The UI (HTML/CSS/JS): sidebar, hero, green PLAY bar |
