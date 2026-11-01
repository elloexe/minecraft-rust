#!/usr/bin/env python3
"""Download the pinned vanilla Minecraft files into a .minecraft-style directory, for CI to
compile the exporters (claudium-prepare compile-exporters). The files are public and used only on
the build runner; nothing of Mojang's is redistributed (only our own compiled exporters ship).

Usage: fetch_vanilla.py <out_dir>   (reads research-lock.json from the repo root / cwd)

Writes:
  <out_dir>/versions/<v>/<v>.json, <v>.jar   (client)
  <out_dir>/server.jar                        (passed to --server-jar)
  <out_dir>/libraries/...                     (downloads.artifact of each library)
  <out_dir>/assets/indexes/<id>.json          (asset index; objects are not fetched)

Skips the ~370 MB of asset objects: compile-exporters needs the jars and libraries, not sounds.
Run with -I; it uses only the standard library.
"""
import hashlib
import json
import os
import sys
import urllib.request

MANIFEST = "https://launchermeta.mojang.com/mc/game/version_manifest_v2.json"


def get(url):
    with urllib.request.urlopen(url, timeout=60) as r:
        return r.read()


def sha1(data):
    h = hashlib.sha1()
    h.update(data)
    return h.hexdigest()


def download(url, dest, want_sha1=None):
    os.makedirs(os.path.dirname(dest), exist_ok=True)
    if os.path.exists(dest):
        if want_sha1 is None or sha1(open(dest, "rb").read()) == want_sha1:
            return
    data = get(url)
    if want_sha1 and sha1(data) != want_sha1:
        raise SystemExit(f"sha1 mismatch for {url}: got {sha1(data)}, want {want_sha1}")
    with open(dest, "wb") as f:
        f.write(data)


def main():
    out = sys.argv[1]
    lock = json.load(open("research-lock.json"))
    mc = lock["minecraft"]
    version = mc["version"]
    accepted = set(mc.get("accepted_manifest_sha1", []))
    accepted.add(mc["manifest_sha1"])

    manifest = json.loads(get(MANIFEST))
    entry = next((v for v in manifest["versions"] if v["id"] == version), None)
    if not entry:
        raise SystemExit(f"version {version} not in Mojang's manifest")
    # The version JSON's sha1 is in its url (.../packages/<sha1>/<v>.json); check it is one we pin.
    url_sha = entry["url"].rsplit("/", 2)[-2]
    if url_sha not in accepted:
        print(f"warning: version JSON sha1 {url_sha} not in research-lock accepted list", flush=True)
    vjson_raw = get(entry["url"])
    vdir = os.path.join(out, "versions", version)
    os.makedirs(vdir, exist_ok=True)
    open(os.path.join(vdir, f"{version}.json"), "wb").write(vjson_raw)
    vjson = json.loads(vjson_raw)

    client = vjson["downloads"]["client"]
    print(f"client.jar {client['size']} bytes", flush=True)
    download(client["url"], os.path.join(vdir, f"{version}.jar"),
             mc.get("client_sha1") or client["sha1"])

    server = vjson["downloads"]["server"]
    print(f"server.jar {server['size']} bytes", flush=True)
    download(server["url"], os.path.join(out, "server.jar"),
             mc.get("server_sha1") or server["sha1"])

    libs = vjson.get("libraries", [])
    n = 0
    for lib in libs:
        art = lib.get("downloads", {}).get("artifact")
        if not art:
            continue
        download(art["url"], os.path.join(out, "libraries", art["path"]), art["sha1"])
        n += 1
    print(f"libraries: {n}", flush=True)

    ai = vjson["assetIndex"]
    download(ai["url"], os.path.join(out, "assets", "indexes", f"{ai['id']}.json"), ai["sha1"])
    print(f"asset index {ai['id']} (objects skipped)", flush=True)
    print("done", flush=True)


if __name__ == "__main__":
    main()
