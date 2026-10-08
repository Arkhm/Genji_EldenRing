//! The Overwatch-style HUD (step 12, DESIGN.md 5h), drawn by the overlay in `crosshair.rs`.
//!
//! The game thread writes a snapshot of Genji's state every frame (`update`); the overlay's render thread draws it.
//! Pictures are Overwatch's own UI art from your extracted files (outside the repository): ability icons, and the
//! hexagon portrait and the ultimate disc built by tools\make_hud_art.py from Overwatch's UI textures. The rest
//! (slanted health segments, ability frames, key boxes) are simple shapes drawn here with the proportions of your
//! Overwatch 2 screenshot (2026-10-08). A missing picture is logged and left out.

use std::path::Path;
use std::sync::Mutex;

use hudhook::imgui::{Context, FontConfig, FontId, FontSource, ImColor32, TextureId, Ui};
use hudhook::RenderContext;

use crate::glog;

const RAW: &str = r"D:\GenjiBridgeAssets\raw";
const HUD_ART: &str = r"D:\GenjiBridgeAssets\hud";
const ICON_SWIFT: &str = r"Abilities\Swift Strike\000000004E94.png";
const ICON_DEFLECT: &str = r"Abilities\Deflect\000000004E95.png";
const ICON_BLADE: &str = r"Abilities\Dragonblade\000000004E93.png";
const ICON_SHURIKEN: &str = r"Abilities\Shuriken\000000004E83.png";
/// Overwatch's HUD numbers are a heavy oblique; the closest every Windows has is Segoe UI Black Italic.
const NUMBER_FONT: &str = r"C:\Windows\Fonts\seguibli.ttf";
const LABEL_FONT: &str = r"C:\Windows\Fonts\bahnschrift.ttf";
const REFERENCE_HEIGHT: f32 = 1080.0;
/// The HUD's slant: horizontal shift per unit of height (Overwatch 2 leans its bars and frames to the right).
const SLANT: f32 = 0.22;

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
    pub deflect_left: Option<f32>,
    pub deflect_cooldown: f32,
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
    deflect_left: None,
    deflect_cooldown: 0.0,
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

/// Textures and fonts, created once by the overlay.
#[derive(Default)]
pub struct Resources {
    portrait: Picture,
    ult_disc: Picture,
    plus: Picture,
    swift: Picture,
    deflect: Picture,
    blade: Picture,
    shuriken: Picture,
    number_big: Option<FontId>,
    number_small: Option<FontId>,
    label: Option<FontId>,
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
        r.ult_disc = load(HUD_ART, "ult_disc.png");
        r.plus = load(HUD_ART, "plus.png");
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
        r.number_big = add(NUMBER_FONT, 34.0);
        r.number_small = add(NUMBER_FONT, 17.0);
        r.label = add(LABEL_FONT, 13.0);
        glog!(
            "HUD",
            "resources: pictures {}/7, fonts {}/3",
            [r.portrait, r.ult_disc, r.plus, r.swift, r.deflect, r.blade, r.shuriken].iter().filter(|p| p.is_some()).count(),
            [r.number_big, r.number_small, r.label].iter().filter(|f| f.is_some()).count()
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
const SOFT: [u8; 4] = [235, 238, 245, 230];
const DIM: [u8; 4] = [210, 215, 225, 190];
const SHADOW: [u8; 4] = [0, 0, 0, 120];
const DARK: [u8; 4] = [25, 28, 40, 170];
const CYAN: [u8; 4] = [100, 230, 255, 255];
const ICON_ON_WHITE: [u8; 4] = [45, 50, 65, 255];

fn col(c: [u8; 4]) -> ImColor32 {
    ImColor32::from_rgba(c[0], c[1], c[2], c[3])
}

/// Draws the HUD (overlay render thread).
pub fn draw(ui: &Ui, r: &Resources, width: f32, height: f32) {
    let s = *STATE.lock().unwrap_or_else(|e| e.into_inner());
    if !s.visible {
        return;
    }
    let k = height / REFERENCE_HEIGHT;
    let draw = ui.get_foreground_draw_list();
    let text = |pos: [f32; 2], font: Option<FontId>, color: [u8; 4], t: &str| {
        let _f = font.map(|f| ui.push_font(f));
        draw.add_text([pos[0] + 1.5 * k, pos[1] + 1.5 * k], col(SHADOW), t);
        draw.add_text(pos, col(color), t);
    };
    let measure = |font: Option<FontId>, t: &str| {
        let _f = font.map(|f| ui.push_font(f));
        ui.calc_text_size(t)
    };
    // A parallelogram leaning right: x0 is its left edge at the bottom.
    let slanted = |x0: f32, y_top: f32, w: f32, h: f32, color: [u8; 4], filled: bool, thickness: f32| {
        let lean = SLANT * h;
        let (a, b, c, d) = ([x0 + lean, y_top], [x0 + lean + w, y_top], [x0 + w, y_top + h], [x0, y_top + h]);
        if filled {
            draw.add_triangle(a, b, c, col(color)).filled(true).build();
            draw.add_triangle(a, c, d, col(color)).filled(true).build();
        } else {
            draw.add_polyline(vec![a, b, c, d, a], col(color)).thickness(thickness).build();
        }
    };
    // A small dark key box with a label ("LSHIFT", "E", "Q").
    let key_box = |center_x: f32, top: f32, label: &str| {
        let t = measure(r.label, label);
        let (w, h) = (t[0] + 10.0 * k, t[1] + 4.0 * k);
        let x = center_x - w / 2.0;
        draw.add_rect([x, top], [x + w, top + h], col([240, 242, 248, 225])).filled(true).rounding(2.0 * k).build();
        let _f = r.label.map(|f| ui.push_font(f));
        draw.add_text([x + 5.0 * k, top + 2.0 * k], col([30, 34, 46, 255]), label);
    };

    // Bottom left: hexagon portrait, "+" box, health numbers, 10 slanted segments.
    let base = height - 62.0 * k; // bottom line of the HUD
    let (pw, ph) = (88.0 * k, 106.0 * k);
    let (px, py) = (36.0 * k, base - ph + 8.0 * k);
    if let Some((id, _)) = r.portrait {
        draw.add_image(id, [px, py], [px + pw, py + ph]).build();
    }
    let hx = px + pw + 18.0 * k;
    let seg_top = base - 34.0 * k;
    let num_y = seg_top - 44.0 * k;
    let plus = 22.0 * k;
    draw.add_rect([hx, num_y + 10.0 * k], [hx + plus, num_y + 10.0 * k + plus], col(SOFT)).filled(true).rounding(2.0 * k).build();
    if let Some((id, _)) = r.plus {
        let m = 4.0 * k;
        draw.add_image(id, [hx + m, num_y + 10.0 * k + m], [hx + plus - m, num_y + 10.0 * k + plus - m]).col(col(ICON_ON_WHITE)).build();
    }
    let hp = s.hp.max(0).to_string();
    let nx = hx + plus + 10.0 * k;
    text([nx, num_y], r.number_big, WHITE, &hp);
    let w = measure(r.number_big, &hp)[0];
    text([nx + w + 8.0 * k, num_y + 13.0 * k], r.number_small, DIM, &s.max_hp.to_string());
    let (seg_w, seg_h, gap) = (21.0 * k, 26.0 * k, 4.0 * k);
    let fill = if s.max_hp > 0 { (s.hp.max(0) as f32 / s.max_hp as f32).clamp(0.0, 1.0) * 10.0 } else { 0.0 };
    for i in 0..10 {
        let x0 = hx + i as f32 * (seg_w + gap);
        slanted(x0, seg_top, seg_w, seg_h, [255, 255, 255, 45], true, 0.0);
        let part = (fill - i as f32).clamp(0.0, 1.0);
        if part >= 0.999 {
            slanted(x0, seg_top, seg_w, seg_h, SOFT, true, 0.0);
        } else if part > 0.0 {
            slanted(x0, seg_top, seg_w * part, seg_h, SOFT, true, 0.0);
        }
    }

    // Bottom centre: the ultimate disc, a ring that fills (or drains during Dragonblade), the number, "Q".
    let radius = 38.0 * k;
    let (cx, cy) = (width / 2.0, base - radius - 22.0 * k);
    match r.ult_disc {
        Some((id, _)) => {
            let e = radius * 1.18;
            draw.add_image(id, [cx - e, cy - e], [cx + e, cy + e]).build();
        }
        None => {
            draw.add_circle([cx, cy], radius, col(DARK)).filled(true).num_segments(64).build();
        }
    }
    let (share, ready) = match s.blade_left {
        Some(left) if s.blade_duration > 0.0 => ((left / s.blade_duration).clamp(0.0, 1.0), false),
        _ => ((s.ult_percent as f32 / 100.0).clamp(0.0, 1.0), s.ult_percent >= 100),
    };
    draw.add_circle([cx, cy], radius, col([255, 255, 255, 70])).thickness(3.0 * k).num_segments(64).build();
    if share > 0.0 {
        let n = (72.0 * share).max(2.0) as usize;
        let start = -std::f32::consts::FRAC_PI_2;
        let points: Vec<[f32; 2]> = (0..=n)
            .map(|i| {
                let a = start + std::f32::consts::TAU * share * i as f32 / n as f32;
                [cx + radius * a.cos(), cy + radius * a.sin()]
            })
            .collect();
        draw.add_polyline(points, col(if ready || s.blade_left.is_some() { CYAN } else { WHITE })).thickness(4.0 * k).build();
    }
    if ready || s.blade_left.is_some() {
        if let Some((id, _)) = r.blade {
            let half = radius * 0.6;
            draw.add_image(id, [cx - half, cy - half], [cx + half, cy + half]).build();
        }
        if let Some(left) = s.blade_left {
            let t = format!("{left:.1}");
            let tw = measure(r.number_small, &t);
            text([cx - tw[0] / 2.0, cy + radius * 0.55], r.number_small, CYAN, &t);
        }
    } else {
        let t = s.ult_percent.to_string();
        let tw = measure(r.number_big, &t);
        text([cx - tw[0] / 2.0, cy - tw[1] / 2.0], r.number_big, WHITE, &t);
    }
    key_box(cx, cy + radius + 8.0 * k, "Q");

    // Bottom right: Swift Strike and Deflect frames (white when ready, dark with seconds while cooling, cyan edge
    // while Deflect is up), then the shuriken with ammo between two cyan lines.
    let (bw, bh) = (52.0 * k, 50.0 * k);
    let box_top = base - bh - 18.0 * k;
    let mut x = width - 395.0 * k;
    for (icon, key, cooldown, active) in [
        (r.swift, "LSHIFT", s.swift_cooldown, None),
        (r.deflect, "E", s.deflect_cooldown, s.deflect_left),
    ] {
        let ready = cooldown <= 0.0;
        slanted(x, box_top, bw, bh, if ready { SOFT } else { DARK }, true, 0.0);
        slanted(x, box_top, bw, bh, if active.is_some() { CYAN } else { [255, 255, 255, 200] }, false, 2.0 * k);
        let lean = SLANT * bh / 2.0;
        if let Some((id, _)) = icon {
            let m = 7.0 * k;
            let tint = if ready { ICON_ON_WHITE } else { [255, 255, 255, 90] };
            draw.add_image(id, [x + lean + m, box_top + m], [x + lean + bw - m, box_top + bh - m]).col(col(tint)).build();
        }
        if !ready {
            let t = format!("{}", cooldown.ceil() as u32);
            let tw = measure(r.number_big, &t);
            text([x + lean + (bw - tw[0]) / 2.0, box_top + (bh - tw[1]) / 2.0], r.number_big, WHITE, &t);
        }
        key_box(x + lean + bw / 2.0, box_top + bh + 6.0 * k, key);
        x += bw + 14.0 * k;
    }
    let ax = x + 26.0 * k;
    let line_w = width - ax - 40.0 * k;
    let ammo_mid = box_top + bh / 2.0;
    for dy in [-30.0 * k, 30.0 * k] {
        let y = ammo_mid + dy;
        draw.add_line([ax, y], [ax + line_w, y - 6.0 * k], col([100, 230, 255, 170])).thickness(1.5 * k).build();
    }
    if let Some((id, [w, h])) = r.shuriken {
        let iw = 74.0 * k;
        let ih = iw * h / w;
        draw.add_image(id, [ax + 6.0 * k, ammo_mid - ih / 2.0], [ax + 6.0 * k + iw, ammo_mid + ih / 2.0]).build();
    }
    let tx = ax + 96.0 * k;
    match s.reloading {
        Some(done) => {
            let t = measure(r.number_small, "RELOADING");
            text([tx, ammo_mid - t[1]], r.number_small, CYAN, "RELOADING");
            let bar = 90.0 * k;
            draw.add_rect([tx, ammo_mid + 4.0 * k], [tx + bar, ammo_mid + 8.0 * k], col([255, 255, 255, 50])).filled(true).build();
            draw.add_rect([tx, ammo_mid + 4.0 * k], [tx + bar * done.clamp(0.0, 1.0), ammo_mid + 8.0 * k], col(CYAN)).filled(true).build();
        }
        None => {
            let ammo = s.ammo.to_string();
            let t = measure(r.number_big, &ammo);
            text([tx, ammo_mid - t[1] / 2.0], r.number_big, WHITE, &ammo);
            text([tx + t[0] + 8.0 * k, ammo_mid - t[1] / 2.0 + 13.0 * k], r.number_small, DIM, &s.ammo_max.to_string());
        }
    }
}
