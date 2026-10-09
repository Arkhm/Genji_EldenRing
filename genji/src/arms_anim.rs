//! Step 10d (DESIGN.md 5k): which of Genji's first-person animations to show, frame by frame. Pure logic: camera.rs
//! tells it the stance, whether we're walking, and this frame's actions; it answers with the clip (and the clip it
//! blends from), and arms.rs draws that. The roles are those of tools\viewmodel_anims.py.
//!
//! - A base for each stance (normal, Dragonblade, Deflect): the resting pose, or the walk loop while running on the
//!   ground.
//! - Actions play once from the start (a new one cuts in), then the base takes over again.
//! - Every change blends from the old pose over FADE seconds, so nothing pops.

/// Seconds a change of clip blends over: quick into an action, a little slower back to the base. Was 0.25 back: with
/// the first (squashing) blend you could see the arm twist on M1 (you, 2026-10-09).
pub const FADE: f32 = 0.08;
pub const FADE_BACK: f32 = 0.15;
/// Ground speed above which the walk loop plays, m/s.
const WALK_SPEED: f32 = 1.5;
/// Length assumed for a clip the export doesn't have (arms.rs reports lengths once the model is loaded).
const UNKNOWN_LENGTH: f32 = 0.3;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Stance {
    Normal,
    Blade,
    Deflect,
}

/// Things that happened this frame, in the order they should be applied (a later one cuts in over an earlier one).
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Event {
    Spawn,
    Jump,
    DoubleJump,
    ClimbStart,
    ClimbLedge,
    ClimbEnd,
    Throw,
    Fan,
    Reload,
    Melee,
    Dash,
    BladeDraw,
    BladeSwing,
    BladeEnd,
    DeflectStart,
    DeflectBlock,
    DeflectEnd,
}

/// One animation at a time: role, which of its variants, time in it, looping or held at the end.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Clip {
    pub role: &'static str,
    pub variant: usize,
    pub t: f32,
    pub looping: bool,
}

/// What to draw: `main`, blended from `from` (frozen where it was) with `mix` the share of `main` (0..1), and which
/// parts show (viewmodel.rs PART_*: the arms and the weapon in hand).
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Pose {
    pub main: Clip,
    pub from: Option<Clip>,
    pub mix: f32,
    pub parts: u8,
}

/// The weapon in hand for an animation (2026-10-09): the katana for everything Dragonblade, the wakizashi (short
/// blade) for Swift Strike, the quick melee and Deflect, the shuriken otherwise.
pub fn parts_for(role: &str) -> u8 {
    use crate::viewmodel::{PART_ARMS, PART_KATANA, PART_SHURIKEN, PART_WAKIZASHI};
    if role.starts_with("blade_") {
        PART_ARMS | PART_KATANA
    } else if role.contains("wall_climb") {
        PART_ARMS // open hands on the wall (your Overwatch clip 3 at 14.1 s); the shuriken floated otherwise
    } else if matches!(role, "throw" | "fan" | "reload") {
        PART_ARMS // the hand is empty while throwing and reloading (your clip 2, 13.2-15.9 s, 16.5 s, 24-25.5 s)
    } else if role.starts_with("deflect_") || role == "dash" || role == "melee" {
        PART_ARMS | PART_WAKIZASHI
    } else {
        PART_ARMS | PART_SHURIKEN
    }
}

#[derive(Default)]
pub struct Animator {
    /// The action playing (one-shot), if any.
    action: Option<Clip>,
    /// The base last shown (to notice changes) and the walk loop's own clock.
    base: Option<Clip>,
    walk_t: f32,
    from: Option<Clip>,
    mix: f32,
    /// The current blend's length (FADE or FADE_BACK).
    fade: f32,
    /// Turns for roles with variants that alternate (swings, fans, blocks).
    turns: std::collections::HashMap<&'static str, usize>,
}

impl Animator {
    /// One frame. `ground_speed`: m/s running on the ground (0 in the air); `lengths(role, variant)`: (seconds,
    /// number of variants) if exported.
    pub fn update(
        &mut self,
        dt: f32,
        stance: Stance,
        ground_speed: f32,
        events: &[Event],
        lengths: &dyn Fn(&str, usize) -> Option<(f32, usize)>,
    ) -> Pose {
        let before = self.current();
        let mut started = false;
        for &e in events {
            // Movement (jumps, the climb, the arms coming up) never cuts into a combat action (you, 2026-10-09: "when
            // you jump, sometimes you cancel the animation of throwing shurikens ... that can't be happening").
            if minor(e) && self.action.is_some_and(|a| !minor_role(a.role)) {
                continue;
            }
            // The Dragonblade draw plays whole; only Dragonblade ending cuts it (you, 2026-10-09: drawn in the air,
            // it didn't show).
            // Swings come only once the draw's second is over (blade.rs), so they may cut it: you, 2026-10-09, the
            // first swing right after the draw showed a second late.
            if e != Event::BladeEnd && e != Event::BladeSwing && self.action.is_some_and(|a| a.role == "blade_draw") {
                continue;
            }
            if let Some(role) = action_role(e, stance) {
                let variants = lengths(role, 0).map_or(1, |(_, n)| n.max(1));
                let turn = self.turns.entry(role).or_insert(0);
                let variant = *turn % variants;
                if variants > 1 && alternates(role) {
                    *turn += 1;
                }
                self.action = Some(Clip { role, variant, t: 0.0, looping: false });
                started = true;
                crate::glog!("ARMS", "play {role} (version {}) for {e:?}", variant + 1);
            }
        }
        // The action advances; when it's over, the base takes over.
        if let Some(a) = self.action.as_mut() {
            if !started {
                a.t += dt;
            }
            let length = lengths(a.role, a.variant).map_or(UNKNOWN_LENGTH, |(l, _)| l);
            if a.t >= length {
                self.action = None;
            }
        }
        let walking = ground_speed > WALK_SPEED;
        if walking {
            self.walk_t += dt * (ground_speed / 5.5).clamp(0.5, 1.5);
        }
        let base_role = match (stance, walking) {
            (Stance::Normal, false) => "idle",
            (Stance::Normal, true) => "walk",
            (Stance::Blade, false) => "blade_idle",
            (Stance::Blade, true) => "blade_walk",
            (Stance::Deflect, false) => "deflect_idle",
            (Stance::Deflect, true) => "deflect_walk",
        };
        let base = Clip { role: base_role, variant: 0, t: if walking { self.walk_t } else { 0.0 }, looping: walking };
        self.base = Some(base);
        let now = self.current().unwrap_or(base);
        // A different clip (or the same action started again): blend from where the old one was.
        if let Some(prev) = before {
            if prev.role != now.role || prev.variant != now.variant || started {
                self.from = Some(prev);
                self.mix = 0.0;
                self.fade = if self.action.is_some() { FADE } else { FADE_BACK };
            }
        }
        if self.from.is_some() {
            self.mix = (self.mix + dt / self.fade.max(1e-3)).min(1.0);
            if self.mix >= 1.0 {
                self.from = None;
            }
        }
        Pose { main: now, from: self.from, mix: if self.from.is_some() { self.mix } else { 1.0 }, parts: parts_for(now.role) }
    }

    fn current(&self) -> Option<Clip> {
        self.action.or(self.base)
    }
}

/// Events that only play when no combat action is playing.
fn minor(e: Event) -> bool {
    matches!(e, Event::Spawn | Event::Jump | Event::DoubleJump | Event::ClimbStart | Event::ClimbLedge | Event::ClimbEnd)
}

/// The roles those events play.
fn minor_role(role: &str) -> bool {
    matches!(
        role,
        "spawn" | "jump" | "double_jump" | "blade_jump" | "blade_double_jump" | "deflect_jump" | "wall_climb" | "wall_climb_ledge"
            | "wall_climb_end" | "blade_wall_climb" | "blade_wall_climb_end"
    )
}

/// Roles whose variants take turns (the others always play their first).
fn alternates(role: &str) -> bool {
    matches!(role, "blade_swing" | "fan" | "deflect_block" | "jump" | "double_jump" | "blade_jump" | "blade_double_jump" | "deflect_jump")
}

/// The one-shot animation for an event in a stance, if any.
fn action_role(e: Event, stance: Stance) -> Option<&'static str> {
    use Event::*;
    use Stance::*;
    Some(match (e, stance) {
        (Spawn, _) => "spawn",
        (Jump, Normal) => "jump",
        (Jump, Blade) => "blade_jump",
        (Jump, Deflect) => "deflect_jump",
        (DoubleJump, Normal) => "double_jump",
        (DoubleJump, Blade) => "blade_double_jump",
        (DoubleJump, Deflect) => "deflect_jump",
        (ClimbStart, Blade) => "blade_wall_climb",
        (ClimbStart, _) => "wall_climb",
        (ClimbLedge, _) => "wall_climb_ledge",
        (ClimbEnd, Blade) => "blade_wall_climb_end",
        (ClimbEnd, _) => "wall_climb_end",
        (Throw, _) => "throw",
        (Fan, _) => "fan",
        (Reload, _) => "reload",
        (Melee, _) => "melee",
        (Dash, Blade) => "blade_dash",
        (Dash, _) => "dash",
        (BladeDraw, _) => "blade_draw",
        (BladeSwing, _) => "blade_swing",
        (BladeEnd, _) => "blade_end",
        (DeflectStart, Blade) => "blade_deflect_start",
        (DeflectStart, _) => "deflect_start",
        (DeflectBlock, _) => "deflect_block",
        (DeflectEnd, Blade) => "blade_deflect_end",
        (DeflectEnd, _) => "deflect_end",
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const DT: f32 = 1.0 / 60.0;

    fn lengths(role: &str, _variant: usize) -> Option<(f32, usize)> {
        Some(match role {
            "throw" => (1.07, 1),
            "reload" => (1.5, 1),
            "blade_swing" => (1.0, 3),
            "fan" => (1.13, 2),
            _ => (0.5, 1),
        })
    }

    fn run(a: &mut Animator, seconds: f32, stance: Stance, speed: f32) -> Pose {
        let mut p = a.update(DT, stance, speed, &[], &lengths);
        for _ in 1..(seconds / DT).round() as usize {
            p = a.update(DT, stance, speed, &[], &lengths);
        }
        p
    }

    #[test]
    fn action_plays_then_base_returns_with_a_blend() {
        let mut a = Animator::default();
        run(&mut a, 0.5, Stance::Normal, 0.0);
        let p = a.update(DT, Stance::Normal, 0.0, &[Event::Throw], &lengths);
        assert_eq!(p.main.role, "throw");
        assert_eq!(p.main.t, 0.0);
        assert_eq!(p.from.map(|c| c.role), Some("idle"));
        let p = run(&mut a, 0.5, Stance::Normal, 0.0);
        assert_eq!(p.main.role, "throw");
        assert!(p.from.is_none() && p.mix == 1.0, "blend over after {FADE} s");
        let p = run(&mut a, 0.6, Stance::Normal, 0.0);
        assert_eq!(p.main.role, "idle");
        assert!(p.from.is_some(), "still blending back ({FADE_BACK} s)");
        assert_eq!(p.from.map(|c| c.role), Some("throw"));
    }

    #[test]
    fn walking_loops_and_stances_switch_base() {
        let mut a = Animator::default();
        let p = run(&mut a, 1.0, Stance::Normal, 5.5);
        assert_eq!(p.main.role, "walk");
        assert!(p.main.looping && p.main.t > 0.9);
        let p = run(&mut a, 0.5, Stance::Blade, 0.0);
        assert_eq!(p.main.role, "blade_idle");
        let p = run(&mut a, 0.5, Stance::Deflect, 5.5);
        assert_eq!(p.main.role, "deflect_walk");
    }

    #[test]
    fn swings_take_turns_and_a_new_action_cuts_in() {
        let mut a = Animator::default();
        let mut seen = Vec::new();
        for _ in 0..4 {
            let p = a.update(DT, Stance::Blade, 0.0, &[Event::BladeSwing], &lengths);
            seen.push(p.main.variant);
            run(&mut a, 0.2, Stance::Blade, 0.0);
        }
        assert_eq!(seen, [0, 1, 2, 0]);
        let p = a.update(DT, Stance::Blade, 0.0, &[Event::Dash], &lengths);
        assert_eq!(p.main.role, "blade_dash");
        assert_eq!(p.from.map(|c| c.role), Some("blade_swing"));
    }

    #[test]
    fn a_jump_does_not_cut_a_throw_but_plays_after_it() {
        let mut a = Animator::default();
        a.update(DT, Stance::Normal, 0.0, &[Event::Throw], &lengths);
        run(&mut a, 0.2, Stance::Normal, 0.0);
        let p = a.update(DT, Stance::Normal, 0.0, &[Event::Jump], &lengths);
        assert_eq!(p.main.role, "throw");
        let p = a.update(DT, Stance::Normal, 0.0, &[Event::Melee], &lengths);
        assert_eq!(p.main.role, "melee", "a combat action still cuts in");
        run(&mut a, 1.0, Stance::Normal, 0.0);
        let p = a.update(DT, Stance::Normal, 0.0, &[Event::Jump], &lengths);
        assert_eq!(p.main.role, "jump");
    }

    #[test]
    fn same_action_again_restarts_with_a_blend() {
        let mut a = Animator::default();
        a.update(DT, Stance::Normal, 0.0, &[Event::Throw], &lengths);
        run(&mut a, 0.88, Stance::Normal, 0.0);
        let p = a.update(DT, Stance::Normal, 0.0, &[Event::Throw], &lengths);
        assert_eq!((p.main.role, p.main.t), ("throw", 0.0));
        assert_eq!(p.from.map(|c| c.role), Some("throw"));
        assert!(p.from.unwrap().t > 0.8);
    }
}
