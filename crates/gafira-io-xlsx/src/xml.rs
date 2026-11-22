//! XML and SpreadsheetML spellings.

use std::fmt::Write as _;

/// `text` inside an element or a quoted attribute.
pub(crate) fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            _ => out.push(c),
        }
    }
    out
}

/// `text` as a SpreadsheetML string: a character XML does not hold goes as
/// `_xHHHH_`, and what already looks like that keeps its underscore.
pub(crate) fn string(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(c) = rest.chars().next() {
        let after = &rest[c.len_utf8()..];
        match c {
            '_' if looks_escaped(rest) => out.push_str("_x005F_"),
            '\t' | '\n' => out.push(c),
            '\r' => {}
            c if c.is_control() || matches!(c, '\u{FFFE}' | '\u{FFFF}') => {
                let _ = write!(out, "_x{:04X}_", u32::from(c));
            }
            _ => out.push(c),
        }
        rest = after;
    }
    escape(&out)
}

fn looks_escaped(text: &str) -> bool {
    let b = text.as_bytes();
    b.len() >= 7 && b[1] == b'x' && b[2..6].iter().all(u8::is_ascii_hexdigit) && b[6] == b'_'
}

/// 0 → `A`, 25 → `Z`, 26 → `AA`.
pub(crate) fn column(index: usize) -> String {
    let mut n = index + 1;
    let mut letters = Vec::new();
    while n > 0 {
        // Below 26: the cast is exact.
        letters.push(char::from(b'A' + ((n - 1) % 26) as u8));
        n = (n - 1) / 26;
    }
    letters.iter().rev().collect()
}

/// `A1` for the first cell.
pub(crate) fn address(row: usize, col: usize) -> String {
    format!("{}{}", column(col), row + 1)
}

/// The shortest decimal that reads back as `v`.
pub(crate) fn number(v: f64) -> String {
    if v.is_finite() {
        format!("{v}")
    } else {
        "0".to_owned()
    }
}

/// The stored width of a column of `px` (at 96 px an inch): characters of
/// the standard font, 7 px each.
pub(crate) fn column_width(px: f64) -> f64 {
    let px = px.round().clamp(1.0, 1790.0);
    (px / 7.0 * 256.0).trunc() / 256.0
}

/// The height of a row of `px`, in points.
pub(crate) fn row_height(px: f64) -> f64 {
    (px.max(0.0) * 0.75 * 100.0).round().min(40_950.0) / 100.0
}
