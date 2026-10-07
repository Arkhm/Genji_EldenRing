# STATUS: Elden Ring still attacks on mouse clicks in Genji mode

Date: 2026-10-07
Step: 3 (Genji's controls), see `docs/DESIGN.md`

## Goal
During gameplay in Genji mode, left click, right click and mouse 5 must not make Elden Ring attack, guard or
use a skill, because they will become Shuriken, Fan of Blades and Melee. Menus must keep working.

## What happens instead
Clicks still attack with Genji mode on (third in-game test, 2026-10-07). The log shows capture is on at the
right times (`MODE Genji input CAPTURED (gameplay)`, released whenever the HUD hides), so the problem is only
the blocking of the mouse.

## What works (verified in game)
- Keyboard: E, Q and Left Shift are hidden from the game during gameplay; F arrives as E (interact); F9 toggles
  Genji mode; every Genji action is logged once per press.
- Menus: capture is released whenever the HUD (UI state `0x07`) is hidden. Q and Esc back out of the Esc menu,
  inventory and map in both modes.
- The game never sees Esc changed by us. Esc not closing NPC dialogue or the grace menu happens while capture is
  released (Q worked there), so it is very likely Elden Ring's own behaviour. Not yet checked with F9 off.

## Attempts
1. **DirectInput hook (`GetDeviceState`).** Expected: the game reads keyboard and mouse through it (the CS2 field
   note says so). Actual: keyboard yes (about 60 reads per second), mouse **0 reads per second** in every
   session. The game gets its mouse state some other way.
2. **Disable the actions in the gameplay input reader (`CSInGamePad.input_code_check`).** Expected: the game's
   check for Attack, Strong Attack, Guard and Skill would answer "not pressed" while the "disabled" flag is set.
   Actual: the game rewrites those flags every frame (about 240 times a second for 4 inputs). We set them again
   every frame, but the game evidently checks between its reset and our write. Clicks attack. (The second test
   report was ambiguous; it probably never worked.)

## What we know for sure
- Mouse buttons are readable: the game's own `MouseDevice.di_mouse_state.buttons` shows every click, and the
  log has all mouse actions.
- With capture off, the library's copy of the game's Attack check says `true` on click (`PAD left click while
  released: game's Attack check says Some(true)`).
- Attack, Strong Attack, Guard and Skill each map to exactly one input in the gameplay reader: 827, 831, 835, 839.

## What we don't know
- Where the game reads the mouse from (DirectInput `GetDeviceData`, Windows Raw Input, or a separate device
  object). One logging-only test would answer it.
- Which function of the gameplay input reader answers "is Attack pressed?". Ghidra (with ghidra-mcp) would
  answer it from `eldenring.exe` on disk; the library already gives the address of that reader's function table
  (`csin_game_pad_vmt`).

## Options from here

| | Option | What you do | Code | Reliability | Cost |
|---|---|---|---|---|---|
| **A (recommended now)** | **Unbind Elden Ring's mouse attacks in the Genji save.** In Elden Ring's keyboard settings, move Attack, Strong Attack, Guard and Skill off the mouse buttons to unused keys (for example numpad keys). Our DLL already reads the mouse for Genji. | Change 4 bindings once | Remove the flag code | Complete: the game has nothing to attack with | 5 minutes |
| B | **Make the game ask us.** Find the gameplay reader's "is this action pressed?" function (Ghidra on the exe on disk, no game running) and replace it through its function table, the same way as the keyboard hook. No timing race. | Nothing (maybe install Ghidra) | New hook | High once found | 1 to 3 sessions |
| C | **Find the mouse's real input path** (log `GetDeviceData` and Raw Input calls for one session), then filter clicks there like the keyboard. | One test session, then another | Diagnostic, then hook | High if the path is simple | 1 to 2 sessions |
| D | **Use the game's own "no weapons" rule** (like Roundtable Hold, where you can't attack) through a status effect while Genji mode is on. Needs research into which effect does it and what else it disables. | Nothing | Small, plus research | Unknown | Research first |

Notes on A: I believe Elden Ring keeps key bindings in the save file, as it did for the button-icon setting,
which would mean the change only affects `ER0000.genji.sl2`. **Not verified.** With A, F9 (plain Elden Ring)
in the Genji save would have no mouse attacks either. Option B can replace A later without you changing
anything back.

## State of the repo
All work is committed and pushed (last code commit `9fc2568`). The game launches and runs cleanly. The flag code
from attempt 2 is still in `genji/src/gamepad.rs`; it does no harm, and is removed or kept depending on the
option chosen.
