//! Microsoft account sign-in (device code flow) and session-server join.
//!
//! Third-party clients need their own Azure app registration that Mojang has
//! approved for the game-services API. Pass its client id with `--client-id`
//! or the `CLAUDIUM_CLIENT_ID` environment variable.

use std::path::PathBuf;
use std::time::{Duration, Instant};

use anyhow::{Context, anyhow, bail};
use serde_json::{Value, json};

#[derive(Clone, Debug)]
pub struct Profile {
    pub name: String,
    pub uuid: u128,
    pub access_token: String,
}

fn cache_path() -> PathBuf {
    crate::data_dir().join("auth.json")
}

fn post_form(url: &str, form: &[(&str, &str)]) -> anyhow::Result<Value> {
    match ureq::post(url).send_form(form) {
        Ok(r) => Ok(r.into_json()?),
        Err(ureq::Error::Status(_, r)) => Ok(r.into_json()?),
        Err(e) => Err(e.into()),
    }
}

fn post_json(url: &str, body: Value) -> anyhow::Result<Value> {
    match ureq::post(url).set("Accept", "application/json").send_json(body) {
        Ok(r) => Ok(r.into_json()?),
        Err(ureq::Error::Status(code, r)) => {
            let text = r.into_string().unwrap_or_default();
            bail!("{url} returned {code}: {text}")
        }
        Err(e) => Err(e.into()),
    }
}

fn microsoft_token(client_id: &str) -> anyhow::Result<(String, String)> {
    // Try the cached refresh token first.
    if let Ok(s) = std::fs::read_to_string(cache_path()) {
        if let Ok(v) = serde_json::from_str::<Value>(&s) {
            if let Some(rt) = v.get("refresh_token").and_then(|t| t.as_str()) {
                let r = post_form(
                    "https://login.microsoftonline.com/consumers/oauth2/v2.0/token",
                    &[
                        ("client_id", client_id),
                        ("grant_type", "refresh_token"),
                        ("refresh_token", rt),
                        ("scope", "XboxLive.signin offline_access"),
                    ],
                )?;
                if let (Some(at), Some(rt)) = (r["access_token"].as_str(), r["refresh_token"].as_str()) {
                    return Ok((at.to_string(), rt.to_string()));
                }
            }
        }
    }

    let dc = post_form(
        "https://login.microsoftonline.com/consumers/oauth2/v2.0/devicecode",
        &[("client_id", client_id), ("scope", "XboxLive.signin offline_access")],
    )?;
    let device_code = dc["device_code"].as_str().ok_or_else(|| anyhow!("device code request failed: {dc}"))?;
    let message = dc["message"].as_str().unwrap_or("Open the sign-in page and enter the code shown.");
    println!("\n== Microsoft sign-in ==\n{message}\n");
    let interval = dc["interval"].as_u64().unwrap_or(5);
    let expires = Instant::now() + Duration::from_secs(dc["expires_in"].as_u64().unwrap_or(900));
    while Instant::now() < expires {
        std::thread::sleep(Duration::from_secs(interval));
        let r = post_form(
            "https://login.microsoftonline.com/consumers/oauth2/v2.0/token",
            &[("client_id", client_id), ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"), ("device_code", device_code)],
        )?;
        if let Some(at) = r["access_token"].as_str() {
            let rt = r["refresh_token"].as_str().unwrap_or_default().to_string();
            return Ok((at.to_string(), rt));
        }
        match r["error"].as_str() {
            Some("authorization_pending") | Some("slow_down") => continue,
            Some(e) => bail!("sign-in failed: {e}"),
            None => bail!("sign-in failed: {r}"),
        }
    }
    bail!("sign-in timed out")
}

/// Signs in and returns the game profile. Caches the refresh token.
pub fn login(client_id: &str) -> anyhow::Result<Profile> {
    let (ms_token, refresh) = microsoft_token(client_id)?;
    let _ = std::fs::create_dir_all(crate::data_dir());
    let _ = std::fs::write(cache_path(), json!({ "refresh_token": refresh }).to_string());

    let xbl = post_json(
        "https://user.auth.xboxlive.com/user/authenticate",
        json!({
            "Properties": { "AuthMethod": "RPS", "SiteName": "user.auth.xboxlive.com", "RpsTicket": format!("d={ms_token}") },
            "RelyingParty": "http://auth.xboxlive.com",
            "TokenType": "JWT"
        }),
    )
    .context("Xbox Live sign-in")?;
    let xbl_token = xbl["Token"].as_str().ok_or_else(|| anyhow!("no Xbox Live token"))?;

    let xsts = post_json(
        "https://xsts.auth.xboxlive.com/xsts/authorize",
        json!({
            "Properties": { "SandboxId": "RETAIL", "UserTokens": [xbl_token] },
            "RelyingParty": "rp://api.minecraftservices.com/",
            "TokenType": "JWT"
        }),
    )
    .context("XSTS authorization (does this account own the game?)")?;
    let xsts_token = xsts["Token"].as_str().ok_or_else(|| anyhow!("no XSTS token"))?;
    let uhs = xsts["DisplayClaims"]["xui"][0]["uhs"].as_str().ok_or_else(|| anyhow!("no user hash"))?;

    let game = post_json(
        "https://api.minecraftservices.com/authentication/login_with_xbox",
        json!({ "identityToken": format!("XBL3.0 x={uhs};{xsts_token}") }),
    )
    .context("game services login (the client id must be approved by Mojang)")?;
    let access = game["access_token"].as_str().ok_or_else(|| anyhow!("no game access token"))?.to_string();

    let profile: Value = ureq::get("https://api.minecraftservices.com/minecraft/profile")
        .set("Authorization", &format!("Bearer {access}"))
        .call()
        .context("fetching profile (does this account own the game?)")?
        .into_json()?;
    let name = profile["name"].as_str().ok_or_else(|| anyhow!("no profile name"))?.to_string();
    let id = profile["id"].as_str().ok_or_else(|| anyhow!("no profile id"))?;
    let uuid = u128::from_str_radix(id, 16)?;
    Ok(Profile { name, uuid, access_token: access })
}

/// The server-id hash: SHA-1 printed as a signed big-endian number in hex.
pub fn server_hash(server_id: &str, secret: &[u8], public_key: &[u8]) -> String {
    use sha1::{Digest, Sha1};
    let mut h = Sha1::new();
    h.update(server_id.as_bytes());
    h.update(secret);
    h.update(public_key);
    let digest = h.finalize();
    num_bigint::BigInt::from_signed_bytes_be(&digest).to_str_radix(16)
}

/// Tells the session server we're joining, so an online-mode server can verify us.
pub fn join_server(profile: &Profile, hash: &str) -> anyhow::Result<()> {
    let r = ureq::post("https://sessionserver.mojang.com/session/minecraft/join").send_json(json!({
        "accessToken": profile.access_token,
        "selectedProfile": format!("{:032x}", profile.uuid),
        "serverId": hash,
    }));
    match r {
        Ok(_) => Ok(()),
        Err(ureq::Error::Status(code, resp)) => bail!("session join failed ({code}): {}", resp.into_string().unwrap_or_default()),
        Err(e) => Err(e.into()),
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn known_hashes() {
        // Well-known test vectors for the signed-hex digest.
        let h = |s: &str| super::server_hash(s, b"", b"");
        assert_eq!(h("Notch"), "4ed1f46bbe04bc756bcb17c0c7ce3e4632f06a48");
        assert_eq!(h("jeb_"), "-7c9d5b0044c130109a5d7b5fb5c317c02b4e28c1");
        assert_eq!(h("simon"), "88e16a1019277b15d58faf0541e11910eb756f6");
    }
}
