//! The player's status effects (poison, scarlet rot, blood loss, frostbite, sleep, madness, death blight) for the
//! Overwatch HUD's status bars (DESIGN.md 5l, part 2): Elden Ring's own HUD is switched off in Genji mode, and with it
//! its status gauges (you, 2026-10-09).
//!
//! Where the numbers come from: the character's resistance module (`ChrInsModuleContainer.resist`, at 0x20 in the
//! module list), which fromsoftware-rs doesn't describe yet. Layout used, as modders publish it (not verified here):
//! from 0x10, seven 4-byte numbers for the buildup of poison, scarlet rot, blood loss, death blight, frostbite, sleep
//! and madness, then the seven maximums in the same order. Checks: the maximums must be 1..=10000, else nothing is
//! shown; whether a status counts up to its maximum or down from it is taken from its value at rest. Every change is
//! logged (`STATUS`), and so are the special effects starting and ending on the character, to check the layout and
//! to find the statuses' active phase.
//!
//! Taken hold (DESIGN.md 5l, part 4; your test of 2026-10-10: "I got scarlet rot and the HUD showed nothing"): the
//! buildup empties in one step when a status takes hold (rot: 5/101, then 101/101). The effects that start within
//! LINK_WINDOW of that moment and last are the status's active phase; the bar then shows the time left of the one
//! that ends first. With none (blood loss: a single burst), the row flashes.

use std::collections::HashMap;

use eldenring::cs::PlayerIns;

use crate::glog;

pub const COUNT: usize = 7;
/// In the order of the resistance module.
pub const NAMES: [&str; COUNT] = ["POISON", "SCARLET ROT", "BLOOD LOSS", "DEATH BLIGHT", "FROSTBITE", "SLEEP", "MADNESS"];
const MODULE_OFFSET: usize = 0x20;
const VALUES_OFFSET: usize = 0x10;

/// A status has taken hold when its bar drops from at least HOLD_FROM to at most HOLD_TO between two frames.
const HOLD_FROM: f32 = 0.8;
const HOLD_TO: f32 = 0.1;
/// Effects starting this many seconds before or after that moment are linked to it.
const LINK_WINDOW: f32 = 0.3;
/// Effects lasting at least this long (or until removed) can be a status's active phase.
const LASTING: f32 = 1.0;
/// Seconds the row flashes when a status takes hold with no lasting effect.
pub const FLASH: f32 = 1.5;
/// Seconds between log lines of an active status's time left.
const HOLD_LOG_EVERY: f32 = 5.0;

/// One status as the HUD shows it.
#[derive(Clone, Copy, Default)]
pub struct Bar {
    /// Share of the way to the status taking hold (0..1).
    pub fill: f32,
    /// Taken hold: share of its time left (0..1) and the seconds left (None: it lasts until removed).
    pub active: Option<(f32, Option<f32>)>,
    /// Taken hold with no lasting effect: a flash, 1 when it starts, 0 when over.
    pub flash: f32,
}

impl Bar {
    pub const EMPTY: Bar = Bar { fill: 0.0, active: None, flash: 0.0 };
}

/// A special effect on the character, as read this frame.
#[derive(Clone, Copy)]
struct Effect {
    id: i32,
    /// Seconds it lasts in all (-1: until removed).
    duration: f32,
    /// The game's countdown to its removal.
    removal_timer: f32,
    /// From its param row: the game's label for what the effect is, and its icon.
    state_info: Option<u16>,
    icon_id: Option<i32>,
}

/// A status that has taken hold.
struct Hold {
    /// Our clock when it took hold.
    at: f32,
    /// The effect the bar follows, once chosen: its id, length, and our clock when it started.
    effect: Option<(i32, f32, f32)>,
    /// The effect's countdown last frame (to tell whether it counts down), and whether our own clock is used instead.
    last_timer: Option<f32>,
    own_clock: bool,
    next_log: f32,
}

#[derive(Default)]
pub struct Watch {
    /// Per status: counts up to its maximum (true) or down from it (false), once seen at rest.
    up: [Option<bool>; COUNT],
    last: Option<[(i32, i32); COUNT]>,
    bad_logged: bool,
    /// Special effects on the character: id -> duration, to log starts and ends.
    effects: HashMap<i32, f32>,
    first_effects: bool,
    /// Seconds until the next change may be logged (values change every frame while building up or draining).
    log_wait: f32,
    /// Effect lines left this second (some effects may come and go every frame).
    effect_lines: u32,
    effect_second: f32,
    /// Seconds since the first frame (our clock).
    clock: f32,
    /// Effects that started recently: (effect, our clock when it started).
    starts: Vec<(Effect, f32)>,
    /// Each status's bar last frame (None: no readings).
    prev_fill: Option<[f32; COUNT]>,
    holds: [Option<Hold>; COUNT],
    /// Seconds of flash left per status.
    flash: [f32; COUNT],
}

impl Watch {
    /// One frame: the bars to show (None when the readings don't look right).
    pub fn update(&mut self, player: &PlayerIns, dt: f32) -> Option<[Bar; COUNT]> {
        self.log_wait -= dt;
        self.clock += dt;
        self.effect_second -= dt;
        if self.effect_second <= 0.0 {
            self.effect_second = 1.0;
            self.effect_lines = 20;
        }
        let effects = effects(player);
        self.log_effects(&effects);
        let Some(values) = read(player) else {
            self.prev_fill = None;
            return None;
        };
        if values.iter().any(|&(_, max)| !(1..=10_000).contains(&max)) {
            if !self.bad_logged {
                glog!("STATUS", "resistance readings don't look right, no status bars: {values:?}");
                self.bad_logged = true;
            }
            self.prev_fill = None;
            return None;
        }
        if self.last != Some(values) && (self.last.is_none() || self.log_wait <= 0.0) {
            self.log_wait = 0.5;
            let changed: Vec<String> = (0..COUNT)
                .filter(|&i| self.last.is_none_or(|l| l[i] != values[i]))
                .map(|i| format!("{} {}/{}", NAMES[i], values[i].0, values[i].1))
                .collect();
            glog!("STATUS", "{}", changed.join(", "));
            if self.last.is_none() {
                // The maximums are the resistances: poison and rot share Immunity, blood loss and frostbite
                // Robustness, sleep and madness Focus. If they pair up, the layout is likely right.
                let pairs = values[0].1 == values[1].1 && values[2].1 == values[4].1 && values[5].1 == values[6].1;
                glog!("STATUS", "maximums pair up as the resistances do: {pairs}");
            }
            self.last = Some(values);
        }
        let mut bars = fills(&mut self.up, &values);
        let fill: [f32; COUNT] = std::array::from_fn(|i| bars[i].fill);
        if let Some(prev) = self.prev_fill {
            for i in 0..COUNT {
                if took_hold(prev[i], fill[i]) {
                    glog!("STATUS", "{} took hold (bar {:.2} -> {:.2}, reading {}/{})", NAMES[i], prev[i], fill[i], values[i].0, values[i].1);
                    if self.holds[i].is_some() {
                        glog!("STATUS", "{}: it was still active; starting over", NAMES[i]);
                    }
                    self.holds[i] = Some(Hold { at: self.clock, effect: None, last_timer: None, own_clock: false, next_log: 0.0 });
                }
            }
        }
        self.prev_fill = Some(fill);
        for (i, bar) in bars.iter_mut().enumerate() {
            self.flash[i] = (self.flash[i] - dt).max(0.0);
            bar.active = self.hold(i, &effects);
            bar.flash = self.flash[i] / FLASH;
        }
        Some(bars)
    }

    /// Status `i`'s active phase this frame: links its effect once LINK_WINDOW has passed, follows it, ends it.
    fn hold(&mut self, i: usize, effects: &[Effect]) -> Option<(f32, Option<f32>)> {
        let clock = self.clock;
        // Effects other statuses already follow aren't linked again.
        let taken: Vec<i32> = self.holds.iter().filter_map(|h| h.as_ref()?.effect.map(|e| e.0)).collect();
        let h = self.holds[i].as_mut()?;
        if h.effect.is_none() {
            if clock - h.at < LINK_WINDOW {
                return None;
            }
            let near: Vec<(Effect, f32)> = self.starts.iter().filter(|(_, t)| (t - h.at).abs() <= LINK_WINDOW).copied().collect();
            for (e, t) in &near {
                glog!(
                    "STATUS",
                    "{}: effect {} started {:+.2} s from it, lasts {:.1} s, time left {:.1}, state_info {:?}, icon {:?}",
                    NAMES[i],
                    e.id,
                    t - h.at,
                    e.duration,
                    e.removal_timer,
                    e.state_info,
                    e.icon_id
                );
            }
            let candidates: Vec<(Effect, f32)> =
                near.into_iter().filter(|(e, _)| !taken.contains(&e.id) && effects.iter().any(|n| n.id == e.id)).collect();
            match pick(&candidates) {
                Some((e, t)) => {
                    glog!("STATUS", "{}: the bar follows effect {} ({:.1} s)", NAMES[i], e.id, e.duration);
                    h.effect = Some((e.id, e.duration, t));
                }
                None => {
                    glog!("STATUS", "{}: no lasting effect started with it; the row flashes", NAMES[i]);
                    self.holds[i] = None;
                    self.flash[i] = FLASH;
                    return None;
                }
            }
        }
        let (id, duration, started) = h.effect?;
        let Some(e) = effects.iter().find(|e| e.id == id) else {
            glog!("STATUS", "{}: effect {id} ended after {:.1} s; no longer active", NAMES[i], clock - started);
            self.holds[i] = None;
            return None;
        };
        if duration < 0.0 {
            return Some((1.0, None));
        }
        // The game's countdown, if it looks like one: within 0..duration and not going up.
        let counts_down =
            e.removal_timer > 0.0 && e.removal_timer <= duration + 0.01 && h.last_timer.is_none_or(|l| e.removal_timer <= l + 0.001);
        if !counts_down && !h.own_clock {
            h.own_clock = true;
            glog!("STATUS", "{}: effect {id}'s time left ({:.2}) isn't counting down; our own clock is used", NAMES[i], e.removal_timer);
        }
        h.last_timer = Some(e.removal_timer);
        let left = if h.own_clock { (duration - (clock - started)).max(0.0) } else { e.removal_timer };
        if clock >= h.next_log {
            h.next_log = clock + HOLD_LOG_EVERY;
            glog!("STATUS", "{}: {left:.1} s of {duration:.1} left (effect {id}, its countdown {:.1})", NAMES[i], e.removal_timer);
        }
        Some(((left / duration).clamp(0.0, 1.0), Some(left)))
    }

    fn log_effects(&mut self, effects: &[Effect]) {
        let now: HashMap<i32, f32> = effects.iter().map(|e| (e.id, e.duration)).collect();
        if self.first_effects {
            for e in effects.iter().filter(|e| !self.effects.contains_key(&e.id)) {
                self.starts.push((*e, self.clock));
            }
        }
        let clock = self.clock;
        self.starts.retain(|(_, t)| clock - t <= 2.0 * LINK_WINDOW + 1.0);
        if !self.first_effects {
            self.first_effects = true;
            let mut ids: Vec<i32> = now.keys().copied().collect();
            ids.sort();
            glog!("STATUS", "special effects on you at the start: {ids:?}");
        } else {
            for (id, duration) in &now {
                if !self.effects.contains_key(id) && self.effect_lines > 0 {
                    self.effect_lines -= 1;
                    glog!("STATUS", "special effect {id} started (lasts {duration:.1} s)");
                }
            }
            for id in self.effects.keys() {
                if !now.contains_key(id) && self.effect_lines > 0 {
                    self.effect_lines -= 1;
                    glog!("STATUS", "special effect {id} ended");
                }
            }
        }
        self.effects = now;
    }
}

/// The bars from the readings; `up` learns each status's direction from its value at rest (0: counts up; the
/// maximum: counts down) and keeps it.
fn fills(up: &mut [Option<bool>; COUNT], values: &[(i32, i32); COUNT]) -> [Bar; COUNT] {
    let mut bars = [Bar::default(); COUNT];
    for (i, &(value, max)) in values.iter().enumerate() {
        if up[i].is_none() {
            if value == 0 {
                up[i] = Some(true);
            } else if value == max {
                up[i] = Some(false);
            }
            if let Some(up) = up[i] {
                glog!("STATUS", "{} counts {} (value {value} of {max} at rest)", NAMES[i], if up { "up" } else { "down" });
            }
        }
        let share = match up[i] {
            Some(true) => value as f32 / max as f32,
            Some(false) => 1.0 - value as f32 / max as f32,
            None => 0.0,
        };
        bars[i].fill = share.clamp(0.0, 1.0);
    }
    bars
}

/// A bar dropping from nearly full to nearly empty in one frame: the status took hold.
fn took_hold(prev: f32, now: f32) -> bool {
    prev >= HOLD_FROM && now <= HOLD_TO
}

/// The effect a status's bar follows, from those that started with it: the lasting one that ends first; with none of
/// finite length, one that lasts until removed.
fn pick(candidates: &[(Effect, f32)]) -> Option<(Effect, f32)> {
    let finite = candidates.iter().filter(|(e, _)| e.duration >= LASTING).min_by(|a, b| a.0.duration.total_cmp(&b.0.duration));
    finite.or_else(|| candidates.iter().find(|(e, _)| e.duration < 0.0)).copied()
}

/// The special effects on the character, with what their param rows say.
fn effects(player: &PlayerIns) -> Vec<Effect> {
    player
        .chr_ins
        .special_effect
        .entries()
        .map(|e| {
            // SAFETY: a live effect's param row, the game's own.
            let row = e.param_data.map(|p| unsafe { p.as_ref() });
            Effect {
                id: e.param_id,
                duration: e.duration,
                removal_timer: e.removal_timer,
                state_info: row.map(|r| r.state_info()),
                icon_id: row.map(|r| r.icon_id()),
            }
        })
        .collect()
}

/// (buildup, maximum) per status, from the resistance module.
fn read(player: &PlayerIns) -> Option<[(i32, i32); COUNT]> {
    let modules: *const u8 = (&*player.chr_ins.modules as *const eldenring::cs::ChrInsModuleContainer).cast();
    // SAFETY: the module list is the game's, for a live player; the resistance module is one of its pointers.
    let resist = unsafe { modules.add(MODULE_OFFSET).cast::<usize>().read() };
    if resist == 0 || resist % 8 != 0 {
        return None;
    }
    let at = |k: usize| unsafe { ((resist + VALUES_OFFSET + 4 * k) as *const i32).read() };
    Some(std::array::from_fn(|i| (at(i), at(COUNT + i))))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn direction_comes_from_the_value_at_rest() {
        let mut up = [None; COUNT];
        let mut v = [(0, 300); COUNT];
        v[2] = (150, 150); // blood loss sits at its maximum at rest: counts down
        let b = fills(&mut up, &v);
        assert!(b.iter().all(|b| b.fill == 0.0));
        v[0] = (75, 300);
        v[2] = (50, 150);
        let b = fills(&mut up, &v);
        assert_eq!(b[0].fill, 0.25);
        assert!((b[2].fill - 2.0 / 3.0).abs() < 1e-6);
        // A status first seen half built up shows nothing until it has been seen at rest.
        let mut up = [None; COUNT];
        assert_eq!(fills(&mut up, &[(100, 300); COUNT])[0].fill, 0.0);
    }

    fn effect(id: i32, duration: f32) -> Effect {
        Effect { id, duration, removal_timer: duration, state_info: None, icon_id: None }
    }

    /// Your rot of 2026-10-10: 5/101 then 101/101 (counting down) is taking hold; draining normally isn't.
    #[test]
    fn taking_hold_is_a_full_bar_emptying_at_once() {
        let fill = |v: f32| 1.0 - v / 101.0;
        assert!(took_hold(fill(5.0), fill(101.0)));
        assert!(took_hold(fill(0.0), fill(101.0)));
        assert!(!took_hold(fill(5.0), fill(8.0)));
        assert!(!took_hold(fill(60.0), fill(101.0)));
    }

    /// Rot started 21000 (180 s) and 505 (900 s) at once: the bar follows the one that ends first; an effect that
    /// lasts until removed only when nothing finite lasts.
    #[test]
    fn the_bar_follows_the_lasting_effect_that_ends_first() {
        let c = [(effect(505, 900.0), 0.0), (effect(21000, 180.0), 0.0), (effect(501, 0.1), 0.0)];
        assert_eq!(pick(&c).map(|x| x.0.id), Some(21000));
        let c = [(effect(501, 0.1), 0.0), (effect(4202, -1.0), 0.0)];
        assert_eq!(pick(&c).map(|x| x.0.id), Some(4202));
        assert!(pick(&[(effect(501, 0.1), 0.0)]).is_none());
    }
}
