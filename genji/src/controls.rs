//! Genji's controls, read once per frame on the game's thread.
//!
//! - Turns raw key and button changes into Genji actions (logged as `ACTION`).
//! - F9 switches Genji mode on and off.
//! - Decides when the game must not see Genji's keys ("blocking"): Genji mode on, the player exists, and
//!   no Elden Ring menu is open.
//!
//! Menu detection (provisional, to be checked against the logs): the game keeps a table of 70 UI states
//! (`CSMenuManImp.ui_states`). Two seconds after the player appears (after every loading screen), the set of
//! visible UI states is recorded as the "gameplay baseline". Any extra visible state counts as "a menu is
//! open". Every change of the visible set is logged so we can learn which number is which screen.

use eldenring::cs::CSMenuManImp;
use fromsoftware_shared::FromStatic;

use crate::dinput::{self, dik, mouse};
use crate::glog;

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
const BASELINE_DELAY: f32 = 2.0;

pub struct Controls {
    enabled: bool,
    blocking: bool,
    held: [Option<f32>; BINDINGS.len()],
    toggle_was_down: bool,
    present_for: f32,
    baseline: Option<u128>,
    last_visible: Option<u128>,
    clock: f32,
    read_report_timer: f32,
}

impl Default for Controls {
    fn default() -> Self {
        Self {
            enabled: true,
            blocking: false,
            held: [None; BINDINGS.len()],
            toggle_was_down: false,
            present_for: 0.0,
            baseline: None,
            last_visible: None,
            clock: 0.0,
            read_report_timer: 0.0,
        }
    }
}

impl Controls {
    pub fn tick(&mut self, dt: f32, player_present: bool) {
        self.clock += dt;
        let keys = dinput::raw_keys();
        let buttons = dinput::raw_mouse();

        // F9: Genji mode on/off.
        let toggle_down = dinput::key_down(&keys, TOGGLE_KEY);
        if toggle_down && !self.toggle_was_down {
            self.enabled = !self.enabled;
            glog!("MODE", "Genji mode {}", if self.enabled { "ON" } else { "OFF (plain Elden Ring)" });
        }
        self.toggle_was_down = toggle_down;

        // Menus.
        let visible = visible_ui_states();
        if visible != self.last_visible {
            log_ui_change(self.last_visible, visible);
            self.last_visible = visible;
        }
        if player_present {
            self.present_for += dt;
            if self.baseline.is_none() && self.present_for >= BASELINE_DELAY {
                self.baseline = visible;
                glog!("UI", "gameplay baseline {}", fmt_set(visible.unwrap_or(0)));
            }
        } else {
            self.present_for = 0.0;
            self.baseline = None;
        }
        let menu_open = match (self.baseline, visible) {
            (Some(base), Some(now)) => now & !base != 0,
            _ => true, // no baseline yet, or menu state unreadable: leave the keys to the game
        };

        let blocking = self.enabled && player_present && !menu_open;
        if blocking != self.blocking {
            self.blocking = blocking;
            dinput::set_blocking(blocking);
            let why = if !self.enabled {
                "Genji mode off"
            } else if !player_present {
                "no player"
            } else if menu_open {
                "menu open"
            } else {
                "gameplay"
            };
            glog!("MODE", "Genji keys {} ({why})", if blocking { "CAPTURED" } else { "passed to the game" });
        }

        // Actions: only while blocking, so menus and plain Elden Ring never trigger Genji.
        for (i, (action, input)) in BINDINGS.iter().enumerate() {
            let down = match input {
                Input::Key(code) => dinput::key_down(&keys, *code),
                Input::Mouse(button) => buttons & (1 << button) != 0,
            };
            match (down && self.blocking, self.held[i]) {
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

    /// Once per second: proves the DirectInput hook is being called.
    pub fn report(&mut self) {
        let (keyboard, mouse) = dinput::take_read_counts();
        self.read_report_timer += 1.0;
        // Every 10 s is enough once we know it works.
        if self.read_report_timer >= 10.0 {
            self.read_report_timer = 0.0;
            glog!("DINPUT", "reads in the last second: keyboard {keyboard}, mouse {mouse}");
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
        (Some(b), Some(a)) => glog!(
            "UI",
            "visible {} (shown {}, hidden {})",
            fmt_set(a),
            fmt_set(a & !b),
            fmt_set(b & !a)
        ),
        (_, Some(a)) => glog!("UI", "menu manager ready, visible {}", fmt_set(a)),
        (_, None) => glog!("UI", "menu manager not available"),
    }
}

fn fmt_set(bits: u128) -> String {
    let items: Vec<String> = (0..128).filter(|i| bits & (1 << i) != 0).map(|i| format!("{i:#04x}")).collect();
    format!("[{}]", items.join(" "))
}
