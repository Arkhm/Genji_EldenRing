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
const COLOR_U8: [u8; 3] = [0, 255, 0];
const THICKNESS: f32 = 1.0;
const LENGTH: f32 = 6.0;
const CENTER_GAP: f32 = 13.0;
const OUTLINE_COLOR: [f32; 3] = [0.0, 0.0, 0.0];
const OUTLINE_OPACITY: f32 = 0.5;
const OUTLINE_THICKNESS: f32 = 1.0;
/// Overwatch's "scale with resolution": sizes are for this screen height.
const REFERENCE_HEIGHT: f32 = 1080.0;

static VISIBLE: AtomicBool = AtomicBool::new(false);
static DEFLECTING: AtomicBool = AtomicBool::new(false);

/// Shows "DEFLECT" under the crosshair while Deflect is up (until Genji's sword is drawn, step 10).
pub fn set_deflecting(on: bool) {
    DEFLECTING.store(on, Ordering::Relaxed);
}

/// Show the crosshair (true while the first-person view is drawn) or hide it.
pub fn set_visible(on: bool) {
    VISIBLE.store(on, Ordering::Relaxed);
}

/// Hooks the game's Direct3D 12 frame presentation. Call once, after the game has started.
pub fn install() {
    let result = hudhook::Hudhook::builder()
        .with::<hudhook::hooks::dx12::ImguiDx12Hooks>(Overlay { frames: 0, failed: false })
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
}

impl ImguiRenderLoop for Overlay {
    fn initialize<'a>(&'a mut self, ctx: &mut Context, _render_context: &'a mut dyn RenderContext) {
        // ImGui would otherwise save window positions to imgui.ini in the game's folder.
        ctx.set_ini_filename(None);
        ctx.set_log_filename(None);
        // Never let ImGui change or show the mouse cursor, or take mouse/keyboard focus from the game.
        ctx.io_mut().config_flags |= imgui::ConfigFlags::NO_MOUSE_CURSOR_CHANGE | imgui::ConfigFlags::NO_MOUSE;
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
        let draw = ui.get_foreground_draw_list();
        for (min, max, color) in shapes(width, height) {
            draw.add_rect(min, max, imgui::ImColor32::from_rgba_f32s(color[0], color[1], color[2], color[3]))
                .filled(true)
                .build();
        }
        if DEFLECTING.load(Ordering::Relaxed) {
            let text = "DEFLECT";
            let size = ui.calc_text_size(text);
            let pos = [(width - size[0]) / 2.0, height / 2.0 + 40.0 * height / REFERENCE_HEIGHT];
            draw.add_text([pos[0] + 1.0, pos[1] + 1.0], imgui::ImColor32::from_rgba(0, 0, 0, 160), text);
            draw.add_text(pos, imgui::ImColor32::from_rgba(COLOR_U8[0], COLOR_U8[1], COLOR_U8[2], 230), text);
        }
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
