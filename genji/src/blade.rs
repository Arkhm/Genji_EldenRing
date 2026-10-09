//! Dragonblade and quick melee (step 9, DESIGN.md 5g).
//!
//! Hits work like Swift Strike's: an invisible point-blank projectile fired at each target from behind its own
//! facing, so the game registers a real hit (number, sound, reaction); damage applied directly if the game registers
//! nothing within CONFIRM_FRAMES. Here the carrier is the Ruin Fragment's Bullet row pointed at **Corpse Piler's**
//! damage row (Rivers of Blood's Ash of War), your choice. Each swing also fires the Blood Blade crescent with its
//! damage removed, as the visible sweep until Genji's own model and animations exist.
//! All rows are changed in memory while Genji mode is on and restored afterwards.

use std::collections::HashMap;

use eldenring::cs::{AtkParam_Pc, Bullet as BulletParam, ChrInsExt, NpcParam, PlayerIns, SoloParamRepository, SpEffectParam};
use fromsoftware_shared::FromStatic;
use glam::Vec3;

use crate::glog;
use crate::look::Look;
use crate::targets::{self, Target};

/// Overwatch, Season 5 (2026-10-06, passed on by you): 6 s; 100 per enemy per swing; a swing about every 0.61 s
/// (derived from ~0.70 s and +15% swing speed, not verified); 5 m forward cone (angle unknown, 60 degrees each side).
const DURATION: f32 = 6.0;
/// Unsheathing before the 6 s start: no swings, the duration doesn't drain (you, 2026-10-08: about 1 s).
const UNSHEATHE: f32 = 1.0;
const SWING_DAMAGE: f32 = 100.0;
const SWING_INTERVAL: f32 = 0.61;
const SWING_REACH: f32 = 5.0;
/// Degrees each side of where we look. Was 60; you found it too narrow (2026-10-08): now the whole front half.
const SWING_HALF_ANGLE: f32 = 90.0;
/// Each swing hit fills this share of the target's bleed bar (a bleed every third swing).
const SWING_BLEED_SHARE: f32 = 0.34;
/// Share of each swing's damage that heals you (your request, 2026-10-08: 30 % lifesteal on every slash).
const LIFESTEAL: f32 = 0.3;
/// The ending roar starts this long before Dragonblade ends (its file lasts 2.9 s; you: "it starts when the blade is
/// about to end").
const END_ROAR_BEFORE: f32 = 2.9;
/// "My ultimate is charging" at most this often when Q is pressed early, seconds.
const CHARGING_LINE_EVERY: f32 = 4.0;
/// Overwatch quick melee: 40. Reach, cone and recovery are not verified.
const MELEE_DAMAGE: f32 = 40.0;
const MELEE_REACH: f32 = 2.5;
const MELEE_HALF_ANGLE: f32 = 45.0;
const MELEE_RECOVERY: f32 = 0.8;
/// Quick melee breaks guards and stances (your request, 2026-10-08: two melee hits on a shield should break the
/// enemy's stance). Unlike swings, melee can be blocked: it hits from our side, on the shield. The first melee on a
/// character does MELEE_STAMINA stamina and MELEE_POISE poise damage; another within MELEE_COMBO_WINDOW does
/// BREAK_VALUE of both, meant to break any guard or stance. Whether the game counts these values for the player's
/// attacks is logged (stamina and poise before and after each melee hit).
const MELEE_STAMINA: u16 = 60;
const MELEE_POISE: f32 = 40.0;
const BREAK_VALUE: u16 = 999;
const MELEE_COMBO_WINDOW: f32 = 5.0;

/// Invisible carrier of the hits: Ruin Fragment's Bullet row.
const CARRIER: u32 = 10_176_000;
/// Rivers of Blood [AOW] Corpse Piler, the damage row of each hit.
const CORPSE_PILER: u32 = 303_400_300;
/// Blood Blade [AOW] crescent, the visible sweep (damage row removed).
const SWEEP: u32 = 2635;
const SWEEP_SPEED: f32 = 20.0;
const SWEEP_LIFE: f32 = SWING_REACH / SWEEP_SPEED;
/// Bleed is applied directly with the Kukri's bleed status effect (as Swift Strike's).
const BLEED_EFFECT: u32 = 3115;
const BLEED_IMMUNE: u16 = 999;
const STRIKE_SPEED: f32 = 50.0;
const STRIKE_GAP: f32 = 0.3;
const CONFIRM_FRAMES: u32 = 10;

#[derive(Clone, Copy)]
struct Saved {
    carrier: [f32; 9],
    carrier_atk: i32,
    carrier_sfx: [i32; 3],
    carrier_homing: i16,
    piler_atk: [u16; 5],
    piler_add_base: bool,
    piler_disable_guard: bool,
    piler_stam: u16,
    piler_poise: f32,
    sweep_atk: i32,
    sweep_sp: i32,
    sweep_life: f32,
    sweep_dist: f32,
    sweep_speed: [f32; 3],
}

struct Pending {
    chr: usize,
    hp_before: i32,
    own_before: i64,
    frames: u32,
    damage: i32,
    attack: u16,
    bleed: i32,
    /// Overwatch ultimate-charge points this hit gives (swings give none).
    ult_points: f32,
    kind: &'static str,
    /// The target's stamina and poise before the hit (melee: guard and stance breaking are logged).
    stamina_before: i32,
    poise_before: f32,
}

/// How a strike lands.
#[derive(Clone, Copy)]
struct Style {
    /// Fired from our side (true: can be blocked, melee) or from behind the target's facing (false).
    from_front: bool,
    /// Shields count (melee) or are ignored (swings: AtkParam "Disable Guard").
    guardable: bool,
    stamina: u16,
    poise: f32,
}

#[derive(Default)]
pub struct Blade {
    /// Seconds of Dragonblade left.
    left: Option<f32>,
    /// Seconds of unsheathing left (before `left` starts draining).
    unsheathe: f32,
    swing_wait: f32,
    melee_wait: f32,
    swings: u32,
    uses: u32,
    saved: Option<Saved>,
    pending: Vec<Pending>,
    /// Characters hit by quick melee: seconds since.
    melee_hits: HashMap<usize, f32>,
    /// Lifesteal earned by swing hits, not applied yet (applied by the camera task, which can change the player).
    heal_owed: f32,
    /// The ending roar played for this Dragonblade.
    end_roar_played: bool,
    /// Seconds until "My ultimate is charging" may play again.
    charging_line_wait: f32,
}

impl Blade {
    pub fn active(&self) -> bool {
        self.left.is_some()
    }

    pub fn left(&self) -> Option<f32> {
        self.left
    }

    /// A quick melee is under way (its recovery): no shuriken then, and it cancels the rest of a burst (Overwatch,
    /// you 2026-10-09).
    pub fn meleeing(&self) -> bool {
        self.melee_wait > 0.0
    }

    /// Unsheathing (the first second of Dragonblade): no Swift Strike or Deflect then (you, 2026-10-08).
    pub fn unsheathing(&self) -> bool {
        self.left.is_some() && self.unsheathe > 0.0
    }

    /// Lifesteal earned since the last call, in hit points.
    pub fn take_heal(&mut self) -> i32 {
        let heal = self.heal_owed.floor();
        self.heal_owed -= heal;
        heal as i32
    }

    pub fn duration(&self) -> f32 {
        DURATION
    }

    /// Timers; call every frame.
    pub fn advance(&mut self, dt: f32) {
        self.swing_wait = (self.swing_wait - dt).max(0.0);
        self.melee_wait = (self.melee_wait - dt).max(0.0);
        self.charging_line_wait = (self.charging_line_wait - dt).max(0.0);
        for age in self.melee_hits.values_mut() {
            *age += dt;
        }
        self.melee_hits.retain(|_, age| *age <= MELEE_COMBO_WINDOW);
        if self.left.is_some() && self.unsheathe > 0.0 {
            self.unsheathe = (self.unsheathe - dt).max(0.0);
            if self.unsheathe <= 0.0 {
                glog!("BLADE", "unsheathed: {DURATION} s of Dragonblade start now");
            }
            return;
        }
        if let Some(left) = self.left {
            let left = left - dt;
            if left <= END_ROAR_BEFORE && !self.end_roar_played {
                self.end_roar_played = true;
                crate::sound::play("blade_end_roar");
            }
            if left <= 0.0 {
                self.left = None;
                crate::sound::play("blade_end");
                crate::sound::play_after("blade_sheathe", 0.25);
                glog!("BLADE", "Dragonblade #{} over after {} swings", self.uses, self.swings);
            } else {
                self.left = Some(left);
            }
        }
    }

    /// While our controller drives: Q (start when the ultimate is full), held left click (swings), mouse 5 (quick
    /// melee). Returns true if Dragonblade started this frame.
    pub fn act(&mut self, q_pressed: bool, swing_held: bool, melee_pressed: bool, player: &PlayerIns, look: &Look, eye: Vec3) -> bool {
        let mut started = false;
        if q_pressed && !self.active() {
            if crate::ult::spend() {
                self.left = Some(DURATION);
                self.unsheathe = UNSHEATHE;
                self.uses += 1;
                self.swings = 0;
                self.swing_wait = 0.0;
                self.end_roar_played = false;
                started = true;
                glog!("BLADE", "Dragonblade #{} start: unsheathing {UNSHEATHE} s, then {DURATION} s", self.uses);
            } else {
                glog!("BLADE", "ultimate not ready ({}%)", crate::ult::percent());
                if self.charging_line_wait <= 0.0 {
                    self.charging_line_wait = CHARGING_LINE_EVERY;
                    crate::sound::voice("voice_ult_charging", false);
                }
            }
        }
        if self.active() && self.unsheathe <= 0.0 && swing_held && self.swing_wait <= 0.0 {
            self.swing_wait = SWING_INTERVAL;
            self.swing(player, look, eye);
        }
        if !self.active() && melee_pressed && self.melee_wait <= 0.0 {
            self.melee_wait = MELEE_RECOVERY;
            self.melee(player, look, eye);
        }
        started
    }

    fn swing(&mut self, player: &PlayerIns, look: &Look, eye: Vec3) {
        self.swings += 1;
        let feet = eye - Vec3::Y * 1.6;
        let hit = in_cone(player, feet, look.flat_forward(), SWING_REACH, SWING_HALF_ANGLE);
        let damage = crate::scaling::scaled(SWING_DAMAGE, player);
        let attack = (SWING_DAMAGE * crate::scaling::dex_multiplier(player)).round() as u16;
        glog!("BLADE", "swing #{}: {} in reach", self.swings, hit.len());
        crate::sound::play("blade_swing");
        crate::sound::play_gain("blade_roar", 0.7);
        let style = Style { from_front: false, guardable: false, stamina: self.saved.map(|s| s.piler_stam).unwrap_or(0), poise: self.saved.map(|s| s.piler_poise).unwrap_or(0.0) };
        for target in &hit {
            let bleed = bleed_share(target, SWING_BLEED_SHARE);
            self.strike(player, target, feet, damage, attack, bleed, 0.0, "swing", style);
        }
        // The visible sweep.
        let forward = look.forward();
        let from = eye + forward * 0.5 - look.up() * 0.3;
        let right = Vec3::Y.cross(forward).normalize_or(look.right());
        let up = forward.cross(right).normalize_or(Vec3::Y);
        if let Err(code) = crate::shuriken::spawn(player, SWEEP as i32, from, right, up, forward) {
            glog!("BLADE", "sweep effect not spawned, engine error {code}");
        }
    }

    fn melee(&mut self, player: &PlayerIns, look: &Look, eye: Vec3) {
        let feet = eye - Vec3::Y * 1.6;
        crate::sound::play("melee_swing");
        crate::sound::voice("voice_melee", false);
        let mut hit = in_cone(player, feet, look.flat_forward(), MELEE_REACH, MELEE_HALF_ANGLE);
        hit.sort_by(|a, b| (a.feet - feet).length().total_cmp(&(b.feet - feet).length()));
        let Some(target) = hit.first() else {
            glog!("BLADE", "quick melee: nothing in reach");
            return;
        };
        let damage = crate::scaling::scaled(MELEE_DAMAGE, player);
        let attack = (MELEE_DAMAGE * crate::scaling::dex_multiplier(player)).round() as u16;
        let id = target.chr.as_ptr() as usize;
        let second = self.melee_hits.contains_key(&id);
        self.melee_hits.insert(id, 0.0);
        let style = if second {
            Style { from_front: true, guardable: true, stamina: BREAK_VALUE, poise: BREAK_VALUE as f32 }
        } else {
            Style { from_front: true, guardable: true, stamina: MELEE_STAMINA, poise: MELEE_POISE }
        };
        glog!("BLADE", "quick melee on character {id:#x}: {} hit within {MELEE_COMBO_WINDOW} s", if second { "second (break)" } else { "first" });
        self.strike(player, target, feet, damage, attack, 0, MELEE_DAMAGE, "quick melee", style);
    }

    #[allow(clippy::too_many_arguments)]
    fn strike(
        &mut self,
        player: &PlayerIns,
        target: &Target,
        our_feet: Vec3,
        damage: i32,
        attack: u16,
        bleed: i32,
        ult_points: f32,
        kind: &'static str,
        style: Style,
    ) {
        let id = target.chr.as_ptr() as usize;
        let chr = unsafe { target.chr.as_ref() };
        let (hp_before, own_before) = (chr.modules.data.hp, targets::own_damage(id));
        let (stamina_before, poise_before) = (chr.modules.data.stamina, chr.modules.super_armor.sa_durability);
        crate::kills::damaged(id);
        if self.saved.is_none() {
            direct(target, damage, kind, "values not applied");
            return;
        }
        if let Ok(params) = unsafe { SoloParamRepository::instance_mut() } {
            if let Some(atk) = params.get_mut::<AtkParam_Pc>(CORPSE_PILER) {
                atk.set_atk_phys(attack);
                atk.set_disable_guard(!style.guardable);
                atk.set_atk_stam(style.stamina);
                atk.set_atk_super_armor(style.poise);
            }
        }
        let center = target.feet + Vec3::Y * (target.height * 0.5);
        let toward_us = Vec3::new(our_feet.x - center.x, 0.0, our_feet.z - center.z).normalize_or(Vec3::X);
        let away = if style.from_front {
            toward_us
        } else {
            let facing = glam::Quat::from(chr.modules.physics.orientation) * Vec3::NEG_Z;
            Vec3::new(-facing.x, 0.0, -facing.z).normalize_or(-toward_us)
        };
        let from = center + away * (target.radius + STRIKE_GAP);
        let forward = -away;
        let right = Vec3::Y.cross(forward).normalize_or(Vec3::X);
        let up = forward.cross(right).normalize_or(Vec3::Y);
        match crate::shuriken::spawn(player, CARRIER as i32, from, right, up, forward) {
            Ok(()) => self.pending.push(Pending {
                chr: id,
                hp_before,
                own_before,
                frames: 0,
                damage,
                attack,
                bleed,
                ult_points,
                kind,
                stamina_before,
                poise_before,
            }),
            Err(code) => {
                direct(target, damage, kind, &format!("spawn failed, engine error {code}"));
                apply_bleed(target, bleed);
                if ult_points > 0.0 {
                    crate::ult::add(ult_points, kind);
                }
            }
        }
    }

    /// Once per frame: hits the game registered are logged (bleed applied); others get the damage directly.
    pub fn confirm(&mut self, player: &PlayerIns) {
        if self.pending.is_empty() {
            return;
        }
        let everyone = targets::all(player);
        let mut still = Vec::new();
        for mut p in std::mem::take(&mut self.pending) {
            p.frames += 1;
            let Some(target) = everyone.iter().find(|t| t.chr.as_ptr() as usize == p.chr) else {
                continue;
            };
            let modules = &unsafe { target.chr.as_ref() }.modules;
            let data = &modules.data;
            let game = (p.hp_before - data.hp) as i64 - (targets::own_damage(p.chr) - p.own_before);
            let (stamina, poise) = (data.stamina, modules.super_armor.sa_durability);
            // A melee hit fully blocked by a shield takes no HP but stamina: that counts as landed too.
            let landed = game > 0 || (p.kind == "quick melee" && (stamina < p.stamina_before || poise < p.poise_before));
            if p.kind == "quick melee" && (landed || p.frames >= CONFIRM_FRAMES) {
                glog!(
                    "BLADE",
                    "quick melee on {:#x}: stamina {} -> {stamina} / {}, poise {:.0} -> {poise:.0} / {:.0}",
                    p.chr, p.stamina_before, data.max_stamina, p.poise_before, modules.super_armor.sa_durability_max
                );
            }
            if landed {
                glog!(
                    "BLADE",
                    "{} HIT on character {:#x}: game dealt {game} (attack power {}, Corpse Piler row), hp now {} / {}",
                    p.kind, p.chr, p.attack, data.hp, data.max_hp
                );
                if p.kind == "swing" {
                    self.heal_owed += LIFESTEAL * game as f32;
                }
                // No Dragonblade hit sound was found in Overwatch's files: the melee hit stands in.
                crate::sound::play_gain("melee_hit", if p.kind == "swing" { 0.8 } else { 1.0 });
            } else if data.hp <= 0 {
                continue;
            } else if p.frames >= CONFIRM_FRAMES {
                direct(target, p.damage, p.kind, &format!("the game registered no hit in {CONFIRM_FRAMES} frames"));
                if p.kind == "swing" {
                    self.heal_owed += LIFESTEAL * p.damage as f32;
                }
            } else {
                still.push(p);
                continue;
            }
            apply_bleed(target, p.bleed);
            if p.ult_points > 0.0 {
                crate::ult::add(p.ult_points, p.kind);
            }
        }
        self.pending = still;
    }

    /// Applies (true) or restores (false) our values on the carrier, Corpse Piler and Blood Blade rows, in memory.
    pub fn set_values(&mut self, on: bool) {
        if on == self.saved.is_some() {
            return;
        }
        let Ok(params) = (unsafe { SoloParamRepository::instance_mut() }) else {
            return;
        };
        match self.saved.take() {
            None => {
                let (Some(c), Some(p), Some(s)) = (
                    params.get::<BulletParam>(CARRIER),
                    params.get::<AtkParam_Pc>(CORPSE_PILER),
                    params.get::<BulletParam>(SWEEP),
                ) else {
                    glog!("BLADE", "carrier, Corpse Piler or Blood Blade row not found; Dragonblade hits lower HP directly");
                    return;
                };
                let saved = Saved {
                    carrier: [
                        c.init_vellocity(), c.max_vellocity(), c.min_vellocity(), c.accel_out_range(), c.gravity_in_range(),
                        c.gravity_out_range(), c.life(), c.dist(), c.dmg_hit_record_life_time(),
                    ],
                    carrier_atk: c.atk_id_bullet(),
                    carrier_sfx: [c.sfx_id_bullet(), c.sfx_id_flick(), c.sfx_id_hit()],
                    carrier_homing: c.homing_angle(),
                    piler_atk: [p.atk_phys(), p.atk_mag(), p.atk_fire(), p.atk_thun(), p.atk_dark()],
                    piler_add_base: p.is_add_base_atk(),
                    piler_disable_guard: p.disable_guard(),
                    piler_stam: p.atk_stam(),
                    piler_poise: p.atk_super_armor(),
                    sweep_atk: s.atk_id_bullet(),
                    sweep_sp: s.sp_effect_id0(),
                    sweep_life: s.life(),
                    sweep_dist: s.dist(),
                    sweep_speed: [s.init_vellocity(), s.max_vellocity(), s.min_vellocity()],
                };
                if let Some(c) = params.get_mut::<BulletParam>(CARRIER) {
                    c.set_atk_id_bullet(CORPSE_PILER as i32);
                    c.set_sfx_id_bullet(-1);
                    c.set_sfx_id_flick(-1);
                    c.set_sfx_id_hit(-1);
                    c.set_init_vellocity(STRIKE_SPEED);
                    c.set_max_vellocity(STRIKE_SPEED);
                    c.set_min_vellocity(STRIKE_SPEED);
                    c.set_accel_out_range(0.0);
                    c.set_gravity_in_range(0.0);
                    c.set_gravity_out_range(0.0);
                    c.set_life(0.2);
                    c.set_dist(5.0);
                    c.set_dmg_hit_record_life_time(0.0);
                    c.set_homing_angle(0);
                }
                if let Some(p) = params.get_mut::<AtkParam_Pc>(CORPSE_PILER) {
                    p.set_is_add_base_atk(true);
                    p.set_atk_mag(0);
                    p.set_atk_fire(0);
                    p.set_atk_thun(0);
                    p.set_atk_dark(0);
                }
                if let Some(s) = params.get_mut::<BulletParam>(SWEEP) {
                    s.set_atk_id_bullet(-1); // a look only: no hit
                    s.set_sp_effect_id0(-1);
                    s.set_life(SWEEP_LIFE);
                    s.set_dist(SWING_REACH);
                    s.set_init_vellocity(SWEEP_SPEED);
                    s.set_max_vellocity(SWEEP_SPEED);
                    s.set_min_vellocity(SWEEP_SPEED);
                }
                glog!("BLADE", "values ON: carrier {CARRIER} -> Corpse Piler {CORPSE_PILER}, sweep {SWEEP} without damage");
                self.saved = Some(saved);
            }
            Some(v) => {
                if let Some(c) = params.get_mut::<BulletParam>(CARRIER) {
                    c.set_atk_id_bullet(v.carrier_atk);
                    c.set_sfx_id_bullet(v.carrier_sfx[0]);
                    c.set_sfx_id_flick(v.carrier_sfx[1]);
                    c.set_sfx_id_hit(v.carrier_sfx[2]);
                    c.set_init_vellocity(v.carrier[0]);
                    c.set_max_vellocity(v.carrier[1]);
                    c.set_min_vellocity(v.carrier[2]);
                    c.set_accel_out_range(v.carrier[3]);
                    c.set_gravity_in_range(v.carrier[4]);
                    c.set_gravity_out_range(v.carrier[5]);
                    c.set_life(v.carrier[6]);
                    c.set_dist(v.carrier[7]);
                    c.set_dmg_hit_record_life_time(v.carrier[8]);
                    c.set_homing_angle(v.carrier_homing);
                }
                if let Some(p) = params.get_mut::<AtkParam_Pc>(CORPSE_PILER) {
                    p.set_atk_phys(v.piler_atk[0]);
                    p.set_atk_mag(v.piler_atk[1]);
                    p.set_atk_fire(v.piler_atk[2]);
                    p.set_atk_thun(v.piler_atk[3]);
                    p.set_atk_dark(v.piler_atk[4]);
                    p.set_is_add_base_atk(v.piler_add_base);
                    p.set_disable_guard(v.piler_disable_guard);
                    p.set_atk_stam(v.piler_stam);
                    p.set_atk_super_armor(v.piler_poise);
                }
                if let Some(s) = params.get_mut::<BulletParam>(SWEEP) {
                    s.set_atk_id_bullet(v.sweep_atk);
                    s.set_sp_effect_id0(v.sweep_sp);
                    s.set_life(v.sweep_life);
                    s.set_dist(v.sweep_dist);
                    s.set_init_vellocity(v.sweep_speed[0]);
                    s.set_max_vellocity(v.sweep_speed[1]);
                    s.set_min_vellocity(v.sweep_speed[2]);
                }
                glog!("BLADE", "values OFF: vanilla rows restored");
            }
        }
    }
}

/// Living characters within `reach` (horizontally) in front of us, within `half_angle` degrees of `forward`, and at
/// a height our sword can reach.
fn in_cone(player: &PlayerIns, feet: Vec3, forward: Vec3, reach: f32, half_angle: f32) -> Vec<Target> {
    let cos = half_angle.to_radians().cos();
    targets::living(player)
        .into_iter()
        .filter(|t| {
            let d = Vec3::new(t.feet.x - feet.x, 0.0, t.feet.z - feet.z);
            let dist = d.length() - t.radius;
            let vertical = t.feet.y <= feet.y + 2.5 && t.feet.y + t.height >= feet.y - 1.0;
            vertical && dist <= reach && (d.length() < 1e-3 || d.normalize().dot(forward) >= cos)
        })
        .collect()
}

/// Bleed buildup for a share of the target's bar (NpcParam `resistBlood`); 0 for immune ones.
fn bleed_share(target: &Target, share: f32) -> i32 {
    let chr = unsafe { target.chr.as_ref() };
    let Ok(params) = (unsafe { SoloParamRepository::instance() }) else {
        return 0;
    };
    match params.get::<NpcParam>(chr.npc_param_id as u32).map(|n| n.resist_blood()) {
        Some(r) if r > 0 && r < BLEED_IMMUNE => (r as f32 * share).round().max(1.0) as i32,
        _ => 0,
    }
}

fn apply_bleed(target: &Target, amount: i32) {
    if amount <= 0 || unsafe { target.chr.as_ref() }.modules.data.hp <= 0 {
        return;
    }
    if let Ok(params) = unsafe { SoloParamRepository::instance_mut() } {
        if let Some(sp) = params.get_mut::<SpEffectParam>(BLEED_EFFECT) {
            sp.set_blood_attack_power(amount);
        }
    }
    unsafe { &mut *target.chr.as_ptr() }.apply_speffect(BLEED_EFFECT as i32, true);
    glog!("BLADE", "bleed {amount} applied to character {:#x}", target.chr.as_ptr() as usize);
}

fn direct(target: &Target, damage: i32, kind: &str, why: &str) {
    if let Some((before, after, max)) = targets::damage(target, damage) {
        glog!("BLADE", "{kind} HIT {damage} directly on character {:#x} ({why}): hp {before} -> {after} / {max}", target.chr.as_ptr() as usize);
    }
}
