//! Spec 100 §3.1 and §9: importing GLB and OBJ as untrusted data.

use gafira_io_model3d::{
    ImportError, ImportLimits, ImportWarning, ModelFormat, detect_format, import,
};

/// A GLB with one triangle, one red material and the given extra JSON members.
fn glb(extra: &str, material: &str) -> Vec<u8> {
    glb_with_buffer(extra, material, "")
}

fn glb_with_buffer(extra: &str, material: &str, buffer_extra: &str) -> Vec<u8> {
    let positions: [f32; 9] = [0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0];
    let mut bin: Vec<u8> = positions.iter().flat_map(|v| v.to_le_bytes()).collect();
    let indices: [u16; 3] = [0, 1, 2];
    bin.extend(indices.iter().flat_map(|v| v.to_le_bytes()));
    while !bin.len().is_multiple_of(4) {
        bin.push(0);
    }
    let json = format!(
        r#"{{"asset":{{"version":"2.0"}},{extra}"scene":0,"scenes":[{{"nodes":[0]}}],"nodes":[{{"mesh":0,"translation":[5,0,0]}}],
        "meshes":[{{"primitives":[{{"attributes":{{"POSITION":0}},"indices":1,"material":0}}]}}],
        "materials":[{material}],
        "buffers":[{{"byteLength":{len}{buffer_extra}}}],
        "bufferViews":[{{"buffer":0,"byteOffset":0,"byteLength":36}},{{"buffer":0,"byteOffset":36,"byteLength":6}}],
        "accessors":[{{"bufferView":0,"componentType":5126,"count":3,"type":"VEC3","min":[0,0,0],"max":[1,1,0]}},
                     {{"bufferView":1,"componentType":5123,"count":3,"type":"SCALAR"}}]}}"#,
        len = bin.len()
    );
    let mut json = json.into_bytes();
    while !json.len().is_multiple_of(4) {
        json.push(b' ');
    }
    let total = 12 + 8 + json.len() + 8 + bin.len();
    let mut out = Vec::new();
    out.extend(b"glTF");
    out.extend(2u32.to_le_bytes());
    out.extend((total as u32).to_le_bytes());
    out.extend((json.len() as u32).to_le_bytes());
    out.extend(b"JSON");
    out.extend(&json);
    out.extend((bin.len() as u32).to_le_bytes());
    out.extend(b"BIN\0");
    out.extend(&bin);
    out
}

const RED: &str = r#"{"pbrMetallicRoughness":{"baseColorFactor":[1,0,0,1],"metallicFactor":0.2,"roughnessFactor":0.4},"doubleSided":true}"#;

const CUBE_OBJ: &str = "mtllib cube.mtl\nv -1 -1 -1\nv 1 -1 -1\nv 1 1 -1\nv -1 1 -1\nv -1 -1 1\nv 1 -1 1\nv 1 1 1\nv -1 1 1\nusemtl red\nf 1 2 3 4\nf 5 6 7 8\nf 1 2 6 5\nf 2 3 7 6\nf 3 4 8 7\nf 4 1 5 8\n";

// Spec 100 §9: the format comes from the content, not only the extension.
#[test]
fn s100_format_is_detected_from_content() {
    assert_eq!(
        detect_format(&glb("", RED), "x.obj"),
        Some(ModelFormat::Glb)
    );
    assert_eq!(
        detect_format(CUBE_OBJ.as_bytes(), "model.OBJ"),
        Some(ModelFormat::Obj)
    );
    assert_eq!(detect_format(b"\x89PNG\r\n\x1a\n....", "x.glb"), None);
    assert_eq!(detect_format(b"hello world", "notes.txt"), None);
}

// Spec 100 AC-01: a valid GLB imports with its material, normalized to the unit sphere.
#[test]
fn s100_glb_imports_with_its_material() {
    let out = import(
        &glb("", RED),
        ModelFormat::Glb,
        &ImportLimits::default(),
        &|| false,
    )
    .expect("valid");
    assert_eq!(out.scene.triangle_count(), 1);
    let m = &out.scene.materials[out.scene.primitives[0].material];
    assert_eq!(m.base_color, [1.0, 0.0, 0.0, 1.0]);
    assert!((m.metallic - 0.2).abs() < 1e-6 && (m.roughness - 0.4).abs() < 1e-6);
    // The node translation does not matter after normalization: centred, radius 1.
    let (min, max) = out.scene.bounds().expect("bounds");
    let centre: Vec<f32> = (0..3).map(|k| (min[k] + max[k]) / 2.0).collect();
    assert!(centre.iter().all(|c| c.abs() < 1e-5), "{centre:?}");
    assert!(out.warnings.is_empty());
}

// Spec 100 AC-02: an OBJ without its MTL imports with the neutral material and a warning.
#[test]
fn s100_obj_without_mtl_uses_the_neutral_material() {
    let out = import(
        CUBE_OBJ.as_bytes(),
        ModelFormat::Obj,
        &ImportLimits::default(),
        &|| false,
    )
    .expect("valid");
    assert_eq!(out.scene.triangle_count(), 12);
    assert_eq!(
        out.scene.materials,
        vec![gafira_model3d::Material::neutral()]
    );
    assert_eq!(out.warnings, vec![ImportWarning::MissingMaterials]);
}

// Spec 100 §3.1: compressed GLB extensions are refused clearly when they are required.
#[test]
fn s100_required_unsupported_extensions_are_refused() {
    let draco = glb(
        r#""extensionsUsed":["KHR_draco_mesh_compression"],"extensionsRequired":["KHR_draco_mesh_compression"],"#,
        RED,
    );
    match import(&draco, ModelFormat::Glb, &ImportLimits::default(), &|| {
        false
    }) {
        Err(ImportError::UnsupportedExtension(e)) => assert_eq!(e, "KHR_draco_mesh_compression"),
        other => panic!("{other:?}"),
    }
    let optional = glb(r#""extensionsUsed":["KHR_materials_clearcoat"],"#, RED);
    let out = import(
        &optional,
        ModelFormat::Glb,
        &ImportLimits::default(),
        &|| false,
    )
    .expect("optional");
    assert_eq!(
        out.warnings,
        vec![ImportWarning::IgnoredExtension(
            "KHR_materials_clearcoat".into()
        )]
    );
}

// Spec 100 §9, AC-15: damaged, oversized, cancelled and degenerate inputs fail cleanly.
#[test]
fn s100_bad_inputs_fail_cleanly() {
    let limits = ImportLimits::default();
    let mut truncated = glb("", RED);
    truncated.truncate(40);
    assert!(matches!(
        import(&truncated, ModelFormat::Glb, &limits, &|| false),
        Err(ImportError::Damaged(_))
    ));
    let mut wrong_length = glb("", RED);
    wrong_length[8] = wrong_length[8].wrapping_add(4);
    assert!(matches!(
        import(&wrong_length, ModelFormat::Glb, &limits, &|| false),
        Err(ImportError::Damaged(_))
    ));
    let tiny = ImportLimits {
        max_file_bytes: 10,
        ..limits
    };
    assert!(matches!(
        import(CUBE_OBJ.as_bytes(), ModelFormat::Obj, &tiny, &|| false),
        Err(ImportError::TooLarge { .. })
    ));
    assert_eq!(
        import(CUBE_OBJ.as_bytes(), ModelFormat::Obj, &limits, &|| true).err(),
        Some(ImportError::Cancelled)
    );
    let nan = "v nan 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n";
    assert!(matches!(
        import(nan.as_bytes(), ModelFormat::Obj, &limits, &|| false),
        Err(ImportError::Scene(_))
    ));
    let few = ImportLimits {
        scene: gafira_model3d::Limits {
            max_triangles: 3,
            ..Default::default()
        },
        ..limits
    };
    assert!(matches!(
        import(CUBE_OBJ.as_bytes(), ModelFormat::Obj, &few, &|| false),
        Err(ImportError::Scene(_))
    ));
    assert!(matches!(
        import(b"not a model", ModelFormat::Obj, &limits, &|| false),
        Err(ImportError::Scene(_)) | Err(ImportError::Damaged(_))
    ));
}

// Spec 100 §9: buffers may not point outside the file.
#[test]
fn s100_external_buffers_are_refused() {
    let bytes = glb_with_buffer("", RED, r#","uri":"../../etc/x.bin""#);
    assert!(matches!(
        import(&bytes, ModelFormat::Glb, &ImportLimits::default(), &|| {
            false
        }),
        Err(ImportError::ExternalReference)
    ));
}
