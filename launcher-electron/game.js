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

// Run a claudium-launcher subcommand, streaming each output line to `onLine`. Resolves with the
// exit code; rejects only if the binary can't be started.
function run(args, onLine = () => {}) {
  return new Promise((resolve, reject) => {
    const bin = findLauncher();
    if (!bin) return reject(new Error('The Claudium launcher engine is missing from this build.'));
    const child = spawn(bin, args, {
      env: { ...process.env, CLAUDIUM_LAUNCHER_HOME: dataHome() },
    });
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

async function signOut(onLine) {
  // Sign out whoever is selected, if any (ignore "no account").
  await run(['account', 'sign-out', '*'], onLine).catch(() => {});
}

// Whether an account is signed in, parsed from `account list`.
async function isSignedIn() {
  const r = await run(['account', 'list']);
  // A signed-in account prints a line; none prints the NEEDS_OWNER hint.
  return r.code === 0 && !/sign in/i.test(r.output) || /microsoft/i.test(r.output);
}

// Install the game if needed, prepare it from the player's Minecraft, then launch. Each step
// streams its progress; the Rust launcher is idempotent, so re-running is cheap once done.
async function play(onLine, extraArgs = []) {
  onLine('Checking the game is installed…');
  await run(['install', 'latest'], onLine);
  onLine('Preparing the game from your Minecraft (first run can take a few minutes)…');
  const prep = await run(['prepare'], onLine);
  if (prep.code !== 0) throw new Error(prep.output.trim() || 'Could not prepare the game. Is Minecraft: Java Edition installed?');
  onLine('Starting Claudium…');
  const played = await run(['play', ...extraArgs], onLine);
  if (played.code !== 0) throw new Error(played.output.trim() || 'The game did not start.');
  return { ok: true };
}

module.exports = { findLauncher, run, signInWithCode, signOut, isSignedIn, play, dataHome };
