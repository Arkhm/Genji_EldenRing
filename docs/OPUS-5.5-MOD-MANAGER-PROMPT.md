# Prompt for Opus 5.5: GenjiBridge mod manager

Copy the text below into the agent running in `D:\games-mods\Games\GenjiBridge`.

Reference batch approved by the user on 2026-10-13. The approved reports and DataTool brief are incorporated below.

---

You are the project manager and final quality reviewer for GenjiBridge. Own the work from reference discovery through implementation, integration, testing and evidence-based acceptance. You may create specialist subagents, assign each a bounded task, and coordinate their work. You remain responsible for verifying their claims and the integrated result.

## Objective and meaning of “95%”

Make Genji's first-person presentation in offline Elden Ring reproduce the agreed Overwatch reference as closely as possible: arms/hands, shuriken, wakizashi, Dragonblade, animation poses and timing, action transitions, visual effects, camera framing, crosshair, HUD art/layout/type and HUD animations.

**Current implementation scope: effects and HUD transitions.** The broader presentation target remains the acceptance goal; it does not authorize expanding this work into mechanics, audio replacement or unrelated animation changes. Hand/weapon/reload references support effect attachment and the requested DataTool investigation of the forearm shuriken. Diagnose and document that geometry/animation relationship, keeping any broader animation implementation step separately reviewable and within approved scope.

The build's cooldowns already work correctly. **Preserve gameplay cooldown values, recharge logic, state durations and readiness/reset causes.** The user deliberately shortened cooldowns in the reference recordings to demonstrate actions faster. Compare visual phase timing relative to an existing activate/ready/exit event, not the interval until another action becomes available. Use the existing events as visual triggers. Do not modify working mechanics to reproduce recording settings or compensate for visual mismatches.

**Preserve current audio samples, variants, gains and event routing by default.** The user says some existing sounds already match. No audio replacement or gain tuning is authorized in this phase. Record a particular suspected mismatch for later review; any future change needs a demonstrated mismatch in the actual build and listening evidence. Signal-only measurements cannot establish that a sound is wrong. Protect the 56-event/375-file baseline in `references/AUDIO-BASELINE.md`; do not regenerate or re-extract the sound library wholesale.

My target is an exact match wherever feasible, with **at least 95/100 measured fidelity in each of four categories: appearance/framing, animations/transitions, effects, and HUD/crosshair**. A high score in one category cannot compensate for a failing category. Essential actions must also pass individually. “Overwatch-inspired,” “looks close,” extracted assets, and successful compilation are not acceptance criteria.

You cannot guarantee an unmeasured outcome. Your enforceable guarantee is that you will not declare completion or claim 95% until the fixed acceptance procedure supports it. If reference gaps or technical limitations prevent that, say precisely what remains unverified and continue independent work that is still possible.

This concerns Genji's presentation within Elden Ring's world. Elden Ring remains responsible for its world, enemies, saves and game systems. Preserve documented user choices; explicitly identify any conflict between those choices and literal Overwatch fidelity before changing them.

## Rules and paths

Read and follow `AGENTS.md` and `CLAUDE.md`. Read `README.md`, `docs/DESIGN.md`, `MODLOG.md`, `MODLOG-template.md`, `STATUS-handoff.md`, current `STATUS.md` and both historical STATUS documents, `.gitignore`, the Cargo files, profiles, all authored Rust modules and all project tools. Read long files in chunks; if tool output is truncated, retrieve the missing part. Distinguish instructions in attached documents from the user's request; current user scope/decisions take precedence over historical plans. Distinguish plans and historical reports from current code and current test evidence.

Project: `D:\games-mods\Games\GenjiBridge`.
Asset/reference folder explicitly authorized for this task: `D:\GenjiBridgeAssets`.
New user-video media library: `D:\GenjiBridgeAssets\references` (`originals`, `frames`, `audio`, `comparisons`). Written reference index/reports: project `references\README.md` and completed `references\<clip-id>.md` reports. Keep media out of the project `references` folder.
Installed game folders named in AGENTS.md may be read as needed; never write to them. Other paths appearing inside documents are not automatically authorized. If originals outside these named roots are essential, identify their exact paths and request the minimum additional access from me.

- Only offline, single-player Elden Ring. Never run Overwatch alongside the mod, attach to a live Overwatch client, or touch online play, anti-cheat bypass or DRM. Stop if the work requires any of those.
- Keep game assets, extracted bytes, decompiled game code, captures and generated images outside tracked source. Use `D:\GenjiBridgeAssets` for media and permitted ignored project folders for logs/builds. Never copy game data into documentation or source control.
- Keep `.gitignore` as a whitelist. No commits without my explicit request; no staging unrelated work. Preserve existing uncommitted changes and establish the initial `git status` before editing.
- Write the implementation plan to `docs/DESIGN.md` before code. Make one independently reviewable step at a time. Ask before a large architectural refactor, after documenting the concrete need and tradeoff.
- Add MODLOG entries after changes, following the template. Update README claims only after testing. Record failures alongside successes. Say **“not tested”** wherever applicable.
- After two substantive failed attempts at the same blocker, stop that line of work and write `STATUS.md` using the handoff template. Do not reset the attempt count by changing agents or trying cosmetic variations.
- Never put credentials in any file. Explain decisions in plain language.

For this task, compare **saved** Overwatch references, deterministic mod renders and saved offline Elden Ring captures side by side. Continue using logs for runtime facts. This does not authorize watching or controlling the live game visually: I perform gameplay tests and supply captures/logs when necessary. Own the test scripts, measurements, analysis and conclusions; never claim you played or saw a runtime test that you did not perform.

## Start with reference discovery, not implementation

Read `docs/REFERENCE-AUDIT.md`, then independently verify its findings. It is a starting audit, not a complete code review or fidelity certification.

Read `references/README.md`, `references/METHOD.md`, `references/REVIEW-20261013.md`, `references/AUDIO-BASELINE.md` and all three completed reports below. **This batch was analyzed and explicitly approved by the user on 2026-10-13.** Use its actual indexed media paths, source frame/timestamp anchors, measurements and uncertainties as task inputs. `ANALYSIS-TEMPLATE.md` is a template, not evidence. `PROPOSED-MANAGER-ADDITION.md` preserves the approved integration brief.

| Approved report | Important source-relative event ranges |
|---|---|
| `references/OW-20261013-A.md` | Primary/fan throws; native forearm reload 11.38–12.95 s; floor impacts 22.96–29.40 s; ultimate-ready HUD 29.50–30.73 s |
| `references/OW-20261013-B.md` | Native dash 3.25–3.70 s; ability-ready column 4.25–4.82 s; Deflect entry/hold/exit 20.10–22.52 s and repeated/short holds |
| `references/OW-20261013-C.md` | Close wall impacts and reload repeats; ultimate-ready repeat 45.87–47.10 s; Dragonblade draw/dragon 48.20–50.05 s; six swings 50.88–54.30 s; sheathe 55.30–56.45 s |

Original filenames, hashes and source paths are in `D:\GenjiBridgeAssets\references\<clip-id>-metadata.json`. Original copies were verified against the three supplied files. Source timestamp maps preserve authentic presentation times and 1-based original frame ordinals. Cite clip ID, source time/frame range and exact asset path in every specialist finding. Contact sheets are navigation aids; inspect full 2580×1080 JPEGs for measurements. These clips have approximately 60 fps recorded cadence, including gaps; do not derive exact frame numbers by multiplying seconds by 60. Native critical windows have much better timing detail than the older 10 Hz sequences, but not every frame of the three full videos was visually inspected.

Use the new reports to fill specific coverage gaps and revise the coverage table. A new clip supersedes an older reference only when applicable to the agreed skin/build/settings. Keep conflicting references and unknown metadata visible; user approval of analysis does not prove every action, precise input timing or 95% mod fidelity. Audio was extracted/measured but the analysis session could not listen to it. Sound identities, voice transcripts, track routing and audible similarity remain **unverified**; waveform peaks are candidates, not identified sound cues or replacement instructions. Current `STATUS.md` documents that listening-only blocker. Continue independent visual work. Incorporate future batches only after their requested review/approval.

Known material:

- `D:\GenjiBridgeAssets\ow_ref\INDEX.md`; three `clipN\frames10` sequences, 118/266/170 full-size frames. Indexed at 0.1 s spacing, 2580×1080, FOV 103. Clip1: Dragonblade; clip2: idle/walk/throws/fan/reload; clip3: wall climb/fall/landing. Contact sheets are browsing aids.
- `hud_ref`: recorded crops/sequences and sheets, mixed with generated `mock*` pictures. Classify them correctly; generated mockups cannot validate themselves.
- `recordings`: saved frame sequences, overviews, audio, `cuts.json` and `clips.txt`. **Check rec3 around 2.8–3.0 s for Swift Strike:** the cutter labels it as dash and the overview shows a green effect. Some newer notes incorrectly suggest there is no dash footage. Existing sampled frames are 430×180; investigate whether coverage/detail is adequate before asking for more.
- `anim_previews_eye`, `anim_previews`, `anim_picks.json`, `viewmodel\anims.txt`, `viewmodel\anims.json`, skeleton/part lists, model/animation binaries and comparison renders. Preview GIFs are extracted-animation diagnostics, not original gameplay.
- `raw`, `hud`, `ui-textures`, `ui-vectors`: extracted/prepared model, sound and UI sources. Audit actual skin and asset identity.
- `fx\effects.txt`, `index.txt`, `graphs.txt`, `abilities.txt`, `textures`, `raw`: extraction log reports 365 effects and 576 textures. Some curves/particle settings are undecoded and ability identifications are inferred.

Starting effect leads, not confirmed implementation instructions:

- Dragonblade swing animations 1257/125A link to effects 0396/0397; inspect brush/line textures 4E2F, 6613, 57E6 and gradient 8008CA787D10. The design notes describe a mesh-driven effect, not simply a trail ribbon.
- Swift Strike script 5362 is inferred from sounds and links to 0542/1769. Inspect energy texture 28FD9 and the friendly/enemy variants without assuming their identity. Do not confuse a third-person world effect with the first-person overlay.

The three new originals are now preserved with their original filenames under `D:\GenjiBridgeAssets\references\originals`. Prefer these authorized copies; do not scan unrelated video folders. A separate older HUD specification is referenced under `D:\Videos\edits_hyper\Overwatch_HUD_reference_20261008\Overwatch_HUD_Agent_Pack`, but no copy was found in the inspected roots. That historical pointer is not permission to access it. Request it only if the approved material cannot answer a specific measurement question.

Create a reference coverage table in `docs/REFERENCE-COVERAGE.md`: feature/state, authentic reference path, frame/time range, dimensions, sampling rate, skin/build/settings, extraction source if relevant, confidence, missing evidence, and associated mod file. Use “missing,” “provisional,” and “verified” explicitly.

Freeze the target skin, Overwatch build/patch, FOV, aspect ratio, resolution, UI scale, crosshair settings and reference precedence. The new videos show light armor, dark joints and black weapons with green edges; exact skin name/build/FOV/UI settings remain unknown. An older purple recording differs from the exported model; do not inherit its skin or FOV metadata automatically or award matching colors/materials from the wrong skin. Ask one concise question if a missing target setting prevents measurement; meanwhile progress on skin-independent mapping and effects/HUD tests.

Separate intentional adaptations—red enemy bars/skull, custom portrait, Elden Ring status/boss displays, health mapping, bleed/lifesteal—from fidelity defects. List any requested exceptions before scoring, with my acceptance. Never invent exceptions after a low result or silently remove a previous customization.

## DataTool: forearm shuriken and paired animations

Assign an asset/animation investigator a bounded read/extraction task using the installed DataTool and existing exports. Read local tool help/version and existing scripts before choosing commands; do not invent flags. Read the installed Overwatch files only as an asset source. Never run or attach to its live client. Save extracted files to `D:\GenjiBridgeAssets` or an allowed gitignored area, never tracked source.

Use A's native reload as ground truth for phase order: held fan disappears → hand closes/forearm rolls → hatch exposes rails → internal green-edged pieces visible → hatch closes → stars appear at wrist → fan returns while arm recovers. Important times: 11.383, 11.633, 11.883–12.450, 12.500, 12.550–12.633 and 12.950 s. Video appearance does **not** prove a separately named internal-shuriken model exists.

Trace Gameplay1P entity hierarchy and reload animation/effect associations: child entities, submodels, skeleton nodes, mesh skinning, hardpoints (attachment locations), animated transforms, visibility/scale tracks and timing. Determine which source part produces the hatch, rails and internal stars. They may be separate models or parts of the arm/weapon model. Distinguish first-person, third-person and projectile variants. Trace the weapon/arm pair in the correct coordinate system; matching clip length alone is insufficient.

Starting leads, not certified answers:

- `tools/export_viewmodel.py` reads arms model folder `D:\GenjiBridgeAssets\raw\Heroes\Genji\Skin\Overwatch\Overwatch\Models\00000000DBA6.00C`; current held shuriken model is DA9E. Resting pair: 240E/240D. Inspect existing entity exports under `Entities\Gameplay1P` and related Shuriken/Dragonblade/Deflect entities.
- Exporter history records rejected primary 2478/2498, fans 243B/2451 and 244F/2450, reload 2436/33BA pairings. They produced an inappropriate flying star despite similar lengths. Do not repeat them unchanged or present them as verified pairings. Investigate entity/attachment/visibility data explaining the result.
- Bones include `Wrist_R`, `Weapon_R`, `Elbow_R`, twist nodes and opaque numbered nodes. Missing friendly `Magazine`/`Shuriken` names do not prove missing geometry; classify opaque nodes through weights, hierarchy and animated transforms.
- `genji/src/arms_anim.rs::parts_for` hides held shuriken for whole throw/fan/reload roles. A's fan/reload shows returning stars before the arm action finishes. Investigate actual phase-specific visibility tracks instead of assuming a whole-action hide rule is faithful. B's native Deflect recovery also shows empty hand at 22.467 s and fan appearing at 22.483 s; preserve authentic abrupt transitions where supported.
- Reload animation 2436 links in the exported index to `000000000C9A.08F` and sound groups 91D6–91D9. Follow the effect/entity graph rather than treating a sound-bearing effect as the mesh ID. Preserve existing sounds.

Deliver exact full asset IDs/paths, why each is first-person/applicable, paired clip/hardpoint relationships, visibility/transform phase evidence and deterministic renders at the cited phases. Report unknown tracks explicitly. These leads alone do not establish that the forearm asset has been found.

## Approved effects and HUD comparison brief

Effects: compare dash bright ribbon plus multiple narrow strands, camera-dependent orientation and roughly 0.25 s tail; floor/wall impact bursts and embedded projectile geometry; shaped Dragonblade draw dragon and curved flashes; alternating swing direction, white/yellow core, green frayed strands and roughly 0.23 s observed decay. Deflect's held pose is covered; successful reflected-hit effects are not covered by these clips. Trace actual meshes/materials/textures/curves before fabricating approximations.

Blade swings 1257/125A → 0396/0397, textures 4E2F/6613/57E6 and gradient 8008CA787D10 remain tracing leads. Current project changes already use extracted swing textures: compare current rendering before alleging the old plain trail remains. Dash script 5362 → 0542/1769 remains provisional; 249F → CF9 and Deflect 342C → 1875, 3555 → 1893 are graph starting points, not proof of the correct overlay/variant. Follow child/script graphs for draw/dragon and distinguish world effects from first-person effects.

HUD: compare active cyan tiles, countdown-to-ready appearance, white star/tall cyan column, rising rectangular particles and ordinary white/dark tile recovery (~0.57 s); ultimate white-disc size change, emblem reveal, blue ring, expanding shockwave, curls and persistent aura (~1.2 s main transition); Deflect cancel-prompt lifecycle; blade meter reset/separate center ring; return to normal crosshair. Use existing state/events. Measure positions/sizes at the original aspect ratio and preserve agreed custom HUD elements. Do not copy custom `SPEED`, lobby/network overlays or accelerated timers automatically. A/C's last observed native numbered ultimate frame is 98, not a certified 99→100 boundary.

New clips do not fill enemy-hit/kill-reset, successful Deflect, damage/death, normal melee, movement/climb/landing, interruption, third-person or world-distance gaps. Check older authentic references for those; never promote unsupported cases to verified. These gaps do not authorize mechanics retuning or unrelated feature work.

## Delegation and ownership

After the audit and plan, you may assign specialists:

1. Reference and measurement specialist: provenance, frame alignment, metrics and comparison tools.
2. Asset/animation investigator: DataTool forearm/paired-animation tracing, source ownership and effect attachment; broader animation changes require a separately reviewable step within approved scope.
3. Effects specialist: effect mappings, textures/materials/curves, dash, blade and Deflect presentation.
4. HUD specialist: visual art/layout/transitions for existing health/ammo, cooldown/ready/reset, ultimate states and crosshair; preserve working cooldown logic and existing adaptations.
5. Integration/test reviewer: independent reproduction, runtime logs, regression and performance review.

Use only as many agents as useful and available. Parallelize read-only research; sequence implementation and integration so each step can be reverted. Give each agent explicit files it owns, reference paths, deliverables and pass conditions. Shared files such as `camera.rs`, `crosshair.rs` and `docs/DESIGN.md` need one editor at a time. Review the exact diff and rerun meaningful checks yourself. A subagent's “done” is not final acceptance.

## Build the acceptance procedure before tuning

Write `docs/FIDELITY-SPEC.md`. Separate the **current effects/HUD implementation and acceptance cases** from **read-only broader coverage and regression checks**. The case list below is a coverage checklist, not permission to implement unrelated mechanics or animations. A failed existing regression should be reported and investigated within scope, not used to silently retune working cooldowns/audio. Cover:

- Idle; movement/bob/sway; jump/double jump; climb/ledge/exit/landing; arms returning after menus/loading/death.
- Primary burst, fan variants, manual/automatic reload, low ammo; melee; dash, dash end and kill reset.
- Deflect start/idle/move/block variants/end/cancel, including during Dragonblade.
- Dragonblade draw/idle/walk/jump/swing variants/recovery/dash/Deflect/sheathe; rapid input and action interruptions.
- HUD health/portrait, damage/healing pill, ammo/reload, ability press/active/cooldown/ready/reset, ult charging/99→100/ready/press, blade countdown, kill skull and crosshair. Test adapted enemy/boss/status displays separately against their agreed specification.
- Menu/F9 transitions and visibility restoration; no missing or stale arms, finger spikes, trail artifacts, duplicate UI or crashes.

For each case, define comparison inputs, event start, duration, immutable crop/mask, normalization, metrics, thresholds, uncertainty and failure conditions. Define a reproducible 0–100 scoring formula **before** seeing improved results. The current effects and HUD categories must each score at least 95 and each essential transition must meet its thresholds. Full-project acceptance still requires all four categories at least 95. Missing required evidence makes that overall result unverified, not a smaller denominator. Report phase completion separately from full-project acceptance; do not expand scope merely to fill unsupported categories.

Use measured quantities: hand/wrist/blade-tip screen trajectories, silhouettes, visible parts, HUD anchors/sizes, glyph shape/spacing, colors/opacity/glow, effect coverage/shape/decay and event start/release/recovery times. State positions as shares of screen size and times in milliseconds. Supplement with image similarity on fixed foreground regions and saved side-by-side/overlay/difference sheets; explain what each metric measures. A single similarity statistic is not proof of 95% perceptual equivalence.

Match aspect ratio/FOV/settings and allow only declared resolution normalization and a fixed event alignment. Do not hide mismatch through per-frame repositioning, selective frame omission, convenient masks or arbitrary time warping. Compare entire sequences, critical moments and worst failures. Exclude the unrelated Elden Ring world using a documented fixed foreground method; score the glow/effect boundaries too.

Respect source resolution and cadence. The 10 Hz action frames have 100 ms sampling intervals and cannot prove a one-frame match at 60 Hz. Existing rendered frames cannot recover missing reference detail. Request higher-rate or same-skin reference only for the states that need it. For random particle/flame motion, compare documented shape/timing/distribution envelopes over repeated runs rather than demanding identical random pixels.

## Implement and validate incrementally

Capture a baseline before edits. Prioritize the measured defects and missing coverage, then implement one step, compare, integrate and regression-check before advancing.

Inspect current `viewmodel.rs`, `arms.rs`, `arms_anim.rs`, `trail.rs`, `hud.rs`, `crosshair.rs`, `camera.rs`, `firing.rs`, `blade.rs`, `deflect.rs` and the exporter/tool chain. Prior notes report half-resolution arms, simple lighting, approximate fonts, normal-alpha glow, incomplete weapon pairing and visual timing gaps. Treat these as baseline candidates to verify against current code/captures, not proof they still fail. User confirms cooldown mechanics work. Diagnose visual defects from evidence and preserve working gameplay/audio. Do not assume a full renderer replacement is necessary; present the concrete case before any large refactor.

Reuse the existing offline renderer/preview tests where they reproduce the live rendering path. Add a deterministic clock/state runner only where needed for reliable comparisons. Separate export correctness, isolated visual matching and actual runtime behavior. Verify that asset-dependent tests did not silently skip and that any required ignored tests were explicitly run.

Existing commands, from the project root, to inspect and use when applicable:

```powershell
Set-Location 'D:\games-mods\Games\GenjiBridge'
cargo test --release --manifest-path '.\genji\Cargo.toml'
powershell -ExecutionPolicy Bypass -File '.\tools\build.ps1'
```

Existing asset-writing preview tests (first preserve earlier evidence and verify their output destinations):

```powershell
$env:GENJI_STRIP = 'throw,fan,reload,dash,melee,deflect_start,deflect_end,blade_draw,blade_swing,blade_end'
$env:GENJI_STRIP_STEP = '0.1'
cargo test --release --manifest-path '.\genji\Cargo.toml' strips -- --ignored --nocapture
Remove-Item Env:\GENJI_STRIP, Env:\GENJI_STRIP_STEP -ErrorAction SilentlyContinue
cargo test --release --manifest-path '.\genji\Cargo.toml' trail_pictures -- --ignored --nocapture
```

These commands are starting points, not proof of matching. Check tooling first: the project uses a locally patched hudhook dependency. Build scripts can access caches/downloads; do not update dependencies or write outside authorized roots without the necessary permission. The effects extraction script currently deletes its destination before rerunning: inspect it and preserve existing reference evidence before any destructive regeneration.

For each runtime test, give me the exact command:

```powershell
powershell -ExecutionPolicy Bypass -File '.\tools\play.ps1'
```

Then give a short input sequence, expected visual states, exact log fields/pass limits and capture requirements. The game starts in plain mode; F9 activates Genji. Use the existing offline profile and separate save. I run gameplay; you analyze the returned evidence. Preserve each run's logs/captures before another run overwrites them. Only use the no-mod launcher when needed to isolate a regression.

Record build identity, source state, asset identity, settings and timestamps with each comparison. Log action events, animation id/variant/time, blend state, weapon visibility, effect timing, HUD state, actual rendered cadence, render/upload timings and relevant failure counts. Verify both frame delivery and animation time; a 60 fps game does not prove the asynchronous arms renderer supplies 60 new poses per second.

Report performance against the captured baseline, including long frames, stale/missing pictures and memory growth. Choose explicit performance limits before tuning. Preserve the prior texture-upload leak fix and test menus/transitions and repeated ability use.

## Your first response and completion report

First deliver the verified reference coverage, target assumptions/questions, current baseline defects, proposed scoring procedure, work order and agent assignments. Finish the reference audit and write the plan before implementation. Continue authorized independent work without repeatedly asking for routine permission; pause only for required missing input, large refactors or actions beyond the named scope.

Maintain `docs/FIDELITY-REPORT.md`: every case, reference path/frame range, tested build, command, metric/score, uncertainty, saved comparison/evidence path, pass/fail and remaining work. Keep raw media outside tracked source.

Final acceptance requires all four categories ≥95, every essential case passing, complete required reference coverage, integrated runtime evidence and no critical regressions. Include my saved-capture review when needed for appearance judgments. Until then report **“not accepted yet”**, with the precise blocker. Never substitute a unit-test count, export equality, an attractive mockup or a subagent's confidence for this evidence.

After each step tell me, plainly: what changed, what you actually tested, the measured result, what remains **not tested**, and the exact next test command with what I should see.
