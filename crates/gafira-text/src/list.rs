//! Bulleted and numbered lists (FD-41, Listas).

use gafira_geom::Size;

use crate::layout::check_input;
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
    /// The marker, such as `•` or `2.`.
    pub marker: String,
    /// Indent of the item from the left of the box, in px: 1.5 em per level below the first.
    pub indent: f32,
    /// Distance from the top of the list to the top of the item, in px.
    pub top: f32,
    /// The marker, a space and the text, laid out in the box minus the indent; the `x` of
    /// its clusters already includes the indent.
    pub layout: TextLayout,
}

/// Lays a list out, one entry per item, stacked from the top.
///
/// The numbers and the font stack are checked as in [`layout_box`]; then a level outside
/// 1 to 4 gives `Err(OutOfRange)` with the field `items.level`. Numbering counts per level
/// and restarts after an item of a shallower level.
pub fn layout_list(
    items: &[ListItem<'_>],
    fonts: &[&[u8]],
    style: &TextStyle,
    dpi: f64,
    bounds: Size,
) -> Result<Vec<ListEntry>, TextError> {
    let size_px = check_input(style, dpi, bounds)?;
    if items.iter().any(|i| !(1..=MAX_LEVEL).contains(&i.level)) {
        return Err(TextError::OutOfRange {
            field: "items.level",
        });
    }
    if fonts.is_empty() {
        return Err(TextError::NoFont);
    }
    let mut counters = [0_u32; MAX_LEVEL as usize];
    let mut top = 0.0_f32;
    let mut entries = Vec::with_capacity(items.len());
    for item in items {
        let depth = usize::from(item.level) - 1;
        counters[depth + 1..].fill(0);
        let marker = match item.kind {
            ListKind::Bullet => {
                counters[depth] = 0;
                BULLETS[depth % BULLETS.len()].to_string()
            }
            ListKind::Numbered => {
                counters[depth] += 1;
                format!("{}.", counters[depth])
            }
        };
        let indent = depth as f32 * 1.5 * size_px;
        let room = (bounds.width() - f64::from(indent)).max(0.0);
        let text = format!("{marker} {}", item.text);
        let mut layout = layout_box(&text, fonts, style, dpi, Size::new(room, bounds.height()))?;
        for line in &mut layout.lines {
            for c in &mut line.clusters {
                c.x += indent;
            }
        }
        let height = layout.height;
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
