//! Eliminations (step 7): a character we damaged that dies within CREDIT_WINDOW counts as our kill. Swift Strike's
//! cooldown resets on one. The window is an assumption (Overwatch's exact rule not verified).
//!
//! Characters are identified by their address and only ever read through the game's live character list, so an
//! unloaded character is never touched.
//!
//! Diagnostic (test of 2026-10-08: a far-away kill didn't reset the dash): a death that isn't counted as ours within
//! JUDGE_AFTER seconds is logged with when we last damaged that character.

use std::collections::{HashMap, HashSet};
use std::sync::Mutex;

use eldenring::cs::WorldChrMan;
use fromsoftware_shared::FromStatic;

use crate::glog;

/// Seconds after our last damage during which a death counts as our elimination.
const CREDIT_WINDOW: f32 = 3.0;
/// A death is judged "not ours" only after this long (a one-shot shuriken kill is matched a frame after the death).
const JUDGE_AFTER: f32 = 0.6;
/// How long we remember damaging a character, for the diagnostic.
const REMEMBER: f32 = 60.0;

#[derive(Default)]
struct State {
    /// Character address -> seconds since we last damaged it (within CREDIT_WINDOW).
    damaged: HashMap<usize, f32>,
    /// Same, kept for REMEMBER seconds (diagnostic).
    remembered: HashMap<usize, f32>,
    /// Every listed character's HP last frame.
    last_hp: HashMap<usize, i32>,
    /// Deaths not (yet) counted as ours: seconds since.
    unjudged: HashMap<usize, f32>,
    /// Deaths counted as ours (so a late one isn't logged as not ours).
    credited: HashSet<usize>,
}

static STATE: Mutex<Option<State>> = Mutex::new(None);

/// Remember that we just damaged this character.
pub fn damaged(chr: usize) {
    let mut guard = STATE.lock().unwrap_or_else(|e| e.into_inner());
    let state = guard.get_or_insert_with(State::default);
    state.damaged.insert(chr, 0.0);
    state.remembered.insert(chr, 0.0);
}

/// Once per frame: returns how many characters we damaged died since the last call.
pub fn tick(dt: f32) -> u32 {
    let mut guard = STATE.lock().unwrap_or_else(|e| e.into_inner());
    let state = guard.get_or_insert_with(State::default);
    for age in state.damaged.values_mut().chain(state.remembered.values_mut()).chain(state.unjudged.values_mut()) {
        *age += dt;
    }
    let Ok(world) = (unsafe { WorldChrMan::instance() }) else {
        return 0;
    };
    let mut kills = 0;
    for entry in world.chr_inses_by_distance.iter() {
        let id = entry.chr_ins.as_ptr() as usize;
        let hp = unsafe { entry.chr_ins.as_ref() }.modules.data.hp;
        if hp <= 0 {
            if let Some(&age) = state.damaged.get(&id) {
                glog!("KILL", "elimination: character {id:#x} died {age:.2} s after our last damage");
                state.damaged.remove(&id);
                state.unjudged.remove(&id);
                state.credited.insert(id);
                kills += 1;
            } else if state.last_hp.get(&id).is_some_and(|&before| before > 0) && !state.credited.contains(&id) {
                state.unjudged.insert(id, 0.0);
            }
        } else {
            state.credited.remove(&id); // alive again (respawned)
        }
        state.last_hp.insert(id, hp);
    }
    let judged: Vec<usize> = state.unjudged.iter().filter(|(_, age)| **age >= JUDGE_AFTER).map(|(id, _)| *id).collect();
    for id in judged {
        state.unjudged.remove(&id);
        match state.remembered.get(&id) {
            Some(age) => glog!(
                "KILL",
                "death NOT counted as ours: character {id:#x}, we last damaged it {:.2} s before it died (window {CREDIT_WINDOW} s)",
                age - JUDGE_AFTER
            ),
            None => glog!("KILL", "death NOT counted as ours: character {id:#x}, not damaged by us in the last {REMEMBER} s"),
        }
    }
    state.damaged.retain(|_, age| *age <= CREDIT_WINDOW);
    state.remembered.retain(|_, age| *age <= REMEMBER);
    kills
}
