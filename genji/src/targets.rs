//! Living characters near the player as targets for Genji's attacks (shuriken, Swift Strike): collision cylinder
//! and a pointer to change their HP. Moved out of `shuriken.rs` (step 7) so every attack uses the same list.

use std::collections::HashMap;
use std::ptr::NonNull;
use std::sync::Mutex;

use eldenring::cs::{ChrIns, PlayerIns, WorldChrMan};
use fromsoftware_shared::FromStatic;
use glam::Vec3;

/// Entries of the game's distance-sorted character list to consider.
const CHARACTERS_MAX: usize = 64;

pub struct Target {
    pub chr: NonNull<ChrIns>,
    pub feet: Vec3,
    pub radius: f32,
    pub height: f32,
}

/// Living characters near the player (not the player).
pub fn living(player: &PlayerIns) -> Vec<Target> {
    list(player, false)
}

/// Characters near the player, dead ones included (a shuriken that kills outright: the game's damage lands before
/// we look, so the character is already dead when we match the shuriken to it).
pub fn all(player: &PlayerIns) -> Vec<Target> {
    list(player, true)
}

fn list(player: &PlayerIns, include_dead: bool) -> Vec<Target> {
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
            if chr.modules.data.hp <= 0 && !include_dead {
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

/// Total HP we took from each character ourselves (by address), so HP changes made by the game itself can be told
/// apart from ours (`shuriken.rs` checks the game's own hit before applying shuriken damage).
static OWN: Mutex<Option<HashMap<usize, i64>>> = Mutex::new(None);

/// All HP we have taken from this character ourselves so far.
pub fn own_damage(chr: usize) -> i64 {
    let guard = OWN.lock().unwrap_or_else(|e| e.into_inner());
    guard.as_ref().and_then(|m| m.get(&chr).copied()).unwrap_or(0)
}

/// Deals `damage` to a target by lowering its HP (never below 0); registers it for kill credit (`kills.rs`).
/// Returns (HP before, HP after, max HP), or None if it was already dead.
pub fn damage(target: &Target, damage: i32) -> Option<(i32, i32, i32)> {
    let chr = unsafe { &mut *target.chr.as_ptr() };
    let data = &mut chr.modules.data;
    if data.hp <= 0 {
        return None;
    }
    let before = data.hp;
    data.hp = (data.hp - damage).max(0);
    let id = target.chr.as_ptr() as usize;
    *OWN.lock().unwrap_or_else(|e| e.into_inner()).get_or_insert_with(HashMap::new).entry(id).or_insert(0) +=
        (before - data.hp) as i64;
    crate::kills::damaged(target.chr.as_ptr() as usize);
    Some((before, data.hp, data.max_hp))
}
