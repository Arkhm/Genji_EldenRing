# MODLOG

Everything that changed and what was tested. Newest entry first. Template:
[MODLOG-template.md](MODLOG-template.md).

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
