"""Builds the sound picker (step 11a, DESIGN.md 5j): a local page that lists Genji's Overwatch sounds so you can find
which is which by ear and tag them. Output: D:\\GenjiBridgeAssets\\sound_picker\\index.html (outside the repository).

Overwatch's sounds come unnamed. Each `.02C` folder is one sound event (its files are takes of the same sound), found
under the animations and effects that play it; the picker lists every event once, with where it appears. Voice lines
are listed per file, by DataTool's situation folders.

Usage: python tools\\make_sound_picker.py   then open the page in your browser.
"""
import json
import os
from collections import OrderedDict

RAW = r'D:\GenjiBridgeAssets\raw'
SKIN = os.path.join(RAW, r'Heroes\Genji\Skin\Overwatch\Overwatch')
VOICE = os.path.join(RAW, r'HeroVoice\Genji')
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
                 'Health', 'HealthPack', 'Healing', 'Respawn', 'KillStreak', 'Multikill', 'Scream', 'ScreamFall',
                 'Stagger', 'Knockdown', 'OnFire', 'Thanks', 'Hello', 'Laugh']
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
               'voice_elimination', 'voice_healing', 'voice_jump', 'voice_respawn']),
])


def rel(path):
    return os.path.relpath(path, OUT).replace('\\', '/')


def main():
    os.makedirs(OUT, exist_ok=True)
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
            e = events.setdefault(event, {'files': [], 'seen': []})
            if where not in e['seen']:
                e['seen'].append(where)
            for f in oggs:
                if f in seen_files:
                    continue
                seen_files.add(f)
                e['files'].append(rel(os.path.join(dirpath, f)))
    events = OrderedDict((k, v) for k, v in events.items() if v['files'])
    voice = OrderedDict()
    for folder in VOICE_FOLDERS:
        d = os.path.join(VOICE, folder)
        if not os.path.isdir(d):
            continue
        files = []
        for dirpath, _, fs in os.walk(d):
            files += [rel(os.path.join(dirpath, f)) for f in sorted(fs) if f.lower().endswith('.ogg')]
        if files:
            voice[folder] = files
    data = {
        'events': [{'id': k, 'files': v['files'], 'seen': v['seen']} for k, v in events.items()],
        'voice': [{'folder': k, 'files': v} for k, v in voice.items()],
        'tags': TAGS,
    }
    html = PAGE.replace('/*DATA*/', json.dumps(data))
    with open(os.path.join(OUT, 'index.html'), 'w', encoding='utf-8') as fh:
        fh.write(html)
    print(f'{len(events)} sound events ({sum(len(v["files"]) for v in events.values())} files), '
          f'{sum(len(v) for v in voice.values())} voice lines in {len(voice)} folders -> {OUT}\\index.html')


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
  display:grid; grid-template-columns: 210px 1fr 230px; gap:10px; align-items:center; }
.card.tagged { background:var(--tagged); }
.meta b { display:block; }
.meta span { color:var(--dim); font-size:12px; }
.takes { display:flex; flex-wrap:wrap; gap:4px; }
.takes button { padding:3px 8px; font-size:12px; }
.takes button.playing { border-color:var(--accent); color:var(--accent); }
.voice-row { display:grid; grid-template-columns: 60px 1fr 230px; gap:10px; align-items:center; padding:4px 10px;
  border-bottom:1px solid var(--line); }
.voice-row.tagged { background:var(--tagged); }
.count { color:var(--dim); }
textarea { width:100%; height:140px; background:#0c0e13; color:var(--text); border:1px solid var(--line); border-radius:6px; }
@media (max-width: 760px) { .card, .voice-row { grid-template-columns: 1fr; } }
</style></head><body>
<header>
  <h1>Genji Sound Picker</h1>
  <p class="help">Click a take to hear it (the volume follows your browser). When a sound is one of Genji's, choose its
  event on the right. Ability sounds are tagged per event (all takes together), voice lines one by one. Your tags are
  remembered in this browser. When you're done, press <b>Export</b> and save the file as
  <b>D:\GenjiBridgeAssets\sound_picks.json</b> (or copy the text and paste it to me).</p>
  <div class="bar">
    <button id="stop">Stop</button>
    <label>Show <select id="filter"><option value="all">everything</option><option value="untagged">untagged only</option>
      <option value="tagged">tagged only</option></select></label>
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
const save = () => { try { localStorage.setItem(KEY, JSON.stringify(picks)); } catch (e) {} update(); };
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
section('Ability and movement sounds (' + DATA.events.length + ' events; first-person first)');
for (const ev of DATA.events) {
  const card = document.createElement('div'); card.className = 'card';
  const meta = document.createElement('div'); meta.className = 'meta';
  meta.innerHTML = `<b>${ev.id.replace('.02C', '')}</b><span>${ev.files.length} take(s) - ${ev.seen.join(', ')}</span>`;
  const takes = document.createElement('div'); takes.className = 'takes';
  ev.files.forEach((f, i) => { const b = document.createElement('button'); b.textContent = '\u25B6 ' + (i + 1);
    b.title = f; b.onclick = () => play(f, b); takes.appendChild(b); });
  const key = 'event:' + ev.id;
  const mark = () => card.classList.toggle('tagged', !!picks[key]);
  card.append(meta, takes, tagSelect(key, mark)); mark();
  main.appendChild(card); rows.push({ el: card, key, text: ev.id + ' ' + ev.seen.join(' ') });
}
section('Voice lines (by situation)');
for (const vf of DATA.voice) {
  const h = document.createElement('h2'); h.style.color = '#ffd36b'; h.textContent = vf.folder + ' (' + vf.files.length + ')'; main.appendChild(h);
  vf.files.forEach((f, i) => {
    const row = document.createElement('div'); row.className = 'voice-row';
    const b = document.createElement('button'); b.textContent = '\u25B6 ' + (i + 1); b.onclick = () => play(f, b);
    const name = document.createElement('span'); name.className = 'count'; name.textContent = f.split('/').pop();
    const key = 'file:' + f;
    const mark = () => row.classList.toggle('tagged', !!picks[key]);
    row.append(b, name, tagSelect(key, mark)); mark();
    main.appendChild(row); rows.push({ el: row, key, text: vf.folder + ' ' + f });
  });
}
function update() {
  const filter = document.getElementById('filter').value, find = document.getElementById('find').value.toLowerCase();
  let shown = 0;
  for (const r of rows) {
    const ok = (filter === 'all' || (filter === 'tagged') === !!picks[r.key]) && (!find || r.text.toLowerCase().includes(find));
    r.el.style.display = ok ? '' : 'none'; if (ok) shown++;
  }
  document.getElementById('count').textContent = Object.keys(picks).length + ' tagged, ' + shown + ' shown';
}
document.getElementById('filter').onchange = update;
document.getElementById('find').oninput = update;
document.getElementById('stop').onclick = () => { if (audio) audio.pause(); };
document.getElementById('export').onclick = () => {
  const out = {};
  for (const ev of DATA.events) { const t = picks['event:' + ev.id]; if (t) (out[t] = out[t] || []).push(...ev.files); }
  for (const [k, t] of Object.entries(picks)) if (k.startsWith('file:')) (out[t] = out[t] || []).push(k.slice(5));
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
