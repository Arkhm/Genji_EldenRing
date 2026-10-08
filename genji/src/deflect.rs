//! Deflect (step 8a, DESIGN.md 5f): E holds the sword up for DURATION seconds (E again ends it early), then COOLDOWN.
//! While it runs, the player can't be damaged from the front: the game's "invincible" character flag is set on every
//! frame where nothing threatens from behind (no living enemy within REAR_RANGE in our rear half, no enemy
//! projectile within REAR_RANGE flying the way we face, i.e. coming from behind). Sending projectiles back is step 8b.

use std::collections::{HashMap, HashSet};

use eldenring::cs::{AtkParam_Npc, AtkParam_Pc, Bullet as BulletParam, CSBulletManager, PlayerIns, SoloParamRepository, WorldChrMan};
use fromsoftware_shared::FromStatic;
use glam::Vec3;

use crate::glog;
use crate::look::Look;

/// Overwatch: 2 s, ended early by pressing E again.
const DURATION: f32 = 2.0;
/// Overwatch: 8 s (you confirmed, 2026-10-08). Was 1 s while we tested; back to 8 s after you validated Deflect.
const COOLDOWN: f32 = 8.0;
/// Enemies and enemy projectiles closer than this in our rear half can hit us: no protection on those frames.
const REAR_RANGE: f32 = 4.0;

/// 8b: enemy projectiles closer than this to our chest, flying at us from the front, are sent back, metres.
const REFLECT_RANGE: f32 = 3.0;
/// The returned projectile is the Bone Dart's rows (Bullet and AtkParam_Pc 10171000), changed in memory while Genji
/// mode is on: straight flight where we aim, at the original's speed, carrying the original's damage values.
const REFLECT_BULLET: u32 = 10_171_000;
const REFLECT_SPEED_MIN: f32 = 20.0;
const REFLECT_SPEED_MAX: f32 = 80.0;
const REFLECT_LIFE: f32 = 3.0;
const REFLECT_RANGE_MAX: f32 = 80.0;

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
    /// Enemy projectiles already sent back (by handle).
    reflected: HashSet<u64>,
    reflected_total: u32,
    /// The Bone Dart rows' vanilla values while ours are applied.
    dart_vanilla: Option<DartValues>,
    /// Caught projectile kinds (Bullet rows) pointed at our damage row while deflecting: row -> its own damage row.
    retargeted: HashMap<u32, i32>,
}

#[derive(Clone, Copy)]
struct DartValues {
    sfx_flick: i32,
    sfx_delete_by_hit: i8,
    sfx_delete_by_life_end: i8,
    init_velocity: f32,
    max_velocity: f32,
    min_velocity: f32,
    accel_out_range: f32,
    gravity_in_range: f32,
    gravity_out_range: f32,
    life: f32,
    dist: f32,
    homing_angle: i16,
    atk_row: u32,
    atk: [u16; 5],
    add_base_atk: bool,
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

    /// For the HUD: seconds of Deflect left (while up) and of cooldown.
    pub fn hud(&self) -> (Option<f32>, f32) {
        (self.left, self.cooldown)
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

    /// 8b, every frame while deflecting: enemy projectiles about to hit us from the front are removed and sent back
    /// along our aim with their own damage values.
    pub fn reflect(&mut self, player: &PlayerIns, look: &Look, eye: Vec3) {
        if !self.active() {
            self.reflected.clear();
            self.restore_kinds();
            return;
        }
        let Some(dart) = self.dart_vanilla else {
            return;
        };
        let me = player.chr_ins.field_ins_handle;
        let chest = eye - Vec3::Y * 0.6;
        let forward = look.flat_forward();
        let mut caught: Vec<(i32, f32)> = Vec::new();
        let Ok(manager) = (unsafe { CSBulletManager::instance_mut() }) else {
            return;
        };
        for bullet in manager.bullets_mut() {
            if bullet.targeting_owner.owner_chr_handle == me {
                continue;
            }
            let handle = unsafe { std::ptr::read(&bullet.field_ins_handle as *const _ as *const u64) };
            if self.reflected.contains(&handle) {
                continue;
            }
            let p = &bullet.physics.position;
            let v = &bullet.physics.velocity;
            let (pos, vel) = (Vec3::new(p.0, p.1, p.2), Vec3::new(v.0, v.1, v.2));
            let flat_vel = Vec3::new(vel.x, 0.0, vel.z);
            let closing = vel.dot(chest - pos) > 0.0;
            if (pos - chest).length() > REFLECT_RANGE || !closing || flat_vel.dot(forward) > 0.0 {
                continue;
            }
            let param_id = [bullet.fly_state.base.param.param_id, bullet.wait_state.base.param.param_id, bullet.exp_state.base.param.param_id]
                .into_iter()
                .find(|id| *id > 0)
                .unwrap_or(-1);
            // Remove it: no life left, no speed, and out of the way.
            let life_ptr = &mut bullet.time_alive as *mut f32;
            unsafe { life_ptr.add(1).write(0.0) };
            bullet.physics.velocity.0 = 0.0;
            bullet.physics.velocity.1 = 0.0;
            bullet.physics.velocity.2 = 0.0;
            bullet.physics.position.1 -= 50.0;
            self.reflected.insert(handle);
            caught.push((param_id, vel.length()));
        }
        for (param_id, speed) in caught {
            self.send_back(player, look, eye, &dart, param_id, speed);
        }
    }

    /// Fires the Bone Dart along our aim with the caught projectile's damage values.
    fn send_back(&mut self, player: &PlayerIns, look: &Look, eye: Vec3, dart: &DartValues, param_id: i32, speed: f32) {
        let mut atk = [0u16; 5];
        let mut source = "not found";
        if let Ok(params) = unsafe { SoloParamRepository::instance_mut() } {
            let atk_id = params.get::<BulletParam>(param_id.max(0) as u32).map(|b| b.atk_id_bullet()).unwrap_or(-1);
            if atk_id >= 0 {
                if let Some(a) = params.get::<AtkParam_Npc>(atk_id as u32) {
                    atk = [a.atk_phys(), a.atk_mag(), a.atk_fire(), a.atk_thun(), a.atk_dark()];
                    source = "AtkParam_Npc";
                } else if let Some(a) = params.get::<AtkParam_Pc>(atk_id as u32) {
                    atk = [a.atk_phys(), a.atk_mag(), a.atk_fire(), a.atk_thun(), a.atk_dark()];
                    source = "AtkParam_Pc";
                }
            }
            let speed = speed.clamp(REFLECT_SPEED_MIN, REFLECT_SPEED_MAX);
            if let Some(b) = params.get_mut::<BulletParam>(REFLECT_BULLET) {
                b.set_init_vellocity(speed);
                b.set_max_vellocity(speed);
                b.set_min_vellocity(speed);
            }
            if let Some(a) = params.get_mut::<AtkParam_Pc>(dart.atk_row) {
                a.set_atk_phys(atk[0]);
                a.set_atk_mag(atk[1]);
                a.set_atk_fire(atk[2]);
                a.set_atk_thun(atk[3]);
                a.set_atk_dark(atk[4]);
            }
        }
        let forward = look.forward();
        let aim_distance = crate::shuriken::aim_distance(player, eye, forward);
        let aim = eye + forward * aim_distance;
        let from = eye + forward * 0.6 + look.right() * 0.08 - look.up() * 0.06;
        let dir = (aim - from).normalize_or(forward);
        let right = Vec3::Y.cross(dir).normalize_or(look.right());
        let up = dir.cross(right).normalize_or(Vec3::Y);
        // Send it back as itself (your choice, 2026-10-08): the same Bullet row, fired by us. An enemy projectile's
        // damage row is an NPC one (AtkParam_Npc); fired by us the game would look it up among the player's rows, so
        // while deflecting that kind is pointed at our damage row (the Bone Dart's AtkParam_Pc, carrying the copied
        // values above). If the game refuses it, the Bone Dart goes instead.
        let mut sent_as = "itself";
        let mut result = Err(3);
        if param_id > 0 {
            let row = param_id as u32;
            if let Ok(params) = unsafe { SoloParamRepository::instance_mut() } {
                if let Some(b) = params.get_mut::<BulletParam>(row) {
                    if !self.retargeted.contains_key(&row) {
                        self.retargeted.insert(row, b.atk_id_bullet());
                    }
                    b.set_atk_id_bullet(dart.atk_row as i32);
                }
            }
            result = crate::shuriken::spawn(player, param_id, from, right, up, dir);
        }
        if result.is_err() {
            sent_as = "Bone Dart (the game refused the original kind)";
            result = crate::shuriken::spawn(player, REFLECT_BULLET as i32, from, right, up, dir);
        }
        // Kill credit: the character under the crosshair, if any (approximation: the dart's own hit isn't tracked).
        let mut credited = None;
        for t in crate::targets::living(player) {
            if crate::shuriken::ray_cylinder(eye, forward, t.feet, t.radius, t.height).is_some_and(|d| d <= aim_distance + 0.5) {
                crate::kills::damaged(t.chr.as_ptr() as usize);
                credited = Some(t.chr.as_ptr() as usize);
                break;
            }
        }
        self.reflected_total += 1;
        glog!(
            "DEFLECT",
            "sent back projectile {param_id} as {sent_as} (#{} total), dart speed {speed:.0} m/s, damage phys/mag/fire/lightning/holy {:?} (from {source}), aim {aim_distance:.1} m{}: {}",
            self.reflected_total,
            atk,
            credited.map(|c| format!(", at character {c:#x}")).unwrap_or_default(),
            match result {
                Ok(()) => "ok".to_string(),
                Err(code) => format!("FAILED, engine error {code}"),
            }
        );
    }

    /// Points the caught kinds back at their own damage rows (when Deflect ends or Genji mode lets go).
    fn restore_kinds(&mut self) {
        if self.retargeted.is_empty() {
            return;
        }
        if let Ok(params) = unsafe { SoloParamRepository::instance_mut() } {
            for (row, atk) in &self.retargeted {
                if let Some(b) = params.get_mut::<BulletParam>(*row) {
                    b.set_atk_id_bullet(*atk);
                }
            }
        }
        glog!("DEFLECT", "{} projectile kinds pointed back at their own damage rows", self.retargeted.len());
        self.retargeted.clear();
    }

    /// Applies (true) or restores (false) the returned projectile's values on the Bone Dart rows, in memory.
    pub fn set_values(&mut self, on: bool) {
        if on == self.dart_vanilla.is_some() {
            return;
        }
        let Ok(params) = (unsafe { SoloParamRepository::instance_mut() }) else {
            return;
        };
        match self.dart_vanilla.take() {
            None => {
                let Some(b) = params.get::<BulletParam>(REFLECT_BULLET) else {
                    glog!("DEFLECT", "Bone Dart Bullet row {REFLECT_BULLET} not found; projectiles are blocked but not sent back");
                    return;
                };
                let atk_row = b.atk_id_bullet() as u32;
                let mut v = DartValues {
                    sfx_flick: b.sfx_id_flick(),
                    sfx_delete_by_hit: b.bullet_sfx_delete_type_by_hit(),
                    sfx_delete_by_life_end: b.bullet_sfx_delete_type_by_life_dead(),
                    init_velocity: b.init_vellocity(),
                    max_velocity: b.max_vellocity(),
                    min_velocity: b.min_vellocity(),
                    accel_out_range: b.accel_out_range(),
                    gravity_in_range: b.gravity_in_range(),
                    gravity_out_range: b.gravity_out_range(),
                    life: b.life(),
                    dist: b.dist(),
                    homing_angle: b.homing_angle(),
                    atk_row,
                    atk: [0; 5],
                    add_base_atk: false,
                };
                if let Some(a) = params.get::<AtkParam_Pc>(atk_row) {
                    v.atk = [a.atk_phys(), a.atk_mag(), a.atk_fire(), a.atk_thun(), a.atk_dark()];
                    v.add_base_atk = a.is_add_base_atk();
                }
                if let Some(b) = params.get_mut::<BulletParam>(REFLECT_BULLET) {
                    b.set_sfx_id_flick(-1); // the bounce effect hangs in the air without gravity (step 6 tests)
                    b.set_bullet_sfx_delete_type_by_hit(1);
                    b.set_bullet_sfx_delete_type_by_life_dead(1);
                    b.set_accel_out_range(0.0);
                    b.set_gravity_in_range(0.0);
                    b.set_gravity_out_range(20.0);
                    b.set_life(REFLECT_LIFE);
                    b.set_dist(REFLECT_RANGE_MAX);
                    b.set_homing_angle(0);
                }
                if let Some(a) = params.get_mut::<AtkParam_Pc>(atk_row) {
                    a.set_is_add_base_atk(true);
                }
                glog!("DEFLECT", "send-back values ON on the Bone Dart rows (bullet {REFLECT_BULLET}, attack {atk_row})");
                self.dart_vanilla = Some(v);
            }
            Some(v) => {
                for (row, atk) in self.retargeted.drain() {
                    if let Some(b) = params.get_mut::<BulletParam>(row) {
                        b.set_atk_id_bullet(atk);
                    }
                }
                if let Some(b) = params.get_mut::<BulletParam>(REFLECT_BULLET) {
                    b.set_sfx_id_flick(v.sfx_flick);
                    b.set_bullet_sfx_delete_type_by_hit(v.sfx_delete_by_hit);
                    b.set_bullet_sfx_delete_type_by_life_dead(v.sfx_delete_by_life_end);
                    b.set_init_vellocity(v.init_velocity);
                    b.set_max_vellocity(v.max_velocity);
                    b.set_min_vellocity(v.min_velocity);
                    b.set_accel_out_range(v.accel_out_range);
                    b.set_gravity_in_range(v.gravity_in_range);
                    b.set_gravity_out_range(v.gravity_out_range);
                    b.set_life(v.life);
                    b.set_dist(v.dist);
                    b.set_homing_angle(v.homing_angle);
                }
                if let Some(a) = params.get_mut::<AtkParam_Pc>(v.atk_row) {
                    a.set_atk_phys(v.atk[0]);
                    a.set_atk_mag(v.atk[1]);
                    a.set_atk_fire(v.atk[2]);
                    a.set_atk_thun(v.atk[3]);
                    a.set_atk_dark(v.atk[4]);
                    a.set_is_add_base_atk(v.add_base_atk);
                }
                glog!("DEFLECT", "send-back values OFF: vanilla Bone Dart restored");
            }
        }
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
            // Judged by where it comes from (its flight direction), not where it is: test of 2026-10-08, arrows
            // from the front were level with our body (or a hair past its centre) on the last frame before the hit
            // and counted as "behind", so every arrow hit got through. Coming from behind = flying the way we face.
            let near = Vec3::new(pos.x - feet.x, 0.0, pos.z - feet.z).length() <= REAR_RANGE;
            let flat_vel = Vec3::new(vel.x, 0.0, vel.z);
            if near && vel.dot(feet - pos) > 0.0 && flat_vel.dot(forward) > 0.0 {
                return Some("enemy projectile coming from behind".to_string());
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
        for _ in 0..((COOLDOWN * 60.0) as u32 + 1) {
            d.tick(DT, false, true);
        }
        assert!(d.tick(DT, true, true), "ready again after the cooldown");
    }

    #[test]
    fn e_again_ends_it_early() {
        let mut d = Deflect::default();
        d.tick(DT, true, true);
        d.tick(DT, false, true);
        assert!(!d.tick(DT, true, true));
        assert!(d.cooldown > COOLDOWN - 0.1);
    }

    #[test]
    fn letting_go_ends_it() {
        let mut d = Deflect::default();
        d.tick(DT, true, true);
        assert!(!d.tick(DT, false, false));
    }
}
