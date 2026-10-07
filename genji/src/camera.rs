//! First-person view and movement, run every frame right after the game computes its camera (`CameraStep`,
//! task group `DrawParamUpdate`; physics has already stepped at this point).
//!
//! While Genji's input is captured and the character can be driven:
//!   1. mouse -> where we look (`look.rs`),
//!   2. WASD -> movement controller (`movement.rs`), Elden Ring's own walking/dodging/jumping switched off,
//!   3. the drawn camera (`pers_cam_1`) gets our rotation and the eye position; the player model is hidden.
//! The game gets everything back for menus, F9, Torrent, ladders and death (third person, model shown).
//!
//! Step 4 tests (2026-10-07): `pers_cam_1` is the camera the game draws; `pers_cam_2` is the game's own follow
//! camera, copied into slot 1 each frame. We start our look from slot 2, never from slot 1 (which holds our
//! own previous write).

use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

use eldenring::cs::{CSCamera, LadderState, PlayerIns, WorldChrMan};
use fromsoftware_shared::FromStatic;
use glam::Vec3;

use crate::dinput::{self, dik};
use crate::firing::Firing;
use crate::gamepad;
use crate::glog;
use crate::look::Look;
use crate::movement::{Mover, Step, Wish};
use crate::shuriken::Thrower;

/// Height of the eyes above the character's feet, in metres.
const EYE_HEIGHT: f32 = 1.6;
/// Camera pushed this far forward (horizontally) from the body's centre, in metres.
const FORWARD_OFFSET: f32 = 0.1;

static WANTED: AtomicBool = AtomicBool::new(false);
static STATE: Mutex<Option<View>> = Mutex::new(None);

/// Ask for first person (and our movement) on (true) or off (false). Applied by `camera_step`.
pub fn set_first_person(on: bool) {
    WANTED.store(on, Ordering::Relaxed);
}

#[derive(Default)]
struct View {
    look: Option<Look>,
    mover: Mover,
    model_hidden: bool,
    handback_reason: Option<&'static str>,
    thrower: Thrower,
    firing: Firing,
}

pub fn camera_step(dt: f32) {
    // The crosshair shows exactly while the first-person view is drawn.
    crate::crosshair::set_visible(camera_step_inner(dt));
}

/// Returns true if the first-person view was drawn this frame.
fn camera_step_inner(dt: f32) -> bool {
    let mut guard = STATE.lock().unwrap_or_else(|e| e.into_inner());
    let view = guard.get_or_insert_with(View::default);

    let Some(player) = (unsafe { WorldChrMan::instance_mut() }).ok().and_then(|w| w.main_player.as_mut()) else {
        view.look = None;
        view.model_hidden = false;
        gamepad::set_movement_blocked(false);
        return false;
    };
    let Ok(camera) = (unsafe { CSCamera::instance_mut() }) else {
        return false;
    };

    view.mover.watch_hp(player);
    view.thrower.tick(player, dt);
    let buttons = gamepad::mouse_buttons();
    let (left_down, right_down) = (buttons & 1 != 0, buttons & 2 != 0);

    let reason = if !WANTED.load(Ordering::Relaxed) {
        Some("Genji input not captured")
    } else if view.mover.gave_up {
        Some("movement controller gave up this session, see MOVE lines")
    } else {
        cannot_drive(player)
    };
    if reason != view.handback_reason {
        glog!("VIEW", "{}", match reason {
            None => "first person + Genji movement".to_string(),
            Some(r) => format!("game camera and movement ({r})"),
        });
        view.handback_reason = reason;
    }

    // Shuriken flight and damage values on the projectile's rows: on in Genji mode, vanilla otherwise.
    view.thrower.set_shuriken_values(reason.is_none());

    if reason.is_some() {
        view.firing.tick(dt, false, false, false);
        view.mover.release(player);
        view.look = None;
        gamepad::set_movement_blocked(false);
        if view.model_hidden {
            player.chr_ins.chr_flags1c5.set_enable_render(true);
            view.model_hidden = false;
        }
        return false;
    }

    // 1. Look.
    let body_forward = {
        let q = glam::Quat::from(player.chr_ins.modules.physics.orientation);
        q * Vec3::NEG_Z
    };
    if view.look.is_none() {
        // Field of view as the game draws it (we only replace the camera's position and rotation). The engine
        // stores the vertical angle in radians; horizontal follows from the aspect ratio.
        let (fov, aspect) = (camera.pers_cam_1.fov, camera.pers_cam_1.aspect_ratio);
        let horizontal = 2.0 * ((fov / 2.0).tan() * aspect).atan();
        let at_16_9 = 2.0 * ((fov / 2.0).tan() * 16.0 / 9.0).atan();
        glog!(
            "CAMERA",
            "fov raw {fov:.4} = {:.1} deg vertical, aspect {aspect:.3}, so {:.1} deg horizontal on this screen, {:.1} deg at 16:9 (Overwatch measures there)",
            fov.to_degrees(),
            horizontal.to_degrees(),
            at_16_9.to_degrees()
        );
    }
    let look = view.look.get_or_insert_with(|| Look::from_game_camera(&camera.pers_cam_2.matrix, body_forward));
    let (dx, dy) = gamepad::mouse_delta();
    look.turn(dx, dy);

    // 2. Move.
    let keys = dinput::raw_keys();
    let axis = |plus: usize, minus: usize| {
        (dinput::key_down(&keys, plus) as i32 - dinput::key_down(&keys, minus) as i32) as f32
    };
    let wish = Wish {
        forward: axis(dik::W, dik::S),
        right: axis(dik::D, dik::A),
        jump: dinput::key_down(&keys, dik::SPACE),
    };
    let look = *look;
    let mut driving = false;
    let feet = match view.mover.step(player, &look, &wish, dt) {
        Step::Driving(feet) => {
            gamepad::set_movement_blocked(true);
            driving = true;
            feet
        }
        Step::GameAction(feet) => {
            // The game moves the character itself (door, grab, knockback...): first person follows it. Elden Ring's
            // own walking stays blocked; unblocking it let its walk animations keep the game action going.
            gamepad::set_movement_blocked(true);
            feet
        }
        Step::NotReady => {
            view.firing.tick(dt, false, false, false);
            // Waiting for collision to load or for the character to settle, or gave up: the game drives.
            gamepad::set_movement_blocked(false);
            if view.model_hidden {
                player.chr_ins.chr_flags1c5.set_enable_render(true);
                view.model_hidden = false;
            }
            return false;
        }
    };

    // 3. Camera and model.
    let eye = feet + Vec3::Y * EYE_HEIGHT + look.flat_forward() * FORWARD_OFFSET;
    look.write_camera(&mut camera.pers_cam_1.matrix, eye);
    player.chr_ins.chr_flags1c5.set_enable_render(false);
    view.model_hidden = true;

    // 4. Shuriken (steps 6a, 6b): bursts on left click, Fan of Blades on right click, while we drive.
    for angle in view.firing.tick(dt, left_down, right_down, driving) {
        view.thrower.throw(player, &look, eye, angle);
    }
    true
}

/// Why the game must drive right now, or None if we can.
fn cannot_drive(player: &PlayerIns) -> Option<&'static str> {
    let modules = &player.chr_ins.modules;
    if modules.data.hp <= 0 {
        Some("dead")
    } else if modules.ride.is_mounted || modules.ride.is_mounting {
        Some("on Torrent")
    } else if modules.ladder.state != LadderState::None {
        Some("on a ladder")
    } else {
        None
    }
}
