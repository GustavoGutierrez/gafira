//! OBJ reading with `tobj` (spec 100 §3.1). Phase 1 imports the geometry only: a referenced
//! `.mtl` is never opened, and the model gets the neutral material with a warning.

use std::io::BufReader;

use gafira_model3d::{Material, Primitive, Scene, SceneError};

use crate::{ImportError, ImportLimits, ImportWarning, budget};

const KNOWN: [&str; 13] = [
    "#", "v", "vt", "vn", "vp", "f", "o", "g", "s", "l", "p", "mtllib", "usemtl",
];

/// True when the start of the text reads as OBJ statements, with a vertex among them.
pub(crate) fn looks_like_obj(bytes: &[u8], name: &str) -> bool {
    let head = &bytes[..bytes.len().min(16 * 1024)];
    let text = match std::str::from_utf8(head) {
        Ok(t) => t,
        // The cut may split a character: keep the valid part.
        Err(e) if e.valid_up_to() > 0 && bytes.len() > head.len() => {
            match std::str::from_utf8(&head[..e.valid_up_to()]) {
                Ok(t) => t,
                Err(_) => return false,
            }
        }
        Err(_) => return false,
    };
    let mut vertices = 0;
    let mut statements = 0;
    let mut unknown = 0;
    for line in text.lines().take(400) {
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        let word = t.split_whitespace().next().unwrap_or("");
        let word = if word.starts_with('#') { "#" } else { word };
        if KNOWN.contains(&word) {
            statements += 1;
            if word == "v" {
                vertices += 1;
            }
        } else {
            unknown += 1;
        }
    }
    let by_name = name.to_ascii_lowercase().ends_with(".obj");
    vertices > 0 && (unknown == 0 || by_name && unknown * 4 < statements)
}

pub(crate) fn read(
    bytes: &[u8],
    limits: &ImportLimits,
    cancelled: &dyn Fn() -> bool,
) -> Result<(Scene, Vec<ImportWarning>), ImportError> {
    let options = tobj::LoadOptions {
        triangulate: true,
        single_index: true,
        ignore_points: true,
        ignore_lines: true,
    };
    let mut reader = BufReader::new(bytes);
    let (models, materials) = tobj::load_obj_buf(&mut reader, &options, |_| {
        Err(tobj::LoadError::OpenFileFailed)
    })
    .map_err(|e| ImportError::Damaged(e.to_string()))?;
    let mut warnings = Vec::new();
    if materials.is_err() {
        warnings.push(ImportWarning::MissingMaterials);
    }
    let mut vertices = 0;
    let mut triangles = 0;
    let mut primitives = Vec::with_capacity(models.len());
    for model in models {
        if cancelled() {
            return Err(ImportError::Cancelled);
        }
        let m = model.mesh;
        budget(
            &mut vertices,
            m.positions.len() / 3,
            limits.scene.max_vertices,
            SceneError::TooManyVertices,
        )?;
        budget(
            &mut triangles,
            m.indices.len() / 3,
            limits.scene.max_triangles,
            SceneError::TooManyTriangles,
        )?;
        if m.indices.is_empty() {
            continue;
        }
        let n = m.positions.len() / 3;
        let positions: Vec<[f32; 3]> = m
            .positions
            .chunks_exact(3)
            .map(|c| [c[0], c[1], c[2]])
            .collect();
        let normals = if m.normals.len() == n * 3 {
            m.normals
                .chunks_exact(3)
                .map(|c| [c[0], c[1], c[2]])
                .collect()
        } else {
            Vec::new()
        };
        // OBJ puts v = 0 at the bottom; the renderer, like glTF, at the top.
        let uvs = if m.texcoords.len() == n * 2 {
            m.texcoords
                .chunks_exact(2)
                .map(|c| [c[0], 1.0 - c[1]])
                .collect()
        } else {
            Vec::new()
        };
        primitives.push(Primitive {
            positions,
            normals,
            uvs,
            indices: m.indices,
            material: 0,
        });
    }
    Ok((
        Scene {
            primitives,
            materials: vec![Material::neutral()],
            textures: Vec::new(),
        },
        warnings,
    ))
}
