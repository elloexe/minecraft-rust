'use strict';
// The only bridge between the page and the main process: a few named actions, nothing else.
const { contextBridge, ipcRenderer } = require('electron');

contextBridge.exposeInMainWorld('claudium', {
  state: () => ipcRenderer.invoke('state'),
  signIn: () => ipcRenderer.invoke('sign-in'),
  signOut: () => ipcRenderer.invoke('sign-out'),
  play: (args) => ipcRenderer.invoke('play', args),
  onStatus: (cb) => ipcRenderer.on('status', (_e, text) => cb(text)),
});
