//! Enemy health bars, Overwatch style (DESIGN.md 5i): Elden Ring's HUD is off in Genji mode, which hides its enemy
//! bars too, so we track nearby characters' health, choose which bars to show and where (projected with the camera
//! we draw from), and `hud.rs` draws them. Runs on the game thread, once per frame, from the camera task.

use std::collections::HashMap;

use eldenring::cs::{CSFeManImp, PlayerIns};
use fromsoftware_shared::FromStatic;
use glam::Vec3;

use crate::glog;
use crate::hud::{BossBar, EnemyBar, ENEMY_BARS};
use crate::look::Look;

/// Bars only for characters this close, metres (the game's line-of-sight check isn't available to us).
const RANGE: f32 = 50.0;
/// A bar shows this long after the character last lost health, seconds.
const SHOW_FOR: f32 = 30.0;
/// Health just lost stays lit lighter this long, then drains (seconds; share of max health per second).
const TRAIL_WAIT: f32 = 0.5;
const TRAIL_DRAIN: f32 = 1.5;
/// Height of the bar above the top of the character's collision cylinder, metres.
const ABOVE_HEAD: f32 = 0.35;
/// The game's tables are logged this often, seconds.
const LOG_EVERY: f32 = 2.0;

struct Track {
    last_hp: i32,
    trail_hp: f32,
    trail_wait: f32,
    since_hit: f32,
    seen: bool,
}

#[derive(Default)]
pub struct EnemyBars {
    tracks: HashMap<usize, Track>,
    log_timer: f32,
    logged_boss: Option<usize>,
}

impl EnemyBars {
    /// Tracks health drops, then returns the bars to draw (nearest first) and the boss bar. `fov` is the vertical
    /// field of view (radians) and `aspect` the width/height ratio of the camera we draw from at `eye`.
    pub fn update(&mut self, player: &PlayerIns, look: &Look, eye: Vec3, fov: f32, aspect: f32, dt: f32) -> ([EnemyBar; ENEMY_BARS], usize, Option<BossBar>) {
        let targets = crate::targets::living(player);
        for track in self.tracks.values_mut() {
            track.seen = false;
        }
        for t in &targets {
            let data = &unsafe { t.chr.as_ref() }.modules.data;
            let (hp, max) = (data.hp, data.max_hp.max(1));
            let track = self.tracks.entry(t.chr.as_ptr() as usize).or_insert(Track {
                last_hp: hp,
                trail_hp: hp as f32,
                trail_wait: 0.0,
                since_hit: f32::MAX,
                seen: true,
            });
            if hp < track.last_hp {
                track.since_hit = 0.0;
                track.trail_wait = TRAIL_WAIT;
                track.trail_hp = track.trail_hp.max(track.last_hp as f32);
            } else if hp > track.last_hp {
                track.trail_hp = hp as f32;
            }
            track.last_hp = hp;
            if track.since_hit < f32::MAX {
                track.since_hit += dt;
            }
            if track.trail_wait > 0.0 {
                track.trail_wait -= dt;
            } else {
                track.trail_hp = (track.trail_hp - max as f32 * TRAIL_DRAIN * dt).max(hp as f32);
            }
            track.seen = true;
        }
        self.tracks.retain(|_, t| t.seen);

        let (right, up, forward) = (look.right(), look.up(), look.forward());
        let tan_y = (fov / 2.0).tan();
        let project = |p: Vec3| -> Option<(f32, f32)> {
            let v = p - eye;
            let z = v.dot(forward);
            if z <= 0.3 {
                return None;
            }
            let (x, y) = (v.dot(right) / (z * tan_y * aspect), v.dot(up) / (z * tan_y));
            (x.abs() < 1.1 && y.abs() < 1.1).then_some((x, y))
        };
        let share = |id: usize, hp: i32, max: i32| {
            let trail = self.tracks.get(&id).map_or(hp as f32, |t| t.trail_hp);
            ((hp as f32 / max as f32).clamp(0.0, 1.0), (trail / max as f32).clamp(0.0, 1.0))
        };

        let mut shown: Vec<(f32, EnemyBar)> = Vec::new();
        for t in &targets {
            let id = t.chr.as_ptr() as usize;
            let data = &unsafe { t.chr.as_ref() }.modules.data;
            let (hp, max) = (data.hp, data.max_hp.max(1));
            let recent = self.tracks.get(&id).is_some_and(|tr| tr.since_hit <= SHOW_FOR);
            let dist = (t.feet - eye).length();
            if !recent || hp >= max || dist > RANGE {
                continue;
            }
            if let Some((x, y)) = project(t.feet + Vec3::Y * (t.height + ABOVE_HEAD)) {
                let (hp_share, trail_share) = share(id, hp, max);
                shown.push((dist, EnemyBar { x, y, hp: hp_share, trail: trail_share }));
            }
        }
        shown.sort_by(|a, b| a.0.total_cmp(&b.0));
        let mut bars = [EnemyBar::default(); ENEMY_BARS];
        let count = shown.len().min(ENEMY_BARS);
        for (slot, (_, bar)) in bars.iter_mut().zip(shown) {
            *slot = bar;
        }

        // The boss: the game's own boss list (set by its events) names the character.
        let fe = unsafe { CSFeManImp::instance() }.ok();
        let mut boss = None;
        if let Some(fe) = fe {
            for entry in fe.boss_health_displays.iter().filter(|b| !b.field_ins_handle.is_empty()) {
                if let Some(t) = targets.iter().find(|t| unsafe { t.chr.as_ref() }.field_ins_handle == entry.field_ins_handle) {
                    let id = t.chr.as_ptr() as usize;
                    let data = &unsafe { t.chr.as_ref() }.modules.data;
                    let (hp_share, trail_share) = share(id, data.hp, data.max_hp.max(1));
                    boss = Some(BossBar { hp: hp_share, trail: trail_share });
                    if self.logged_boss != Some(id) {
                        glog!("BARS", "boss bar: character {id:#x} (name message {}), hp {} / {}", entry.fmg_id, data.hp, data.max_hp);
                        self.logged_boss = Some(id);
                    }
                    break;
                }
            }
        }
        if boss.is_none() {
            self.logged_boss = None;
        }

        // Diagnostics: what the game itself keeps in its enemy-tag and boss tables while its HUD is off.
        self.log_timer += dt;
        if self.log_timer >= LOG_EVERY {
            self.log_timer = 0.0;
            if let Some(fe) = fe {
                let tags: Vec<String> = fe
                    .enemy_chr_tag_displays
                    .iter()
                    .filter(|e| !e.field_ins_handle.is_empty())
                    .map(|e| format!("{} vis {} at ({:.0}, {:.0}) dmg {}", e.field_ins_handle, e.is_visible, e.screen_pos.0, e.screen_pos.1, e.damage_taken))
                    .collect();
                let bosses = fe.boss_health_displays.iter().filter(|b| !b.field_ins_handle.is_empty()).count();
                if !tags.is_empty() || bosses > 0 || count > 0 {
                    glog!("BARS", "ours: {count} bar(s){} | game tags: [{}] | game boss entries: {bosses}", if boss.is_some() { " + boss" } else { "" }, tags.join("; "));
                }
            }
        }
        (bars, count, boss)
    }
}
