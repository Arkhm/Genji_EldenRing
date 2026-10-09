//! Step 11b-2 (DESIGN.md 5j): when movement, health and kills play their sounds and voice lines. Pure logic, no
//! game access: each call returns the cues for this frame and `camera.rs` plays them, so the rules are tested here.
//! All rules are guesses at Overwatch's unless noted.

use std::collections::VecDeque;

use crate::glog;
use crate::movement::MoveEvents;

/// One footstep per this much running on the ground, metres. Your recording (2026-10-09): a step about every 0.235 s
/// at full speed (5.5 m/s).
const STRIDE: f32 = 1.3;
/// A short grab sound this often while climbing a wall, seconds.
const CLIMB_GRAB_EVERY: f32 = 0.2;
/// The jump grunt at most this often, seconds.
const JUMP_VOICE_EVERY: f32 = 1.5;
/// Landing faster than this (m/s) makes the landing sound; louder up to LAND_LOUD.
const LAND_QUIET: f32 = 2.0;
const LAND_LOUD: f32 = 12.0;
/// A fall from this high (m) also makes the landing grunt.
const LAND_VOICE_FALL: f32 = 3.0;
/// A pain line at most this often, seconds.
const PAIN_EVERY: f32 = 3.0;
/// After an HP drop, wait this long for a hit reaction (the game taking over) before choosing pain or stagger.
const REACTION_WAIT: f32 = 0.15;
/// Still in the hit reaction after this long: knocked down.
const KNOCKDOWN_AFTER: f32 = 0.9;
/// Critical health below this share of max HP; re-armed above LOW_HEALTH_REARM.
const LOW_HEALTH: f32 = 0.30;
const LOW_HEALTH_REARM: f32 = 0.40;
/// Healed this share of max HP or more at once (not by lifesteal): a flask.
const FLASK_HEAL: f32 = 0.15;
/// In the air this long before dying: the scream instead of the death line.
const FALL_DEATH_AIR: f32 = 1.0;
/// Multikill: this many kills within MULTIKILL_WINDOW seconds.
const MULTIKILL: usize = 3;
const MULTIKILL_WINDOW: f32 = 4.0;
/// A kill-streak line every this many kills without dying.
const STREAK_EVERY: u32 = 10;
/// The plain elimination line on every this-many-th kill (Overwatch doesn't comment on every one).
const ELIMINATION_LINE_EVERY: u32 = 3;

/// A sound to play now.
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Cue {
    Play(&'static str, f32),
    Voice(&'static str, bool),
}

#[derive(Default)]
pub struct Cues {
    clock: f32,
    footstep_travel: f32,
    climb_grab_wait: f32,
    jump_voice_at: Option<f32>,
    pain_at: Option<f32>,
    last_hp: Option<(i32, i32)>,
    low_said: bool,
    /// An HP drop waiting to be called pain or stagger: seconds since.
    hit_pending: Option<f32>,
    /// In a hit reaction: seconds so far, and whether the knockdown played.
    reaction: Option<(f32, bool)>,
    dead: bool,
    respawn_pending: bool,
    kill_times: VecDeque<f32>,
    multikill_said: bool,
    streak: u32,
    kills: u32,
}

impl Cues {
    /// Movement sounds for this frame (only while our controller drives).
    pub fn movement(&mut self, ev: &MoveEvents, climbing: bool, dt: f32) -> Vec<Cue> {
        let mut out = Vec::new();
        self.footstep_travel += ev.ground_travel;
        if self.footstep_travel >= STRIDE {
            self.footstep_travel -= STRIDE;
            out.push(Cue::Play("footstep", 1.0));
        }
        if ev.ground_travel == 0.0 && ev.landed.is_none() {
            // Standing or in the air: the next step comes a stride after moving again.
            self.footstep_travel = self.footstep_travel.min(STRIDE / 2.0);
        }
        if ev.jumped || ev.double_jumped {
            if ev.double_jumped {
                out.push(Cue::Play("double_jump", 1.0));
            }
            if self.jump_voice_at.is_none_or(|t| self.clock - t >= JUMP_VOICE_EVERY) {
                self.jump_voice_at = Some(self.clock);
                out.push(Cue::Voice("voice_jump", false));
            }
        }
        if ev.climb_started {
            out.push(Cue::Play("wall_climb", 1.0));
            self.climb_grab_wait = CLIMB_GRAB_EVERY;
        } else if climbing {
            self.climb_grab_wait -= dt;
            if self.climb_grab_wait <= 0.0 {
                self.climb_grab_wait += CLIMB_GRAB_EVERY;
                out.push(Cue::Play("wall_climb_loop", 1.0));
            }
        }
        if ev.vaulted {
            out.push(Cue::Play("wall_climb_ledge", 1.0));
        }
        if let Some((speed, fallen)) = ev.landed {
            if speed > LAND_QUIET {
                let gain = (0.3 + 0.7 * (speed - LAND_QUIET) / (LAND_LOUD - LAND_QUIET)).clamp(0.3, 1.0);
                out.push(Cue::Play("land", gain));
            }
            if fallen >= LAND_VOICE_FALL {
                out.push(Cue::Voice("voice_jump_end", false));
            }
            self.footstep_travel = 0.0;
        }
        out
    }

    /// Health sounds for this frame. `lifesteal`: HP Dragonblade gave back this frame (not a flask). `reacting`: the
    /// game played its own animation last frame (a hit reaction after a drop). `airborne_for`: seconds in the air.
    /// `genji`: Genji mode is on (otherwise only the bookkeeping runs). `in_control`: our controller drives.
    #[allow(clippy::too_many_arguments)]
    pub fn health(
        &mut self,
        hp: i32,
        max_hp: i32,
        lifesteal: i32,
        reacting: bool,
        airborne_for: f32,
        genji: bool,
        in_control: bool,
        dt: f32,
    ) -> Vec<Cue> {
        self.clock += dt;
        let mut out = Vec::new();
        let max = max_hp.max(1) as f32;
        let last = self.last_hp.replace((hp, max_hp));

        if hp <= 0 {
            if !self.dead {
                self.dead = true;
                self.respawn_pending = genji;
                self.streak = 0;
                self.hit_pending = None;
                self.reaction = None;
                if genji {
                    let falling = airborne_for >= FALL_DEATH_AIR;
                    glog!("SOUND", "death{}", if falling { " after a fall: scream" } else { "" });
                    out.push(Cue::Voice(if falling { "voice_fall_death" } else { "voice_death" }, true));
                }
            }
            return out;
        }
        if self.dead {
            self.dead = false;
            self.low_said = false;
        }
        if self.respawn_pending && in_control {
            self.respawn_pending = false;
            if genji {
                out.push(Cue::Voice("voice_respawn", false));
            }
        }
        let Some((last_hp, last_max)) = last else {
            return out;
        };
        if last_max != max_hp || last_hp <= 0 {
            return out; // level up, or back from death: not a hit or a heal
        }

        // Hits: wait a moment to see whether the game plays a hit reaction.
        if hp < last_hp {
            self.hit_pending.get_or_insert(0.0);
        }
        if let Some(waited) = self.hit_pending.as_mut() {
            *waited += dt;
            if reacting && self.reaction.is_none() {
                self.hit_pending = None;
                self.reaction = Some((0.0, false));
                if genji {
                    out.push(Cue::Voice("voice_stagger", true));
                }
                self.pain_at = Some(self.clock);
            } else if *waited >= REACTION_WAIT {
                self.hit_pending = None;
                if genji && self.pain_at.is_none_or(|t| self.clock - t >= PAIN_EVERY) {
                    self.pain_at = Some(self.clock);
                    out.push(Cue::Voice("voice_pain", false));
                }
            }
        }
        if let Some((time, knocked)) = self.reaction.as_mut() {
            if reacting {
                *time += dt;
                if *time >= KNOCKDOWN_AFTER && !*knocked {
                    *knocked = true;
                    if genji {
                        out.push(Cue::Play("knockdown", 1.0));
                        out.push(Cue::Voice("voice_knockdown", false));
                    }
                }
            } else {
                self.reaction = None;
            }
        }

        // Critical health.
        let share = hp as f32 / max;
        if share < LOW_HEALTH && !self.low_said {
            self.low_said = true;
            if genji {
                out.push(Cue::Play("low_health", 1.0));
            }
        } else if share > LOW_HEALTH_REARM {
            self.low_said = false;
        }

        // A flask (a big heal at once that isn't Dragonblade's lifesteal).
        let healed = hp - last_hp - lifesteal.max(0);
        if healed as f32 >= FLASK_HEAL * max && genji {
            out.push(Cue::Play("health_pack", 1.0));
            out.push(Cue::Voice("voice_healing", false));
        }
        out
    }

    /// The elimination sound and, sometimes, a line: a multikill first, then a melee kill, a kill streak, and the
    /// plain elimination line every third kill. `melee`: a quick melee hit a character within the last second.
    pub fn kills(&mut self, count: u32, melee: bool) -> Vec<Cue> {
        if count == 0 {
            return Vec::new();
        }
        let mut out = vec![Cue::Play("kill", 1.0)];
        for _ in 0..count {
            self.kill_times.push_back(self.clock);
        }
        while self.kill_times.front().is_some_and(|t| self.clock - t > MULTIKILL_WINDOW) {
            self.kill_times.pop_front();
        }
        if self.kill_times.len() < 2 {
            self.multikill_said = false;
        }
        let before = self.kills;
        self.kills += count;
        self.streak += count;
        let line = if self.kill_times.len() >= MULTIKILL && !self.multikill_said {
            self.multikill_said = true;
            Some(("voice_multikill", true))
        } else if melee {
            Some(("voice_melee_kill", false))
        } else if self.streak / STREAK_EVERY > (self.streak - count) / STREAK_EVERY {
            Some(("voice_killstreak", true))
        } else if (before..self.kills).any(|k| k % ELIMINATION_LINE_EVERY == 0) {
            Some(("voice_elimination", false))
        } else {
            None
        };
        if let Some((event, urgent)) = line {
            glog!("SOUND", "kill {}: {event}", self.kills);
            out.push(Cue::Voice(event, urgent));
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DT: f32 = 1.0 / 60.0;

    fn voices(cues: &[Cue]) -> Vec<&'static str> {
        cues.iter().filter_map(|c| if let Cue::Voice(e, _) = c { Some(*e) } else { None }).collect()
    }

    #[test]
    fn footstep_every_stride() {
        let mut c = Cues::default();
        let run = MoveEvents { ground_travel: 5.5 * DT, ..Default::default() };
        let steps: usize = (0..60).map(|_| c.movement(&run, false, DT).len()).sum();
        assert_eq!(steps, (5.5 / STRIDE) as usize);
    }

    #[test]
    fn pain_is_rate_limited_and_a_reaction_staggers() {
        let mut c = Cues::default();
        let mut hp = 1000;
        let mut all = Vec::new();
        c.health(hp, 1000, 0, false, 0.0, true, true, DT);
        // A hit every half second for 4 s, no reaction: pain at 0 s and 3 s.
        for frame in 0..240 {
            if frame % 30 == 0 {
                hp -= 10;
            }
            all.extend(c.health(hp, 1000, 0, false, 0.0, true, true, DT));
        }
        assert_eq!(voices(&all), ["voice_pain", "voice_pain"]);
        // A hit the game reacts to, lasting a second: stagger, then knockdown.
        let mut all = Vec::new();
        hp -= 10;
        all.extend(c.health(hp, 1000, 0, false, 0.0, true, true, DT));
        for _ in 0..60 {
            all.extend(c.health(hp, 1000, 0, true, 0.0, true, true, DT));
        }
        assert_eq!(voices(&all), ["voice_stagger", "voice_knockdown"]);
        assert!(all.contains(&Cue::Play("knockdown", 1.0)));
    }

    #[test]
    fn death_respawn_low_health_and_flask() {
        let mut c = Cues::default();
        c.health(1000, 1000, 0, false, 0.0, true, true, DT);
        let low = c.health(200, 1000, 0, false, 0.0, true, true, DT);
        assert!(low.contains(&Cue::Play("low_health", 1.0)));
        assert!(c.health(190, 1000, 0, false, 0.0, true, true, DT).iter().all(|q| *q != Cue::Play("low_health", 1.0)));
        // Lifesteal doesn't count as a flask; a flask does.
        assert!(voices(&c.health(400, 1000, 210, false, 0.0, true, true, DT)).is_empty());
        assert_eq!(voices(&c.health(700, 1000, 0, false, 0.0, true, true, DT)), ["voice_healing"]);
        // Death after a long fall screams; the respawn line waits for control.
        assert_eq!(voices(&c.health(0, 1000, 0, false, 2.0, true, false, DT)), ["voice_fall_death"]);
        assert!(voices(&c.health(1000, 1000, 0, false, 0.0, true, false, DT)).is_empty());
        assert_eq!(voices(&c.health(1000, 1000, 0, false, 0.0, true, true, DT)), ["voice_respawn"]);
    }

    #[test]
    fn kill_lines() {
        let mut c = Cues::default();
        let mut tick = |c: &mut Cues, seconds: f32| {
            for _ in 0..(seconds / DT) as u32 {
                c.health(1000, 1000, 0, false, 0.0, true, true, DT);
            }
        };
        tick(&mut c, 1.0);
        // First kill: the plain line (every third: kills 1, 4, 7...).
        assert_eq!(voices(&c.kills(1, false)), ["voice_elimination"]);
        tick(&mut c, 1.0);
        assert!(voices(&c.kills(1, false)).is_empty());
        tick(&mut c, 1.0);
        // Third kill within 4 s: multikill.
        assert_eq!(voices(&c.kills(1, false)), ["voice_multikill"]);
        tick(&mut c, 5.0);
        assert_eq!(voices(&c.kills(1, true)), ["voice_melee_kill"]);
        for _ in 0..5 {
            tick(&mut c, 5.0);
            c.kills(1, false);
        }
        tick(&mut c, 5.0);
        // Tenth kill: the streak line.
        assert_eq!(voices(&c.kills(1, false)), ["voice_killstreak"]);
    }
}
