//! Enemy health bars, Overwatch style (DESIGN.md 5i): Elden Ring's HUD is off in Genji mode, which hides its enemy
//! bars too, so we track nearby characters' health, choose which bars to show and where (projected with the camera
//! we draw from), and `hud.rs` draws them. Runs on the game thread, once per frame, from the camera task.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};

use eldenring::cs::{CSFeManImp, PlayerIns};
use fromsoftware_shared::FromStatic;
use glam::Vec3;

use crate::glog;
use crate::hud::{EnemyBar, ENEMY_BARS, NAME_BYTES};
use crate::look::Look;

/// Bars only for characters this close, metres (the game's line-of-sight check isn't available to us); bosses
/// further.
const RANGE: f32 = 50.0;
const BOSS_RANGE: f32 = 80.0;
/// A bar shows this long after the character last lost health, seconds.
const SHOW_FOR: f32 = 30.0;
/// Health just lost stays lit lighter this long, then drains (seconds; share of max health per second).
const TRAIL_WAIT: f32 = 0.5;
const TRAIL_DRAIN: f32 = 1.5;
/// Height of the bar above the top of the character's collision cylinder, metres.
const ABOVE_HEAD: f32 = 0.35;
/// The game's tables are logged this often, seconds.
const LOG_EVERY: f32 = 2.0;
/// If the game leaves a boss's name empty while its HUD is off, its HUD is switched on this long once, so it fills
/// the name in (seconds).
const NAME_PEEK: f32 = 0.4;

/// Asks `controls.rs` to switch Elden Ring's HUD on for a moment (to get a boss's name).
static HUD_PEEK: AtomicBool = AtomicBool::new(false);

pub fn hud_peek_wanted() -> bool {
    HUD_PEEK.load(Ordering::Relaxed)
}

/// The important part of a boss's name, in capitals (you, 2026-10-09: "only their names, no grand titles"): the part
/// before a comma, " of " or " the ", without a leading title. "Margit, the Fell Omen" -> "MARGIT", "Godrick the
/// Grafted" -> "GODRICK", "Radagon of the Golden Order" -> "RADAGON", "Starscourge Radahn" -> "RADAHN".
pub fn short_name(full: &str) -> String {
    let mut name = full.trim();
    if let Some(i) = name.find(',') {
        name = &name[..i];
    }
    for separator in [" of ", " the "] {
        if let Some(i) = name.find(separator) {
            name = &name[..i];
            break;
        }
    }
    for title in ["Starscourge ", "Lichdragon ", "Dragonlord ", "Promised Consort "] {
        if let Some(rest) = name.strip_prefix(title) {
            name = rest;
        }
    }
    name.trim().to_uppercase()
}

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
    /// Bosses' short names, by character.
    names: HashMap<usize, String>,
    /// The boss a HUD peek ran for (once per boss), and the seconds of it left.
    peeked: Option<usize>,
    peek_left: f32,
}

impl EnemyBars {
    /// Tracks health drops, then returns the bars to draw (bosses first, then nearest first). `fov` is the vertical
    /// field of view (radians) and `aspect` the width/height ratio of the camera we draw from at `eye`.
    pub fn update(&mut self, player: &PlayerIns, look: &Look, eye: Vec3, fov: f32, aspect: f32, dt: f32) -> ([EnemyBar; ENEMY_BARS], usize) {
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

        // Bosses: the game's own boss list (set by its events, also for field bosses) names the characters.
        let fe = unsafe { CSFeManImp::instance() }.ok();
        let bosses: Vec<usize> = fe
            .map(|fe| {
                fe.boss_health_displays
                    .iter()
                    .filter(|b| !b.field_ins_handle.is_empty())
                    .filter_map(|b| targets.iter().find(|t| unsafe { t.chr.as_ref() }.field_ins_handle == b.field_ins_handle))
                    .map(|t| t.chr.as_ptr() as usize)
                    .collect()
            })
            .unwrap_or_default();
        if bosses.first() != self.logged_boss.as_ref() {
            if let Some(id) = bosses.first() {
                glog!("BARS", "boss: character {id:#x} gets the skull bar above its head");
            }
            self.logged_boss = bosses.first().copied();
        }
        // Names: the game writes each boss bar's name into its own display data (`boss_list_tag_data`).
        if let Some(fe) = fe {
            for &id in &bosses {
                if self.names.contains_key(&id) {
                    continue;
                }
                let Some(t) = targets.iter().find(|t| t.chr.as_ptr() as usize == id) else {
                    continue;
                };
                let handle = unsafe { t.chr.as_ref() }.field_ins_handle;
                let tags = &fe.frontend_values.boss_list_tag_data;
                let full = tags
                    .iter()
                    .find(|tag| tag.field_ins_handle == handle)
                    .map(|tag| tag.chr_name.to_string())
                    .filter(|n| !n.trim().is_empty())
                    // One boss and one name: take it even if the game hasn't filled in the handle.
                    .or_else(|| {
                        let named: Vec<String> = tags.iter().map(|tag| tag.chr_name.to_string()).filter(|n| !n.trim().is_empty()).collect();
                        (bosses.len() == 1 && named.len() == 1).then(|| named[0].clone())
                    });
                if let Some(full) = full {
                    let short = short_name(&full);
                    glog!("BARS", "boss {id:#x} is \"{full}\": shown as \"{short}\"{}", if self.peek_left > 0.0 { " (during the HUD peek)" } else { "" });
                    self.names.insert(id, short);
                    if self.peeked == Some(id) {
                        self.peek_left = 0.0;
                    }
                } else if self.peeked != Some(id) {
                    glog!("BARS", "boss {id:#x}: the game has no name for it with its HUD off; switching its HUD on for {NAME_PEEK} s");
                    self.peeked = Some(id);
                    self.peek_left = NAME_PEEK;
                }
            }
        }
        if self.peek_left > 0.0 {
            self.peek_left -= dt;
            if self.peek_left <= 0.0 && self.peeked.is_some_and(|id| !self.names.contains_key(&id)) {
                glog!("BARS", "boss {:#x}: still no name after the HUD peek", self.peeked.unwrap_or(0));
            }
        }
        HUD_PEEK.store(self.peek_left > 0.0, Ordering::Relaxed);
        self.names.retain(|id, _| self.tracks.contains_key(id));

        let mut shown: Vec<(bool, f32, EnemyBar)> = Vec::new();
        for t in &targets {
            let id = t.chr.as_ptr() as usize;
            let data = &unsafe { t.chr.as_ref() }.modules.data;
            let (hp, max) = (data.hp, data.max_hp.max(1));
            let boss = bosses.contains(&id);
            let recent = self.tracks.get(&id).is_some_and(|tr| tr.since_hit <= SHOW_FOR);
            let dist = (t.feet - eye).length();
            // A boss's bar shows the whole fight; others only after losing health.
            let wanted = if boss { dist <= BOSS_RANGE } else { recent && hp < max && dist <= RANGE };
            if !wanted {
                continue;
            }
            if let Some((x, y)) = project(t.feet + Vec3::Y * (t.height + ABOVE_HEAD)) {
                let (hp_share, trail_share) = share(id, hp, max);
                let mut bar = EnemyBar { x, y, hp: hp_share, trail: trail_share, boss, ..EnemyBar::default() };
                if let Some(name) = self.names.get(&id) {
                    // Cut on a character boundary.
                    let mut end = name.len().min(NAME_BYTES);
                    while !name.is_char_boundary(end) {
                        end -= 1;
                    }
                    bar.name[..end].copy_from_slice(&name.as_bytes()[..end]);
                    bar.name_len = end as u8;
                }
                shown.push((boss, dist, bar));
            }
        }
        shown.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.total_cmp(&b.1)));
        let mut bars = [EnemyBar::default(); ENEMY_BARS];
        let count = shown.len().min(ENEMY_BARS);
        for (slot, (_, _, bar)) in bars.iter_mut().zip(shown) {
            *slot = bar;
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
                    glog!("BARS", "ours: {count} bar(s) | game tags: [{}] | game boss entries: {bosses}", tags.join("; "));
                }
            }
        }
        (bars, count)
    }
}

#[cfg(test)]
mod tests {
    use super::short_name;

    #[test]
    fn boss_names_lose_their_titles() {
        assert_eq!(short_name("Margit, the Fell Omen"), "MARGIT");
        assert_eq!(short_name("Godrick the Grafted"), "GODRICK");
        assert_eq!(short_name("Radagon of the Golden Order"), "RADAGON");
        assert_eq!(short_name("Starscourge Radahn"), "RADAHN");
        assert_eq!(short_name("Rennala, Queen of the Full Moon"), "RENNALA");
        assert_eq!(short_name("Tree Sentinel"), "TREE SENTINEL");
        assert_eq!(short_name("Elemer of the Briar"), "ELEMER");
        assert_eq!(short_name("Lichdragon Fortissax"), "FORTISSAX");
    }
}
