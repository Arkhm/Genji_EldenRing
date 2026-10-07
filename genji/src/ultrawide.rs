//! Removes Elden Ring's forced 16:9 picture so ultrawide monitors have no black bars.
//!
//! Same patch as techiew's UltrawideFix (MIT licence, https://github.com/techiew/EldenRingMods): find the
//! byte pattern below in the game's code, then turn the conditional jump 0x94 bytes after it (`74`, "jump if
//! equal") into an unconditional jump (`EB`). The patch is made in memory while the game runs; no file on
//! disk is changed.

use std::ffi::c_void;
use std::time::{Duration, Instant};

use pelite::pe64::{Pe, PeView};
use windows::Win32::System::Diagnostics::Debug::FlushInstructionCache;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Memory::{
    MEM_COMMIT, MEMORY_BASIC_INFORMATION, PAGE_EXECUTE_READWRITE, PAGE_GUARD, PAGE_NOACCESS,
    PAGE_PROTECTION_FLAGS, VirtualProtect, VirtualQuery,
};
use windows::Win32::System::Threading::GetCurrentProcess;
use windows::core::PCWSTR;

use crate::glog;

const PATTERN: [u8; 26] = [
    0x48, 0xc7, 0x45, 0xb8, 0xfe, 0xff, 0xff, 0xff, 0x48, 0x89, 0x58, 0x10, 0x48, 0x89, 0x70, 0x18, 0x48,
    0x89, 0x78, 0x20, 0x0f, 0x29, 0x70, 0xc8, 0x48, 0x8b,
];
const OFFSET: usize = 0x94;
const ORIGINAL: u8 = 0x74;
const PATCHED: u8 = 0xEB;

const IMAGE_SCN_MEM_EXECUTE: u32 = 0x2000_0000;

/// Retries for a while: parts of the game's code may only become readable after start-up.
const GIVE_UP_AFTER: Duration = Duration::from_secs(60);
const RETRY_EVERY: Duration = Duration::from_millis(500);

pub fn apply() {
    let started = Instant::now();
    let mut attempts = 0;
    loop {
        attempts += 1;
        let matches = find_matches();
        match matches.as_slice() {
            [] => {}
            [address] => {
                patch(*address + OFFSET, attempts, started);
                return;
            }
            many => {
                glog!("WIDE", "pattern found {} times, expected once; not patching", many.len());
                return;
            }
        }
        if started.elapsed() > GIVE_UP_AFTER {
            glog!("WIDE", "pattern not found after {attempts} scans ({:.0}s); not patching", started.elapsed().as_secs_f32());
            return;
        }
        std::thread::sleep(RETRY_EVERY);
    }
}

fn patch(address: usize, attempts: u32, started: Instant) {
    let byte = address as *mut u8;
    let current = unsafe { byte.read_volatile() };
    let when = format!("scan {attempts}, {:.1}s", started.elapsed().as_secs_f32());
    if current == PATCHED {
        glog!("WIDE", "already patched at {address:#x} ({when})");
        return;
    }
    if current != ORIGINAL {
        glog!("WIDE", "unexpected byte {current:#04x} at {address:#x}, expected {ORIGINAL:#04x}; not patching ({when})");
        return;
    }
    unsafe {
        let mut old = PAGE_PROTECTION_FLAGS::default();
        if VirtualProtect(byte as *const c_void, 1, PAGE_EXECUTE_READWRITE, &mut old).is_err() {
            glog!("WIDE", "VirtualProtect failed at {address:#x}; not patching");
            return;
        }
        byte.write_volatile(PATCHED);
        let _ = VirtualProtect(byte as *const c_void, 1, old, &mut old);
        let _ = FlushInstructionCache(GetCurrentProcess(), Some(byte as *const c_void), 1);
    }
    let after = unsafe { byte.read_volatile() };
    glog!("WIDE", "patched {address:#x}: {ORIGINAL:#04x} -> {after:#04x} ({when})");
}

/// Addresses where PATTERN starts, searched in the readable parts of the exe's code sections.
fn find_matches() -> Vec<usize> {
    let Ok(module) = (unsafe { GetModuleHandleW(PCWSTR::null()) }) else {
        return Vec::new();
    };
    let base = module.0 as usize;
    let view = unsafe { PeView::module(base as *const u8) };
    let mut found = Vec::new();
    for section in view.section_headers() {
        if section.Characteristics & IMAGE_SCN_MEM_EXECUTE == 0 {
            continue;
        }
        let start = base + section.VirtualAddress as usize;
        let end = start + section.VirtualSize as usize;
        for (from, to) in readable_ranges(start, end) {
            let bytes = unsafe { std::slice::from_raw_parts(from as *const u8, to - from) };
            found.extend(
                bytes
                    .windows(PATTERN.len())
                    .enumerate()
                    .filter(|(_, w)| *w == PATTERN)
                    .map(|(i, _)| from + i),
            );
        }
    }
    found
}

/// Splits [start, end) into runs of committed, readable memory (skips no-access and guard pages).
fn readable_ranges(start: usize, end: usize) -> Vec<(usize, usize)> {
    let mut ranges: Vec<(usize, usize)> = Vec::new();
    let mut at = start;
    while at < end {
        let mut info = MEMORY_BASIC_INFORMATION::default();
        let size = std::mem::size_of::<MEMORY_BASIC_INFORMATION>();
        if unsafe { VirtualQuery(Some(at as *const c_void), &mut info, size) } == 0 {
            break;
        }
        let region_end = (info.BaseAddress as usize + info.RegionSize).min(end);
        let readable = info.State == MEM_COMMIT
            && (info.Protect & PAGE_NOACCESS).0 == 0
            && (info.Protect & PAGE_GUARD).0 == 0;
        if readable {
            match ranges.last_mut() {
                Some(last) if last.1 == at => last.1 = region_end,
                _ => ranges.push((at, region_end)),
            }
        }
        at = region_end;
    }
    ranges
}
