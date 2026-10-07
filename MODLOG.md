# MODLOG

Everything that changed and what was tested. Newest entry first. Template:
[MODLOG-template.md](MODLOG-template.md).

## 2026-10-07 Input diagnostics, and the "controller-only prompts" report

**Changed:** New `genji/src/inputdiag.rs`, wired into `genji/src/lib.rs` (commit `0da4be5`). Read-only: while the
game window is focused it logs each key and mouse button press (`INPUT key down Q`), polls the four XInput
controller slots 4 times per second, logs connects and disconnects, and writes a once-per-second activity
summary for connected pads.
**Why:** You reported that the game showed only controller buttons and the keyboard couldn't back out of the
options; you had to plug in a controller. Likely causes found:
1. **Back on keyboard is Q, not Esc.** Esc is hard-bound to opening and closing the main menu, so it can't back
   out of options screens. (Source: Steam community / PC controls guides.)
2. **Elden Ring has a setting for which button icons it shows:** System → Sound and Display → "Device for
   On-Screen Prompts" (Game Pad or Keyboard and Mouse). It is stored in the save, and `ER0000.genji.sl2` is a
   copy of your `ER0000.sl2`, so it carried that setting over.
3. Possible but unconfirmed: Steam Input is forced on for Elden Ring in your Steam settings
   (`UseSteamControllerConfig = 2` in your `localconfig.vdf`, read only), and the ViGEm virtual-gamepad driver
   is installed and running. Either could present a virtual controller. The diagnostic will show whether one
   exists.
**Tested how:** Built without warnings. Not run in game yet.
**Result:** Not tested.
**Still broken / not tested:** Everything in game. GetAsyncKeyState sees what Windows sees, not necessarily what
Elden Ring accepts; good enough to tell "the key never arrived" from "the game ignored it".
**Next:** You test (see the next entry's procedure, one launch covers both).

## 2026-10-07 Ultrawide fix inside genji.dll

**Changed:** New `genji/src/ultrawide.rs`, started from `genji/src/lib.rs`; `pelite` and `windows` added to
`genji/Cargo.toml` (commit `df286b8`).
**Why:** You asked for ultrawide support back (your main monitor is 3440×1440). Instead of loading the old
2022 `UltrawideFix.dll`, the same patch now lives in our DLL: techiew's UltrawideFix (MIT licence) finds one
byte pattern in the game's code and changes the jump 0x94 bytes after it from `74` to `EB`, which removes the
forced 16:9 picture. In memory only; no game file is changed. Retries for up to 60 s, checks the original byte
before writing, logs every outcome with the `WIDE` tag.
**Tested how:** Offline: searched `eldenring.exe` 2.7.1.0 on disk (read only) for the pattern.
**Result:** Pattern found exactly once (file offset `0x19ecfca`), byte at +0x94 is `74` (`test eax,eax; je`),
which is the expected original. Build OK, no warnings.
**Still broken / not tested:** Not tested in game. The HUD will probably stay at 16:9 in the middle of the
screen (the original fix behaved the same way).
**Next:** Your test run. Expect a log line like `WIDE   patched 0x7ff...: 0x74 -> 0xeb (scan 1, 0.0s)` and
no black bars.

## 2026-10-07 Step 1 tested in game: PASSED

**Changed:** Nothing. Test of commit `601a681`.
**Why:** Step 1 pass condition: the DLL loads through me3, boots, and logs the player's state.
**Tested how:** You launched with `tools/play.ps1`, loaded your character, walked, fast travelled once, quit to
the title screen. I read `logs/genji.log` (374 lines, 387 s).
**Result:**
- Loaded and booted: `CSTask ready after 0.0s` at 0.029 s. No `PANIC` or `ERROR` lines.
- `player appeared` at 61.5 s; `player gone` at 242.6 s and back at 244.8 s (the fast travel); gone again at
  271.4 s (quit to title).
- 200 position ticks, 195 in block `m60_42_36_00` (Limgrave). Five ticks during loading showed block
  `m255_255_255_255` with local position 0,0,0: the game hadn't placed the player in a block yet.
- Positions changed while walking (for example local `(-47.34, 92.38, 80.18)` to `(-46.22, 92.36, 78.78)` over
  three seconds).
- **Finding:** the engine's own physics position (`pos`) jumps between two frames of reference: `(-11.48, 90.59,
  -56.89)` before the fast travel and `(-2.76, 2.51, 1.80)` a few minutes later at almost the same block-local
  spot. The engine re-centres its physics world while you play. Block-local coordinates are the stable ones;
  the movement step must resync after these jumps (the CS2 field note warned about the same thing).
- HP read 760/800, then 760/760 most of the time; the maximum changes with what's equipped, so this looks right.
- Frame rate 20 to 60 fps, mostly 45 to 60.
**Still broken / not tested:** Controller-only prompts (entry above). Black bars on ultrawide (entry above).
**Next:** Step 2 (Overwatch asset recon), after the test of the two entries above.

## 2026-10-07 Step 1: hello DLL, me3 profile, save backup

**Changed:** New Rust crate `genji/` (`Cargo.toml`, `src/lib.rs`, `src/log.rs`), me3 profile
`profile/genji.me3`, scripts `tools/build.ps1`, `tools/backup_saves.ps1`, `tools/play.ps1`. `docs/DESIGN.md`
risk list updated.
**Why:** Prove the whole pipeline (build, load through me3, find the game's objects, log) before any Genji code.
The DLL changes nothing in the game. It waits for the game's task system, then once per second logs the
player's position, map block, HP and frame rate, and logs when the player appears or disappears (loading
screens). A crash inside our code is logged and switches the DLL off instead of crashing the game.
**Tested how:** Built with `tools/build.ps1`; ran `tools/backup_saves.ps1`; checked the profile with
`me3 profile show`.
**Result:** Build OK, no warnings, `build/genji.dll` 264,192 bytes. Backup OK: 6 files, 110.5 MB, in
`backups/saves/20261007-005004/` (`ER0000.sl2`, `ER0000.co2` and their `.bak`, `GraphicsConfig.xml`,
`steam_autocloud.vdf`). me3 reads the profile: savefile `ER0000.genji.sl2`, Start Online false, `genji.dll`
enabled. Found and fixed before building: the crates.io `eldenring` 0.14.0 only supports exe 2.6.2.0 and would
have crashed on load; switched to fromsoftware-rs GitHub main pinned at `59fbd3b` (supports 2.7.1.0).
**Still broken / not tested:** **Not tested in game.** Unknown until you run it: whether me3 loads the DLL,
whether the boot wait succeeds, whether the logged values make sense. Without UltrawideFix, an ultrawide
monitor will show black bars again.
**Next:** You run `tools/play.ps1`, load your character, walk, quit; I read `logs/genji.log`.

## 2026-10-07 Old mods moved out of the Elden Ring folder

**Changed:** Moved these out of `D:\SteamLibrary\steamapps\common\ELDEN RING\Game` (with your permission) into
`backups/game-folder-old-mods/` in this project (gitignored, not in git): `dinput8.dll` (Elden Mod Loader),
`mod_loader_config.ini`, `mod_loader_log.txt`, `mods/` (`ersc.dll`, `UltrawideFix.dll`, `RemoveVignette.dll`),
`SeamlessCoop/`, `ersc_launcher.exe`, `ModEngine2/`, `reshade-shaders/`, `ReShadePreset.ini`. About 165 MB.
Moved, not deleted, so any of it can be put back.
**Why:** `dinput8.dll` loaded Seamless Co-op (online) on every game start, including under me3. The other
loaders could also clash with our DLL's hooks.
**Tested how:** Counted files before and after each move; listed the game folder afterwards.
**Result:** mods 7/7 files, SeamlessCoop 57/57, ModEngine2 315/315, reshade-shaders 25/25 arrived. The game
folder now holds only Elden Ring's own files (`eldenring.exe`, `Data0-3`, `regulation.bin`, DLLs from the game,
`EasyAntiCheat/`, `movie/`, `sd/`, `crashdumps/`).
**Still broken / not tested:** The game hasn't been launched since. Your saves were not touched; the Seamless
Co-op save `ER0000.co2` is still in `%APPDATA%\EldenRing`. To get rid of the old mods for good, delete
`backups/game-folder-old-mods/`. To restore one, move it back into the game folder.
**Next:** Step 1, the hello DLL.

## 2026-10-07 Build tools and me3 checked

**Changed:** You installed Visual Studio 2022 Build Tools (C++ workload) and updated me3. Nothing in the repo.
**Why:** The step 0 check found no C++ linker, so Rust couldn't build a DLL; me3 was on 0.11.0.
**Tested how:** Rebuilt the same minimal Rust DLL that failed in step 0; ran `me3 --version` and `me3 info`.
**Result:** The DLL builds (`linktest.dll`, 9728 bytes). `me3 0.13.0`; me3 finds Steam and its profile folder.
**Still broken / not tested:** Nothing loaded into the game yet.
**Next:** Step 1.

## 2026-10-07 Step 0: research, plan and repository setup

**Changed:** `AGENTS.md`, `CLAUDE.md` (points Claude Code at `AGENTS.md`), `.gitignore` (whitelist),
`docs/DESIGN.md`, `README.md`, `MODLOG.md`, `MODLOG-template.md`, `STATUS-handoff.md`.
**Why:** Pick the route and write the plan before any code.
**Tested how:** Read-only checks of the machine. Tested the `.gitignore` whitelist with dummy files in
`owassets/` and `logs/`. Test-built a minimal Rust DLL.
**Result:** Route chosen: rewrite Genji's kit as a Rust DLL in Elden Ring, loaded by me3, with assets from your
Overwatch install. Elden Ring `eldenring.exe` 2.7.1.0, Steam build 25080141. Overwatch Steam build 25517475.
The whitelist ignored even `.md` files inside `owassets/`. The DLL build **failed**: no Microsoft C++ linker.
Found `dinput8.dll` in the game folder loading Seamless Co-op.
**Still broken / not tested:** No game code yet.
**Next:** Install build tools; decide about the old mod loader.
