"""A small local page to choose the Deflect block sound by ear, from your two block recordings only (rec7, rec8 of
D:\\GenjiBridgeAssets\\recordings\\clips.txt, 2026-10-09). Your test: the first choice "was restarting the deflect sound".

The page (D:\\GenjiBridgeAssets\\block_picker\\index.html, outside the repository) shows:
- each whole clip, with a picture of its loudness over time;
- every short sound found in it: each loudness peak at least 6 dB over its surroundings, cut from 0.06 s before the
  peak to 0.3 s after (the cut is marked on the loudness picture), as candidates in block_picker\\cuts\\.
You tick the ones that are the block, add notes, and export block_picks.json into D:\\GenjiBridgeAssets; then
tools\\make_sounds.py uses exactly those.

Usage: python tools\\make_block_picker.py   (needs numpy in D:\\GenjiBridgeAssets\\pylib, and Pillow)
"""
import json
import os
import sys
import wave

sys.path.insert(0, r'D:\GenjiBridgeAssets\pylib')
import numpy as np  # noqa: E402
from PIL import Image, ImageDraw  # noqa: E402

REC = r'D:\GenjiBridgeAssets\recordings'
OUT = r'D:\GenjiBridgeAssets\block_picker'
CLIPS = [7, 8]
HOP = 0.01
BEFORE, AFTER = 0.06, 0.30


def load(k):
    with wave.open(os.path.join(REC, f'rec{k}.wav')) as w:
        sr = w.getframerate()
        a = np.frombuffer(w.readframes(w.getnframes()), dtype=np.int16).reshape(-1, w.getnchannels())
    return a, sr


def loudness(mono, sr):
    hop = int(sr * HOP)
    return np.array([20 * np.log10(np.sqrt(np.mean(mono[i:i + hop] ** 2)) + 1e-9) for i in range(0, len(mono) - hop, hop)])


def peaks(db):
    """Times of loudness peaks at least 6 dB over the quietest point in the 0.15 s before them, 0.15 s apart."""
    out = []
    for i in range(1, len(db) - 1):
        if db[i] < -35 or db[i] < db[i - 1] or db[i] < db[i + 1]:
            continue
        before = db[max(0, i - 15):i]
        if len(before) and db[i] - before.min() >= 6 and (not out or i * HOP - out[-1] >= 0.15):
            out.append(round(i * HOP, 2))
    return out


def save_wav(path, seg, sr, channels):
    seg = seg.astype(np.float32)
    fade = min(int(0.005 * sr), len(seg) // 2)
    if fade:
        ramp = np.linspace(0, 1, fade)[:, None]
        seg[:fade] *= ramp
        seg[-fade:] *= ramp[::-1]
    with wave.open(path, 'wb') as w:
        w.setnchannels(channels)
        w.setsampwidth(2)
        w.setframerate(sr)
        w.writeframes(seg.astype(np.int16).tobytes())


def picture(path, db, marks, width=900, height=120):
    img = Image.new('RGB', (width, height), (24, 26, 32))
    d = ImageDraw.Draw(img)
    n = len(db)
    for t0, t1, label in marks:
        x0, x1 = int(t0 / (n * HOP) * width), int(t1 / (n * HOP) * width)
        d.rectangle([x0, 0, x1, height], fill=(40, 70, 90))
        d.text((x0 + 2, 2), label, fill=(160, 220, 255))
    pts = [(int(i / n * width), int(height - (min(0, max(-60, v)) + 60) / 60 * (height - 10))) for i, v in enumerate(db)]
    d.line(pts, fill=(240, 200, 90), width=2)
    for s in range(0, int(n * HOP) + 1):
        x = int(s / (n * HOP) * width)
        d.line([x, height - 8, x, height], fill=(200, 200, 200))
        d.text((x + 2, height - 18), f'{s}s', fill=(200, 200, 200))
    img.save(path)


def main():
    os.makedirs(os.path.join(OUT, 'cuts'), exist_ok=True)
    clips = []
    for k in CLIPS:
        audio, sr = load(k)
        mono = audio.astype(np.float32).mean(axis=1) / 32768
        db = loudness(mono, sr)
        dur = len(mono) / sr
        cands = []
        for t in peaks(db):
            start, end = max(0.0, t - BEFORE), min(dur, t + AFTER)
            name = f'rec{k}_{t:.2f}'
            save_wav(os.path.join(OUT, 'cuts', name + '.wav'), audio[int(start * sr):int(end * sr)], sr, audio.shape[1])
            cands.append({'name': name, 'file': f'cuts/{name}.wav', 'peak': t, 'start': round(start, 2), 'end': round(end, 2),
                          'db': round(float(db[int(round(t / HOP))]), 1)})
        picture(os.path.join(OUT, f'rec{k}.png'), db, [(c['start'], c['end'], f"{c['peak']:.2f}") for c in cands])
        clips.append({'rec': k, 'whole': f'../recordings/rec{k}.wav', 'picture': f'rec{k}.png', 'seconds': round(dur, 2), 'cands': cands})
        print(f'rec{k}: {len(cands)} candidates')
    html = PAGE.replace('/*DATA*/', json.dumps(clips))
    with open(os.path.join(OUT, 'index.html'), 'w', encoding='utf-8') as fh:
        fh.write(html)
    print('->', os.path.join(OUT, 'index.html'))


PAGE = r'''<!doctype html>
<html><head><meta charset="utf-8"><title>Deflect block picker</title>
<style>
body { background: #15171c; color: #dde1e8; font: 14px system-ui, sans-serif; margin: 16px; max-width: 980px; }
h2 { font-size: 17px; margin: 22px 0 6px; }
img { display: block; border-radius: 4px; margin: 6px 0; max-width: 100%; }
.c { display: flex; gap: 10px; align-items: center; padding: 6px 8px; margin: 4px 0; background: #1f2229; border-radius: 6px; flex-wrap: wrap; }
.c.on { outline: 2px solid #3fb6a8; }
.c b { width: 110px; }
input[type=text] { flex: 1; min-width: 160px; background: #2a2e37; color: #dde1e8; border: 1px solid #3a3f4a; border-radius: 4px; padding: 4px; }
button { background: #2a2e37; color: #dde1e8; border: 1px solid #3a3f4a; border-radius: 4px; padding: 6px 12px; cursor: pointer; }
.muted { color: #9aa3b2; font-size: 12px; }
</style></head><body>
<h1 style="font-size:20px">Deflect block: your two recordings</h1>
<div>Listen to each whole clip, then to the short sounds found in it (the blue areas on the loudness picture, labelled with their time). Tick the ones that are the block, the clang when a shot hits the sword, without the Deflect start. Notes are kept with your choice. Press Export and save the file as <b>D:\GenjiBridgeAssets\block_picks.json</b>.</div>
<div id="list"></div>
<p><button id="export">Export block_picks.json</button> <span id="count" class="muted"></span></p>
<script>
const DATA = /*DATA*/;
const KEY = 'genji-block-picks';
let picks = {};
try { picks = JSON.parse(localStorage.getItem(KEY) || '{}'); } catch (e) {}
const save = () => { try { localStorage.setItem(KEY, JSON.stringify(picks)); } catch (e) {} count(); };
const count = () => { document.getElementById('count').textContent = Object.values(picks).filter(p => p.use).length + ' ticked'; };
const list = document.getElementById('list');
for (const clip of DATA) {
  const h = document.createElement('h2');
  h.textContent = 'Clip rec' + clip.rec + ' (' + clip.seconds + ' s)';
  list.appendChild(h);
  const whole = document.createElement('div');
  whole.innerHTML = '<audio controls preload="none" src="' + clip.whole + '"></audio><img src="' + clip.picture + '">';
  list.appendChild(whole);
  for (const c of clip.cands) {
    const p = picks[c.name] || { use: false, note: '' };
    const row = document.createElement('div');
    row.className = 'c' + (p.use ? ' on' : '');
    row.innerHTML = '<b>at ' + c.peak.toFixed(2) + ' s</b><audio controls preload="none" src="' + c.file + '"></audio>' +
      '<label><input type="checkbox"' + (p.use ? ' checked' : '') + '> block</label>' +
      '<input type="text" placeholder="notes" value="' + (p.note || '').replace(/"/g, '&quot;') + '">' +
      '<span class="muted">' + c.start.toFixed(2) + '-' + c.end.toFixed(2) + ' s, peak ' + c.db + ' dB</span>';
    row.querySelector('input[type=checkbox]').onchange = e => { picks[c.name] = { ...(picks[c.name] || { note: '' }), use: e.target.checked, file: c.file }; row.className = 'c' + (e.target.checked ? ' on' : ''); save(); };
    row.querySelector('input[type=text]').onchange = e => { picks[c.name] = { ...(picks[c.name] || { use: false }), note: e.target.value, file: c.file }; save(); };
    list.appendChild(row);
  }
}
count();
document.getElementById('export').onclick = () => {
  const blob = new Blob([JSON.stringify(picks, null, 1)], { type: 'application/json' });
  const a = document.createElement('a');
  a.href = URL.createObjectURL(blob);
  a.download = 'block_picks.json';
  a.click();
};
</script></body></html>
'''

if __name__ == '__main__':
    main()
