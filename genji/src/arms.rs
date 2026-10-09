//! Step 10c (DESIGN.md 5k): Genji's arms in game. A thread of our own ("genji-arms") draws the arms with
//! `viewmodel.rs` at half the screen's size, split over ARM_THREADS cores, whenever the game thread asks for a pose;
//! the overlay (crosshair.rs) copies each finished picture into a texture and stretches it over the screen, under
//! the HUD and the crosshair. The game thread never waits for the drawing.
//!
//! The same pose asked for again isn't drawn again (a resting pose stays in the texture as it is).
//!
//! Logged every few seconds: pictures drawn, average and longest drawing time (ARMS lines); the overlay logs its copy
//! time the same way.

use std::path::Path;
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

use crate::glog;
use crate::viewmodel::{self, Canvas, Viewmodel};

/// Cores the drawing is split over (the game keeps the rest).
const ARM_THREADS: usize = 4;
/// Timing report every this many seconds.
const REPORT_EVERY: Duration = Duration::from_secs(5);

#[derive(Clone, Copy, PartialEq)]
struct Request {
    role: &'static str,
    t: f32,
    looping: bool,
}

struct Shared {
    request: Option<Request>,
    frame: Option<Arc<Vec<u8>>>,
    seq: u64,
}

static SHARED: Mutex<Shared> = Mutex::new(Shared { request: None, frame: None, seq: 0 });
static WAKE: Condvar = Condvar::new();

/// Starts the drawing thread for pictures of `w` x `h` (called once by the overlay).
pub fn start(w: usize, h: usize) {
    let spawned = std::thread::Builder::new().name("genji-arms".into()).spawn(move || run(w, h));
    if let Err(e) = spawned {
        glog!("ARMS", "drawing thread not started ({e}); no arms this session");
    }
}

/// Asks for the arms in animation `role` at `t` seconds (game thread, every frame while first person shows).
pub fn request(role: &'static str, t: f32, looping: bool) {
    let mut s = SHARED.lock().unwrap_or_else(|e| e.into_inner());
    s.request = Some(Request { role, t, looping });
    WAKE.notify_one();
}

/// The newest finished picture, if newer than `seen` (overlay).
pub fn latest(seen: u64) -> Option<(u64, Arc<Vec<u8>>)> {
    let s = SHARED.lock().unwrap_or_else(|e| e.into_inner());
    (s.seq > seen).then(|| s.frame.clone().map(|f| (s.seq, f))).flatten()
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
    let mut canvas = Canvas::new(w, h);
    let mut missing: Vec<&'static str> = Vec::new();
    let (mut drawn, mut total, mut longest, mut since) = (0u32, Duration::ZERO, Duration::ZERO, Instant::now());
    let mut first = true;
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
        let Some(anim) = vm.find(req.role) else {
            if !missing.contains(&req.role) {
                glog!("ARMS", "no animation \"{}\" in the export; nothing drawn for it", req.role);
                missing.push(req.role);
            }
            continue;
        };
        let started = Instant::now();
        let pose = vm.pose(anim, req.t, req.looping);
        canvas.clear();
        let stats = viewmodel::draw(&vm, &pose, &mut canvas, ARM_THREADS);
        let frame = Arc::new(canvas.rgba.clone());
        let took = started.elapsed();
        last = Some(req);
        {
            let mut s = SHARED.lock().unwrap_or_else(|e| e.into_inner());
            s.frame = Some(frame);
            s.seq += 1;
        }
        if first {
            first = false;
            glog!("ARMS", "first picture: {} ({}) in {took:?}, {stats:?}", req.role, vm.anims[anim].id);
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
                req.role,
                stats.pixels
            );
            (drawn, total, longest, since) = (0, Duration::ZERO, Duration::ZERO, Instant::now());
        }
    }
}
