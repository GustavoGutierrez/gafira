//! Reading separated values (S82-08): what spreadsheets, banks and
//! databases write as `.csv` and `.tsv`.

use gafira_doc::{CellStyle, TableCell, TableNode, TextAlign};
use gafira_table::{CellValue, value};

use crate::look::color;
use crate::package::MAX_FILE;
use crate::read::{MAX_DATA_ROWS, clean, lines};
use crate::{ReadError, ReadSheet};

/// The characters that may separate, the likeliest first.
const SEPARATORS: [char; 4] = [',', ';', '\t', '|'];

/// The rows looked at to find the separator.
const SAMPLE: usize = 20;

/// The words of the cells, in px.
const FONT: &str = "Inter";
const SIZE: f64 = 16.0;
const INK: u32 = 0x1F_2937;

/// The width of a character, of a column and the height of a line, in px.
const CHARACTER: f64 = 9.0;
const WIDTH: std::ops::RangeInclusive<f64> = 56.0..=320.0;
const LINE: f64 = 20.0;
const PADDING: f64 = 8.0;

/// The table of a file of separated values.
pub fn read_csv(bytes: &[u8], name: &str) -> Result<ReadSheet, ReadError> {
    if bytes.len() > MAX_FILE {
        return Err(ReadError::TooLarge);
    }
    let text = decode(bytes).ok_or(ReadError::Damaged)?;
    // What some spreadsheets write first to say the separator.
    let told = text
        .strip_prefix("sep=")
        .and_then(|rest| Some((rest.chars().next()?, rest)))
        .filter(|(c, _)| SEPARATORS.contains(c));
    let (separator, body) = match told {
        Some((c, rest)) => (c, rest[c.len_utf8()..].trim_start_matches(['\r', '\n'])),
        None => (separator(&text), text.as_str()),
    };
    let mut grid = Grid::default();
    split(body, separator, usize::MAX, |row| grid.add(row));
    grid.table(name)
}

/// The text of `bytes`; `None` when they are not words.
fn decode(bytes: &[u8]) -> Option<String> {
    let utf16 = |bytes: &[u8], unit: fn([u8; 2]) -> u16| {
        let units = bytes.chunks_exact(2).map(|b| unit([b[0], b[1]]));
        char::decode_utf16(units)
            .map(|c| c.unwrap_or(char::REPLACEMENT_CHARACTER))
            .collect::<String>()
    };
    let text = match bytes {
        [0xFF, 0xFE, rest @ ..] => utf16(rest, u16::from_le_bytes),
        [0xFE, 0xFF, rest @ ..] => utf16(rest, u16::from_be_bytes),
        [0xEF, 0xBB, 0xBF, rest @ ..] => String::from_utf8_lossy(rest).into_owned(),
        _ => match std::str::from_utf8(bytes) {
            Ok(text) => text.to_owned(),
            Err(_) => bytes.iter().map(|b| windows_1252(*b)).collect(),
        },
    };
    (!text.contains('\0')).then_some(text)
}

/// A byte of the encoding Windows gives western languages.
fn windows_1252(byte: u8) -> char {
    const HIGH: [char; 32] = [
        '€', '\u{81}', '‚', 'ƒ', '„', '…', '†', '‡', 'ˆ', '‰', 'Š', '‹', 'Œ', '\u{8D}', 'Ž',
        '\u{8F}', '\u{90}', '‘', '’', '“', '”', '•', '–', '—', '˜', '™', 'š', '›', 'œ', '\u{9D}',
        'ž', 'Ÿ',
    ];
    match byte {
        0x80..=0x9F => HIGH[usize::from(byte - 0x80)],
        _ => char::from(byte),
    }
}

/// The separator whose quotes close best and that gives the first rows the
/// most alike counts of cells.
fn separator(text: &str) -> char {
    let mut best = (usize::MAX, 0, SEPARATORS[0]);
    for candidate in SEPARATORS {
        let mut counts: Vec<usize> = Vec::new();
        let wrong = split(text, candidate, SAMPLE, |row| {
            if row.len() > 1 || row.iter().any(|c| !c.is_empty()) {
                counts.push(row.len());
            }
        });
        // Rows with as many cells as the first one, if it has several.
        let first = counts.first().copied().unwrap_or(0);
        let alike = counts.iter().filter(|n| **n == first).count();
        let most = counts.iter().copied().max().unwrap_or(0);
        let score = if first > 1 {
            alike * 1000 + first.min(999)
        } else {
            usize::from(most > 1)
        };
        if wrong < best.0 || (wrong == best.0 && score > best.1) {
            best = (wrong, score, candidate);
        }
    }
    best.2
}

/// Hands over the rows of `text`, at most `most`; how many quotes are out
/// of their place.
fn split(text: &str, separator: char, most: usize, mut row: impl FnMut(&[String])) -> usize {
    let mut cells: Vec<String> = Vec::new();
    let mut cell = String::new();
    let mut quoted = false;
    let (mut rows, mut wrong) = (0, 0);
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if quoted {
            match c {
                '"' if chars.peek() == Some(&'"') => {
                    chars.next();
                    cell.push('"');
                }
                '"' => {
                    quoted = false;
                    let closes = |c: &char| *c == separator || matches!(c, '\r' | '\n');
                    wrong += usize::from(!chars.peek().is_none_or(closes));
                }
                _ => cell.push(c),
            }
            continue;
        }
        match c {
            // Only at the start: inches stay as they are.
            '"' if cell.is_empty() => quoted = true,
            '\r' | '\n' => {
                if c == '\r' && chars.peek() == Some(&'\n') {
                    chars.next();
                }
                cells.push(std::mem::take(&mut cell));
                row(&cells);
                cells.clear();
                rows += 1;
                if rows == most {
                    return wrong;
                }
            }
            _ if c == separator => cells.push(std::mem::take(&mut cell)),
            _ => {
                wrong += usize::from(c == '"');
                cell.push(c);
            }
        }
    }
    if !cell.is_empty() || !cells.is_empty() {
        cells.push(cell);
        row(&cells);
    }
    wrong
}

/// The cells a table or a chart can hold, and how many there are in all.
#[derive(Default)]
struct Grid {
    rows: Vec<Vec<String>>,
    /// The rows so far, and the area with words.
    count: usize,
    first: Option<usize>,
    last: usize,
    left: usize,
    right: usize,
}

impl Grid {
    fn add(&mut self, row: &[String]) {
        let at = self.count;
        self.count += 1;
        let filled = |c: &String| !c.trim().is_empty();
        let Some(left) = row.iter().position(filled) else {
            if self.first.is_some() && self.rows.len() < MAX_DATA_ROWS {
                self.rows.push(Vec::new());
            }
            return;
        };
        let right = row.iter().rposition(filled).unwrap_or(left);
        if self.first.is_none() {
            self.first = Some(at);
            self.left = left;
        }
        self.left = self.left.min(left);
        self.right = self.right.max(right);
        self.last = at;
        if self.rows.len() < MAX_DATA_ROWS {
            // Past the columns a table holds, whatever its first one is.
            let most = left + TableNode::MAX_COLUMNS;
            self.rows
                .push(row.iter().take(most).map(|c| clean(c)).collect());
        }
    }

    fn table(mut self, name: &str) -> Result<ReadSheet, ReadError> {
        let first = self.first.ok_or(ReadError::Empty)?;
        let used = (self.last + 1 - first, self.right + 1 - self.left);
        self.rows.truncate(used.0);
        let cols = used.1.min(TableNode::MAX_COLUMNS);
        let word = |r: usize, c: usize| {
            let row = self.rows.get(r);
            row.and_then(|row| row.get(self.left + c))
                .map_or("", String::as_str)
        };
        let count = self.rows.len().min(TableNode::MAX_ROWS);
        let names = (0..cols).all(|c| value(word(0, c)) == CellValue::Text);
        let numbers = (1..count)
            .any(|r| (0..cols).any(|c| matches!(value(word(r, c)), CellValue::Number(..))));
        let header = names && numbers;
        let mut style = CellStyle::new(FONT, SIZE, color(INK));
        style.align = TextAlign::Left;
        let longest = |text: &str| text.lines().map(|l| l.chars().count()).max().unwrap_or(0);
        let width = |c: usize| {
            let most = (0..count).map(|r| longest(word(r, c))).max().unwrap_or(0);
            // Far below 2^53: the cast is exact.
            (most as f64 * CHARACTER + PADDING * 2.0).clamp(*WIDTH.start(), *WIDTH.end())
        };
        let columns: Vec<f64> = (0..cols).map(width).collect();
        let height = |r: usize| {
            let lines = (0..cols).map(|c| word(r, c).lines().count()).max();
            (lines.unwrap_or(1).clamp(1, 50) as f64) * LINE + PADDING * 2.0
        };
        let mut table = TableNode {
            rows: (0..count).map(height).collect(),
            columns,
            cells: Vec::with_capacity(count * cols),
            merges: Vec::new(),
            spacing: 0.0,
            padding: PADDING,
            header,
        };
        for r in 0..count {
            for c in 0..cols {
                let text = word(r, c).to_owned();
                let mut style = style.clone();
                style.bold = header && r == 0;
                if matches!(value(&text), CellValue::Number(..)) {
                    style.align = TextAlign::Right;
                }
                table.cells.push(TableCell { text, style });
            }
        }
        lines(&mut table);
        let size = (table.columns.iter().sum(), table.rows.iter().sum());
        if !table.is_valid() {
            return Err(ReadError::Damaged);
        }
        Ok(ReadSheet {
            name: name.to_owned(),
            table,
            size,
            used,
            data: (0..self.rows.len())
                .map(|r| (0..cols).map(|c| word(r, c).to_owned()).collect())
                .collect(),
        })
    }
}
