'use strict';
// Claudium launcher — Electron main process. No local web server (the old client had one, which
// made it slow to start): the window loads the renderer directly, and talks to this process over
// a tiny, locked-down IPC bridge (preload.js).
const { app, BrowserWindow, ipcMain, dialog, shell, nativeImage } = require('electron');
const path = require('path');
const auth = require('./auth');
const game = require('./game');

let win = null;

function icon() {
  try { return nativeImage.createFromPath(path.join(__dirname, 'build', 'icon.png')); } catch { return undefined; }
}

function createWindow() {
  win = new BrowserWindow({
    width: 1100,
    height: 720,
    minWidth: 900,
    minHeight: 600,
    title: 'Claudium',
    backgroundColor: '#1b1b1b',
    autoHideMenuBar: true,
    icon: icon(),
    webPreferences: {
      preload: path.join(__dirname, 'preload.js'),
      contextIsolation: true,
      sandbox: true,
    },
  });
  win.loadFile(path.join(__dirname, 'renderer', 'index.html'));
  win.webContents.setWindowOpenHandler(({ url }) => {
    if (/^https?:\/\//.test(url)) shell.openExternal(url);
    return { action: 'deny' };
  });
}

// The seamless Microsoft sign-in: an embedded window on Microsoft's own page. We never see the
// password; we only read the `code` when Microsoft redirects to oauth20_desktop.srf.
function signInWindow() {
  return new Promise((resolve, reject) => {
    const authWin = new BrowserWindow({
      width: 520,
      height: 720,
      parent: win,
      modal: true,
      title: 'Sign in with Microsoft',
      backgroundColor: '#ffffff',
      autoHideMenuBar: true,
      webPreferences: { contextIsolation: true, sandbox: true, partition: `ms-login-${Date.now()}` },
    });
    authWin.loadURL(auth.authorizeUrl());
    let settled = false;
    const finish = (fn, arg) => {
      if (settled) return;
      settled = true;
      fn(arg);
      if (!authWin.isDestroyed()) authWin.close();
    };
    const check = (navUrl) => {
      if (!navUrl || !navUrl.startsWith(auth.REDIRECT_URI)) return;
      let params;
      try { params = new URL(navUrl).searchParams; } catch { return; }
      const code = params.get('code');
      const error = params.get('error');
      if (code) finish(resolve, code);
      else if (error) finish(reject, new Error(params.get('error_description') || error));
    };
    authWin.webContents.on('will-redirect', (_e, u) => check(u));
    authWin.webContents.on('will-navigate', (_e, u) => check(u));
    authWin.webContents.on('did-navigate', (_e, u) => check(u));
    authWin.on('closed', () => { if (!settled) { settled = true; reject(new Error('The sign-in window was closed.')); } });
  });
}

// ----- IPC: the renderer asks, the main process does -------------------------------------------

ipcMain.handle('state', () => {
  const cfg = game.loadConfig();
  return {
    account: cfg.account ? { name: cfg.account.name, uuid: cfg.account.uuid } : null,
    gamePath: cfg.gamePath || null,
    gameFound: !!game.findGame(cfg),
    clientId: auth.clientId(),
    usingPublicId: auth.clientId() === auth.DEFAULT_CLIENT_ID,
  };
});

ipcMain.handle('sign-in', async () => {
  const code = await signInWindow();
  const account = await auth.codeToAccount(code);
  const cfg = game.loadConfig();
  cfg.account = account;
  game.saveConfig(cfg);
  return { name: account.name, uuid: account.uuid };
});

ipcMain.handle('sign-out', () => {
  const cfg = game.loadConfig();
  delete cfg.account;
  game.saveConfig(cfg);
  return true;
});

ipcMain.handle('choose-game', async () => {
  const res = await dialog.showOpenDialog(win, {
    title: 'Choose the Claudium game executable',
    properties: ['openFile'],
    defaultPath: game.configDir(),
  });
  if (res.canceled || !res.filePaths[0]) return null;
  const cfg = game.loadConfig();
  cfg.gamePath = res.filePaths[0];
  game.saveConfig(cfg);
  return cfg.gamePath;
});

ipcMain.handle('play', async (_e, args) => {
  const status = (s) => { if (win && !win.isDestroyed()) win.webContents.send('status', s); };
  let cfg = game.loadConfig();
  if (!cfg.account) throw new Error('Sign in with Microsoft first.');
  // The game ships inside the installer; if it is not there (a dev build), fetch it once.
  let gamePath = game.findGame(cfg);
  if (!gamePath) {
    gamePath = await game.downloadGame(status);
    cfg = game.loadConfig();
  }
  // Refresh the Minecraft token if it is stale, so online servers accept it.
  if (!cfg.account.expiresAt || Date.now() > cfg.account.expiresAt - 60_000) {
    if (cfg.account.refresh) {
      cfg.account = await auth.refresh(cfg.account.refresh);
      game.saveConfig(cfg);
    }
  }
  status('Starting Claudium…');
  return game.play(gamePath, cfg.account, Array.isArray(args) ? args : []);
});

app.whenReady().then(createWindow);
app.on('window-all-closed', () => { if (process.platform !== 'darwin') app.quit(); });
app.on('activate', () => { if (BrowserWindow.getAllWindows().length === 0) createWindow(); });
