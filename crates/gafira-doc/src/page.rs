//! A page and its node arena.

use std::collections::BTreeMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use gafira_geom::Length;

use crate::layout::default_bleed;
use crate::{GridSettings, GuideSet, Margins, Node, NodeId, PageId, PageSize};

/// One page: its size, flags, root order and the arena that owns its nodes.
///
/// Ids in `roots` and in node `children` resolve only inside this page's own
/// `nodes`. Structural violations are representable; `Document::validate`
/// rejects them. The page flags never change a node's own `locked`/`visible`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Page {
    /// The page id, unique in the document.
    pub id: PageId,
    /// Display name.
    #[serde(default)]
    pub name: String,
    /// Dimensions in any unit.
    pub size: PageSize,
    /// Bleed past the page edge, the same on all four sides.
    #[serde(default = "default_bleed")]
    pub bleed: Length,
    /// Page margins.
    #[serde(default)]
    pub margins: Margins,
    /// Guide lines, in the order they were added.
    #[serde(default)]
    pub guides: GuideSet,
    /// The background grid.
    #[serde(default)]
    pub grid: GridSettings,
    /// Whether the whole page is protected from edits.
    #[serde(default)]
    pub locked: bool,
    /// Whether the whole page is hidden.
    #[serde(default)]
    pub hidden: bool,
    /// Top-level node ids, back to front.
    #[serde(default)]
    pub roots: Vec<NodeId>,
    /// The arena, iterating in ascending [`NodeId`] order.
    #[serde(default, with = "crate::arena")]
    pub nodes: BTreeMap<NodeId, Arc<Node>>,
}

impl Page {
    /// An unlocked, visible page with no nodes.
    pub fn new(id: PageId, name: String, size: PageSize) -> Self {
        Self {
            id,
            name,
            size,
            bleed: default_bleed(),
            margins: Margins::default(),
            guides: GuideSet::default(),
            grid: GridSettings::default(),
            locked: false,
            hidden: false,
            roots: Vec::new(),
            nodes: BTreeMap::new(),
        }
    }

    /// The node with this id in this page's arena, if any.
    pub fn node(&self, id: NodeId) -> Option<&Node> {
        self.nodes.get(&id).map(Arc::as_ref)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use gafira_geom::{Length, Unit};

    use super::*;
    use crate::{ElementBase, GroupNode, NodeKind, Placement};

    fn size() -> PageSize {
        PageSize {
            width: Length::new(100.0, Unit::Px),
            height: Length::new(50.0, Unit::Px),
        }
    }

    fn node(id: u64) -> Arc<Node> {
        Arc::new(Node::new(
            ElementBase::new(NodeId::new(id), Placement::new(0.0, 0.0, 1.0, 1.0)),
            NodeKind::Group(GroupNode {}),
        ))
    }

    #[test]
    fn page_defaults() {
        let p = Page::new(PageId::new(1), "Home".to_string(), size());
        assert_eq!(p.id, PageId::new(1));
        assert_eq!(p.name, "Home");
        assert_eq!(p.size, size());
        assert!(!p.locked);
        assert!(!p.hidden);
        assert!(p.roots.is_empty());
        assert!(p.nodes.is_empty());
    }

    #[test]
    fn iteration_is_in_ascending_id_order() {
        let mut p = Page::new(PageId::new(1), String::new(), size());
        for id in [10, 2, 7] {
            p.nodes.insert(NodeId::new(id), node(id));
        }
        let order: Vec<u64> = p.nodes.keys().map(|k| k.get()).collect();
        assert_eq!(order, [2, 7, 10]);
    }

    #[test]
    fn lookup_stays_inside_the_page() {
        let first = Page::new(PageId::new(1), String::new(), size());
        let mut second = Page::new(PageId::new(2), String::new(), size());
        for id in [4, 9, 12] {
            second.nodes.insert(NodeId::new(id), node(id));
        }
        assert!(first.node(NodeId::new(9)).is_none());
        for id in [4, 9, 12] {
            let found = second.node(NodeId::new(id)).unwrap();
            assert_eq!(found.base.id, NodeId::new(id));
        }
        assert!(second.node(NodeId::new(5)).is_none());
    }

    #[test]
    fn structural_violations_are_representable() {
        let mut p = Page::new(PageId::new(1), String::new(), size());
        let mut looped = Node::new(
            ElementBase::new(NodeId::new(3), Placement::new(0.0, 0.0, 1.0, 1.0)),
            NodeKind::Group(GroupNode {}),
        );
        looped.children.push(NodeId::new(3));
        p.nodes.insert(NodeId::new(3), Arc::new(looped));
        p.roots.push(NodeId::new(99));
        p.roots.push(NodeId::new(99));
        assert!(p.node(p.roots[0]).is_none());
        assert_eq!(p.roots.len(), 2);
        let n = p.node(NodeId::new(3)).unwrap();
        assert_eq!(n.children, [NodeId::new(3)]);
    }

    #[test]
    fn page_flags_do_not_cascade() {
        let mut p = Page::new(PageId::new(1), String::new(), size());
        p.nodes.insert(NodeId::new(4), node(4));
        p.locked = true;
        p.hidden = true;
        let n = p.node(NodeId::new(4)).unwrap();
        assert!(n.base.visible);
        assert!(!n.base.locked);
    }

    #[test]
    fn cloning_a_page_shares_its_nodes() {
        let mut p = Page::new(PageId::new(1), String::new(), size());
        p.nodes.insert(NodeId::new(4), node(4));
        let c = p.clone();
        assert!(Arc::ptr_eq(
            &p.nodes[&NodeId::new(4)],
            &c.nodes[&NodeId::new(4)]
        ));
    }

    fn page_json(extra: &str) -> String {
        let size = r#""size":{"width":{"value":1,"unit":"px"},"height":{"value":2,"unit":"px"}}"#;
        format!(r#"{{"id":1,{size}{extra}}}"#)
    }

    fn node_json(id: u64) -> String {
        let b = format!(r#"{{"id":{id},"placement":{{"x":0,"y":0,"w":1,"h":1}}}}"#);
        format!(r#"{{"base":{b},"kind":{{"type":"group"}}}}"#)
    }

    fn ids(p: &Page) -> Vec<u64> {
        p.nodes.keys().map(|k| k.get()).collect()
    }

    #[test]
    fn page_wire_form_writes_every_field() {
        let p = Page::new(PageId::new(1), "P".to_string(), size());
        let json = serde_json::to_string(&p).unwrap();
        assert_eq!(
            json,
            concat!(
                r#"{"id":1,"name":"P","size":{"width":{"value":100.0,"unit":"px"},"#,
                r#""height":{"value":50.0,"unit":"px"}},"bleed":{"value":0.0,"unit":"mm"},"#,
                r#""margins":{"top":{"value":0.0,"unit":"px"},"right":{"value":0.0,"unit":"px"},"#,
                r#""bottom":{"value":0.0,"unit":"px"},"left":{"value":0.0,"unit":"px"},"#,
                r#""linked":true},"guides":[],"grid":{"visible":false,"#,
                r#""step":{"value":10.0,"unit":"px"},"subdivisions":1,"style":"lines","#,
                r#""snap":true},"locked":false,"hidden":false,"roots":[],"nodes":[]}"#
            )
        );
        assert_eq!(serde_json::from_str::<Page>(&json).unwrap(), p);
    }

    #[test]
    fn arena_is_written_as_an_ascending_array() {
        let mut p = Page::new(PageId::new(1), String::new(), size());
        for id in [10, 2, 7] {
            p.nodes.insert(NodeId::new(id), node(id));
        }
        let v = serde_json::to_value(&p).unwrap();
        let written: Vec<u64> = v["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .map(|n| n["base"]["id"].as_u64().unwrap())
            .collect();
        assert_eq!(written, [2, 7, 10]);
        let back: Page = serde_json::from_value(v).unwrap();
        assert_eq!(back, p);
    }

    #[test]
    fn unordered_nodes_load_and_duplicates_fail() {
        let nodes = format!(
            r#","roots":[7,2],"nodes":[{},{}]"#,
            node_json(7),
            node_json(2)
        );
        let p: Page = serde_json::from_str(&page_json(&nodes)).unwrap();
        assert_eq!(ids(&p), [2, 7]);
        assert_eq!(p.roots, [NodeId::new(7), NodeId::new(2)]);
        let dup = format!(r#","nodes":[{},{}]"#, node_json(5), node_json(5));
        let err = serde_json::from_str::<Page>(&page_json(&dup)).unwrap_err();
        assert!(err.to_string().contains("duplicate node id 5"), "{err}");
    }

    #[test]
    fn page_defaults_required_fields_and_unknown_fields() {
        let p: Page = serde_json::from_str(&page_json(r#","extra":{}"#)).unwrap();
        assert_eq!(p, Page::new(PageId::new(1), String::new(), p.size));
        assert!(serde_json::from_str::<Page>(r#"{"id":1}"#).is_err());
        assert!(serde_json::from_str::<Page>(&page_json("").replace(r#""id":1,"#, "")).is_err());
        assert!(serde_json::from_str::<Page>(&page_json(r#","nodes":{}"#)).is_err());
        assert!(serde_json::from_str::<Page>(&page_json(r#","locked":"yes""#)).is_err());
    }
}
