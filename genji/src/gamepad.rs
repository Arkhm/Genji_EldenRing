//! Access to the game's gameplay input reader (`CSInGamePad`).
//!
//! Elden Ring has separate input readers ("pads") for gameplay and for menus. The gameplay one turns keys and
//! mouse buttons into actions (Attack, Guard, Event Action, Jump, ...). Each action maps to "mapped inputs",
//! and each mapped input has an enable check (`input_code_check`: usable when `state_1 && !state_2`). We switch
//! actions off by setting `state_2` on their mapped inputs, and restore the saved values afterwards. Because
//! only the gameplay pad is touched, menus can never be affected.

use std::collections::HashMap;

use eldenring::cs::{CSInGamePad, CSMenuViewerPad, CSPad, UserInputKey};
use eldenring::fd4::FD4PadManager;
use fromsoftware_shared::{FromStatic, Superclass};

/// Elden Ring actions switched off while Genji mode captures input: all weapon use.
pub const BLOCKED_ACTIONS: [UserInputKey; 4] =
    [UserInputKey::Attack, UserInputKey::StrongAttack, UserInputKey::Guard, UserInputKey::Skill];

/// The signals that could mean "gameplay, no menu". Logged on every change to learn which one is reliable.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct PadSignals {
    pub in_game_found: bool,
    pub in_game_allow_polling: bool,
    pub in_game_enable_use: bool,
    pub menu_allow_polling: bool,
    pub menu_enable_use: bool,
}

impl PadSignals {
    /// Current rule for "gameplay": the gameplay pad is enabled and allowed to poll.
    pub fn gameplay(&self) -> bool {
        self.in_game_found && self.in_game_allow_polling && self.in_game_enable_use
    }
}

pub fn signals() -> PadSignals {
    let Ok(manager) = (unsafe { FD4PadManager::instance() }) else {
        return PadSignals::default();
    };
    let mut s = PadSignals::default();
    for map_ptr in manager.pad_entry_map_list.iter() {
        let map = unsafe { map_ptr.as_ref() };
        for pair in map.iter() {
            let entry = &pair.second;
            let pad: &CSPad = unsafe { entry.entry.as_ref() };
            if pad.as_subclass::<CSInGamePad>().is_some() {
                s.in_game_found = true;
                s.in_game_allow_polling = pad.allow_polling;
                s.in_game_enable_use = entry.enable_use;
            } else if pad.as_subclass::<CSMenuViewerPad>().is_some() {
                s.menu_allow_polling = pad.allow_polling;
                s.menu_enable_use = entry.enable_use;
            }
        }
    }
    s
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
