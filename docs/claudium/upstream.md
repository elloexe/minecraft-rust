# Keeping up with Horizon

This repository is a **copy** of [horizon-rust/horizon](https://github.com/horizon-rust/horizon),
not a GitHub fork, so nothing arrives from upstream automatically. Bringing their
work in is a deliberate merge.

## One-time setup

```sh
git remote add upstream https://github.com/horizon-rust/horizon
git fetch upstream main
```

Because the copy was made without upstream's history, the two trees share no
commits and git will not merge them by default. The first merge needs:

```sh
git merge upstream/main --allow-unrelated-histories
```

After that first merge the histories are joined and later merges are ordinary:

```sh
git fetch upstream main && git merge upstream/main
```

## The base we copied from

| | |
|---|---|
| Upstream | `horizon-rust/horizon` |
| Commit | `c995c9b76ebd1523dde0b06dee7351a0758b5286` |
| Copied | 2026-10-30 |

Record each later merge here, so the next one knows where it starts from.

## What we removed from the copy

These were upstream infrastructure that cannot work outside their repository.
If a merge reintroduces them, drop them again.

- `.github/workflows/`: everything except `ci.yml`. The release, nightly and
  parity workflows need their self-hosted macOS runner, their release signing
  keys and the maintainer's own Minecraft install.
- `ci.yml`'s `parity-scope`, `parity`, `vanilla-parity` and `jfr-analysis` jobs,
  for the same reason. The `verify` and `launcher` jobs are kept as they were.
- `.github/CODEOWNERS`, `trusted-reviewers.txt`, `release-line.txt`.
- Upstream's agent and contributor docs, which describe their issue tracker and
  review process: moved to `docs/upstream/` for reference.

The ~290 differential parity harnesses in `tools/` were **kept**. They are the
most valuable thing in the repository and the reason upstream behaves like
vanilla; wiring them up needs a machine with a real Minecraft 26.3 install.
