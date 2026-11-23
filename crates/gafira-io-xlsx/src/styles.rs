//! `styles.xml`: the looks of the cells, each one written once.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use gafira_color::Color;
use gafira_doc::{ShapeStroke, TableNode, TextAlign, VerticalAlign};
use gafira_table::NumberFormat;

use crate::xml::{escape, number};
use crate::{DECL, MAIN};

/// The elements of a cell's looks, as they are written.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Looks {
    font: String,
    fill: String,
    border: String,
    align: String,
}

/// `FFRRGGBB` of `color` over white: a sheet has no transparency.
fn argb(color: Color) -> String {
    let Color::Srgb(c) = color.convert(gafira_color::ColorSpaceId::Srgb) else {
        return "FF000000".to_owned();
    };
    let channel = |v: f32| {
        let over = v * c.a + (1.0 - c.a);
        // Within 0..=255: the cast is exact.
        (over.clamp(0.0, 1.0) * 255.0).round() as u8
    };
    format!(
        "FF{:02X}{:02X}{:02X}",
        channel(c.r),
        channel(c.g),
        channel(c.b)
    )
}

fn line(tag: &str, stroke: Option<ShapeStroke>, to_px: f64) -> String {
    let Some(stroke) = stroke.filter(|s| s.width > 0.0) else {
        return format!("<{tag}/>");
    };
    let width = stroke.width * to_px;
    let style = if width <= 1.25 {
        "thin"
    } else if width <= 2.5 {
        "medium"
    } else {
        "thick"
    };
    format!(
        "<{tag} style=\"{style}\"><color rgb=\"{}\"/></{tag}>",
        argb(stroke.color)
    )
}

/// The looks of a cell of `t`; `to_px` turns its px into px of a sheet.
pub(crate) fn looks(t: &TableNode, r: usize, c: usize, to_px: f64) -> Looks {
    let Some(cell) = t.cell(r, c) else {
        return Styles::plain();
    };
    let s = &cell.style;
    let mut font = String::from("<font>");
    if s.bold {
        font.push_str("<b/>");
    }
    if s.italic {
        font.push_str("<i/>");
    }
    // Points, to the half, as Excel keeps them.
    let points = (s.size * to_px * 0.75 * 2.0).round() / 2.0;
    let _ = write!(
        font,
        "<sz val=\"{}\"/><color rgb=\"{}\"/><name val=\"{}\"/></font>",
        number(points.clamp(1.0, 409.0)),
        argb(s.color),
        escape(&s.font)
    );
    let fill = match s.fill.filter(|f| f.alpha() > 0.0) {
        Some(color) => format!(
            "<fill><patternFill patternType=\"solid\"><fgColor rgb=\"{}\"/></patternFill></fill>",
            argb(color)
        ),
        None => NO_FILL.to_owned(),
    };
    // The edge two cells share may be kept by either of them.
    let shared = t.spacing == 0.0;
    let next = |r: Option<usize>, c: Option<usize>| {
        t.cell(r?, c?)
            .filter(|_| shared)
            .map(|cell| cell.style.borders)
    };
    let b = s.borders;
    let left = b
        .left
        .or(next(Some(r), c.checked_sub(1)).and_then(|o| o.right));
    let right = b.right.or(next(Some(r), Some(c + 1)).and_then(|o| o.left));
    let top = b
        .top
        .or(next(r.checked_sub(1), Some(c)).and_then(|o| o.bottom));
    let bottom = b.bottom.or(next(Some(r + 1), Some(c)).and_then(|o| o.top));
    let border = format!(
        "<border>{}{}{}{}<diagonal/></border>",
        line("left", left, to_px),
        line("right", right, to_px),
        line("top", top, to_px),
        line("bottom", bottom, to_px)
    );
    let horizontal = match s.align {
        TextAlign::Left => "left",
        TextAlign::Center => "center",
        TextAlign::Right => "right",
        TextAlign::Justify => "justify",
    };
    let vertical = match s.valign {
        VerticalAlign::Top => "top",
        VerticalAlign::Middle => "center",
        VerticalAlign::Bottom => "bottom",
    };
    Looks {
        font,
        fill,
        border,
        align: format!(
            "<alignment horizontal=\"{horizontal}\" vertical=\"{vertical}\" wrapText=\"1\"{}/>",
            // The room inside the cell, at the side the words lean on.
            if t.padding > 0.0 && matches!(s.align, TextAlign::Left | TextAlign::Right) {
                " indent=\"1\""
            } else {
                ""
            }
        ),
    }
}

const NO_FILL: &str = "<fill><patternFill patternType=\"none\"/></fill>";
const NO_BORDER: &str = "<border><left/><right/><top/><bottom/><diagonal/></border>";
const PLAIN_FONT: &str = "<font><sz val=\"11\"/><name val=\"Calibri\"/></font>";

/// A list whose items are written once.
#[derive(Default)]
struct Once {
    list: Vec<String>,
    at: BTreeMap<String, usize>,
}

impl Once {
    fn with(items: &[&str]) -> Self {
        let mut once = Self::default();
        for item in items {
            once.id(item);
        }
        once
    }

    fn id(&mut self, item: &str) -> usize {
        if let Some(at) = self.at.get(item) {
            return *at;
        }
        self.list.push(item.to_owned());
        self.at.insert(item.to_owned(), self.list.len() - 1);
        self.list.len() - 1
    }
}

/// The styles of a workbook.
pub(crate) struct Styles {
    fonts: Once,
    fills: Once,
    borders: Once,
    formats: Once,
    cells: Once,
}

impl Default for Styles {
    fn default() -> Self {
        Self {
            fonts: Once::with(&[PLAIN_FONT]),
            // The first two are fixed by the format.
            fills: Once::with(&[
                NO_FILL,
                "<fill><patternFill patternType=\"gray125\"/></fill>",
            ]),
            borders: Once::with(&[NO_BORDER]),
            formats: Once::default(),
            cells: Once::with(&[
                "<xf numFmtId=\"0\" fontId=\"0\" fillId=\"0\" borderId=\"0\" xfId=\"0\"/>",
            ]),
        }
    }
}

/// The first id of a number format of one's own.
const FIRST_FORMAT: usize = 164;

impl Styles {
    fn plain() -> Looks {
        Looks {
            font: PLAIN_FONT.to_owned(),
            fill: NO_FILL.to_owned(),
            border: NO_BORDER.to_owned(),
            align: String::new(),
        }
    }

    /// The id of `looks`, with the format of a number and how it was
    /// written.
    pub(crate) fn id(&mut self, looks: &Looks, written: Option<(NumberFormat, &str)>) -> usize {
        let font = self.fonts.id(&looks.font);
        let fill = self.fills.id(&looks.fill);
        let border = self.borders.id(&looks.border);
        let format = match written.map(|(f, words)| code(f, words)) {
            Some(code) if code != "General" => FIRST_FORMAT + self.formats.id(&code),
            _ => 0,
        };
        self.cells.id(&format!(
            "<xf numFmtId=\"{format}\" fontId=\"{font}\" fillId=\"{fill}\" borderId=\"{border}\" xfId=\"0\" applyNumberFormat=\"1\" applyFont=\"1\" applyFill=\"1\" applyBorder=\"1\" applyAlignment=\"1\">{}</xf>",
            looks.align
        ))
    }

    /// The part.
    pub(crate) fn xml(&self) -> String {
        let mut out = format!("{DECL}<styleSheet xmlns=\"{MAIN}\">");
        if !self.formats.list.is_empty() {
            let _ = write!(out, "<numFmts count=\"{}\">", self.formats.list.len());
            for (i, code) in self.formats.list.iter().enumerate() {
                let _ = write!(
                    out,
                    "<numFmt numFmtId=\"{}\" formatCode=\"{}\"/>",
                    FIRST_FORMAT + i,
                    escape(code)
                );
            }
            out.push_str("</numFmts>");
        }
        for (tag, list) in [
            ("fonts", &self.fonts.list),
            ("fills", &self.fills.list),
            ("borders", &self.borders.list),
        ] {
            let _ = write!(
                out,
                "<{tag} count=\"{}\">{}</{tag}>",
                list.len(),
                list.concat()
            );
        }
        let _ = write!(
            out,
            "<cellStyleXfs count=\"1\"><xf numFmtId=\"0\" fontId=\"0\" fillId=\"0\" borderId=\"0\"/></cellStyleXfs><cellXfs count=\"{}\">{}</cellXfs><cellStyles count=\"1\"><cellStyle name=\"Normal\" xfId=\"0\" builtinId=\"0\"/></cellStyles></styleSheet>",
            self.cells.list.len(),
            self.cells.list.concat()
        );
        out
    }
}

/// The format code that shows a number as it was written: its decimals, its
/// thousands, its sign of percent or of currency.
fn code(format: NumberFormat, words: &str) -> String {
    let digits: String = words
        .chars()
        .filter(|c| c.is_ascii_digit() || matches!(c, '.' | ','))
        .collect();
    let signs = digits.matches(['.', ',']).count();
    let decimals = match digits.rfind(['.', ',']) {
        Some(at) => {
            let sign = &digits[at..=at];
            // The only sign of its kind is the decimal one.
            if digits.matches(sign).count() == 1 {
                digits.len() - at - 1
            } else {
                0
            }
        }
        None => 0,
    };
    let grouped = signs > usize::from(decimals > 0);
    let mut body = String::from(if grouped { "#,##0" } else { "0" });
    if decimals > 0 {
        body.push('.');
        body.push_str(&"0".repeat(decimals));
    }
    match format {
        NumberFormat::General if body == "0" => "General".to_owned(),
        NumberFormat::General => body,
        NumberFormat::Percent => format!("{body}%"),
        NumberFormat::Currency(sign)
            if words.trim_start().trim_start_matches('-').starts_with(sign) =>
        {
            format!("\"{sign}\"{body}")
        }
        NumberFormat::Currency(sign) => format!("{body} \"{sign}\""),
    }
}
