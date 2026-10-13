# Opus 5.5 manager addition — approved and incorporated

The user approved this text and reference batch on 2026-10-13. **Its instructions are now incorporated into `docs/OPUS-5.5-MOD-MANAGER-PROMPT.md`.** The outdated pending-video/originals-missing statements were replaced and the broader historical objective reconciled with the current effects/HUD scope. This filename is retained as the proposal record; use the integrated manager prompt for dispatch. The approval does not certify audio identities or actual mod fidelity.

---

## Current scope and user decisions

The current implementation priority is **effects and HUD transitions**. The build's cooldowns already work correctly; preserve gameplay cooldown values, recharge logic, state durations and readiness/reset causes. The user intentionally shortened cooldowns in the reference recordings to demonstrate actions faster. Compare event-relative effect and HUD animation phases, not the interval until another action becomes available. Use the existing game events as visual triggers. Do not modify working mechanics to reproduce custom recording settings.

Preserve current audio samples, variants, gains and event routing by default: the user says some sounds already match. No new audio replacement is authorized by these references. A particular future audio change needs a demonstrated mismatch in the actual build and listening evidence. Signal-only measurements cannot establish that a sound is wrong.

Hand/weapon/reload observations support effect attachment and the requested DataTool investigation of forearm shuriken. Diagnose and document that geometry/animation relationship; keep any broader animation implementation step separately reviewable and within approved scope.

## New authentic references

Read project `references/README.md`, `METHOD.md`, `REVIEW-20261013.md`, `AUDIO-BASELINE.md` and all three reports:

- `references/OW-20261013-A.md`: primary/fan throw effects; native reload 11.38–12.95 s; ultimate-ready HUD 29.50–30.73 s.
- `references/OW-20261013-B.md`: native dash 3.25–3.70 s; ability-ready column 4.25–4.82 s; Deflect entry/hold/exit 20.10–22.52 s and repeat/short-hold cases.
- `references/OW-20261013-C.md`: close wall impacts; repeated reloads; ultimate-ready repeat 45.87–47.10 s; Dragonblade draw/dragon 48.20–50.05 s; six swings 50.88–54.30 s; sheathe 55.30–56.45 s.

Media lives outside the repository in `D:\GenjiBridgeAssets\references\originals`, `frames`, `audio`, `comparisons`. Original filenames, hashes and source paths are in `<clip-id>-metadata.json`. Native/source time maps preserve authentic PTS and 1-based source frame ordinals. Cite clip ID, source time/frame range and exact asset path in every specialist finding. Contact sheets are navigation aids; inspect full 2580×1080 JPEGs for measurements. These three clips have approximately 60 fps recorded cadence, including gaps; do not derive exact frames by multiplying seconds by 60.

The new recordings visibly show light armor, dark joints and black weapons with green edges. Exact skin name/build/FOV/UI settings remain unknown. Do not inherit the purple skin or FOV metadata of an older recording automatically. Freeze the agreed reference target before scoring appearance. Keep user customizations visible in the acceptance contract.

## DataTool: forearm shuriken and paired animations

Assign an asset/animation investigator a bounded read/extraction task using the installed DataTool and existing exports. Read the local tool help/version and existing scripts before choosing commands; do not invent flags. Read the user's installed Overwatch files only as an asset source. Never run or attach to its live client. Save new extracted files to `D:\GenjiBridgeAssets` or an allowed gitignored area, never tracked source.

Use A's native reload frames as the ground truth for phase order: held fan disappears → hand closes/forearm rolls → hatch exposes rails → internal green-edged pieces visible → hatch closes → stars appear at wrist → fan returns while arm recovers. Important times are 11.383, 11.633, 11.883–12.450, 12.500, 12.550–12.633 and 12.950 s. Video appearance does **not** prove that a separately named internal-shuriken model exists.

Trace the Gameplay1P entity hierarchy and reload animation/effect associations, including child entities, submodels, skeleton nodes, mesh skinning, hardpoints (attachment locations), animated transforms, visibility/scale tracks and timing. Determine which exact source part produces the hatch, internal rails and internal stars. They may be separate models or parts of the arm/weapon model. Distinguish first-person, third-person and projectile variants. Trace the corresponding weapon/arm animation pair in the correct coordinate system; matching clip length alone is insufficient.

Starting leads, not certified answers:

- `tools/export_viewmodel.py` reads arms model folder `D:\GenjiBridgeAssets\raw\Heroes\Genji\Skin\Overwatch\Overwatch\Models\00000000DBA6.00C`; current held shuriken model is DA9E. The exporter currently includes resting pair 240E/240D. Read the existing entity exports under `Entities\Gameplay1P` and related Shuriken/Dragonblade/Deflect entities.
- The exporter explicitly records rejected pairings: primary 2478/2498, fans 243B/2451 and 244F/2450, reload 2436/33BA. Those tests produced an inappropriate flying star despite similar lengths. Do not repeat them unchanged or present them as verified pairings. Investigate entity/attachment/visibility data explaining the result.
- Existing bones include `Wrist_R`, `Weapon_R`, `Elbow_R`, twist nodes and opaque numbered nodes. An absent friendly `Magazine`/`Shuriken` name does not prove absent geometry; classify opaque nodes by weights, hierarchy and animated transforms.
- `genji/src/arms_anim.rs::parts_for` presently hides held shuriken for all throw/fan/reload roles. A's fan and reload show returning stars before the entire arm action finishes. Verify phase-specific visibility from actual asset tracks instead of reusing a whole-action hide rule.
- Exported index links reload animation 2436 to effect `000000000C9A.08F` and sound groups 91D6–91D9; follow effect/entity graph rather than treating a sound-bearing effect as the mesh ID. Preserve existing sound files.

Deliver exact full asset IDs/paths, why each is first-person/applicable, paired clip/hardpoint relationships, visibility/transform phase evidence and deterministic renders at the cited phases. Report unknown tracks explicitly. Do not claim the forearm asset has been found based on these leads alone.

## Effects and HUD specialists

Effects: compare dash bright ribbon plus multiple narrow strands, camera-dependent orientation and roughly 0.25 s tail; floor/wall impact burst and embedded projectile geometry; shaped Dragonblade draw dragon and curved flashes; alternating swing direction, white/yellow core, green frayed strands and roughly 0.23 s observed decay. Deflect's held pose is covered; successful reflected-hit effects are not covered by these clips. Trace real source meshes/materials/textures/curves before fabricating approximations.

Existing effect leads include blade swings 1257/125A → 0396/0397, textures 4E2F/6613/57E6 and gradient 8008CA787D10. Current project changes already use extracted swing textures, so verify current rendering before alleging the old plain trail remains. Dash script 5362 → 0542/1769 remains a provisional mapping; 249F → CF9 and Deflect animation/effect leads 342C → 1875, 3555 → 1893 are graph starting points, not proof of which overlay/variant is correct. Follow child/script graphs for draw/dragon and distinguish world effects from first-person effects.

HUD: compare active cyan tiles, countdown-to-ready appearance, white star/tall cyan column, rising rectangular particles and ordinary white/dark tile recovery (~0.57 s); ultimate white-disc collapse/growth, emblem reveal, blue ring, expanding shockwave, curls and persistent aura (~1.2 s main transition); Deflect cancel prompt lifecycle; blade meter reset and separate center ring; return to normal crosshair. Use the existing state/events. Measure positions/sizes at the original aspect ratio and preserve agreed custom HUD elements. Do not copy custom `SPEED`, lobby/network overlays or accelerated timers automatically.

## Manager acceptance and gaps

Own the integrated review. Assign bounded specialists and require provenance, baseline, changed files, rendered/captured comparisons and test results. Read their output and verify their claims; compilation or extracted assets alone do not establish visual fidelity.

Capture/deterministically render matched phases, align by visible transitions plus logged mod events, normalize agreed framing and compare side by side. Mask unrelated world backgrounds. Score effect silhouette, brightness/color progression, trajectory, decay and HUD geometry/phase timing separately. Freeze metric definitions, weights, reference crops and tolerances before implementation. Use the prompt's 95/100 **per-category and essential-action gates** only for supported measured categories; no global average may conceal a failing transition. Do not claim a guaranteed 95% result until evidence supports it. Mark runtime comparisons **not tested** if no actual saved build captures were supplied.

New videos do not fill enemy-hit/kill-reset, successful Deflect, damage/death, normal melee, movement/climb/landing, interruption, third-person or world-distance gaps. Check older authentic references for those; never promote an unsupported category to verified. Listening is unavailable in the current analysis; sound identities/transcripts remain unverified. Continue independent visual work while keeping the audio blocker documented.
