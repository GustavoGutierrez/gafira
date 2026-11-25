//! The deterministic CPU rasterizer (D-130). The image is split into horizontal bands that
//! can be rendered in any order or in parallel by the caller; each band is supersampled on a
//! regular grid, depth-tested, shaded once per sample (opaque and masked), then blended
//! triangles are drawn back to front, and the samples are averaged into straight-alpha sRGB.

use glam::{Mat3, Vec3};

use crate::camera::{Camera, CameraKind};
use crate::light::{Lighting, Rig};
use crate::pose::{Pose, to_quat};
use crate::scene::{AlphaMode, Material, Scene, Texture};

/// What to draw and at which size.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RenderParams {
    pub pose: Pose,
    pub camera: Camera,
    pub lighting: Lighting,
    pub width: u32,
    pub height: u32,
    /// Samples per pixel side (1 to 4): 1 while dragging, more for the final picture.
    pub samples: u32,
}

impl RenderParams {
    /// The largest side the renderer draws (spec 100 §8.2).
    pub const MAX_SIDE: u32 = 8192;
    pub const MAX_SAMPLES: u32 = 4;

    pub fn clamped(self) -> Self {
        Self {
            pose: self.pose.sanitized(),
            camera: self.camera.clamped(),
            width: self.width.clamp(1, Self::MAX_SIDE),
            height: self.height.clamp(1, Self::MAX_SIDE),
            samples: self.samples.clamp(1, Self::MAX_SAMPLES),
            ..self
        }
    }
}

/// Renders the whole picture: `width * height * 4` bytes, sRGB, straight alpha.
pub fn render(scene: &Scene, params: &RenderParams) -> Vec<u8> {
    let prepared = Prepared::new(scene, params);
    let mut out = Vec::with_capacity(prepared.width as usize * prepared.height as usize * 4);
    for i in 0..prepared.band_count() {
        out.extend(prepared.render_band(i));
    }
    out
}

#[derive(Debug, Clone, Copy)]
struct Vertex {
    /// Sample-space position (x right, y down).
    sx: f32,
    sy: f32,
    /// Linear in screen space; larger is nearer.
    depth: f32,
    /// 1/w for perspective-correct attributes (1 in orthographic).
    inv_w: f32,
    /// View-space position, normal and uv, each pre-multiplied by `inv_w`.
    pos: [f32; 3],
    nrm: [f32; 3],
    uv: [f32; 2],
}

#[derive(Debug, Clone, Copy)]
struct Triangle {
    v: [u32; 3],
    material: u32,
    /// Bounding rows in sample space.
    y0: f32,
    y1: f32,
}

/// A scene transformed for one set of parameters, ready to render band by band.
pub struct Prepared<'a> {
    scene: &'a Scene,
    pub width: u32,
    pub height: u32,
    samples: u32,
    band_rows: u32,
    vertices: Vec<Vertex>,
    opaque: Vec<Triangle>,
    /// Blended triangles, sorted far to near.
    blended: Vec<Triangle>,
    rig: Rig,
    perspective: bool,
    lut: [f32; 256],
}

/// Keeps one band's sample buffers near a million samples.
const SAMPLES_PER_BAND: u32 = 1 << 20;

impl<'a> Prepared<'a> {
    pub fn new(scene: &'a Scene, params: &RenderParams) -> Self {
        let p = params.clamped();
        let ss = p.samples;
        let (sw, sh) = ((p.width * ss) as f32, (p.height * ss) as f32);
        let half = sw.min(sh) / 2.0;
        let model_rot = to_quat(p.pose.rotation);
        let scale = Vec3::from_array(p.pose.scale);
        let offset = Vec3::from_array(p.pose.position);
        let cam = p.camera;
        let basis = cam.basis();
        let view_rot = Mat3::from_cols(basis.right, basis.up, basis.back).transpose();
        let eye = Vec3::from_array(cam.eye_for_render());
        let normal_mat = Mat3::from_quat(model_rot) * Mat3::from_diagonal(Vec3::ONE / scale);
        let perspective = cam.kind == CameraKind::Perspective;

        let mut vertices: Vec<Vertex> = Vec::with_capacity(scene.vertex_count());
        let mut opaque = Vec::new();
        let mut blended: Vec<(f32, Triangle)> = Vec::new();
        let project = |view: Vec3, nrm: Vec3, uv: [f32; 2]| -> Option<Vertex> {
            let [nx, ny, dist] = cam.ndc(view.to_array())?;
            let inv_w = if perspective { 1.0 / dist } else { 1.0 };
            Some(Vertex {
                sx: sw / 2.0 + nx * half,
                sy: sh / 2.0 - ny * half,
                depth: if perspective { inv_w } else { -dist },
                inv_w,
                pos: (view * inv_w).to_array(),
                nrm: (nrm * inv_w).to_array(),
                uv: [uv[0] * inv_w, uv[1] * inv_w],
            })
        };
        let near = Camera::near() * 1.5;

        for prim in &scene.primitives {
            let base = vertices.len() as u32;
            let views: Vec<(Vec3, Vec3, [f32; 2])> = prim
                .positions
                .iter()
                .enumerate()
                .map(|(i, &pos)| {
                    let world = model_rot * (Vec3::from_array(pos) * scale) + offset;
                    let view = view_rot * (world - eye);
                    let n = view_rot
                        * (normal_mat
                            * Vec3::from_array(
                                prim.normals.get(i).copied().unwrap_or([0.0, 0.0, 1.0]),
                            ));
                    (
                        view,
                        n.normalize_or_zero(),
                        prim.uvs.get(i).copied().unwrap_or([0.0, 0.0]),
                    )
                })
                .collect();
            // Vertices in front of the eye keep their slot; the others become a placeholder.
            let mut ok = Vec::with_capacity(views.len());
            for &(v, n, uv) in &views {
                match project(v, n, uv) {
                    Some(vx) if !perspective || -v.z >= near => {
                        vertices.push(vx);
                        ok.push(true);
                    }
                    _ => {
                        vertices.push(Vertex {
                            sx: 0.0,
                            sy: 0.0,
                            depth: 0.0,
                            inv_w: 0.0,
                            pos: [0.0; 3],
                            nrm: [0.0; 3],
                            uv: [0.0; 2],
                        });
                        ok.push(false);
                    }
                }
            }
            let mat = &scene.materials[prim.material];
            let is_blend = mat.alpha_mode == AlphaMode::Blend;
            let mut push = |vs: [u32; 3], vertices: &Vec<Vertex>| {
                let [a, b, c] = vs.map(|i| vertices[i as usize]);
                let area = (b.sx - a.sx) * (c.sy - a.sy) - (b.sy - a.sy) * (c.sx - a.sx);
                if area.abs() < 1e-12 || !area.is_finite() {
                    return;
                }
                let t = Triangle {
                    v: vs,
                    material: prim.material as u32,
                    y0: a.sy.min(b.sy).min(c.sy),
                    y1: a.sy.max(b.sy).max(c.sy),
                };
                if is_blend {
                    blended.push(((a.depth + b.depth + c.depth) / 3.0, t));
                } else {
                    opaque.push(t);
                }
            };
            for tri in prim.indices.chunks_exact(3) {
                let idx = [tri[0], tri[1], tri[2]];
                let inside = idx.map(|i| ok[i as usize]);
                if inside.iter().all(|&b| b) {
                    push(idx.map(|i| base + i), &vertices);
                    continue;
                }
                if inside.iter().all(|&b| !b) {
                    continue;
                }
                // Clip against the near plane (perspective only) and fan the polygon.
                let poly = clip_near(&idx.map(|i| views[i as usize]), near);
                if poly.len() < 3 {
                    continue;
                }
                let start = vertices.len() as u32;
                let mut n: u32 = 0;
                for (v, nrm, uv) in poly {
                    if let Some(vx) = project(v, nrm, uv) {
                        vertices.push(vx);
                        n += 1;
                    }
                }
                for k in 1..n.saturating_sub(1) {
                    push([start, start + k, start + k + 1], &vertices);
                }
            }
        }
        blended.sort_by(|a, b| a.0.total_cmp(&b.0));
        let rows_per_band = (SAMPLES_PER_BAND / (p.width * ss * ss)).clamp(1, p.height);
        let mut lut = [0.0_f32; 256];
        for (i, v) in lut.iter_mut().enumerate() {
            *v = srgb_to_linear(i as f32 / 255.0);
        }
        Self {
            scene,
            width: p.width,
            height: p.height,
            samples: ss,
            band_rows: rows_per_band,
            vertices,
            opaque,
            blended: blended.into_iter().map(|(_, t)| t).collect(),
            rig: p.lighting.rig(),
            perspective,
            lut,
        }
    }

    pub fn band_count(&self) -> usize {
        self.height.div_ceil(self.band_rows) as usize
    }

    /// The output rows `[start, end)` of a band.
    pub fn band_rows(&self, index: usize) -> (u32, u32) {
        let start = index as u32 * self.band_rows;
        (start, (start + self.band_rows).min(self.height))
    }

    /// Renders one band: `width * rows * 4` bytes, sRGB, straight alpha.
    pub fn render_band(&self, index: usize) -> Vec<u8> {
        let (r0, r1) = self.band_rows(index);
        let ss = self.samples;
        let sw = self.width * ss;
        let s0 = r0 * ss;
        let rows = (r1 - r0) * ss;
        let n = (sw * rows) as usize;
        let mut depth = vec![f32::NEG_INFINITY; n];
        let mut tri_id = vec![u32::MAX; n];
        let mut bary = vec![[0.0_f32; 2]; n];
        let (fy0, fy1) = (s0 as f32, (s0 + rows) as f32);

        for (ti, t) in self.opaque.iter().enumerate() {
            if t.y1 < fy0 || t.y0 > fy1 {
                continue;
            }
            let mat = &self.scene.materials[t.material as usize];
            let cutoff = match mat.alpha_mode {
                AlphaMode::Mask(c) => Some(c),
                _ => None,
            };
            self.scan(t, sw, s0, rows, |idx, b1, b2, d| {
                if d <= depth[idx] {
                    return;
                }
                if let Some(c) = cutoff {
                    let (uv, _) = self.interp_uv(t, b1, b2);
                    if self.base_alpha(mat, uv) < c {
                        return;
                    }
                }
                depth[idx] = d;
                tri_id[idx] = ti as u32;
                bary[idx] = [b1, b2];
            });
        }

        // Shade: premultiplied, tone-mapped linear colour per sample.
        let mut color = vec![[0.0_f32; 4]; n];
        for i in 0..n {
            if tri_id[i] != u32::MAX {
                let t = &self.opaque[tri_id[i] as usize];
                let [b1, b2] = bary[i];
                let mut c = self.shade(t, b1, b2);
                if !matches!(
                    self.scene.materials[t.material as usize].alpha_mode,
                    AlphaMode::Blend
                ) {
                    c[3] = 1.0;
                }
                color[i] = [c[0] * c[3], c[1] * c[3], c[2] * c[3], c[3]];
            }
        }
        for t in &self.blended {
            if t.y1 < fy0 || t.y0 > fy1 {
                continue;
            }
            self.scan(t, sw, s0, rows, |idx, b1, b2, d| {
                if d <= depth[idx] {
                    return;
                }
                let c = self.shade(t, b1, b2);
                let a = c[3];
                let dst = color[idx];
                color[idx] = [
                    c[0] * a + dst[0] * (1.0 - a),
                    c[1] * a + dst[1] * (1.0 - a),
                    c[2] * a + dst[2] * (1.0 - a),
                    a + dst[3] * (1.0 - a),
                ];
            });
        }

        // Resolve: average the samples of each pixel, then straight-alpha sRGB.
        let w = self.width as usize;
        let mut out = vec![0_u8; w * (r1 - r0) as usize * 4];
        let inv = 1.0 / (ss * ss) as f32;
        for py in 0..(r1 - r0) as usize {
            for px in 0..w {
                let mut acc = [0.0_f32; 4];
                for sy in 0..ss as usize {
                    let row = (py * ss as usize + sy) * sw as usize;
                    for sx in 0..ss as usize {
                        let c = color[row + px * ss as usize + sx];
                        for k in 0..4 {
                            acc[k] += c[k];
                        }
                    }
                }
                let a = acc[3] * inv;
                let o = (py * w + px) * 4;
                if a <= 0.0 {
                    continue;
                }
                for k in 0..3 {
                    out[o + k] = to_u8(linear_to_srgb((acc[k] * inv / a).clamp(0.0, 1.0)));
                }
                out[o + 3] = to_u8(a.clamp(0.0, 1.0));
            }
        }
        out
    }

    /// Visits every sample of the band inside a triangle, with perspective-correct
    /// barycentrics of vertices 1 and 2 and the screen-linear depth.
    fn scan(
        &self,
        t: &Triangle,
        sw: u32,
        s0: u32,
        rows: u32,
        mut visit: impl FnMut(usize, f32, f32, f32),
    ) {
        let [a, b, c] = t.v.map(|i| self.vertices[i as usize]);
        let area = (b.sx - a.sx) * (c.sy - a.sy) - (b.sy - a.sy) * (c.sx - a.sx);
        let inv_area = 1.0 / area;
        let min_x = a.sx.min(b.sx).min(c.sx).floor().max(0.0) as i64;
        let max_x = (a.sx.max(b.sx).max(c.sx).ceil() as i64).min(sw as i64 - 1);
        let min_y = (t.y0.floor() as i64).max(s0 as i64);
        let max_y = (t.y1.ceil() as i64).min((s0 + rows) as i64 - 1);
        if min_x > max_x || min_y > max_y {
            return;
        }
        // Top-left fill rule so shared edges are drawn exactly once.
        let bias = |p: &Vertex, q: &Vertex| -> f32 {
            let (dx, dy) = (q.sx - p.sx, q.sy - p.sy);
            let top_left = if area > 0.0 {
                dy < 0.0 || (dy == 0.0 && dx > 0.0)
            } else {
                dy > 0.0 || (dy == 0.0 && dx < 0.0)
            };
            if top_left { 0.0 } else { -1e-7 }
        };
        let (bias0, bias1, bias2) = (bias(&b, &c), bias(&c, &a), bias(&a, &b));
        for y in min_y..=max_y {
            let py = y as f32 + 0.5;
            let row = ((y as u32 - s0) * sw) as usize;
            for x in min_x..=max_x {
                let px = x as f32 + 0.5;
                let w0 = ((c.sx - b.sx) * (py - b.sy) - (c.sy - b.sy) * (px - b.sx)) * inv_area;
                let w1 = ((a.sx - c.sx) * (py - c.sy) - (a.sy - c.sy) * (px - c.sx)) * inv_area;
                let w2 = 1.0 - w0 - w1;
                if w0 + bias0 < 0.0 || w1 + bias1 < 0.0 || w2 + bias2 < 0.0 {
                    continue;
                }
                let d = w0 * a.depth + w1 * b.depth + w2 * c.depth;
                let iw = w0 * a.inv_w + w1 * b.inv_w + w2 * c.inv_w;
                if iw <= 0.0 {
                    continue;
                }
                let b1 = w1 * b.inv_w / iw;
                let b2 = w2 * c.inv_w / iw;
                visit(row + x as usize, b1, b2, d);
            }
        }
    }

    /// Perspective-correct attributes at barycentrics (b1, b2): uv and 1/w.
    fn interp_uv(&self, t: &Triangle, b1: f32, b2: f32) -> ([f32; 2], f32) {
        let [a, b, c] = t.v.map(|i| self.vertices[i as usize]);
        let b0 = 1.0 - b1 - b2;
        let (wa, wb, wc) = (1.0 / a.inv_w, 1.0 / b.inv_w, 1.0 / c.inv_w);
        let uv = [
            b0 * a.uv[0] * wa + b1 * b.uv[0] * wb + b2 * c.uv[0] * wc,
            b0 * a.uv[1] * wa + b1 * b.uv[1] * wb + b2 * c.uv[1] * wc,
        ];
        (uv, 1.0)
    }

    fn base_alpha(&self, mat: &Material, uv: [f32; 2]) -> f32 {
        let tex_a = mat
            .base_texture
            .map_or(1.0, |i| self.sample(&self.scene.textures[i], uv)[3]);
        mat.base_color[3] * tex_a
    }

    /// Bilinear, repeating texture sample: linear RGB, alpha.
    fn sample(&self, tex: &Texture, uv: [f32; 2]) -> [f32; 4] {
        let (w, h) = (tex.width as f32, tex.height as f32);
        let u = uv[0] - uv[0].floor();
        let v = uv[1] - uv[1].floor();
        let x = u * w - 0.5;
        let y = v * h - 0.5;
        let (x0, y0) = (x.floor(), y.floor());
        let (fx, fy) = (x - x0, y - y0);
        let wrap = |i: f32, n: u32| -> usize { (i as i64).rem_euclid(n as i64) as usize };
        let px = |xi: f32, yi: f32| -> [f32; 4] {
            let o = (wrap(yi, tex.height) * tex.width as usize + wrap(xi, tex.width)) * 4;
            let p = &tex.rgba[o..o + 4];
            [
                self.lut[p[0] as usize],
                self.lut[p[1] as usize],
                self.lut[p[2] as usize],
                p[3] as f32 / 255.0,
            ]
        };
        let (c00, c10, c01, c11) = (
            px(x0, y0),
            px(x0 + 1.0, y0),
            px(x0, y0 + 1.0),
            px(x0 + 1.0, y0 + 1.0),
        );
        let mut out = [0.0; 4];
        for k in 0..4 {
            let top = c00[k] + (c10[k] - c00[k]) * fx;
            let bot = c01[k] + (c11[k] - c01[k]) * fx;
            out[k] = top + (bot - top) * fy;
        }
        out
    }

    /// Shades one sample: tone-mapped linear RGB and alpha.
    fn shade(&self, t: &Triangle, b1: f32, b2: f32) -> [f32; 4] {
        let [a, b, c] = t.v.map(|i| self.vertices[i as usize]);
        let b0 = 1.0 - b1 - b2;
        let attr3 = |f: fn(&Vertex) -> [f32; 3]| -> Vec3 {
            let (fa, fb, fc) = (f(&a), f(&b), f(&c));
            Vec3::new(
                b0 * fa[0] / a.inv_w + b1 * fb[0] / b.inv_w + b2 * fc[0] / c.inv_w,
                b0 * fa[1] / a.inv_w + b1 * fb[1] / b.inv_w + b2 * fc[1] / c.inv_w,
                b0 * fa[2] / a.inv_w + b1 * fb[2] / b.inv_w + b2 * fc[2] / c.inv_w,
            )
        };
        let pos = attr3(|v| v.pos);
        let mut n = attr3(|v| v.nrm).normalize_or(Vec3::Z);
        let view_dir = if self.perspective {
            (-pos).normalize_or(Vec3::Z)
        } else {
            Vec3::Z
        };
        // Two-sided lighting: the normal faces the viewer.
        if n.dot(view_dir) < 0.0 {
            n = -n;
        }
        let mat = &self.scene.materials[t.material as usize];
        let (uv, _) = self.interp_uv(t, b1, b2);
        let mut base = mat.base_color;
        if let Some(ti) = mat.base_texture {
            let s = self.sample(&self.scene.textures[ti], uv);
            for k in 0..4 {
                base[k] *= s[k];
            }
        }
        let albedo = Vec3::new(base[0], base[1], base[2]);
        let metallic = mat.metallic.clamp(0.0, 1.0);
        let rough = mat.roughness.clamp(0.04, 1.0);
        let f0 = Vec3::splat(0.04).lerp(albedo, metallic);
        let diffuse_albedo = albedo * (1.0 - metallic);
        let rig = &self.rig;
        let hemi = (n.y * 0.5 + 0.5).clamp(0.0, 1.0);
        let ambient = Vec3::from_array(rig.ground).lerp(Vec3::from_array(rig.sky), hemi);
        let shininess = (2.0 / (rough.powi(4) + 1e-4) - 2.0).clamp(2.0, 2048.0);
        let norm = (shininess + 8.0) / (8.0 * std::f32::consts::PI);
        let mut col = diffuse_albedo * ambient + f0 * ambient * (1.0 - rough * 0.7);
        for l in &rig.lights {
            let ldir = Vec3::from_array(l.dir).normalize_or(Vec3::Z);
            let ndl = n.dot(ldir);
            if ndl <= 0.0 {
                continue;
            }
            let lc = Vec3::from_array(l.color);
            let h = (ldir + view_dir).normalize_or(n);
            let spec = n.dot(h).max(0.0).powf(shininess) * norm * rig.specular;
            let fresnel = f0 + (Vec3::ONE - f0) * (1.0 - view_dir.dot(h).clamp(0.0, 1.0)).powi(5);
            col += (diffuse_albedo * ndl + fresnel * spec * ndl) * lc;
        }
        col += Vec3::from_array(mat.emissive);
        let mapped = aces(col);
        [mapped.x, mapped.y, mapped.z, base[3].clamp(0.0, 1.0)]
    }
}

/// Near-plane clipping of a view-space triangle (Sutherland–Hodgman, one plane).
fn clip_near(tri: &[(Vec3, Vec3, [f32; 2]); 3], near: f32) -> Vec<(Vec3, Vec3, [f32; 2])> {
    let inside = |v: &Vec3| -v.z >= near;
    let mut out = Vec::with_capacity(4);
    for i in 0..3 {
        let cur = tri[i];
        let nxt = tri[(i + 1) % 3];
        let (ci, ni) = (inside(&cur.0), inside(&nxt.0));
        if ci {
            out.push(cur);
        }
        if ci != ni {
            let t = (-near - cur.0.z) / (nxt.0.z - cur.0.z);
            out.push((
                cur.0.lerp(nxt.0, t),
                cur.1.lerp(nxt.1, t).normalize_or(Vec3::Z),
                [
                    cur.2[0] + (nxt.2[0] - cur.2[0]) * t,
                    cur.2[1] + (nxt.2[1] - cur.2[1]) * t,
                ],
            ));
        }
    }
    out
}

/// The ACES filmic curve (Narkowicz fit), so bright highlights roll off instead of clipping.
fn aces(c: Vec3) -> Vec3 {
    let c = c * 0.9;
    ((c * (2.51 * c + 0.03)) / (c * (2.43 * c + 0.59) + 0.14)).clamp(Vec3::ZERO, Vec3::ONE)
}

fn srgb_to_linear(c: f32) -> f32 {
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

fn linear_to_srgb(c: f32) -> f32 {
    if c <= 0.0031308 {
        c * 12.92
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    }
}

fn to_u8(v: f32) -> u8 {
    (v * 255.0 + 0.5) as u8
}
