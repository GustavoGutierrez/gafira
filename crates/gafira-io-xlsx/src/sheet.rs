//! A worksheet part.

use std::fmt::Write as _;

use gafira_table::{CellValue, Layout, anchors, value};

use crate::styles::{Styles, looks};
use crate::xml::{address, column_width, number, row_height, string};
use crate::{DECL, MAIN, Sheet};

/// The worksheet of `sheet`; its looks go to `styles`.
pub(crate) fn write(sheet: &Sheet<'_>, styles: &mut Styles) -> String {
    let t = sheet.table;
    let to_px = if sheet.dpi.is_finite() && sheet.dpi > 0.0 {
        96.0 / sheet.dpi
    } else {
        1.0
    };
    let layout = Layout::of(t, sheet.size);
    let (rows, cols) = (t.rows.len(), t.columns.len());
    let last = address(rows.saturating_sub(1), cols.saturating_sub(1));
    let mut out = format!(
        "{DECL}<worksheet xmlns=\"{MAIN}\"><sheetPr><pageSetUpPr fitToPage=\"1\"/></sheetPr><dimension ref=\"A1:{last}\"/>"
    );
    // A row that names the columns stays in sight.
    if t.header && rows > 1 {
        out.push_str("<sheetViews><sheetView workbookViewId=\"0\"><pane ySplit=\"1\" topLeftCell=\"A2\" activePane=\"bottomLeft\" state=\"frozen\"/></sheetView></sheetViews>");
    }
    out.push_str("<cols>");
    for (i, (x0, x1)) in layout.columns.iter().enumerate() {
        let width = column_width((x1 - x0) * to_px);
        let n = i + 1;
        let _ = write!(
            out,
            "<col min=\"{n}\" max=\"{n}\" width=\"{}\" customWidth=\"1\"/>",
            number(width)
        );
    }
    out.push_str("</cols><sheetData>");
    let seen: Vec<(usize, usize)> = anchors(t).collect();
    for (r, (y0, y1)) in layout.rows.iter().enumerate() {
        let _ = write!(
            out,
            "<row r=\"{}\" ht=\"{}\" customHeight=\"1\">",
            r + 1,
            number(row_height((y1 - y0) * to_px))
        );
        for c in 0..cols {
            let Some(cell) = t.cell(r, c) else { continue };
            let at = address(r, c);
            // A cell a merge hides keeps its borders and says nothing.
            let words = if seen.contains(&(r, c)) {
                cell.text.as_str()
            } else {
                ""
            };
            let value = value(words);
            let format = match value {
                CellValue::Number(_, format) => Some((format, words)),
                CellValue::Empty | CellValue::Text => None,
            };
            let s = styles.id(&looks(t, r, c, to_px), format);
            let _ = match value {
                CellValue::Empty => write!(out, "<c r=\"{at}\" s=\"{s}\"/>"),
                CellValue::Number(n, _) => {
                    write!(out, "<c r=\"{at}\" s=\"{s}\"><v>{}</v></c>", number(n))
                }
                CellValue::Text => write!(
                    out,
                    "<c r=\"{at}\" s=\"{s}\" t=\"inlineStr\"><is><t xml:space=\"preserve\">{}</t></is></c>",
                    string(words)
                ),
            };
        }
        out.push_str("</row>");
    }
    out.push_str("</sheetData>");
    if !t.merges.is_empty() {
        let _ = write!(out, "<mergeCells count=\"{}\">", t.merges.len());
        for m in &t.merges {
            let _ = write!(
                out,
                "<mergeCell ref=\"{}:{}\"/>",
                address(m.row, m.col),
                address(m.row + m.rows - 1, m.col + m.cols - 1)
            );
        }
        out.push_str("</mergeCells>");
    }
    // Printed on one page across, as many as it takes down.
    out.push_str(
        "<pageSetup orientation=\"landscape\" fitToWidth=\"1\" fitToHeight=\"0\"/></worksheet>",
    );
    out
}
