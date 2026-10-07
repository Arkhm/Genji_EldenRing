# MODLOG

Everything that changed and what was tested. Newest entry first. Template:
[MODLOG-template.md](MODLOG-template.md).

## 2026-10-07 Step 5a: choose the ray filter from 32 candidates at takeover

**Changed:** `genji/src/movement.rs`: at takeover, a 3 m ray down from 1 m above the feet is cast with each of
the 32 single filter bits and the CS2 filter `0x2000058`; the one whose surface (plus the 8 cm gap) is closest
to where the game holds the feet, within 0.3 m, becomes the filter for all rays. If none is close enough, the
controller keeps waiting and the game keeps control (logged every 2 s). The whole table, plus the game's own
"on solid ground", "touching ground" and "adjust to hi collision" flags, is logged (`FILTER` lines) at takeover
and every time the controller thinks you walked off an edge. The old probe is removed.
**Why:** Third 5a test (entry below): the CS2 filter's surface was 0.9 m below the feet, so the controller
thought you were in mid-air and gave up.
**Tested how:** Build OK, no warnings; unit test passes.
**Result:** Not tested in game.
**Still broken / not tested:** Whether any single bit gives the surface the game stands on. If the chosen bit
only covers some surfaces (for example ground but not objects), walking onto the others will look like walking
off an edge; the `FILTER` table at that moment will show it.
**Next:** Your test.

## 2026-10-07 Step 5a third test: FAILED (ray surface 0.9 m below the feet)

**Changed:** Nothing. Test of commit `281354f`.
**Why:** Step 5a pass condition.
**Tested how:** You played; I read `logs/genji.log` (135 lines).
**Result:** You saw third person and Space jumping (plain Elden Ring). Log: collision ready after 6 frames;
controller on at 22.112 s with feet at y 78.77, but every filter in the probe hit at 77.86, 0.91 m below the
feet. The controller's ground ray (from 0.5 m above the feet, 1.1 m long) couldn't reach it, so it decided
you were airborne, tried to drop you, the game held you up (outside push 2.0 m in 0.5 s), and after 30 frames
the safety net gave up. My explanation (not confirmed): Elden Ring has detailed collision near the player and
rougher low-detail collision; filter `0x2000058` seems to hit the rough one, which was 8 cm under the feet in one
place and 0.9 m in another. It would also explain seeing the inside of the hill in the previous test.
**Still broken / not tested:** Ground detection.
**Next:** Choose the filter by experiment (entry above).

## 2026-10-07 Step 5a: wait for collision, keep the 8 cm ground gap, death black box

**Changed:** `genji/src/movement.rs`: (1) the controller only takes over once a ray straight down hits the
ground; until then the game keeps control (`waiting for the map's collision to load`, `collision ready after N
frames`); the probe now runs once per takeover. (2) Feet are placed 8 cm above the ray hit (`GROUND_GAP`), the
gap the game keeps. (3) Black box: the last 60 frames (position, outside push, our vertical speed and ground
state, the game's falling / on-ground flags and fall timer, HP) are written to the log (`BLACKBOX` lines)
whenever HP drops. `genji/src/camera.rs`: HP is watched before anything else each frame (so the black box is
written even on the frame of death); Elden Ring's own movement is only switched off once our controller is
actually driving (before, waiting would have frozen you).
**Why:** The probe test (entry below).
**Tested how:** Build OK, no warnings; unit test passes.
**Result:** Not tested in game.
**Still broken / not tested:** Cause of the instant death (black box will show it). Whether 8 cm is right
everywhere (outside push while standing should now be about 0).
**Next:** Your test.

## 2026-10-07 Step 5a probe tested in game: rays work; two new problems found

**Changed:** Nothing. Test of commit `22613bc`.
**Why:** Find out why every ray missed.
**Tested how:** You stood still, walked, died, respawned and walked again. I read `logs/genji.log` (302 lines).
**Result:**
- **Rays:** on the first frame after loading, every combination missed. 20 frames later filter `0x2000058` hit
  the ground at the right height (y 81.14 under feet at 81.18), as did `0x58`, `0x2000000`, `0x0` and `0x40`;
  `0xFFFFFFFF`, `0x1`, `0x10` missed. The 4th component of the position makes no difference. Cause of the 5a
  failure: **collision isn't loaded right after a loading screen**, and the 30-frame safety net fired before it
  loaded. The filter was right.
- **Fight with the game:** standing still, we wrote feet at y 1.13 (the ray hit), the game showed 1.21 every
  second: it keeps the feet 8 cm above that surface. Pushing 8 cm into the ground every frame gave 2.4 m of
  outside push per 0.5 s while standing still. You saw: bugged lighting and the inside of a small hill.
- **Instant death** at 71.7 s while walking downhill: HP 760 at 71.52 s, 0 at 72.52 s, no damage in between.
  Cause unknown.
- **After respawning it worked:** collision ready on frame 1; mouse look; walking at 5.50 m/s (checked from
  real positions: z 1.98 to 4.72 in 0.5 s); walked off a ledge, fell 0.98 s (about 8 m), landed at 16.7 m/s and
  survived. Engine re-centring was handled (`position jumped 104.6 m`). Outside push while walking still 3 to 4.5
  m per 0.5 s (the same 8 cm fight, now also happening sideways on slopes).
**Still broken / not tested:** See entry above.
**Next:** Fixes above.

## 2026-10-07 Step 5a: ray probe, starting direction fix, sensitivity 6.90

**Changed:** `genji/src/movement.rs`: a one-off probe on the 1st and 20th frame of each takeover casts a 3 m ray
straight down from 1 m above the feet with 8 collision filters x 3 values of the position's 4th component, and
logs which hit (`PROBE` lines). `genji/src/look.rs`: starts from the character's facing when the game's camera
is still at (0, 0, 0) after loading; sensitivity set to your Overwatch value 6.90. `genji/src/camera.rs`: the
`VIEW` line now says when the controller has given up (the 5a log claimed "first person + Genji movement"
after the give-up, which was wrong).
**Why:** The 5a test failed (entry below): every ray missed.
**Tested how:** Build OK, no warnings. Offline: the library's address for the ray function (`0xc71e00`) starts
with a normal function opening in `eldenring.exe` (`40 55 56 57 41 54 41 55 41 56 41 57 ...`), so it looks
right; the bytes just before it don't look like the usual padding, so not certain.
**Result:** Not tested in game.
**Still broken / not tested:** Movement (blocked on the rays).
**Next:** One test session to read the `PROBE` lines.

## 2026-10-07 Step 5a tested in game: FAILED (collision rays never hit; safety net handed back)

**Changed:** Nothing. Test of commit `30c0419`.
**Why:** Step 5a pass condition.
**Tested how:** You played; I read `logs/genji.log` (753 lines).
**Result:** You saw third person, Space jumping, walls doing nothing special, fall damage, no clipping. That was
plain Elden Ring: the controller took over at 27.316 s (`LOOK ... right match 1.000, up match 1.000, forward
match 1.000`, `controller ON`, both blockers on: 4 weapon and 7 movement inputs), but the first ground ray
missed (`left the ground` on frame 1) and no ray hit anything, so after 30 frames the safety net fired:
`GAVE UP: no ray hit anything in 30 frames` at 28.084 s and gave everything back to the game for the session.
While "falling", `outside push 2.21 m` in 0.5 s: the game's physics pushed the character back up out of the
ground our position writes were pushing it into, so the ground was there but our rays didn't see it.
The look started at exactly yaw 0, pitch 0: the game's camera wasn't set up yet on that frame.
**Still broken / not tested:** Ray casts; everything after them.
**Next:** Probe.

## 2026-10-07 Step 5a: own mouse look and walking (movement controller, first slice)

**Changed:** New `genji/src/look.rs` (mouse to yaw/pitch, Overwatch formula: degrees = counts x 0.0066 x
sensitivity, placeholder sensitivity 6) and `genji/src/movement.rs` (walking at 5.5 m/s relative to where you
look; wall rays at 0.65, 1.1, 1.6 m with sliding along walls; ground ray with 0.5 m step-up and 0.6 m snap-down;
our own gravity, 17 m/s^2 placeholder; game gravity off while driving and restored after). `genji/src/camera.rs`
now coordinates look, movement and camera each frame, and hands everything back to the game for menus, F9,
Torrent, ladders and death. `genji/src/gamepad.rs` can now also switch off Elden Ring's own movement actions
(move forward/back/left/right, dodge, backstep tap, jump) while our controller drives. `genji/src/dinput.rs`:
WASD codes. `glam` added. Plan written first in `docs/DESIGN.md` section 5b (commit `e45446d`).
**Why:** Step 5a of the plan.
**Tested how:** Build OK, no warnings; unit test passes; checked the built DLL contains the new code.
**Result:** Not tested in game.
**Still broken / not tested:** Everything in game. Unknowns the log will answer: axis conventions (`LOOK start
... right match / up match / forward match` should all be close to 1.000); whether the collision filter
`0x2000058` hits this game's ground (if not, the controller gives up after 30 frames: `GAVE UP: no ray hit`);
whether Elden Ring's own movement is really switched off (`outside push` should stay near 0 while walking);
real speed (`speed 5.50 m/s`). Safety nets: no ray hits in 30 frames, or falling for 5 s, hands control back to
the game for the session. No jump in 5a (Space does nothing in Genji mode now); falls do no damage yet.
**Next:** Your test.

## 2026-10-07 Step 4 tested in game: PASSED (step 4 done); 0.1 s delay before capture

**Changed:** `genji/src/controls.rs`: capture (and first person) switches on only after gameplay has held for
0.1 s; release stays instant. `README.md`, `docs/DESIGN.md` updated. Test of commit `35cc87a`.
**Why:** Step 4 pass condition. The log showed a one-frame flicker: going back from a tab to the Esc menu shows
the HUD (`0x07`) one frame before the Esc bar (`0x1c`), so capture and first person turned on for about 20 ms
(31.733 to 31.754 s, 53.339 to 53.359 s).
**Tested how:** You set Camera Auto Rotate and Camera Auto Wall Recovery Off, strafed, opened and closed the
Esc menu. I read `logs/genji.log` (170 lines).
**Result:** You saw: strafing no longer turns the view; the Esc menu is third person. Log: every Esc press
(`0x1c` shown at 30.5, 39.9, 52.1 s) released capture and first person. The delay fix builds without
warnings; **not tested in game** (it only matters for that one frame).
**Still broken / not tested:** Cutscenes, NPC dialogue camera, death camera, Torrent and ladders in first person.
**Next:** Step 5, movement.

## 2026-10-07 Step 4: Esc menu counts as a menu; camera settings for the Genji save

**Changed:** `genji/src/controls.rs`: gameplay now means UI state `0x07` (HUD) visible **and** `0x1c` (Esc menu
tab bar) not visible. `docs/DESIGN.md` and `README.md` list two Elden Ring camera settings for the Genji save.
**Why:** From the first camera test: (1) in the Esc menu the view stayed first person (and Genji's keys stayed
captured), and switched to third person only after choosing a tab. The log shows the Esc menu keeps the HUD
visible and adds `0x1c` (with `0x17`); a tab replaces `0x1c` with its own state (`0x1d`, `0x25`, `0x34`) and hides
the HUD; going back shows `0x1c` again (for example 184.158 to 184.822 s). (2) Strafing (A, D, W+A, W+D) turned
the view by itself: that is Elden Ring's Camera Auto Rotate (an option since patch 1.04). Our first-person view
uses the game's camera direction for now, so: Camera Auto Rotate Off and Camera Auto Wall Recovery Off.
**Tested how:** Build OK, no warnings.
**Result:** Not tested in game.
**Still broken / not tested:** If `0x1c` turns out to appear outside the Esc menu, capture would drop there.
Whether the two settings fully stop the strafing drift.
**Next:** Your test.

## 2026-10-07 Step 4 first version tested in game: MOSTLY PASSED

**Changed:** Nothing. Test of commit `0b7bc83`.
**Why:** Step 4 pass condition: first-person view at head height following the mouse, body hidden.
**Tested how:** You walked, jumped, looked around fast and up/down, opened menus, toggled F9. I read
`logs/genji.log` (699 lines).
**Result:**
- You saw: overall good; walking, jumping, looking fast, up and down all normal; looking straight ahead does
  **not** tilt down. Problems: strafing turns the view by itself; the Esc menu stays first person.
- Log: `pers_cam_1` is the camera the game draws with (your first-person view came from writing it); `pers_cam_2`
  holds the game's own follow camera and is copied into slot 1 each frame (`mask 0x00`). No feedback: the next
  frame's game camera never followed our eye position (for example game cam `(2.98, 3.88, -2.36)` vs eye
  `(-0.65, 4.03, -1.92)`). Field of view 0.838 rad (about 48 degrees vertical), near plane 0.05 m. The camera
  task ran 79 to 120 frames per 2 s report (the frame rate).
**Still broken / not tested:** Strafing drift, Esc menu (entry above).
**Next:** Fixes above.

## 2026-10-07 Step 4, first version: first-person camera

**Changed:** New `genji/src/camera.rs`; `genji/src/lib.rs` registers a third per-frame task (stage
`DrawParamUpdate`); `genji/src/controls.rs` turns first person on and off together with Genji's input capture.
**Why:** Step 4 of the plan. First version, kept simple: Elden Ring keeps doing the mouse look (its sensitivity,
up/down limits and smoothing stay as they are). Every frame, right after the game computes its third-person
camera (`CameraStep`), we move that camera to the player's eyes (feet + 1.6 m, 0.15 m forward) and keep the
direction the game chose. The player's model is hidden (`chr_flags1c5.enable_render = false`) and shown again
when first person ends. First person is on exactly when Genji's controls are captured: Genji mode on, player
present, HUD visible. Menus, graces, dialogue and F9 give the normal camera back.
**Tested how:** Build OK, no warnings.
**Result:** Not tested in game.
**Still broken / not tested:** Everything in game. Unknown: whether `pers_cam_1` is the camera the game draws
with (the log reports all four slots); whether the game feeds our position back into its next camera (the log's
`game cam was` would then follow our eye position); possible one-frame lag or jitter; whether hiding the model
also hides things we want (shadow, effects). No Genji arms yet (step 10). The view still tilts down a little
when looking straight ahead, because the game's camera aims at the character from above.
**Next:** Your test.

## 2026-10-07 Step 3 tested in game: PASSED (step 3 done)

**Changed:** Removed `STATUS.md` (resolved; it stays in git history at commit `c4d13ab`). `README.md` and
`docs/DESIGN.md` updated. No code change. Test of commit `c4d13ab`.
**Why:** Step 3 pass condition: Genji's keys and clicks reach our DLL, not Elden Ring, during gameplay only.
**Tested how:** You clicked in Genji mode, toggled F9 and clicked, backed out of menus. I read `logs/genji.log`
(410 lines).
**Result:**
- You saw: clicks don't attack in Genji mode; with F9 off they attack again; everything else working.
- Log: `weapon actions blocked in GameFlowStep: 4 mapped inputs` every time capture started, `weapon actions
  released` every time it ended (menu at 47.0 s, F9 at 53.6 s). Game's Attack check on left click:
  `Some(false)` 22 of 22 times while captured, `Some(true)` 13 of 13 times while released.
- The keybinding test without the mod was skipped as no longer needed. Whether Elden Ring only accepts mouse
  buttons for Attack stays unconfirmed.
**Still broken / not tested:** Controller play (not tried). NPC dialogue and grace menus: Esc doesn't close them,
Q does; capture is released there, so very likely Elden Ring's own behaviour (not checked with F9 off).
**Next:** Step 4, first-person camera.

## 2026-10-07 Step 3, third try: weapon blocking moved to the right stage of the frame; no-mod launcher

**Changed:** `genji/src/gamepad.rs` (blocker restored, now runs in its own task in the `GameFlowStep` stage),
`genji/src/controls.rs` (turns weapon blocking on and off with keyboard capture), `genji/src/lib.rs` (second
per-frame task). New `profile/genji-nomod.me3` and `tools/play_nomod.ps1` (the Genji save without our DLL).
`README.md`, `docs/DESIGN.md` and `STATUS.md` updated.
**Why:** Option A failed: you couldn't bind Attack to any keyboard key, only mouse buttons. The log shows capture
was released the whole time you were in the settings (from 31.4 s), and in that state the hook changes nothing,
so it's very likely Elden Ring's own rule. You then reset the bindings to defaults. Meanwhile the game's stage
order (fromsoftware-rs `CSTaskGroupIndex`) explained attempt 2: `PadStep` (input read, flags reset) runs early,
the player's actions are decided in the middle of the frame, and our write at `ChrIns_PostPhysics` came after
both. Now the flags are set in `GameFlowStep`, the stage right after `PadStep`.
**Tested how:** Build OK, no warnings; unit test passes; `me3 profile show` reads the no-mod profile (Genji
save, no natives, offline).
**Result:** Not tested in game.
**Still broken / not tested:** Whether the game decides actions after `GameFlowStep` (the stage order says so;
not proven). Whether Elden Ring itself refuses keyboard keys for Attack (no-mod launch can confirm).
**Next:** Your test with default bindings.

## 2026-10-07 Step 3: option A, Elden Ring's weapon actions unbound from the mouse in the Genji save

**Changed:** Removed the flag-blocking code (`ActionBlocker`) from `genji/src/gamepad.rs` and
`genji/src/controls.rs`. The left-click diagnostic now logs on every click, in both modes. `STATUS.md` marked
resolved; `docs/DESIGN.md` and `README.md` describe the one-time key setup.
**Why:** You chose option A from `STATUS.md`. Recommended bindings: Attack (RH) Numpad 1, Strong Attack (RH)
Numpad 2, Guard (LH) Numpad 3, Skill Numpad 4; F5 to F8 if there is no numpad.
**Tested how:** Build OK, no warnings; unit test passes.
**Result:** Not tested in game. Expected after you rebind: clicks do nothing in Elden Ring, in both modes, and the
log says `PAD left click: game's Attack check says Some(false)`.
**Still broken / not tested:** Whether Elden Ring stores key bindings per save (I believe so, not verified). If
it doesn't, your normal saves would get the numpad bindings too; you'd see it next time you play normally.
**Next:** Your rebind and a short check, then step 4 (first-person camera).

## 2026-10-07 Step 3, HUD rule tested in game: menus PASSED, mouse blocking FAILED, stopped (STATUS.md)

**Changed:** New `STATUS.md`. No code change. Test of commit `9fc2568`.
**Why:** Third in-game test of step 3. Two real attempts at blocking the mouse have now failed, so per
`AGENTS.md` the work stops and `STATUS.md` lays out the options.
**Tested how:** You backed out of menus with Q and Esc in both modes, clicked with Genji mode on and off, and
talked to an NPC and rested at a grace in Genji mode. I read `logs/genji.log` (1,022 lines, 105 Genji actions).
**Result:**
- Menus PASSED: capture released every time the HUD hid (`released to the game (HUD hidden: menu or loading)`)
  and came back after (for example released 35.1 s, captured 38.8 s). Q and Esc back out everywhere.
- Mouse FAILED: clicks attack with Genji mode on. Blocking flags re-set every frame (23,356 game resets by
  157 s), but the game still sees Attack. With capture off the game's Attack check says `Some(true)` on click.
- NPC dialogue and grace menu: Q backs out, Esc doesn't. Capture was released there, and the mod never changes
  Esc, so this is very likely Elden Ring's own behaviour (not yet checked with F9 off).
**Still broken / not tested:** Blocking Elden Ring's mouse attacks.
**Next:** Your choice of option in `STATUS.md`.

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
