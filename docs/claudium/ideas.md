# Ideas worth building

Features that would make Claudium worth choosing over both vanilla and upstream
Horizon. Each one is picked because the engine *already has the hard part* —
these are not wishes, they are short paths from machinery that exists.

Ordered by (value to a player) ÷ (work).

## 1. Auto graphics tuning on first run

**The pitch.** The first time you play, Claudium spends ten seconds flying a
test scene, measures your machine, and picks graphics settings that hold your
monitor's refresh rate. No more guessing at render distance.

**Why it is cheap.** `--benchmark` already exists and already does the hard
part: it creates a world at a fixed seed, flies four scripted scenes, and
reports average FPS, 1% lows, frame-time percentiles, GPU and resident memory
(`docs/benchmark.md`, `docs/benchmark-report.md`). The missing piece is only a
policy that maps a report onto `horizon-graphics.txt` and `options.txt`, plus a
first-run prompt.

**Make it an option,** not a surprise: offer it on first launch and in Video
Settings as "Tune for this computer", and always show what it changed.

## 2. Undo for building

**The pitch.** Ctrl+Z takes back the block you just placed or broke. Hold it to
walk back a whole mistake. Creative only, by default.

**Why it is cheap.** Every edit already flows through one place and already
carries what undo needs: the engine records block changes to drive relighting
and remeshing, and `BlockSection::set` already reads the old state out of
storage before writing (that is the optimisation in
`docs/performance/paletted-container-access-results.md`). A bounded ring of
`(position, old state, new state)` is a few hundred bytes a player.

**Why it is a differentiator.** Neither vanilla nor any major client has this.
Anyone who has misclicked a WorldEdit-less build wants it.

## 3. See the seed before you commit to it

**The pitch.** On the Create World screen, a map of the terrain around spawn,
drawn before the world exists. Biomes, coastlines, where the mountains are.
Reroll until you like it.

**Why it is cheap.** The distant-terrain path already samples world generation
*without* loading chunks, across a radius of up to 1024 chunks, on background
threads (`docs/graphics.md`, "Distant terrain"). Rendering that sampling to a
2D map is a new view over data the engine already produces.

**Caveat worth knowing up front:** the docs say direct far sampling
approximates vertical noise, fluid boundaries and lighting, and does not
synthesize unexplored structures. So the map is honest about terrain and
biomes, and must not promise villages.

## 4. A real performance overlay

**The pitch.** F3 tells you the FPS. This tells you *why*: a frame-time graph
with the 1% low marked, and the chunk pipeline underneath — how many sections
are queued to generate, light, mesh and upload.

**Why it is cheap.** The benchmark already computes every one of those figures;
the chunk pipeline counters already exist because the benchmark driver waits on
them ("fewer than 100 section meshes a second", `docs/benchmark.md`). This is a
HUD over counters that are already live.

**Why it matters.** It turns "the game stutters sometimes" into a bug report
with a cause, which is worth a lot when the next thing on the list is fixing
frame pacing.

## 5. Recording and playing back a build

**The pitch.** Start recording, build, stop. Claudium writes a small file of
block changes with timestamps that plays back as a timelapse, in-world.

**Why it is cheap.** It is the same change log as undo (#2), written to disk
instead of a ring buffer, and replayed through the same bulk-edit path.

## Options, not features

Small switches worth having because the engine can already do them and someone
will want them off:

- **Quiet mode** — one switch for every network call the game can make, so a
  fully offline play session is one click, not five settings.
- **Keep the sky still** — the launcher already does this (Claudium draws a flat
  background); offer the same for in-game clouds and weather on weak machines.
- **Per-instance render distance and FOV**, remembered with the instance rather
  than globally.
- **Safe mode** — start an instance with every mod off, from the launcher's
  instance menu, for when a mod breaks a world you care about.

## Deliberately not on this list

- **A Java mod translator.** It exists twice already; see
  [roadmap.md](roadmap.md) for why the remaining part cannot be mechanised.
- **Beating upstream's frame times by micro-optimising.** The obvious levers are
  taken. The honest targets need profiling on a real machine; also in
  [roadmap.md](roadmap.md).
