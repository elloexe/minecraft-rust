# Running Claudium with sign-in (you + your brother)

The game plays singleplayer and offline/LAN with no setup. To use the launcher's
Play button and join online-mode servers, each machine needs a Microsoft client
ID in `CLAUDIUM_MSA_CLIENT_ID`.

1. Put the client ID on one line in `client-id.txt` next to these scripts.
   (`client-id.txt` is git-ignored, so it never gets committed.)
2. Run `run-claudium.bat` (Windows) or `./run-claudium.sh` (Linux/macOS)
   instead of the exe directly. It sets the variable, then starts the launcher.

Getting a client ID:
- **Your own (permanent):** register an Azure app + Mojang review — see
  `docs/microsoft-account.md`. Doesn't break, no dependency on anyone else.
- **Temporary:** a known public Minecraft-launcher client ID also works until
  yours is approved. It's not committed here; paste it into `client-id.txt`.
