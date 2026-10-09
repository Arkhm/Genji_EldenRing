//! Genji's first-person arms (step 10, DESIGN.md 5k): our own small software renderer. It reads what
//! `tools\export_viewmodel.py` wrote to D:\GenjiBridgeAssets\viewmodel (outside the repository), poses the mesh for
//! an animation frame and draws it into an RGBA picture with a depth buffer, Genji's textures and simple lighting.
//! The overlay lays that picture over the game (step 10c); first-person arms are always drawn over the world, so
//! nothing has to mix with Elden Ring's own 3D.
//!
//! Space: Blender's, as exported (metres, Z up, Genji looking along -Y, his eye at EYE, his right hand towards -X).
//! The view: x right, y up, z forward (distance in front of the eye).

#![allow(dead_code)] // step 10b: used by the preview test; the game uses it from step 10c

use std::path::Path;

use glam::{Vec2, Vec3};

pub const DIR: &str = r"D:\GenjiBridgeAssets\viewmodel";
/// Genji's eye in the model's space (found with Blender, 2026-10-09: from there the previews look as in Overwatch).
pub const EYE: Vec3 = Vec3::new(0.0, 0.05, 1.65);
/// Your Overwatch field of view: 103 degrees horizontal at 16:9 (the same as camera.rs), as a vertical angle.
pub fn vertical_fov() -> f32 {
    2.0 * ((103f32.to_radians() / 2.0).tan() * 9.0 / 16.0).atan()
}
const NEAR: f32 = 0.02;
/// Light: a key light from above, left and in front (towards it, in view space), plus ambient.
const LIGHT: Vec3 = Vec3::new(-0.35, 0.75, -0.55);
const AMBIENT: f32 = 0.42;
const DIFFUSE: f32 = 0.7;

pub struct Model {
    pos: Vec<Vec3>,
    nrm: Vec<Vec3>,
    uv: Vec<Vec2>,
    bones: Vec<[u8; 4]>,
    weights: Vec<[f32; 4]>,
    indices: Vec<u32>,
    bone_count: usize,
}

pub struct Anim {
    pub role: String,
    pub id: String,
    pub fps: f32,
    pub frames: usize,
    offset: usize,
}

impl Anim {
    pub fn seconds(&self) -> f32 {
        (self.frames.saturating_sub(1)) as f32 / self.fps
    }
}

pub struct Texture {
    w: usize,
    h: usize,
    rgba: Vec<u8>,
}

pub struct Viewmodel {
    model: Model,
    pub anims: Vec<Anim>,
    /// Every frame of every animation: per bone a 3x4 matrix by rows.
    mats: Vec<f32>,
    albedo: Texture,
    emission: Texture,
}

/// A 3x4 skinning matrix (rows).
pub type Mat = [f32; 12];

fn read_u32(b: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(b[at..at + 4].try_into().unwrap())
}

fn read_f32(b: &[u8], at: usize) -> f32 {
    f32::from_le_bytes(b[at..at + 4].try_into().unwrap())
}

fn load_texture(path: &Path) -> Result<Texture, String> {
    let (rgba, w, h) = crate::hud::decode_png(path)?;
    Ok(Texture { w: w as usize, h: h as usize, rgba })
}

impl Viewmodel {
    pub fn load(dir: &Path) -> Result<Self, String> {
        let read = |name: &str| std::fs::read(dir.join(name)).map_err(|e| format!("{name}: {e}"));
        let b = read("model.bin")?;
        if &b[0..4] != b"GVM1" {
            return Err("model.bin: not a GVM1 file".into());
        }
        let (nv, ni, nb) = (read_u32(&b, 4) as usize, read_u32(&b, 8) as usize, read_u32(&b, 12) as usize);
        const STRIDE: usize = 8 * 4 + 4 + 4 * 4;
        if b.len() != 16 + nv * STRIDE + ni * 4 {
            return Err(format!("model.bin: size {} doesn't match {nv} vertices and {ni} indices", b.len()));
        }
        let mut model = Model {
            pos: Vec::with_capacity(nv),
            nrm: Vec::with_capacity(nv),
            uv: Vec::with_capacity(nv),
            bones: Vec::with_capacity(nv),
            weights: Vec::with_capacity(nv),
            indices: Vec::with_capacity(ni),
            bone_count: nb,
        };
        for i in 0..nv {
            let at = 16 + i * STRIDE;
            let f = |k: usize| read_f32(&b, at + 4 * k);
            model.pos.push(Vec3::new(f(0), f(1), f(2)));
            model.nrm.push(Vec3::new(f(3), f(4), f(5)));
            model.uv.push(Vec2::new(f(6), f(7)));
            let bones = [b[at + 32], b[at + 33], b[at + 34], b[at + 35]];
            if bones.iter().any(|&x| x as usize >= nb) {
                return Err(format!("model.bin: vertex {i} uses bone {bones:?} of {nb}"));
            }
            model.bones.push(bones);
            model.weights.push(std::array::from_fn(|k| read_f32(&b, at + 36 + 4 * k)));
        }
        let at = 16 + nv * STRIDE;
        model.indices = (0..ni).map(|k| read_u32(&b, at + 4 * k)).collect();
        if model.indices.iter().any(|&i| i as usize >= nv) {
            return Err("model.bin: an index past the vertices".into());
        }

        let list = String::from_utf8(read("anims.txt")?).map_err(|e| e.to_string())?;
        let mut anims = Vec::new();
        for line in list.lines().filter(|l| !l.trim().is_empty()) {
            let p: Vec<&str> = line.split_whitespace().collect();
            let bad = || format!("anims.txt: bad line {line:?}");
            if p.len() != 5 {
                return Err(bad());
            }
            anims.push(Anim {
                role: p[0].into(),
                id: p[1].into(),
                fps: p[2].parse().map_err(|_| bad())?,
                frames: p[3].parse().map_err(|_| bad())?,
                offset: p[4].parse().map_err(|_| bad())?,
            });
        }
        let raw = read("anims.bin")?;
        let mats: Vec<f32> = raw.chunks_exact(4).map(|c| f32::from_le_bytes(c.try_into().unwrap())).collect();
        let total: usize = anims.iter().map(|a| a.frames).sum();
        if mats.len() != total * nb * 12 {
            return Err(format!("anims.bin: {} floats, expected {}", mats.len(), total * nb * 12));
        }
        Ok(Self {
            model,
            anims,
            mats,
            albedo: load_texture(&dir.join("albedo.png"))?,
            emission: load_texture(&dir.join("emission.png"))?,
        })
    }

    /// The first animation for `role`, if exported.
    pub fn find(&self, role: &str) -> Option<usize> {
        self.anims.iter().position(|a| a.role == role)
    }

    /// Variant `k` of `role` (wrapping), if exported.
    pub fn find_variant(&self, role: &str, k: usize) -> Option<usize> {
        let all: Vec<usize> = (0..self.anims.len()).filter(|&i| self.anims[i].role == role).collect();
        (!all.is_empty()).then(|| all[k % all.len()])
    }

    /// Two poses blended: `w` of `b` (0..1), the rest of `a` (short blends between clips). Each bone's rotation is
    /// blended as a rotation and its position as a position. Mixing the matrices' numbers directly (the first way)
    /// squashed and twisted the arm while blending: you saw it on M1 (2026-10-09).
    pub fn blend(a: &[Mat], b: &[Mat], w: f32) -> Vec<Mat> {
        a.iter()
            .zip(b)
            .map(|(x, y)| {
                let ((rx, sx, tx), (ry, sy, ty)) = (parts(x), parts(y));
                from_parts(rx.slerp(ry, w), sx * (1.0 - w) + sy * w, tx.lerp(ty, w))
            })
            .collect()
    }

    /// The skinning matrices of animation `anim` at `t` seconds (held at the last frame, or wrapped if `looping`),
    /// blended linearly between the two nearest frames.
    pub fn pose(&self, anim: usize, t: f32, looping: bool) -> Vec<Mat> {
        let a = &self.anims[anim];
        let nb = self.model.bone_count;
        let last = a.frames.saturating_sub(1);
        let mut f = (t.max(0.0) * a.fps).max(0.0);
        if looping && last > 0 {
            f %= last as f32;
        }
        let f = f.min(last as f32);
        let (i0, frac) = (f.floor() as usize, f.fract());
        let i1 = (i0 + 1).min(last);
        let frame = |i: usize, b: usize| {
            let at = ((a.offset + i) * nb + b) * 12;
            &self.mats[at..at + 12]
        };
        (0..nb)
            .map(|b| {
                let (m0, m1) = (frame(i0, b), frame(i1, b));
                std::array::from_fn(|k| m0[k] + (m1[k] - m0[k]) * frac)
            })
            .collect()
    }
}

/// An RGBA picture with a depth buffer.
pub struct Canvas {
    pub w: usize,
    pub h: usize,
    /// The camera: the eye in the model's space and the vertical field of view (radians).
    pub eye: Vec3,
    pub vfov: f32,
    /// Radians the arms are turned down around the eye (up if negative): while climbing, the camera's pitch, so the
    /// hands stay on the wall when you look up (Overwatch).
    pub tilt: f32,
    pub rgba: Vec<u8>,
    depth: Vec<f32>,
}

impl Canvas {
    pub fn new(w: usize, h: usize) -> Self {
        Self { w, h, eye: EYE, vfov: vertical_fov(), tilt: 0.0, rgba: vec![0; w * h * 4], depth: vec![0.0; w * h] }
    }

    pub fn clear(&mut self) {
        self.rgba.fill(0);
        self.depth.fill(0.0);
    }
}

/// What a draw did (logged).
#[derive(Debug, Default)]
pub struct Stats {
    pub triangles: usize,
    pub culled: usize,
    pub pixels: usize,
    /// The area drawn in, [x0, y0, x1, y1) in pixels (with a pixel around for the edge colours), if any.
    pub rect: Option<[u32; 4]>,
}

/// A skinning matrix as a rotation, a stretch and a translation: M = R * S with S symmetric (polar decomposition).
/// Some of Genji's bones are scaled up to 4 times and sheared (Overwatch's twist and stretch helpers), so a plain
/// scale-and-rotation split loses them.
fn parts(m: &Mat) -> (glam::Quat, glam::Mat3, Vec3) {
    let mm = glam::Mat3::from_cols(Vec3::new(m[0], m[4], m[8]), Vec3::new(m[1], m[5], m[9]), Vec3::new(m[2], m[6], m[10]));
    // R = lim (R + R^-T) / 2, starting from M (converges in a few steps for these matrices).
    let mut r = mm;
    for _ in 0..12 {
        let next = (r + r.inverse().transpose()) * 0.5;
        let done = (next - r).to_cols_array().iter().all(|v| v.abs() < 1e-6);
        r = next;
        if done {
            break;
        }
    }
    let stretch = r.transpose() * mm;
    (glam::Quat::from_mat3(&r).normalize(), stretch, Vec3::new(m[3], m[7], m[11]))
}

fn from_parts(q: glam::Quat, stretch: glam::Mat3, t: Vec3) -> Mat {
    let a = glam::Mat3::from_quat(q) * stretch;
    let (c0, c1, c2) = (a.x_axis, a.y_axis, a.z_axis);
    [c0.x, c1.x, c2.x, t.x, c0.y, c1.y, c2.y, t.y, c0.z, c1.z, c2.z, t.z]
}

fn apply(m: &Mat, p: Vec3) -> Vec3 {
    Vec3::new(
        m[0] * p.x + m[1] * p.y + m[2] * p.z + m[3],
        m[4] * p.x + m[5] * p.y + m[6] * p.z + m[7],
        m[8] * p.x + m[9] * p.y + m[10] * p.z + m[11],
    )
}

fn rotate(m: &Mat, n: Vec3) -> Vec3 {
    Vec3::new(m[0] * n.x + m[1] * n.y + m[2] * n.z, m[4] * n.x + m[5] * n.y + m[6] * n.z, m[8] * n.x + m[9] * n.y + m[10] * n.z)
}

/// Model space -> view space (x right, y up, z forward from the eye). Looking along -Y with Z up, the right hand
/// side is -X (the first preview had +X and came out mirrored).
fn to_view(p: Vec3, eye: Vec3) -> Vec3 {
    let d = p - eye;
    Vec3::new(-d.x, d.z, -d.y)
}

fn sample(t: &Texture, uv: Vec2) -> [f32; 3] {
    // Blender's v goes up; the picture's rows go down. Bilinear, wrapping.
    let x = uv.x.rem_euclid(1.0) * t.w as f32 - 0.5;
    let y = (1.0 - uv.y).rem_euclid(1.0) * t.h as f32 - 0.5;
    let (x0, y0) = (x.floor(), y.floor());
    let (fx, fy) = (x - x0, y - y0);
    let px = |xi: f32, yi: f32| {
        let xi = (xi as i64).rem_euclid(t.w as i64) as usize;
        let yi = (yi as i64).rem_euclid(t.h as i64) as usize;
        let at = (yi * t.w + xi) * 4;
        [t.rgba[at] as f32, t.rgba[at + 1] as f32, t.rgba[at + 2] as f32]
    };
    let (a, b, c, d) = (px(x0, y0), px(x0 + 1.0, y0), px(x0, y0 + 1.0), px(x0 + 1.0, y0 + 1.0));
    std::array::from_fn(|k| {
        let top = a[k] + (b[k] - a[k]) * fx;
        let bottom = c[k] + (d[k] - c[k]) * fx;
        (top + (bottom - top) * fy) / 255.0
    })
}

/// The mesh posed and moved into view space, and its visible triangles set up for drawing.
struct Prepared {
    nrm: Vec<Vec3>,
    tris: Vec<Tri>,
    culled: usize,
}

/// A triangle facing the eye, on screen: its corners (screen position, 1/depth) and where it covers.
struct Tri {
    v: [usize; 3],
    s: [Vec2; 3],
    iz: [f32; 3],
    area: f32,
    x0: usize,
    x1: usize,
    y0: usize,
    y1: usize,
}

fn prepare(vm: &Viewmodel, pose: &[Mat], w: usize, h: usize, eye: Vec3, vfov: f32, tilt: f32) -> Prepared {
    let (st, ct) = tilt.sin_cos();
    // Turn down by `tilt` around the eye's sideways axis (view space: y up, z forward).
    let turn = |v: Vec3| Vec3::new(v.x, v.y * ct - v.z * st, v.y * st + v.z * ct);
    let m = &vm.model;
    let mut view = Vec::with_capacity(m.pos.len());
    let mut nrm = Vec::with_capacity(m.pos.len());
    for i in 0..m.pos.len() {
        let (mut p, mut n) = (Vec3::ZERO, Vec3::ZERO);
        for k in 0..4 {
            let wgt = m.weights[i][k];
            if wgt > 0.0 {
                let mat = &pose[m.bones[i][k] as usize];
                p += wgt * apply(mat, m.pos[i]);
                n += wgt * rotate(mat, m.nrm[i]);
            }
        }
        view.push(turn(to_view(p, eye)));
        let n = n.normalize_or_zero();
        nrm.push(turn(Vec3::new(-n.x, n.z, -n.y)));
    }
    let (wf, hf) = (w as f32, h as f32);
    let focal = 1.0 / (vfov / 2.0).tan();
    let aspect = wf / hf;
    let screen = |v: Vec3| Vec2::new((v.x / v.z * focal / aspect * 0.5 + 0.5) * wf, (0.5 - v.y / v.z * focal * 0.5) * hf);
    let (mut tris, mut culled) = (Vec::new(), 0);
    for tri in m.indices.chunks_exact(3) {
        let v = [tri[0] as usize, tri[1] as usize, tri[2] as usize];
        let (va, vb, vc) = (view[v[0]], view[v[1]], view[v[2]]);
        if va.z < NEAR || vb.z < NEAR || vc.z < NEAR {
            culled += 1;
            continue;
        }
        // Back faces: the triangle's own normal, turned to agree with its vertex normals, facing away from the eye.
        let mut g = (vb - va).cross(vc - va);
        if g.dot(nrm[v[0]] + nrm[v[1]] + nrm[v[2]]) < 0.0 {
            g = -g;
        }
        if g.dot(va) >= 0.0 {
            culled += 1;
            continue;
        }
        let s = [screen(va), screen(vb), screen(vc)];
        let area = (s[1] - s[0]).perp_dot(s[2] - s[0]);
        if area.abs() < 1e-6 {
            continue;
        }
        let x0 = s[0].x.min(s[1].x).min(s[2].x).floor().max(0.0) as usize;
        let x1 = (s[0].x.max(s[1].x).max(s[2].x).ceil().max(0.0) as usize).min(w);
        let y0 = s[0].y.min(s[1].y).min(s[2].y).floor().max(0.0) as usize;
        let y1 = (s[0].y.max(s[1].y).max(s[2].y).ceil().max(0.0) as usize).min(h);
        if x0 < x1 && y0 < y1 {
            tris.push(Tri { v, s, iz: [1.0 / va.z, 1.0 / vb.z, 1.0 / vc.z], area, x0, x1, y0, y1 });
        }
    }
    Prepared { nrm, tris, culled }
}

/// One horizontal band of the picture: its rows, and its part of the colour and depth buffers.
struct Band<'a> {
    rows: std::ops::Range<usize>,
    rgba: &'a mut [u8],
    depth: &'a mut [f32],
    /// Triangles that reach into these rows.
    tris: Vec<u32>,
}

fn raster(vm: &Viewmodel, p: &Prepared, w: usize, band: &mut Band) -> usize {
    let m = &vm.model;
    let light = LIGHT.normalize();
    let mut pixels = 0;
    for &ti in &band.tris {
        let t = &p.tris[ti as usize];
        let [a, b, c] = t.v;
        let [sa, sb, sc] = t.s;
        let [iza, izb, izc] = t.iz;
        for y in t.y0.max(band.rows.start)..t.y1.min(band.rows.end) {
            for x in t.x0..t.x1 {
                let q = Vec2::new(x as f32 + 0.5, y as f32 + 0.5);
                let wa = (sb - q).perp_dot(sc - q) / t.area;
                let wb = (sc - q).perp_dot(sa - q) / t.area;
                let wc = 1.0 - wa - wb;
                if wa < 0.0 || wb < 0.0 || wc < 0.0 {
                    continue;
                }
                let iz = wa * iza + wb * izb + wc * izc;
                let at = (y - band.rows.start) * w + x;
                if iz <= band.depth[at] {
                    continue;
                }
                band.depth[at] = iz;
                // Perspective-correct weights.
                let (pa, pb, pc) = (wa * iza / iz, wb * izb / iz, wc * izc / iz);
                let uv = m.uv[a] * pa + m.uv[b] * pb + m.uv[c] * pc;
                let n = (p.nrm[a] * pa + p.nrm[b] * pb + p.nrm[c] * pc).normalize_or_zero();
                let albedo = sample(&vm.albedo, uv);
                let glow = sample(&vm.emission, uv);
                let lit = AMBIENT + DIFFUSE * n.dot(light).max(0.0);
                let px = &mut band.rgba[at * 4..at * 4 + 4];
                for k in 0..3 {
                    px[k] = ((albedo[k] * lit + glow[k]).min(1.0) * 255.0) as u8;
                }
                px[3] = 255;
                pixels += 1;
            }
        }
    }
    if pixels > 0 {
        pad_edges(band.rgba, w, band.rows.len());
    }
    pixels
}

/// Rows per band: thin bands, handed out in turn, share the work evenly even though the arms sit low on the screen.
const BAND_ROWS: usize = 32;

/// Draws the model posed with `pose` into `canvas` (which keeps what's there: clear it first), on `threads`
/// threads at once.
pub fn draw(vm: &Viewmodel, pose: &[Mat], canvas: &mut Canvas, threads: usize) -> Stats {
    let (w, h) = (canvas.w, canvas.h);
    let p = prepare(vm, pose, w, h, canvas.eye, canvas.vfov, canvas.tilt);
    let mut bands: Vec<Band> = canvas
        .rgba
        .chunks_mut(BAND_ROWS * w * 4)
        .zip(canvas.depth.chunks_mut(BAND_ROWS * w))
        .enumerate()
        .map(|(i, (rgba, depth))| Band { rows: i * BAND_ROWS..((i + 1) * BAND_ROWS).min(h), rgba, depth, tris: Vec::new() })
        .collect();
    let count = bands.len();
    for (ti, t) in p.tris.iter().enumerate() {
        for band in &mut bands[t.y0 / BAND_ROWS..t.y1.div_ceil(BAND_ROWS).min(count)] {
            band.tris.push(ti as u32);
        }
    }
    let threads = threads.max(1);
    let mut per_thread: Vec<Vec<Band>> = (0..threads).map(|_| Vec::new()).collect();
    for (i, band) in bands.into_iter().enumerate() {
        per_thread[i % threads].push(band);
    }
    let p = &p;
    let pixels: usize = std::thread::scope(|scope| {
        let jobs: Vec<_> = per_thread
            .into_iter()
            .map(|mut mine| scope.spawn(move || mine.iter_mut().map(|band| raster(vm, p, w, band)).sum::<usize>()))
            .collect();
        jobs.into_iter().map(|j| j.join().unwrap_or(0)).sum()
    });
    let rect = p.tris.iter().fold(None, |acc: Option<[u32; 4]>, t| {
        let r = [t.x0.saturating_sub(1) as u32, t.y0.saturating_sub(1) as u32, (t.x1 + 1).min(w) as u32, (t.y1 + 1).min(h) as u32];
        Some(acc.map_or(r, |a| union(a, r)))
    });
    Stats { triangles: p.tris.len(), culled: p.culled, pixels, rect }
}

/// The smallest rectangle holding both.
pub fn union(a: [u32; 4], b: [u32; 4]) -> [u32; 4] {
    [a[0].min(b[0]), a[1].min(b[1]), a[2].max(b[2]), a[3].max(b[3])]
}

/// Gives the empty pixels next to the drawing the drawing's colour (still fully see-through), so stretching the
/// picture on the graphics card blends the edges with the arm's colour instead of black (no dark outline). Within
/// one band (`rows` rows of `w`).
fn pad_edges(rgba: &mut [u8], w: usize, rows: usize) {
    let alpha: Vec<u8> = rgba.chunks_exact(4).map(|p| p[3]).collect();
    for y in 0..rows {
        for x in 0..w {
            let i = y * w + x;
            if alpha[i] != 0 {
                continue;
            }
            let near = [
                (x > 0).then(|| i - 1),
                (x + 1 < w).then(|| i + 1),
                (y > 0).then(|| i - w),
                (y + 1 < rows).then(|| i + w),
            ];
            if let Some(n) = near.into_iter().flatten().find(|&n| alpha[n] != 0) {
                let (dst, src) = (i * 4, n * 4);
                for k in 0..3 {
                    rgba[dst + k] = rgba[src + k];
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_png(path: &Path, c: &Canvas, background: [u8; 3]) {
        let mut out = Vec::with_capacity(c.w * c.h * 3);
        for px in c.rgba.chunks_exact(4) {
            let a = px[3] as f32 / 255.0;
            for k in 0..3 {
                out.push((px[k] as f32 * a + background[k] as f32 * (1.0 - a)) as u8);
            }
        }
        let file = std::fs::File::create(path).unwrap();
        let mut enc = png::Encoder::new(std::io::BufWriter::new(file), c.w as u32, c.h as u32);
        enc.set_color(png::ColorType::Rgb);
        enc.write_header().unwrap().write_image_data(&out).unwrap();
    }

    /// Step 10b's first picture: Genji's arms in a few poses from his eye, at your screen size (2580x1080) and at
    /// half size, written next to the exported files, with the drawing times. Skipped where the export doesn't exist
    /// (it's outside the repository). Run: cargo test --release viewmodel -- --nocapture
    #[test]
    fn previews() {
        let dir = Path::new(DIR);
        if !dir.join("model.bin").is_file() {
            return;
        }
        let t = std::time::Instant::now();
        let vm = Viewmodel::load(dir).unwrap();
        println!("loaded {} animations in {:?}", vm.anims.len(), t.elapsed());
        for (role, at) in [("idle", 0.0), ("blade_idle", 0.0), ("deflect_idle", 0.0), ("reload", 0.6), ("throw", 0.15)] {
            let anim = vm.find(role).unwrap_or_else(|| panic!("no {role}"));
            let pose = vm.pose(anim, at, false);
            for (w, h) in [(2580, 1080), (1290, 540)] {
                let mut canvas = Canvas::new(w, h);
                let t = std::time::Instant::now();
                let stats = draw(&vm, &pose, &mut canvas, 4);
                let took = t.elapsed();
                println!("{role} at {at} s, {w}x{h}: {took:?}, {stats:?}");
                assert!(stats.pixels > 0, "{role}: nothing drawn");
                // Every pixel drawn (and its edge colour) lies inside the reported area: the overlay copies only that.
                let [x0, y0, x1, y1] = stats.rect.expect("an area").map(|v| v as usize);
                for (i, px) in canvas.rgba.chunks_exact(4).enumerate() {
                    let (x, y) = (i % w, i / w);
                    if px[3] != 0 || px[0] != 0 {
                        assert!(x >= x0 && x < x1 && y >= y0 && y < y1, "{role}: pixel ({x}, {y}) outside {:?}", stats.rect);
                    }
                }
                write_png(&dir.join(format!("preview_{role}_{w}.png")), &canvas, [120, 128, 140]);
            }
        }
    }

    /// Drawn in 4 bands at once, the picture is the same as in one; and the timings of both.
    #[test]
    fn bands_draw_the_same_picture() {
        let dir = Path::new(DIR);
        if !dir.join("model.bin").is_file() {
            return;
        }
        let vm = Viewmodel::load(dir).unwrap();
        let pose = vm.pose(vm.find("blade_idle").unwrap(), 0.0, false);
        let mut one = Canvas::new(1290, 540);
        let mut four = Canvas::new(1290, 540);
        let t = std::time::Instant::now();
        draw(&vm, &pose, &mut one, 1);
        let t1 = t.elapsed();
        let t = std::time::Instant::now();
        draw(&vm, &pose, &mut four, 4);
        let t4 = t.elapsed();
        println!("blade_idle 1290x540: 1 thread {t1:?}, 4 threads {t4:?}");
        let alpha = |c: &Canvas| c.rgba.chunks_exact(4).map(|p| p[3]).collect::<Vec<_>>();
        assert_eq!(alpha(&one), alpha(&four));
    }

    /// Camera candidates for the Dragonblade pose at your screen's shape (3440x1440, drawn at half size), to find where
    /// Overwatch's camera is (your test of 2026-10-09: the left arm's cut end showed at the bottom). Writes
    /// camera_<name>.png. Run: cargo test --release cameras -- --nocapture --ignored
    #[test]
    #[ignore]
    fn cameras() {
        let dir = Path::new(DIR);
        let vm = Viewmodel::load(dir).unwrap();
        let ow = vertical_fov();
        for (name, eye, vfov) in [
            ("now", EYE, ow),
            ("down_2cm", EYE + Vec3::Z * 0.02, ow),
            ("down_4cm", EYE + Vec3::Z * 0.04, ow),
            ("down_6cm", EYE + Vec3::Z * 0.06, ow),
            ("socket_01AC", Vec3::new(0.0, 0.0, 1.616), ow),
            ("socket_0012", Vec3::new(0.0, 0.003, 1.621), ow),
            ("now_fov60", EYE, 60f32.to_radians()),
            ("socket_01AC_fov60", Vec3::new(0.0, 0.0, 1.616), 60f32.to_radians()),
        ] {
            for role in ["blade_idle", "idle"] {
                let pose = vm.pose(vm.find(role).unwrap(), 0.0, false);
                let mut canvas = Canvas::new(1720, 720);
                canvas.eye = eye;
                canvas.vfov = vfov;
                draw(&vm, &pose, &mut canvas, 4);
                write_png(&dir.join(format!("camera_{role}_{name}.png")), &canvas, [120, 128, 140]);
            }
        }
    }

    /// Our throw at several times and fields of view, next to your Overwatch frame (kill clip, during a throw), at
    /// its 2580x1080 shape: which field of view does Overwatch draw the arms with? Writes fov_throw_<deg>_<ms>.png.
    #[test]
    #[ignore]
    fn fov_versus_overwatch() {
        let dir = Path::new(DIR);
        let vm = Viewmodel::load(dir).unwrap();
        let throw = vm.find("throw").unwrap();
        for deg in [70.5f32, 60.0, 50.0, 42.0] {
            for ms in [0, 50, 100, 150, 200, 300] {
                let pose = vm.pose(throw, ms as f32 / 1000.0, false);
                let mut canvas = Canvas::new(1290, 540);
                canvas.vfov = deg.to_radians();
                draw(&vm, &pose, &mut canvas, 4);
                write_png(&dir.join(format!("fov_throw_{deg}_{ms}.png")), &canvas, [120, 128, 140]);
            }
        }
    }

    /// An animation as a strip of pictures (every `step` seconds) at your recordings' 2580x1080 shape, half size, for
    /// comparing with D:\GenjiBridgeAssets\ow_ref. Writes strip_<role>_<k>.png. Roles from GENJI_STRIP (comma
    /// separated), default throw. Run: cargo test --release strips -- --ignored
    #[test]
    #[ignore]
    fn strips() {
        let dir = Path::new(DIR);
        let vm = Viewmodel::load(dir).unwrap();
        let roles = std::env::var("GENJI_STRIP").unwrap_or_else(|_| "throw".into());
        for role in roles.split(',') {
            // "role" or "role#k" for variant k.
            let (name, v) = role.split_once('#').map_or((role, 0), |(r, k)| (r, k.parse().unwrap()));
            let anim = vm.find_variant(name, v).unwrap_or_else(|| panic!("no {role}"));
            let len = vm.anims[anim].seconds();
            let mut k = 0;
            let mut t = 0.0;
            while t <= len + 1e-3 {
                let pose = vm.pose(anim, t, false);
                let mut canvas = Canvas::new(1290, 540);
                draw(&vm, &pose, &mut canvas, 4);
                write_png(&dir.join(format!("strip_{}_{k:02}.png", role.replace('#', "_v"))), &canvas, [120, 128, 140]);
                k += 1;
                t += 0.15;
            }
        }
    }

    /// Blending halfway between two poses keeps every bone rigid (no squashing), and the ends are the poses.
    #[test]
    fn blend_keeps_bones_rigid() {
        let dir = Path::new(DIR);
        if !dir.join("model.bin").is_file() {
            return;
        }
        let vm = Viewmodel::load(dir).unwrap();
        let a = vm.pose(vm.find("idle").unwrap(), 0.0, false);
        let b = vm.pose(vm.find("throw").unwrap(), 0.3, false);
        let mid = Viewmodel::blend(&a, &b, 0.5);
        // The volume each bone gives the mesh (the determinant) stays between the two poses': no squashing flat.
        let det = |m: &Mat| glam::Mat3::from_cols(Vec3::new(m[0], m[4], m[8]), Vec3::new(m[1], m[5], m[9]), Vec3::new(m[2], m[6], m[10])).determinant();
        for (m, (x0, y0)) in mid.iter().zip(a.iter().zip(&b)) {
            let (lo, hi) = (det(x0).min(det(y0)), det(x0).max(det(y0)));
            assert!(det(m) >= lo * 0.9 - 1e-3 && det(m) <= hi * 1.1 + 1e-3, "{} not within {lo}..{hi}", det(m));
        }
        let end = Viewmodel::blend(&a, &b, 1.0);
        let worst = end.iter().flatten().zip(b.iter().flatten()).map(|(p, q)| (p - q).abs()).fold(0.0, f32::max);
        assert!(worst < 1e-3, "blend at 1 differs from the pose by {worst}");
    }

    #[test]
    fn pose_holds_and_loops() {
        let dir = Path::new(DIR);
        if !dir.join("model.bin").is_file() {
            return;
        }
        let vm = Viewmodel::load(dir).unwrap();
        let walk = vm.find("walk").unwrap();
        let len = vm.anims[walk].seconds();
        let start = vm.pose(walk, 0.0, true);
        let wrapped = vm.pose(walk, len, true);
        assert!(start.iter().flatten().zip(wrapped.iter().flatten()).all(|(a, b)| (a - b).abs() < 1e-4));
        let reload = vm.find("reload").unwrap();
        let end = vm.pose(reload, vm.anims[reload].seconds(), false);
        let past = vm.pose(reload, 99.0, false);
        assert_eq!(end, past);
    }
}
