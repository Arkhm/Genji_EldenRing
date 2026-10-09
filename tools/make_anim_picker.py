"""Step 10a (DESIGN.md 5k): a local page to name Genji's 129 first-person animations, like the sound picker.

Input: the previews tools\\render_anim_previews.py rendered with `-- --all-1p`, from Genji's eye with your field of
view (D:\\GenjiBridgeAssets\\anim_previews_eye\\1P_<id>), turned into looping GIFs here. Output: D:\\GenjiBridgeAssets\\anim_picker\\index.html (outside the repository). Each card
shows the animation, its length, the folder Overwatch keeps it in, the sounds you tagged on it (sound picker), and my
guess for what it is (GUESSES below); you correct the tag, add notes, and export anim_picks.json.

Usage: python tools\\make_anim_picker.py
"""
import json
import os
import sys

sys.path.insert(0, os.path.dirname(__file__))
from make_sound_picker import clip_length  # noqa: E402
from make_sounds import ABILITY  # noqa: E402

MODEL = r'D:\GenjiBridgeAssets\raw\Heroes\Genji\Skin\Overwatch\Overwatch\Models\00000000DBA6.00C'
PREVIEWS = r'D:\GenjiBridgeAssets\anim_previews_eye'
OUT = r'D:\GenjiBridgeAssets\anim_picker'

# What the mod can play (DESIGN.md 5k, step 10d). "other" and "unused" for the rest.
TAGS = [
    ('Base', ['idle', 'walk', 'jump', 'double_jump', 'fall', 'land', 'wall_climb', 'wall_climb_end']),
    ('Shuriken', ['throw', 'fan', 'reload']),
    ('Swift Strike', ['dash']),
    ('Deflect', ['deflect_start', 'deflect_loop', 'deflect_block', 'deflect_end']),
    ('Dragonblade', ['blade_draw', 'blade_idle', 'blade_walk', 'blade_swing', 'blade_end']),
    ('Quick melee', ['melee']),
    ('Other', ['spawn', 'holster', 'fidget', 'emote', 'layer', 'other', 'unused']),
]
# My guesses from the eye-view previews (2026-10-09), only where the picture and the sounds you tagged agree; you
# correct them on the page.
GUESSES = {
    '000000002478': 'throw',          # 128/32, your shuriken-throw sounds; the hand flicks forward
    '00000000243B': 'fan',            # 2/32, your Fan of Blades sounds
    '000000002436': 'reload',         # your four reload sounds
    '000000002434': 'melee',          # your melee-hit sound; the arm swings across
    '000000003555': 'deflect_end',    # your Deflect-end sound; both hands come down
    '000000003576': 'deflect_loop',   # both hands up in front, held
    '0000000012E6': 'blade_draw',     # from the empty hand to both hands on the hilt, 1.0 s
    '000000001251': 'blade_idle',     # both hands on the hilt over the right shoulder
    '000000001256': 'blade_swing',    # from the hilt across to the right
    '0000000034D2': 'blade_swing',    # 2/36: down and back up to the hilt
    '00000000242E': 'idle',           # the right hand forward, still
    '0000000033F1': 'wall_climb',     # your wall-climb sound; hands reach up
    '0000000033F4': 'wall_climb_end', # your ledge sound
    '0000000083CA': 'spawn',          # the hand comes up from below, 0.2 s
    '0000000083CB': 'holster',        # the hand goes down, 0.13 s
}
# Folders whose animations only move part of the arms (layers added on top of a base pose in Overwatch): shown alone,
# the arms stay out of view.
LAYER_FOLDERS = ('4/',)


def make_gif(anim):
    """A looping GIF of an animation's frames (every 2nd frame of 30: 66 ms each, then a short pause), or None."""
    from PIL import Image
    frames_dir = os.path.join(PREVIEWS, '1P_' + anim)
    if not os.path.isfile(os.path.join(frames_dir, 'done')):
        return None
    gif = frames_dir + '.gif'
    if not os.path.isfile(gif):
        pngs = sorted(f for f in os.listdir(frames_dir) if f.endswith('.png'))
        frames = [Image.open(os.path.join(frames_dir, f)).convert('RGB').quantize(colors=128) for f in pngs]
        if not frames:
            return None
        frames[0].save(gif, save_all=True, append_images=frames[1:], duration=[66] * (len(frames) - 1) + [600], loop=0)
    return gif


def animations():
    found = {}
    root = os.path.join(MODEL, 'Animations')
    for dirpath, _, files in os.walk(root):
        for f in files:
            if f.endswith('.owanimclip'):
                found[f[:-len('.owanimclip')]] = os.path.relpath(dirpath, root).replace('\\', '/')
    return found


def sound_events():
    """Animation id -> the sound events you tagged on its sounds (from tools\\make_sounds.py's table)."""
    by_group = {}
    for event, parts in ABILITY.items():
        for gid, _ in parts:
            by_group.setdefault(gid.rjust(4, '0'), []).append(event)
    out = {}
    root = os.path.join(MODEL, 'AnimationEffects')
    for anim in os.listdir(root) if os.path.isdir(root) else []:
        sounds = os.path.join(root, anim, 'Sounds')
        if os.path.isdir(sounds):
            events = []
            for g in os.listdir(sounds):
                events += by_group.get(g[-8:-4], [])
            out[anim] = sorted(set(events))
    return out


def main():
    os.makedirs(OUT, exist_ok=True)
    sounds = sound_events()
    cards = []
    for anim, folder in sorted(animations().items(), key=lambda kv: (kv[1], kv[0])):
        gif = make_gif(anim)
        length = clip_length(MODEL, anim)
        cards.append({
            'id': anim,
            'folder': folder,
            'gif': os.path.relpath(gif, OUT).replace('\\', '/') if gif else None,
            'seconds': length[0] if length else None,
            'sounds': sounds.get(anim, []),
            'guess': GUESSES.get(anim, ''),
            'layer': folder.startswith(LAYER_FOLDERS),
        })
    html = PAGE.replace('/*DATA*/', json.dumps({'cards': cards, 'tags': TAGS}))
    with open(os.path.join(OUT, 'index.html'), 'w', encoding='utf-8') as fh:
        fh.write(html)
    print(f'{len(cards)} animations, {sum(1 for c in cards if c["gif"])} with previews, {sum(1 for c in cards if c["guess"])} guessed -> {OUT}\\index.html')


PAGE = r'''<!doctype html>
<html><head><meta charset="utf-8"><title>Genji first-person animations</title>
<style>
body { background: #15171c; color: #dde1e8; font: 14px system-ui, sans-serif; margin: 16px; }
h1 { font-size: 20px; margin: 0 0 6px; }
.bar { position: sticky; top: 0; background: #15171c; padding: 8px 0; z-index: 2; display: flex; gap: 12px; align-items: center; flex-wrap: wrap; }
.grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(330px, 1fr)); gap: 12px; }
.card { background: #1f2229; border-radius: 8px; padding: 8px; border: 2px solid transparent; }
.card.tagged { border-color: #3fb6a8; }
.card img { width: 100%; border-radius: 4px; background: #0d0e11; display: block; }
.meta { font-size: 12px; color: #9aa3b2; margin: 6px 0; }
.snd { color: #e8c86a; }
select, textarea, button { background: #2a2e37; color: #dde1e8; border: 1px solid #3a3f4a; border-radius: 4px; font: inherit; }
select { width: 100%; padding: 4px; }
textarea { width: 100%; box-sizing: border-box; height: 34px; margin-top: 4px; padding: 4px; }
button { padding: 6px 12px; cursor: pointer; }
.guess { color: #8fd18f; font-size: 12px; }
</style></head><body>
<h1>Genji's first-person animations</h1>
<div>Pick what each animation is. Green: my guess, already selected. Yellow: the sounds you tagged on it. Your picks are kept in this browser; press Export when done and save the file as <b>D:\GenjiBridgeAssets\anim_picks.json</b>.</div>
<div class="bar">
  <label>Show: <select id="filter" style="width:auto">
    <option value="all">all</option><option value="untagged">not tagged</option><option value="tagged">tagged</option>
  </select></label>
  <span id="count"></span>
  <button id="export">Export anim_picks.json</button>
</div>
<div class="grid" id="grid"></div>
<script>
const DATA = /*DATA*/;
const KEY = 'genji-anim-picks';
let saved = {};
try { saved = JSON.parse(localStorage.getItem(KEY) || '{}'); } catch (e) {}
for (const c of DATA.cards) if (!saved[c.id] && c.guess) saved[c.id] = { tag: c.guess, note: '' };
const save = () => { try { localStorage.setItem(KEY, JSON.stringify(saved)); } catch (e) {} draw(); };
const options = (sel) => '<option value="">(not tagged)</option>' + DATA.tags.map(([g, ts]) =>
  '<optgroup label="' + g + '">' + ts.map(t => '<option' + (t === sel ? ' selected' : '') + '>' + t + '</option>').join('') + '</optgroup>').join('');
function draw() {
  const f = document.getElementById('filter').value;
  const grid = document.getElementById('grid');
  grid.innerHTML = '';
  let n = 0;
  for (const c of DATA.cards) {
    const p = saved[c.id] || { tag: '', note: '' };
    if (p.tag) n++;
    if (f === 'tagged' && !p.tag || f === 'untagged' && p.tag) continue;
    const d = document.createElement('div');
    d.className = 'card' + (p.tag ? ' tagged' : '');
    d.innerHTML = (c.gif ? '<img loading="lazy" src="' + c.gif + '">' : '<div class="meta">(no preview)</div>') +
      '<div class="meta"><b>' + c.id.replace(/^0+/, '') + '</b> &middot; ' + (c.seconds ?? '?') + ' s &middot; folder ' + c.folder +
      (c.sounds.length ? '<br><span class="snd">sounds: ' + c.sounds.join(', ') + '</span>' : '') +
      (c.guess ? '<br><span class="guess">my guess: ' + c.guess + '</span>' : '') +
      (c.layer ? '<br>probably a layer: it moves only part of the arms, so alone they stay out of view (skip it if empty)' : '') + '</div>' +
      '<select>' + options(p.tag) + '</select><textarea placeholder="notes">' + (p.note || '') + '</textarea>';
    d.querySelector('select').onchange = e => { saved[c.id] = { ...(saved[c.id] || { note: '' }), tag: e.target.value }; save(); };
    d.querySelector('textarea').onchange = e => { saved[c.id] = { ...(saved[c.id] || { tag: '' }), note: e.target.value }; save(); };
    grid.appendChild(d);
  }
  document.getElementById('count').textContent = n + ' of ' + DATA.cards.length + ' tagged';
}
document.getElementById('filter').onchange = draw;
document.getElementById('export').onclick = () => {
  const blob = new Blob([JSON.stringify(saved, null, 1)], { type: 'application/json' });
  const a = document.createElement('a');
  a.href = URL.createObjectURL(blob);
  a.download = 'anim_picks.json';
  a.click();
};
draw();
</script></body></html>
'''

if __name__ == '__main__':
    main()
