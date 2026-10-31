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

const status = (s) => { if (win && !win.isDestroyed()) win.webContents.send('status', s); };

ipcMain.handle('state', async () => {
  let acct = { signedIn: false, owner: false, name: '' };
  try { acct = await game.accountState(); } catch { /* keep defaults */ }
  return {
    signedIn: acct.signedIn,
    owner: acct.owner,
    accountName: acct.name,
    engineFound: !!game.findLauncher() && !!game.findGameBundle(),
    usingPublicId: auth.clientId() === auth.DEFAULT_CLIENT_ID,
  };
});

ipcMain.handle('sign-in', async () => {
  // The seamless part: an embedded window on Microsoft's own page captures the code.
  const code = await signInWindow();
  // The proven Rust launcher exchanges it and stores the account.
  await game.signInWithCode(code);
  return true;
});

ipcMain.handle('sign-out', async () => {
  await game.signOut();
  return true;
});

ipcMain.handle('play', async (_e, args) => {
  return game.play(status, Array.isArray(args) ? args : []);
});

app.whenReady().then(createWindow);
app.on('window-all-closed', () => { if (process.platform !== 'darwin') app.quit(); });
app.on('activate', () => { if (BrowserWindow.getAllWindows().length === 0) createWindow(); });
