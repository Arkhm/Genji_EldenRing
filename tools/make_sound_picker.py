"""Builds the sound picker (step 11a, DESIGN.md 5j): a local page that lists Genji's Overwatch sounds so you can find
which is which by ear and tag them. Output: D:\\GenjiBridgeAssets\\sound_picker\\index.html (outside the repository).

Overwatch's sounds come unnamed. Each `.02C` folder is one sound event (its files are takes of the same sound), found
under the animations and effects that play it; the picker lists every event once, with where it appears and how long
each take is. Voice lines come from a second DataTool extraction with subtitles (2026-10-09:
`extract-hero-voice ... --subtitles-with-sounds` into D:\\GenjiBridgeAssets\\voice_subs), so each file name carries its
words; the obvious ones are tagged in advance (you can change them).

Usage: python tools\\make_sound_picker.py   then open the page in your browser.
"""
import json
import os
import struct
from collections import OrderedDict

RAW = r'D:\GenjiBridgeAssets\raw'
SKIN = os.path.join(RAW, r'Heroes\Genji\Skin\Overwatch\Overwatch')
VOICE = r'D:\GenjiBridgeAssets\voice_subs\HeroVoice\Genji'
OUT = r'D:\GenjiBridgeAssets\sound_picker'
# The first-person model's animations first (what you hear as Genji), then third person, then effects.
SOURCES = [
    ('1P', os.path.join(SKIN, r'Models\00000000DBA6.00C\AnimationEffects')),
    ('3P', os.path.join(SKIN, r'Models\00000000DB92.00C\AnimationEffects')),
    ('Model 29DC', os.path.join(SKIN, r'Models\0000000029DC.00C\AnimationEffects')),
    ('Effect', os.path.join(SKIN, 'Effects')),
    ('Effect', os.path.join(SKIN, r'Entities\Gameplay3P\Effects')),
    ('Other', SKIN),
]
VOICE_FOLDERS = ['Ultimate', 'Pain', 'Death', 'Elimination', 'Melee', 'MeleeEnemyKilledRemark', 'Jumping', 'JumpEnd',
                 'HealthPack', 'Healing', 'Respawn', 'KillStreak', 'Multikill', 'Scream', 'ScreamFall', 'Stagger',
                 'Knockdown', 'OnFire', 'Unknown']
# From `Unknown`, only the effort sounds (it also holds hundreds of unrelated lines).
UNKNOWN_KEEP = ('(Swift Strike)', 'Swift strike!', '(exertion)', '(jumping sound)', '(grunts)', '(inhale)', '(exhale)')
# Tags given in advance, by (folder path start, words in the file name): the first match wins.
VOICE_DEFAULTS = [
    ('Ultimate/Ready', '', 'voice_ult_ready'),
    ('Ultimate/Charging', '', 'voice_ult_charging'),
    ('Ultimate/AlmostReady', '', 'voice_ult_charging'),
    ('Ultimate', 'blade of the dragon god', 'voice_ult_activate'),
    ('Pain', '', 'voice_pain'),
    ('Death', '', 'voice_death'),
    ('Elimination', '', 'voice_elimination'),
    ('HealthPack', '', 'voice_healing'),
    ('Respawn', '', 'voice_respawn'),
    ('Jumping', '', 'voice_jump'),
    ('Melee', '(shouts)', 'voice_melee'),
    ('Unknown', 'Swift Strike', 'voice_dash'),
    ('Unknown', 'Swift strike', 'voice_dash'),
]
# The tags (events the mod plays). Keep in step with genji/src/sound.rs.
TAGS = OrderedDict([
    ('Shuriken', ['shuriken_throw', 'fan_throw', 'shuriken_hit_enemy', 'shuriken_hit_wall', 'reload']),
    ('Swift Strike', ['dash', 'dash_hit', 'dash_reset']),
    ('Deflect', ['deflect_start', 'deflect_block', 'deflect_end']),
    ('Dragonblade', ['blade_unsheathe', 'blade_swing', 'blade_hit', 'blade_sheathe']),
    ('Quick melee', ['melee_swing', 'melee_hit']),
    ('Movement', ['jump', 'double_jump', 'wall_climb', 'land', 'footstep']),
    ('Ultimate', ['ult_ready_sfx']),
    ('Voice', ['voice_ult_activate', 'voice_ult_ready', 'voice_ult_charging', 'voice_pain', 'voice_death',
               'voice_elimination', 'voice_healing', 'voice_jump', 'voice_respawn', 'voice_dash', 'voice_melee']),
])


def rel(path):
    return os.path.relpath(path, OUT).replace('\\', '/')


def duration(path):
    """Seconds, from the Ogg Vorbis header (sample rate) and the last page's sample count; None if unreadable."""
    try:
        with open(path, 'rb') as fh:
            data = fh.read()
    except OSError:
        return None
    i = data.find(b'\x01vorbis')
    last = data.rfind(b'OggS')
    if i < 0 or last < 0:
        return None
    rate = int.from_bytes(data[i + 12:i + 16], 'little')
    samples = int.from_bytes(data[last + 6:last + 14], 'little')
    return round(samples / rate, 2) if rate else None


def clip_length(model_dir, anim_id):
    """(seconds, frames) of an animation, or None. Read from its .owanimclip (DataTool's format, owm-formats/
    owanimclip.txt: u16 version x2, u32 bone count, f32 frames per second, u32 frame count, then per bone a name and
    tracks of keyframes); the length is the last keyframe's frame over the frames per second. The AnimationEffects
    folders are named after the animation that plays their sounds."""
    for dirpath, _, files in os.walk(os.path.join(model_dir, 'Animations')):
        name = anim_id + '.owanimclip'
        if name not in files:
            continue
        with open(os.path.join(dirpath, name), 'rb') as fh:
            data = fh.read()
        bones = int.from_bytes(data[4:8], 'little')
        fps = struct.unpack('<f', data[8:12])[0]
        at, last = 16, 0
        try:
            for _ in range(bones):
                # .NET string: 7-bit encoded length, then UTF-8.
                length, shift = 0, 0
                while True:
                    b = data[at]; at += 1
                    length |= (b & 0x7F) << shift; shift += 7
                    if b < 0x80:
                        break
                at += length
                tracks = int.from_bytes(data[at:at + 4], 'little'); at += 4
                for _ in range(tracks):
                    keys = int.from_bytes(data[at + 4:at + 8], 'little')
                    comps = int.from_bytes(data[at + 8:at + 12], 'little')
                    at += 12
                    for _ in range(keys):
                        last = max(last, int.from_bytes(data[at:at + 4], 'little'))
                        at += 4 + 4 * comps
        except IndexError:
            return None
        return (round(last / fps, 2), last) if fps > 0 else None
    return None


def main():
    os.makedirs(OUT, exist_ok=True)
    anims = {}  # "1P 000000002434" -> [seconds, frames]
    events = OrderedDict()  # .02C folder name -> {files: [...], seen: [...]}
    seen_files = set()
    for label, root in SOURCES:
        if not os.path.isdir(root):
            continue
        for dirpath, _, files in os.walk(root):
            oggs = sorted(f for f in files if f.lower().endswith('.ogg'))
            if not oggs or not os.path.basename(dirpath).endswith('.02C'):
                continue
            event = os.path.basename(dirpath)
            parts = os.path.relpath(dirpath, root).split(os.sep)
            where = f'{label} {parts[0].replace(".00D", "")}' if parts and parts[0] not in ('Sounds',) else label
            if root.endswith('AnimationEffects') and where not in anims and parts:
                length = clip_length(os.path.dirname(root), parts[0])
                if length:
                    anims[where] = list(length)
            e = events.setdefault(event, {'files': [], 'seen': []})
            if where not in e['seen']:
                e['seen'].append(where)
            for f in oggs:
                if f in seen_files:
                    continue
                seen_files.add(f)
                e['files'].append(rel(os.path.join(dirpath, f)))
                e.setdefault('secs', []).append(duration(os.path.join(dirpath, f)))
    events = OrderedDict((k, v) for k, v in events.items() if v['files'])
    voice = OrderedDict()
    defaults = {}
    for folder in VOICE_FOLDERS:
        d = os.path.join(VOICE, folder)
        if not os.path.isdir(d):
            continue
        files = []
        for dirpath, _, fs in os.walk(d):
            for f in sorted(fs):
                if not f.lower().endswith('.ogg'):
                    continue
                if folder == 'Unknown' and not any(k in f for k in UNKNOWN_KEEP):
                    continue
                path = rel(os.path.join(dirpath, f))
                sub = os.path.relpath(dirpath, VOICE).replace('\\', '/')
                words = f.split('-', 1)[1][:-4] if '-' in f else f
                files.append({'path': path, 'words': words, 'where': sub})
                for start, needle, tag in VOICE_DEFAULTS:
                    if sub.startswith(start) and needle in f:
                        defaults['file:' + path] = tag
                        break
        if files:
            voice[folder] = files
    data = {
        'events': [{'id': k, 'files': v['files'], 'secs': v.get('secs', []), 'seen': v['seen']} for k, v in events.items()],
        'defaults': defaults,
        'anims': anims,
        'voice': [{'folder': k, 'files': v} for k, v in voice.items()],
        'tags': TAGS,
    }
    html = PAGE.replace('/*DATA*/', json.dumps(data))
    with open(os.path.join(OUT, 'index.html'), 'w', encoding='utf-8') as fh:
        fh.write(html)
    print(f'{len(events)} sound events ({sum(len(v["files"]) for v in events.values())} files), '
          f'{sum(len(v) for v in voice.values())} voice lines in {len(voice)} folders ({len(defaults)} tagged in advance) -> {OUT}\\index.html')


PAGE = r'''<!doctype html>
<html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1">
<title>Genji Sound Picker</title>
<style>
:root { --bg:#16181f; --panel:#1f2330; --line:#2d3344; --text:#e6e8ef; --dim:#9aa1b4; --accent:#57e3ff; --tagged:#1d3b2c; }
* { box-sizing: border-box; }
body { margin:0; background:var(--bg); color:var(--text); font:14px/1.4 system-ui, sans-serif; }
header { position:sticky; top:0; z-index:2; background:#101218; border-bottom:1px solid var(--line); padding:10px 16px; }
h1 { font-size:18px; margin:0 0 6px; }
.help { color:var(--dim); font-size:13px; margin:0 0 8px; max-width:1100px; }
.bar { display:flex; gap:8px; flex-wrap:wrap; align-items:center; }
button, select, input { background:var(--panel); color:var(--text); border:1px solid var(--line); border-radius:6px;
  padding:5px 9px; font:inherit; }
button:hover { border-color:var(--accent); cursor:pointer; }
main { padding:12px 16px 80px; max-width:1200px; }
h2 { font-size:15px; margin:22px 0 8px; color:var(--accent); }
.card { background:var(--panel); border:1px solid var(--line); border-radius:8px; padding:8px 10px; margin:6px 0;
  display:grid; grid-template-columns: 210px 1fr 230px 220px; gap:10px; align-items:center; }
.card.noted:not(.tagged), .voice-row.noted:not(.tagged) { background:#2b2a1c; }
input.note { width:100%; }
.card.tagged { background:var(--tagged); }
.meta b { display:block; }
.meta span { color:var(--dim); font-size:12px; }
.takes { display:flex; flex-wrap:wrap; gap:4px; }
.takes button { padding:3px 8px; font-size:12px; }
.takes button.playing { border-color:var(--accent); color:var(--accent); }
.voice-row { display:grid; grid-template-columns: 60px 1fr 230px 220px; gap:10px; align-items:center; padding:4px 10px;
  border-bottom:1px solid var(--line); }
.voice-row.tagged { background:var(--tagged); }
.count { color:var(--dim); }
textarea { width:100%; height:140px; background:#0c0e13; color:var(--text); border:1px solid var(--line); border-radius:6px; }
@media (max-width: 900px) { .card, .voice-row { grid-template-columns: 1fr; } }
</style></head><body>
<header>
  <h1>Genji Sound Picker</h1>
  <p class="help">Click a take to hear it (the volume follows your browser). When a sound is one of Genji's, choose its
  event on the right, and/or type what it is in the box next to it (anything: "dash, the start", "maybe deflect"...).
  Ability sounds are tagged per event (all takes together), voice lines one by one. Your tags and notes are
  remembered in this browser. When you're done, press <b>Export</b> and save the file as
  <b>D:\GenjiBridgeAssets\sound_picks.json</b> (or copy the text and paste it to me).</p>
  <div class="bar">
    <button id="stop">Stop</button>
    <label>Show <select id="filter"><option value="all">everything</option><option value="untagged">untagged only</option>
      <option value="tagged">tagged or noted only</option></select></label>
    <input id="find" placeholder="find an id...">
    <span class="count" id="count"></span>
    <button id="export">Export</button>
  </div>
</header>
<main id="main"></main>
<script>
const DATA = /*DATA*/;
const KEY = 'genji-sound-picks-v1';
let picks = {};
try { picks = JSON.parse(localStorage.getItem(KEY) || '{}'); } catch (e) { picks = {}; }
// Voice tags given in advance: applied once each (a change of yours is kept).
let applied = {};
try { applied = JSON.parse(localStorage.getItem(KEY + '-defaults') || '{}'); } catch (e) { applied = {}; }
for (const [k, t] of Object.entries(DATA.defaults || {})) if (!applied[k]) { if (!picks[k]) picks[k] = t; applied[k] = 1; }
try { localStorage.setItem(KEY + '-defaults', JSON.stringify(applied)); localStorage.setItem(KEY, JSON.stringify(picks)); } catch (e) {}
const NOTES = 'genji-sound-notes-v1';
let notes = {};
try { notes = JSON.parse(localStorage.getItem(NOTES) || '{}'); } catch (e) { notes = {}; }
const save = () => { try { localStorage.setItem(KEY, JSON.stringify(picks)); localStorage.setItem(NOTES, JSON.stringify(notes)); } catch (e) {} update(); };
// A free text box: what you think the sound is.
function noteBox(key, onchange) {
  const box = document.createElement('input'); box.className = 'note'; box.placeholder = 'what is it? (optional)';
  box.value = notes[key] || '';
  box.oninput = () => { const v = box.value.trim(); if (v) notes[key] = box.value; else delete notes[key]; save(); onchange(); };
  return box;
}
let audio = null, playingBtn = null;
function play(src, btn) {
  if (audio) { audio.pause(); }
  if (playingBtn) playingBtn.classList.remove('playing');
  audio = new Audio(src); audio.play().catch(() => {});
  playingBtn = btn; if (btn) { btn.classList.add('playing'); audio.onended = () => btn.classList.remove('playing'); }
}
function tagSelect(key, onchange) {
  const sel = document.createElement('select');
  sel.innerHTML = '<option value="">(not Genji / skip)</option>' + Object.entries(DATA.tags).map(([g, ts]) =>
    `<optgroup label="${g}">` + ts.map(t => `<option value="${t}">${t.replaceAll('_', ' ')}</option>`).join('') + '</optgroup>').join('');
  sel.value = picks[key] || '';
  sel.onchange = () => { if (sel.value) picks[key] = sel.value; else delete picks[key]; save(); onchange(); };
  return sel;
}
const main = document.getElementById('main');
const rows = [];
function section(title) { const h = document.createElement('h2'); h.textContent = title; main.appendChild(h); }
section('Ability and movement sounds (' + DATA.events.length + ' events), grouped by the animation that plays them');
const intro = document.createElement('p'); intro.className = 'help';
intro.textContent = 'Each group is one animation (one move) of Genji, first person (1P) first, with how long it lasts. ' +
  'A short 1P animation with 3 sounds is likely a throw; a ~1.5 s one with clicks a reload; a 2 s one Deflect, and so on.';
main.appendChild(intro);
let lastGroup = null;
for (const ev of DATA.events) {
  const group = ev.seen[0] || '';
  if (group !== lastGroup) {
    lastGroup = group;
    const g = document.createElement('h2'); g.style.color = '#c9b8ff'; g.style.fontSize = '14px';
    const len = (DATA.anims || {})[group];
    const count = DATA.events.filter(e => (e.seen[0] || '') === group).length;
    g.textContent = (group.startsWith('1P') ? 'First-person animation ' : group.startsWith('3P') ? 'Third-person animation ' : '') +
      group.replace(/^(1P|3P) /, '') + (len ? ' \u00B7 ' + len[0] + ' s (' + len[1] + ' frames)' : '') + ' \u00B7 ' + count + ' sound(s)';
    main.appendChild(g); rows.push({ el: g, key: null, text: group });
  }
  const card = document.createElement('div'); card.className = 'card';
  const meta = document.createElement('div'); meta.className = 'meta';
  meta.innerHTML = `<b>${ev.id.replace('.02C', '')}</b><span>${ev.files.length} take(s) - ${ev.seen.join(', ')}</span>`;
  const takes = document.createElement('div'); takes.className = 'takes';
  ev.files.forEach((f, i) => { const b = document.createElement('button');
    const secs = (ev.secs || [])[i]; b.textContent = '\u25B6 ' + (i + 1) + (secs != null ? ' \u00B7 ' + secs + 's' : '');
    b.title = f; b.onclick = () => play(f, b); takes.appendChild(b); });
  const key = 'event:' + ev.id;
  const mark = () => { card.classList.toggle('tagged', !!picks[key]); card.classList.toggle('noted', !!notes[key]); };
  card.append(meta, takes, tagSelect(key, mark), noteBox(key, mark)); mark();
  main.appendChild(card); rows.push({ el: card, key, text: ev.id + ' ' + ev.seen.join(' ') });
}
section('Voice lines (with their words; the obvious ones are tagged already)');
for (const vf of DATA.voice) {
  const h = document.createElement('h2'); h.style.color = '#ffd36b'; h.textContent = vf.folder + ' (' + vf.files.length + ')'; main.appendChild(h);
  vf.files.forEach((v, i) => {
    const f = v.path;
    const row = document.createElement('div'); row.className = 'voice-row';
    const b = document.createElement('button'); b.textContent = '\u25B6 ' + (i + 1); b.onclick = () => play(f, b);
    const name = document.createElement('span');
    const words = document.createElement('b'); words.textContent = v.words;
    const where = document.createElement('span'); where.className = 'count'; where.textContent = '  ' + v.where;
    name.append(words, where);
    const key = 'file:' + f;
    const mark = () => { row.classList.toggle('tagged', !!picks[key]); row.classList.toggle('noted', !!notes[key]); };
    row.append(b, name, tagSelect(key, mark), noteBox(key, mark)); mark();
    main.appendChild(row); rows.push({ el: row, key, text: vf.folder + ' ' + v.where + ' ' + v.words });
  });
}
function update() {
  const filter = document.getElementById('filter').value, find = document.getElementById('find').value.toLowerCase();
  let shown = 0;
  for (const r of rows) {
    if (r.key === null) { r.el.style.display = ''; continue; }
    const done = !!picks[r.key] || !!notes[r.key];
    const ok = (filter === 'all' || (filter === 'tagged') === done) && (!find || r.text.toLowerCase().includes(find) || (notes[r.key] || '').toLowerCase().includes(find));
    r.el.style.display = ok ? '' : 'none'; if (ok) shown++;
  }
  document.getElementById('count').textContent = Object.keys(picks).length + ' tagged, ' + Object.keys(notes).length + ' noted, ' + shown + ' shown';
}
document.getElementById('filter').onchange = update;
document.getElementById('find').oninput = update;
document.getElementById('stop').onclick = () => { if (audio) audio.pause(); };
document.getElementById('export').onclick = () => {
  const out = {};
  for (const ev of DATA.events) { const t = picks['event:' + ev.id]; if (t) (out[t] = out[t] || []).push(...ev.files); }
  for (const [k, t] of Object.entries(picks)) if (k.startsWith('file:')) (out[t] = out[t] || []).push(k.slice(5));
  // Your notes, with the files they're about (the tools skip this part; I read it).
  const noted = {};
  for (const [k, n] of Object.entries(notes)) {
    const ev = k.startsWith('event:') ? DATA.events.find(e => 'event:' + e.id === k) : null;
    noted[k] = { note: n, tag: picks[k] || '', files: ev ? ev.files : [k.slice(5)] };
  }
  if (Object.keys(noted).length) out._notes = noted;
  const text = JSON.stringify(out, null, 1);
  const a = document.createElement('a'); a.href = URL.createObjectURL(new Blob([text], { type: 'application/json' }));
  a.download = 'sound_picks.json'; a.click();
  let box = document.getElementById('box');
  if (!box) { box = document.createElement('textarea'); box.id = 'box'; document.querySelector('header').appendChild(box); }
  box.value = text; box.select();
};
update();
</script></body></html>
'''

if __name__ == '__main__':
    main()
