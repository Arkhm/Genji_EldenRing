# MODLOG

Everything that changed and what was tested. Newest entry first. Template:
[MODLOG-template.md](MODLOG-template.md).

## 2026-10-08 Game's follow camera kept in line with our view (health bars, enemy sounds)

**Changed:** `genji/src/camera.rs`: every frame in first person, our view is also written into the game's follow
camera (`CSCamera.pers_cam_2` and `WorldChrMan.chr_cam`), not only the drawn camera. Every 2 s a `CAMERA` line logs
how far the game's follow camera had turned away from our view before the overwrite.
**Why:** Your test: often after a hit, the enemy's health bar sat at the side of the screen and its sound came from
there too, until F9 twice. Likely cause (not yet proven): the game places those with its own follow camera, which
kept facing where it was when first person started; F9 twice restarts our view from it, which matches the fix you
found.
**Tested how:** Build OK.
**Result:** Not tested in game.
**Still broken / not tested:** Whether bars and sounds now follow; whether this disturbs the game's camera when
first person ends (it should snap back to its own).
**Next:** FOV.

## 2026-10-08 Swift Strike passes through bodies (living and dead)

**Changed:** `genji/src/obstacles.rs`: `bodies()` lists every character's cylinder, dead or alive. `genji/src/movement.rs`:
during a dash, the map rays (wall stop and the "aimed at the ground" ray) skip hits within 0.6 m of a body and go on.
**Why:** Your test of commit `e43bc3c`: shielded enemies, and enemies killed in the path, body-blocked the dash. The
log shows why: the map rays also hit characters. Aiming slightly down at an enemy, the "aimed at the ground" ray hit
the enemy's body (e.g. "3.8 m ahead" where the view meets the ground about 9 m ahead), so the dash stopped there.
**Tested how:** Build OK; 22 unit tests pass.
**Result:** Not tested in game.
**Still broken / not tested:** Dashing through shielded soldiers and dead bodies.
**Next:** Camera sync (health bars, sound), FOV, then the half bar of bleed.

## 2026-10-08 Swift Strike: wider hit reach; stops where you aim at the ground

**Changed:** `genji/src/swift.rs`: the dash hits characters within 0.5 m beyond our body (`REACH`; a soldier is hit
when the dash passes within 1.25 m of its centre). `genji/src/movement.rs`: aiming down, the dash stops where the
view meets the ground (ray from eye height, `DASH aimed at the ground ... stops there`); otherwise unchanged.
**Why:** Your test of commit `eb01728` (23 dashes): enemies with shields took no dash damage, and the dash should
damage what it touches up to its end. The near-miss log showed the missed enemies (soldiers, radius 0.40) passed only
0.07-0.13 m (and 0.35-0.46 m) outside the old reach. Dash damage lowers HP directly, so a shield can't block it; the
shielded soldiers most likely were these near misses (a held shield sticks out beyond the body's collision radius).
Your rule for the ground: Genji stops where the dash hits the ground. I briefly also made shield deflections count as
shuriken hits; you said shields blocking shuriken is right, so that was reverted before commit.
**Tested how:** Build OK; 22 unit tests pass (reach test updated).
**Result:** Not tested in game.
**Still broken / not tested:** Shielded enemies with the new reach; the ground stop; no sound or hit reaction on
dash hits; the bleed idea (asked you).
**Next:** Your test.

## 2026-10-07 Step 7 tested; dash near-miss logging

**Changed:** `genji/src/swift.rs`, `genji/src/movement.rs`, `genji/src/camera.rs`: at the end of every dash, each
character that came within 2 m of our reach without being hit is logged (`DASH near miss: ... passed X m outside our
reach`, and whether it was at our height), plus a summary line per dash.
**Why:** Your test of commit `031aec1`: dash, dash from jumps, runes for dash kills and the kill reset (dash and
shuriken kills) all work; but sometimes a dash doesn't hit the enemy. The log only had hits (15 dashes, 11 hits, 7
kills), so it couldn't say why.
**Tested how:** Build OK; 22 unit tests pass.
**Result:** Not tested in game. Also seen in that log: dashes that hit an enemy were often shorter (8-13 m, with
"wall hits"), and after dash #13 something moved the character 5 m without us ("outside push 5.27 m"); not yet
explained.
**Still broken / not tested:** Missed dashes; open question whether a dash into the ground should stop there (asked
you).
**Next:** A session dashing at enemies, then fix from the numbers.

## 2026-10-07 Step 7: Swift Strike (dash, damage, cooldown, kill reset)

**Changed:** `genji/src/movement.rs`: Left Shift starts a dash of 15 m at 50 m/s (about 0.3 s) along the view: through
characters, stopped by walls (same wall rays as walking), along the ground at full speed when looking flat or down,
lifting off when looking up, no gravity or wall climb during it, falling normally after it in the air; `DASH` start
and end lines with the distance travelled. New `genji/src/swift.rs`: 8 s cooldown, 50 damage once to each character
the dash's stretch touches (`DASH HIT`), reset on an elimination (`SWIFT reset`), 3 unit tests. New
`genji/src/kills.rs`: characters we damaged (shuriken or dash) that die within 3 s are our eliminations (`KILL`). New
`genji/src/targets.rs`: the target list and HP damage moved out of `shuriken.rs` (no behaviour change for shuriken,
except their hits now count for eliminations). `genji/src/camera.rs`: wiring. `docs/DESIGN.md`: section 5d.
Earlier test: you confirmed the crosshair shows up.
**Why:** Step 7.
**Tested how:** Build OK, no warnings; 22 unit tests pass.
**Result:** Not tested in game. Values from memory of Overwatch, not verified.
**Still broken / not tested:** Everything in game; whether dash kills give runes (the engine registers no hit for
dash damage), no hit reaction from enemies on a dash hit.
**Next:** Your test.

## 2026-10-07 Crosshair (hudhook overlay), first part of step 12

**Changed:** New `genji/src/crosshair.rs`: hooks Direct3D 12's frame presentation with **hudhook 0.9.3** (MIT,
crates.io, only its Direct3D 12 part) and draws your Overwatch crosshair on ImGui's foreground layer: four green bars
(length 6, gap 13, thickness 1, 80%) with a 1 px black outline at 50%, sizes scaled from 1080p to your screen height
(1440p: gap 17 px, length 8 px). ImGui's `imgui.ini`/log files are switched off (they'd be written into the game
folder) and it may not touch the mouse cursor or take input. `genji/src/camera.rs`: crosshair visible exactly while
the first-person view is drawn. `genji/src/lib.rs`: installs the hook after CSTask is ready. `genji/Cargo.toml`:
hudhook. Earlier test: you said the Overwatch-measured shuriken values feel better.
**Why:** You asked for a crosshair; option B (a real overlay), since step 12's HUD needs it anyway.
**Tested how:** Build OK, no warnings; 19 unit tests pass (2 new: bar positions at 1440p, outline around bars).
**Result:** Not tested in game. How Overwatch converts its numbers to pixels is assumed (pixels at 1080p, gap from
the centre to each bar), not verified; "outline shift" isn't modelled.
**Still broken / not tested:** Whether the hook works under me3; how it looks next to Overwatch's.
**Next:** Your test; then step 7 (Swift Strike).

## 2026-10-07 Shuriken values from an Overwatch video measurement

**Changed:** `genji/src/firing.rs`: burst cycle 0.68 -> 0.88 s (start to start: 0.2 s burst + 0.68 s recovery),
fan cycle 0.75 -> 0.68 s, fan 9 -> 4.5 degrees each side (9 in total). `genji/src/shuriken.rs`: speed 60 -> 75 m/s.
Unchanged: 30 shuriken, 0.1 s between burst shuriken, 1.5 s reload, 27/54 damage.
**Why:** Make it exactly like Genji. Values from another AI's analysis you passed on: measured in an Overwatch video
(0.10 s between shuriken, 0.687 s between fans, reload about 1.5 s) and Blizzard's notes (0.68 s recovery,
75 m/s, 9 degree spread). The burst cycle is inferred from those, not measured. Field of view measured last session:
Elden Ring 48 degrees vertical (76.7 horizontal at 16:9, about 93.5 on your 21:9 screen), Overwatch 103 horizontal at
16:9; you chose to keep Elden Ring's.
**Tested how:** Build OK; 17 unit tests pass.
**Result:** Not tested in game.
**Still broken / not tested:** How it feels compared with Overwatch.
**Next:** Your test, then step 7 (Swift Strike).

## 2026-10-07 Log the game's field of view; step 6b tested

**Changed:** `genji/src/camera.rs`: when first person starts, a `CAMERA` line logs the field of view the game draws
with (raw value, vertical and horizontal degrees, and the horizontal value at 16:9 for comparison with Overwatch).
**Why:** You asked for the exact FOV of both games, to match Genji exactly. It isn't known yet for Elden Ring.
**Tested how:** Build OK; 17 unit tests pass. Step 6b in game (commit `627249a`): you said everything works.
**Result:** FOV line not tested in game.
**Still broken / not tested:** Whether the engine's value is vertical (assumed, as usual for the engine; the log will
show whether vertical or horizontal fits the numbers).
**Next:** One short session to read the value; then exact Genji values for 6b.

## 2026-10-07 Step 6b: burst of 3, Fan of Blades, 30 shuriken, automatic reload

**Changed:** New `genji/src/firing.rs` (timing only, 6 unit tests): hold left click = bursts of 3 (0.1 s apart,
new burst every 0.68 s); hold right click = 3 at once, centre and 9 degrees to each side (every 0.75 s); shared
recovery; 30 shuriken, 1.5 s automatic reload at 0. `genji/src/shuriken.rs`: `throw` takes a sideways angle, logged
as `side ... deg`. `genji/src/camera.rs`: uses `firing` instead of one throw per click. `docs/DESIGN.md`: 6b plan.
Earlier test: you confirmed the game now starts in plain Elden Ring.
**Why:** Step 6b.
**Tested how:** Build OK, no warnings; 17 unit tests pass (burst spacing and cycle, fan spread, no fan during a
burst, reload, burst dropped when the controller lets go).
**Result:** Not tested in game. Timing values are from memory of Overwatch, not verified.
**Still broken / not tested:** Everything in game; how the values feel compared with Overwatch.
**Next:** Your test, then step 7 (Swift Strike).

## 2026-10-07 Game starts in plain Elden Ring (Genji mode off until F9)

**Changed:** `genji/src/controls.rs`: Genji mode starts off. `README.md`, `docs/DESIGN.md`: noted.
**Why:** You asked for the game to start in normal Elden Ring.
**Tested how:** Build OK; unit tests pass.
**Result:** Not tested in game.
**Still broken / not tested:** Startup in plain mode, F9 turning Genji mode on.
**Next:** Your test, then step 6b.

## 2026-10-07 Shuriken limit fix tested in game: no limit

**Changed:** Nothing. Test of commit `9e55321`.
**Why:** Check the one-shot flag fix.
**Tested how:** You threw at walls and enemies. I read `logs/genji.log`.
**Result:** You saw it working. Log: 510 clicks, 510 throws `ok`, 0 refused; 22 hits, 9 headshots; the second
pool never needed an extra allocation (`effects 0/0` all session), so its 64 reserved slots are freed and reused.
**Still broken / not tested:** Nothing for this fix.
**Next:** Step 6b.

## 2026-10-07 Shuriken limit fixed (one-shot flag found with Ghidra); P debug switch removed

**Changed:** `genji/src/shuriken.rs`: every throw sets bit 0 of the spawn request's flags field (`unk44 = 1`,
`SPAWN_ONE_SHOT`); spawn variants, the projectile switch and their logging removed. `genji/src/camera.rs`: P no
longer does anything. `genji/src/dinput.rs`: unused P key code removed. `STATUS.md`: "Resolved" section.
`docs/DESIGN.md`: one-shot flag. `README.md`: shuriken line. Outside git: Ghidra 12.1.4 (official release,
SHA-256 matches GitHub's) in `third_party/ghidra/`, its project and outputs in `third_party/ghidra-work/`
(gitignored); `eldenring.exe` was only read.
**Why:** Throws were refused after 256 (STATUS.md). Ghidra showed the per-throw emitter record is freed only when
that bit is set (details in STATUS.md).
**Tested how:** Build OK, no warnings; 11 unit tests pass. Re-read the part 2 log: with `unk44 = 1` the pool's live
count stayed flat (32-33) while 11 records were created, so they were freed; I had misread that test.
**Result:** Not tested in game.
**Still broken / not tested:** Unlimited throwing in game; whether the flag changes anything else about the throw
(the part 2 session showed throws `ok` with it, hits weren't checked in that window).
**Next:** Your test: 300+ throws at walls and an enemy.

## 2026-10-07 STATUS option B part 2 tested in game: every variant leaks; options updated

**Changed:** `STATUS.md` only. Test of commit `5ee9fa5`.
**Why:** Find a spawn-request field that releases the per-throw record.
**Tested how:** About 70 throws, then 10 per variant with P. I read `logs/genji.log`.
**Result:** All variants leaked one record per throw: baseline 26/26 (after the buffer filled; 64 earlier throws
couldn't be judged), goods 1700 16/16, dummy poly 0 10/10, unk44 1 11/11, target = owner 13/13, behavior/magic 0
12/12, unk10 1 11/11, unk28/2c 1 13/13, unk40 1 6/6. You saw the dagger come from above on the 3rd or 4th P: variant
#2 (`dummy_poly_id = 0` picks an attachment point on the model). Remaining options in `STATUS.md`: Ghidra headless
(recommended), more blind variants, or ghidra-mcp.
**Still broken / not tested:** Unlimited throwing.
**Next:** Your choice in `STATUS.md`.

## 2026-10-07 STATUS option B, second part: spawn-request variants (P switches); one record per throw

**Changed:** `genji/src/shuriken.rs`: the projectile is fixed at the Throwing Dagger again; **P** now switches
between 9 spawn-request variants (baseline; `goods_id` 1700; `dummy_poly_id` 0; `unk44` 1; target = owner;
behavior/magic id 0; `unk10` 1; `unk28`/`unk2c` 1; `unk40` 1). Every throw logs the second pool's extra count before
and after (`VARIANT throw #n with #v: ... -> ... (+1, leaked | +0)`). `genji/src/camera.rs`: P calls the variant
switch. `STATUS.md`: option B result.
**Why:** Your option B test (entry below) showed the leak is one record per spawn call, whatever the projectile.
**Tested how:** Build OK, no warnings; 11 unit tests pass.
**Result:** Not tested in game.
**Still broken / not tested:** Which field (if any) releases the record.
**Next:** Your variant session.

## 2026-10-07 STATUS option B tested in game: every projectile leaks one record per throw

**Changed:** Nothing. Test of commit `27781d5`.
**Why:** Compare projectile rows.
**Tested how:** Bone Dart until refused, rest at a grace, P through all rows. I read `logs/genji.log`.
**Result:** You saw: Bone Dart hit the limit; resting didn't help; P seemed to do nothing. Log: P worked (all five
rows were switched in, twice), but the pool is shared, so once full every row is refused. Per 2 s interval the pool
grew by exactly the number of successful throws (+22/22, +22/22, +20/20, +19/19 ...); 256 throws succeeded, then
55 refused. The pool stayed 192/192 from 108 s to the end (80 s, including the rest). The impact counter stayed 0
(the impact detection didn't fire this session; to look at separately).
**Still broken / not tested:** Unlimited throwing.
**Next:** Spawn-request variants (entry above).

## 2026-10-07 STATUS option B: compare projectile rows (P switches)

**Changed:** `genji/src/shuriken.rs`: the projectile is now one of five player-owned rows from your
regulation.bin, switched with **P**: Throwing Dagger 10170000 (start), Bone Dart 10171000, Kukri 10173000, Ruin
Fragment 10176000, Glintstone Pebble 10400000 (sorcery; its attack row is 40000, so the attack row is now read from
the bullet instead of assuming the same id). Each gets the same shuriken values while active and its vanilla values
back when you switch or leave Genji mode. The `SHURIKEN` line shows the active projectile, impacts since the switch,
the pool levels, the peak extra count since the switch, and a heuristic count of marked buffer slots. `PROJECTILE`
lines mark each switch. `genji/src/camera.rs`: P key; `genji/src/dinput.rs`: P code.
**Why:** You chose option B in `STATUS.md`.
**Tested how:** Build OK, no warnings; 11 unit tests pass.
**Result:** Not tested in game.
**Still broken / not tested:** Whether other rows behave the same with our values (speed, no drop); P might do
something in Elden Ring itself (not bound by default as far as I know).
**Next:** Your comparison session.

## 2026-10-07 STATUS option A result: pool entries are plain impact records; diagnostic removed (it froze the game)

**Changed:** `genji/src/pooldiag.rs` removed (kept in git history at `b0ad34a`), its call removed from
`genji/src/shuriken.rs`, module removed from `genji/src/lib.rs`. `STATUS.md` updated with the result and the
remaining options.
**Why:** Your test of `3309b4d`: the screen froze for moments. The log shows each diagnostic run took about 4 s on
the game's frame (39.108 -> 43.541 s, 53.115 -> 57.171 s, 66.882 -> 72.890 s): my code checked the memory before
every 8-byte read, thousands of times in a row. Not your PC or Codex.
**Tested how:** Build OK, no warnings; 11 unit tests pass.
**Result:** Diagnostic output (from your test): all 64 buffer slots and the head entry have **no class name**
(plain data, not objects); a live entry holds a small id (`0xff00`, `0xff24`), empty handles (`0xffffffff...`), a
3x3 rotation at `+0x900` and a position at `+0x930` near the impact point, and 0.1 / 0.785 (45 degrees) / 79.2 at
`+0x6f0` / `+0x6fc`, `+0x700` / `+0x8fc`. Not matched to any dagger Bullet value. No "next" pointer found.
**Still broken / not tested:** Unlimited throwing. The build without the diagnostic is not tested in game (it
removes code only).
**Next:** Your choice: option B (recommended) or C in `STATUS.md`.

## 2026-10-07 Step 6 diagnostic: trigger fixed (it never ran)

**Changed:** `genji/src/shuriken.rs`: the pool diagnostic now runs whenever the second pool's live list has a head
(`+0x28` non-zero), at most every 10 s.
**Why:** Your test of `b0ad34a`: 45 throws, no `POOLDIAG` lines. I made it wait for the pool's extra-allocation
count (`+0x38`), which stays 0 until the 64 built-in slots are used up; 45 hits never got there. My mistake.
**Tested how:** Build OK, no warnings.
**Result:** Not tested in game.
**Still broken / not tested:** Diagnostic output.
**Next:** Same short test again.

## 2026-10-07 Step 6: STATUS.md option A, name what fills the bullet manager's second pool; hit effect back on

**Changed:** New `genji/src/pooldiag.rs` (read-only diagnostic), called from `genji/src/shuriken.rs` at most every
10 s while the second pool holds anything; `genji/src/lib.rs` registers it. `genji/src/shuriken.rs`: the hit effect
(SFX 300002) is back on (your choice); bounce effect stays off, flight visual stays on "Deletion".
- The diagnostic reads C++ class names from RTTI (each object's virtual table is preceded by a pointer to RTTI data
  holding the name, `.?AV...@@`): for all 64 buffer slots (counted by class), for the head entry and any objects
  embedded in its first 0x200 bytes; it logs the head entry's first bytes and plausible float fields (timers); it
  looks for the entry's "next" pointer (a field pointing to another entry of the same class) and walks the live list,
  counting by class. Every table and RTTI pointer is checked to lie inside the exe image, every other read is checked
  with VirtualQuery. Log lines `POOLDIAG`.
**Why:** You chose option A in `STATUS.md`, and to keep the hit effect.
**Tested how:** Build OK, no warnings; 11 unit tests pass.
**Result:** Not tested in game.
**Still broken / not tested:** Throwing still stops after the pool fills (this build only diagnoses). The entries
may not carry a class name (then the log says "(no class)" and we use the raw bytes and timers instead).
**Next:** Your short test: about 20 throws at a wall, then quit.

## 2026-10-07 Step 6 hit-effect fix tested in game: FAILED; stopped, STATUS.md written

**Changed:** New `STATUS.md`. No code change. Test of commit `479075c`.
**Why:** Fifth failed attempt at "throwing stops"; AGENTS.md says stop and write `STATUS.md`.
**Tested how:** Many throws at a wall. I read `logs/genji.log`.
**Result:** You saw: no effects on the wall any more, but throwing still stops after a while. Log: all values
applied (bounce, hit effect none, flight visual deleted); the effect pool still filled: `0/0` at 46 s, `80/80` at
56 s, `165/165` at 66 s, `192/192` at 68.8 s; throw #257 failed at 68.8 s; 28 failures in 284 throws. So the pool
that fills is not visual effects. Options in `STATUS.md` (recommended: a read-only diagnostic that names the class
of the objects filling the pool).
**Still broken / not tested:** Unlimited throwing.
**Next:** Your choice of option in `STATUS.md`.

## 2026-10-07 Step 6: shuriken hit effect switched off (fifth attempt at "throwing stops")

**Changed:** `genji/src/shuriken.rs`: while Genji mode is active, the dagger's hit effect (`sfxId_Hit`, vanilla
300002) is set to none, restored with the other values. Consequence: no spark where a shuriken lands, until
Genji's own visuals replace the dagger's.
**Why:** Your test (entry below): with the flight visual set to "Deletion", the effect pool still filled. What
creates effects, and what's been ruled out: the projectiles (removed right after hitting, attempt 2), the bounce
effect (off, attempt 3), the flight visual (set to delete, attempt 4, applied per the log). The pool only grows when
shuriken hit walls, not when thrown at the sky, and the stuck-looking daggers vanish after a while: that matches the
hit effect, the last effect this projectile creates. **If this doesn't stop the pool from filling, I stop and write
`STATUS.md` (AGENTS.md rule).**
**Tested how:** Build OK, no warnings; 11 unit tests pass.
**Result:** Not tested in game.
**Still broken / not tested:** In game.
**Next:** Your test; the pool line should stay low while you hit walls.

## 2026-10-07 Step 6 flight-visual deletion tested in game: FAILED (effect pool still fills)

**Changed:** Nothing. Test of commit `bdc4955`.
**Why:** Step 6 checks.
**Tested how:** Many throws at a wall, then at the dragons. I read `logs/genji.log` and your screenshot (daggers
stuck in the wall).
**Result:** The new values were applied (`visual on hit/life end 0/0 -> deleted`). Still: effect pool `6/6` at
64.8 s, `145/145` at 80.9 s, `192/192` from 86.5 s; throw #257 failed at 86.5 s; 40 failed throws. After your
death the pool was back to 0 (149.6 s) and climbed again (140 at 200 s). You saw stuck daggers that disappear after
some time, and throwing stops, also at the dragons.
**Still broken / not tested:** See entry above.
**Next:** Fix above.

## 2026-10-07 Step 6: shuriken flight visual deleted on hit (the effect-pool leak)

**Changed:** `genji/src/shuriken.rs`: while Genji mode is active, the dagger's Bullet row also gets
`bulletSfxDeleteType_byHit` and `bulletSfxDeleteType_byLifeDead` = 1 ("Deletion": the flight visual, the dagger
model, is removed when the projectile hits or its life ends), restored with the other values. Meaning of the
values from the community Paramdex shipped with WitchyBND (`ER/Meta/BulletParam.xml`): 0 = "Emitter Stopped"
(visual left to finish by itself), 1 = "Deletion". The dagger uses 0.
**Why:** Your test (entry below) and its log proved where the limit is.
**Tested how:** Build OK, no warnings; 11 unit tests pass.
**Result:** Not tested in game.
**Still broken / not tested:** Whether the visual's deletion also removes its slot in the effect pool (the pool
line will show it: effects should go back down shortly after each throw).
**Next:** Your test.

## 2026-10-07 Step 6 bounce-effect fix tested in game: the effect pool is the limit (proved)

**Changed:** Nothing. Test of commit `04d899f`.
**Why:** Step 6 checks.
**Tested how:** You threw at a wall until throwing stopped, died, and killed the dragon. I read `logs/genji.log`
and your screenshot.
**Result:**
- You saw: daggers now stick *into* the wall (handles showing) instead of hanging in front of it, and some
  disappear; throwing still stops at some point.
- Log: the effect pool's live count climbed while you hit the wall (`effects 9/9`, `23/23`, `43/43` ... `170/170`
  within 22 s; about 7 per second of wall hits); the bullet pools stayed at 0. **Throw #257 failed at the exact
  moment `effects 192/192`** (192 extra + 64 built-in = 256). Later the count went back down (156), so effects can
  be freed; ours weren't. 95 body hits, **19 headshots**: headshots work.
- So the bounce effect wasn't the whole story: the dagger's flight visual itself stays behind ("Emitter
  Stopped").
**Still broken / not tested:** See entry above.
**Next:** Fix above.

## 2026-10-07 Step 6: switch off the dagger's bounce effect for shuriken; log the bullet manager's pools

**Changed:** `genji/src/shuriken.rs`: while Genji mode is active, the dagger's Bullet row also gets "bounce effect"
(`sfxId_Flick`, vanilla 300003) set to none, restored with the other values. Every 2 s, and on any failed throw,
the log shows the fill levels of the bullet manager's first three pools (alive / heap allocations ever), read from
fromsoftware-rs's layout: bullets (128 + 128), bullet effects (64 + 192), other (4 + 28).
**Why:** Your test (entry below): the projectiles were all removed after hitting, yet daggers still hung on the
wall and throw #257 failed again. My explanation (not verified; the pool levels in the next log will show it): what
hangs on the wall is the bounce *effect*, which falls under the projectile's gravity (0 for shuriken) and so never
ends; effects have their own pool of 64 + 192 = 256. The stuck-removal from the previous change never ran (no
projectile was ever stuck), and stays as a safety net.
**Tested how:** Build OK, no warnings; 11 unit tests pass.
**Result:** Not tested in game.
**Still broken / not tested:** If the pools' "ever allocated" counter turns out to be the real limit (the library
says it never decreases), extra effects beyond the 64-slot buffer would eventually run out even when they end; the
log will show it.
**Next:** Your test.

## 2026-10-07 Step 6 stuck-shuriken fix tested in game: daggers still hang on the wall, throw #257 fails

**Changed:** Nothing. Test of commit `9325717`.
**Why:** Step 6 checks.
**Tested how:** You threw at walls and enemies. I read `logs/genji.log` and your screenshot (daggers still on the
wall).
**Result:** You saw: daggers disappear (some), but throwing stops again; enemies get hit; headshots probably work.
Log: `live` was 0 or 1 in every report and `ended` always equalled `thrown`: **every projectile was removed by
the engine shortly after its throw**; no stuck projectile was ever found (`stuck 0`, nothing removed by either
method). Throw #257 failed with error 4 at 131.5 s, with 0 of ours alive. 12 hits by 129 s.
**Still broken / not tested:** See entry above.
**Next:** Fix above.

## 2026-10-07 Step 6: remove shuriken that got stuck after hitting a wall; count hits at impact

**Changed:** `genji/src/shuriken.rs`: the tracker now follows our shuriken in every state (wait, fly, explosion),
not only flying. Impact = the first frame a shuriken slows below 30 m/s (half its speed); the hit test runs then,
over the stretch it flew since its last fast frame. A shuriken that stays below 1 m/s after impact is "stuck" and
gets removed: first its remaining life (the engine's private `life`, right after `time_alive`) is set to 0; if it
is still there 0.5 s later, it is sent straight down at 30 m/s so it hits the ground like a vanilla dagger. The
`SHURIKEN` report shows live and stuck counts and how many stuck ones went away after each method.
**Why:** Your test (entry below): shuriken that hit a wall stay there forever.
**Tested how:** Build OK, no warnings; 11 unit tests pass.
**Result:** Not tested in game.
**Still broken / not tested:** Whether either removal method works (the log will say which). A shuriken stuck in a
moving enemy may move faster than 1 m/s and not count as stuck; it goes when the enemy dies, as before.
**Next:** Your test.

## 2026-10-07 Step 6 rework tested in game: aim, headshots and runes work; shuriken pile up on walls

**Changed:** Nothing. Test of commit `f05ff79`.
**Why:** Step 6 checks.
**Tested how:** You threw at targets near and far, hit bodies and heads, killed enemies, hit walls and the dragon
near Limgrave. I read `logs/genji.log` and your screenshot (a wall full of daggers).
**Result:**
- You saw: **aim centred, runes for kills, headshots working.** But shuriken that hit a wall stay stuck, and
  throwing stops again after a while (also against the dragon, which has too much HP to die); killing an enemy
  removes the ones stuck in it and lets you throw again.
- Log: `SHURIKEN flying 0` most of the time: the tracker only followed bullets in the flying state, so stuck ones
  were invisible to it. The "oldest" ages it reported (up to 178 s in the first minute) aren't plausible, so
  `time_alive` isn't seconds since the throw. 35 hits, 3 headshots by 88 s.
- Cause (my explanation): the dagger's bounce ("flick", effect 300003): a bounced dagger normally falls under
  gravity and ends on the ground; with our zero gravity it hangs in front of the wall forever.
**Still broken / not tested:** See entry above.
**Next:** Fix above.

## 2026-10-07 Step 6: aim at the crosshair target, Overwatch hand position, no endless shuriken, Overwatch damage

**Changed:** `genji/src/shuriken.rs` rewritten (one commit; the parts depend on each other):
- **Hand position** like your Overwatch screenshot: 0.6 m forward, 0.08 m right, 0.06 m down from the eye (just
  right of and below the centre, not at the far right).
- **Aim at the real crosshair target:** a ray along the view against the map, plus the characters' collision
  cylinders, finds what's under the crosshair (up to 80 m); the shuriken flies from the hand to that point. Before,
  it aimed at a fixed 20 m point, so far shots drifted.
- **No endless shuriken:** after 80 m they now fall (gravity 20 m/s^2 out of range) so they end on the ground and
  the engine removes them; life set to 3 s. Every 2 s a `SHURIKEN` line reports how many of ours are flying, the
  oldest one's age, the life the engine actually uses, and totals.
- **Overwatch damage:** the engine's own damage is reduced to 1 (it still gives the hit reaction, aggro and the
  "last hit by" record); we follow each shuriken in flight and, when it ends next to a character (within 0.4 m of
  its collision cylinder, checking the stretch it flew in its last frame), take 27 HP, or 54 if it hit the top 20%
  of the character's height (headshot). `DAMAGE_MULTIPLIER` (1.0) scales both. `HIT` lines show body/headshot,
  damage and HP.
- The old 2-second HP watcher is replaced by these direct hit lines.
**Why:** Your test (entry below).
**Tested how:** Build OK, no warnings; 11 unit tests pass (new: the crosshair ray enters a character straight
ahead at the right distance; misses beside, above and behind and when looking straight up; works looking down).
**Result:** Not tested in game.
**Still broken / not tested:** Everything in game. Not sure: whether kills by our damage give runes (the engine's
1-damage hit just before sets "last hit by" to you; please check runes after a kill); whether a shuriken hitting a
wall right next to an enemy counts as a hit (it will, within 0.4 m); whether headshot height fits all enemies.
Elden Ring enemies have far more HP than Overwatch heroes (bosses have thousands), so 27/54 may feel weak; the
multiplier is there for that. The projectile still looks like a dagger (Genji's visuals come later).
**Next:** Your test.

## 2026-10-07 Step 6c early version tested in game: three problems

**Changed:** Nothing. Test of commit `80d34c1`.
**Why:** Step 6 checks.
**Tested how:** You threw at near and far targets and enemies, died and respawned. I read `logs/genji.log`.
**Result:**
- You saw: straight, no drop; not centred far away; looks weird (still a dagger); for a while you couldn't throw
  (after a headshot or getting hit, not sure which); after dying you hit a dog and an enemy underground took damage;
  Overwatch numbers: 27 body, 54 headshot; the shuriken should start just right of and below the centre (screenshot).
- Log: 377 clicks, 373 throws. **Throws #257 to #277 all failed** with engine error 4 (construction failed),
  from 128.7 s until your death at 140.3 s; after the reload they worked again. 256 successful throws since
  loading = the bullet manager's 128 bullets + 128 extra: none of ours had been removed (my explanation: without
  gravity a shuriken that hits nothing never ends). That also explains the far-away "underground" hit.
**Still broken / not tested:** See entry above.
**Next:** Fixes above.

## 2026-10-07 Step 6c (early): shuriken flight and damage values set in memory; throw from the right hand

**Changed:** `genji/src/shuriken.rs`: (1) commit `d981cbd`: the throw starts 0.4 m forward, 0.18 m right and
0.15 m down from the eye (Genji's right hand) and is aimed at the crosshair point 20 m ahead; (2) while Genji mode
is active, the dagger's Bullet row `10170000` gets 60 m/s at constant speed, no gravity, 80 m range (life 1.43 s),
and its AtkParam_Pc row gets base physical attack 150 (vanilla 67); the vanilla values are restored whenever Genji
mode lets go (menus, F9, death, Torrent, ladders). In memory only, through fromsoftware-rs `SoloParamRepository`;
nothing on disk changes. `genji/src/camera.rs` switches the values on and off. `docs/DESIGN.md`: 6c now uses
in-memory edits instead of a modified `regulation.bin`.
**Why:** Your 6a test (entry below): the dagger's hilt showed in the middle of the view; the projectile drops.
Speed and damage values are approximate (Overwatch), the damage a first guess.
**Tested how:** Build OK, no warnings; 8 unit tests pass.
**Result:** Not tested in game.
**Still broken / not tested:** Whether projectiles already spawned keep their values (they should). The real
Throwing Dagger item, if you throw it in Genji mode with R, gets the shuriken values too. Damage needs tuning.
**Next:** Your test.

## 2026-10-07 Step 6a tested in game: PASSED

**Changed:** Nothing. Test of commit `70d37c8`.
**Why:** Step 6a pass condition.
**Tested how:** You threw daggers at walls, ground, sky and an animal. I read `logs/genji.log`.
**Result:** You saw: daggers fly and hit enemies, nothing odd; the dagger's hilt shows in the view at spawn;
the projectile drops. Log: 157 throws, all `ok` (no engine errors); 37 `HIT` lines on an animal at 12 to 18 m,
3 to 5 damage each.
**Still broken / not tested:** Hilt in view, drop, low damage (entry above).
**Next:** Fixes above.

## 2026-10-07 Step 6a: left click throws a projectile (Elden Ring's Throwing Dagger) from the eye

**Changed:** New `genji/src/shuriken.rs`; `genji/src/camera.rs` throws on each left-click press while our
controller drives and runs a hit watcher every frame; `genji/src/lib.rs` registers the module. Outside the repo
(gitignored): WitchyBND 3.0.1.0 (`WitchyBND-3.0.1.0-win-x64.zip`, 48,308,252 bytes, SHA-256 `a3e6b2a0...0364`,
official GitHub release of ividyon/WitchyBND) in `third_party/witchybnd/`; a copy of your `regulation.bin`
(identical hash) unpacked into `erdata/vanilla/`, with `Bullet`, `EquipParamGoods`, `BehaviorParam_PC` and
`AtkParam_Pc` converted to XML. The game folder was only read.
**Why:** Step 6a of the plan (`docs/DESIGN.md` section 5c).
- Projectile: item 1700 "Throwing Dagger" points to Bullet row `10170000` (speed 32 m/s, life 0.85 s, range
  15 m, some gravity, effect 300001) and attack row `10170000` (67 physical, correction 120).
- Spawn request: our own copy of the engine's 0x110-byte layout, filled like a working Elden Ring mod does
  (justbustin/minecraft-crossover-bridge, MIT): owner = the player's handle, behavior/magic/goods/dummy-poly -1,
  no target, right/up/forward/position from our look and eye. Size and field offsets are checked at compile time.
- Logging: `THROW #n bullet ... from ... dir ... : ok` or the engine's error code; `HIT character ... hp a -> b`
  for any nearby character losing HP within 2 s of a throw.
**Tested how:** Build OK, no warnings; 8 unit tests pass; size and offset checks pass at compile time.
**Result:** Not tested in game.
**Still broken / not tested:** Everything in game. Unknown: whether the engine accepts the request on this
version (error 3 = invalid bullet id, 4 = construction failed); whether the dagger looks right flying from the
eye. One dagger per click, no ammo or burst yet (6b).
**Next:** Your test.

## 2026-10-07 Step 5e tested in game: PASSED (step 5 done)

**Changed:** Nothing in code. Test of commit `b523430` (three sessions). `README.md`, `docs/DESIGN.md` updated.
**Why:** Step 5e pass condition.
**Tested how:** (1) flask and weapon swaps while running; (2) a door and a chest; (3) getting hit by enemies.
I read `logs/genji.prev.log` and `logs/genji.log`.
**Result:**
- You saw: flask and weapon swap work while running; door and chest work; getting hit doesn't break anything.
- Log: item pickup `63000` handed over for 0.48 to 0.64 s; chest (probably `101012`, root motion 0.0049) 1.23 s;
  door `60370` (root motion 0.0094, then 0.0028) 1.00 s + 2.99 s; one more `12020100` (root motion 0.0756) 0.61 s.
  Each time we drove again afterwards. 5 hits taken in session 3, none triggered a hand-over (their root motion
  stayed below the threshold). Animations starting `12...` (`12000000`, `12020100`, `12022100`) appear to be a
  different weapon stance; they don't interfere.
**Still broken / not tested:** Being grabbed (no grabbing enemy nearby). Heavy knockback is not handed to the
game (only root motion above 0.002 is).
**Next:** Step 6, Shuriken and Fan of Blades.

## 2026-10-07 Step 5e, second try: step aside only when the game's animation moves the character

**Changed:** `genji/src/movement.rs`: a non-idle animation now hands control to the game only while its root
motion (`CSChrBehaviorModule.root_motion`, the movement the animation itself asks for) is above 0.002; in-place
animations (flask, weapon swap) no longer interrupt our movement. A game action ends when the root motion has
stayed below the threshold for 0.3 s (or the animation is idle again). Every animation change is logged with its
root motion (`ACTION animation A -> B (root motion R)`), to learn the unit and tune the threshold.
`genji/src/camera.rs`: Elden Ring's own walking stays blocked during game actions.
**Why:** First 5e test (entry below).
**Tested how:** Not built yet at the time of writing (tool trouble); see the commit for the build result.
**Result:** Not tested in game.
**Still broken / not tested:** The meaning and unit of `root_motion` are not documented; the threshold is a guess.
If it's wrong, either doors and grabs won't hand over (we'd keep driving through them), or in-place animations
will still interrupt movement. The log will show which.
**Next:** Your test.

## 2026-10-07 Step 5e first test: item pickup passed; flask and weapon swap broke movement

**Changed:** Nothing. Test of commit `ffb6e98`.
**Why:** Step 5e pass condition.
**Tested how:** You picked up items, drank the flask, pressed the arrow keys (weapon/item switching), talked to
an NPC and rode Torrent. I read `logs/genji.log`.
**Result:**
- Item pickup: passed (`63000` -> `2004050`, game action 0.44 s, then we drove again).
- Flask (`50110`, `50111`, `50112`) and weapon swaps (`2029002`, `29022`, `29001`, `2029021`, `2029031`,
  `2029051`) started game actions, as designed, but Elden Ring's own walking was unblocked during them; you kept
  moving, so its walk/run animations (`2020000`, `2020100`, `2022100`, `2202020`, `2202040`, `2202115`) played and,
  not being idle, kept the game action going: 4.3 s, 14.9 s and longer. You saw "dull" movement and no double jump:
  that was Elden Ring's own walking.
- NPC dialogue and Torrent: worked. Doors, levers, chests, hits and grabs: not reached.
**Still broken / not tested:** See entry above.
**Next:** Second try (entry above).

## 2026-10-07 Step 5e: step aside while the game plays its own animations

**Changed:** `genji/src/movement.rs`: the controller now returns one of three results (`Driving`, `GameAction`,
`NotReady`). Any animation other than standing idle (`2000000`), "none" (`-1`) or the idle seen at the first
takeover means the game is doing something itself (door, lever, chest, item pickup, flask, hit reaction, being
grabbed): the controller hands movement and gravity back at once and takes over again when the animation is idle
and the game reports the character on the ground for 0.3 s. `genji/src/camera.rs`: during a game action the view
stays first person, following the character as the game moves it (mouse look still works); Elden Ring's own
movement is unblocked meanwhile. Log lines `ACTION game action: animation N`, `game action continues`, `game
action over after Xs`.
**Why:** Step 5e of the plan. Menus, dialogue, cutscenes, ladders, Torrent and death already hand back (third
person). Evidence for the rule: in the 5b/5c/5d logs the animation was `2000000` in more than 600 movement
reports while we drove (plus 8 reports of `63000` and 4 of `-1`).
**Tested how:** Build OK, no warnings; 8 unit tests pass.
**Result:** Not tested in game.
**Still broken / not tested:** Everything in game. Unknown: what `63000` is (it would now count as a game action);
whether other stances (two-handing, different weapons) use a different idle animation (then the controller
would never drive; the log would show a game action that never ends).
**Next:** Your test.

## 2026-10-07 Step 5d: one-climb-per-airtime rolled back

**Changed:** `genji/src/movement.rs` restored to its state at commit `2dceb8e` (before `68a5a2e`). Climbing works
as in the passed 5d test again: a 1 s budget per airtime that can be split across several climbs.
**Why:** You tested the one-climb change: "climbing got way worse", and asked for the rollback (you just won't
use the repeat climb).
**Tested how:** Build OK, no warnings; 8 unit tests pass. The test log (25 climb starts) doesn't show clearly
what felt worse: the ones that finished reached the top normally (0.32 to 0.49 s). One climb ended after 0.02 s,
which fits a guess (not verified): on uneven rock the wall check can miss for a single frame, and with the
change that one frame ended the climb for the whole airtime.
**Result:** Rollback not tested in game (the code is identical to the version you tested and passed).
**Still broken / not tested:** Repeat climbs in one airtime are possible again, by your choice.
**Next:** 5e.

## 2026-10-07 Step 5d: one wall climb per airtime

**Changed:** `genji/src/movement.rs`: however a climb ends (Space released, top reached, time up, ceiling), the
rest of the climb time is used up; the next climb needs the feet on the ground.
**Why:** Your request: jumping off a wall and going back let you climb again with the time left over.
**Tested how:** Build OK, no warnings; 8 unit tests pass.
**Result:** Not tested in game.
**Still broken / not tested:** In game.
**Next:** Your quick check, then 5e.

## 2026-10-07 Steps 5c and 5d tested in game: PASSED

**Changed:** Nothing. Test of commits `44a6178` and `2dceb8e`.
**Why:** Steps 5c and 5d pass conditions.
**Tested how:** You double jumped and climbed walls. I read `logs/genji.log` (1,268 lines).
**Result:** You: "perfect", except that a second climb was possible after jumping off a wall (entry above).
Log: 65 jumps, 40 double jumps, 31 wall climbs (18 reached the top and vaulted, 3 ended by releasing Space,
4 ran out of time, the rest ended at ceilings; 7 ceiling hits in total), no `GAVE UP`, no HP lost by anyone,
no damage to you.
**Still broken / not tested:** Nothing reported.
**Next:** One climb per airtime (entry above).

## 2026-10-07 Steps 5c and 5d: double jump and wall climb

**Changed:** `genji/src/movement.rs`, two commits. 5c (`44a6178`): a second Space press in the air jumps again at
full jump speed, once per airtime, restored on landing. 5d: holding Space in the air while facing a wall within
0.3 m (chest-height ray along where you look) climbs at 7.5 m/s for up to 1 s (about 7.5 m), restored on
landing; ends on releasing Space, running out of time, or a ceiling; when the wall ends at chest height (top
reached) a 6.5 m/s hop carries the feet over the edge. Climbing doesn't use up the double jump. All values are
approximate Overwatch values, **not verified**. Log lines: `double jump`, `wall climb start`, `wall climb end`,
`reached the top`.
**Why:** Steps 5c and 5d of the plan.
**Tested how:** Build OK, no warnings; 8 unit tests pass (new: the vault lifts the feet 1.24 m, more than the
1.1 m to the edge; climb height 7.5 m).
**Result:** Not tested in game.
**Still broken / not tested:** Everything in game. Climbing only checks straight ahead at chest height, so
narrow posts, fences and slanted walls may not count as walls.
**Next:** Your test.

## 2026-10-07 Step 5b tested in game: PASSED (5b done)

**Changed:** Nothing. Test of commit `fa6e894`. `README.md`, `docs/DESIGN.md` updated.
**Why:** Step 5b pass condition, and the damage-to-characters fix.
**Tested how:** You played without F9, walked and jumped into enemies, died and respawned. I read
`logs/genji.log` (1,146 lines).
**Result:** You saw: no damage to enemies, even after dying; no third-person moment noticed. Log: 16 contacts
with characters, **0 HP lost** by them; animation while touching mostly `2000000` (most likely standing idle).
Takeover waited for the game to settle: 0.34 s after loading, 1.62 s after the first respawn, 0.30 s later.
All HP drops in the log were enemies hitting you.
**Still broken / not tested:** After the first respawn the floor didn't render for a moment; the log shows the
game took 1.6 s that time before its collision was ready, so probably the game still streaming the area (not
verified).
**Next:** 5c double jump, 5d wall climb.

## 2026-10-07 Step 5b: take over only after the game has landed the character; log our animation

**Changed:** `genji/src/movement.rs`: before taking over, the game itself must report the character standing on
solid ground and not falling for 0.3 s in a row (after 3 s it takes over anyway and logs that); the waiting
counters reset whenever control goes back to the game. The current animation (TimeAct `anim_id`) is logged at
takeover, in every movement report, and with every `TOUCH` line. `genji/src/obstacles.rs`: `TOUCH` lines carry our
animation.
**Why:** Your three tests (entry below): damage to touched characters only happens when the controller took
over right after loading or respawning, and pressing F9 twice (which lets the game land the character)
stopped it. My explanation, **not verified**: the controller grabbed the character while it was still dropping
onto the ground after the loading screen, so the game never saw it land and kept treating it as falling or
attacking (Elden Ring has plunging attacks that damage and stagger what you fall on). The animation numbers in
the next log will show whether that's right.
**Tested how:** Build OK, no warnings; 6 unit tests pass.
**Result:** Not tested in game.
**Still broken / not tested:** The damage itself, until your test.
**Next:** Your test without F9.

## 2026-10-07 Step 5b obstacle fix tested in game (3 sessions): no fall damage, no rock clipping; damage to characters depends on F9

**Changed:** Nothing. Test of commit `f905288`.
**Why:** Step 5b pass condition.
**Tested how:** You ran three sessions: (1) with F9 off/on, (2) spawned partly inside the swamp floor, F9 twice
fixed it, no damage to an enemy, (3) without F9: damage and staggers, worse after dying. I read `logs/genji.log`
and `logs/genji.prev.log`.
**Result:**
- You saw: no fall damage; no clipping into rocks. Characters took damage and were staggered when the controller
  had taken over right after loading or respawning; pressing F9 twice made it stop completely, even when jumping
  into them.
- Log, session with F9: touched characters 9 times (radius 0.8 to 1.9 m), **no HP lost**.
- Log, last session without F9: **13 hits of 3 to 4 HP** on 4 characters while touching them (for example
  `417 -> 414 -> 411` within 0.1 s, then every 0.3 to 0.4 s). The filter chosen was `0x2` (all bits see the same
  surface, so any works).
**Still broken / not tested:** See entry above.
**Next:** Fix above.

## 2026-10-07 Step 5b: characters are obstacles, no fall damage, foot-level wall check in the air

**Changed:** New `genji/src/obstacles.rs`: nearby living characters (from the game's distance-sorted list) are
upright cylinders (their collision radius and height); moves into them are cut, sideways moves slide around them;
a contact monitor logs touching and any HP a touched character loses (`TOUCH` lines). `genji/src/movement.rs`:
uses it; fall damage removed (falls only logged); in the air a wall ray at 0.1 m is added; the landing ray starts
0.5 m above the feet instead of 0.3 m; reports count `character blocks`. `genji/src/lib.rs`: new module.
**Why:** Your 5b test (entry below): you prefer no fall damage in Genji mode; walking into an animal damaged it
repeatedly; a timed jump got you inside a rock.
**Tested how:** Build OK, no warnings; 6 unit tests pass (new: walking straight into a character stops; a
diagonal move slides around it; walking away is never blocked; characters far away, above or below are
ignored).
**Result:** Not tested in game.
**Still broken / not tested:** Whether the animal damage came from overlapping it (my explanation, not
verified: nothing logged other characters' HP in that session; the `TOUCH` lines will show it now). Rock fix:
you couldn't reproduce the bug yourself, so it may stay unverified.
**Next:** Your test.

## 2026-10-07 Step 5b tested in game: jumps work; three issues

**Changed:** Nothing. Test of commit `1596351`.
**Why:** Step 5b pass condition.
**Tested how:** You jumped around, onto rocks, under an overhang, dropped from ledges, walked into an animal. I
read `logs/genji.log` (1,389 lines) and your screenshot.
**Result:**
- You saw: jumps feel similar to Overwatch; fall damage works. Log: 84 jumps, 1 ceiling hit, landings from 0 to
  3.5 m with no damage, and two damaging falls: 10.5 m -> 159 (exactly the formula with max HP 760) and
  10.0 m -> 124 (the formula with max HP 760 gives 127; the max HP at that moment wasn't logged, so the difference
  is unexplained; moot now that fall damage is removed).
- Issues: (1) you prefer **no fall damage** in Genji mode; (2) a specifically timed jump into a rock (screenshot)
  got you inside it, not reproducible afterwards; (3) **walking into an animal dealt it repeated damage**.
**Still broken / not tested:** See entry above.
**Next:** Fixes above.

## 2026-10-07 Step 5b: jump, ceiling check, fall damage

**Changed:** `genji/src/movement.rs`: Space jumps when on the ground, or up to 0.1 s after walking off an edge;
jump height 1.2 m, gravity 17 m/s^2 (approximate Overwatch feel, **not verified** against Overwatch); full air
control at running speed; a ray above the head stops the rise under ceilings; landing applies Elden Ring-style
fall damage, none up to 8 m fallen, rising linearly to all of max HP at 20 m (measured from the highest point of
the jump, or from the edge you walked off); the fall-damage reference moves along when the engine re-centres its
coordinates. `genji/src/camera.rs` passes Space to the controller. Unit tests for the damage curve and the jump
height.
**Why:** Step 5b of the plan. Fall damage kept on purpose although Overwatch has none, so Elden Ring's world
stays dangerous; easy to change.
**Tested how:** Build OK, no warnings; 3 unit tests pass (fall damage: 0 at 1.2 m and 8 m, 400 of 800 at 14 m,
800 at 20 m and above; a simulated 60 fps jump peaks within 8 cm of 1.2 m).
**Result:** Not tested in game.
**Still broken / not tested:** Everything in game. Setting HP directly for fall damage: whether the game shows
the damage and handles death normally is not verified.
**Next:** Your test.

## 2026-10-07 Step 5a tested in game: PASSED (5a done)

**Changed:** Nothing. Test of commit `7a2ef25`. `README.md`, `docs/DESIGN.md` updated.
**Why:** Step 5a pass condition: look, walk on flat ground and slopes, walls stop you, no falling through the
ground, no deaths.
**Tested how:** You stood still and walked around for about a minute. I read `logs/genji.log` (288 lines).
**Result:** You saw: no deaths, walking works fine. Log: controller on from 21.9 s to 84.6 s with no hand-back,
no `HP dropped`, no `GAVE UP`; 123 reports, 51 while moving, top speed 5.50 m/s; walls stopped you in 4 reports.
**Still broken / not tested:** Some outside push remains while walking (average 0.51 m per 0.5 s while moving;
probably slopes), not visible to you. Ledges weren't tried in this session (falling worked in an earlier one).
**Next:** 5b, jump, gravity and fall damage.

## 2026-10-07 Step 5a: zero the game's fall timer while driving

**Changed:** `genji/src/movement.rs`: `CSChrFallModule.fall_timer` is set to 0 every frame while our controller
drives, and once more when it hands back. Notes in the file updated.
**Why:** Fourth 5a test (entry below): every death came at fall timer 12.03 s.
**Tested how:** Build OK, no warnings; unit test passes.
**Result:** Not tested in game.
**Still broken / not tested:** No fall damage at all while our controller drives (the game's fall logic is
neutralised); our own fall damage is part of 5b.
**Next:** Your test.

## 2026-10-07 Step 5a fourth test: first person and movement work; deaths explained (fall timer)

**Changed:** Nothing. Test of commit `a599be0`.
**Why:** Step 5a pass condition.
**Tested how:** You stood, looked right, died; respawned, walked, died. I read `logs/genji.log` (634 lines), and
the black box.
**Result:**
- First person, Space doing nothing, mouse look: working.
- Filter table: after 23 frames of waiting (feet still settling 16 cm above ground after loading), **all bits
  hit the same surface** (except `0x1`, `0x10`, `0x40000000`, which hit nothing). There is no separate
  low-detail layer; the 0.9 m gap in the third test was the character still dropping onto the ground after the
  loading screen. Filter `0x2000058` chosen.
- Ground fight fixed: outside push 0.00 m in the walking reports; walking reached 5.50 m/s.
- **Deaths:** both with full HP, both at the game's **fall timer 12.03 s**. The black box shows the fall timer
  rising every frame the whole time (10.73 to 12.03 s over the last 60 frames) while the game itself reported
  "on ground 1, falling 0" and the position didn't change at all. This is the game's "fell into a pit" kill.
  Looking right was a coincidence.
- The two 800 -> 760 HP changes right after respawning had fall timer 0; the same 800 -> 760 appears in the step
  1 log in plain Elden Ring, so it's not damage from us.
**Still broken / not tested:** The deaths (fix above).
**Next:** Fix above.

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
