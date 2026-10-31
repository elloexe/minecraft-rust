'use strict';
// Microsoft sign-in, ported from the proven flow in the user's prior client.
//
// The public Minecraft-launcher client id signs in through login.live.com's authorization-code
// flow, which it is built for and which Mojang already approves — so no Azure app registration is
// needed. Microsoft -> Xbox Live -> XSTS -> Minecraft tokens are exchanged after sign-in. The
// access token is only kept in memory and in the session file the game reads once.

const AUTH_BASE = 'https://login.live.com';
const REDIRECT_URI = 'https://login.live.com/oauth20_desktop.srf';
// The official launcher's public client id (overridable with CLAUDIUM_MSA_CLIENT_ID).
const DEFAULT_CLIENT_ID = '00000000402b5328';
const SCOPE = 'XboxLive.signin offline_access';
const UA = 'claudium-launcher/0.1';

function clientId() {
  const id = (process.env.CLAUDIUM_MSA_CLIENT_ID || '').trim();
  return id || DEFAULT_CLIENT_ID;
}

function authorizeUrl() {
  const q = new URLSearchParams({
    client_id: clientId(),
    response_type: 'code',
    redirect_uri: REDIRECT_URI,
    scope: SCOPE,
    prompt: 'select_account',
  });
  return `${AUTH_BASE}/oauth20_authorize.srf?${q.toString()}`;
}

async function post(url, body, form = false) {
  const res = await fetch(url, {
    method: 'POST',
    headers: {
      'User-Agent': UA,
      Accept: 'application/json',
      'Content-Type': form ? 'application/x-www-form-urlencoded' : 'application/json',
    },
    body: form ? new URLSearchParams(body).toString() : JSON.stringify(body),
  });
  const text = await res.text();
  let data = {};
  try { data = text ? JSON.parse(text) : {}; } catch { data = { raw: text }; }
  return { ok: res.ok, status: res.status, data };
}

const XERR = {
  2148916233: 'This Microsoft account has no Xbox profile yet. Sign in once at xbox.com, then try again.',
  2148916235: 'Xbox Live is not available in your country.',
  2148916236: 'This account needs adult verification on xbox.com first.',
  2148916237: 'This account needs adult verification on xbox.com first.',
  2148916238: 'This is a child account. An adult has to add it to a Microsoft family first.',
};

// An authorization code (captured from the sign-in window) -> Minecraft account + tokens.
async function codeToAccount(code) {
  const tok = await post(`${AUTH_BASE}/oauth20_token.srf`, {
    client_id: clientId(),
    code,
    grant_type: 'authorization_code',
    redirect_uri: REDIRECT_URI,
    scope: SCOPE,
  }, true);
  if (!tok.ok) throw new Error(`Microsoft sign-in failed: ${tok.data.error_description || tok.data.error || tok.status}`);
  return finish(tok.data);
}

async function refresh(refreshToken) {
  const tok = await post(`${AUTH_BASE}/oauth20_token.srf`, {
    grant_type: 'refresh_token',
    client_id: clientId(),
    refresh_token: refreshToken,
    scope: SCOPE,
  }, true);
  if (!tok.ok) throw new Error('Your Microsoft sign-in has expired. Sign in again.');
  return finish(tok.data);
}

async function finish(tokenData) {
  const mc = await minecraftLogin(tokenData.access_token);
  return {
    type: 'msa',
    uuid: mc.uuid,
    name: mc.name,
    xuid: mc.xuid,
    accessToken: mc.accessToken,
    refresh: tokenData.refresh_token,
    expiresAt: mc.expiresAt,
  };
}

async function minecraftLogin(msAccessToken) {
  const xbl = await post('https://user.auth.xboxlive.com/user/authenticate', {
    Properties: { AuthMethod: 'RPS', SiteName: 'user.auth.xboxlive.com', RpsTicket: `d=${msAccessToken}` },
    RelyingParty: 'http://auth.xboxlive.com',
    TokenType: 'JWT',
  });
  if (!xbl.ok) throw new Error('Xbox Live sign-in failed. Try again in a minute.');
  const uhs = xbl.data.DisplayClaims?.xui?.[0]?.uhs;
  const xsts = await post('https://xsts.auth.xboxlive.com/xsts/authorize', {
    Properties: { SandboxId: 'RETAIL', UserTokens: [xbl.data.Token] },
    RelyingParty: 'rp://api.minecraftservices.com/',
    TokenType: 'JWT',
  });
  if (!xsts.ok) throw new Error(XERR[xsts.data?.XErr] || 'Xbox Live would not let this account sign in to Minecraft.');
  const xuid = xsts.data.DisplayClaims?.xui?.[0]?.xid;
  const mc = await post('https://api.minecraftservices.com/authentication/login_with_xbox', {
    identityToken: `XBL3.0 x=${uhs};${xsts.data.Token}`,
  });
  if (!mc.ok) {
    if (mc.status === 403) throw new Error('Minecraft sign-in refused this app (the Microsoft app id is not approved by Mojang).');
    throw new Error('Minecraft sign-in failed. Try again in a minute.');
  }
  const res = await fetch('https://api.minecraftservices.com/minecraft/profile', {
    headers: { 'User-Agent': UA, Authorization: `Bearer ${mc.data.access_token}` },
  });
  if (res.status === 404) throw new Error("This Microsoft account doesn't own Minecraft: Java Edition.");
  if (!res.ok) throw new Error('Could not read your Minecraft profile. Try again in a minute.');
  const profile = await res.json();
  return {
    uuid: profile.id,
    name: profile.name,
    xuid: xuid || '',
    accessToken: mc.data.access_token,
    expiresAt: Date.now() + 23 * 60 * 60 * 1000,
  };
}

module.exports = { authorizeUrl, codeToAccount, refresh, REDIRECT_URI, clientId, DEFAULT_CLIENT_ID };
