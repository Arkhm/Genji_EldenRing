# Genji in Elden Ring

Play Elden Ring offline as Genji from Overwatch, in first person: double jump, wall climb, shuriken, Swift
Strike, Deflect and Dragonblade.

It is a native Rust DLL loaded by [me3](https://github.com/garyttierney/me3). Elden Ring runs its world,
enemies and saves; the DLL replaces how you move, fight and see. Genji's model, animations and sounds are
converted at build time from **your own** Overwatch install and never leave your machine. This repository
contains no game files.

Fan project, single-player and offline only. Not affiliated with Blizzard, FromSoftware or Bandai Namco. You
need to own both games.

How it works and the build plan: [docs/DESIGN.md](docs/DESIGN.md). Change history: [MODLOG.md](MODLOG.md).

## What works

- `tools/play.ps1` backs up your saves, builds `genji.dll` and starts Elden Ring offline through me3 with its
  own save (`ER0000.genji.sl2`). The DLL loads, and logs the player's position, map block and HP to
  `logs/genji.log` (tested in game 2026-10-07).

## Not tested yet

- Ultrawide fix (no black bars) built into `genji.dll`.
- Input diagnostics in the log.

## What doesn't work yet

All of Genji: steps 2 to 13 of [docs/DESIGN.md](docs/DESIGN.md#5-the-plan-step-by-step).
