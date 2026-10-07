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
  logged as Genji actions; F does Elden Ring's interact; the game starts in plain Elden Ring and F9 switches Genji mode
  on and off (off = attacks work again); menus are left alone (Q and Esc back out). Keep Elden Ring's default key bindings.
- First-person camera (step 4, tested 2026-10-07): while Genji mode is active, the view is at eye height and
  your character's body is hidden; menus, the Esc menu, graces, dialogue and F9 give the normal camera back.
- Genji's movement, first part (step 5a, tested 2026-10-07): your own mouse look with your Overwatch
  sensitivity (6.90), walking at Overwatch speed (5.5 m/s) relative to where you look, walls stop you, steps and
  slopes work; you can't walk through other characters. Jumping with Space (step 5b, tested 2026-10-07), with
  Overwatch-like height and air control, and no fall damage in Genji mode. Double jump and wall climb (hold Space
  facing a wall) (steps 5c and 5d, tested 2026-10-07). Doors, chests, item pickups, flask and weapon swaps work
  in Genji mode: the game moves the character for its own animations and you keep the first-person view
  (step 5e, tested 2026-10-07; being grabbed not tested).
- Shuriken, first part (step 6a, tested 2026-10-07): left click throws one (an Elden Ring Throwing Dagger with
  Overwatch values: 60 m/s, no drop, 27 damage, 54 on headshots) from just right of and below the centre, toward
  what's under the crosshair; kills give runes. No throw limit (510 throws in a row, tested 2026-10-07).
  Bursts of 3, Fan of Blades on right click, 30 shuriken and the automatic reload (step 6b) are built, **not
  tested** yet.
- `tools/play_nomod.ps1` starts the Genji save without our DLL, to tell mod problems from game behaviour.

## Settings the Genji save needs once

In Elden Ring's System menu, Camera options: **Camera Auto Rotate Off** and **Camera Auto Wall Recovery Off**.
Keep the default key bindings and the HUD on.

## What doesn't work yet

Genji's abilities and looks: steps 2 and 6 to 13 of [docs/DESIGN.md](docs/DESIGN.md#5-the-plan-step-by-step).
