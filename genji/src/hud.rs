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
/// Bars of the ultimate's charge (ULT GAINED spec, section 2: 67).
const ULT_TICKS: usize = 67;
/// The ultimate becoming ready, from the hard cut to the ready state (ULT GAINED spec: 72 frames), seconds.
const ULT_GAIN: f32 = 1.2;
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
/// The "+" pill stays yellow this long after health last went up, seconds (your Overwatch screenshots: it shows
/// while healing; Elden Ring's flask restores health over a moment).
const HEAL_HOLD: f32 = 0.8;
/// The pink "health went down" arrow shows this long after the last damage, seconds.
const DAMAGE_HOLD: f32 = 1.2;

/// Ability used: the tile is pushed down, its top dropping by this share of its height (clip part 3: about 10 of
/// 64 px), within PRESS_IN seconds.
const PRESS_DEPTH: f32 = 0.16;
const PRESS_IN: f32 = 0.12;
/// Pieces of the blue fire band around the ready ultimate.
const AURA_PIECES: usize = 120;

/// Enemy health bars drawn at most (DESIGN.md 5i).
pub const ENEMY_BARS: usize = 8;

/// Longest boss name kept, in bytes (UTF-8).
pub const NAME_BYTES: usize = 32;

/// One enemy's health bar: where (screen, -1 to 1 from the centre, y up), how full (health, and health just lost
/// still lit lighter), whether it's a boss (the long bar with the skull badge) and its name (bosses; may be empty).
#[derive(Clone, Copy, Default)]
pub struct EnemyBar {
    pub x: f32,
    pub y: f32,
    pub hp: f32,
    pub trail: f32,
    pub boss: bool,
    pub name: [u8; NAME_BYTES],
    pub name_len: u8,
}

impl EnemyBar {
    pub fn name(&self) -> &str {
        std::str::from_utf8(&self.name[..self.name_len as usize]).unwrap_or("")
    }
}

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
    pub enemies: [EnemyBar; ENEMY_BARS],
    pub enemy_count: usize,
    /// Eliminations so far, to play the kill skull when it goes up.
    pub kills: u32,
    /// Your status effects building up (status.rs), in its order.
    pub statuses: [crate::status::Bar; crate::status::COUNT],
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
    enemies: [EnemyBar { x: 0.0, y: 0.0, hp: 0.0, trail: 0.0, boss: false, name: [0; NAME_BYTES], name_len: 0 }; ENEMY_BARS],
    enemy_count: 0,
    kills: 0,
    statuses: [crate::status::Bar { fill: 0.0 }; crate::status::COUNT],
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
    skull: Picture,
    kill_skull: Picture,
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
    /// Soft across (strips: bright in the middle, clear at both edges), and fading from top to bottom (fire band).
    ribbon: Picture,
    fade: Picture,
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
    /// Health last frame (None while the HUD is hidden, so coming back doesn't count as a change).
    last_hp: Option<i32>,
    last_max_hp: i32,
    healed_at: Option<f32>,
    damaged_at: Option<f32>,
    /// How far the "+" pill shows healing (yellow) and damage (pink arrow), 0-1, eased.
    heal_shown: f32,
    damage_shown: f32,
    last_now: f32,
    blade_was_on: bool,
    ult_pressed_at: Option<f32>,
    /// Kill count last seen, and when the kill skull last started.
    kills: Option<u32>,
    kill_at: Option<f32>,
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
        r.skull = load(HUD_ART, "skull.png");
        r.kill_skull = load(HUD_ART, "kill_skull.png");
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
        r.ribbon = make(64, 4, &|u, _| {
            let t = smooth(1.0 - (2.0 * u - 1.0).abs());
            [255, 255, 255, (255.0 * t) as u8]
        });
        r.fade = make(4, 64, &|_, v| [255, 255, 255, (255.0 * (1.0 - v).powf(1.1)) as u8]);
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
            "resources: pictures {}/15, fonts {}/5",
            [
                r.portrait, r.plus, r.glow_ring, r.dash_ring, r.flare, r.glow_dot, r.swift, r.deflect, r.blade, r.shuriken,
                r.white, r.tile_grad, r.bar, r.ribbon, r.fade,
            ]
                .iter()
                .filter(|p| p.is_some())
                .count(),
            [r.italic_ult, r.italic_cd, r.bold_big, r.bold_small, r.label].iter().filter(|f| f.is_some()).count()
        );
        r
    }
}

pub(crate) fn decode_png(path: &Path) -> Result<(Vec<u8>, u32, u32), String> {
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
/// The status bars' colours, in status.rs's order (poison, scarlet rot, blood loss, death blight, frostbite, sleep,
/// madness), close to Elden Ring's own gauges.
const STATUS_COLOURS: [[u8; 4]; crate::status::COUNT] = [
    [168, 204, 64, 255],
    [232, 96, 44, 255],
    [222, 40, 52, 255],
    [176, 160, 120, 255],
    [150, 208, 245, 255],
    [160, 130, 225, 255],
    [245, 176, 40, 255],
];
const SHADOW: [u8; 4] = [0, 0, 0, 110];
const CYAN: [u8; 4] = [90, 235, 255, 255];
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

/// Kill skull (your Overwatch clip, 2026-10-09; 60 fps frames from the kill): a red skull pops up over the crosshair
/// big (2.6 times its final size by frame 3, fading in over 2 frames), holds to frame 6, snaps down to its final size
/// (about 44 px at 1080p) by frame 8, then fades out by frame 32 (about half a second). Measured from the clip.
const KILL_SKULL_W: f32 = 44.0;
/// Red like the enemy health bars (you, 2026-10-09: "I want it red"; Overwatch's is pink, the enemy colour there).
const KILL_RED: [u8; 4] = [232, 48, 56, 255];
const KILL_SCALE: [(f32, f32); 7] = [(0.0, 1.9), (1.0, 2.2), (2.0, 2.45), (3.0, 2.6), (6.0, 2.6), (7.0, 1.3), (8.0, 1.0)];
const KILL_ALPHA: [(f32, f32); 9] =
    [(0.0, 0.35), (1.0, 0.75), (2.0, 1.0), (8.0, 1.0), (12.0, 0.72), (16.0, 0.47), (20.0, 0.28), (26.0, 0.12), (32.0, 0.0)];

/// The kill skull, drawn before the crosshair so the crosshair stays on top of it (as in Overwatch). A new kill
/// restarts it.
pub fn draw_kill_skull(ui: &Ui, r: &mut Resources, width: f32, height: f32) {
    let s = *STATE.lock().unwrap_or_else(|e| e.into_inner());
    let now = ui.time() as f32;
    if s.visible && r.anim.kills.is_some_and(|k| s.kills > k) {
        glog!("HUD", "kill skull (kill {})", s.kills);
        r.anim.kill_at = Some(now);
    }
    r.anim.kills = Some(s.kills);
    let Some(at) = r.anim.kill_at.filter(|_| s.visible) else {
        return;
    };
    let frame = (now - at) * 60.0;
    if frame > KILL_ALPHA[KILL_ALPHA.len() - 1].0 {
        r.anim.kill_at = None;
        return;
    }
    let k = height / REFERENCE_HEIGHT;
    let draw = ui.get_foreground_draw_list();
    let size = KILL_SKULL_W * k * key(frame, &KILL_SCALE);
    picture(&draw, r.kill_skull, [width / 2.0, height / 2.0], size, 0.0, alpha(KILL_RED, key(frame, &KILL_ALPHA)));
}

/// Draws the HUD (overlay render thread).
pub fn draw(ui: &Ui, r: &mut Resources, width: f32, height: f32) {
    let s = *STATE.lock().unwrap_or_else(|e| e.into_inner());
    if !s.visible {
        r.anim.last_hp = None;
        return;
    }
    let now = ui.time() as f32;
    // Healing and damage, from health going up or down (not when the maximum changes, or from 0 when you come back).
    if let Some(prev) = r.anim.last_hp {
        if s.max_hp == r.anim.last_max_hp && prev > 0 {
            if s.hp > prev {
                if r.anim.healed_at.is_none_or(|t| now - t > HEAL_HOLD) {
                    glog!("HUD", "healing indicator on ({prev} -> {} of {})", s.hp, s.max_hp);
                }
                r.anim.healed_at = Some(now);
            } else if s.hp < prev {
                r.anim.damaged_at = Some(now);
            }
        }
    }
    r.anim.last_hp = Some(s.hp);
    r.anim.last_max_hp = s.max_hp;
    let dt = (now - r.anim.last_now).clamp(0.0, 0.1);
    r.anim.last_now = now;
    let toward = |v: f32, on: bool, seconds: f32| if on { (v + dt / seconds).min(1.0) } else { (v - dt / seconds).max(0.0) };
    r.anim.heal_shown = toward(r.anim.heal_shown, r.anim.healed_at.is_some_and(|t| now - t < HEAL_HOLD), 0.15);
    r.anim.damage_shown = toward(r.anim.damage_shown, r.anim.damaged_at.is_some_and(|t| now - t < DAMAGE_HOLD), 0.2);
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

    // ---- Enemy health bars (DESIGN.md 5i, your Overwatch screenshots): a row of segments above each damaged enemy,
    // lit red for the health left (Overwatch's are pink), dark see-through red for what's lost, health just lost
    // lighter for a moment. Bosses (and field bosses) get a longer bar of 20 thin segments with a skull badge on
    // its left (your reference and pick, 2026-10-08): Overwatch's skull-in-a-diamond badge, red with a yellow skull.
    // Drawn first, under everything else.
    let segments = |cx: f32, bottom: f32, n: usize, seg_w: f32, seg_h: f32, gap: f32, hp: f32, trail: f32| {
        let total = n as f32 * seg_w + (n - 1) as f32 * gap;
        let (x0, y0) = (cx - total / 2.0, bottom - seg_h);
        draw.add_rect([x0 - 2.0 * k, y0 - 2.0 * k], [x0 + total + 2.0 * k, bottom + 2.0 * k], col([20, 8, 12, 90])).filled(true).rounding(2.5 * k).build();
        for i in 0..n {
            let sx = x0 + i as f32 * (seg_w + gap);
            let lit = (hp * n as f32 - i as f32).clamp(0.0, 1.0);
            let lost = (trail * n as f32 - i as f32).clamp(0.0, 1.0);
            draw.add_rect([sx, y0], [sx + seg_w, bottom], col([80, 20, 26, 160])).filled(true).rounding(1.5 * k).build();
            if lost > lit {
                draw.add_rect([sx + seg_w * lit, y0], [sx + seg_w * lost, bottom], col([255, 180, 180, 230])).filled(true).build();
            }
            if lit > 0.0 {
                let (top, low) = (col([255, 112, 112, 255]), col([232, 48, 56, 255]));
                draw.add_rect_filled_multicolor([sx, y0], [sx + seg_w * lit, bottom], top, top, low, low);
            }
        }
    };
    for bar in &s.enemies[..s.enemy_count.min(ENEMY_BARS)] {
        let (sx, sy) = ((bar.x + 1.0) / 2.0 * width, (1.0 - bar.y) / 2.0 * height);
        if !bar.boss {
            segments(sx, sy, 10, 8.6 * k, 10.0 * k, 2.0 * k, bar.hp, bar.trail);
            continue;
        }
        // Raised above the head (you, 2026-10-09), the name under it in the bar's red, as Overwatch writes it.
        let sy = sy - 22.0 * k;
        let (n, seg_w, seg_h, gap) = (20, 7.0 * k, 11.0 * k, 1.5 * k);
        let total = n as f32 * seg_w + (n - 1) as f32 * gap;
        let name = bar.name();
        if !name.is_empty() {
            let t = measure(r.bold_small, name);
            text([sx + 10.0 * k - t[0] / 2.0, sy + 4.0 * k], r.bold_small, [240, 72, 78, 255], name);
        }
        segments(sx + 10.0 * k, sy, n, seg_w, seg_h, gap, bar.hp, bar.trail);
        // The badge, centred on the bar's left end: Overwatch's diamond with the skull cut out, tinted red, over a
        // smaller light yellow diamond that shows through the skull.
        let (bx, by, half) = (sx + 10.0 * k - total / 2.0 - 10.0 * k, sy - seg_h / 2.0, 15.0 * k);
        let inner = half * 0.62;
        let yellow = col([255, 222, 120, 255]);
        draw.add_triangle([bx, by - inner], [bx + inner, by], [bx, by + inner], yellow).filled(true).build();
        draw.add_triangle([bx, by - inner], [bx, by + inner], [bx - inner, by], yellow).filled(true).build();
        picture(&draw, r.skull, [bx, by], half * 2.0, 0.0, [214, 36, 48, 255]);
    }

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
    let num_mid = seg_top - 22.0 * k;
    // The "+" pill (your Overwatch screenshots, 2026-10-08): a rounded pill, its left 56 % white with a dark "+", the
    // rest dark see-through grey. Damage: the white goes dark too, the "+" light grey, and a pink arrow pointing down
    // shows in the right part. Healing: the left part yellow with a dark "+", the right part dark yellow, a yellow
    // outline.
    let (heal, hurt) = (r.anim.heal_shown, r.anim.damage_shown * (1.0 - r.anim.heal_shown));
    let (pill_w, pill_h) = (40.0 * k, 24.0 * k);
    let (x0, y0, x1, y1) = (hx, num_mid - pill_h / 2.0, hx + pill_w, num_mid + pill_h / 2.0);
    let split = x0 + pill_w * 0.56;
    let dark = [40, 40, 58, 160];
    let right = mix(dark, [118, 98, 18, 215], heal);
    let left = mix(mix(PANEL_WHITE, dark, hurt), [245, 214, 32, 255], heal);
    let cross = mix(mix([60, 60, 78, 255], [205, 205, 212, 255], hurt), [92, 76, 10, 255], heal);
    let round = 4.0 * k;
    draw.add_rect([split, y0], [x1, y1], col(right)).filled(true).rounding(round).round_top_left(false).round_bot_left(false).build();
    draw.add_rect([x0, y0], [split, y1], col(left)).filled(true).rounding(round).round_top_right(false).round_bot_right(false).build();
    draw.add_rect([x0, y0], [x1, y1], col(mix([20, 20, 30, 110], [250, 226, 40, 255], heal))).thickness((1.0 + 0.6 * heal) * k).rounding(round).build();
    picture(&draw, r.plus, [(x0 + split) / 2.0, num_mid], pill_h * 0.68, 0.0, cross);
    if hurt > 0.0 {
        let (ax, ay) = ((split + x1) / 2.0, num_mid);
        draw.add_triangle([ax - 5.0 * k, ay - 3.0 * k], [ax + 5.0 * k, ay - 3.0 * k], [ax, ay + 4.0 * k], col(alpha([238, 64, 238, 255], hurt)))
            .filled(true)
            .build();
    }
    let hp = s.hp.max(0).to_string();
    let nx = x1 + 10.0 * k;
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

    // ---- Above the health: your status effects building up (DESIGN.md 5l; you, 2026-10-09: "a custom like
    // overwatch hp bar, on the left side for each status"). One row each, only while building up: its name in its
    // colour over 10 thin segments like the health bar's, filling in that colour; full, it pulses.
    let mut row_bottom = num_mid - 24.0 * k;
    for (i, bar) in s.statuses.iter().enumerate() {
        if bar.fill < 0.01 {
            continue;
        }
        let c = STATUS_COLOURS[i];
        let thin = 7.0 * k;
        let y0 = row_bottom - thin;
        let pulse = if bar.fill >= 0.999 { 0.75 + 0.25 * (now * 9.0).sin() } else { 1.0 };
        let fill = bar.fill * 10.0;
        for j in 0..10 {
            let x0 = hx + j as f32 * (seg_w + gap);
            draw.add_rect([x0, y0], [x0 + seg_w, row_bottom], col(alpha(c, 0.22))).filled(true).build();
            let part = (fill - j as f32).clamp(0.0, 1.0);
            if part > 0.0 {
                draw.add_rect([x0, y0], [x0 + seg_w * part, row_bottom], col(alpha(c, pulse))).filled(true).build();
            }
        }
        let name = crate::status::NAMES[i];
        let t = measure(r.label, name);
        text([hx, y0 - 3.0 * k - t[1]], r.label, c, name);
        row_bottom = y0 - t[1] - 9.0 * k;
    }

    // ---- Bottom centre: the ultimate. Your clip and its analyses (2026-10-08), at 1080 lines: centre 150 px above
    // the bottom; U = the white disc's radius, 40 px; Q cap centred 1.93 U below the centre.
    let (cx, cy) = (width / 2.0, height - 150.0 * k);
    let u = 40.0 * k;
    let since_press = r.anim.ult_pressed_at.map_or(f32::MAX, |t| now - t);
    let since_gain = r.anim.ult_ready_at.map_or(f32::MAX, |t| now - t);
    // The Q cap: `fill` 0 = the charging style (dark see-through square, thin white outline at half strength, white
    // letter), 1 = the ready style (solid white square, `letter` colour); `a` = overall strength.
    let qcap = |fill: f32, letter: [u8; 4], a: f32| {
        let s = 22.0 * k;
        let (x0, y0) = (cx - s / 2.0, cy + 1.93 * u - s / 2.0);
        let (x1, y1) = (x0 + s, y0 + s);
        draw.add_rect([x0, y0], [x1, y1], col(alpha([20, 24, 34, 255], 0.45 * a * (1.0 - fill)))).filled(true).rounding(3.0 * k).build();
        draw.add_rect([x0, y0], [x1, y1], col(alpha(WHITE, 0.5 * a * (1.0 - fill)))).thickness(1.4 * k).rounding(3.0 * k).build();
        draw.add_rect([x0, y0], [x1, y1], col(alpha(PANEL_WHITE, a * fill))).filled(true).rounding(3.0 * k).build();
        let t = measure(r.label, "Q");
        let _f = r.label.map(|f| ui.push_font(f));
        let c = mix(WHITE, letter, fill);
        draw.add_text([cx - t[0] / 2.0, y0 + (s - t[1]) / 2.0], col(alpha(c, a)), "Q");
    };
    // The charging meter (ULT GAINED spec, part 2 section 2), back to front: dotted outer ring (90 dots, 1.875 U), thin
    // grey ring (1.40 U), the charge as 67 teal bars (0.95-1.20 U) filling clockwise from 12 o'clock and breathing
    // slightly, a thin teal circle (0.80-0.85 U) with 4 small breaks, a dark see-through disc (0.78 U), the big
    // italic number.
    let meter = |a: f32, percent: u32| {
        for i in 0..90 {
            let ang = TAU * i as f32 / 90.0;
            draw.add_circle([cx + 1.875 * u * ang.cos(), cy + 1.875 * u * ang.sin()], 0.9 * k, col(alpha([59, 58, 68, 255], a))).filled(true).build();
        }
        draw.add_circle([cx, cy], 1.4 * u, col(alpha([66, 64, 72, 255], a))).thickness(2.0 * k).num_segments(96).build();
        draw.add_circle([cx, cy], 0.78 * u, col(alpha([29, 27, 35, 255], 0.85 * a))).filled(true).num_segments(96).build();
        let brk = 4f32.to_radians();
        for q in 0..4 {
            let start = (30.0 + 90.0 * q as f32).to_radians() + brk;
            arc_at(&draw, [cx, cy], 0.825 * u, start, FRAC_PI_2 - 2.0 * brk, col(alpha([50, 158, 148, 255], a)), 2.0 * k);
        }
        let lit = (percent as f32 / 100.0 * ULT_TICKS as f32).floor() as usize;
        let breathe = 1.0 + 0.035 * (now * TAU / 1.55).sin();
        let bar = TAU * 1.075 * u * 3.0 / 360.0;
        for i in 0..lit.min(ULT_TICKS) {
            let ang = -FRAC_PI_2 + TAU * (i as f32 + 0.5) / ULT_TICKS as f32;
            let (p0, p1) = ([cx + 0.95 * u * ang.cos(), cy + 0.95 * u * ang.sin()], [cx + 1.2 * u * ang.cos(), cy + 1.2 * u * ang.sin()]);
            draw.add_line(p0, p1, col(alpha([28, 217, 196, 255], 0.25 * a * breathe))).thickness(bar * 2.2).build();
            draw.add_line(p0, p1, col(alpha([28, 217, 196, 255], a * breathe))).thickness(bar).build();
            draw.add_line(p0, p1, col(alpha([89, 255, 255, 255], 0.8 * a * breathe))).thickness(bar * 0.4).build();
        }
        let t = percent.to_string();
        let size = measure(r.italic_ult, &t);
        let pos = [cx - size[0] / 2.0, cy - size[1] / 2.0];
        let _f = r.italic_ult.map(|f| ui.push_font(f));
        draw.add_text([pos[0] + 1.5 * k, pos[1] + 1.5 * k], col(alpha(OUTLINE, 0.6 * a)), &t);
        draw.add_text(pos, col(alpha([213, 213, 213, 255], a)), &t);
    };
    if ult_ready && since_gain < ULT_GAIN {
        // Becoming ready (ULT GAINED spec): collapse, pop, charge-up, flash and shockwave, flame eruption, afterglow,
        // then a snap to the ready state, 1.2 s in all.
        let t = since_gain * 60.0;
        ult_gain(&draw, r, [cx, cy], u, k, t, now);
        let ready = key(t, &[(3.0, 0.0), (17.0, 0.85), (19.0, 0.85), (71.0, 1.0)]);
        let letter = if t >= 19.0 { [44, 230, 251, 255] } else { [40, 40, 48, 255] };
        qcap(0.0, WHITE, if t < 3.0 { 0.15 } else { 0.15 * (1.0 - ready / 0.85) });
        qcap(1.0, letter, ready);
    } else if ult_ready {
        // Ready (your second analysis of the clip, ULT FULL spec, 2026-10-08). Back to front: a static glow halo with
        // a slight irregular flicker; a soft blue flame aura boiling in place; the static badge (faint hairline ring,
        // thin bright ring with gaps at 3 and 9 o'clock, blue glow, thick cyan ring, navy outline, white disc, glyph,
        // inward triangles); every ~1.37 s a burst of stepped, flipbook-style flares (BUILD, PEAK, blank, SECOND,
        // TENDRIL), with the disc's half on the peak side tinted during PEAK; the Q cap on top.
        let flicker = 1.0 + 0.04 * (0.6 * (now * 7.1).sin() + 0.4 * (now * 12.7 + 1.0).sin() * (now * 3.3).cos());
        idle_halo(&draw, r, [cx, cy], u, flicker);
        ult_aura(&draw, r.fade, [cx, cy], now, u, 1.0);
        let look = Look::ready();
        look.rings(&draw, r, [cx, cy], u, k);
        let burst = Burst::at(now);
        // PEAK flash: the disc's half on the big flare's side goes pale cyan and the glyph teal there.
        if let Some(b) = burst.filter(|b| b.step() == Step::Peak) {
            let a = b.spike_dir();
            let (ca, sa) = (a.cos(), a.sin());
            let (min, max) = if ca.abs() >= sa.abs() {
                if ca > 0.0 { ([cx, cy - u], [cx + u, cy + u]) } else { ([cx - u, cy - u], [cx, cy + u]) }
            } else if sa > 0.0 {
                ([cx - u, cy], [cx + u, cy + u])
            } else {
                ([cx - u, cy - u], [cx + u, cy])
            };
            draw.with_clip_rect_intersect(min, max, || {
                draw.add_circle([cx, cy], 1.0 * u, col([232, 241, 249, 255])).filled(true).num_segments(96).build();
                picture(&draw, r.blade, [cx, cy], 1.3 * u, 0.0, [37, 90, 114, 255]);
            });
        }
        if let Some(b) = burst {
            b.draw(&draw, r, [cx, cy], u, k);
        }
        look.triangles(&draw, [cx, cy], u, k);
        qcap(1.0, [40, 40, 48, 255], 1.0);
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
        let fill = 1.0 - ((t - 0.15) / 0.2).clamp(0.0, 1.0);
        qcap(fill, [40, 40, 48, 255], 1.0);
    } else {
        meter(1.0, s.ult_percent);
        qcap(0.0, WHITE, 1.0);
    }

    // ---- Dragonblade time left, around the crosshair (your Overwatch screenshots, 2026-10-08): a white ring of
    // about 30 px for the time left, running clockwise from 12 o'clock, leaving a grey trail for the time used, and
    // the whole seconds left ("6s") below it.
    if let Some(left) = s.blade_left {
        let share = if s.blade_duration > 0.0 { (left / s.blade_duration).clamp(0.0, 1.0) } else { 0.0 };
        let c = [width / 2.0, height / 2.0];
        let (radius, th) = (29.5 * k, 3.5 * k);
        let start = -FRAC_PI_2;
        arc_at(&draw, c, radius, start + share * TAU, (1.0 - share) * TAU, col([150, 150, 155, 130]), th);
        if share > 0.0 {
            arc_at(&draw, c, radius, start, share * TAU, col([245, 245, 248, 235]), th);
        }
        let t = format!("{}s", left.ceil().max(0.0) as u32);
        let size = measure(r.bold_small, &t);
        text([c[0] - size[0] / 2.0, c[1] + 42.0 * k - size[1] / 2.0], r.bold_small, WHITE, &t);
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
    // Ammo panel (your screenshot of 2026-10-08, its 4 degree tilt taken out): 16 px right of the E tile; its top line
    // 8 px below the tile tops, its bottom line on the tiles' underlines; two straight cyan lines with short angled
    // ends, each with the same brighter piece; see-through inside.
    let ax = x + 2.0 * k;
    let aw = width - ax - 50.0 * k;
    let (pt, pb) = (box_top + 8.0 * k, box_top + size + 5.25 * k);
    draw.add_rect([ax, pt], [ax + aw, pb], col([255, 255, 255, 14])).filled(true).build();
    for y in [pt, pb] {
        let inward = if y == pt { 4.0 * k } else { -4.0 * k };
        for (th, a) in [(9.0, 0.15), (5.0, 0.35), (3.0, 1.0)] {
            draw.add_line([ax + 4.0 * k, y], [ax + aw - 4.0 * k, y], col(alpha(CYAN, a))).thickness(th * k).build();
        }
        // The brighter, thicker piece: the same stretch on both lines (52% to 92% of the width, measured on your
        // Overwatch screenshot), bulging outward.
        let (sx0, sx1) = (ax + aw * 0.52, ax + aw * 0.92);
        let out = -inward * 0.25;
        for (th, a) in [(11.0, 0.25), (5.5, 1.0)] {
            draw.add_line([sx0, y + out], [sx1, y + out], col(alpha([130, 248, 255, 255], a))).thickness(th * k).build();
        }
        draw.add_line([ax, y + inward], [ax + 4.0 * k, y], col(CYAN)).thickness(2.5 * k).build();
        draw.add_line([ax + aw - 4.0 * k, y], [ax + aw, y + inward], col(CYAN)).thickness(2.5 * k).build();
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

/// A soft strip along `pts`, `width(i)` wide and `strength(i)` strong at each point (the texture is bright in the
/// middle and clear at both edges). Neighbouring quads share their corners, so it is one smooth ribbon.
fn strip(draw: &DrawListMut, tex: Picture, pts: &[[f32; 2]], width: &dyn Fn(usize) -> f32, strength: &dyn Fn(usize) -> f32, color: [u8; 4]) {
    let Some((id, _)) = tex else {
        return;
    };
    let n = pts.len();
    if n < 2 {
        return;
    }
    let sides = |i: usize| -> ([f32; 2], [f32; 2]) {
        let (a, b) = (pts[i.saturating_sub(1)], pts[(i + 1).min(n - 1)]);
        let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
        let len = (dx * dx + dy * dy).sqrt().max(1e-3);
        let (nx, ny) = (-dy / len * width(i) / 2.0, dx / len * width(i) / 2.0);
        ([pts[i][0] + nx, pts[i][1] + ny], [pts[i][0] - nx, pts[i][1] - ny])
    };
    let mut prev = sides(0);
    for i in 1..n {
        let cur = sides(i);
        let a = 0.5 * (strength(i - 1) + strength(i));
        if a > 0.003 {
            draw.add_image_quad(id, prev.0, prev.1, cur.1, cur.0).col(col(alpha(color, a))).build();
        }
        prev = cur;
    }
}

/// Soft blue flame aura of the ready ultimate (ULT FULL spec 3.3): wispy tongues licking outward from the thick
/// ring, densest against it and gone by about 2.2 U, churning in place (standing waves that rise and fall, plus a
/// drift of 2 degrees a second clockwise; it does not spin). Quads sharing their corners, fading outward.
fn ult_aura(draw: &DrawListMut, fade: Picture, c: [f32; 2], now: f32, u: f32, scale: f32) {
    let Some((id, _)) = fade else {
        return;
    };
    let drift = now * 2f32.to_radians();
    for (reach, amp, color, strength, seed) in [(0.75, 0.4, [60, 170, 250, 255], 0.9, 0.0), (0.45, 0.25, [45, 127, 225, 255], 0.7, 3.0)] {
        let outer = |th: f32| -> f32 {
            let a = th - drift;
            let n = 0.45 * (5.0 * a + seed).sin() * (1.7 * now + seed).sin()
                + 0.3 * (9.0 * a + 1.0 + seed).sin() * (2.9 * now + 2.0).sin()
                + 0.2 * (14.0 * a + 2.0).sin() * (4.3 * now + 1.0 + seed).sin()
                + 0.25 * (3.0 * a + 4.0 + seed).sin() * (1.1 * now + 3.0).sin();
            (1.25 + reach + amp * n.max(-0.5)) * u
        };
        let at = |th: f32, rr: f32| [c[0] + rr * th.cos(), c[1] + rr * th.sin()];
        for i in 0..AURA_PIECES {
            let (t0, t1) = (TAU * i as f32 / AURA_PIECES as f32, TAU * (i + 1) as f32 / AURA_PIECES as f32);
            draw.add_image_quad(id, at(t0, 1.25 * u), at(t1, 1.25 * u), at(t1, outer(t1)), at(t0, outer(t0)))
                .col(col(alpha(color, strength * scale)))
                .build();
        }
    }
}

/// A value at `t` along keyframes (t, value), straight lines between them, held before the first and after the last.
fn key(t: f32, keys: &[(f32, f32)]) -> f32 {
    let (first, last) = (keys[0], keys[keys.len() - 1]);
    if t <= first.0 {
        return first.1;
    }
    if t >= last.0 {
        return last.1;
    }
    let i = keys.windows(2).position(|w| t < w[1].0).unwrap_or(0);
    let ((t0, v0), (t1, v1)) = (keys[i], keys[i + 1]);
    v0 + (v1 - v0) * (t - t0) / (t1 - t0)
}

/// A colour at `t` along keyframes (t, 0xRRGGBB), blended between them.
fn key_col(t: f32, keys: &[(f32, u32)]) -> [u8; 4] {
    let ch = |shift: u32| -> f32 {
        let v: Vec<(f32, f32)> = keys.iter().map(|(t, c)| (*t, ((c >> shift) & 0xFF) as f32)).collect();
        key(t, &v)
    };
    [ch(16) as u8, ch(8) as u8, ch(0) as u8, 255]
}

/// The ultimate's badge, as the ready state shows it or as the "becoming ready" animation changes it.
struct Look {
    disc_r: f32,
    disc: [u8; 4],
    glyph: [u8; 4],
    /// Thick ring: its base and its brighter core.
    thick: [u8; 4],
    thick_core: [u8; 4],
    /// Blue glow between the thick ring and the thin one.
    glow: [u8; 4],
    navy: f32,
    thin_r: f32,
    thin: [u8; 4],
    /// The thin ring's gaps at 3 and 9 o'clock.
    gaps: bool,
    hairline: f32,
    /// Where the triangles' tips are (their base is 0.12 U further out).
    tri_tip: f32,
}

impl Look {
    /// The ready state (ULT FULL spec, section 1-2), in U.
    fn ready() -> Look {
        Look {
            disc_r: 1.0,
            disc: [233, 234, 244, 255],
            glyph: [29, 29, 48, 255],
            thick: [52, 214, 247, 255],
            thick_core: [50, 255, 252, 255],
            glow: [64, 182, 245, 255],
            navy: 1.0,
            thin_r: 1.4,
            thin: [168, 234, 255, 255],
            gaps: true,
            hairline: 70.0 / 255.0,
            tri_tip: 1.4,
        }
    }

    /// Hairline, thin ring, glow, thick ring, navy outline, disc and glyph (the triangles go on top separately).
    fn rings(&self, draw: &DrawListMut, r: &Resources, c: [f32; 2], u: f32, k: f32) {
        draw.add_circle(c, 1.91 * u, col(alpha([168, 234, 255, 255], self.hairline))).thickness(1.0 * k).num_segments(128).build();
        if self.gaps {
            let gap = 11f32.to_radians();
            for start in [gap, PI + gap] {
                arc_at(draw, c, self.thin_r * u, start, PI - 2.0 * gap, col(self.thin), 2.0 * k);
            }
        } else {
            draw.add_circle(c, self.thin_r * u, col(self.thin)).thickness(2.0 * k).num_segments(128).build();
        }
        draw.add_circle(c, 1.31 * u, col(self.glow)).thickness(0.13 * u).num_segments(96).build();
        draw.add_circle(c, 1.1625 * u, col(self.thick)).thickness(0.175 * u).num_segments(96).build();
        draw.add_circle(c, 1.1625 * u, col(self.thick_core)).thickness(0.09 * u).num_segments(96).build();
        draw.add_circle(c, (self.disc_r + 0.025) * u, col(alpha([48, 85, 150, 255], self.navy))).thickness(0.05 * u).num_segments(96).build();
        draw.add_circle(c, self.disc_r * u, col(self.disc)).filled(true).num_segments(96).build();
        picture(draw, r.blade, c, 1.3 * u * self.disc_r, 0.0, self.glyph);
    }

    fn triangles(&self, draw: &DrawListMut, c: [f32; 2], u: f32, k: f32) {
        for side in [-1.0f32, 1.0] {
            let (tip, base) = (c[0] + side * self.tri_tip * u, c[0] + side * (self.tri_tip + 0.12) * u);
            draw.add_triangle([base, c[1] - 3.0 * k], [base, c[1] + 3.0 * k], [tip, c[1]], col(WHITE)).filled(true).build();
        }
    }
}

/// The ready state's glow halo (ULT FULL spec 2, 3.2): just outside, middle and far colours, `flicker` about 1.
fn idle_halo(draw: &DrawListMut, r: &Resources, c: [f32; 2], u: f32, flicker: f32) {
    picture(draw, r.glow_dot, c, 5.0 * u, 0.0, alpha([45, 104, 186, 255], 0.85 * flicker));
    picture(draw, r.glow_dot, c, 3.8 * u, 0.0, alpha([45, 127, 225, 255], 0.85 * flicker));
    picture(draw, r.glow_dot, c, 2.9 * u, 0.0, alpha([55, 160, 247, 255], 0.9 * flicker));
}

/// Shockwave band, inner and outer radius in U, by 60 fps frame from 20 to 33 after reaching 100 % (spec D2).
const SHOCKWAVE: [(f32, f32, f32); 14] = [
    (20.0, 2.55, 3.73), (21.0, 2.88, 4.23), (22.0, 3.40, 4.65), (23.0, 3.85, 5.25), (24.0, 4.48, 5.55),
    (25.0, 5.00, 6.18), (26.0, 5.60, 6.40), (27.0, 5.98, 6.83), (28.0, 6.70, 7.28), (29.0, 7.23, 7.83),
    (30.0, 7.55, 7.98), (31.0, 7.75, 8.18), (32.0, 8.10, 8.33), (33.0, 8.28, 8.48),
];

/// The ultimate becoming ready (ULT GAINED spec), `t` in 60 fps frames since it reached 100 % (0-72), everything but
/// the Q cap. Phases: A collapse (0-8), B pop (9-12), C charge-up (12-18), D flash and shockwave (19-33), E flame
/// eruption (28-50, a new drawing every 2 frames), F afterglow (51-71), then the ready state takes over.
fn ult_gain(draw: &DrawListMut, r: &Resources, c: [f32; 2], u: f32, k: f32, t: f32, now: f32) {
    let f = t.floor();
    // A4: a dark backing inside the thin ring, so the shrinking disc floats in a "black hole" (until the flash).
    if f < 19.0 {
        let thin_r = key(t, &[(0.0, 1.43), (2.0, 1.53), (5.0, 1.65), (8.0, 1.55), (12.0, 1.50), (15.0, 1.45), (17.0, 1.43)]);
        draw.add_circle(c, thin_r * u, col(alpha([32, 35, 47, 255], 0.7))).filled(true).num_segments(96).build();
        // A5: a faint outer halo growing.
        picture(draw, r.glow_dot, c, 4.0 * u, 0.0, alpha(key_col(t, &[(0.0, 0x2A3248), (8.0, 0x20304A)]), 0.6 * (t / 8.0).min(1.0)));
    }
    // D4 then F: the cyan bloom grows to 3.3 U while the flames erupt, then shrinks, cools and fades.
    if f >= 19.0 {
        let radius = key(t, &[(19.0, 1.8), (31.0, 3.3), (50.0, 3.0), (71.0, 2.2)]);
        let color = key_col(t, &[(19.0, 0x2CFFFC), (50.0, 0x2CFFFC), (55.0, 0x27D6FA), (65.0, 0x24A0F7), (71.0, 0x2070C5)]);
        picture(draw, r.glow_dot, c, 2.6 * radius * u, 0.0, alpha(color, key(t, &[(19.0, 0.9), (50.0, 0.85), (71.0, 0.6)])));
        // The ready state's aura and halo come up during the afterglow.
        if f >= 51.0 {
            let up = ((t - 51.0) / 20.0).clamp(0.0, 1.0);
            idle_halo(draw, r, c, u, up);
            ult_aura(draw, r.fade, c, now, u, up);
        }
    }
    // A5-C: the thick ring forms from swirling blue smoke and charges from dark blue to cyan.
    let thick = key_col(t, &[
        (1.0, 0x242D3F), (3.0, 0x24425E), (5.0, 0x245A81), (7.0, 0x2570A3), (8.0, 0x2579B3), (9.0, 0x2682C0), (10.0, 0x2792D6),
        (11.0, 0x279EE6), (12.0, 0x28A9EF), (13.0, 0x29B8F5), (15.0, 0x2BCFF8), (17.0, 0x2CE1FA), (18.0, 0x2CE6FB),
    ]);
    // C: a blue glow grows around the charging ring.
    if (8.0..19.0).contains(&f) {
        picture(draw, r.glow_dot, c, 4.2 * u, 0.0, alpha(thick, 0.45 * ((t - 8.0) / 10.0).min(1.0)));
    }
    if (1.0..19.0).contains(&f) {
        if let Some((id, _)) = r.fade {
            let swirl = now * 1.5;
            let outer = |th: f32| {
                let a = th + swirl;
                (1.2 + 0.18 + 0.12 * ((5.0 * a).sin() * 0.6 + (11.0 * a + 1.0).sin() * 0.4)) * u
            };
            let at = |th: f32, rr: f32| [c[0] + rr * th.cos(), c[1] + rr * th.sin()];
            for i in 0..AURA_PIECES {
                let (t0, t1) = (TAU * i as f32 / AURA_PIECES as f32, TAU * (i + 1) as f32 / AURA_PIECES as f32);
                draw.add_image_quad(id, at(t0, 1.05 * u), at(t1, 1.05 * u), at(t1, outer(t1)), at(t0, outer(t0)))
                    .col(col(alpha(thick, 0.7 * (1.0 - (t - 1.0) / 18.0))))
                    .build();
            }
        }
    }
    // D2-D3: the shockwave: a thick soft cyan band expanding to 8.5 U and thinning, broken into arcs from frame 28;
    // a thin teal ring trailing inside it from frame 23 to 26.
    if (20.0..34.0).contains(&t) {
        let i = SHOCKWAVE.iter().position(|s| s.0 > t).unwrap_or(SHOCKWAVE.len()).max(1) - 1;
        let (a, b) = (SHOCKWAVE[i], SHOCKWAVE[(i + 1).min(SHOCKWAVE.len() - 1)]);
        let s = if b.0 > a.0 { ((t - a.0) / (b.0 - a.0)).clamp(0.0, 1.0) } else { 0.0 };
        let (inner, outer) = ((a.1 + (b.1 - a.1) * s) * u, (a.2 + (b.2 - a.2) * s) * u);
        let strength = if t < 31.0 { 0.95 } else { 0.95 - 0.35 * (t - 31.0) / 3.0 };
        if let Some((id, _)) = r.ribbon {
            const PIECES: usize = 180;
            for p in 0..PIECES {
                let (t0, t1) = (TAU * p as f32 / PIECES as f32, TAU * (p + 1) as f32 / PIECES as f32);
                // From frame 28, 5 small gaps (about 4 degrees each) break the band into arcs.
                if t >= 28.0 && (0..5).any(|g| {
                    let mid = TAU * (g as f32 + 0.3 * hash(g + 7)) / 5.0;
                    let d = (0.5 * (t0 + t1) - mid).rem_euclid(TAU);
                    d.min(TAU - d) < 2f32.to_radians()
                }) {
                    continue;
                }
                let at = |th: f32, rr: f32| [c[0] + rr * th.cos(), c[1] + rr * th.sin()];
                draw.add_image_quad(id, at(t0, inner), at(t0, outer), at(t1, outer), at(t1, inner))
                    .col(col(alpha([48, 228, 250, 255], strength)))
                    .build();
            }
        }
        if (23.0..27.0).contains(&t) {
            let rr = key(t, &[(23.0, 3.6), (26.0, 4.8)]) * u;
            draw.add_circle(c, rr, col(alpha([47, 185, 198, 255], 1.0 - (t - 23.0) / 4.0))).thickness(1.5 * k).num_segments(160).build();
        }
    }
    // E: the flame eruption, a new drawing every 2 frames (30 a second), held in between.
    if (28.0..51.0).contains(&t) {
        eruption(draw, r, c, u, k, ((t - 28.0) / 2.0).floor() as u32);
    }
    // The badge, as each phase has it.
    let mut look = Look::ready();
    if f < 19.0 {
        look.disc_r = key(t, &[(0.0, 0.85), (1.0, 0.80), (2.0, 0.78), (3.0, 0.73), (4.0, 0.68), (5.0, 0.63), (6.0, 0.60), (7.0, 0.55), (8.0, 0.50), (9.0, 0.58), (10.0, 0.80), (11.0, 0.93), (12.0, 1.0)]);
        look.disc = if f < 12.0 { [232, 232, 232, 230] } else { [231, 231, 231, 255] };
        look.glyph = if f < 1.0 {
            [0, 0, 0, 0]
        } else if f < 9.0 {
            alpha(key_col(t, &[(1.0, 0xE0E0E0), (8.0, 0xB0B0B0)]), 0.4 * (t - 1.0) / 7.0)
        } else {
            key_col(t, &[(11.0, 0xA7A7A7), (18.0, 0x7D7D7D)])
        };
        look.thick = if f < 1.0 { [0, 0, 0, 0] } else { thick };
        look.thick_core = look.thick;
        look.glow = if f < 12.0 { [0, 0, 0, 0] } else { key_col(t, &[(12.0, 0x20426E), (18.0, 0x1F5794)]) };
        look.navy = if f < 12.0 { 0.0 } else { 1.0 };
        look.thin_r = key(t, &[(0.0, 1.43), (2.0, 1.53), (5.0, 1.65), (8.0, 1.55), (12.0, 1.50), (15.0, 1.45), (17.0, 1.43)]);
        look.thin = key_col(t, &[(0.0, 0xC8C8D0), (8.0, 0xDADADF), (12.0, 0xDADADF), (18.0, 0xA8EAFF)]);
        look.gaps = f >= 15.0;
        look.hairline = key(t, &[(12.0, 0.0), (18.0, 70.0 / 255.0)]);
        look.tri_tip = key(t, &[(0.0, 1.63), (11.0, 1.73), (12.0, 1.70), (13.0, 1.63), (14.0, 1.53), (15.0, 1.45), (16.0, 1.40)]);
    } else {
        look.thin_r = key(t, &[(19.0, 1.43), (25.0, 1.40)]);
        look.disc = if f == 19.0 { [235, 254, 254, 255] } else { [232, 254, 253, 255] };
        look.glyph = key_col(t, &[
            (19.0, 0x88FEFB), (25.0, 0x63FEF9), (30.0, 0x36FDF7), (35.0, 0x36FDF7), (40.0, 0x2BCBF2), (45.0, 0x2079D5),
            (55.0, 0x1F6FC0), (60.0, 0x18436F), (65.0, 0x183D6B), (68.0, 0x152D4C), (71.0, 0x15243A),
        ]);
    }
    look.rings(draw, r, c, u, k);
    // F: the first ready-state flares start near the end.
    if f >= 61.0 {
        if let Some(b) = Burst::at(now) {
            b.draw(draw, r, c, u, k);
        }
    }
    look.triangles(draw, c, u, k);
    // D1: the one-frame flash: a filled cyan disc out to 3.2 U with a white-hot centre, over everything.
    if f == 19.0 {
        draw.add_circle(c, 3.2 * u, col(alpha([48, 228, 250, 255], 0.92))).filled(true).num_segments(128).build();
        picture(draw, r.glow_dot, c, 3.0 * u, 0.0, WHITE);
        picture(draw, r.glow_dot, c, 1.6 * u, 0.0, WHITE);
    }
}

/// One drawing of the flame eruption (spec E), `index` 0-10 (each held 2 frames): a solid ragged mass with a spike
/// and a streak (0-1), the mass swelling and thinning with hooked curls separating (2-3), only the curls' outlines
/// (4-5), the curls flying out to 6 U, thinning and breaking into dashes (6-8), fragments (9-10).
fn eruption(draw: &DrawListMut, r: &Resources, c: [f32; 2], u: f32, k: f32, index: u32) {
    // The curls: mostly upward and to the sides (angles in degrees, 0 = 3 o'clock, clockwise).
    const CURLS: [(f32, f32); 6] = [(-170.0, 1.0), (-125.0, -1.0), (-85.0, 1.0), (-45.0, -1.0), (-5.0, 1.0), (150.0, -1.0)];
    let curls = |reach: f32, fill: f32, line: f32, broken: f32| {
        for (n, (deg, side)) in CURLS.iter().enumerate() {
            let vary = 1.0 + 0.12 * (hash(n as u32 + 40) - 0.5);
            flare(draw, r, c, u, k, deg.to_radians(), *side, Shape::Loop, reach * vary, 1.0, fill, line, broken);
        }
    };
    let mass = |reach: f32, strength: f32| {
        let Some((id, _)) = r.white else {
            return;
        };
        let seed = index as f32 * 1.7;
        let edge = |th: f32| {
            // Lopsided: bigger upward (screen y grows downward), torn edges, a few tongues.
            let torn = 0.22 * (5.0 * th + seed).sin() + 0.15 * (11.0 * th + 2.0 * seed).sin() + 0.08 * (23.0 * th + 3.0 * seed).sin();
            let up = 0.7 * (-th.sin()).max(0.0).powf(1.5);
            let tongues = 0.35 * (3.0 * th + seed).sin().max(0.0).powi(10);
            ((reach - 0.3 + torn + up + tongues).max(1.3)) * u
        };
        let at = |th: f32, rr: f32| [c[0] + rr * th.cos(), c[1] + rr * th.sin()];
        for i in 0..AURA_PIECES {
            let (t0, t1) = (TAU * i as f32 / AURA_PIECES as f32, TAU * (i + 1) as f32 / AURA_PIECES as f32);
            draw.add_image_quad(id, at(t0, 1.0 * u), at(t1, 1.0 * u), at(t1, edge(t1)), at(t0, edge(t0)))
                .col(col(alpha([60, 232, 255, 255], strength)))
                .build();
        }
        picture(draw, r.glow_dot, c, 6.4 * u, 0.0, alpha([60, 232, 255, 255], 0.5 * strength));
        picture(draw, r.glow_dot, c, 2.6 * u, 0.0, alpha(WHITE, strength));
    };
    match index {
        0 | 1 => {
            mass(2.5 + 0.25 * index as f32, 0.85);
            flare(draw, r, c, u, k, (-50f32).to_radians(), 1.0, Shape::Spike, 1.05, 1.0, 0.6, 1.4, 0.0);
            flare(draw, r, c, u, k, PI, -1.0, Shape::Spike, 0.8, 0.9, 0.5, 0.9, 0.0);
        }
        2 | 3 => {
            mass(3.1 + 0.2 * (index - 2) as f32, 0.45);
            curls(1.3 + 0.2 * (index - 2) as f32, 0.25, 1.5, 0.0);
        }
        4 | 5 => curls(1.7 + 0.3 * (index - 4) as f32, 0.0, 1.6, 0.0),
        6..=8 => {
            let s = (index - 6) as f32;
            curls(2.4 + 0.3 * s, 0.0, 1.2 - 0.3 * s, 0.15 + 0.17 * s);
        }
        _ => curls(3.2 + 0.2 * (index - 9) as f32, 0.0, 0.5, 0.8 + 0.12 * (index - 9) as f32),
    }
}

/// Average time from one flare burst to the next (spec 4.1: 1.25-1.48 s, 1.37 s on average).
const BURST_EVERY: f32 = 1.37;

#[derive(Clone, Copy, PartialEq)]
enum Step {
    Build,
    Peak,
    Blank,
    Second,
    Tendril,
}

#[derive(Clone, Copy)]
enum Shape {
    /// A tongue of fire hugging the ring, then curling into a hooked loop (a "6"), hollow in the middle.
    Loop,
    /// The same tongue stretched into a long pointed blade with two small barbs, like a dragon fin.
    Spike,
}

/// One flare burst of the ready ultimate (spec 4): a pair of flares on opposite sides (a loop and a spike), drawn
/// as held poses that cut from one to the next with no blending, at about 12-15 drawings a second.
#[derive(Clone, Copy)]
struct Burst {
    /// 60 fps frames since the burst started (0-24).
    frame: u32,
    seed: u32,
    /// Direction of the loop (the spike is opposite), radians, 0 = 3 o'clock, clockwise.
    dir: f32,
    /// Mirror of the drawings (+1 or -1).
    side: f32,
}

impl Burst {
    /// The burst on screen at `now`, if any: one per 1.37 s slot, starting 0-0.12 s into it.
    fn at(now: f32) -> Option<Burst> {
        let slot = (now / BURST_EVERY).floor();
        let seed = (slot as i64).rem_euclid(1 << 30) as u32;
        let start = slot * BURST_EVERY + 0.12 * hash(seed * 31 + 1);
        let frame = ((now - start) * 60.0).floor();
        if !(0.0..25.0).contains(&frame) {
            return None;
        }
        // A new direction each burst, at least 60 degrees from the last one.
        let angle = |s: u32| TAU * hash(s * 31 + 2);
        let mut dir = angle(seed);
        let diff = (dir - angle(seed.wrapping_sub(1))).rem_euclid(TAU);
        if diff.min(TAU - diff) < 60f32.to_radians() {
            dir += 120f32.to_radians();
        }
        let side = if hash(seed * 31 + 4) < 0.5 { 1.0 } else { -1.0 };
        Some(Burst { frame: frame as u32, seed, dir, side })
    }

    /// Spec 4.2: BUILD 4 frames, PEAK 5, blank 5, SECOND 5, TENDRIL 6.
    fn step(&self) -> Step {
        match self.frame {
            0..=3 => Step::Build,
            4..=8 => Step::Peak,
            9..=13 => Step::Blank,
            14..=18 => Step::Second,
            _ => Step::Tendril,
        }
    }

    fn spike_dir(&self) -> f32 {
        self.dir + PI
    }

    fn draw(&self, draw: &DrawListMut, r: &Resources, c: [f32; 2], u: f32, k: f32) {
        let h = |n: u32| hash(self.seed * 31 + n);
        let (loop_len, spike_len) = (0.9 + 0.15 * h(6), 0.92 + 0.1 * h(5));
        let second_dir = TAU * h(7);
        match self.step() {
            // Small and thin: hollow outlines, medium brightness.
            Step::Build => {
                flare(draw, r, c, u, k, self.dir, self.side, Shape::Loop, 0.62 * loop_len, 0.65, 0.12, 0.7, 0.0);
                flare(draw, r, c, u, k, self.spike_dir(), -self.side, Shape::Spike, 0.62 * spike_len, 0.65, 0.12, 0.7, 0.0);
            }
            // The same pair redrawn about 40 % bigger, thicker and brighter, with the translucent fill.
            Step::Peak => {
                flare(draw, r, c, u, k, self.dir, self.side, Shape::Loop, loop_len, 1.0, 0.45, 1.3, 0.0);
                flare(draw, r, c, u, k, self.spike_dir(), -self.side, Shape::Spike, spike_len, 1.0, 0.45, 1.3, 0.0);
            }
            Step::Blank => {}
            // One smaller, dimmer flare at a new angle; sometimes a small one opposite.
            Step::Second => {
                flare(draw, r, c, u, k, second_dir, self.side, Shape::Loop, 0.5, 0.6, 0.15, 0.8, 0.0);
                if h(8) > 0.6 {
                    flare(draw, r, c, u, k, second_dir + PI, -self.side, Shape::Loop, 0.35, 0.5, 0.1, 0.7, 0.0);
                }
            }
            // A thin jagged electric thread, fading out and breaking into pieces (the only fade).
            Step::Tendril => {
                let t = (self.frame - 19) as f32 / 6.0;
                let dir = if h(9) < 0.4 { second_dir } else { TAU * h(10) };
                tendril(draw, r.ribbon, c, u, k, dir, 3.0 + 0.8 * h(11), self.seed, 1.0 - t, t);
            }
        }
    }
}

/// A point on a Catmull-Rom curve through `ctrl`, at `f` (0-1 along it).
fn catmull(ctrl: &[(f32, f32)], f: f32) -> (f32, f32) {
    let n = ctrl.len();
    let s = f.clamp(0.0, 1.0) * (n - 1) as f32;
    let i = (s.floor() as usize).min(n - 2);
    let t = s - i as f32;
    let (p0, p1, p2, p3) = (ctrl[i.saturating_sub(1)], ctrl[i], ctrl[i + 1], ctrl[(i + 2).min(n - 1)]);
    let cr = |a: f32, b: f32, c: f32, d: f32| {
        0.5 * (2.0 * b + (c - a) * t + (2.0 * a - 5.0 * b + 4.0 * c - d) * t * t + (3.0 * b - a - 3.0 * c + d) * t * t * t)
    };
    (cr(p0.0, p1.0, p2.0, p3.0), cr(p0.1, p1.1, p2.1, p3.1))
}

/// One flare drawing (spec 4.4), rooted on the outer edge of the thick ring at angle `dir`, mirrored by `side`.
/// `reach`: 1 = PEAK size (the part beyond the ring scales); `bright`: outline strength; `fill`: strength of the
/// translucent body; `line`: outline thickness factor; `broken`: share of the outline missing (dashes). Body and
/// outline are strips of quads sharing their corners.
#[allow(clippy::too_many_arguments)]
fn flare(draw: &DrawListMut, r: &Resources, c: [f32; 2], u: f32, k: f32, dir: f32, side: f32, shape: Shape, reach: f32, bright: f32, fill: f32, line: f32, broken: f32) {
    // Centreline (radius in U, angle from `dir` in radians) and half-widths in U on each side.
    const LOOP: [(f32, f32); 10] = [
        (1.25, -0.6), (1.35, -0.35), (1.6, -0.12), (1.95, 0.05), (2.3, 0.25),
        (2.45, 0.5), (2.3, 0.72), (2.0, 0.78), (1.75, 0.65), (1.68, 0.45),
    ];
    const SPIKE: [(f32, f32); 6] = [(1.25, -0.4), (1.33, -0.15), (1.6, 0.0), (2.2, 0.04), (2.9, 0.05), (3.7, 0.04)];
    const N: usize = 40;
    let ctrl: &[(f32, f32)] = match shape {
        Shape::Loop => &LOOP,
        Shape::Spike => &SPIKE,
    };
    let thick = 0.6 + 0.4 * reach.min(1.0);
    let widths = |f: f32| -> (f32, f32) {
        match shape {
            Shape::Loop => {
                let w = 0.3 * (PI * f.powf(0.8)).sin().max(0.0).powf(0.6);
                (w * thick, w * thick)
            }
            Shape::Spike => {
                let w = 0.28 * (1.0 - f).max(0.0).powf(0.9) * (f * 5.0).min(1.0);
                // Barbs on one edge: each rises along the blade and drops sharply.
                let barb = |at: f32, size: f32| if f > at - 0.07 && f <= at { size * (f - (at - 0.07)) / 0.07 } else { 0.0 };
                (w * thick, (w + barb(0.55, 0.12) + barb(0.72, 0.09)) * thick)
            }
        }
    };
    let center: Vec<[f32; 2]> = (0..N)
        .map(|j| {
            let (rr, a) = catmull(ctrl, j as f32 / (N - 1) as f32);
            let rr = 1.25 + (rr - 1.25) * reach;
            let ang = dir + side * a;
            [c[0] + rr * u * ang.cos(), c[1] + rr * u * ang.sin()]
        })
        .collect();
    let mut left = Vec::with_capacity(N);
    let mut right = Vec::with_capacity(N);
    for j in 0..N {
        let (a, b) = (center[j.saturating_sub(1)], center[(j + 1).min(N - 1)]);
        let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
        let len = (dx * dx + dy * dy).sqrt().max(1e-3);
        let (nx, ny) = (-dy / len, dx / len);
        let (wl, wr) = widths(j as f32 / (N - 1) as f32);
        left.push([center[j][0] + nx * wl * u, center[j][1] + ny * wl * u]);
        right.push([center[j][0] - nx * wr * u, center[j][1] - ny * wr * u]);
    }
    if let Some((id, _)) = r.white {
        for j in 0..N - 1 {
            draw.add_image_quad(id, left[j], right[j], right[j + 1], left[j + 1]).col(col(alpha([47, 184, 255, 255], fill))).build();
        }
    }
    let seed = (dir.rem_euclid(TAU) * 1000.0) as u32;
    let on = |j: usize| if broken > 0.0 && hash(seed + (j as u32 / 3)) < broken { 0.0 } else { 1.0 };
    // No body (the eruption's curls): one tapered stroke along the centre line instead of two edges.
    if fill <= 0.0 {
        let taper = |j: usize| (PI * j as f32 / (N - 1) as f32).sin().powf(0.6).max(0.2);
        strip(draw, r.ribbon, &center, &|j| 6.0 * k * line * taper(j), &|j| 0.55 * bright * on(j), [56, 255, 255, 255]);
        strip(draw, r.ribbon, &center, &|j| 2.2 * k * line * taper(j), &|j| bright * on(j), [224, 255, 255, 255]);
        return;
    }
    for edge in [&left, &right] {
        strip(draw, r.ribbon, edge, &|_| 7.0 * k * line, &|j| 0.5 * bright * on(j), [56, 255, 255, 255]);
        strip(draw, r.ribbon, edge, &|_| 1.8 * k * line, &|j| 0.95 * bright * on(j), [224, 255, 255, 255]);
    }
}

/// The TENDRIL step (spec 4.5): a thin jagged crackle from 1.5 U out to `len` U, mostly radial, sometimes with a
/// hook at the end; fading by `a` and breaking into pieces as `broken` goes from 0 to 1.
#[allow(clippy::too_many_arguments)]
fn tendril(draw: &DrawListMut, ribbon: Picture, c: [f32; 2], u: f32, k: f32, dir: f32, len: f32, seed: u32, a: f32, broken: f32) {
    const SEG: usize = 12;
    let hook = hash(seed * 31 + 50) > 0.5;
    let pts: Vec<[f32; 2]> = (0..=SEG)
        .map(|j| {
            let f = j as f32 / SEG as f32;
            let rr = 1.5 + (len - 1.5) * f;
            let off = if j == 0 { 0.0 } else { (hash(seed * 31 + 60 + j as u32) - 0.5) * 0.18 };
            let ang = dir + if hook && j >= SEG - 1 { 0.12 * (j + 2 - SEG) as f32 } else { 0.0 };
            let (s, co) = ang.sin_cos();
            [c[0] + rr * u * co - off * u * s, c[1] + rr * u * s + off * u * co]
        })
        .collect();
    for j in 0..SEG {
        if hash(seed * 31 + 100 + j as u32) < broken * 0.7 {
            continue;
        }
        let piece = [pts[j], pts[j + 1]];
        strip(draw, ribbon, &piece, &|_| 4.0 * k, &|_| 0.5 * a, [111, 216, 255, 255]);
        strip(draw, ribbon, &piece, &|_| 1.6 * k, &|_| a, [200, 240, 255, 255]);
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Keyframes: held before and after, straight lines between (the "becoming ready" tables rely on it).
    #[test]
    fn keyframes() {
        let k = [(0.0, 0.85), (8.0, 0.5), (12.0, 1.0)];
        assert_eq!(key(-1.0, &k), 0.85);
        assert!((key(4.0, &k) - 0.675).abs() < 1e-5);
        assert!((key(10.0, &k) - 0.75).abs() < 1e-5);
        assert_eq!(key(20.0, &k), 1.0);
        assert_eq!(key_col(5.0, &[(0.0, 0x000000), (10.0, 0x204060)]), [16, 32, 48, 255]);
    }

    /// Spec 4.1-4.2: a burst every 1.25-1.49 s, its poses in order BUILD, PEAK, blank, SECOND, TENDRIL, 25 frames.
    #[test]
    fn bursts_follow_the_spec() {
        let mut starts = Vec::new();
        let mut last: Option<Burst> = None;
        for i in 0..(60 * 30) {
            let now = i as f32 / 60.0;
            let b = Burst::at(now);
            if let Some(b) = b {
                if last.is_none() {
                    assert_eq!(b.frame, 0, "a burst starts at its first frame (t = {now})");
                    starts.push(now);
                }
                let expected = match b.frame {
                    0..=3 => Step::Build,
                    4..=8 => Step::Peak,
                    9..=13 => Step::Blank,
                    14..=18 => Step::Second,
                    _ => Step::Tendril,
                };
                assert!(b.step() == expected);
            }
            last = b;
        }
        assert!(starts.len() >= 20);
        for w in starts.windows(2) {
            let gap = w[1] - w[0];
            assert!((1.2..1.52).contains(&gap), "burst gap {gap}");
        }
    }
}
