//! The keys of the columns (spec 104 §7.3 rule 6): a title as a simple key,
//! unique in the table, for `{{key}}`.

use std::collections::BTreeSet;

/// A simple key for the column titled `name`, not in `taken`: lower case
/// ASCII letters, digits and `_`, the accents taken off, `_2`, `_3`… after
/// it when it is taken.
#[must_use]
pub fn key_for(name: &str, taken: &BTreeSet<String>) -> String {
    let mut base = String::new();
    for c in name.chars().flat_map(char::to_lowercase) {
        match plain(c) {
            Some(c) => base.push(c),
            None if !base.ends_with('_') => base.push('_'),
            None => {}
        }
    }
    let mut base = base.trim_matches('_').to_owned();
    if base.is_empty() {
        base = "col".to_owned();
    } else if base.starts_with(|c: char| c.is_ascii_digit()) {
        base = format!("c_{base}");
    }
    if !taken.contains(&base) {
        return base;
    }
    (2..)
        .map(|n| format!("{base}_{n}"))
        .find(|k| !taken.contains(k))
        .unwrap_or(base)
}

/// `c` as a letter or digit of a key, its accent taken off.
fn plain(c: char) -> Option<char> {
    let c = match c {
        'á' | 'à' | 'â' | 'ä' | 'ã' | 'å' => 'a',
        'é' | 'è' | 'ê' | 'ë' => 'e',
        'í' | 'ì' | 'î' | 'ï' => 'i',
        'ó' | 'ò' | 'ô' | 'ö' | 'õ' => 'o',
        'ú' | 'ù' | 'û' | 'ü' => 'u',
        'ñ' => 'n',
        'ç' => 'c',
        c => c,
    };
    (c.is_ascii_alphanumeric()).then_some(c)
}

/// The letters of the column at `index` in a spreadsheet: `A`, …, `Z`,
/// `AA`, ….
#[must_use]
pub fn column_letter(index: usize) -> String {
    let mut n = index + 1;
    let mut out = Vec::new();
    while n > 0 {
        let rem = (n - 1) % 26;
        // Below 26: the cast is exact.
        out.push(char::from(b'A' + rem as u8));
        n = (n - 1) / 26;
    }
    out.iter().rev().collect()
}
