# Genji in Elden Ring: design

Plain-language description of how this project works and the order we build it in. Terms are explained
the first time they appear.

## 1. The question: passthrough or rewrite?

You gave two examples of mixing games:

| | SkyCraft (passthrough) | iw4L (complete rewrite) |
|---|---|---|
| What it does | Runs **both** games at once. Minecraft runs hidden and computes the player; Skyrim draws the world. A plugin in each game talks to the other through shared memory. | Runs **neither** original game. A brand-new engine, written in Rust, reads MW2's files from your install and re-implements the gameplay. |
| Needs the source game running? | Yes, all the time | No, only its files on disk |
| Works because | Minecraft is offline-capable, moddable (Fabric) and has no anti-cheat | MW2's files are well documented and its gameplay is small enough to rewrite |

### Verdict: the rewrite approach, applied to Genji only, with Elden Ring as the host

**A passthrough is not possible here, for three reasons:**
1. **Overwatch is online-only.** It needs a Battle.net login and a server for every match. There is no
   offline mode to run "hidden in the background".
2. **Overwatch has anti-cheat.** To read Genji's state out of a running Overwatch, we would have to attach to
   a protected online client. That breaks hard rule 5 and risks a ban on your Battle.net account.
3. **There is nothing to gain.** SkyCraft passes Minecraft through because Minecraft's systems (block
   physics, crafting, redstone) are huge. Genji's kit is small: a few moves and four abilities. That is
   cheap to rewrite and very expensive to pass through.

**Rewriting all of Elden Ring the iw4L way is also wrong.** iw4L rebuilt MW2's engine, which is feasible
because MW2 maps are small arenas. Elden Ring is a 60 GB open world with its own physics, AI, scripting and
save system. Rebuilding it would take years, and you would lose the bosses and the world, which are the point.

**So we take the iw4L idea (rewrite the source game's part from scratch, and read its assets from your own
install) but put it inside Elden Ring.** Elden Ring keeps running its world, enemies, bosses and saves.
Our code replaces how *you* move, fight and look:

- Genji's **mechanics** are rewritten in Rust: movement, double jump, wall climb, shuriken, Swift Strike,
  Deflect, Dragonblade, ultimate charge.
- Genji's **assets** (model, animations, sounds, voice lines, icons) are converted at build time from your
  own Overwatch install, into a gitignored folder. Overwatch itself never runs.

This route has already been proven on this exact Elden Ring build: universal-modder's knowledge base has a
field note on a "Counter-Strike 2 in Elden Ring" conversion (same exe 2.7.1.0, Steam build 25080141). It
used a Rust DLL, me3 and CS2 assets converted from the user's install. We follow its route and its list of
gotchas.

## 2. The tools you pointed at, and what each one is for

| Tool | What it is | Use here |
|---|---|---|
| [universal-modder](https://github.com/rehan-remade/universal-modder) | Skills, safety rules and a shared knowledge base for AI agents that mod games | **Most useful.** It has the Elden Ring field note above (route, the fall-death trap, input hooking, rendering into the frame), the FromSoft playbook, and the safety rules. We use its knowledge and method. We do **not** need its fal image generator, which needs an API key (hard rule 6): our art comes from your Overwatch install. A copy lives in `third_party/` (gitignored). |
| [ghidra-mcp](https://github.com/bethington/ghidra-mcp) | Lets an agent drive Ghidra, a free disassembler (it turns a program's machine code back into readable functions) | **Useful later, as needed.** When the fromsoftware-rs library (below) doesn't describe something we need (for example, exactly where incoming hits are processed, for Deflect), we look it up in `eldenring.exe` with Ghidra. Needs Java 21 (you have it) and Ghidra 12.1.3. Elden Ring's exe is partly obfuscated (Arxan), so some functions are hard to read. |
| [rea](https://github.com/morluto/rea) | A broader reverse-engineering agent tool (native binaries, JavaScript apps, .NET, websites) | **Least useful.** For a native Windows game it overlaps with ghidra-mcp, and its Windows support is marked experimental. Not used unless ghidra-mcp falls short. |

Other tools the plan relies on:

- **me3**: the current FromSoftware mod loader. It starts Elden Ring **without** EasyAntiCheat, so the game
  runs offline, and it loads our DLL and our `regulation.bin`. It uses a **separate save file**, so your
  normal saves are never touched.
- **fromsoftware-rs** (`eldenring` crate, v0.14): Rust descriptions of Elden Ring's internal structures
  (player, physics, camera, bullets, params), maintained by the Elden Ring modding community.
- **hudhook**: draws our own graphics and menus inside the game's frame (Direct3D 12 + ImGui, a library for
  on-screen UI).
- **DataTool** (overtools/OWLib, v2.24, released 2026-09-15): reads Overwatch's data files and extracts hero
  models, animations, sounds and voice lines.
- **WitchyBND**: unpacks and repacks Elden Ring's `regulation.bin` (the game's balance tables, called
  **params**: weapons, bullets, status effects).

## 3. How it works (architecture)

```
me3 (launches the game offline, no EAC, separate save ER0000.genji.sl2)
 └─ eldenring.exe  ── Elden Ring stays in charge of the world, enemies, AI, saves
     └─ genji.dll  (our Rust code)
         ├─ boot      waits until the game is ready, finds the game's objects
         ├─ input     intercepts keyboard/mouse; Genji's keys go to us, not to Elden Ring
         ├─ kit       Genji's state machine: moves, abilities, cooldowns, ult charge
         ├─ world     reads/writes the player, spawns projectiles, applies and reads damage
         ├─ camera    first-person camera at Genji's eye height
         ├─ render    draws Genji's first-person arms, sword and effects into the frame
         ├─ hud       Overwatch-style HUD: health, cooldowns, ult meter, crosshair
         ├─ audio     plays Genji's sounds and voice lines
         └─ log       writes logs/genji.log: every state change, with numbers
regulation.bin  built from YOUR vanilla copy + our extra rows (shuriken bullets, damage effects)
owassets/       Genji's assets converted from YOUR Overwatch install (gitignored, never committed)
```

**Who decides what:** Elden Ring is authoritative for the world, enemies, damage to you and saves. Our DLL is
authoritative for how the player moves, attacks and what the player sees.

**Damage scale:** Overwatch numbers (a shuriken does tens of damage) mean nothing against Elden Ring bosses
with thousands of HP. Every number lives in a config file (`genji.toml`) with a damage multiplier, so you can
tune it without rebuilding.

**Folder layout (planned):**

| Path | Contents | In git? |
|---|---|---|
| `genji/` | the Rust DLL | yes |
| `tools/` | Python scripts: asset pipeline, param builder, save backup | yes |
| `profile/` | the me3 profile and `genji.toml` | yes |
| `docs/` | this file and notes | yes |
| `build/` | built DLL and `regulation.bin` | no |
| `owassets/` | converted Overwatch assets | no |
| `erdata/` | unpacked copy of your vanilla `regulation.bin` | no |
| `backups/` | copies of your saves | no |
| `logs/` | game logs | no |
| `third_party/` | universal-modder, DataTool, WitchyBND | no |

## 4. Genji's kit: what we rebuild

Overwatch changes numbers every season. The values below are approximate reference values from memory.
**Not verified** against the current patch; they go into `genji.toml` and get checked before tuning.

| Overwatch | Default key | Elden Ring implementation |
|---|---|---|
| Cyber-Agility: double jump | Space (in air) | Our movement controller adds a second jump |
| Cyber-Agility: wall climb | hold Space at a wall | Ray casts against map collision; climb for about 1 s |
| Shuriken (3 in a line, ~27 dmg each) | Left mouse | Spawn real engine projectiles (bullets) with our own param rows, so Elden Ring computes the hits |
| Fan of Blades (3 spread) | Right mouse | Same projectiles, spread pattern |
| Swift Strike (dash ~15 m, ~50 dmg, ~8 s cooldown, resets on kill) | Shift | Movement controller dash + damage along the path; listen for kills to reset |
| Deflect (~2 s, ~8 s cooldown) | E | Block frontal hits; send enemy projectiles back where we can |
| Quick melee | Mouse button 5 (your choice) | Short-range hit |
| Dragonblade (~5 s, ~110 dmg swings) | Q | Sword mode: swings with a wide hitbox; Swift Strike resets on kill |
| Ultimate charge | | Built from damage dealt, shown on the HUD |
| Health ~250 | | Mapped onto Elden Ring HP (configurable) |
| (Elden Ring interact) | F | The mod passes F to the game as E, so doors, graces, items and talking still work |
| Genji mode on/off | F9 | Off = plain Elden Ring (the game starts with it off) |

**One-time settings in the Genji save** (Elden Ring's System menu, Camera options): Camera Auto Rotate **Off**,
Camera Auto Wall Recovery **Off**. Until our own movement and look control (step 5), the first-person view uses
the game's camera direction, and auto-rotate swings it around when you strafe.

**Elden Ring's weapon actions** (Attack, Strong Attack, Guard, Skill) are switched off in the game's gameplay input
reader while Genji mode captures input, in the `GameFlowStep` stage right after the game reads input. (Rebinding
them away from the mouse was tried first; Elden Ring's key settings only accepted mouse buttons for them.)

Keys chosen 2026-10-07: Overwatch layout, melee on mouse button 5. Genji's keys are hidden from the game only
during gameplay; in menus everything passes through (Q stays "Back"). R stays Elden Ring's flask; Genji
reloads automatically.

**Movement** is Overwatch-style (constant speed, fast air control) instead of Elden Ring's animation-driven
movement. Per the CS2 field note, a movement controller that writes the position every frame **must turn off
the game's own gravity while it drives**. Otherwise the game builds up a hidden fall speed and kills you on
landing. We do that from the first movement step.

**Elden Ring things we hand back to the game:** ladders, mounting Torrent, doors and levers, item pickup,
grace menus, cutscenes, death. Our controller steps aside for these and resyncs afterwards.

## 5. The plan, step by step

Each step is small, revertible and ends with a test you run, plus a log I read. Steps 3 to 9 are
"mechanics with placeholder visuals". Steps 10 to 12 make it look and sound like Genji.

| # | Step | You test by | Pass condition (from the log) |
|---|---|---|---|
| 0 | Repo, rules, this plan | reading this file | |
| 1 | **Done 2026-10-07.** **Lab setup**: build tools, me3 profile with separate save, save backup script, "hello" DLL that logs boot and the player's position once per second | launch through me3, walk around, quit | `genji.log` shows boot OK and positions changing; your normal save untouched |
| 2 | **Asset recon**: DataTool lists heroes and extracts Genji (model, animations, sounds) from your Steam Overwatch into `owassets/`. **Blocked since 2026-10-07** until DataTool supports Overwatch 2.25; steps 3 to 9 go first. | `tools\extract_genji.ps1` | file counts and names logged; Genji's first-person model found (or not) |
| 3 | **Done 2026-10-07.** **Input**: our DLL reads keys and mouse; Elden Ring stops seeing Genji's keys | press each key in game | each key logged once per press; Elden Ring doesn't roll on Shift |
| 4 | **Done 2026-10-07.** **First-person camera** | look around, walk | camera at head height follows the mouse; your character body hidden |
| 5 | **Done 2026-10-07** (see 5b). **Movement**: Overwatch speed, double jump, wall climb, safe gravity and fall damage | run, jump, climb, drop from a ledge | speeds and jump heights match `genji.toml`; no fall death from small drops |
| 6 | **Shuriken + Fan of Blades** (params + projectile spawn) | hit an enemy | `HIT` lines with damage; projectiles never pass through walls |
| 7 | **Swift Strike** with kill reset (built 2026-10-07, not tested in game; see 5d) | dash through enemies | dash distance ~15 m; cooldown resets after a kill |
| 8 | **Deflect** | block an archer | frontal hits blocked while active; returned projectiles logged |
| 9 | **Dragonblade + ult charge** | build charge, press Q | charge numbers rise with damage; swing hits logged |
| 10 | **Viewmodel rendering**: Genji's first-person arms and sword drawn into the frame, with his animations | look at your hands | (visual check by you) plus render errors logged as zero |
| 11 | **Sound and voice lines** | use each ability | sounds play; logged event list |
| 12 | **HUD**: health, cooldowns, ult meter, crosshair (crosshair built 2026-10-07 ahead of the rest, `crosshair.rs`, hudhook Direct3D 12 overlay; not tested in game) | play | (visual check by you) |
| 13 | Polish: Torrent, ladders, menus, death, balance | play the game | |

## 5b. Step 5 in detail: movement

**Why the look and the movement come together.** Today Elden Ring still does the mouse look and moves the
character relative to *its* camera. As soon as our DLL decides where you look, it must also decide where W goes,
or the two drift apart. So step 5 takes over both at once.

**What the game keeps:** the world, collision geometry, enemies, damage to you. **What we take over while Genji
mode is captured:** where you look, where the character is, how fast it moves, gravity.

**How (from the CS2 field note, which did the same on this game version):**
- **Look:** mouse movement (the game's own raw mouse counts, `MouseDevice.di_mouse_state.lx/ly`) turns into yaw
  (left/right) and pitch (up/down), Overwatch-style: degrees = counts x 0.0066 x sensitivity. We build the whole
  camera matrix ourselves instead of copying the game's direction. The axis conventions are taken from the game's
  own camera at the moment we take over, and logged.
- **Moving:** WASD from our keyboard hook, relative to our yaw, Overwatch ground speed (about 5.5 m/s). Each frame
  (after physics, `ChrIns_PostPhysics`) we read the character's current position, add our movement, and write it
  back with `chr_proxy_pos_update_requested`. We never store the absolute position, because the engine re-centres
  its coordinates while you play (seen in our step 1 log).
- **Collision:** rays against the map (`CSPhysWorld::cast_ray`, filter `0x2000058` from the CS2 note): one down to
  find the ground (and climb small steps and slopes), several forward at knee, waist and head height to stop at
  walls and slide along them.
- **Gravity:** while we drive, the game's own gravity is off (`gravity_multiplier = 0`, `gravity_disabled = true`),
  restored when we hand back. The CS2 field note found that leaving it on builds a hidden fall speed that kills
  you on landing. We do our own gravity and our own fall damage.
- **Elden Ring's movement keys are switched off** in its gameplay input reader (the same way as the weapon actions):
  move forward/back/left/right, dodge/backstep, jump.
- **Character facing:** the body turns to our yaw (it's hidden in first person anyway).

**Sub-steps (each one tested in game before the next):**

| | What | You test by | Pass condition (log) |
|---|---|---|---|
| 5a | **Done 2026-10-07.** Own mouse look + walking on flat ground and slopes, walls stop you, game gravity off | walk, look, walk into walls, up a hill | yaw/pitch follow the mouse; speed about 5.5 m/s; no walking through walls; no falling through the ground |
| 5b | **Done 2026-10-07** (no fall damage, your choice). Our gravity, falling off ledges, fall damage, jump | drop from ledges of different heights, jump | landing speeds logged; damage only from big falls; no scripted fall death |
| 5c | **Done 2026-10-07.** Double jump | jump twice | second jump only in the air, once per airtime |
| 5d | **Done 2026-10-07.** Wall climb | hold Space facing a wall | climbs about 1 s, then stops; resets on landing |
| 5e | **Done 2026-10-07** (grabs untested). Hand back to the game for ladders, Torrent, doors, levers, cutscenes, death | use each one | each one works as in plain Elden Ring |

**Risks:** the axis conventions and the ray-cast filter are taken from the CS2 note and the library, not yet seen
on this machine; slopes and stairs are the usual pain point; if the game moves the character itself (knockback,
being grabbed by an enemy), we must notice and step aside. All numbers go into constants first and into
`genji.toml` once they're tuned.

## 5c. Step 6 in detail: Shuriken and Fan of Blades

**What Overwatch does** (approximate, from memory, **not verified**; all values become constants, later
`genji.toml`): left click throws 3 shuriken one after another in a straight line (a short burst); right click throws
3 at once in a horizontal fan; 30 shuriken, reloaded automatically when empty (about 1.5 s); shuriken fly fast and
straight, no drop.

**How in Elden Ring:** shuriken are the game's own projectiles ("bullets"). Our DLL decides *when* and *where*
(burst timing, fan spread, ammo, reload, from the eye along where you look) and asks the engine to spawn them
(`CSBulletManager::spawn_bullet`). The engine does the rest: flight, collision with the map and with enemies,
damage, hit effects, enemies reacting and fighting back. This is what the CS2 project did for its guns.

**Which projectile:** every bullet the engine spawns is a row of the game's balance tables (`regulation.bin`):
`BulletParam` (speed, range, gravity, visual effect) pointing to an `AtkParam` row (damage).
- **6a:** spawn an *existing* Elden Ring projectile (a throwing-knife-like row) on each click, straight from the
  eye. To pick the row, a copy of your `regulation.bin` is unpacked with **WitchyBND** (official GitHub release,
  into the gitignored `third_party/`) into the gitignored `erdata/` folder; your game file is only read.
- **6b:** the Overwatch timing: burst of 3, fan of 3, ammo and reload.
- **6c (changed 2026-10-07):** instead of a modified copy of `regulation.bin`, the DLL edits the dagger's rows
  **in the game's memory** while Genji mode is active (fromsoftware-rs gives typed access: `SoloParamRepository`):
  no drop, 60 m/s, 80 m range, more damage; the vanilla values are restored whenever Genji mode lets go, and
  nothing on disk changes. A separate row of our own (so the real Throwing Dagger item is never affected) would
  need the modified-file route; kept as an option if needed.

**Library gap:** `BulletSpawnData` (the spawn request) has private fields and no constructor in fromsoftware-rs.
We build our own copy of its memory layout (taken from the library source, 0x110 bytes) and pass that.

**One-shot flag (found with Ghidra, 2026-10-07, see `STATUS.md`):** every spawn also creates an *emitter* record
(the bullet manager's second pool: 64 slots plus 192 extra). The engine frees it only if bit 0 of the request's
flags field (`unk44`) is set and its bullets are gone; otherwise it lives until the area reloads and after 256
throws every spawn is refused. We always set that bit. Bit 3 of the same field asks for network sync: never set.

**6b plan (2026-10-07):** a small timing module (`genji/src/firing.rs`, no game access, unit-tested) decides each
frame how many shuriken to throw and at which sideways angle; `shuriken.rs` throws them as in 6a.
- Hold left click: bursts of 3 straight shuriken, 0.1 s apart, a new burst every 0.88 s while held (0.2 s burst
  + 0.68 s recovery).
- Hold right click: Fan of Blades, 3 at once (centre and 4.5 degrees to each side, 9 in total, turned around the
  view's up axis), every 0.68 s while held. Shuriken fly at 75 m/s.
- Both share one recovery: no fan during a burst, no burst right after a fan.
- 30 shuriken, one used per throw; at 0 the reload starts by itself and takes 1.5 s. No reload key: Elden Ring
  uses R for items, which you use in Genji mode.
- Throws happen only while our movement controller drives (as in 6a); a burst still in progress is dropped
  otherwise. Timers keep running.
- Values (updated 2026-10-07) from a measurement of an Overwatch video and Blizzard's notes, passed on by you; the
  burst cycle is inferred. All are constants in `firing.rs`.
- Log: `FIRE` lines for burst start, fan, reload start and end, with ammo left.

**Logging:** every throw (time, origin, direction, bullet row, result code from the engine); HP changes of
characters near where a shuriken lands (`HIT` lines), so damage is checked from numbers.

| | What | You test by | Pass condition (log) |
|---|---|---|---|
| 6a | **Done 2026-10-07.** One existing projectile per left click, from the eye along the view | throw at walls and an enemy | `THROW ... ok`; you see it fly; `HIT` with HP lost |
| 6b | Burst of 3, fan of 3, 30 ammo, reload (built 2026-10-07, not tested in game) | hold and click both buttons, empty the ammo | 3 throws per click at the right spacing; reload logged |
| 6c | Our own shuriken rows (speed, damage, look) | fight a few enemies | damage per hit as configured |

**Risks:** the spawn request's fields are only partly documented (some `unk` values may matter); the engine may
refuse a row without a matching owner or "goods" id (error codes 3 and 4 in the library); picking a good row to
start from takes reading the params.

## 5d. Step 7 in detail: Swift Strike

**What Overwatch does** (from memory, **not verified**; all values are constants): Left Shift dashes about 15 m
in the direction you look (up and down included) in about 0.3 s (50 m/s), passing through enemies and damaging
each one it passes once for 50; walls stop it. Cooldown 8 s, reset when an enemy Genji damaged dies (an
elimination).

**How in Elden Ring:**
- `movement.rs`: Left Shift (only while our controller drives and the cooldown is ready) starts a dash: for 0.3 s
  the controller moves at 50 m/s along the view, ignores gravity and other characters, and still stops at walls
  (same wall rays as walking). On the ground and looking flat or down, the dash runs along the ground at full speed;
  looking up lifts off. Aiming at the ground, it stops where the view meets the ground (2026-10-08). At the end,
  in the air, you start falling normally. No wall climb during the dash.
- `swift.rs` (new): cooldown; each frame of the dash, the stretch travelled is checked against every nearby
  character's collision cylinder (our body: radius 0.35 m, height 1.75 m, plus 0.5 m reach); each one touched takes 50 once (HP set
  directly, like the shuriken), logged as `DASH HIT`.
- `kills.rs` (new): remembers every character we damaged (shuriken or dash) for 3 s; if one of them dies in that
  time, that's an elimination: the Swift Strike cooldown resets (`SWIFT reset`).
- `targets.rs` (new): the "living characters near the player" list, moved out of `shuriken.rs` so both use it.

**Risks:** the game itself registers no hit for dash damage: no hit reaction and possibly **no runes** for a dash
kill (shuriken kills give runes because the engine also registers their 1-damage hit). We'll see in the test;
if runes are missing, the dash can spawn an invisible 1-damage projectile on each enemy it hits.

| | What | You test by | Pass condition (log) |
|---|---|---|---|
| 7 | Dash, damage, cooldown, kill reset | dash on flat ground, into a wall, up, through enemies; kill with shuriken then dash | `DASH` distance about 15 m in about 0.3 s; stops at walls; `DASH HIT` 50 per enemy; `SWIFT reset` after a kill |

## 5e. How Genji's damage is dealt (changed 2026-10-08)

**Before:** the game's own hit did almost nothing (attack power 1) and we lowered the enemy's HP directly by the
Overwatch damage. HP was right, but the floating damage number showed only the game's tiny part, and nothing reacted
to dash hits.

**Now (your choice, option 2):** the game deals the damage, like a real weapon. Our number becomes the **attack
power** of the hit: Overwatch base x dexterity scaling (`scaling.rs`) x ATTACK_FACTOR (2.0, a first guess to bring
Overwatch numbers to Elden Ring's scale: an early dagger has about 100-150 attack power). Enemy defenses, damage
negation and shields then apply as for any weapon, and the damage number is right.
- **Shuriken:** the dagger's AtkParam row gets that attack power (in memory, updated when dexterity changes).
- **Headshots:** the game has no headshots for projectiles. We still detect them from where the shuriken ends; on a
  confirmed headshot we add the same damage again ourselves (x2 like Overwatch). That extra part isn't in the
  number.
- **Swift Strike (next part):** each enemy the dash touches gets an invisible point-blank projectile (the Kukri's
  row, in memory: no flight visual, our attack power), and every hit makes the enemy bleed (since 2026-10-08), so the game registers a real
  hit: number, sound, reaction, bleed.
- The `HIT` lines log the attack power and what the game dealt, to tune ATTACK_FACTOR.

## 6. Risks and unknowns (honest list)

- ~~Build tools are missing.~~ Resolved 2026-10-07: Visual Studio 2022 Build Tools installed.
- ~~Old mod loader in your game folder.~~ Resolved 2026-10-07: `dinput8.dll`, Seamless Co-op, ModEngine2 and
  the other old mods were moved to `backups/game-folder-old-mods/`.
- **fromsoftware-rs version pin.** The crates.io release (0.14.0) only supports Elden Ring 2.6.2.0. We depend
  on its GitHub main branch pinned to commit `59fbd3b`, which supports 2.7.1.0. When Elden Ring updates, this
  pin has to move to a commit that supports the new version.
- **First-person models:** Overwatch has first-person arm models and animations. Whether DataTool exports them
  for Genji in a usable form is **not verified** yet (step 2 answers it). Fallback: use the third-person model
  with the head hidden.
- **Animations in a custom renderer** are the biggest piece of work (the CS2 project needed its own Direct3D 12
  renderer). It works in that project; it is a lot of code.
- **Deflect** needs to know about hits before Elden Ring applies them. fromsoftware-rs may not expose that;
  this is where Ghidra comes in.
- **Game updates:** a new Elden Ring patch can break the DLL until fromsoftware-rs updates. Don't let Steam
  update Elden Ring in the middle of a step if you can avoid it.
- **Size of the job:** the CS2 conversion took about three days of agent sessions, with the user testing every
  feature in game. Expect similar.
