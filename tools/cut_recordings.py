"""Step 11a, recorded sounds (DESIGN.md 5j): Overwatch plays some of Genji's sounds from sound banks that DataTool
doesn't extract (Swift Strike, the Deflect block, ...), so you recorded them in the Practice Range. This cuts your
clips into one short file per sound, for the sound picker.

For each clip (in D:\\GenjiBridgeAssets\\recordings\\clips.txt, one path a line, in recording order: rec1, rec2, ...):
the audio is extracted (CapCut's ffmpeg), the start of every sound is found from the loudness (a rise of 10 dB within
30 ms, above -45 dB), and each sound is cut from just before its start to the next start (at most 2.5 s) into
D:\\GenjiBridgeAssets\\recordings\\cuts\\rec<k>_<seconds>.wav, with a frame of the video at that moment. Guesses of
what each cut is (from the video, 2026-10-09) go into cuts.json; you correct them in the picker by ear.

Usage: python tools\\cut_recordings.py   (needs numpy, in D:\\GenjiBridgeAssets\\pylib)
"""
import json
import os
import subprocess
import sys
import wave

sys.path.insert(0, r'D:\GenjiBridgeAssets\pylib')
import numpy as np  # noqa: E402

FFMPEG = os.path.expandvars(r'%LOCALAPPDATA%\CapCut\Apps\8.1.1.3417\ffmpeg.exe')
R = r'D:\GenjiBridgeAssets\recordings'
CUTS = os.path.join(R, 'cuts')
MAX_CUT = 2.5
# Guesses from the video (rec number, from-second, to-second, tag): what you were doing at that moment.
GUESSES = [
    (5, 1.0, 1.3, 'ult_ready_sfx'),        # the ultimate turns ready (bottom centre lights up)
    (4, 0.7, 1.0, 'deflect_start'),        # sword up, 2 s of Deflect
    (4, 4.6, 4.9, 'deflect_start'),
    (4, 2.9, 3.2, 'deflect_end'),          # sword down
    (4, 6.8, 7.0, 'deflect_end'),
    (4, 3.5, 3.8, 'cooldown_ready'),       # the E tile's ready flash
    (4, 7.4, 7.6, 'cooldown_ready'),
    (3, 3.8, 4.0, 'cooldown_ready'),       # the LSHIFT tile's ready flash
    (3, 2.8, 3.0, 'dash'),                 # Swift Strike (green trail)
    (2, 0.0, 99, 'footstep'),              # walking around the empty room
    (1, 1.9, 3.7, 'deflect_block'),        # bots shooting into Deflect
    (1, 12.2, 14.4, 'deflect_block'),
]
# Cut by hand (rec number, start, end in seconds, name): your Deflect-block clips of 2026-10-09 (rec7, rec8; "proper
# deflect blocking sound"). rec7: the bots' shots hit the sword at about 0.2, 0.56, 0.8 and 1.2 s, each a short
# swish-clang about 13 dB over Deflect's hum (the video shows the sword swinging at each). rec8: only its first block
# is clean; an explosion covers the rest from 0.5 s.
MANUAL_CUTS = [
    (7, 0.12, 0.50, 'rec7_block_0.12'),
    (7, 0.52, 0.78, 'rec7_block_0.52'),
    (7, 0.76, 1.06, 'rec7_block_0.76'),
    (7, 1.18, 1.50, 'rec7_block_1.18'),
    (8, 0.00, 0.34, 'rec8_block_0.00'),
]


def load(path):
    with wave.open(path) as w:
        n, sr = w.getnframes(), w.getframerate()
        a = np.frombuffer(w.readframes(n), dtype=np.int16).reshape(-1, w.getnchannels())
    return a, sr


def onsets(mono, sr, rise=10.0, floor=-45.0):
    hop = int(sr * 0.01)
    rms = np.array([np.sqrt(np.mean(mono[i:i + hop] ** 2) + 1e-12) for i in range(0, len(mono) - hop, hop)])
    db = 20 * np.log10(rms)
    out, last = [], -1.0
    for i in range(3, len(db)):
        t = i * 0.01
        if db[i] > floor and db[i] - db[i - 3] > rise and t - last > 0.15:
            out.append(round(t, 2))
            last = t
    return out


def main():
    clips = [l.strip() for l in open(os.path.join(R, 'clips.txt'), encoding='utf-8') if l.strip()]
    os.makedirs(CUTS, exist_ok=True)
    cuts = []
    for k, clip in enumerate(clips, 1):
        wav = os.path.join(R, f'rec{k}.wav')
        if not os.path.isfile(wav):
            subprocess.run([FFMPEG, '-v', 'error', '-y', '-i', clip, '-vn', '-ac', '2', '-ar', '48000', wav], check=True)
        audio, sr = load(wav)
        mono = audio.astype(np.float32).mean(axis=1) / 32768
        dur = len(mono) / sr
        ons = onsets(mono, sr)
        for i, t in enumerate(ons):
            start = max(0.0, t - 0.03)
            end = min(ons[i + 1] - 0.02 if i + 1 < len(ons) else dur, t + MAX_CUT)
            name = f'rec{k}_{t:05.2f}'
            path = os.path.join(CUTS, name + '.wav')
            seg = audio[int(start * sr):int(end * sr)].astype(np.float32)
            # 5 ms fades so the cut doesn't click.
            fade = min(int(0.005 * sr), len(seg) // 2)
            if fade:
                ramp = np.linspace(0, 1, fade)[:, None]
                seg[:fade] *= ramp
                seg[-fade:] *= ramp[::-1]
            with wave.open(path, 'wb') as w:
                w.setnchannels(audio.shape[1])
                w.setsampwidth(2)
                w.setframerate(sr)
                w.writeframes(seg.astype(np.int16).tobytes())
            # A frame of the video (kept from earlier runs: older clips may have been moved since; their audio is
            # kept in rec<k>.wav).
            thumb = os.path.join(CUTS, name + '.jpg')
            if not os.path.isfile(thumb) and os.path.isfile(clip):
                subprocess.run([FFMPEG, '-v', 'error', '-y', '-ss', f'{t + 0.1:.2f}', '-i', clip, '-frames:v', '1',
                                '-vf', 'scale=344:144', thumb], check=True)
            guess = next((g[3] for g in GUESSES if g[0] == k and g[1] <= t <= g[2]), '')
            cuts.append({'name': name, 'rec': k, 'at': t, 'secs': round(end - start, 2), 'guess': guess})
        print(f'rec{k}: {len(ons)} sounds')
    for k, start, end, name in MANUAL_CUTS:
        audio, sr = load(os.path.join(R, f'rec{k}.wav'))
        seg = audio[int(start * sr):int(end * sr)].astype(np.float32)
        fade = min(int(0.005 * sr), len(seg) // 2)
        ramp = np.linspace(0, 1, fade)[:, None]
        seg[:fade] *= ramp
        seg[-fade:] *= ramp[::-1]
        with wave.open(os.path.join(CUTS, name + '.wav'), 'wb') as w:
            w.setnchannels(audio.shape[1])
            w.setsampwidth(2)
            w.setframerate(sr)
            w.writeframes(seg.astype(np.int16).tobytes())
        print('cut by hand:', name)
    with open(os.path.join(R, 'cuts.json'), 'w', encoding='utf-8') as fh:
        json.dump(cuts, fh, indent=1)
    print(len(cuts), 'cuts ->', CUTS)


if __name__ == '__main__':
    main()
