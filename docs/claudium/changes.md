# Changes on top of Horizon

Everything in this repository that is not upstream Horizon, newest first. Keep
this current: it is what tells us, and anyone reading the project, which parts
are ours.

## Unreleased

- Copied Horizon `c995c9b` into this repository as the base, kept its MIT licence
  and recorded the derivation in `NOTICE.md`. Removed upstream workflows that
  depend on their runners and signing keys (see `upstream.md`).
- Removed the previous from-scratch Claudium client (~7.5k lines: wgpu renderer,
  world generation, Java Edition protocol client, egui launcher). It is still on
  the `main` branch if anything there is worth recovering.

## Planned

- Our own launcher, replacing upstream's `launcher/`.
