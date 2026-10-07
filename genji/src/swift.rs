//! Swift Strike (step 7): cooldown, kill reset and damage along the dash. The dash movement itself is in
//! `movement.rs` (DASH_*). Values from memory of Overwatch, **not verified** (DESIGN.md, 5d).

use std::collections::HashSet;

use eldenring::cs::PlayerIns;
use glam::Vec3;

use crate::glog;
use crate::targets::{self, Target};

/// Damage to each character the dash passes through.
const DAMAGE: i32 = 50;
/// Cooldown after a dash starts, seconds.
const COOLDOWN: f32 = 8.0;
/// Our body as seen by the dash's hit test: radius and height in metres (same as the movement controller).
const BODY_RADIUS: f32 = 0.35;
const BODY_HEIGHT: f32 = 1.75;

#[derive(Default)]
pub struct Swift {
    /// Seconds until the next dash is allowed.
    cooldown: f32,
    /// Characters already hit by the current dash (each only once).
    hit: HashSet<usize>,
    dashes: u32,
}

impl Swift {
    /// Once per frame: cooldown, and kill reset (`eliminations` from `kills::tick`).
    pub fn tick(&mut self, dt: f32, eliminations: u32) {
        self.cooldown = (self.cooldown - dt).max(0.0);
        if eliminations > 0 && self.cooldown > 0.0 {
            glog!("SWIFT", "reset by an elimination ({:.1} s of cooldown left)", self.cooldown);
            self.cooldown = 0.0;
        }
    }

    pub fn ready(&self) -> bool {
        self.cooldown <= 0.0
    }

    /// A dash started (the movement controller accepted it).
    pub fn started(&mut self) {
        self.cooldown = COOLDOWN;
        self.hit.clear();
        self.dashes += 1;
        glog!("SWIFT", "dash #{} started, cooldown {COOLDOWN} s", self.dashes);
    }

    /// The dash moved our feet from `from` to `to` this frame: damage every character on that stretch once.
    pub fn sweep(&mut self, player: &PlayerIns, from: Vec3, to: Vec3) {
        for target in targets::living(player) {
            let id = target.chr.as_ptr() as usize;
            if self.hit.contains(&id) || !touches(from, to, &target) {
                continue;
            }
            self.hit.insert(id);
            if let Some((before, after, max)) = targets::damage(&target, DAMAGE) {
                glog!("DASH", "HIT {DAMAGE} on character {id:#x}: hp {before} -> {after} / {max}");
            }
        }
    }
}

/// Does our body, moving with its feet from `from` to `to`, touch the target's cylinder?
fn touches(from: Vec3, to: Vec3, target: &Target) -> bool {
    // Vertical overlap anywhere along the stretch.
    let (low, high) = (from.y.min(to.y), from.y.max(to.y) + BODY_HEIGHT);
    if high < target.feet.y || low > target.feet.y + target.height {
        return false;
    }
    // Horizontal distance from the target's axis to the stretch.
    let a = Vec3::new(from.x, 0.0, from.z);
    let b = Vec3::new(to.x, 0.0, to.z);
    let p = Vec3::new(target.feet.x, 0.0, target.feet.z);
    let ab = b - a;
    let t = if ab.length_squared() < 1e-8 { 0.0 } else { ((p - a).dot(ab) / ab.length_squared()).clamp(0.0, 1.0) };
    (a + ab * t - p).length() <= target.radius + BODY_RADIUS
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ptr::NonNull;

    fn target(x: f32, z: f32, y: f32) -> Target {
        Target { chr: NonNull::dangling(), feet: Vec3::new(x, y, z), radius: 0.5, height: 1.8 }
    }

    #[test]
    fn passing_through_or_beside_hits() {
        let (from, to) = (Vec3::ZERO, Vec3::new(0.0, 0.0, -1.0));
        assert!(touches(from, to, &target(0.0, -0.5, 0.0)));
        assert!(touches(from, to, &target(0.8, -0.5, 0.0)), "body radius + target radius = 0.85");
        assert!(!touches(from, to, &target(0.9, -0.5, 0.0)));
    }

    #[test]
    fn ahead_behind_above_miss() {
        let (from, to) = (Vec3::ZERO, Vec3::new(0.0, 0.0, -1.0));
        assert!(!touches(from, to, &target(0.0, -2.0, 0.0)));
        assert!(!touches(from, to, &target(0.0, 1.0, 0.0)));
        assert!(!touches(from, to, &target(0.0, -0.5, 2.0)));
    }

    #[test]
    fn cooldown_and_kill_reset() {
        let mut s = Swift::default();
        assert!(s.ready());
        s.started();
        s.tick(7.9, 0);
        assert!(!s.ready());
        s.tick(0.0, 1);
        assert!(s.ready());
    }
}
