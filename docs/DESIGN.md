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
owassets/       (unused: C: is full; Genji's assets live in D:\GenjiBridgeAssets, outside the repo, never committed)
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
| `D:\GenjiBridgeAssets` (outside the repo) | extracted and converted Overwatch assets | no |
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
| 2 | **Asset recon**: DataTool lists heroes and extracts Genji (model, animations, sounds) into `D:\GenjiBridgeAssets\raw`. Your live Overwatch is newer than DataTool v2.24.1 supports, so (2026-10-08) you downloaded the Sept 8 build with Steam's `download_depot` (depots 2357571, 2357572, 2437288), combined with hard links into `D:\SteamDepotDownloads\app_2357570\Overwatch_2.24.1`; DataTool reads it (build prometheus-2_24_1_0-153264). | `tools\extract_genji.ps1` | file counts and names logged; Genji's first-person model found (or not) |
| 3 | **Done 2026-10-07.** **Input**: our DLL reads keys and mouse; Elden Ring stops seeing Genji's keys | press each key in game | each key logged once per press; Elden Ring doesn't roll on Shift |
| 4 | **Done 2026-10-07.** **First-person camera** | look around, walk | camera at head height follows the mouse; your character body hidden |
| 5 | **Done 2026-10-07** (see 5b). **Movement**: Overwatch speed, double jump, wall climb, safe gravity and fall damage | run, jump, climb, drop from a ledge | speeds and jump heights match `genji.toml`; no fall death from small drops |
| 6 | **Shuriken + Fan of Blades** (params + projectile spawn) | hit an enemy | `HIT` lines with damage; projectiles never pass through walls |
| 7 | **Swift Strike** with kill reset (built 2026-10-07, not tested in game; see 5d) | dash through enemies | dash distance ~15 m; cooldown resets after a kill |
| 8 | **Deflect** (done 2026-10-08, see 5f) | block an archer | frontal hits blocked while active; returned projectiles logged |
| 9 | **Dragonblade + ult charge** (done 2026-10-08, see 5g) | build charge, press Q | charge numbers rise with damage; swing hits logged |
| 10 | (plan in 5k) **Viewmodel rendering**: Genji's first-person arms and sword drawn into the frame, with his animations | look at your hands | (visual check by you) plus render errors logged as zero |
| 11 | **Done 2026-10-09** (see 5j). **Sound and voice lines** | use each ability | sounds play; logged event list |
| 12 | **HUD**: health, cooldowns, ult meter, crosshair (crosshair done 2026-10-07; the rest planned in 5h, built before 10 and 11) | play | (visual check by you) |
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
- 30 shuriken, one used per throw; at 0 the reload starts by itself and takes 1.5 s.
  - It starts once the last throw's recovery is over (2026-10-09), so the throw's animation finishes first. The same
    goes for R pressed during a throw.
- R reloads too (2026-10-09). You moved Elden Ring's "use item" to Z for this, and R is hidden from the game in Genji
  mode.
  - Pressed during a burst, the reload starts when the burst ends.
  - R does nothing with a full magazine or during Dragonblade.
- No reload during a quick melee or its 0.8 s recovery (2026-10-09):
  - R does nothing then, and an empty magazine waits for the melee to end before reloading.
  - A melee cancels a reload in progress, and its remaining clicks are cut (as in Overwatch).
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
- `kills.rs` (new): remembers every character we damaged (shuriken or dash) for 5 s (was 3; community figure for Overwatch's elimination credit); if one of them dies in that
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

## 5f. Step 8 in detail: Deflect

**What Overwatch does** (Overwatch wiki and patch notes, searched 2026-10-08; the current cooldown is not confirmed,
a 2025 patch raised it from 8 s to an unknown value): E holds the sword up for 2 s. Everything that comes from less
than 90 degrees off where Genji faces is caught. Projectiles are sent back where Genji aims. Melee attacks are
blocked but not sent back. Beams and area damage get through. E again ends it early. No shooting while deflecting
(Swift Strike allowed). Cooldown 8 s.

**8a, blocking (first):** `deflect.rs` (new): E starts it, E again or 2 s ends it, then the 8 s cooldown. While it
runs, the player's "invincible" flag (ChrIns flags, used by the game while loading areas) is set on every frame where
nothing threatens from behind: no living enemy within 4 m in our rear half and no enemy projectile closing in from
behind. That approximates "front only": an enemy hitting us from the front can't, one behind us can. Shuriken are
held while deflecting. The overlay shows "DEFLECT" under the crosshair. Logged: start, end (why), every frame the flag
was off because of a rear threat, and HP changes while deflecting.

**8b, sending projectiles back (built 2026-10-08):**
- Enemy projectiles within 3 m of our chest, flying at us from the front, are removed.
- For each one, the same kind of projectile (its own Bullet row) is fired back along our aim, owned by us.
- While deflecting, that kind's Bullet row points at our damage row (the Bone Dart's AtkParam_Pc, changed in memory
  to carry the original's damage values). Fired by us, the game looks damage up among the player's rows, where the
  NPC row doesn't exist.
- If the game refuses a kind, the Bone Dart (changed in memory) goes instead.
- Kill credit goes to the character under the crosshair (approximation).

**Unknowns:** whether the invincible flag also stops hit reactions and status buildup (the log and you will tell);
how enemy melee that starts in front but swings around is judged.

## 5g. Step 9 in detail: Dragonblade, ultimate charge, quick melee (plan 2026-10-08)

**Overwatch numbers** (Season 5 patch of 2026-10-06, passed on by you; swing interval and cost derived, not
verified): Dragonblade 6 s; 100 damage per enemy per swing; a swing about every 0.61 s; 5 m forward cone (angle
unknown, we use 60 degrees each side); ultimate cost about 2000 points; passive charge 5 points/s, paused while
Dragonblade is active; quick melee 40. Activating it resets Swift Strike; Swift Strike keeps its own damage during
it; no shuriken during it; Deflect is allowed (no swings while it's up). Swings ignore shields (AtkParam "Disable
Guard"); the cone is 90 degrees each side (you found 60 too narrow). Quick melee can be blocked and breaks guards and
stances: strong stamina/poise damage, overwhelming on a second melee within 5 s (your request, 2026-10-08).

**How in Elden Ring** (your choices, 2026-10-08):
- **Hits:** a swing (hold left click) hits every character within 5 m in the cone, with the same mechanism as Swift
  Strike: an invisible point-blank projectile from behind each target, here carrying **Corpse Piler's damage row**
  (Rivers of Blood's Ash of War, AtkParam_Pc 303400300) so it lands as a Corpse Piler hit; attack power 100 x
  dexterity. Carrier: the Ruin Fragment's Bullet row (10176000), changed in memory while Genji mode is on (the dash
  keeps the Kukri's). Direct damage as fallback.
- **Bleed:** each swing hit fills a third of the target's bleed bar (a bleed every third swing), applied directly
  like the dash's.
- **Visible sweep** (until Genji's own model and animations exist, then removed): each swing fires the **Blood Blade**
  crescent (Ash of War, Bullet 2635) along the view, 5 m, with its damage row removed in memory, so it's only a look.
- **Quick melee (mouse 5):** 40 x dexterity to the nearest character within 2.5 m in front, same mechanism (Kukri
  carrier), outside Dragonblade; about 0.8 s between uses (not verified).
- **Ultimate charge** (`ult.rs`): counted in Overwatch damage (27/54 per shuriken hit, 50 per dash hit, 40 per melee
  hit; swings don't charge, as in Overwatch) plus 5 per second, paused while Dragonblade is active; Q when full.
- **Overlay:** "ULT n%" under the crosshair, "DRAGONBLADE n s" while active.

## 5h. Step 12 in detail: the Overwatch HUD (plan 2026-10-08, ahead of steps 10 and 11)

Drawn by the overlay (hudhook, like the crosshair), only while the first-person view is on, laid out like your
Overwatch screenshot, sizes scaled from 1080p to your screen height:
- **Bottom left:** Genji's portrait (from your extracted files), health as "current | max" and a bar of 10 white
  segments (Overwatch draws 25 HP per segment; Elden Ring's health is larger, so each segment is a tenth).
- **Bottom centre:** the ultimate: a ring that fills with the charge and the percentage inside; when full, the
  Dragonblade icon and "Q"; during Dragonblade, the ring drains with the seconds left.
- **Bottom right:** Swift Strike (LSHIFT) and Deflect (E) icons with their cooldown seconds (darkened while cooling,
  highlighted while Deflect is up), the shuriken icon with ammo "n | 30", "RELOADING" while reloading.
- Art (2026-10-08, your request: Overwatch's own HUD designs). DataTool's `dump-ui-textures` (20,623 textures,
  3.6 GB) and `extract-vector-images` (1,502 SVGs) went to `D:\GenjiBridgeAssets`. Contact sheets were used to find
  the pieces. `tools\make_hud_art.py` builds `D:\GenjiBridgeAssets\hud`:
  - the portrait cut to Overwatch's hexagon with its outline (UI textures 39E89, 27AB5);
  - the ultimate's dark disc (21660);
  - the "+" health icon (vector 0022).
- Shapes drawn with the proportions of your Overwatch 2 screenshot: slanted health segments, slanted ability frames
  (white when ready, dark with seconds while cooling), key boxes, cyan ammo lines.
- Numbers in Segoe UI Black Italic (Overwatch's heavy oblique digits aren't a font file DataTool extracts); labels
  in Bahnschrift.
- Overwatch stores its HUD layout as UI data, not pictures, so positions follow the screenshot.
- The temporary "ULT n%" and "DEFLECT" texts go. (Update, 2026-10-08: Elden Ring's own HUD is now switched off in
  Genji mode, and the gameplay rule no longer depends on it.)
- **Animations from your analysed clip (2026-10-08).** Your clip is in
  `D:\Videos\edits_hyper\Overwatch_HUD_reference_20261008\Overwatch_HUD_Agent_Pack`, with its written spec
  (`HUD_ANIMATION_SPEC.txt`). Its frames are at 1080 lines, so sizes, colours and timings were measured on them
  directly. Frames and comparison sheets are in `D:\GenjiBridgeAssets\hud_ref`, extracted with CapCut's ffmpeg.
  - **Ultimate ready** (rebuilt from your second analysis, the "ULT FULL" spec; U = the white disc's radius, 40 px):
    - **Static layers:** halo; faint hairline ring at 1.9 U; thin pale ring at 1.4 U with gaps at 3 and 9 o'clock;
      blue glow; thick cyan ring; navy outline; white disc; glyph; inward triangles; Q cap. None of them move or
      pulse. The halo flickers by about 4%.
    - **Boiling aura:** soft blue flame tongues rising and falling in place, drifting 2° a second. It is the only smooth
      animation.
    - **Flare bursts:** every 1.25-1.49 s, drawn like a flipbook. Each pose is held for 4-6 frames and cuts straight to
      the next:
      - BUILD: a small, thin pair, a hooked loop and a spike on opposite sides;
      - PEAK: the same pair about 40% bigger and brighter, filled, and the disc's half on the spike's side tinted;
      - blank;
      - SECOND: a smaller, dimmer flare at a new angle;
      - TENDRIL: a jagged thread that fades and breaks up.
    - A new direction each burst, at least 60° from the last.
    - **Roll back:** the earlier smooth version is saved as the git tag `hud-ult-flames-v1`.
  - **Charging meter** (the "ULT GAINED" spec, section 2):
    - a dotted outer ring of 90 dots and a thin grey ring;
    - 67 teal bars filling clockwise from 12 o'clock, breathing ±3.5%;
    - a thin teal circle with 4 breaks;
    - a dark see-through centre;
    - the italic grey number;
    - the Q cap: dark with a thin outline.
  - **Becoming ready** (same spec, 1.2 s, in 60 fps frames from the moment it reaches 100%):
    - The meter hard-cuts to a white disc that shrinks while a dark-blue smoke ring forms (0-8).
    - The disc pops back to full size with a grey glyph (9-12).
    - The ring charges to cyan, the triangles snap in and the thin ring's gaps open (12-18).
    - A one-frame cyan flash (19), then a shockwave ring expanding to 8.5 U and breaking into arcs (20-33).
    - A stepped flame eruption at 30 drawings a second: a lopsided solid mass with a spike, then hooked curls
      flying out and breaking up (28-50).
    - An afterglow that cools the cyan glyph back to navy (51-71), with the first ready-state flares from 61.
    - A snap to the ready state (72).
    - Built from shared parts: `Look` (the badge with adjustable sizes and colours), keyframe tables (`key`,
      `key_col`), `ult_gain` and `eruption`.
  - **Health "+" pill** (your Overwatch screenshots):
    - Normal: the left 56% is white with a dark "+", and the rest dark see-through grey.
    - Healing: health going up turns it yellow (left yellow, right dark yellow, yellow outline). It stays yellow 0.8 s
      after the last rise.
    - Damage: health going down turns the whole pill dark, the "+" light grey, and shows a pink arrow pointing down
      for 1.2 s.
    - Each state fades in and out over 0.15-0.2 s.
    - A change of maximum health doesn't count, and neither does coming back to the HUD.
  - **Ultimate pressed (0.4 s):** the ring expands and fades, and the emblem gives way to the empty meter with its
    "0". Q becomes outlined.
  - **During Dragonblade:** the meter shows the charge, as in the clip. The draining ring with seconds is gone.
  - **Ability used:** the tile turns cyan with the badge outlined. Swift Strike stays cyan 0.4 s; Deflect while up.
  - **Cooldown:** a grey tile with cyan rising from the bottom and whole seconds over the emblem. Right after the cyan,
    the tile fades to grey while the number fades in.
  - **Cooldown finished (0.6 s):** tall cyan light bars down to the screen's bottom edge, a white star, a glow column,
    square particles, and the tile glowing before it turns white again.
  - **Layout:** the tiles stand upright, symmetric and the same size, per your screenshot. The slight lean in the clip
    was the pressed tile, not the resting one.
  - **Pressed:** the tile is pushed down, its top dropping about 10 of its 64 px within 0.12 s. It stays pushed
    through the cooldown and pops back up when the ready flash starts.
  - **Dragonblade time left** (your screenshots): a white ring of about 30 px around the crosshair. The white is the
    time left, running clockwise from 12 o'clock; the time used is a grey trail. The seconds ("6s") sit below it.
  - **Ammo panel lined up with the tiles** (your screenshot, with Overwatch's 4° tilt taken out):
    - its bottom line is on the tiles' underlines, and its top line 8 px below the tile tops;
    - it starts 16 px right of the E tile;
    - both lines are straight, with the same brighter, thicker piece (52% to 92% of the width) on each.
  - **Portrait** (your picture and Overwatch screenshot, 2026-10-08):
    - Genji's bust, the picture you sent, in the dark hexagon with its cyan outline.
    - The helmet stands out over the hexagon's top edges, in front of the outline: its top is level with the top corner,
      the visor 39% of the way down, the face at 88% of the width. The body fills the inside.
    - A soft blue glow rises behind the head.
    - Built by `tools\make_hud_art.py` from `D:\GenjiBridgeAssets\hud\portrait_source.png`.
  - **Kill skull** (your clip, 2026-10-09): on each elimination, Overwatch's skull pops up in red, like the enemy bars (Overwatch's is pink), over the crosshair.
    - Timing, in 60 fps frames: 2.6 times its size by frame 3, held to frame 6, down to about 44 px at 1080p by frame
      8, faded out by frame 32 (about half a second).
    - Drawn before the crosshair, so the crosshair stays on top. A new kill restarts it.
    - The picture is the hole in the boss badge's skull-in-a-diamond, cut out by `tools\make_hud_art.py`
      (`kill_skull.png`).
  - **Not recorded:** a kill resetting Swift Strike early, and the ultimate becoming full. The reset reuses the
    cooldown-finished flash, and the ult's full moment keeps a short burst. Both are guesses.

## 5i. Enemy health bars, Overwatch style (plan 2026-10-08)

Switching Elden Ring's HUD off in Genji mode also hid the enemies' health bars, the boss bar and the damage numbers.
You chose to keep Elden Ring's HUD off and draw our own bars, in Overwatch's style, red instead of Overwatch's pink.

- **Which enemies:** living characters within 50 m that lost health in the last 30 s and aren't at full health, on
  screen, nearest first, at most 8.
  - Health drops are tracked for every nearby character, whatever caused them (our hits, the game's, bleed).
  - The game's own line-of-sight check isn't available to us, so a bar can show through a wall. The distance limit
    keeps that rare.
- **Where:** above the head (the top of the character's collision cylinder, plus 0.35 m), projected with the camera
  we draw from. This is the same projection the camera log already uses.
- **Look** (your Overwatch screenshots):
  - 10 segments in a row, lit red for the health left; the segment being emptied lights partly;
  - lost segments dark see-through red;
  - health just lost flashes lighter, then drains away (0.5 s);
  - no damage numbers (Overwatch shows none).
- **Names:** Overwatch shows the name under the bar. Our library has no lookup for the game's text (enemy names live
  in its message files), so names are left out for now.
- **Boss** (changed 2026-10-08, your request):
  - **Which characters:** the game's boss list (`CSFeManImp.boss_health_displays`, set by its events, also for field
    bosses) names them. It is filled while the HUD is off (tested on Margit).
  - **Bar:** above the boss's head, not at the top of the screen: 20 thin segments, shown the whole fight within
    80 m.
  - **Badge:** Overwatch's skull-in-a-diamond (vector `00082C`, your pick) on its left, red with a yellow skull.
  - **Name** (2026-10-09): under the bar, in its red, capitals, only the important part: the text before a comma,
    " of " or " the ", without titles like "Starscourge". So "Margit, the Fell Omen" shows as MARGIT.
    - **Source:** the game writes each boss bar's name into its display data (`boss_list_tag_data[..].chr_name`).
    - **Fallback:** if that is empty with the HUD off, Elden Ring's HUD is switched on for 0.4 s once per boss so the
      game fills it in.
  - **Raised:** 22 px higher than the other bars, so the name fits above the head.
- **Diagnostics:** every 2 s, the game's own enemy-tag and boss tables are logged (filled or not, visible, screen
  position). If they turn out to be filled with the HUD off, the game's own visibility could be used later.
- **Code:** `enemy_bars.rs` (game thread: tracking, projection, which bars) feeds `hud::State`; `hud.rs` draws.

## 5j. Step 11 in detail: sound and voice (plan 2026-10-09)

**What we have:** 3,572 Overwatch sound files (Ogg) from your extraction in `D:\GenjiBridgeAssets
aw`.
- **Voice lines:** 1,795, sorted by situation in `HeroVoice\Genji\<situation>` (Ultimate, Pain, Death, Elimination,
  Melee, Jumping, Respawn and so on), but the lines inside each folder are unnamed.
- **Ability and movement sounds:** unnamed, grouped by the animation or effect that plays them:
  - first-person model `00000000DBA6` (20 animations);
  - third-person model `00000000DB92` (51 animations);
  - `Effects` (93 effects);
  - `Entities\Gameplay3P\Effects` (88 effects).
  Nothing in the files says which is the shuriken throw or the dash, so they have to be found by ear.
- **Voice lines with their words** (found 2026-10-09). DataTool's own `list-subtitles-real` showed the game keeps
  subtitles for Genji's lines (voice set `0x54`).
  - Re-extracting with `extract-hero-voice <out> Genji --subtitles-with-sounds` (into
    `D:\GenjiBridgeAssets\voice_subs`) names each file after its words, for example `00000001FEDD.0B2-(Japanese)_
    Take the blade of the dragon god!.ogg` ("Ryūjin no ken wo kurae!").
  - This finds the ult line, "My ultimate is ready/charging", pain, death, eliminations, healing, respawn, and the
    "(Swift Strike)" shouts without listening.
  - Ability sounds have no subtitles; those stay "by ear".
- **Animation previews** (2026-10-09, your idea: "finding the animation we already find the sound").
  - Overwatch files ability sounds under the animation that plays them. `tools\render_anim_previews.py` runs in Steam's
    Blender 5.2, in the background, with the DataTool team's importer add-on (io_scene_owm 3.3.1) loaded from
    `D:\GenjiBridgeAssets\tools`, so nothing is installed into Blender.
  - It renders every animation that plays sounds (20 first person, 51 third person) as small frames, and the picker
    shows each as a looping GIF above its sounds.
  - **For step 10:** the importer reads Genji's first-person model (`00000000DBA6`: arms, 88 bones, textures) and its
    animations. His eye is about (0, 0.05, 1.65) in Blender's space, looking along -Y; the sword is a separate model.

**11a. Sound picker (find the sounds):**
- `tools\make_sound_picker.py` writes a local page, `D:\GenjiBridgeAssets\sound_picker\index.html`. It lists every
  group with a play button for each sound (your browser plays Ogg from the files on D:).
- You tag sounds with the events below; the page remembers your tags in the browser.
- The page exports the tags as a small JSON file, which you save as `D:\GenjiBridgeAssets\sound_picks.json`.
- `tools\make_sounds.py` copies the tagged files into `D:\GenjiBridgeAssets\sound\<event>\01.ogg`, `02.ogg` and so
  on.
- Nothing is copied into the repository.

**11b. Playback (the mod plays them):**
- A sound thread in our DLL plays Ogg files with the `rodio` audio library through Windows' default output. That is
  separate from Elden Ring's own audio engine; nothing of the game's is changed.
- Each event picks one of its files at random (several takes, like Overwatch).
- **Volume:** Elden Ring's own sound-effects setting (`GameSettings.sfx_volume`, 0-10), times one overall level of
  ours.
- **Missing files:** an event with no files is skipped and logged once.

**Events** (tags in the picker):
- **Shuriken:** throw (each shuriken), Fan of Blades throw, hit on an enemy, hit on a wall, reload.
- **Swift Strike:** dash, dash hit, dash reset (a kill resets it).
- **Deflect:** start, a projectile deflected, end.
- **Dragonblade:** unsheathe, swing, swing hit, sheathe (end).
- **Quick melee:** swing, hit.
- **Movement:** jump, double jump, wall climb, landing.
- **Ultimate:** ready (the sound when it reaches 100%).
- **Voice:**
  - the Dragonblade line ("Ryūjin no ken wo kurae!");
  - "ultimate ready" and "ultimate charging" (Q pressed early);
  - pain (on hits, at most every few seconds);
  - death;
  - elimination (sometimes, after kills);
  - healing.

**11b-2: movement and voice extras** (plan 2026-10-09). All rules below are guesses at Overwatch's, unless noted.

*Movement* (`movement.rs` reports what happened each frame, `camera.rs` plays):

| When | Sound |
|---|---|
| Running on the ground | one footstep every 1.3 m (your recording: a step about every 0.235 s at full speed, 5.5 m/s) |
| Jump | "Jumping" voice grunt (at most every 1.5 s, and only if no other line is playing) |
| Double jump | the double-jump whoosh (2D98), plus the same grunt |
| Wall climb start | one climb sound (16DC or 930F) |
| While climbing | a short grab (16CF, "the loop") every 0.2 s |
| Climb reaching the top (vault) | ledge sound (16D2) |
| Landing | the landing thud (3473), louder the faster the fall; a "JumpEnd" grunt after a fall of 3 m or more |

`make_sounds.py` splits your wall-climb tag in two for this: `wall_climb` (16DC + 930F) and `wall_climb_loop`
(16CF).

*Health and voice* (from the player's HP each frame):

| When | Sound |
|---|---|
| Hit (HP drops) | a pain line, at most every 3 s |
| Hit that makes the game play a hit reaction (the game takes over right after the drop) | a stagger line instead |
| Still in that reaction after 0.9 s (knocked down) | the knockdown thud (92E8) and a knockdown line |
| HP falls below 30% | Overwatch's critical-health sound (re-armed above 40%) |
| Healed 15% or more at once, not by Dragonblade's lifesteal (a flask) | the health-pack sound and a healing line |
| Death | a death line; a scream instead if you were falling for over 1 s |
| Back in control after dying | a respawn line |

*Kills:* the elimination line every third kill stays, but these come first, in this order:
- 3 kills within 4 s: a multikill line;
- a kill within 1 s of a quick melee hit: a melee-kill line;
- every 10th kill without dying: a kill-streak line.

*Not used:* the 7.9 s healing loop (388B): nothing in Elden Ring heals that slowly over time.

**Silencing the Tarnished's voice** (2026-10-09, `game_voice.rs`; your test: Elden Ring's pain grunt covered
Genji's line):
- The game picks the player's voice through a sound switch, named by the table
  `WwiseValueToStrParam_Switch_PlayerVoiceType`.
- While Genji input is captured, the player's rows get a name no sound bank has, in memory, and are put back after.
  - Only rows below 50 are touched (`_00_PC_...` to `_15_PC_...`); the rows from 50 up are NPC and enemy voices.
- Tested 2026-10-09: it works at once, and an unknown name plays nothing.
- The same kind of table exists for the player's shoes (footsteps), equipment and attacks, should those need silencing
  too.

**Later (not in 11):** muting Elden Ring's other own sounds for the things Genji replaces (the Tarnished's grunts, the
Throwing Dagger's sound). This needs the game's sound IDs and is looked at once ours play.

## 5k. Step 10 in detail: Genji's first-person arms and sword (plan 2026-10-09)

**Goal:** Genji's arms (and his sword during Dragonblade) on screen in first person, moving with Overwatch's own
first-person animations for each action, as in Overwatch.

**What we have:**
- His first-person model `00000000DBA6`: arms and hands, 88 bones, its textures.
- 129 first-person animations in Overwatch's format, which the Blender importer (io_scene_owm) reads.
- Only 20 of the 129 play sounds, so only those were previewed for the sound picker. Idle, walking and the rest are
  still unnamed.
- The sword is a separate model.

**How to draw it** (A chosen: you agreed 2026-10-09, "yes, continue 10c"):
- **A. Our own small software renderer (recommended).**
  - The DLL draws the arms itself, on the CPU, into a picture the size of the screen's lower part: triangles, a
    depth buffer, Genji's textures, simple lighting.
  - The overlay (the same one that draws the HUD) replaces a texture with that picture each frame and lays it over
    the game.
  - First-person arms are always drawn on top of the world in shooters anyway, so nothing has to mix with Elden Ring's
    own 3D.
  - Everything stays in our Rust code. The same renderer can write a picture to a file outside the game, so the
    framing can be checked before any game test.
  - Cost: the drawing (a few ms, to be measured) plus the texture copy (the overlay library creates an upload buffer
    and waits for it; about 1-2 ms, to be measured). Both logged.
- **B. A real Direct3D 12 drawing pass inside the overlay.** Best quality and speed, but the overlay library doesn't
  open its drawing to us. It means our own graphics pipeline and hooks: much more work and more ways to crash the game.
- **C. Elden Ring draws Genji.** His model and animations converted to Elden Ring's formats (FLVER, HKX) and loaded
  as the player's body. Very hard conversions; Elden Ring's camera and animation system would fight our controller.

**Overwatch reference** (your recordings, 2026-10-09): `D:\GenjiBridgeAssets\ow_ref\INDEX.md` lists what each of
your three clips shows and when (Dragonblade; walking, throws, Fan of Blades, reload; wall climb, landing), with a
frame every 0.1 s. The resting pose matches ours: the wrist at the same place on the screen, the same size. So the eye
and the field of view are right.

**Steps:**
- **10a. Name the animations:**
  - Render all 129 as looping previews (the same Blender script, all animations instead of only those with sounds).
  - An animation picker page like the sound picker, with my guesses filled in: idle, walk, jump, throw, Fan of Blades,
    reload, Swift Strike, Deflect start/loop/end, Dragonblade draw/idle/swings/end, quick melee, wall climb...
  - You correct the tags and export them.
- **10b. Export and a first picture:**
  - A Blender script writes the model (positions, normals, texture coordinates, bone weights), its textures, and
    every picked animation baked as bone poses per frame, to `D:\GenjiBridgeAssets\viewmodel` (outside the
    repository).
  - The renderer draws the idle pose from Genji's eye with your field of view into a PNG at your screen size.
  - You compare it with Overwatch.
- **10c. In game, idle only:** the arms drawn over the game; drawing and copy times logged.
  - Built 2026-10-09 (`arms.rs`, `crosshair.rs`). The overlay creates a texture half the screen's size (1290x540 on
    your screen) and starts a thread of ours, "genji-arms".
  - Each frame the game thread asks for a pose: each stance's resting pose for now (normal, Dragonblade, Deflect).
  - The thread draws it with `viewmodel.rs` (option A), split over 4 cores in bands of 32 rows handed out in turn. The game
    thread never waits for it.
  - Each frame the overlay copies the newest picture into the texture and stretches it over the screen, under the
    enemy bars, the kill skull, the crosshair and the HUD.
  - Empty pixels next to the arms take the arm's colour (still see-through), so the stretched edges don't get a dark
    outline.
  - Measured outside the game at 1290x540 on 4 cores: 4-12 ms per picture (Dragonblade is the heaviest).
- **10d. Actions play their animations:** throws, fan, reload, dash, Deflect, melee, Dragonblade (with the sword), jump
  and climb; idle and walk blend by speed.
  - Built 2026-10-09 without the sword, which is next. `arms_anim.rs` decides the clip each frame from the stance and
    the frame's actions (camera.rs notices them starting).
  - **Base:** each stance's resting pose, or its walk loop above 1.5 m/s on the ground.
  - **Actions:** throw (first shuriken of a burst), Fan of Blades, reload, quick melee, Swift Strike, the Dragonblade
    draw, swings and end, the Deflect start, block and end, jumps, the wall climb (start, top, letting go), and the
    arms coming up after menus or loading. Each plays once, and a new action cuts in.
  - Clips with several versions take turns (swings, fans, blocks, jumps).
  - Jumps, the climb and the arms coming up never cut into a combat action (2026-10-09). They play only when
    nothing else is playing.
  - **Blends:** 0.08 s into an action, 0.25 s back to the base.
  - Checked against your recordings: the throw has the same wind-up and release as clip2 at 13.1 s. The Dragonblade draw
    (12E6) has the same course as clip1 at 2.2 s (up, slash down, both hands on the hilt), but it's hard to judge
    without the sword.
- **Weapons** (2026-10-09): Overwatch's first-person entity (`Gameplay1P.owentity`) attaches the shuriken stack (DA9E)
  to `Weapon_R`, and the Dragonblade katana (DA9D) and the wakizashi (DAA1) to `Weapon_L`.
  - They're exported as parts of the model, fixed to those bones.
  - The arms show the katana in Dragonblade, the wakizashi for Swift Strike, the quick melee and Deflect, and the
    shuriken otherwise.
- **10e. Polish:** sway when turning, bob when walking, lighting closer to Overwatch's look.

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
