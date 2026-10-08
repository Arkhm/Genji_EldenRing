//! Ultimate charge (step 9, DESIGN.md 5g): counted in Overwatch damage, not Elden Ring damage (dexterity scaling and
//! Elden Ring's health would otherwise change how fast it fills), plus Overwatch's passive charge.

use std::sync::Mutex;

use crate::glog;

/// Overwatch: about 2000 points (2125 minus the 6% of 2026-08-11; passed on by you, not verified exactly).
pub const COST: f32 = 2000.0;
/// Overwatch: 5 points per second, paused while Dragonblade is active.
const PASSIVE_PER_SECOND: f32 = 5.0;

struct State {
    charge: f32,
    reported_tenth: u32,
}

static STATE: Mutex<State> = Mutex::new(State { charge: 0.0, reported_tenth: 0 });

/// Adds charge for damage dealt, in Overwatch points (`why` is logged when a tenth of the bar is crossed).
pub fn add(points: f32, why: &str) {
    let mut s = STATE.lock().unwrap_or_else(|e| e.into_inner());
    s.charge = (s.charge + points).min(COST);
    report(&mut s, why);
}

/// Passive charge; `paused` while Dragonblade is active.
pub fn tick(dt: f32, paused: bool) {
    if paused {
        return;
    }
    let mut s = STATE.lock().unwrap_or_else(|e| e.into_inner());
    s.charge = (s.charge + PASSIVE_PER_SECOND * dt).min(COST);
    report(&mut s, "passive");
}

fn report(s: &mut State, why: &str) {
    let tenth = (s.charge / COST * 10.0) as u32;
    if tenth != s.reported_tenth {
        s.reported_tenth = tenth;
        glog!("ULT", "charge {:.0} / {COST} ({}%), last from {why}", s.charge, tenth * 10);
    }
}

pub fn percent() -> u32 {
    let s = STATE.lock().unwrap_or_else(|e| e.into_inner());
    (s.charge / COST * 100.0) as u32
}

/// **Testing only (2026-10-08, like the 1 s Deflect cooldown): F7 fills the charge. Remove once Dragonblade is validated.**
pub fn fill_for_testing() {
    let mut s = STATE.lock().unwrap_or_else(|e| e.into_inner());
    s.charge = COST;
    report(&mut s, "F7 (testing)");
}

/// Spends a full charge; false if not full.
pub fn spend() -> bool {
    let mut s = STATE.lock().unwrap_or_else(|e| e.into_inner());
    if s.charge < COST {
        return false;
    }
    s.charge = 0.0;
    s.reported_tenth = 0;
    true
}
