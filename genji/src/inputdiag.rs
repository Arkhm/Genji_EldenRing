//! Input diagnostics: which keys reach Windows while the game window is focused, and which XInput
//! controllers the game process can see. Used to find out why the game shows controller prompts.
//! Read-only: nothing here changes what the game receives.

use windows::Win32::Foundation::ERROR_SUCCESS;
use windows::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState;
use windows::Win32::UI::Input::XboxController::{
    XINPUT_CAPABILITIES, XINPUT_FLAG, XINPUT_STATE, XInputGetCapabilities, XInputGetState,
};
use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId};

use crate::glog;

/// Polling empty controller slots is slow, so pads are checked a few times per second, not every frame.
const PAD_POLL_SECONDS: f32 = 0.25;
/// Stick and trigger values below these count as "resting" (XInput's recommended dead zones).
const STICK_DEADZONE: i32 = 7849;
const TRIGGER_THRESHOLD: u8 = 30;

#[derive(Default, Clone, Copy)]
struct Pad {
    connected: bool,
    last_packet: u32,
    packets: u32,
    active_polls: u32,
    last_buttons: u16,
}

#[derive(Default)]
pub struct InputDiag {
    focused: bool,
    /// One bit per virtual-key code.
    keys_down: [u64; 4],
    pads: [Pad; 4],
    poll_timer: f32,
}

impl InputDiag {
    /// Call every frame.
    pub fn tick(&mut self, dt: f32) {
        let focused = game_window_focused();
        if focused != self.focused {
            self.focused = focused;
            glog!("INPUT", "game window {}", if focused { "focused" } else { "not focused" });
        }
        if focused {
            self.poll_keys();
        }
        self.poll_timer += dt;
        if self.poll_timer >= PAD_POLL_SECONDS {
            self.poll_timer = 0.0;
            self.poll_pads();
        }
    }

    /// Call once per second: one summary line, only when something happened.
    pub fn summary(&mut self) {
        for (slot, pad) in self.pads.iter_mut().enumerate() {
            if pad.connected && pad.packets > 0 {
                glog!(
                    "INPUT",
                    "pad {slot}: {} state changes, {} polls with input past the dead zone, buttons {:#06x}",
                    pad.packets, pad.active_polls, pad.last_buttons
                );
            }
            pad.packets = 0;
            pad.active_polls = 0;
        }
    }

    fn poll_keys(&mut self) {
        // 0x01..0x06 are mouse buttons; the rest are keyboard keys. The generic Shift/Ctrl/Alt codes
        // (0x10..0x12) are skipped because the left/right versions are reported too.
        for vk in (0x01..=0xFEusize).filter(|vk| !(0x10..=0x12).contains(vk)) {
            let down = unsafe { GetAsyncKeyState(vk as i32) } as u16 & 0x8000 != 0;
            let (word, bit) = (vk / 64, 1u64 << (vk % 64));
            if down && self.keys_down[word] & bit == 0 {
                glog!("INPUT", "key down {}", key_name(vk));
            }
            if down {
                self.keys_down[word] |= bit;
            } else {
                self.keys_down[word] &= !bit;
            }
        }
    }

    fn poll_pads(&mut self) {
        for slot in 0..4u32 {
            let pad = &mut self.pads[slot as usize];
            let mut state = XINPUT_STATE::default();
            let connected = unsafe { XInputGetState(slot, &mut state) } == ERROR_SUCCESS.0;
            if connected != pad.connected {
                pad.connected = connected;
                if connected {
                    let mut caps = XINPUT_CAPABILITIES::default();
                    unsafe { XInputGetCapabilities(slot, XINPUT_FLAG(0), &mut caps) };
                    glog!(
                        "INPUT",
                        "pad {slot} CONNECTED (type {}, subtype {}, flags {:#06x})",
                        caps.Type.0, caps.SubType.0, caps.Flags.0
                    );
                } else {
                    glog!("INPUT", "pad {slot} disconnected");
                }
            }
            if !connected {
                continue;
            }
            if state.dwPacketNumber != pad.last_packet {
                pad.last_packet = state.dwPacketNumber;
                pad.packets += 1;
            }
            let g = &state.Gamepad;
            let past_deadzone = [g.sThumbLX, g.sThumbLY, g.sThumbRX, g.sThumbRY]
                .iter()
                .any(|v| (*v as i32).abs() > STICK_DEADZONE)
                || g.bLeftTrigger > TRIGGER_THRESHOLD
                || g.bRightTrigger > TRIGGER_THRESHOLD
                || g.wButtons.0 != 0;
            if past_deadzone {
                pad.active_polls += 1;
            }
            pad.last_buttons = g.wButtons.0;
        }
    }
}

fn game_window_focused() -> bool {
    let window = unsafe { GetForegroundWindow() };
    let mut pid = 0u32;
    unsafe { GetWindowThreadProcessId(window, Some(&mut pid)) };
    pid == std::process::id()
}

fn key_name(vk: usize) -> String {
    let name = match vk {
        0x01 => "MouseLeft",
        0x02 => "MouseRight",
        0x04 => "MouseMiddle",
        0x05 => "Mouse4",
        0x06 => "Mouse5",
        0x08 => "Backspace",
        0x09 => "Tab",
        0x0D => "Enter",
        0x1B => "Esc",
        0x20 => "Space",
        0x25 => "Left",
        0x26 => "Up",
        0x27 => "Right",
        0x28 => "Down",
        0xA0 => "LShift",
        0xA1 => "RShift",
        0xA2 => "LCtrl",
        0xA3 => "RCtrl",
        0xA4 => "LAlt",
        0xA5 => "RAlt",
        0x30..=0x39 | 0x41..=0x5A => return (vk as u8 as char).to_string(),
        0x70..=0x87 => return format!("F{}", vk - 0x6F),
        _ => return format!("vk{vk:#04x}"),
    };
    name.to_string()
}
