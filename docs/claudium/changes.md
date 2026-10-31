# Changes on top of Horizon

Everything in this repository that is not upstream Horizon, newest first. Keep
this current: it is what tells us, and anyone reading the project, which parts
are ours.

## Unreleased

### Launcher

- Rebuilt the Play row as the Minecraft launcher's play bar: a full-width band
  under the hero with its own fill and a rule along the top, the version picker
  on the left, the one big green PLAY in the middle of the window and the
  secondary actions on the right. PLAY is set in capitals and 320px wide.

- Moved the account to the top of the sidebar, where the Minecraft launcher has
  it: face, name and how you are signed in, opening the same account menu. The
  top bar now carries nothing but the page tabs, and Settings moved to the foot
  of the sidebar.
- Removed upstream's sponsor advertising: the sidebar card, the Servers page
  banner, the timer that counted how long it had been on screen, `partner.rs`,
  its three images and its strings in all seven languages. Horizon's own
  community links (Sponsor, Discord, Website) went with them.

- Repainted the launcher in the Minecraft launcher's colours: near-black sidebar,
  dark neutral page, Minecraft green for the one main action, white text at three
  weights (`horizon-theme`'s `night::VANILLA` and `VANILLA_LIGHT`). White on the
  green face is 4.6:1; `accent_hi` is the lighter green that stays readable as
  text on both the page and the sidebar.
- Replaced Horizon's painted night sky with a flat surface, as the real launcher
  has (`launcher/src/sky.rs`, `FLAT`). This skips the bands, stars, clouds, sun
  and skylines, their baked sprites and the window wake-ups that animated them,
  so the launcher now stands completely still at rest. Set `FLAT` to `false` to
  get the sky back; the module still implements it in full.

### Base

- Copied Horizon `c995c9b` into this repository as the base, kept its MIT licence
  and recorded the derivation in `NOTICE.md`. Removed upstream workflows that
  depend on their runners and signing keys (see `upstream.md`).
- Removed the previous from-scratch Claudium client (~7.5k lines: wgpu renderer,
  world generation, Java Edition protocol client, egui launcher). It is still on
  the `main` branch if anything there is worth recovering.

## Planned

- Our own launcher, replacing upstream's `launcher/`.
