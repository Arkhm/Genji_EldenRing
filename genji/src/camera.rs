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

use eldenring::cs::{CSCam, CSCamera, ChrCam, LadderState, LockCamParam, PlayerIns, SoloParamRepository, WorldChrMan};
use fromsoftware_shared::FromStatic;
use glam::Vec3;

use crate::dinput::{self, dik};
use crate::deflect::Deflect;
use crate::firing::Firing;
use crate::gamepad;
use crate::glog;
use crate::look::Look;
use crate::movement::{Mover, Step, Wish};
use crate::shuriken::Thrower;
use crate::swift::Swift;

/// Height of the eyes above the character's feet, in metres.
const EYE_HEIGHT: f32 = 1.6;
/// Your Overwatch field of view (2026-10-08): 103 degrees, which Overwatch measures horizontally on a 16:9 screen.
/// Elden Ring's camera stores the vertical angle, so we write the vertical angle that gives 103 at 16:9 (70.5);
/// on your 21:9 screen that shows about 119 degrees across, as Overwatch does (assuming both widen the view on
/// ultrawide screens rather than crop it). Elden Ring's own is 48 vertical (76.7 at 16:9), restored outside
/// first person.
const OVERWATCH_FOV: f32 = 103.0;

/// Vertical field of view (radians) for a horizontal one measured at 16:9.
fn vertical_fov(horizontal_16_9_degrees: f32) -> f32 {
    2.0 * ((horizontal_16_9_degrees.to_radians() / 2.0).tan() * 9.0 / 16.0).atan()
}

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
    swift: Swift,
    shift_was_down: bool,
    /// Seconds until the next CAMERA sync report.
    sync_report: f32,
    f8_was_down: bool,
    deflect: Deflect,
    e_was_down: bool,
    /// LockCamParam field of view of every row, saved while ours is applied (STATUS.md option B).
    lock_cam_fov: Option<Vec<(u32, f32)>>,
}

pub fn camera_step(dt: f32) {
    // The crosshair shows exactly while the first-person view is drawn.
    crate::crosshair::set_visible(camera_step_inner(dt));
}

/// Returns true if the first-person view was drawn this frame.
fn camera_step_inner(dt: f32) -> bool {
    let mut guard = STATE.lock().unwrap_or_else(|e| e.into_inner());
    let view = guard.get_or_insert_with(View::default);

    let world = unsafe { WorldChrMan::instance_mut() }.ok();
    // The game's follow camera object (see step 3b below); a raw pointer, read before borrowing the player.
    let chr_cam = world.as_ref().and_then(|w| w.chr_cam);
    let Some(player) = world.and_then(|w| w.main_player.as_mut()) else {
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
    view.swift.tick(dt, crate::kills::tick(dt));
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
    view.swift.set_values(reason.is_none());
    view.swift.confirm(player);

    // STATUS.md option B: the Overwatch field of view through the game's own setting (LockCamParam), while first
    // person is on; restored otherwise.
    set_lock_cam_fov(&mut view.lock_cam_fov, reason.is_none());

    if reason.is_some() {
        view.firing.tick(dt, false, false, false);
        view.deflect.tick(dt, false, false);
        view.deflect.release(player);
        crate::crosshair::set_deflecting(false);
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
        dash: {
            // Swift Strike (step 7): Left Shift pressed (not held) and the cooldown ready.
            let down = dinput::key_down(&keys, dik::LSHIFT);
            let pressed = down && !view.shift_was_down;
            view.shift_was_down = down;
            if pressed && !view.swift.ready() {
                glog!("SWIFT", "not ready");
            }
            pressed && view.swift.ready()
        },
    };
    let look = *look;
    let mut driving = false;
    let feet = match view.mover.step(player, &look, &wish, dt) {
        Step::Driving(feet) => {
            gamepad::set_movement_blocked(true);
            driving = true;
            if view.mover.dash_started {
                view.swift.started();
            }
            if let Some((from, to)) = view.mover.dash_segment {
                view.swift.sweep(player, from, to);
            }
            if view.mover.dash_ended {
                view.swift.ended();
            }
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
            view.deflect.tick(dt, false, false);
            view.deflect.release(player);
            crate::crosshair::set_deflecting(false);
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
    // 3b. The game's own cameras follow our view too. Test of 2026-10-08: enemy health bars and enemy sounds sat at
    // the side of the screen until F9 was pressed twice; they are placed with the game's follow camera
    // (`pers_cam_2`, ChrCam), which kept facing where it was when first person started. Logged: how far it had
    // turned away from our view before each overwrite.
    view.sync_report -= dt;
    if view.sync_report <= 0.0 {
        view.sync_report = 2.0;
        let ours = Vec3::new(camera.pers_cam_1.matrix.2.0, camera.pers_cam_1.matrix.2.1, camera.pers_cam_1.matrix.2.2);
        let game = Vec3::new(camera.pers_cam_2.matrix.2.0, camera.pers_cam_2.matrix.2.1, camera.pers_cam_2.matrix.2.2);
        let angle = ours.normalize_or_zero().dot(game.normalize_or_zero()).clamp(-1.0, 1.0).acos().to_degrees();
        let chr_fov = chr_cam.map(|c| unsafe { c.as_ref() }.pers_cam.fov.to_degrees()).unwrap_or(f32::NAN);
        glog!(
            "CAMERA",
            "game follow camera was {angle:.1} deg away from our view; the game's own fov this frame: follow {:.1}, chr_cam {chr_fov:.1} (before our overwrite)",
            camera.pers_cam_2.fov.to_degrees()
        );
    }
    look.write_camera(&mut camera.pers_cam_2.matrix, eye);
    // 3c. Overwatch field of view on every camera that follows us. F8 marks of 2026-10-08: chr_cam still had the
    // game's 48 degrees while the screen showed 70.5, which puts anything placed with it about 1.6 times further
    // from the centre (health bars at the side).
    let fov = vertical_fov(OVERWATCH_FOV);
    camera.pers_cam_1.fov = fov;
    camera.pers_cam_2.fov = fov;
    if let Some(mut chr_cam) = chr_cam {
        let cam = &mut unsafe { chr_cam.as_mut() }.pers_cam;
        look.write_camera(&mut cam.matrix, eye);
        cam.fov = fov;
    }

    // F8: diagnostic snapshot of every camera, for health bars drawn in the wrong place (test of 2026-10-08).
    let f8 = dinput::key_down(&dinput::raw_keys(), dik::F8);
    if f8 && !view.f8_was_down {
        snapshot(camera, chr_cam, player, eye);
    }
    view.f8_was_down = f8;
    player.chr_ins.chr_flags1c5.set_enable_render(false);
    view.model_hidden = true;

    // 4. Deflect (step 8a): E, while we drive. No shuriken while deflecting (Overwatch).
    let e_down = dinput::key_down(&dinput::raw_keys(), dik::E);
    let e_pressed = e_down && !view.e_was_down;
    view.e_was_down = e_down;
    let deflecting = view.deflect.tick(dt, e_pressed, driving);
    view.deflect.apply(player, feet, look.flat_forward());
    crate::crosshair::set_deflecting(deflecting);

    // 5. Shuriken (steps 6a, 6b): bursts on left click, Fan of Blades on right click, while we drive.
    for angle in view.firing.tick(dt, left_down, right_down, driving && !deflecting) {
        view.thrower.throw(player, &look, eye, angle);
    }
    true
}

/// Logs every camera (position, direction, field of view) and where each would put the nearest characters on
/// screen (-1 = left edge, 0 = centre, +1 = right edge), so a health bar drawn in the wrong place can be matched
/// to the camera the game placed it with.
fn snapshot(camera: &CSCamera, chr_cam: Option<std::ptr::NonNull<ChrCam>>, player: &PlayerIns, eye: Vec3) {
    let mut cams: Vec<(&str, &CSCam)> = vec![
        ("pers_cam_1 (drawn)", &camera.pers_cam_1),
        ("pers_cam_2 (follow)", &camera.pers_cam_2),
        ("pers_cam_3", &camera.pers_cam_3),
        ("pers_cam_4", &camera.pers_cam_4),
    ];
    if let Some(c) = chr_cam {
        cams.push(("chr_cam", &unsafe { c.as_ref() }.pers_cam));
    }
    let targets = crate::targets::living(player);
    let mut near: Vec<_> = targets.iter().map(|t| ((t.feet - eye).length(), t)).collect();
    near.sort_by(|a, b| a.0.total_cmp(&b.0));
    near.truncate(3);
    glog!("MARK", "F8 snapshot: eye ({:.2}, {:.2}, {:.2}), camera mask {:#x}", eye.x, eye.y, eye.z, camera.camera_mask);
    for (name, cam) in cams {
        let m = &cam.matrix;
        let right = Vec3::new(m.0.0, m.0.1, m.0.2);
        let up = Vec3::new(m.1.0, m.1.1, m.1.2);
        let forward = Vec3::new(m.2.0, m.2.1, m.2.2);
        let pos = Vec3::new(m.3.0, m.3.1, m.3.2);
        let half_h = (cam.fov / 2.0).tan() * cam.aspect_ratio;
        let screen: Vec<String> = near
            .iter()
            .map(|(dist, t)| {
                let v = t.feet + Vec3::Y * t.height - pos;
                let z = v.dot(forward);
                if z <= 0.0 {
                    format!("{:#x} {dist:.1} m: behind", t.chr.as_ptr() as usize)
                } else {
                    format!("{:#x} {dist:.1} m: x {:+.2} y {:+.2}", t.chr.as_ptr() as usize, v.dot(right) / (z * half_h), v.dot(up) / (z * (cam.fov / 2.0).tan()))
                }
            })
            .collect();
        glog!(
            "MARK",
            "{name}: pos ({:.2}, {:.2}, {:.2}) {:.2} m from eye, forward ({:.2}, {:.2}, {:.2}), fov {:.1} aspect {:.3} | {}",
            pos.x, pos.y, pos.z,
            (pos - eye).length(),
            forward.x, forward.y, forward.z,
            cam.fov.to_degrees(),
            cam.aspect_ratio,
            screen.join(" | ")
        );
    }
}

/// Applies (on) or restores (off) the Overwatch field of view on every LockCamParam row, in memory.
fn set_lock_cam_fov(saved: &mut Option<Vec<(u32, f32)>>, on: bool) {
    if on == saved.is_some() {
        return;
    }
    let Ok(params) = (unsafe { SoloParamRepository::instance_mut() }) else {
        return;
    };
    match saved.take() {
        None => {
            let fov = vertical_fov(OVERWATCH_FOV).to_degrees();
            let mut old = Vec::new();
            for (id, row) in params.rows_mut::<LockCamParam>() {
                old.push((id, row.cam_fov_y()));
                row.set_cam_fov_y(fov);
            }
            let mut values: Vec<f32> = old.iter().map(|(_, f)| *f).collect();
            values.sort_by(f32::total_cmp);
            values.dedup();
            glog!("CAMERA", "LockCamParam fov -> {fov:.1} on {} rows (were {:?})", old.len(), values);
            *saved = Some(old);
        }
        Some(old) => {
            for (id, row) in params.rows_mut::<LockCamParam>() {
                if let Some((_, fov)) = old.iter().find(|(i, _)| *i == id) {
                    row.set_cam_fov_y(*fov);
                }
            }
            glog!("CAMERA", "LockCamParam fov restored on {} rows", old.len());
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overwatch_103_is_about_70_5_vertical() {
        assert!((vertical_fov(103.0).to_degrees() - 70.53).abs() < 0.05);
        // Elden Ring's own 48 degrees vertical is 76.7 at 16:9 (logged 2026-10-07).
        assert!((vertical_fov(76.7).to_degrees() - 48.0).abs() < 0.05);
    }
}
