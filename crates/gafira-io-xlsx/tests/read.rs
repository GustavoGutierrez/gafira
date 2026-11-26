//! S82: a workbook read as tables.

use std::io::{Cursor, Write};

use gafira_color::{Color, Rgba};
use gafira_doc::{CellMerge, CellStyle, ShapeStroke, TableNode, TextAlign, VerticalAlign};
use gafira_io_xlsx::{Locale, ReadError, ReadSheet, Sheet, read, read_csv, show, workbook};
use gafira_table::{BorderSet, CellValue, Range, set_borders, value};
use proptest::prelude::*;

const ES: Locale = Locale {
    comma: true,
    yes: "VERDADERO",
    no: "FALSO",
};
const EN: Locale = Locale {
    comma: false,
    yes: "TRUE",
    no: "FALSE",
};

fn rgb(code: u32) -> Color {
    let channel = |shift: u32| ((code >> shift) & 0xFF) as f32 / 255.0;
    Color::Srgb(Rgba::new(channel(16), channel(8), channel(0), 1.0))
}

/// `#RRGGBB` of a colour.
fn hex(color: Color) -> String {
    let Color::Srgb(c) = color else {
        panic!("sRGB")
    };
    let byte = |v: f32| (v * 255.0).round() as u8;
    format!("{:02X}{:02X}{:02X}", byte(c.r), byte(c.g), byte(c.b))
}

fn texts(sheet: &ReadSheet) -> Vec<Vec<&str>> {
    sheet
        .table
        .cells
        .chunks(sheet.table.columns.len())
        .map(|row| row.iter().map(|c| c.text.as_str()).collect())
        .collect()
}

/// A workbook of these parts; what is not given is as small as can be.
fn book(sheets: &[(&str, &str)], parts: &[(&str, &str)]) -> Vec<u8> {
    let mut all: Vec<(String, String)> = Vec::new();
    let (mut list, mut rels) = (String::new(), String::new());
    for (i, (name, xml)) in sheets.iter().enumerate() {
        let n = i + 1;
        let (name, state) = name.split_once('|').unwrap_or((name, ""));
        list.push_str(&format!(
            "<sheet name=\"{name}\" sheetId=\"{n}\" r:id=\"r{n}\" {state}/>"
        ));
        rels.push_str(&format!("<Relationship Id=\"r{n}\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet\" Target=\"worksheets/s{n}.xml\"/>"));
        all.push((
            format!("xl/worksheets/s{n}.xml"),
            format!("<worksheet xmlns=\"{NS}\" xmlns:r=\"{REL}\">{xml}</worksheet>"),
        ));
    }
    let extra: String = [
        ("sharedStrings", "sharedStrings.xml"),
        ("styles", "styles.xml"),
        ("theme", "theme/theme1.xml"),
    ]
    .iter()
    .filter(|(_, file)| parts.iter().any(|(name, _)| name.ends_with(file)))
    .map(|(kind, file)| {
        format!("<Relationship Id=\"x{kind}\" Type=\"{REL}/{kind}\" Target=\"{file}\"/>")
    })
    .collect();
    let workbook = parts
        .iter()
        .find(|(n, _)| *n == "workbookPr")
        .map_or("", |(_, x)| *x);
    all.push(("xl/workbook.xml".into(), format!("<workbook xmlns=\"{NS}\" xmlns:r=\"{REL}\">{workbook}<sheets>{list}</sheets></workbook>")));
    all.push((
        "xl/_rels/workbook.xml.rels".into(),
        format!("<Relationships xmlns=\"{PKG}\">{rels}{extra}</Relationships>"),
    ));
    all.push(("_rels/.rels".into(), format!("<Relationships xmlns=\"{PKG}\"><Relationship Id=\"a\" Type=\"{REL}/officeDocument\" Target=\"xl/workbook.xml\"/></Relationships>")));
    for (name, xml) in parts.iter().filter(|(n, _)| *n != "workbookPr") {
        all.push(((*name).to_owned(), (*xml).to_owned()));
    }
    let mut once: std::collections::BTreeMap<String, String> = std::collections::BTreeMap::new();
    once.extend(all);
    zipped(&once.into_iter().collect::<Vec<_>>())
}

fn zipped(parts: &[(String, String)]) -> Vec<u8> {
    let mut out = Cursor::new(Vec::new());
    let mut zip = zip::ZipWriter::new(&mut out);
    for (name, text) in parts {
        zip.start_file(name.as_str(), zip::write::SimpleFileOptions::default())
            .expect("a part");
        zip.write_all(text.as_bytes()).expect("written");
    }
    zip.finish().expect("a zip");
    out.into_inner()
}

const NS: &str = "http://schemas.openxmlformats.org/spreadsheetml/2006/main";
const REL: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";
const PKG: &str = "http://schemas.openxmlformats.org/package/2006/relationships";

fn one(xml: &str, parts: &[(&str, &str)]) -> ReadSheet {
    let mut sheets = read(&book(&[("Hoja", xml)], parts), ES).expect("read");
    assert_eq!(sheets.len(), 1);
    sheets.remove(0)
}

// S82-01, S82-02: what Gafira writes it reads back: words, numbers, sizes,
// merges and looks.
#[test]
fn s82_01_what_is_written_reads_back() {
    let mut t = TableNode::new(
        2,
        3,
        (140.0, 40.0),
        &CellStyle::new("Inter", 16.0, rgb(0x112233)),
    );
    for (cell, words) in t
        .cells
        .iter_mut()
        .zip(["Mes", "1234,5", "007", "a\nb", "15%", "=1+1"])
    {
        cell.text = words.to_owned();
    }
    let style = &mut t.cells[0].style;
    style.fill = Some(rgb(0xFF8800));
    style.bold = true;
    style.align = TextAlign::Right;
    style.valign = VerticalAlign::Top;
    set_borders(
        &mut t,
        Range::cell(0, 0),
        BorderSet::All,
        Some(ShapeStroke::new(rgb(0x0000FF), 2.0)),
    );
    t.columns = vec![140.0, 70.0, 210.0];
    let size = gafira_table::size(&t);
    let bytes = workbook(&[Sheet {
        name: "Datos",
        table: &t,
        size,
        dpi: 96.0,
    }])
    .expect("written");
    let sheets = read(&bytes, ES).expect("read");
    let back = &sheets[0];
    assert_eq!(back.name, "Datos");
    assert_eq!(
        texts(back),
        [["Mes", "1234,5", "007"], ["a\nb", "15%", "=1+1"]]
    );
    let (a, b) = (&back.table.cells[0].style, &t.cells[0].style);
    assert_eq!(
        (a.fill, a.color, a.bold, a.align, a.valign),
        (b.fill, b.color, true, b.align, b.valign)
    );
    assert_eq!((a.font.as_str(), a.size), ("Inter", 16.0));
    assert_eq!(a.borders.top, b.borders.top);
    for (x, y) in back.table.columns.iter().zip(&t.columns) {
        assert!((x - y).abs() < 1.5, "{:?}", back.table.columns);
    }
    assert!((back.size.1 - 80.0).abs() < 1.0);
    assert!(back.table.is_valid());
}

// S82-01: each kind of cell: shared and rich words, inline ones, numbers,
// the saved value of a formula, logical ones and errors.
#[test]
fn s82_01_kinds_of_cells() {
    let sheet = one(
        "<sheetData><row r=\"1\"><c r=\"A1\" t=\"s\"><v>1</v></c><c r=\"B1\" t=\"inlineStr\"><is><t>dentro</t></is></c><c r=\"C1\"><v>3.5</v></c></row>\
         <row r=\"2\"><c r=\"A2\"><f>C1*2</f><v>7</v></c><c r=\"B2\" t=\"str\"><f>\"x\"</f><v>texto</v></c><c r=\"C2\" t=\"b\"><v>1</v></c></row>\
         <row r=\"3\"><c r=\"A3\" t=\"e\"><v>#DIV/0!</v></c><c r=\"B3\" t=\"b\"><v>0</v></c><c r=\"C3\" t=\"s\"><v>99</v></c></row></sheetData>",
        &[(
            "xl/sharedStrings.xml",
            &format!(
                "<sst xmlns=\"{NS}\"><si><t>cero</t></si><si><r><t>ri</t></r><r><rPr><b/></rPr><t>co_x000A_</t></r><rPh sb=\"0\" eb=\"1\"><t>no</t></rPh></si></sst>"
            ),
        )],
    );
    assert_eq!(
        texts(&sheet),
        [
            ["rico\n", "dentro", "3,5"],
            ["7", "texto", "VERDADERO"],
            ["#DIV/0!", "FALSO", ""]
        ]
    );
}

// S82-01: formats of numbers and dates.
#[test]
fn s82_01_numbers_are_shown_as_their_format_says() {
    let es = |v: f64, code: &str| show(v, code, false, ES);
    let en = |v: f64, code: &str| show(v, code, false, EN);
    assert_eq!(es(1234.5, "General"), "1234,5");
    assert_eq!(en(1234.5, "General"), "1234.5");
    assert_eq!(en(0.1 + 0.2, "General"), "0.3");
    assert_eq!(en(-7.0, "0"), "-7");
    assert_eq!(es(1234.5, "#,##0.00"), "1.234,50");
    assert_eq!(en(1_234_567.0, "#,##0"), "1,234,567");
    assert_eq!(
        en(1234.0, "#,##0"),
        "1234",
        "with one sign it could be a decimal"
    );
    assert_eq!(es(0.125, "0.0%"), "12,5%");
    assert_eq!(en(30.0, "\"$\"#,##0.00"), "$30.00");
    assert_eq!(es(30.0, "#,##0.00\\ [$€-C0A]"), "30,00 €");
    assert_eq!(en(-5.0, "0.00;(0.00)"), "(5.00)");
    assert_eq!(en(0.0, "0;-0;\"—\""), "—");
    assert_eq!(en(5.0, "[Red]0"), "5");
    assert_eq!(en(5.0, "0\" kg\""), "5 kg");
    assert_eq!(en(5.0, "@"), "5");
    assert_eq!(en(12345.678, "0.00E+00"), "12345.678", "in full");
    // 45356 is the fifth of March of 2024.
    assert_eq!(es(45356.0, "dd/mm/yyyy"), "05/03/2024");
    assert_eq!(es(45356.0, "m/d/yy"), "3/5/24");
    assert_eq!(es(45356.75, "yyyy-mm-dd hh:mm"), "2024-03-05 18:00");
    assert_eq!(es(45356.75, "h:mm AM/PM"), "6:00 PM");
    assert_eq!(es(0.5, "[h]:mm"), "12:00");
    assert_eq!(es(1.0, "dd/mm/yyyy"), "01/01/1900");
    assert_eq!(
        es(60.0, "dd/mm/yyyy"),
        "29/02/1900",
        "the day Excel made up"
    );
    assert_eq!(es(61.0, "dd/mm/yyyy"), "01/03/1900");
    assert_eq!(show(0.0, "dd/mm/yyyy", true, ES), "01/01/1904");
    assert_eq!(es(-1.0, "dd/mm/yyyy"), "-1", "not a date");
    assert_eq!(es(f64::NAN, "0"), "");
}

proptest! {
    // S82-01: what is shown as a number reads back as that number.
    #[test]
    fn s82_01_a_shown_number_reads_back(
        n in -9_999_999_i64..9_999_999,
        decimals in 0_usize..5,
        which in 0_usize..5,
        comma in any::<bool>(),
    ) {
        let v = n as f64 / 1000.0;
        let zeros = "0".repeat(decimals);
        let point = if decimals > 0 { "." } else { "" };
        let code = match which {
            0 => "General".to_owned(),
            1 => format!("0{point}{zeros}"),
            2 => format!("#,##0{point}{zeros}"),
            3 => format!("0{point}{zeros}%"),
            _ => format!("\"$\"#,##0{point}{zeros}"),
        };
        let locale = Locale { comma, ..EN };
        let words = show(v, &code, false, locale);
        let CellValue::Number(back, _) = value(&words) else {
            return Err(TestCaseError::fail(format!("{v} as {code} is {words}")));
        };
        let step = match which { 0 => 1e-3, 3 => 10_f64.powi(-(decimals as i32) - 2), _ => 10_f64.powi(-(decimals as i32)) };
        prop_assert!((back - v).abs() <= step * 0.500_001 + 1e-9, "{v} as {code} is {words}: {back}");
    }
}

// S82-02: fonts, fills, borders and alignment; colours of the theme with a
// tint, indexed ones and their own.
#[test]
fn s82_02_looks() {
    let styles = format!(
        "<styleSheet xmlns=\"{NS}\"><numFmts count=\"1\"><numFmt numFmtId=\"170\" formatCode=\"0.0\"/></numFmts>\
         <fonts><font><sz val=\"11\"/><name val=\"Calibri\"/></font><font><b/><i val=\"0\"/><sz val=\"12\"/><color theme=\"4\"/><name val=\"Arial\"/></font></fonts>\
         <fills><fill><patternFill patternType=\"none\"/></fill><fill><patternFill patternType=\"gray125\"/></fill><fill><patternFill patternType=\"solid\"><fgColor theme=\"4\" tint=\"0.5\"/></patternFill></fill><fill><patternFill patternType=\"solid\"><fgColor indexed=\"10\"/></patternFill></fill></fills>\
         <borders><border/><border><left/><right style=\"medium\"><color rgb=\"FF00FF00\"/></right><top/><bottom style=\"thin\"><color auto=\"1\"/></bottom></border></borders>\
         <cellXfs><xf numFmtId=\"0\" fontId=\"0\" fillId=\"0\" borderId=\"0\"/><xf numFmtId=\"170\" fontId=\"1\" fillId=\"2\" borderId=\"1\"><alignment horizontal=\"center\" vertical=\"top\"/></xf><xf numFmtId=\"0\" fontId=\"0\" fillId=\"3\" borderId=\"0\"/></cellXfs></styleSheet>"
    );
    let theme = "<a:theme xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\"><a:themeElements><a:clrScheme name=\"x\"><a:dk1><a:sysClr val=\"windowText\" lastClr=\"000000\"/></a:dk1><a:lt1><a:sysClr val=\"window\" lastClr=\"FFFFFF\"/></a:lt1><a:dk2><a:srgbClr val=\"111111\"/></a:dk2><a:lt2><a:srgbClr val=\"EEEEEE\"/></a:lt2><a:accent1><a:srgbClr val=\"0000FF\"/></a:accent1></a:clrScheme></a:themeElements></a:theme>";
    let sheet = one(
        "<cols><col min=\"1\" max=\"1\" width=\"20\" customWidth=\"1\"/></cols><sheetData><row r=\"1\" ht=\"30\" customHeight=\"1\"><c r=\"A1\" s=\"1\"><v>2</v></c><c r=\"B1\" s=\"2\" t=\"inlineStr\"><is><t>x</t></is></c><c r=\"C1\"><v>1</v></c></row></sheetData>",
        &[("xl/styles.xml", &styles), ("xl/theme/theme1.xml", theme)],
    );
    let cell = |i: usize| &sheet.table.cells[i];
    let a = &cell(0).style;
    assert_eq!(cell(0).text, "2,0");
    assert_eq!(
        (a.font.as_str(), a.size, a.bold, a.italic),
        ("Arial", 16.0, true, false)
    );
    assert_eq!(hex(a.color), "0000FF");
    assert_eq!(hex(a.fill.expect("a fill")), "8080FF");
    assert_eq!((a.align, a.valign), (TextAlign::Center, VerticalAlign::Top));
    let right = a.borders.right.expect("a border");
    assert_eq!((hex(right.color), right.width), ("00FF00".to_owned(), 2.0));
    assert_eq!(hex(a.borders.bottom.expect("a border").color), "000000");
    assert!(a.borders.top.is_none());
    assert_eq!(hex(cell(1).style.fill.expect("a fill")), "FF0000");
    assert_eq!(
        (cell(1).style.align, cell(2).style.align),
        (TextAlign::Left, TextAlign::Right)
    );
    assert!(
        (sheet.table.columns[0] - 140.0).abs() < 2.0,
        "{:?}",
        sheet.table.columns
    );
    assert!((sheet.table.rows[0] - 40.0).abs() < 0.5);
}

// S82-02: a sheet with no border at all gets thin lines; a table with a
// format of Excel keeps its header and its bands.
#[test]
fn s82_02_lines_and_formatted_tables() {
    let rows = "<sheetData><row r=\"1\"><c r=\"A1\" t=\"inlineStr\"><is><t>Mes</t></is></c></row><row r=\"2\"><c r=\"A2\"><v>1</v></c></row><row r=\"3\"><c r=\"A3\"><v>2</v></c></row></sheetData>";
    let plain = one(rows, &[]);
    assert!(plain.table.cells[0].style.borders.top.is_some());
    assert!(!plain.table.header);
    let with_table = one(
        &format!("{rows}<tableParts count=\"1\"><tablePart r:id=\"t1\"/></tableParts>"),
        &[
            (
                "xl/worksheets/_rels/s1.xml.rels",
                &format!(
                    "<Relationships xmlns=\"{PKG}\"><Relationship Id=\"t1\" Type=\"{REL}/table\" Target=\"../tables/table1.xml\"/></Relationships>"
                ),
            ),
            (
                "xl/tables/table1.xml",
                &format!(
                    "<table xmlns=\"{NS}\" ref=\"A1:A3\"><tableStyleInfo name=\"TableStyleMedium2\" showRowStripes=\"1\"/></table>"
                ),
            ),
        ],
    );
    let fill = |i: usize| with_table.table.cells[i].style.fill;
    assert!(with_table.table.header && with_table.table.cells[0].style.bold);
    assert!(
        fill(0).is_some() && fill(1).is_some() && fill(2).is_none(),
        "a header and bands"
    );
}

// S82-03: only what is used, less what is hidden, within the limits.
#[test]
fn s82_03_the_area_in_use() {
    let sheet = one(
        "<cols><col min=\"4\" max=\"4\" hidden=\"1\"/></cols><sheetData><row r=\"3\"><c r=\"C3\"><v>1</v></c><c r=\"D3\"><v>9</v></c><c r=\"E3\"><v>2</v></c></row><row r=\"4\" hidden=\"1\"><c r=\"C4\"><v>9</v></c></row><row r=\"5\"><c r=\"C5\"><v>3</v></c></row><row r=\"90\"><c r=\"Z90\" s=\"0\"/></row></sheetData><mergeCells><mergeCell ref=\"C5:E5\"/></mergeCells>",
        &[],
    );
    assert_eq!(texts(&sheet), [["1", "2"], ["3", ""]]);
    assert_eq!(
        sheet.table.merges,
        vec![CellMerge {
            row: 1,
            col: 0,
            rows: 1,
            cols: 2
        }]
    );
    let rows: String = (1..=80)
        .map(|r| {
            let cells: String = (0..30)
                .map(|c| format!("<c r=\"{}{r}\"><v>{c}</v></c>", column(c)))
                .collect();
            format!("<row r=\"{r}\">{cells}</row>")
        })
        .collect();
    let big = one(&format!("<sheetData>{rows}</sheetData>"), &[]);
    assert_eq!((big.table.rows.len(), big.table.columns.len()), (50, 20));
    assert_eq!(big.used, (80, 30));
    // S83-06: a chart takes more rows than a table.
    assert_eq!(
        (
            big.data.len(),
            big.data[79].len(),
            big.data[79][19].as_str()
        ),
        (80, 20, "19")
    );
    assert!(big.table.is_valid());
}

fn column(index: usize) -> String {
    let letter = |i: usize| char::from(b'A' + i as u8).to_string();
    if index < 26 {
        letter(index)
    } else {
        format!("{}{}", letter(index / 26 - 1), letter(index % 26))
    }
}

// S82-06: the visible sheets with something in them, in order; the hidden
// ones only when there is no other.
#[test]
fn s82_06_sheets() {
    let full = "<sheetData><row r=\"1\"><c r=\"A1\"><v>1</v></c></row></sheetData>";
    let names = |sheets: &[(&str, &str)]| {
        read(&book(sheets, &[]), ES).map(|s| s.into_iter().map(|s| s.name).collect::<Vec<_>>())
    };
    assert_eq!(
        names(&[
            ("Uno", full),
            ("Vacía", "<sheetData/>"),
            ("Oculta|state=\"hidden\"", full),
            ("Dos &amp; tres", full)
        ]),
        Ok(vec!["Uno".to_owned(), "Dos & tres".to_owned()])
    );
    assert_eq!(
        names(&[("Oculta|state=\"veryHidden\"", full)]),
        Ok(vec!["Oculta".to_owned()])
    );
    assert_eq!(names(&[("Vacía", "<sheetData/>")]), Err(ReadError::Empty));
}

// S82-04, S82-07: what is not a workbook says what it is.
#[test]
fn s82_04_hostile_files() {
    assert_eq!(read(b"", ES), Err(ReadError::Damaged));
    assert_eq!(read(b"PK\x03\x04 nada", ES), Err(ReadError::Damaged));
    let ole = [0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1, 0, 0];
    assert_eq!(read(&ole, ES), Err(ReadError::Unsupported));
    let full = "<sheetData><row r=\"1\"><c r=\"A1\"><v>1</v></c></row></sheetData>";
    let good = book(&[("Hoja", full)], &[]);
    for end in (0..good.len()).step_by(7) {
        let _ = read(&good[..end], ES);
    }
    // A DTD is refused, not expanded.
    let bomb = zipped(&[
        (
            "_rels/.rels".into(),
            format!(
                "<Relationships xmlns=\"{PKG}\"><Relationship Id=\"a\" Type=\"{REL}/officeDocument\" Target=\"xl/workbook.xml\"/></Relationships>"
            ),
        ),
        (
            "xl/workbook.xml".into(),
            "<!DOCTYPE x [<!ENTITY a \"aaaa\">]><workbook>&a;</workbook>".into(),
        ),
    ]);
    assert_eq!(read(&bomb, ES), Err(ReadError::Damaged));
    // A part that expands past the limit.
    let huge = book(
        &[(
            "Hoja",
            &format!("<sheetData>{}</sheetData>", " ".repeat(40 << 20)),
        )],
        &[],
    );
    assert!(huge.len() < 200_000, "it is small packed");
    assert_eq!(read(&huge, ES), Err(ReadError::TooLarge));
    // Far, wrong and repeated addresses; merges over each other.
    let odd = one(
        "<sheetData><row r=\"1\"><c r=\"A1\"><v>1</v></c><c r=\"A1\"><v>2</v></c><c r=\"ZZZZZ9\"><v>3</v></c><c r=\"1A\"><v>4</v></c><c><v>5</v></c><c r=\"B1\" s=\"4294967295\"><v>x</v></c></row><row r=\"99999999999\"><c r=\"A1\"><v>6</v></c></row><row r=\"2\"><c r=\"A2\"><v>7</v></c></row></sheetData><mergeCells><mergeCell ref=\"A1:B2\"/><mergeCell ref=\"B1:B2\"/><mergeCell ref=\"nada\"/><mergeCell ref=\"A1:XFD1048576\"/></mergeCells>",
        &[],
    );
    assert!(odd.table.is_valid());
    // A path that leaves the package is just not there.
    let escape = book(&[("Hoja", full)], &[("xl/_rels/workbook.xml.rels", "no")]);
    let _ = read(&escape, ES);
}

proptest! {
    // S82-04: any damage is an error or a valid table.
    #[test]
    fn s82_04_any_byte_changed(at in any::<prop::sample::Index>(), byte in any::<u8>()) {
        let full = "<sheetData><row r=\"1\"><c r=\"A1\"><v>1</v></c></row></sheetData>";
        let mut bytes = book(&[("Hoja", full)], &[]);
        let at = at.index(bytes.len());
        bytes[at] = byte;
        if let Ok(sheets) = read(&bytes, ES) {
            prop_assert!(sheets.iter().all(|s| s.table.is_valid()));
        }
    }
}

fn csv(text: &str) -> ReadSheet {
    read_csv(text.as_bytes(), "datos").expect("read")
}

// S82-08: the separator is found or told; quotes keep separators and lines.
#[test]
fn s82_08_separators_and_quotes() {
    let want = [["a", "b"], ["1", "2"]];
    for text in [
        "a,b\n1,2",
        "a;b\r\n1;2\r\n",
        "a\tb\n1\t2\n",
        "a|b\r1|2",
        "sep=;\na;b\n1;2\n",
    ] {
        assert_eq!(texts(&csv(text)), want, "{text:?}");
    }
    assert_eq!(
        texts(&csv("x;y\n1,5;2,5\n3,5;4,5")),
        [["x", "y"], ["1,5", "2,5"], ["3,5", "4,5"]]
    );
    assert_eq!(
        texts(&csv("\"a, \"\"b\"\"\",\"dos\r\nlineas\"\nsolo\n")),
        [["a, \"b\"", "dos\nlineas"], ["solo", ""]]
    );
    assert_eq!(texts(&csv("uno\ndos")), [["uno"], ["dos"]]);
    assert_eq!(
        texts(&csv("5\" x,b\n\"sin cerrar,c")),
        [["5\" x", "b"], ["sin cerrar,c", ""]]
    );
}

// S82-08: the words stay as they are; nothing is a formula.
#[test]
fn s82_08_words_looks_and_area() {
    let s = csv("Mes,Ventas\nEnero,007\nFebrero,12.5\n=1+1,\n,,\n\n");
    assert_eq!(
        texts(&s),
        [
            ["Mes", "Ventas"],
            ["Enero", "007"],
            ["Febrero", "12.5"],
            ["=1+1", ""]
        ]
    );
    assert!(s.table.header && s.table.cells[0].style.bold);
    assert_eq!(
        (s.table.cells[5].style.align, s.table.cells[4].style.align),
        (TextAlign::Right, TextAlign::Left)
    );
    assert!(s.table.cells[0].style.borders.top.is_some());
    assert_eq!(s.name, "datos");
    let wide = csv("a,una columna bastante mas ancha\nb,c");
    assert!(wide.table.columns[1] > wide.table.columns[0]);
    assert!(!wide.table.header, "no numbers below: it may be data");
    let row = vec!["1"; 30].join(",");
    let big = csv(&vec![row; 80].join("\n"));
    assert_eq!(
        (big.table.rows.len(), big.table.columns.len(), big.used),
        (50, 20, (80, 30))
    );
    assert_eq!((big.data.len(), big.data[79].len()), (80, 20));
    assert_eq!(s.data[2], ["Febrero", "12.5"]);
}

// S82-08: encodings.
#[test]
fn s82_08_encodings() {
    let utf16 = |le: bool| -> Vec<u8> {
        "\u{FEFF}año;€\n1;2"
            .encode_utf16()
            .flat_map(|u| if le { u.to_le_bytes() } else { u.to_be_bytes() })
            .collect()
    };
    for bytes in [
        utf16(true),
        utf16(false),
        "\u{FEFF}año;€\n1;2".as_bytes().to_vec(),
        b"a\xF1o;\x80\n1;2".to_vec(),
    ] {
        assert_eq!(
            texts(&read_csv(&bytes, "x").expect("read")),
            [["año", "€"], ["1", "2"]]
        );
    }
    assert_eq!(read_csv(b"", "x"), Err(ReadError::Empty));
    assert_eq!(read_csv(b" ;\n;", "x"), Err(ReadError::Empty));
    assert_eq!(
        read_csv(b"a,\0\0\x01b", "x"),
        Err(ReadError::Damaged),
        "not words"
    );
    assert_eq!(
        read_csv(&vec![b'a'; (32 << 20) + 1], "x"),
        Err(ReadError::TooLarge)
    );
}

proptest! {
    // S82-08: any bytes are an error or a valid table.
    #[test]
    fn s82_08_any_bytes(bytes in prop::collection::vec(any::<u8>(), 0..300)) {
        if let Ok(sheet) = read_csv(&bytes, "x") {
            prop_assert!(sheet.table.is_valid());
        }
    }

    // S82-08: what is quoted reads back.
    #[test]
    fn s82_08_quoted_cells_read_back(cells in prop::collection::vec("[a-z\",;\n|]{0,6}x", 4)) {
        let quoted: Vec<String> = cells.iter().map(|c| format!("\"{}\"", c.replace('"', "\"\""))).collect();
        let sheet = csv(&format!("{},{}\n{},{}", quoted[0], quoted[1], quoted[2], quoted[3]));
        let back: Vec<&str> = sheet.table.cells.iter().map(|c| c.text.as_str()).collect();
        prop_assert_eq!(back, cells.iter().map(String::as_str).collect::<Vec<_>>());
    }
}

proptest! {
    // S82-04: any code shows any number in some way.
    #[test]
    fn s82_04_any_code(code in "[0#?.,;%@\"\\\\\\[\\]$<>=E+hmsdyaAMP/ _*-]{0,24}", v in any::<f64>(), small in -100_000.0..100_000.0_f64) {
        let _ = show(v, &code, false, ES);
        let _ = show(small, &code, true, EN);
    }
}

// S82-04: elements inside elements, more than a stack holds.
#[test]
fn s82_04_deep_parts() {
    let deep = |open: &str, n: usize| format!("{}{}", open.repeat(n), "</a>".repeat(n));
    for open in [
        "<a>",
        "<a b='/>'>",
        "<!-- </a> --><a>",
        "<a><![CDATA[</a>]]>",
        "<a><?x </a>?>",
    ] {
        let book = zipped(&[("xl/workbook.xml".into(), deep(open, 300_000))]);
        assert_eq!(read(&book, ES), Err(ReadError::Damaged), "{open}");
    }
    let full = "<sheetData><row r=\"1\"><c r=\"A1\"><v>1</v></c></row></sheetData>";
    let book = book(
        &[(
            "Hoja",
            &format!("{full}<x b=\"a>b\"/><!-- <a><a> -->{}", deep("<a>", 50)),
        )],
        &[],
    );
    assert!(read(&book, ES).is_ok(), "a little is fine");
}

// S104-30: bulk create reads every row and column of a sheet and of a file
// of separated values; a table still takes what it holds.
#[test]
fn s104_30_the_reach_of_bulk_create() {
    use gafira_io_xlsx::{Reach, read_csv_reach, read_reach};
    let all = Reach {
        rows: 100_001,
        columns: 500,
    };
    let rows: String = (1..=300)
        .map(|r| {
            let cells: String = (0..30)
                .map(|c| format!("<c r=\"{}{r}\"><v>{c}</v></c>", column(c)))
                .collect();
            format!("<row r=\"{r}\">{cells}</row>")
        })
        .collect();
    let bytes = book(&[("Hoja", &format!("<sheetData>{rows}</sheetData>"))], &[]);
    let sheet = read_reach(&bytes, ES, all).expect("read").remove(0);
    assert_eq!((sheet.data.len(), sheet.data[299].len()), (300, 30));
    assert_eq!(sheet.data[299][29], "29");
    assert_eq!(
        (sheet.table.rows.len(), sheet.table.columns.len()),
        (50, 20)
    );
    let text: String = (0..300)
        .map(|r| {
            let cells: Vec<String> = (0..30).map(|c| format!("{r}.{c}")).collect();
            cells.join(";") + "\n"
        })
        .collect();
    let csv = read_csv_reach(text.as_bytes(), "datos", all).expect("csv");
    assert_eq!((csv.data.len(), csv.data[299].len()), (300, 30));
    assert_eq!(csv.data[299][29], "299.29");
    let cut = read_csv(text.as_bytes(), "datos").expect("csv");
    assert_eq!((cut.data.len(), cut.data[0].len()), (201, 20));
}
