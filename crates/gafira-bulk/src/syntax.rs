//! The syntax of a field in a text (spec 104 §7.3): `{{key}}` with a simple
//! key, or `{{["Any title"]}}` with a JSON string; `\{{` writes `{{` and
//! `\\` a backslash. Anything else is text, as it is written.

use std::ops::Range;

/// The column a field names: by its simple key or by its original title.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum FieldRef {
    Key(String),
    Name(String),
}

/// A piece of a text: words as they are, or a field.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Piece {
    Text(String),
    Field(FieldRef),
}

/// How `field` is written in a text.
#[must_use]
pub fn token(field: &FieldRef) -> String {
    match field {
        FieldRef::Key(key) => format!("{{{{{key}}}}}"),
        FieldRef::Name(name) => {
            let quoted = serde_json::to_string(name).unwrap_or_else(|_| format!("\"{name}\""));
            format!("{{{{[{quoted}]}}}}")
        }
    }
}

/// The pieces of `source`, each with the bytes of the source it came from;
/// the ranges follow one another and cover the whole source. A text piece
/// is the source as it is, except an escape (`\{{`, `\\`), which is a piece
/// of its own.
#[must_use]
pub fn parse(source: &str) -> Vec<(Range<usize>, Piece)> {
    let mut out: Vec<(Range<usize>, Piece)> = Vec::new();
    let mut start = 0;
    let mut at = 0;
    let flush = |out: &mut Vec<(Range<usize>, Piece)>, from: usize, to: usize| {
        if to > from {
            out.push((from..to, Piece::Text(source[from..to].to_owned())));
        }
    };
    while at < source.len() {
        let rest = &source[at..];
        if rest.starts_with("\\\\") {
            flush(&mut out, start, at);
            out.push((at..at + 2, Piece::Text("\\".to_owned())));
            at += 2;
            start = at;
        } else if rest.starts_with("\\{{") {
            flush(&mut out, start, at);
            out.push((at..at + 3, Piece::Text("{{".to_owned())));
            at += 3;
            start = at;
        } else if rest.starts_with("{{")
            && let Some((field, len)) = field(rest)
        {
            flush(&mut out, start, at);
            out.push((at..at + len, Piece::Field(field)));
            at += len;
            start = at;
        } else {
            at += rest.chars().next().map_or(1, char::len_utf8);
        }
    }
    flush(&mut out, start, source.len());
    out
}

/// The fields of `source`, in order.
#[must_use]
pub fn fields(source: &str) -> Vec<FieldRef> {
    parse(source)
        .into_iter()
        .filter_map(|(_, piece)| match piece {
            Piece::Field(field) => Some(field),
            Piece::Text(_) => None,
        })
        .collect()
}

/// The field `text` starts with (it starts with `{{`) and its length.
fn field(text: &str) -> Option<(FieldRef, usize)> {
    let inner = &text[2..];
    let skipped = inner.len() - inner.trim_start_matches(' ').len();
    let body = &inner[skipped..];
    let (field, used) = if let Some(quoted) = body.strip_prefix('[') {
        let end = string_end(quoted)?;
        let name: String = serde_json::from_str(&quoted[..end]).ok()?;
        let after = quoted[end..].strip_prefix(']')?;
        (FieldRef::Name(name), body.len() - after.len())
    } else {
        let len = body
            .bytes()
            .take_while(|b| b.is_ascii_alphanumeric() || *b == b'_')
            .count();
        let key = &body[..len];
        if !gafira_doc::DataSet::is_key(key) {
            return None;
        }
        (FieldRef::Key(key.to_owned()), len)
    };
    let tail = &body[used..];
    let spaces = tail.len() - tail.trim_start_matches(' ').len();
    tail[spaces..].starts_with("}}").then(|| {
        let len = 2 + skipped + used + spaces + 2;
        (field, len)
    })
}

/// The end, past its closing quote, of the JSON string `text` starts with.
fn string_end(text: &str) -> Option<usize> {
    let mut chars = text.char_indices();
    if chars.next()?.1 != '"' {
        return None;
    }
    let mut escaped = false;
    for (at, c) in chars {
        match c {
            _ if escaped => escaped = false,
            '\\' => escaped = true,
            '"' => return Some(at + 1),
            _ => {}
        }
    }
    None
}
