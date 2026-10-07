//! Where Genji looks: yaw (left/right) and pitch (up/down), driven by the mouse, Overwatch-style.
//!
//! The axis conventions are taken from the game's own camera when we take over: the starting direction is the
//! game's forward vector, and whether "turning right" increases or decreases yaw is decided by comparing our
//! right vector with the game's (logged as `LOOK`).

use glam::Vec3;
use fromsoftware_shared::F32ViewMatrix;

use crate::glog;

/// Overwatch's mouse formula: degrees turned per mouse count = 0.0066 x sensitivity.
const OW_DEGREES_PER_COUNT: f32 = 0.0066;
/// Overwatch sensitivity: the player's own Overwatch setting (6.90, told 2026-10-07).
pub const OW_SENSITIVITY: f32 = 6.9;
/// Pitch limit, degrees.
const PITCH_LIMIT: f32 = 89.0;

#[derive(Clone, Copy)]
pub struct Look {
    pub yaw: f32,
    pub pitch: f32,
    /// +1 or -1: which way yaw turns relative to the game's "right".
    right_sign: f32,
}

impl Look {
    /// Starts from the game's current camera, or from the character's facing (`body_forward`) when the game's
    /// camera isn't set up yet (right after loading it sits at 0,0,0 facing +Z: seen in the step 4 and 5a logs).
    pub fn from_game_camera(m: &F32ViewMatrix, body_forward: Vec3) -> Self {
        let camera_ready = m.3.0 != 0.0 || m.3.1 != 0.0 || m.3.2 != 0.0;
        let forward = if camera_ready {
            Vec3::new(m.2.0, m.2.1, m.2.2).normalize_or(Vec3::Z)
        } else {
            glog!("LOOK", "game camera not set up yet; starting from the character's facing");
            body_forward.normalize_or(Vec3::Z)
        };
        let game_right = Vec3::new(m.0.0, m.0.1, m.0.2);
        let game_up = Vec3::new(m.1.0, m.1.1, m.1.2);
        let yaw = forward.x.atan2(forward.z);
        let pitch = forward.y.clamp(-1.0, 1.0).asin();
        let ours = Vec3::new(yaw.cos(), 0.0, -yaw.sin());
        let right_sign = if ours.dot(game_right) >= 0.0 { 1.0 } else { -1.0 };
        let look = Self { yaw, pitch, right_sign };
        glog!(
            "LOOK",
            "start yaw {:.1} pitch {:.1} deg, right sign {right_sign:+}, right match {:.3}, up match {:.3}, forward match {:.3}",
            yaw.to_degrees(),
            pitch.to_degrees(),
            look.right().dot(game_right),
            look.up().dot(game_up),
            look.forward().dot(forward)
        );
        look
    }

    /// Applies mouse movement (DirectInput counts; x right positive, y down positive).
    pub fn turn(&mut self, dx: i32, dy: i32) {
        let k = (OW_DEGREES_PER_COUNT * OW_SENSITIVITY).to_radians();
        self.yaw = wrap(self.yaw + self.right_sign * k * dx as f32);
        let limit = PITCH_LIMIT.to_radians();
        self.pitch = (self.pitch - k * dy as f32).clamp(-limit, limit);
    }

    pub fn forward(&self) -> Vec3 {
        let (sy, cy) = self.yaw.sin_cos();
        let (sp, cp) = self.pitch.sin_cos();
        Vec3::new(sy * cp, sp, cy * cp)
    }

    /// Forward along the ground (no pitch), length 1.
    pub fn flat_forward(&self) -> Vec3 {
        Vec3::new(self.yaw.sin(), 0.0, self.yaw.cos())
    }

    /// Right along the ground, length 1. Same direction as the game's "right".
    pub fn right(&self) -> Vec3 {
        Vec3::new(self.yaw.cos(), 0.0, -self.yaw.sin()) * self.right_sign
    }

    pub fn up(&self) -> Vec3 {
        let up = self.forward().cross(self.right()).normalize_or(Vec3::Y);
        if up.y < 0.0 { -up } else { up }
    }

    /// Writes our rotation and the eye position into a camera matrix (keeps the matrix's 4th components).
    pub fn write_camera(&self, m: &mut F32ViewMatrix, eye: Vec3) {
        let (r, u, f) = (self.right(), self.up(), self.forward());
        (m.0.0, m.0.1, m.0.2) = (r.x, r.y, r.z);
        (m.1.0, m.1.1, m.1.2) = (u.x, u.y, u.z);
        (m.2.0, m.2.1, m.2.2) = (f.x, f.y, f.z);
        (m.3.0, m.3.1, m.3.2) = (eye.x, eye.y, eye.z);
    }
}

fn wrap(angle: f32) -> f32 {
    let tau = std::f32::consts::TAU;
    (angle + std::f32::consts::PI).rem_euclid(tau) - std::f32::consts::PI
}
