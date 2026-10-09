"""Which of Genji's first-person animations (model 00000000DBA6) the mod plays for what (step 10, DESIGN.md 5k).

From your animation picker tags (D:\\GenjiBridgeAssets\\anim_picks.json, 2026-10-09) with the changes we agreed on:
- Genji has three stances (normal, Dragonblade, Deflect), each with the same set: a frozen pose (idle), two short
  moves of about 0.25 s and 0.4 s (jump-type) and a 0.47 s walk.
- Idle is each stance's frozen pose (240E, 124B, 342B), not 3645.
- 3631 is Deflect ending during Dragonblade (from both palms up back to the hilt), the pair of 3632.
- 1256 is probably Swift Strike during Dragonblade (0.33 s, about the dash's 0.3 s, from the hilt).
- 33F3 and 363E are probably the end of a wall climb (0.63 s each, normal and Dragonblade), pairs of the climbs
  33F1 and 363D (1.03 s each), not spawns.
Animation ids are the last four hex digits of 00000000XXXX.
"""

# role -> animation ids (several = variants, picked at random or in turn).
ANIMS = {
    # Normal stance.
    'idle': ['240E'],
    'walk': ['2439'],
    'jump': ['242E', '3645'],
    'double_jump': ['242F', '3646'],
    'wall_climb': ['33F1'],
    'wall_climb_ledge': ['33F4'],
    'wall_climb_end': ['33F3'],
    'spawn': ['83CA'],
    'holster': ['83CB'],
    # Shuriken.
    'throw': ['2478'],
    'fan': ['243B', '244F'],
    'reload': ['2436'],
    # Swift Strike and quick melee.
    'dash': ['249F'],
    'melee': ['2434'],
    # Deflect stance.
    'deflect_start': ['342C'],
    'deflect_idle': ['342B'],
    'deflect_walk': ['3576'],
    'deflect_jump': ['359D', '364D'],
    'deflect_land': ['359E', '364E'],
    'deflect_block': ['24A2', '24A3', '344C', '344D'],
    'deflect_end': ['3555'],
    # Dragonblade stance.
    'blade_draw': ['12E6'],
    'blade_idle': ['124B'],
    'blade_walk': ['13F5'],
    'blade_jump': ['1251', '364A'],
    'blade_double_jump': ['1252', '364B'],
    'blade_swing_prepare': ['3B52'],
    'blade_swing': ['1257', '125A', '34D2'],
    'blade_swing_recover': ['1259', '125C'],
    'blade_dash': ['1256'],
    'blade_deflect_start': ['3632'],
    'blade_deflect_end': ['3631'],
    'blade_wall_climb': ['363D'],
    'blade_wall_climb_end': ['363E'],
    'blade_end': ['34A2'],
    # Later: the "hello" emote (you: "we can add a key to that later").
    'emote_hello': ['2503'],
}


def full_id(short):
    return short.rjust(12, '0')
