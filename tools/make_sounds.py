"""Step 11a, last part (DESIGN.md 5j): copies the sounds you picked into D:\\GenjiBridgeAssets\\sound\\<event>\\NN.ogg,
the folder the mod plays from. Outside the repository.

Inputs:
- D:\\GenjiBridgeAssets\\sound_picks.json: your export from the sound picker (tags, and notes per sound group).
- D:\\GenjiBridgeAssets\\sound_picker\\index.html: the picker itself, for the take numbers exactly as you saw them.
- Your recorded sounds (tools\\cut_recordings.py): copied as .wav under whatever tag you gave them.

Your notes (2026-10-09) were read by hand into ABILITY below. A note of only numbers ("1 3 4 5 7 8", "3-7") lists the
right takes (the others belong to other skins); a note describing parts ("1-3, 8, 9 melee hit / 4-7 and 10 melee swing")
splits one group into several events. Voice lines follow their tags, minus the ones you marked "dont use", plus the
situation folders you named in notes (kill streak, multikill, ...).

Usage: python tools\\make_sounds.py
"""
import json
import os
import shutil
from collections import OrderedDict

ASSETS = r'D:\GenjiBridgeAssets'
PICKS = os.path.join(ASSETS, 'sound_picks.json')
PICKER = os.path.join(ASSETS, 'sound_picker', 'index.html')
OUT = os.path.join(ASSETS, 'sound')

# event -> [(sound group id, takes (1-based, as numbered in the picker) or None for all)]
ABILITY = OrderedDict([
    ('shuriken_throw', [('5DAA', [1, 3, 4, 5, 7, 8])]),
    ('fan_throw', [('5DAB', [2, 8, 9, 10, 11])]),
    ('shuriken_hit_wall', [('2D8D', [1, 2, 9, 19])]),
    # "1 is body shot hit, 2 is head shot hit"; "this one is for critical shots".
    ('hit_body', [('1610', [1])]),
    ('hit_head', [('1610', [2])]),
    ('hit_critical', [('346F', None)]),
    # The reload animation (1P 2436, 1.5 s) plays these four, one after another.
    ('reload_1', [('91D6', None)]),
    ('reload_2', [('91D7', None)]),
    ('reload_3', [('91D8', None)]),
    ('reload_4', [('91D9', None)]),
    # 0829 (1P 2434, the quick-melee animation) tagged melee hit. 2D10 ("1-3, 8, 9 melee hit / 4-7 and 10 is melee
    # swing") was here too, but it plays only under the four Deflect-block animations you named (24A2, 24A3, 344C, 344D,
    # animation picker 2026-10-09): it is most likely Deflect's block sound. Left out until your own recording of the
    # block ("for now, skip the deflect block sound"); the quick melee has no swing sound of its own for now.
    ('melee_hit', [('0829', None)]),
    # "sounds like deflect start"; "12 could also be that".
    ('deflect_start', [('2D18', None), ('FF72', [12])]),
    ('deflect_end', [('FF84', None)]),
    # "2 is important: the principal roar when you release the blade, it starts right with the ult line";
    # "when he swings the blade to unsheathe".
    ('blade_unsheathe', [('2D38', [2])]),
    ('blade_draw', [('2D37', None), ('2D38', [1])]),
    # 5DA6 tagged blade swing; in 5DA7 "2 6 7 are actually blade swings", "1 3 5 8 9 are roars that come with each
    # swing", "4 is a heart beat"; 1945 "the roars from each blade swing".
    ('blade_swing', [('5DA6', None), ('5DA7', [2, 6, 7])]),
    ('blade_roar', [('5DA7', [1, 3, 5, 8, 9]), ('1945', None)]),
    ('blade_heartbeat', [('5DA7', [4])]),
    # "it starts when the blade is about to end: the ending roar"; "1 blade ending, 2 is sheathing the blade";
    # 2D3A "same thing as the number one of 2D36".
    ('blade_end_roar', [('401C', None)]),
    ('blade_end', [('2D36', [1]), ('2D3A', None)]),
    ('blade_sheathe', [('2D36', [2])]),
    # "from 1 to 13: sound when he is actually wall climbing"; 930F "wall climbing and falling"; 16CF "actually, this
    # is the loop of wall climbing" (2026-10-09, second export); "when he is about to finish climbing to a ledge".
    # Split for playback (step 11b-2): one climb sound at the start, then the short 16CF grabs repeated while climbing.
    ('wall_climb', [('16DC', list(range(1, 14))), ('930F', None)]),
    ('wall_climb_loop', [('16CF', None)]),
    ('wall_climb_ledge', [('16D2', None)]),
    ('double_jump', [('2D98', None)]),
    # "falling sound when he reaches his foot on land".
    ('land', [('3473', None)]),
    # Named in notes, not tagged: "the elimination sound, we could add this when killing an enemy"; "sound when you
    # try to use a skill that is in cooldown"; "sound when you get at critical health"; "1 is healing sound";
    # "healing pack sound"; "when he gets stunned and falls into the ground".
    ('kill', [('60A9', None)]),
    ('not_ready', [('0FCB', None)]),
    ('low_health', [('1B32', None)]),
    ('healing', [('388B', [1])]),
    ('health_pack', [('F511', None)]),
    ('knockdown', [('92E8', None)]),
])
# Voice: situation folder -> event, for lines you named in notes without a tag.
VOICE_FOLDERS = {
    'MeleeEnemyKilledRemark': 'voice_melee_kill',
    'KillStreak': 'voice_killstreak',
    'Multikill': 'voice_multikill',
    'Scream': 'voice_fall_death',
    'Stagger': 'voice_stagger',
    'Knockdown': 'voice_knockdown',
    'JumpEnd': 'voice_jump_end',
}
# Left out on purpose: "lets just not use the deflect actual block sound, it has too much background noise" (you,
# 2026-10-09; that was rec1). Now replaced by RECORDED below.
DROP = set()
# Recordings tagged here rather than in the picker: your clean Deflect-block clips (rec7, rec8, 2026-10-09), cut by hand
# in tools\cut_recordings.py (MANUAL_CUTS). They replace any other sounds for the event.
RECORDED = {
    'deflect_block': ['../recordings/cuts/rec7_block_0.12.wav', '../recordings/cuts/rec7_block_0.52.wav',
                      '../recordings/cuts/rec7_block_0.76.wav', '../recordings/cuts/rec7_block_1.18.wav',
                      '../recordings/cuts/rec8_block_0.00.wav'],
}
# Events the mod plays that you haven't found yet (reported, not an error).
WANTED = ['melee_swing', 'dash', 'dash_hit', 'dash_reset', 'deflect_block', 'blade_hit', 'jump', 'ult_ready_sfx', 'footstep']


def picker_data():
    s = open(PICKER, encoding='utf-8').read()
    return json.loads(s[s.index('const DATA = ') + 13:s.index(';\nconst KEY')])


def main():
    data = picker_data()
    picks = json.load(open(PICKS, encoding='utf-8'))
    notes = picks.pop('_notes', {})
    groups = {e['id'][-8:-4]: e['files'] for e in data['events']}  # "00000000930F.02C" -> "930F"
    picker_dir = os.path.dirname(PICKER)
    out = OrderedDict()
    for event, parts in ABILITY.items():
        files = []
        for gid, takes in parts:
            group = groups.get(gid.rjust(4, '0'))
            if group is None:
                print(f'  ! {event}: sound group {gid} not found in the picker')
                continue
            chosen = range(1, len(group) + 1) if takes is None else takes
            files += [group[t - 1] for t in chosen if 1 <= t <= len(group)]
        out[event] = files
    # Your recorded sounds (tools/cut_recordings.py), whatever their tag. Where you recorded an event, your recording
    # replaces the sounds from Overwatch's files for it (Deflect start and end: the recorded start holds the whole
    # 2 s hum; mixing both sources at random would sound uneven).
    for tag, files in picks.items():
        recorded = [f for f in files if '/recordings/cuts/' in f]
        if recorded:
            out[tag] = recorded
    out.update(RECORDED)
    # Your choice in the Deflect-block picker (tools/make_block_picker.py), once exported: exactly the ticked sounds.
    block_picks = os.path.join(ASSETS, 'block_picks.json')
    if os.path.isfile(block_picks):
        chosen = [p['file'] for p in json.load(open(block_picks, encoding='utf-8')).values() if p.get('use')]
        if chosen:
            out['deflect_block'] = ['../block_picker/' + f for f in chosen]
            print(f'deflect_block: {len(chosen)} sounds from your block picker')
    for tag in DROP:
        out.pop(tag, None)
    # Voice: your tags, minus "dont use"; folders you named in notes.
    skip = {k[5:] for k, n in notes.items() if k.startswith('file:') and 'dont use' in n['note'].lower()}
    for tag, files in picks.items():
        if not tag.startswith('voice_'):
            continue
        for f in files:
            if f in skip:
                continue
            folder = f.split('/HeroVoice/Genji/', 1)[-1].split('/', 1)[0]
            event = VOICE_FOLDERS.get(folder, tag)
            out.setdefault(event, []).append(f)
    for key, n in notes.items():
        if key.startswith('file:') and not n['tag']:
            f = key[5:]
            folder = f.split('/HeroVoice/Genji/', 1)[-1].split('/', 1)[0]
            if folder in VOICE_FOLDERS:
                out.setdefault(VOICE_FOLDERS[folder], []).append(f)
    # Copy.
    if os.path.isdir(OUT):
        shutil.rmtree(OUT)
    index = OrderedDict()
    for event, files in out.items():
        files = list(OrderedDict.fromkeys(files))
        if not files:
            continue
        os.makedirs(os.path.join(OUT, event), exist_ok=True)
        index[event] = []
        for i, f in enumerate(files, 1):
            src = os.path.normpath(os.path.join(picker_dir, f))
            ext = os.path.splitext(src)[1].lower()  # .ogg from Overwatch's files, .wav from your recordings
            dst = os.path.join(OUT, event, f'{i:02d}{ext}')
            shutil.copyfile(src, dst)
            index[event].append({'file': f'{i:02d}{ext}', 'from': os.path.relpath(src, ASSETS)})
        print(f'{event:20s} {len(files):3d}')
    with open(os.path.join(OUT, 'index.json'), 'w', encoding='utf-8') as fh:
        json.dump(index, fh, indent=1, ensure_ascii=False)
    missing = [w for w in WANTED if w not in index and w not in DROP]
    print(f'{len(index)} events, {sum(len(v) for v in index.values())} files -> {OUT}')
    print('not found yet:', ', '.join(missing) if missing else 'none')


if __name__ == '__main__':
    main()
