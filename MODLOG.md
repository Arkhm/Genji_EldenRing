# MODLOG

Everything that changed and what was tested. Newest entry first. Template:
[MODLOG-template.md](MODLOG-template.md).

## 2026-10-09 Swift Strike cancels Deflect; F6 hides the arms (flicker test)

**Changed:**
- `deflect.rs`: `cancel()`. `camera.rs` calls it when a dash starts, so Deflect ends at once. Its end sound plays, its
  cooldown starts, and the protection goes off.
- `arms.rs` and `crosshair.rs`: F6 switches the arms off and on (logged `ARMS F6: ...`). It's a test switch.
**Why:** Your test:
- "when using reflect, if you dash, the reflect should instantly be canceled" (Overwatch).
- "the lights of the map are flickering ... gets a little dark for a small second". Your log shows the arms' picture
  was copied only 2-13 times in each 5-20 s stretch, so the copies can't cause a fast, constant flicker. The cause
  isn't known: F6 shows whether the arms have anything to do with it.
**Tested how:** `cargo build --release` and `cargo test --release` pass (44 tests).
**Result:** Built. **Not tested in game.**
**Still broken / not tested:**
- The flicker.
- The frame rate in your log was 43-55 most of the time (60 in earlier sessions). Not known whether that's the area
  (open field, grass) or us; F6 helps tell.
**Next:** You compare with the arms on and off (F6), and with Genji mode off (F9).

## 2026-10-09 Your Deflect block picks; Dragonblade arms sit lower (cut end hidden)

**Changed:**
- Your block picks (`block_picks.json`: rec7 at 0.56 and 1.19 s, rec8 at 0.51, 0.66 and 0.81 s) are copied to
  `D:/GenjiBridgeAssets`. `make_sounds.py` re-run: `deflect_block` is now exactly those 5.
- `arms.rs` and `camera.rs`: in the Dragonblade stance the arms are drawn 6 cm lower (`BLADE_DROP`). They slide down and
  back up over a quarter of a second when the stance changes.
- `viewmodel.rs`: the camera (eye, field of view) is part of the picture being drawn. Two comparison tests, run only
  on request:
  - `cameras`: eye positions and fields of view for the Dragonblade and resting poses;
  - `fov_versus_overwatch`: our throw at four fields of view next to your Overwatch frame.
**Why:** Your test: the block picks, "now it's good", and the screenshot: "his left arm is sticking out a little of the
screen". On your 3440x1440 screen the left arm's cut end (the model stops there) showed at the bottom in the
Dragonblade pose.
**Tested how:** Pictures at your screen's shape:
- **Field of view:** our throw at 70.5 degrees vertical (your 103) has the same hand size and nearly the same place as
  your Overwatch frame from the kill clip. So the field of view stays; drawing the arms bigger (60 degrees hides the
  cut end too) would no longer match.
- **Eye at the model's camera socket** (0, 0, 1.616): the cut end still shows.
- **Arms lower:** at 2 cm the cut end shows, at 4 cm a sliver, at 6 cm nothing. Lowering every stance would push the
  resting arm away from where Overwatch has it, so only Dragonblade is lowered.
- `cargo test --release` passes (44 tests).
- Your log of the leak-fix test: no memory errors, copies 3-4 ms, only when the pose changed.
**Result:** Built. **Not tested in game.**
**Still broken / not tested:**
- During the quarter-second slide the picture changes every frame. That's the first real test of the copy fix with
  frame-by-frame updates.
- Other Dragonblade animations (10d) may show the cut end in other places.
**Next:** Your check, then 10d.

## 2026-10-09 Deflect block picker

**Changed:**
- **New `tools/make_block_picker.py`:** a local page, `D:/GenjiBridgeAssets/block_picker/index.html`, with only your two
  block recordings (rec7, rec8).
  - Each whole clip, with a picture of its loudness over time.
  - Every short sound found in it: each loudness peak at least 6 dB over the 0.15 s before it, cut from 0.06 s before
    the peak to 0.3 s after, and marked on the picture. That gives 7 candidates in rec7 and 6 in rec8.
  - Each has a tick box ("block") and a notes box, and the page exports `block_picks.json`.
- `tools/make_sounds.py`: once `D:/GenjiBridgeAssets/block_picks.json` exists, `deflect_block` uses exactly the ticked
  sounds.
**Why:** You: "the sound of deflect was really bad, it was restarting. can you do that html sound picker for me? but
only with the deflect block sound that I sent?" My guess: two of my five cuts (rec7 at 0.12 s and rec8 at 0 s)
start at the very beginning of the clips, and probably hold Deflect's own start sound.
**Tested how:** The script ran and the loudness pictures were checked. The page wasn't opened (it's like the other
pickers).
**Result:** Built.
**Still broken / not tested:** Until you export your choice, the five cuts from before stay in use.
**Next:** You pick, export, and I run `make_sounds.py`.

## 2026-10-09 Arms: fix the memory leak that hid them and crashed the game

**Changed:**
- **New `tools/patch_hudhook.py`:** makes a fixed copy of the overlay library (hudhook 0.9.3, MIT) in `third_party/`,
  which is never committed, from Rust's download cache.
  - `genji/Cargo.toml` uses that copy (`[patch.crates-io]`), and `tools/build.ps1` runs the script before building.
  - The fix: the texture update now makes one upload buffer per texture and reuses it, and moves the texture back to
    the copy state before each later update.
- `arms.rs`: a pose asked for again isn't drawn or copied again. The resting pose is copied once, not 60 times a
  second.
- `crosshair.rs`: the texture keeps its picture while first person is hidden, so the arms are back at once after a
  menu.
**Why:** Your test of 10c: "after the deflect ended, the arm disappeared", "when I tried to use the blade, it didn't
show up either", "when I opened the game's menu, the game crashed". Your log:
- Your game runs at 3440x1440, so the arms' picture is 1720x720 (5 MB).
- Copying it into the texture took 7-8 ms every frame, inside the game's frame (frame rate 43-51).
- At 140 s: "texture copy failed ... Not enough memory resources", and the arms were hidden for the rest of the
  session (it came right after a Deflect, by chance).
- The crash came at the menu after that, when the game needed memory.
- hudhook's `replace_texture` made a new 5 MB upload buffer on every call and never released it (its own comment says
  the leak is "necessary"). That's about 300 MB a second at 60 frames.
**Tested how:** `cargo build --release` (with the fixed copy) and `cargo test --release` pass (44 tests).
**Result:** Built. **Not tested in game.**
**Still broken / not tested:**
- Whether the copy is now fast (logged: `ARMS overlay:` lines).
- Whether memory stays flat when the picture changes every frame. That only happens in 10d, with animations.
- Drawing at 1720x720 took 9-10 ms on average on our thread (fine: the game doesn't wait for it).
**Next:** Your test; the Deflect block picker.

## 2026-10-09 Deflect block sound from your new recordings

**Changed:**
- `tools/cut_recordings.py`:
  - your two new clips are rec7 and rec8 in `recordings/clips.txt`;
  - five blocks are cut by hand (`MANUAL_CUTS`), 0.26-0.38 s each: four from rec7 (at 0.12, 0.52, 0.76 and 1.18 s),
    one from rec8 (at 0 s);
  - thumbnails are only made when the clip still exists (your older clips are no longer at their paths; their
    audio is kept).
- `tools/make_sounds.py`: `deflect_block` is back, with those five recordings (`RECORDED`). Re-run: 55 events, 368
  files.
- `genji/src/deflect.rs`: the block sound plays once in each frame that Deflect catches one or more shots.
**Why:** You: "here are 2 clips with proper deflect blocking sound. separate that for me, and add it."
**Tested how:**
- I chose the cuts from the clips' loudness (one value every 20 ms) and a video frame every 0.1 s. In rec7 the
  bots' shots hit the sword four times. Each hit is a short swish-clang 13 dB over Deflect's hum, and the sword
  swings in the video at the same moments.
- rec8: an explosion covers everything after 0.5 s, so only its first block is used.
- Levels: peaks from -5 to -11 dB, like your other recordings.
- `cargo test --release` passes (44 tests, every sound file decodes).
**Result:** Built. **Not tested in game, and I haven't listened:** the choice is from the numbers and the video.
**Still broken / not tested:**
- Deflect's hum is under every cut (about 13 dB below the block), so during Deflect the hum is a little louder
  for a moment at each block.
- Melee hits blocked by Deflect make no block sound yet; only caught projectiles do.
**Next:** Your test, together with 10c.

## 2026-10-09 Step 10c: Genji's arms in game (resting poses)

**Changed:**
- `genji/src/viewmodel.rs`: draws on several cores.
  - The mesh is posed once and its visible triangles are sorted into bands of 32 rows.
  - The bands are handed out in turn to the threads (the arms sit low on the screen, so equal halves would leave one
    core doing most of the work).
  - Empty pixels next to the arms get the arm's colour, still see-through, so the stretched picture has no dark
    outline.
  - New test: 4 threads give the same picture as 1.
- **New `genji/src/arms.rs`:** our own drawing thread, "genji-arms". It loads the export once, then draws whatever pose
  the game thread last asked for, on 4 cores. It logs drawing times every 5 s (`ARMS` lines).
- `crosshair.rs` (the overlay):
  - creates a texture half the screen's size;
  - each frame copies the newest picture into it (copy time logged every 5 s);
  - draws it over the whole screen, under everything else of ours, while first person shows;
  - a failure in the copy only hides the arms.
- `camera.rs`: asks for each stance's resting pose: normal, Dragonblade or Deflect.
**Why:** You: "yes, continue 10c". Plan in DESIGN.md 5k.
**Tested how:**
- `cargo build --release` and `cargo test --release` pass (44 tests).
- Drawing times outside the game, 1290x540, 4 cores: Dragonblade 10-12 ms (was 23 ms on one core), idle 4 ms,
  Deflect 5 ms, reload 5 ms.
**Result:** Built. **Not tested in game.**
**Still broken / not tested:**
- Whether the texture copy is fast enough inside the game, and whether the game's own frame rate drops: both logged.
- The arms don't move yet: no walking sway, no actions (10d), no sword.
- Edges are a little soft (half resolution).
**Next:** Your test. Then 10d (actions play their animations).

## 2026-10-09 Step 10b: model export and the first pictures of Genji's arms

**Changed:**
- **New `tools/export_viewmodel.py`** (runs in Blender). It writes to `D:/GenjiBridgeAssets/viewmodel`:
  - the first-person mesh: 17,026 vertices, 26,608 triangles, 88 bones, 4 bone weights each;
  - its colour texture (2048) and glow texture (1024);
  - the 49 animations of `tools/viewmodel_anims.py`, baked as one matrix per bone per frame (940 frames, 4 MB).
  - It checks its own output: the mesh posed the mod's way against Blender's posed mesh, on two frames of every
    animation.
- **New `genji/src/viewmodel.rs`:** loads that export, poses the mesh for any animation time (blending between
  frames, holding or looping) and draws it into an RGBA picture.
  - The drawing uses a depth buffer, perspective-correct texture coordinates, Genji's colour texture with his green
    glow, and one light plus ambient.
  - Two tests: preview pictures of 5 poses at 2580x1080 and 1290x540, with timings; and holding and looping.
- `hud.rs`: `decode_png` shared with it.
**Why:** Step 10b of DESIGN.md 5k: check the arms outside the game first.
**Tested how:**
- Export check: largest difference 0.000 mm on all 49 animations.
- The preview pictures were compared with Blender rendering the same poses from the same eye with the same field of
  view (idle 240E and Dragonblade idle 124B): same position and shape.
- `cargo test --release` passes.
- Drawing times, on one processor core:

| Pose | 2580x1080 | 1290x540 |
|---|---|---|
| Idle | 28 ms | 9 ms |
| Dragonblade (both arms close) | 83 ms | 23 ms |
| Deflect | 33 ms | 11 ms |
| Reload | 31 ms | 9 ms |

**Result:** The first picture came out mirrored left to right: looking along -Y with Z up, right is -X. Fixed, now it
matches Blender.
**Still broken / not tested:**
- Nothing in the game yet.
- Too slow to draw on the game's own thread at full size. 10c: draw at half size on a thread of its own (and split
  across cores if needed), then have the graphics card stretch it to the screen.
- Lighting is simple (no normal map yet), and the sword isn't exported yet.
**Next:** Your look at the pictures, then 10c (in game, idle only).

## 2026-10-09 Animation list agreed; Deflect-block sounds taken out of the quick melee

**Changed:**
- **New `tools/viewmodel_anims.py`:** the first-person animation for each role (idle, walk, jumps, climb, throw, fan,
  reload, dash, melee, the Deflect and Dragonblade sets), from your picker tags plus the changes we agreed on:
  - idle = each stance's frozen pose (240E, 124B, 342B);
  - 3631 = Deflect ending during Dragonblade;
  - 1256 = Swift Strike during Dragonblade (probably);
  - 33F3 and 363E = the end of a wall climb (probably).
- `tools/make_sounds.py`: the quick melee keeps only its own sound (0829, from its animation 2434). Re-run: 54 events,
  363 files.
  - Sound group 2D10, used until now for the melee's swing and hit, plays only under the four Deflect-block animations
    you named (24A2, 24A3, 344C, 344D), so it is most likely Deflect's block sound.
  - It's left out, as you asked, until your own recording of the block.
  - So the quick melee has no swing sound for now: the shout still plays, and its hit sound is 0829.
**Why:** You: "lets do your changes. also, I will record a blocking sound for deflect. for now, skip the deflect block
sound."
**Tested how:** `make_sounds.py` ran. `cargo test --release` passes (41 tests, all sound files decode).
**Result:** Done. **Not tested in game.** A quick melee that misses is now silent apart from the shout.
**Still broken / not tested:** None of the new roles in `viewmodel_anims.py` are checked in game yet.
**Next:** 10b: the export and a first picture of the arms.

## 2026-10-09 Step 10a: animation picker for Genji's first-person animations

**Changed:**
- `docs/DESIGN.md` 5k: the plan for step 10 (first-person arms).
  - Recommended: our own small software renderer, drawn through the overlay.
  - The alternatives: our own Direct3D 12 pass, or converting Genji into Elden Ring's own formats.
- Step 11 is marked done in the plan table.
- `tools/render_anim_previews.py`: `-- --all-1p` renders all 129 first-person animations from Genji's eye, with your
  Overwatch field of view at your screen's shape (430x180), into `D:\GenjiBridgeAssets\anim_previews_eye`.
- **New `tools/make_anim_picker.py`:** writes `D:\GenjiBridgeAssets\anim_picker\index.html`.
  - One card per animation: preview, length, folder, the sounds you tagged on it, and my guess.
  - Tags (idle, throw, fan, reload, Deflect, Dragonblade, melee, climb...) and notes.
  - Exports `anim_picks.json`.
- Also fixed two broken characters in DESIGN.md: a `\01` that had turned into a control character, and a path.
**Why:** You: "lets proceed". Step 10 needs to know which of the 129 animations is which; only the 20 that play sounds
had previews.
**Tested how:**
- Blender rendered all 129 without errors (4 minutes), and the picker lists all 129 with previews.
- I looked at the previews, as contact sheets:
  - From the eye they look like Overwatch: the right hand forward at rest, both hands on the hilt over the right
    shoulder for Dragonblade, both palms up for Deflect.
  - 15 guesses, where the picture and your sound tags agree.
- The page itself wasn't opened (it's like the sound picker).
**Result:** The framed previews (the first try) were too hard to tell apart; the eye view works.
**Still broken / not tested:**
- About 70 animations in Overwatch's folder `4/...` show nothing alone. They seem to be layers (walking sway, climbing,
  etc.) that move only part of the arms on top of a base pose, so they need a base pose under them to be seen.
- The renderer choice (5k, A) waits for your OK before 10c.
**Next:** You tag the animations. Then 10b: the export and a first picture of the arms at your screen size.

## 2026-10-09 Silence only the Tarnished, not NPCs and enemies

**Changed:** `genji/src/game_voice.rs` now renames only the player's rows of `PlayerVoiceType` (ids below 50).
**Why:** Your test: the silencing works ("its working now"). But the log shows the table holds 174 rows:
- 12 are the player's (`_00_PC_Male_Young_Neutral` to `_15_PC_Female_Aged_Deeper`);
- the rest are NPC and enemy voices (`_50_NPC_Male_Young`, `NPC_vc300` to `NPC_vc454`).
All 174 were renamed, which would also silence humanoid enemies' and NPCs' voices in Genji mode.
**Tested how:** Read from your log. `cargo build --release` and `cargo test --release` pass (41 tests).
**Result:** Built. **Not tested in game.**
**Still broken / not tested:** Whether NPC voices were actually silenced before wasn't checked; now they can't be.
**Next:** Silence Elden Ring's footsteps and armour sounds the same way.

## 2026-10-09 Silence the Tarnished's grunts; critical-health sound audible

**Changed:**
- **New `genji/src/game_voice.rs`:** while Genji input is captured, death included, every row of Elden Ring's table
  `WwiseValueToStrParam_Switch_PlayerVoiceType` gets a name no sound bank has.
  - That table gives the name of the sound switch that picks the player's voice.
  - The names are restored in memory when capture ends, and the originals are logged (`VOICE`).
- `cues.rs` and `sound.rs`: the critical-health sound now plays 0.35 s after the hit that crosses 30%, at full level
  (was at the moment of the hit, at 0.6).
**Why:** Your test:
- "when I take a hit, I hear the elden ring char grunt of pain, not genji".
- Below 30% health, nothing was heard.
- Your log shows Genji's pain line did play, 0.15 s after each hit (265.2 s, 279.7 s, 291.5 s): under Elden Ring's
  grunt.
- The critical-health sound also played (267.9 s and 291.3 s), but exactly on the hit, quieter, and much of its
  energy is below 200 Hz.
**Tested how:** `cargo build --release` and `cargo test --release` pass (41 tests). The silencing idea comes from the
table's name in the fromsoftware-rs crate.
**Result:** Built. **Not tested in game.**
**Still broken / not tested:**
- It may only take effect after a loading screen, or the game may fall back to a default voice for an unknown name.
- A pain line is still skipped if another line (like the dash shout) is playing at that moment.
**Next:** Your test. If the grunts stay, read the `VOICE` lines and try the next idea.

## 2026-10-09 Step 11b-2: movement, health and kill sounds

**Changed:**
- **New `genji/src/cues.rs`:** the rules for when each of these sounds plays (plan in DESIGN.md 5j, "11b-2"). It has no
  game access; `camera.rs` plays what it returns. Four tests.
  - **Movement:**
    - a footstep every 1.3 m run;
    - jump and double-jump grunt (at most every 1.5 s), plus the double-jump whoosh;
    - wall climb: one sound at the start, a short grab every 0.2 s while climbing, the ledge sound on reaching the top;
    - landing thud, louder the faster the fall, with a grunt after falls of 3 m or more.
  - **Health:**
    - a pain line on hits, at most every 3 s;
    - a stagger line instead when the game plays a hit reaction, plus the knockdown thud and line if it lasts 0.9 s;
    - critical-health sound below 30%;
    - health-pack sound and a healing line for a flask (15% or more at once, not lifesteal);
    - a death line, or a scream after falling for over 1 s;
    - a respawn line once you're back in control.
  - **Kills:** first a multikill line (3 kills within 4 s), else a melee-kill line, else a kill-streak line (every 10th
    kill without dying), else the elimination line on every third kill, as before.
- `movement.rs`: `MoveEvents` (jumped, double jump, climb start, vault, landing speed and height, distance run),
  `climbing()` and `airborne_for()`.
- `blade.rs`: `melee_recent()`.
- `sound.rs`: footsteps at 2.5 (your recording is quiet: it peaks 28 dB under your dash), critical health at 0.6.
- `tools/make_sounds.py`: your wall-climb tag split into `wall_climb` (16DC and 930F) and `wall_climb_loop` (16CF).
  Re-run: 55 events, 373 files.
**Why:** You: "add the next step of sounds".
**Tested how:**
- `cargo build --release` and `cargo test --release` pass (41 tests).
- The cues tests check footsteps per distance, the pain limit, stagger then knockdown, critical health, lifesteal
  not counted as a flask, the fall scream, the respawn line waiting for control, and the kill-line order.
**Result:** Built. **Not tested in game.**
**Still broken / not tested:**
- All rules and numbers are guesses at Overwatch's.
- Whether Elden Ring's hit reactions show up as "the game takes over" is assumed. If they don't, every hit gets the
  pain line and no stagger or knockdown.
- The 7.9 s healing loop (388B) isn't used.
- Elden Ring's own footsteps and grunts still play underneath.
**Next:** Your test.

## 2026-10-09 No reload during the quick melee

**Changed:**
- `genji/src/firing.rs`: `set_meleeing()`, called each frame from `camera.rs` with whether a quick melee (animation
  plus recovery, 0.8 s) is on.
  - R does nothing during it.
  - An empty magazine waits for the melee to end before its automatic reload.
  - Starting a melee cancels a reload in progress, and a reload asked for during a burst. The ammo stays where it was.
- The reload's four clicks now play on their own channels, so a cancelled reload cuts the clicks still to come
  (`sound::play_on_after`).
- New test `no_reload_during_a_melee`.
**Why:** You: "you shouldn't be able to reload in the melee animation or the recover."
**Tested how:** `cargo build --release` and `cargo test --release` pass (37 tests).
**Result:** Built. **Not tested in game.**
**Still broken / not tested:** The melee cancelling a reload is what I remember Overwatch doing, not verified.
**Next:** 11b-2 sounds.

## 2026-10-09 R reloads

**Changed:**
- `genji/src/firing.rs`: `reload()`, a manual reload.
  - Nothing happens with a full magazine or a reload already on.
  - Pressed during a burst, the reload starts once the burst's last shuriken is out.
- `camera.rs`: R calls it while the controller drives, except during Dragonblade.
- `dinput.rs`: R is hidden from the game in Genji mode, like E, Q, Left Shift and F, so it never uses an item even with
  the default keys.
- The reload sounds and the HUD's "RELOADING" follow the reload as before.
- New test `r_reloads_after_the_burst_and_not_when_full`.
**Why:** You: "I changed the use item from R to Z because I want to reload using R."
**Tested how:** `cargo build --release` and `cargo test --release` pass (36 tests).
**Result:** Built. **Not tested in game.**
**Still broken / not tested:** Whether Overwatch lets you reload in the middle of a burst isn't verified. Here the burst
finishes first.
**Next:** Your check, then 11b-2.

## 2026-10-09 Genji mode works again after the key change (UI state 0x0b)

**Changed:** `genji/src/controls.rs`: UI state 0x0b added to the states seen during play (`GAMEPLAY_UI_STATES`).
**Why:** You moved Elden Ring's "use item" from R to Z, and Genji mode stopped working. Your log (`logs/genji.log`):
- From the moment the Esc menu opened (99.99 s), state 0x0b was on in plain play too, next to the HUD (0x07):
  `[0x05 0x07 0x08 0x0b 0x1a]`.
- It hid together with the HUD when a menu tab opened (105.2 s and 138.6 s).
- The mod counts any state it doesn't know as a menu (the safe side), so it never took control again. F9 at 132.1 s
  and 135.7 s turned Genji mode on, but "input CAPTURED" never followed.
**Tested how:** Read from your log. `cargo build --release` and `cargo test --release` pass (35 tests).
**Result:** Built. **Not tested in game.**
**Still broken / not tested:**
- What 0x0b really is isn't known. It behaves like a part of the HUD, and it was seen only in this session.
- If it ever shows up in a real menu, the mod would stay in control there. The Esc menu (0x1c) and its tabs still
  release control.
- R does nothing in Genji mode yet: the reload is still automatic only. That's the next step.
**Next:** R to reload.

## 2026-10-09 Kill skull in red

**Changed:** `genji/src/hud.rs`: the kill skull is red (232, 48, 56), the same red as the enemy health bars. It was pink,
as in Overwatch.
**Why:** You: "yeah, I want it red."
**Tested how:** `cargo build --release` and `cargo test --release` pass (35 tests).
**Result:** Built. **Not tested in game.**
**Still broken / not tested:** The whole kill skull is still to be seen in game.
**Next:** Your check, then 11b-2.

## 2026-10-09 Kills: no double count on a dead body, quieter elimination sound, kill skull at the crosshair

**Changed:**
- `genji/src/kills.rs`: hitting a body that is already dead no longer marks it as "damaged by us".
  - This covers bodies counted as our kill and deaths judged "not ours".
  - A body that died this very frame is still accepted, so a one-shot kill by the game's own hit keeps its credit.
- `genji/src/sound.rs`: the elimination sound is at 0.5, about 6 dB quieter.
- `genji/src/hud.rs` and `crosshair.rs`: the kill skull. A pink skull pops up big over the crosshair, snaps down to
  about 44 px and fades out in about half a second. It is drawn under the crosshair, and each new kill restarts it.
- `camera.rs` passes the kill count to the HUD.
- `tools/make_hud_art.py`: `kill_skull.png`, the skull cut out of the boss badge's vector (00000000082C). Run just this
  part with `--kill-skull`.
**Why:** Your play test and clip (`MedalTVOverwatch20261009103728341-trim-...mp4`):
- "the elimination sound is kinda loud".
- A burst whose first shuriken kills played the sound again for the other two.
- "add this animation of the skull when you kill an enemy".
**Tested how:**
- The double count: the rest of the burst called `kills::damaged` on the dead body, so the next frame counted it as a new
  death "ours". Found by reading the code; not reproduced in game.
- The skull timing was measured frame by frame from your clip (60 fps), from the size of the pink pixels and their
  brightness against the background. The picture was checked by eye against the clip.
- `cargo build --release` and `cargo test --release` pass (35 tests).
**Result:** Built. **Not tested in game.**
**Still broken / not tested:**
- The skull is pink like the clip; the enemy bars are red at your request. Say if it should be red too.
- The skull's colour and fade come from a compressed video with bloom, so they're close, not exact.
**Next:** Your check, then 11b-2.

## 2026-10-09 Sound levels and the Dragonblade sheathe timing

**Changed:**
- `genji/src/sound.rs`: a per-event level table (`EVENT_GAIN`).
  - The shuriken throw is at 0.45, about 7 dB quieter.
  - "Ryujin no ken wo kurae!" is at 2.5, about 8 dB louder.
  - The overall limit was raised from 2.0 to 3.0 so the louder voice line isn't capped.
- `genji/src/blade.rs`:
  - The blade-ending and sheathe sounds are now queued together with the ending roar. They start the moment the
    roar goes quiet (2.82 s after it begins).
  - Before, they were sent when the timer reached 0, and the sheathe had a further 0.25 s delay.
  - The roar now starts 2.82 s before the end (was 2.9 s).
**Why:** Your play test: "the m1 shuriken is too loud compared to the others", "the ult voice line is quite low",
"the sheathing sound is a bit late. it has to come instantly after the ending roar".
**Tested how:** Measured the files with ffmpeg:
- Throw: mean about -15 dB, peak about -2 dB. Hits: mean about -26 dB.
- Ult line: peak -13 dB. The unsheathe roar played with it peaks at -1 dB.
- Ending roar: audible for 2.82 s, then silence to 2.91 s.
`cargo build --release` and `cargo test --release` pass (35 tests).
**Result:** Built. **Not tested in game.**
**Still broken / not tested:**
- The new levels are estimates from the measurements; your ears decide.
- The sheathe timing relies on the audio thread's own delay, so it can't drift from the roar.
- If the Dragonblade ever ended early, the end sounds would still play at the roar's end. Nothing ends it early now.
**Next:** Your levels check, then 11b-2 (movement and voice extras).

## 2026-10-09 Step 11b-1: the mod plays Genji's sounds (abilities)

**Changed:**
- **New `genji/src/sound.rs`:** an audio thread of our own that plays the files in `D:\GenjiBridgeAssets\sound\<event>\`
  through Windows' default output, using the `rodio` library (0.19, Ogg Vorbis and WAV only). Elden Ring's audio
  isn't touched.
  - Each event picks a random take, never the same twice in a row.
  - Named channels can be stopped (Deflect's hum).
  - Voice lines don't overlap: a new line is skipped while one plays, unless it's urgent.
  - Volume: Elden Ring's sound effects setting (read once a second) times 0.8.
  - A missing event is logged once.
- **Hooks:**
  - **Shuriken:** each throw; Fan of Blades once per fan.
  - **Hits:** body and headshot sounds on confirmed hits; wall impacts quieter with distance, none past 40 m.
  - **Reload:** its 4 sounds at 0, 0.35, 0.75 and 1.1 s (estimated).
  - **Swift Strike:** your recorded dash plus a "(Swift Strike)" shout; body-hit sound on dash hits.
  - **Deflect:** your recorded start with its hum on a channel, cut if Deflect ends early; then the end.
  - **Not ready:** Shift or E pressed on cooldown.
  - **Cooldown ready:** when Swift Strike's or Deflect's cooldown finishes.
  - **Ult:** the ready sound and "My ultimate is ready"; "My ultimate is charging" when Q is pressed early (at most
    every 4 s).
  - **Dragonblade:** "Ryujin no ken wo kurae!" (cuts in) with the big roar and the draw at Q. Each swing gets a blade
    swing plus a roar. Hits use the melee hit (no Dragonblade hit sound was found). The ending roar starts 2.9 s
    before the end, then the blade ending and the sheathing.
  - **Quick melee:** swing, a shout, and the hit sound on hits.
  - **Kills:** Overwatch's elimination sound, and an elimination line on every third kill.
**Why:** Step 11; your picks and recordings.
**Tested how:**
- The build passes, with 35 unit tests. A new test decodes all 373 sound files with the mod's own decoder, and none is
  empty or silent.
- Not run in game: whether the audio thread works inside Elden Ring, the levels, and the timings.
**Result:** Not tested in game.
**Still broken / not tested:**
- In game.
- Movement and other voice sounds (footsteps, wall climb, double jump, landing, pain, death, respawn, healing, low
  health) are step 11b-2.
- Elden Ring's own sounds for the same actions (the Throwing Dagger, the Tarnished's grunts) still play.
**Next:** Your test; then 11b-2.

## 2026-10-09 Third sound export: your recordings in, Deflect block left out

**Changed:** `tools/make_sounds.py`:
- **Your third export,** copied to `D:\GenjiBridgeAssets\sound_picks.json`:
  - dash: 3 recorded takes (rec3 at 2.92, 7.39, 10.31 s);
  - footsteps: 22;
  - Deflect start and end: 2 each;
  - cooldown ready: 3;
  - ult ready: 1.
- **Deflect block** is dropped on purpose (your call: too much background noise).
- **Recordings win:** where you recorded an event, the recordings replace the sounds from Overwatch's files for it
  (Deflect start and end).
- **Result:** 54 events, 373 files in `D:\GenjiBridgeAssets\sound`.
**Why:** You: "lets just not use the deflect actual block sound... stick just to the deflect start and end".
**Tested how:** The tool ran (counts above).
**Result:** The sound set is complete enough for playback.
**Still broken / not tested:**
- **Not found:** dash hit, Dragonblade hit (the body-hit and melee-hit sounds will stand in), jump (the jump voice
  covers it).
- Dash reset doesn't exist in Overwatch.
**Next:** Step 11b, playback in the mod.

## 2026-10-09 Your Overwatch recordings cut into single sounds, in the picker

**Changed:**
- New `tools/cut_recordings.py`. It reads your 6 Practice Range clips (`D:\GenjiBridgeAssets\recordings\clips.txt`,
  rec1-rec6 in recording order) and extracts their audio with CapCut's ffmpeg.
- **Finding the sounds:** each sound's start comes from the loudness (a rise of 10 dB within 30 ms, above -45 dB).
  Each sound is cut from just before its start to the next one (at most 2.5 s), with 5 ms fades, into
  `D:\GenjiBridgeAssets\recordings\cuts`, plus a frame of the video at that moment. That gives 86 cuts.
- **Guesses tagged in advance (36),** from what the video shows:
  - rec5 1.12 s: ult ready (the bottom centre lights up);
  - rec4 0.86/4.71 s: Deflect start (2 s with the sword up); rec4 3.03/6.88 s: Deflect end;
  - rec4 3.64/7.49 s and rec3 3.92 s: cooldown ready (the tile's ready flash);
  - rec3 2.92 s: Swift Strike (green trail);
  - rec2: footsteps (walking in the empty room);
  - rec1: Deflect blocks (bots shooting into Deflect).
- `tools/make_sound_picker.py`: a "Recorded" section with frame, play button, tag and note per cut, and a new
  "Interface" tag group (cooldown ready, kill, not ready, low health).
- `tools/make_sounds.py`: copies recorded sounds (.wav) under their tags too.
**Why:** You recorded the sounds DataTool can't reach (dash, Deflect, footsteps, ult ready, cooldown ready).
**Tested how:**
- The cuts were made, and the page has all 86 cuts with frames, its script passing a syntax check.
- `make_sounds.py` still gives the same 50 events with your current picks.
- I can't hear: the guesses come from the video and need your ears.
**Result:** Ready for you to check.
**Still broken / not tested:** The guesses. rec1 has steady map noise behind the sounds.
**Next:** You check the "Recorded" section and export; then step 11b.

## 2026-10-09 Second sound export; Swift Strike, Deflect block and Dragonblade hit aren't in the extraction

**Changed:**
- `tools/make_sounds.py`: your second export. Group `16CF` ("actually, this is the loop of wall climbing") is added to
  wall climb, giving 350 files in 50 events.
- No footstep tag was in the file. The walking-sound tag was probably made after the export.
**Why:** You: Swift Strike is a long sound (almost a second) and isn't in the candidates; the Dragonblade hit and the
Deflect block aren't anywhere either.
**Tested how (dead ends recorded):**
- **Untagged groups:** 110 groups weren't tagged; about 30 of them last 0.6-1.4 s. You had already listened through
  them.
- **Ability extraction:** DataTool's `extract-abilities` gave only the ability icons.
- **Hidden `extract-anything` mode on ability and character IDs:** DataTool's `list-abilities` gives the IDs (Swift
  Strike `0DA.09E`, Deflect `0D9.09E`, Dragonblade `0D8.09E`, Genji `029.075`). The mode found nothing for any of
  them: its finder has no handler for those ID types.
- **The same mode on all 37 of Genji's entities (`.003`):** 113 sounds, every one already in our extraction.
- Conclusion: these three sounds aren't reachable through DataTool's extractions. Overwatch most likely keeps them in
  sound banks that DataTool doesn't unpack this way.
**Result:** Still missing: Swift Strike, Deflect block, Dragonblade hit (and dash reset, which Overwatch doesn't
have).
**Still broken / not tested:** See above.
**Next:** Recording the three sounds from Overwatch itself (your choice).

## 2026-10-09 Your sound picks copied into place (tools/make_sounds.py)

**Changed:**
- **New `tools/make_sounds.py`:** reads your export (copied from your Downloads to `D:\GenjiBridgeAssets\sound_picks.json`)
  and the picker's own take numbering. It copies the chosen takes into `D:\GenjiBridgeAssets\sound\<event>\01.ogg`
  and so on, plus an `index.json` (outside the repository).
- **How your notes were read** (by hand, into the tool's table):
  - **Numbers only** ("1 3 4 5 7 8", "3-7", "from 1 to 13") are the right takes; the others belong to other skins.
  - **Descriptions** split a group: melee hit 1-3, 8, 9 and swing 4-7, 10; the Dragonblade group into swings 2 6 7,
    roars 1 3 5 8 9 and heartbeat 4; blade ending and sheathing; the ult roar (take 2).
  - **Voice lines:** your tags, without the 14 eliminations you marked "dont use", plus the folders you named
    (melee kill, kill streak, multikill, fall death, stagger, knockdown, jump end).
  - **Extras you found:** kill sound, "not ready" sound, low health, healing, health pack, body/head/critical hit
    sounds.
- **Result:** 50 events, 340 files.
**Why:** You exported your picks.
**Tested how:** The tool ran; every chosen take was found and copied (counts above in the run, e.g. shuriken throw 6,
fan 5, eliminations 76).
**Result:** Done.
**Still broken / not tested:**
- **Not found yet:** Swift Strike dash, dash hit, dash reset, Deflect block, Dragonblade hit, jump, ult ready, footstep.
- The reload's 4 sounds play one after another; their exact timing within the 1.5 s isn't in the files, so it will
  be estimated.
**Next:** You look for the missing ones (candidates sent); step 11b, playback in the mod.

## 2026-10-09 Animation previews in the sound picker (Blender)

**Changed:**
- **Your idea:** "finding the animation we already find the sound". Overwatch files ability sounds under the
  animation that plays them, so seeing the animation names its sounds.
- **Blender and the importer:**
  - With your OK, downloaded the DataTool team's Blender importer `io_scene_owm-3.3.1.zip` (280 KB, from
    github.com/overtools/io_scene_owm releases) and unpacked it into `D:\GenjiBridgeAssets\tools`.
  - It is loaded by our script only, never installed into Blender. It works with Steam's Blender 5.2.2 even though it
    targets 4.2+.
- **New `tools/render_anim_previews.py`** (runs inside Blender, in the background):
  - It imports Genji's first-person model (`00000000DBA6`) and third-person model (`00000000DB92`), and plays every
    animation that has sounds: 20 and 51.
  - It renders every 2nd frame at 320x180, textured, into `D:\GenjiBridgeAssets\anim_previews\<1P|3P>_<id>\`.
  - Each move is framed to fit: first person from just behind and above the eye, third person from the front.
  - Animations already rendered are skipped.
- **`tools/make_sound_picker.py`:** makes a looping GIF of each animation's frames and shows it above its sounds.
**Why:** Seeing the move tells which sounds are which (and step 10 needs this setup anyway).
**Tested how:**
- Contact sheets of all 71 animations, checked by me. First-person arms and third-person moves read clearly, for
  example a fall and death in `847`.
- The picker's 71 GIFs exist, and its script passes a syntax check. Not opened in a browser by me.
**What went wrong on the way (and was fixed):**
- The first cameras faced the wrong way.
- Genji's exact eye cuts the arms off at the screen's edge.
- Blender collections don't take slice steps.
**Still broken / not tested:**
- First-person `957` and `3DC8` move their arms far, so they come out small.
- The sword isn't in the first-person model, so sword-only motions show little.
**Next:** You tag with the previews; then step 11b.

## 2026-10-09 Sound picker grouped by animation, with each animation's length

**Changed:** `tools/make_sound_picker.py`:
- The ability sounds are now grouped under the animation (Genji's move) that first plays them, first person first.
  Each group shows the animation's length.
- **How the lengths are read:** the `AnimationEffects` folders are named after their animations, and all 20
  first-person ones match an `.owanimclip` file. The length is the last keyframe's frame over the frames per second,
  read from DataTool's file format (`owm-formats/owanimclip.txt` in its source).
- **A misread along the way:** the 127 at the start of the header is the bone count, not the frame count.
- Example: first-person animation `2436` lasts 1.5 s and plays 4 different sounds, which fits Genji's reload.
**Why:** You: "if they are grouped by the animations, isnt easier to check the animations then?"
**Tested how:** 74 animations got lengths (0.1-3.9 s, believable). The page script passes a syntax check. Not opened
in a browser by me.
**Result:** Built.
**Still broken / not tested:** Seeing the animations themselves needs a 3D renderer (see the answer to you).
**Next:** Your call on rendering the animations.

## 2026-10-09 Sound picker: voice lines with their words and tagged in advance; take lengths shown

**Changed:**
- **Research** (your request: an easier way to identify the sounds):
  - **Community lists:** none maps Genji's ability sounds. The old Overwatch Item Tracker sound lists (2017-2018) match
    none of our 2,144 sound IDs, and an OWLib issue confirms ability sounds are only grouped by the animations that
    reference them.
  - **DataTool's subtitles:** they cover voice. `list-subtitles-real` showed Genji's voice set `0x54` has 1,262
    subtitled lines. `extract-hero-voice ... Genji --subtitles-with-sounds` (offline, read-only on the old Overwatch
    build) put all 1,366 voice files, named after their words, into `D:\GenjiBridgeAssets\voice_subs`.
- `tools/make_sound_picker.py`:
  - **Voice lines** now come from those named files: 276 lines in 19 situation folders. From `Unknown` it keeps only
    effort sounds: "(Swift Strike)", "(exertion)", "(jumping sound)" and similar.
  - **Tagged in advance** (178, applied once; your own changes are kept):
    - ult line "(Japanese) Take the blade of the dragon god!" (2 takes);
    - "My ultimate is ready" / "My blade is ready to be unleashed";
    - "charging" / "almost ready";
    - pain (19), death (6), eliminations (90), healing (3), respawn (27), jump grunts (3), melee shouts (9), dash
      shouts (15).
  - **New tags:** voice dash, voice melee.
  - **Ability sound takes** now show their length, read from each file's header.
**Why:** You: "check on the internet if there's a way to identify these audios easier".
**Tested how:** The page data was checked: all voice files exist, the advance tags are right for the Ultimate folder,
and the take lengths read; the page script passes a syntax check. Not opened in a browser by me.
**Result:** Voice is identified without listening. Ability sounds still need your ears; the lengths help.
**Still broken / not tested:** The page in your browser. Voice lines you tagged before this change were under the
old file paths, so those tags don't carry over (the advance tags cover them).
**Next:** You finish the ability sounds and export; then step 11b.

## 2026-10-09 Step 11a: sound picker, to find Genji's sounds by ear

**Changed:**
- `docs/DESIGN.md` 5j: the sound plan (picker, playback, events).
- New `tools/make_sound_picker.py`. It writes `D:\GenjiBridgeAssets\sound_picker\index.html`, a local page outside the
  repository:
  - **148 ability and movement sound events** (778 takes), first-person first. Overwatch files these sounds unnamed
    under the animations and effects that play them; each `.02C` folder is one event with its takes, and duplicates
    across animations are removed.
  - **256 voice lines** from 22 of DataTool's situation folders (Ultimate, Pain, Death, Elimination and others).
  - Every take has a play button and a tag menu (the events of DESIGN.md 5j). Tags are remembered in the browser.
  - **Export** saves `sound_picks.json`, and also shows the text to paste.
**Why:** You: "lets start the audios". Nothing in the files says which sound is which, so they have to be found by
ear.
**Tested how:**
- The page's data was checked from the shell: all 1,034 file paths exist, and they're Ogg Vorbis (browsers play it).
- The page itself is not tested: the built-in browser can't run local pages.
**Result:** Built; not used yet.
**Still broken / not tested:** The page in your browser.
**Next:** You tag the sounds. Then step 11b: `tools/make_sounds.py` copies them, and the mod plays them.

## 2026-10-09 Quick melee also cancels a shuriken burst

**Changed:** `blade.rs` (`Blade::meleeing()`), `camera.rs`: a quick melee (mouse 5) cuts a burst short like a
dash does, so you throw 1 or 2 and melee. No shuriken while the melee is under way (its 0.8 s recovery).
**Why:** You: "melee also do the same. you can cancel the burst with melee".
**Tested how:** The build and the 34 unit tests pass (the burst-cancel test covers any block, dash or melee). Not
tested in game.
**Result:** Not tested in game.
**Still broken / not tested:** In game. The 0.8 s is our melee's recovery; Overwatch's own melee lockout isn't
measured.
**Next:** Your test.

## 2026-10-09 Shuriken: a dash cancels the burst, none during the dash, always 3 per burst or fan

**Changed:**
- `camera.rs`, `movement.rs`: no shuriken while Swift Strike's dash is under way (`Mover::dashing()`). A burst under
  way when the dash starts is cut short, so the dash wins whatever the order. The shuriken not thrown aren't spent,
  so the ammo can end on numbers that aren't multiples of 3.
- `firing.rs`: a burst or fan always throws 3 once it starts, even with only 1 or 2 left. It spends what's left, then
  reloads.
- Log: `[FIRE] burst cut short (n not thrown: dash, ...), ammo a`.
**Why:** You, about Overwatch: "if you m1, and throw only one shuriken, you dash, canceling the rest", "you also cant
shoot while in dash", "you can end up with 1 or 2 remaining ammo, but you still shoots 3 shurikens".
**Tested how:** 34 unit tests pass, two of them new: the last 1 or 2 shuriken still throw 3 (burst and fan); a dash
cancels the rest of a burst and keeps its ammo. Not tested in game.
**Result:** Not tested in game.
**Still broken / not tested:** In game. The recovery after a cancelled burst still runs from the burst's start
(0.88 s), as before; whether Overwatch shortens it after a dash isn't known.
**Next:** Your test.

## 2026-10-09 Boss names under the boss bar, bar raised

**Changed:**
- **Name** (`enemy_bars.rs`, `hud.rs`): the boss's name is drawn under its bar, in the bar's red, in capitals.
  - **Where it comes from** (your idea: "just grab the name when it enters the boss fight"): the game prepares each
    boss bar's name in its own display data (`boss_list_tag_data[..].chr_name`), and we copy it from there.
  - **Fallback:** if the game leaves it empty while its HUD is off, Elden Ring's HUD is switched on for 0.4 s, once per
    boss (`controls.rs`), so the game fills it in. You may see Elden Ring's HUD flash once at the start of a fight.
- **Shortened** (`short_name`, unit-tested): only the part before a comma, " of " or " the ", without leading titles
  (Starscourge, Lichdragon, Dragonlord, Promised Consort). For example "Margit, the Fell Omen" -> MARGIT,
  "Godrick the Grafted" -> GODRICK, "Rennala, Queen of the Full Moon" -> RENNALA, "Starscourge Radahn" -> RADAHN.
  Names without a title stay whole ("Tree Sentinel" -> TREE SENTINEL).
- **Bar raised:** the boss bar sits 22 px higher.
- **Log:** `[BARS] boss … is "full name": shown as "SHORT"`. If the game has no name, the log says so and when the
  HUD peek runs.
**Why:** You: "raise the bar a little, and put the name of the boss under it... only their names, no grand titles".
**Tested how:** The build and the 32 unit tests pass (a new one covers the name shortening). Not tested in game.
**Result:** Not tested in game.
**Still broken / not tested:**
- In game: whether the game fills the name with its HUD off, or only during the peek.
- Non-English game text: the comma rule works, the " of " / " the " rules are English only.
**Next:** Your test.

## 2026-10-08 Boss bars above the boss with a skull badge; boss fights no longer leave Genji mode

**Changed:**
- **Boss fights handed the camera back** (`controls.rs`).
  - The cause, from your log: UI state 0x03 appeared for 8 s in the middle of the Margit fight (228.9-240.4 s) while
    you moved, fought and took hits. It cleared when you died.
  - The rule counts every unknown state as a menu, so Genji mode let go. That's what looked like a cutscene taking
    over.
  - 0x03 now counts as gameplay. What the game uses it for is unknown; it showed only during that fight.
  - Real cutscenes and menus still release Genji mode, which comes back by itself afterwards.
- **Boss bar** (`enemy_bars.rs`, `hud.rs`):
  - The top-of-screen bar is gone.
  - Bosses named in the game's boss list get a bar above their head instead: 20 thin segments, shown the whole fight
    within 80 m, before any other bar.
  - The game fills its boss list with its HUD off; your log showed 1 entry during Margit.
- **Skull badge:** Overwatch's skull-in-a-diamond (vector `00082C`, your pick), tinted red, over a small light yellow
  diamond that shows through the skull. `tools/make_hud_art.py` builds it into
  `D:\GenjiBridgeAssets\hud\skull.png`.
**Why:** You: "cutscenes in the boss fight takes me out of genji mode", "remove the top bar... only the one above the
boss head", "add a skull logo".
**Tested how:**
- The build and the 31 unit tests pass.
- A mock-up of the boss bar (`D:\GenjiBridgeAssets\hud_ref\mock15_boss.png`). The badge was moved 4 px left so it
  doesn't cover the first segment.
**Result:** Not tested in game.
**Still broken / not tested:**
- In game.
- Bosses with an intro cutscene: the cutscene releases Genji mode as a menu would, which is expected. Not tested
  whether it comes back on its own afterwards (it should, like after menus).
- Bosses or named enemies without a game boss bar don't get the skull bar. The game doesn't list them.
**Next:** Your test, then step 11 (sound).

## 2026-10-08 Dragonblade lifesteal, no dash/Deflect while unsheathing, Overwatch-style enemy health bars

**Changed:**
- **Lifesteal** (`blade.rs`, `camera.rs`):
  - Every Dragonblade swing heals you 30% of the damage it deals. That's the game's confirmed damage, or ours when we
    apply it directly.
  - The healing is collected per swing and applied once a frame, never above max health and not while dead.
  - Log: `[BLADE] lifesteal +n: hp a -> b / max`. The health pill turns yellow too.
- **Unsheathing lock** (`camera.rs`, `blade.rs unsheathing()`): Swift Strike and Deflect can't start during
  Dragonblade's 1 s unsheathe. Log: `[SWIFT]` or `[DEFLECT] not while Dragonblade unsheathes`.
- **Enemy health bars** (new `enemy_bars.rs`, `hud.rs`; plan in DESIGN.md 5i). Your choice: keep Elden Ring's HUD off
  and draw our own, Overwatch style, red.
  - **Which enemies:** every nearby character's health is tracked. A bar shows above enemies within 50 m that lost
    health in the last 30 s and aren't full, on screen, nearest 8.
  - **Where:** projected with the camera we draw from.
  - **Look:** 10 segments (104x10 px at 1080p), lit red for the health left, dark red for what's lost; health just
    lost stays light for 0.5 s, then drains.
  - **Boss:** a 20-segment bar at the top centre, if the game's boss list names a character near you.
  - **Log:** every 2 s, `[BARS]` shows our bar count and what the game itself keeps in its enemy-tag and boss tables
    with its HUD off.
**Why:** You: "30% lifesteal in each slash", "lock deflect and dash during the unsheathe animation", "the HP of the
enemies disappeared", then "a overwatch bar would be awesome! instead of pink, you can use red".
**Tested how:** The build and the 31 unit tests pass. Not tested in game.
**Result:** Not tested in game.
**Still broken / not tested:**
- All of the above, in game.
- No enemy names under the bars. Our library has no lookup for the game's text yet.
- No damage numbers (Overwatch shows none; Elden Ring did).
- Bars can show through walls. The game's line-of-sight check isn't available to us; 50 m limits it.
- The boss bar depends on the game filling its boss list while its HUD is off. The `[BARS]` lines will show.
**Next:** Your test, then step 11 (sound).

## 2026-10-08 HUD: health "+" pill, yellow while healing, pink arrow on damage

**Changed (`genji/src/hud.rs`):**
- **The "+" box is now Overwatch's pill,** 40x24 px with rounded corners. Normally its left 56% is white with a dark
  "+", and the rest dark see-through grey.
- **Healing:** whenever health goes up (your R flask, or any other healing), the pill turns yellow:
  - left part yellow with a dark olive "+", right part dark yellow, yellow outline;
  - it stays yellow 0.8 s after the last rise.
- **Damage:** whenever health goes down, the pill's white part goes dark, the "+" turns light grey, and a pink arrow
  pointing down shows in the right part for 1.2 s. Healing takes priority.
- **Transitions:** each state fades in and out over 0.15-0.2 s.
- **What doesn't count:** health changing because the maximum changed (levelling, gear), coming back from a hidden
  HUD, or rising from 0.
- **Log:** `[HUD] healing indicator on (a -> b of max)` each time healing starts.
- **Docs:** README's "what works" list updated with the HUD parts you checked in game today. The outdated "DEFLECT
  under the crosshair" line is fixed.
**Why:** You, with Overwatch screenshots of the pill: normal, damaged, healing.
**Tested how:**
- A mock-up of the three states next to your screenshots (`D:\GenjiBridgeAssets\hud_ref\mock13_pill.png`). It showed
  the damaged pill's two halves at different darkness; fixed by drawing them side by side, not one over the other.
- The build and the 31 unit tests pass.
**Result:** Not tested in game.
**Still broken / not tested:** In game: drink a flask, take a hit.
**Next:** Your test; then step 12 is done.

## 2026-10-08 HUD: the ultimate becoming ready (99% to 100%), and the charging meter redone to match

**Changed (`genji/src/hud.rs`)**, following your "ULT GAINED" spec:
- **Charging meter (spec section 2)**, needed so the hard cut starts from the right picture:
  - 67 teal bars from 0.95 to 1.20 U with bright cores and a glow, filling clockwise from 12 o'clock and breathing
    ±3.5% every 1.55 s;
  - a thin teal circle at 0.825 U with 4 breaks;
  - a dark centre (#1D1B23, 85%);
  - a thin grey ring at 1.40 U and a dotted ring of 90 dots at 1.875 U;
  - the italic number in light grey;
  - the Q cap dark and see-through, with a thin outline at half strength.
- **Becoming ready** (`ult_gain`, 1.2 s, played when the charge reaches 100%), every phase from the spec's tables in
  60 fps frames:
  - **A, collapse:** a dark backing; the white disc shrinking from 0.85 to 0.50 U while the grey glyph fades in; the
    thin ring growing to 1.65 U and back; the triangles outside; the thick ring forming from swirling smoke,
    dark blue to bright.
  - **B, pop:** the disc back to 1.0 U in 3 frames, no overshoot.
  - **C, charge-up:** the ring to cyan, a blue glow growing, the triangles snapping in, the thin ring's gaps opening
    at frame 15, the hairline fading in.
  - **D, flash and shockwave:** a one-frame cyan flash over 3.2 U; the measured shockwave band expanding to 8.5 U
    and breaking into arcs; the trailing teal hairline; a cyan bloom.
  - **E, eruption** (a new drawing every 2 frames): a lopsided solid mass with a spike and a streak; it swells and
    hooked curls separate; then only the curls, flying out, thinning and breaking into dashes; then fragments.
  - **F, afterglow:** the bloom cools and fades; the glyph cools from cyan to navy; the disc stays pale cyan; the
    ready-state halo and aura come up; the first idle flares from frame 61.
  - **G:** a snap to the ready state at 1.2 s.
  - **Q cap:** a ghost at first, then solid white with a dark Q, which turns cyan from the flash on.
- **Code:**
  - The badge is now one shared drawing, `Look`, used by both the ready state and this animation.
  - `ult_aura` takes a strength; `flare` can break into dashes and, with no body, draws one tapered stroke.
  - The old 0.6 s burst at 100% is gone.
  - A unit test covers the keyframe helper.
**Why:** You: "do the animation when the ult hits 100%", with the spec and the frame sheets.
**Tested how:**
- A mock-up of every frame, drawn the way ImGui draws from the same formulas (`D:\GenjiBridgeAssets\hud_ref\
  mock12_A.png`, `mock12_B.png`), compared with your frame sheets.
- Tuned once: the eruption mass made lopsided and glowing, and the curls drawn as single tapered strokes.
- The build and the 31 unit tests pass.
**Result:** The mock-up's phases line up with the spec and your sheets. Not tested in game.
**Still broken / not tested:**
- In game. F7 fills the charge at once, which shows this animation.
- The new charging meter, in game.
- Additive light is approximated with normal blending.
**Next:** Your test.

## 2026-10-08 HUD: ready ultimate rebuilt from the "ULT FULL" spec (flipbook flare bursts)

**Changed:**
- The previous version is saved as the git tag `hud-ult-flames-v1` (commit 90ddd1d), as you asked.
- `genji/src/hud.rs`, the ready ultimate, following your second agent's spec:
  - **Static:** the rings, sizes and colours from the spec's table, in units of the disc radius:
    - a faint hairline ring at 1.9 U;
    - the thin pale ring at 1.4 U, with gaps at 3 and 9 o'clock where the triangles sit;
    - blue glow between the rings, then the thick cyan ring (#32FFFC core);
    - a navy outline, the disc (#E9EAF4) and the glyph (#1D1D30);
    - triangles from 1.40 to 1.52 U.
    The old breathing, the wandering arcs and the earlier flame tongues are gone (the spec: nothing rotates, pulses or
    sprays).
  - **Halo:** the spec's colours, flickering about 4%, irregularly.
  - **Boiling aura:** standing waves of soft blue flame from 1.25 to about 2.2 U that rise and fall in place, drifting
    2° a second clockwise.
  - **Flare bursts** (`Burst`, `flare`, `tendril`):
    - Timing: one every 1.37 s slot, starting 0-0.12 s into it, so 1.25-1.49 s apart.
    - Poses, in 60 fps frames: BUILD 4, PEAK 5, blank 5, SECOND 5, TENDRIL 6. Each pose is held, then hard-cut.
    - BUILD: a small, thin pair, a hooked loop and a spike opposite it.
    - PEAK: the same pair about 40% longer, thicker and brighter, filled. The disc's half on the spike's side goes pale
      cyan and the glyph teal (#255A72), drawn clipped to that half.
    - SECOND: one smaller, dimmer loop at a new angle, sometimes with a small one opposite.
    - TENDRIL: a jagged thread from 1.5 U to 3.0-3.8 U that fades and breaks into pieces.
    - Each burst takes a new direction and a mirror, at least 60° from the last.
  - **Shapes:** each flare is a centreline through control points (polar, in U) with widths on each side. The spike
    has two barbs. Each is drawn as a filled body plus glowing outlines, from quads sharing their corners.
  - A unit test checks the burst timing and the pose order against the spec.
**Why:** You: "save this one... and lets try with this one", with the spec and its frame sheets.
**Tested how:**
- A mock-up drawn the way ImGui draws, from the same formulas, compared pose by pose with the clip's cycle f236-f261:
  `D:\GenjiBridgeAssets\hud_ref\mock11b_ult.png`. It was tuned twice: wider, bigger-curled flames; a denser aura.
- The build and the 30 unit tests pass.
**Result:** The mock-up's poses look like the clip's. Its glow is weaker than in game, where the glow texture is
brighter. Not tested in game.
**Still broken / not tested:**
- In game.
- The spec says Overwatch adds light (additive blending). ImGui blends normally, so very bright overlaps are slightly
  less bright here.
**Next:** Your test. If it's worse, roll back to the tag `hud-ult-flames-v1`.

## 2026-10-08 HUD: ultimate flames redone as smooth blue fire and flame-tongue flashes

**Changed (`genji/src/hud.rs`):**
- **What was wrong:** the ready ultimate's "electric ribbons" were separate thick line pieces. They overlapped and
  showed as rectangles, the "yanky, not glued" look you saw. The blob particles are also gone.
- **Blue fire band:** a band from the ring out to about 90 px. Its outer edge flickers in tongues that drift around,
  and it fades outward. There are two layers: deep blue, and a brighter cyan one closer in.
- **Flame tongues** (clip part 1):
  - Quick flashes shaped like a flame tongue: two glowing cyan edges meeting at both ends, a faint fill between
    them, and a ragged outer edge.
  - Each one grows along the ring, curls outward, then fades.
  - Up to 3 at a time, at a new place each time.
- **How they're drawn:** both are continuous strips of textured quads that share their corners, so there are no
  seams. New generated textures: `ribbon` (soft across) and `fade` (fading outward).
**Why:** You: the flames "look so bad... yanky and not even glued. I can see the rectangles".
**Tested how:**
- A mock-up drawn the way ImGui draws (textured triangles, alpha blending), from the same formulas, compared with
  60 fps frames of your clip: `D:\GenjiBridgeAssets\hud_ref\mock10_ult.png`. It was tuned twice: a wider, brighter
  band; bigger, ragged, more saturated tongues.
- The build and the 29 unit tests pass.
- numpy for the mock-up was installed to `D:\GenjiBridgeAssets\pylib`, outside the repo.
**Result:** In the mock-up, smooth and close to the clip's look. Not tested in game.
**Still broken / not tested:** In game.
**Next:** Your test.

## 2026-10-08 HUD: new portrait with the helmet standing out of the hexagon; bright pieces back on the ammo lines

**Changed:**
- **Portrait** (`tools/make_hud_art.py`):
  - It now uses the bust picture you sent, copied to `D:\GenjiBridgeAssets\hud\portrait_source.png`. It isn't in your
    extracted files: I compared it with all 397 of Genji's and the UI's tall pictures and found no match.
  - Layout measured on your Overwatch screenshot:
    - the helmet's top is level with the hexagon's top corner, the visor 39% of the way down, the face's front at 88%
      of the width;
    - the helmet is drawn over the top edges of the outline;
    - the body is inside the outline;
    - a soft blue glow rises behind the head.
  - The previous picture is kept as `portrait_hex_previous.png`.
- **Ammo panel lines** (`genji/src/hud.rs`): the brighter, thicker piece is back. It sits on the same stretch of both
  lines (52% to 92% of the width, measured on your Overwatch screenshot: the same on top and bottom), bulging outward.
**Why:** You: "use this image instead... his head needs to stand out of the hex cyan cover"; "add back the bright
pieces, but this time, make them symmetrical".
**Tested how:** Rebuilt the portrait picture and compared it side by side with your Overwatch screenshot
(`D:\GenjiBridgeAssets\hud_ref\portrait_cmp.png`): same framing. The DLL builds. Not seen in game.
**Result:** Not tested in game.
**Still broken / not tested:**
- In game.
- Your picture is 180x310, so the portrait is slightly soft (it's shown about 140 px tall at 1440p).
**Next:** Your test.

## 2026-10-08 HUD: Dragonblade countdown ring, ammo panel lined up with the tiles

**Changed (`genji/src/hud.rs`):**
- **Dragonblade time left:** your Overwatch screenshots (6s, 5s, 1s) replace my cyan bar.
  - A white ring around the crosshair, radius about 30 px and 3.5 px thick, measured on them.
  - The white is the time left, running clockwise from 12 o'clock; the time used turns grey.
  - "6s" style seconds sit 42 px below the centre.
- **Ammo panel:** measured on your Overwatch screenshot, after taking out its 4° tilt (the whole bottom-right group is
  slightly rotated in Overwatch).
  - Its bottom line sits exactly on the LSHIFT/E underlines, and its top line 8 px below the tile tops.
  - It starts 16 px right of the E tile.
  - Both lines are straight, 3 px thick with a glow, with short angled ends. The thicker "tabs" are gone.
**Why:** You: the red lines (tile underlines and the panel's bottom line) and the yellow lines should be straight,
as in Overwatch; and the ult countdown is a white ring with a grey trail.
**Tested how:** The build passes. Positions come from pixel measurements of your screenshots. Not seen in game.
**Result:** Not tested in game.
**Still broken / not tested:**
- In game.
- The ring's size assumes your screenshots were at 1080 lines, like your clip. If they came from your 1440p screen,
  it is a third too big.
- The two round icons above Overwatch's ammo panel (perks) aren't drawn: Genji mode has no perks.
**Next:** Your test.

## 2026-10-08 HUD: upright tiles, the "pushed down" press, Dragonblade timer under the crosshair

**Changed (`genji/src/hud.rs`):**
- **LSHIFT/E tiles:**
  - Upright, symmetric and the same size, with rounded corners, as in your Overwatch screenshot. My lean came from
    misreading the clip: only the pressed tile leans.
  - 14 px apart. The key badges are 25 px tall.
- **Pressing LSHIFT or E:** besides turning cyan, the tile is pushed down. Its top drops about 10 of 64 px within
  0.12 s while its bottom stays, measured on clip part 3, frames 26-34. It stays pushed slightly less through the
  cooldown and pops back up when the ready flash starts.
- **Dragonblade time left:** a cyan bar 46 px under the crosshair, shrinking toward its middle. `State.blade_duration`
  and `blade.rs duration()` are back.
- **Ammo panel:** see-through instead of dark. Each cyan line has a brighter, thicker stretch, as in your
  screenshot.
**Why:** You, with your Overwatch screenshot next to ours: tiles off angle, press animation missing, Dragonblade
timer wanted.
**Tested how:** The build passes. A mock-up of the press sequence from the same formulas was compared with the clip's
frames (`D:\GenjiBridgeAssets\hud_ref\mock7_press.png`). Not seen in game.
**Result:** The mock-up's press lines up with the clip. Not tested in game.
**Still broken / not tested:**
- In game.
- The Dragonblade timer's look is my guess.
- F7 (fill ult) is still in.
**Next:** Your test.

## 2026-10-08 HUD: ultimate and ability tiles rebuilt from your analysed Overwatch clip

**Changed:**
- `genji/src/hud.rs`, rebuilt from your clip and its spec (`HUD_ANIMATION_SPEC.txt`; summary in DESIGN.md 5h). Sizes,
  colours and timings were measured on its frames, which are at 1080 lines.
- **Ultimate:**
  - Centre 150 px above the bottom; white disc 39.5, thick cyan band 43-50, pale ring 56, side markers, Q badge
    22 px.
  - Ready: blue halo, fine arcs, electric ribbons.
  - Pressed: ring expands and fades, emblem fades, the empty meter's big "0" fades in, Q goes from filled to outlined
    (0.4 s).
  - During Dragonblade: the meter, not the old draining ring.
- **LSHIFT/E tiles:**
  - 64 px, leaning. Cyan while used (Swift Strike 0.4 s, Deflect while up), with the badge outlined.
  - Cooldown: grey tile with cyan rising, and a big italic number with a dark edge. The tile fades from cyan to grey
    as the number fades in.
  - Ready flash: light bars down to the screen's bottom, white star, glow column, square particles, tile glowing
    back to white, badge filling in (0.6 s).
  - The "pressed" dip from the previous entry is gone: the clip shows no dip.
- **Badges:** "Q", "E" and "LSHIFT" are filled when ready and outlined otherwise.
- **Ammo panel:** moved to fit the new tiles.
- **Code:** generated textures (white, the tile's top-to-bottom gradient, a soft light bar); fonts 50 px (ult) and
  42 px (cooldowns); labels 15 px. `blade.rs` lost its unused `duration()`.
**Why:** You: "check this folder... do the build for me".
**Tested how:**
- Frames extracted from the clips, measured, and put on sheets.
- A mock-up drawn from the same formulas (Python, outside the game) compared frame by frame with the clip:
  `D:\GenjiBridgeAssets\hud_ref\mock6_ult.png` and `mock6_tile.png`. Tuned twice: thicker ribbons, and a more
  solid, longer-lasting flash.
- The build and the 29 unit tests pass.
**Result:** The mock-up's ult press, tile states and flash line up with the clip. The ready ult's ribbons are
still simpler than Overwatch's. Not tested in game.
**Still broken / not tested:**
- Everything above, in game.
- Not in your clip: the reset by a kill (reuses the ready flash) and the ultimate becoming full.
- The Dragonblade time left is no longer shown (Overwatch shows it near the crosshair, which is outside this step).
- F7 (fill ult) is still in.
**Next:** Your test.

## 2026-10-08 HUD: "pressed" cyan ability boxes; bigger ultimate with breathing aura, blue flames and electric wisps

**Changed:**
- `genji/src/hud.rs`, ability boxes: when you dash or Deflect, the box dips in and springs back (0.22 s) and turns
  cyan with a dark icon, as in your Overwatch frame. Swift Strike stays cyan 0.45 s (about the dash), Deflect for as
  long as it is up. The key label stays lit meanwhile. The old pulsing cyan outline during Deflect is gone.
- `genji/src/hud.rs`, ultimate:
  - Bigger (radius 34 to 41 at 1080p), sized from your screenshot: white centre, thick bright blue/cyan ring, faint
    outer ring, side markers outside the ring, Q just below.
  - Ready: a blue aura breathing slowly (about 4 s per cycle); blue flame particles rising out of the ring and fading;
    5 electric wisps that grow out of the ring, curl sideways, flicker and fade, each at a new place every time.
    The rotating light rays are gone.
- `swift.rs`, `deflect.rs`, `camera.rs`: use counters passed to the HUD (to know when to play the press).
**Why:** You, with Overwatch frames of the ready ultimate and the pressed Swift Strike box.
**Tested how:** Build and the 29 unit tests pass. The ult's wisps and flames were checked as a mock-up drawn from the
same formulas outside the game (Python), to tune their length and curl. Not seen in game.
**Result:** Not tested in game.
**Still broken / not tested:** Everything above in game; F7 (fill ult) still in for testing.
**Next:** Your test.

## 2026-10-08 Portrait: back to the original picture, fitted to fill the hexagon

**Changed:** `tools/make_hud_art.py`: the portrait is Genji's helmet picture again (`0000000040C7`, your choice:
"use this image, only fix the portrait"), zoomed so it fills the hexagon edge to edge, with the thick glowing cyan
outline. It's an opaque square, so it stays inside the hexagon (no rising over the top edge, which would show a
hard-edged block; that option remains for a transparent picture, `RISE_ABOVE`).
**Why:** You, while I was building the previous entry.
**Tested how:** Rebuilt `D:\GenjiBridgeAssets\hud\portrait_hex.png` and looked at it on a grey background. No code
change in the mod (it reads the picture at startup).
**Result:** Picture as intended; not seen in game yet.
**Still broken / not tested:** In game.
**Next:** Your test of the HUD changes.

## 2026-10-08 HUD: Overwatch-like portrait, same-size health numbers, ultimate higher with light, new cooldown flash

**Changed:**
- `tools/make_hud_art.py`, the portrait:
  - Now Overwatch's side bust (`0000000106EC`), zoomed to head and shoulders.
  - It fills a dark hexagon; the helmet rises over the top edge, in front of the outline.
  - Thick cyan outline from the hexagon's own edge (Overwatch's outline texture is open at the top) with a blurred
    glow.
- `genji/src/hud.rs`:
  - Portrait drawn at its proportions.
  - Health numbers both the same size (max dimmer).
  - Ultimate raised by 34 px (at 1080p).
  - Ultimate ready: the spinning rings are replaced by light (a wide breathing blue halo, a brighter inner glow,
    faint slowly turning light rays, the glow ring).
  - Cooldown finished: a rectangle of light centred on the box, as tall above as below, shrinking to the box while
    fading, fast (0.45 s), with a white core and pixel sparks drifting up.

**Why:** Your test of commit `af6711b`, points 2-4 (with your Overwatch frames).
**Tested how:** Build OK; unit tests pass. Looked at the portrait picture: it matches your screenshot's layout.
**Result:** Not tested in game.
**Still broken / not tested:** In game. F7 test key still in.
**Next:** Your screenshots.

## 2026-10-08 Elden Ring's HUD switched off in Genji mode

**Changed:**
- `genji/src/controls.rs`:
  - While Genji input is captured, Elden Ring's HUD option is set to Off (`GameSettings.hud_type`, the same as
    Options > Display > HUD) and set back to your setting when capture ends (menus, F9, loading).
  - The "gameplay" rule no longer needs the HUD's UI state 0x07. Gameplay = 0x08 visible, 0x07 visible or the HUD
    switched off by us, and no UI state outside the ones seen during play in your logs (0x04, 05, 07, 08, 09, 0f,
    11, 12, 13, 17, 1a).
  - An unknown state counts as a menu (safe side) and is logged once.
  - `MODE` lines log the switch.
- `genji/src/lib.rs`: if our per-frame code crashes, the HUD is switched back on.

**Why:** Your test of commit `af6711b`, point 1: Elden Ring's HUD makes it uglier; remove it completely in Genji
mode, rune gains included.
**Tested how:** Build OK; unit tests pass.
**Result:** Not tested in game.
**Still broken / not tested:**
- Whether hiding the HUD also hides enemy health bars and damage numbers (likely; they're part of it).
- Whether some pop-up during play uses a state not in the list (it would release Genji input briefly; the log will
  name it).

**Next:** Your test.

## 2026-10-08 HUD closer to Overwatch 2, with its animations

**Changed:**
- `genji/src/hud.rs` rewritten after your Overwatch screenshots:
  - Health: upright bold numbers with a divider; straight white segments; a soft cyan glow behind the hexagon portrait.
  - Ultimate, charging: navy disc, teal inner ring, faint outer ring, dotted circle, the charge as 48 cyan ticks,
    italic number.
  - Ultimate, ready: pulsing blue glow ring, two counter-rotating rings of dashes, white centre with the Dragonblade
    icon, white side markers, a light burst when it becomes ready, filled Q box.
  - Ultimate during Dragonblade: glow, a ring draining with the time left, the seconds.
  - Abilities: straight light boxes with a white underline when ready; while cooling, grey with cyan rising from the
    bottom and italic seconds; outlined key box while cooling, filled when ready.
  - Abilities, cooldown finishing: a vertical beam of light and a burst.
  - Abilities, Deflect up: a pulsing cyan edge.
  - Ammo: panel between two glowing cyan bars with angled ends; bold numbers with a cyan divider.
- `tools/make_hud_art.py`: also copies Overwatch's UI effect pictures (glow ring, ring of dashes, light flare, soft
  glow; found on contact sheets of the blue UI textures).
- `genji/src/swift.rs`, `deflect.rs`, `camera.rs`: full cooldowns for the HUD.
- **Temporary for testing: F7 fills the ultimate again (remove once the HUD is approved).**

**Why:** Your test of commit `173a5d5`: good but far from Overwatch; the ultimate ring and Overwatch's little HUD
animations (cooldown finished, ultimate ready). Overwatch's animations are engine effects, not pictures in its UI
files (searched 808 blue UI textures; the ultimate's blue dragon flame isn't among them), so they're redrawn with
Overwatch's glow textures.
**Tested how:** Build OK, no warnings; unit tests pass.
**Result:** Not tested in game.
**Still broken / not tested:** In game; the blue dragon flame of the ready ultimate is approximated (glow and
rotating rings), not reproduced.
**Next:** Your screenshots.

## 2026-10-08 HUD with Overwatch's own UI art

**Changed:**
- DataTool `dump-ui-textures` and `extract-vector-images` to `D:\GenjiBridgeAssets\ui-textures`, `ui-vectors`
  (20,623 + 1,502 files, outside the repository).
- Contact sheets in `D:\GenjiBridgeAssets\sheets` to find the HUD pieces.
- New `tools/make_hud_art.py`: builds `D:\GenjiBridgeAssets\hud` (hexagon portrait with outline, the ultimate's
  dark disc, the "+" icon).
- `genji/src/hud.rs` rewritten in Overwatch 2's style from your screenshot:
  - hexagon portrait, "+" box, italic health numbers, 10 slanted white segments;
  - the dark disc with a filling ring, the number, and a Q key box;
  - slanted ability frames: white with dark icon when ready, dark with seconds while cooling, cyan edge while
    Deflect is up;
  - LSHIFT/E key boxes;
  - shuriken with italic ammo between cyan lines;
  - Segoe UI Black Italic numbers.
- `docs/DESIGN.md` 5h.

**Why:** Your test of commit `285d212`: every HUD element works, but it should use Overwatch's own designs.
**Tested how:** Build OK, no warnings; unit tests pass. I looked at the generated portrait picture (hexagon with cyan
outline).
**Result:** Not tested in game.
**Still broken / not tested:**
- In game.
- Overwatch's exact HUD layout and font aren't extractable as files; positions follow your screenshot.

**Next:** Your screenshot.

## 2026-10-08 Dragonblade: 1 s unsheathing

**Changed:** `genji/src/blade.rs`: after Q, 1 s of unsheathing during which you can't swing and the 6 s don't drain
(`BLADE unsheathed` line when it ends). `docs/DESIGN.md` unchanged in substance (duration "6 s, with a separate
unsheathing animation" was already noted).
**Why:** You (test of commit `285d212`): Dragonblade has a 1 s unsheathing in Overwatch. In the same test: all HUD
elements work, but the HUD should use Overwatch's own designs (next entry).
**Tested how:** Build OK; unit tests pass.
**Result:** Not tested in game.
**Still broken / not tested:** In game.
**Next:** Overwatch's HUD art.

## 2026-10-08 Step 12: Overwatch-style HUD

**Changed:**
- New `genji/src/hud.rs`, drawn by the overlay while the first-person view is on, scaled from 1080p to your screen:
  - Bottom left: Genji's portrait, health "current | max" and a bar of 10 segments.
  - Bottom centre: an ultimate ring with the percentage. When full, the Dragonblade icon and "Q"; during Dragonblade
    it drains with the seconds left.
  - Bottom right: Swift Strike (LSHIFT) and Deflect (E) icons with cooldown seconds (darkened while cooling,
    highlighted while Deflect is up), and the shuriken icon with ammo "n | 30" or "RELOADING" with a bar.
  - Images are Overwatch's own, read at startup from `D:\GenjiBridgeAssets\raw` (never in the repository; a missing
    one is logged and skipped).
  - Font: Windows' Bahnschrift.
- `genji/src/crosshair.rs`: loads the HUD at startup and draws it after the crosshair. The temporary "ULT n%" and
  "DEFLECT" texts are removed.
- `genji/src/camera.rs`: fills the HUD every frame.
- `genji/src/firing.rs`, `swift.rs`, `deflect.rs`, `blade.rs`: small getters.
- `genji/Cargo.toml`: `png` 0.17 (MIT/Apache) to read the images.
- `docs/DESIGN.md`: section 5h.

**Why:** You chose the HUD before the model (step 10).
**Tested how:** Build OK, no warnings; unit tests pass.
**Result:** Not tested in game. A `HUD resources:` line at startup says what loaded.
**Still broken / not tested:** Everything visual; Elden Ring's own HUD stays on screen too.
**Next:** Your test, with a screenshot.

## 2026-10-08 Step 2 unblocked: old Overwatch build via download_depot; extraction to D:

**Changed:**
- `tools/extract_genji.ps1`: reads from the old build by default; writes to `D:\GenjiBridgeAssets\raw` (your
  choice: C: is full; outside the repository, so never committed).
- `docs/DESIGN.md`: step 2 route.
- Outside the repository:
  - `D:\SteamDepotDownloads\app_2357570\Overwatch_2.24.1` combines the three depots you downloaded (99 hard links,
    no extra space; the downloads themselves unchanged).

**Why:** DataTool v2.24.1 (2026-09-15) can't read your live Overwatch. You downloaded the Sept 8 build with Steam's
console (`download_depot 2357570`: depots 2357571, 2357572 English, 2437288).
**Tested how:** DataTool `list-heroes` on the combined folder: build `prometheus-2_24_1_0-153264`, CASC ready, Genji
listed.
**Result:** Works; full extraction running.
**Still broken / not tested:** Extraction output.
**Next:** Look at what came out (model, first-person parts, animations, sounds).

## 2026-10-08 Step 9 validated; F7 test key removed

**Changed:** `genji/src/camera.rs`, `genji/src/ult.rs`, `genji/src/dinput.rs`: the temporary F7 full-charge key is
removed. `README.md`, `docs/DESIGN.md`: step 9 done.
**Why:** You validated Dragonblade (and melee, Deflect during it); your test of commit `4260db3`: the ghost dash
works and no more falling through the map.
**Tested how:** Build OK; unit tests pass.
**Result:** Works.
**Still broken / not tested:** Nothing known.
**Next:** Step 10 needs Overwatch's assets (step 2).

## 2026-10-08 Fell through the map after a ghost dash: three fixes

**Changed:** `genji/src/movement.rs`:
1. Mid-dash, a push from the game's collision is only undone when a character is within 2 m (pushes out of terrain
   are kept).
2. In the air, every frame, a ray from 0.6 m above the feet checks we haven't sunk into the ground; if we have, the
   feet go back on top.
3. A dash lifts off the ground only when looking up more than about 7 degrees (was 3).
4. After the controller gave up, it may take over again after a respawn or when Genji input is released (F9 off,
   or a menu).

`genji/src/camera.rs`: wiring.

**Why:** Your test of commit `0e13c9f`: everything else works (Dragonblade through shields, wider cone, melee
breaking guards, Deflect during Dragonblade). But the ghost dash (dash, kill, 180, dash again) dropped you through
the map, and after respawning Genji mode never came back. The ghost dash itself worked (three resets 0.4 s apart).
The log shows:
- Dash #5 started on the ground looking 3 degrees up, so it lifted off and skimmed at ground height with no ground
  check.
- Rising terrain pushed us 0.3 m per frame back out, and the dash undid every push (the body-redirect fix).
- It ended inside the terrain, the landing ray (starting below the surface) never found ground, and the 5 s
  falling safety net gave up for the whole session.

**Tested how:** Build OK; 29 unit tests pass.
**Result:** Not tested in game.
**Still broken / not tested:**
- Ghost dash on slopes.
- Whether the sunk check misfires next to ledges (it could put us on top of a ledge we jump beside; that's also
  how a vault ends).

**Next:** Your test.

## 2026-10-08 Dragonblade ignores shields, wider cone; melee breaks guards; Deflect allowed during Dragonblade

**Changed:**
- `genji/src/blade.rs`, swings:
  - They now use Corpse Piler's row with "Disable Guard" (AtkParam `disableGuard`: shields are ignored).
  - The cone is 90 degrees each side (was 60).
- `genji/src/blade.rs`, quick melee:
  - It hits from our side, so a shield blocks it.
  - The first melee on a character does 60 stamina and 40 poise damage. Another within 5 s does 999 of both, meant
    to break the guard or stance.
  - Each melee logs the target's stamina and poise before and after. A melee fully blocked by a shield counts as
    landed when its stamina or poise drops.
- `genji/src/camera.rs`: Deflect works during Dragonblade again (no swings while it's up).

**Why:** Your test of commit `bb9ce3c`: everything works, but shields shouldn't block Dragonblade. Log: some swings
did 30 where unblocked ones did 153 at the same attack power (80% shield absorption), although fired from behind:
the game seems to judge blocking by where the attacker stands. You asked for melee to break a shielded enemy's stance
in about 2 hits, a wider cone, and Deflect during Dragonblade (allowed in Overwatch).
**Tested how:** Build OK; 29 unit tests pass.
**Result:** Not tested in game.
**Still broken / not tested:**
- Whether the game applies stamina/poise values for the player's attacks (Paramdex says those fields are "NPCs only";
  the log will show).
- F7 test key still in.

**Next:** Your test.

## 2026-10-08 Step 9: Dragonblade, ultimate charge, quick melee

**Changed:**
- New `genji/src/ult.rs`:
  - Ultimate charge in Overwatch points: 27/54 per confirmed shuriken hit (27 for a killing blow), 50 per Swift
    Strike hit, 40 per quick melee hit.
  - Plus 5 per second, paused during Dragonblade. Cost 2000.
  - `ULT` lines every 10%.
- New `genji/src/blade.rs`:
  - Q when full starts Dragonblade for 6 s; it resets Swift Strike.
  - Holding left click swings every 0.61 s. Each swing hits every living character within 5 m in a 60-degree-each-side
    cone (and at reachable height).
  - Each hit is an invisible point-blank projectile from behind the target. Its carrier is the Ruin Fragment's
    Bullet row, pointed at Corpse Piler's damage row (Rivers of Blood); attack power 100 x dexterity. Direct damage
    is the fallback.
  - A third of the target's bleed bar per swing hit.
  - Each swing fires the Blood Blade crescent with its damage row removed (only a look).
  - Mouse 5 is quick melee outside Dragonblade: 40 x dexterity to the nearest character within 2.5 m in front, same
    carrier, 0.8 s recovery.
  - All rows changed in memory while Genji mode is on.
- `genji/src/camera.rs`: wiring. No shuriken and no Deflect during Dragonblade.
- `genji/src/crosshair.rs`: "ULT n%", "ULT READY (Q)", "DRAGONBLADE n s" under the crosshair.
- `genji/src/swift.rs`, `genji/src/shuriken.rs`: charge hooks; Swift Strike reset.

**Why:** Step 9 (DESIGN.md 5g) with your Season 5 numbers and choices.
**Tested how:** Build OK, no warnings; 29 unit tests pass.
**Result:** Not tested in game.
**Still broken / not tested:**
- Everything in game: whether the Corpse Piler row's hit and the crescent look right.
- The swing cone angle and the melee recovery aren't Overwatch-verified.
- Swing hits don't add ultimate charge (as in Overwatch).

**Next:** Your test. **Temporary for testing: F7 fills the ultimate (remove once Dragonblade is validated).**

## 2026-10-08 Step 9 plan; reminder for step 10

**Changed:** `docs/DESIGN.md` section 5g with your Overwatch numbers and choices (Corpse Piler hits with bleed,
Blood Blade crescent as the visible sweep until Genji's model exists; the dash keeps its own damage).
**Why:** Step 9. Also checked: DataTool is still v2.24.1 (2026-09-15) and can't read your Overwatch build.
**Tested how:** Nothing to test.
**Result:** Plan only.
**Still broken / not tested:** Step 2 (Overwatch assets). **Reminder (your request): if we reach step 10 without a
DataTool that reads your Overwatch, give you the exact Steam console steps to download an older build
(`download_depot`).**
**Next:** Build step 9.

## 2026-10-08 Deflect validated; cooldown back to 8 s

**Changed:** `genji/src/deflect.rs`: cooldown 8 s (was 1 s for testing). `README.md`, `docs/DESIGN.md`: step 8 done.
**Why:** Your test of commit `2b8366b`: projectiles come back as themselves, damage good; you validated Deflect.
**Tested how:** You played; I read `logs/genji.log`: 76 projectiles sent back as themselves, 0 Bone Dart
fallbacks, 13 kinds (arrows, pebbles, the mage's volleys, other spells). Build OK; 29 unit tests pass.
**Result:** Works (cooldown change itself not tested in game).
**Still broken / not tested:** Nothing known for Deflect.
**Next:** Step 9, Dragonblade and ultimate charge.

## 2026-10-08 Deflect sends projectiles back as themselves

**Changed:** `genji/src/deflect.rs`:
- A caught projectile is fired back with its own Bullet row, so arrows come back as arrows and pebbles as pebbles,
  owned by us.
- While deflecting, each caught kind's Bullet row points at our damage row (the Bone Dart's AtkParam_Pc, carrying
  the copied values), because fired by us the game looks damage up among the player's rows. Restored when Deflect
  ends or Genji mode lets go.
- If the game refuses a kind, the Bone Dart goes instead (`as Bone Dart` in the log).

**Why:** Your choice (option B). Your test of commit `fbd85e9`: Deflect works on arrows, Glintstone Pebbles and the
two-headed mage's volleys. Log: 40 sent back, all ok (arrows 100 physical, pebbles 120-190 magic, six caught in one
frame).
**Tested how:** Build OK; 29 unit tests pass.
**Result:** Not tested in game.
**Still broken / not tested:**
- How special kinds behave fired by us (homing, exploding spells).
- An enemy firing the same kind while we deflect also uses our row (same values).
- Cooldown still 1 s for testing.

**Next:** Your test.

## 2026-10-08 Step 8b: Deflect sends projectiles back; cooldown 1 s for testing

**Changed:**
- `genji/src/deflect.rs`, while deflecting, every frame:
  - Enemy projectiles within 3 m of our chest that fly at us from the front are removed (no life, no speed, moved
    50 m down).
  - For each one, a Bone Dart is fired along our aim at the original's speed (20-80 m/s). It carries the
    original's damage values (physical, magic, fire, lightning, holy from its AtkParam_Npc row).
  - The Bone Dart rows are changed in memory while Genji mode is on (straight flight, no homing, no bounce effect,
    80 m) and restored afterwards.
  - The character under the crosshair gets kill credit (approximation).
  - `DEFLECT sent back ...` logs each one.
- **Cooldown 1 s while we test (your request); back to 8 s (confirmed by you) once Deflect is validated.**
- `genji/src/shuriken.rs`: aim helpers shared.
- `genji/src/camera.rs`: wiring.

**Why:** Step 8b. Your test of commit `bf12089`: no damage with arrows anywhere on screen; melee blocked too. In
Overwatch, anything shooting you within your view can be deflected (your screenshot); ours covers 90 degrees each
side.
**Tested how:** Build OK; 29 unit tests pass.
**Result:** Not tested in game.
**Still broken / not tested:**
- Sending back in game.
- Whether removing the enemy projectile this way works for every kind (arrows, spells, fire pots).
- Cooldown is 1 s (testing).

**Next:** Your test.

## 2026-10-08 Deflect: projectiles judged by where they come from

**Changed:** `genji/src/deflect.rs`: an enemy projectile switches protection off only if it flies the way we face
(it comes from behind), within 4 m and closing in. Before, its position decided.
**Why:** Your test of commit `244f095`: blocking works, but arrows still hurt depending on the view; you need at least
the central 80% of the screen protected. Log: every hit taken while deflecting came on a frame marked "enemy
projectile closing in from behind" (13 uses, 10 hits of 54): arrows from the front were level with or a hair past
our body's centre on the last frame before the hit. Now everything flying at us within 90 degrees of where we look
is blocked (wider than the screen).
**Tested how:** Build OK; 29 unit tests pass.
**Result:** Not tested in game. Several melee enemies at once not tested yet either.
**Still broken / not tested:** Arrows from the sides of the screen; melee from several enemies.
**Next:** Your test; then 8b (sending projectiles back).

## 2026-10-08 Step 8a: Deflect blocks frontal damage (E)

**Changed:**
- New `genji/src/deflect.rs` (3 unit tests):
  - E starts Deflect for 2 s; E again ends it early; then an 8 s cooldown.
  - While it runs, the game's "invincible" character flag is set on every frame where nothing threatens from
    behind: no living enemy within 4 m in our rear half, no enemy projectile closing in from behind.
  - Logs: start, end (and why), when protection was off and why, and HP changes while deflecting.
- `genji/src/camera.rs`: wiring; no shuriken while deflecting.
- `genji/src/crosshair.rs`: "DEFLECT" under the crosshair while active.
- `genji/src/lib.rs`: the flag is cleared if our per-frame code ever crashes.
- `docs/DESIGN.md`: section 5f (plan for 8a and 8b).

**Why:** Step 8. Overwatch's Deflect (wiki and patch notes): 2 s, about 8 s cooldown, frontal (under 90 degrees),
melee blocked, projectiles sent back (8b, next), E again cancels, no shooting meanwhile.
**Tested how:** Build OK; 29 unit tests pass.
**Result:** Not tested in game.
**Still broken / not tested:**
- Whether the invincible flag also stops hit reactions and status buildup.
- Projectiles aren't sent back yet (8b).

**Next:** Your test; then 8b.

## 2026-10-08 Elimination window 5 s (Overwatch community figure)

**Changed:** `genji/src/kills.rs`: a character we damaged that dies within 5 s (was 3) counts as our elimination.
`docs/DESIGN.md`: noted.
**Why:** You asked for Overwatch's rule. Blizzard publishes no number (any elimination credit resets Swift Strike);
community reports put elimination credit at about 4-5 s, "no later than 5 s" (not official). Your last test: bleed on
every dash works (17 hits, 17 bleeds); dash pushes ignored (0.01-0.07 m each); one death 3.84 s after our last hit
didn't count.
**Tested how:** Build OK; 26 unit tests pass.
**Result:** Not tested in game.
**Still broken / not tested:** Nothing new.
**Next:** Step 8, Deflect.

## 2026-10-08 Swift Strike: every hit bleeds; pushes ignored mid-dash; log deaths that aren't ours

**Changed:**
- `genji/src/swift.rs`: each dash hit gives bleed buildup 9999, a sure bleed proc; immune enemies (bleed resistance
  999+) get none. This replaces half a bar.
- `genji/src/movement.rs`: during a dash, movement made by the game's own collision is undone
  (`DASH ignored a N m push`).
- `genji/src/kills.rs` (rewritten, same behaviour): a death not counted as ours within 0.6 s is logged
  (`KILL death NOT counted as ours`, with when we last damaged it).
- `README.md`, `docs/DESIGN.md`: bleed.

**Why:** Your test of commits `a8b77cf`, `e8de50d`.
- Strikes on shielded enemies now land.
- Bleed procced on the big monster after 2 dashes, not on guards: their two 56 buildups (bar 112) were 4+ s
  apart and buildup fades.
- You asked for a 100% bleed per dash hit, no meter.
- In a row of enemies one died in front of you and the dash got redirected: the log shows a 2.07 m push from the
  game's collision mid-dash.
- A far-away kill didn't reset the dash: nothing in the log says why (deaths that aren't ours weren't logged).

**Tested how:** Build OK; 26 unit tests pass.
**Result:** Not tested in game.
**Still broken / not tested:** The far-away kill: next time the log says why.
**Next:** Your test.

## 2026-10-08 Swift Strike: bleed applied directly, strike from behind the enemy's facing, kill grace, no fizzle

**Changed:**
- `genji/src/swift.rs`: once the strike lands (or the direct fallback is used), the bleed is applied to the
  enemy directly (the Kukri's status effect 3115 with our buildup, via the game's apply-status-effect function;
  `DASH bleed N applied`). The strike now comes from behind the enemy's own facing. An elimination up to 0.25 s
  before a dash starts also resets it (`SWIFT reset by an elimination 0.xx s before this dash`); 1 new unit test.
- `genji/src/movement.rs`: the "aimed at the ground" stop only counts hits at most 0.3 m above our feet.

**Why:** Your test of commit `aefce72`.
- No bleed bar ever appeared, although the strikes landed (game dealt 42-99) with buildup set to half the bar.
- Shielded enemies sometimes took 7-8. The strike's "behind" was measured from us, and we are inside or next to
  the enemy at that moment.
- The Fan of Blades + dash combo sometimes didn't reset. In the log, twice the view ray hit something at eye height
  next to us and the dash ended after 0.1-0.7 m, cooldown spent. Also a kill counted just before the dash started
  reset nothing.

**Tested how:** Build OK; 26 unit tests pass.
**Result:** Not tested in game.
**Still broken / not tested:** Whether a directly applied status effect builds up bleed (if not, next is the hit
itself carrying it via AtkParam's own status-effect slot).
**Next:** Your test.

## 2026-10-08 Swift Strike: real game hit (invisible Kukri) with half a bar of bleed

**Changed:** `genji/src/swift.rs`: each character the dash touches gets an invisible point-blank projectile from the
Kukri's rows, fired from behind it (shields face you): attack power 50 x dexterity x 1.0 (`DASH_ATTACK_FACTOR`, to stay
near the damage you liked), bleed buildup = half of that character's own bleed resistance (NpcParam `resistBlood`;
999+ = immune, none). The Kukri rows are changed in memory while Genji mode is on (no flight visual, 50 m/s, 0.2 s,
"Is Base Attack Added") and restored afterwards. If the game registers no hit within 10 frames, the damage is applied
directly as before (`DASH HIT ... directly`). Logs: `DASH strike on ...` (attack, bleed, resistance), `DASH HIT ...
game dealt`. `genji/src/shuriken.rs`: the projectile spawn is a shared function. `genji/src/camera.rs`: wiring.
Earlier test: you confirmed the dash now resets after one-shot shuriken kills.
**Why:** Your choices: the game deals the damage (number, sound, reaction) and half a bar of bleed per dash hit.
**Tested how:** Build OK; 25 unit tests pass.
**Result:** Not tested in game.
**Still broken / not tested:** Whether the strike lands from behind; bleed amounts in game (resistances may be
raised in later areas by effects the NpcParam value doesn't include).
**Next:** Your test.

## 2026-10-08 One-shot shuriken kills count as eliminations (Swift Strike reset)

**Changed:** `genji/src/shuriken.rs`: shuriken are matched against characters that died within the last 0.5 s too;
such a hit is a killing blow (`HIT killing blow`) and counts for the Swift Strike reset. `genji/src/targets.rs`:
`all()` lists dead characters as well.
**Why:** Your test of commit `28f466f`: shuriken now deal real damage (78-85 per hit at attack power 82, 55-78 on a
tougher one, 3 when guarded), but killing with the Fan of Blades + dash combo didn't reset the dash, even across
several kills. Log: goats (55 HP) died to one shuriken; the game's damage lands before we match the shuriken to a
character, and the match skipped dead ones, so those kills were never ours (no HIT, no KILL lines). Dash kills were
fine (`SWIFT reset` after each).
**Tested how:** Build OK; 25 unit tests pass.
**Result:** Not tested in game.
**Still broken / not tested:** Kill reset after one-shot kills.
**Next:** Your test; then Swift Strike's real hit and the half bar of bleed.

## 2026-10-08 Shuriken attack power actually used ("Is Base Attack Added")

**Changed:** `genji/src/shuriken.rs`: while Genji mode is on, the dagger's AtkParam row also gets "Is Base Attack
Added" (`isAddBaseAtk`), restored with the other vanilla values.
**Why:** Your test of commit `af55c29`: shuriken now do about 3 damage to everything. Log: `attack power 82`, yet
`game dealt 4` on every hit, exactly as with attack power 1. Paramdex notes (WitchyBND): for the player's attacks
the damage is a base attack power from the item or weapon times the row's correction percentage, and `atkPhys` only
counts with "Is Base Attack Added". Our throws carry no item, so the base was about 0 and every hit dealt the
game's minimum.
**Tested how:** Build OK; 25 unit tests pass.
**Result:** Not tested in game. If the game still deals 4, the next step is to throw "as" the Throwing Dagger item
(goods id 1700) so the base comes from it.
**Still broken / not tested:** Shuriken damage.
**Next:** Your test.

## 2026-10-08 Test of damage option 2 part 1 and FOV option B: health bars fixed

**Changed:** Nothing in code. `STATUS.md` (health bars) marked resolved and moved to
`docs/STATUS-2026-10-08-health-bars.md`.
**Why:** Test of commits `daa46d6`, `af55c29`.
**Tested how:** You played; I read `logs/genji.log`.
**Result:** Health bars stay on the enemies (far ones slightly off, sound right: fine by you). The game now produces
the 70.5 field of view itself. Dash damage "perfect" (still lowered directly: 76 at dexterity 25). Shuriken: far too
weak (see the entry above).
**Still broken / not tested:** Shuriken damage.
**Next:** Fix shuriken damage.

## 2026-10-08 Shuriken damage dealt by the game (damage option 2, part 1)

**Changed:** `genji/src/shuriken.rs`: the dagger's AtkParam attack power is now 27 x dexterity multiplier x 2.0
(`ATTACK_FACTOR`, a first guess), updated whenever dexterity changes (`THROW shuriken attack power ...`); we no longer
lower HP for body hits. A confirmed headshot gets the game's damage again on top (x2, not in the floating number).
`HIT` lines now log what the game dealt and the attack power; `BLOCKED` when it dealt nothing. `docs/DESIGN.md`:
section 5e (how damage is dealt).
**Why:** Your choice: the game deals the damage (right floating numbers; defenses and shields apply like any weapon).
**Tested how:** Build OK; 25 unit tests pass.
**Result:** Not tested in game. How much the game deals at this attack power against Limgrave enemies is unknown;
the HIT lines will show it.
**Still broken / not tested:** Damage balance (ATTACK_FACTOR); dash still lowers HP directly (part 2 next).
**Next:** Your test, then part 2: dash via an invisible Kukri hit, with the half bar of bleed.

## 2026-10-08 Health bars, STATUS option B: Overwatch FOV through LockCamParam

**Changed:** `genji/src/camera.rs`: while first person is on, every LockCamParam row's `camFovY` (the game's own
field-of-view setting) is set to 70.5 in memory; restored whenever first person ends (`CAMERA LockCamParam fov`
lines). The direct camera writes stay. The 2-second `CAMERA` line now also shows the field of view the game itself
produced that frame (before our overwrite): 70.5 there means the game took it from the param.
**Why:** Your choice (STATUS.md option B): if the game places health bars with its own FOV setting, picture and bars
agree.
**Tested how:** Build OK; 25 unit tests pass.
**Result:** Not tested in game.
**Still broken / not tested:** Health bars.
**Next:** Damage option 2 (the game deals the damage).

## 2026-10-08 Test of dexterity scaling and chr_cam FOV; STATUS.md for health bars

**Changed:** `STATUS.md` (new: health bars, two failed attempts, options); the old one (shuriken limit, resolved)
moved to `docs/STATUS-2026-10-07-shuriken-limit.md`. Test of commits `a61dc64`, `4405f4e`.
**Why:** You: damage still seems weak; health bars further away than before.
**Tested how:** I read `logs/genji.log`.
**Result:** Scaling works in the numbers: `SCALING dexterity 25: x1.52 (shuriken 41/82, Swift Strike 76)`; hits take
41 / 82 / 76 HP (e.g. headshot hp 210 -> 128). What looks weak is the floating damage number: it shows only the
game's own small hit (4), not the HP we take directly. Health bars: two attempts failed, written up in STATUS.md.
**Still broken / not tested:** Damage numbers show only the game's part; health bars.
**Next:** Your choice in STATUS.md, and how damage should be dealt (asked you).

## 2026-10-08 Dexterity scaling for all Genji damage; full shuriken damage again

**Changed:** New `genji/src/scaling.rs`: damage = Overwatch base x (1 + 1.5 x curve(dexterity)/100), with Elden Ring's
own "Default" scaling curve (CalcCorrectGraph row 0, read from the game at runtime; your regulation.bin: 18 -> 25%,
60 -> 75%, 80 -> 90%, exponents 1.2/-1.2/1/1). 1.5 is grade A in the game's terms. A `SCALING` line logs your
dexterity and the resulting damage whenever it changes. Used by shuriken (27/54) and Swift Strike (50).
`genji/src/shuriken.rs`: a confirmed hit deals our full (scaled) damage on top of the game's own 4-6 again, as
before the last change. 2 unit tests for the curve.
**Why:** You: blocking works, but damage felt weaker (the game's part was being subtracted); change back, and all of
Genji's abilities should scale with dexterity.
**Tested how:** Build OK; 25 unit tests pass.
**Result:** Not tested in game.
**Still broken / not tested:** In game; balance (example: dexterity 40 -> x1.81: 49/98 shuriken, 91 dash).
**Next:** Your test; then the half bar of bleed.

## 2026-10-08 chr_cam gets the Overwatch FOV too (health bars)

**Changed:** `genji/src/camera.rs`: the game's ChrCam gets the same field of view as the screen (70.5 vertical).
**Why:** Your F8 marks: all cameras face where we look, but chr_cam still had 48 degrees, which places things about
1.6 times further from the screen centre: likely the misplaced health bars (more often far away, you said). Not
proven: the marks don't say which bar was wrong.
**Tested how:** Build OK.
**Result:** Not tested in game.
**Still broken / not tested:** Health bars.
**Next:** F8 again if a bar is still wrong.

## 2026-10-08 F8 camera snapshot (health bars in the wrong place)

**Changed:** `genji/src/camera.rs`, `genji/src/dinput.rs`: in first person, F8 writes `MARK` lines: every camera's
position, direction, field of view and aspect, and where each would put the 3 nearest characters on screen.
**Why:** Your test: health bars still sometimes sit in the wrong place. The 2-second `CAMERA` lines show the game's
follow camera now stays within 1-10 degrees of our view, which can't put a bar on the other side of the screen, so
the game places bars with something else. Press F8 when you see a wrong bar; I'll match it to a camera.
**Tested how:** Build OK; 23 unit tests pass.
**Result:** Not tested in game.
**Still broken / not tested:** Health bars sometimes misplaced.
**Next:** Your F8 marks.

## 2026-10-08 Shuriken damage only when the game confirms the hit (shield blocks do nothing)

**Changed:** `genji/src/shuriken.rs`: a shuriken ending next to a character becomes a pending hit; it does damage
only if, within 10 frames, the character loses HP to the game's own small hit (`HIT ... game N + ours M`, total =
27/54); otherwise `HIT BLOCKED`, no damage. The 2-second report counts blocked ones. `genji/src/targets.rs`: keeps
the total HP we took from each character ourselves, to tell the game's HP changes from ours.
**Why:** Your test: shuriken blocked by shields still did damage that didn't show on the health bar. The log of that
session proves it: real hits are always preceded by the game's own 4-6 HP (e.g. 187, then 181 at the next hit), and
the shield-blocked ones by none (152 -> 152, 125 -> 125, 83 -> 83, 56 -> 56, 29 -> 29). It also means body shots
did 31-32 instead of 27; now the game's part is subtracted.
**Tested how:** Build OK; 23 unit tests pass.
**Result:** Not tested in game. Earlier in that test: dashes now go their full 15 m through enemies (body fix
works).
**Still broken / not tested:** Blocked shuriken in game; damage from other sources (your own weapon, other enemies)
landing in the same 10 frames would also count as confirmation.
**Next:** Your test.

## 2026-10-08 First person uses your Overwatch FOV (103)

**Changed:** `genji/src/camera.rs`: in first person, the drawn camera and the follow camera get a vertical field of
view of 70.5 degrees, which is Overwatch's 103 measured at 16:9 (about 119 across on your 21:9 screen, assuming both
games widen rather than crop on ultrawide). Outside first person the game's own (48 vertical) comes back. 1 new unit
test for the conversion. `genji/src/firing.rs`: fan comment updated (it now also looks as wide as in Overwatch).
**Why:** You asked for the same FOV as Overwatch.
**Tested how:** Build OK; 23 unit tests pass.
**Result:** Not tested in game.
**Still broken / not tested:** Whether the game keeps our FOV (it may overwrite it); the next `CAMERA fov` line
when first person starts will show the game's value before ours.
**Next:** Your test; then the half bar of bleed on Swift Strike.

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
