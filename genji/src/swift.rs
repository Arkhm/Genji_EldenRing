//! Swift Strike (step 7): cooldown, kill reset and damage along the dash. The dash movement itself is in
//! `movement.rs` (DASH_*). Values from memory of Overwatch, **not verified** (DESIGN.md, 5d).

use std::collections::{HashMap, HashSet};

use eldenring::cs::{AtkParam_Pc, Bullet as BulletParam, ChrInsExt, NpcParam, PlayerIns, SoloParamRepository, SpEffectParam};
use fromsoftware_shared::FromStatic;
use glam::Vec3;

use crate::glog;
use crate::targets::{self, Target};

/// Damage to each character the dash passes through (Overwatch), before dexterity scaling.
const DAMAGE: i32 = 50;
/// The dash's hit is a real game hit (your choice, 2026-10-08; DESIGN.md 5e): an invisible projectile of the Kukri's
/// rows (Bullet and AtkParam_Pc 10173000, status effect 3115 for its bleed) fired at each character the dash
/// touches, from behind it (shields face you), at point blank. Its values are changed in memory while Genji mode is
/// on and restored afterwards; a real Kukri thrown in plain Elden Ring is unchanged.
const KUKRI: u32 = 10_173_000;
const STRIKE_SPEED: f32 = 50.0;
const STRIKE_LIFE: f32 = 0.2;
const STRIKE_RANGE: f32 = 5.0;
/// Starts this far outside the target's collision radius, metres.
const STRIKE_GAP: f32 = 0.3;
/// Attack power = 50 x dexterity multiplier x this. 1.0 keeps the dash near the damage you called perfect (76 at
/// dexterity 25, when it lowered HP directly).
const DASH_ATTACK_FACTOR: f32 = 1.0;
/// Every dash hit makes the target bleed (your choice, 2026-10-08, replacing half a bar): the buildup is far above any
/// bleed bar. Half a bar didn't proc on guards: two hits 4+ s apart, and the buildup fades in between. A target with
/// bleed resistance 999 or more (NpcParam `resistBlood`) is immune: no buildup.
const BLEED_PROC: i32 = 9999;
const BLEED_IMMUNE: u16 = 999;
/// If the game registered no hit within this many frames, the dash's damage is applied directly instead.
const CONFIRM_FRAMES: u32 = 10;
/// An elimination this many seconds before a dash starts also resets it. Test of 2026-10-08: Fan of Blades, then
/// Shift right away: the fan's kill was counted a moment before the dash started (the dash was still ready, so
/// nothing to reset), and the dash then went on cooldown although the kill showed during it.
const KILL_GRACE: f32 = 0.25;
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
    /// The Kukri rows' vanilla values while ours are applied.
    kukri_vanilla: Option<KukriValues>,
    /// Strikes waiting for the game's hit.
    pending: Vec<Strike>,
    /// Seconds since an elimination that happened while the dash was ready (for KILL_GRACE).
    ready_kill_age: Option<f32>,
}

#[derive(Clone, Copy)]
struct KukriValues {
    sfx_bullet: i32,
    sfx_flick: i32,
    init_velocity: f32,
    max_velocity: f32,
    min_velocity: f32,
    accel_out_range: f32,
    gravity_in_range: f32,
    gravity_out_range: f32,
    life: f32,
    dist: f32,
    atk_row: u32,
    atk_phys: u16,
    add_base_atk: bool,
    sp_effect: i32,
    blood: i32,
}

/// A strike fired at a character, waiting for the game's hit.
struct Strike {
    chr: usize,
    hp_before: i32,
    own_before: i64,
    frames: u32,
    damage: i32,
    attack: u16,
    bleed: i32,
}

impl Swift {
    /// Once per frame: cooldown, and kill reset (`eliminations` from `kills::tick`).
    pub fn tick(&mut self, dt: f32, eliminations: u32) {
        self.cooldown = (self.cooldown - dt).max(0.0);
        self.ready_kill_age = self.ready_kill_age.map(|a| a + dt).filter(|a| *a <= KILL_GRACE);
        if eliminations > 0 {
            if self.cooldown > 0.0 {
                glog!("SWIFT", "reset by an elimination ({:.1} s of cooldown left)", self.cooldown);
                self.cooldown = 0.0;
            } else {
                self.ready_kill_age = Some(0.0);
            }
        }
    }

    /// The full cooldown, seconds (HUD).
    pub fn cooldown_max(&self) -> f32 {
        COOLDOWN
    }

    /// Seconds of cooldown left (HUD).
    pub fn cooldown(&self) -> f32 {
        self.cooldown
    }

    pub fn ready(&self) -> bool {
        self.cooldown <= 0.0
    }

    /// Dashes started so far (the HUD plays its "pressed" animation when this changes).
    pub fn uses(&self) -> u32 {
        self.dashes
    }

    /// Dragonblade's activation resets Swift Strike (Overwatch).
    pub fn reset(&mut self, why: &str) {
        if self.cooldown > 0.0 {
            glog!("SWIFT", "reset by {why} ({:.1} s of cooldown left)", self.cooldown);
            self.cooldown = 0.0;
        }
    }

    /// A dash started (the movement controller accepted it).
    pub fn started(&mut self) {
        self.cooldown = COOLDOWN;
        self.hit.clear();
        self.near.clear();
        self.dashes += 1;
        glog!("SWIFT", "dash #{} started, cooldown {COOLDOWN} s", self.dashes);
        if let Some(age) = self.ready_kill_age.take() {
            self.cooldown = 0.0;
            glog!("SWIFT", "reset by an elimination {age:.2} s before this dash");
        }
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
            self.strike(player, &target, to);
        }
    }

    /// Applies (true) or restores (false) the strike values on the Kukri's rows, in memory.
    pub fn set_values(&mut self, on: bool) {
        if on == self.kukri_vanilla.is_some() {
            return;
        }
        let Ok(params) = (unsafe { SoloParamRepository::instance_mut() }) else {
            return;
        };
        match self.kukri_vanilla.take() {
            None => {
                let Some(bullet) = params.get::<BulletParam>(KUKRI) else {
                    glog!("DASH", "Kukri Bullet row {KUKRI} not found; dash hits lower HP directly");
                    return;
                };
                let (atk_row, sp_effect) = (bullet.atk_id_bullet() as u32, bullet.sp_effect_id0());
                let mut v = KukriValues {
                    sfx_bullet: bullet.sfx_id_bullet(),
                    sfx_flick: bullet.sfx_id_flick(),
                    init_velocity: bullet.init_vellocity(),
                    max_velocity: bullet.max_vellocity(),
                    min_velocity: bullet.min_vellocity(),
                    accel_out_range: bullet.accel_out_range(),
                    gravity_in_range: bullet.gravity_in_range(),
                    gravity_out_range: bullet.gravity_out_range(),
                    life: bullet.life(),
                    dist: bullet.dist(),
                    atk_row,
                    atk_phys: 0,
                    add_base_atk: false,
                    sp_effect,
                    blood: 0,
                };
                if let Some(atk) = params.get::<AtkParam_Pc>(atk_row) {
                    (v.atk_phys, v.add_base_atk) = (atk.atk_phys(), atk.is_add_base_atk());
                }
                if let Some(sp) = params.get::<SpEffectParam>(sp_effect as u32) {
                    v.blood = sp.blood_attack_power();
                }
                if let Some(bullet) = params.get_mut::<BulletParam>(KUKRI) {
                    bullet.set_sfx_id_bullet(-1); // invisible
                    bullet.set_sfx_id_flick(-1);
                    bullet.set_init_vellocity(STRIKE_SPEED);
                    bullet.set_max_vellocity(STRIKE_SPEED);
                    bullet.set_min_vellocity(STRIKE_SPEED);
                    bullet.set_accel_out_range(0.0);
                    bullet.set_gravity_in_range(0.0);
                    bullet.set_gravity_out_range(0.0);
                    bullet.set_life(STRIKE_LIFE);
                    bullet.set_dist(STRIKE_RANGE);
                }
                if let Some(atk) = params.get_mut::<AtkParam_Pc>(atk_row) {
                    atk.set_is_add_base_atk(true);
                }
                glog!(
                    "DASH",
                    "strike values ON on the Kukri rows (bullet {KUKRI}, attack {atk_row}, status effect {sp_effect}): vanilla attack {}, vanilla bleed buildup {}",
                    v.atk_phys,
                    v.blood
                );
                self.kukri_vanilla = Some(v);
            }
            Some(v) => {
                if let Some(bullet) = params.get_mut::<BulletParam>(KUKRI) {
                    bullet.set_sfx_id_bullet(v.sfx_bullet);
                    bullet.set_sfx_id_flick(v.sfx_flick);
                    bullet.set_init_vellocity(v.init_velocity);
                    bullet.set_max_vellocity(v.max_velocity);
                    bullet.set_min_vellocity(v.min_velocity);
                    bullet.set_accel_out_range(v.accel_out_range);
                    bullet.set_gravity_in_range(v.gravity_in_range);
                    bullet.set_gravity_out_range(v.gravity_out_range);
                    bullet.set_life(v.life);
                    bullet.set_dist(v.dist);
                }
                if let Some(atk) = params.get_mut::<AtkParam_Pc>(v.atk_row) {
                    atk.set_atk_phys(v.atk_phys);
                    atk.set_is_add_base_atk(v.add_base_atk);
                }
                if let Some(sp) = params.get_mut::<SpEffectParam>(v.sp_effect as u32) {
                    sp.set_blood_attack_power(v.blood);
                }
                glog!("DASH", "strike values OFF: vanilla Kukri restored");
            }
        }
    }

    /// Fires the strike at one character: dexterity-scaled attack power and half of its bleed bar.
    fn strike(&mut self, player: &PlayerIns, target: &Target, our_feet: Vec3) {
        let id = target.chr.as_ptr() as usize;
        let damage = crate::scaling::scaled(DAMAGE as f32, player);
        let attack = (DAMAGE as f32 * crate::scaling::dex_multiplier(player) * DASH_ATTACK_FACTOR).round() as u16;
        let chr = unsafe { target.chr.as_ref() };
        let (hp_before, own_before) = (chr.modules.data.hp, targets::own_damage(id));
        crate::kills::damaged(id);
        let Some(v) = self.kukri_vanilla else {
            self.direct(target, damage, "strike values not applied");
            return;
        };
        // Half of this character's bleed bar.
        let mut resist = 0u16;
        let mut bleed = 0;
        if let Ok(params) = unsafe { SoloParamRepository::instance_mut() } {
            if let Some(npc) = params.get::<NpcParam>(chr.npc_param_id as u32) {
                resist = npc.resist_blood();
                if resist > 0 && resist < BLEED_IMMUNE {
                    bleed = BLEED_PROC;
                }
            }
            if let Some(sp) = params.get_mut::<SpEffectParam>(v.sp_effect as u32) {
                sp.set_blood_attack_power(bleed);
            }
            if let Some(atk) = params.get_mut::<AtkParam_Pc>(v.atk_row) {
                atk.set_atk_phys(attack);
            }
        }
        // From behind the character's own facing (a shield is held in front; test of 2026-10-08: placing it behind
        // as seen from us was nearly random, since we are inside or next to it at that moment), at chest height.
        let center = target.feet + Vec3::Y * (target.height * 0.5);
        let facing = glam::Quat::from(chr.modules.physics.orientation) * Vec3::NEG_Z;
        let mut away = Vec3::new(-facing.x, 0.0, -facing.z);
        if away.length_squared() < 1e-4 {
            away = Vec3::new(center.x - our_feet.x, 0.0, center.z - our_feet.z);
        }
        let away = away.normalize_or(Vec3::X);
        let from = center + away * (target.radius + STRIKE_GAP);
        let forward = -away;
        let right = Vec3::Y.cross(forward).normalize_or(Vec3::X);
        let up = forward.cross(right).normalize_or(Vec3::Y);
        match crate::shuriken::spawn(player, KUKRI as i32, from, right, up, forward) {
            Ok(()) => {
                glog!(
                    "DASH",
                    "strike on character {id:#x}: attack power {attack}, bleed buildup {bleed} (its bleed resistance {resist}{})",
                    if resist >= BLEED_IMMUNE { ", immune" } else { "" }
                );
                self.pending.push(Strike { chr: id, hp_before, own_before, frames: 0, damage, attack, bleed });
            }
            Err(code) => self.direct(target, damage, &format!("spawn failed, engine error {code}")),
        }
    }

    /// Puts the bleed buildup on the character directly: the Kukri's status effect (3115) with our buildup, applied
    /// to it. Test of 2026-10-08: the invisible Kukri hit landed but no bleed bar ever appeared, so the projectile
    /// doesn't pass its effect on when spawned this way.
    fn bleed(&self, target: &Target, amount: i32) {
        let Some(v) = self.kukri_vanilla else {
            return;
        };
        if amount <= 0 || unsafe { target.chr.as_ref() }.modules.data.hp <= 0 {
            return;
        }
        if let Ok(params) = unsafe { SoloParamRepository::instance_mut() } {
            if let Some(sp) = params.get_mut::<SpEffectParam>(v.sp_effect as u32) {
                sp.set_blood_attack_power(amount);
            }
        }
        unsafe { &mut *target.chr.as_ptr() }.apply_speffect(v.sp_effect, true);
        glog!("DASH", "bleed {amount} applied to character {:#x} (status effect {})", target.chr.as_ptr() as usize, v.sp_effect);
    }

    /// The dash's damage applied by lowering HP (when the game's hit isn't available).
    fn direct(&self, target: &Target, damage: i32, why: &str) {
        let id = target.chr.as_ptr() as usize;
        if let Some((before, after, max)) = targets::damage(target, damage) {
            glog!("DASH", "HIT {damage} directly on character {id:#x} ({why}): hp {before} -> {after} / {max}");
        }
    }

    /// Once per frame: strikes the game hit are logged; ones it didn't hit within CONFIRM_FRAMES get the damage
    /// directly.
    pub fn confirm(&mut self, player: &PlayerIns) {
        if self.pending.is_empty() {
            return;
        }
        let everyone = targets::all(player);
        let mut still = Vec::new();
        for mut s in std::mem::take(&mut self.pending) {
            s.frames += 1;
            let Some(target) = everyone.iter().find(|t| t.chr.as_ptr() as usize == s.chr) else {
                continue; // unloaded
            };
            let data = &unsafe { target.chr.as_ref() }.modules.data;
            let game = (s.hp_before - data.hp) as i64 - (targets::own_damage(s.chr) - s.own_before);
            if game > 0 {
                glog!(
                    "DASH",
                    "HIT on character {:#x}: game dealt {game} (attack power {}), hp now {} / {} (after {} frames)",
                    s.chr, s.attack, data.hp, data.max_hp, s.frames
                );
                self.bleed(target, s.bleed);
                crate::ult::add(DAMAGE as f32, "Swift Strike");
            } else if data.hp <= 0 {
                continue; // died meanwhile (another hit)
            } else if s.frames >= CONFIRM_FRAMES {
                self.direct(target, s.damage, &format!("the game registered no strike hit in {CONFIRM_FRAMES} frames"));
                self.bleed(target, s.bleed);
                crate::ult::add(DAMAGE as f32, "Swift Strike");
            } else {
                still.push(s);
            }
        }
        self.pending = still;
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

    #[test]
    fn kill_just_before_a_dash_resets_it() {
        let mut s = Swift::default();
        s.tick(0.016, 1); // kill while ready
        s.tick(0.1, 0);
        s.started();
        assert!(s.ready(), "kill 0.1 s before the dash");
        s.tick(0.016, 1);
        s.tick(0.3, 0);
        s.started();
        assert!(!s.ready(), "kill 0.3 s before the dash is too early");
    }
}
