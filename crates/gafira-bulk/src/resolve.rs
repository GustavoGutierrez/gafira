//! The template filled in with a row (spec 104 §7.4, §10): a copy of the
//! template pages with the fields of their texts replaced by the values of
//! the row, without changing the template.

use std::collections::BTreeMap;
use std::sync::Arc;

use gafira_doc::{
    DataSet, Document, Node, NodeId, NodeKind, Page, PageId, Paragraph, RichText, Span,
    ValidationError,
};

use crate::syntax::{FieldRef, Piece, parse};

/// The order of the pages of a run: every template page of the first row,
/// then of the second… (§10.1).
#[must_use]
pub fn plan(rows: &[usize], pages: &[PageId]) -> Vec<(usize, PageId)> {
    rows.iter()
        .flat_map(|row| pages.iter().map(move |page| (*row, *page)))
        .collect()
}

/// The template pages of `doc`: those `set` names that are in it, in the
/// order of the document, or every page that is not hidden.
#[must_use]
pub fn template_pages(doc: &Document, set: &DataSet) -> Vec<PageId> {
    let chosen: Vec<PageId> = doc
        .pages()
        .iter()
        .filter(|p| set.template_pages.contains(&p.id))
        .map(|p| p.id)
        .collect();
    if !chosen.is_empty() {
        return chosen;
    }
    doc.pages()
        .iter()
        .filter(|p| !p.hidden)
        .map(|p| p.id)
        .collect()
}

/// The value of `field` in the row at `row` of `set`: the column with that
/// key or title, else `fila` or `row`, the number of the row from 1.
#[must_use]
pub fn lookup(set: &DataSet, row: usize, field: &FieldRef) -> Option<String> {
    let column = match field {
        FieldRef::Key(key) => set.column_by_key(key),
        FieldRef::Name(name) => set.column_by_name(name),
    };
    match (column, field) {
        (Some(column), _) => set.value(row, column.id).map(str::to_owned),
        (None, FieldRef::Key(key)) if key == "fila" || key == "row" => Some((row + 1).to_string()),
        (None, _) => None,
    }
}

/// `text` with its fields replaced; the fields with no value stay written
/// and are listed. Values are never read as fields.
#[must_use]
pub fn resolve_plain(
    text: &str,
    value: &dyn Fn(&FieldRef) -> Option<String>,
) -> (String, Vec<FieldRef>) {
    let mut out = String::with_capacity(text.len());
    let mut missing = Vec::new();
    for (range, piece) in parse(text) {
        match piece {
            Piece::Text(words) => out.push_str(&words),
            Piece::Field(field) => match value(&field) {
                Some(v) => out.push_str(&v),
                None => {
                    out.push_str(&text[range]);
                    missing.push(field);
                }
            },
        }
    }
    (out, missing)
}

/// `rich` with its fields replaced (§7.4): the fixed words keep their
/// styles, a field takes the style of the span it starts in, even when it is
/// written across spans, and a line in a value starts a new paragraph. The
/// fields with no value stay written and are listed.
#[must_use]
pub fn resolve_rich(
    rich: &RichText,
    value: &dyn Fn(&FieldRef) -> Option<String>,
) -> (RichText, Vec<FieldRef>) {
    let mut missing = Vec::new();
    let mut paragraphs = Vec::new();
    for paragraph in &rich.paragraphs {
        let whole: String = paragraph.spans.iter().map(|s| s.text.as_str()).collect();
        if !whole.contains("{{") && !whole.contains('\\') {
            paragraphs.push(paragraph.clone());
            continue;
        }
        // Where each span starts in the paragraph.
        let mut starts = Vec::with_capacity(paragraph.spans.len());
        let mut at = 0;
        for span in &paragraph.spans {
            starts.push(at);
            at += span.text.len();
        }
        let span_at = |byte: usize| starts.iter().rposition(|s| *s <= byte).unwrap_or(0);
        let mut spans: Vec<Span> = Vec::new();
        let push = |spans: &mut Vec<Span>, text: &str, index: usize| {
            let style = &paragraph.spans[index].style;
            match spans.last_mut() {
                Some(last) if last.style == *style => last.text.push_str(text),
                _ => spans.push(Span {
                    text: text.to_owned(),
                    style: style.clone(),
                }),
            }
        };
        for (range, piece) in parse(&whole) {
            match piece {
                // Words as they are: cut where the spans are.
                Piece::Text(words) if words.len() == range.len() => {
                    let mut from = range.start;
                    while from < range.end {
                        let index = span_at(from);
                        let end = starts.get(index + 1).copied().unwrap_or(whole.len());
                        let to = end.min(range.end);
                        push(&mut spans, &whole[from..to], index);
                        from = to;
                    }
                }
                Piece::Text(words) => push(&mut spans, &words, span_at(range.start)),
                Piece::Field(field) => {
                    let index = span_at(range.start);
                    match value(&field) {
                        Some(v) => push(&mut spans, &v, index),
                        None => {
                            push(&mut spans, &whole[range.clone()], index);
                            missing.push(field);
                        }
                    }
                }
            }
        }
        if spans.is_empty()
            && let Some(first) = paragraph.spans.first()
        {
            spans.push(Span {
                text: String::new(),
                style: first.style.clone(),
            });
        }
        paragraphs.extend(split_lines(spans));
    }
    (RichText { paragraphs }, missing)
}

/// `spans` as paragraphs, one more at each `\n` of their words.
fn split_lines(spans: Vec<Span>) -> Vec<Paragraph> {
    let mut out = vec![Paragraph::default()];
    for span in spans {
        let mut lines = span.text.split('\n');
        if let Some(first) = lines.next()
            && let Some(current) = out.last_mut()
        {
            current.spans.push(Span {
                text: first.to_owned(),
                style: span.style.clone(),
            });
        }
        for line in lines {
            out.push(Paragraph {
                spans: vec![Span {
                    text: line.to_owned(),
                    style: span.style.clone(),
                }],
            });
        }
    }
    // A span left empty by a line break is not kept beside others.
    for paragraph in &mut out {
        if paragraph.spans.len() > 1 {
            paragraph.spans.retain(|s| !s.text.is_empty());
        }
    }
    out
}

/// The texts of `node` with their fields replaced, and the fields missing.
fn resolve_node(node: &mut Node, value: &dyn Fn(&FieldRef) -> Option<String>) -> Vec<FieldRef> {
    let mut missing = Vec::new();
    let mut rich = |content: &mut RichText| {
        let (out, lost) = resolve_rich(content, value);
        *content = out;
        missing.extend(lost);
    };
    match &mut node.kind {
        NodeKind::Text(text) => rich(&mut text.content),
        NodeKind::Shape(shape) => {
            if let Some(text) = &mut shape.text {
                rich(&mut text.content);
            }
        }
        NodeKind::Table(table) => {
            for cell in &mut table.cells {
                let (out, lost) = resolve_plain(&cell.text, value);
                cell.text = out;
                missing.extend(lost);
            }
        }
        _ => {}
    }
    missing
}

/// Whether `node` has a text with a field or an escape to resolve.
fn has_fields(node: &Node) -> bool {
    let rich = |r: &RichText| {
        r.spans()
            .any(|s| s.text.contains("{{") || s.text.contains('\\'))
    };
    match &node.kind {
        NodeKind::Text(text) => rich(&text.content),
        NodeKind::Shape(shape) => shape.text.as_ref().is_some_and(|t| rich(&t.content)),
        NodeKind::Table(table) => table
            .cells
            .iter()
            .any(|c| c.text.contains("{{") || c.text.contains('\\')),
        _ => false,
    }
}

/// Every field written in the template pages, with its page and element.
fn written(doc: &Document, pages: &[PageId]) -> Vec<(PageId, NodeId, FieldRef)> {
    let mut out = Vec::new();
    for page in pages.iter().filter_map(|id| doc.page(*id)) {
        for (id, node) in &page.nodes {
            if !has_fields(node) {
                continue;
            }
            // Nothing resolves: every field comes back as missing.
            let mut copy = (**node).clone();
            let none = |_: &FieldRef| None;
            let seen = resolve_node(&mut copy, &none);
            out.extend(seen.into_iter().map(|f| (page.id, *id, f)));
        }
    }
    out
}

/// How many elements of the template pages use each field (§7.1).
#[must_use]
pub fn usage(doc: &Document, pages: &[PageId]) -> BTreeMap<FieldRef, usize> {
    let mut seen = std::collections::BTreeSet::new();
    let mut out = BTreeMap::new();
    for (page, node, field) in written(doc, pages) {
        if seen.insert((page, node, field.clone())) {
            *out.entry(field).or_insert(0) += 1;
        }
    }
    out
}

/// The fields of the template pages no column of `set` has (§9: they block
/// a run), each with its page and element.
#[must_use]
pub fn unknown(doc: &Document, set: &DataSet, pages: &[PageId]) -> Vec<(PageId, NodeId, FieldRef)> {
    written(doc, pages)
        .into_iter()
        .filter(|(_, _, field)| !knows(set, field))
        .collect()
}

/// Whether `set` knows `field`, whatever its rows.
fn knows(set: &DataSet, field: &FieldRef) -> bool {
    match field {
        FieldRef::Key(key) if key == "fila" || key == "row" => true,
        FieldRef::Key(key) => set.column_by_key(key).is_some(),
        FieldRef::Name(name) => set.column_by_name(name).is_some(),
    }
}

/// A document of resolved pages and the row each page came from.
#[derive(Clone, Debug)]
pub struct Resolved {
    pub document: Document,
    /// The index in the table of the row of each page, in page order.
    pub rows: Vec<usize>,
    /// The fields with no value, by page of the result, element and field.
    pub missing: Vec<(PageId, NodeId, FieldRef)>,
}

/// A new document with the template pages `pages` of `doc` once per row of
/// `rows`, in [`plan`] order, every field replaced by the value of its row
/// (§10.1). Pages and elements get new ids; a page is named after its
/// template page and the number of its row (`Frente #2`). The result has
/// the assets of `doc`, no timeline and no table (§10.2: it does not carry
/// the rows of other people). `doc` does not change.
///
/// # Errors
/// The rule the result breaks, which a valid template does not.
pub fn resolve(
    doc: &Document,
    set: &DataSet,
    rows: &[usize],
    pages: &[PageId],
) -> Result<Resolved, ValidationError> {
    let mut counter: u64 = 1;
    let mut next = || {
        let id = counter;
        counter += 1;
        id
    };
    let mut out_pages = Vec::new();
    let mut out_rows = Vec::new();
    let mut missing = Vec::new();
    for (row, page_id) in plan(rows, pages) {
        let Some(source) = doc.page(page_id) else {
            continue;
        };
        let value = |field: &FieldRef| lookup(set, row, field);
        let id = PageId::new(next());
        let ids: BTreeMap<NodeId, NodeId> = source
            .nodes
            .keys()
            .map(|old| (*old, NodeId::new(next())))
            .collect();
        let map = |old: &NodeId| ids.get(old).copied().unwrap_or(*old);
        let mut page: Page = Page::clone(source);
        page.id = id;
        page.name = if source.name.is_empty() {
            format!("#{}", row + 1)
        } else {
            format!("{} #{}", source.name, row + 1)
        };
        page.roots = source.roots.iter().map(map).collect();
        page.nodes = source
            .nodes
            .iter()
            .map(|(old, node)| {
                let mut node = Node::clone(node);
                node.base.id = map(old);
                node.children = node.children.iter().map(map).collect();
                if let NodeKind::Line(line) = &mut node.kind {
                    for end in [&mut line.start, &mut line.end] {
                        if let Some(attached) = &mut end.attached {
                            attached.node = map(&attached.node);
                        }
                    }
                }
                if has_fields(&node) {
                    let lost = resolve_node(&mut node, &value);
                    missing.extend(lost.into_iter().map(|f| (id, map(old), f)));
                }
                (map(old), Arc::new(node))
            })
            .collect();
        out_pages.push(Arc::new(page));
        out_rows.push(row);
    }
    let document =
        Document::from_parts_with_assets(counter, doc.dpi(), out_pages, doc.assets().clone())?;
    Ok(Resolved {
        document,
        rows: out_rows,
        missing,
    })
}
