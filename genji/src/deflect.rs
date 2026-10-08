//! Deflect (step 8a, DESIGN.md 5f): E holds the sword up for DURATION seconds (E again ends it early), then COOLDOWN.
//! While it runs, the player can't be damaged from the front: the game's "invincible" character flag is set on every
//! frame where nothing threatens from behind (no living enemy within REAR_RANGE in our rear half, no enemy
//! projectile closing in from behind). Sending projectiles back is step 8b.

use eldenring::cs::{CSBulletManager, PlayerIns, WorldChrMan};
use fromsoftware_shared::FromStatic;
use glam::Vec3;

use crate::glog;

/// Overwatch: 2 s, ended early by pressing E again.
const DURATION: f32 = 2.0;
/// Overwatch: 8 s (a 2025 patch raised it to a value not confirmed here).
const COOLDOWN: f32 = 8.0;
/// Enemies and enemy projectiles closer than this in our rear half can hit us: no protection on those frames.
const REAR_RANGE: f32 = 4.0;

#[derive(Default)]
pub struct Deflect {
    /// Seconds left while deflecting.
    left: Option<f32>,
    /// Seconds until it can be used again.
    cooldown: f32,
    uses: u32,
    /// We set the invincible flag (so we only ever clear our own).
    flag_set: bool,
    /// Frames this use where a rear threat switched protection off.
    rear_frames: u32,
    last_hp: Option<i32>,
}

impl Deflect {
    /// Once per frame. `e_pressed`: E went down this frame; `can_use`: our controller drives.
    /// Returns true while deflecting.
    pub fn tick(&mut self, dt: f32, e_pressed: bool, can_use: bool) -> bool {
        self.cooldown = (self.cooldown - dt).max(0.0);
        match self.left {
            Some(left) => {
                let left = left - dt;
                if !can_use {
                    self.end("Genji mode let go");
                } else if e_pressed {
                    self.end("E pressed again");
                } else if left <= 0.0 {
                    self.end("time up");
                } else {
                    self.left = Some(left);
                }
            }
            None if e_pressed && can_use => {
                if self.cooldown > 0.0 {
                    glog!("DEFLECT", "not ready ({:.1} s left)", self.cooldown);
                } else {
                    self.left = Some(DURATION);
                    self.uses += 1;
                    self.rear_frames = 0;
                    self.last_hp = None;
                    glog!("DEFLECT", "#{} start ({DURATION} s)", self.uses);
                }
            }
            None => {}
        }
        self.left.is_some()
    }

    fn end(&mut self, why: &str) {
        self.left = None;
        self.cooldown = COOLDOWN;
        glog!(
            "DEFLECT",
            "#{} end ({why}); protection off on {} frames because of a threat from behind; cooldown {COOLDOWN} s",
            self.uses,
            self.rear_frames
        );
    }

    pub fn active(&self) -> bool {
        self.left.is_some()
    }

    /// While deflecting: sets the invincible flag unless something threatens from behind. `feet`: our position;
    /// `forward`: where we look, flat. Call every frame (also when not deflecting: it clears the flag).
    pub fn apply(&mut self, player: &mut PlayerIns, feet: Vec3, forward: Vec3) {
        if !self.active() {
            self.release(player);
            return;
        }
        let hp = player.chr_ins.modules.data.hp;
        if let Some(last) = self.last_hp {
            if hp != last {
                glog!("DEFLECT", "HP changed while deflecting: {last} -> {hp} (protection {})", if self.flag_set { "on" } else { "off" });
            }
        }
        self.last_hp = Some(hp);
        let threat = rear_threat(player, feet, forward);
        if let Some(what) = &threat {
            if self.flag_set || self.rear_frames == 0 {
                glog!("DEFLECT", "protection off: {what}");
            }
            self.rear_frames += 1;
        }
        let protect = threat.is_none();
        player.chr_ins.chr_flags1c5.set_is_invincible(protect);
        self.flag_set = protect;
    }

    /// Clears our invincible flag (menus, F9, death, not deflecting).
    pub fn release(&mut self, player: &mut PlayerIns) {
        if self.flag_set {
            player.chr_ins.chr_flags1c5.set_is_invincible(false);
            self.flag_set = false;
        }
    }
}

/// Never leave the player invincible after a crash of our per-frame code.
pub fn emergency_clear() {
    if let Some(player) = unsafe { WorldChrMan::instance_mut() }.ok().and_then(|w| w.main_player.as_mut()) {
        player.chr_ins.chr_flags1c5.set_is_invincible(false);
    }
}

/// Something that can hit us from behind right now, described for the log.
fn rear_threat(player: &PlayerIns, feet: Vec3, forward: Vec3) -> Option<String> {
    let behind = |p: Vec3| {
        let d = Vec3::new(p.x - feet.x, 0.0, p.z - feet.z);
        d.length() <= REAR_RANGE && d.dot(forward) < 0.0
    };
    for t in crate::targets::living(player) {
        if behind(t.feet) {
            return Some(format!("enemy {:#x} behind us within {REAR_RANGE} m", t.chr.as_ptr() as usize));
        }
    }
    let me = player.chr_ins.field_ins_handle;
    if let Ok(manager) = unsafe { CSBulletManager::instance() } {
        for bullet in manager.bullets() {
            if bullet.targeting_owner.owner_chr_handle == me {
                continue;
            }
            let p = &bullet.physics.position;
            let v = &bullet.physics.velocity;
            let (pos, vel) = (Vec3::new(p.0, p.1, p.2), Vec3::new(v.0, v.1, v.2));
            if behind(pos) && vel.dot(feet - pos) > 0.0 {
                return Some("enemy projectile closing in from behind".to_string());
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    const DT: f32 = 1.0 / 60.0;

    #[test]
    fn lasts_two_seconds_then_cools_down() {
        let mut d = Deflect::default();
        assert!(d.tick(DT, true, true));
        for _ in 0..115 {
            assert!(d.tick(DT, false, true));
        }
        for _ in 0..10 {
            d.tick(DT, false, true);
        }
        assert!(!d.active());
        assert!(!d.tick(DT, true, true), "on cooldown");
        for _ in 0..(8 * 60) {
            d.tick(DT, false, true);
        }
        assert!(d.tick(DT, true, true), "ready again after 8 s");
    }

    #[test]
    fn e_again_ends_it_early() {
        let mut d = Deflect::default();
        d.tick(DT, true, true);
        d.tick(DT, false, true);
        assert!(!d.tick(DT, true, true));
        assert!(d.cooldown > 7.9);
    }

    #[test]
    fn letting_go_ends_it() {
        let mut d = Deflect::default();
        d.tick(DT, true, true);
        assert!(!d.tick(DT, false, false));
    }
}
