//! `gen-icons`: the application and `.gaf` document icons (03 §5.3, DIS-13).
//!
//! Every icon comes from the brand SVGs through this code, never from a hand
//! export: the application tile is composed as SVG text from the symbol and
//! rasterized with resvg at each Linux `hicolor` size; the `.gaf` document
//! icon is the brand master, copied and rasterized the same way. The same
//! inputs give the same bytes, so the tracked files can be checked.

use std::path::Path;

use resvg::tiny_skia::{Pixmap, Transform};
use resvg::usvg::{self, roxmltree};

/// The Linux `hicolor` sizes of 03 §5.3, in pixels.
pub const SIZES: [u32; 8] = [16, 24, 32, 48, 64, 128, 256, 512];

/// The application symbol (the source of the tile).
pub const SYMBOL_SOURCE: &str = "assets/gafira-logo-symbol.svg";
/// The `.gaf` document icon of the brand manual (§7.12).
pub const DOCUMENT_SOURCE: &str = "docs/manuals/marca/assets/gafira-gaf-icon.svg";
/// Where the generated `hicolor` tree lives, relative to the repository root.
pub const OUT_DIR: &str = "assets/app-icon/hicolor";
/// The Windows and macOS icons, relative to the repository root: of the
/// application, and of a `.gaf` file (spec 090).
pub const WINDOWS_APP: &str = "assets/app-icon/windows/gafira.ico";
pub const WINDOWS_DOCUMENT: &str = "assets/app-icon/windows/gafira-project.ico";
pub const MACOS_APP: &str = "assets/app-icon/macos/gafira.icns";
pub const MACOS_DOCUMENT: &str = "assets/app-icon/macos/gafira-project.icns";
/// The sizes of an `.ico`, in pixels.
pub const ICO_SIZES: [u32; 7] = [16, 24, 32, 48, 64, 128, 256];
/// The sizes of an `.icns`, in pixels, and the kind of each.
const ICNS_SIZES: [(u32, &[u8; 4]); 5] = [
    (128, b"ic07"),
    (256, b"ic08"),
    (512, b"ic09"),
    (1024, b"ic10"),
    (64, b"ic12"),
];
/// The icon name of the application (`Icon=` of the desktop entry).
pub const APP_ICON: &str = "gafira";
/// The icon name of the `.gaf` MIME type `application/vnd.gafira.project+zip`
/// (freedesktop: the `/` becomes `-`).
pub const DOCUMENT_ICON: &str = "application-vnd.gafira.project+zip";

/// The side of the tile SVG, in user units.
const SIDE: f64 = 512.0;
/// The share of the tile side the symbol occupies (03 §5.3: 62 %).
const SYMBOL_SHARE: f64 = 0.62;
/// The corner radius of the tile: 3/16 of the side (03 §5.3 fixes only
/// "rounded"; recorded as an open visual decision in spec 046).
const CORNER_RADIUS: f64 = SIDE * 3.0 / 16.0;
/// The tile color, `brand.canvas` (03 §3, §5.3).
const TILE_COLOR: &str = "#FAFAF7";

/// One generated file: its path relative to the repository root and its bytes.
#[derive(Debug, PartialEq, Eq)]
pub struct Output {
    pub path: String,
    pub bytes: Vec<u8>,
}

/// Generate every icon file from the brand SVGs under `root`.
///
/// # Errors
/// When a source cannot be read, is not the SVG shape the generator expects,
/// or cannot be rasterized.
pub fn generate(root: &Path) -> Result<Vec<Output>, String> {
    let symbol = read(root, SYMBOL_SOURCE)?;
    let tile = tile_svg(&symbol)?;
    let document = read(root, DOCUMENT_SOURCE)?;
    let mut out = Vec::new();
    for (svg, windows, macos) in [
        (&tile, WINDOWS_APP, MACOS_APP),
        (&document, WINDOWS_DOCUMENT, MACOS_DOCUMENT),
    ] {
        let tree = usvg::Tree::from_data(svg.as_bytes(), &usvg::Options::default())
            .map_err(|e| format!("{windows}: {e}"))?;
        out.push(Output {
            path: windows.to_owned(),
            bytes: ico(&tree)?,
        });
        out.push(Output {
            path: macos.to_owned(),
            bytes: icns(&tree)?,
        });
    }
    push_theme(&mut out, "apps", APP_ICON, tile.into_bytes())?;
    push_theme(&mut out, "mimetypes", DOCUMENT_ICON, document.into_bytes())?;
    Ok(out)
}

fn read(root: &Path, relative: &str) -> Result<String, String> {
    std::fs::read_to_string(root.join(relative)).map_err(|e| format!("{relative}: {e}"))
}

/// The scalable SVG and one PNG per size of `SIZES` for one icon name.
fn push_theme(
    out: &mut Vec<Output>,
    context: &str,
    name: &str,
    svg: Vec<u8>,
) -> Result<(), String> {
    let tree = usvg::Tree::from_data(&svg, &usvg::Options::default())
        .map_err(|e| format!("{name}.svg: {e}"))?;
    for size in SIZES {
        out.push(Output {
            path: format!("{OUT_DIR}/{size}x{size}/{context}/{name}.png"),
            bytes: render(&tree, size)?,
        });
    }
    out.push(Output {
        path: format!("{OUT_DIR}/scalable/{context}/{name}.svg"),
        bytes: svg,
    });
    Ok(())
}

/// `tree` as an `.ico`: the small sizes as bitmaps, which every tool reads,
/// and the largest as a PNG.
fn ico(tree: &usvg::Tree) -> Result<Vec<u8>, String> {
    let images = ICO_SIZES
        .iter()
        .map(|size| match size {
            256 => render(tree, *size),
            _ => Ok(bitmap(&pixels(tree, *size)?)),
        })
        .collect::<Result<Vec<_>, String>>()?;
    let count = u16::try_from(images.len()).map_err(|e| e.to_string())?;
    let mut out = vec![0, 0, 1, 0];
    out.extend(count.to_le_bytes());
    let mut at = 6 + 16 * images.len();
    for (size, image) in ICO_SIZES.iter().zip(&images) {
        // 256 is written as 0.
        let side = u8::try_from(*size).unwrap_or(0);
        out.extend([side, side, 0, 0, 1, 0, 32, 0]);
        for number in [image.len(), at] {
            let number = u32::try_from(number).map_err(|e| e.to_string())?;
            out.extend(number.to_le_bytes());
        }
        at += image.len();
    }
    out.extend(images.concat());
    Ok(out)
}

/// The bitmap of an `.ico`: its header, its straight pixels from the bottom
/// row up and a mask of nothing, as the opacity says it all.
fn bitmap(pixmap: &Pixmap) -> Vec<u8> {
    let (w, h) = (pixmap.width(), pixmap.height());
    let mask = (w.div_ceil(32) * 4 * h) as usize;
    let mut out = Vec::new();
    for number in [40, w, h * 2] {
        out.extend(number.to_le_bytes());
    }
    out.extend([1, 0, 32, 0]);
    out.extend([0; 24]);
    for y in (0..h).rev() {
        for x in 0..w {
            let p = pixmap.pixel(x, y).map(|p| p.demultiply());
            out.extend(p.map_or([0; 4], |p| [p.blue(), p.green(), p.red(), p.alpha()]));
        }
    }
    out.extend(vec![0; mask]);
    out
}

/// `tree` as an `.icns`.
fn icns(tree: &usvg::Tree) -> Result<Vec<u8>, String> {
    let mut body = Vec::new();
    let length = |n: usize| u32::try_from(n + 8).map_err(|e| e.to_string());
    for (size, kind) in ICNS_SIZES {
        let png = render(tree, size)?;
        body.extend(kind);
        body.extend(length(png.len())?.to_be_bytes());
        body.extend(png);
    }
    let mut out = b"icns".to_vec();
    out.extend(length(body.len())?.to_be_bytes());
    out.extend(body);
    Ok(out)
}

/// Rasterize `tree` into a `size` × `size` PNG.
fn render(tree: &usvg::Tree, size: u32) -> Result<Vec<u8>, String> {
    pixels(tree, size)?
        .encode_png()
        .map_err(|e| format!("png of {size} px: {e}"))
}

/// Rasterize `tree` into `size` × `size` pixels.
fn pixels(tree: &usvg::Tree, size: u32) -> Result<Pixmap, String> {
    let mut pixmap = Pixmap::new(size, size).ok_or_else(|| format!("no pixmap of {size} px"))?;
    let scale = size as f32;
    let transform =
        Transform::from_scale(scale / tree.size().width(), scale / tree.size().height());
    resvg::render(tree, transform, &mut pixmap.as_mut());
    Ok(pixmap)
}

/// The application tile (03 §5.3): a rounded `#FAFAF7` square with the
/// symbol's paths centred at 62 % of the side.
///
/// # Errors
/// When the symbol is not a plain SVG of filled paths over a `0 0 w h` view box.
pub fn tile_svg(symbol: &str) -> Result<String, String> {
    let doc = roxmltree::Document::parse(symbol).map_err(|e| format!("{SYMBOL_SOURCE}: {e}"))?;
    let svg = doc.root_element();
    let view_box = svg
        .attribute("viewBox")
        .ok_or_else(|| format!("{SYMBOL_SOURCE}: no viewBox"))?;
    let numbers: Vec<f64> = view_box
        .split([' ', ','])
        .filter(|s| !s.is_empty())
        .map(str::parse)
        .collect::<Result<_, _>>()
        .map_err(|e| format!("{SYMBOL_SOURCE}: viewBox {view_box}: {e}"))?;
    let [min_x, min_y, width, height] = numbers[..] else {
        return Err(format!("{SYMBOL_SOURCE}: viewBox {view_box}"));
    };
    if min_x != 0.0 || min_y != 0.0 || width <= 0.0 || height <= 0.0 {
        return Err(format!(
            "{SYMBOL_SOURCE}: viewBox {view_box} must start at 0 0"
        ));
    }
    let scale = SIDE * SYMBOL_SHARE / width.max(height);
    let dx = (SIDE - width * scale) / 2.0;
    let dy = (SIDE - height * scale) / 2.0;

    let mut paths = String::new();
    for node in svg.descendants().filter(roxmltree::Node::is_element) {
        match node.tag_name().name() {
            "svg" | "defs" | "namedview" | "title" | "desc" | "metadata" => {}
            "path" => {
                if node.attribute("transform").is_some() {
                    return Err(format!("{SYMBOL_SOURCE}: a path has a transform"));
                }
                let d = node
                    .attribute("d")
                    .ok_or_else(|| format!("{SYMBOL_SOURCE}: a path has no d"))?;
                let fill = property(node, "fill").unwrap_or("#000000");
                paths.push_str(&format!("    <path fill=\"{fill}\""));
                if let Some(rule) = property(node, "fill-rule") {
                    paths.push_str(&format!(" fill-rule=\"{rule}\""));
                }
                paths.push_str(&format!(" d=\"{d}\"/>\n"));
            }
            other => return Err(format!("{SYMBOL_SOURCE}: unexpected <{other}>")),
        }
    }
    if paths.is_empty() {
        return Err(format!("{SYMBOL_SOURCE}: no paths"));
    }
    Ok(format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{SIDE}\" height=\"{SIDE}\" \
         viewBox=\"0 0 {SIDE} {SIDE}\">\n  <title>Gafira</title>\n  \
         <rect width=\"{SIDE}\" height=\"{SIDE}\" rx=\"{CORNER_RADIUS}\" fill=\"{TILE_COLOR}\"/>\n  \
         <g transform=\"translate({dx} {dy}) scale({scale})\">\n{paths}  </g>\n</svg>\n"
    ))
}

/// A presentation property: the `style` declaration wins over the attribute
/// (CSS cascade), as resvg reads it.
fn property<'a>(node: roxmltree::Node<'a, '_>, name: &str) -> Option<&'a str> {
    let from_style = node.attribute("style").and_then(|style| {
        style.split(';').find_map(|decl| {
            let (key, value) = decl.split_once(':')?;
            (key.trim() == name).then_some(value.trim())
        })
    });
    from_style.or_else(|| node.attribute(name))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
    }

    fn file<'a>(out: &'a [Output], path: &str) -> &'a Output {
        out.iter()
            .find(|o| o.path == path)
            .unwrap_or_else(|| panic!("{path} not generated"))
    }

    // DIS-13: rebuilding the icons from scratch gives identical files.
    #[test]
    fn dis_13_generation_is_deterministic() {
        let first = generate(&root()).expect("generate");
        let second = generate(&root()).expect("generate");
        assert_eq!(first, second);
    }

    // DIS-13: the tracked icons are the generator's output (stable hash).
    #[test]
    fn dis_13_tracked_icons_match_the_generator() {
        for out in generate(&root()).expect("generate") {
            let tracked = std::fs::read(root().join(&out.path))
                .unwrap_or_else(|e| panic!("{}: {e}; run `cargo xtask gen-icons`", out.path));
            assert!(
                tracked == out.bytes,
                "{} is stale; run `cargo xtask gen-icons`",
                out.path
            );
        }
    }

    // DIS-13, 03 §5.3: every Linux size exists for both icons, with its own dimensions.
    #[test]
    fn dis_13_pngs_have_the_hicolor_sizes() {
        let out = generate(&root()).expect("generate");
        // And the Windows and macOS ones.
        assert_eq!(out.len(), 2 * (SIZES.len() + 1) + 4);
        for (context, name) in [("apps", APP_ICON), ("mimetypes", DOCUMENT_ICON)] {
            for size in SIZES {
                let png = file(
                    &out,
                    &format!("{OUT_DIR}/{size}x{size}/{context}/{name}.png"),
                );
                let pixmap = Pixmap::decode_png(&png.bytes).expect("decode");
                assert_eq!(
                    (pixmap.width(), pixmap.height()),
                    (size, size),
                    "{}",
                    png.path
                );
            }
            file(&out, &format!("{OUT_DIR}/scalable/{context}/{name}.svg"));
        }
    }

    // S90-02: an .ico has every size where it says, the small ones as bitmaps
    // with the pixels of the PNG, and an .icns every piece.
    #[test]
    fn s90_02_the_windows_and_macos_icons() {
        let out = generate(&root()).expect("generate");
        for (windows, macos, context, name) in [
            (WINDOWS_APP, MACOS_APP, "apps", APP_ICON),
            (WINDOWS_DOCUMENT, MACOS_DOCUMENT, "mimetypes", DOCUMENT_ICON),
        ] {
            let ico = &file(&out, windows).bytes;
            let number = |at: usize, n: usize| -> usize {
                let bytes = &ico[at..at + n];
                bytes.iter().rev().fold(0, |v, b| v << 8 | usize::from(*b))
            };
            assert_eq!(
                (number(0, 2), number(2, 2), number(4, 2)),
                (0, 1, ICO_SIZES.len())
            );
            let mut end = 6 + 16 * ICO_SIZES.len();
            for (i, size) in ICO_SIZES.iter().enumerate() {
                let entry = 6 + 16 * i;
                assert_eq!(number(entry, 1), *size as usize % 256);
                let (length, at) = (number(entry + 8, 4), number(entry + 12, 4));
                assert_eq!(at, end);
                end += length;
                if *size == 256 {
                    assert!(ico[at..].starts_with(b"\x89PNG"));
                    continue;
                }
                assert_eq!((number(at, 4), number(at + 8, 4)), (40, 2 * *size as usize));
                if *size == 48 {
                    let path = format!("{OUT_DIR}/48x48/{context}/{name}.png");
                    let png = Pixmap::decode_png(&file(&out, &path).bytes).expect("decode");
                    let want = png.pixel(24, 10).expect("pixel").demultiply();
                    let pixel = at + 40 + ((47 - 10) * 48 + 24) * 4;
                    let got = &ico[pixel..pixel + 4];
                    assert_eq!(got, [want.blue(), want.green(), want.red(), want.alpha()]);
                }
            }
            assert_eq!(end, ico.len());
            let icns = &file(&out, macos).bytes;
            assert!(icns.starts_with(b"icns"));
            let told = u32::from_be_bytes([icns[4], icns[5], icns[6], icns[7]]);
            assert_eq!(told as usize, icns.len());
        }
    }

    // 03 §5.3: the tile is #FAFAF7 with rounded corners and the symbol inside the central 62 %.
    #[test]
    fn tile_has_rounded_canvas_corners_and_the_symbol_at_62_percent() {
        let out = generate(&root()).expect("generate");
        let png = file(&out, &format!("{OUT_DIR}/512x512/apps/{APP_ICON}.png"));
        let pixmap = Pixmap::decode_png(&png.bytes).expect("decode");
        let at = |x: u32, y: u32| {
            let p = pixmap.pixel(x, y).expect("pixel").demultiply();
            [p.red(), p.green(), p.blue(), p.alpha()]
        };
        assert_eq!(at(0, 0)[3], 0, "the corner is cut by the radius");
        assert_eq!(
            at(256, 4),
            [0xFA, 0xFA, 0xF7, 0xFF],
            "the tile is brand.canvas"
        );
        let (mut lo, mut hi) = (u32::MAX, 0);
        for y in 0..512 {
            for x in 0..512 {
                let [r, g, b, a] = at(x, y);
                if a == 0xFF && [r, g, b] != [0xFA, 0xFA, 0xF7] {
                    lo = lo.min(x.min(y));
                    hi = hi.max(x.max(y));
                }
            }
        }
        // The symbol's view box is 512 × 0.62 = 317.44 px, from 97.28 to 414.72.
        assert!(lo >= 97 && hi <= 415, "symbol pixels span {lo}..={hi}");
        assert!(
            hi - lo > 280,
            "the symbol fills most of its box: {lo}..={hi}"
        );
    }

    #[test]
    fn tile_rejects_a_symbol_it_cannot_copy_faithfully() {
        let shifted = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="10 0 10 10"><path d="M0 0H1V1Z"/></svg>"#;
        assert!(tile_svg(shifted).is_err());
        let circle =
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10"><circle r="1"/></svg>"#;
        assert!(tile_svg(circle).is_err());
        let empty = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10"/>"#;
        assert!(tile_svg(empty).is_err());
    }

    #[test]
    fn style_fill_wins_over_the_attribute() {
        let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10"><path fill="#000000" style="fill:#ff6b57" d="M0 0H1V1Z"/></svg>"##;
        let tile = tile_svg(svg).expect("tile");
        assert!(tile.contains("fill=\"#ff6b57\""), "{tile}");
    }
}
