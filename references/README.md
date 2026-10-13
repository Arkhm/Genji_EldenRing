# Genji reference library

Created: 2026-10-13.

**New-video intake status: three supplied videos analyzed and approved by the user on 2026-10-13. Findings and the DataTool brief are incorporated into `docs/OPUS-5.5-MOD-MANAGER-PROMPT.md`.**

Start with [the approved review](REVIEW-20261013.md), then [the inspection method](METHOD.md), [audio preservation notes](AUDIO-BASELINE.md), and [the approved integration brief](PROPOSED-MANAGER-ADDITION.md).

This folder contains the written reference index and analysis reports for the mod manager and its specialist agents. Recordings, extracted frames/audio and comparison images live in `D:\GenjiBridgeAssets\references`, outside the source repository, following AGENTS.md.

## Locations

| Location | Purpose |
|---|---|
| `D:\GenjiBridgeAssets\references\originals` | Unmodified copies of videos supplied by the user; preserve original filenames and file hashes. |
| `D:\GenjiBridgeAssets\references\frames` | Per-clip original-resolution frames, time maps and labeled contact sheets. |
| `D:\GenjiBridgeAssets\references\audio` | Per-clip extracted audio, timestamped sound excerpts and measurement files. |
| `D:\GenjiBridgeAssets\references\comparisons` | Saved side-by-side, overlay and difference images for later mod validation. |
| Project `references\<clip-id>.md` | Human-readable analysis with exact source/time/frame references, measurements and uncertainties. |

Store only authored Markdown documentation in this project folder. Never store videos, images, audio, extracted game bytes or generated asset files here. Keep the whitelist ignore rules unchanged.

## New clip index

| Clip ID | Original filename / external path | Duration / dimensions / frame rate | Skin / build / settings | Actions and timestamp ranges | Analysis status |
|---|---|---|---|---|---|
| OW-20261013-A | `D:\Videos\Edits\MedalTVOverwatch20261013180953506-trim-1791926141896.mp4`; preserved same-name copy under external `originals` | 33.05 s; 2580×1080; approx. 59.67 fps | Light armor/green-edged black weapons; exact skin/build/FOV unknown; shortened cooldowns supplied by user | Throws 4.70–18.60; reload 11.38–12.95; floor impacts 22.96–29.40; ult-ready 29.50–30.73 | [Approved; prompt integrated](OW-20261013-A.md) |
| OW-20261013-B | `D:\Videos\Edits\MedalTVOverwatch20261013181049319-trim-1791926175787.mp4`; preserved same-name copy under external `originals` | 34.54 s; 2580×1080; approx. 59.71 fps | Same visible appearance; exact metadata unknown; shortened cooldowns | Dash 3.25–3.70 and repeats; ready HUD 4.25–4.82; Deflect 20.10–22.52 and repeats | [Approved; prompt integrated](OW-20261013-B.md) |
| OW-20261013-C | `D:\Videos\Clips\Overwatch\MedalTVOverwatch20261013181158955.mp4`; preserved same-name copy under external `originals` | 60.30 s; 2580×1080; approx. 59.69 fps | Same visible appearance; exact metadata unknown; shortened cooldowns | Impacts/reloads 0–23.47; ult-ready 45.87–47.10; blade draw 48.20–50.05; swings 50.88–54.30; sheathe 55.30–56.45 | [Approved; prompt integrated](OW-20261013-C.md) |

Original copies have matching SHA-256 hashes. All nine audio tracks decoded separately; listening identities remain unverified. See per-clip external metadata and source time maps. The build's working cooldowns and existing correct audio must be preserved.

## Existing references

The previous library remains in `D:\GenjiBridgeAssets\ow_ref`, `hud_ref`, `recordings`, animation-preview folders and the asset exports. Start with `docs/REFERENCE-AUDIT.md` and `D:\GenjiBridgeAssets\ow_ref\INDEX.md` for coverage and known limitations. The previous audit is a dated snapshot; check current code/history before relying on it.

## What each new analysis should contain

Use `references\ANALYSIS-TEMPLATE.md` for every clip:

- File identity and recording metadata, settings visible or supplied, sampling uncertainty and audio availability.
- Timestamped actions and transitions: throws, fans, reload, melee, dash, Deflect, Dragonblade, movement, jump/climb/landing and interruption sequences, wherever present.
- Event phases and durations, hand/weapon screen positions and trajectories, visible parts, pose transitions, camera response and effect appearance/decay.
- HUD states, positions, sizes, numbers, colors, flashes, cooldowns, ready/reset moments and crosshair changes.
- Sound onsets/offsets, audible identity, voice transcription where possible, overlapping sounds, timing relative to animation/hits, and measured levels separately from listening observations.
- Agent-ready findings: exact frame/time anchors, what can be reproduced or tested, relevant project modules, conflicting evidence and still-missing coverage.

Analyze full sequences, not only attractive keyframes. Keep observed facts, measurements, inferences and unknowns distinct. A video without an input display cannot establish the exact key/button press time. A visible event does not prove its internal gameplay cause. Unknown skin/build/FOV stays unknown until verified.

Use source presentation timestamps (the time each recorded frame is meant to appear), rather than assuming every video has a constant frame rate. Keep audio and video on one source-relative clock, recording any offset. Preserve original frame numbers/timestamps when making slower previews or crops; those previews do not increase measurement precision. Report seconds and milliseconds, normalized screen positions and pixel units at the original resolution. Do not invent sub-frame precision.

New references take precedence over older material only when they are reviewed, cover the same state and match the agreed target skin/build/settings. Keep contradictory evidence visible. The user explicitly approved this batch on 2026-10-13; its findings and brief are now in `docs/OPUS-5.5-MOD-MANAGER-PROMPT.md`. Future batches still need any requested approval before prompt integration. Analysis approval is not runtime fidelity acceptance, and the template is not evidence.

## Verification

From PowerShell:

```powershell
Set-Location 'D:\games-mods\Games\GenjiBridge'
Get-Content '.\references\README.md'
Get-ChildItem -LiteralPath 'D:\GenjiBridgeAssets\references' -Directory
```

Expected now: three approved clips incorporated into the manager prompt and external folders `originals`, `frames`, `audio`, `comparisons`. Saved visual evidence and timestamp maps were inspected/validated. Audio listening identities and actual mod fidelity: **not tested**.

To recheck the decoded source-frame mappings without decoding videos again:

```powershell
python -B '.\tools\analyze_reference_videos.py' reindex OW-20261013-A
python -B '.\tools\analyze_reference_videos.py' reindex OW-20261013-B
python -B '.\tools\analyze_reference_videos.py' reindex OW-20261013-C
```

Expected source/extracted frame-reference counts: A 1971/653, B 2062/578, C 3598/890. Each selected frame must map to an authentic source timestamp and an existing JPEG. This checks reference integrity, not game behavior. Media preparation/detail modes are intended for new output windows; ffmpeg does not overwrite an existing extraction automatically.
