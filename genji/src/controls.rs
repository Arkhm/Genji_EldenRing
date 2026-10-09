//! Genji's controls, read once per frame on the game's thread.
//!
//! - Turns raw key and button changes into Genji actions (logged as `ACTION`).
//! - F9 switches Genji mode on and off. The game starts in plain Elden Ring (Genji mode off).
//! - "Captures" Genji's input while Genji mode is on, the player exists and the HUD is visible. While
//!   captured:
//!   - the keyboard hook hides E, Q, Left Shift and F from the game, and passes F on as E (interact).
//!   - Elden Ring's weapon actions (Attack, Strong Attack, Guard, Skill) are switched off in the gameplay
//!     input reader (`gamepad.rs`, applied in the `GameFlowStep` stage), which covers the mouse buttons. The
//!     left-click diagnostic below logs what the game's Attack check says.
//!
//! "HUD visible" = UI state 0x07 in the game's UI-state table (`CSMenuManImp.ui_states`). In the test sessions
//! (2026-10-07) it was hidden whenever a menu tab was open (inventory, map, grace, ...) and during loading,
//! and stayed visible through pop-ups, item pickups and interaction prompts.
//! Exception: the Esc menu itself (the bar of tabs) keeps the HUD visible. It shows UI state 0x1c (always
//! together with 0x17); choosing a tab replaces 0x1c with the tab's own state (0x1d, 0x25, 0x34 seen) and hides
//! the HUD. So "gameplay" = 0x07 visible and 0x1c not visible.
//! Rules that failed: "any UI state beyond a baseline" (pop-ups counted as menus) and "gameplay input reader
//! enabled" (it stays enabled in menus).
//!
//! 2026-10-08: Elden Ring's HUD is switched off while Genji input is captured (your request: the game's HUD
//! clutters the Overwatch one), with the game's own HUD option (`GameSettings.hud_type`), and restored when capture
//! ends. So the rule no longer needs 0x07: "gameplay" = 0x08 visible, 0x07 visible or the HUD switched off by us,
//! and no UI state outside GAMEPLAY_UI_STATES (the states seen during play in your logs; the Esc menu's 0x1c and
//! every menu tab are outside it). An unknown state counts as a menu (the safe side) and is logged.
//! UI-state changes are still logged, to map them later.

use eldenring::cs::{CSMenuManImp, GameDataMan, HudType};
use fromsoftware_shared::FromStatic;

use crate::camera;
use crate::dinput::{self, dik};
use crate::gamepad;
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
/// UI state that is visible exactly while the in-game HUD is on screen.
const HUD_UI_STATE: u32 = 0x07;
/// Present in every gameplay set of UI states in your logs (and in menus); absent on loading screens.
const WORLD_UI_STATE: u32 = 0x08;
/// UI states seen during play (pop-ups, item pickups, prompts included), from your logs of 2026-10-07/08.
/// 0x03 (2026-10-08): shown for 8 s in the middle of your Margit fight while you moved, fought and took hits (and
/// gone when you died); counted as a menu, it handed the camera back mid-fight ("cutscenes take me out of Genji
/// mode"). What the game uses it for is not known; your log shows it only during that boss fight.
const GAMEPLAY_UI_STATES: [u32; 12] = [0x03, 0x04, 0x05, 0x07, 0x08, 0x09, 0x0f, 0x11, 0x12, 0x13, 0x17, 0x1a];
/// Gameplay must hold this long before capture switches on (release is instant). Going back from a menu tab
/// to the Esc menu shows the HUD one frame before the Esc menu's own state, which flicked capture on for a frame.
const CAPTURE_DELAY: f32 = 0.1;

pub struct Controls {
    enabled: bool,
    capturing: bool,
    held: [Option<f32>; BINDINGS.len()],
    toggle_was_down: bool,
    click_was_down: bool,
    last_ui: Option<u128>,
    clock: f32,
    report_timer: f32,
    gameplay_for: f32,
    /// Elden Ring's own HUD setting, saved while we switch its HUD off (Genji input captured).
    hud_saved: Option<HudType>,
    /// UI states outside GAMEPLAY_UI_STATES already logged.
    unknown_logged: u128,
}

impl Default for Controls {
    fn default() -> Self {
        Self {
            enabled: false,
            capturing: false,
            held: [None; BINDINGS.len()],
            toggle_was_down: false,
            click_was_down: false,
            last_ui: None,
            clock: 0.0,
            report_timer: 0.0,
            gameplay_for: 0.0,
            hud_saved: None,
            unknown_logged: 0,
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

        // In gameplay = HUD visible.
        let ui = visible_ui_states();
        if ui != self.last_ui {
            log_ui_change(self.last_ui, ui);
            self.last_ui = ui;
        }
        let allowed: u128 = GAMEPLAY_UI_STATES.iter().fold(0, |acc, s| acc | (1 << s));
        let hud = ui.is_some_and(|bits| {
            let hud_shown_or_ours = bits & (1 << HUD_UI_STATE) != 0 || self.hud_saved.is_some();
            bits & (1 << WORLD_UI_STATE) != 0 && hud_shown_or_ours && bits & !allowed == 0
        });
        if let Some(bits) = ui {
            let unknown = bits & !allowed & !self.unknown_logged;
            if unknown != 0 && bits & (1 << WORLD_UI_STATE) != 0 {
                glog!("UI", "state(s) {} not known from gameplay: counted as a menu", fmt_set(unknown));
                self.unknown_logged |= unknown;
            }
        }

        let gameplay = self.enabled && player_present && hud;
        self.gameplay_for = if gameplay { self.gameplay_for + dt } else { 0.0 };
        let capturing = gameplay && (self.capturing || self.gameplay_for >= CAPTURE_DELAY);
        if capturing != self.capturing {
            self.capturing = capturing;
            dinput::set_blocking(capturing);
            gamepad::set_weapons_blocked(capturing);
            camera::set_first_person(capturing);
            self.set_game_hud_hidden(capturing);
            let why = if !self.enabled {
                "Genji mode off"
            } else if !player_present {
                "no player"
            } else if !hud {
                "menu or loading"
            } else {
                "gameplay"
            };
            glog!("MODE", "Genji input {} ({why})", if capturing { "CAPTURED" } else { "released to the game" });
        }

        // A short HUD peek so the game fills in a boss's name (`enemy_bars.rs` asks; it happens once per boss).
        if self.capturing {
            if let Some(saved) = self.hud_saved {
                if let Some(settings) = unsafe { GameDataMan::instance_mut() }.ok().map(|g| &mut *g.game_settings) {
                    let want = if crate::enemy_bars::hud_peek_wanted() {
                        if saved == HudType::Off { HudType::On } else { saved }
                    } else {
                        HudType::Off
                    };
                    if settings.hud_type != want {
                        glog!("MODE", "Elden Ring HUD {:?} (boss name peek)", want);
                        settings.hud_type = want;
                    }
                }
            }
        }

        let click_now = buttons & (1 << mouse::LEFT) != 0;
        // Actions: only while capturing, so menus and plain Elden Ring never trigger Genji.
        for (i, (action, input)) in BINDINGS.iter().enumerate() {
            let down = match input {
                Input::Key(code) => dinput::key_down(&keys, *code),
                Input::Mouse(button) => buttons & (1 << button) != 0,
            };
            // Diagnostic: does the game see left click as Attack? Expected `false` while captured, `true` otherwise.
            if *action == Action::Shuriken && down && !self.click_was_down {
                glog!("PAD", "left click: game's Attack check says {:?}", gamepad::attack_polled());
            }
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
        self.click_was_down = click_now;
    }

    /// Switches Elden Ring's HUD off (true) or back to your setting (false), with the game's own HUD option.
    fn set_game_hud_hidden(&mut self, hidden: bool) {
        let Some(settings) = unsafe { GameDataMan::instance_mut() }.ok().map(|g| &mut *g.game_settings) else {
            return;
        };
        match (hidden, self.hud_saved) {
            (true, None) => {
                self.hud_saved = Some(settings.hud_type);
                settings.hud_type = HudType::Off;
                glog!("MODE", "Elden Ring HUD off (was {:?})", self.hud_saved.unwrap());
            }
            (false, Some(saved)) => {
                settings.hud_type = saved;
                self.hud_saved = None;
                glog!("MODE", "Elden Ring HUD back to {saved:?}");
            }
            _ => {}
        }
    }

    /// Once per second.
    pub fn report(&mut self) {
        let keyboard = dinput::take_read_count();
        self.report_timer += 1.0;
        if self.report_timer >= 10.0 {
            self.report_timer = 0.0;
            glog!("DINPUT", "keyboard reads in the last second: {keyboard}");
        }
    }
}

/// After a crash of our per-frame code: the game's HUD back on, so it's never left switched off.
pub fn emergency_hud_on() {
    if let Some(g) = unsafe { GameDataMan::instance_mut() }.ok() {
        if g.game_settings.hud_type == HudType::Off {
            g.game_settings.hud_type = HudType::On;
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
