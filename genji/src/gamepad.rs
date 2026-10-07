//! Access to the game's gameplay input reader (`CSInGamePad`).
//!
//! Elden Ring has separate input readers ("pads") for gameplay and for menus. The gameplay one turns keys and
//! mouse buttons into actions (Attack, Guard, Event Action, Jump, ...). Each action maps to "mapped inputs",
//! and each mapped input has an enable check (`input_code_check`: usable when `state_1 && !state_2`). We switch
//! actions off by setting `state_2` on their mapped inputs, and restore the saved values afterwards. Because
//! only the gameplay pad is touched, menus can never be affected.
//!
//! Tested 2026-10-07: this blocks attacks, but the game resets the flags every frame (about 240 resets per
//! second for 4 inputs); it works because we set them again each frame before the game checks them. If
//! attacks ever leak through, this timing is the first suspect.
//!
//! Also tested: the gameplay pad stays enabled and polling while menus are open, so it can't tell us
//! whether a menu is open (see `controls.rs` for the HUD rule used instead).

use std::collections::HashMap;

use eldenring::cs::{CSInGamePad, UserInputKey};
use eldenring::fd4::FD4PadManager;
use fromsoftware_shared::FromStatic;

/// Elden Ring actions switched off while Genji mode captures input: all weapon use.
pub const BLOCKED_ACTIONS: [UserInputKey; 4] =
    [UserInputKey::Attack, UserInputKey::StrongAttack, UserInputKey::Guard, UserInputKey::Skill];

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
pub struct ActionBlocker {
    saved: HashMap<i32, (bool, bool)>,
    /// Frames where an entry we had blocked was found enabled again (the game rewrote it).
    pub reenabled_by_game: u32,
}

impl ActionBlocker {
    /// Call every frame while blocking. Returns how many mapped inputs are blocked.
    pub fn block(&mut self) -> usize {
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
                if !self.saved.contains_key(input) {
                    self.saved.insert(*input, (check.state_1, check.state_2));
                } else if !check.state_2 {
                    self.reenabled_by_game += 1;
                }
                check.state_2 = true;
            }
        }
        mapped.len()
    }

    /// Puts every flag we changed back the way it was.
    pub fn release(&mut self) {
        if self.saved.is_empty() {
            return;
        }
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
    }

    /// The mapped inputs and their original flags, for the log.
    pub fn describe(&self) -> String {
        let mut items: Vec<_> = self.saved.iter().collect();
        items.sort();
        items.iter().map(|(k, (a, b))| format!("{k}:{}{}", *a as u8, *b as u8)).collect::<Vec<_>>().join(" ")
    }
}
