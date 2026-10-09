//! genji.dll: Genji's kit inside Elden Ring.
//!
//! - Logs the player's state once per second (step 1).
//! - Ultrawide fix (no black bars).
//! - Genji's controls: captures Genji's keys from the game during gameplay, F9 toggles (step 3).
//! - First-person camera while Genji's controls are captured (step 4).
//! - Genji's own mouse look and movement (step 5).
//! - Shuriken: the engine's own projectiles, thrown from the eye (step 6).

mod blade;
mod camera;
mod controls;
mod crosshair;
mod deflect;
mod dinput;
mod enemy_bars;
mod firing;
mod gamepad;
mod hud;
mod kills;
mod log;
mod look;
mod movement;
mod obstacles;
mod scaling;
mod shuriken;
mod sound;
mod swift;
mod targets;
mod ult;
mod ultrawide;

use std::ffi::c_void;
use std::panic::{self, AssertUnwindSafe};
use std::path::PathBuf;
use std::time::{Duration, Instant};

use eldenring::cs::{CSTaskGroupIndex, CSTaskImp, WorldChrMan};
use eldenring::fd4::FD4TaskData;
use fromsoftware_shared::{FromStatic, SharedTaskImpExt};

use controls::Controls;

const DLL_PROCESS_ATTACH: u32 = 1;

unsafe extern "system" {
    fn GetModuleFileNameW(module: *mut c_void, filename: *mut u16, size: u32) -> u32;
}

fn module_path(module: *mut c_void) -> PathBuf {
    let mut buf = vec![0u16; 1024];
    let len = unsafe { GetModuleFileNameW(module, buf.as_mut_ptr(), buf.len() as u32) } as usize;
    PathBuf::from(String::from_utf16_lossy(&buf[..len]))
}

#[unsafe(no_mangle)]
/// # Safety
///
/// Called by Windows' loader. Do not call this yourself.
pub unsafe extern "system" fn DllMain(module: *mut c_void, reason: u32, _reserved: *mut c_void) -> i32 {
    if reason != DLL_PROCESS_ATTACH {
        return 1;
    }
    let dll_path = module_path(module);
    let log_path = log::init(&dll_path);

    panic::set_hook(Box::new(|info| {
        glog!("PANIC", "{info}");
    }));

    glog!("BOOT", "genji.dll {} attached, pid {}", env!("CARGO_PKG_VERSION"), std::process::id());
    glog!("BOOT", "dll: {}", dll_path.display());
    glog!("BOOT", "log: {}", log_path.map(|p| p.display().to_string()).unwrap_or_default());

    // Nothing that touches the game may run on the loader's thread.
    std::thread::spawn(boot);
    std::thread::spawn(ultrawide::apply);
    1
}

fn boot() {
    let started = Instant::now();
    let cs_task = match CSTaskImp::wait_for_instance(Duration::from_secs(300)) {
        Ok(task) => task,
        Err(e) => {
            glog!("BOOT", "FAILED waiting for CSTask after {:.1}s: {e:?}", started.elapsed().as_secs_f32());
            return;
        }
    };
    glog!("BOOT", "CSTask ready after {:.1}s", started.elapsed().as_secs_f32());
    dinput::install();

    let mut monitor = Monitor::default();
    let mut disabled = false;
    cs_task.run_recurring(
        move |data: &FD4TaskData| {
            if disabled {
                return;
            }
            // A bug in our code must never take the game down: log it and switch ourselves off.
            if panic::catch_unwind(AssertUnwindSafe(|| monitor.tick(data))).is_err() {
                glog!("ERROR", "per-frame task panicked; genji.dll disabled for this session");
                dinput::set_blocking(false); // never leave the player's keys captured
                gamepad::set_weapons_blocked(false);
                camera::set_first_person(false);
                controls::emergency_hud_on();
                disabled = true;
            }
        },
        CSTaskGroupIndex::ChrIns_PostPhysics,
    );
    glog!("BOOT", "per-frame task registered (ChrIns_PostPhysics)");

    // Weapon blocking must run right after the game reads input (PadStep), before it decides the player's
    // actions. See gamepad.rs.
    let mut block_disabled = false;
    cs_task.run_recurring(
        move |_: &FD4TaskData| {
            if block_disabled {
                return;
            }
            if panic::catch_unwind(gamepad::weapon_block_step).is_err() {
                glog!("ERROR", "weapon-block task panicked; weapon blocking disabled for this session");
                block_disabled = true;
            }
        },
        CSTaskGroupIndex::GameFlowStep,
    );
    glog!("BOOT", "weapon-block task registered (GameFlowStep)");

    // Overlay (crosshair): hooks the game's frame presentation; the game's renderer exists once CSTask does.
    crosshair::install();

    // First-person camera: right after the game computes its camera (CameraStep). See camera.rs.
    let mut camera_disabled = false;
    cs_task.run_recurring(
        move |data: &FD4TaskData| {
            if camera_disabled {
                return;
            }
            let dt = data.delta_time.time;
            if panic::catch_unwind(move || camera::camera_step(dt)).is_err() {
                glog!("ERROR", "camera task panicked; first person disabled for this session");
                deflect::emergency_clear(); // never leave the player invincible
                camera_disabled = true;
            }
        },
        CSTaskGroupIndex::DrawParamUpdate,
    );
    glog!("BOOT", "camera task registered (DrawParamUpdate)");
}

/// Logs the player's state once per second, plus every time the player appears or disappears.
#[derive(Default)]
struct Monitor {
    frames: u32,
    elapsed: f32,
    player_present: bool,
    controls: Controls,
}

impl Monitor {
    fn tick(&mut self, data: &FD4TaskData) {
        self.frames += 1;
        self.elapsed += data.delta_time.time;

        let world = unsafe { WorldChrMan::instance() }.ok();
        let player = world.and_then(|w| w.main_player.as_ref());

        if player.is_some() != self.player_present {
            self.player_present = player.is_some();
            glog!("STATE", "player {}", if self.player_present { "appeared" } else { "gone (loading or menu)" });
        }
        self.controls.tick(data.delta_time.time, self.player_present);

        if self.elapsed < 1.0 {
            return;
        }
        self.controls.report();
        let fps = self.frames as f32 / self.elapsed;
        self.frames = 0;
        self.elapsed = 0.0;

        let Some(player) = player else {
            glog!("TICK", "fps {fps:.0} no player");
            return;
        };
        let physics = &player.chr_ins.modules.physics;
        let stats = &player.chr_ins.modules.data;
        let p = &physics.position;
        let b = &player.block_position;
        glog!(
            "TICK",
            "fps {fps:.0} pos ({:.2}, {:.2}, {:.2}) block {} local ({:.2}, {:.2}, {:.2}) hp {}/{} ground {}",
            p.0, p.1, p.2,
            player.current_block_id,
            b.x, b.y, b.z,
            stats.hp, stats.max_hp,
            physics.standing_on_solid_ground,
        );
    }
}
