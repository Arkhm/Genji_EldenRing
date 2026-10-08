//! Swift Strike (step 7): cooldown, kill reset and damage along the dash. The dash movement itself is in
//! `movement.rs` (DASH_*). Values from memory of Overwatch, **not verified** (DESIGN.md, 5d).

use std::collections::{HashMap, HashSet};

use eldenring::cs::PlayerIns;
use glam::Vec3;

use crate::glog;
use crate::targets::{self, Target};

/// Damage to each character the dash passes through.
const DAMAGE: i32 = 50;
/// Cooldown after a dash starts, seconds.
const COOLDOWN: f32 = 8.0;
/// Characters that came this close (gap between the cylinders, metres) without being hit are logged at the end of
/// the dash (diagnostic for missed dashes, test of 2026-10-07).
const NEAR_MISS: f32 = 2.0;
/// Our body as seen by the dash's hit test: radius and height in metres (same as the movement controller).
const BODY_RADIUS: f32 = 0.35;
/// Extra reach of the dash's hit beyond our body, metres. Test of 2026-10-08: misses passed 0.07-0.46 m outside the
/// body-only reach (soldiers, radius 0.40), while the dash visibly went through them. With 0.5 the dash hits what it
/// passes within 1.25 m of a soldier's centre. Overwatch's exact width is not verified.
const REACH: f32 = 0.5;
const BODY_HEIGHT: f32 = 1.75;

#[derive(Default)]
pub struct Swift {
    /// Seconds until the next dash is allowed.
    cooldown: f32,
    /// Characters already hit by the current dash (each only once).
    hit: HashSet<usize>,
    /// Closest approach of characters not hit in the current dash: gap in metres, vertical overlap, their radius.
    near: HashMap<usize, (f32, bool, f32)>,
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
        self.near.clear();
        self.dashes += 1;
        glog!("SWIFT", "dash #{} started, cooldown {COOLDOWN} s", self.dashes);
    }

    /// The dash moved our feet from `from` to `to` this frame: damage every character on that stretch once.
    pub fn sweep(&mut self, player: &PlayerIns, from: Vec3, to: Vec3) {
        for target in targets::living(player) {
            let id = target.chr.as_ptr() as usize;
            if self.hit.contains(&id) {
                continue;
            }
            let (gap, overlaps) = closest(from, to, &target);
            if gap > 0.0 || !overlaps {
                let entry = self.near.entry(id).or_insert((f32::INFINITY, false, target.radius));
                if gap < entry.0 {
                    *entry = (gap, overlaps, target.radius);
                }
                continue;
            }
            self.near.remove(&id);
            self.hit.insert(id);
            let damage = crate::scaling::scaled(DAMAGE as f32, player);
            if let Some((before, after, max)) = targets::damage(&target, damage) {
                glog!("DASH", "HIT {damage} (dexterity-scaled from {DAMAGE}) on character {id:#x}: hp {before} -> {after} / {max}");
            }
        }
    }

    /// The dash ended: log characters that came close but weren't hit.
    pub fn ended(&mut self) {
        let mut near: Vec<_> = self.near.iter().filter(|(_, n)| n.0 <= NEAR_MISS).collect();
        near.sort_by(|a, b| a.1.0.total_cmp(&b.1.0));
        for (id, (gap, overlaps, radius)) in near {
            glog!(
                "DASH",
                "near miss: character {id:#x} (radius {radius:.2}) passed {gap:.2} m outside our reach{}",
                if *overlaps { "" } else { ", and not at our height" }
            );
        }
        glog!("DASH", "dash #{}: {} hit, {} near misses within {NEAR_MISS} m", self.dashes, self.hit.len(), self.near.values().filter(|n| n.0 <= NEAR_MISS).count());
        self.near.clear();
    }
}

/// Does our body, moving with its feet from `from` to `to`, touch the target's cylinder?
#[cfg(test)]
fn touches(from: Vec3, to: Vec3, target: &Target) -> bool {
    let (gap, overlaps) = closest(from, to, target);
    gap <= 0.0 && overlaps
}

/// Closest approach of our body moving with its feet from `from` to `to` to the target's cylinder: horizontal gap
/// between the two (negative or 0 = touching) and whether they overlap vertically along the stretch.
fn closest(from: Vec3, to: Vec3, target: &Target) -> (f32, bool) {
    let (low, high) = (from.y.min(to.y), from.y.max(to.y) + BODY_HEIGHT);
    let overlaps = high >= target.feet.y && low <= target.feet.y + target.height;
    // Horizontal distance from the target's axis to the stretch.
    let a = Vec3::new(from.x, 0.0, from.z);
    let b = Vec3::new(to.x, 0.0, to.z);
    let p = Vec3::new(target.feet.x, 0.0, target.feet.z);
    let ab = b - a;
    let t = if ab.length_squared() < 1e-8 { 0.0 } else { ((p - a).dot(ab) / ab.length_squared()).clamp(0.0, 1.0) };
    ((a + ab * t - p).length() - target.radius - BODY_RADIUS - REACH, overlaps)
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
        assert!(touches(from, to, &target(1.3, -0.5, 0.0)), "body radius + target radius + reach = 1.35");
        assert!(!touches(from, to, &target(1.4, -0.5, 0.0)));
    }

    #[test]
    fn ahead_behind_above_miss() {
        let (from, to) = (Vec3::ZERO, Vec3::new(0.0, 0.0, -1.0));
        assert!(!touches(from, to, &target(0.0, -2.5, 0.0)));
        assert!(!touches(from, to, &target(0.0, 1.5, 0.0)));
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
