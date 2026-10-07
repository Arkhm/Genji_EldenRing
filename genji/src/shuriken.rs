//! Shuriken (step 6a): left click throws one projectile from the eye along where you look.
//!
//! The engine does the flying, collision, damage, hit effects and enemy reactions: we only ask it to spawn one of
//! its own projectiles ("bullets") with `CSBulletManager::spawn_bullet`. For now the projectile is Elden Ring's
//! Throwing Dagger (Bullet row 10170000, attack row 10170000: 67 physical, 32 m/s, 15 m range), found by reading
//! a copy of your `regulation.bin` (`erdata/`, never committed).
//!
//! The spawn request: fromsoftware-rs's `BulletSpawnData` has private fields and no constructor, so we fill our
//! own copy of its 0x110-byte layout. The values follow a working Elden Ring mod (justbustin/minecraft-crossover-
//! bridge, MIT, `er-bridge/src/game.cpp`): owner = the player's handle (hits, aggro and kills count as ours);
//! behavior, magic, goods and dummy-poly ids -1; no target; right/up/forward/position at 0x50/0x60/0x70/0x80;
//! everything else zero.
//!
//! A hit watcher logs HP drops of nearby characters for 2 s after each throw (`HIT` lines).

use std::collections::HashMap;

use eldenring::cs::{
    AtkParam_Pc, Bullet as BulletParam, BulletSpawnData, CSBulletManager, PlayerIns, SoloParamRepository, WorldChrMan,
};
use fromsoftware_shared::FromStatic;
use glam::Vec3;

use crate::glog;
use crate::look::Look;

/// Elden Ring's Throwing Dagger projectile (Bullet and AtkParam_Pc row).
const THROWING_DAGGER_BULLET: i32 = 10_170_000;
/// The throw starts from Genji's right hand, not the middle of the view (6a test: the dagger's hilt showed in the
/// middle of the screen at spawn): this far forward, right and down from the eye, metres.
const HAND_FORWARD: f32 = 0.4;
const HAND_RIGHT: f32 = 0.18;
const HAND_DOWN: f32 = 0.15;
/// The throw is aimed at the crosshair point this far ahead, so it still lands where you look, metres.
/// The miss from the hand offset is about 0.13 m at 5 m and none at this distance.
const AIM_DISTANCE: f32 = 20.0;
/// Shuriken flight, set in memory on the dagger's Bullet row while Genji mode is active (6c): straight (no
/// gravity, no slow-down), fast and long range. Approximate Overwatch values, not verified. The game's file is
/// never changed; the original values are restored whenever Genji mode lets go (menus, F9), so a real Throwing
/// Dagger thrown in plain Elden Ring is unchanged.
const SHURIKEN_SPEED: f32 = 60.0;
const SHURIKEN_RANGE: f32 = 80.0;
/// Base physical attack of the dagger's attack row while Genji mode is active (vanilla 67; the 6a test did 3 to 5
/// damage per hit on an animal). A first guess, to be tuned in game.
const SHURIKEN_ATK_PHYS: u16 = 150;
/// How long after a throw HP drops are attributed to it, seconds.
const HIT_WATCH_SECONDS: f32 = 2.0;
/// Characters further than this from the player are not watched, metres.
const HIT_WATCH_RANGE: f32 = 40.0;
/// Entries of the game's distance-sorted character list to watch.
const HIT_WATCH_MAX: usize = 64;

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

/// The vanilla values we change, kept to restore them.
#[derive(Clone, Copy)]
struct VanillaValues {
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

#[derive(Default)]
pub struct Thrower {
    /// Some while our shuriken values are applied in memory.
    vanilla: Option<VanillaValues>,
    clock: f32,
    throws: u32,
    watch_until: f32,
    /// Character pointer -> HP last seen, while watching for hits.
    watched: HashMap<usize, i32>,
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
        match (on, self.vanilla) {
            (true, None) => {
                // Read the vanilla values first, then write ours.
                let (Some(bullet), Some(atk)) = (params.get::<BulletParam>(id), params.get::<AtkParam_Pc>(id)) else {
                    glog!("THROW", "Bullet or AtkParam_Pc row {id} not found; shuriken values not applied");
                    return;
                };
                let vanilla = VanillaValues {
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
                let shuriken = VanillaValues {
                    init_velocity: SHURIKEN_SPEED,
                    max_velocity: SHURIKEN_SPEED,
                    min_velocity: SHURIKEN_SPEED,
                    accel_out_range: 0.0,
                    gravity_in_range: 0.0,
                    gravity_out_range: 0.0,
                    life: SHURIKEN_RANGE / SHURIKEN_SPEED + 0.1,
                    dist: SHURIKEN_RANGE,
                    atk_phys: SHURIKEN_ATK_PHYS,
                };
                write_values(params, id, &shuriken);
                glog!(
                    "THROW",
                    "shuriken values ON: speed {} -> {SHURIKEN_SPEED} m/s, gravity {}/{} -> 0, range {} -> {SHURIKEN_RANGE} m, atk {} -> {SHURIKEN_ATK_PHYS}",
                    vanilla.init_velocity, vanilla.gravity_in_range, vanilla.gravity_out_range, vanilla.dist, vanilla.atk_phys
                );
                self.vanilla = Some(vanilla);
            }
            (false, Some(vanilla)) => {
                write_values(params, id, &vanilla);
                glog!("THROW", "shuriken values OFF: vanilla Throwing Dagger restored");
                self.vanilla = None;
            }
            _ => {}
        }
    }

    /// Throws one projectile from `eye` along where `look` points.
    pub fn throw(&mut self, player: &PlayerIns, look: &Look, eye: Vec3) {
        let view_forward = look.forward();
        let view_right = look.right(); // horizontal right is perpendicular to any pitched forward (no roll)
        let view_up = look.up();
        let from = eye + view_forward * HAND_FORWARD + view_right * HAND_RIGHT - view_up * HAND_DOWN;
        let aim = eye + view_forward * AIM_DISTANCE;
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
            "#{} bullet {THROWING_DAGGER_BULLET} from ({:.2}, {:.2}, {:.2}) dir ({:.2}, {:.2}, {:.2}) owner {owner:#018x}: {}",
            self.throws,
            from.x, from.y, from.z,
            forward.x, forward.y, forward.z,
            match result {
                Ok(()) => "ok".to_string(),
                Err(code) => format!("FAILED, engine error {code} (3 = invalid bullet id, 4 = construction failed)"),
            }
        );
        if result.is_ok() {
            if self.clock >= self.watch_until {
                self.watched = snapshot(player);
            }
            self.watch_until = self.clock + HIT_WATCH_SECONDS;
        }
    }

    /// Every frame: logs HP drops of watched characters shortly after a throw.
    pub fn tick(&mut self, player: &PlayerIns, dt: f32) {
        self.clock += dt;
        if self.clock >= self.watch_until || self.watched.is_empty() {
            return;
        }
        let me = player_pos(player);
        for (id, hp, pos) in characters(player) {
            match self.watched.get(&id) {
                Some(&before) if hp < before => {
                    glog!(
                        "HIT",
                        "character {id:#x} hp {before} -> {hp} ({} damage), {:.1} m away",
                        before - hp,
                        (pos - me).length()
                    );
                    self.watched.insert(id, hp);
                }
                None if (pos - me).length() <= HIT_WATCH_RANGE => {
                    self.watched.insert(id, hp);
                }
                _ => {}
            }
        }
    }
}

/// Writes flight and damage values to the Bullet and AtkParam_Pc rows `id` in memory.
fn write_values(params: &mut SoloParamRepository, id: u32, v: &VanillaValues) {
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

fn player_pos(player: &PlayerIns) -> Vec3 {
    let p = player.chr_ins.modules.physics.position;
    Vec3::new(p.0, p.1, p.2)
}

/// Nearby characters except the player: (pointer, HP, position).
fn characters(player: &PlayerIns) -> Vec<(usize, i32, Vec3)> {
    let Ok(world) = (unsafe { WorldChrMan::instance() }) else {
        return Vec::new();
    };
    let me = &player.chr_ins as *const _ as usize;
    world
        .chr_inses_by_distance
        .iter()
        .take(HIT_WATCH_MAX)
        .filter_map(|entry| {
            let id = entry.chr_ins.as_ptr() as usize;
            if id == me {
                return None;
            }
            let chr = unsafe { entry.chr_ins.as_ref() };
            let p = chr.modules.physics.position;
            Some((id, chr.modules.data.hp, Vec3::new(p.0, p.1, p.2)))
        })
        .collect()
}

fn snapshot(player: &PlayerIns) -> HashMap<usize, i32> {
    let me = player_pos(player);
    characters(player)
        .into_iter()
        .filter(|(_, _, pos)| (*pos - me).length() <= HIT_WATCH_RANGE)
        .map(|(id, hp, _)| (id, hp))
        .collect()
}
