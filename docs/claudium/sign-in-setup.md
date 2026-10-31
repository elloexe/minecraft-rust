# Microsoft sign-in: the copy-paste setup

Goal: your own Mojang-approved client ID so you (and your brother) sign in with
your own Microsoft accounts. Do this once. The only slow part is Mojang's review
(days) — start it now so it runs while you play offline.

## Step 1 — Azure (portal.azure.com), ~5 min

Microsoft Entra ID → App registrations → **New registration**. Paste these:

- **Name:** `Claudium Launcher`
- **Supported account types:** Personal Microsoft accounts only
- **Redirect URI:** leave blank. If the form forces one, pick platform
  "Mobile and desktop applications" and value:
  `https://login.microsoftonline.com/common/oauth2/nativeclient`

Click **Register**. Then:

- Left menu → **Authentication** → Advanced settings →
  **Allow public client flows: Yes** → Save.
- Copy the **Application (client) ID** (a GUID like `xxxxxxxx-xxxx-...`). That is
  your `CLAUDIUM_MSA_CLIENT_ID`.

No client secret, no API permissions to add.

## Step 2 — Mojang review (aka.ms/mce-reviewappid), ~3 min then wait

Open the form and paste:

- **Application name:** `Claudium Launcher`
- **Client ID:** the GUID from step 1
- **Project URL:** `https://github.com/elloexe/minecraft-rust`
  (⚠ must be reachable when they review — keep the repo public until approved,
  or give a public page instead)
- **Description:**
  `Open-source launcher and client for Minecraft: Java Edition. It signs players
  into their own Microsoft accounts to play a copy of the game they own, using
  the standard XboxLive.signin offline_access scopes via the OAuth device-code
  flow. No account sharing, no offline-mode bypass.`

Submit. Approval is days to weeks; you get an email.

## Step 3 — use it (once approved)

Put the GUID on one line in `scripts/client-id.txt` and launch with
`scripts/run-claudium.bat` (or `.sh`). Each machine needs the file once. When you
make the repo private, this still works — the ID lives on your machines, not in
the repo.

## Until approved

Play singleplayer and together on LAN/offline (join by username) — no sign-in
needed. Online-mode public servers and the one-click Play button wait for the
approval.
