//! Green effects (DESIGN.md 5l, parts 3 and 5), drawn by the overlay under the arms:
//! - Dragonblade's trail, redone from your Overwatch recording (clip1, frames 0079-0081; your test of 2026-10-10: the
//!   first one "looks nothing like Overwatch"). Overwatch draws a soft streak along the path the blade just swept: a
//!   white core, a yellow band either side and a green glow fading out, brush strands splaying near the blade; the
//!   white is gone by 0.1 s, the yellow by 0.13 s, the glow by 0.2 s, widening as it fades. The arms thread
//!   (arms.rs) records where the blade is along the animation itself, 120 times a second of it, not only in the
//!   pictures it draws; the ribbon follows a point POINT of the way along the blade. Only where the blade moves fast,
//!   so a blade held still leaves nothing. The katana in Dragonblade's swings and its dash; for now the same ribbon on
//!   the wakizashi in Swift Strike (no recording of Overwatch's dash yet);
//! - Swift Strike: green streaks rushing out along the screen's edges for as long as the dash lasts (not redone yet).

use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::Instant;

use hudhook::imgui::{DrawListMut, ImColor32, TextureId, Ui};

/// How long a blade position stays in the trail, seconds (frame 0081, 0.2 s after the swing: wisps only).
pub const LIFE: f32 = 0.2;
/// Samples of the blade per second of animation (arms.rs).
pub const RATE: f32 = 120.0;
/// The ribbon follows this share of the way from the blade's base to its tip.
const POINT: f32 = 0.8;
/// Blade speed (screen heights per second) below which the trail doesn't show, and above which it's full.
const SLOW: f32 = 1.2;
const FAST: f32 = 4.0;
/// A blade moving more than this between two samples (screen heights) is a snap between two animations: not joined.
const MAX_JUMP: f32 = 0.6;
/// Share of the ribbon's length over which its old end tapers, and how wide its new end stays (at the blade).
const OLD_TAPER: f32 = 0.3;
const NEW_TAPER: f32 = 0.1;
const NEW_END_WIDTH: f32 = 0.15;

/// One layer of the ribbon. Widths in screen heights (the soft texture is solid over about the middle half of it);
/// `grow` is added by the end of LIFE.
struct Layer {
    width: f32,
    grow: f32,
    life: f32,
    strength: f32,
    color: [u8; 3],
}

/// Measured on frame 0079 at the swing (shares of the screen's height): white core 0.03 wide, yellow (250, 255, 110)
/// either side, green glow out to 0.12; 0.1 s later (0080) the newest part solid yellow 0.08 wide, the rest faint
/// green.
const LAYERS: [Layer; 3] = [
    Layer { width: 0.2, grow: 0.08, life: LIFE, strength: 0.6, color: [150, 255, 60] },
    Layer { width: 0.1, grow: 0.07, life: 0.13, strength: 0.95, color: [244, 255, 100] },
    Layer { width: 0.06, grow: 0.0, life: 0.09, strength: 1.0, color: [255, 255, 240] },
];
/// Brush strands where the ribbon frays at the blade: where across the glow each one runs (shares of its width, from
/// the middle) and how far back along the ribbon it reaches (seconds of age).
const STRANDS: [(f32, f32); 5] = [(-0.3, 0.055), (-0.17, 0.08), (-0.05, 0.05), (0.12, 0.07), (0.26, 0.06)];
const STRAND_WIDTH: f32 = 0.014;
const STRAND_COLOR: [u8; 3] = [226, 255, 130];
/// At the blade the strands splay out to this many times their spacing.
const STRAND_SPLAY: f32 = 0.8;

/// Swift Strike's streaks.
const STREAKS: usize = 28;
const STREAK_FADE_OUT: f32 = 0.2;
const CORE: [u8; 3] = [214, 255, 170];
const GLOW: [u8; 3] = [96, 255, 64];

/// Where the blade was at one moment: its base (where the blade leaves the guard) and its tip, as shares of the
/// screen (0..1 across and down).
#[derive(Clone, Copy)]
struct Sample {
    at: Instant,
    base: [f32; 2],
    tip: [f32; 2],
    /// Starts a new ribbon (not joined to the one before).
    new_stroke: bool,
}

/// A sample as the ribbon uses it: its age in seconds instead of a time.
#[derive(Clone, Copy)]
pub struct Point {
    pub age: f32,
    pub base: [f32; 2],
    pub tip: [f32; 2],
    pub new_stroke: bool,
}

/// One piece of ribbon, in pixels: corners left-old, right-old, right-new, left-new. The texture across it (left to
/// right) is the soft ribbon: clear at both edges, full in the middle.
pub struct Quad {
    pub p: [[f32; 2]; 4],
    pub color: [u8; 4],
}

static SAMPLES: Mutex<VecDeque<Sample>> = Mutex::new(VecDeque::new());
static BROKEN: AtomicBool = AtomicBool::new(true);
static DASHING: AtomicBool = AtomicBool::new(false);
static DASH_TIMES: Mutex<(Option<Instant>, Option<Instant>)> = Mutex::new((None, None));

/// The arms thread: where the blade was at `at`.
pub fn push_at(at: Instant, base: [f32; 2], tip: [f32; 2]) {
    let mut s = SAMPLES.lock().unwrap_or_else(|e| e.into_inner());
    let new_stroke = BROKEN.swap(false, Ordering::Relaxed);
    s.push_back(Sample { at, base, tip, new_stroke });
    while s.len() > 160 {
        s.pop_front();
    }
}

/// The arms thread: no blade trail now; the next sample starts a new ribbon.
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

fn smooth(x: f32) -> f32 {
    let x = x.clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

/// 1 when new, 0 at `life` seconds old: holds, then drops (frame 0080: the yellow still solid where 0.08 s old).
fn fade(age: f32, life: f32) -> f32 {
    smooth(1.0 - (age / life).powi(2))
}

/// The ribbon's pieces for `points` (oldest first) on a `width` x `height` screen.
pub fn ribbon(points: &[Point], width: f32, height: f32) -> Vec<Quad> {
    let mut quads = Vec::new();
    let mut start = 0;
    for i in 1..=points.len() {
        let breaks = i == points.len() || points[i].new_stroke || {
            let (a, b) = (lerp(points[i - 1].base, points[i - 1].tip, POINT), lerp(points[i].base, points[i].tip, POINT));
            (((b[0] - a[0]) * width / height).powi(2) + (b[1] - a[1]).powi(2)).sqrt() > MAX_JUMP
        };
        if breaks {
            stroke(&points[start..i], width, height, &mut quads);
            start = i;
        }
    }
    quads
}

/// One unbroken stroke's pieces.
fn stroke(pts: &[Point], width: f32, height: f32, quads: &mut Vec<Quad>) {
    let n = pts.len();
    if n < 2 {
        return;
    }
    let mut c: Vec<[f32; 2]> = pts
        .iter()
        .map(|p| {
            let q = lerp(p.base, p.tip, POINT);
            [q[0] * width, q[1] * height]
        })
        .collect();
    // Smoothed twice (ends kept): the animation's keyframes put kinks in the path.
    for _ in 0..2 {
        let prev = c.clone();
        for j in 1..n - 1 {
            c[j] = [(prev[j - 1][0] + 2.0 * prev[j][0] + prev[j + 1][0]) / 4.0, (prev[j - 1][1] + 2.0 * prev[j][1] + prev[j + 1][1]) / 4.0];
        }
    }
    // Speed (screen heights a second) at each point, from its neighbours, averaged with theirs so it doesn't flicker.
    let raw: Vec<f32> = (0..n)
        .map(|j| {
            let (a, b) = (j.saturating_sub(1), (j + 1).min(n - 1));
            let dt = (pts[a].age - pts[b].age).max(1e-3);
            ((c[b][0] - c[a][0]).powi(2) + (c[b][1] - c[a][1]).powi(2)).sqrt() / height / dt
        })
        .collect();
    let fast: Vec<f32> = (0..n)
        .map(|j| {
            let (a, b) = (j.saturating_sub(1), (j + 1).min(n - 1));
            let speed = (raw[a] + 2.0 * raw[j] + raw[b]) / 4.0;
            ((speed - SLOW) / (FAST - SLOW)).clamp(0.0, 1.0)
        })
        .collect();
    // Along the stroke: tapers at its old end to nothing, at its new end (the blade) to NEW_END_WIDTH.
    let taper: Vec<f32> = (0..n)
        .map(|j| {
            let f = j as f32 / (n - 1) as f32;
            let new = NEW_END_WIDTH + (1.0 - NEW_END_WIDTH) * smooth((1.0 - f) / NEW_TAPER);
            smooth(f / OLD_TAPER) * new
        })
        .collect();
    // The direction across the ribbon at each point.
    let normal: Vec<[f32; 2]> = (0..n)
        .map(|j| {
            let (a, b) = (c[j.saturating_sub(1)], c[(j + 1).min(n - 1)]);
            let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
            let len = (dx * dx + dy * dy).sqrt().max(1e-3);
            [-dy / len, dx / len]
        })
        .collect();
    let mut strip = |centre: &dyn Fn(usize) -> [f32; 2], half: &dyn Fn(usize) -> f32, strength: &dyn Fn(usize) -> f32, color: [u8; 3]| {
        let side = |j: usize| {
            let (p, h) = (centre(j), half(j));
            ([p[0] + normal[j][0] * h, p[1] + normal[j][1] * h], [p[0] - normal[j][0] * h, p[1] - normal[j][1] * h])
        };
        let mut prev = side(0);
        for j in 1..n {
            let cur = side(j);
            let a = 0.5 * (strength(j - 1) + strength(j));
            if a > 0.003 {
                let alpha = (a.clamp(0.0, 1.0) * 255.0) as u8;
                quads.push(Quad { p: [prev.0, prev.1, cur.1, cur.0], color: [color[0], color[1], color[2], alpha] });
            }
            prev = cur;
        }
    };
    let grown = |j: usize, l: &Layer| (l.width + l.grow * (pts[j].age / LIFE).min(1.0)) * height * taper[j];
    for l in &LAYERS {
        strip(&|j| c[j], &|j| grown(j, l) / 2.0, &|j| l.strength * fast[j] * fade(pts[j].age, l.life), l.color);
    }
    let glow = &LAYERS[0];
    for &(across, life) in &STRANDS {
        let offset = |j: usize| {
            let splay = 1.0 + STRAND_SPLAY * (1.0 - pts[j].age / life).clamp(0.0, 1.0);
            let o = across * grown(j, glow) * splay;
            [c[j][0] + normal[j][0] * o, c[j][1] + normal[j][1] * o]
        };
        strip(&offset, &|_| STRAND_WIDTH * height / 2.0, &|j| 0.9 * fast[j] * smooth(1.0 - pts[j].age / life), STRAND_COLOR);
    }
}

/// The overlay, before the arms: the blade trail (with the HUD's soft ribbon texture) and Swift Strike's streaks.
pub fn draw(ui: &Ui, ribbon_texture: Option<TextureId>, width: f32, height: f32) {
    let now = Instant::now();
    let points: Vec<Point> = {
        let mut s = SAMPLES.lock().unwrap_or_else(|e| e.into_inner());
        while s.front().is_some_and(|x| now.saturating_duration_since(x.at).as_secs_f32() > LIFE) {
            s.pop_front();
        }
        s.iter()
            .map(|x| Point { age: now.saturating_duration_since(x.at).as_secs_f32(), base: x.base, tip: x.tip, new_stroke: x.new_stroke })
            .collect()
    };
    let draw = ui.get_foreground_draw_list();
    if let Some(id) = ribbon_texture {
        for q in ribbon(&points, width, height) {
            draw.add_image_quad(id, q.p[0], q.p[1], q.p[2], q.p[3])
                .col(ImColor32::from_rgba(q.color[0], q.color[1], q.color[2], q.color[3]))
                .build();
        }
    }
    dash_streaks(&draw, now, width, height);
}

/// Swift Strike: streaks rushing outward near the screen's edges, fading in fast and out over STREAK_FADE_OUT.
fn dash_streaks(draw: &DrawListMut, now: Instant, width: f32, height: f32) {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn swing(n: usize, newest_age: f32) -> Vec<Point> {
        // A straight fast swing from the upper right to the lower left, as in frame 0079: 6 screen heights a second.
        (0..n)
            .map(|j| {
                let age = newest_age + (n - 1 - j) as f32 / RATE;
                let f = age * 6.0 * 1080.0 / 2580.0;
                let tip = [0.45 + f, 0.6 - f * 0.45];
                Point { age, base: [tip[0] - 0.12, tip[1] + 0.2], tip, new_stroke: false }
            })
            .collect()
    }

    /// The layers last as long as on your recording: white gone by 0.1 s, yellow by 0.13 s, the glow by 0.2 s.
    #[test]
    fn layers_fade_as_in_the_recording() {
        let colours = |age: f32| -> Vec<[u8; 3]> {
            ribbon(&swing(8, age), 2580.0, 1080.0).iter().map(|q| [q.color[0], q.color[1], q.color[2]]).collect()
        };
        let has = |v: &[[u8; 3]], c: [u8; 3]| v.contains(&c);
        let fresh = colours(0.0);
        assert!(has(&fresh, LAYERS[2].color) && has(&fresh, LAYERS[1].color) && has(&fresh, LAYERS[0].color));
        let mid = colours(0.1);
        assert!(!has(&mid, LAYERS[2].color) && has(&mid, LAYERS[1].color) && has(&mid, LAYERS[0].color));
        assert!(colours(0.2).is_empty());
    }

    /// A blade held still leaves nothing; a jump between two animations isn't joined.
    #[test]
    fn still_blades_and_jumps_leave_no_ribbon() {
        let still: Vec<Point> = (0..10).map(|j| Point { age: j as f32 / RATE, base: [0.5, 0.8], tip: [0.6, 0.5], new_stroke: false }).collect();
        assert!(ribbon(&still, 2580.0, 1080.0).is_empty());
        let jump = [
            Point { age: 0.01, base: [0.1, 0.8], tip: [0.1, 0.5], new_stroke: false },
            Point { age: 0.0, base: [0.9, 0.8], tip: [0.9, 0.5], new_stroke: false },
        ];
        assert!(ribbon(&jump, 2580.0, 1080.0).is_empty());
    }

    /// The ribbon drawn the way the overlay draws it (two triangles per piece, the soft texture across), over a grey
    /// like the recording's wall, for comparing with D:\GenjiBridgeAssets\ow_ref\clip1\frames10\0079-0081.jpg. With the
    /// exported arms, Dragonblade's swing at GENJI_TRAIL_T seconds (default 0.25) and 0.1 and 0.2 s after it; writes
    /// trail_<ms>.png next to the export. Run: cargo test --release trail_pictures -- --ignored --nocapture
    #[test]
    #[ignore]
    fn trail_pictures() {
        use crate::viewmodel::{self, Canvas, Viewmodel};
        let dir = std::path::Path::new(viewmodel::DIR);
        let vm = Viewmodel::load(dir).unwrap();
        let katana = vm.blade_line(2).unwrap();
        let anim = vm.find("blade_swing").unwrap();
        let (w, h) = (1290usize, 540usize);
        let mut canvas = Canvas::new(w, h);
        canvas.parts = crate::arms_anim::parts_for("blade_swing");
        canvas.eye = viewmodel::EYE + glam::Vec3::Z * crate::arms::BLADE_DROP;
        let at: f32 = std::env::var("GENJI_TRAIL_T").ok().and_then(|v| v.parse().ok()).unwrap_or(0.25);
        for later in [0.0f32, 0.1, 0.2] {
            // The blade 120 times a second up to `at`; shown `later` seconds after (the swing goes on meanwhile).
            let now = at + later;
            let mut points = Vec::new();
            let mut t = (now - LIFE).max(0.0);
            while t <= now + 1e-4 {
                let pose = vm.pose(anim, t, false);
                if let Some((base, tip)) = katana.on_screen(&pose, &canvas) {
                    points.push(Point { age: now - t, base, tip, new_stroke: false });
                }
                t += 1.0 / RATE;
            }
            let quads = ribbon(&points, w as f32, h as f32);
            let mut rgb = vec![[150f32, 148.0, 135.0]; w * h];
            for q in &quads {
                let a = q.color[3] as f32 / 255.0;
                let c = [q.color[0] as f32, q.color[1] as f32, q.color[2] as f32];
                for (tri, us) in [([q.p[0], q.p[1], q.p[2]], [0.0, 1.0, 1.0]), ([q.p[0], q.p[2], q.p[3]], [0.0, 1.0, 0.0])] {
                    fill_triangle(&mut rgb, w, h, tri, us, c, a);
                }
            }
            canvas.clear();
            viewmodel::draw(&vm, &vm.pose(anim, now, false), &mut canvas, 4);
            let mut out = Vec::with_capacity(w * h * 3);
            for (i, px) in canvas.rgba.chunks_exact(4).enumerate() {
                let a = px[3] as f32 / 255.0;
                for k in 0..3 {
                    out.push((px[k] as f32 * a + rgb[i][k] * (1.0 - a)) as u8);
                }
            }
            let path = dir.join(format!("trail_{:03}.png", (later * 1000.0).round() as u32));
            let file = std::fs::File::create(&path).unwrap();
            let mut enc = png::Encoder::new(std::io::BufWriter::new(file), w as u32, h as u32);
            enc.set_color(png::ColorType::Rgb);
            enc.write_header().unwrap().write_image_data(&out).unwrap();
            println!("{}: {} samples, {} pieces", path.display(), points.len(), quads.len());
        }
    }

    /// One triangle, `u` across the ribbon interpolated over it, the soft texture's alpha (as hud.rs makes it) times
    /// `a`, blended over `rgb`.
    fn fill_triangle(rgb: &mut [[f32; 3]], w: usize, h: usize, p: [[f32; 2]; 3], u: [f32; 3], c: [f32; 3], a: f32) {
        let area = (p[1][0] - p[0][0]) * (p[2][1] - p[0][1]) - (p[2][0] - p[0][0]) * (p[1][1] - p[0][1]);
        if area.abs() < 1e-6 {
            return;
        }
        let (x0, x1) = (p.iter().map(|v| v[0]).fold(f32::MAX, f32::min).max(0.0), p.iter().map(|v| v[0]).fold(f32::MIN, f32::max).min(w as f32 - 1.0));
        let (y0, y1) = (p.iter().map(|v| v[1]).fold(f32::MAX, f32::min).max(0.0), p.iter().map(|v| v[1]).fold(f32::MIN, f32::max).min(h as f32 - 1.0));
        for y in y0 as usize..=y1.max(0.0) as usize {
            for x in x0 as usize..=x1.max(0.0) as usize {
                let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
                let w0 = ((p[1][0] - px) * (p[2][1] - py) - (p[2][0] - px) * (p[1][1] - py)) / area;
                let w1 = ((p[2][0] - px) * (p[0][1] - py) - (p[0][0] - px) * (p[2][1] - py)) / area;
                let w2 = 1.0 - w0 - w1;
                if w0 < 0.0 || w1 < 0.0 || w2 < 0.0 {
                    continue;
                }
                let uu = w0 * u[0] + w1 * u[1] + w2 * u[2];
                let k = a * smooth(1.0 - (2.0 * uu - 1.0).abs());
                let d = &mut rgb[y * w + x];
                for i in 0..3 {
                    d[i] += (c[i] - d[i]) * k;
                }
            }
        }
    }
}
