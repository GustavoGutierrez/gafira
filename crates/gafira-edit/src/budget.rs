//! The history's memory estimate (Q7): what a snapshot holds that the next
//! newer state does not share with it.

use std::mem::size_of;
use std::sync::Arc;

use gafira_doc::{Document, Node, NodeId, NodeKind, Page};
use serde_json::Value;

/// Bytes of `old` that `newer` does not share by `Arc` identity.
///
/// A fixed size model, not real heap: record sizes plus the heap length of
/// strings and extension payloads. Pages and nodes that are pointer-equal in
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
        NodeKind::Group(_) | NodeKind::Shape(_) => 0,
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
        Page, PageSize, Placement,
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
}
