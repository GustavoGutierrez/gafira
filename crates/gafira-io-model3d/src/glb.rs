//! GLB (glTF 2.0 binary) reading with `gltf` (spec 100 §3.1, §9). Only the embedded binary
//! chunk is read: a buffer or image that points to a URI is refused, so nothing outside the
//! file is ever opened. Every accessor and view range is checked before it is read.

use std::collections::BTreeMap;
use std::io::Cursor;

use gafira_model3d::{AlphaMode, Material, Primitive, Scene, SceneError, Texture};
use glam::{Mat3, Mat4, Vec3};
use gltf::Semantic;
use gltf::mesh::Mode;

use crate::{ImportError, ImportLimits, ImportWarning, budget};

fn damaged(why: impl Into<String>) -> ImportError {
    ImportError::Damaged(why.into())
}

/// Header checks before handing the bytes to the parser: magic, version and length.
fn check_header(bytes: &[u8]) -> Result<(), ImportError> {
    if bytes.len() < 20 || &bytes[0..4] != b"glTF" {
        return Err(damaged("not a GLB header"));
    }
    let word = |o: usize| {
        u32::from_le_bytes([bytes[o], bytes[o + 1], bytes[o + 2], bytes[o + 3]]) as usize
    };
    if word(4) != 2 {
        return Err(damaged("not glTF 2.0"));
    }
    if word(8) != bytes.len() {
        return Err(damaged("the length in the header does not match the file"));
    }
    Ok(())
}

/// The first extension the file requires. None is decoded here (Draco, Meshopt, KTX2...),
/// and the parser would only call it invalid, so it is read from the JSON chunk first.
fn required_extension(bytes: &[u8]) -> Option<String> {
    let len = u32::from_le_bytes([bytes[12], bytes[13], bytes[14], bytes[15]]) as usize;
    let json = bytes.get(20..20usize.checked_add(len)?)?;
    let value: serde_json::Value = serde_json::from_slice(json).ok()?;
    value
        .get("extensionsRequired")?
        .as_array()?
        .first()?
        .as_str()
        .map(str::to_string)
}

pub(crate) fn read(
    bytes: &[u8],
    limits: &ImportLimits,
    cancelled: &dyn Fn() -> bool,
) -> Result<(Scene, Vec<ImportWarning>), ImportError> {
    check_header(bytes)?;
    if let Some(e) = required_extension(bytes) {
        return Err(ImportError::UnsupportedExtension(e));
    }
    let gltf = gltf::Gltf::from_slice(bytes).map_err(|e| damaged(e.to_string()))?;
    let doc = &gltf.document;
    let mut warnings = Vec::new();
    if let Some(e) = doc.extensions_required().next() {
        return Err(ImportError::UnsupportedExtension(e.to_string()));
    }
    for e in doc.extensions_used() {
        if !doc.extensions_required().any(|r| r == e) {
            warnings.push(ImportWarning::IgnoredExtension(e.to_string()));
        }
    }
    let blob: &[u8] = gltf.blob.as_deref().unwrap_or(&[]);
    for buffer in doc.buffers() {
        match buffer.source() {
            gltf::buffer::Source::Bin => {
                if buffer.length() > blob.len() {
                    return Err(damaged("the binary chunk is shorter than its buffer"));
                }
            }
            gltf::buffer::Source::Uri(_) => return Err(ImportError::ExternalReference),
        }
    }
    for view in doc.views() {
        let end = view
            .offset()
            .checked_add(view.length())
            .ok_or_else(|| damaged("view range"))?;
        if end > view.buffer().length() {
            return Err(damaged("a buffer view is outside its buffer"));
        }
    }

    let mut builder = Builder {
        limits,
        blob,
        scene: Scene::default(),
        materials: BTreeMap::new(),
        textures: BTreeMap::new(),
        warnings,
        vertices: 0,
        triangles: 0,
    };
    let scene = doc
        .default_scene()
        .or_else(|| doc.scenes().next())
        .ok_or_else(|| damaged("no scene"))?;
    for node in scene.nodes() {
        builder.node(&node, Mat4::IDENTITY, 0, cancelled)?;
    }
    if builder.scene.materials.is_empty() {
        builder.scene.materials.push(Material::neutral());
    }
    Ok((builder.scene, builder.warnings))
}

struct Builder<'a> {
    limits: &'a ImportLimits,
    blob: &'a [u8],
    scene: Scene,
    /// glTF material index (None = default material) to scene material index.
    materials: BTreeMap<Option<usize>, usize>,
    /// glTF image index to scene texture index (None when it could not be decoded).
    textures: BTreeMap<usize, Option<usize>>,
    warnings: Vec<ImportWarning>,
    vertices: usize,
    triangles: usize,
}

impl Builder<'_> {
    fn node(
        &mut self,
        node: &gltf::Node<'_>,
        parent: Mat4,
        depth: usize,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<(), ImportError> {
        if depth > self.limits.max_node_depth {
            return Err(damaged("the node tree is too deep"));
        }
        if cancelled() {
            return Err(ImportError::Cancelled);
        }
        let local = Mat4::from_cols_array_2d(&node.transform().matrix());
        let world = parent * local;
        if !world.is_finite() {
            return Err(ImportError::Scene(SceneError::NonFinite));
        }
        if let Some(mesh) = node.mesh() {
            for prim in mesh.primitives() {
                self.primitive(&prim, world)?;
            }
        }
        for child in node.children() {
            self.node(&child, world, depth + 1, cancelled)?;
        }
        Ok(())
    }

    fn check_accessor(&self, accessor: &gltf::Accessor<'_>) -> Result<(), ImportError> {
        let view = accessor
            .view()
            .ok_or_else(|| damaged("sparse or empty accessors are not supported"))?;
        if accessor.count() == 0 {
            return Ok(());
        }
        let elem = accessor.size();
        let stride = view.stride().unwrap_or(elem);
        let need = accessor
            .offset()
            .checked_add(
                (accessor.count() - 1)
                    .checked_mul(stride)
                    .ok_or_else(|| damaged("accessor size"))?,
            )
            .and_then(|v| v.checked_add(elem))
            .ok_or_else(|| damaged("accessor size"))?;
        if need > view.length() {
            return Err(damaged("an accessor is outside its view"));
        }
        Ok(())
    }

    fn primitive(&mut self, prim: &gltf::Primitive<'_>, world: Mat4) -> Result<(), ImportError> {
        let mode = prim.mode();
        if !matches!(
            mode,
            Mode::Triangles | Mode::TriangleStrip | Mode::TriangleFan
        ) {
            if !self.warnings.contains(&ImportWarning::NonTriangleParts) {
                self.warnings.push(ImportWarning::NonTriangleParts);
            }
            return Ok(());
        }
        let pos_acc = prim
            .get(&Semantic::Positions)
            .ok_or_else(|| damaged("a part has no positions"))?;
        self.check_accessor(&pos_acc)?;
        let n = pos_acc.count();
        let index_count = match prim.indices() {
            Some(a) => {
                self.check_accessor(&a)?;
                a.count()
            }
            None => n,
        };
        budget(
            &mut self.vertices,
            n,
            self.limits.scene.max_vertices,
            SceneError::TooManyVertices,
        )?;
        budget(
            &mut self.triangles,
            index_count / 3,
            self.limits.scene.max_triangles,
            SceneError::TooManyTriangles,
        )?;
        if self.scene.primitives.len() >= self.limits.scene.max_primitives {
            return Err(ImportError::Scene(SceneError::TooManyPrimitives(
                self.scene.primitives.len() + 1,
            )));
        }
        for semantic in [Semantic::Normals, Semantic::TexCoords(0)] {
            if let Some(a) = prim.get(&semantic) {
                self.check_accessor(&a)?;
            }
        }
        let material_index = self.material(&prim.material())?;
        let tex_set = prim
            .material()
            .pbr_metallic_roughness()
            .base_color_texture()
            .map_or(0, |t| t.tex_coord());
        if tex_set != 0
            && let Some(a) = prim.get(&Semantic::TexCoords(tex_set))
        {
            self.check_accessor(&a)?;
        }

        let blob = self.blob;
        let reader = prim.reader(|b| match b.source() {
            gltf::buffer::Source::Bin => Some(blob),
            gltf::buffer::Source::Uri(_) => None,
        });
        let normal_mat = Mat3::from_mat4(world).inverse().transpose();
        let positions: Vec<[f32; 3]> = reader
            .read_positions()
            .ok_or_else(|| damaged("positions"))?
            .map(|p| world.transform_point3(Vec3::from_array(p)).to_array())
            .collect();
        if positions.len() != n {
            return Err(damaged("positions"));
        }
        let normals: Vec<[f32; 3]> = match reader.read_normals() {
            Some(it) => it
                .map(|v| {
                    (normal_mat * Vec3::from_array(v))
                        .normalize_or_zero()
                        .to_array()
                })
                .collect(),
            None => Vec::new(),
        };
        let normals = if normals.len() == n && normals.iter().all(|v| v != &[0.0; 3]) {
            normals
        } else {
            Vec::new()
        };
        let uvs: Vec<[f32; 2]> = reader
            .read_tex_coords(tex_set)
            .map(|t| t.into_f32().collect())
            .unwrap_or_default();
        let uvs = if uvs.len() == n { uvs } else { Vec::new() };
        let raw: Vec<u32> = match reader.read_indices() {
            Some(i) => i.into_u32().collect(),
            None => (0..n as u32).collect(),
        };
        let indices = match mode {
            Mode::TriangleStrip => (2..raw.len())
                .flat_map(|i| {
                    if i % 2 == 0 {
                        [raw[i - 2], raw[i - 1], raw[i]]
                    } else {
                        [raw[i - 1], raw[i - 2], raw[i]]
                    }
                })
                .collect(),
            Mode::TriangleFan => (2..raw.len())
                .flat_map(|i| [raw[0], raw[i - 1], raw[i]])
                .collect(),
            _ => {
                let mut r = raw;
                r.truncate(r.len() / 3 * 3);
                r
            }
        };
        self.scene.primitives.push(Primitive {
            positions,
            normals,
            uvs,
            indices,
            material: material_index,
        });
        Ok(())
    }

    fn material(&mut self, m: &gltf::Material<'_>) -> Result<usize, ImportError> {
        if let Some(&i) = self.materials.get(&m.index()) {
            return Ok(i);
        }
        let pbr = m.pbr_metallic_roughness();
        let base_texture = match pbr.base_color_texture() {
            Some(info) => {
                self.texture(info.texture().source().index(), &info.texture().source())?
            }
            None => None,
        };
        let metal_rough_texture = match pbr.metallic_roughness_texture() {
            Some(info) => {
                self.texture(info.texture().source().index(), &info.texture().source())?
            }
            None => None,
        };
        let occlusion_texture = match m.occlusion_texture() {
            Some(info) => {
                self.texture(info.texture().source().index(), &info.texture().source())?
            }
            None => None,
        };
        let emissive_texture = match m.emissive_texture() {
            Some(info) => {
                self.texture(info.texture().source().index(), &info.texture().source())?
            }
            None => None,
        };
        let alpha_mode = match m.alpha_mode() {
            gltf::material::AlphaMode::Opaque => AlphaMode::Opaque,
            gltf::material::AlphaMode::Mask => AlphaMode::Mask(m.alpha_cutoff().unwrap_or(0.5)),
            gltf::material::AlphaMode::Blend => AlphaMode::Blend,
        };
        let material = Material {
            base_color: pbr.base_color_factor(),
            base_texture,
            metal_rough_texture,
            occlusion_texture,
            emissive_texture,
            metallic: pbr.metallic_factor(),
            roughness: pbr.roughness_factor(),
            emissive: m.emissive_factor(),
            alpha_mode,
            double_sided: m.double_sided(),
        };
        if self.scene.materials.len() >= self.limits.scene.max_materials {
            return Err(ImportError::Scene(SceneError::TooManyMaterials(
                self.scene.materials.len() + 1,
            )));
        }
        self.scene.materials.push(material);
        let i = self.scene.materials.len() - 1;
        self.materials.insert(m.index(), i);
        Ok(i)
    }

    /// Decodes an embedded PNG or JPEG image once; failures become a warning.
    fn texture(
        &mut self,
        index: usize,
        image: &gltf::Image<'_>,
    ) -> Result<Option<usize>, ImportError> {
        if let Some(&t) = self.textures.get(&index) {
            return Ok(t);
        }
        let bytes = match image.source() {
            gltf::image::Source::View { view, .. } => {
                let start = view.offset();
                &self.blob[start..start + view.length()]
            }
            gltf::image::Source::Uri { .. } => return Err(ImportError::ExternalReference),
        };
        let decoded = decode(bytes, self.limits.scene.max_texture_side);
        let slot = match decoded {
            Some(t) if self.scene.textures.len() < self.limits.scene.max_textures => {
                self.scene.textures.push(t);
                Some(self.scene.textures.len() - 1)
            }
            _ => {
                if !self.warnings.contains(&ImportWarning::TextureSkipped) {
                    self.warnings.push(ImportWarning::TextureSkipped);
                }
                None
            }
        };
        self.textures.insert(index, slot);
        Ok(slot)
    }
}

fn decode(bytes: &[u8], max_side: u32) -> Option<Texture> {
    let mut reader = image::ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .ok()?;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(max_side);
    limits.max_image_height = Some(max_side);
    limits.max_alloc = Some(512 * 1024 * 1024);
    reader.limits(limits);
    let img = reader.decode().ok()?.to_rgba8();
    Some(Texture {
        width: img.width(),
        height: img.height(),
        rgba: img.into_raw(),
    })
}
