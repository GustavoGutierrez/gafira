//! The icons of the interface (spec 037, 03 §13, D-87): Phosphor (MIT),
//! weight Regular, and Fill for active states.
//!
//! `build.rs` turns the vendored SVG files of `assets/icons/phosphor/` into
//! flat contours embedded in the binary. At draw time an icon is rasterised
//! once per physical pixel size with a coverage (anti-aliased, non-zero)
//! scanline fill into a white alpha texture, and the painter tints it with a
//! theme token, so it stays crisp at any scale and has no colour of its own.

use std::collections::HashMap;

use egui::{Color32, ColorImage, Pos2, Rect, TextureHandle, TextureOptions, Vec2};

#[cfg(test)]
mod path_data;

include!(concat!(env!("OUT_DIR"), "/icons.rs"));

/// The side of the Phosphor grid in the stored units (hundredths of 256).
const GRID_UNITS: f32 = 25_600.0;

/// The weight of an icon (D-87).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum Weight {
    Regular,
    /// Active states; an icon without a Fill file draws its Regular one.
    Fill,
}

/// Every icon the interface draws; each one is a vendored Phosphor file.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum Icon {
    Home,
    Undo,
    Redo,
    Versions,
    Export,
    Templates,
    Pieces,
    Text,
    Identity,
    Library,
    Drawing,
    BringForward,
    SendBackward,
    Delete,
    ZoomOut,
    ZoomIn,
    Expand,
    RotateLeft,
    RotateRight,
    FlipHorizontal,
    FlipVertical,
    AlignLeft,
    AlignCenter,
    AlignRight,
    AlignTop,
    AlignMiddle,
    AlignBottom,
    DistributeHorizontal,
    DistributeVertical,
    BringToFront,
    SendToBack,
    Magnet,
    /// Ocultar and Mostrar in Capas (spec 048).
    Visible,
    Hidden,
    /// Bloquear and Desbloquear in Capas (spec 048).
    Locked,
    Unlocked,
    /// Notas and Páginas in the bottom bar (spec 049).
    Notes,
    Pages,
    /// The page header (spec 056): Subir, Duplicar and Añadir página.
    PageUp,
    PageCopy,
    PageAdd,
    /// The cards of Elementos and its «Volver» arrow (spec 064).
    CategoryShapes,
    CategoryCollage,
    CategoryFrames,
    CategoryIcons,
    CategoryQrCodes,
    CategoryTables,
    CategoryCharts,
    CategoryIllustrations,
    Back,
    /// «Luces y efectos» and its elements (spec 065): the category, also
    /// «Brillo», and «Destello de lente».
    CategoryLights,
    LensFlare,
    /// «Destello de cine», «Rayos de sol» and «Luces bokeh» (D-104).
    Streak,
    SunRays,
    Bokeh,
}

impl Icon {
    #[cfg(test)]
    pub(crate) const ALL: [Self; 55] = [
        Self::Home,
        Self::Undo,
        Self::Redo,
        Self::Versions,
        Self::Export,
        Self::Templates,
        Self::Pieces,
        Self::Text,
        Self::Identity,
        Self::Library,
        Self::Drawing,
        Self::BringForward,
        Self::SendBackward,
        Self::Delete,
        Self::ZoomOut,
        Self::ZoomIn,
        Self::Expand,
        Self::RotateLeft,
        Self::RotateRight,
        Self::FlipHorizontal,
        Self::FlipVertical,
        Self::AlignLeft,
        Self::AlignCenter,
        Self::AlignRight,
        Self::AlignTop,
        Self::AlignMiddle,
        Self::AlignBottom,
        Self::DistributeHorizontal,
        Self::DistributeVertical,
        Self::BringToFront,
        Self::SendToBack,
        Self::Magnet,
        Self::Visible,
        Self::Hidden,
        Self::Locked,
        Self::Unlocked,
        Self::Notes,
        Self::Pages,
        Self::PageUp,
        Self::PageCopy,
        Self::PageAdd,
        Self::CategoryShapes,
        Self::CategoryCollage,
        Self::CategoryFrames,
        Self::CategoryIcons,
        Self::CategoryQrCodes,
        Self::CategoryTables,
        Self::CategoryCharts,
        Self::CategoryIllustrations,
        Self::Back,
        Self::CategoryLights,
        Self::LensFlare,
        Self::Streak,
        Self::SunRays,
        Self::Bokeh,
    ];

    /// The Phosphor name of the icon (its file without weight suffix).
    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Home => "house",
            Self::Undo => "arrow-u-up-left",
            Self::Redo => "arrow-u-up-right",
            Self::Versions => "clock-counter-clockwise",
            Self::Export => "export",
            Self::Templates => "layout",
            Self::Pieces => "shapes",
            Self::Text => "text-t",
            Self::Identity => "palette",
            Self::Library => "images",
            Self::Drawing => "pencil-simple",
            Self::BringForward => "arrow-up",
            Self::SendBackward => "arrow-down",
            Self::Delete => "trash",
            Self::ZoomOut => "minus",
            Self::ZoomIn => "plus",
            Self::Expand => "caret-down",
            Self::RotateLeft => "arrow-counter-clockwise",
            Self::RotateRight => "arrow-clockwise",
            Self::FlipHorizontal => "flip-horizontal",
            Self::FlipVertical => "flip-vertical",
            Self::AlignLeft => "align-left",
            Self::AlignCenter => "align-center-horizontal",
            Self::AlignRight => "align-right",
            Self::AlignTop => "align-top",
            Self::AlignMiddle => "align-center-vertical",
            Self::AlignBottom => "align-bottom",
            Self::DistributeHorizontal => "columns",
            Self::DistributeVertical => "rows",
            Self::BringToFront => "arrow-line-up",
            Self::SendToBack => "arrow-line-down",
            Self::Magnet => "magnet",
            Self::Visible => "eye",
            Self::Hidden => "eye-slash",
            Self::Locked => "lock-simple",
            Self::Unlocked => "lock-simple-open",
            Self::Notes => "note-pencil",
            Self::Pages => "squares-four",
            Self::PageUp => "caret-up",
            Self::PageCopy => "copy",
            Self::PageAdd => "file-plus",
            Self::CategoryShapes => "polygon",
            Self::CategoryCollage => "grid-four",
            Self::CategoryFrames => "frame-corners",
            Self::CategoryIcons => "smiley",
            Self::CategoryQrCodes => "qr-code",
            Self::CategoryTables => "table",
            Self::CategoryCharts => "chart-bar",
            Self::CategoryIllustrations => "paint-brush-broad",
            Self::Back => "arrow-left",
            Self::CategoryLights => "sparkle",
            Self::LensFlare => "sun",
            Self::Streak => "arrows-out-line-horizontal",
            Self::SunRays => "sun-horizon",
            Self::Bokeh => "circles-three",
        }
    }

    /// Whether the Fill weight is vendored: the icons that show an active
    /// state (the rail tabs, 03 §8.1).
    pub(crate) fn has_fill(self) -> bool {
        matches!(
            self,
            Self::Templates
                | Self::Pieces
                | Self::Text
                | Self::Identity
                | Self::Library
                | Self::Drawing
        )
    }

    /// The table key of the icon in `weight`.
    fn key(self, weight: Weight) -> String {
        match weight {
            Weight::Fill if self.has_fill() => format!("fill/{}-fill", self.name()),
            _ => format!("regular/{}", self.name()),
        }
    }

    /// The contours of the icon, in hundredths of a grid unit.
    pub(crate) fn geometry(self, weight: Weight) -> &'static [&'static [[i16; 2]]] {
        let key = self.key(weight);
        GEOMETRY
            .binary_search_by(|(k, _)| (*k).cmp(key.as_str()))
            .map_or(&[], |i| GEOMETRY[i].1)
    }
}

/// The coverage of `contours` on a `side` × `side` pixel square, each value
/// in [0, 1], row by row. Non-zero winding with anti-aliased edges: the
/// signed area of every edge is accumulated per pixel and summed along the
/// rows (the accumulation rasteriser of font-rs).
pub(crate) fn coverage(contours: &[&[[i16; 2]]], side: usize) -> Vec<f32> {
    let side = side.max(1);
    let mut acc = vec![0.0_f32; side * side + 4];
    let scale = side as f32 / GRID_UNITS;
    let limit = side as f32;
    let at = |p: [i16; 2]| {
        [
            (f32::from(p[0]) * scale).clamp(0.0, limit),
            (f32::from(p[1]) * scale).clamp(0.0, limit),
        ]
    };
    for contour in contours {
        for (i, a) in contour.iter().enumerate() {
            let b = contour[(i + 1) % contour.len()];
            edge(&mut acc, side, at(*a), at(b));
        }
    }
    let mut sum = 0.0_f32;
    acc[..side * side]
        .iter()
        .map(|d| {
            sum += d;
            sum.abs().min(1.0)
        })
        .collect()
}

/// Add the signed area one edge leaves on each pixel to `acc`.
fn edge(acc: &mut [f32], side: usize, p0: [f32; 2], p1: [f32; 2]) {
    if p0[1] == p1[1] {
        return;
    }
    let (dir, p0, p1) = if p0[1] < p1[1] {
        (1.0, p0, p1)
    } else {
        (-1.0, p1, p0)
    };
    let dxdy = (p1[0] - p0[0]) / (p1[1] - p0[1]);
    let mut x = p0[0];
    // Coordinates are clamped to [0, side]: the casts truncate non-negative values.
    let first = p0[1] as usize;
    let last = (p1[1].ceil() as usize).min(side);
    for y in first..last {
        let row = y * side;
        let dy = ((y + 1) as f32).min(p1[1]) - (y as f32).max(p0[1]);
        let xnext = x + dxdy * dy;
        let d = dy * dir;
        let (x0, x1) = if x < xnext { (x, xnext) } else { (xnext, x) };
        let x0floor = x0.floor();
        let x0i = x0floor as usize;
        let x1ceil = x1.ceil();
        let x1i = x1ceil as usize;
        if x1i <= x0i + 1 {
            let xmf = 0.5 * (x + xnext) - x0floor;
            acc[row + x0i] += d - d * xmf;
            acc[row + x0i + 1] += d * xmf;
        } else {
            let s = (x1 - x0).recip();
            let x0f = x0 - x0floor;
            let a0 = 0.5 * s * (1.0 - x0f) * (1.0 - x0f);
            let x1f = x1 - x1ceil + 1.0;
            let am = 0.5 * s * x1f * x1f;
            acc[row + x0i] += d * a0;
            if x1i == x0i + 2 {
                acc[row + x0i + 1] += d * (1.0 - a0 - am);
            } else {
                let a1 = s * (1.5 - x0f);
                acc[row + x0i + 1] += d * (a1 - a0);
                for xi in x0i + 2..x1i - 1 {
                    acc[row + xi] += d * s;
                }
                let a2 = a1 + (x1i - x0i - 3) as f32 * s;
                acc[row + x1i - 1] += d * (1.0 - a2 - am);
            }
            acc[row + x1i] += d * am;
        }
        x = xnext;
    }
}

/// The rasterised icons, by icon, weight and side in physical pixels.
#[derive(Clone, Default)]
struct Textures(HashMap<(Icon, Weight, u32), TextureHandle>);

/// The white alpha texture of `icon` at `side` physical pixels, made once.
fn texture(ctx: &egui::Context, icon: Icon, weight: Weight, side: u32) -> TextureHandle {
    let id = egui::Id::new("gafira.icons");
    if let Some(found) = ctx.data(|d| {
        d.get_temp::<Textures>(id)
            .and_then(|t| t.0.get(&(icon, weight, side)).cloned())
    }) {
        return found;
    }
    let pixels = side as usize;
    let alpha = coverage(icon.geometry(weight), pixels);
    let image = ColorImage::new(
        [pixels, pixels],
        alpha
            .iter()
            // In [0, 1] by construction, so the cast cannot wrap.
            .map(|a| Color32::from_white_alpha((a * 255.0).round() as u8))
            .collect(),
    );
    let handle = ctx.load_texture(
        format!("icon/{}/{weight:?}/{side}", icon.name()),
        image,
        TextureOptions::LINEAR,
    );
    ctx.data_mut(|d| {
        d.get_temp_mut_or_default::<Textures>(id)
            .0
            .insert((icon, weight, side), handle.clone());
    });
    handle
}

/// The side in physical pixels of an icon of `size` points.
fn physical(ctx: &egui::Context, size: f32) -> u32 {
    let side = (size * ctx.pixels_per_point()).round().clamp(1.0, 512.0);
    // Clamped to [1, 512]: the cast is exact.
    side as u32
}

/// Draw `icon` filling `rect` (square), tinted with `color`, on the pixel
/// grid so its edges stay sharp.
pub(crate) fn paint(
    painter: &egui::Painter,
    rect: Rect,
    icon: Icon,
    weight: Weight,
    color: Color32,
) {
    let ctx = painter.ctx();
    let ppp = ctx.pixels_per_point();
    let size = rect.width().min(rect.height());
    let side = physical(ctx, size);
    let snapped = (rect.center() - Vec2::splat(size / 2.0)).to_vec2() * ppp;
    let min = Pos2::new(snapped.x.round() / ppp, snapped.y.round() / ppp);
    let target = Rect::from_min_size(min, Vec2::splat(side as f32 / ppp));
    let texture = texture(ctx, icon, weight, side);
    let uv = Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0));
    painter.image(texture.id(), target, uv, color);
}

/// Draw `icon` as a square of `size` centred on `centre`, turned by `angle`
/// radians (the cards of Elementos, spec 064).
pub(crate) fn paint_turned(
    painter: &egui::Painter,
    centre: Pos2,
    size: f32,
    angle: f32,
    icon: Icon,
    weight: Weight,
    color: Color32,
) {
    let ctx = painter.ctx();
    let texture = texture(ctx, icon, weight, physical(ctx, size));
    let uv = Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0));
    let mut mesh = egui::Mesh::with_texture(texture.id());
    mesh.add_rect_with_uv(Rect::from_center_size(centre, Vec2::splat(size)), uv, color);
    mesh.rotate(egui::emath::Rot2::from_angle(angle), centre);
    painter.add(mesh);
}

/// `icon` as an egui image of `size` points, for buttons with a label.
pub(crate) fn image(
    ctx: &egui::Context,
    icon: Icon,
    weight: Weight,
    size: f32,
    color: Color32,
) -> egui::Image<'static> {
    let texture = texture(ctx, icon, weight, physical(ctx, size));
    egui::Image::from_texture((texture.id(), Vec2::splat(size))).tint(color)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::fs;
    use std::path::{Path, PathBuf};

    use super::*;

    fn assets() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/icons/phosphor")
    }

    fn vendored() -> Vec<String> {
        let mut found = Vec::new();
        for weight in ["fill", "regular"] {
            for entry in fs::read_dir(assets().join(weight))
                .expect("icon folder")
                .flatten()
            {
                let name = entry.file_name().to_string_lossy().into_owned();
                if let Some(stem) = name.strip_suffix(".svg") {
                    found.push(format!("{weight}/{stem}"));
                }
            }
        }
        found.sort();
        found
    }

    fn used() -> BTreeSet<String> {
        Icon::ALL
            .iter()
            .flat_map(|icon| {
                let mut keys = vec![icon.key(Weight::Regular)];
                if icon.has_fill() {
                    keys.push(icon.key(Weight::Fill));
                }
                keys
            })
            .collect()
    }

    #[test]
    fn s37_02_every_icon_has_geometry_inside_its_grid_in_each_weight() {
        for icon in Icon::ALL {
            for weight in [Weight::Regular, Weight::Fill] {
                let contours = icon.geometry(weight);
                assert!(!contours.is_empty(), "{icon:?} {weight:?}");
                for contour in contours {
                    assert!(contour.len() >= 3, "{icon:?}: a contour is a polygon");
                    for p in *contour {
                        for v in p {
                            assert!((0..=25_600).contains(v), "{icon:?}: {p:?} leaves the grid");
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn s37_02_the_vendored_files_are_exactly_the_icons_the_ui_names() {
        // Only what the interface draws is vendored (D-87, ARQ-75).
        let files: BTreeSet<String> = vendored().into_iter().collect();
        assert_eq!(files, used());
        let table: BTreeSet<String> = GEOMETRY.iter().map(|(k, _)| (*k).to_owned()).collect();
        assert_eq!(table, files, "build.rs converted every file");
    }

    #[test]
    fn s37_02_the_embedded_table_is_the_parse_of_the_files() {
        for key in vendored() {
            let svg = fs::read_to_string(assets().join(format!("{key}.svg"))).expect("svg");
            let parsed = path_data::svg_contours(&svg).expect("every icon parses");
            let embedded = GEOMETRY
                .iter()
                .find(|(k, _)| *k == key)
                .map(|(_, g)| *g)
                .expect("embedded");
            assert_eq!(parsed.len(), embedded.len(), "{key}");
            for (a, b) in parsed.iter().zip(embedded) {
                assert_eq!(a.len(), b.len(), "{key}");
                for (p, q) in a.iter().zip(*b) {
                    for axis in 0..2 {
                        let stored = f32::from(q[axis]) / 100.0;
                        assert!((p[axis] - stored).abs() <= 0.005 + 1e-3, "{key}");
                        assert!(stored <= path_data::GRID, "{key}");
                    }
                }
            }
        }
    }

    #[test]
    fn s37_03_a_square_outline_is_hollow_and_its_stroke_is_solid() {
        // A 16-unit stroke around 32..224, as Phosphor draws one: the
        // outer square one way round, the inner one the other.
        let outer: &[[i16; 2]] = &[
            [2_400, 2_400],
            [23_200, 2_400],
            [23_200, 23_200],
            [2_400, 23_200],
        ];
        let inner: &[[i16; 2]] = &[
            [4_000, 4_000],
            [4_000, 21_600],
            [21_600, 21_600],
            [21_600, 4_000],
        ];
        let side = 64;
        let alpha = coverage(&[outer, inner], side);
        let at = |x: usize, y: usize| alpha[y * side + x];
        assert!(at(32, 32) < 0.01, "the centre is empty: {}", at(32, 32));
        assert!(at(9, 32) > 0.99, "the stroke is covered: {}", at(9, 32));
        assert!(at(2, 2) < 0.01, "outside the outline is empty");
        assert!(alpha.iter().all(|a| (0.0..=1.0).contains(a)));
    }

    #[test]
    fn s37_03_edges_are_anti_aliased_by_area() {
        // A square from 0 to half a pixel past the middle covers that pixel half.
        let half: &[[i16; 2]] = &[[0, 0], [14_400, 0], [14_400, 25_600], [0, 25_600]];
        let alpha = coverage(&[half], 16);
        // 144 units of 256 is 9 px of 16: pixel 8 full, pixel 9 empty.
        assert!(alpha[8] > 0.99 && alpha[9] < 0.01, "{:?}", &alpha[..16]);
        let offset: &[[i16; 2]] = &[[0, 0], [14_000, 0], [14_000, 25_600], [0, 25_600]];
        let alpha = coverage(&[offset], 16);
        // 140 units is 8.75 px: pixel 8 is three quarters covered.
        assert!((alpha[8] - 0.75).abs() < 0.01, "{}", alpha[8]);
    }

    #[test]
    fn s37_03_the_fill_weight_falls_back_to_regular_without_a_fill_file() {
        assert_eq!(
            Icon::Undo.geometry(Weight::Fill),
            Icon::Undo.geometry(Weight::Regular)
        );
        assert_ne!(
            Icon::Pieces.geometry(Weight::Fill),
            Icon::Pieces.geometry(Weight::Regular)
        );
    }

    /// The `[[asset]]` tables of `assets/LICENSES.toml` as `key = "value"` pairs.
    fn licence_entries() -> Vec<Vec<(String, String)>> {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let text = fs::read_to_string(root.join("assets/LICENSES.toml")).expect("licence file");
        let mut entries: Vec<Vec<(String, String)>> = Vec::new();
        for line in text.lines().map(str::trim) {
            if line == "[[asset]]" {
                entries.push(Vec::new());
            } else if let (Some(entry), Some((k, v))) = (entries.last_mut(), line.split_once('=')) {
                entry.push((
                    k.trim().trim_matches('"').to_owned(),
                    v.trim().trim_matches('"').to_owned(),
                ));
            }
        }
        entries
    }

    #[test]
    fn s37_04_every_vendored_icon_is_registered_with_its_mit_licence_and_hash() {
        // ARQ-75: each icon file has its own entry; the hash is BLAKE3 of the file.
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let entries = licence_entries();
        for key in vendored() {
            let path = format!("assets/icons/phosphor/{key}.svg");
            let entry = entries
                .iter()
                .find(|e| e.iter().any(|(k, v)| k == "path" && *v == path))
                .unwrap_or_else(|| panic!("{path} has no entry in assets/LICENSES.toml"));
            let get = |name: &str| {
                entry
                    .iter()
                    .find(|(k, _)| k == name)
                    .map_or("", |(_, v)| v.as_str())
            };
            assert_eq!(get("license"), "MIT", "{path}");
            assert_eq!(get("version"), "2.1.1", "{path}");
            assert!(root.join(get("license_file")).is_file(), "{path}");
            let bytes = fs::read(root.join(&path)).expect("icon file");
            assert_eq!(
                get("hash"),
                blake3::hash(&bytes).to_hex().as_str(),
                "{path}"
            );
        }
    }

    #[test]
    fn s37_04_the_notice_names_phosphor_its_version_and_licence() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let notices = fs::read_to_string(root.join("THIRD_PARTY_NOTICES.md")).expect("notices");
        let licence = fs::read_to_string(assets().join("LICENSE")).expect("licence");
        assert!(notices.contains("Phosphor") && notices.contains("2.1.1"));
        assert!(licence.starts_with("MIT License"));
        assert!(notices.contains("Copyright (c) 2023 Phosphor Icons"));
    }
}
