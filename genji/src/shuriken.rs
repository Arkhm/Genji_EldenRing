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

use eldenring::cs::{BulletSpawnData, CSBulletManager, PlayerIns, WorldChrMan};
use fromsoftware_shared::FromStatic;
use glam::Vec3;

use crate::glog;
use crate::look::Look;

/// Elden Ring's Throwing Dagger projectile (Bullet and AtkParam_Pc row).
const THROWING_DAGGER_BULLET: i32 = 10_170_000;
/// Spawn this far in front of the eye, metres.
const SPAWN_FORWARD: f32 = 0.5;
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

#[derive(Default)]
pub struct Thrower {
    clock: f32,
    throws: u32,
    watch_until: f32,
    /// Character pointer -> HP last seen, while watching for hits.
    watched: HashMap<usize, i32>,
}

impl Thrower {
    /// Throws one projectile from `eye` along where `look` points.
    pub fn throw(&mut self, player: &PlayerIns, look: &Look, eye: Vec3) {
        let forward = look.forward();
        let right = look.right(); // horizontal right is perpendicular to any pitched forward (no roll)
        let up = forward.cross(right).normalize_or(Vec3::Y);
        let up = if up.y < 0.0 { -up } else { up };
        let from = eye + forward * SPAWN_FORWARD;
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
