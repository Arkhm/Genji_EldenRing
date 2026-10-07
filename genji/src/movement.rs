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
//! - Instant deaths with full HP: the black box showed the game's fall timer (`CSChrFallModule.fall_timer`)
//!   counting up the whole time we drove, even standing still with the game saying "on ground", and every death
//!   came at fall timer 12.03 s: the game's "fell into a pit" kill. We zero it every frame while driving (we do
//!   our own falling). This is a different death from the CS2 note's landing death, which zeroing didn't stop
//!   and which the gravity switch-off prevents.
//! - All 32 filter bits see the same surface (no separate low-detail layer); the 0.9 m gap in one test was the
//!   character still dropping onto the ground after the loading screen.

use eldenring::cs::{CSHavokMan, PlayerIns};
use eldenring::position::{HavokPosition, PositionDelta};
use fromsoftware_shared::FromStatic;
use glam::Vec3;

use crate::glog;
use crate::look::Look;
use crate::obstacles::{self, ContactMonitor};

/// Overwatch ground speed for most heroes, metres per second.
const RUN_SPEED: f32 = 5.5;
/// Gravity, metres per second squared. Approximate Overwatch feel, not verified against Overwatch.
const GRAVITY: f32 = 17.0;
/// Jump height, metres. Approximate Overwatch value, not verified.
const JUMP_HEIGHT: f32 = 1.2;
/// Genji's double jump: one extra jump in the air, same height as a normal jump (approximate Overwatch value,
/// not verified), restored on landing.
const AIR_JUMPS: u32 = 1;
/// Genji's wall climb: hold Space in the air while facing a wall to climb it at CLIMB_SPEED for up to
/// CLIMB_TIME seconds (about 7.5 m), restored on landing. Approximate Overwatch values, not verified.
const CLIMB_SPEED: f32 = 7.5;
const CLIMB_TIME: f32 = 1.0;
/// How far in front of the body (beyond its radius) a wall counts as climbable, metres.
const CLIMB_REACH: f32 = 0.3;
/// Height of the ray that looks for the wall to climb (chest), metres above the feet.
const CLIMB_RAY_HEIGHT: f32 = 1.1;
/// When the wall ends at chest height (top reached), the feet are still about CLIMB_RAY_HEIGHT below the edge;
/// this upward speed lifts them about 1.24 m with our gravity (v^2 / 2g), enough to clear it, m/s.
const VAULT_SPEED: f32 = 6.5;
/// After walking off an edge you can still jump for this long, seconds ("coyote time").
const COYOTE_TIME: f32 = 0.1;
/// Height of the top of the head above the feet, for ceiling checks, metres.
const HEAD_HEIGHT: f32 = 1.75;
// No fall damage in Genji mode (your choice, 2026-10-07, like Overwatch). Falls are only logged. An Elden
// Ring-style version (none to 8 m, lethal at 20 m) was built and worked in game; see MODLOG.
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
/// In the air there are no steps to walk up, so we also look for walls at foot level. Without it, a timed jump
/// could carry the feet into the edge of a rock and inside it (5b test, 2026-10-07).
const AIR_WALL_RAY_HEIGHTS: [f32; 4] = [0.1, 0.65, 1.1, 1.6];
/// Before taking over, the game itself must report the character standing on solid ground (not falling) for
/// this long, seconds. Taking over while the character was still dropping onto the ground after a loading screen
/// left the game's animation state "in the air", and touching or jumping into enemies then damaged and staggered
/// them (5b test, 2026-10-07; pressing F9 twice, which lets the game land, made it go away).
const SETTLE_TIME: f32 = 0.3;
/// Take over anyway after waiting this long for the game to report the character settled, seconds.
const SETTLE_GIVE_UP: f32 = 3.0;
/// The game's "standing idle" animation. While we drive, the game's own locomotion is switched off, so its
/// animation stays on idle (more than 600 movement reports in the 5b/5c/5d logs). Any other animation means the
/// game is doing something itself (door, lever, chest, item pickup, flask, hit reaction, being grabbed): we step
/// aside and let it move the character, with the first-person camera following (step 5e).
const IDLE_ANIMATION: i32 = 2_000_000;
/// "No animation" (seen right after loading).
const NO_ANIMATION: i32 = -1;
/// A non-idle animation counts as "the game moves the character" only while its root motion
/// (`CSChrBehaviorModule.root_motion`, the movement the animation itself asks for) is above this. In-place
/// animations (flask `5011x`, weapon swaps `29xxx` / `2029xxx`) stay below it and we keep driving through them.
/// First 5e test (2026-10-07): stepping aside for every non-idle animation, with Elden Ring's walking unblocked,
/// let its own walk/run animations (`2020100`, `2022100`, `2202xxx`) keep the "game action" going for up to 15 s.
/// The unit of root_motion isn't known yet; it's logged with every animation change to tune this.
const ROOT_MOTION_THRESHOLD: f32 = 0.002;
/// A game action ends when its root motion stayed below the threshold for this long, seconds.
const ACTION_STILL_TIME: f32 = 0.3;
/// Landing ray starts this far above the feet, so a surface we've slightly sunk into is still found, metres.
const LANDING_RAY_LIFT: f32 = 0.5;
/// Ray-cast filter from the CS2 field note. It hits *some* map collision, but in one 5a test it hit a surface
/// 0.9 m below the feet (probably the low-detail collision), so the filter is now chosen at takeover from the
/// 32 single bits (see `choose_filter`). This value is only one of the candidates.
const CS2_FILTER: u32 = 0x0200_0058;
/// The chosen filter's surface must be within this distance of where the game holds the feet, metres.
const FILTER_TOLERANCE: f32 = 0.3;
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
    /// Ray-cast filter chosen at takeover (0 = none chosen yet).
    filter: u32,
    choose_log_timer: f32,
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
    /// Highest point since leaving the ground (for fall damage).
    apex_y: f32,
    /// Time since leaving the ground without jumping (coyote time).
    since_left_ground: f32,
    jump_was_down: bool,
    air_jumps_used: u32,
    climbing: bool,
    climb_used: f32,
    contacts: ContactMonitor,
    settled_for: f32,
    waited_for: f32,
    /// Set while the game plays its own animation: (animation, controller clock when it started).
    game_action: Option<(i32, f32)>,
    /// The idle animation seen at the first takeover, in case a stance uses a different idle than 2000000.
    first_idle: Option<i32>,
    last_seen_animation: i32,
    /// Time the root motion has stayed below the threshold during a game action.
    action_still_for: f32,
    character_blocks: u32,
    clock: f32,
    black_box: std::collections::VecDeque<FrameRecord>,
    last_hp: Option<i32>,
}

/// What the controller did this frame.
pub enum Step {
    /// We drive; the character's feet are here.
    Driving(Vec3),
    /// The game is doing something itself (see IDLE_ANIMATION); the feet are here. Keep the first-person view.
    GameAction(Vec3),
    /// Not ready (collision loading, settling after a loading screen) or gave up: the game drives, normal view.
    NotReady,
}

/// Movement wish from the keyboard: forward (+1 W / -1 S), right (+1 D / -1 A), jump held (Space).
pub struct Wish {
    pub forward: f32,
    pub right: f32,
    pub jump: bool,
}

impl Mover {
    /// Moves the character one frame (see `Step`).
    pub fn step(&mut self, player: &mut PlayerIns, look: &Look, wish: &Wish, dt: f32) -> Step {
        let feet_now = {
            let p = player.chr_ins.modules.physics.position;
            Vec3::new(p.0, p.1, p.2)
        };
        if self.gave_up || dt <= 0.0 {
            return Step::NotReady;
        }
        self.clock += dt;

        // 5e: the game's own actions. Step aside only while a non-idle animation moves the character.
        let animation = current_animation(player);
        let root_motion = {
            let r = &player.chr_ins.modules.behavior.root_motion;
            Vec3::new(r.0, r.1, r.2).length()
        };
        let free = animation == IDLE_ANIMATION || animation == NO_ANIMATION || Some(animation) == self.first_idle;
        if animation != self.last_seen_animation {
            glog!(
                "ACTION",
                "animation {} -> {animation} (root motion {root_motion:.4}){}",
                self.last_seen_animation,
                if self.game_action.is_some() { " [game drives]" } else { "" }
            );
            self.last_seen_animation = animation;
        }
        let moves = !free && root_motion > ROOT_MOTION_THRESHOLD;
        match self.game_action {
            None if moves => {
                glog!("ACTION", "game action: animation {animation} moves the character (root motion {root_motion:.4}); the game drives");
                self.release(player);
                self.game_action = Some((animation, self.clock));
                self.action_still_for = 0.0;
                return Step::GameAction(feet_now);
            }
            Some(_) if !free => {
                self.action_still_for = if moves { 0.0 } else { self.action_still_for + dt };
                if self.action_still_for < ACTION_STILL_TIME {
                    return Step::GameAction(feet_now);
                }
                // Still a non-idle animation, but it stopped moving the character: take over again below.
            }
            _ => {}
        }
        // While waiting to take over again after a game action, keep the first-person view.
        let not_ready = if self.game_action.is_some() { Step::GameAction(feet_now) } else { Step::NotReady };

        let Some(world) = (unsafe { CSHavokMan::instance() }).ok() else {
            return not_ready;
        };
        let phys_world: &eldenring::cs::CSPhysWorld = &world.phys_world;

        if !self.active {
            let p = player.chr_ins.modules.physics.position;
            let feet = Vec3::new(p.0, p.1, p.2);
            self.choose_log_timer -= dt;
            let log_now = self.choose_log_timer <= 0.0;
            if log_now {
                self.choose_log_timer = 2.0;
            }
            match choose_filter(phys_world, player, feet, p.3, log_now) {
                Some(filter) => self.filter = filter,
                None => {
                    if self.waiting_frames == 0 {
                        glog!("MOVE", "waiting: no collision surface found right under the feet yet");
                    }
                    self.waiting_frames += 1;
                    return not_ready;
                }
            }
            self.ever_hit = true;
            // The game must have landed the character itself (see SETTLE_TIME).
            let physics = &player.chr_ins.modules.physics;
            let settled = physics.standing_on_solid_ground && !physics.is_falling;
            self.waited_for += dt;
            self.settled_for = if settled { self.settled_for + dt } else { 0.0 };
            if self.settled_for < SETTLE_TIME {
                if self.waited_for < SETTLE_GIVE_UP {
                    return not_ready;
                }
                glog!("MOVE", "game never reported the character settled in {SETTLE_GIVE_UP}s; taking over anyway");
            }
            glog!(
                "MOVE",
                "settled after {:.2}s: on solid ground {} falling {} ground touch state {} animation {}",
                self.waited_for,
                physics.standing_on_solid_ground as u8,
                physics.is_falling as u8,
                player.chr_ins.modules.behavior.ground_touch_state,
                current_animation(player)
            );
            self.settled_for = 0.0;
            self.waited_for = 0.0;
            if self.first_idle.is_none() && animation != NO_ANIMATION {
                self.first_idle = Some(animation);
                glog!("ACTION", "idle animation at first takeover: {animation}");
            }
            if let Some((last, since)) = self.game_action.take() {
                glog!("ACTION", "game action over after {:.2}s (last animation {last}); we drive again", self.clock - since);
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
                "controller ON, ray filter {:#x} (game gravity was x{:.2}, disabled {})",
                self.filter,
                physics.gravity_multiplier,
                physics.gravity_disabled
            );
        }
        self.frames_since_start += 1;

        let p0 = player.chr_ins.modules.physics.position;
        let w = p0.3;
        let start = Vec3::new(p0.0, p0.1, p0.2);
        let mut frame_push = 0.0;
        if let Some(last) = self.last_written {
            let moved = (start - last).length();
            if moved > 3.0 {
                glog!("MOVE", "position jumped {moved:.1} m since our last write (engine re-centred coordinates, or a teleport)");
                // Keep the fall-damage reference in the new coordinates.
                self.apex_y += start.y - last.y;
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
        let others = obstacles::nearby(&player.chr_ins, start);
        let (wanted, char_blocked) = obstacles::clip(start, wanted, RADIUS, HEAD_HEIGHT, &others);
        if char_blocked {
            self.character_blocks += 1;
        }
        let heights: &[f32] = if self.grounded { &WALL_RAY_HEIGHTS } else { &AIR_WALL_RAY_HEIGHTS };
        let horizontal = self.clip_horizontal(phys_world, player, start, wanted, w, heights);
        let mut pos = start + horizontal;

        // Jump: Space pressed (not held) while on the ground, or just after walking off an edge.
        let jump_pressed = wish.jump && !self.jump_was_down;
        self.jump_was_down = wish.jump;
        let can_jump = self.grounded || (self.vy <= 0.0 && self.since_left_ground < COYOTE_TIME);
        if jump_pressed && can_jump {
            self.vy = (2.0 * GRAVITY * JUMP_HEIGHT).sqrt();
            self.grounded = false;
            self.since_left_ground = f32::INFINITY; // no coyote jump after a real jump
            self.fall_time = 0.0;
            self.apex_y = pos.y;
            glog!("MOVE", "jump at ({:.2}, {:.2}, {:.2}), {:.2} m/s up", pos.x, pos.y, pos.z, self.vy);
        } else if jump_pressed && !self.grounded && self.air_jumps_used < AIR_JUMPS {
            self.air_jumps_used += 1;
            self.vy = (2.0 * GRAVITY * JUMP_HEIGHT).sqrt();
            self.since_left_ground = f32::INFINITY;
            self.apex_y = self.apex_y.max(pos.y);
            glog!("MOVE", "double jump at ({:.2}, {:.2}, {:.2}), {:.2} m/s up", pos.x, pos.y, pos.z, self.vy);
        }

        // Vertical: stick to the ground, or fall.
        if self.grounded {
            match ray(phys_world, player, self.filter, pos + Vec3::Y * MAX_STEP, Vec3::NEG_Y * (MAX_STEP + SNAP_DOWN), w) {
                Some(hit) => {
                    self.ever_hit = true;
                    pos.y = hit.y + GROUND_GAP;
                    self.vy = 0.0;
                    self.air_jumps_used = 0;
                    self.climb_used = 0.0;
                }
                None => {
                    self.grounded = false;
                    self.ground_misses += 1;
                    self.since_left_ground = 0.0;
                    self.apex_y = pos.y;
                    glog!("MOVE", "left the ground at ({:.2}, {:.2}, {:.2}); what each filter sees here:", pos.x, pos.y, pos.z);
                    choose_filter(phys_world, player, pos, w, true);
                }
            }
        }
        if !self.grounded {
            self.since_left_ground += dt;
            // Wall climb: Space held, facing a wall within reach, climb time left.
            let wall_ahead = ray(
                phys_world,
                player,
                self.filter,
                pos + Vec3::Y * CLIMB_RAY_HEIGHT,
                look.flat_forward() * (RADIUS + CLIMB_REACH),
                w,
            )
            .is_some();
            let time_left = self.climb_used < CLIMB_TIME;
            if wish.jump && wall_ahead && time_left {
                if !self.climbing {
                    self.climbing = true;
                    glog!("MOVE", "wall climb start at ({:.2}, {:.2}, {:.2}), {:.2}s left", pos.x, pos.y, pos.z, CLIMB_TIME - self.climb_used);
                }
                self.vy = CLIMB_SPEED;
                self.climb_used += dt;
                self.apex_y = self.apex_y.max(pos.y);
            } else if self.climbing {
                self.climbing = false;
                if wish.jump && time_left {
                    // The wall ended in front of the chest: we reached the top. Hop over the edge.
                    self.vy = self.vy.max(VAULT_SPEED);
                    glog!("MOVE", "wall climb: reached the top after {:.2}s, vault", self.climb_used);
                } else {
                    glog!(
                        "MOVE",
                        "wall climb end after {:.2}s ({})",
                        self.climb_used,
                        if !wish.jump { "Space released" } else { "out of climb time" }
                    );
                }
            }
            // Ceiling: stop rising when the head would hit something.
            if self.vy > 0.0
                && ray(phys_world, player, self.filter, pos + Vec3::Y * (HEAD_HEIGHT - 0.2), Vec3::Y * (self.vy * dt + 0.25), w)
                    .is_some()
            {
                glog!("MOVE", "hit a ceiling at {:.1} m/s", self.vy);
                self.vy = 0.0;
                if self.climbing {
                    self.climbing = false;
                    self.climb_used = CLIMB_TIME;
                }
            }
            if !self.climbing {
                self.vy = (self.vy - GRAVITY * dt).max(-MAX_FALL_SPEED);
            }
            self.fall_time += dt;
            let drop = (-self.vy * dt).max(0.0);
            let lift = LANDING_RAY_LIFT;
            match ray(phys_world, player, self.filter, pos + Vec3::Y * lift, Vec3::NEG_Y * (lift + drop), w) {
                Some(hit) if self.vy <= 0.0 => {
                    self.ever_hit = true;
                    glog!(
                        "MOVE",
                        "landed after {:.2}s at {:.1} m/s ({:.2}, {:.2}, {:.2})",
                        self.fall_time, -self.vy, hit.x, hit.y, hit.z
                    );
                    pos.y = hit.y + GROUND_GAP;
                    let fallen = self.apex_y - pos.y;
                    self.vy = 0.0;
                    self.grounded = true;
                    self.fall_time = 0.0;
                    self.air_jumps_used = 0;
                    self.climb_used = 0.0;
                    self.climbing = false;
                    glog!("MOVE", "fell {fallen:.1} m (no fall damage in Genji mode)");
                }
                _ => {
                    pos.y += self.vy * dt;
                    self.apex_y = self.apex_y.max(pos.y);
                }
            }
        }

        // Safety nets.
        if !self.ever_hit && self.frames_since_start >= NO_HIT_GIVE_UP_FRAMES {
            glog!("MOVE", "GAVE UP: no ray hit anything in {} frames (collision filter wrong?)", self.frames_since_start);
            self.give_up(player);
            return Step::NotReady;
        }
        if self.fall_time > MAX_FALL_SECONDS {
            glog!("MOVE", "GAVE UP: falling for {:.1}s; handing back to the game", self.fall_time);
            self.give_up(player);
            return Step::NotReady;
        }

        let physics = &mut player.chr_ins.modules.physics;
        physics.gravity_multiplier = 0.0;
        physics.gravity_disabled = true;
        physics.position = HavokPosition(pos.x, pos.y, pos.z, w);
        physics.chr_proxy_pos_update_requested = true;
        face(physics, look);
        player.chr_ins.modules.fall.fall_timer = 0.0;
        self.last_written = Some(pos);
        self.record(player, pos, frame_push);
        self.contacts.update(pos, RADIUS, &others, current_animation(player));

        // Report.
        self.moved += horizontal.length();
        self.report_timer += dt;
        if self.report_timer >= REPORT_EVERY {
            let speed = self.moved / self.report_timer;
            glog!(
                "MOVE",
                "speed {speed:.2} m/s | yaw {:.1} pitch {:.1} | {} | vy {:.1} | wall hits {} | character blocks {} | outside push {:.2} m | animation {} | pos ({:.2}, {:.2}, {:.2})",
                look.yaw.to_degrees(),
                look.pitch.to_degrees(),
                if self.grounded { "ground" } else { "air" },
                self.vy,
                self.wall_hits,
                self.character_blocks,
                self.outside_push,
                current_animation(player),
                pos.x, pos.y, pos.z
            );
            self.outside_push = 0.0;
            self.report_timer = 0.0;
            self.moved = 0.0;
            self.wall_hits = 0;
            self.character_blocks = 0;
        }
        Step::Driving(pos)
    }

    /// Gives control back to the game (menus, Torrent, ladders, death, F9).
    pub fn release(&mut self, player: &mut PlayerIns) {
        self.settled_for = 0.0;
        self.waited_for = 0.0;
        if !self.active {
            return;
        }
        let physics = &mut player.chr_ins.modules.physics;
        if let Some((multiplier, disabled)) = self.saved_gravity.take() {
            physics.gravity_multiplier = multiplier;
            physics.gravity_disabled = disabled;
        }
        let multiplier = physics.gravity_multiplier;
        player.chr_ins.modules.fall.fall_timer = 0.0;
        self.active = false;
        self.last_written = None;
        self.waiting_frames = 0;
        self.black_box.clear();
        glog!("MOVE", "controller OFF, game gravity restored (x{multiplier:.2})");
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
    fn clip_horizontal(
        &mut self,
        world: &eldenring::cs::CSPhysWorld,
        player: &PlayerIns,
        start: Vec3,
        wanted: Vec3,
        w: f32,
        heights: &[f32],
    ) -> Vec3 {
        let len = wanted.length();
        if len < 1e-5 {
            return Vec3::ZERO;
        }
        let filter = self.filter;
        let free = |delta: Vec3| -> f32 {
            let l = delta.length();
            if l < 1e-5 {
                return 0.0;
            }
            let dir = delta / l;
            let mut allowed = l;
            for &h in heights {
                let origin = start + Vec3::Y * h;
                if let Some(hit) = ray(world, player, filter, origin, dir * (l + RADIUS), w) {
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

/// The animation the game is currently playing on the character (TimeAct animation id).
pub fn current_animation(player: &PlayerIns) -> i32 {
    let time_act = &player.chr_ins.modules.time_act;
    let queue = &time_act.anim_queue;
    queue[time_act.read_idx as usize % queue.len()].anim_id
}

/// Picks the ray filter whose surface is right under the feet: casts a 3 m ray down from 1 m above the feet with
/// each of the 32 single bits (and the CS2 filter), and keeps the one whose hit (+ the 8 cm gap) is closest to the
/// feet, if within `FILTER_TOLERANCE`. Logs the whole table when `log` is set.
fn choose_filter(world: &eldenring::cs::CSPhysWorld, player: &PlayerIns, feet: Vec3, w: f32, log: bool) -> Option<u32> {
    let candidates = std::iter::once(CS2_FILTER).chain((0..32).map(|bit| 1u32 << bit));
    let mut best: Option<(u32, f32)> = None;
    let mut table = Vec::new();
    for filter in candidates {
        let hit = ray(world, player, filter, feet + Vec3::Y, Vec3::NEG_Y * 3.0, w);
        if let Some(h) = hit {
            let off = (h.y + GROUND_GAP - feet.y).abs();
            table.push(format!("{filter:#x}:{:+.2}", h.y - feet.y));
            if best.is_none_or(|(_, b)| off < b) {
                best = Some((filter, off));
            }
        }
    }
    let chosen = best.filter(|(_, off)| *off <= FILTER_TOLERANCE);
    if log || chosen.is_some() {
        let physics = &player.chr_ins.modules.physics;
        glog!(
            "FILTER",
            "feet y {:.2} | game: on solid ground {} touching ground {} adjust-to-hi-collision {} | hits (surface minus feet): {}",
            feet.y,
            physics.standing_on_solid_ground as u8,
            physics.is_touching_ground as u8,
            physics.adjust_to_hi_collision as u8,
            if table.is_empty() { "none".to_string() } else { table.join(" ") }
        );
        match (chosen, best) {
            (Some((f, off)), _) => glog!("FILTER", "chosen {f:#x} (surface {off:.2} m from where the game holds the feet, after the 8 cm gap)"),
            (None, Some((f, off))) => glog!("FILTER", "none chosen: closest is {f:#x}, {off:.2} m off (limit {FILTER_TOLERANCE})"),
            (None, None) => glog!("FILTER", "none chosen: no filter hit anything"),
        }
    }
    chosen.map(|(f, _)| f)
}

/// Casts a ray against the map; returns the hit point.
fn ray(world: &eldenring::cs::CSPhysWorld, player: &PlayerIns, filter: u32, origin: Vec3, delta: Vec3, w: f32) -> Option<Vec3> {
    let hit = world.cast_ray(
        filter,
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

#[cfg(test)]
mod tests {
    use super::*;


    #[test]
    fn vault_clears_the_edge() {
        // Rise from a vault must exceed the distance from the feet to the edge (the climb ray's height).
        let rise = VAULT_SPEED * VAULT_SPEED / (2.0 * GRAVITY);
        assert!(rise > CLIMB_RAY_HEIGHT + 0.05, "vault rise {rise:.2} m vs edge {CLIMB_RAY_HEIGHT} m above the feet");
    }

    #[test]
    fn climb_height_is_about_overwatch() {
        let height = CLIMB_SPEED * CLIMB_TIME;
        assert!((6.0..=9.0).contains(&height), "climb height {height:.1} m");
    }

    #[test]
    fn jump_reaches_jump_height() {
        // Simulate a jump at 60 fps with the same integration as `step` and check the apex.
        let dt = 1.0 / 60.0;
        let mut vy = (2.0 * GRAVITY * JUMP_HEIGHT).sqrt();
        let (mut y, mut apex) = (0.0f32, 0.0f32);
        for _ in 0..200 {
            vy -= GRAVITY * dt;
            y += vy * dt;
            apex = apex.max(y);
        }
        assert!((apex - JUMP_HEIGHT).abs() < 0.08, "apex {apex:.3} m vs {JUMP_HEIGHT} m");
    }
}
