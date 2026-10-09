//! Step 10c (DESIGN.md 5k): Genji's arms in game. A thread of our own ("genji-arms") draws the arms with
//! `viewmodel.rs` at half the screen's size, split over ARM_THREADS cores, whenever the game thread asks for a pose;
//! the overlay (crosshair.rs) copies each finished picture into a texture and stretches it over the screen, under
//! the HUD and the crosshair. The game thread never waits for the drawing.
//!
//! What to draw comes from arms_anim.rs (step 10d): a clip, blended from the previous one for a moment. The same pose
//! asked for again isn't drawn again (a resting pose stays in the texture as it is).
//!
//! Logged every few seconds: pictures drawn, average and longest drawing time (ARMS lines); the overlay logs its copy
//! time the same way.

use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Condvar, Mutex, OnceLock};
use std::time::{Duration, Instant};

use crate::arms_anim::{Clip, Pose};
use crate::glog;
use crate::viewmodel::{self, Canvas, Viewmodel};

/// Cores the drawing is split over (the game keeps the rest).
const ARM_THREADS: usize = 4;
/// Timing report every this many seconds.
const REPORT_EVERY: Duration = Duration::from_secs(5);

#[derive(Clone, Copy, PartialEq)]
struct Request {
    pose: Pose,
    /// Metres the arms are drawn lower (the camera raised): see BLADE_DROP.
    drop: f32,
    /// Radians the arms are turned down (climbing: the camera's pitch).
    tilt: f32,
}

/// Each role's variants' lengths in seconds, known once the drawing thread has loaded the export.
static LENGTHS: OnceLock<HashMap<String, Vec<f32>>> = OnceLock::new();

/// (seconds of `role`'s variant `variant`, number of variants), once the export is loaded (game thread).
pub fn length(role: &str, variant: usize) -> Option<(f32, usize)> {
    let v = LENGTHS.get()?.get(role)?;
    Some((v[variant % v.len()], v.len()))
}

/// The Dragonblade stance is drawn this much lower (your test of 2026-10-09: on your 3440x1440 screen the left arm's
/// cut end showed at the bottom; at 4 cm a sliver still shows, at 6 cm none). camera.rs slides it in and out.
pub const BLADE_DROP: f32 = 0.06;

struct Shared {
    request: Option<Request>,
    frame: Option<Arc<Vec<u8>>>,
    seq: u64,
    /// The area that changed since the overlay last took a picture (old arms to clear and new ones), so it copies
    /// only that (None: nothing changed).
    dirty: Option<[u32; 4]>,
}

/// F6 switches the arms off and on (a test switch: your test of 2026-10-09 saw the game's light flicker, to tell
/// whether the arms cause it).
static SHOWN: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(true);

pub fn toggle() {
    let on = !SHOWN.fetch_xor(true, std::sync::atomic::Ordering::Relaxed);
    glog!("ARMS", "F6: arms {}", if on { "shown" } else { "hidden" });
}

pub fn shown() -> bool {
    SHOWN.load(std::sync::atomic::Ordering::Relaxed)
}

static SHARED: Mutex<Shared> = Mutex::new(Shared { request: None, frame: None, seq: 0, dirty: None });
static WAKE: Condvar = Condvar::new();

/// Starts the drawing thread for pictures of `w` x `h` (called once by the overlay).
pub fn start(w: usize, h: usize) {
    let spawned = std::thread::Builder::new().name("genji-arms".into()).spawn(move || run(w, h));
    if let Err(e) = spawned {
        glog!("ARMS", "drawing thread not started ({e}); no arms this session");
    }
}

/// Asks for the arms in `pose` (arms_anim.rs), drawn `drop` metres lower and turned down by `tilt` radians (game
/// thread, every frame while first person shows).
pub fn request(pose: Pose, drop: f32, tilt: f32) {
    let mut s = SHARED.lock().unwrap_or_else(|e| e.into_inner());
    s.request = Some(Request { pose, drop, tilt });
    WAKE.notify_one();
}

/// The newest finished picture if newer than `seen`, with the area to copy ([x0, y0, x1, y1); None: nothing)
/// (overlay).
pub fn latest(seen: u64) -> Option<(u64, Arc<Vec<u8>>, Option<[u32; 4]>)> {
    let mut s = SHARED.lock().unwrap_or_else(|e| e.into_inner());
    if s.seq <= seen {
        return None;
    }
    let dirty = s.dirty.take();
    s.frame.clone().map(|f| (s.seq, f, dirty))
}

fn run(w: usize, h: usize) {
    let t = Instant::now();
    let vm = match Viewmodel::load(Path::new(viewmodel::DIR)) {
        Ok(vm) => vm,
        Err(e) => {
            glog!("ARMS", "model not loaded from {} ({e}); run tools\\export_viewmodel.py. No arms this session", viewmodel::DIR);
            return;
        }
    };
    glog!("ARMS", "model and {} animations loaded in {:?}; drawing at {w}x{h} on {ARM_THREADS} cores", vm.anims.len(), t.elapsed());
    let mut lengths: HashMap<String, Vec<f32>> = HashMap::new();
    for a in &vm.anims {
        lengths.entry(a.role.clone()).or_default().push(a.seconds());
    }
    let _ = LENGTHS.set(lengths);
    // The blades whose trails the overlay draws (trail.rs): the katana and the wakizashi.
    let (katana, wakizashi) = (vm.blade_line(2), vm.blade_line(3));
    for (name, line) in [("katana", &katana), ("wakizashi", &wakizashi)] {
        match line {
            Some(l) => glog!("ARMS", "{name} blade for its trail: bone {}, {:.2} m long", l.bone, l.base.distance(l.tip)),
            None => glog!("ARMS", "{name}: no blade found for its trail (an export without bones.txt?)"),
        }
    }
    let mut canvas = Canvas::new(w, h);
    let mut missing: Vec<&'static str> = Vec::new();
    let (mut drawn, mut total, mut longest, mut since) = (0u32, Duration::ZERO, Duration::ZERO, Instant::now());
    let mut first = true;
    // Where the last picture had arms: they must be cleared from the texture when the next one is copied.
    let mut last_rect: Option<[u32; 4]> = None;
    // The pose last drawn: the same pose again (a resting pose, every frame) is neither drawn nor copied again.
    let mut last: Option<Request> = None;
    loop {
        let req = {
            let mut s = SHARED.lock().unwrap_or_else(|e| e.into_inner());
            loop {
                if let Some(r) = s.request.take() {
                    break r;
                }
                s = WAKE.wait(s).unwrap_or_else(|e| e.into_inner());
            }
        };
        if last == Some(req) {
            continue;
        }
        let mut posed = |clip: &Clip| -> Option<Vec<viewmodel::Mat>> {
            let Some(anim) = vm.find_variant(clip.role, clip.variant) else {
                if !missing.contains(&clip.role) {
                    glog!("ARMS", "no animation \"{}\" in the export; nothing drawn for it", clip.role);
                    missing.push(clip.role);
                }
                return None;
            };
            Some(vm.pose(anim, clip.t, clip.looping))
        };
        let started = Instant::now();
        let Some(main) = posed(&req.pose.main) else {
            continue;
        };
        let pose = match req.pose.from.as_ref().and_then(|c| posed(c)) {
            Some(from) if req.pose.mix < 1.0 => vm.blend(&from, &main, req.pose.mix),
            _ => main,
        };
        canvas.clear();
        canvas.eye = viewmodel::EYE + glam::Vec3::Z * req.drop;
        canvas.tilt = req.tilt;
        canvas.parts = req.pose.parts;
        let stats = viewmodel::draw(&vm, &pose, &mut canvas, ARM_THREADS);
        // The blade's green trail (trail.rs): the katana in Dragonblade's swings and its dash, the wakizashi in
        // Swift Strike's dash.
        let role = req.pose.main.role;
        let line = if (role.starts_with("blade_swing") || role == "blade_dash") && req.pose.parts & viewmodel::PART_KATANA != 0 {
            katana.as_ref()
        } else if role == "dash" && req.pose.parts & viewmodel::PART_WAKIZASHI != 0 {
            wakizashi.as_ref()
        } else {
            None
        };
        match line.and_then(|l| l.on_screen(&pose, &canvas)) {
            Some((base, tip)) => crate::trail::push(base, tip),
            None => crate::trail::pause(),
        }
        let frame = Arc::new(canvas.rgba.clone());
        let took = started.elapsed();
        last = Some(req);
        let this_rect = stats.rect;
        {
            let mut s = SHARED.lock().unwrap_or_else(|e| e.into_inner());
            // The old arms' area and the new one, added to what the overlay hasn't taken yet.
            for r in [last_rect, this_rect].into_iter().flatten() {
                s.dirty = Some(s.dirty.map_or(r, |d| viewmodel::union(d, r)));
            }
            s.frame = Some(frame);
            s.seq += 1;
            last_rect = this_rect;
        }
        if first {
            first = false;
            glog!("ARMS", "first picture: {} in {took:?}, {stats:?}", req.pose.main.role);
        }
        drawn += 1;
        total += took;
        longest = longest.max(took);
        if since.elapsed() >= REPORT_EVERY {
            glog!(
                "ARMS",
                "{drawn} pictures in {:.1} s, average {:.1} ms, longest {:.1} ms (last: {}, {} pixels)",
                since.elapsed().as_secs_f32(),
                total.as_secs_f32() * 1000.0 / drawn as f32,
                longest.as_secs_f32() * 1000.0,
                req.pose.main.role,
                stats.pixels
            );
            (drawn, total, longest, since) = (0, Duration::ZERO, Duration::ZERO, Instant::now());
        }
    }
}
