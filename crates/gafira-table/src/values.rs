//! The values of the cells: numbers for a spreadsheet and for charts
//! (S81-14, S81-15).

use gafira_doc::TableNode;

/// How a number was written.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum NumberFormat {
    General,
    Percent,
    Currency(char),
}

/// What a cell holds.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CellValue {
    Empty,
    Number(f64, NumberFormat),
    Text,
}

/// The most digits a number keeps exactly in a spreadsheet.
const MAX_DIGITS: usize = 15;

/// The value of the words of a cell. A number only when nothing is lost or
/// guessed: `007` is a code, and `1.234` is one thing in one country and
/// another in the next.
#[must_use]
pub fn value(words: &str) -> CellValue {
    let words = words.trim();
    if words.is_empty() {
        return CellValue::Empty;
    }
    let (body, format) = if let Some(body) = words.strip_suffix('%') {
        (body.trim_end(), NumberFormat::Percent)
    } else if let Some((body, sign)) = currency(words) {
        (body, NumberFormat::Currency(sign))
    } else {
        (words, NumberFormat::General)
    };
    let (negative, digits) = match body.strip_prefix('-') {
        Some(rest) => (true, rest.trim_start()),
        None => (false, body),
    };
    let Some(number) = plain_number(digits) else {
        return CellValue::Text;
    };
    let number = if negative { -number } else { number };
    let number = if format == NumberFormat::Percent {
        number / 100.0
    } else {
        number
    };
    CellValue::Number(number, format)
}

/// The words without their currency sign, and the sign.
fn currency(words: &str) -> Option<(&str, char)> {
    ['$', '€', '£', '¥'].into_iter().find_map(|sign| {
        words
            .strip_prefix(sign)
            .map(str::trim_start)
            .or_else(|| words.strip_suffix(sign).map(str::trim_end))
            .map(|body| (body, sign))
    })
}

/// Digits with at most one decimal sign and thousands in their place.
fn plain_number(text: &str) -> Option<f64> {
    if !text
        .chars()
        .all(|c| c.is_ascii_digit() || c == '.' || c == ',')
        || text.ends_with(['.', ','])
    {
        return None;
    }
    let dots = text.matches('.').count();
    let commas = text.matches(',').count();
    // Which sign is the decimal one, and which groups the thousands.
    let (whole, decimals, group) = match (dots, commas) {
        (0, 0) => (text, "", None),
        (1, 0) | (0, 1) => {
            let (whole, decimals) = text.split_once(['.', ','])?;
            // Three digits after one sign: decimals here, thousands there.
            // A lone zero before it can only be decimals.
            if decimals.len() == 3 && whole != "0" {
                return None;
            }
            (whole, decimals, None)
        }
        (_, 0) => (text, "", Some('.')),
        (0, _) => (text, "", Some(',')),
        _ => {
            let last = text.rfind(['.', ','])?;
            let sign = text[last..].chars().next()?;
            let other = if sign == '.' { ',' } else { '.' };
            (&text[..last], &text[last + 1..], Some(other))
        }
    };
    let whole = match group {
        None => whole.to_owned(),
        Some(sign) => {
            let groups: Vec<&str> = whole.split(sign).collect();
            let right = groups.iter().skip(1).all(|g| g.len() == 3)
                && groups.first().is_some_and(|g| (1..=3).contains(&g.len()));
            if !right {
                return None;
            }
            groups.concat()
        }
    };
    let only_digits = |s: &str| s.chars().all(|c| c.is_ascii_digit());
    let well_formed = !whole.is_empty()
        && only_digits(&whole)
        && only_digits(decimals)
        && (whole.len() == 1 || !whole.starts_with('0'))
        && whole.len() + decimals.len() <= MAX_DIGITS;
    if !well_formed {
        return None;
    }
    if decimals.is_empty() {
        whole.parse().ok()
    } else {
        format!("{whole}.{decimals}").parse().ok()
    }
}

/// The data of a table, as a chart takes it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TableData {
    /// The names of the columns of numbers.
    pub headers: Vec<String>,
    /// The words of the first column, if it is not one of numbers.
    pub categories: Vec<String>,
    /// The columns of numbers; `None` where a cell holds none.
    pub columns: Vec<Vec<Option<f64>>>,
}

/// The categories and the columns of numbers of `t`. A column is one of
/// numbers when it holds some and no words.
#[must_use]
pub fn data(t: &TableNode) -> TableData {
    let first = usize::from(t.header);
    let words = |r: usize, c: usize| t.cell(r, c).map_or("", |cell| cell.text.as_str());
    let mut out = TableData::default();
    for c in 0..t.columns.len() {
        let values: Vec<CellValue> = (first..t.rows.len()).map(|r| value(words(r, c))).collect();
        let numbers = values.iter().any(|v| matches!(v, CellValue::Number(..)))
            && !values.contains(&CellValue::Text);
        if numbers {
            out.headers.push(if t.header {
                words(0, c).to_owned()
            } else {
                String::new()
            });
            out.columns.push(
                values
                    .iter()
                    .map(|v| match v {
                        CellValue::Number(n, _) => Some(*n),
                        CellValue::Empty | CellValue::Text => None,
                    })
                    .collect(),
            );
        } else if c == 0 {
            out.categories = (first..t.rows.len())
                .map(|r| words(r, 0).to_owned())
                .collect();
        }
    }
    out
}
