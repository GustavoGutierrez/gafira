//! Renders a `.glb` or `.obj` to a PNG for a visual check (spec 100):
//! `cargo run --release -p gafira-io-model3d --example render3d -- <model> <out.png> [size] [preset] [lighting]`.

use std::time::Instant;

use gafira_io_model3d::{ImportLimits, detect_format, import};
use gafira_model3d::{Camera, Lighting, Pose, RenderParams, ViewPreset, render};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let (input, output) = (&args[1], &args[2]);
    let size: u32 = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(512);
    let preset = match args.get(4).map(String::as_str) {
        Some("front") => ViewPreset::Front,
        Some("left") => ViewPreset::Left,
        Some("top") => ViewPreset::Top,
        _ => ViewPreset::Isometric,
    };
    let lighting = args
        .get(5)
        .map_or(Lighting::Studio, |s| Lighting::from_id(s));
    let bytes = std::fs::read(input)?;
    let format = detect_format(&bytes, input).ok_or("unknown format")?;
    let t = Instant::now();
    let model = import(&bytes, format, &ImportLimits::default(), &|| false)?;
    println!(
        "import {:?}: {} triangles, {} textures, warnings {:?}",
        t.elapsed(),
        model.scene.triangle_count(),
        model.scene.textures.len(),
        model.warnings
    );
    let params = RenderParams {
        pose: Pose::default(),
        camera: Camera::default().with_preset(preset),
        lighting,
        width: size,
        height: size,
        samples: 2,
    };
    let t = Instant::now();
    let rgba = render(&model.scene, &params);
    println!("render {:?}", t.elapsed());
    image::save_buffer(output, &rgba, size, size, image::ColorType::Rgba8)?;
    Ok(())
}
