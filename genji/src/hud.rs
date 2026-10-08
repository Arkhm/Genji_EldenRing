//! The Overwatch-style HUD (step 12, DESIGN.md 5h), drawn by the overlay in `crosshair.rs`.
//!
//! The game thread writes a snapshot of Genji's state every frame (`update`); the overlay's render thread draws it.
//! Icons and the portrait are Overwatch's own images, read at startup from your extracted files (outside the
//! repository); a missing file only removes that picture.

use std::path::Path;
use std::sync::Mutex;

use hudhook::imgui::{Context, FontConfig, FontId, FontSource, ImColor32, TextureId, Ui};
use hudhook::RenderContext;

use crate::glog;

/// Where tools\extract_genji.ps1 put Overwatch's files (your choice, 2026-10-08).
const ASSETS: &str = r"D:\GenjiBridgeAssets\raw";
const PORTRAIT: &str = r"Heroes\Genji\GUI\0000000040C7.01C.png";
const ICON_SWIFT: &str = r"Abilities\Swift Strike\000000004E94.png";
const ICON_DEFLECT: &str = r"Abilities\Deflect\000000004E95.png";
const ICON_BLADE: &str = r"Abilities\Dragonblade\000000004E93.png";
const ICON_SHURIKEN: &str = r"Abilities\Shuriken\000000004E83.png";
const FONT: &str = r"C:\Windows\Fonts\bahnschrift.ttf";
const FONT_FALLBACK: &str = r"C:\Windows\Fonts\segoeuib.ttf";
const REFERENCE_HEIGHT: f32 = 1080.0;

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

/// Textures and fonts, created once by the overlay.
#[derive(Default)]
pub struct Resources {
    portrait: Option<(TextureId, [f32; 2])>,
    swift: Option<(TextureId, [f32; 2])>,
    deflect: Option<(TextureId, [f32; 2])>,
    blade: Option<(TextureId, [f32; 2])>,
    shuriken: Option<(TextureId, [f32; 2])>,
    big: Option<FontId>,
    small: Option<FontId>,
}

// SAFETY: font ids are pointers into ImGui's font atlas; they are created and used only on the overlay's render
// thread (hudhook moves the render loop there once, at startup). The bound only exists for that one-time move.
unsafe impl Send for Resources {}
unsafe impl Sync for Resources {}

impl Resources {
    /// Loads the images and fonts (called from the overlay's initialize, before the font atlas is built).
    pub fn load(ctx: &mut Context, render: &mut dyn RenderContext) -> Self {
        let mut r = Resources::default();
        let mut load = |name: &str| -> Option<(TextureId, [f32; 2])> {
            let path = Path::new(ASSETS).join(name);
            match decode_png(&path) {
                Ok((rgba, w, h)) => match render.load_texture(&rgba, w, h) {
                    Ok(id) => Some((id, [w as f32, h as f32])),
                    Err(e) => {
                        glog!("HUD", "texture upload failed for {}: {e:?}", path.display());
                        None
                    }
                },
                Err(e) => {
                    glog!("HUD", "image {} not loaded: {e}", path.display());
                    None
                }
            }
        };
        r.portrait = load(PORTRAIT);
        r.swift = load(ICON_SWIFT);
        r.deflect = load(ICON_DEFLECT);
        r.blade = load(ICON_BLADE);
        r.shuriken = load(ICON_SHURIKEN);
        let scale = (ctx.io().display_size[1] / REFERENCE_HEIGHT).max(0.5);
        let font = std::fs::read(FONT).or_else(|_| std::fs::read(FONT_FALLBACK));
        match font {
            Ok(bytes) => {
                // ImGui keeps a pointer to the font data for the atlas: give it data that lives forever.
                let data: &'static [u8] = Box::leak(bytes.into_boxed_slice());
                let fonts = ctx.fonts();
                fonts.add_font(&[FontSource::DefaultFontData { config: None }]);
                r.big = Some(fonts.add_font(&[FontSource::TtfData { data, size_pixels: 40.0 * scale, config: Some(FontConfig::default()) }]));
                r.small = Some(fonts.add_font(&[FontSource::TtfData { data, size_pixels: 20.0 * scale, config: Some(FontConfig::default()) }]));
            }
            Err(e) => glog!("HUD", "font not loaded ({e}); default font used"),
        }
        glog!(
            "HUD",
            "resources: portrait {}, icons {}/4, fonts {}",
            r.portrait.is_some(),
            [r.swift, r.deflect, r.blade, r.shuriken].iter().filter(|i| i.is_some()).count(),
            r.big.is_some()
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
const DIM: [u8; 4] = [200, 200, 210, 170];
const SHADOW: [u8; 4] = [0, 0, 0, 150];
const PANEL: [u8; 4] = [20, 24, 36, 150];
const ACCENT: [u8; 4] = [90, 230, 255, 255];

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
        draw.add_text([pos[0] + 2.0 * k, pos[1] + 2.0 * k], col(SHADOW), t);
        draw.add_text(pos, col(color), t);
    };
    let measure = |font: Option<FontId>, t: &str| {
        let _f = font.map(|f| ui.push_font(f));
        ui.calc_text_size(t)
    };

    // Bottom left: portrait, health numbers, 10 segments.
    let (px, py, ps) = (40.0 * k, height - 170.0 * k, 110.0 * k);
    draw.add_rect([px - 3.0 * k, py - 3.0 * k], [px + ps + 3.0 * k, py + ps + 3.0 * k], col(PANEL)).filled(true).build();
    if let Some((id, _)) = r.portrait {
        draw.add_image(id, [px, py], [px + ps, py + ps]).build();
    }
    draw.add_rect([px - 3.0 * k, py - 3.0 * k], [px + ps + 3.0 * k, py + ps + 3.0 * k], col(ACCENT)).thickness(2.0 * k).build();
    let hx = px + ps + 24.0 * k;
    let hp = s.hp.max(0);
    let hp_text = format!("{hp}");
    text([hx, py + 10.0 * k], r.big, WHITE, &hp_text);
    let w = measure(r.big, &hp_text)[0];
    text([hx + w + 6.0 * k, py + 26.0 * k], r.small, DIM, &format!("| {}", s.max_hp));
    let (seg_w, seg_h, gap) = (26.0 * k, 24.0 * k, 5.0 * k);
    let fill = if s.max_hp > 0 { (hp as f32 / s.max_hp as f32).clamp(0.0, 1.0) * 10.0 } else { 0.0 };
    for i in 0..10 {
        let x0 = hx + i as f32 * (seg_w + gap);
        let y0 = py + 66.0 * k;
        draw.add_rect([x0, y0], [x0 + seg_w, y0 + seg_h], col([255, 255, 255, 50])).filled(true).build();
        let part = (fill - i as f32).clamp(0.0, 1.0);
        if part > 0.0 {
            draw.add_rect([x0, y0], [x0 + seg_w * part, y0 + seg_h], col([245, 245, 245, 235])).filled(true).build();
        }
    }

    // Bottom centre: the ultimate ring.
    let (cx, cy, radius) = (width / 2.0, height - 120.0 * k, 46.0 * k);
    draw.add_circle([cx, cy], radius + 6.0 * k, col(PANEL)).filled(true).num_segments(64).build();
    draw.add_circle([cx, cy], radius, col([255, 255, 255, 60])).thickness(6.0 * k).num_segments(64).build();
    let (share, ready) = match s.blade_left {
        Some(left) if s.blade_duration > 0.0 => ((left / s.blade_duration).clamp(0.0, 1.0), false),
        _ => ((s.ult_percent as f32 / 100.0).clamp(0.0, 1.0), s.ult_percent >= 100),
    };
    if share > 0.0 {
        let n = (64.0 * share).max(2.0) as usize;
        let start = -std::f32::consts::FRAC_PI_2;
        let points: Vec<[f32; 2]> = (0..=n)
            .map(|i| {
                let a = start + std::f32::consts::TAU * share * i as f32 / n as f32;
                [cx + radius * a.cos(), cy + radius * a.sin()]
            })
            .collect();
        draw.add_polyline(points, col(if ready || s.blade_left.is_some() { ACCENT } else { WHITE })).thickness(6.0 * k).build();
    }
    if ready || s.blade_left.is_some() {
        if let Some((id, _)) = r.blade {
            let half = radius * 0.62;
            draw.add_image(id, [cx - half, cy - half], [cx + half, cy + half]).build();
        }
        let label = match s.blade_left {
            Some(left) => format!("{left:.1}"),
            None => "Q".to_string(),
        };
        let lw = measure(r.small, &label);
        text([cx - lw[0] / 2.0, cy + radius + 10.0 * k], r.small, ACCENT, &label);
    } else {
        let label = format!("{}%", s.ult_percent);
        let lw = measure(r.big, &label);
        text([cx - lw[0] / 2.0, cy - lw[1] / 2.0], r.big, WHITE, &label);
    }

    // Bottom right: Swift Strike, Deflect, ammo.
    let size = 64.0 * k;
    let base_y = height - 160.0 * k;
    let mut x = width - 520.0 * k;
    for (icon, key, cooldown, active) in [
        (r.swift, "LSHIFT", s.swift_cooldown, None),
        (r.deflect, "E", s.deflect_cooldown, s.deflect_left),
    ] {
        draw.add_rect([x, base_y], [x + size, base_y + size], col(PANEL)).filled(true).build();
        if let Some((id, _)) = icon {
            let tint = if cooldown > 0.0 { [255, 255, 255, 90] } else { [255, 255, 255, 255] };
            draw.add_image(id, [x + 4.0 * k, base_y + 4.0 * k], [x + size - 4.0 * k, base_y + size - 4.0 * k]).col(col(tint)).build();
        }
        let border = if active.is_some() { ACCENT } else if cooldown > 0.0 { [255, 255, 255, 60] } else { [255, 255, 255, 200] };
        draw.add_rect([x, base_y], [x + size, base_y + size], col(border)).thickness(2.0 * k).build();
        if cooldown > 0.0 {
            let t = format!("{}", cooldown.ceil() as u32);
            let tw = measure(r.big, &t);
            text([x + (size - tw[0]) / 2.0, base_y + (size - tw[1]) / 2.0], r.big, WHITE, &t);
        }
        let kw = measure(r.small, key);
        text([x + (size - kw[0]) / 2.0, base_y + size + 6.0 * k], r.small, DIM, key);
        x += size + 18.0 * k;
    }
    let ax = width - 330.0 * k;
    if let Some((id, [w, h])) = r.shuriken {
        let iw = 110.0 * k;
        let ih = iw * h / w;
        draw.add_image(id, [ax, base_y + 10.0 * k], [ax + iw, base_y + 10.0 * k + ih]).build();
    }
    let tx = ax + 125.0 * k;
    match s.reloading {
        Some(done) => {
            text([tx, base_y + 14.0 * k], r.small, ACCENT, "RELOADING");
            let bw = 120.0 * k;
            draw.add_rect([tx, base_y + 44.0 * k], [tx + bw, base_y + 50.0 * k], col([255, 255, 255, 50])).filled(true).build();
            draw.add_rect([tx, base_y + 44.0 * k], [tx + bw * done.clamp(0.0, 1.0), base_y + 50.0 * k], col(ACCENT)).filled(true).build();
        }
        None => {
            let ammo = format!("{}", s.ammo);
            text([tx, base_y + 4.0 * k], r.big, WHITE, &ammo);
            let w = measure(r.big, &ammo)[0];
            text([tx + w + 6.0 * k, base_y + 20.0 * k], r.small, DIM, &format!("| {}", s.ammo_max));
        }
    }
}
