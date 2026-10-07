# MODLOG

Newest entry first. Template: [MODLOG-template.md](MODLOG-template.md).

## 2026-10-07: Step 0, research, plan and repository setup

**What changed:** Compared the two example approaches and checked the three tools. Chose the route: rewrite
Genji's kit as a Rust DLL inside Elden Ring, loaded by me3, with assets converted from the local Overwatch
install. Wrote the plan in `docs/DESIGN.md`. Set up the repository rules.

**Files:** `AGENTS.md`, `CLAUDE.md` (points Claude Code at `AGENTS.md`), `.gitignore` (whitelist),
`docs/DESIGN.md`, `README.md`, `MODLOG.md`, `MODLOG-template.md`, `STATUS-handoff.md`.

**How to test:** nothing to run. Read `docs/DESIGN.md`. Check that `git status` shows no game files.

**Result:** no game code yet, so there is nothing to test in game. Machine checks done (read-only):
- Elden Ring: `eldenring.exe` 2.7.1.0, Steam build 25080141, on `D:\SteamLibrary\...\ELDEN RING\Game`.
- Overwatch: Steam build 25517475, 77 GB, on `D:\SteamLibrary\steamapps\common\Overwatch`.
- me3 0.11.0 installed (0.13.0 is current). Rust 1.99 MSVC installed. Java 21 installed.
- **Failed:** a minimal Rust DLL build. There is no Microsoft C++ linker on this PC; Git's unrelated
  `link.exe` is found instead. Visual Studio Build Tools are needed.
- **Found:** `dinput8.dll` (Elden Mod Loader) in the game folder loads `ersc.dll` (Seamless Co-op, online),
  `UltrawideFix.dll` and `RemoveVignette.dll` on every game start. This has to be off for offline Genji
  sessions. Waiting for your decision; nothing in the game folder has been changed.

**Dead ends:** a passthrough (SkyCraft style) is impossible: Overwatch is online-only with anti-cheat. A full
engine rewrite (iw4L style) of Elden Ring is far too large. Details in `docs/DESIGN.md` section 1.

**Undo:** delete the files listed above.
