//! Bulleted and numbered lists (FD-41, Listas).

use gafira_geom::Size;

use crate::layout::{check_input, check_output};
use crate::{TextError, TextLayout, TextStyle, layout_box};

/// Deepest level of a list.
const MAX_LEVEL: u8 = 4;

/// The markers of the bullet levels, in a cycle.
const BULLETS: [char; 3] = ['\u{2022}', '\u{25e6}', '\u{25aa}'];

/// How an item is marked.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ListKind {
    /// A bullet that depends on the level.
    Bullet,
    /// `1.`, `2.`, counted per level.
    Numbered,
}

/// One item of a list.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ListItem<'a> {
    /// The text of the item.
    pub text: &'a str,
    /// Level, from 1 to 4.
    pub level: u8,
    /// Bullet or number.
    pub kind: ListKind,
}

/// One item laid out.
#[derive(Clone, Debug, PartialEq)]
pub struct ListEntry {
    /// The marker, such as `•` or `2.`, to draw at `indent`, on the first baseline.
    pub marker: String,
    /// Indent of the marker from the left of the box, in px: 1.5 em per level below the
    /// first, so the marker of level 1 is at the edge.
    pub indent: f32,
    /// Distance from the top of the list to the top of the item, in px.
    pub top: f32,
    /// The text of the item, hanging: every line starts at the indent plus the widest marker
    /// of the level in the list plus a space, and the `x` of its clusters already includes
    /// that. Its ranges index the text of the item alone.
    pub layout: TextLayout,
}

/// Lays a list out, one entry per item, stacked from the top.
///
/// The numbers (as in [`layout_box`]) and the font stack are checked first, then a level
/// outside 1 to 4 gives `Err(OutOfRange)` with the field `items.level`. Numbering counts the
/// numbered items of each level: a bullet adds nothing and restarts nothing, and an item of
/// a shallower level restarts the deeper levels.
pub fn layout_list(
    items: &[ListItem<'_>],
    fonts: &[&[u8]],
    style: &TextStyle,
    dpi: f64,
    bounds: Size,
) -> Result<Vec<ListEntry>, TextError> {
    let size_px = check_input(style, dpi, bounds)?;
    if fonts.is_empty() {
        return Err(TextError::NoFont);
    }
    if items.iter().any(|i| !(1..=MAX_LEVEL).contains(&i.level)) {
        return Err(TextError::OutOfRange {
            field: "items.level",
        });
    }
    let mut counters = [0_u32; MAX_LEVEL as usize];
    let markers: Vec<String> = items
        .iter()
        .map(|item| {
            let depth = usize::from(item.level) - 1;
            counters[depth + 1..].fill(0);
            match item.kind {
                ListKind::Bullet => BULLETS[depth % BULLETS.len()].to_string(),
                ListKind::Numbered => {
                    counters[depth] += 1;
                    format!("{}.", counters[depth])
                }
            }
        })
        .collect();
    // Where the text starts after the marker, for the widest marker of each level.
    let mut hang = [0.0_f32; MAX_LEVEL as usize];
    let mut measured: Vec<(u8, &str)> = Vec::new();
    for (item, marker) in items.iter().zip(&markers) {
        if !measured.contains(&(item.level, marker.as_str())) {
            measured.push((item.level, marker.as_str()));
            let at = text_start(marker, fonts, style, dpi)?;
            let level = usize::from(item.level) - 1;
            hang[level] = hang[level].max(at);
        }
    }
    let mut top = 0.0_f32;
    let mut entries = Vec::with_capacity(items.len());
    for (item, marker) in items.iter().zip(markers) {
        let depth = usize::from(item.level) - 1;
        let indent = depth as f32 * 1.5 * size_px;
        let start = indent + hang[depth];
        let room = (bounds.width() - f64::from(start)).max(0.0);
        let mut layout = layout_box(
            item.text,
            fonts,
            style,
            dpi,
            Size::new(room, bounds.height()),
        )?;
        for line in &mut layout.lines {
            for c in &mut line.clusters {
                c.x += start;
            }
        }
        let height = layout.height;
        check_output(&layout)?;
        if !(indent.is_finite() && top.is_finite() && (top + height).is_finite()) {
            return Err(TextError::NonFinite { field: "output" });
        }
        entries.push(ListEntry {
            marker,
            indent,
            top,
            layout,
        });
        top += height;
    }
    Ok(entries)
}

/// Where the text starts after a marker and a space, in px from the marker: the `x` of a
/// letter put after them, so the spacing and the kerning of the style are counted.
fn text_start(
    marker: &str,
    fonts: &[&[u8]],
    style: &TextStyle,
    dpi: f64,
) -> Result<f32, TextError> {
    let probe = format!("{marker} x");
    let wide = Size::new(1e9, 1e9);
    let laid = layout_box(&probe, fonts, style, dpi, wide)?;
    Ok(laid
        .lines
        .first()
        .and_then(|l| l.clusters.iter().map(|c| c.x).reduce(f32::max))
        .unwrap_or(0.0))
}
