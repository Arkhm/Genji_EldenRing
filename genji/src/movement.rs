//! Genji's movement controller, sub-step 5a: walking at Overwatch speed relative to where you look, with
//! collision from ray casts against the map, our own gravity, and the game's gravity switched off.
//!
//! Each frame we read the character's current position, add our movement, and write it back. We never keep
//! the absolute position, because the engine re-centres its coordinates while you play.
//!
//! Why the game's gravity must be off while we drive (CS2 field note, same game version): with it on, the
//! physics body builds up a hidden falling speed while we write positions, and touching down at that speed
//! plays the scripted fall death.
//!
//! Safety nets: if no ray ever hits anything (wrong collision filter), or the character falls for more than
//! 5 s, the controller gives up and hands control back to the game for the rest of the session.
//!
//! Learned in the 5a tests (2026-10-07):
//! - Right after a loading screen the map's collision isn't loaded yet: every ray misses for the first frames
//!   (probe: all miss on frame 1, filter 0x2000058 hits on frame 20). So we only take over once a ray straight
//!   down hits the ground; until then the game keeps control.
//! - The game keeps the character's feet about 8 cm above the surface our rays hit (we wrote 1.13, the game
//!   showed 1.21, every frame). Snapping exactly onto the hit pushed the body into the ground every frame and
//!   the game pushed it back out (2.4 m of "outside push" per 0.5 s while standing still). We now keep that gap.
//! - An unexplained instant death (HP 760 -> 0 in one second, no damage) while walking downhill. A record of the
//!   last 60 frames is written to the log whenever HP drops, to find out why.

use eldenring::cs::{CSHavokMan, PlayerIns};
use eldenring::position::{HavokPosition, PositionDelta};
use fromsoftware_shared::FromStatic;
use glam::Vec3;

use crate::glog;
use crate::look::Look;

/// Overwatch ground speed for most heroes, metres per second.
const RUN_SPEED: f32 = 5.5;
/// Placeholder gravity until 5b tunes jumps, metres per second squared.
const GRAVITY: f32 = 17.0;
const MAX_FALL_SPEED: f32 = 55.0;
/// Highest step the character walks up without jumping, metres.
const MAX_STEP: f32 = 0.5;
/// How far below the feet we still "stick" to the ground when walking downhill, metres.
const SNAP_DOWN: f32 = 0.6;
/// Gap the game keeps between the feet and the surface our rays hit, metres (measured: 0.08).
const GROUND_GAP: f32 = 0.08;
/// Body radius kept between the character's centre and walls, metres.
const RADIUS: f32 = 0.35;
/// Heights (above the feet) of the rays that look for walls. All above MAX_STEP so steps don't count as walls.
const WALL_RAY_HEIGHTS: [f32; 3] = [0.65, 1.1, 1.6];
/// Ray-cast filter that hits map collision (CS2 field note).
const MAP_FILTER: u32 = 0x0200_0058;
/// Give up if the ground ray hasn't hit anything this many frames after taking over.
const NO_HIT_GIVE_UP_FRAMES: u32 = 30;
const MAX_FALL_SECONDS: f32 = 5.0;
const REPORT_EVERY: f32 = 0.5;
/// Frames kept in the black box.
const BLACK_BOX_FRAMES: usize = 60;

/// One frame of movement state, for the black box.
#[derive(Clone, Copy, Default)]
struct FrameRecord {
    time: f32,
    pos: Vec3,
    outside_push: f32,
    vy: f32,
    grounded: bool,
    game_falling: bool,
    game_on_ground: bool,
    fall_timer: f32,
    hp: i32,
}

#[derive(Default)]
pub struct Mover {
    active: bool,
    /// Set when a safety net fired; the controller stays off for the session.
    pub gave_up: bool,
    vy: f32,
    grounded: bool,
    fall_time: f32,
    frames_since_start: u32,
    ever_hit: bool,
    saved_gravity: Option<(f32, bool)>,
    report_timer: f32,
    moved: f32,
    wall_hits: u32,
    ground_misses: u32,
    /// Where we put the character last frame, to measure movement that didn't come from us.
    last_written: Option<Vec3>,
    /// Distance the character moved since our last write, without us (game movement leaking through,
    /// knockback, ...). Jumps over 3 m are counted as the engine re-centring its coordinates instead.
    outside_push: f32,
    /// Waiting for the map's collision to load before taking over.
    waiting_frames: u32,
    clock: f32,
    black_box: std::collections::VecDeque<FrameRecord>,
    last_hp: Option<i32>,
}

/// Movement wish from the keyboard: forward (+1 W / -1 S), right (+1 D / -1 A).
pub struct Wish {
    pub forward: f32,
    pub right: f32,
}

impl Mover {
    /// Moves the character one frame. Returns the new feet position, or None if the controller is not driving.
    pub fn step(&mut self, player: &mut PlayerIns, look: &Look, wish: &Wish, dt: f32) -> Option<Vec3> {
        if self.gave_up || dt <= 0.0 {
            return None;
        }
        let world = unsafe { CSHavokMan::instance() }.ok()?;
        let phys_world: &eldenring::cs::CSPhysWorld = &world.phys_world;

        self.clock += dt;
        if !self.active {
            let p = player.chr_ins.modules.physics.position;
            let feet = Vec3::new(p.0, p.1, p.2);
            if ray(phys_world, player, feet + Vec3::Y, Vec3::NEG_Y * 3.0, p.3).is_none() {
                if self.waiting_frames == 0 {
                    glog!("MOVE", "waiting for the map's collision to load before taking over");
                }
                self.waiting_frames += 1;
                return None;
            }
            if self.waiting_frames > 0 {
                glog!("MOVE", "collision ready after {} frames of waiting", self.waiting_frames);
                self.waiting_frames = 0;
            }
            let physics = &player.chr_ins.modules.physics;
            self.saved_gravity = Some((physics.gravity_multiplier, physics.gravity_disabled));
            self.active = true;
            self.vy = 0.0;
            self.grounded = true;
            self.fall_time = 0.0;
            self.frames_since_start = 0;
            self.report_timer = 0.0;
            glog!(
                "MOVE",
                "controller ON (game gravity was x{:.2}, disabled {})",
                physics.gravity_multiplier,
                physics.gravity_disabled
            );
        }
        self.frames_since_start += 1;
        if self.frames_since_start == 1 {
            probe(phys_world, player, world);
        }

        let p0 = player.chr_ins.modules.physics.position;
        let w = p0.3;
        let start = Vec3::new(p0.0, p0.1, p0.2);
        let mut frame_push = 0.0;
        if let Some(last) = self.last_written {
            let moved = (start - last).length();
            if moved > 3.0 {
                glog!("MOVE", "position jumped {moved:.1} m since our last write (engine re-centred coordinates, or a teleport)");
            } else {
                self.outside_push += moved;
                frame_push = moved;
            }
        }

        // Horizontal: wish direction relative to where we look, along the ground.
        let mut dir = look.flat_forward() * wish.forward + look.right() * wish.right;
        if dir.length_squared() > 1e-6 {
            dir = dir.normalize();
        }
        let wanted = dir * RUN_SPEED * dt;
        let horizontal = self.clip_horizontal(phys_world, player, start, wanted, w);
        let mut pos = start + horizontal;

        // Vertical: stick to the ground, or fall.
        if self.grounded {
            match ray(phys_world, player, pos + Vec3::Y * MAX_STEP, Vec3::NEG_Y * (MAX_STEP + SNAP_DOWN), w) {
                Some(hit) => {
                    self.ever_hit = true;
                    pos.y = hit.y + GROUND_GAP;
                    self.vy = 0.0;
                }
                None => {
                    self.grounded = false;
                    self.ground_misses += 1;
                    glog!("MOVE", "left the ground at ({:.2}, {:.2}, {:.2})", pos.x, pos.y, pos.z);
                }
            }
        }
        if !self.grounded {
            self.vy = (self.vy - GRAVITY * dt).max(-MAX_FALL_SPEED);
            self.fall_time += dt;
            let drop = (-self.vy * dt).max(0.0);
            let lift = 0.3;
            match ray(phys_world, player, pos + Vec3::Y * lift, Vec3::NEG_Y * (lift + drop), w) {
                Some(hit) if self.vy <= 0.0 => {
                    self.ever_hit = true;
                    glog!(
                        "MOVE",
                        "landed after {:.2}s at {:.1} m/s ({:.2}, {:.2}, {:.2})",
                        self.fall_time, -self.vy, hit.x, hit.y, hit.z
                    );
                    pos.y = hit.y + GROUND_GAP;
                    self.vy = 0.0;
                    self.grounded = true;
                    self.fall_time = 0.0;
                }
                _ => pos.y += self.vy * dt,
            }
        }

        // Safety nets.
        if !self.ever_hit && self.frames_since_start >= NO_HIT_GIVE_UP_FRAMES {
            glog!("MOVE", "GAVE UP: no ray hit anything in {} frames (collision filter wrong?)", self.frames_since_start);
            self.give_up(player);
            return None;
        }
        if self.fall_time > MAX_FALL_SECONDS {
            glog!("MOVE", "GAVE UP: falling for {:.1}s; handing back to the game", self.fall_time);
            self.give_up(player);
            return None;
        }

        let physics = &mut player.chr_ins.modules.physics;
        physics.gravity_multiplier = 0.0;
        physics.gravity_disabled = true;
        physics.position = HavokPosition(pos.x, pos.y, pos.z, w);
        physics.chr_proxy_pos_update_requested = true;
        face(physics, look);
        self.last_written = Some(pos);
        self.record(player, pos, frame_push);

        // Report.
        self.moved += horizontal.length();
        self.report_timer += dt;
        if self.report_timer >= REPORT_EVERY {
            let speed = self.moved / self.report_timer;
            glog!(
                "MOVE",
                "speed {speed:.2} m/s | yaw {:.1} pitch {:.1} | {} | vy {:.1} | wall hits {} | outside push {:.2} m | pos ({:.2}, {:.2}, {:.2})",
                look.yaw.to_degrees(),
                look.pitch.to_degrees(),
                if self.grounded { "ground" } else { "air" },
                self.vy,
                self.wall_hits,
                self.outside_push,
                pos.x, pos.y, pos.z
            );
            self.outside_push = 0.0;
            self.report_timer = 0.0;
            self.moved = 0.0;
            self.wall_hits = 0;
        }
        Some(pos)
    }

    /// Gives control back to the game (menus, Torrent, ladders, death, F9).
    pub fn release(&mut self, player: &mut PlayerIns) {
        if !self.active {
            return;
        }
        let physics = &mut player.chr_ins.modules.physics;
        if let Some((multiplier, disabled)) = self.saved_gravity.take() {
            physics.gravity_multiplier = multiplier;
            physics.gravity_disabled = disabled;
        }
        self.active = false;
        self.last_written = None;
        self.waiting_frames = 0;
        self.black_box.clear();
        glog!("MOVE", "controller OFF, game gravity restored (x{:.2})", physics.gravity_multiplier);
    }

    /// Adds this frame to the black box; dumps the black box to the log when HP drops.
    fn record(&mut self, player: &PlayerIns, pos: Vec3, outside_push: f32) {
        let modules = &player.chr_ins.modules;
        let rec = FrameRecord {
            time: self.clock,
            pos,
            outside_push,
            vy: self.vy,
            grounded: self.grounded,
            game_falling: modules.physics.is_falling,
            game_on_ground: modules.physics.standing_on_solid_ground,
            fall_timer: modules.fall.fall_timer,
            hp: modules.data.hp,
        };
        if self.black_box.len() == BLACK_BOX_FRAMES {
            self.black_box.pop_front();
        }
        self.black_box.push_back(rec);
    }

    /// Call every frame, before anything else: when HP drops, writes the black box to the log.
    pub fn watch_hp(&mut self, player: &PlayerIns) {
        let hp = player.chr_ins.modules.data.hp;
        if let Some(last) = self.last_hp
            && hp < last
            && !self.black_box.is_empty()
        {
            glog!("BLACKBOX", "HP dropped {last} -> {hp}; last {} frames (t = controller clock):", self.black_box.len());
            for r in &self.black_box {
                glog!(
                    "BLACKBOX",
                    "t {:.3} pos ({:.2}, {:.2}, {:.2}) push {:.3} vy {:.1} ours {} | game falling {} on ground {} fall timer {:.2} | hp {}",
                    r.time, r.pos.x, r.pos.y, r.pos.z, r.outside_push, r.vy,
                    if r.grounded { "ground" } else { "air" },
                    r.game_falling as u8, r.game_on_ground as u8, r.fall_timer, r.hp
                );
            }
        }
        self.last_hp = Some(hp);
    }

    fn give_up(&mut self, player: &mut PlayerIns) {
        self.release(player);
        self.gave_up = true;
    }

    /// How much of `wanted` (horizontal) can be moved without entering a wall. Slides along walls by trying
    /// the two ground axes separately when the full move is blocked.
    fn clip_horizontal(&mut self, world: &eldenring::cs::CSPhysWorld, player: &PlayerIns, start: Vec3, wanted: Vec3, w: f32) -> Vec3 {
        let len = wanted.length();
        if len < 1e-5 {
            return Vec3::ZERO;
        }
        let free = |delta: Vec3| -> f32 {
            let l = delta.length();
            if l < 1e-5 {
                return 0.0;
            }
            let dir = delta / l;
            let mut allowed = l;
            for h in WALL_RAY_HEIGHTS {
                let origin = start + Vec3::Y * h;
                if let Some(hit) = ray(world, player, origin, dir * (l + RADIUS), w) {
                    let dist = Vec3::new(hit.x - origin.x, 0.0, hit.z - origin.z).length() - RADIUS;
                    allowed = allowed.min(dist.max(0.0));
                }
            }
            allowed
        };
        let full = free(wanted);
        if full >= len * 0.999 {
            return wanted;
        }
        self.wall_hits += 1;
        let along_x = Vec3::new(wanted.x, 0.0, 0.0);
        let along_z = Vec3::new(0.0, 0.0, wanted.z);
        let candidates = [
            wanted.normalize() * full,
            along_x.normalize_or_zero() * free(along_x),
            along_z.normalize_or_zero() * free(along_z),
        ];
        candidates.into_iter().max_by(|a, b| a.length().total_cmp(&b.length())).unwrap_or(Vec3::ZERO)
    }
}

/// One-off diagnostic: casts a 3 m ray straight down from 1 m above the feet with several collision filters
/// and several values for the position's 4th component, and logs which combinations hit. Added after the
/// first 5a test (2026-10-07), where every ray with filter 0x2000058 missed, even straight down at the ground.
fn probe(world: &eldenring::cs::CSPhysWorld, player: &PlayerIns, havok_man: &CSHavokMan) {
    const FILTERS: [u32; 8] = [0x0200_0058, 0xFFFF_FFFF, 0x0000_0058, 0x0200_0000, 0x0000_0001, 0x0000_0000, 0x0000_0040, 0x0000_0010];
    let p = player.chr_ins.modules.physics.position;
    let origin = Vec3::new(p.0, p.1 + 1.0, p.2);
    glog!(
        "PROBE",
        "feet ({:.2}, {:.2}, {:.2}) w {:.3} | CSHavokMan {:p} phys_world {:p}",
        p.0, p.1, p.2, p.3,
        havok_man as *const CSHavokMan,
        world as *const eldenring::cs::CSPhysWorld
    );
    for w in [p.3, 0.0, 1.0] {
        let results: Vec<String> = FILTERS
            .iter()
            .map(|&filter| {
                match world.cast_ray(filter, &HavokPosition(origin.x, origin.y, origin.z, w), PositionDelta(0.0, -3.0, 0.0), player) {
                    Some(hit) => format!("{filter:#x}:HIT y={:.2}", hit.1),
                    None => format!("{filter:#x}:miss"),
                }
            })
            .collect();
        glog!("PROBE", "w={w:.3}: {}", results.join(" "));
    }
}

/// Casts a ray against the map; returns the hit point.
fn ray(world: &eldenring::cs::CSPhysWorld, player: &PlayerIns, origin: Vec3, delta: Vec3, w: f32) -> Option<Vec3> {
    let hit = world.cast_ray(
        MAP_FILTER,
        &HavokPosition(origin.x, origin.y, origin.z, w),
        PositionDelta(delta.x, delta.y, delta.z),
        player,
    )?;
    Some(Vec3::new(hit.0, hit.1, hit.2))
}

/// Turns the (hidden) body to face where we look. The character's forward is its rotation applied to (0, 0, -1)
/// (fromsoftware-rs debug-line example).
fn face(physics: &mut eldenring::cs::CSChrPhysicsModule, look: &Look) {
    let f = look.flat_forward();
    let angle = (-f.x).atan2(-f.z);
    let q = glam::Quat::from_rotation_y(angle);
    physics.orientation = eldenring::rotation::Quaternion(q.x, q.y, q.z, q.w);
}
