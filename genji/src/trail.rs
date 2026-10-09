//! Green effects (DESIGN.md 5l, part 3; you, 2026-10-09: "add the green effect at least on the dash and the dragon
//! swing"), drawn by the overlay under the arms:
//! - a trail behind the blade: the arms thread (arms.rs) records where the katana's (Dragonblade swings, its dash) or
//!   the wakizashi's (Swift Strike) blade is on screen in each picture; the overlay joins the last TRAIL_LIFE seconds
//!   of them into a fading green ribbon. Only where the blade moves fast, so a blade held still leaves nothing;
//! - Swift Strike: green streaks rushing out along the screen's edges for as long as the dash lasts.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::Instant;

use hudhook::imgui::{DrawListMut, ImColor32, Ui};

/// How long a blade position stays in the trail, seconds.
const TRAIL_LIFE: f32 = 0.16;
/// Blade-tip speed (screen heights per second) below which the trail doesn't show, and above which it's full.
const SLOW: f32 = 1.2;
const FAST: f32 = 4.0;
/// A blade tip moving more than this between two pictures (screen heights) is a snap between two animation frames or
/// a cut to another animation: not joined into the ribbon.
const MAX_JUMP: f32 = 0.6;
/// Overwatch's Dragonblade green: a bright core, a deeper glow around it.
const CORE: [u8; 3] = [214, 255, 170];
const GLOW: [u8; 3] = [96, 255, 64];
/// Swift Strike's streaks.
const STREAKS: usize = 28;
const STREAK_FADE_OUT: f32 = 0.2;

/// Where the blade was in one picture: its base (where the blade leaves the guard) and its tip, as shares of the
/// screen (0..1 across and down).
#[derive(Clone, Copy)]
struct Sample {
    at: Instant,
    base: [f32; 2],
    tip: [f32; 2],
    /// Starts a new ribbon (not joined to the one before).
    new_stroke: bool,
}

static SAMPLES: Mutex<VecDeque<Sample>> = Mutex::new(VecDeque::new());
static BROKEN: AtomicBool = AtomicBool::new(true);
static DASHING: AtomicBool = AtomicBool::new(false);
static DASH_TIMES: Mutex<(Option<Instant>, Option<Instant>)> = Mutex::new((None, None));

/// The arms thread: the blade's position in the picture just drawn.
pub fn push(base: [f32; 2], tip: [f32; 2]) {
    let mut s = SAMPLES.lock().unwrap_or_else(|e| e.into_inner());
    let new_stroke = BROKEN.swap(false, Ordering::Relaxed);
    s.push_back(Sample { at: Instant::now(), base, tip, new_stroke });
    while s.len() > 64 {
        s.pop_front();
    }
}

/// The arms thread: no blade trail in this picture; the next one starts a new ribbon.
pub fn pause() {
    BROKEN.store(true, Ordering::Relaxed);
}

/// The game thread, every frame: Swift Strike's dash under way.
pub fn set_dashing(on: bool) {
    if DASHING.swap(on, Ordering::Relaxed) != on {
        let mut t = DASH_TIMES.lock().unwrap_or_else(|e| e.into_inner());
        if on {
            *t = (Some(Instant::now()), None);
        } else {
            t.1 = Some(Instant::now());
        }
    }
}

fn col(c: [u8; 3], a: f32) -> ImColor32 {
    ImColor32::from_rgba(c[0], c[1], c[2], (a.clamp(0.0, 1.0) * 255.0) as u8)
}

fn lerp(a: [f32; 2], b: [f32; 2], f: f32) -> [f32; 2] {
    [a[0] + (b[0] - a[0]) * f, a[1] + (b[1] - a[1]) * f]
}

fn quad(draw: &DrawListMut, p: [[f32; 2]; 4], c: ImColor32) {
    draw.add_triangle(p[0], p[1], p[2], c).filled(true).build();
    draw.add_triangle(p[0], p[2], p[3], c).filled(true).build();
}

/// The overlay, before the arms: the blade trail and Swift Strike's streaks.
pub fn draw(ui: &Ui, width: f32, height: f32) {
    let now = Instant::now();
    let samples: Vec<Sample> = {
        let mut s = SAMPLES.lock().unwrap_or_else(|e| e.into_inner());
        while s.front().is_some_and(|x| now.duration_since(x.at).as_secs_f32() > TRAIL_LIFE) {
            s.pop_front();
        }
        s.iter().copied().collect()
    };
    let draw = ui.get_foreground_draw_list();
    let px = |p: [f32; 2]| [p[0] * width, p[1] * height];
    for pair in samples.windows(2) {
        let (a, b) = (pair[0], pair[1]);
        if b.new_stroke {
            continue;
        }
        let dt = b.at.duration_since(a.at).as_secs_f32().max(1e-3);
        let (dx, dy) = ((b.tip[0] - a.tip[0]) * width / height, b.tip[1] - a.tip[1]);
        let jump = (dx * dx + dy * dy).sqrt();
        if jump > MAX_JUMP {
            continue;
        }
        let speed = jump / dt;
        let fast = ((speed - SLOW) / (FAST - SLOW)).clamp(0.0, 1.0);
        let fresh = 1.0 - now.duration_since(a.at).as_secs_f32() / TRAIL_LIFE;
        let strength = fast * fresh.clamp(0.0, 1.0);
        if strength <= 0.01 {
            continue;
        }
        // The glow over the whole blade, then a brighter core along its outer part.
        quad(&draw, [px(a.base), px(a.tip), px(b.tip), px(b.base)], col(GLOW, 0.35 * strength));
        let (ac, bc) = (lerp(a.base, a.tip, 0.45), lerp(b.base, b.tip, 0.45));
        let (at, bt) = (lerp(a.base, a.tip, 0.97), lerp(b.base, b.tip, 0.97));
        quad(&draw, [px(ac), px(at), px(bt), px(bc)], col(CORE, 0.55 * strength));
    }

    // Swift Strike: streaks rushing outward near the screen's edges, fading in fast and out over STREAK_FADE_OUT.
    let (start, end) = *DASH_TIMES.lock().unwrap_or_else(|e| e.into_inner());
    let Some(start) = start else {
        return;
    };
    let since = now.duration_since(start).as_secs_f32();
    let fade = match end {
        Some(end) => 1.0 - now.duration_since(end).as_secs_f32() / STREAK_FADE_OUT,
        None => (since / 0.05).min(1.0),
    };
    if fade <= 0.0 {
        return;
    }
    let (cx, cy) = (width / 2.0, height / 2.0);
    let reach = (cx * cx + cy * cy).sqrt();
    for i in 0..STREAKS {
        let seed = i as f32 * 12.9898;
        let r1 = (seed.sin() * 43758.547).fract().abs();
        let r2 = ((seed + 4.1).sin() * 24634.635).fract().abs();
        let angle = std::f32::consts::TAU * (i as f32 + r1 * 0.8) / STREAKS as f32;
        // Each streak runs outward and starts again, at its own pace.
        let phase = (since * (1.6 + 1.4 * r2) + r1).fract();
        let inner = reach * (0.42 + 0.5 * phase);
        let len = reach * (0.10 + 0.14 * r2);
        let (dx, dy) = (angle.cos(), angle.sin());
        let p0 = [cx + dx * inner, cy + dy * inner];
        let p1 = [cx + dx * (inner + len), cy + dy * (inner + len)];
        let a = fade * (1.0 - (phase - 0.6).max(0.0) / 0.4) * (0.35 + 0.35 * r1);
        let k = height / 1080.0;
        draw.add_line(p0, p1, col(GLOW, a * 0.6)).thickness((5.0 + 4.0 * r2) * k).build();
        draw.add_line(p0, p1, col(CORE, a)).thickness((1.5 + 1.5 * r2) * k).build();
    }
}
