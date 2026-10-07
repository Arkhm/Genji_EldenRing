//! Shuriken (step 6): left click throws one of the engine's own projectiles from Genji's right hand toward
//! whatever is under the crosshair; Overwatch damage (27 body, 54 head) is applied by us.
//!
//! The engine does the flying, collision, hit effects, hit reactions and enemy aggro: we ask it to spawn one of
//! its own projectiles ("bullets") with `CSBulletManager::spawn_bullet`. The projectile is Elden Ring's Throwing
//! Dagger (Bullet and AtkParam_Pc row 10170000), found by reading a copy of your `regulation.bin` (`erdata/`,
//! never committed). While Genji mode is active its rows get shuriken values in memory (see VanillaValues).
//!
//! The spawn request: fromsoftware-rs's `BulletSpawnData` has private fields and no constructor, so we fill our
//! own copy of its 0x110-byte layout. The values follow a working Elden Ring mod (justbustin/minecraft-crossover-
//! bridge, MIT, `er-bridge/src/game.cpp`): owner = the player's handle (hits, aggro and kills count as ours);
//! behavior, magic, goods and dummy-poly ids -1; no target; right/up/forward/position at 0x50/0x60/0x70/0x80;
//! everything else zero.
//!
//! Test of 2026-10-07 (6c early): exactly the 257th throw since loading and every one after it failed with engine
//! error 4, until a reload. The manager holds 128 bullets plus 128 extra: none of ours was ever removed. Without
//! gravity a shuriken that hits nothing (thrown at the sky or far away) apparently flies on forever (it also hit an
//! enemy "underground" far away). Now they fall after SHURIKEN_RANGE, so they end on the ground like vanilla
//! daggers do after 15 m; live shuriken are tracked and reported to prove it.

use std::collections::HashMap;
use std::ptr::NonNull;

use eldenring::cs::{
    AtkParam_Pc, Bullet as BulletParam, BulletSpawnData, CSBulletManager, CSHavokMan, ChrIns, PlayerIns,
    SoloParamRepository, WorldChrMan,
};
use eldenring::position::{HavokPosition, PositionDelta};
use fromsoftware_shared::FromStatic;
use glam::Vec3;

use crate::glog;
use crate::look::Look;

/// STATUS.md option B (2026-10-07): projectile rows that were compared. Result: Bone Dart fills the second pool exactly
/// like the dagger, and the pool grows by exactly one record per successful *throw* (not per hit), never freed until
/// the area reloads. The projectile type wasn't the cause (see SPAWN_ONE_SHOT); we use the dagger (index 0).
const CANDIDATES: [(i32, &str); 5] = [
    (10_170_000, "Throwing Dagger"),
    (10_171_000, "Bone Dart"),
    (10_173_000, "Kukri"),
    (10_176_000, "Ruin Fragment"),
    (10_400_000, "Glintstone Pebble"),
];

/// Where the shuriken appears, relative to the eye: like Overwatch's Genji (your screenshot, 2026-10-07: just right
/// of and below the centre of the screen, not at the far right). Metres forward, right, down.
const HAND_FORWARD: f32 = 0.6;
const HAND_RIGHT: f32 = 0.08;
const HAND_DOWN: f32 = 0.06;

/// Shuriken flight, set in memory on the dagger's Bullet row while Genji mode is active: straight and fast up to
/// SHURIKEN_RANGE, then falling (OUT_OF_RANGE_GRAVITY) so it ends on the ground and the engine removes it.
/// Approximate Overwatch speed, not verified. The game's file is never changed; vanilla values are restored
/// whenever Genji mode lets go (menus, F9), so a real Throwing Dagger in plain Elden Ring is unchanged.
const SHURIKEN_SPEED: f32 = 60.0;
const SHURIKEN_RANGE: f32 = 80.0;
const OUT_OF_RANGE_GRAVITY: f32 = 20.0;
/// Life the engine is told, seconds (logged against what the engine actually uses).
const SHURIKEN_LIFE: f32 = 3.0;
/// The dagger's bounce effect ("flick", SFX 300003) is switched off for shuriken. Test of 2026-10-07: the
/// projectiles themselves were all removed after hitting ("live 0"), yet daggers still hung on the wall and the
/// 257th throw failed again. What hangs is the bounce *effect*: it falls under the projectile's gravity, which is 0
/// for shuriken, so it never ends. The bullet manager keeps those effects in a second pool of 64 + 192 = 256.
const NO_SFX: i32 = -1;
/// What the engine does with the flight visual (the dagger model, SFX 300001) when the projectile hits or its life
/// ends: 0 = "Emitter Stopped" (the visual is left to finish on its own), 1 = "Deletion" (removed at once). Names
/// from the community Paramdex shipped with WitchyBND (`ER/Meta/BulletParam.xml`, enum BULLET_SFX_DELETE_TYPE).
/// The dagger uses 0. Test of 2026-10-07 (bounce effect already off): daggers stuck *into* the wall instead, and
/// the effect pool's live count only climbed (9, 23, 43 ... 170 in 22 s, about 7 per second of wall hits): the
/// flight visuals were never removed. Shuriken use 1.
const SFX_DELETE: i8 = 1;
/// The dagger's hit effect (SFX 300002) stays on (your choice, 2026-10-07). Switching it off (attempt 5) removed the
/// stuck daggers from walls but did not stop the second pool from filling, so it isn't the cause.

/// The engine's own damage is reduced to almost nothing (it still gives the hit reaction, aggro and the "last hit
/// by" record); the Overwatch damage below is applied by us when a shuriken ends next to a character.
const ENGINE_ATK_PHYS: u16 = 1;
/// Overwatch shuriken damage (your numbers, 2026-10-07). DAMAGE_MULTIPLIER scales both if Elden Ring's health pools
/// turn out to need it.
const BODY_DAMAGE: f32 = 27.0;
const HEAD_DAMAGE: f32 = 54.0;
const DAMAGE_MULTIPLIER: f32 = 1.0;
/// A hit at or above this share of the character's height counts as a headshot.
const HEAD_SHARE: f32 = 0.8;
/// A shuriken that ends within this distance of a character's collision cylinder hit it, metres.
const HIT_MARGIN: f32 = 0.4;

/// Map collision filter for the aim ray (step 5 tests: all filter bits see the same surface).
const MAP_FILTER: u32 = 0x0200_0058;
/// Entries of the game's distance-sorted character list to consider.
const CHARACTERS_MAX: usize = 64;
const REPORT_EVERY: f32 = 2.0;
/// Test of 2026-10-07: a shuriken that hits a wall "flicks" (the dagger's bounce) and, with our zero gravity, hangs
/// in the air in front of the wall for good, keeping its slot in the bullet manager (screenshot: a wall full of
/// daggers; throwing stopped after the pool filled; killing an enemy freed the ones stuck in it). Impact is now
/// the first frame a shuriken slows below IMPACT_SPEED; afterwards, below STUCK_SPEED it counts as stuck and we
/// remove it: first by setting its remaining life to 0, and if it is still there after PUSH_AFTER seconds, by
/// sending it straight down so it hits the ground like a vanilla dagger. Both are logged to see which works.
const IMPACT_SPEED: f32 = 0.5 * SHURIKEN_SPEED;
const STUCK_SPEED: f32 = 1.0;
const PUSH_AFTER: f32 = 0.5;
const PUSH_DOWN_SPEED: f32 = 30.0;

/// Spawn-request flag bit 0 ("one shot"). Every spawn creates an emitter record in the bullet manager's second pool;
/// the engine frees it only once this bit is set and the record's bullets are gone. Without it the record lives
/// until the area reloads, and after 256 of them every throw is refused (STATUS.md, Ghidra 2026-10-07: copied into
/// the record at +0x4c4, checked by the record update at RVA 0x3900c0). Bit 3 (0x8) would add network sync: never.
const SPAWN_ONE_SHOT: u32 = 1;

/// Our copy of the engine's bullet spawn request (same layout as fromsoftware-rs `BulletSpawnData`).
#[repr(C, align(16))]
struct SpawnRequest {
    owner: u64,
    behavior_id: i32,
    magic_id: i32,
    unk10: u32,
    bullet_id: i32,
    goods_id: i32,
    dummy_poly_id: i32,
    target: u64,
    unk28: u32,
    unk2c: u32,
    unk30: [f32; 4],
    unk40: u32,
    unk44: u32,
    pad48: [u8; 8],
    right: [f32; 4],
    up: [f32; 4],
    forward: [f32; 4],
    position: [f32; 4],
    rest: [u8; 0x80],
}
const _: () = assert!(std::mem::size_of::<SpawnRequest>() == 0x110);
const _: () = assert!(std::mem::size_of::<BulletSpawnData>() == 0x110);
const _: () = assert!(std::mem::align_of::<SpawnRequest>() >= std::mem::align_of::<BulletSpawnData>());
const _: () = {
    use std::mem::offset_of;
    assert!(offset_of!(SpawnRequest, bullet_id) == 0x14);
    assert!(offset_of!(SpawnRequest, target) == 0x20);
    assert!(offset_of!(SpawnRequest, right) == 0x50);
    assert!(offset_of!(SpawnRequest, up) == 0x60);
    assert!(offset_of!(SpawnRequest, forward) == 0x70);
    assert!(offset_of!(SpawnRequest, position) == 0x80);
};

/// Flight and damage values of the dagger's rows (vanilla ones kept to restore them).
#[derive(Clone, Copy)]
struct RowValues {
    init_velocity: f32,
    max_velocity: f32,
    min_velocity: f32,
    accel_out_range: f32,
    gravity_in_range: f32,
    gravity_out_range: f32,
    life: f32,
    dist: f32,
    sfx_flick: i32,
    sfx_hit: i32,
    sfx_delete_by_hit: i8,
    sfx_delete_by_life_end: i8,
    /// The AtkParam_Pc row the bullet uses (not always the bullet's own id: Glintstone Pebble uses 40000).
    atk_row: u32,
    atk_phys: u16,
}

/// One of our shuriken, as tracked frame to frame.
struct Flying {
    /// Position and velocity on the last frame it was still flying at speed. At 60 m/s a shuriken moves about
    /// 1 m per frame, so the impact lies in the stretch after this position; the hit test covers that stretch.
    pos: Vec3,
    vel: Vec3,
    age: f32,
    /// Seen flying at speed at least once (a brand-new bullet may report no speed on its first frame).
    was_flying: bool,
    /// The impact has been handled (hit or miss).
    resolved: bool,
    /// Removal attempts for a shuriken that stopped after hitting something (see STUCK_SPEED).
    stuck_for: f32,
    life_zeroed: bool,
    pushed_down: bool,
}

/// A character as a target: collision cylinder and a pointer to change its HP.
struct Target {
    chr: NonNull<ChrIns>,
    feet: Vec3,
    radius: f32,
    height: f32,
}

#[derive(Default)]
pub struct Thrower {
    /// Some while our shuriken values are applied in memory.
    vanilla: Option<RowValues>,
    clock: f32,
    throws: u32,
    flying: HashMap<u64, Flying>,
    report_timer: f32,
    ended: u32,
    hits: u32,
    headshots: u32,
    removed_after_life_zero: u32,
    removed_after_push: u32,
    /// Index into CANDIDATES (fixed at the dagger).
    candidate: usize,
    impacts: u32,
    peak_extra: u32,
}

impl Thrower {
    /// Applies (true) or restores (false) the shuriken values on the dagger's rows in memory.
    pub fn set_shuriken_values(&mut self, on: bool) {
        if on == self.vanilla.is_some() {
            return;
        }
        let Ok(params) = (unsafe { SoloParamRepository::instance_mut() }) else {
            return;
        };
        let id = self.bullet_id() as u32;
        match self.vanilla {
            None => {
                let Some(bullet) = params.get::<BulletParam>(id) else {
                    glog!("THROW", "Bullet row {id} not found; shuriken values not applied");
                    return;
                };
                let atk_row = bullet.atk_id_bullet() as u32;
                let Some(atk) = params.get::<AtkParam_Pc>(atk_row) else {
                    glog!("THROW", "AtkParam_Pc row {atk_row} (of bullet {id}) not found; shuriken values not applied");
                    return;
                };
                let vanilla_hit = bullet.sfx_id_hit();
                let vanilla = RowValues {
                    init_velocity: bullet.init_vellocity(),
                    max_velocity: bullet.max_vellocity(),
                    min_velocity: bullet.min_vellocity(),
                    accel_out_range: bullet.accel_out_range(),
                    gravity_in_range: bullet.gravity_in_range(),
                    gravity_out_range: bullet.gravity_out_range(),
                    life: bullet.life(),
                    dist: bullet.dist(),
                    sfx_flick: bullet.sfx_id_flick(),
                    sfx_hit: bullet.sfx_id_hit(),
                    sfx_delete_by_hit: bullet.bullet_sfx_delete_type_by_hit(),
                    sfx_delete_by_life_end: bullet.bullet_sfx_delete_type_by_life_dead(),
                    atk_row,
                    atk_phys: atk.atk_phys(),
                };
                let shuriken = RowValues {
                    init_velocity: SHURIKEN_SPEED,
                    max_velocity: SHURIKEN_SPEED,
                    min_velocity: SHURIKEN_SPEED,
                    accel_out_range: 0.0,
                    gravity_in_range: 0.0,
                    gravity_out_range: OUT_OF_RANGE_GRAVITY,
                    life: SHURIKEN_LIFE,
                    dist: SHURIKEN_RANGE,
                    sfx_flick: NO_SFX,
                    sfx_hit: vanilla_hit,
                    sfx_delete_by_hit: SFX_DELETE,
                    sfx_delete_by_life_end: SFX_DELETE,
                    atk_row,
                    atk_phys: ENGINE_ATK_PHYS,
                };
                write_values(params, id, &shuriken);
                glog!(
                    "THROW",
                    "shuriken values ON for {} (bullet {id}, attack {atk_row}): speed {} -> {SHURIKEN_SPEED} m/s, gravity {} -> 0 then {OUT_OF_RANGE_GRAVITY} after {SHURIKEN_RANGE} m (was after {} m), life {} -> {SHURIKEN_LIFE} s, bounce effect {} -> none, hit effect {} kept, visual on hit/life end {}/{} -> deleted, engine atk {} -> {ENGINE_ATK_PHYS}",
                    CANDIDATES[self.candidate].1,
                    vanilla.init_velocity, vanilla.gravity_in_range, vanilla.dist, vanilla.life, vanilla.sfx_flick, vanilla.sfx_hit,
                    vanilla.sfx_delete_by_hit, vanilla.sfx_delete_by_life_end, vanilla.atk_phys
                );
                self.vanilla = Some(vanilla);
            }
            Some(vanilla) => {
                write_values(params, id, &vanilla);
                glog!("THROW", "shuriken values OFF: vanilla {} restored", CANDIDATES[self.candidate].1);
                self.vanilla = None;
            }
        }
    }

    fn bullet_id(&self) -> i32 {
        CANDIDATES[self.candidate].0
    }

    /// Throws one shuriken from Genji's hand toward what is under the crosshair, turned `angle` radians to the
    /// side around the view's up axis (positive = left; Fan of Blades).
    pub fn throw(&mut self, player: &PlayerIns, look: &Look, eye: Vec3, angle: f32) {
        let view_forward = look.forward();
        let view_right = look.right(); // horizontal right is perpendicular to any pitched forward (no roll)
        let view_up = look.up();
        let aim_distance = aim_distance(player, eye, view_forward);
        let aim = eye + view_forward * aim_distance;
        let from = eye + view_forward * HAND_FORWARD + view_right * HAND_RIGHT - view_up * HAND_DOWN;
        let forward = (aim - from).normalize_or(view_forward);
        let forward = glam::Quat::from_axis_angle(view_up, angle) * forward;
        let right = Vec3::Y.cross(forward).normalize_or(view_right);
        let right = if right.dot(view_right) < 0.0 { -right } else { right };
        let up = forward.cross(right).normalize_or(Vec3::Y);
        let up = if up.y < 0.0 { -up } else { up };
        let owner = unsafe { std::ptr::read(&player.chr_ins.field_ins_handle as *const _ as *const u64) };
        let request = SpawnRequest {
            owner,
            behavior_id: -1,
            magic_id: -1,
            unk10: 0,
            bullet_id: self.bullet_id(),
            goods_id: -1,
            dummy_poly_id: -1,
            target: u64::MAX,
            unk28: 0,
            unk2c: 0,
            unk30: [0.0; 4],
            unk40: 0,
            unk44: SPAWN_ONE_SHOT,
            pad48: [0; 8],
            right: [right.x, right.y, right.z, 0.0],
            up: [up.x, up.y, up.z, 0.0],
            forward: [forward.x, forward.y, forward.z, 0.0],
            position: [from.x, from.y, from.z, 1.0],
            rest: [0; 0x80],
        };
        let Ok(manager) = (unsafe { CSBulletManager::instance_mut() }) else {
            glog!("THROW", "bullet manager not available");
            return;
        };
        let data = unsafe { &*(&request as *const SpawnRequest as *const BulletSpawnData) };
        let result = manager.spawn_bullet(data);
        self.throws += 1;
        if result.is_err() {
            glog!("THROW", "{}", pool_levels(manager));
        }
        glog!(
            "THROW",
            "#{} aim {aim_distance:.1} m, side {:.1} deg, from ({:.2}, {:.2}, {:.2}) dir ({:.2}, {:.2}, {:.2}): {}",
            self.throws,
            angle.to_degrees(),
            from.x, from.y, from.z,
            forward.x, forward.y, forward.z,
            match result {
                Ok(()) => "ok".to_string(),
                Err(code) => format!(
                    "FAILED, engine error {code} (3 = invalid bullet id, 4 = construction failed); {} of ours alive",
                    self.flying.len()
                ),
            }
        );
    }

    /// Every frame: follows our shuriken; at impact applies Overwatch damage; removes shuriken that got stuck.
    pub fn tick(&mut self, player: &PlayerIns, dt: f32) {
        self.clock += dt;
        let Ok(manager) = (unsafe { CSBulletManager::instance_mut() }) else {
            return;
        };
        let current = self.bullet_id();
        let mut seen = std::collections::HashSet::new();
        let mut impacts: Vec<(Vec3, Vec3, f32)> = Vec::new();
        let (mut live, mut stuck, mut sample_life) = (0u32, 0u32, None);
        for bullet in manager.bullets_mut() {
            let ids = [
                bullet.wait_state.base.param.param_id,
                bullet.fly_state.base.param.param_id,
                bullet.exp_state.base.param.param_id,
            ];
            if !ids.contains(&current) {
                continue;
            }
            let handle = unsafe { std::ptr::read(&bullet.field_ins_handle as *const _ as *const u64) };
            let p = &bullet.physics.position;
            let v = &bullet.physics.velocity;
            let (pos, vel) = (Vec3::new(p.0, p.1, p.2), Vec3::new(v.0, v.1, v.2));
            let speed = vel.length();
            // `life` and `life_max` follow `time_alive` in CSBulletIns (private in fromsoftware-rs).
            let life_ptr = &mut bullet.time_alive as *mut f32;
            sample_life.get_or_insert_with(|| unsafe { (life_ptr.add(1).read(), life_ptr.add(2).read()) });
            live += 1;
            seen.insert(handle);
            let entry = self.flying.entry(handle).or_insert(Flying {
                pos,
                vel,
                age: 0.0,
                was_flying: false,
                resolved: false,
                stuck_for: 0.0,
                life_zeroed: false,
                pushed_down: false,
            });
            entry.age += dt;
            if !entry.resolved {
                if speed >= IMPACT_SPEED {
                    entry.pos = pos;
                    entry.vel = vel;
                    entry.was_flying = true;
                } else if entry.was_flying {
                    // Slowed sharply: it hit something between the last fast position and here.
                    impacts.push((entry.pos, entry.vel, entry.age));
                    entry.resolved = true;
                    self.impacts += 1;
                }
            }
            if entry.resolved && speed < STUCK_SPEED {
                stuck += 1;
                entry.stuck_for += dt;
                if !entry.life_zeroed {
                    unsafe { life_ptr.add(1).write(0.0) };
                    entry.life_zeroed = true;
                } else if entry.stuck_for >= PUSH_AFTER && !entry.pushed_down {
                    bullet.physics.velocity.0 = 0.0;
                    bullet.physics.velocity.1 = -PUSH_DOWN_SPEED;
                    bullet.physics.velocity.2 = 0.0;
                    entry.pushed_down = true;
                }
            }
        }
        // Shuriken the engine removed since the last frame.
        let gone: Vec<Flying> = {
            let handles: Vec<u64> = self.flying.keys().filter(|h| !seen.contains(h)).copied().collect();
            handles.into_iter().filter_map(|h| self.flying.remove(&h)).collect()
        };
        for shuriken in gone {
            self.ended += 1;
            if !shuriken.resolved && shuriken.was_flying {
                impacts.push((shuriken.pos, shuriken.vel, shuriken.age));
            }
            if shuriken.pushed_down {
                self.removed_after_push += 1;
            } else if shuriken.life_zeroed {
                self.removed_after_life_zero += 1;
            }
        }
        for (pos, vel, age) in impacts {
            self.resolve(player, pos, vel, age, dt);
        }

        self.report_timer += dt;
        if self.report_timer >= REPORT_EVERY && self.throws > 0 {
            self.report_timer = 0.0;
            let extra = unsafe { ((manager as *const CSBulletManager as *const u8).add(0x38) as *const u32).read() };
            self.peak_extra = self.peak_extra.max(extra);
            glog!(
                "SHURIKEN",
                "[{}] impacts since switch {} | {} | peak extra {} | buffer slots marked {}/64 (heuristic)",
                CANDIDATES[self.candidate].1,
                self.impacts,
                pool_levels(manager),
                self.peak_extra,
                buffer_marked(manager)
            );
            glog!(
                "SHURIKEN",
                "live {live} (stuck {stuck}){} | thrown {} ended {} hits {} headshots {} | stuck ones removed after life=0: {}, after push down: {}",
                sample_life.map(|(l, m)| format!(", engine life {l:.2} / max {m:.2}")).unwrap_or_default(),
                self.throws,
                self.ended,
                self.hits,
                self.headshots,
                self.removed_after_life_zero,
                self.removed_after_push
            );
        }
    }

    /// A shuriken ended somewhere between its last seen position and where its velocity would have taken it by
    /// the next frame: if that stretch passes next to a character, it hit it (closest character wins).
    fn resolve(&mut self, player: &PlayerIns, last_pos: Vec3, vel: Vec3, age: f32, dt: f32) {
        const SAMPLES: usize = 8;
        let reach = vel * (dt.max(1.0 / 60.0) * 1.5);
        let mut best: Option<(f32, Target, Vec3)> = None;
        for target in targets(player) {
            for i in 0..=SAMPLES {
                let pos = last_pos + reach * (i as f32 / SAMPLES as f32);
                if pos.y < target.feet.y - 0.3 || pos.y > target.feet.y + target.height + 0.3 {
                    continue;
                }
                let gap = Vec3::new(pos.x - target.feet.x, 0.0, pos.z - target.feet.z).length() - target.radius;
                if gap <= HIT_MARGIN && best.as_ref().is_none_or(|(g, _, _)| gap < *g) {
                    best = Some((gap, Target { ..target }, pos));
                    break; // the first sample along the flight that touches it is where it hit
                }
            }
        }
        let Some((_, target, pos)) = best else {
            return;
        };
        let head = pos.y >= target.feet.y + HEAD_SHARE * target.height;
        let damage = ((if head { HEAD_DAMAGE } else { BODY_DAMAGE }) * DAMAGE_MULTIPLIER).round() as i32;
        let chr = unsafe { &mut *target.chr.as_ptr() };
        let data = &mut chr.modules.data;
        if data.hp <= 0 {
            return;
        }
        let before = data.hp;
        data.hp = (data.hp - damage).max(0);
        self.hits += 1;
        if head {
            self.headshots += 1;
        }
        glog!(
            "HIT",
            "{} {damage} on character {:#x}: hp {before} -> {} / {} (shuriken flew {:.2} s, hit at {:.0}% of its height)",
            if head { "HEADSHOT" } else { "body" },
            target.chr.as_ptr() as usize,
            data.hp,
            data.max_hp,
            age,
            100.0 * (pos.y - target.feet.y) / target.height
        );
    }
}

/// Fill levels of the bullet manager's first three pools, read from fromsoftware-rs's layout (the counters are
/// private there): each `BufferAndAllocLinkedList` is 0x20 bytes, `allocated_count` (alive) at +0x18 and
/// `allocations_counter` (heap allocations ever, "doesn't decrease") at +0x1C. Pools: bullets (128 + 128),
/// bullet effects (64 + 192), unknown (4 + 28).
fn pool_levels(manager: &CSBulletManager) -> String {
    let base = manager as *const CSBulletManager as *const u8;
    let read = |offset: usize| unsafe { (base.add(offset) as *const u32).read() };
    format!(
        "pools alive/ever-allocated: bullets {}/{} effects {}/{} other {}/{}",
        read(0x18), read(0x1C), read(0x38), read(0x3C), read(0x58), read(0x5C)
    )
}

/// How many of the second pool's 64 buffer slots carry the mark seen on live entries (`0xff` in the second byte,
/// POOLDIAG 2026-10-07: `0xff00`, `0xff24`). A heuristic only: free slots may keep old marks.
fn buffer_marked(manager: &CSBulletManager) -> u32 {
    let base = manager as *const CSBulletManager as *const u8;
    let buffer = unsafe { (base.add(0x20) as *const u64).read() } as usize;
    if buffer == 0 {
        return 0;
    }
    (0..64)
        .filter(|i| {
            let first = unsafe { ((buffer + i * 0x9D0) as *const u64).read() };
            first & 0xff00 == 0xff00
        })
        .count() as u32
}

/// Distance along the view to what's under the crosshair: map (ray) or a character (cylinder), up to the range.
fn aim_distance(player: &PlayerIns, eye: Vec3, forward: Vec3) -> f32 {
    let mut best = SHURIKEN_RANGE;
    if let Ok(havok) = unsafe { CSHavokMan::instance() } {
        let w = player.chr_ins.modules.physics.position.3;
        let delta = forward * SHURIKEN_RANGE;
        if let Some(hit) = havok.phys_world.cast_ray(
            MAP_FILTER,
            &HavokPosition(eye.x, eye.y, eye.z, w),
            PositionDelta(delta.x, delta.y, delta.z),
            player,
        ) {
            best = best.min((Vec3::new(hit.0, hit.1, hit.2) - eye).length());
        }
    }
    for t in targets(player) {
        if let Some(dist) = ray_cylinder(eye, forward, t.feet, t.radius, t.height) {
            best = best.min(dist);
        }
    }
    best
}

/// Distance along the ray (`eye`, unit `forward`) to where it enters an upright cylinder standing at `feet`.
fn ray_cylinder(eye: Vec3, forward: Vec3, feet: Vec3, radius: f32, height: f32) -> Option<f32> {
    let flat = Vec3::new(forward.x, 0.0, forward.z);
    let flat_len = flat.length();
    if flat_len < 1e-3 {
        return None; // looking straight up or down
    }
    let d = flat / flat_len;
    // Ray-circle intersection on the ground plane, in horizontal distance s; 3D distance = s / flat_len.
    let o = Vec3::new(eye.x - feet.x, 0.0, eye.z - feet.z);
    let b = o.dot(d);
    let c = o.length_squared() - radius * radius;
    let disc = b * b - c;
    if disc < 0.0 {
        return None;
    }
    let s = -b - disc.sqrt();
    if s <= 0.0 {
        return None;
    }
    let dist = s / flat_len;
    let y = eye.y + forward.y * dist;
    (y >= feet.y && y <= feet.y + height).then_some(dist)
}

/// Living characters near the player (not the player).
fn targets(player: &PlayerIns) -> Vec<Target> {
    let Ok(world) = (unsafe { WorldChrMan::instance() }) else {
        return Vec::new();
    };
    let me = &player.chr_ins as *const ChrIns as usize;
    world
        .chr_inses_by_distance
        .iter()
        .take(CHARACTERS_MAX)
        .filter_map(|entry| {
            if entry.chr_ins.as_ptr() as usize == me {
                return None;
            }
            let chr = unsafe { entry.chr_ins.as_ref() };
            if chr.modules.data.hp <= 0 {
                return None;
            }
            let physics = &chr.modules.physics;
            let p = physics.position;
            Some(Target {
                chr: entry.chr_ins,
                feet: Vec3::new(p.0, p.1, p.2),
                radius: physics.hit_radius.max(physics.chr_hit_radius).clamp(0.2, 3.0),
                height: physics.hit_height.max(physics.chr_hit_height).clamp(0.5, 8.0),
            })
        })
        .collect()
}

/// Writes flight and damage values to the Bullet and AtkParam_Pc rows `id` in memory.
fn write_values(params: &mut SoloParamRepository, id: u32, v: &RowValues) {
    if let Some(bullet) = params.get_mut::<BulletParam>(id) {
        bullet.set_init_vellocity(v.init_velocity);
        bullet.set_max_vellocity(v.max_velocity);
        bullet.set_min_vellocity(v.min_velocity);
        bullet.set_accel_out_range(v.accel_out_range);
        bullet.set_gravity_in_range(v.gravity_in_range);
        bullet.set_gravity_out_range(v.gravity_out_range);
        bullet.set_life(v.life);
        bullet.set_dist(v.dist);
        bullet.set_sfx_id_flick(v.sfx_flick);
        bullet.set_sfx_id_hit(v.sfx_hit);
        bullet.set_bullet_sfx_delete_type_by_hit(v.sfx_delete_by_hit);
        bullet.set_bullet_sfx_delete_type_by_life_dead(v.sfx_delete_by_life_end);
    }
    if let Some(atk) = params.get_mut::<AtkParam_Pc>(v.atk_row) {
        atk.set_atk_phys(v.atk_phys);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aim_hits_a_character_straight_ahead() {
        // Eye at 1.6 m looking along +z; a 1.8 m tall, 0.5 m radius character 10 m ahead.
        let d = ray_cylinder(Vec3::new(0.0, 1.6, 0.0), Vec3::Z, Vec3::new(0.0, 0.0, 10.0), 0.5, 1.8);
        assert!((d.unwrap() - 9.5).abs() < 1e-4, "{d:?}");
    }

    #[test]
    fn aim_misses_beside_above_and_behind() {
        let eye = Vec3::new(0.0, 1.6, 0.0);
        assert!(ray_cylinder(eye, Vec3::Z, Vec3::new(2.0, 0.0, 10.0), 0.5, 1.8).is_none(), "beside");
        assert!(ray_cylinder(eye, Vec3::Z, Vec3::new(0.0, 0.0, 10.0), 0.5, 1.0).is_none(), "too short: passes above");
        assert!(ray_cylinder(eye, Vec3::Z, Vec3::new(0.0, 0.0, -10.0), 0.5, 1.8).is_none(), "behind");
        assert!(ray_cylinder(eye, Vec3::Y, Vec3::new(0.0, 0.0, 0.0), 0.5, 1.8).is_none(), "looking straight up");
    }

    #[test]
    fn aim_works_looking_down_at_a_character() {
        // Looking 30 degrees down: the ray drops 1.6 m over about 2.77 m and hits a character standing 2.5 m ahead.
        let f = Vec3::new(0.0, -(30f32.to_radians().sin()), 30f32.to_radians().cos());
        let d = ray_cylinder(Vec3::new(0.0, 1.6, 0.0), f, Vec3::new(0.0, 0.0, 2.5), 0.5, 1.8).unwrap();
        assert!((d * f.z - 2.0).abs() < 1e-3, "entered at horizontal 2.0 m, got {}", d * f.z);
    }
}
