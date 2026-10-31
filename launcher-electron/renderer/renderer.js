'use strict';
// The page's behaviour. All real work (sign-in, prepare, launch) happens in the main process and
// the bundled Rust launcher, over the `claudium` bridge from preload.js.
const $ = (sel) => document.querySelector(sel);

const els = {
  account: $('#account'),
  avatar: $('#avatar'),
  accountName: $('#account-name'),
  accountSub: $('#account-sub'),
  play: $('#play'),
  status: $('#status'),
  folder: $('#folder'),
  signOut: $('#sign-out'),
  clientIdNote: $('#client-id-note'),
};

let state = { signedIn: false, engineFound: false };
let busy = false;

function setStatus(text, kind = '') {
  els.status.textContent = text || '';
  els.status.className = `status ${kind}`;
}

function render() {
  if (state.owner) {
    els.avatar.textContent = (state.accountName || 'C').charAt(0).toUpperCase();
    els.accountName.textContent = state.accountName || 'Signed in';
    els.accountSub.textContent = 'Microsoft · owns Minecraft';
  } else if (state.signedIn) {
    els.avatar.textContent = '!';
    els.accountName.textContent = state.accountName || 'Signed in';
    els.accountSub.textContent = "Doesn't own Minecraft — click to switch";
  } else {
    els.avatar.textContent = 'C';
    els.accountName.textContent = 'Not signed in';
    els.accountSub.textContent = 'Click to sign in';
  }
  els.clientIdNote.textContent = state.usingPublicId
    ? 'Using the public launcher app id (no setup needed).'
    : 'Using a configured Microsoft app id.';

  els.play.disabled = busy || !(state.owner && state.engineFound);
  if (busy) els.play.textContent = 'WORKING…';
  else if (!state.owner) els.play.textContent = 'SIGN IN TO PLAY';
  else if (!state.engineFound) els.play.textContent = 'ENGINE MISSING';
  else els.play.textContent = 'PLAY';
}

async function refresh() {
  state = await window.claudium.state();
  render();
  if (state.signedIn && !state.owner) {
    setStatus("That account doesn't own Minecraft: Java Edition. Click your name to sign in with the one that does (or Sign out in Settings).", 'error');
  }
}

els.account.addEventListener('click', async () => {
  // If a good account is signed in, the chip opens Settings. Otherwise (no account, or one that
  // doesn't own Minecraft) it opens Microsoft sign-in so you can sign in / switch accounts.
  if (state.owner) { switchTab('settings'); return; }
  setStatus('Opening Microsoft sign-in…');
  try {
    if (state.signedIn) { await window.claudium.signOut(); } // drop the wrong account first
    await window.claudium.signIn();
    await refresh();
    if (state.owner) setStatus('Signed in.', 'ok');
  } catch (e) {
    setStatus(e.message || 'Sign-in failed.', 'error');
  }
});

els.play.addEventListener('click', async () => {
  busy = true; render();
  try {
    await window.claudium.play([]);
    setStatus('Claudium is running.', 'ok');
  } catch (e) {
    setStatus(e.message || 'Could not start the game.', 'error');
  } finally {
    busy = false; render();
  }
});

els.folder.addEventListener('click', () => switchTab('settings'));
els.signOut.addEventListener('click', async () => {
  await window.claudium.signOut();
  setStatus('Signed out.');
  await refresh();
});

function switchTab(name) {
  document.querySelectorAll('.nav-item').forEach((b) => b.classList.toggle('active', b.dataset.tab === name));
  document.querySelectorAll('.tab').forEach((t) => t.classList.toggle('hidden', t.dataset.tab !== name));
}
document.querySelectorAll('.nav-item').forEach((b) => b.addEventListener('click', () => switchTab(b.dataset.tab)));

// Progress from the main process and the Rust launcher (install, prepare, launch).
window.claudium.onStatus((text) => setStatus(text));

refresh();
