//! Eliminations (step 7): a character we damaged that dies within CREDIT_WINDOW counts as our kill. Swift Strike's
//! cooldown resets on one. The window is an assumption (Overwatch's exact rule not verified).
//!
//! Characters are identified by their address and only ever read through the game's live character list, so an
//! unloaded character is never touched.

use std::collections::HashMap;
use std::sync::Mutex;

use eldenring::cs::WorldChrMan;
use fromsoftware_shared::FromStatic;

use crate::glog;

/// Seconds after our last damage during which a death counts as our elimination.
const CREDIT_WINDOW: f32 = 3.0;

/// Character address -> seconds since we last damaged it.
static DAMAGED: Mutex<Option<HashMap<usize, f32>>> = Mutex::new(None);

/// Remember that we just damaged this character.
pub fn damaged(chr: usize) {
    let mut guard = DAMAGED.lock().unwrap_or_else(|e| e.into_inner());
    guard.get_or_insert_with(HashMap::new).insert(chr, 0.0);
}

/// Once per frame: returns how many characters we damaged died since the last call.
pub fn tick(dt: f32) -> u32 {
    let mut guard = DAMAGED.lock().unwrap_or_else(|e| e.into_inner());
    let Some(map) = guard.as_mut() else {
        return 0;
    };
    if map.is_empty() {
        return 0;
    }
    for age in map.values_mut() {
        *age += dt;
    }
    let mut kills = 0;
    if let Ok(world) = unsafe { WorldChrMan::instance() } {
        for entry in world.chr_inses_by_distance.iter() {
            let id = entry.chr_ins.as_ptr() as usize;
            let Some(&age) = map.get(&id) else {
                continue;
            };
            let chr = unsafe { entry.chr_ins.as_ref() };
            if chr.modules.data.hp <= 0 {
                glog!("KILL", "elimination: character {id:#x} died {age:.2} s after our last damage");
                map.remove(&id);
                kills += 1;
            }
        }
    }
    map.retain(|_, age| *age <= CREDIT_WINDOW);
    kills
}
