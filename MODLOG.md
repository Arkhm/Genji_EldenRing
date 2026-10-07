# MODLOG

Everything that changed and what was tested. Newest entry first. Template:
[MODLOG-template.md](MODLOG-template.md).

## 2026-10-07 Step 3, menu fix: capture only while the HUD is visible

**Changed:** `genji/src/controls.rs` (capture rule, diagnostic), `genji/src/gamepad.rs` (removed the pad-signal
code, added notes and an Attack check for the diagnostic).
**Why:** The second test showed the gameplay input reader stays enabled while menus are open, so Q was
captured in menus. In both test logs, UI state `0x07` behaves like the HUD: hidden whenever a real menu is
open (Esc menu `0x1d`/`0x1e`, map `0x3d`, grace or map `0x25`) and during loading; visible through pop-ups,
item pickups and prompts (`0x13`, `0x17`, `0x1c`, `0x2b`). New rule: capture = Genji mode on, player present,
HUD visible. Added a diagnostic: with capture off, a left click logs what the game's Attack check answers.
**Tested how:** Build OK, no warnings; unit test passes.
**Result:** Not tested in game.
**Still broken / not tested:** If Elden Ring's HUD option is "Auto" or "Off", this rule fails (no capture when
the HUD hides). Cutscenes and NPC dialogue not checked.
**Next:** Your test: Q backs out of menus; attack works with F9 off.

## 2026-10-07 Step 3, second try, tested in game: MOSTLY PASSED

**Changed:** Nothing. Test of commit `a1a9456`.
**Why:** Step 3 pass condition.
**Tested how:** You played through the test list; I read `logs/genji.log` (492 lines).
**Result:**
- Left and right click no longer attack. Shift, E and Q do nothing in the game. All Genji actions were logged
  once per press (`Shuriken`, `FanOfBlades`, `SwiftStrike`, `Deflect`, `Dragonblade`, `Melee` on mouse 5,
  `Jump`, `Interact`).
- F interacts. With F9 (Genji off), F jumps again, as in plain Elden Ring.
- Weapon actions: `4 mapped inputs (original flags 827:10 831:10 835:10 839:10)`. The game resets our
  "disabled" flag every frame (15,544 resets by 97 s), yet attacks were blocked: we set it again every frame
  before the game checks. Works, but depends on timing.
- **Failed:** Q did not back out of menus (Esc did). Cause: the "gameplay input reader enabled" signal stayed
  on in menus (`in-game pad found 1 polling 1 enabled 1` the whole time a menu was open), so Q stayed captured.
- Unclear from the report: whether left click attacked with Genji mode off. The new diagnostic answers it.
**Still broken / not tested:** Menu detection (fixed in the entry above, not tested).
**Next:** Menu fix.

## 2026-10-07 Step 3, second try: block Elden Ring's actions in its gameplay input reader

**Changed:** New `genji/src/gamepad.rs`; `genji/src/controls.rs` rewritten; mouse handling removed from
`genji/src/dinput.rs` (and from its unit test); `genji/src/lib.rs` registers the new module.
**Why:** The first try failed in game (entry below). Two fixes:
1. **Mouse:** Elden Ring has separate input readers for gameplay (`CSInGamePad`) and menus
   (`CSMenuViewerPad`). The gameplay one turns keys and buttons into actions like Attack, Guard, Event Action
   and Jump. While captured, we now switch off Attack, Strong Attack, Guard and Skill there. Each action has
   enable flags (`input_code_check`), and we set the "disabled" flag on them and restore the saved values on
   release. This covers the mouse buttons, and can't affect menus because only the gameplay reader is touched.
   Mouse buttons for Genji's actions are read from the game's own mouse state.
2. **Menus:** "In gameplay" now means the gameplay input reader is enabled and allowed to poll, instead of
   comparing UI-state numbers. The keyboard part (hide E, Q, Left Shift, F; F arrives as E) is unchanged and
   now follows this new signal.
New log lines: `PAD` (every change of the four reader signals), `PAD weapon actions blocked: N mapped inputs
(original flags ...)`, and every 10 s whether the game switched our flags back on by itself.
**Tested how:** Build OK, no warnings; unit test still passes (keyboard filter and hook path).
**Result:** Not tested in game.
**Still broken / not tested:** Both fixes are untested guesses based on the fromsoftware-rs descriptions of
these structures. In particular: whether the game's real input check uses those enable flags the way the
library describes, and whether the reader signals change when a menu opens. The log will show both.
**Next:** Your test.

## 2026-10-07 Step 3 tested in game: FAILED (nothing changed)

**Changed:** Nothing. Test of commit `7ccc067`.
**Why:** Step 3 pass condition.
**Tested how:** You played, pressed the Genji keys, F at an object, opened menus, toggled F9. I read
`logs/genji.log` (424 lines).
**Result:** You saw: clicks and keys still did Elden Ring things, F jumped, menus worked, F9 changed nothing
visible. The log shows why:
- The keyboard hook works: `keyboard 60-61` reads per second through it. **The mouse never goes through it:
  `mouse 0` the whole session.** So clicks always reached the game.
- The menu rule was wrong. UI state `0x13` turns on and off every few seconds during normal play, and
  `0x17`, `0x1c` and `0x2b` appear for prompts and pop-ups. Each one counted as "menu open" and gave the keys
  back. Keys were captured only for a few seconds at a time (for example 167.4 to 167.9 s, 170.2 to 180.7 s).
- While captured, it did work: `SwiftStrike`, `Deflect` and `Dragonblade` were logged at 171 to 173 s, and at
  221.6 s F was captured and reached the game as E (an interaction followed). F jumped whenever the keys were
  released.
- UI states seen (for later mapping): 0x05 always; 0x04 0x06 0x08 during loading; 0x07 in gameplay (hidden
  when 0x25 opened, probably the map or grace menu, before a fast travel); 0x13 flickering; 0x17, 0x1c, 0x2b
  short-lived pop-ups or prompts.
**Still broken / not tested:** Everything in step 3.
**Next:** Second try (entry above).

## 2026-10-07 Step 3: Genji's controls (keys captured from the game)

**Changed:** New `genji/src/dinput.rs` (DirectInput hook) and `genji/src/controls.rs` (actions, F9 toggle, menu
rule), wired in `genji/src/lib.rs`. `docs/DESIGN.md` key table updated. Removed the input diagnostic
(`genji/src/inputdiag.rs`) in a separate commit before this one.
**Why:** Step 3 of the plan. Layout chosen by you: Overwatch keys, melee on mouse button 5.
- Left click Shuriken, right click Fan of Blades, Left Shift Swift Strike, E Deflect, Q Dragonblade, mouse 5
  Melee, Space Jump (logged only; the game still gets Space until the movement step), F = Elden Ring's interact
  (the game sees E), F9 = Genji mode on/off.
- How: Elden Ring reads keyboard and mouse through DirectInput's `GetDeviceState`. We swap that entry in
  DirectInput's function table for ours. Ours calls the real one, records the raw keys, and, only while
  "captured", blanks Genji's keys and turns F into E. No game code is changed.
- Menus (provisional rule): 2 s after the player appears, the set of visible UI states is recorded as the
  gameplay baseline; any extra visible UI state = menu open = all keys go to the game. Every change is logged
  (`UI visible [...] (shown [...], hidden [...])`) so we can learn the numbers.
- If our per-frame code ever crashes, capture is switched off so your keys can't stay blocked.
**Tested how:** Offline unit test (`cargo test --release` in `genji/`): installs the real hook in a test
process, checks the function table points at our hook and that calls reach the real DirectInput (an
unacquired device answers `DIERR_NOTACQUIRED`), installing twice is harmless, and the filter: not captured =
nothing changes; captured = E, Q, Left Shift, F hidden, F arrives as E, Space untouched; left, right and mouse 5
hidden, middle and mouse 4 kept.
**Result:** Test passed (1 test, 0.04 s). Build OK, no warnings, `build/genji.dll` 291,840 bytes.
**Still broken / not tested:** **Not tested in game.** Unknown: whether Elden Ring uses the hooked interface
(the log's `DINPUT reads` line answers it), whether the menu rule is right (boss health bars, item pickup
pop-ups or messages may count as "menus" and briefly hand the keys back to the game), and whether F→E works
for all interactions. The keys are fixed in code for now; the settings file (`genji.toml`) comes in a later
step.
**Next:** Your in-game test, then step 4 (first-person camera).

## 2026-10-07 Step 2 (Overwatch assets): BLOCKED, waiting for a DataTool update

**Changed:** New `tools/get_datatool.ps1` (downloads the latest official DataTool release into
`third_party/datatool/`) and `tools/extract_genji.ps1` (runs DataTool read-only and offline, extracts Genji's
skin with animations, his abilities and voice lines into `owassets/raw/`, writes `logs/extract-summary.txt`).
Downloaded DataTool v2.24.1.0+1149 (`toolchain-release.zip`, 75,453,689 bytes, SHA-256 `64042bdd...840482c`,
from github.com/overtools/OWLib releases) into `third_party/datatool/` (gitignored). `docs/DESIGN.md` plan order
updated.
**Why:** Step 2 of the plan: get Genji's assets from your own Overwatch install.
**Tested how:** Ran `DataTool.exe <Overwatch> list-heroes --online=false`, then `tools/extract_genji.ps1`.
**Result:** **Blocked.** Your Steam Overwatch updated on 2026-10-06 to build `prometheus-2_25_0_0-154088`
(the new season). The newest DataTool supports 2.24.1.0.153480 and refuses newer builds:
`unable to parse TRG. invalid version 14, expected 5 ... 13`. Neither DataTool nor its file library TACTLib
has unreleased support yet (checked their GitHub: last DataTool update 2026-09-15, last TACTLib commit
2026-09-24). Patching it ourselves is not an option: each build also needs decryption keys that the DataTool
team gets out of the Overwatch game itself, which would mean working around Overwatch's protection (hard
rule 5). Good sign: DataTool did open the Steam install and read its build number, so the Steam layout itself
looks supported (not proven).
- Found while testing: `--online` is on by default in DataTool ("allow downloading of corrupted files" from
  Blizzard). Our script always passes `--online=false`.
- First script version crashed on Windows PowerShell 5.1 (a tool writing to its error stream counted as a
  script failure) before showing its own message. Fixed by running DataTool as a separate process; now it
  stops with "This DataTool doesn't support the installed Overwatch build yet".
**Still broken / not tested:** The extraction itself (`extract-unlocks`, `extract-abilities`,
`extract-hero-voice` and their exact search syntax) is **not tested**. Whether Genji's first-person model
comes out is unknown.
**Next:** Recent DataTool updates came 8 to 24 days after an Overwatch patch. When a new release is out, run
`tools\get_datatool.ps1` then `tools\extract_genji.ps1`. Meanwhile, continue with the mechanics (steps 3 to 9),
which don't need Overwatch assets.

## 2026-10-07 Ultrawide and input diagnostics tested in game: PASSED

**Changed:** Nothing. Test of commits `df286b8` (ultrawide) and `0da4be5` (input diagnostics).
**Why:** Check the ultrawide fix; find out why only controller prompts showed.
**Tested how:** You launched with `tools/play.ps1`, played, changed "Device for On-Screen Prompts" to Keyboard
and Mouse. I read `logs/genji.log` (151 lines).
**Result:**
- Ultrawide: `WIDE patched 0x7ff76a41da5e: 0x74 -> 0xeb (scan 1, 0.0s)` at 0.051 s. You confirmed no black bars.
- Prompts: you confirmed keyboard icons after changing the setting. Cause: the setting, copied from your old
  save. No controller of any kind (real, Steam Input or ViGEm virtual) appeared in the log the whole session;
  keyboard and mouse presses all arrived (`Esc`, `Q` x4, `W A S D`, `Space`, `F`, `LShift`, mouse clicks).
**Still broken / not tested:** The controller plug-in part of the test wasn't done, so connecting a pad is
not verified in the diagnostic (not needed now).
**Next:** Step 2.

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
