# Audio preservation and reference measurements

Status: preserve existing audio; listening comparison **not verified**. User says some audio in the build is already correct. The exact accepted event list is not supplied, so **every existing event/sample/routing/gain is protected by default**. This reference batch does not authorize audio replacement or tuning.

## Read-only build baseline

`D:\GenjiBridgeAssets\references\current-build-audio-baseline.json` records the current sound index: 56 events and 375 indexed file entries, with paths, sizes and SHA-256 hashes. It describes asset availability, not proof that each event plays correctly at runtime. No sound files were modified.

| Existing event | Indexed variants |
|---|---:|
| shuriken_throw | 6 |
| fan_throw | 5 |
| reload_1 / reload_2 / reload_3 / reload_4 | 5 each |
| dash | 3 |
| deflect_start / deflect_end | 2 each |
| blade_draw | 2 |
| blade_swing | 6 |
| blade_sheathe | 1 |
| ult_ready_sfx | 1 |

Existing `genji/src/sound.rs` has MASTER=0.8 and user-adjusted event gains including throw=0.45, ultimate voice=2.5, kill=0.5, footstep=2.5. These are baseline observations, **not new recommended levels**. Preserve them. The older exporter/index and extracted effect sound IDs are useful tracing leads, not instructions to overwrite samples that already work.

## Reference tracks

Each supplied video has three separate stereo AAC tracks at 48 kHz. PCM WAV copies and decode logs are under `D:\GenjiBridgeAssets\references\audio\<clip-id>`. Tracks 0 and 1 contain nonzero signal; track 2 decodes to digital silence in all three clips. Game/microphone routing for tracks 0 and 1 is not established by listening. Do not sum them and accidentally double shared audio.

| Clip | Track | Peak dBFS | Overall RMS dBFS |
|---|---|---:|---:|
| A | 0 | -16.02 | -40.80 |
| A | 1 | -16.07 | -40.72 |
| B | 0 | -4.86 | -31.23 |
| B | 1 | -4.75 | -31.16 |
| C | 0 | -10.83 | -36.09 |
| C | 1 | -4.96 | -30.24 |

dBFS measures recorded digital signal relative to full scale; RMS measures average level. Recording gain, mixed background and source track differences mean these numbers are **not mod gain targets**.

`trackK-metrics.json` contains signal candidates: 10 ms windows, a rise of more than 8 dB over 30 ms, above -48 dBFS, with at least 100 ms between candidates. This can miss quiet events and detect unrelated noise. The clip reports list selected candidates near visual actions without assigning audible identities. Raw signal timing has not been corrected for codec/display latency; do not claim exact sound-to-animation synchronization from it.

## Listening limitation and future comparison

This session cannot accept audio input. MP3 conversion first failed because the installed ffmpeg has no `libmp3lame` encoder; forwarding a successfully extracted WAV then returned `audio content omitted because you do not support audio input`. Sound identities, voice transcripts, perceived pitch/timbre, tails and audible similarity are unverified. The blocker and two attempts are recorded in root `STATUS.md`. Visual work continued independently.

A manager should first inspect current assets and event logs and obtain a saved offline build capture for a specific suspected mismatch. Play matched reference/build excerpts, control recording volume, and compare which existing variant/routing/timing actually differs. If the build already matches, record **preserve** and move on. If identity/listening evidence is unavailable, record **unverified**. No blanket regeneration, re-extraction, sample replacement, gain normalization or sound.rs changes are justified by this batch.

For manual listening, open each `track0.wav`/`track1.wav` separately in a player and seek to the candidates in the reports. A short reload excerpt is also available at `D:\GenjiBridgeAssets\references\audio\OW-20261013-A\reload_review.wav` (source excerpt starts at 11.3 s). Hearing it is a user review step, not something this agent has verified.
