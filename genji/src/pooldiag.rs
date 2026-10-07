//! Diagnostic (STATUS.md option A, 2026-10-07): what fills the bullet manager's second pool?
//!
//! Throwing stops when that pool (fromsoftware-rs `unk_bullet_sfx_related`: 0x9D0-byte entries, 64 in a buffer +
//! up to 192 extra) is full. It is not our projectiles and not a visual effect. Most C++ objects in the game start
//! with a pointer to their class's virtual function table; the 8 bytes before that table point to RTTI data that
//! contains the class name as text (`.?AVCSSomething@@`). This module reads those names, read-only:
//! - every slot of the 64-entry buffer: class names counted;
//! - the live list from its head: the "next" pointer's position inside an entry is found by looking for a field
//!   that points to another entry of the same class; then the list is walked and counted.
//! Every pointer is checked to lie inside the game's executable image (for tables and RTTI) before it is read.

use std::collections::BTreeMap;

use eldenring::cs::CSBulletManager;
use pelite::pe64::{Pe, PeView};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Memory::{MEM_COMMIT, MEMORY_BASIC_INFORMATION, PAGE_GUARD, PAGE_NOACCESS, VirtualQuery};
use windows::core::PCWSTR;

use crate::glog;

/// Offset of the second pool inside CSBulletManager, and its fields (fromsoftware-rs layout).
const POOL: usize = 0x20;
const ENTRY_SIZE: usize = 0x9D0;
const BUFFER_SLOTS: usize = 64;
const MAX_WALK: usize = 300;

struct Image {
    base: usize,
    end: usize,
}

impl Image {
    fn get() -> Option<Self> {
        let module = unsafe { GetModuleHandleW(PCWSTR::null()) }.ok()?;
        let base = module.0 as usize;
        let view = unsafe { PeView::module(base as *const u8) };
        let size = view.optional_header().SizeOfImage as usize;
        Some(Self { base, end: base + size })
    }

    fn contains(&self, addr: usize, len: usize) -> bool {
        addr >= self.base && addr.saturating_add(len) <= self.end
    }
}

/// True if `len` bytes at `addr` are committed, readable memory.
fn readable(addr: usize, len: usize) -> bool {
    if addr == 0 {
        return false;
    }
    let mut info = MEMORY_BASIC_INFORMATION::default();
    let size = std::mem::size_of::<MEMORY_BASIC_INFORMATION>();
    if unsafe { VirtualQuery(Some(addr as *const _), &mut info, size) } == 0 {
        return false;
    }
    let region_end = info.BaseAddress as usize + info.RegionSize;
    info.State == MEM_COMMIT
        && (info.Protect & PAGE_NOACCESS).0 == 0
        && (info.Protect & PAGE_GUARD).0 == 0
        && addr + len <= region_end
}

fn read_u64(addr: usize) -> Option<u64> {
    readable(addr, 8).then(|| unsafe { (addr as *const u64).read_unaligned() })
}

/// Class name of the object at `obj` from its vtable's RTTI, if it looks valid.
fn class_name(image: &Image, obj: usize) -> Option<String> {
    let vtable = read_u64(obj)? as usize;
    if !image.contains(vtable.checked_sub(8)?, 16) {
        return None;
    }
    let col = unsafe { ((vtable - 8) as *const u64).read() } as usize;
    if !image.contains(col, 0x18) {
        return None;
    }
    let signature = unsafe { (col as *const u32).read() };
    let type_rva = unsafe { ((col + 0x0C) as *const u32).read() } as usize;
    let type_desc = image.base + type_rva;
    if signature != 1 || !image.contains(type_desc + 0x10, 4) {
        return None;
    }
    let mut name = Vec::new();
    for i in 0..200 {
        let at = type_desc + 0x10 + i;
        if !image.contains(at, 1) {
            return None;
        }
        let c = unsafe { (at as *const u8).read() };
        if c == 0 {
            break;
        }
        name.push(c);
    }
    Some(String::from_utf8_lossy(&name).into_owned())
}

/// Logs what the pool holds. Read-only.
pub fn report(manager: &CSBulletManager) {
    let Some(image) = Image::get() else {
        glog!("POOLDIAG", "executable image not found");
        return;
    };
    let mgr = manager as *const CSBulletManager as usize;
    let buffer = read_u64(mgr + POOL).unwrap_or(0) as usize;
    let head = read_u64(mgr + POOL + 0x08).unwrap_or(0) as usize;
    let alive = unsafe { ((mgr + POOL + 0x18) as *const u32).read() };
    glog!("POOLDIAG", "pool 2: buffer {buffer:#x} head {head:#x} alive(extra) {alive}");

    // 1. The 64 buffer slots.
    let mut names: BTreeMap<String, u32> = BTreeMap::new();
    if readable(buffer, ENTRY_SIZE * BUFFER_SLOTS) {
        for slot in 0..BUFFER_SLOTS {
            let name = class_name(&image, buffer + slot * ENTRY_SIZE).unwrap_or_else(|| "(no class)".into());
            *names.entry(name).or_default() += 1;
        }
    }
    glog!("POOLDIAG", "buffer slots by class: {names:?}");

    // 2. The live list from its head.
    if !readable(head, ENTRY_SIZE) {
        glog!("POOLDIAG", "head not readable");
        return;
    }
    let head_class = class_name(&image, head);
    glog!("POOLDIAG", "head class {head_class:?}");
    let first_bytes: Vec<String> = (0..0x60)
        .step_by(8)
        .map(|o| format!("+{o:#05x}={:#018x}", read_u64(head + o).unwrap_or(0)))
        .collect();
    glog!("POOLDIAG", "head first bytes: {}", first_bytes.join(" "));
    // Also some floats (timers often show up as small positive numbers).
    let floats: Vec<String> = (0..ENTRY_SIZE)
        .step_by(4)
        .filter_map(|o| {
            let v = unsafe { ((head + o) as *const f32).read_unaligned() };
            (v.is_finite() && v.abs() > 0.01 && v.abs() < 1000.0 && v.fract() != 0.0).then(|| format!("+{o:#05x}={v:.3}"))
        })
        .take(40)
        .collect();
    glog!("POOLDIAG", "head plausible floats: {}", floats.join(" "));

    // Objects embedded in the entry (in case the entry itself doesn't start with a vtable).
    let embedded: Vec<String> = (0..0x200)
        .step_by(8)
        .filter_map(|o| class_name(&image, head + o).map(|n| format!("+{o:#05x} {n}")))
        .take(8)
        .collect();
    glog!("POOLDIAG", "objects inside the head entry: {}", if embedded.is_empty() { "none".into() } else { embedded.join(", ") });

    // Find the "next" field: a pointer to another readable entry of the same class.
    let mut next_offset = None;
    for o in (0..ENTRY_SIZE).step_by(8) {
        let Some(p) = read_u64(head + o) else { continue };
        let p = p as usize;
        if p == 0 || p == head || !readable(p, 16) {
            continue;
        }
        if class_name(&image, p).is_some() && class_name(&image, p) == head_class {
            next_offset = Some(o);
            break;
        }
    }
    let Some(next) = next_offset else {
        glog!("POOLDIAG", "no next pointer found in the head entry");
        return;
    };
    let mut count = 0;
    let mut at = head;
    let mut classes: BTreeMap<String, u32> = BTreeMap::new();
    while at != 0 && count < MAX_WALK && readable(at, ENTRY_SIZE) {
        let name = class_name(&image, at).unwrap_or_else(|| "(no class)".into());
        *classes.entry(name).or_default() += 1;
        count += 1;
        at = read_u64(at + next).unwrap_or(0) as usize;
    }
    glog!("POOLDIAG", "live list via next at +{next:#x}: {count} entries, by class {classes:?}");
}
