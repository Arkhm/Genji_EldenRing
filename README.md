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
- Ultrawide: no black bars (tested 2026-10-07).
- Genji's controls (step 3, tested 2026-10-07): during gameplay, Elden Ring's weapon actions are off (clicks
  don't attack, guard or use skills) and E, Q and Left Shift are hidden from it; all Genji keys and clicks are
  logged as Genji actions; F does Elden Ring's interact; F9 switches Genji mode off and on (plain Elden Ring,
  attacks work again); menus are left alone (Q and Esc back out). Keep Elden Ring's default key bindings.
- `tools/play_nomod.ps1` starts the Genji save without our DLL, to tell mod problems from game behaviour.

## Settings the Genji save needs once

In Elden Ring's System menu, Camera options: **Camera Auto Rotate Off** and **Camera Auto Wall Recovery Off**.
Keep the default key bindings and the HUD on.

## What doesn't work yet

Genji's abilities, movement, camera and looks: steps 2 and 4 to 13 of [docs/DESIGN.md](docs/DESIGN.md#5-the-plan-step-by-step).
