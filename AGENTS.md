# AGENTS.md

## Project
Play Elden Ring offline as Genji from Overwatch, in first person, with his full kit (movement, shuriken,
Swift Strike, Deflect, Dragonblade), built as a native Rust DLL that me3 loads into Elden Ring.

## Hard rules, never break these

1. **Never write game assets, decompiled code, or extracted game files into this
   repository.** They stay on this machine, untracked. If you need to read game
   data, read it from the install path at runtime or extract it to a gitignored
   folder.
2. **`.gitignore` is a whitelist.** It ignores everything and includes only
   source files. Do not switch it to a normal ignore list.
3. **Do not run `git commit` unless I asked.** Stage nothing beyond what the
   current task requires.
4. **Do not touch anything outside this project folder** unless I explicitly name
   the path. This includes my game installs: read them, never write to them.
5. **Single-player and offline only.** If this touches online play, anti-cheat, or
   DRM, stop and tell me. Offline play with anti-cheat switched off through the
   game's own option is fine. Bypassing anti-cheat is never fine.
6. **Never put credentials in the repo or in any file you can read:** no API keys,
   no tokens, no passwords.

## How to work

- **Plan before code.** For anything more than a small fix, write the plan to
  `docs/DESIGN.md` first, then implement one step at a time.
- **One thing at a time.** Do not bundle unrelated changes. I want to be able to
  revert a single step.
- **Log, don't look.** You cannot see the game. Instrument instead: write
  positions, counts, timings, and state transitions to a log file so you can
  verify from the numbers. Do not try to visually inspect the game.
- **Tell me how to test it.** After every change, say the exact command to run and
  what I should see. "Done" without a test procedure is not done.
- **Ask before large refactors.** If you think the architecture is wrong, say so
  and explain, then wait for me.
- **Explain in plain language.** I am not the programmer here. If you use a term,
  explain it the first time.

## Honesty

- If something is not tested, write **"not tested"**. Never imply you verified
  something you didn't.
- If you are not sure, say you are not sure. A confident wrong answer costs me
  hours.
- If you hit something you cannot solve after two real attempts, stop and write
  up `STATUS.md` (see `STATUS-handoff.md`) instead of trying variations at random.
- Record failures alongside successes. A dead end I can see is worth more than a
  dead end I have to watch you repeat.

## Keep these files updated

- `MODLOG.md`: add an entry after every change. Template in
  `MODLOG-template.md`.
- `docs/DESIGN.md`: how the project works, in plain language. Update when the
  architecture changes, not on every commit.
- `README.md`: the "what works / what doesn't work" list. Test before you claim
  something works.

## Environment

- OS: Windows 11 Pro (10.0.26200)
- Game A: Elden Ring 1.17.1 (eldenring.exe 2.7.1.0, Steam build 25080141), installed at
  `D:\SteamLibrary\steamapps\common\ELDEN RING\Game`
- Game B: Overwatch (Steam app 2357570, Steam build 25517475), installed at
  `D:\SteamLibrary\steamapps\common\Overwatch`. Used only as an asset source: its files
  are read by DataTool, and Overwatch is never run alongside the mod.
- Loader: me3 (0.11.0 installed on this machine; 0.13.0 is the current release)
- Which game is authoritative for the player: A (Elden Ring)
- Language and version: Rust stable 1.99 with the MSVC target (`x86_64-pc-windows-msvc`);
  Python 3.13 for build-time tools
- Agent: Claude Code
