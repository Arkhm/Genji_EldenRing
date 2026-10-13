# Reference timing and inspection method

Status: analysis approved by the user on 2026-10-13 and incorporated into the manager prompt. Only authored documentation and analysis source are inside the project. All original videos, decoded images, audio and generated measurement files are outside it in `D:\GenjiBridgeAssets\references`.

## Identity and timing

The supplied originals were copied with unchanged filenames into external `originals/`; each copy matched the supplied file's SHA-256 hash. Metadata and hashes are in `<clip-id>-metadata.json`. All videos are HEVC, 2580×1080, with three AAC stereo 48 kHz audio tracks. Reported average frame rates are approximately 59.67, 59.71 and 59.69 fps; they must not be used as an exact frame clock.

Each timestamp is a decoded **presentation timestamp** (PTS): when that recorded frame is meant to be displayed, relative to the beginning of its own clip. A, B and C have separate clocks. Native frames are usually about 16.7 ms apart, with occasional gaps. A boundary first seen at one frame can have occurred between it and the preceding frame; videos without an input display cannot establish exact button-press time.

Full decoded timestamp maps record 1,971 original video frames for A, 2,062 for B and 3,598 for C. They live in `frames/<clip-id>/source-timestamps.json`. These are 1-based frame ordinals in presentation order, not filesystem image numbers and not assumed time×60. Each extracted window has `timestamps.json` mapping `fNNNNN.jpg` to source PTS and `source_frame_1based`.

The installed ffmpeg prints an invalid formatted `pts_time` at zero on this machine. The script therefore uses the integer `pts` and decoded time base instead; the first frame is included and verified at zero.

## Inspection coverage and precision

Full-clip survey sheets were inspected at roughly 0.5-second spacing (actual selected spacing is often 0.5167 s): A 64, B 67, C 117 frames. Action sections were examined more closely at roughly 0.08–0.10-second requested spacing (actual selection is often 0.0833 or 0.1167 s). Important reload, throw, dash, Deflect, blade and HUD transitions were extracted at native recorded-frame spacing. Critical phase anchors were inspected on those sheets. **Not every native frame of all three videos was individually inspected**; subtle events between coarse samples can remain unmarked.

There are 2,121 extracted frame references across all windows, some duplicating the same source frame. All were successfully matched to full source timestamp maps and existing JPEG files. Survey/detail contact sheets are reduced for browsing; corresponding JPEGs retain original 2580×1080 resolution. Crops retain the source time and do not create extra timing precision. Do not measure pixel dimensions on resized sheet thumbnails.

Times in broad event tables are approximate review ranges. Detailed anchor tables identify authentic frame PTS, rounded to milliseconds, and source ordinals. Use the full timestamp JSON for exact recorded values. Color descriptions are visual observations, not calibrated HDR/material measurements. Skin name, game build, FOV, HUD scale and input configuration are not established from the filenames. Visible hands have light armor, dark joints and green-accented black weapons.

## Comparison guidance

Approximate recorded layout: crosshair (x/W≈0.50, y/H≈0.50), ultimate center (≈0.50, ≈0.86), dash tile (≈0.845, ≈0.875), portrait/health at lower left, Deflect and ammunition to the right of dash. These are navigation estimates, not pixel-certified measurements. Compare original-resolution crops and measure actual centers, outlines and sizes before using them as acceptance tolerances. The aspect ratio is 43:18; account for it rather than stretching the HUD to an arbitrary display.

Separate event-relative visual phase duration from gameplay state duration. Align videos at a visible effect/HUD boundary and log the mod's corresponding event. Match camera/FOV/aspect/skin where possible. Compare effect masks, silhouettes, color progression, brightness, trail direction, phase timing and HUD layout. Background differences between Overwatch and Elden Ring should be masked. Save both successes and failures, and mark unsupported categories **not tested**.

Do not port `WAITING FOR PLAYERS`, network/FPS overlays, custom `SPEED` status, or shortened timer/charge intervals automatically. They appear in the recording but do not establish required Elden Ring mod behavior. No live game was launched or visually inspected during this task.

Audio measurements use separate tracks and a 10 ms analysis window. Peaks are threshold-based candidates and may miss quiet events or detect unrelated noise. Listening was unavailable; see `AUDIO-BASELINE.md` and root `STATUS.md`.

Validation results are saved at `D:\GenjiBridgeAssets\references\verification-20261013.json`: all three copy hashes matched, all 2,121 selected JPEG headers/dimensions/integrity checks passed, nine WAV formats/durations were checked, all source maps matched, documentation links resolved, and all 375 indexed existing sound hashes remained identical. An initial one-off documentation-link check failed because Windows used a non-UTF-8 default text encoding; explicit UTF-8 fixed that check. This failure did not alter media or invalidate the native timestamp extraction. No game behavior was tested.
