# Reference analysis: <clip-id>

**Template only. Not an analyzed clip or verified reference.**

## Identity and recording conditions

- Original filename and absolute media path:
- File hash (SHA-256), size and receipt date:
- Game/source and target relevance:
- Duration, dimensions, frame rate / timestamp behavior:
- Audio tracks, channels, sample rate and audio/video offset:
- Skin, build/patch, FOV, resolution/UI scale, crosshair and sound settings:
- Which settings are verified, supplied by the user, inferred or unknown:
- Compression, occlusion, frame gaps, slow motion or other limits:
- Frame/audio extraction command and tool version, if used:
- External frame/time-map/audio/comparison locations:

## Coverage and action timeline

| Event ID | Start–end source timestamp | Source frame IDs/timestamps | Action / state / transition | Observation and confidence | Frame/audio evidence paths |
|---|---|---|---|---|---|
| <event> | <seconds> | <frames> | <action> | <observed / inferred / uncertain> | <absolute paths> |

List all relevant events, repetitions, variants and interruptions. Flag absent actions explicitly.

## Animation, camera and weapon measurements

For each event: anticipation, visible release/contact, active action, recovery and return to rest; durations with source-supported uncertainty; hand/wrist/blade-tip anchors as x/width and y/height; apparent weapon size/orientation; visible weapon parts; motion path; body/camera movement; blend/cancel behavior and overlapping actions.

| Event ID / phase | Time / frame | Measurement and units | Value / uncertainty | Evidence | Meaning for implementation |
|---|---|---|---|---|---|
| <event> | <time> | <quantity> | <value> | <path> | <finding> |

Do not infer exact input times from animation onset alone. Note whether the observed motion belongs to the hands, camera, world or a combination.

## Effects

Per effect: onset, peak, disappearance, location/extent, direction/path, colors/core/glow, opacity, strands/particles, layering/occlusion and repeated-run variation. Record sampled pixel values separately from inferred material colors; compression and scene lighting affect captured colors. Link possible extracted assets as hypotheses until confirmed.

## HUD and crosshair

Record health/portrait, ammo/reload, ability tiles and cooldown/ready/reset transitions, ultimate charge/ready/press, blade timer, kill indicators and crosshair whenever visible. Include source pixel geometry plus normalized anchors, label/glyph details, state-change timestamps and animation phases. Flag cropped or unreadable HUD areas.

## Sound and voice

| Sound ID | Onset–offset timestamp | Audible sound / voice words | Related event and relative offset | Overlap / confidence | Audio excerpt / measured level |
|---|---|---|---|---|---|
| <sound> | <time> | <description> | <event and offset> | <limits> | <path and units> |

State whether audio was actually listened to, analyzed numerically or transcribed by a tool. Do not identify a sound from its waveform alone. Distinguish attack, projectile, impact, enemy, environment, UI and voice when possible. Do not describe a mixed recording as an isolated game sound. Mark unintelligible words rather than inventing a transcript.

## Findings for agents

| Finding / test case | Reference event IDs | Measured target | Relevant module(s) | Confidence / open question | Suggested validation |
|---|---|---|---|---|---|
| <finding> | <events> | <target> | <files> | <status> | <comparison> |

Map to the current authored files after inspecting them. Separate observations from proposed implementation. Include deviations from older references and any settings/skin/build explanation supported by evidence.

## Missing evidence and completion status

- Covered actions/states:
- Missing or ambiguous actions/states:
- Needed follow-up footage/settings, if any:
- What was actually inspected/measured/listened to:
- What remains **not tested**:
- Index and manager-prompt updates:

Reference analysis documents the source footage. It does not certify the mod's fidelity or runtime behavior.
