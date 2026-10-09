//! Overwatch crosshair drawn over the game's picture (first part of step 12, the HUD).
//!
//! hudhook (MIT, github.com/veeenu/hudhook) hooks Direct3D 12's frame presentation and gives us an ImGui frame to
//! draw into; we draw four bars on its foreground layer while the first-person view is active. Nothing else of ImGui
//! is used: no windows, no input taken from the game.
//!
//! Your Overwatch settings (screenshot, 2026-10-07): Crosshairs, green, thickness 1, length 6, center gap 13, opacity
//! 80%, black outline at 50%, outline thickness 1, outline shift 1, no dot (dot opacity 0), scale with resolution on.
//! How Overwatch turns these numbers into pixels is not documented; assumed here (not verified): they are pixels at
//! 1080p, scaled by screen height / 1080 ("scale with resolution"), the gap is from the screen centre to the inner
//! end of each bar, and the outline is a border of that many pixels around each bar. "Outline shift" is not modelled.

use std::sync::atomic::{AtomicBool, Ordering};

use hudhook::imgui::{self, Context, Ui};
use hudhook::{ImguiRenderLoop, RenderContext};

use crate::glog;

const COLOR: [f32; 3] = [0.0, 1.0, 0.0];
const OPACITY: f32 = 0.8;
const THICKNESS: f32 = 1.0;
const LENGTH: f32 = 6.0;
const CENTER_GAP: f32 = 13.0;
const OUTLINE_COLOR: [f32; 3] = [0.0, 0.0, 0.0];
const OUTLINE_OPACITY: f32 = 0.5;
const OUTLINE_THICKNESS: f32 = 1.0;
/// Overwatch's "scale with resolution": sizes are for this screen height.
const REFERENCE_HEIGHT: f32 = 1080.0;

static VISIBLE: AtomicBool = AtomicBool::new(false);

/// Show the crosshair (true while the first-person view is drawn) or hide it.
pub fn set_visible(on: bool) {
    VISIBLE.store(on, Ordering::Relaxed);
}

/// Hooks the game's Direct3D 12 frame presentation. Call once, after the game has started.
pub fn install() {
    let result = hudhook::Hudhook::builder()
        .with::<hudhook::hooks::dx12::ImguiDx12Hooks>(Overlay {
            frames: 0,
            failed: false,
            hud: crate::hud::Resources::default(),
            arms: ArmsTexture::default(),
        })
        .build()
        .apply();
    match result {
        Ok(()) => glog!("HUD", "Direct3D 12 overlay hook installed"),
        Err(e) => glog!("HUD", "overlay hook FAILED: {e:?}; no crosshair this session"),
    }
}

struct Overlay {
    frames: u64,
    failed: bool,
    /// The HUD's images and fonts (step 12).
    hud: crate::hud::Resources,
    /// Genji's arms (step 10c, arms.rs): the picture the drawing thread made, copied into a texture.
    arms: ArmsTexture,
}

/// The arms' texture, half the screen's size, stretched over it when drawn.
#[derive(Default)]
struct ArmsTexture {
    id: Option<imgui::TextureId>,
    size: (u32, u32),
    /// The last picture copied in (arms.rs numbers them).
    seq: u64,
    /// A picture is in the texture.
    ready: bool,
    copies: u32,
    copy_time: std::time::Duration,
    copy_longest: std::time::Duration,
    report_at: Option<std::time::Instant>,
}

impl ImguiRenderLoop for Overlay {
    fn initialize<'a>(&'a mut self, ctx: &mut Context, render_context: &'a mut dyn RenderContext) {
        // ImGui would otherwise save window positions to imgui.ini in the game's folder.
        ctx.set_ini_filename(None);
        ctx.set_log_filename(None);
        // Never let ImGui change or show the mouse cursor, or take mouse/keyboard focus from the game.
        ctx.io_mut().config_flags |= imgui::ConfigFlags::NO_MOUSE_CURSOR_CHANGE | imgui::ConfigFlags::NO_MOUSE;
        self.hud = crate::hud::Resources::load(ctx, render_context);
        // The arms: drawn at half the screen's size (DESIGN.md 5k), on their own thread.
        let [w, h] = ctx.io().display_size;
        let (w, h) = (((w / 2.0).round() as u32).max(1), ((h / 2.0).round() as u32).max(1));
        match render_context.load_texture(&vec![0u8; (w * h * 4) as usize], w, h) {
            Ok(id) => {
                self.arms.id = Some(id);
                self.arms.size = (w, h);
                crate::arms::start(w as usize, h as usize);
            }
            Err(e) => glog!("ARMS", "texture not created ({e:?}); no arms this session"),
        }
    }

    fn before_render<'a>(&'a mut self, _ctx: &mut Context, render_context: &'a mut dyn RenderContext) {
        // Inside the game's frame presentation too: a bug here hides the arms, never takes the game down.
        let arms = &mut self.arms;
        if std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| copy_arms(arms, render_context))).is_err() {
            glog!("ERROR", "arms texture copy panicked; arms hidden for this session");
            self.arms.id = None;
            self.arms.ready = false;
        }
    }

    fn render(&mut self, ui: &mut Ui) {
        // Runs inside the game's frame presentation: a bug here must never take the game down.
        if std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| self.draw(ui))).is_err() {
            glog!("ERROR", "overlay draw panicked; crosshair hidden for this session");
            set_visible(false);
            self.failed = true;
        }
    }
}

/// Copies the newest picture of the arms (arms.rs) into their texture, and logs how long copies take.
fn copy_arms(a: &mut ArmsTexture, render_context: &mut dyn RenderContext) {
    let Some(id) = a.id else {
        return;
    };
    if !VISIBLE.load(Ordering::Relaxed) {
        return; // the texture keeps its picture for when first person shows again
    }
    let Some((seq, rgba)) = crate::arms::latest(a.seq) else {
        return;
    };
    a.seq = seq;
    let started = std::time::Instant::now();
    match render_context.replace_texture(id, &rgba, a.size.0, a.size.1) {
        Ok(()) => a.ready = true,
        Err(e) => {
            glog!("ARMS", "texture copy failed ({e:?}); arms hidden for this session");
            a.id = None;
            a.ready = false;
            return;
        }
    }
    let took = started.elapsed();
    a.copies += 1;
    a.copy_time += took;
    a.copy_longest = a.copy_longest.max(took);
    let report_at = *a.report_at.get_or_insert_with(std::time::Instant::now);
    if report_at.elapsed().as_secs_f32() >= 5.0 {
        glog!(
            "ARMS",
            "overlay: {} pictures copied to the texture, average {:.2} ms, longest {:.2} ms",
            a.copies,
            a.copy_time.as_secs_f32() * 1000.0 / a.copies as f32,
            a.copy_longest.as_secs_f32() * 1000.0
        );
        (a.copies, a.copy_time, a.copy_longest, a.report_at) = (0, Default::default(), Default::default(), None);
    }
}

impl Overlay {
    fn draw(&mut self, ui: &mut Ui) {
        if self.failed {
            return;
        }
        let [width, height] = ui.io().display_size;
        if self.frames == 0 {
            glog!("HUD", "first overlay frame, screen {width} x {height}");
        }
        self.frames += 1;
        if !VISIBLE.load(Ordering::Relaxed) {
            return;
        }
        // Genji's arms, stretched over the whole screen, under everything else we draw.
        if let (Some(id), true, true) = (self.arms.id, self.arms.ready, crate::arms::shown()) {
            ui.get_foreground_draw_list().add_image(id, [0.0, 0.0], [width, height]).build();
        }
        crate::hud::draw_kill_skull(ui, &mut self.hud, width, height);
        {
            // ImGui allows one draw list in use at a time: this one ends before the HUD takes its own.
            let draw = ui.get_foreground_draw_list();
            for (min, max, color) in shapes(width, height) {
                draw.add_rect(min, max, imgui::ImColor32::from_rgba_f32s(color[0], color[1], color[2], color[3]))
                    .filled(true)
                    .build();
            }
        }
        crate::hud::draw(ui, &mut self.hud, width, height);
    }
}

/// The crosshair as filled rectangles (top-left, bottom-right, RGBA), outline first, in whole pixels.
fn shapes(width: f32, height: f32) -> Vec<([f32; 2], [f32; 2], [f32; 4])> {
    let scale = height / REFERENCE_HEIGHT;
    let px = |v: f32| (v * scale).round().max(1.0);
    let (thickness, length, gap, outline) = (px(THICKNESS), px(LENGTH), (CENTER_GAP * scale).round(), px(OUTLINE_THICKNESS));
    let (cx, cy) = ((width / 2.0).floor(), (height / 2.0).floor());
    let half = (thickness / 2.0).floor();
    // Bars: right, left, down, up.
    let bars = [
        ([cx + gap, cy - half], [cx + gap + length, cy - half + thickness]),
        ([cx - gap - length, cy - half], [cx - gap, cy - half + thickness]),
        ([cx - half, cy + gap], [cx - half + thickness, cy + gap + length]),
        ([cx - half, cy - gap - length], [cx - half + thickness, cy - gap]),
    ];
    let line = [COLOR[0], COLOR[1], COLOR[2], OPACITY];
    let edge = [OUTLINE_COLOR[0], OUTLINE_COLOR[1], OUTLINE_COLOR[2], OUTLINE_OPACITY];
    let mut out = Vec::new();
    for (min, max) in bars {
        // Border around the bar (four strips, so the bar itself isn't darkened by the outline under it).
        let (x0, y0, x1, y1) = (min[0] - outline, min[1] - outline, max[0] + outline, max[1] + outline);
        out.push(([x0, y0], [x1, min[1]], edge));
        out.push(([x0, max[1]], [x1, y1], edge));
        out.push(([x0, min[1]], [min[0], max[1]], edge));
        out.push(([max[0], min[1]], [x1, max[1]], edge));
    }
    for (min, max) in bars {
        out.push((min, max, line));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bars_sit_at_the_gap_and_scale_with_height() {
        let s = shapes(3440.0, 1440.0);
        assert_eq!(s.len(), 20);
        let bars = &s[16..];
        let (cx, cy) = (1720.0, 720.0);
        // 1440p: gap 13 -> 17 px, length 6 -> 8 px, thickness 1 -> 1 px.
        assert_eq!(bars[0].0[0], cx + 17.0);
        assert_eq!(bars[0].1[0] - bars[0].0[0], 8.0);
        assert_eq!(bars[0].1[1] - bars[0].0[1], 1.0);
        assert_eq!(bars[1].1[0], cx - 17.0);
        assert_eq!(bars[2].0[1], cy + 17.0);
        assert_eq!(bars[3].1[1], cy - 17.0);
    }

    #[test]
    fn outline_surrounds_each_bar_without_covering_it() {
        let s = shapes(1920.0, 1080.0);
        let (outline, bars) = (&s[..16], &s[16..]);
        for (i, bar) in bars.iter().enumerate() {
            let strips = &outline[i * 4..i * 4 + 4];
            for strip in strips {
                let overlap_x = strip.1[0].min(bar.1[0]) - strip.0[0].max(bar.0[0]);
                let overlap_y = strip.1[1].min(bar.1[1]) - strip.0[1].max(bar.0[1]);
                assert!(overlap_x <= 0.0 || overlap_y <= 0.0, "outline covers bar {i}");
            }
            assert_eq!(strips[0].0, [bar.0[0] - 1.0, bar.0[1] - 1.0]);
            assert_eq!(strips[1].1, [bar.1[0] + 1.0, bar.1[1] + 1.0]);
        }
    }
}
