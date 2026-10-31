'use strict';
// Finding, fetching and launching the Claudium game (the Rust `claudium-client`), plus a tiny
// JSON config. The game ships inside the installer (extraResources), so it is normally found
// right next to the launcher with nothing to download; the download is a dev/fallback path.
const fs = require('fs');
const os = require('os');
const path = require('path');
const { spawn } = require('child_process');

const EXE = process.platform === 'win32' ? 'claudium-client.exe' : 'claudium-client';
const REPO = 'elloexe/minecraft-rust';
const UA = 'claudium-launcher';

function configDir() {
  const base = process.env.CLAUDIUM_LAUNCHER_HOME
    || (process.platform === 'win32'
      ? path.join(process.env.APPDATA || os.homedir(), 'claudium-launcher')
      : path.join(process.env.XDG_CONFIG_HOME || path.join(os.homedir(), '.config'), 'claudium-launcher'));
  fs.mkdirSync(base, { recursive: true });
  return base;
}
function configPath() { return path.join(configDir(), 'launcher.json'); }
function loadConfig() { try { return JSON.parse(fs.readFileSync(configPath(), 'utf8')); } catch { return {}; } }
function saveConfig(cfg) { fs.writeFileSync(configPath(), JSON.stringify(cfg, null, 2), { mode: 0o600 }); }

// Where the game binary might be: bundled in the installer (resources/game), a configured path,
// next to the launcher, or in the launcher's config folder.
function findGame(cfg = loadConfig()) {
  const candidates = [
    cfg.gamePath,
    process.resourcesPath && path.join(process.resourcesPath, 'game', EXE),
    path.join(path.dirname(process.execPath), EXE),
    path.join(path.dirname(process.execPath), 'game', EXE),
    path.join(configDir(), EXE),
  ].filter(Boolean);
  for (const c of candidates) {
    try { if (fs.statSync(c).isFile()) return c; } catch { /* keep looking */ }
  }
  return null;
}

// Fallback for a launcher built without the bundled game: download it from the latest release.
async function downloadGame(onStatus) {
  onStatus?.('Finding the latest game build…');
  const rel = await fetch(`https://api.github.com/repos/${REPO}/releases/latest`, {
    headers: { 'User-Agent': UA, Accept: 'application/vnd.github+json' },
  });
  if (!rel.ok) throw new Error('No game release found yet, and the game is not bundled. Build a release or set the game path in Settings.');
  const data = await rel.json();
  const asset = (data.assets || []).find((a) => a.name === 'Claudium.exe' || a.name === EXE);
  if (!asset) throw new Error('The latest release has no game executable. Set the game path in Settings.');
  onStatus?.('Downloading the game…');
  const bin = await fetch(asset.browser_download_url, { headers: { 'User-Agent': UA } });
  if (!bin.ok) throw new Error('Could not download the game.');
  const dest = path.join(configDir(), EXE);
  fs.writeFileSync(dest, Buffer.from(await bin.arrayBuffer()), { mode: 0o755 });
  const cfg = loadConfig(); cfg.gamePath = dest; saveConfig(cfg);
  onStatus?.('Game ready.');
  return dest;
}

// Writes the session file the game reads once (auth.rs AuthSession::parse) and launches it.
function play(gamePath, account, args = []) {
  const cfg = loadConfig();
  const sessionFile = path.join(configDir(), `session-${Date.now()}.json`);
  fs.writeFileSync(sessionFile, JSON.stringify({
    name: account.name,
    uuid: account.uuid,
    access_token: account.accessToken,
    user_type: 'msa',
    xuid: account.xuid || '',
  }), { mode: 0o600 });
  const child = spawn(gamePath, ['--session-file', sessionFile, ...args], {
    cwd: cfg.gameDir || configDir(),
    detached: true,
    stdio: 'ignore',
    env: { ...process.env },
  });
  child.unref();
  return { pid: child.pid, game: gamePath };
}

module.exports = { loadConfig, saveConfig, findGame, downloadGame, play, configDir, EXE };
