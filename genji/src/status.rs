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

use std::collections::HashMap;

use eldenring::cs::PlayerIns;

use crate::glog;

pub const COUNT: usize = 7;
/// In the order of the resistance module.
pub const NAMES: [&str; COUNT] = ["POISON", "SCARLET ROT", "BLOOD LOSS", "DEATH BLIGHT", "FROSTBITE", "SLEEP", "MADNESS"];
const MODULE_OFFSET: usize = 0x20;
const VALUES_OFFSET: usize = 0x10;

/// One status as the HUD shows it.
#[derive(Clone, Copy, Default)]
pub struct Bar {
    /// Share of the way to the status taking hold (0..1).
    pub fill: f32,
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
}

impl Watch {
    /// One frame: the bars to show (None when the readings don't look right).
    pub fn update(&mut self, player: &PlayerIns, dt: f32) -> Option<[Bar; COUNT]> {
        self.log_wait -= dt;
        self.effect_second -= dt;
        if self.effect_second <= 0.0 {
            self.effect_second = 1.0;
            self.effect_lines = 20;
        }
        self.log_effects(player);
        let values = read(player)?;
        if values.iter().any(|&(_, max)| !(1..=10_000).contains(&max)) {
            if !self.bad_logged {
                glog!("STATUS", "resistance readings don't look right, no status bars: {values:?}");
                self.bad_logged = true;
            }
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
        Some(fills(&mut self.up, &values))
    }

    fn log_effects(&mut self, player: &PlayerIns) {
        let now: HashMap<i32, f32> = player.chr_ins.special_effect.entries().map(|e| (e.param_id, e.duration)).collect();
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
}
