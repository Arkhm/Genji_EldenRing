//! Read-only access to the game's gameplay input reader (`CSInGamePad`): the mouse buttons as the game read
//! them, and the game's own "is Attack pressed?" answer (used to check the Genji save's key bindings).
//!
//! Dead ends, 2026-10-07 (details in MODLOG.md and STATUS.md):
//! - Blocking Attack/Guard/Skill by setting the reader's `input_code_check` flags every frame failed: the game
//!   rewrites them every frame and checks in between. Instead, Elden Ring's weapon actions are unbound from
//!   the mouse in the Genji save's own key settings.
//! - The reader stays enabled and polling while menus are open, so it can't detect menus (HUD rule instead).

use eldenring::cs::{CSInGamePad, UserInputKey};
use eldenring::fd4::FD4PadManager;
use fromsoftware_shared::FromStatic;


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

