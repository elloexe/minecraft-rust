'use strict';
// The page's behaviour. All real work (sign-in, launch) happens in the main process over the
// `claudium` bridge from preload.js.
const $ = (sel) => document.querySelector(sel);

const els = {
  account: $('#account'),
  avatar: $('#avatar'),
  accountName: $('#account-name'),
  accountSub: $('#account-sub'),
  play: $('#play'),
  status: $('#status'),
  folder: $('#folder'),
  chooseGame: $('#choose-game'),
  gamePath: $('#game-path'),
  signOut: $('#sign-out'),
  clientIdNote: $('#client-id-note'),
};

let state = { account: null, gameFound: false };

function setStatus(text, kind = '') {
  els.status.textContent = text || '';
  els.status.className = `status ${kind}`;
}

function render() {
  const signedIn = !!state.account;
  if (signedIn) {
    els.avatar.textContent = (state.account.name[0] || 'C').toUpperCase();
    els.accountName.textContent = state.account.name;
    els.accountSub.textContent = 'Microsoft account';
  } else {
    els.avatar.textContent = 'C';
    els.accountName.textContent = 'Not signed in';
    els.accountSub.textContent = 'Click to sign in';
  }
  els.gamePath.textContent = state.gamePath || 'Not set';
  els.clientIdNote.textContent = state.usingPublicId
    ? 'Using the public launcher app id (no setup needed).'
    : 'Using a configured Microsoft app id.';

  // The Play button reflects what is missing.
  els.play.disabled = !(signedIn && state.gameFound);
  if (!signedIn) els.play.textContent = 'SIGN IN TO PLAY';
  else if (!state.gameFound) els.play.textContent = 'SET GAME IN SETTINGS';
  else els.play.textContent = 'PLAY';
}

async function refresh() {
  state = await window.claudium.state();
  render();
}

async function signIn() {
  setStatus('Opening Microsoft sign-in…');
  try {
    const account = await window.claudium.signIn();
    setStatus(`Signed in as ${account.name}.`, 'ok');
    await refresh();
  } catch (e) {
    setStatus(e.message || 'Sign-in failed.', 'error');
  }
}

els.account.addEventListener('click', () => {
  if (state.account) switchTab('settings');
  else signIn();
});

els.play.addEventListener('click', async () => {
  setStatus('Starting Claudium…');
  try {
    await window.claudium.play([]);
    setStatus('Claudium is starting.', 'ok');
  } catch (e) {
    setStatus(e.message || 'Could not start the game.', 'error');
  }
});

els.chooseGame.addEventListener('click', async () => {
  const picked = await window.claudium.chooseGame();
  if (picked) { setStatus('Game set.', 'ok'); await refresh(); }
});
els.folder.addEventListener('click', () => switchTab('settings'));
els.signOut.addEventListener('click', async () => {
  await window.claudium.signOut();
  setStatus('Signed out.');
  await refresh();
});

// Tabs
function switchTab(name) {
  document.querySelectorAll('.nav-item').forEach((b) => b.classList.toggle('active', b.dataset.tab === name));
  document.querySelectorAll('.tab').forEach((t) => t.classList.toggle('hidden', t.dataset.tab !== name));
}
document.querySelectorAll('.nav-item').forEach((b) => b.addEventListener('click', () => switchTab(b.dataset.tab)));

refresh();
