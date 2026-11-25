//! The imported model as plain data (spec 100 §6.2): triangle primitives, materials and
//! decoded textures, normalized to the unit sphere. Never serialized into the `.gaf`; the
//! original file is, and the scene is rebuilt from it.

use std::fmt;

/// A decoded texture: sRGB, straight alpha, row-major RGBA8.
#[derive(Debug, Clone, PartialEq)]
pub struct Texture {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AlphaMode {
    Opaque,
    /// Fragments under the cutoff are discarded.
    Mask(f32),
    Blend,
}

/// The tested subset of glTF PBR (spec 100 §6.1); OBJ maps onto it.
#[derive(Debug, Clone, PartialEq)]
pub struct Material {
    /// Linear RGBA factor.
    pub base_color: [f32; 4],
    pub base_texture: Option<usize>,
    /// glTF metallic-roughness texture: green scales roughness, blue scales metallic.
    pub metal_rough_texture: Option<usize>,
    /// Ambient occlusion in the red channel.
    pub occlusion_texture: Option<usize>,
    /// sRGB emissive colour, multiplied by `emissive`.
    pub emissive_texture: Option<usize>,
    pub metallic: f32,
    pub roughness: f32,
    /// Linear RGB.
    pub emissive: [f32; 3],
    pub alpha_mode: AlphaMode,
    pub double_sided: bool,
}

impl Material {
    /// The neutral material of a model without materials (spec 100 AC-02).
    pub fn neutral() -> Self {
        Self {
            base_color: [0.72, 0.72, 0.74, 1.0],
            base_texture: None,
            metal_rough_texture: None,
            occlusion_texture: None,
            emissive_texture: None,
            metallic: 0.0,
            roughness: 0.6,
            emissive: [0.0; 3],
            alpha_mode: AlphaMode::Opaque,
            double_sided: true,
        }
    }
}

/// Triangles sharing one material, in the model's own coordinates.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Primitive {
    pub positions: Vec<[f32; 3]>,
    /// One per position, or empty (computed by [`Scene::normalized`]).
    pub normals: Vec<[f32; 3]>,
    /// One per position, or empty.
    pub uvs: Vec<[f32; 2]>,
    /// Three per triangle.
    pub indices: Vec<u32>,
    pub material: usize,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Scene {
    pub primitives: Vec<Primitive>,
    pub materials: Vec<Material>,
    pub textures: Vec<Texture>,
}

/// Limits of an imported model (spec 100 §9); the importer checks them before building it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    pub max_vertices: usize,
    pub max_triangles: usize,
    pub max_materials: usize,
    pub max_textures: usize,
    pub max_texture_side: u32,
    pub max_primitives: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            max_vertices: 2_000_000,
            max_triangles: 2_000_000,
            max_materials: 512,
            max_textures: 128,
            max_texture_side: 8192,
            max_primitives: 8192,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SceneError {
    /// No triangle to draw.
    Empty,
    TooManyVertices(usize),
    TooManyTriangles(usize),
    TooManyMaterials(usize),
    TooManyTextures(usize),
    TooManyPrimitives(usize),
    TextureTooLarge {
        width: u32,
        height: u32,
    },
    /// A texture whose bytes do not match its size.
    BadTexture,
    /// An index outside its primitive's vertices, or a count that is not a multiple of 3.
    BadIndices,
    /// A normal or UV list whose length is not the vertex count.
    BadAttributes,
    BadMaterial,
    /// NaN or infinite coordinates.
    NonFinite,
    /// Every vertex in one point: no size to frame.
    Degenerate,
}

impl fmt::Display for SceneError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => write!(f, "the model has no triangles"),
            Self::TooManyVertices(n) => write!(f, "the model has too many vertices ({n})"),
            Self::TooManyTriangles(n) => write!(f, "the model has too many triangles ({n})"),
            Self::TooManyMaterials(n) => write!(f, "the model has too many materials ({n})"),
            Self::TooManyTextures(n) => write!(f, "the model has too many textures ({n})"),
            Self::TooManyPrimitives(n) => write!(f, "the model has too many parts ({n})"),
            Self::TextureTooLarge { width, height } => {
                write!(f, "a texture is too large ({width}x{height})")
            }
            Self::BadTexture => write!(f, "a texture is damaged"),
            Self::BadIndices => write!(f, "the triangles are damaged"),
            Self::BadAttributes => write!(f, "the normals or texture coordinates are damaged"),
            Self::BadMaterial => write!(f, "a material is damaged"),
            Self::NonFinite => write!(f, "the model has invalid coordinates"),
            Self::Degenerate => write!(f, "the model has no size"),
        }
    }
}

impl std::error::Error for SceneError {}

impl Scene {
    pub fn vertex_count(&self) -> usize {
        self.primitives.iter().map(|p| p.positions.len()).sum()
    }

    pub fn triangle_count(&self) -> usize {
        self.primitives.iter().map(|p| p.indices.len() / 3).sum()
    }

    /// Checks the structure and the limits; the renderer relies on a validated scene.
    pub fn validate(&self, limits: &Limits) -> Result<(), SceneError> {
        if self.primitives.len() > limits.max_primitives {
            return Err(SceneError::TooManyPrimitives(self.primitives.len()));
        }
        if self.materials.len() > limits.max_materials {
            return Err(SceneError::TooManyMaterials(self.materials.len()));
        }
        if self.textures.len() > limits.max_textures {
            return Err(SceneError::TooManyTextures(self.textures.len()));
        }
        let vertices = self.vertex_count();
        if vertices > limits.max_vertices {
            return Err(SceneError::TooManyVertices(vertices));
        }
        let triangles = self.triangle_count();
        if triangles > limits.max_triangles {
            return Err(SceneError::TooManyTriangles(triangles));
        }
        if triangles == 0 {
            return Err(SceneError::Empty);
        }
        for t in &self.textures {
            if t.width > limits.max_texture_side || t.height > limits.max_texture_side {
                return Err(SceneError::TextureTooLarge {
                    width: t.width,
                    height: t.height,
                });
            }
            if t.width == 0
                || t.height == 0
                || t.rgba.len() != t.width as usize * t.height as usize * 4
            {
                return Err(SceneError::BadTexture);
            }
        }
        for m in &self.materials {
            let finite = m
                .base_color
                .iter()
                .chain(&m.emissive)
                .all(|v| v.is_finite())
                && m.metallic.is_finite()
                && m.roughness.is_finite();
            let bad_texture = [
                m.base_texture,
                m.metal_rough_texture,
                m.occlusion_texture,
                m.emissive_texture,
            ]
            .iter()
            .any(|t| t.is_some_and(|t| t >= self.textures.len()));
            if !finite || bad_texture {
                return Err(SceneError::BadMaterial);
            }
        }
        for p in &self.primitives {
            if p.indices.len() % 3 != 0
                || p.indices.iter().any(|&i| i as usize >= p.positions.len())
            {
                return Err(SceneError::BadIndices);
            }
            if (!p.normals.is_empty() && p.normals.len() != p.positions.len())
                || (!p.uvs.is_empty() && p.uvs.len() != p.positions.len())
            {
                return Err(SceneError::BadAttributes);
            }
            if p.material >= self.materials.len() {
                return Err(SceneError::BadMaterial);
            }
            let finite = p.positions.iter().flatten().all(|v| v.is_finite())
                && p.normals.iter().flatten().all(|v| v.is_finite())
                && p.uvs.iter().flatten().all(|v| v.is_finite());
            if !finite {
                return Err(SceneError::NonFinite);
            }
        }
        Ok(())
    }

    /// The bounding box `(min, max)` of every vertex that a triangle uses.
    pub fn bounds(&self) -> Option<([f32; 3], [f32; 3])> {
        let mut min = [f32::INFINITY; 3];
        let mut max = [f32::NEG_INFINITY; 3];
        let mut any = false;
        for p in &self.primitives {
            for &i in &p.indices {
                let v = p.positions[i as usize];
                for k in 0..3 {
                    min[k] = min[k].min(v[k]);
                    max[k] = max[k].max(v[k]);
                }
                any = true;
            }
        }
        any.then_some((min, max))
    }

    /// Validates, then centres the model on the origin, scales it to fit the unit sphere
    /// (spec 100 §3.1) and fills missing normals with smooth ones.
    pub fn normalized(mut self, limits: &Limits) -> Result<Self, SceneError> {
        self.validate(limits)?;
        let (min, max) = self.bounds().ok_or(SceneError::Empty)?;
        let centre = [0, 1, 2].map(|k| (min[k] + max[k]) / 2.0);
        // The farthest vertex from the centre of the box: tighter than the box's half
        // diagonal, and still nothing leaves the sphere when the model turns.
        let mut radius = 0.0_f32;
        for p in &self.primitives {
            for &i in &p.indices {
                let v = p.positions[i as usize];
                let d = (0..3)
                    .map(|k| (v[k] - centre[k]).powi(2))
                    .sum::<f32>()
                    .sqrt();
                radius = radius.max(d);
            }
        }
        if !radius.is_finite() || radius <= f32::EPSILON * 16.0 {
            return Err(SceneError::Degenerate);
        }
        for p in &mut self.primitives {
            for v in &mut p.positions {
                *v = [0, 1, 2].map(|k| (v[k] - centre[k]) / radius);
            }
            if p.normals.is_empty() {
                p.normals = smooth_normals(&p.positions, &p.indices);
            } else {
                for n in &mut p.normals {
                    *n = unit(*n).unwrap_or([0.0, 0.0, 1.0]);
                }
            }
        }
        Ok(self)
    }
}

fn unit(v: [f32; 3]) -> Option<[f32; 3]> {
    let l = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    (l > 1e-20 && l.is_finite()).then(|| v.map(|c| c / l))
}

/// Area-weighted vertex normals.
fn smooth_normals(positions: &[[f32; 3]], indices: &[u32]) -> Vec<[f32; 3]> {
    let mut acc = vec![[0.0_f32; 3]; positions.len()];
    for t in indices.chunks_exact(3) {
        let [a, b, c] = [t[0], t[1], t[2]].map(|i| positions[i as usize]);
        let e1 = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
        let e2 = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
        let n = [
            e1[1] * e2[2] - e1[2] * e2[1],
            e1[2] * e2[0] - e1[0] * e2[2],
            e1[0] * e2[1] - e1[1] * e2[0],
        ];
        for &i in t {
            for k in 0..3 {
                acc[i as usize][k] += n[k];
            }
        }
    }
    acc.into_iter()
        .map(|n| unit(n).unwrap_or([0.0, 0.0, 1.0]))
        .collect()
}
