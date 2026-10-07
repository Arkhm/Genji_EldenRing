# STATUS: shuriken throwing stops after 256 wall hits (engine error 4)

## Resolved (2026-10-07, option C1; tested in game: 510 throws, 0 refused)
Ghidra 12.1.4 (headless, no full analysis; project and output in the gitignored `third_party/ghidra-work/`)
decompiled `spawn_bullet` (RVA 0x3A2CB0) and the code that frees the second pool's records:
- `spawn_bullet` -> RVA 0x3A5A20 allocates a record (0x9D0 bytes, RVA 0x3A7CC0; heap allocations count +0x38) and
  copies the converted request into it (record +0x330).
- Every frame RVA 0x3A9840 walks the live records and calls the record update (RVA 0x3900C0). The update keeps a
  record alive while its "emitting" byte (+0x329, set to 1 at spawn) is set or it still has bullets (+0x320). The
  emitting byte is cleared only when record +0x4C4 is set, and that byte comes from **bit 0 of the request's
  `unk44` ("flags")** (RVA 0x38E550 converts the request). Other code that ends emitters (by owner, by id) is
  driven by the game's own attacks, which we don't use.
- So with `unk44 = 0` (what we sent) every record emits forever and is never freed.
- The part 2 log agrees once read correctly: during the `unk44 = 1` variant the second pool's live count stayed at
  32-33 while its "ever allocated" count rose 32 -> 43, so those records were freed. I had only compared the count
  just before and after each throw, where a record always appears; the release happens a frame later.
Fix: `unk44 = 1` on every throw (`SPAWN_ONE_SHOT` in `genji/src/shuriken.rs`); the P debug switch is removed.
Bit 3 (0x8) of the same field adds network sync, never set.

Date: 2026-10-07
Step: 6 (Shuriken), see `docs/DESIGN.md` section 5c

## Goal
Throw shuriken without limit. Pass condition: hundreds of throws at walls and tough enemies, never refused.

## What happens instead
After roughly 256 shuriken have hit walls (or a living enemy), every further throw is refused by the engine:
`CSBulletManager::spawn_bullet` returns error 4 ("construction failed"). It recovers when the area reloads (death)
and, slowly, by itself (entries expire after a while).

## What we know for sure (from the logs)
- The bullet manager has three pools (fromsoftware-rs `BufferAndAllocLinkedList`: buffer + extra allocations).
  The second one (fromsoftware-rs calls it `unk_bullet_sfx_related`, 0x9D0-byte entries, 64 in the buffer + up to
  192 extra) is the limit: **the first refused throw happens exactly when its extra count reaches 192/192**, in every
  session since we started logging it (4 sessions).
- It grows only when shuriken **hit something** (walls; enemies that don't die). Throws at the sky don't grow it.
  When an enemy dies, entries tied to it go away.
- It is **not** our projectiles: they are removed by the engine right after hitting (`live 0`, `ended = thrown`).
- It is **not** a visual effect: with the bounce effect, the hit effect and the flight visual all switched off
  (`sfxId_Flick`, `sfxId_Hit` = none, flight visual "Deletion"), nothing is visible on the wall, and the pool still
  fills at the same rate (0 -> 80 -> 165 -> 192 in about 20 s of throwing).
- The rest of the shuriken work is verified in game: aim at the crosshair target, Overwatch hand position, no drop,
  hit detection, 27/54 damage, headshots (19 in one session), runes for kills.

## Attempts
1. Gravity after 80 m (projectiles that hit nothing never ended). Real problem, real fix, but not this one.
2. Remove "stuck" projectiles. Never triggered: no projectile stays alive.
3. Bounce effect off. Daggers stopped hanging in the air; pool still filled.
4. Flight visual set to "Deletion". Applied; pool still filled.
5. Hit effect off. Nothing visible any more; pool still filled.

## What we don't know
- What object the 0x9D0-byte entries are. The name "sfx related" in fromsoftware-rs is a guess by its authors.
- Which Bullet/AtkParam setting makes the engine create one per hit, and how long it lives. Candidates seen in the
  Bullet row: `dmgHitRecordLifeTime` (0 for the dagger; "0" might mean "until removed"), hit records against map
  collision, the bullet's targeting/homing data.
- Whether every projectile does this, or the Throwing Dagger row in particular.

## Option A result (2026-10-07)
The read-only diagnostic ran three times (it also froze the game for about 4 s each time: it checked memory before
every 8-byte read on the game's frame; removed again):
- The entries are **not C++ objects**: no class name in any of the 64 buffer slots, none embedded in the first
  0x200 bytes. Plain data records.
- Content of a live entry: a small id at `+0x000` (`0xff00`, `0xff24`); many `0xffffffff...` (empty handles);
  a **3x3 rotation** at `+0x900..+0x928` and a **position** at `+0x930..+0x938` near the impact point;
  `+0x6f0` = 0.1, `+0x6fc` = `+0x700` = 0.785 (45 degrees in radians), `+0x8fc` = 79.2. None of these matches a
  value of the dagger's Bullet row directly (homing 10 degrees, lock limit 30, ranges 2/7/80...).
- So: each hit leaves a data record at the impact point that outlives the projectile and expires after a while.
  What system owns it (impact decal, AI noise, hit record, ...) is not identified; I won't build a fix on a guess.

## Option B result (2026-10-07): one record per throw, never freed
- Bone Dart fills the second pool exactly like the dagger. Once the pool is full, every projectile is refused (it is
  shared), so switching rows couldn't help.
- Measured per 2-second interval: **pool growth = number of successful throws, exactly** (22 throws -> +22, 20 -> +20,
  ...), and exactly 256 throws succeeded before the first refusal. So it is **one record per spawn call**, not per hit;
  the earlier "only on wall hits" was wrong (the extra counter only starts after the 64 buffer slots are used, which
  made early sky throws look free).
- Resting at a grace does not free them (192/192 for 80 s, including a rest); only an area reload (death) did.
- The record's position and rotation fit the spawn point and direction, not the impact.
- So the cause is most likely **our spawn request**: a field we leave at 0 / -1 that, in the game's own throws, makes
  the engine release this record. Next experiment: P switches spawn-request variants (item id, dummy poly, the
  bridge mod's "flags", target, behavior/magic id, unknown fields); the log shows +1 or +0 per throw.

## Option B, part 2 result (2026-10-07): no tested request field releases the record
Every throw leaked one record with every variant: baseline 26/26 (after the 64 buffer slots filled), `goods_id` 1700
16/16, `dummy_poly_id` 0 10/10 (this one spawned the dagger from above: it picks an attachment point on the model),
`unk44` ("flags") 1 11/11, target = owner 13/13, behavior/magic id 0 12/12, `unk10` 1 11/11, `unk28`/`unk2c` 1 13/13,
`unk40` 1 6/6. Untested parts of the request: `unk30` (a 4-float vector), the block 0x90..0x10F, `unk100`.
Nothing about this in the universal-modder notes or fromsoftware-rs issues.

## Options now

| | Option | What it does | Cost |
|---|---|---|---|
| **C1 (recommended)** | **Ghidra, headless** (no MCP needed): decompile `CSBulletManager::spawn_bullet` (RVA 0x3A2CB0) and the code that allocates and frees the second pool's records, to see what the game's own throws set that ours don't, or which call releases the record. | Download Ghidra 12.1.4 (`ghidra_12.1.4_PUBLIC_20260921.zip`, 543 MB, official NationalSecurityAgency/ghidra GitHub release) into `third_party/` (gitignored); Java 21 is installed. Analysis of the 87 MB exe takes a while (likely an hour or more, unattended). The exe is only read. Parts may be obfuscated (Arxan). |
| C2 | Same through ghidra-mcp (interactive) | More setup (Ghidra plugin + MCP server registration in Claude Code, outside this folder). Not needed for this question. |
| B3 | Keep testing the untested request parts (`unk30`, 0x90..0x10F, `unk100`) | Cheap, but guessing; the 0x50-byte block is too large to try blindly. |
| W | Workaround: none found that's acceptable (the records clear only on an area reload). | |

## Options from here (earlier)

| | Option | What it does | Cost |
|---|---|---|---|
| **B (recommended now)** | Try other projectile rows (a sorcery bolt that bursts on impact, an arrow, another throwable) with a debug key to switch between them, logging how much the pool grows per wall hit. | Finds a projectile that doesn't leave records; we give it the shuriken values. | One or two sessions |
| C | Ghidra: find the code that writes into this pool (we now know an entry's shape: id at 0, rotation at 0x900, position at 0x930). | The real answer. | Larger; Ghidra 12 install |
| D | `dmgHitRecordLifeTime` | Still a guess. | Tiny |

## Options from here (original)

| | Option | What it does | Cost | Risk |
|---|---|---|---|---|
| **A (recommended first)** | **Identify the entries.** A diagnostic build reads the first live entry of that pool and looks up its C++ class name (every such object starts with a pointer whose RTTI data names the class), and logs it with its first bytes. One short test session: throw at a wall 20 times, quit. | Tells us *what* fills the pool; often the class name alone points to the setting (e.g. a "hit record" class -> `dmgHitRecordLifeTime`). | Small | Low: read-only |
| B | **Try other projectile rows.** A debug key cycles through a few other Elden Ring projectiles (another throwable, an arrow, a sorcery bolt) while the log shows how much the pool grows per wall hit. | Finds a row that doesn't leave anything behind; we'd then give it the shuriken values. | Small to medium, one or two sessions | The best row may look different in flight |
| C | **Reverse-engineer the hit path** with Ghidra (ghidra-mcp): where `spawn_bullet`'s hit handling allocates into that pool, and what controls the entry's lifetime. | The real answer, independent of guesses. | Medium to large; needs Ghidra 12 installed | Elden Ring's exe is partly obfuscated |
| D | **Try `dmgHitRecordLifeTime`** directly (set it to a short time, e.g. 0.5 s). | Might be the fix outright. | Tiny | A guess; that's what the rule says not to do without evidence. Best done *after* A says it's a hit record. |

My recommendation: **A**, then D or B depending on the class name. A costs one short session and turns the next
change from a guess into a decision.

## State of the repo
All work is committed and pushed (last code commit `479075c`). The game runs normally; Genji mode works (movement,
first person, shuriken until the pool fills). In Genji mode shuriken currently have no hit effect (attempt 5); I'd
keep it off until we know more, or restore it, your choice.
