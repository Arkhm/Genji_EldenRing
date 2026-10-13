"""Prepare authentic timestamped video/audio evidence; no game or mod access.

All media output stays in D:/GenjiBridgeAssets/references. Written reports are authored separately.
Uses the installed CapCut ffmpeg and Pillow; numpy is read from the existing external pylib.
"""
import argparse
import hashlib
import json
import math
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import wave

from PIL import Image, ImageDraw, ImageFont

ROOT = Path('D:/GenjiBridgeAssets/references')
FFMPEG = Path(os.environ['LOCALAPPDATA']) / 'CapCut/Apps/8.1.1.3417/ffmpeg.exe'
SOURCES = {
    'OW-20261013-A': Path('D:/Videos/Edits/MedalTVOverwatch20261013180953506-trim-1791926141896.mp4'),
    'OW-20261013-B': Path('D:/Videos/Edits/MedalTVOverwatch20261013181049319-trim-1791926175787.mp4'),
    'OW-20261013-C': Path('D:/Videos/Clips/Overwatch/MedalTVOverwatch20261013181158955.mp4'),
}


def run(args, log):
    with log.open('w', encoding='utf-8') as out:
        result = subprocess.run([str(FFMPEG), '-hide_banner', '-nostdin', *args], stdout=out, stderr=out)
    if result.returncode:
        raise RuntimeError(f'ffmpeg exit {result.returncode}: {log}')


def frames_from_log(log, folder):
    result = []
    pattern = re.compile(r'\bn:\s*(\d+)\s+pts:\s*(-?\d+)')
    timebase = None
    for line in log.read_text(encoding='utf-8').splitlines():
        config = re.search(r'config in time_base:\s*(\d+)/(\d+)', line)
        if config:
            timebase = int(config[1]) / int(config[2])
        match = pattern.search(line)
        if match:
            if timebase is None:
                raise RuntimeError('No decoded timebase found')
            number, pts = int(match[1]), int(match[2]) * timebase
            result.append({'sequence': number + 1, 'source_pts_seconds': pts,
                           'file': str(folder / f'f{number + 1:05}.jpg')})
    (folder / 'timestamps.json').write_text(json.dumps(result, indent=2), encoding='utf-8')
    return result


def font(size):
    return ImageFont.load_default(size=size)


def sheets(rows, folder, prefix, crop=None, cols=4, count=24):
    cell_w, cell_h = 480, 228
    if crop:
        cell_h = 250
    for offset in range(0, len(rows), count):
        page = rows[offset:offset + count]
        canvas = Image.new('RGB', (cols * cell_w, math.ceil(len(page) / cols) * cell_h), '#10141b')
        draw = ImageDraw.Draw(canvas)
        for k, row in enumerate(page):
            pic = Image.open(row['file']).convert('RGB')
            if crop:
                pic = pic.crop(crop)
            pic.thumbnail((cell_w - 8, cell_h - 29))
            x, y = (k % cols) * cell_w, (k // cols) * cell_h
            canvas.paste(pic, (x + 4, y + 26))
            draw.text((x + 6, y + 3), f"{row['source_pts_seconds']:.4f}s  {Path(row['file']).name}",
                      font=font(16), fill='#fff68f')
        canvas.save(folder / f'{prefix}_{offset // count:02}.jpg', quality=93)


def prepare(clip):
    src = SOURCES[clip]
    dst = ROOT / 'originals' / src.name
    dst.parent.mkdir(parents=True, exist_ok=True)
    with src.open('rb') as source_file:
        digest = hashlib.file_digest(source_file, 'sha256').hexdigest()
    if not dst.exists():
        shutil.copy2(src, dst)
    with dst.open('rb') as copied:
        if hashlib.file_digest(copied, 'sha256').hexdigest() != digest:
            raise RuntimeError('Original copy hash mismatch')
    folder = ROOT / 'frames' / clip / 'survey'
    folder.mkdir(parents=True, exist_ok=True)
    log = folder / 'decode.log'
    run(['-i', str(dst), '-map', '0:v:0', '-an', '-vf',
         "select='isnan(prev_selected_t)+gte(t-prev_selected_t,0.5)',showinfo",
         '-vsync', '0', '-q:v', '3', str(folder / 'f%05d.jpg')], log)
    rows = frames_from_log(log, folder)
    sheets(rows, folder, 'overview')
    audio = ROOT / 'audio' / clip
    audio.mkdir(parents=True, exist_ok=True)
    for track in range(3):
        run(['-i', str(dst), '-map', f'0:a:{track}', '-vn', '-c:a', 'pcm_s16le',
             str(audio / f'track{track}.wav')], audio / f'track{track}.log')
    record = {'clip_id': clip, 'source_path': str(src), 'preserved_path': str(dst),
              'sha256': digest, 'bytes': src.stat().st_size, 'ffmpeg': str(FFMPEG),
              'survey_frames': len(rows), 'survey_time_map': str(folder / 'timestamps.json'),
              'audio': [str(audio / f'track{k}.wav') for k in range(3)]}
    (ROOT / f'{clip}-metadata.json').write_text(json.dumps(record, indent=2), encoding='utf-8')
    print(json.dumps(record))


def detail(clip, start, end, name, step, crop):
    src = ROOT / 'originals' / SOURCES[clip].name
    folder = ROOT / 'frames' / clip / name
    folder.mkdir(parents=True, exist_ok=True)
    selection = f"between(t,{start},{end})"
    if step:
        selection += f"*(isnan(prev_selected_t)+gte(t-prev_selected_t,{step}))"
    log = folder / 'decode.log'
    run(['-i', str(src), '-map', '0:v:0', '-an', '-vf', f"select='{selection}',showinfo",
         '-vsync', '0', '-q:v', '2', str(folder / 'f%05d.jpg')], log)
    rows = frames_from_log(log, folder)
    sheets(rows, folder, 'detail', crop=crop)
    print(f'{clip} {name}: {len(rows)} frames, {start}..{end}; {folder}')


def audio_metrics(clip):
    sys.path.insert(0, 'D:/GenjiBridgeAssets/pylib')
    import numpy as np
    folder = ROOT / 'audio' / clip
    for track in range(3):
        path = folder / f'track{track}.wav'
        with wave.open(str(path)) as w:
            sr, channels = w.getframerate(), w.getnchannels()
            x = np.frombuffer(w.readframes(w.getnframes()), dtype=np.int16).reshape(-1, channels).astype(np.float64) / 32768
        mono = x.mean(axis=1)
        hop = int(sr * 0.01)
        db = np.array([20 * np.log10(np.sqrt(np.mean(mono[k:k + hop] ** 2)) + 1e-10)
                       for k in range(0, len(mono), hop)])
        peaks = []
        for i in range(3, len(db)):
            if db[i] - db[i - 3] > 8 and db[i] > -48 and (not peaks or i * 0.01 - peaks[-1]['time_seconds'] >= 0.1):
                peaks.append({'time_seconds': round(i * 0.01, 3), 'rms_dbfs': round(float(db[i]), 2)})
        record = {'track': track, 'sample_rate': sr, 'channels': channels,
                  'peak_dbfs': round(float(20 * np.log10(np.max(np.abs(x)) + 1e-10)), 2),
                  'overall_rms_dbfs': round(float(20 * np.log10(np.sqrt(np.mean(x ** 2)) + 1e-10)), 2),
                  'candidate_onsets': peaks,
                  'note': 'Signal measurements only; peaks do not identify a sound.'}
        (folder / f'track{track}-metrics.json').write_text(json.dumps(record, indent=2), encoding='utf-8')
        print(json.dumps(record))


def reindex(clip):
    """Recover original decoded-frame IDs without assuming constant-rate video."""
    folder = ROOT / 'frames' / clip
    log = folder / 'source-timestamps.log'
    source = []
    timebase = None
    for line in log.read_text(encoding='utf-8-sig').splitlines():
        config = re.search(r'config in time_base:\s*(\d+)/(\d+)', line)
        if config:
            timebase = int(config[1]) / int(config[2])
        match = re.search(r'\bn:\s*(\d+)\s+pts:\s*(-?\d+)', line)
        if match:
            if timebase is None:
                raise RuntimeError('No decoded source timebase')
            source.append({'source_frame_1based': int(match[1]) + 1,
                           'source_pts_seconds': int(match[2]) * timebase})
    if not source or source[0]['source_pts_seconds'] != 0:
        raise RuntimeError('Source must include its first frame at zero')
    for a, b in zip(source, source[1:]):
        if b['source_frame_1based'] != a['source_frame_1based'] + 1 or b['source_pts_seconds'] <= a['source_pts_seconds']:
            raise RuntimeError('Nonsequential source frames/timestamps')
    lookup = {round(row['source_pts_seconds'], 7): row['source_frame_1based'] for row in source}
    (folder / 'source-timestamps.json').write_text(json.dumps(source, indent=2), encoding='utf-8')
    total = 0
    for path in folder.glob('*/timestamps.json'):
        rows = json.loads(path.read_text(encoding='utf-8'))
        for row in rows:
            row['source_frame_1based'] = lookup[round(row['source_pts_seconds'], 7)]
            if not Path(row['file']).is_file():
                raise RuntimeError(f"Missing image: {row['file']}")
        path.write_text(json.dumps(rows, indent=2), encoding='utf-8')
        total += len(rows)
    metadata_path = ROOT / f'{clip}-metadata.json'
    metadata = json.loads(metadata_path.read_text(encoding='utf-8'))
    metadata['survey_frames'] = len(json.loads((folder / 'survey/timestamps.json').read_text(encoding='utf-8')))
    metadata['decoded_source_frames'] = len(source)
    metadata['source_time_map'] = str(folder / 'source-timestamps.json')
    metadata_path.write_text(json.dumps(metadata, indent=2), encoding='utf-8')
    print(f'{clip}: {len(source)} source frames; {total} extracted frame references verified')


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('mode', choices=['prepare', 'detail', 'audio', 'reindex'])
    parser.add_argument('clip', choices=SOURCES)
    parser.add_argument('--start', type=float, default=0)
    parser.add_argument('--end', type=float, default=0)
    parser.add_argument('--name', default='detail')
    parser.add_argument('--step', type=float, default=0)
    parser.add_argument('--crop', type=int, nargs=4)
    args = parser.parse_args()
    if args.mode == 'prepare':
        prepare(args.clip)
    elif args.mode == 'audio':
        audio_metrics(args.clip)
    elif args.mode == 'reindex':
        reindex(args.clip)
    else:
        detail(args.clip, args.start, args.end, args.name, args.step, args.crop)


if __name__ == '__main__':
    main()
