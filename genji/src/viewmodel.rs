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
type Mat = [f32; 12];

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
    pub rgba: Vec<u8>,
    depth: Vec<f32>,
}

impl Canvas {
    pub fn new(w: usize, h: usize) -> Self {
        Self { w, h, rgba: vec![0; w * h * 4], depth: vec![0.0; w * h] }
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
fn to_view(p: Vec3) -> Vec3 {
    let d = p - EYE;
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

/// Draws the model posed with `pose` into `canvas` (which keeps what's there: clear it first).
pub fn draw(vm: &Viewmodel, pose: &[Mat], canvas: &mut Canvas) -> Stats {
    let m = &vm.model;
    let (w, h) = (canvas.w as f32, canvas.h as f32);
    let focal = 1.0 / (vertical_fov() / 2.0).tan();
    let aspect = w / h;
    // Skin every vertex into view space.
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
        view.push(to_view(p));
        let n = n.normalize_or_zero();
        nrm.push(Vec3::new(-n.x, n.z, -n.y));
    }
    let screen = |v: Vec3| Vec2::new((v.x / v.z * focal / aspect * 0.5 + 0.5) * w, (0.5 - v.y / v.z * focal * 0.5) * h);
    let light = LIGHT.normalize();
    let mut stats = Stats::default();
    for tri in m.indices.chunks_exact(3) {
        let [a, b, c] = [tri[0] as usize, tri[1] as usize, tri[2] as usize];
        let (va, vb, vc) = (view[a], view[b], view[c]);
        if va.z < NEAR || vb.z < NEAR || vc.z < NEAR {
            stats.culled += 1;
            continue;
        }
        // Back faces: the triangle's own normal, turned to agree with its vertex normals, facing away from the eye.
        let mut g = (vb - va).cross(vc - va);
        if g.dot(nrm[a] + nrm[b] + nrm[c]) < 0.0 {
            g = -g;
        }
        if g.dot(va) >= 0.0 {
            stats.culled += 1;
            continue;
        }
        let (sa, sb, sc) = (screen(va), screen(vb), screen(vc));
        let area = (sb - sa).perp_dot(sc - sa);
        if area.abs() < 1e-6 {
            continue;
        }
        let x0 = sa.x.min(sb.x).min(sc.x).floor().max(0.0) as usize;
        let x1 = (sa.x.max(sb.x).max(sc.x).ceil() as usize).min(canvas.w);
        let y0 = sa.y.min(sb.y).min(sc.y).floor().max(0.0) as usize;
        let y1 = (sa.y.max(sb.y).max(sc.y).ceil() as usize).min(canvas.h);
        if x0 >= x1 || y0 >= y1 {
            continue;
        }
        stats.triangles += 1;
        let (iza, izb, izc) = (1.0 / va.z, 1.0 / vb.z, 1.0 / vc.z);
        for y in y0..y1 {
            for x in x0..x1 {
                let p = Vec2::new(x as f32 + 0.5, y as f32 + 0.5);
                let wa = (sb - p).perp_dot(sc - p) / area;
                let wb = (sc - p).perp_dot(sa - p) / area;
                let wc = 1.0 - wa - wb;
                if wa < 0.0 || wb < 0.0 || wc < 0.0 {
                    continue;
                }
                let iz = wa * iza + wb * izb + wc * izc;
                let at = y * canvas.w + x;
                if iz <= canvas.depth[at] {
                    continue;
                }
                canvas.depth[at] = iz;
                // Perspective-correct weights.
                let (pa, pb, pc) = (wa * iza / iz, wb * izb / iz, wc * izc / iz);
                let uv = m.uv[a] * pa + m.uv[b] * pb + m.uv[c] * pc;
                let n = (nrm[a] * pa + nrm[b] * pb + nrm[c] * pc).normalize_or_zero();
                let albedo = sample(&vm.albedo, uv);
                let glow = sample(&vm.emission, uv);
                let lit = AMBIENT + DIFFUSE * n.dot(light).max(0.0);
                let px = &mut canvas.rgba[at * 4..at * 4 + 4];
                for k in 0..3 {
                    px[k] = ((albedo[k] * lit + glow[k]).min(1.0) * 255.0) as u8;
                }
                px[3] = 255;
                stats.pixels += 1;
            }
        }
    }
    stats
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
                let stats = draw(&vm, &pose, &mut canvas);
                let took = t.elapsed();
                println!("{role} at {at} s, {w}x{h}: {took:?}, {stats:?}");
                assert!(stats.pixels > 0, "{role}: nothing drawn");
                write_png(&dir.join(format!("preview_{role}_{w}.png")), &canvas, [120, 128, 140]);
            }
        }
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
