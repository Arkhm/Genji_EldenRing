//! First-person camera (step 4, first version).
//!
//! Elden Ring keeps doing the mouse look: sensitivity, up/down limits and everything else stay the game's own.
//! Every frame, right after the game computes its third-person camera (`CameraStep`), we move that camera to the
//! player's eyes and keep the direction the game chose. The player's own model is hidden so we don't look at
//! the inside of the head.
//!
//! Which camera slot the game draws with isn't known yet: `CSCamera` has four (`pers_cam_1..4`) plus a mask
//! that copies between them. We write `pers_cam_1` and log all four, so the logs tell whether that was right.

use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

use eldenring::cs::{CSCamera, WorldChrMan};
use fromsoftware_shared::{F32Vector4, FromStatic};

use crate::glog;

/// Height of the eyes above the character's feet, in metres.
const EYE_HEIGHT: f32 = 1.6;
/// Camera pushed this far forward (horizontally) from the body's centre, in metres.
const FORWARD_OFFSET: f32 = 0.15;
/// Seconds between camera reports in the log.
const REPORT_EVERY: f32 = 2.0;

static WANTED: AtomicBool = AtomicBool::new(false);
static STATE: Mutex<FirstPerson> = Mutex::new(FirstPerson { active: false, report_timer: 0.0, frames: 0 });

/// Ask for first person on (true) or off (false). Applied by `camera_step`.
pub fn set_first_person(on: bool) {
    WANTED.store(on, Ordering::Relaxed);
}

struct FirstPerson {
    active: bool,
    report_timer: f32,
    frames: u32,
}

/// Runs every frame in the `DrawParamUpdate` task group, right after the game's `CameraStep`.
pub fn camera_step(dt: f32) {
    let mut state = STATE.lock().unwrap_or_else(|e| e.into_inner());
    let wanted = WANTED.load(Ordering::Relaxed);

    let Some(player) = (unsafe { WorldChrMan::instance_mut() }).ok().and_then(|w| w.main_player.as_mut()) else {
        state.active = false;
        return;
    };
    let Ok(camera) = (unsafe { CSCamera::instance_mut() }) else {
        return;
    };

    if !wanted {
        if state.active {
            player.chr_ins.chr_flags1c5.set_enable_render(true);
            state.active = false;
            glog!("CAM", "first person OFF, player model shown again");
        }
        return;
    }

    let feet = player.chr_ins.modules.physics.position;
    let cam = &mut *camera.pers_cam_1;
    let forward = (cam.matrix.2.0, cam.matrix.2.1, cam.matrix.2.2);
    let flat_len = (forward.0 * forward.0 + forward.2 * forward.2).sqrt().max(1e-4);
    let eye = (
        feet.0 + forward.0 / flat_len * FORWARD_OFFSET,
        feet.1 + EYE_HEIGHT,
        feet.2 + forward.2 / flat_len * FORWARD_OFFSET,
    );
    let before = (cam.matrix.3.0, cam.matrix.3.1, cam.matrix.3.2);
    cam.matrix.3 = F32Vector4(eye.0, eye.1, eye.2, 1.0);
    player.chr_ins.chr_flags1c5.set_enable_render(false);

    if !state.active {
        state.active = true;
        state.report_timer = REPORT_EVERY; // report right away
        glog!("CAM", "first person ON");
    }
    state.frames += 1;
    state.report_timer += dt;
    if state.report_timer >= REPORT_EVERY {
        state.report_timer = 0.0;
        let pos = |c: &eldenring::cs::CSPersCam| format!("({:.2}, {:.2}, {:.2})", c.matrix.3.0, c.matrix.3.1, c.matrix.3.2);
        glog!(
            "CAM",
            "frames {} | game cam was ({:.2}, {:.2}, {:.2}) -> eye ({:.2}, {:.2}, {:.2}), feet ({:.2}, {:.2}, {:.2}), forward ({:.2}, {:.2}, {:.2})",
            state.frames,
            before.0, before.1, before.2,
            eye.0, eye.1, eye.2,
            feet.0, feet.1, feet.2,
            forward.0, forward.1, forward.2
        );
        glog!(
            "CAM",
            "slots 1 {} 2 {} 3 {} 4 {} | mask {:#04x} | fov {:.3} aspect {:.3} near {:.3} far {:.0}",
            pos(&camera.pers_cam_1),
            pos(&camera.pers_cam_2),
            pos(&camera.pers_cam_3),
            pos(&camera.pers_cam_4),
            camera.camera_mask,
            camera.pers_cam_1.fov,
            camera.pers_cam_1.aspect_ratio,
            camera.pers_cam_1.near_plane,
            camera.pers_cam_1.far_plane
        );
        state.frames = 0;
    }
}
