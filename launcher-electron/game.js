'use strict';
// Thin wrapper over the bundled Rust launcher (claudium-launcher), which owns the proven
// pipeline: the account store, downloading/installing the game, preparing resources from the
// player's Minecraft (jar + Java), and launching. The Electron side only adds the seamless
// sign-in window (it captures the code and hands it to `account sign-in --code`) and the UI.
const fs = require('fs');
const os = require('os');
const path = require('path');
const { spawn } = require('child_process');

const BIN = process.platform === 'win32' ? 'claudium-launcher.exe' : 'claudium-launcher';

// A stable data home so the account and prepared files persist between runs.
function dataHome() {
  const base = process.env.CLAUDIUM_LAUNCHER_HOME
    || (process.platform === 'win32'
      ? path.join(process.env.APPDATA || os.homedir(), 'Claudium')
      : path.join(process.env.XDG_DATA_HOME || path.join(os.homedir(), '.local', 'share'), 'claudium'));
  fs.mkdirSync(base, { recursive: true });
  return base;
}

// Where the bundled Rust launcher is: in the installer's resources, or next to the app in dev.
function findLauncher() {
  const candidates = [
    process.env.CLAUDIUM_LAUNCHER_BIN,
    process.resourcesPath && path.join(process.resourcesPath, 'bin', BIN),
    path.join(path.dirname(process.execPath), BIN),
    path.join(__dirname, 'bin', BIN),
  ].filter(Boolean);
  for (const c of candidates) {
    try { if (fs.statSync(c).isFile()) return c; } catch { /* keep looking */ }
  }
  return null;
}

// Where the bundled game folder is (the claudium-client binary + tools + research-lock.json),
// shipped inside the installer's resources, or beside the app in dev.
function findGameBundle() {
  const candidates = [
    process.env.CLAUDIUM_GAME_DIR,
    process.resourcesPath && path.join(process.resourcesPath, 'game'),
    path.join(path.dirname(process.execPath), 'game'),
    path.join(__dirname, 'game'),
  ].filter(Boolean);
  const client = process.platform === 'win32' ? 'claudium-client.exe' : 'claudium-client';
  for (const c of candidates) {
    try { if (fs.statSync(path.join(c, client)).isFile()) return c; } catch { /* keep looking */ }
  }
  return null;
}

// The Java runtime bundled in the installer, used to run the precompiled exporters when preparing
// the game from the player's Minecraft — so no Java install and no Mojang download. Returns its
// home, or null. (Exporters are precompiled in CI, so a runtime is enough; no compiler needed.)
function findJava() {
  const bin = process.platform === 'win32' ? 'java.exe' : 'java';
  const dirs = ['runtime', 'jdk'];
  const bases = [process.resourcesPath, path.dirname(process.execPath), __dirname].filter(Boolean);
  const envp = process.env.CLAUDIUM_BUNDLED_JAVA;
  if (envp) { try { if (fs.statSync(path.join(envp, 'bin', bin)).isFile()) return envp; } catch { /* fall through */ } }
  for (const base of bases) {
    for (const d of dirs) {
      const home = path.join(base, d);
      try { if (fs.statSync(path.join(home, 'bin', bin)).isFile()) return home; } catch { /* keep looking */ }
    }
  }
  return null;
}

// The version tag to install the bundle under: the Minecraft version it targets, from its
// research-lock.json, so the launcher shows "26.3". Falls back to "bundled".
function bundleTag(gameDir) {
  try {
    const lock = JSON.parse(fs.readFileSync(path.join(gameDir, 'research-lock.json'), 'utf8'));
    return (lock && lock.minecraft && lock.minecraft.version) || 'bundled';
  } catch { return 'bundled'; }
}

// Run a claudium-launcher subcommand, streaming each output line to `onLine`. Resolves with the
// exit code; rejects only if the binary can't be started.
function run(args, onLine = () => {}) {
  return new Promise((resolve, reject) => {
    const bin = findLauncher();
    if (!bin) return reject(new Error('The Claudium launcher engine is missing from this build.'));
    const env = { ...process.env, CLAUDIUM_LAUNCHER_HOME: dataHome() };
    // Give every engine call the bundled Java runtime, so preparing needs no Java install and no
    // download of Mojang's runtime: JAVA_HOME for the engine, and its bin on PATH.
    const java = findJava();
    if (java) {
      env.JAVA_HOME = java;
      const sep = process.platform === 'win32' ? ';' : ':';
      env.PATH = path.join(java, 'bin') + sep + (env.PATH || '');
    }
    const child = spawn(bin, args, { env });
    let out = '';
    const feed = (buf) => {
      out += buf.toString();
      let nl;
      while ((nl = out.indexOf('\n')) >= 0) {
        const line = out.slice(0, nl).trimEnd();
        out = out.slice(nl + 1);
        if (line) onLine(line);
      }
    };
    const tail = [];
    child.stdout.on('data', (b) => { feed(b); tail.push(b.toString()); });
    child.stderr.on('data', (b) => { feed(b); tail.push(b.toString()); });
    child.on('error', reject);
    child.on('close', (code) => resolve({ code, output: tail.join('').slice(-4000) }));
  });
}

async function signInWithCode(code) {
  const r = await run(['account', 'sign-in', '--code', code]);
  if (r.code !== 0) throw new Error(r.output.trim() || 'Sign-in failed.');
}

// The real sign-in state, from the engine's JSON (the source of truth):
//   { signedIn, owner (owns Minecraft), name, configured, uuids }
async function accountState() {
  const r = await run(['account', 'list', '--json']);
  try {
    const s = r.output.slice(r.output.indexOf('{'), r.output.lastIndexOf('}') + 1);
    const d = JSON.parse(s);
    const accts = Array.isArray(d.accounts) ? d.accounts : [];
    const sel = accts.find((a) => a.selected) || accts[0] || null;
    return {
      signedIn: accts.length > 0,
      owner: d.owner === true,
      name: sel ? sel.name : '',
      configured: d.sign_in_configured !== false,
      uuids: accts.map((a) => a.uuid),
    };
  } catch {
    return { signedIn: false, owner: false, name: '', configured: true, uuids: [] };
  }
}

async function isSignedIn() {
  return (await accountState()).signedIn;
}

// Sign out every signed-in account (by UUID — the engine needs a real name/UUID, not "*").
async function signOut(onLine) {
  const st = await accountState();
  for (const uuid of st.uuids) {
    await run(['account', 'sign-out', uuid], onLine).catch(() => {});
  }
}

// Install the game if needed, prepare it from the player's Minecraft, then launch. Each step
// streams its progress; the Rust launcher is idempotent, so re-running is cheap once done.
async function play(onLine, extraArgs = []) {
  const gameDir = findGameBundle();
  if (!gameDir) throw new Error('The game is missing from this build (no bundled game folder).');
  const tag = bundleTag(gameDir);

  onLine('Setting up the game…');
  const inst = await run(['install', tag, '--from', gameDir], onLine);
  if (inst.code !== 0) throw new Error(inst.output.trim() || 'Could not set up the bundled game.');

  onLine('Preparing the game from your Minecraft (first run can take a few minutes)…');
  const prep = await run(['prepare', '--version', tag], onLine);
  if (prep.code !== 0) throw new Error(prep.output.trim() || 'Could not prepare the game. Is Minecraft: Java Edition installed, and Java available?');

  onLine('Starting Claudium…');
  const played = await run(['play', '--version', tag, ...extraArgs], onLine);
  if (played.code !== 0) throw new Error(played.output.trim() || 'The game did not start.');
  return { ok: true };
}

module.exports = { findLauncher, findGameBundle, run, signInWithCode, signOut, isSignedIn, accountState, play, dataHome };
