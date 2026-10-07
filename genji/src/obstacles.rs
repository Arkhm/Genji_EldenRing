//! Other characters (enemies, animals, NPCs) as obstacles for our movement controller.
//!
//! Our collision rays only see the map. In the 5b test (2026-10-07), walking into an animal put us partly
//! inside it every frame, and the animal took repeated damage. Each nearby character is now treated as an
//! upright cylinder (its collision radius and height): we can walk up to it and slide around it, not into it.
//! A contact monitor logs any HP change of a character we touch, to check the damage is gone.

use std::collections::HashMap;

use eldenring::cs::{ChrIns, WorldChrMan};
use fromsoftware_shared::FromStatic;
use glam::Vec3;

use crate::glog;

/// Characters further than this (horizontally) are ignored, metres.
const RANGE: f32 = 6.0;
/// How many entries of the game's distance-sorted character list we look at.
const MAX_CHECKED: usize = 64;
/// Extra distance that still counts as "touching" for the contact monitor, metres.
const CONTACT_MARGIN: f32 = 0.15;

#[derive(Clone, Copy)]
pub struct Obstacle {
    id: usize,
    pos: Vec3,
    radius: f32,
    height: f32,
    hp: i32,
}

/// Living characters near `center`, except `me`.
pub fn nearby(me: &ChrIns, center: Vec3) -> Vec<Obstacle> {
    let Ok(world) = (unsafe { WorldChrMan::instance() }) else {
        return Vec::new();
    };
    let me_id = me as *const ChrIns as usize;
    world
        .chr_inses_by_distance
        .iter()
        .take(MAX_CHECKED)
        .filter_map(|entry| {
            let id = entry.chr_ins.as_ptr() as usize;
            if id == me_id {
                return None;
            }
            let chr = unsafe { entry.chr_ins.as_ref() };
            let modules = &chr.modules;
            if modules.data.hp <= 0 {
                return None;
            }
            let physics = &modules.physics;
            let p = physics.position;
            let pos = Vec3::new(p.0, p.1, p.2);
            if Vec3::new(pos.x - center.x, 0.0, pos.z - center.z).length() > RANGE {
                return None;
            }
            let radius = physics.hit_radius.max(physics.chr_hit_radius).clamp(0.2, 3.0);
            let height = physics.hit_height.max(physics.chr_hit_height).clamp(0.5, 8.0);
            Some(Obstacle { id, pos, radius, height, hp: modules.data.hp })
        })
        .collect()
}

/// Removes the part of `delta` (horizontal move from `start`) that would take us into a character.
/// Returns the clipped move and whether anything was in the way.
pub fn clip(start: Vec3, delta: Vec3, my_radius: f32, my_height: f32, obstacles: &[Obstacle]) -> (Vec3, bool) {
    let mut delta = delta;
    let mut blocked = false;
    for ob in obstacles {
        // Only characters overlapping us vertically.
        if ob.pos.y > start.y + my_height || ob.pos.y + ob.height < start.y {
            continue;
        }
        let min = my_radius + ob.radius;
        let to = Vec3::new(ob.pos.x - start.x, 0.0, ob.pos.z - start.z);
        let now = to.length();
        let after = Vec3::new(ob.pos.x - (start.x + delta.x), 0.0, ob.pos.z - (start.z + delta.z)).length();
        if after < min && after < now && now > 1e-4 {
            // Moving into it: drop the component of the move that points at it (slide around it).
            let n = to / now;
            let toward = delta.dot(n);
            if toward > 0.0 {
                delta -= n * toward;
                blocked = true;
            }
        }
    }
    (delta, blocked)
}

/// Logs HP changes of characters we touch.
#[derive(Default)]
pub struct ContactMonitor {
    touching: HashMap<usize, i32>,
}

impl ContactMonitor {
    /// `my_animation`: the animation the game plays on us, logged with any damage to show whether the game
    /// still treats us as attacking or falling.
    pub fn update(&mut self, me: Vec3, my_radius: f32, obstacles: &[Obstacle], my_animation: i32) {
        let mut now = HashMap::new();
        for ob in obstacles {
            let dist = Vec3::new(ob.pos.x - me.x, 0.0, ob.pos.z - me.z).length();
            if dist <= my_radius + ob.radius + CONTACT_MARGIN {
                match self.touching.get(&ob.id) {
                    None => glog!(
                        "TOUCH",
                        "touching character {:#x} (radius {:.2}, hp {}) | our animation {my_animation}",
                        ob.id, ob.radius, ob.hp
                    ),
                    Some(&before) if ob.hp < before => glog!(
                        "TOUCH",
                        "character {:#x} LOST HP while we touch it: {before} -> {} | our animation {my_animation}",
                        ob.id, ob.hp
                    ),
                    _ => {}
                }
                now.insert(ob.id, ob.hp);
            }
        }
        for id in self.touching.keys() {
            if !now.contains_key(id) {
                glog!("TOUCH", "no longer touching {id:#x}");
            }
        }
        self.touching = now;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ob(x: f32, y: f32, z: f32) -> Obstacle {
        Obstacle { id: 1, pos: Vec3::new(x, y, z), radius: 0.5, height: 1.8, hp: 100 }
    }

    #[test]
    fn walking_straight_into_a_character_stops() {
        // Character 0.9 m ahead along +z; we're 0.35 + 0.5 = 0.85 m apart at contact. Move 0.1 m toward it.
        let (d, blocked) = clip(Vec3::ZERO, Vec3::new(0.0, 0.0, 0.1), 0.35, 1.75, &[ob(0.0, 0.0, 0.9)]);
        assert!(blocked);
        assert!(d.length() < 1e-5, "moved {d:?}");
    }

    #[test]
    fn diagonal_move_slides_around() {
        let (d, blocked) = clip(Vec3::ZERO, Vec3::new(0.1, 0.0, 0.1), 0.35, 1.75, &[ob(0.0, 0.0, 0.9)]);
        assert!(blocked);
        assert!((d.x - 0.1).abs() < 1e-5 && d.z.abs() < 1e-5, "slide kept {d:?}");
    }

    #[test]
    fn walking_away_is_never_blocked() {
        let delta = Vec3::new(0.0, 0.0, -0.1);
        let (d, blocked) = clip(Vec3::ZERO, delta, 0.35, 1.75, &[ob(0.0, 0.0, 0.5)]);
        assert!(!blocked);
        assert_eq!(d, delta);
    }

    #[test]
    fn far_or_other_height_is_ignored() {
        let delta = Vec3::new(0.0, 0.0, 0.1);
        assert!(!clip(Vec3::ZERO, delta, 0.35, 1.75, &[ob(0.0, 0.0, 3.0)]).1, "far ahead");
        assert!(!clip(Vec3::ZERO, delta, 0.35, 1.75, &[ob(0.0, 3.0, 0.9)]).1, "above our head");
        assert!(!clip(Vec3::ZERO, delta, 0.35, 1.75, &[ob(0.0, -3.0, 0.9)]).1, "far below our feet");
    }
}
