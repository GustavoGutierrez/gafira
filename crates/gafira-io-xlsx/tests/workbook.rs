//! S81-14: the Excel workbook of the tables.

use std::io::{Cursor, Read};

use gafira_color::{Color, Rgba};
use gafira_doc::{CellStyle, ShapeStroke, TableNode, TextAlign, VerticalAlign};
use gafira_io_xlsx::{Sheet, workbook};
use gafira_table::{BorderSet, Range, merge, set_borders};

fn rgb(r: f32, g: f32, b: f32) -> Color {
    Color::Srgb(Rgba::new(r, g, b, 1.0))
}

fn table(words: &[&[&str]]) -> TableNode {
    let style = CellStyle::new("Inter", 16.0, rgb(0.0, 0.0, 0.0));
    let mut t = TableNode::new(words.len(), words[0].len(), (96.0, 32.0), &style);
    for (r, row) in words.iter().enumerate() {
        for (c, text) in row.iter().enumerate() {
            t.cell_mut(r, c).expect("a cell").text = (*text).to_owned();
        }
    }
    t
}

fn parts(sheets: &[(&str, &TableNode)]) -> Vec<(String, String)> {
    let sheets: Vec<Sheet<'_>> = sheets
        .iter()
        .map(|(name, table)| Sheet {
            name,
            table,
            size: gafira_table::size(table),
            dpi: 96.0,
        })
        .collect();
    let bytes = workbook(&sheets).expect("a workbook");
    let mut zip = zip::ZipArchive::new(Cursor::new(bytes)).expect("a zip");
    (0..zip.len())
        .map(|i| {
            let mut file = zip.by_index(i).expect("a part");
            let mut text = String::new();
            file.read_to_string(&mut text).expect("utf-8");
            let name = file.name().expect("a name").into_owned();
            roxmltree::Document::parse(&text).unwrap_or_else(|e| panic!("{name}: {e}"));
            (name, text)
        })
        .collect()
}

fn part(parts: &[(String, String)], name: &str) -> String {
    parts
        .iter()
        .find(|(n, _)| n == name)
        .unwrap_or_else(|| panic!("{name}"))
        .1
        .clone()
}

/// The attribute `name` of the cell `at`, its type and its value.
fn cell(sheet: &str, at: &str) -> (Option<String>, Option<String>) {
    let doc = roxmltree::Document::parse(sheet).expect("XML");
    let c = doc
        .descendants()
        .find(|n| n.has_tag_name((NS, "c")) && n.attribute("r") == Some(at))
        .unwrap_or_else(|| panic!("the cell {at}"));
    let value = c
        .descendants()
        .filter(|n| n.has_tag_name((NS, "v")) || n.has_tag_name((NS, "t")))
        .find_map(|n| n.text().map(str::to_owned));
    (c.attribute("t").map(str::to_owned), value)
}

const NS: &str = "http://schemas.openxmlformats.org/spreadsheetml/2006/main";

#[test]
fn s81_14_numbers_are_numbers_and_words_are_words() {
    let t = table(&[
        &["Mes", "Ventas", "Código"],
        &["Enero", "1.234,50", "007"],
        &["=SUM(B2)", "15%", "a < b & \"c\""],
        &["", "$ 30", " espacio "],
    ]);
    let parts = parts(&[("Ventas", &t)]);
    let sheet = part(&parts, "xl/worksheets/sheet1.xml");
    let text = |s: &str| (Some("inlineStr".to_owned()), Some(s.to_owned()));
    assert_eq!(cell(&sheet, "A1"), text("Mes"));
    assert_eq!(cell(&sheet, "B2"), (None, Some("1234.5".to_owned())));
    assert_eq!(cell(&sheet, "C2"), text("007"));
    assert_eq!(cell(&sheet, "A3"), text("=SUM(B2)"));
    assert!(!sheet.contains("<f>"), "never a formula");
    assert_eq!(cell(&sheet, "B3"), (None, Some("0.15".to_owned())));
    assert_eq!(cell(&sheet, "C3"), text("a < b & \"c\""));
    assert_eq!(cell(&sheet, "B4"), (None, Some("30".to_owned())));
    assert!(sheet.contains("xml:space=\"preserve\"> espacio <"));
    assert_eq!(
        cell(&sheet, "A4"),
        (None, None),
        "an empty one keeps its looks"
    );
    assert!(sheet.contains("<dimension ref=\"A1:C4\"/>"));
}

#[test]
fn s81_14_control_characters_do_not_break_the_file() {
    let t = table(&[&["a\u{1}b", "c\nd", "_x0041_"]]);
    let parts = parts(&[("Hoja", &t)]);
    let sheet = part(&parts, "xl/worksheets/sheet1.xml");
    assert!(sheet.contains("a_x0001_b"));
    assert!(sheet.contains("_x005F_x0041_"), "not read as an escape");
    assert!(sheet.contains("c\nd"));
}

#[test]
fn s81_14_sizes_merges_and_looks() {
    let mut t = table(&[&["a", "b"], &["c", "d"]]);
    t.columns = vec![96.0, 192.0];
    t.rows = vec![32.0, 64.0];
    merge(&mut t, Range::new((1, 0), (1, 1)));
    let style = &mut t.cell_mut(0, 0).expect("a cell").style;
    style.fill = Some(rgb(1.0, 0.0, 0.0));
    style.color = rgb(0.0, 0.0, 1.0);
    style.bold = true;
    style.align = TextAlign::Right;
    style.valign = VerticalAlign::Top;
    let pen = ShapeStroke::new(rgb(0.0, 1.0, 0.0), 4.0);
    set_borders(&mut t, Range::cell(0, 0), BorderSet::Right, Some(pen));
    let parts = parts(&[("Hoja", &t)]);
    let sheet = part(&parts, "xl/worksheets/sheet1.xml");
    let doc = roxmltree::Document::parse(&sheet).expect("XML");
    let widths: Vec<f64> = doc
        .descendants()
        .filter(|n| n.has_tag_name((NS, "col")))
        .map(|n| n.attribute("width").expect("w").parse().expect("a number"))
        .collect();
    assert!((widths[0] - 13.71).abs() < 0.02, "{widths:?}: 96 px");
    assert!((widths[1] - 27.43).abs() < 0.02, "{widths:?}: 192 px");
    assert!(
        sheet.contains("ht=\"24\"") && sheet.contains("ht=\"48\""),
        "points"
    );
    assert!(sheet.contains("<mergeCell ref=\"A2:B2\"/>"));
    let styles = part(&parts, "xl/styles.xml");
    for piece in [
        "<fgColor rgb=\"FFFF0000\"/>",
        "<color rgb=\"FF0000FF\"/>",
        "<b/>",
        "horizontal=\"right\"",
        "vertical=\"top\"",
        "<right style=\"thick\"><color rgb=\"FF00FF00\"/></right>",
        "<sz val=\"12\"/>",
        "<name val=\"Inter\"/>",
        "wrapText=\"1\"",
    ] {
        assert!(styles.contains(piece), "{piece}\n{styles}");
    }
}

#[test]
fn s81_14_a_sheet_a_table_with_names_excel_takes() {
    let t = table(&[&["a"]]);
    let long = "Un nombre de hoja demasiado largo para Excel";
    let parts = parts(&[
        ("Plan/2026: [x]", &t),
        ("plan 2026  x ", &t),
        (long, &t),
        ("", &t),
        ("", &t),
    ]);
    let book = part(&parts, "xl/workbook.xml");
    let doc = roxmltree::Document::parse(&book).expect("XML");
    let names: Vec<&str> = doc
        .descendants()
        .filter(|n| n.has_tag_name((NS, "sheet")))
        .filter_map(|n| n.attribute("name"))
        .collect();
    assert_eq!(names.len(), 5);
    let mut seen: Vec<String> = names.iter().map(|n| n.to_lowercase()).collect();
    seen.sort();
    seen.dedup();
    assert_eq!(seen.len(), 5, "no two alike: {names:?}");
    for name in names {
        assert!((1..=31).contains(&name.chars().count()), "{name}");
        assert!(
            !name.contains(['/', '\\', '?', '*', '[', ']', ':']),
            "{name}"
        );
    }
    assert!(workbook(&[]).is_err(), "a workbook has a sheet at least");
}

#[test]
fn s81_14_a_header_row_stays_in_sight() {
    let mut t = table(&[&["Mes", "Ventas"], &["Enero", "1"]]);
    t.header = true;
    let parts = parts(&[("Hoja", &t)]);
    let sheet = part(&parts, "xl/worksheets/sheet1.xml");
    assert!(sheet.contains("<pane ySplit=\"1\" topLeftCell=\"A2\""));
}
