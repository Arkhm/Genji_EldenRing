//! Silences the Tarnished's own voice (grunts of pain, effort, death) while Genji mode drives, so only Genji's lines
//! are heard (your test of 2026-10-09: "when I take a hit, I hear the elden ring char grunt of pain, not genji").
//!
//! How: Elden Ring picks the player's voice with a sound "switch" whose value comes from the table
//! `WwiseValueToStrParam_Switch_PlayerVoiceType` (voice type number -> switch name). While Genji mode drives, the
//! player's rows get a name the game's sound banks don't have, so the switch selects no voice; the names are put back
//! when Genji mode lets go. Nothing is written to files or the save. Tested 2026-10-09: works at once.
//!
//! Only the player's rows (ids below 50: `_00_PC_Male_Young_Neutral` to `_15_PC_Female_Aged_Deeper`). The same table
//! names NPC and enemy voices (50 and up: `_50_NPC_Male_Young`, `NPC_vc300`...); the first version renamed all 174
//! rows, which silenced those too.

use eldenring::cs::{SoloParamRepository, WwiseValueToStrParam_Switch_PlayerVoiceType};
use fromsoftware_shared::FromStatic;

use crate::glog;

/// Rows below this id are the player's voices.
const PLAYER_ROWS_BELOW: u32 = 50;
/// A switch name no sound bank has.
const SILENT: &[u8] = b"GenjiBridgeSilent";

#[derive(Default)]
pub struct GameVoice {
    /// Each row's original name while ours is in place.
    saved: Option<Vec<(u32, [u8; 32])>>,
}

impl GameVoice {
    /// Silent while `on`, the game's own voice otherwise. Cheap to call every frame.
    pub fn set_silent(&mut self, on: bool) {
        if on == self.saved.is_some() {
            return;
        }
        let Ok(params) = (unsafe { SoloParamRepository::instance_mut() }) else {
            return;
        };
        match self.saved.take() {
            None => {
                let mut silent = [0u8; 32];
                silent[..SILENT.len()].copy_from_slice(SILENT);
                let mut old = Vec::new();
                for (id, row) in params.rows_mut::<WwiseValueToStrParam_Switch_PlayerVoiceType>() {
                    if id < PLAYER_ROWS_BELOW {
                        old.push((id, *row.param_str()));
                        row.set_param_str(silent);
                    }
                }
                let names: Vec<String> = old.iter().map(|(id, s)| format!("{id}={}", name(s))).collect();
                glog!("VOICE", "Tarnished voice silenced: {} player rows of PlayerVoiceType renamed (were {})", old.len(), names.join(", "));
                self.saved = Some(old);
            }
            Some(old) => {
                for (id, row) in params.rows_mut::<WwiseValueToStrParam_Switch_PlayerVoiceType>() {
                    if let Some((_, s)) = old.iter().find(|(i, _)| *i == id) {
                        row.set_param_str(*s);
                    }
                }
                glog!("VOICE", "Tarnished voice back ({} rows restored)", old.len());
            }
        }
    }
}

fn name(s: &[u8; 32]) -> String {
    let end = s.iter().position(|b| *b == 0).unwrap_or(s.len());
    String::from_utf8_lossy(&s[..end]).into_owned()
}
