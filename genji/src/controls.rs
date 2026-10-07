//! Genji's controls, read once per frame on the game's thread.
//!
//! - Turns raw key and button changes into Genji actions (logged as `ACTION`).
//! - F9 switches Genji mode on and off.
//! - "Captures" Genji's input while Genji mode is on, the player exists and the game's gameplay input reader is
//!   active (it isn't while a menu is open). While captured:
//!   - the keyboard hook hides E, Q, Left Shift and F from the game, and passes F on as E (interact);
//!   - Elden Ring's weapon actions (Attack, Strong Attack, Guard, Skill) are switched off in the gameplay input
//!     reader, which covers the mouse buttons.
//!
//! First try (2026-10-07) decided "menu open" from the game's UI-state table; it treated pop-ups and prompts
//! as menus and released the keys every few seconds. UI-state changes are still logged, to map them later.

use eldenring::cs::CSMenuManImp;
use fromsoftware_shared::FromStatic;

use crate::dinput::{self, dik};
use crate::gamepad::{self, ActionBlocker, PadSignals};
use crate::glog;

/// DirectInput mouse button indexes.
mod mouse {
    pub const LEFT: u8 = 0;
    pub const RIGHT: u8 = 1;
    pub const BUTTON5: u8 = 4;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Shuriken,
    FanOfBlades,
    SwiftStrike,
    Deflect,
    Dragonblade,
    Melee,
    Jump,
    Interact,
}

enum Input {
    Key(usize),
    Mouse(u8),
}

/// Overwatch layout, melee on mouse button 5. Interact (F) is passed on to the game as E.
const BINDINGS: [(Action, Input); 8] = [
    (Action::Shuriken, Input::Mouse(mouse::LEFT)),
    (Action::FanOfBlades, Input::Mouse(mouse::RIGHT)),
    (Action::SwiftStrike, Input::Key(dik::LSHIFT)),
    (Action::Deflect, Input::Key(dik::E)),
    (Action::Dragonblade, Input::Key(dik::Q)),
    (Action::Melee, Input::Mouse(mouse::BUTTON5)),
    (Action::Jump, Input::Key(dik::SPACE)),
    (Action::Interact, Input::Key(dik::F)),
];

const TOGGLE_KEY: usize = dik::F9;

pub struct Controls {
    enabled: bool,
    capturing: bool,
    held: [Option<f32>; BINDINGS.len()],
    toggle_was_down: bool,
    last_signals: Option<PadSignals>,
    last_ui: Option<u128>,
    blocker: ActionBlocker,
    block_logged: bool,
    clock: f32,
    report_timer: f32,
}

impl Default for Controls {
    fn default() -> Self {
        Self {
            enabled: true,
            capturing: false,
            held: [None; BINDINGS.len()],
            toggle_was_down: false,
            last_signals: None,
            last_ui: None,
            blocker: ActionBlocker::default(),
            block_logged: false,
            clock: 0.0,
            report_timer: 0.0,
        }
    }
}

impl Controls {
    pub fn tick(&mut self, dt: f32, player_present: bool) {
        self.clock += dt;
        let keys = dinput::raw_keys();
        let buttons = gamepad::mouse_buttons();

        // F9: Genji mode on/off.
        let toggle_down = dinput::key_down(&keys, TOGGLE_KEY);
        if toggle_down && !self.toggle_was_down {
            self.enabled = !self.enabled;
            glog!("MODE", "Genji mode {}", if self.enabled { "ON" } else { "OFF (plain Elden Ring)" });
        }
        self.toggle_was_down = toggle_down;

        // Is the game in gameplay (gameplay input reader active)?
        let signals = gamepad::signals();
        if Some(signals) != self.last_signals {
            glog!(
                "PAD",
                "in-game pad found {} polling {} enabled {} | menu pad polling {} enabled {}",
                signals.in_game_found as u8,
                signals.in_game_allow_polling as u8,
                signals.in_game_enable_use as u8,
                signals.menu_allow_polling as u8,
                signals.menu_enable_use as u8
            );
            self.last_signals = Some(signals);
        }
        let ui = visible_ui_states();
        if ui != self.last_ui {
            log_ui_change(self.last_ui, ui);
            self.last_ui = ui;
        }

        let capturing = self.enabled && player_present && signals.gameplay();
        if capturing != self.capturing {
            self.capturing = capturing;
            dinput::set_blocking(capturing);
            let why = if !self.enabled {
                "Genji mode off"
            } else if !player_present {
                "no player"
            } else if !signals.gameplay() {
                "gameplay input inactive (menu?)"
            } else {
                "gameplay"
            };
            glog!("MODE", "Genji input {} ({why})", if capturing { "CAPTURED" } else { "released to the game" });
        }
        if capturing {
            let count = self.blocker.block();
            if !self.block_logged {
                self.block_logged = true;
                glog!("PAD", "weapon actions blocked: {count} mapped inputs (original flags {})", self.blocker.describe());
            }
        } else {
            self.blocker.release();
            self.block_logged = false;
        }

        // Actions: only while capturing, so menus and plain Elden Ring never trigger Genji.
        for (i, (action, input)) in BINDINGS.iter().enumerate() {
            let down = match input {
                Input::Key(code) => dinput::key_down(&keys, *code),
                Input::Mouse(button) => buttons & (1 << button) != 0,
            };
            match (down && self.capturing, self.held[i]) {
                (true, None) => {
                    self.held[i] = Some(self.clock);
                    glog!("ACTION", "{action:?} down");
                }
                (false, Some(since)) => {
                    self.held[i] = None;
                    glog!("ACTION", "{action:?} up after {:.2}s", self.clock - since);
                }
                _ => {}
            }
        }
    }

    /// Once per second.
    pub fn report(&mut self) {
        let keyboard = dinput::take_read_count();
        self.report_timer += 1.0;
        if self.report_timer >= 10.0 {
            self.report_timer = 0.0;
            glog!(
                "DINPUT",
                "keyboard reads in the last second: {keyboard}; weapon flags re-enabled by the game since start: {}",
                self.blocker.reenabled_by_game
            );
        }
    }
}

/// Bit i set = UI state i is visible. None if the menu manager isn't available.
fn visible_ui_states() -> Option<u128> {
    let menu = unsafe { CSMenuManImp::instance() }.ok()?;
    Some(
        menu.ui_states
            .iter()
            .enumerate()
            .filter(|(_, s)| s.visible())
            .fold(0u128, |acc, (i, _)| acc | (1 << i)),
    )
}

fn log_ui_change(before: Option<u128>, after: Option<u128>) {
    match (before, after) {
        (Some(b), Some(a)) => glog!("UI", "visible {} (shown {}, hidden {})", fmt_set(a), fmt_set(a & !b), fmt_set(b & !a)),
        (_, Some(a)) => glog!("UI", "menu manager ready, visible {}", fmt_set(a)),
        (_, None) => glog!("UI", "menu manager not available"),
    }
}

fn fmt_set(bits: u128) -> String {
    let items: Vec<String> = (0..128).filter(|i| bits & (1 << i) != 0).map(|i| format!("{i:#04x}")).collect();
    format!("[{}]", items.join(" "))
}
