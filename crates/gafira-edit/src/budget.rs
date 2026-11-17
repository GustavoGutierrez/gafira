//! The history's memory estimate (Q7): what a snapshot holds that the next
//! newer state does not share with it.

use std::mem::size_of;
use std::sync::Arc;

use gafira_doc::{Document, Guide, Node, NodeId, NodeKind, Page};
use serde_json::Value;

/// Bytes of `old` that `newer` does not share by `Arc` identity.
///
/// A fixed size model, not real heap: record sizes plus the heap length of
/// strings, guides and extension payloads. Pages and nodes that are pointer-equal in
/// `newer` count nothing.
pub(crate) fn unshared_bytes(old: &Document, newer: &Document) -> usize {
    let record = size_of::<Document>() + size_of_val(old.pages());
    let pages: usize = old
        .pages()
        .iter()
        .map(|page| {
            let counterpart = newer.pages().iter().find(|p| p.id == page.id);
            match counterpart {
                Some(same) if Arc::ptr_eq(page, same) => 0,
                _ => page_bytes(page, counterpart.map(|p| &**p)),
            }
        })
        .sum();
    record + pages
}

/// The page record and the nodes of `page` that `newer` does not share.
fn page_bytes(page: &Page, newer: Option<&Page>) -> usize {
    let record = size_of::<Page>()
        + page.name.len()
        + page.roots.len() * size_of::<NodeId>()
        + page.guides.0.len() * size_of::<Guide>()
        + page.nodes.len() * (size_of::<NodeId>() + size_of::<Arc<Node>>());
    let nodes: usize = page
        .nodes
        .iter()
        .filter(|(id, node)| {
            !newer
                .and_then(|p| p.nodes.get(id))
                .is_some_and(|n| Arc::ptr_eq(node, n))
        })
        .map(|(_, node)| node_bytes(node))
        .sum();
    record + nodes
}

fn node_bytes(node: &Node) -> usize {
    let heap = match &node.kind {
        NodeKind::Extension(ext) => ext.type_id.len() + value_bytes(&ext.payload),
        NodeKind::Text(text) => text
            .content
            .spans()
            .map(|span| span.text.len() + span.style.font.len() + size_of::<gafira_doc::Span>())
            .sum(),
        NodeKind::Group(_) | NodeKind::Shape(_) | NodeKind::Image(_) => 0,
    };
    size_of::<Node>()
        + node.base.name.as_deref().map_or(0, str::len)
        + node.children.len() * size_of::<NodeId>()
        + heap
}

/// The size of a JSON value: one record per value plus string and key bytes.
fn value_bytes(value: &Value) -> usize {
    size_of::<Value>()
        + match value {
            Value::String(text) => text.len(),
            Value::Array(items) => items.iter().map(value_bytes).sum(),
            Value::Object(map) => map.iter().map(|(k, v)| k.len() + value_bytes(v)).sum(),
            Value::Null | Value::Bool(_) | Value::Number(_) => 0,
        }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use gafira_doc::{
        DocMut, Document, EditToken, ElementBase, ExtensionNode, GroupNode, Node, NodeId, NodeKind,
        Page, PageId, PageSize, Placement,
    };
    use gafira_geom::{Length, Unit};

    use super::unshared_bytes;

    fn size() -> PageSize {
        PageSize {
            width: Length::new(10.0, Unit::Px),
            height: Length::new(10.0, Unit::Px),
        }
    }

    /// A document with one page of `count` group nodes; `name` and `kind`
    /// shape node 0 (the others are plain groups).
    fn document(count: usize, name: Option<String>, kind: Option<NodeKind>) -> Document {
        let token = EditToken::__new_for_gafira_edit_only();
        let mut doc = Document::default();
        let mut edit = doc.edit(&token);
        let page = edit.alloc_page_id().expect("ids are available");
        let mut page = Page::new(page, String::new(), size());
        for i in 0..count {
            let id = edit.alloc_node_id().expect("ids are available");
            let mut base = ElementBase::new(id, Placement::new(0.0, 0.0, 1.0, 1.0));
            let mut node_kind = NodeKind::Group(GroupNode {});
            if i == 0 {
                base.name = name.clone();
                node_kind = kind.clone().unwrap_or(node_kind);
            }
            page.nodes.insert(id, Arc::new(Node::new(base, node_kind)));
            page.roots.push(id);
        }
        edit.pages_mut().push(Arc::new(page));
        doc
    }

    /// `doc` with the placement of the first `count` nodes of page 0 moved.
    fn moved(doc: &Document, count: usize) -> Document {
        let token = EditToken::__new_for_gafira_edit_only();
        let mut next = doc.clone();
        let mut edit: DocMut<'_> = next.edit(&token);
        let page = edit.as_doc().pages()[0].id;
        let ids: Vec<NodeId> = edit.as_doc().pages()[0]
            .nodes
            .keys()
            .copied()
            .take(count)
            .collect();
        for id in ids {
            edit.node_mut(page, id)
                .expect("the node exists")
                .base
                .placement
                .x = 5.0;
        }
        next
    }

    fn plain(count: usize) -> Document {
        document(count, None, None)
    }

    #[test]
    fn a_state_that_shares_every_page_counts_only_its_own_record() {
        let few = plain(2);
        let many = plain(100);
        let record = unshared_bytes(&few, &few.clone());
        assert!(record > 0);
        assert_eq!(unshared_bytes(&many, &many.clone()), record);
    }

    #[test]
    fn one_changed_node_does_not_count_its_siblings() {
        let doc = plain(100);
        let shared = unshared_bytes(&doc, &doc.clone());
        let one = unshared_bytes(&doc, &moved(&doc, 1));
        let two = unshared_bytes(&doc, &moved(&doc, 2));
        let all = unshared_bytes(&doc, &moved(&doc, 100));
        // The page record is paid once; every further node costs its own.
        assert!(one > shared && two > one);
        assert_eq!(all - one, 99 * (two - one));
    }

    #[test]
    fn a_page_missing_from_the_newer_state_counts_in_full() {
        let doc = plain(20);
        let empty = Document::default();
        assert_eq!(
            unshared_bytes(&doc, &empty),
            unshared_bytes(&doc, &moved(&doc, 20))
        );
    }

    // S38-03: the outline and the radius live inside the node record, so a
    // stroked shape costs what a plain one does.
    #[test]
    fn s38_03_an_outline_adds_no_heap_to_the_estimate() {
        use gafira_color::{Color, Rgba};
        use gafira_doc::{ShapeGeometry, ShapeNode, ShapeStroke};
        let bare = ShapeNode::new(ShapeGeometry::Rect);
        let mut stroked = bare;
        stroked.stroke = Some(ShapeStroke::new(
            Color::Srgb(Rgba::new(0.0, 0.0, 0.0, 1.0)),
            4.0,
        ));
        stroked.corner_radius = 8.0;
        let newer = Document::default();
        let cost = |shape| unshared_bytes(&document(1, None, Some(NodeKind::Shape(shape))), &newer);
        assert_eq!(cost(stroked), cost(bare));
        assert_eq!(cost(bare), unshared_bytes(&plain(1), &newer));
    }

    #[test]
    fn strings_and_payloads_count_their_heap_length() {
        let base = plain(1);
        let named = document(1, Some("n".repeat(1000)), None);
        let newer = Document::default();
        assert_eq!(
            unshared_bytes(&named, &newer),
            unshared_bytes(&base, &newer) + 1000
        );

        let token = EditToken::__new_for_gafira_edit_only();
        let mut titled = base.clone();
        let page = titled.pages()[0].id;
        titled
            .edit(&token)
            .page_mut(page)
            .expect("the page exists")
            .name = "p".repeat(300);
        assert_eq!(
            unshared_bytes(&titled, &newer),
            unshared_bytes(&base, &newer) + 300
        );

        let extension = |payload: serde_json::Value| {
            NodeKind::Extension(ExtensionNode {
                type_id: "x".into(),
                schema_version: 1,
                payload,
            })
        };
        let small = document(1, None, Some(extension(serde_json::json!("a"))));
        let big = document(1, None, Some(extension(serde_json::json!("a".repeat(500)))));
        assert!(unshared_bytes(&big, &newer) >= unshared_bytes(&small, &newer) + 499);
    }

    // The exact model, term by term, against a newer state that shares
    // nothing: every size below is written out from the records themselves.

    const NODE_ID: usize = size_of::<NodeId>();
    const NODE: usize = size_of::<Node>();
    const VALUE: usize = size_of::<serde_json::Value>();
    /// A node's slot in a page arena: its key and its `Arc` handle.
    const ARENA_SLOT: usize = NODE_ID + size_of::<Arc<Node>>();

    fn page(id: u64) -> Page {
        Page::new(PageId::new(id), String::new(), size())
    }

    fn group(id: u64) -> Node {
        let base = ElementBase::new(NodeId::new(id), Placement::new(0.0, 0.0, 1.0, 1.0));
        Node::new(base, NodeKind::Group(GroupNode {}))
    }

    fn with_node(mut page: Page, node: Node) -> Page {
        page.nodes.insert(node.base.id, Arc::new(node));
        page
    }

    fn of_pages(pages: Vec<Page>) -> Document {
        let token = EditToken::__new_for_gafira_edit_only();
        let mut doc = Document::default();
        doc.edit(&token)
            .pages_mut()
            .extend(pages.into_iter().map(Arc::new));
        doc
    }

    /// What `doc` alone costs when nothing is shared with a later state.
    fn alone(doc: &Document) -> usize {
        unshared_bytes(doc, &Document::default())
    }

    #[test]
    fn the_document_record_counts_one_handle_per_page() {
        // Seven pages: a subtraction instead of the sum would underflow.
        let doc = of_pages((1..=7).map(page).collect());
        let handle = size_of::<Arc<Page>>();
        assert_eq!(
            unshared_bytes(&doc, &doc.clone()),
            size_of::<Document>() + 7 * handle
        );
    }

    #[test]
    fn a_page_counts_its_record_name_roots_and_arena_slots() {
        let mut full = page(1);
        full.name = "abc".into();
        full.roots = vec![NodeId::new(10), NodeId::new(11)];
        for id in [10, 11, 12] {
            full = with_node(full, group(id));
        }
        let doc = of_pages(vec![full]);
        let expected = size_of::<Document>()
            + size_of::<Arc<Page>>()
            + (size_of::<Page>() + 3 + 2 * NODE_ID + 3 * ARENA_SLOT)
            + 3 * NODE;
        assert_eq!(alone(&doc), expected);
    }

    #[test]
    fn a_node_counts_its_record_name_and_children() {
        let plain = alone(&of_pages(vec![with_node(page(1), group(10))]));
        let mut busy = group(10);
        busy.base.name = Some("hello".into());
        // Past 24 children the list outweighs the node record itself.
        busy.children = (20..50).map(NodeId::new).collect();
        let busy = alone(&of_pages(vec![with_node(page(1), busy)]));
        assert_eq!(busy - plain, 5 + 30 * NODE_ID);
    }

    #[test]
    fn an_extension_counts_its_type_id_and_payload_exactly() {
        let extension = |type_id: &str, payload: serde_json::Value| {
            let kind = NodeKind::Extension(ExtensionNode {
                type_id: type_id.into(),
                schema_version: 1,
                payload,
            });
            let mut node = group(10);
            node.kind = kind;
            alone(&of_pages(vec![with_node(page(1), node)]))
        };
        let base = extension("x", serde_json::Value::Null);
        // Only the type id differs.
        assert_eq!(extension("0123456789", serde_json::Value::Null), base + 9);
        // Each case: payload, its size in the model.
        let cases = [
            (serde_json::json!("a".repeat(500)), VALUE + 500),
            (serde_json::json!({"abc": null}), 2 * VALUE + 3),
            (serde_json::json!([null, null, "xy"]), 4 * VALUE + 2),
            (serde_json::json!({"k": {"lm": [true]}}), 4 * VALUE + 3),
        ];
        for (payload, size) in cases {
            let label = payload.to_string();
            assert_eq!(extension("x", payload), base - VALUE + size, "{label}");
        }
    }
}
