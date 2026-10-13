# Overwatch reference audit

Date: 2026-10-13. Project: `D:\games-mods\Games\GenjiBridge`.
Asset location inspected read-only: `D:\GenjiBridgeAssets`.

## Finding

**Yes: reference material exists and is useful for side-by-side work. No: it does not yet establish 95% fidelity for the complete mod.** There are original Overwatch frame sequences, HUD reference crops, extracted models/animations/effects, and generated mod comparison pictures. These have different purposes and must not be treated as interchangeable evidence.

This was a documentation and reference audit. No game was launched, no extraction was rerun, and no DLL was changed. Game behavior and current fidelity: **not tested**.

## What was reviewed

- Project rules, README, change/handoff templates, ignore whitelist, build configuration and loader profiles.
- DESIGN.md's architecture, implementation plan, visual/HUD/animation sections and current effect-reader findings.
- MODLOG.md's entry index and relevant detailed entries on references, HUD reconstruction, rendering, animation problems, weapons, effects and recent untested changes.
- Source/tool structure across `genji/src` and `tools`, with closer inspection of the presentation modules, animation selection, renderer/exporter, recording cutter and launch/build scripts.
- Recursive asset inventories, reference indexes/picks, effects index and extraction log, and selected saved images.

This is **not a claim that every source line, historical log line, third-party dependency, or binary asset was individually reviewed**. The asset tree alone contains tens of thousands of files. The manager must read the complete authored files it relies on before editing them.

Existing changes at the start were `.gitignore`, `docs/DESIGN.md`, and the new `tools/extract_effects.ps1` / `tools/fx_reader/` files. They were left intact.

The repository changed during this audit: the final status also showed a modification to `genji/src/trail.rs`, while the earlier ignore/extraction changes no longer appeared as pending. This audit did not edit those files. The manager must take a fresh baseline; the recorded inventory is a dated snapshot, not a locked checkout.

## Available material

| Location under D:\GenjiBridgeAssets | Verified contents | Use and limitation |
|---|---|---|
| `ow_ref\INDEX.md` | Written action/timestamp index and recording settings | Primary starting point. Says 2580×1080, FOV 103, dark/purple skin that differs from the exported model. |
| `ow_ref\clip1\frames10` | 118 JPEG frames, `0001.jpg`–`0118.jpg` | Dragonblade draw, idle, swings and sheathe. Index specifies 0.1 s spacing; frame time = (number − 1)/10 s. |
| `ow_ref\clip2\frames10` | 266 JPEG frames | Idle, walking, primary throws, fan, reload. Same 0.1 s spacing. |
| `ow_ref\clip3\frames10` | 170 JPEG frames | Walking, wall climb, falling/landing. Same 0.1 s spacing. |
| `ow_ref\clipN\sheet_*.jpg` | Seven contact sheets across the three clips | Browsing aid, 0.3 s spacing according to the index. Use original frames for measurements. |
| `hud_ref` | 1,034 PNGs total, including frame directories and sheets | Recorded HUD crops plus generated mockups. `mock*.png` and comparison images are not independent Overwatch ground truth. |
| `recordings` | 730 files: 528 PNGs, 94 JPEGs, 106 WAVs, `clips.txt`, `cuts.json` | Older action/sound recordings, frames, overviews and cuts. Original videos are referenced elsewhere. |
| `anim_previews_eye` | 129 GIFs and 1,362 PNGs, plus other files/log | Extracted first-person animations rendered from the eye. Useful to identify clips; not proof of in-game fidelity. |
| `anim_previews` | 180 GIFs and 3,350 PNGs, plus other files/logs | Mixed first/third-person previews and diagnostic renders. |
| `anim_picks.json` | User tags and notes | Some identifications are uncertain. Cross-check `tools/viewmodel_anims.py` and `viewmodel\anims.txt`, which include later corrections. |
| `viewmodel` | Model/animation/shuriken binaries, skeleton/part lists, texture maps, 49 animation entries, generated comparisons and strips | Existing mod renderer/export evidence. `export_check.txt` reports 0.000 mm on its sampled export checks; this is not a fidelity percentage. |
| `hud` | 58 images, including `portrait_source.png` | Art prepared for the mod, not a complete captured HUD specification. |
| `ui-textures`, `ui-vectors` | 20,623 PNGs and 1,502 SVGs | Extracted UI pieces; they do not by themselves establish layout or animation timing. |
| `fx` | `effects.txt`, `index.txt`, `graphs.txt`, `abilities.txt`; 677 PNGs and 16,267 binary chunks | Extraction log reports 365 effects and 576 textures. The PNG count includes additional material/gradient images; do not equate it with the log's texture count. |
| `raw` | 7,554 files, including 3,572 Oggs, 1,904 PNGs, 524 `.owanim` and 469 `.owanimclip` files | Offline extraction source material. Identify first-person assets and the target skin explicitly. |

HUD frame-directory counts: `01` 80; `01f` 479; `02` 119; `03` 119; `04` 16; `05` 95; `05f` 95; `pick` 6. Folder names alone are insufficient descriptions: the manager should label the state/action and establish each sequence's time mapping.

The project's `owassets` folder yielded no files in its recursive inventory. References are primarily in the external asset folder. `build` contains a DLL/PDB; `logs` contains extraction and gameplay logs. A log supports state/timing claims, not visual equivalence.

## Samples actually checked

- Opened the saved `ow_ref\clip1\sheet_00.jpg`: Overwatch Dragonblade sequence, including the green swing trail, is visible.
- Opened `hud_ref\s01f.png`: reference ultimate-ready aura/flare sequence with printed times.
- Opened `recordings\rec3_sheet.png` and `rec3_overview.png`: the overview shows a green dash effect at its 3 s panel.
- Decoded image dimensions: clip1 frame 0079, clip2 frame 0011 and clip3 frame 0142 are 2580×1080; `recordings\rec3_frames\f001.png` is 430×180; `hud_ref\01f\f001.png` is 380×340; `viewmodel\trail_000.png` is 1290×540.
- Confirmed the existence of effect textures `000000004E2F.png`, `000000006613.png`, `0000000057E6.png`, `000000028FD9.png` and gradient `8008CA787D10.png` in `fx\textures`.

Saved references were inspected; live game visuals were not inspected.

## Gaps and contradictions the manager must resolve

1. **Dash footage is not entirely absent.** DESIGN.md and trail comments say there is no dash recording, but `tools/cut_recordings.py` identifies `rec3` at approximately 2.8–3.0 s as a dash, and its saved overview shows the effect. Existing frames are small; exact sampling times and coverage need verification. First investigate this material, then request better footage if necessary.
2. **Original videos are not inside either inspected folder.** No MP4/MOV/MKV/AVI/WebM files were found in the asset inventory. `ow_ref\clipN\source.txt` and `recordings\clips.txt` point to `D:\Videos\Edits`. Those external targets were not accessed or checked for existence in this audit. Historical notes say some older clips moved. Do not promise that the originals are available.
3. **The detailed HUD text specifications are referenced outside the inspected locations.** DESIGN.md points to `D:\Videos\edits_hyper\Overwatch_HUD_reference_20261008\Overwatch_HUD_Agent_Pack`. No copy of `HUD_ANIMATION_SPEC.txt` was found in the two inspected locations. The manager needs the file or must reconstruct and label a specification from existing frames. The external pack was not checked.
4. **Skin mismatch prevents a literal appearance score.** The index explicitly warns that the purple recording skin differs from the export; `rec3` also looks different. Decide which skin/build/settings are authoritative. Existing footage can still measure some action timing and screen positions, but cannot certify different colors, silhouettes or skin-specific effects.
5. **The 10 Hz action sequences cannot prove frame-perfect timing at 60 Hz.** Preserve sampling uncertainty. Finer rendered frames do not create missing reference information.
6. **Effects are partly decoded.** The extraction is present, but curve interpretation, particle settings, placement and the distinction between Swift Strike effects 0542 and 1769 are unresolved. The ability-to-script map is inferred; `abilities.txt` says no ability was found. Keep these identifications provisional.
7. **No complete acceptance suite or measured 95% report was found in the authored project files reviewed.** Existing comparison pictures are useful beginnings, not a current end-to-end runtime certification. The latest trail/status changes are explicitly marked not tested in game.
8. **Intentional customizations need an explicit specification.** Red enemy bars/skull, custom portrait, Elden Ring status/boss displays, health mapping, bleed and lifesteal are recorded user decisions. Fonts, alpha-based glow and the half-resolution/simple-lighting renderer are current approximations. Decide which are accepted adaptations and which must be brought to the Overwatch target; do not silently excuse them in a score.

## Suggested next step

Give the manager `docs/OPUS-5.5-MOD-MANAGER-PROMPT.md`. Its first deliverable should be a reference coverage table, a frozen comparison specification and baseline evidence, before visual implementation changes.

To verify the principal reference sequences from PowerShell:

```powershell
Set-Location 'D:\games-mods\Games\GenjiBridge'
Get-Content '.\docs\REFERENCE-AUDIT.md'
1..3 | ForEach-Object { $f = Get-ChildItem -LiteralPath "D:\GenjiBridgeAssets\ow_ref\clip$_\frames10" -Filter '*.jpg' -File; "clip$_ : $($f.Count) frames" }
```

Expected: this report, then counts 118, 266 and 170. This checks reference availability only; the mod remains **not tested** by this audit.
