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
    /// The part each vertex belongs to: 0 the arms, then the weapons (PART_* below).
    part: Vec<u8>,
    indices: Vec<u32>,
    /// The arms' bones (those in anims.bin), then the shuriken's own bones.
    bone_count: usize,
    extra_bones: usize,
}

/// The shuriken stack's bones relative to the hand bone, frame by frame, for arm animations that have a shuriken
/// animation of their own (tools\export_viewmodel.py, SHURIKEN_PAIRS); the first pair is the resting pose, used for
/// every other animation.
struct Shuriken {
    hand: usize,
    bones: usize,
    /// (arm animation id, frames, first frame in `mats`).
    pairs: Vec<(String, usize, usize)>,
    mats: Vec<f32>,
}

/// The arms' skeleton (bones.txt): per bone its parent and its rest matrix in world space, and an order with every
/// parent before its children. Blends turn each bone relative to its parent, around its own joint.
struct Skeleton {
    parent: Vec<Option<usize>>,
    rest: Vec<Mat>,
    rest_inv: Vec<Mat>,
    order: Vec<usize>,
}

impl Skeleton {
    fn parse(list: &str, nb: usize) -> Result<Self, String> {
        let (mut parent, mut rest) = (Vec::new(), Vec::new());
        for line in list.lines().filter(|l| !l.trim().is_empty()) {
            let p: Vec<&str> = line.split_whitespace().collect();
            let bad = || format!("bones.txt: bad line {line:?}");
            if p.len() != 15 || p[0].parse::<usize>() != Ok(parent.len()) {
                return Err(bad());
            }
            let up: i64 = p[1].parse().map_err(|_| bad())?;
            parent.push(if up < 0 { None } else { Some(up as usize) });
            let mut m = [0.0; 12];
            for (k, v) in m.iter_mut().enumerate() {
                *v = p[3 + k].parse().map_err(|_| bad())?;
            }
            rest.push(m);
        }
        if parent.len() != nb || parent.iter().flatten().any(|&q| q >= nb) {
            return Err(format!("bones.txt: {} bones, expected {nb}", parent.len()));
        }
        // Parents first.
        let mut order = Vec::with_capacity(nb);
        let mut placed = vec![false; nb];
        while order.len() < nb {
            let before = order.len();
            for i in 0..nb {
                if !placed[i] && parent[i].is_none_or(|q| placed[q]) {
                    placed[i] = true;
                    order.push(i);
                }
            }
            if order.len() == before {
                return Err("bones.txt: the parents go round in a loop".into());
            }
        }
        let rest_inv = rest.iter().map(inverse).collect();
        Ok(Self { parent, rest, rest_inv, order })
    }
}

/// Parts of the model (tools\export_viewmodel.py, WEAPONS), as bits of a mask of what to show.
pub const PART_ARMS: u8 = 1 << 0;
pub const PART_SHURIKEN: u8 = 1 << 1;
pub const PART_KATANA: u8 = 1 << 2;
pub const PART_WAKIZASHI: u8 = 1 << 3;

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
    /// Per part: its colour texture and, if it has one, its glow texture.
    textures: Vec<(Texture, Option<Texture>)>,
    shuriken: Option<Shuriken>,
    /// Without it (an older export), blends turn each bone around the model's origin.
    skeleton: Option<Skeleton>,
}

/// A 3x4 skinning matrix (rows).
pub type Mat = [f32; 12];

/// Share of a weapon's length, from its grip end, where its blade is taken to start (the hilt is about a quarter).
const BLADE_START: f32 = 0.28;

/// A weapon's blade in the rest pose (Viewmodel::blade_line).
pub struct BladeLine {
    pub bone: usize,
    pub base: Vec3,
    pub tip: Vec3,
}

impl BladeLine {
    /// Base and tip on a canvas as shares of it (0..1 across and down), posed; None if either is behind the eye or
    /// far off screen (passing right by the eye, where the picture of it blows up).
    pub fn on_screen(&self, pose: &[Mat], canvas: &Canvas) -> Option<([f32; 2], [f32; 2])> {
        let m = &pose[self.bone];
        let aspect = canvas.w as f32 / canvas.h as f32;
        let near = |p: [f32; 2]| p.iter().all(|v| (-1.0..=2.0).contains(v));
        let base = project(apply(m, self.base), canvas.eye, canvas.vfov, canvas.tilt, aspect).filter(|p| near(*p))?;
        let tip = project(apply(m, self.tip), canvas.eye, canvas.vfov, canvas.tilt, aspect).filter(|p| near(*p))?;
        Some((base, tip))
    }
}

/// Where a model-space point lands on the screen, as in `prepare` (same view, tilt and perspective), as shares of
/// the canvas; None behind the eye.
pub fn project(p: Vec3, eye: Vec3, vfov: f32, tilt: f32, aspect: f32) -> Option<[f32; 2]> {
    let v = to_view(p, eye);
    let (st, ct) = tilt.sin_cos();
    let v = Vec3::new(v.x, v.y * ct - v.z * st, v.y * st + v.z * ct);
    if v.z < NEAR {
        return None;
    }
    let focal = 1.0 / (vfov / 2.0).tan();
    Some([v.x / v.z * focal / aspect * 0.5 + 0.5, 0.5 - v.y / v.z * focal * 0.5])
}

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
        // GVM1: the arms only; GVM2: a part number per vertex (the weapons); GVM3: also extra bones (the shuriken's).
        let (with_parts, head) = match &b[0..4] {
            b"GVM1" => (false, 16),
            b"GVM2" => (true, 16),
            b"GVM3" => (true, 20),
            _ => return Err("model.bin: not a GVM1, GVM2 or GVM3 file".into()),
        };
        let (nv, ni, nb) = (read_u32(&b, 4) as usize, read_u32(&b, 8) as usize, read_u32(&b, 12) as usize);
        let extra = if head == 20 { read_u32(&b, 16) as usize } else { 0 };
        let stride: usize = 8 * 4 + 4 + 4 * 4 + if with_parts { 4 } else { 0 };
        if b.len() != head + nv * stride + ni * 4 {
            return Err(format!("model.bin: size {} doesn't match {nv} vertices and {ni} indices", b.len()));
        }
        let mut model = Model {
            pos: Vec::with_capacity(nv),
            nrm: Vec::with_capacity(nv),
            uv: Vec::with_capacity(nv),
            bones: Vec::with_capacity(nv),
            weights: Vec::with_capacity(nv),
            part: Vec::with_capacity(nv),
            indices: Vec::with_capacity(ni),
            bone_count: nb,
            extra_bones: extra,
        };
        for i in 0..nv {
            let at = head + i * stride;
            let f = |k: usize| read_f32(&b, at + 4 * k);
            model.pos.push(Vec3::new(f(0), f(1), f(2)));
            model.nrm.push(Vec3::new(f(3), f(4), f(5)));
            model.uv.push(Vec2::new(f(6), f(7)));
            let bones = [b[at + 32], b[at + 33], b[at + 34], b[at + 35]];
            if bones.iter().any(|&x| x as usize >= nb + extra) {
                return Err(format!("model.bin: vertex {i} uses bone {bones:?} of {}", nb + extra));
            }
            model.bones.push(bones);
            model.weights.push(std::array::from_fn(|k| read_f32(&b, at + 36 + 4 * k)));
            model.part.push(if with_parts { b[at + 52] } else { 0 });
        }
        let at = head + nv * stride;
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
        // parts.txt: "part albedo emission" ("-": none); without it, the arms' two textures.
        let list = std::fs::read_to_string(dir.join("parts.txt")).unwrap_or_else(|_| "0 albedo.png emission.png".into());
        let mut textures = Vec::new();
        for line in list.lines().filter(|l| !l.trim().is_empty()) {
            let p: Vec<&str> = line.split_whitespace().collect();
            if p.len() != 3 || p[0].parse::<usize>() != Ok(textures.len()) {
                return Err(format!("parts.txt: bad line {line:?}"));
            }
            let albedo = load_texture(&dir.join(p[1]))?;
            let glow = if p[2] == "-" { None } else { Some(load_texture(&dir.join(p[2]))?) };
            textures.push((albedo, glow));
        }
        if let Some(&p) = model.part.iter().find(|&&p| p as usize >= textures.len()) {
            return Err(format!("model.bin: part {p} has no textures in parts.txt"));
        }
        // shuriken.txt: "hand <bone> bones <n>", then "arm_id shuriken_id frames offset" per pair.
        let shuriken = match std::fs::read_to_string(dir.join("shuriken.txt")) {
            Ok(list) if extra > 0 => {
                let mut lines = list.lines().filter(|l| !l.trim().is_empty());
                let head: Vec<&str> = lines.next().unwrap_or("").split_whitespace().collect();
                let bad = |what: &str| format!("shuriken.txt: bad {what}");
                if head.len() != 4 || head[0] != "hand" || head[2] != "bones" {
                    return Err(bad("first line"));
                }
                let hand: usize = head[1].parse().map_err(|_| bad("hand"))?;
                let bones: usize = head[3].parse().map_err(|_| bad("bones"))?;
                if hand >= nb || bones != extra {
                    return Err(bad("bone numbers"));
                }
                let mut pairs = Vec::new();
                for line in lines {
                    let p: Vec<&str> = line.split_whitespace().collect();
                    if p.len() != 4 {
                        return Err(bad("line"));
                    }
                    pairs.push((p[0].to_string(), p[2].parse().map_err(|_| bad("frames"))?, p[3].parse().map_err(|_| bad("offset"))?));
                }
                let raw = read("shuriken.bin")?;
                let mats: Vec<f32> = raw.chunks_exact(4).map(|c| f32::from_le_bytes(c.try_into().unwrap())).collect();
                let total: usize = pairs.iter().map(|p| p.1).sum();
                if pairs.is_empty() || mats.len() != total * bones * 12 {
                    return Err(bad("shuriken.bin size"));
                }
                Some(Shuriken { hand, bones, pairs, mats })
            }
            _ => None,
        };
        let skeleton = match std::fs::read_to_string(dir.join("bones.txt")) {
            Ok(list) => Some(Skeleton::parse(&list, nb)?),
            Err(_) => None,
        };
        Ok(Self { model, anims, mats, textures, shuriken, skeleton })
    }

    /// A weapon part's blade, for its trail (trail.rs): the bone it's fixed to, and in the rest pose where the blade
    /// starts and its tip. Its two vertices farthest apart are taken as its ends; the one nearer the hand bone's joint
    /// is the grip end, and the blade is taken to start BLADE_START of the way from there. None without the skeleton.
    pub fn blade_line(&self, part: u8) -> Option<BladeLine> {
        let sk = self.skeleton.as_ref()?;
        let m = &self.model;
        let verts: Vec<usize> = (0..m.pos.len()).filter(|&i| m.part[i] == part).collect();
        let first = *verts.first()?;
        let bone = m.bones[first][0] as usize;
        let centre = verts.iter().map(|&i| m.pos[i]).sum::<Vec3>() / verts.len() as f32;
        let far = |from: Vec3| verts.iter().map(|&i| m.pos[i]).max_by(|a, b| a.distance(from).total_cmp(&b.distance(from)));
        let a = far(centre)?;
        let b = far(a)?;
        let r = &sk.rest[bone];
        let joint = Vec3::new(r[3], r[7], r[11]);
        let (grip, tip) = if a.distance(joint) < b.distance(joint) { (a, b) } else { (b, a) };
        Some(BladeLine { bone, base: grip.lerp(tip, BLADE_START), tip })
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

    /// Two poses blended: `w` of `b` (0..1), the rest of `a` (between two frames, and short blends between clips).
    /// Each bone turns relative to its parent, around its own joint, the way the animation moves it. History (your
    /// tests of 2026-10-09): mixing the matrices' numbers directly squashed the fingers; turning each bone's matrix
    /// as a whole turned it around the model's origin at the feet, 1.5 m from the hand, and threw the fingers out
    /// for a moment ("his fingers simply expand and twist").
    pub fn blend(&self, a: &[Mat], b: &[Mat], w: f32) -> Vec<Mat> {
        let Some(sk) = &self.skeleton else {
            return blend_each(a, b, w);
        };
        let nb = sk.parent.len();
        // Bone matrices (rest -> posed bone in world space), then each relative to its parent.
        let posed = |s: &[Mat]| -> Vec<Mat> { (0..nb).map(|i| compose(&s[i], &sk.rest[i])).collect() };
        let (pa, pb) = (posed(a), posed(b));
        let local = |p: &[Mat], i: usize| match sk.parent[i] {
            Some(q) => compose(&inverse(&p[q]), &p[i]),
            None => p[i],
        };
        let mut out = vec![[0.0; 12]; nb];
        for &i in &sk.order {
            let l = mix(&local(&pa, i), &local(&pb, i), w);
            out[i] = match sk.parent[i] {
                Some(q) => compose(&out[q], &l),
                None => l,
            };
        }
        let mut s: Vec<Mat> = (0..nb).map(|i| compose(&out[i], &sk.rest_inv[i])).collect();
        // The shuriken's bones: relative to the hand.
        let hand = self.shuriken.as_ref().map_or(0, |sh| sh.hand);
        let (ia, ib) = (inverse(&a[hand]), inverse(&b[hand]));
        for k in nb..a.len().min(b.len()) {
            let rel = mix(&compose(&ia, &a[k]), &compose(&ib, &b[k]), w);
            s.push(compose(&s[hand], &rel));
        }
        s
    }

    /// The skinning matrices of animation `anim` at `t` seconds (held at the last frame, or wrapped if `looping`):
    /// the arms' bones, then the shuriken's (on the hand, from the shuriken animation paired with `anim`, or the
    /// resting one).
    pub fn pose(&self, anim: usize, t: f32, looping: bool) -> Vec<Mat> {
        let mut m = self.pose_arms(anim, t, looping);
        if self.model.extra_bones > 0 {
            let hand = self.shuriken.as_ref().map_or(0, |s| s.hand);
            let hand_m = m[hand];
            match &self.shuriken {
                Some(sh) => {
                    let a = &self.anims[anim];
                    let (pair, frames, offset) = sh.pairs.iter().find(|p| p.0 == a.id).map_or((false, sh.pairs[0].1, sh.pairs[0].2), |p| (true, p.1, p.2));
                    let last = frames.saturating_sub(1);
                    let mut f = if pair { t.max(0.0) * a.fps } else { 0.0 };
                    if looping && last > 0 {
                        f %= last as f32;
                    }
                    let f = f.min(last as f32);
                    let (i0, frac) = (f.floor() as usize, f.fract());
                    let i1 = (i0 + 1).min(last);
                    let rel = |i: usize| -> Vec<Mat> {
                        (0..sh.bones).map(|b| sh.mats[((offset + i) * sh.bones + b) * 12..][..12].try_into().unwrap()).collect()
                    };
                    let r = if frac > 0.0 && i1 != i0 { blend_each(&rel(i0), &rel(i1), frac) } else { rel(i0) };
                    m.extend(r.iter().map(|r| compose(&hand_m, r)));
                }
                None => m.extend(std::iter::repeat_n(hand_m, self.model.extra_bones)),
            }
        }
        m
    }

    /// The arms' bones only.
    fn pose_arms(&self, anim: usize, t: f32, looping: bool) -> Vec<Mat> {
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
        // Between two stored frames, each bone turns around its joint (blend): Overwatch turns some finger and weapon
        // bones by up to 180 degrees between two frames.
        let m0: Vec<Mat> = (0..nb).map(|b| frame(i0, b).try_into().unwrap()).collect();
        if frac <= 0.0 || i0 == i1 {
            return m0;
        }
        let m1: Vec<Mat> = (0..nb).map(|b| frame(i1, b).try_into().unwrap()).collect();
        self.blend(&m0, &m1, frac)
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
    /// Which parts to draw (PART_* bits).
    pub parts: u8,
    pub rgba: Vec<u8>,
    depth: Vec<f32>,
}

impl Canvas {
    pub fn new(w: usize, h: usize) -> Self {
        Self { w, h, eye: EYE, vfov: vertical_fov(), tilt: 0.0, parts: PART_ARMS | PART_SHURIKEN, rgba: vec![0; w * h * 4], depth: vec![0.0; w * h] }
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

/// Each matrix blended on its own: rotation turned, stretch and translation mixed (right for matrices whose
/// translation is small: relative to a parent or to the hand).
fn blend_each(a: &[Mat], b: &[Mat], w: f32) -> Vec<Mat> {
    a.iter().zip(b).map(|(x, y)| mix(x, y, w)).collect()
}

fn mix(x: &Mat, y: &Mat, w: f32) -> Mat {
    let ((rx, sx, tx), (ry, sy, ty)) = (parts(x), parts(y));
    from_parts(rx.slerp(ry, w), sx * (1.0 - w) + sy * w, tx.lerp(ty, w))
}

/// The inverse of a 3x4 matrix (as a 4x4 with 0 0 0 1 below).
fn inverse(m: &Mat) -> Mat {
    let a = glam::Mat3::from_cols(Vec3::new(m[0], m[4], m[8]), Vec3::new(m[1], m[5], m[9]), Vec3::new(m[2], m[6], m[10])).inverse();
    let t = -(a * Vec3::new(m[3], m[7], m[11]));
    let (c0, c1, c2) = (a.x_axis, a.y_axis, a.z_axis);
    [c0.x, c1.x, c2.x, t.x, c0.y, c1.y, c2.y, t.y, c0.z, c1.z, c2.z, t.z]
}

fn from_parts(q: glam::Quat, stretch: glam::Mat3, t: Vec3) -> Mat {
    let a = glam::Mat3::from_quat(q) * stretch;
    let (c0, c1, c2) = (a.x_axis, a.y_axis, a.z_axis);
    [c0.x, c1.x, c2.x, t.x, c0.y, c1.y, c2.y, t.y, c0.z, c1.z, c2.z, t.z]
}

/// a then b: the matrix that applies `b` first, then `a` (both 3x4, by rows).
fn compose(a: &Mat, b: &Mat) -> Mat {
    let mut out = [0.0; 12];
    for r in 0..3 {
        for c in 0..4 {
            let mut v = if c == 3 { a[r * 4 + 3] } else { 0.0 };
            for k in 0..3 {
                v += a[r * 4 + k] * b[k * 4 + c];
            }
            out[r * 4 + c] = v;
        }
    }
    out
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

fn prepare(vm: &Viewmodel, pose: &[Mat], w: usize, h: usize, eye: Vec3, vfov: f32, tilt: f32, parts: u8) -> Prepared {
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
        if parts & (1 << m.part[v[0]]) == 0 {
            continue; // a part not shown now (a weapon put away)
        }
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
                let (albedo_tex, glow_tex) = &vm.textures[m.part[a] as usize];
                let albedo = sample(albedo_tex, uv);
                let glow = glow_tex.as_ref().map_or([0.0; 3], |t| sample(t, uv));
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
    let p = prepare(vm, pose, w, h, canvas.eye, canvas.vfov, canvas.tilt, canvas.parts);
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
        for (role, at) in [("idle", 0.0), ("blade_idle", 0.0), ("deflect_idle", 0.0), ("reload", 0.6), ("throw", 0.15), ("melee", 0.2), ("dash", 0.15)] {
            let anim = vm.find(role).unwrap_or_else(|| panic!("no {role}"));
            let pose = vm.pose(anim, at, false);
            for (w, h) in [(2580, 1080), (1290, 540)] {
                let mut canvas = Canvas::new(w, h);
                canvas.parts = crate::arms_anim::parts_for(role);
                if role.starts_with("blade_") {
                    canvas.eye = EYE + Vec3::Z * crate::arms::BLADE_DROP;
                }
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
                // As in the game: the weapon this animation shows, and Dragonblade's lower camera.
                canvas.parts = crate::arms_anim::parts_for(name);
                if name.starts_with("blade_") {
                    canvas.eye = EYE + Vec3::Z * crate::arms::BLADE_DROP;
                }
                draw(&vm, &pose, &mut canvas, 4);
                write_png(&dir.join(format!("strip_{}_{k:02}.png", role.replace('#', "_v"))), &canvas, [120, 128, 140]);
                k += 1;
                t += std::env::var("GENJI_STRIP_STEP").ok().and_then(|v| v.parse().ok()).unwrap_or(0.15);
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
        let mid = vm.blend(&a, &b, 0.5);
        // The volume each bone gives the mesh (the determinant) stays between the two poses': no squashing flat.
        let det = |m: &Mat| glam::Mat3::from_cols(Vec3::new(m[0], m[4], m[8]), Vec3::new(m[1], m[5], m[9]), Vec3::new(m[2], m[6], m[10])).determinant();
        for (m, (x0, y0)) in mid.iter().zip(a.iter().zip(&b)) {
            let (lo, hi) = (det(x0).min(det(y0)), det(x0).max(det(y0)));
            assert!(det(m) >= lo * 0.9 - 1e-3 && det(m) <= hi * 1.1 + 1e-3, "{} not within {lo}..{hi}", det(m));
        }
        let end = vm.blend(&a, &b, 1.0);
        let worst = end.iter().flatten().zip(b.iter().flatten()).map(|(p, q)| (p - q).abs()).fold(0.0, f32::max);
        assert!(worst < 1e-3, "blend at 1 differs from the pose by {worst}");
    }

    /// Candidate resting poses with the shuriken, at your recordings' screen shape, for comparing with Overwatch's
    /// (clip2 at 1.0 s). Writes rest_<id>.png. Run on request.
    #[test]
    #[ignore]
    fn rest_candidates() {
        let dir = Path::new(DIR);
        let vm = Viewmodel::load(dir).unwrap();
        for id in ["240E", "242B", "242E", "3645", "2439", "242F", "3646"] {
            let Some(anim) = vm.anims.iter().position(|a| a.id == id) else {
                continue;
            };
            let pose = vm.pose(anim, 0.0, false);
            let mut canvas = Canvas::new(1290, 540);
            draw(&vm, &pose, &mut canvas, 4);
            write_png(&dir.join(format!("rest_{id}.png")), &canvas, [120, 128, 140]);
        }
    }

    /// Between two stored frames no bone collapses: its volume (determinant) stays between the two frames' (your test of
    /// 2026-10-09: fingers expanded and twisted for a moment; Overwatch turns some bones up to 180 degrees between two
    /// frames). Every animation sampled 120 times a second.
    #[test]
    fn no_bone_collapses_between_frames() {
        let dir = Path::new(DIR);
        if !dir.join("model.bin").is_file() {
            return;
        }
        let vm = Viewmodel::load(dir).unwrap();
        let det = |m: &Mat| glam::Mat3::from_cols(Vec3::new(m[0], m[4], m[8]), Vec3::new(m[1], m[5], m[9]), Vec3::new(m[2], m[6], m[10])).determinant();
        for (i, a) in vm.anims.iter().enumerate() {
            let steps = (a.seconds() * 120.0) as usize;
            for k in 0..steps {
                let t = k as f32 / 120.0;
                let f = t * a.fps;
                let (f0, f1) = (f.floor() / a.fps, (f.floor() + 1.0).min((a.frames - 1) as f32) / a.fps);
                let (p, p0, p1) = (vm.pose(i, t, false), vm.pose(i, f0, false), vm.pose(i, f1, false));
                for b in 0..p.len() {
                    let (d, lo, hi) = (det(&p[b]), det(&p0[b]).min(det(&p1[b])), det(&p0[b]).max(det(&p1[b])));
                    assert!(d >= lo * 0.95 - 1e-3 && d <= hi * 1.05 + 1e-3, "{} {} bone {b} at {t:.3} s: volume {d} outside {lo}..{hi}", a.role, a.id);
                }
            }
        }
    }

    /// Between two stored frames no point of the arms flies off: each lies no farther from the nearer of its two
    /// frame positions than those are apart (a point turning around one joint stays within that), plus 3 cm for
    /// points turned by several joints at once (2.6 cm at most, in the few frames where an animation turns a joint
    /// very far). Your test of 2026-10-09 ("his fingers simply expand and twist for micro seconds"): turning whole skinning matrices
    /// threw finger points tens of centimetres out; this prints the old way's worst next to the new one's.
    #[test]
    fn no_point_flies_off_between_frames() {
        let dir = Path::new(DIR);
        if !dir.join("bones.txt").is_file() {
            return;
        }
        let vm = Viewmodel::load(dir).unwrap();
        let m = &vm.model;
        let verts: Vec<usize> = (0..m.pos.len()).filter(|&i| m.part[i] == 0).step_by(3).collect();
        let skin = |pose: &[Mat], i: usize| -> Vec3 { (0..4).map(|k| apply(&pose[m.bones[i][k] as usize], m.pos[i]) * m.weights[i][k]).sum() };
        let excess = |p: &[Mat], p0: &[Mat], p1: &[Mat]| -> f32 {
            verts
                .iter()
                .map(|&i| {
                    let (x, x0, x1) = (skin(p, i), skin(p0, i), skin(p1, i));
                    x.distance(x0).min(x.distance(x1)) - x0.distance(x1)
                })
                .fold(f32::MIN, f32::max)
        };
        let (mut worst_old, mut worst_new) = ((0.0f32, String::new()), (0.0f32, String::new()));
        for (i, a) in vm.anims.iter().enumerate() {
            for f in 0..a.frames.saturating_sub(1) {
                let (p0, p1) = (vm.pose(i, f as f32 / a.fps, false), vm.pose(i, (f + 1) as f32 / a.fps, false));
                for frac in [0.25f32, 0.5, 0.75] {
                    let p = vm.pose(i, (f as f32 + frac) / a.fps, false);
                    let e = excess(&p, &p0, &p1);
                    if e > worst_new.0 {
                        worst_new = (e, format!("{} {} frame {f}+{frac}", a.role, a.id));
                    }
                    let old = blend_each(&p0, &p1, frac);
                    let e = excess(&old, &p0, &p1);
                    if e > worst_old.0 {
                        worst_old = (e, format!("{} {} frame {f}+{frac}", a.role, a.id));
                    }
                }
            }
        }
        println!("worst point off its path between frames: old way {:.1} cm ({}), now {:.1} cm ({})", worst_old.0 * 100.0, worst_old.1, worst_new.0 * 100.0, worst_new.1);
        assert!(worst_new.0 < 0.03, "a point flies {:.1} cm off between frames ({})", worst_new.0 * 100.0, worst_new.1);
    }

    /// The blades for the green trails (trail.rs) are found, about the right lengths, and in each Dragonblade swing
    /// and dash the tip moves on screen fast enough for the trail to show (above trail.rs's SLOW, 1.2 screen heights
    /// a second), but not all the time. Prints each one's fastest speed and share of time above that.
    #[test]
    fn blades_move_fast_enough_for_trails() {
        let dir = Path::new(DIR);
        if !dir.join("bones.txt").is_file() {
            return;
        }
        let vm = Viewmodel::load(dir).unwrap();
        let (katana, wakizashi) = (vm.blade_line(2).unwrap(), vm.blade_line(3).unwrap());
        let (kl, wl) = (katana.base.distance(katana.tip), wakizashi.base.distance(wakizashi.tip));
        println!("katana blade {kl:.2} m, wakizashi blade {wl:.2} m");
        assert!((0.5..1.4).contains(&kl) && (0.25..0.9).contains(&wl), "blade lengths {kl} {wl}");
        let mut canvas = Canvas::new(1290, 540);
        for (i, a) in vm.anims.iter().enumerate() {
            let line = match a.role.as_str() {
                r if r.starts_with("blade_swing") || r == "blade_dash" => &katana,
                "dash" => &wakizashi,
                _ => continue,
            };
            canvas.eye = if a.role.starts_with("blade_") { EYE + Vec3::Z * crate::arms::BLADE_DROP } else { EYE };
            let step = 1.0 / 60.0;
            let (mut last, mut fastest, mut fast_time, mut t) = (None, 0.0f32, 0.0f32, 0.0f32);
            while t <= a.seconds() {
                let tip = line.on_screen(&vm.pose(i, t, false), &canvas).map(|(_, tip)| tip);
                if let (Some(p), Some(q)) = (last, tip) {
                    let [p, q]: [[f32; 2]; 2] = [p, q];
                    let speed = (((q[0] - p[0]) * 1290.0 / 540.0).powi(2) + (q[1] - p[1]).powi(2)).sqrt() / step;
                    fastest = fastest.max(speed);
                    if speed > 1.2 {
                        fast_time += step;
                    }
                }
                last = tip;
                t += step;
            }
            println!("{} {}: fastest {fastest:.1} screen heights/s, {:.0}% of {:.2} s above the trail's threshold", a.role, a.id, 100.0 * fast_time / a.seconds(), a.seconds());
            if a.role == "blade_swing" || a.role == "dash" {
                assert!(fastest > 4.0, "{} {}: the blade never moves fast enough for a full trail", a.role, a.id);
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
