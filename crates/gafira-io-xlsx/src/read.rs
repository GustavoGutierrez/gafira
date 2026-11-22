//! Reading a workbook (spec 082): its sheets as tables.
//!
//! The file is anyone's: what is wrong in it is left out, or is an error.

use std::collections::{BTreeMap, BTreeSet};

use gafira_doc::{CellMerge, CellStyle, ShapeStroke, TableCell, TableNode, TextAlign};
use gafira_table::{BorderSet, Range, set_borders};
use roxmltree::Node;

use crate::Locale;
use crate::look::{Look, PX_IN_POINT, Palette, color, looks, tinted};
use crate::number::show;
use crate::package::{Local, Package, Relation, parse};

/// Why a workbook cannot be read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReadError {
    /// The old `.xls`, the binary `.xlsb`, or a workbook with a password.
    Unsupported,
    TooLarge,
    Damaged,
    /// No sheet holds anything.
    Empty,
}

/// A sheet as a table.
#[derive(Clone, Debug, PartialEq)]
pub struct ReadSheet {
    pub name: String,
    pub table: TableNode,
    /// Its box at 96 px an inch.
    pub size: (f64, f64),
    /// The rows and columns in use, before the limits of a table.
    pub used: (usize, usize),
    /// The words row by row, up to [`MAX_DATA_ROWS`] and the columns of a
    /// table: what a chart takes (spec 083).
    pub data: Vec<Vec<String>>,
}

impl ReadSheet {
    /// Whether the table holds less than the sheet.
    #[must_use]
    pub fn is_cut(&self) -> bool {
        self.used.0 > self.table.rows.len() || self.used.1 > self.table.columns.len()
    }
}

/// The most rows of [`ReadSheet::data`].
pub const MAX_DATA_ROWS: usize = 201;

/// The rows and columns of the largest sheet.
const SHEET_ROWS: usize = 1_048_576;
const SHEET_COLUMNS: usize = 16_384;

/// The most sheets read, and the most merges of a sheet.
const MAX_SHEETS: usize = 100;
const MAX_MERGES: usize = 2000;

/// The room inside a cell of a sheet, in px.
const PADDING: f64 = 3.0;

/// The lines of a sheet that draws none of its own.
const LINES: u32 = 0xD4_D4D4;

/// What every sheet of a workbook shares.
struct Book<'l> {
    strings: Vec<String>,
    looks: Vec<Look>,
    palette: Palette,
    date1904: bool,
    locale: &'l Locale,
}

/// The sheets with something to show: the hidden ones only when there is no
/// other.
pub fn read(bytes: &[u8], locale: Locale) -> Result<Vec<ReadSheet>, ReadError> {
    let mut package = Package::open(bytes)?;
    let book_part = package
        .relations("")?
        .into_iter()
        .find(|r| r.kind == "officeDocument")
        .map_or_else(|| "xl/workbook.xml".to_owned(), |r| r.target);
    if book_part.ends_with(".bin") {
        return Err(ReadError::Unsupported);
    }
    let text = package.text(&book_part)?.ok_or(ReadError::Damaged)?;
    let xml = parse(&text)?;
    let root = xml.root_element();
    if !root.has_tag_name_local("workbook") {
        return Err(ReadError::Damaged);
    }
    let relations = package.relations(&book_part)?;
    let mut book = Book {
        strings: Vec::new(),
        looks: Vec::new(),
        palette: Palette::default(),
        date1904: root
            .child("workbookPr")
            .is_some_and(|n| n.flag("date1904", false)),
        locale: &locale,
    };
    let mut shared = |kind: &str| -> Result<Option<String>, ReadError> {
        match relations.iter().find(|r| r.kind == kind) {
            Some(relation) => package.text(&relation.target),
            None => Ok(None),
        }
    };
    if let Some(text) = shared("theme")? {
        book.palette.read_theme(&text);
    }
    if let Some(text) = shared("styles")? {
        book.looks = looks(&text, &mut book.palette)?;
    }
    if let Some(text) = shared("sharedStrings")? {
        let xml = parse(&text)?;
        book.strings = xml.root_element().all("si").map(words).collect();
    }
    let (mut shown, mut hidden) = (Vec::new(), Vec::new());
    let mut wrong = None;
    let listed = root.child("sheets");
    for node in listed.iter().flat_map(|n| n.all("sheet")).take(MAX_SHEETS) {
        let part = node
            .relation()
            .and_then(|id| relations.iter().find(|r| r.id == id));
        let Some(part) = part else { continue };
        let name = node.attribute("name").unwrap_or_default().trim().to_owned();
        match sheet(&mut package, part, name, &book) {
            Ok(Some(sheet)) => match node.attribute("state") {
                Some("hidden" | "veryHidden") => hidden.push(sheet),
                _ => shown.push(sheet),
            },
            Ok(None) => {}
            // The other sheets may be fine.
            Err(error) => wrong = wrong.or(Some(error)),
        }
    }
    if shown.is_empty() {
        shown = hidden;
    }
    if shown.is_empty() {
        return Err(wrong.unwrap_or(ReadError::Empty));
    }
    Ok(shown)
}

/// The words of a string: its runs, but not the guides to read them.
fn words(node: Node<'_, '_>) -> String {
    let mut out = String::new();
    for child in node.children() {
        let text = if child.has_tag_name_local("t") {
            child.text()
        } else if child.has_tag_name_local("r") {
            child.child("t").and_then(|t| t.text())
        } else {
            None
        };
        out.push_str(text.unwrap_or_default());
    }
    unescape(&out)
}

/// `_xHHHH_` as the character it is, from `text`.
fn unescape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = rest.find("_x") {
        out.push_str(&rest[..at]);
        rest = &rest[at..];
        let code = rest
            .get(2..6)
            .filter(|_| rest.as_bytes().get(6) == Some(&b'_'))
            .filter(|code| code.bytes().all(|b| b.is_ascii_hexdigit()))
            .and_then(|code| u32::from_str_radix(code, 16).ok())
            .and_then(char::from_u32);
        match code {
            Some(c) => {
                out.push(c);
                rest = &rest[7..];
            }
            None => {
                out.push_str("_x");
                rest = &rest[2..];
            }
        }
    }
    out.push_str(rest);
    out
}

/// `text` as a cell holds it: lines, no other control, not too long.
pub(crate) fn clean(text: &str) -> String {
    let mut out = String::with_capacity(text.len().min(TableNode::MAX_CELL_TEXT));
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        let c = match c {
            '\r' if chars.peek() == Some(&'\n') => continue,
            '\r' => '\n',
            '\t' => ' ',
            c => c,
        };
        if c.is_control() && c != '\n' {
            continue;
        }
        if out.len() + c.len_utf8() > TableNode::MAX_CELL_TEXT {
            break;
        }
        out.push(c);
    }
    out
}

/// The row and the column of an address, `B3`.
fn address(text: &str) -> Option<(usize, usize)> {
    let text = text.trim().replace('$', "");
    let letters = text.bytes().take_while(u8::is_ascii_alphabetic).count();
    if !(1..=3).contains(&letters) {
        return None;
    }
    let col = text[..letters].bytes().fold(0, |n, b| {
        n * 26 + usize::from(b.to_ascii_uppercase() - b'A') + 1
    });
    let digits = &text[letters..];
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let row: usize = digits.parse().ok()?;
    ((1..=SHEET_ROWS).contains(&row) && col <= SHEET_COLUMNS).then(|| (row - 1, col - 1))
}

/// The cells of a range, `A1:B2`, its ends included.
#[derive(Clone, Copy)]
struct Area {
    rows: (usize, usize),
    cols: (usize, usize),
}

impl Area {
    fn read(text: &str) -> Option<Self> {
        let (a, b) = text.split_once(':').unwrap_or((text, text));
        let (a, b) = (address(a)?, address(b)?);
        Some(Self {
            rows: (a.0.min(b.0), a.0.max(b.0)),
            cols: (a.1.min(b.1), a.1.max(b.1)),
        })
    }

    fn touches(&self, other: &Self) -> bool {
        self.rows.0 <= other.rows.1
            && other.rows.0 <= self.rows.1
            && self.cols.0 <= other.cols.1
            && other.cols.0 <= self.cols.1
    }

    fn contains(&self, row: usize, col: usize) -> bool {
        (self.rows.0..=self.rows.1).contains(&row) && (self.cols.0..=self.cols.1).contains(&col)
    }
}

/// A cell as the sheet holds it.
struct Cell<'a> {
    kind: &'a str,
    value: &'a str,
    inline: Option<String>,
    style: Option<usize>,
}

impl Cell<'_> {
    fn is_empty(&self, book: &Book<'_>) -> bool {
        match (&self.inline, self.kind) {
            (Some(text), _) => text.is_empty(),
            (None, "s") => book.string(self.value).is_empty(),
            (None, _) => self.value.is_empty(),
        }
    }
}

impl Book<'_> {
    fn string(&self, index: &str) -> &str {
        let index: Option<usize> = index.parse().ok();
        index
            .and_then(|i| self.strings.get(i))
            .map_or("", String::as_str)
    }

    /// What a cell shows, and the side it leans on by itself.
    fn shown(&self, cell: &Cell<'_>, code: &str) -> (String, TextAlign) {
        if let Some(text) = &cell.inline {
            return (clean(text), TextAlign::Left);
        }
        match cell.kind {
            "s" => (clean(self.string(cell.value)), TextAlign::Left),
            "str" | "inlineStr" => (clean(&unescape(cell.value)), TextAlign::Left),
            "b" => {
                let yes = matches!(cell.value, "1" | "true");
                let word = if yes { self.locale.yes } else { self.locale.no };
                (word.to_owned(), TextAlign::Center)
            }
            "e" => (clean(cell.value), TextAlign::Center),
            // A date written out.
            "d" => (clean(cell.value), TextAlign::Right),
            _ => match cell.value.parse::<f64>() {
                Ok(v) => (
                    clean(&show(v, code, self.date1904, *self.locale)),
                    TextAlign::Right,
                ),
                Err(_) => (clean(cell.value), TextAlign::Left),
            },
        }
    }
}

/// The width of a stored column, in px: characters of 7 px.
fn column_px(width: f64) -> f64 {
    ((256.0 * width.clamp(0.0, 255.0) + 18.0) / 256.0 * 7.0).trunc()
}

/// The columns and the rows of a sheet that are not as the others.
struct Sizes {
    columns: Vec<Columns>,
    column: f64,
    row: f64,
    rows: BTreeMap<usize, (f64, Option<usize>)>,
    hidden: BTreeSet<usize>,
}

struct Columns {
    from: usize,
    to: usize,
    width: Option<f64>,
    hidden: bool,
    style: Option<usize>,
}

impl Sizes {
    fn of(&self, col: usize) -> Option<&Columns> {
        self.columns.iter().find(|c| (c.from..=c.to).contains(&col))
    }

    fn width(&self, col: usize) -> f64 {
        self.of(col).and_then(|c| c.width).unwrap_or(self.column)
    }

    fn col_hidden(&self, col: usize) -> bool {
        self.of(col).is_some_and(|c| c.hidden)
    }
}

/// The columns, the default sizes and the rows' of `root`.
fn sizes(root: Node<'_, '_>) -> Sizes {
    let format = root.child("sheetFormatPr");
    let float = |name: &str| format.and_then(|n| n.float(name));
    let column = match float("defaultColWidth") {
        Some(width) => column_px(width),
        None => 64.0,
    };
    let columns = root
        .child("cols")
        .into_iter()
        .flat_map(|n| n.all("col").collect::<Vec<_>>())
        .take(SHEET_COLUMNS)
        .filter_map(|n| {
            let width = n.float("width").map(column_px);
            Some(Columns {
                from: n.index("min")?.checked_sub(1)?,
                to: n.index("max")?.checked_sub(1)?,
                width,
                hidden: n.flag("hidden", false) || width.is_some_and(|w| w <= 0.0),
                style: n.index("style"),
            })
        })
        .collect();
    Sizes {
        columns,
        column: column.max(1.0),
        row: (float("defaultRowHeight").unwrap_or(15.0) * PX_IN_POINT).clamp(1.0, 546.0),
        rows: BTreeMap::new(),
        hidden: BTreeSet::new(),
    }
}

/// The cells of a sheet by row and column; the rows' sizes go to `sizes`.
fn cells<'a>(root: Node<'a, '_>, sizes: &mut Sizes) -> BTreeMap<(usize, usize), Cell<'a>> {
    let mut out = BTreeMap::new();
    let mut next_row = 0;
    for row in root
        .child("sheetData")
        .into_iter()
        .flat_map(|n| n.all("row").collect::<Vec<_>>())
    {
        let at = match row.attribute("r") {
            Some(text) => text
                .trim()
                .parse::<usize>()
                .ok()
                .and_then(|n| n.checked_sub(1)),
            None => Some(next_row),
        };
        let Some(at) = at.filter(|r| *r < SHEET_ROWS) else {
            continue;
        };
        next_row = at + 1;
        let height = row.float("ht").map(|ht| (ht * PX_IN_POINT).min(546.0));
        if row.flag("hidden", false) || height.is_some_and(|h| h <= 0.0) {
            sizes.hidden.insert(at);
        }
        let style = row.index("s").filter(|_| row.flag("customFormat", false));
        if height.is_some() || style.is_some() {
            let height = height.unwrap_or(sizes.row);
            sizes.rows.insert(at, (height, style));
        }
        let mut next = 0;
        for c in row.all("c") {
            let place = match c.attribute("r") {
                Some(text) => address(text),
                None => (next < SHEET_COLUMNS).then_some((at, next)),
            };
            let Some(place) = place else { continue };
            next = place.1 + 1;
            out.insert(
                place,
                Cell {
                    kind: c.attribute("t").unwrap_or("n"),
                    value: c.child("v").and_then(|v| v.text()).unwrap_or_default(),
                    inline: c.child("is").map(words),
                    style: c.index("s"),
                },
            );
        }
    }
    out
}

/// The merges of a sheet, none over another.
fn merges(root: Node<'_, '_>) -> Vec<Area> {
    let mut out: Vec<Area> = Vec::new();
    for node in root
        .child("mergeCells")
        .into_iter()
        .flat_map(|n| n.all("mergeCell").collect::<Vec<_>>())
        .take(MAX_MERGES)
    {
        if let Some(area) = node.attribute("ref").and_then(Area::read)
            && !out.iter().any(|other| other.touches(&area))
        {
            out.push(area);
        }
    }
    out
}

/// A sheet as a table; `None` when it holds nothing, or is not of cells.
fn sheet(
    package: &mut Package<'_>,
    part: &Relation,
    name: String,
    book: &Book<'_>,
) -> Result<Option<ReadSheet>, ReadError> {
    if part.kind != "worksheet" {
        return Ok(None);
    }
    let text = package.text(&part.target)?.ok_or(ReadError::Damaged)?;
    let xml = parse(&text)?;
    let root = xml.root_element();
    if !root.has_tag_name_local("worksheet") {
        return Err(ReadError::Damaged);
    }
    let mut sizes = sizes(root);
    let cells = cells(root, &mut sizes);
    let merges = merges(root);
    let Some(area) = used(&cells, &merges, &sizes, book) else {
        return Ok(None);
    };
    let shown = |r: &usize| !sizes.hidden.contains(r);
    let in_area = sizes.hidden.range(area.rows.0..=area.rows.1).count();
    let used_rows = area.rows.1 + 1 - area.rows.0 - in_area;
    let all_rows: Vec<usize> = (area.rows.0..=area.rows.1)
        .filter(shown)
        .take(MAX_DATA_ROWS)
        .collect();
    let rows = &all_rows[..all_rows.len().min(TableNode::MAX_ROWS)];
    let all_cols: Vec<usize> = (area.cols.0..=area.cols.1)
        .filter(|c| !sizes.col_hidden(*c))
        .collect();
    let used = (used_rows, all_cols.len());
    let cols: Vec<usize> = all_cols.into_iter().take(TableNode::MAX_COLUMNS).collect();
    if rows.is_empty() || cols.is_empty() {
        return Ok(None);
    }
    let formatted = formatted_tables(package, &part.target, root)?;
    let mut table = TableNode {
        columns: cols.iter().map(|c| sizes.width(*c).max(1.0)).collect(),
        rows: rows
            .iter()
            .map(|r| sizes.rows.get(r).map_or(sizes.row, |(h, _)| *h).max(1.0))
            .collect(),
        cells: Vec::with_capacity(rows.len() * cols.len()),
        merges: Vec::new(),
        spacing: 0.0,
        padding: PADDING,
        header: formatted
            .iter()
            .any(|f| f.header && f.area.rows.0 == rows[0]),
    };
    let look_of = |r: usize, c: usize| {
        let cell = cells.get(&(r, c));
        cell.and_then(|cell| cell.style)
            .or_else(|| sizes.rows.get(&r).and_then(|(_, style)| *style))
            .or_else(|| sizes.of(c).and_then(|c| c.style))
    };
    let data = all_rows
        .iter()
        .map(|r| {
            let word = |c: &usize| {
                let code = look_of(*r, *c).and_then(|i| book.looks.get(i));
                let code = code.map_or("", |look| look.code.as_str());
                let cell = cells.get(&(*r, *c));
                cell.map_or_else(String::new, |cell| book.shown(cell, code).0)
            };
            cols.iter().map(word).collect()
        })
        .collect();
    for r in rows {
        for c in &cols {
            let cell = cells.get(&(*r, *c));
            let index = look_of(*r, *c);
            table
                .cells
                .push(made(cell, index, formatted.iter(), (*r, *c), book));
        }
    }
    for merge in &merges {
        let inside = |list: &[usize], (from, to): (usize, usize)| {
            let first = list.iter().position(|i| (from..=to).contains(i))?;
            let count = list.iter().filter(|i| (from..=to).contains(*i)).count();
            Some((first, count))
        };
        let (Some((row, rows_in)), Some((col, cols_in))) =
            (inside(rows, merge.rows), inside(&cols, merge.cols))
        else {
            continue;
        };
        // The words of a merge are the ones of its first cell, even hidden.
        let anchor = (merge.rows.0, merge.cols.0);
        if (rows[row], cols[col]) != anchor
            && let Some(cell) = cells.get(&anchor)
            && let Some(slot) = table.cell_mut(row, col)
        {
            *slot = made(Some(cell), cell.style, formatted.iter(), anchor, book);
        }
        for r in row..row + rows_in {
            for c in col..col + cols_in {
                if (r, c) != (row, col)
                    && let Some(cell) = table.cell_mut(r, c)
                {
                    cell.text.clear();
                }
            }
        }
        if rows_in * cols_in > 1 {
            table.merges.push(CellMerge {
                row,
                col,
                rows: rows_in,
                cols: cols_in,
            });
        }
    }
    lines(&mut table);
    let size = (table.columns.iter().sum(), table.rows.iter().sum());
    Ok(table.is_valid().then_some(ReadSheet {
        name,
        table,
        size,
        used,
        data,
    }))
}

/// The area with something in it: words, or borders around no words.
fn used(
    cells: &BTreeMap<(usize, usize), Cell<'_>>,
    merges: &[Area],
    sizes: &Sizes,
    book: &Book<'_>,
) -> Option<Area> {
    let hidden = |row: usize, col: usize| sizes.hidden.contains(&row) || sizes.col_hidden(col);
    let filled: BTreeSet<(usize, usize)> = cells
        .iter()
        .filter(|(_, cell)| !cell.is_empty(book))
        .map(|(at, _)| *at)
        .collect();
    let mut area: Option<Area> = None;
    let mut add = |with: Area| {
        let a = area.get_or_insert(with);
        a.rows = (a.rows.0.min(with.rows.0), a.rows.1.max(with.rows.1));
        a.cols = (a.cols.0.min(with.cols.0), a.cols.1.max(with.cols.1));
    };
    let one = |(row, col): (usize, usize)| Area {
        rows: (row, row),
        cols: (col, col),
    };
    let mut words = false;
    for at in filled.iter().filter(|at| !hidden(at.0, at.1)) {
        words = true;
        add(one(*at));
    }
    for merge in merges {
        if filled.contains(&(merge.rows.0, merge.cols.0)) {
            words = true;
            add(*merge);
        }
    }
    if !words {
        return None;
    }
    for (at, cell) in cells {
        let edged = cell
            .style
            .and_then(|s| book.looks.get(s))
            .is_some_and(|l| l.style.borders != gafira_doc::CellBorders::default());
        if edged && !hidden(at.0, at.1) {
            add(one(*at));
        }
    }
    area
}

/// The cell of the table for `cell`, in the look at `index`.
fn made<'a>(
    cell: Option<&Cell<'_>>,
    index: Option<usize>,
    formatted: impl Iterator<Item = &'a Formatted>,
    (row, col): (usize, usize),
    book: &Book<'_>,
) -> TableCell {
    let look = index
        .and_then(|i| book.looks.get(i))
        .cloned()
        .unwrap_or_default();
    let (text, leans) = cell.map_or_else(
        || (String::new(), TextAlign::Left),
        |cell| book.shown(cell, &look.code),
    );
    let mut style = look.style;
    style.align = look.align.unwrap_or(leans);
    for table in formatted {
        table.dress(&mut style, look.auto_color, row, col, &book.palette);
    }
    TableCell { text, style }
}

/// A table with a format of Excel.
struct Formatted {
    area: Area,
    header: bool,
    /// The last row adds up.
    totals: bool,
    stripes: bool,
    /// The colour of the theme, and whether the header is filled with it.
    accent: Option<(usize, bool)>,
}

impl Formatted {
    fn read(text: &str) -> Option<Self> {
        let xml = parse(text).ok()?;
        let root = xml.root_element();
        let info = root.child("tableStyleInfo");
        let name = info.and_then(|n| n.attribute("name")).unwrap_or_default();
        // `TableStyleMedium9`: seven colours, over and over.
        let accent = name.strip_prefix("TableStyle").and_then(|rest| {
            let digits = rest.trim_start_matches(|c: char| !c.is_ascii_digit());
            let number: usize = digits.parse().ok()?;
            let step = number.checked_sub(1)? % 7;
            let theme = if step == 0 { 1 } else { 3 + step };
            Some((theme, !rest.starts_with("Light")))
        });
        Some(Self {
            area: Area::read(root.attribute("ref")?)?,
            header: root.index("headerRowCount") != Some(0),
            totals: root.index("totalsRowCount").is_some_and(|n| n > 0),
            stripes: info.is_some_and(|n| n.flag("showRowStripes", false)),
            accent,
        })
    }

    /// Dresses the cell at `row`, `col` where it has no look of its own.
    fn dress(
        &self,
        style: &mut CellStyle,
        auto_color: bool,
        row: usize,
        col: usize,
        palette: &Palette,
    ) {
        if !self.area.contains(row, col) {
            return;
        }
        let is_header = self.header && row == self.area.rows.0;
        if is_header || (self.totals && row == self.area.rows.1) {
            style.bold = true;
        }
        let Some((theme, filled)) = self.accent else {
            return;
        };
        let accent = palette.theme(theme);
        if is_header {
            if filled && style.fill.is_none() {
                style.fill = Some(color(accent));
                if auto_color {
                    style.color = color(palette.theme(0));
                }
            }
            return;
        }
        let first = self.area.rows.0 + usize::from(self.header);
        let band = row.saturating_sub(first).is_multiple_of(2);
        let total = self.totals && row == self.area.rows.1;
        if self.stripes && band && !total && style.fill.is_none() {
            style.fill = Some(color(tinted(accent, 0.8)));
        }
    }
}

/// The most tables with a format read of a sheet.
const MAX_FORMATTED: usize = 64;

fn formatted_tables(
    package: &mut Package<'_>,
    part: &str,
    root: Node<'_, '_>,
) -> Result<Vec<Formatted>, ReadError> {
    let Some(list) = root.child("tableParts") else {
        return Ok(Vec::new());
    };
    let relations = package.relations(part)?;
    let mut out = Vec::new();
    for node in list.all("tablePart").take(MAX_FORMATTED) {
        let target = node
            .relation()
            .and_then(|id| relations.iter().find(|r| r.id == id));
        if let Some(target) = target
            && let Some(text) = package.text(&target.target)?
        {
            out.extend(Formatted::read(&text));
        }
    }
    Ok(out)
}

/// Thin lines for a table that has none, so that its grid is seen.
pub(crate) fn lines(table: &mut TableNode) {
    let none = gafira_doc::CellBorders::default();
    if table.cells.iter().all(|c| c.style.borders == none) {
        let all = Range::all(table);
        let stroke = ShapeStroke::new(color(LINES), 1.0);
        set_borders(table, all, BorderSet::All, Some(stroke));
    }
}
