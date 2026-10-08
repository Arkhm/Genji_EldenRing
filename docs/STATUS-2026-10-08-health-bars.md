# STATUS: enemy health bars drawn away from the enemy in first person

## Resolved (2026-10-08, option B)
With the Overwatch field of view set through LockCamParam (commit `daa46d6`), the game produces 70.5 itself
(`CAMERA ... the game's own fov this frame: follow 70.5, chr_cam 70.5`) and you saw the bars stay on the enemies.
Far-away enemies' bars are slightly off, but their sounds are right; you said that's not a problem.

Date: 2026-10-08
Step: 4 (first-person camera) and 12 (HUD); see `docs/DESIGN.md`

## Goal
Enemy health bars (and enemy sounds) sit on the enemy in Genji mode, with the Overwatch field of view (103, i.e.
70.5 degrees vertical). Pass condition: a fight with several enemies near and far, bars always above the enemy.

## What happens instead
Bars appear away from the enemy, toward the side of the screen (your screenshots of 2026-10-08), more often for
enemies far away. After the last change (chr_cam FOV) you saw them further away than before.

## Attempts
1. **Game follow camera kept in line with our view** (commit `fc4057a`). Before it, the game's follow camera kept
   facing where it was when first person started (F9 twice fixed it, which fits). Afterwards the `CAMERA` lines show
   the game camera within 1-10 degrees of our view each frame. Result: better ("some are working"), not fixed.
2. **chr_cam given the same field of view** (commit `a61dc64`). The F8 marks showed every camera facing where we look,
   but chr_cam at the game's 48 degrees while the screen shows 70.5, which places things about 1.6 times further from
   the centre. Result: you saw the bars further away than before. So chr_cam isn't (only) what places them.

## What we know for sure
- The F8 marks (2026-10-08, two sessions): pers_cam_1, pers_cam_2 and chr_cam have the same position and direction
  as our view; pers_cam_3/4 are a fixed far-away camera (145-378 m away, fov 43): not the gameplay view.
- The bar error started with first person and got worse when the field of view changed from 48 to 70.5 (first on
  the drawn camera, then also on chr_cam).
- Elden Ring's own field of view comes from LockCamParam (`camFovY`; your regulation.bin: 132 rows default, others
  43-56 degrees), not from a fixed value.

## What we don't know
- What the game uses to place health bars on screen: possibly its own projection built from LockCamParam's field of
  view, or a camera object fromsoftware-rs doesn't describe (ChrCam has private `ex_follow_cam`, `aim_cam`,
  `dist_view_cam`). Answerable with Ghidra (follow the floating-bar drawing code), or by experiment B below.

## Options from here
| | Option | What it does | Cost / risk |
|---|---|---|---|
| A | Keep Elden Ring's field of view (48) in first person | The camera sync (attempt 1) already fixed the direction part; with no FOV mismatch the bars should sit right | Small. Loses the Overwatch FOV |
| **B (recommended)** | Set the Overwatch FOV through the game's own setting: LockCamParam `camFovY` in memory while Genji mode is on (like the shuriken values), restored afterwards | If the bars use the game's own FOV setting, render and bars agree | Small, one session. If bars are still off, we know they don't use it, and A or C remain |
| C | Ghidra: find the code that places the floating bars | The real answer | Larger, like the shuriken-limit investigation |

## State of the repo
All committed and pushed (last code commit `4405f4e`). The game runs normally; Genji mode works. Bars are off in
first person; F8 still writes camera snapshots.
