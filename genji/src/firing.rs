//! Shuriken timing (step 6b): Overwatch's burst of 3 on left click, Fan of Blades on right click, 30 shuriken and
//! the reload (automatic when empty, or R). Pure logic, no game access: `tick` says which throws to make this frame, and
//! `shuriken.rs` makes them.
//!
//! Overwatch rules (you, 2026-10-09): Swift Strike and quick melee stop shooting at once, so a dash or a melee cancels
//! the rest of a burst (the shuriken not thrown aren't spent, so the ammo needn't be a multiple of 3); no shuriken
//! during the dash or the melee; a burst or fan always throws 3 even with 1 or 2 left (it spends what's left).
//! No reload during a quick melee or its recovery (you, 2026-10-09): R does nothing then, an empty magazine waits for
//! the melee to end, and a melee cancels a reload in progress (as in Overwatch: you reload again after it).
//!
//! Values (2026-10-07): from a video measurement of Overwatch you passed on (another AI's analysis of 10 consecutive
//! throws) plus Blizzard's patch notes (0.68 s recovery, 75 m/s): 0.10 s between burst shuriken, 0.687 s between
//! fans measured, reload about 1.5 s. The burst cycle is inferred (0.2 s burst + 0.68 s recovery), not measured.

use crate::glog;

/// Shuriken per magazine.
const AMMO: u32 = 30;
/// Shuriken per burst (left click) and per fan (right click).
const BURST_SHOTS: u32 = 3;
const FAN_SHOTS: u32 = 3;
/// Time between the shuriken of one burst, seconds.
const BURST_SPACING: f32 = 0.1;
/// Time from the start of one burst to the earliest next burst or fan, seconds: the burst itself (2 x 0.1 s) plus
/// Overwatch's 0.68 s recovery. Was 0.68 (too fast).
const BURST_CYCLE: f32 = 0.88;
/// Time from a fan to the earliest next burst or fan, seconds (Overwatch's 0.68 s recovery; measured 0.687). Was 0.75.
const FAN_CYCLE: f32 = 0.68;
/// Sideways angle of the fan's outer shuriken, degrees: Genji's fan is 9 degrees in total, 4.5 to each side. With the
/// Overwatch field of view in first person (camera.rs, 2026-10-08) it also looks as wide as in Overwatch. Was 9.0.
const FAN_ANGLE: f32 = 4.5;
/// Reload time, seconds (automatic when empty, or R).
const RELOAD_TIME: f32 = 1.5;

#[derive(Debug)]
pub struct Firing {
    ammo: u32,
    /// Seconds until the next burst or fan may start.
    recovery: f32,
    /// Shuriken still to come in the current burst, and seconds until the next one.
    burst_left: u32,
    burst_timer: f32,
    /// Seconds left of a reload in progress.
    reload: Option<f32>,
    /// R pressed during a burst: the reload starts when the burst is over.
    reload_wanted: bool,
    /// A quick melee (animation and recovery) is on: no reload.
    meleeing: bool,
}

impl Default for Firing {
    fn default() -> Self {
        Self { ammo: AMMO, recovery: 0.0, burst_left: 0, burst_timer: 0.0, reload: None, reload_wanted: false, meleeing: false }
    }
}

impl Firing {
    /// For the HUD: shuriken left, the magazine size, and the share of a reload done (while reloading).
    pub fn hud(&self) -> (u32, u32, Option<f32>) {
        (self.ammo, AMMO, self.reload.map(|left| 1.0 - left / RELOAD_TIME))
    }

    /// R (your request, 2026-10-09: "I want to reload using R"): reloads unless the magazine is full or a reload is
    /// already on. During a burst, the reload starts when the burst is over (the burst isn't cut short).
    pub fn reload(&mut self) {
        if self.meleeing {
            glog!("FIRE", "R: no reload during the quick melee");
            return;
        }
        if self.ammo < AMMO && self.reload.is_none() && !self.reload_wanted {
            self.reload_wanted = true;
            glog!("FIRE", "R: reload asked with {} left{}", self.ammo, if self.burst_left > 0 { " (after this burst)" } else { "" });
        }
    }

    /// Whether a quick melee (animation and recovery) is on, before `tick`. Starting one cancels a reload in progress
    /// and a reload asked for during a burst.
    pub fn set_meleeing(&mut self, on: bool) {
        if on && !self.meleeing {
            if self.reload.take().is_some() {
                glog!("FIRE", "reload cancelled by the quick melee, ammo {}", self.ammo);
            }
            self.reload_wanted = false;
        }
        self.meleeing = on;
    }

    /// One frame. `primary`/`secondary`: left/right click held. `can_throw`: the controller drives this frame.
    /// Returns the sideways angle (radians, positive = left) of every shuriken to throw now.
    pub fn tick(&mut self, dt: f32, primary: bool, secondary: bool, can_throw: bool) -> Vec<f32> {
        let mut shots = Vec::new();
        self.recovery = (self.recovery - dt).max(0.0);

        if let Some(left) = self.reload.as_mut() {
            *left -= dt;
            if *left <= 0.0 {
                self.reload = None;
                self.ammo = AMMO;
                glog!("FIRE", "reload done, ammo {}", self.ammo);
            }
            return shots;
        }

        if self.reload_wanted && self.burst_left == 0 {
            self.reload_wanted = false;
            if self.ammo < AMMO {
                self.reload = Some(RELOAD_TIME);
                glog!("FIRE", "reloading by R ({RELOAD_TIME} s), {} left", self.ammo);
                return shots;
            }
        }

        if self.burst_left > 0 {
            if !can_throw {
                glog!("FIRE", "burst cut short ({} not thrown: dash, melee, Deflect, Dragonblade or the game driving), ammo {}", self.burst_left, self.ammo);
                self.burst_left = 0;
            } else {
                self.burst_timer -= dt;
                if self.burst_timer <= 0.0 {
                    self.take(&mut shots, 0.0);
                    self.burst_left -= 1;
                    self.burst_timer += BURST_SPACING;
                }
            }
        } else if can_throw && self.recovery <= 0.0 && self.ammo > 0 {
            if primary {
                self.take(&mut shots, 0.0);
                self.burst_left = BURST_SHOTS - 1;
                self.burst_timer = BURST_SPACING;
                self.recovery = BURST_CYCLE;
                glog!("FIRE", "burst, ammo {} after this burst", self.ammo.saturating_sub(self.burst_left));
            } else if secondary {
                let angle = FAN_ANGLE.to_radians();
                for i in 0..FAN_SHOTS {
                    let side = i as f32 - (FAN_SHOTS - 1) as f32 / 2.0;
                    self.take(&mut shots, side * angle);
                }
                self.recovery = FAN_CYCLE;
                glog!("FIRE", "fan of blades, ammo {}", self.ammo);
            }
        }

        if self.ammo == 0 && self.burst_left == 0 && !self.meleeing {
            self.reload = Some(RELOAD_TIME);
            glog!("FIRE", "empty, reloading ({RELOAD_TIME} s)");
        }
        shots
    }

    /// One shuriken: always thrown once its burst or fan has started; spends one if any are left.
    fn take(&mut self, shots: &mut Vec<f32>, angle: f32) {
        self.ammo = self.ammo.saturating_sub(1);
        shots.push(angle);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DT: f32 = 1.0 / 60.0;

    /// Runs `seconds` of frames and returns (time, angle) of every throw.
    fn run(f: &mut Firing, seconds: f32, primary: bool, secondary: bool) -> Vec<(f32, f32)> {
        let mut out = Vec::new();
        let frames = (seconds / DT).round() as u32;
        for i in 0..frames {
            for a in f.tick(DT, primary, secondary, true) {
                out.push((i as f32 * DT, a));
            }
        }
        out
    }

    #[test]
    fn burst_is_three_shots_a_tenth_apart() {
        let mut f = Firing::default();
        let shots = run(&mut f, 0.5, true, false);
        assert_eq!(shots.len(), 3);
        assert!((shots[1].0 - shots[0].0 - BURST_SPACING).abs() < 2.0 * DT);
        assert!((shots[2].0 - shots[1].0 - BURST_SPACING).abs() < 2.0 * DT);
        assert!(shots.iter().all(|s| s.1 == 0.0));
    }

    #[test]
    fn held_click_repeats_bursts_at_the_cycle() {
        let mut f = Firing::default();
        let shots = run(&mut f, BURST_CYCLE + 0.3, true, false);
        assert_eq!(shots.len(), 6);
        assert!((shots[3].0 - shots[0].0 - BURST_CYCLE).abs() < 2.0 * DT);
    }

    #[test]
    fn fan_is_three_at_once_spread_sideways() {
        let mut f = Firing::default();
        let shots = run(&mut f, 0.5, false, true);
        assert_eq!(shots.len(), 3);
        assert!(shots.iter().all(|s| s.0 == shots[0].0));
        let a = FAN_ANGLE.to_radians();
        assert!((shots[0].1 + a).abs() < 1e-6 && shots[1].1 == 0.0 && (shots[2].1 - a).abs() < 1e-6);
    }

    #[test]
    fn no_fan_during_a_burst() {
        let mut f = Firing::default();
        run(&mut f, 0.05, true, false);
        let shots = run(&mut f, 0.3, false, true);
        assert!(shots.iter().all(|s| s.1 == 0.0), "only the rest of the burst: {shots:?}");
    }

    #[test]
    fn empty_reloads_then_throws_again() {
        let mut f = Firing::default();
        let mut thrown = 0;
        for _ in 0..1000 {
            thrown += f.tick(DT, true, false, true).len();
            if f.reload.is_some() {
                break;
            }
        }
        assert_eq!(thrown, AMMO as usize);
        assert!(f.reload.is_some());
        assert!(run(&mut f, RELOAD_TIME - 0.1, true, false).is_empty());
        assert!(!run(&mut f, 0.2, true, false).is_empty());
    }

    #[test]
    fn last_one_or_two_still_throw_three() {
        for left in [1, 2] {
            let mut f = Firing { ammo: left, ..Firing::default() };
            assert_eq!(run(&mut f, 0.5, true, false).len(), 3, "burst with {left} left");
            assert_eq!(f.ammo, 0);
            assert!(f.reload.is_some());
            let mut f = Firing { ammo: left, ..Firing::default() };
            assert_eq!(run(&mut f, 0.1, false, true).len(), 3, "fan with {left} left");
            assert!(f.reload.is_some());
        }
    }

    #[test]
    fn dash_cancels_the_rest_of_a_burst_and_keeps_its_ammo() {
        let mut f = Firing::default();
        assert_eq!(f.tick(DT, true, false, true).len(), 1);
        // The dash: no throwing allowed for 12 frames.
        let during: usize = (0..12).map(|_| f.tick(DT, true, false, false).len()).sum();
        assert_eq!(during, 0);
        assert_eq!(f.ammo, AMMO - 1);
        assert_eq!(f.burst_left, 0);
    }

    #[test]
    fn r_reloads_after_the_burst_and_not_when_full() {
        let mut f = Firing::default();
        f.reload();
        assert!(!f.reload_wanted && f.reload.is_none(), "full magazine: no reload");
        assert_eq!(f.tick(DT, true, false, true).len(), 1);
        f.reload();
        // The rest of the burst is still thrown, then the reload starts.
        assert_eq!(run(&mut f, 0.3, false, false).len(), 2);
        assert!(f.reload.is_some());
        assert_eq!(f.ammo, AMMO - 3);
        assert!(run(&mut f, RELOAD_TIME - 0.3, true, false).is_empty(), "no throws while reloading");
        run(&mut f, 0.3, false, false);
        assert!(f.reload.is_none());
        assert_eq!(f.ammo, AMMO);
    }

    #[test]
    fn no_reload_during_a_melee() {
        // R during the melee does nothing.
        let mut f = Firing { ammo: 10, ..Firing::default() };
        f.set_meleeing(true);
        f.reload();
        run(&mut f, 0.1, false, false);
        assert!(f.reload.is_none());
        // A melee cancels a reload in progress; the ammo stays.
        let mut f = Firing { ammo: 10, ..Firing::default() };
        f.reload();
        run(&mut f, 0.5, false, false);
        assert!(f.reload.is_some());
        f.set_meleeing(true);
        run(&mut f, RELOAD_TIME, false, false);
        assert!(f.reload.is_none());
        assert_eq!(f.ammo, 10);
        // Empty: the automatic reload waits for the melee to end.
        let mut f = Firing { ammo: 0, ..Firing::default() };
        f.set_meleeing(true);
        run(&mut f, 0.3, false, false);
        assert!(f.reload.is_none());
        f.set_meleeing(false);
        run(&mut f, DT, false, false);
        assert!(f.reload.is_some());
    }

    #[test]
    fn burst_dropped_when_not_driving() {
        let mut f = Firing::default();
        assert_eq!(f.tick(DT, true, false, true).len(), 1);
        assert!(f.tick(DT, true, false, false).is_empty());
        assert_eq!(f.burst_left, 0);
        assert_eq!(f.ammo, AMMO - 1);
    }
}
