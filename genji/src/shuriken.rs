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

/// Elden Ring's Throwing Dagger projectile (Bullet and AtkParam_Pc row).
const THROWING_DAGGER_BULLET: i32 = 10_170_000;

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
    atk_phys: u16,
}

/// A shuriken in flight, as last seen.
struct Flying {
    pos: Vec3,
    /// Velocity, m/s: at 60 m/s a shuriken moves about 1 m per frame, so its last seen position can be well short
    /// of where it hit; the hit test covers the stretch it flew after that.
    vel: Vec3,
    age: f32,
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
        let id = THROWING_DAGGER_BULLET as u32;
        match self.vanilla {
            None => {
                let (Some(bullet), Some(atk)) = (params.get::<BulletParam>(id), params.get::<AtkParam_Pc>(id)) else {
                    glog!("THROW", "Bullet or AtkParam_Pc row {id} not found; shuriken values not applied");
                    return;
                };
                let vanilla = RowValues {
                    init_velocity: bullet.init_vellocity(),
                    max_velocity: bullet.max_vellocity(),
                    min_velocity: bullet.min_vellocity(),
                    accel_out_range: bullet.accel_out_range(),
                    gravity_in_range: bullet.gravity_in_range(),
                    gravity_out_range: bullet.gravity_out_range(),
                    life: bullet.life(),
                    dist: bullet.dist(),
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
                    atk_phys: ENGINE_ATK_PHYS,
                };
                write_values(params, id, &shuriken);
                glog!(
                    "THROW",
                    "shuriken values ON: speed {} -> {SHURIKEN_SPEED} m/s, gravity {} -> 0 then {OUT_OF_RANGE_GRAVITY} after {SHURIKEN_RANGE} m (was after {} m), life {} -> {SHURIKEN_LIFE} s, engine atk {} -> {ENGINE_ATK_PHYS}",
                    vanilla.init_velocity, vanilla.gravity_in_range, vanilla.dist, vanilla.life, vanilla.atk_phys
                );
                self.vanilla = Some(vanilla);
            }
            Some(vanilla) => {
                write_values(params, id, &vanilla);
                glog!("THROW", "shuriken values OFF: vanilla Throwing Dagger restored");
                self.vanilla = None;
            }
        }
    }

    /// Throws one shuriken from Genji's hand toward what is under the crosshair.
    pub fn throw(&mut self, player: &PlayerIns, look: &Look, eye: Vec3) {
        let view_forward = look.forward();
        let view_right = look.right(); // horizontal right is perpendicular to any pitched forward (no roll)
        let view_up = look.up();
        let aim_distance = aim_distance(player, eye, view_forward);
        let aim = eye + view_forward * aim_distance;
        let from = eye + view_forward * HAND_FORWARD + view_right * HAND_RIGHT - view_up * HAND_DOWN;
        let forward = (aim - from).normalize_or(view_forward);
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
            bullet_id: THROWING_DAGGER_BULLET,
            goods_id: -1,
            dummy_poly_id: -1,
            target: u64::MAX,
            unk28: 0,
            unk2c: 0,
            unk30: [0.0; 4],
            unk40: 0,
            unk44: 0,
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
        glog!(
            "THROW",
            "#{} aim {aim_distance:.1} m, from ({:.2}, {:.2}, {:.2}) dir ({:.2}, {:.2}, {:.2}): {}",
            self.throws,
            from.x, from.y, from.z,
            forward.x, forward.y, forward.z,
            match result {
                Ok(()) => "ok".to_string(),
                Err(code) => format!(
                    "FAILED, engine error {code} (3 = invalid bullet id, 4 = construction failed); {} of ours flying",
                    self.flying.len()
                ),
            }
        );
    }

    /// Every frame: follows our shuriken in flight; when one ends next to a character, applies Overwatch damage.
    pub fn tick(&mut self, player: &PlayerIns, dt: f32) {
        self.clock += dt;
        let Ok(manager) = (unsafe { CSBulletManager::instance() }) else {
            return;
        };
        let mut now: HashMap<u64, Flying> = HashMap::new();
        let mut sample_life = None;
        for bullet in manager.bullets() {
            if bullet.fly_state.base.param.param_id != THROWING_DAGGER_BULLET {
                continue;
            }
            let handle = unsafe { std::ptr::read(&bullet.field_ins_handle as *const _ as *const u64) };
            let p = &bullet.physics.position;
            // `life` and `life_max` follow `time_alive` in CSBulletIns (private in fromsoftware-rs).
            let life_ptr = &bullet.time_alive as *const f32;
            sample_life.get_or_insert_with(|| unsafe { (life_ptr.add(1).read(), life_ptr.add(2).read()) });
            let v = &bullet.physics.velocity;
            now.insert(
                handle,
                Flying { pos: Vec3::new(p.0, p.1, p.2), vel: Vec3::new(v.0, v.1, v.2), age: bullet.time_alive },
            );
        }
        let ended: Vec<Flying> = self
            .flying
            .drain()
            .filter(|(handle, _)| !now.contains_key(handle))
            .map(|(_, f)| f)
            .collect();
        self.flying = now;
        for shuriken in ended {
            self.ended += 1;
            self.resolve(player, &shuriken, dt);
        }

        self.report_timer += dt;
        if self.report_timer >= REPORT_EVERY && (self.throws > 0) {
            self.report_timer = 0.0;
            let oldest = self.flying.values().map(|f| f.age).fold(0.0, f32::max);
            glog!(
                "SHURIKEN",
                "flying {} (oldest {oldest:.1} s{}) | thrown {} ended {} hits {} headshots {}",
                self.flying.len(),
                sample_life.map(|(l, m)| format!(", engine life {l:.2} / max {m:.2}")).unwrap_or_default(),
                self.throws,
                self.ended,
                self.hits,
                self.headshots
            );
        }
    }

    /// A shuriken ended somewhere between its last seen position and where its velocity would have taken it by
    /// the next frame: if that stretch passes next to a character, it hit it (closest character wins).
    fn resolve(&mut self, player: &PlayerIns, shuriken: &Flying, dt: f32) {
        const SAMPLES: usize = 8;
        let reach = shuriken.vel * (dt.max(1.0 / 60.0) * 1.5);
        let mut best: Option<(f32, Target, Vec3)> = None;
        for target in targets(player) {
            for i in 0..=SAMPLES {
                let pos = shuriken.pos + reach * (i as f32 / SAMPLES as f32);
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
            shuriken.age,
            100.0 * (pos.y - target.feet.y) / target.height
        );
    }
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
    }
    if let Some(atk) = params.get_mut::<AtkParam_Pc>(id) {
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
