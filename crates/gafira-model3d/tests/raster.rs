//! Spec 100: the CPU rasterizer (D-130). Transparent background, determinism, bands, limits.

use gafira_model3d::{
    AlphaMode, Camera, Lighting, Limits, Material, Pose, Primitive, RenderParams, Scene,
    SceneError, Texture, ViewPreset, render,
};

/// A cube from -1 to 1 (before normalization), one primitive, one material.
fn cube(material: Material) -> Scene {
    cube_with(material, Vec::new())
}

fn cube_with(material: Material, textures: Vec<Texture>) -> Scene {
    let mut positions = Vec::new();
    let mut indices = Vec::new();
    let faces: [([f32; 3], [f32; 3], [f32; 3]); 6] = [
        ([0.0, 0.0, 1.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]),
        ([0.0, 0.0, -1.0], [-1.0, 0.0, 0.0], [0.0, 1.0, 0.0]),
        ([1.0, 0.0, 0.0], [0.0, 0.0, -1.0], [0.0, 1.0, 0.0]),
        ([-1.0, 0.0, 0.0], [0.0, 0.0, 1.0], [0.0, 1.0, 0.0]),
        ([0.0, 1.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, -1.0]),
        ([0.0, -1.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]),
    ];
    for (n, u, v) in faces {
        let base = positions.len() as u32;
        for (su, sv) in [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)] {
            positions.push([0, 1, 2].map(|k| n[k] + su * u[k] + sv * v[k]));
        }
        indices.extend([base, base + 1, base + 2, base, base + 2, base + 3]);
    }
    let uvs = (0..positions.len())
        .map(|i| [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]][i % 4])
        .collect();
    Scene {
        primitives: vec![Primitive {
            positions,
            normals: Vec::new(),
            uvs,
            indices,
            material: 0,
        }],
        materials: vec![material],
        textures,
    }
    .normalized(&Limits::default())
    .expect("valid cube")
}

fn params(w: u32, h: u32) -> RenderParams {
    RenderParams {
        pose: Pose::default(),
        camera: Camera::default().with_preset(ViewPreset::Isometric),
        lighting: Lighting::Studio,
        width: w,
        height: h,
        samples: 2,
    }
}

fn alpha(img: &[u8], w: u32, x: u32, y: u32) -> u8 {
    img[((y * w + x) * 4 + 3) as usize]
}

// Spec 100 §7, AC-12: the background is transparent; the model is opaque.
#[test]
fn s100_render_has_a_transparent_background() {
    let scene = cube(Material::neutral());
    let img = render(&scene, &params(120, 90));
    assert_eq!(img.len(), 120 * 90 * 4);
    for (x, y) in [(0, 0), (119, 0), (0, 89), (119, 89)] {
        assert_eq!(alpha(&img, 120, x, y), 0, "corner {x},{y}");
    }
    assert_eq!(alpha(&img, 120, 60, 45), 255);
    // Straight alpha: transparent pixels carry no colour.
    assert!(
        img.chunks_exact(4)
            .filter(|p| p[3] == 0)
            .all(|p| p[0] == 0 && p[1] == 0 && p[2] == 0)
    );
}

// Spec 100 §8.2: the same version and inputs give the same pixels; bands equal the whole image.
#[test]
fn s100_render_is_deterministic_and_banded() {
    let scene = cube(Material::neutral());
    let p = params(97, 61);
    let a = render(&scene, &p);
    let b = render(&scene, &p);
    assert_eq!(a, b);
    let prepared = gafira_model3d::Prepared::new(&scene, &p);
    let mut joined = Vec::new();
    for i in 0..prepared.band_count() {
        joined.extend(prepared.render_band(i));
    }
    assert_eq!(joined, a);
}

// Spec 100 §7: edges are antialiased (partial alpha exists with several samples).
#[test]
fn s100_edges_are_smoothed() {
    let scene = cube(Material::neutral());
    let img = render(&scene, &params(80, 80));
    assert!(img.chunks_exact(4).any(|p| p[3] > 0 && p[3] < 255));
}

// Spec 100 §3.4: zoom grows the opaque area; the model scale does too, independently.
#[test]
fn s100_zoom_and_scale_change_the_coverage() {
    let scene = cube(Material::neutral());
    let cover = |p: &RenderParams| {
        render(&scene, p)
            .chunks_exact(4)
            .filter(|px| px[3] > 127)
            .count()
    };
    let base = params(64, 64);
    let zoomed = RenderParams {
        camera: Camera {
            zoom: 0.5,
            ..base.camera
        },
        ..base
    };
    let scaled = RenderParams {
        pose: Pose {
            scale: [0.5; 3],
            ..base.pose
        },
        ..base
    };
    let c0 = cover(&base);
    assert!(cover(&zoomed) < c0 / 2);
    assert!(cover(&scaled) < c0 / 2);
}

// Spec 100 §3.4 "Mover": moving the model shifts its pixels inside the frame.
#[test]
fn s100_position_moves_the_model_in_its_frame() {
    let scene = cube(Material::neutral());
    let base = RenderParams {
        camera: Camera::default().with_preset(ViewPreset::Front),
        ..params(100, 100)
    };
    let moved = RenderParams {
        pose: Pose {
            position: [0.5, 0.0, 0.0],
            scale: [0.3; 3],
            ..Pose::default()
        },
        ..base
    };
    let img = render(&scene, &moved);
    // Opaque right of the centre, transparent left of it.
    assert_eq!(alpha(&img, 100, 72, 50), 255);
    assert_eq!(alpha(&img, 100, 30, 50), 0);
}

// Spec 100 §6.1: base colour texture and the Mask alpha mode cut holes.
#[test]
fn s100_mask_textures_cut_holes() {
    // 2x2 texture: left column transparent, right opaque red.
    let tex = Texture {
        width: 2,
        height: 2,
        rgba: vec![0, 0, 0, 0, 255, 0, 0, 255, 0, 0, 0, 0, 255, 0, 0, 255],
    };
    let mut m = Material::neutral();
    m.base_texture = Some(0);
    m.alpha_mode = AlphaMode::Mask(0.5);
    let scene = cube_with(m, vec![tex]);
    let p = RenderParams {
        camera: Camera::default().with_preset(ViewPreset::Front),
        ..params(100, 100)
    };
    let img = render(&scene, &p);
    // The front face shows the texture: some pixels are see-through where the back faces are
    // also masked, and the opaque ones are red.
    let opaque: Vec<&[u8]> = img.chunks_exact(4).filter(|px| px[3] == 255).collect();
    assert!(!opaque.is_empty());
    assert!(opaque.iter().all(|px| px[0] > px[1] && px[0] > px[2]));
}

// Spec 100 §7 / lighting presets: six different pictures; Dark is the darkest and Studio is
// brighter than Soft; Warm is redder and Cool bluer than Studio.
#[test]
fn s100_lighting_presets_differ() {
    let scene = cube(Material::neutral());
    let sums = |l: Lighting| {
        let img = render(
            &scene,
            &RenderParams {
                lighting: l,
                ..params(64, 64)
            },
        );
        let mut rgb = [0u64; 3];
        for p in img.chunks_exact(4).filter(|p| p[3] == 255) {
            for k in 0..3 {
                rgb[k] += u64::from(p[k]);
            }
        }
        rgb
    };
    let lum = |l: Lighting| sums(l).iter().sum::<u64>();
    let all: Vec<u64> = Lighting::ALL.iter().map(|l| lum(*l)).collect();
    for (i, a) in all.iter().enumerate() {
        for b in &all[i + 1..] {
            assert_ne!(a, b, "two presets give the same picture");
        }
    }
    let darkest = Lighting::ALL.iter().min_by_key(|l| lum(**l)).copied();
    assert_eq!(darkest, Some(Lighting::Dark));
    assert!(lum(Lighting::Studio) > lum(Lighting::Soft));
    let (studio, warm, cool) = (
        sums(Lighting::Studio),
        sums(Lighting::Warm),
        sums(Lighting::Cool),
    );
    let ratio = |s: [u64; 3]| s[0] as f64 / s[2] as f64;
    assert!(ratio(warm) > ratio(studio) && ratio(cool) < ratio(studio));
    assert_eq!(Lighting::from_id("dramatic"), Lighting::Dark);
}

// Spec 100 §9: damaged or empty scenes are refused, not drawn.
#[test]
fn s100_invalid_scenes_are_refused() {
    let empty = Scene {
        primitives: vec![],
        materials: vec![Material::neutral()],
        textures: vec![],
    };
    assert_eq!(empty.normalized(&Limits::default()), Err(SceneError::Empty));
    let mut bad = cube(Material::neutral());
    bad.primitives[0].indices.push(9999);
    bad.primitives[0].indices.extend([0, 1]);
    assert_eq!(
        bad.validate(&Limits::default()),
        Err(SceneError::BadIndices)
    );
    let mut nan = cube(Material::neutral());
    nan.primitives[0].positions[0][0] = f32::NAN;
    assert_eq!(nan.validate(&Limits::default()), Err(SceneError::NonFinite));
    let flat = Scene {
        primitives: vec![Primitive {
            positions: vec![[1.0; 3]; 3],
            indices: vec![0, 1, 2],
            ..Primitive::default()
        }],
        materials: vec![Material::neutral()],
        textures: vec![],
    };
    assert_eq!(
        flat.normalized(&Limits::default()),
        Err(SceneError::Degenerate)
    );
    let small = Limits {
        max_triangles: 4,
        ..Limits::default()
    };
    assert_eq!(
        cube(Material::neutral()).validate(&small),
        Err(SceneError::TooManyTriangles(12))
    );
}

// Spec 100 §8.2: the size is clamped to the renderer's limit, never unbounded.
#[test]
fn s100_render_size_is_bounded() {
    let p = RenderParams {
        width: 100_000,
        height: 0,
        ..params(1, 1)
    }
    .clamped();
    assert!(p.width <= RenderParams::MAX_SIDE && p.height >= 1);
}
