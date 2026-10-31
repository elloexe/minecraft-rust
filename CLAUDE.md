# Claudium: a map for agents

Minecraft: Java Edition 26.3 in Rust (wgpu). This repository is a **copy of
[Claudium](https://github.com/elloexe/minecraft-rust)** that we modify. Almost all
the code here is theirs; read [NOTICE.md](NOTICE.md) once.

Upstream's own agent map is kept at
[docs/upstream/claudium-CLAUDE.md](docs/upstream/claudium-CLAUDE.md) and is the
best guide to the codebase itself — crate layout, where features live, how the
parity harnesses work. Read it for orientation, but note what does **not** apply
here, below.

## What does not carry over from upstream's docs

Upstream's instructions were written for *their* repository. In this one:

- **Do not** use their `/claim`, `/unclaim` or `/triage` issue commands, their
  PR labels (`roadmap:<id>`, `release-now`, `skip-release`), or their release
  batching. Those act on `elloexe/minecraft-rust`'s tracker, not ours.
- **Do not** adopt their attribution lines. Several name Ludovic Marie and his
  GitHub account specifically; they are for his agents, not for this repository.
  Use this repository's own commit and PR attribution.
- Their CI references a self-hosted macOS runner, release signing keys and the
  maintainer's Minecraft install. Those workflows are removed here; only
  `.github/workflows/ci.yml` remains.

## Rules that bite

- **Never commit Mojang's files**: game jars, assets, decompiled source, prepared
  resources (`.local/`). `tools/check_public_tree.py` rejects them, and
  `git config core.hooksPath .githooks` runs it on commit. Do this once after
  cloning.
- **Keep the licence intact.** `LICENSE` holds Claudium's copyright notice and the
  MIT licence requires it to survive in this copy. Do not remove it, do not
  reword it, and do not describe this project as written from scratch.
- **Vanilla is the specification** wherever it has the feature, down to the order
  of random calls. That is why upstream behaves like the real game, and it is
  easy to break by accident. Before changing gameplay, check whether a harness in
  `tools/test_*_parity.py` covers it.
- **Record what we change.** Every change we make on top of upstream goes in
  [docs/claudium/changes.md](docs/claudium/changes.md). Without it we lose track
  of which parts are ours, and the next upstream merge becomes guesswork.
- **Keep your runs away from real player data.** Point `CLAUDIUM_HOME`,
  `CLAUDIUM_LAUNCHER_HOME`, `XDG_DATA_HOME` and `XDG_CONFIG_HOME` at fresh
  `mktemp -d` folders, with `CLAUDIUM_NO_AUDIO=1`. Never write to
  `~/.local/share/claudium*`.

## Merging upstream

[docs/claudium/upstream.md](docs/claudium/upstream.md) has the procedure and
records the commit we copied from. The first merge needs
`--allow-unrelated-histories`; later ones do not.

## Checks

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --lib --bins --tests
cd launcher && cargo clippy --all-targets -- -D warnings && cargo test --lib
```

The launcher is its own Cargo workspace, so a workspace-wide `cargo` run in the
root does not build it. Check it separately.
