'use strict';
// Finding and launching the Claudium game (the Rust `claudium-client`), and a tiny JSON config.
const fs = require('fs');
const os = require('os');
const path = require('path');
const { spawn } = require('child_process');

const EXE = process.platform === 'win32' ? 'claudium-client.exe' : 'claudium-client';

// A small config file beside the OS config dir: the account and where the game binary is.
function configDir() {
  const base = process.env.CLAUDIUM_LAUNCHER_HOME
    || (process.platform === 'win32'
      ? path.join(process.env.APPDATA || os.homedir(), 'claudium-launcher')
      : path.join(process.env.XDG_CONFIG_HOME || path.join(os.homedir(), '.config'), 'claudium-launcher'));
  fs.mkdirSync(base, { recursive: true });
  return base;
}

function configPath() { return path.join(configDir(), 'launcher.json'); }

function loadConfig() {
  try { return JSON.parse(fs.readFileSync(configPath(), 'utf8')); } catch { return {}; }
}
function saveConfig(cfg) {
  fs.writeFileSync(configPath(), JSON.stringify(cfg, null, 2), { mode: 0o600 });
}

// Where the game binary might be: the configured path, next to the launcher, or on PATH.
function findGame(cfg) {
  const candidates = [
    cfg.gamePath,
    path.join(path.dirname(process.execPath), EXE),
    path.join(path.dirname(process.execPath), 'game', EXE),
    path.join(configDir(), EXE),
  ].filter(Boolean);
  for (const c of candidates) {
    try { if (fs.statSync(c).isFile()) return c; } catch { /* keep looking */ }
  }
  return null;
}

// Writes the session file the game reads once (auth.rs AuthSession::parse) and launches it.
function play(cfg, account, args = []) {
  const game = findGame(cfg);
  if (!game) throw new Error('Game not found. Set the Claudium game path in Settings.');
  const sessionFile = path.join(configDir(), `session-${Date.now()}.json`);
  fs.writeFileSync(sessionFile, JSON.stringify({
    name: account.name,
    uuid: account.uuid,
    access_token: account.accessToken,
    user_type: 'msa',
    xuid: account.xuid || '',
  }), { mode: 0o600 });

  const gameArgs = ['--session-file', sessionFile, ...args];
  const child = spawn(game, gameArgs, {
    cwd: cfg.gameDir || configDir(),
    detached: true,
    stdio: 'ignore',
    env: { ...process.env },
  });
  child.unref();
  return { pid: child.pid, game };
}

module.exports = { loadConfig, saveConfig, findGame, play, configDir, EXE };
