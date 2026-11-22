//! The looks of a workbook: its theme and its styles (S82-02).

use std::collections::BTreeMap;

use gafira_color::{Color, Rgba};
use gafira_doc::{CellBorders, CellStyle, ShapeStroke, TextAlign, TextNode, VerticalAlign};
use roxmltree::Node;

use crate::ReadError;
use crate::package::{Local, parse};

/// The colours of the standard theme, in the order cells name them.
const THEME: [u32; 12] = [
    0xFF_FFFF, 0x00_0000, 0xE7_E6E6, 0x44_546A, 0x44_72C4, 0xED_7D31, 0xA5_A5A5, 0xFF_C000,
    0x5B_9BD5, 0x70_AD47, 0x05_63C1, 0x95_4F72,
];

/// The names of the theme's colours, in that order.
const THEME_NAMES: [&str; 12] = [
    "lt1", "dk1", "lt2", "dk2", "accent1", "accent2", "accent3", "accent4", "accent5", "accent6",
    "hlink", "folHlink",
];

/// The colours by number of the oldest versions, from the eighth; the first
/// eight are the same as these.
const INDEXED: [u32; 56] = [
    0x00_0000, 0xFF_FFFF, 0xFF_0000, 0x00_FF00, 0x00_00FF, 0xFF_FF00, 0xFF_00FF, 0x00_FFFF,
    0x80_0000, 0x00_8000, 0x00_0080, 0x80_8000, 0x80_0080, 0x00_8080, 0xC0_C0C0, 0x80_8080,
    0x99_99FF, 0x99_3366, 0xFF_FFCC, 0xCC_FFFF, 0x66_0066, 0xFF_8080, 0x00_66CC, 0xCC_CCFF,
    0x00_0080, 0xFF_00FF, 0xFF_FF00, 0x00_FFFF, 0x80_0080, 0x80_0000, 0x00_8080, 0x00_00FF,
    0x00_CCFF, 0xCC_FFFF, 0xCC_FFCC, 0xFF_FF99, 0x99_CCFF, 0xFF_99CC, 0xCC_99FF, 0xFF_CC99,
    0x33_66FF, 0x33_CCCC, 0x99_CC00, 0xFF_CC00, 0xFF_9900, 0xFF_6600, 0x66_6699, 0x96_9696,
    0x00_3366, 0x33_9966, 0x00_3300, 0x33_3300, 0x99_3300, 0x99_3366, 0x33_3399, 0x33_3333,
];

/// A colour of `0xRRGGBB`.
pub(crate) fn color(rgb: u32) -> Color {
    // Within 0..=255: the cast is exact.
    let channel = |shift: u32| ((rgb >> shift) & 0xFF) as f32 / 255.0;
    Color::Srgb(Rgba::new(channel(16), channel(8), channel(0), 1.0))
}

fn hex(text: &str) -> Option<u32> {
    let rgb = match text.len() {
        6 => text,
        // The first two digits are an opacity sheets do not use.
        8 => text.get(2..)?,
        _ => return None,
    };
    u32::from_str_radix(rgb, 16).ok()
}

/// `rgb` lighter or darker by `tint`, −1 to 1.
pub(crate) fn tinted(rgb: u32, tint: f64) -> u32 {
    if tint == 0.0 {
        return rgb;
    }
    let tint = tint.clamp(-1.0, 1.0);
    let channel = |shift: u32| f64::from((rgb >> shift) & 0xFF) / 255.0;
    let (r, g, b) = (channel(16), channel(8), channel(0));
    let (max, min) = (r.max(g).max(b), r.min(g).min(b));
    let l = f64::midpoint(max, min);
    let chroma = max - min;
    let s = if chroma == 0.0 {
        0.0
    } else {
        chroma / (1.0 - (2.0 * l - 1.0).abs())
    };
    let l = if tint < 0.0 {
        l * (1.0 + tint)
    } else {
        l * (1.0 - tint) + tint
    };
    let new = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let low = l - new / 2.0;
    let out = |v: f64| -> u32 {
        // The hue keeps the place of each channel between the two ends.
        let place = if chroma == 0.0 {
            0.0
        } else {
            (v - min) / chroma
        };
        // Within 0..=255: the cast is exact.
        ((low + place * new).clamp(0.0, 1.0) * 255.0).round() as u32
    };
    out(r) << 16 | out(g) << 8 | out(b)
}

/// The colours a workbook names.
pub(crate) struct Palette {
    theme: [u32; 12],
    indexed: Vec<u32>,
}

impl Default for Palette {
    fn default() -> Self {
        Self {
            theme: THEME,
            indexed: INDEXED.to_vec(),
        }
    }
}

impl Palette {
    /// With the colours of the theme part.
    pub fn read_theme(&mut self, text: &str) {
        let Ok(xml) = parse(text) else { return };
        let Some(scheme) = xml
            .descendants()
            .find(|n| n.has_tag_name_local("clrScheme"))
        else {
            return;
        };
        for (slot, name) in self.theme.iter_mut().zip(THEME_NAMES) {
            let found = scheme.child(name).and_then(|n| {
                n.children()
                    .find_map(|c| hex(c.attribute("lastClr").or_else(|| c.attribute("val"))?))
            });
            *slot = found.unwrap_or(*slot);
        }
    }

    /// The colour of the theme at `index`.
    pub fn theme(&self, index: usize) -> u32 {
        self.theme.get(index).copied().unwrap_or(0)
    }

    /// The colour an element names; `None` for the automatic one.
    fn read(&self, node: Node<'_, '_>) -> Option<u32> {
        if node.flag("auto", false) {
            return None;
        }
        let rgb = if let Some(text) = node.attribute("rgb") {
            hex(text)?
        } else if let Some(index) = node.index("theme") {
            *self.theme.get(index)?
        } else {
            let index = node.index("indexed")?;
            let index = if index < 8 { index } else { index - 8 };
            *self.indexed.get(index)?
        };
        Some(tinted(rgb, node.float("tint").unwrap_or(0.0)))
    }
}

/// How a cell of the workbook looks.
#[derive(Clone, Debug)]
pub(crate) struct Look {
    pub style: CellStyle,
    /// Whether the words are of the automatic colour.
    pub auto_color: bool,
    /// `None` where what the cell holds decides.
    pub align: Option<TextAlign>,
    /// The code of its numbers.
    pub code: String,
}

impl Default for Look {
    fn default() -> Self {
        Self {
            style: plain(),
            auto_color: true,
            align: None,
            code: String::new(),
        }
    }
}

/// Words with no style.
pub(crate) fn plain() -> CellStyle {
    let mut style = CellStyle::new("Calibri", 11.0 * PX_IN_POINT, color(0));
    style.align = TextAlign::Left;
    style.valign = VerticalAlign::Bottom;
    style
}

/// Px at 96 an inch in a point.
pub(crate) const PX_IN_POINT: f64 = 4.0 / 3.0;

/// The longest name of a font kept, in bytes.
const MAX_FONT: usize = 128;

/// The most styles of cells read.
const MAX_LOOKS: usize = 65_536;

struct Font {
    name: String,
    size: f64,
    bold: bool,
    italic: bool,
    color: Option<u32>,
}

/// The looks of the part of styles, in the order cells name them.
pub(crate) fn looks(text: &str, palette: &mut Palette) -> Result<Vec<Look>, ReadError> {
    let xml = parse(text)?;
    let root = xml.root_element();
    if let Some(list) = root.child("colors").and_then(|n| n.child("indexedColors")) {
        for (i, node) in list.all("rgbColor").enumerate().take(64) {
            let at = if i < 8 { i } else { i - 8 };
            if let (Some(slot), Some(rgb)) = (
                palette.indexed.get_mut(at),
                node.attribute("rgb").and_then(hex),
            ) {
                *slot = rgb;
            }
        }
    }
    let palette = &*palette;
    let list = |name: &str, item: &'static str| {
        root.child(name)
            .into_iter()
            .flat_map(move |n| n.all(item).collect::<Vec<_>>())
            .take(MAX_LOOKS)
    };
    let fonts: Vec<Font> = list("fonts", "font").map(|n| font(n, palette)).collect();
    let fills: Vec<Option<u32>> = list("fills", "fill").map(|n| fill(n, palette)).collect();
    let borders: Vec<CellBorders> = list("borders", "border")
        .map(|n| border(n, palette))
        .collect();
    let codes: BTreeMap<usize, &str> = list("numFmts", "numFmt")
        .filter_map(|n| Some((n.index("numFmtId")?, n.attribute("formatCode")?)))
        .collect();
    Ok(list("cellXfs", "xf")
        .map(|xf| {
            let mut look = Look::default();
            if let Some(font) = xf.index("fontId").and_then(|i| fonts.get(i)) {
                look.style.font.clone_from(&font.name);
                look.style.size = font.size;
                look.style.bold = font.bold;
                look.style.italic = font.italic;
                look.style.color = color(font.color.unwrap_or(0));
                look.auto_color = font.color.is_none_or(|c| c == palette.theme(1));
            }
            let fill = xf.index("fillId").and_then(|i| fills.get(i));
            look.style.fill = fill.copied().flatten().map(color);
            if let Some(border) = xf.index("borderId").and_then(|i| borders.get(i)) {
                look.style.borders = *border;
            }
            if let Some(node) = xf.child("alignment") {
                look.align = match node.attribute("horizontal") {
                    Some("left" | "fill") => Some(TextAlign::Left),
                    Some("center" | "centerContinuous") => Some(TextAlign::Center),
                    Some("right") => Some(TextAlign::Right),
                    Some("justify" | "distributed") => Some(TextAlign::Justify),
                    _ => None,
                };
                look.style.valign = match node.attribute("vertical") {
                    Some("top" | "justify") => VerticalAlign::Top,
                    Some("center" | "distributed") => VerticalAlign::Middle,
                    _ => VerticalAlign::Bottom,
                };
            }
            let id = xf.index("numFmtId").unwrap_or(0);
            let code = codes.get(&id).copied();
            look.code = code.unwrap_or_else(|| built_in(id)).to_owned();
            look
        })
        .collect())
}

fn font(node: Node<'_, '_>, palette: &Palette) -> Font {
    let on = |name: &str| node.child(name).is_some_and(|n| n.flag("val", true));
    let size = node
        .child("sz")
        .and_then(|n| n.float("val"))
        .unwrap_or(11.0);
    let name = node
        .child("name")
        .and_then(|n| n.attribute("val"))
        .map(str::trim)
        .filter(|name| !name.is_empty() && name.len() <= MAX_FONT)
        .unwrap_or("Calibri");
    Font {
        name: name.to_owned(),
        size: (size.clamp(1.0, 409.0) * PX_IN_POINT).min(TextNode::MAX_SIZE),
        bold: on("b"),
        italic: on("i"),
        color: node.child("color").and_then(|n| palette.read(n)),
    }
}

fn fill(node: Node<'_, '_>, palette: &Palette) -> Option<u32> {
    if let Some(gradient) = node.child("gradientFill") {
        return palette.read(gradient.child("stop")?.child("color")?);
    }
    let pattern = node.child("patternFill")?;
    match pattern.attribute("patternType") {
        None | Some("none" | "gray125") => None,
        Some(_) => palette.read(pattern.child("fgColor")?),
    }
}

fn border(node: Node<'_, '_>, palette: &Palette) -> CellBorders {
    let side = |names: [&str; 2]| {
        let edge = names.iter().find_map(|name| node.child(name))?;
        let width = match edge.attribute("style")? {
            "none" => return None,
            "thick" | "double" => 3.0,
            style if style.starts_with("medium") || style == "slantDashDot" => 2.0,
            _ => 1.0,
        };
        let rgb = edge.child("color").and_then(|n| palette.read(n));
        Some(ShapeStroke::new(color(rgb.unwrap_or(0)), width))
    };
    CellBorders {
        top: side(["top", "top"]),
        right: side(["right", "end"]),
        bottom: side(["bottom", "bottom"]),
        left: side(["left", "start"]),
    }
}

/// The code of the formats every workbook has.
fn built_in(id: usize) -> &'static str {
    match id {
        1 => "0",
        2 => "0.00",
        3 => "#,##0",
        4 => "#,##0.00",
        5 | 6 => "\"$\"#,##0;-\"$\"#,##0",
        7 | 8 => "\"$\"#,##0.00;-\"$\"#,##0.00",
        9 => "0%",
        10 => "0.00%",
        // The short date and the ones of eastern calendars: each system
        // writes them its way.
        14 | 27..=31 | 34..=36 | 50..=58 => "yyyy-mm-dd",
        15 => "d-mmm-yy",
        16 => "d-mmm",
        17 => "mmm-yy",
        18 => "h:mm AM/PM",
        19 => "h:mm:ss AM/PM",
        20 | 32 => "h:mm",
        21 | 33 => "h:mm:ss",
        22 => "yyyy-mm-dd h:mm",
        37 | 38 => "#,##0;(#,##0)",
        39 | 40 => "#,##0.00;(#,##0.00)",
        41 => "#,##0;-#,##0;\"-\"",
        42 => "\"$\"#,##0;-\"$\"#,##0;\"$\"\"-\"",
        43 => "#,##0.00;-#,##0.00;\"-\"",
        44 => "\"$\"#,##0.00;-\"$\"#,##0.00;\"$\"\"-\"",
        45 => "mm:ss",
        46 => "[h]:mm:ss",
        47 => "mm:ss.0",
        // Words as they are, scientific ones and fractions: in full.
        _ => "",
    }
}
