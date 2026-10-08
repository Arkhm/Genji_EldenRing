//! The Overwatch-style HUD (step 12, DESIGN.md 5h), drawn by the overlay in `crosshair.rs`.
//!
//! The game thread writes a snapshot of Genji's state every frame (`update`); the overlay's render thread draws it.
//! Pictures are Overwatch's own UI art from your extracted files (outside the repository): ability icons, the hexagon
//! portrait, and effect textures (glow ring, ring of dashes, light flare) built into D:\GenjiBridgeAssets\hud by
//! tools\make_hud_art.py. Shapes and animations follow your Overwatch 2 screenshots (2026-10-08): Overwatch's own
//! HUD animations are engine effects, not pictures in its UI files, so they are redrawn here. A missing picture is
//! logged and left out.

use std::f32::consts::{FRAC_PI_2, TAU};
use std::path::Path;
use std::sync::Mutex;

use hudhook::imgui::{Context, DrawListMut, FontConfig, FontId, FontSource, ImColor32, TextureId, Ui};
use hudhook::RenderContext;

use crate::glog;

const RAW: &str = r"D:\GenjiBridgeAssets\raw";
const HUD_ART: &str = r"D:\GenjiBridgeAssets\hud";
const ICON_SWIFT: &str = r"Abilities\Swift Strike\000000004E94.png";
const ICON_DEFLECT: &str = r"Abilities\Deflect\000000004E95.png";
const ICON_BLADE: &str = r"Abilities\Dragonblade\000000004E93.png";
const ICON_SHURIKEN: &str = r"Abilities\Shuriken\000000004E83.png";
/// Overwatch's ultimate and cooldown digits are a heavy oblique (closest on Windows: Segoe UI Black Italic); health
/// and ammo are upright bold (Segoe UI Bold); key labels Bahnschrift.
const ITALIC_FONT: &str = r"C:\Windows\Fonts\seguibli.ttf";
const BOLD_FONT: &str = r"C:\Windows\Fonts\segoeuib.ttf";
const LABEL_FONT: &str = r"C:\Windows\Fonts\bahnschrift.ttf";
const REFERENCE_HEIGHT: f32 = 1080.0;
/// Ticks around the ultimate ring (Overwatch shows the charge as cyan ticks).
const ULT_TICKS: usize = 48;
/// Length of the burst when the ultimate becomes ready, seconds.
const FLASH: f32 = 0.6;
/// Length of the flash when a cooldown finishes ("fast", your Overwatch frames), seconds.
const COOLDOWN_FLASH: f32 = 0.45;

/// Everything the HUD shows, written by the game thread each frame.
#[derive(Clone, Copy, Default)]
pub struct State {
    pub visible: bool,
    pub hp: i32,
    pub max_hp: i32,
    pub ammo: u32,
    pub ammo_max: u32,
    /// Share of the reload done, while reloading.
    pub reloading: Option<f32>,
    pub swift_cooldown: f32,
    pub swift_cooldown_max: f32,
    pub deflect_left: Option<f32>,
    pub deflect_cooldown: f32,
    pub deflect_cooldown_max: f32,
    pub ult_percent: u32,
    pub blade_left: Option<f32>,
    pub blade_duration: f32,
}

static STATE: Mutex<State> = Mutex::new(State {
    visible: false,
    hp: 0,
    max_hp: 0,
    ammo: 0,
    ammo_max: 0,
    reloading: None,
    swift_cooldown: 0.0,
    swift_cooldown_max: 1.0,
    deflect_left: None,
    deflect_cooldown: 0.0,
    deflect_cooldown_max: 1.0,
    ult_percent: 0,
    blade_left: None,
    blade_duration: 0.0,
});

pub fn update(state: State) {
    *STATE.lock().unwrap_or_else(|e| e.into_inner()) = state;
}

pub fn hide() {
    STATE.lock().unwrap_or_else(|e| e.into_inner()).visible = false;
}

type Picture = Option<(TextureId, [f32; 2])>;

/// Textures and fonts, created once by the overlay; animation timers.
#[derive(Default)]
pub struct Resources {
    portrait: Picture,
    plus: Picture,
    swift: Picture,
    deflect: Picture,
    blade: Picture,
    shuriken: Picture,
    glow_ring: Picture,
    dash_ring: Picture,
    flare: Picture,
    glow_dot: Picture,
    italic_big: Option<FontId>,
    italic_small: Option<FontId>,
    bold_big: Option<FontId>,
    bold_small: Option<FontId>,
    label: Option<FontId>,
    anim: Anim,
}

/// When things last became ready (overlay clock, seconds), to play their flash.
#[derive(Default)]
struct Anim {
    swift_was_cooling: bool,
    deflect_was_cooling: bool,
    ult_was_ready: bool,
    swift_ready_at: Option<f32>,
    deflect_ready_at: Option<f32>,
    ult_ready_at: Option<f32>,
}

// SAFETY: font ids are pointers into ImGui's font atlas; they are created and used only on the overlay's render
// thread (hudhook moves the render loop there once, at startup). The bound only exists for that one-time move.
unsafe impl Send for Resources {}
unsafe impl Sync for Resources {}

impl Resources {
    /// Loads the pictures and fonts (called from the overlay's initialize, before the font atlas is built).
    pub fn load(ctx: &mut Context, render: &mut dyn RenderContext) -> Self {
        let mut r = Resources::default();
        let mut load = |dir: &str, name: &str| -> Picture {
            let path = Path::new(dir).join(name);
            match decode_png(&path) {
                Ok((rgba, w, h)) => match render.load_texture(&rgba, w, h) {
                    Ok(id) => Some((id, [w as f32, h as f32])),
                    Err(e) => {
                        glog!("HUD", "texture upload failed for {}: {e:?}", path.display());
                        None
                    }
                },
                Err(e) => {
                    glog!("HUD", "picture {} not loaded: {e} (run tools\\make_hud_art.py?)", path.display());
                    None
                }
            }
        };
        r.portrait = load(HUD_ART, "portrait_hex.png");
        r.plus = load(HUD_ART, "plus.png");
        r.glow_ring = load(HUD_ART, "glow_ring.png");
        r.dash_ring = load(HUD_ART, "dash_ring.png");
        r.flare = load(HUD_ART, "flare.png");
        r.glow_dot = load(HUD_ART, "glow_dot.png");
        r.swift = load(RAW, ICON_SWIFT);
        r.deflect = load(RAW, ICON_DEFLECT);
        r.blade = load(RAW, ICON_BLADE);
        r.shuriken = load(RAW, ICON_SHURIKEN);
        let k = (ctx.io().display_size[1] / REFERENCE_HEIGHT).max(0.5);
        let fonts = ctx.fonts();
        fonts.add_font(&[FontSource::DefaultFontData { config: None }]);
        let mut add = |path: &str, size: f32| -> Option<FontId> {
            match std::fs::read(path) {
                Ok(bytes) => {
                    // ImGui keeps a pointer to the font data for the atlas: give it data that lives forever.
                    let data: &'static [u8] = Box::leak(bytes.into_boxed_slice());
                    Some(fonts.add_font(&[FontSource::TtfData { data, size_pixels: size * k, config: Some(FontConfig::default()) }]))
                }
                Err(e) => {
                    glog!("HUD", "font {path} not loaded ({e})");
                    None
                }
            }
        };
        r.italic_big = add(ITALIC_FONT, 34.0);
        r.italic_small = add(ITALIC_FONT, 16.0);
        r.bold_big = add(BOLD_FONT, 30.0);
        r.bold_small = add(BOLD_FONT, 18.0);
        r.label = add(LABEL_FONT, 12.0);
        glog!(
            "HUD",
            "resources: pictures {}/10, fonts {}/5",
            [r.portrait, r.plus, r.glow_ring, r.dash_ring, r.flare, r.glow_dot, r.swift, r.deflect, r.blade, r.shuriken]
                .iter()
                .filter(|p| p.is_some())
                .count(),
            [r.italic_big, r.italic_small, r.bold_big, r.bold_small, r.label].iter().filter(|f| f.is_some()).count()
        );
        r
    }
}

fn decode_png(path: &Path) -> Result<(Vec<u8>, u32, u32), String> {
    let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let mut decoder = png::Decoder::new(std::io::BufReader::new(file));
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = decoder.read_info().map_err(|e| e.to_string())?;
    let mut buf = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf).map_err(|e| e.to_string())?;
    buf.truncate(info.buffer_size());
    let rgba = match info.color_type {
        png::ColorType::Rgba => buf,
        png::ColorType::Rgb => buf.chunks(3).flat_map(|p| [p[0], p[1], p[2], 255]).collect(),
        png::ColorType::GrayscaleAlpha => buf.chunks(2).flat_map(|p| [p[0], p[0], p[0], p[1]]).collect(),
        png::ColorType::Grayscale => buf.iter().flat_map(|&g| [g, g, g, 255]).collect(),
        other => return Err(format!("unsupported PNG colour type {other:?}")),
    };
    Ok((rgba, info.width, info.height))
}

const WHITE: [u8; 4] = [255, 255, 255, 255];
const PANEL_WHITE: [u8; 4] = [232, 235, 240, 235];
const DIM: [u8; 4] = [215, 220, 230, 200];
const SHADOW: [u8; 4] = [0, 0, 0, 110];
const NAVY: [u8; 4] = [16, 30, 52, 205];
const TEAL: [u8; 4] = [60, 205, 225, 220];
const CYAN: [u8; 4] = [90, 235, 255, 255];
const BLUE_GLOW: [u8; 4] = [70, 170, 255, 255];
const ICON_ON_WHITE: [u8; 4] = [40, 45, 60, 255];
const COOLING: [u8; 4] = [95, 100, 112, 175];

fn col(c: [u8; 4]) -> ImColor32 {
    ImColor32::from_rgba(c[0], c[1], c[2], c[3])
}

fn alpha(c: [u8; 4], a: f32) -> [u8; 4] {
    [c[0], c[1], c[2], (c[3] as f32 * a.clamp(0.0, 1.0)) as u8]
}

/// A picture centred on `center`, `size` wide, rotated by `angle`, tinted.
fn picture(draw: &DrawListMut, pic: Picture, center: [f32; 2], size: f32, angle: f32, tint: [u8; 4]) {
    let Some((id, [w, h])) = pic else {
        return;
    };
    let (hw, hh) = (size / 2.0, size / 2.0 * h / w);
    let (s, c) = angle.sin_cos();
    let corner = |x: f32, y: f32| [center[0] + x * c - y * s, center[1] + x * s + y * c];
    draw.add_image_quad(id, corner(-hw, -hh), corner(hw, -hh), corner(hw, hh), corner(-hw, hh)).col(col(tint)).build();
}

/// Draws the HUD (overlay render thread).
pub fn draw(ui: &Ui, r: &mut Resources, width: f32, height: f32) {
    let s = *STATE.lock().unwrap_or_else(|e| e.into_inner());
    if !s.visible {
        return;
    }
    let now = ui.time() as f32;
    // Flashes when something becomes ready.
    let swift_cooling = s.swift_cooldown > 0.0;
    if r.anim.swift_was_cooling && !swift_cooling {
        r.anim.swift_ready_at = Some(now);
    }
    r.anim.swift_was_cooling = swift_cooling;
    let deflect_cooling = s.deflect_cooldown > 0.0;
    if r.anim.deflect_was_cooling && !deflect_cooling {
        r.anim.deflect_ready_at = Some(now);
    }
    r.anim.deflect_was_cooling = deflect_cooling;
    let ult_ready = s.ult_percent >= 100 && s.blade_left.is_none();
    if ult_ready && !r.anim.ult_was_ready {
        r.anim.ult_ready_at = Some(now);
    }
    r.anim.ult_was_ready = ult_ready;

    let k = height / REFERENCE_HEIGHT;
    let draw = ui.get_foreground_draw_list();
    let text = |pos: [f32; 2], font: Option<FontId>, color: [u8; 4], t: &str| {
        let _f = font.map(|f| ui.push_font(f));
        draw.add_text([pos[0] + 1.5 * k, pos[1] + 1.5 * k], col(alpha(SHADOW, color[3] as f32 / 255.0)), t);
        draw.add_text(pos, col(color), t);
    };
    let measure = |font: Option<FontId>, t: &str| {
        let _f = font.map(|f| ui.push_font(f));
        ui.calc_text_size(t)
    };
    // A key box ("LSHIFT", "E", "Q"): filled light when the ability is ready, outlined when not.
    let key_box = |center_x: f32, top: f32, label: &str, ready: bool| {
        let t = measure(r.label, label);
        let (w, h) = (t[0] + 10.0 * k, t[1] + 4.0 * k);
        let x = center_x - w / 2.0;
        if ready {
            draw.add_rect([x, top], [x + w, top + h], col(PANEL_WHITE)).filled(true).rounding(2.0 * k).build();
        } else {
            draw.add_rect([x, top], [x + w, top + h], col([20, 24, 34, 140])).filled(true).rounding(2.0 * k).build();
            draw.add_rect([x, top], [x + w, top + h], col([200, 205, 215, 200])).thickness(1.2 * k).rounding(2.0 * k).build();
        }
        let _f = r.label.map(|f| ui.push_font(f));
        draw.add_text([x + 5.0 * k, top + 2.0 * k], col(if ready { [30, 34, 46, 255] } else { [225, 228, 235, 255] }), label);
    };
    let base = height - 62.0 * k;

    // ---- Bottom left: hexagon portrait, "+" box, health "hp | max", 10 white segments.
    // The picture is the hexagon (200 of its 272 px width) with the helmet rising above and 36 px of glow around.
    let hex_w = 92.0 * k;
    let (img_w, img_h) = r.portrait.map(|(_, [w, h])| (w, h)).unwrap_or((272.0, 364.0));
    let pw = hex_w * img_w / 200.0;
    let ph = pw * img_h / img_w;
    let glow_below = ph * 36.0 / img_h;
    let px = 30.0 * k;
    let py = base + 8.0 * k + glow_below - ph;
    if let Some((id, _)) = r.portrait {
        draw.add_image(id, [px, py], [px + pw, py + ph]).build();
    }
    let hx = px + pw - 8.0 * k;
    let seg_top = base - 30.0 * k;
    let plus = 22.0 * k;
    let num_mid = seg_top - 22.0 * k;
    draw.add_rect([hx, num_mid - plus / 2.0], [hx + plus, num_mid + plus / 2.0], col(PANEL_WHITE)).filled(true).rounding(2.0 * k).build();
    picture(&draw, r.plus, [hx + plus / 2.0, num_mid], plus * 0.7, 0.0, ICON_ON_WHITE);
    let hp = s.hp.max(0).to_string();
    let nx = hx + plus + 10.0 * k;
    let t = measure(r.bold_big, &hp);
    text([nx, num_mid - t[1] / 2.0], r.bold_big, WHITE, &hp);
    let sep_x = nx + t[0] + 6.0 * k;
    draw.add_line([sep_x, num_mid - 11.0 * k], [sep_x, num_mid + 12.0 * k], col(DIM)).thickness(1.5 * k).build();
    let max = s.max_hp.to_string();
    let m = measure(r.bold_big, &max);
    text([sep_x + 7.0 * k, num_mid - m[1] / 2.0], r.bold_big, DIM, &max);
    let (seg_w, seg_h, gap) = (17.0 * k, 22.0 * k, 3.0 * k);
    let fill = if s.max_hp > 0 { (s.hp.max(0) as f32 / s.max_hp as f32).clamp(0.0, 1.0) * 10.0 } else { 0.0 };
    for i in 0..10 {
        let x0 = hx + i as f32 * (seg_w + gap);
        draw.add_rect([x0, seg_top], [x0 + seg_w, seg_top + seg_h], col([255, 255, 255, 45])).filled(true).build();
        let part = (fill - i as f32).clamp(0.0, 1.0);
        if part > 0.0 {
            draw.add_rect([x0, seg_top], [x0 + seg_w * part, seg_top + seg_h], col(PANEL_WHITE)).filled(true).build();
        }
    }

    // ---- Bottom centre: the ultimate.
    let radius = 34.0 * k;
    let (cx, cy) = (width / 2.0, base - radius - 58.0 * k);
    // Faint outer rings and the dotted circle.
    for i in 0..60 {
        let a = TAU * i as f32 / 60.0;
        let rr = radius * 1.62;
        draw.add_circle([cx + rr * a.cos(), cy + rr * a.sin()], 0.9 * k, col([255, 255, 255, 55])).filled(true).build();
    }
    draw.add_circle([cx, cy], radius * 1.32, col([255, 255, 255, 40])).thickness(1.5 * k).num_segments(72).build();
    if let Some(left) = s.blade_left {
        // Dragonblade: glowing ring draining with the time left, the blade icon, seconds.
        let pulse = 0.75 + 0.25 * (now * 6.0).sin();
        picture(&draw, r.glow_ring, [cx, cy], radius * 3.0, 0.0, alpha(BLUE_GLOW, pulse));
        draw.add_circle([cx, cy], radius, col(NAVY)).filled(true).num_segments(64).build();
        let share = if s.blade_duration > 0.0 { (left / s.blade_duration).clamp(0.0, 1.0) } else { 0.0 };
        arc(&draw, [cx, cy], radius * 1.08, share, col(CYAN), 4.0 * k);
        picture(&draw, r.blade, [cx, cy - 6.0 * k], radius * 1.1, 0.0, WHITE);
        let t = format!("{left:.1}");
        let tw = measure(r.italic_small, &t);
        text([cx - tw[0] / 2.0, cy + radius * 0.38], r.italic_small, CYAN, &t);
        key_box(cx, cy + radius * 1.62 + 6.0 * k, "Q", false);
    } else if ult_ready {
        // Ready: pulsing blue glow, spinning light, white centre with the icon, side markers, burst when it arrives.
        // Light around it: a wide soft blue halo breathing, a brighter inner glow, faint slow light rays.
        let breathe = 0.5 + 0.5 * (now * 2.2).sin();
        picture(&draw, r.glow_dot, [cx, cy], radius * (5.2 + 0.6 * breathe), 0.0, alpha(BLUE_GLOW, 0.55 + 0.25 * breathe));
        picture(&draw, r.glow_dot, [cx, cy], radius * 3.2, 0.0, alpha([150, 225, 255, 255], 0.5 + 0.3 * breathe));
        picture(&draw, r.flare, [cx, cy], radius * 4.2, now * 0.25, alpha([140, 210, 255, 255], 0.35 + 0.2 * breathe));
        picture(&draw, r.glow_ring, [cx, cy], radius * 3.0, 0.0, alpha(BLUE_GLOW, 0.6 + 0.4 * breathe));
        draw.add_circle([cx, cy], radius * 1.12, col(alpha(BLUE_GLOW, 0.9))).thickness(4.0 * k).num_segments(72).build();
        draw.add_circle([cx, cy], radius * 0.92, col(PANEL_WHITE)).filled(true).num_segments(64).build();
        picture(&draw, r.blade, [cx, cy], radius * 1.25, 0.0, ICON_ON_WHITE);
        for side in [-1.0f32, 1.0] {
            let x = cx + side * radius * 1.42;
            draw.add_triangle([x, cy - 5.0 * k], [x, cy + 5.0 * k], [x - side * 7.0 * k, cy], col(WHITE)).filled(true).build();
        }
        if let Some(t0) = r.anim.ult_ready_at {
            let p = (now - t0) / FLASH;
            if p < 1.0 {
                picture(&draw, r.flare, [cx, cy], radius * (2.0 + 4.0 * p), 0.0, alpha(WHITE, 1.0 - p));
            }
        }
        key_box(cx, cy + radius * 1.62 + 6.0 * k, "Q", true);
    } else {
        // Charging: navy disc, teal ring, cyan ticks for the charge, italic number.
        draw.add_circle([cx, cy], radius, col(NAVY)).filled(true).num_segments(64).build();
        draw.add_circle([cx, cy], radius * 0.86, col(TEAL)).thickness(2.0 * k).num_segments(64).build();
        let lit = (s.ult_percent as f32 / 100.0 * ULT_TICKS as f32).floor() as usize;
        for i in 0..ULT_TICKS {
            let a = -FRAC_PI_2 + TAU * i as f32 / ULT_TICKS as f32;
            let (r0, r1) = (radius * 1.05, radius * 1.24);
            let c = if i < lit { col(CYAN) } else { col([255, 255, 255, 30]) };
            draw.add_line([cx + r0 * a.cos(), cy + r0 * a.sin()], [cx + r1 * a.cos(), cy + r1 * a.sin()], c).thickness(2.2 * k).build();
        }
        let t = s.ult_percent.to_string();
        let tw = measure(r.italic_big, &t);
        text([cx - tw[0] / 2.0, cy - tw[1] / 2.0], r.italic_big, WHITE, &t);
        key_box(cx, cy + radius * 1.62 + 6.0 * k, "Q", false);
    }

    // ---- Bottom right: Swift Strike and Deflect boxes, then the ammo panel.
    let size = 52.0 * k;
    let box_top = base - size - 16.0 * k;
    let mut x = width - 410.0 * k;
    for (icon, key, cooldown, max, active, ready_at) in [
        (r.swift, "LSHIFT", s.swift_cooldown, s.swift_cooldown_max, None, r.anim.swift_ready_at),
        (r.deflect, "E", s.deflect_cooldown, s.deflect_cooldown_max, s.deflect_left, r.anim.deflect_ready_at),
    ] {
        let ready = cooldown <= 0.0;
        let (x0, y0, x1, y1) = (x, box_top, x + size, box_top + size);
        if ready {
            draw.add_rect_filled_multicolor([x0, y0], [x1, y1], col(WHITE), col(PANEL_WHITE), col([205, 210, 220, 235]), col([215, 220, 228, 235]));
            picture(&draw, icon, [x0 + size / 2.0, y0 + size / 2.0], size * 0.8, 0.0, ICON_ON_WHITE);
            draw.add_rect([x0, y1 + 3.0 * k], [x1, y1 + 5.0 * k], col(PANEL_WHITE)).filled(true).build();
        } else {
            draw.add_rect([x0, y0], [x1, y1], col(COOLING)).filled(true).build();
            // Cyan rising from the bottom as the cooldown runs.
            let done = (1.0 - cooldown / max.max(0.01)).clamp(0.0, 1.0);
            draw.add_rect([x0, y1 - size * done], [x1, y1], col(alpha(CYAN, 0.85))).filled(true).build();
            picture(&draw, icon, [x0 + size / 2.0, y0 + size / 2.0], size * 0.8, 0.0, [255, 255, 255, 90]);
            let t = format!("{}", cooldown.ceil() as u32);
            let tw = measure(r.italic_big, &t);
            text([x0 + (size - tw[0]) / 2.0, y0 + (size - tw[1]) / 2.0], r.italic_big, WHITE, &t);
        }
        if active.is_some() {
            let pulse = 0.6 + 0.4 * (now * 10.0).sin();
            draw.add_rect([x0 - 2.0 * k, y0 - 2.0 * k], [x1 + 2.0 * k, y1 + 2.0 * k], col(alpha(CYAN, pulse))).thickness(3.0 * k).build();
        }
        // Ready flash (your Overwatch frames, 2026-10-08): a rectangle of light centred on the box, as tall above as
        // below, that shrinks to the box while losing its light, fast; a white core; pixel sparks drifting up.
        if let Some(t0) = ready_at {
            let p = (now - t0) / COOLDOWN_FLASH;
            if p < 1.0 {
                let ease = 1.0 - (1.0 - p) * (1.0 - p);
                let fade = (1.0 - p).powf(1.4);
                let (mx, my) = (x0 + size / 2.0, y0 + size / 2.0);
                let half_h = size * (0.5 + 2.6 * (1.0 - ease));
                let half_w = size * (0.42 + 0.08 * (1.0 - ease));
                let edge = col(alpha(BLUE_GLOW, 0.0));
                let body = col(alpha([70, 175, 255, 255], 0.85 * fade));
                let core = col(alpha([215, 245, 255, 255], 0.95 * fade));
                draw.add_rect_filled_multicolor([mx - half_w, my - half_h], [mx - half_w * 0.25, my + half_h], edge, body, body, edge);
                draw.add_rect_filled_multicolor([mx - half_w * 0.25, my - half_h], [mx, my + half_h], body, core, core, body);
                draw.add_rect_filled_multicolor([mx, my - half_h], [mx + half_w * 0.25, my + half_h], core, body, body, core);
                draw.add_rect_filled_multicolor([mx + half_w * 0.25, my - half_h], [mx + half_w, my + half_h], body, edge, edge, body);
                picture(&draw, r.glow_dot, [mx, my], size * (1.6 + 1.2 * (1.0 - ease)), 0.0, alpha(WHITE, 0.8 * fade));
                for i in 0..14 {
                    let f = i as f32;
                    let sx = mx + ((f * 37.0).sin() * 0.5) * size * 1.1;
                    let sy = my + ((f * 53.0).cos() * 0.5) * size * 2.2 - size * 1.2 * ease;
                    let sq = (2.0 + (i % 3) as f32 * 1.5) * k;
                    draw.add_rect([sx, sy], [sx + sq, sy + sq], col(alpha([170, 230, 255, 255], fade))).filled(true).build();
                }
            }
        }
        key_box(x0 + size / 2.0, y1 + 10.0 * k, key, ready);
        x += size + 12.0 * k;
    }
    // Ammo panel between two glowing cyan bars.
    let ax = x + 14.0 * k;
    let aw = width - ax - 36.0 * k;
    let (pt, pb) = (box_top - 6.0 * k, box_top + size + 6.0 * k);
    draw.add_rect([ax, pt], [ax + aw, pb], col([20, 24, 34, 90])).filled(true).build();
    for y in [pt, pb] {
        for (th, a) in [(9.0, 0.15), (5.0, 0.35), (2.5, 1.0)] {
            draw.add_line([ax + 6.0 * k, y], [ax + aw - 6.0 * k, y], col(alpha(CYAN, a))).thickness(th * k).build();
        }
        draw.add_line([ax, y + if y == pt { 6.0 } else { -6.0 } * k], [ax + 6.0 * k, y], col(CYAN)).thickness(2.0 * k).build();
        draw.add_line([ax + aw - 6.0 * k, y], [ax + aw, y + if y == pt { 6.0 } else { -6.0 } * k], col(CYAN)).thickness(2.0 * k).build();
    }
    let mid = (pt + pb) / 2.0;
    let icon_w = 78.0 * k;
    picture(&draw, r.shuriken, [ax + 18.0 * k + icon_w / 2.0, mid], icon_w, 0.0, WHITE);
    let tx = ax + 118.0 * k;
    match s.reloading {
        Some(done) => {
            let t = measure(r.bold_small, "RELOADING");
            text([tx, mid - t[1]], r.bold_small, CYAN, "RELOADING");
            let bar = 90.0 * k;
            draw.add_rect([tx, mid + 4.0 * k], [tx + bar, mid + 8.0 * k], col([255, 255, 255, 50])).filled(true).build();
            draw.add_rect([tx, mid + 4.0 * k], [tx + bar * done.clamp(0.0, 1.0), mid + 8.0 * k], col(CYAN)).filled(true).build();
        }
        None => {
            let ammo = s.ammo.to_string();
            let t = measure(r.bold_big, &ammo);
            text([tx, mid - t[1] / 2.0], r.bold_big, WHITE, &ammo);
            let sx = tx + t[0] + 7.0 * k;
            draw.add_line([sx, mid - 11.0 * k], [sx, mid + 11.0 * k], col(CYAN)).thickness(2.0 * k).build();
            let m = s.ammo_max.to_string();
            let mt = measure(r.bold_big, &m);
            text([sx + 7.0 * k, mid - mt[1] / 2.0], r.bold_big, WHITE, &m);
        }
    }
}

/// A clockwise arc from 12 o'clock covering `share` of the circle.
fn arc(draw: &DrawListMut, center: [f32; 2], radius: f32, share: f32, color: ImColor32, thickness: f32) {
    if share <= 0.0 {
        return;
    }
    let n = (72.0 * share).max(2.0) as usize;
    let points: Vec<[f32; 2]> = (0..=n)
        .map(|i| {
            let a = -FRAC_PI_2 + TAU * share * i as f32 / n as f32;
            [center[0] + radius * a.cos(), center[1] + radius * a.sin()]
        })
        .collect();
    draw.add_polyline(points, color).thickness(thickness).build();
}
