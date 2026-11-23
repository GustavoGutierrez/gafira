//! The parts of a workbook: a ZIP from anyone, read within limits (S82-04).

use std::io::{Cursor, Read};

use roxmltree::{Document, Node, ParsingOptions};
use zip::ZipArchive;
use zip::result::ZipError;

use crate::ReadError;

/// The most bytes of the file, of a part and of all the parts read.
pub(crate) const MAX_FILE: usize = 32 << 20;
const MAX_PART: u64 = 32 << 20;
const MAX_TOTAL: u64 = 256 << 20;

/// The most elements, attributes and texts of a part.
const MAX_NODES: u32 = 2_000_000;

pub(crate) struct Package<'a> {
    zip: ZipArchive<Cursor<&'a [u8]>>,
    /// The bytes that may still be read.
    left: u64,
}

/// What a part says about another one.
pub(crate) struct Relation {
    pub id: String,
    /// The last word of its type: `worksheet`, `styles`…
    pub kind: String,
    /// The part it names.
    pub target: String,
}

impl<'a> Package<'a> {
    pub fn open(bytes: &'a [u8]) -> Result<Self, ReadError> {
        // A compound file: the old `.xls`, or a workbook with a password.
        if bytes.starts_with(&[0xD0, 0xCF, 0x11, 0xE0]) {
            return Err(ReadError::Unsupported);
        }
        if bytes.len() > MAX_FILE {
            return Err(ReadError::TooLarge);
        }
        let zip = ZipArchive::new(Cursor::new(bytes)).map_err(|_| ReadError::Damaged)?;
        Ok(Self {
            zip,
            left: MAX_TOTAL,
        })
    }

    /// The text of a part; `None` when there is no such part.
    pub fn text(&mut self, name: &str) -> Result<Option<String>, ReadError> {
        let part = match self.zip.by_name(name) {
            Ok(part) => part,
            Err(ZipError::FileNotFound) => return Ok(None),
            Err(_) => return Err(ReadError::Damaged),
        };
        let most = MAX_PART.min(self.left);
        let mut bytes = Vec::new();
        part.take(most + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| ReadError::Damaged)?;
        let size = bytes.len() as u64;
        if size > most {
            return Err(ReadError::TooLarge);
        }
        self.left -= size;
        let text = String::from_utf8(bytes).map_err(|_| ReadError::Damaged)?;
        Ok(Some(match text.strip_prefix('\u{FEFF}') {
            Some(rest) => rest.to_owned(),
            None => text,
        }))
    }

    /// What `part` says about others; nothing when it says it wrong.
    pub fn relations(&mut self, part: &str) -> Result<Vec<Relation>, ReadError> {
        let (folder, file) = part.rsplit_once('/').unwrap_or(("", part));
        let slash = if folder.is_empty() { "" } else { "/" };
        let Some(text) = self.text(&format!("{folder}{slash}_rels/{file}.rels"))? else {
            return Ok(Vec::new());
        };
        let Ok(xml) = parse(&text) else {
            return Ok(Vec::new());
        };
        Ok(xml
            .root_element()
            .children()
            .filter(|n| n.has_tag_name_local("Relationship"))
            .filter(|n| n.attribute("TargetMode") != Some("External"))
            .filter_map(|n| {
                Some(Relation {
                    id: n.attribute("Id")?.to_owned(),
                    kind: n.attribute("Type")?.rsplit('/').next()?.to_owned(),
                    target: resolve(folder, n.attribute("Target")?),
                })
            })
            .collect())
    }
}

/// The name of the part `target` is, seen from `folder`.
fn resolve(folder: &str, target: &str) -> String {
    let mut path: Vec<&str> = if target.starts_with('/') {
        Vec::new()
    } else {
        folder.split('/').filter(|p| !p.is_empty()).collect()
    };
    for piece in target.split('/') {
        match piece {
            "" | "." => {}
            ".." => {
                path.pop();
            }
            _ => path.push(piece),
        }
    }
    path.join("/")
}

/// The most elements inside one another: the parser goes down a stack.
const MAX_DEPTH: usize = 256;

/// Whether `text` nests its elements past [`MAX_DEPTH`]. What is malformed
/// is left for the parser, which stops there.
fn is_too_deep(text: &str) -> bool {
    let mut depth = 0_usize;
    let mut rest = text;
    while let Some(at) = rest.find('<') {
        rest = &rest[at + 1..];
        let past = |end: &str| rest.find(end).map_or("", |at| &rest[at + end.len()..]);
        if rest.starts_with("!--") {
            rest = past("-->");
        } else if rest.starts_with("![CDATA[") {
            rest = past("]]>");
        } else if rest.starts_with('?') {
            rest = past("?>");
        } else if rest.starts_with('/') {
            depth = depth.saturating_sub(1);
        } else {
            // To the end of the tag: a value may hold its sign.
            let mut quote = None;
            let end = rest.bytes().position(|b| match (quote, b) {
                (None, b'"' | b'\'') => {
                    quote = Some(b);
                    false
                }
                (None, b) => b == b'>',
                (Some(q), b) => {
                    if q == b {
                        quote = None;
                    }
                    false
                }
            });
            let Some(end) = end else { return false };
            if !rest[..end].ends_with('/') {
                depth += 1;
                if depth > MAX_DEPTH {
                    return true;
                }
            }
            rest = &rest[end..];
        }
    }
    false
}

/// A part as XML: no DTD, no more than the nodes allowed.
pub(crate) fn parse(text: &str) -> Result<Document<'_>, ReadError> {
    if is_too_deep(text) {
        return Err(ReadError::Damaged);
    }
    let options = ParsingOptions {
        allow_dtd: false,
        nodes_limit: MAX_NODES,
    };
    Document::parse_with_options(text, options).map_err(|error| match error {
        roxmltree::Error::NodesLimitReached => ReadError::TooLarge,
        _ => ReadError::Damaged,
    })
}

/// Elements by their local name: the two dialects of the standard differ in
/// their namespaces.
pub(crate) trait Local<'a, 'input: 'a> {
    fn has_tag_name_local(&self, name: &str) -> bool;
    fn child(&self, name: &str) -> Option<Node<'a, 'input>>;
    fn all(&self, name: &'static str) -> impl Iterator<Item = Node<'a, 'input>>;
    /// The attribute, if it says yes; `default` when it is not there.
    fn flag(&self, name: &str, default: bool) -> bool;
    fn float(&self, name: &str) -> Option<f64>;
    fn index(&self, name: &str) -> Option<usize>;
    /// The relation the element names.
    fn relation(&self) -> Option<&'a str>;
}

impl<'a, 'input: 'a> Local<'a, 'input> for Node<'a, 'input> {
    fn has_tag_name_local(&self, name: &str) -> bool {
        self.is_element() && self.tag_name().name() == name
    }

    fn child(&self, name: &str) -> Option<Node<'a, 'input>> {
        self.children().find(|n| n.has_tag_name_local(name))
    }

    fn all(&self, name: &'static str) -> impl Iterator<Item = Node<'a, 'input>> {
        self.children().filter(move |n| n.has_tag_name_local(name))
    }

    fn flag(&self, name: &str, default: bool) -> bool {
        self.attribute(name)
            .map_or(default, |v| matches!(v, "1" | "true" | "on"))
    }

    fn float(&self, name: &str) -> Option<f64> {
        let v: f64 = self.attribute(name)?.trim().parse().ok()?;
        v.is_finite().then_some(v)
    }

    fn index(&self, name: &str) -> Option<usize> {
        self.attribute(name)?.trim().parse().ok()
    }

    fn relation(&self) -> Option<&'a str> {
        self.attributes()
            .find(|a| {
                a.name() == "id" && a.namespace().is_some_and(|ns| ns.contains("relationships"))
            })
            .map(|a| a.value())
    }
}
