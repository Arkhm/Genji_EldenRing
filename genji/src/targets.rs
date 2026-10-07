//! Living characters near the player as targets for Genji's attacks (shuriken, Swift Strike): collision cylinder
//! and a pointer to change their HP. Moved out of `shuriken.rs` (step 7) so every attack uses the same list.

use std::ptr::NonNull;

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
    crate::kills::damaged(target.chr.as_ptr() as usize);
    Some((before, data.hp, data.max_hp))
}
