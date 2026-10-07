//! The game's gameplay input reader (`CSInGamePad`): mouse buttons as the game read them, the game's own
//! "is Attack pressed?" answer, and switching Elden Ring's weapon actions off while Genji mode captures input.
//!
//! How the switching works: each action (Attack, Guard, ...) maps to "mapped inputs", and each mapped input has
//! an enable check (`input_code_check`: usable when `state_1 && !state_2`). We set `state_2` on the mapped inputs
//! of the weapon actions, and restore the saved values on release. Only the gameplay reader is touched, so
//! menus can't be affected.
//!
//! WHEN it runs matters. The game resets these flags every frame in `PadStep` (when it reads the devices) and
//! decides the player's actions later in the frame. The first version (2026-10-07) wrote the flags at
//! `ChrIns_PostPhysics`, the end of the frame, so the game had already reset and checked them: clicks still
//! attacked. Now the blocker runs in `GameFlowStep`, the stage right after `PadStep`.
//!
//! Also tested 2026-10-07: the gameplay reader stays enabled and polling while menus are open, so it can't
//! detect menus (see the HUD rule in `controls.rs`).

use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

use eldenring::cs::{CSInGamePad, UserInputKey};
use eldenring::fd4::FD4PadManager;
use fromsoftware_shared::FromStatic;

use crate::glog;

/// Elden Ring actions switched off while Genji mode captures input: all weapon use.
const BLOCKED_ACTIONS: [UserInputKey; 4] =
    [UserInputKey::Attack, UserInputKey::StrongAttack, UserInputKey::Guard, UserInputKey::Skill];

static WEAPONS_BLOCKED: AtomicBool = AtomicBool::new(false);
static BLOCKER: Mutex<Option<ActionBlocker>> = Mutex::new(None);

/// Ask for Elden Ring's weapon actions to be off (true) or on (false). Applied by `weapon_block_step`.
pub fn set_weapons_blocked(on: bool) {
    WEAPONS_BLOCKED.store(on, Ordering::Relaxed);
}

/// Runs every frame in the `GameFlowStep` task group: after the game has read input, before it uses it.
pub fn weapon_block_step() {
    let mut guard = BLOCKER.lock().unwrap_or_else(|e| e.into_inner());
    let blocker = guard.get_or_insert_with(ActionBlocker::default);
    if WEAPONS_BLOCKED.load(Ordering::Relaxed) {
        let first = !blocker.active;
        let count = blocker.block();
        if first {
            glog!("PAD", "weapon actions blocked in GameFlowStep: {count} mapped inputs (original flags {})", blocker.describe());
        }
    } else if blocker.active {
        blocker.release();
        glog!("PAD", "weapon actions released");
    }
}

/// Raw mouse buttons as the game itself read them (bit i = DirectInput button i).
pub fn mouse_buttons() -> u8 {
    let Some(pad) = in_game_pad() else { return 0 };
    let device = unsafe { pad.pad_device.as_ref() };
    let mouse = unsafe { device.mouse_device.as_ref() };
    mouse
        .di_mouse_state
        .buttons
        .iter()
        .enumerate()
        .fold(0u8, |acc, (i, b)| if b & 0x80 != 0 { acc | (1 << i) } else { acc })
}

/// What the gameplay pad currently answers for "is Attack pressed?" (the library's copy of the game's check).
pub fn attack_polled() -> Option<bool> {
    Some(in_game_pad()?.poll_digital_input(UserInputKey::Attack))
}

fn in_game_pad() -> Option<&'static CSInGamePad> {
    let manager = unsafe { FD4PadManager::instance() }.ok()?;
    manager.get_in_game_pad()
}

fn in_game_pad_mut() -> Option<&'static mut CSInGamePad> {
    let manager = unsafe { FD4PadManager::instance_mut() }.ok()?;
    manager.get_in_game_pad_mut()
}

/// Switches Elden Ring actions off and on in the gameplay pad, remembering the original enable flags.
#[derive(Default)]
struct ActionBlocker {
    saved: HashMap<i32, (bool, bool)>,
    active: bool,
}

impl ActionBlocker {
    /// Returns how many mapped inputs are blocked.
    fn block(&mut self) -> usize {
        let Some(pad) = in_game_pad_mut() else { return 0 };
        let mapped: Vec<i32> = {
            let groups = unsafe { pad.input_type_group.as_ref() };
            BLOCKED_ACTIONS
                .iter()
                .filter_map(|action| groups.find(action))
                .flat_map(|group| group.iter().map(|(input, _)| input).collect::<Vec<_>>())
                .collect()
        };
        let checks = unsafe { pad.input_code_check.as_mut() };
        for input in &mapped {
            if let Some(check) = checks.find_mut(input) {
                self.saved.entry(*input).or_insert((check.state_1, check.state_2));
                check.state_2 = true;
            }
        }
        self.active = true;
        mapped.len()
    }

    /// Puts every flag we changed back the way it was.
    fn release(&mut self) {
        if let Some(pad) = in_game_pad_mut() {
            let checks = unsafe { pad.input_code_check.as_mut() };
            for (input, (state_1, state_2)) in &self.saved {
                if let Some(check) = checks.find_mut(input) {
                    check.state_1 = *state_1;
                    check.state_2 = *state_2;
                }
            }
        }
        self.saved.clear();
        self.active = false;
    }

    /// The mapped inputs and their original flags, for the log.
    fn describe(&self) -> String {
        let mut items: Vec<_> = self.saved.iter().collect();
        items.sort();
        items.iter().map(|(k, (a, b))| format!("{k}:{}{}", *a as u8, *b as u8)).collect::<Vec<_>>().join(" ")
    }
}
