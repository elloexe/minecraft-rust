# Notice

Claudium is built on [Horizon](https://github.com/horizon-rust/horizon), an
independent reimplementation of Minecraft: Java Edition in Rust by Ludovic Marie
and the Horizon contributors. Horizon is MIT licensed, and this project is a
derivative of it: the great majority of the code in this repository was written
by the Horizon contributors, not by us.

Horizon's licence and copyright notice are kept verbatim in [LICENSE](LICENSE),
as the MIT licence requires of any copy or derivative. Please do not read this
project's history as a from-scratch implementation — the engine, the world
generation, the renderer, the protocol and the parity work are Horizon's.

## What is ours

Changes made in this repository on top of upstream Horizon are listed in
[docs/claudium/changes.md](docs/claudium/changes.md). They are also MIT licensed,
under the same terms.

## Relationship to upstream

This is a copy, not a GitHub fork, so it carries no automatic link to upstream
and receives no updates from it. Pulling in Horizon's later work is a manual
merge; [docs/claudium/upstream.md](docs/claudium/upstream.md) describes how.

Bugs in code inherited from Horizon should not be reported to the Horizon
project unless they have been reproduced against upstream Horizon itself.

## Minecraft

Not an official Minecraft product. Not approved by or associated with Mojang or
Microsoft. This project contains no Minecraft code, assets or data; running it
requires your own legally obtained copy of Minecraft: Java Edition.
