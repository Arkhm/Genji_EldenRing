//! The Overwatch-style HUD (step 12, DESIGN.md 5h), drawn by the overlay in `crosshair.rs`.
//!
//! The game thread writes a snapshot of Genji's state every frame (`update`); the overlay's render thread draws it.
//! Pictures are Overwatch's own UI art from your extracted files (outside the repository): ability icons, the hexagon
//! portrait, and effect textures (glow ring, ring of dashes, light flare) built into D:\GenjiBridgeAssets\hud by
//! tools\make_hud_art.py. Shapes and animations follow your Overwatch 2 screenshots and your analysed clip
//! (D:\Videos\edits_hyper\Overwatch_HUD_reference_20261008, HUD_ANIMATION_SPEC.txt, 2026-10-08): sizes, colours and
//! timings of the ultimate and LSHIFT tile were measured on its frames, which are at 1080 lines, so they're used
//! directly here as "1080-line pixels" (`k`). Overwatch's own HUD animations are engine effects, not pictures in its
//! UI files, so they are redrawn here. A missing picture is logged and left out.

use std::f32::consts::{FRAC_PI_2, PI, TAU};
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
/// Ticks of the ultimate's charge, between its inner and outer rings.
const ULT_TICKS: usize = 48;
/// Length of the burst when the ultimate becomes full (not in your clip: kept short), seconds.
const FLASH: f32 = 0.6;
/// Ultimate pressed: the ring expands and fades, the emblem gives way to the empty meter (clip part 2), seconds.
const ULT_PRESS: f32 = 0.4;
/// Cooldown finished: light bars, star, glow column, square particles, tile back to white (clip part 5), seconds.
const READY_FLASH: f32 = 0.6;
/// Ability used: the tile turns cyan over this long (clip part 3), seconds.
const LIT_IN: f32 = 0.07;
/// How long Swift Strike's tile stays cyan after the dash starts (clip part 3: about 0.4 s), seconds.
const SWIFT_LIT: f32 = 0.4;
/// After the cyan: the tile fades to grey while the countdown number fades in (clip part 3), seconds.
const COOLDOWN_ENTRY: f32 = 0.17;
/// Ability used: the tile is pushed down, its top dropping by this share of its height (clip part 3: about 10 of
/// 64 px), within PRESS_IN seconds.
const PRESS_DEPTH: f32 = 0.16;
const PRESS_IN: f32 = 0.12;
/// Electric ribbons around the ready ultimate (slots; each shows a ribbon for part of its cycle), and blue flames.
const ULT_WISPS: u32 = 3;
const ULT_FLAMES: u32 = 30;

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
    /// Uses so far, to play the "pressed" animation when they change.
    pub swift_uses: u32,
    pub deflect_uses: u32,
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
    swift_uses: 0,
    deflect_uses: 0,
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
    /// Made here, not loaded: plain white (solid shapes), white to light grey top to bottom (tiles), and a soft
    /// vertical bar (light streaks).
    white: Picture,
    tile_grad: Picture,
    bar: Picture,
    italic_ult: Option<FontId>,
    italic_cd: Option<FontId>,
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
    /// Use counters last seen (None until the first frame, so loading in doesn't play a press).
    swift_uses: Option<u32>,
    deflect_uses: Option<u32>,
    swift_pressed_at: Option<f32>,
    deflect_pressed_at: Option<f32>,
    swift_was_lit: bool,
    deflect_was_lit: bool,
    swift_lit_off_at: Option<f32>,
    deflect_lit_off_at: Option<f32>,
    blade_was_on: bool,
    ult_pressed_at: Option<f32>,
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
        let mut make = |w: u32, h: u32, pixel: &dyn Fn(f32, f32) -> [u8; 4]| -> Picture {
            let mut rgba = Vec::with_capacity((w * h * 4) as usize);
            for y in 0..h {
                for x in 0..w {
                    rgba.extend_from_slice(&pixel((x as f32 + 0.5) / w as f32, (y as f32 + 0.5) / h as f32));
                }
            }
            render.load_texture(&rgba, w, h).ok().map(|id| (id, [w as f32, h as f32]))
        };
        r.white = make(4, 4, &|_, _| [255; 4]);
        // Ready tile: 248 at the top to 212 at the bottom (clip part 3).
        r.tile_grad = make(4, 64, &|_, v| {
            let g = (255.0 - 38.0 * v) as u8;
            [g, g, g, 255]
        });
        r.bar = make(32, 128, &|u, v| {
            let across = (1.0 - (2.0 * u - 1.0).abs()).powf(0.3);
            let up = smooth((v / 0.3).min(1.0));
            [255, 255, 255, (255.0 * across * up) as u8]
        });
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
        // Clip: the ultimate's "0" is about 35 px tall, the cooldown digits about 30 px.
        r.italic_ult = add(ITALIC_FONT, 50.0);
        r.italic_cd = add(ITALIC_FONT, 42.0);
        r.bold_big = add(BOLD_FONT, 30.0);
        r.bold_small = add(BOLD_FONT, 18.0);
        r.label = add(LABEL_FONT, 15.0);
        glog!(
            "HUD",
            "resources: pictures {}/13, fonts {}/5",
            [
                r.portrait, r.plus, r.glow_ring, r.dash_ring, r.flare, r.glow_dot, r.swift, r.deflect, r.blade, r.shuriken,
                r.white, r.tile_grad, r.bar,
            ]
                .iter()
                .filter(|p| p.is_some())
                .count(),
            [r.italic_ult, r.italic_cd, r.bold_big, r.bold_small, r.label].iter().filter(|f| f.is_some()).count()
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
const CYAN: [u8; 4] = [90, 235, 255, 255];
const ICON_ON_WHITE: [u8; 4] = [40, 45, 60, 255];
/// Clip parts 3-4: the active tile and the cooldown fill are a saturated cyan; the waiting tile a see-through grey.
const ACTIVE_CYAN: [u8; 4] = [70, 255, 245, 255];
const FILL_CYAN: [u8; 4] = [68, 255, 250, 255];
const COOLING_GREY: [u8; 4] = [128, 130, 134, 205];
const ICON_DARK: [u8; 4] = [22, 24, 30, 255];
const OUTLINE: [u8; 4] = [15, 18, 26, 255];

fn col(c: [u8; 4]) -> ImColor32 {
    ImColor32::from_rgba(c[0], c[1], c[2], c[3])
}

fn mix(a: [u8; 4], b: [u8; 4], f: f32) -> [u8; 4] {
    let f = f.clamp(0.0, 1.0);
    std::array::from_fn(|i| (a[i] as f32 + (b[i] as f32 - a[i] as f32) * f) as u8)
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
    if r.anim.swift_uses.is_some_and(|u| u != s.swift_uses) {
        r.anim.swift_pressed_at = Some(now);
    }
    r.anim.swift_uses = Some(s.swift_uses);
    if r.anim.deflect_uses.is_some_and(|u| u != s.deflect_uses) {
        r.anim.deflect_pressed_at = Some(now);
    }
    r.anim.deflect_uses = Some(s.deflect_uses);
    let swift_lit = r.anim.swift_pressed_at.is_some_and(|t| now - t < SWIFT_LIT);
    if r.anim.swift_was_lit && !swift_lit {
        r.anim.swift_lit_off_at = Some(now);
    }
    r.anim.swift_was_lit = swift_lit;
    let deflect_lit = s.deflect_left.is_some();
    if r.anim.deflect_was_lit && !deflect_lit {
        r.anim.deflect_lit_off_at = Some(now);
    }
    r.anim.deflect_was_lit = deflect_lit;
    let blade_on = s.blade_left.is_some();
    if blade_on && !r.anim.blade_was_on {
        r.anim.ult_pressed_at = Some(now);
    }
    r.anim.blade_was_on = blade_on;

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
    // A key badge ("LSHIFT", "E", "Q"): filled white with a dark label when the ability is ready (`filled` 1), a white
    // outline with a light label when not (0); in between while it changes. `lean`: share of its width its right
    // side sits lower.
    let badge = |center_x: f32, top: f32, label: &str, filled: f32, lean: f32| {
        let t = measure(r.label, label);
        let h = 25.0 * k;
        let w = (t[0] + 16.0 * k).max(h - 1.0 * k);
        let (x0, x1) = (center_x - w / 2.0, center_x + w / 2.0);
        let d = w * lean;
        tile_quad(&draw, r.white, x0, top, x1, top + h, d, alpha([20, 24, 34, 255], 0.45 * (1.0 - filled)));
        tile_quad(&draw, r.white, x0, top, x1, top + h, d, alpha(PANEL_WHITE, filled));
        let edge = vec![[x0, top], [x1, top + d], [x1, top + h + d], [x0, top + h], [x0, top]];
        draw.add_polyline(edge, col(alpha([228, 230, 236, 255], 0.9 * (1.0 - filled)))).thickness(1.6 * k).build();
        let _f = r.label.map(|f| ui.push_font(f));
        draw.add_text([center_x - t[0] / 2.0, top + d / 2.0 + (h - t[1]) / 2.0], col(mix([228, 230, 236, 255], [26, 30, 40, 255], filled)), label);
    };
    // Big italic digits with a dark edge (ultimate and cooldown numbers).
    let digits = |center: [f32; 2], font: Option<FontId>, a: f32, t: &str| {
        let size = measure(font, t);
        let pos = [center[0] - size[0] / 2.0, center[1] - size[1] / 2.0];
        let _f = font.map(|f| ui.push_font(f));
        for (dx, dy) in [(-1.6, 0.0), (1.6, 0.0), (0.0, -1.6), (0.0, 1.6), (-1.2, -1.2), (1.2, 1.2), (-1.2, 1.2), (1.2, -1.2)] {
            draw.add_text([pos[0] + dx * k, pos[1] + dy * k], col(alpha(OUTLINE, 0.85 * a)), t);
        }
        draw.add_text(pos, col(alpha(WHITE, a)), t);
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

    // ---- Bottom centre: the ultimate. Sizes from your clip (1080-line pixels): centre 150 above the bottom; white
    // disc 39.5; dark line 41; bright cyan band 43-50; blue 51-53; pale ring 56; side markers 54-60; Q badge 22 px,
    // its top 66 below the centre.
    let (cx, cy) = (width / 2.0, height - 150.0 * k);
    let q_top = cy + 66.0 * k;
    let since_press = r.anim.ult_pressed_at.map_or(f32::MAX, |t| now - t);
    // The empty/charging meter (clip part 2, end): see-through dark centre, thin pale outer ring, thin teal inner ring
    // with gaps at top and bottom, the charge as ticks between them, the big italic number.
    let meter = |a: f32, percent: u32| {
        picture(&draw, r.glow_dot, [cx, cy], 130.0 * k, 0.0, alpha([6, 10, 24, 255], 0.55 * a));
        draw.add_circle([cx, cy], 55.0 * k, col(alpha([22, 28, 52, 255], 0.5 * a))).filled(true).num_segments(96).build();
        draw.add_circle([cx, cy], 56.0 * k, col(alpha([225, 230, 245, 255], 0.55 * a))).thickness(2.0 * k).num_segments(96).build();
        let gap = 0.14;
        for start in [-FRAC_PI_2 + gap, FRAC_PI_2 + gap] {
            arc_at(&draw, [cx, cy], 34.0 * k, start, PI - 2.0 * gap, col(alpha([45, 200, 195, 255], 0.9 * a)), 2.0 * k);
        }
        let lit = (percent as f32 / 100.0 * ULT_TICKS as f32).floor() as usize;
        for i in 0..ULT_TICKS {
            let ang = -FRAC_PI_2 + TAU * i as f32 / ULT_TICKS as f32;
            let (r0, r1) = (40.0 * k, 48.0 * k);
            if i >= lit {
                continue;
            }
            let c = alpha(CYAN, a);
            draw.add_line([cx + r0 * ang.cos(), cy + r0 * ang.sin()], [cx + r1 * ang.cos(), cy + r1 * ang.sin()], col(c)).thickness(2.2 * k).build();
        }
        digits([cx, cy], r.italic_ult, a, &percent.to_string());
    };
    if ult_ready {
        // Clip part 1, back to front: soft blue halo (brightest near the rim, breathing and flickering), blue flames,
        // fine arcs with wandering highlights, pale ring, bright cyan ring, white disc with the dark emblem, side
        // markers, short electric ribbons appearing at changing places on the rim, Q badge.
        let breathe = 0.5 + 0.5 * (now * 1.6).sin();
        let flicker = 0.85 + 0.15 * (now * 13.0).sin() * (now * 7.3).cos();
        picture(&draw, r.glow_dot, [cx, cy], 205.0 * k * (1.0 + 0.06 * breathe), 0.0, alpha([60, 140, 255, 255], (0.6 + 0.25 * breathe) * flicker));
        picture(&draw, r.glow_dot, [cx, cy], 150.0 * k, 0.0, alpha([90, 190, 255, 255], 0.75 + 0.25 * breathe));
        ult_flames(&draw, r.glow_dot, [cx, cy], 41.0 * k, now);
        for i in 0..4 {
            let f = i as f32;
            let start = now * (0.35 + 0.1 * f) * if i % 2 == 0 { 1.0 } else { -1.0 } + f * 1.7;
            let len = 0.9 + 0.5 * (now * 0.9 + f).sin();
            arc_at(&draw, [cx, cy], (61.0 + 2.5 * f) * k, start, len, col(alpha([170, 230, 255, 255], 0.35 + 0.25 * breathe)), 1.3 * k);
        }
        draw.add_circle([cx, cy], 56.0 * k, col(alpha([190, 245, 255, 255], 0.7 + 0.3 * flicker))).thickness(2.5 * k).num_segments(96).build();
        draw.add_circle([cx, cy], 52.0 * k, col([55, 181, 247, 255])).thickness(3.0 * k).num_segments(96).build();
        draw.add_circle([cx, cy], 46.5 * k, col([50, 255, 252, 255])).thickness(8.0 * k).num_segments(96).build();
        draw.add_circle([cx, cy], 41.5 * k, col([40, 100, 165, 255])).thickness(3.0 * k).num_segments(96).build();
        draw.add_circle([cx, cy], 39.5 * k, col([233, 233, 242, 255])).filled(true).num_segments(96).build();
        picture(&draw, r.blade, [cx, cy], 52.0 * k, 0.0, ICON_DARK);
        ult_wisps(&draw, [cx, cy], now, k);
        for side in [-1.0f32, 1.0] {
            let x = cx + side * 60.0 * k;
            draw.add_triangle([x, cy - 4.5 * k], [x, cy + 4.5 * k], [x - side * 6.0 * k, cy], col(WHITE)).filled(true).build();
        }
        if let Some(t0) = r.anim.ult_ready_at {
            let p = (now - t0) / FLASH;
            if p < 1.0 {
                picture(&draw, r.flare, [cx, cy], 80.0 * k * (1.0 + 2.0 * p), 0.0, alpha(WHITE, 1.0 - p));
            }
        }
        badge(cx, q_top, "Q", 1.0, 0.0);
    } else if since_press < ULT_PRESS {
        // Clip part 2: the halo and ribbons drop away; the turquoise ring expands from 69 to about 135 px, thinning
        // and fading; the white disc flares then fades; the empty meter with its "0" fades in; Q goes from filled to
        // outlined.
        let t = since_press;
        meter(((t - 0.1) / 0.15).clamp(0.0, 1.0), s.ult_percent);
        let e = 1.0 - (1.0 - (t / 0.3).min(1.0)).powi(3);
        let ring_a = 1.0 - ((t - 0.1) / 0.2).clamp(0.0, 1.0);
        draw.add_circle([cx, cy], (69.0 + 66.0 * e) * k, col(alpha([60, 235, 212, 255], ring_a))).thickness((14.0 - 12.0 * e) * k).num_segments(128).build();
        let disc_a = 1.0 - ((t - 0.05) / 0.15).clamp(0.0, 1.0);
        let disc = if t < 0.06 { [250, 252, 245, 255] } else { [233, 233, 242, 255] };
        draw.add_circle([cx, cy], 55.0 * k, col(alpha([200, 200, 215, 255], 0.7 * disc_a))).thickness(1.5 * k).num_segments(96).build();
        draw.add_circle([cx, cy], 39.5 * k, col(alpha(disc, disc_a))).filled(true).num_segments(96).build();
        picture(&draw, r.blade, [cx, cy], 52.0 * k, 0.0, alpha(ICON_DARK, disc_a));
        badge(cx, q_top, "Q", 1.0 - ((t - 0.15) / 0.2).clamp(0.0, 1.0), 0.0);
    } else {
        meter(1.0, s.ult_percent);
        badge(cx, q_top, "Q", 0.0, 0.0);
    }

    // ---- Dragonblade time left, under the crosshair (Overwatch shows ultimate time near the reticle; its exact look
    // is not in your clip): a cyan bar shrinking towards its middle.
    if let Some(left) = s.blade_left {
        let share = if s.blade_duration > 0.0 { (left / s.blade_duration).clamp(0.0, 1.0) } else { 0.0 };
        let (mx, my) = (width / 2.0, height / 2.0 + 46.0 * k);
        let (half, th) = (42.0 * k, 4.0 * k);
        draw.add_rect([mx - half - 1.5 * k, my - th / 2.0 - 1.5 * k], [mx + half + 1.5 * k, my + th / 2.0 + 1.5 * k], col([10, 14, 24, 150])).filled(true).rounding(2.0 * k).build();
        picture(&draw, r.glow_dot, [mx, my], half * 2.6 * share.max(0.2), 0.0, alpha([60, 180, 255, 255], 0.5));
        draw.add_rect([mx - half * share, my - th / 2.0], [mx + half * share, my + th / 2.0], col(CYAN)).filled(true).rounding(1.5 * k).build();
    }

    // ---- Bottom right: Swift Strike and Deflect tiles (clip parts 3-5; your screenshot of 2026-10-08: upright,
    // symmetric, the same size), then the ammo panel. LSHIFT's left edge is 434 px from the right edge, its top 172 px
    // above the bottom.
    let size = 64.0 * k;
    let round = 3.0 * k;
    let box_top = height - 172.0 * k;
    let mut x = width - 434.0 * k;
    for (icon, key, cooldown, max, lit, lit_since, lit_off_at, ready_at) in [
        (r.swift, "LSHIFT", s.swift_cooldown, s.swift_cooldown_max, swift_lit, r.anim.swift_pressed_at, r.anim.swift_lit_off_at, r.anim.swift_ready_at),
        (r.deflect, "E", s.deflect_cooldown, s.deflect_cooldown_max, deflect_lit, r.anim.deflect_pressed_at, r.anim.deflect_lit_off_at, r.anim.deflect_ready_at),
    ] {
        let ready = cooldown <= 0.0;
        let flash_t = ready_at.map(|t0| now - t0).filter(|t| *t < READY_FLASH);
        let entry = lit_off_at.map_or(1.0, |t| ((now - t) / COOLDOWN_ENTRY).clamp(0.0, 1.0));
        // Pressed (clip part 3): the tile is pushed down, its top dropping by about a sixth of its height within
        // 0.12 s while its bottom stays; it stays a little less pushed through the cooldown and pops back up as the
        // ready flash starts (part 5).
        let depth = if lit {
            lit_since.map_or(1.0, |t| 1.0 - (1.0 - ((now - t) / PRESS_IN).clamp(0.0, 1.0)).powi(2))
        } else if !ready {
            1.0 - 0.25 * entry
        } else {
            flash_t.map_or(0.0, |t| 0.75 * (1.0 - t / 0.1).max(0.0))
        };
        let (x0, x1, y1) = (x, x + size, box_top + size);
        let y0 = box_top + depth * PRESS_DEPTH * size;
        let mid = [x0 + size / 2.0, (y0 + y1) / 2.0];
        let solid = |top: f32, bottom: f32, c: [u8; 4], round_top: bool| {
            if c[3] > 0 {
                draw.add_rect([x0, top], [x1, bottom], col(c)).filled(true).rounding(round).round_top_left(round_top).round_top_right(round_top).build();
            }
        };
        let gradient = |tint: [u8; 4]| {
            if let Some((id, _)) = r.tile_grad {
                draw.add_image_rounded(id, [x0, y0], [x1, y1], round).col(col(tint)).build();
            }
        };
        if lit {
            // Used (part 3): the tile turns saturated cyan, the emblem stays dark.
            let blend = lit_since.map_or(1.0, |t| ((now - t) / LIT_IN).clamp(0.0, 1.0));
            gradient(mix(WHITE, ACTIVE_CYAN, blend));
            picture(&draw, icon, mid, size * 0.8, 0.0, ICON_DARK);
        } else if !ready {
            // Cooldown (parts 3-4): grey tile, cyan rising from the bottom, the emblem, the whole seconds left. Right
            // after the cyan, the tile fades to grey while the number fades in.
            solid(y0, y1, COOLING_GREY, true);
            let done = (1.0 - cooldown / max.max(0.01)).clamp(0.0, 1.0);
            let fill_top = y1 - (y1 - y0) * done;
            solid(fill_top, y1, FILL_CYAN, done > 0.97);
            if done > 0.02 && done < 0.97 {
                draw.add_line([x0, fill_top], [x1, fill_top], col(alpha([225, 255, 255, 255], 0.8))).thickness(1.5 * k).build();
            }
            if entry < 1.0 {
                solid(y0, y1, alpha(ACTIVE_CYAN, 1.0 - entry), true);
            }
            picture(&draw, icon, mid, size * 0.8, 0.0, ICON_DARK);
            digits(mid, r.italic_cd, entry, &format!("{}", cooldown.ceil() as u32));
        } else {
            // Ready: off-white tile, dark emblem. Just after a cooldown (part 5) the tile is luminous cyan first,
            // the emblem showing through it again from 0.1 s, white again by about 0.55 s.
            gradient(WHITE);
            match flash_t {
                Some(t) => {
                    let glow = if t < 0.12 { 0.95 } else { 0.95 * (1.0 - ((t - 0.12) / 0.43).clamp(0.0, 1.0)) };
                    solid(y0, y1, alpha([170, 245, 255, 255], glow), true);
                    // The emblem shows through as light blue first, dark again by the end.
                    let back = ((t - 0.1) / 0.15).clamp(0.0, 1.0);
                    let tint = mix([110, 190, 240, 255], ICON_DARK, ((t - 0.25) / 0.3).clamp(0.0, 1.0));
                    picture(&draw, icon, mid, size * 0.8, 0.0, alpha(tint, back));
                }
                None => picture(&draw, icon, mid, size * 0.8, 0.0, ICON_DARK),
            }
        }
        // Thin white underline, then the key badge (filled only when ready and not in use; after a cooldown it fills
        // in towards the end of the flash, clip part 5).
        draw.add_rect([x0, y1 + 4.0 * k], [x1, y1 + 6.5 * k], col(alpha(WHITE, 0.95))).filled(true).rounding(1.0 * k).build();
        let filled = match (ready && !lit, flash_t) {
            (false, _) => 0.0,
            (true, Some(t)) => ((t - 0.25) / 0.25).clamp(0.0, 1.0),
            (true, None) => 1.0,
        };
        badge(x0 + size / 2.0, y1 + 23.0 * k, key, filled, 0.0);
        if let Some(t) = flash_t {
            ready_flash(&draw, r, mid, size, t, height, k);
        }
        x += size + 14.0 * k;
    }
    // Ammo panel (your screenshot): see-through, between two thin glowing cyan lines with angled ends, each with a
    // brighter, thicker stretch (top line towards the right, bottom line in the middle).
    let ax = x + 12.0 * k;
    let aw = width - ax - 50.0 * k;
    let (pt, pb) = (height - 154.0 * k, height - 79.0 * k);
    draw.add_rect([ax, pt], [ax + aw, pb], col([255, 255, 255, 14])).filled(true).build();
    for (y, from, to) in [(pt, 0.55, 0.95), (pb, 0.45, 0.78)] {
        let inward = if y == pt { 6.0 * k } else { -6.0 * k };
        for (th, a) in [(8.0, 0.15), (4.0, 0.35), (2.0, 1.0)] {
            draw.add_line([ax + 6.0 * k, y], [ax + aw - 6.0 * k, y], col(alpha(CYAN, a))).thickness(th * k).build();
        }
        let (sx0, sx1) = (ax + aw * from, ax + aw * to);
        for (th, a) in [(12.0, 0.2), (4.5, 1.0)] {
            draw.add_line([sx0, y], [sx1, y], col(alpha([120, 245, 255, 255], a))).thickness(th * k).build();
        }
        draw.add_line([ax, y + inward], [ax + 6.0 * k, y], col(CYAN)).thickness(2.0 * k).build();
        draw.add_line([ax + aw - 6.0 * k, y], [ax + aw, y + inward], col(CYAN)).thickness(2.0 * k).build();
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

/// A number in 0..1 from a seed (the same seed always gives the same number).
fn hash(n: u32) -> f32 {
    let mut x = n.wrapping_mul(0x9E37_79B9) ^ 0x85EB_CA6B;
    x ^= x >> 15;
    x = x.wrapping_mul(0x2C1B_3C6D);
    x ^= x >> 12;
    (x & 0xFFFF) as f32 / 65536.0
}

/// Blue flames licking out of the ultimate's ring: soft glow particles that rise outward, swirl a little and fade.
fn ult_flames(draw: &DrawListMut, glow: Picture, center: [f32; 2], radius: f32, now: f32) {
    for i in 0..ULT_FLAMES {
        let period = 0.9 + 0.6 * hash(i * 7 + 1);
        let t = now / period + hash(i * 7 + 2);
        let (cycle, u) = (t.floor() as u32, t.fract());
        let seed = i * 977 + cycle * 131;
        let a = TAU * hash(seed) + (hash(seed + 1) - 0.5) * 0.8 * u;
        let rr = radius * (1.2 + 0.6 * u * (0.6 + 0.4 * hash(seed + 2)));
        let fade = (u * 6.0).min(1.0) * (1.0 - u).powf(1.3);
        let size = radius * (0.55 + 0.5 * u) * (0.7 + 0.5 * hash(seed + 3));
        let c = [center[0] + rr * a.cos(), center[1] + rr * a.sin()];
        picture(draw, glow, c, size, 0.0, alpha([60, 150, 255, 255], 0.8 * fade));
        picture(draw, glow, c, size * 0.45, 0.0, alpha([150, 225, 255, 255], 0.7 * fade));
    }
}

/// Electric ribbons (clip part 1): short, irregular cyan ribbons appear at changing points on the rim, some curling
/// around the circle, others stretching outward; they brighten, change shape and vanish quickly. Each slot shows one
/// for part of its cycle, at a new place each time.
fn ult_wisps(draw: &DrawListMut, center: [f32; 2], now: f32, k: f32) {
    const POINTS: usize = 22;
    for i in 0..ULT_WISPS {
        let period = 0.8 + 0.6 * hash(i * 13 + 1);
        let t = now / period + hash(i * 13 + 2);
        let (cycle, u) = (t.floor() as u32, t.fract());
        let seed = i * 7919 + cycle * 271;
        let window = 0.3 + 0.25 * hash(seed + 5);
        if u > window {
            continue;
        }
        let v = u / window;
        let life = (PI * v).sin().powf(0.5);
        let dir = if hash(seed + 1) < 0.5 { -1.0 } else { 1.0 };
        let start = TAU * hash(seed);
        let outward = hash(seed + 6);
        let length = (50.0 + 60.0 * hash(seed + 2)) * k;
        let sweep = length / (58.0 * k);
        let phase = hash(seed + 4) * 50.0;
        let path = |f: f32, extra: f32| -> [f32; 2] {
            let a = start + dir * sweep * f;
            let jag = ((f * 9.0 + now * 12.0 + phase).sin() * 3.0 + (f * 31.0 - now * 40.0 + phase).sin() * 1.0) * k * (0.3 + f);
            let rr = (54.0 + (6.0 + 50.0 * outward) * f * f + extra) * k + jag;
            [center[0] + rr * a.cos(), center[1] + rr * a.sin()]
        };
        let points: Vec<[f32; 2]> = (0..POINTS).map(|j| path(j as f32 / (POINTS - 1) as f32, 0.0)).collect();
        // Sometimes a short branch stretching outward from the middle.
        let branch: Vec<[f32; 2]> = if hash(seed + 7) > 0.55 {
            (0..8).map(|j| path(0.5 + 0.05 * j as f32, 3.0 * j as f32)).collect()
        } else {
            Vec::new()
        };
        for (width, color, strength) in [(30.0, [40, 140, 255, 255], 0.35), (14.0, [60, 220, 255, 255], 0.8), (4.0, [220, 250, 255, 255], 0.95)] {
            for line in [&points, &branch] {
                for j in 0..line.len().saturating_sub(1) {
                    let f = j as f32 / (line.len() - 1) as f32;
                    let taper = 0.25 + 0.75 * (PI * f).sin().powf(0.6);
                    draw.add_line(line[j], line[j + 1], col(alpha(color, life * strength)))
                        .thickness(width * k * taper)
                        .build();
                }
            }
        }
    }
}

/// A picture (usually plain white) stretched over a leaning tile: left edge at x0 from top to bottom, right edge at
/// x1, `drop` lower.
fn tile_quad(draw: &DrawListMut, tex: Picture, x0: f32, top: f32, x1: f32, bottom: f32, drop: f32, tint: [u8; 4]) {
    let Some((id, _)) = tex else {
        return;
    };
    if tint[3] == 0 {
        return;
    }
    draw.add_image_quad(id, [x0, top], [x1, top + drop], [x1, bottom + drop], [x0, bottom]).col(col(tint)).build();
}

/// An arc from angle `start` (radians, 0 = 3 o'clock, clockwise on screen) covering `len` radians.
fn arc_at(draw: &DrawListMut, center: [f32; 2], radius: f32, start: f32, len: f32, color: ImColor32, thickness: f32) {
    let n = ((len / TAU) * 96.0).max(2.0) as usize;
    let points: Vec<[f32; 2]> = (0..=n)
        .map(|i| {
            let a = start + len * i as f32 / n as f32;
            [center[0] + radius * a.cos(), center[1] + radius * a.sin()]
        })
        .collect();
    draw.add_polyline(points, color).thickness(thickness).build();
}

/// Cooldown finished (clip part 5), `t` seconds in, around the tile centred on `c`: long narrow cyan bars from high
/// above down to the bottom of the screen, their tops sinking as they fade (0-0.22 s); a white star burst on the tile
/// (0-0.17 s); a soft blue glow column above the tile (0.08-0.5 s); small square particles lifting and fading,
/// mostly above the tile (0.12-0.6 s).
fn ready_flash(draw: &DrawListMut, r: &Resources, c: [f32; 2], size: f32, t: f32, screen_h: f32, k: f32) {
    if t < 0.25 {
        let fade = if t < 0.12 { 1.0 } else { 1.0 - (t - 0.12) / 0.13 };
        let sink = smooth((t / 0.25).min(1.0));
        // (offset from the centre, width, height above the centre, colour, strength), measured on the clip.
        for (dx, w, up, color, a) in [
            (0.0, 90.0, 300.0, [40, 150, 255, 255], 0.7),
            (0.0, 30.0, 380.0, [70, 235, 255, 255], 1.0),
            (0.0, 9.0, 380.0, [215, 250, 255, 255], 0.9),
            (-32.0, 13.0, 260.0, [60, 220, 255, 255], 0.85),
            (32.0, 13.0, 300.0, [60, 220, 255, 255], 0.85),
            (-41.0, 6.0, 180.0, [80, 190, 255, 255], 0.6),
            (42.0, 6.0, 210.0, [80, 190, 255, 255], 0.6),
        ] {
            let top = c[1] - up * k * (1.0 - 0.85 * sink);
            let half = w * k / 2.0 * (1.0 - 0.2 * sink);
            let bx = c[0] + dx * k;
            if let Some((id, _)) = r.bar {
                draw.add_image(id, [bx - half, top], [bx + half, screen_h]).col(col(alpha(color, a * fade))).build();
            }
        }
    }
    if t < 0.17 {
        let a = 1.0 - t / 0.17;
        let grow = 1.0 + 0.3 * t / 0.17;
        picture(draw, r.glow_dot, c, 160.0 * k, 0.0, alpha([80, 220, 255, 255], a));
        picture(draw, r.glow_dot, c, 90.0 * k, 0.0, alpha(WHITE, a));
        picture(draw, r.glow_dot, c, 45.0 * k, 0.0, alpha(WHITE, a));
        let white = col(alpha(WHITE, a));
        let (v, h) = (75.0 * k * grow, 30.0 * k * grow);
        let (vw, hw) = (4.0 * k, 3.5 * k);
        draw.add_triangle([c[0], c[1] - v], [c[0] + vw, c[1]], [c[0] - vw, c[1]], white).filled(true).build();
        draw.add_triangle([c[0], c[1] + v], [c[0] + vw, c[1]], [c[0] - vw, c[1]], white).filled(true).build();
        draw.add_triangle([c[0] - h, c[1]], [c[0], c[1] - hw], [c[0], c[1] + hw], white).filled(true).build();
        draw.add_triangle([c[0] + h, c[1]], [c[0], c[1] - hw], [c[0], c[1] + hw], white).filled(true).build();
    }
    if (0.08..0.5).contains(&t) {
        let a = if t < 0.17 { (t - 0.08) / 0.09 } else { 1.0 - (t - 0.17) / 0.33 };
        let top = c[1] - size / 2.0 - 150.0 * k * (1.0 - 0.3 * ((t - 0.08) / 0.42));
        if let Some((id, _)) = r.bar {
            draw.add_image(id, [c[0] - 36.0 * k, top], [c[0] + 36.0 * k, c[1]]).col(col(alpha([60, 190, 255, 255], 0.9 * a))).build();
        }
        // The tile glows at its edges meanwhile.
        picture(draw, r.glow_dot, c, size * 2.2, 0.0, alpha([90, 220, 255, 255], 0.8 * a));
    }
    for i in 0..34u32 {
        let born = 0.12 + 0.12 * hash(i * 3 + 1);
        let life = 0.25 + 0.2 * hash(i * 3 + 2);
        let q = (t - born) / life;
        if !(0.0..1.0).contains(&q) {
            continue;
        }
        let spread = hash(i * 5 + 3) - 0.5;
        let px = c[0] + spread * spread.abs().sqrt() * 2.0 * 40.0 * k;
        let near_edge = hash(i * 5 + 7) < 0.2;
        let py0 = if near_edge { c[1] + (hash(i * 5 + 8) - 0.5) * size } else { c[1] - size / 2.0 - (10.0 + 120.0 * hash(i * 5 + 4)) * k };
        let py = py0 - 35.0 * k * q;
        let sq = (3.0 + 4.0 * hash(i * 5 + 6)) * k;
        let color = if hash(i * 5 + 9) < 0.5 { [90, 230, 255, 255] } else { [60, 150, 255, 255] };
        let a = (1.0 - q).powf(1.2) * (q * 6.0).min(1.0);
        draw.add_rect([px, py], [px + sq, py + sq], col(alpha(color, a))).filled(true).build();
    }
}

fn smooth(x: f32) -> f32 {
    x * x * (3.0 - 2.0 * x)
}
