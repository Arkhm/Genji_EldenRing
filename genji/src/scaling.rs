//! Dexterity scaling for all of Genji's damage (your request, 2026-10-08).
//!
//! Damage = Overwatch base x (1 + DEX_SCALING x curve(dexterity) / 100), where the curve is Elden Ring's own
//! "Default" scaling curve (CalcCorrectGraph row 0, the one dexterity weapons use), read from the game's params at
//! runtime. Your regulation.bin (2026-10-08): dexterity 1 -> 0%, 18 -> 25%, 60 -> 75%, 80 -> 90%, 150 -> 110%,
//! exponents 1.2, -1.2, 1, 1. DEX_SCALING 1.5 is what the game would show as an "A" dexterity scaling.

use eldenring::cs::{CalcCorrectGraph, PlayerIns, SoloParamRepository};
use fromsoftware_shared::FromStatic;

use crate::glog;

/// How strongly damage follows dexterity (1.5 = grade A in the game's terms: A is 1.40 to 1.75).
const DEX_SCALING: f32 = 1.5;

/// One stage boundary of a scaling curve: stat value, curve value (percent), exponent towards the next stage.
type Stage = (f32, f32, f32);

/// Row 0 of your regulation.bin, used if the game's params can't be read.
const DEFAULT_CURVE: [Stage; 5] = [(1.0, 0.0, 1.2), (18.0, 25.0, -1.2), (60.0, 75.0, 1.0), (80.0, 90.0, 1.0), (150.0, 110.0, 1.0)];

/// The game's scaling formula: within a stage, the share of the way to the next one, raised to the exponent (a
/// negative exponent bends the other way).
fn curve(stages: &[Stage; 5], stat: f32) -> f32 {
    if stat <= stages[0].0 {
        return stages[0].1;
    }
    for pair in stages.windows(2) {
        let ((x0, y0, e), (x1, y1, _)) = (pair[0], pair[1]);
        if stat <= x1 {
            let t = ((stat - x0) / (x1 - x0)).clamp(0.0, 1.0);
            let shaped = if e >= 0.0 { t.powf(e) } else { 1.0 - (1.0 - t).powf(-e) };
            return y0 + (y1 - y0) * shaped;
        }
    }
    stages[4].1
}

fn game_curve() -> [Stage; 5] {
    let Ok(params) = (unsafe { SoloParamRepository::instance() }) else {
        return DEFAULT_CURVE;
    };
    let Some(row) = params.get::<CalcCorrectGraph>(0) else {
        return DEFAULT_CURVE;
    };
    [
        (row.stage_max_val0(), row.stage_max_grow_val0(), row.adj_pt_max_grow_val0()),
        (row.stage_max_val1(), row.stage_max_grow_val1(), row.adj_pt_max_grow_val1()),
        (row.stage_max_val2(), row.stage_max_grow_val2(), row.adj_pt_max_grow_val2()),
        (row.stage_max_val3(), row.stage_max_grow_val3(), row.adj_pt_max_grow_val3()),
        (row.stage_max_val4(), row.stage_max_grow_val4(), row.adj_pt_max_grow_val4()),
    ]
}

static LAST: std::sync::Mutex<Option<u32>> = std::sync::Mutex::new(None);

/// Damage multiplier for the player's current dexterity (after buffs).
pub fn dex_multiplier(player: &PlayerIns) -> f32 {
    let dex = unsafe { player.player_game_data.as_ref() }.effective_dexterity;
    let percent = curve(&game_curve(), dex as f32);
    let multiplier = 1.0 + DEX_SCALING * percent / 100.0;
    let mut last = LAST.lock().unwrap_or_else(|e| e.into_inner());
    if *last != Some(dex) {
        *last = Some(dex);
        glog!(
            "SCALING",
            "dexterity {dex}: curve {percent:.1}%, damage x{multiplier:.2} (shuriken {:.0}/{:.0}, Swift Strike {:.0})",
            27.0 * multiplier,
            54.0 * multiplier,
            50.0 * multiplier
        );
    }
    multiplier
}

/// Base damage scaled by dexterity, rounded.
pub fn scaled(base: f32, player: &PlayerIns) -> i32 {
    (base * dex_multiplier(player)).round() as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn soft_caps_match_the_game() {
        assert_eq!(curve(&DEFAULT_CURVE, 1.0), 0.0);
        assert!((curve(&DEFAULT_CURVE, 18.0) - 25.0).abs() < 1e-3);
        assert!((curve(&DEFAULT_CURVE, 60.0) - 75.0).abs() < 1e-3);
        assert!((curve(&DEFAULT_CURVE, 80.0) - 90.0).abs() < 1e-3);
        assert!((curve(&DEFAULT_CURVE, 99.0) - 95.43).abs() < 0.01);
    }

    #[test]
    fn curve_bends_like_the_game() {
        // Below 18 it starts slow (exponent 1.2), from 18 to 60 it starts fast (exponent -1.2).
        let mid_low = curve(&DEFAULT_CURVE, 9.5);
        assert!(mid_low < 12.5 && mid_low > 10.0, "{mid_low}");
        let mid_high = curve(&DEFAULT_CURVE, 39.0);
        assert!(mid_high > 50.0, "{mid_high}");
    }
}
