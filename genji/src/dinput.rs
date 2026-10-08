//! Intercepts DirectInput's `GetDeviceState`, which Elden Ring uses to read the keyboard.
//!
//! Every time the game reads the keyboard, our hook calls the real function, records the raw keys for our own
//! use, then (only while `set_blocking(true)`) blanks the keys Genji owns and turns F into E, so the game never
//! sees Genji's keys and F still does Elden Ring's "interact".
//!
//! The mouse is not handled here: tested 2026-10-07, Elden Ring never reads the mouse through this function
//! (0 calls per second). Mouse buttons are read from, and weapon actions blocked in, the game's gameplay input
//! reader instead (see `gamepad.rs`).
//!
//! How the hook works: all DirectInput devices of one kind share one table of function pointers (a "vtable").
//! We create our own throwaway keyboard device to find that table, then swap entry 9 (`GetDeviceState`) for
//! our function. No game code is changed.

use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicUsize, Ordering};

use windows::Win32::Devices::HumanInterfaceDevice::{
    DIRECTINPUT_VERSION, DirectInput8Create, GUID_SysKeyboard, IDirectInput8A, IDirectInput8W,
    IDirectInputDevice8A, IDirectInputDevice8W,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Memory::{PAGE_PROTECTION_FLAGS, PAGE_READWRITE, VirtualProtect};
use windows::core::{Interface, PCWSTR};

use crate::glog;

/// DirectInput keyboard scan codes ("DIK_" codes) we care about.
pub mod dik {
    pub const W: usize = 0x11;
    pub const A: usize = 0x1E;
    pub const S: usize = 0x1F;
    pub const D: usize = 0x20;
    pub const Q: usize = 0x10;
    pub const E: usize = 0x12;
    pub const F: usize = 0x21;
    pub const LSHIFT: usize = 0x2A;
    pub const SPACE: usize = 0x39;
    pub const F8: usize = 0x42;
    pub const F9: usize = 0x43;
}

/// Keys the game must not see while Genji mode is active.
const BLOCKED_KEYS: [usize; 4] = [dik::E, dik::Q, dik::LSHIFT, dik::F];
/// Pressing REMAP_FROM makes the game see REMAP_TO (F does Elden Ring's E "interact").
const REMAP_FROM: usize = dik::F;
const REMAP_TO: usize = dik::E;

const GET_DEVICE_STATE_SLOT: usize = 9;
const KEYBOARD_STATE_SIZE: u32 = 256;

type GetDeviceStateFn = unsafe extern "system" fn(*mut c_void, u32, *mut c_void) -> i32;

static ORIGINAL_W: AtomicUsize = AtomicUsize::new(0);
static ORIGINAL_A: AtomicUsize = AtomicUsize::new(0);

static BLOCKING: AtomicBool = AtomicBool::new(false);
static RAW_KEYS: [AtomicU64; 4] = [const { AtomicU64::new(0) }; 4];
static KEYBOARD_READS: AtomicU32 = AtomicU32::new(0);

/// While true, Genji's keys are hidden from the game.
pub fn set_blocking(on: bool) {
    BLOCKING.store(on, Ordering::Relaxed);
}

/// Raw keyboard state as the hardware reported it (before blanking), one bit per DIK code.
pub fn raw_keys() -> [u64; 4] {
    std::array::from_fn(|i| RAW_KEYS[i].load(Ordering::Relaxed))
}

pub fn key_down(keys: &[u64; 4], code: usize) -> bool {
    keys[code / 64] & (1 << (code % 64)) != 0
}

/// Keyboard reads since the last call (to check the hook is live).
pub fn take_read_count() -> u32 {
    KEYBOARD_READS.swap(0, Ordering::Relaxed)
}

pub fn install() {
    match unsafe { install_inner() } {
        Ok(msg) => glog!("DINPUT", "{msg}"),
        Err(e) => glog!("DINPUT", "hook NOT installed: {e}"),
    }
}

unsafe fn install_inner() -> Result<String, String> {
    let instance = unsafe { GetModuleHandleW(PCWSTR::null()) }.map_err(|e| format!("GetModuleHandle: {e}"))?;

    // Wide-character (W) interface: the usual one.
    let mut di_w: Option<IDirectInput8W> = None;
    unsafe {
        DirectInput8Create(instance.into(), DIRECTINPUT_VERSION, &IDirectInput8W::IID, &mut di_w as *mut _ as *mut *mut c_void, None)
    }
    .map_err(|e| format!("DirectInput8Create(W): {e}"))?;
    let di_w = di_w.ok_or("DirectInput8Create(W) returned nothing")?;
    let mut dev_w: Option<IDirectInputDevice8W> = None;
    unsafe { di_w.CreateDevice(&GUID_SysKeyboard, &mut dev_w, None) }.map_err(|e| format!("CreateDevice(W): {e}"))?;
    let dev_w = dev_w.ok_or("CreateDevice(W) returned nothing")?;
    let w = unsafe { patch_vtable(dev_w.as_raw(), hook_w as GetDeviceStateFn as usize, &ORIGINAL_W) }?;

    // ANSI (A) interface, in case the game uses that one.
    let mut di_a: Option<IDirectInput8A> = None;
    unsafe {
        DirectInput8Create(instance.into(), DIRECTINPUT_VERSION, &IDirectInput8A::IID, &mut di_a as *mut _ as *mut *mut c_void, None)
    }
    .map_err(|e| format!("DirectInput8Create(A): {e}"))?;
    let di_a = di_a.ok_or("DirectInput8Create(A) returned nothing")?;
    let mut dev_a: Option<IDirectInputDevice8A> = None;
    unsafe { di_a.CreateDevice(&GUID_SysKeyboard, &mut dev_a, None) }.map_err(|e| format!("CreateDevice(A): {e}"))?;
    let dev_a = dev_a.ok_or("CreateDevice(A) returned nothing")?;
    let a = unsafe { patch_vtable(dev_a.as_raw(), hook_a as GetDeviceStateFn as usize, &ORIGINAL_A) }?;

    // Keep the DirectInput objects alive forever so dinput8.dll (and the patched tables) can never unload.
    std::mem::forget((di_w, dev_w, di_a, dev_a));
    Ok(format!("GetDeviceState hooked: W {w}, A {a}"))
}

/// Replaces the GetDeviceState entry of the object's vtable with `hook`, saving the original in `original`.
unsafe fn patch_vtable(object: *mut c_void, hook: usize, original: &AtomicUsize) -> Result<String, String> {
    let vtable = unsafe { *(object as *const *mut usize) };
    let slot = unsafe { vtable.add(GET_DEVICE_STATE_SLOT) };
    let current = unsafe { slot.read() };
    if current == hook_w as GetDeviceStateFn as usize || current == hook_a as GetDeviceStateFn as usize {
        return Ok(format!("vtable {vtable:p} already hooked"));
    }
    original.store(current, Ordering::SeqCst);
    let mut old = PAGE_PROTECTION_FLAGS::default();
    unsafe { VirtualProtect(slot as *const c_void, 8, PAGE_READWRITE, &mut old) }
        .map_err(|e| format!("VirtualProtect: {e}"))?;
    unsafe { slot.write(hook) };
    let _ = unsafe { VirtualProtect(slot as *const c_void, 8, old, &mut old) };
    Ok(format!("vtable {vtable:p} slot {GET_DEVICE_STATE_SLOT} {current:#x} -> {hook:#x}"))
}

unsafe extern "system" fn hook_w(this: *mut c_void, size: u32, data: *mut c_void) -> i32 {
    unsafe { call_and_filter(&ORIGINAL_W, this, size, data) }
}

unsafe extern "system" fn hook_a(this: *mut c_void, size: u32, data: *mut c_void) -> i32 {
    unsafe { call_and_filter(&ORIGINAL_A, this, size, data) }
}

unsafe fn call_and_filter(original: &AtomicUsize, this: *mut c_void, size: u32, data: *mut c_void) -> i32 {
    let real: GetDeviceStateFn = unsafe { std::mem::transmute(original.load(Ordering::Relaxed)) };
    let result = unsafe { real(this, size, data) };
    if result < 0 || data.is_null() {
        return result;
    }
    // No panics may escape into the game from here; nothing below can panic.
    if size == KEYBOARD_STATE_SIZE {
        unsafe { filter_keyboard(&mut *(data as *mut [u8; 256])) };
    }
    result
}

fn filter_keyboard(keys: &mut [u8; 256]) {
    KEYBOARD_READS.fetch_add(1, Ordering::Relaxed);
    let mut bits = [0u64; 4];
    for (code, state) in keys.iter().enumerate() {
        if state & 0x80 != 0 {
            bits[code / 64] |= 1 << (code % 64);
        }
    }
    for (slot, value) in RAW_KEYS.iter().zip(bits) {
        slot.store(value, Ordering::Relaxed);
    }
    if !BLOCKING.load(Ordering::Relaxed) {
        return;
    }
    let remap = keys[REMAP_FROM] & 0x80 != 0;
    for code in BLOCKED_KEYS {
        keys[code] = 0;
    }
    if remap {
        keys[REMAP_TO] = 0x80;
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    /// One test, because the hook and the blocking flag are process-wide.
    #[test]
    fn hook_forwards_and_filter_blanks_genji_keys() {
        // 1. Install twice: the second call must notice it is already hooked.
        install();
        install();

        // 2. A fresh device's vtable now points at our hook, and calling through it reaches the real
        //    function: an unacquired device answers DIERR_NOTACQUIRED (0x8007000C) instead of crashing.
        let instance = unsafe { GetModuleHandleW(PCWSTR::null()) }.unwrap();
        let mut di: Option<IDirectInput8W> = None;
        unsafe {
            DirectInput8Create(instance.into(), DIRECTINPUT_VERSION, &IDirectInput8W::IID, &mut di as *mut _ as *mut *mut c_void, None)
        }
        .unwrap();
        let di = di.unwrap();
        let mut dev: Option<IDirectInputDevice8W> = None;
        unsafe { di.CreateDevice(&GUID_SysKeyboard, &mut dev, None) }.unwrap();
        let dev = dev.unwrap();
        let vtable = unsafe { *(dev.as_raw() as *const *const usize) };
        let slot = unsafe { vtable.add(GET_DEVICE_STATE_SLOT).read() };
        assert_eq!(slot, hook_w as GetDeviceStateFn as usize, "vtable slot not hooked");
        let mut buf = [0u8; 256];
        let hr = unsafe { hook_w(dev.as_raw(), 256, buf.as_mut_ptr() as *mut c_void) };
        assert_eq!(hr as u32, 0x8007_000C, "expected DIERR_NOTACQUIRED from the real function");

        // 3. Keyboard filter, not blocking: nothing changes, raw keys recorded.
        set_blocking(false);
        let mut keys = [0u8; 256];
        for code in [dik::F, dik::Q, dik::LSHIFT, dik::SPACE, dik::E] {
            keys[code] = 0x80;
        }
        let before = keys;
        filter_keyboard(&mut keys);
        assert_eq!(keys, before);
        let raw = raw_keys();
        for code in [dik::F, dik::Q, dik::LSHIFT, dik::SPACE, dik::E] {
            assert!(key_down(&raw, code));
        }

        // 4. Blocking: E, Q, Shift, F hidden; F shows up as E; Space untouched.
        set_blocking(true);
        keys[dik::E] = 0; // only F held: the game must still see E
        filter_keyboard(&mut keys);
        assert_eq!(keys[dik::E], 0x80, "F must reach the game as E");
        assert_eq!(keys[dik::F], 0);
        assert_eq!(keys[dik::Q], 0);
        assert_eq!(keys[dik::LSHIFT], 0);
        assert_eq!(keys[dik::SPACE], 0x80, "Space is not captured in step 3");
        assert!(key_down(&raw_keys(), dik::F), "raw state keeps the real key");
        assert!(!key_down(&raw_keys(), dik::E), "raw state must not contain the remapped E");

        // 5. Blocking, E pressed alone (Deflect): the game sees nothing.
        let mut keys = [0u8; 256];
        keys[dik::E] = 0x80;
        filter_keyboard(&mut keys);
        assert_eq!(keys[dik::E], 0);

        set_blocking(false);
    }
}
