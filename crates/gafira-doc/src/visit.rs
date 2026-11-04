//! The visitor contract over the node kinds that exist (02 §4.1).
//!
//! The domain owns the dispatch and nothing else: an operation over a document
//! is a visitor written in the crate that needs it, and the domain does not
//! change (ARQ-20). The trait covers the three kinds `NodeKind` has today, and
//! every `visit_*` method has a default that delegates to `visit_node`, so a
//! kind added later is not a breaking change for the implementors (ARQ-134).
//!
//! The walk visits every reachable node once, the siblings in the saved order
//! (back to front), in the two orders of ARQ-136. The page is borrowed, never
//! copied: [`PageView`] and [`MeasureCx`] hold a `&Page`, and the walk
//! answers with owned values (ARQ-135).

use std::collections::BTreeSet;

use gafira_geom::{Point, Rect};

use crate::contract::ParamSchema;
use crate::{ElementBase, ExtensionNode, GroupNode, Node, NodeId, NodeKind, Page, ShapeNode};

/// The order a walk visits the nodes in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WalkOrder {
    /// The parent comes before its children.
    PreOrder,
    /// The children come before their parent.
    PostOrder,
}

/// A borrowed view of a page and its arena.
///
/// The view owns nothing: it borrows the page, and through it the arena, for
/// as long as the borrow lives (ARQ-135).
pub struct PageView<'a> {
    page: &'a Page,
}

impl<'a> PageView<'a> {
    /// Borrows the page and, through it, its arena.
    #[must_use]
    pub fn new(page: &'a Page) -> Self {
        Self { page }
    }

    /// The borrowed page.
    #[must_use]
    pub fn page(&self) -> &'a Page {
        self.page
    }

    /// The node with this id in the page's arena, if any.
    #[must_use]
    pub fn node(&self, id: NodeId) -> Option<&'a Node> {
        self.page.node(id)
    }
}

/// The context a node is measured in: the page, borrowed, and the target's DPI.
///
/// The context carries no time: the instant a measurement is taken at is a
/// separate argument, because the domain owns no clock and no time unit
/// (ARQ-136).
pub struct MeasureCx<'a> {
    page: &'a Page,
    dpi: f32,
}

impl<'a> MeasureCx<'a> {
    /// A context over the page, for a target at `dpi`.
    ///
    /// The DPI is not checked: a non-finite or non-positive one is
    /// representable, and the measurement answers with the degenerate result
    /// (ARQ-136).
    #[must_use]
    pub fn new(page: &'a Page, dpi: f32) -> Self {
        Self { page, dpi }
    }

    /// The page being measured.
    #[must_use]
    pub fn page(&self) -> &'a Page {
        self.page
    }

    /// The target's resolution, in pixels per inch.
    #[must_use]
    pub fn dpi(&self) -> f32 {
        self.dpi
    }

    /// True when the DPI can measure: finite and greater than zero.
    #[must_use]
    pub fn is_measurable(&self) -> bool {
        self.dpi.is_finite() && self.dpi > 0.0
    }
}

/// What a node answers about itself.
///
/// The instant is a type parameter because the domain names no time type: the
/// caller passes the instant it measures at, and the domain never reads a
/// clock (ARQ-136). The layer table of CI (V5) gives `gafira-doc` no time
/// crate, so the instant belongs to the caller.
pub trait NodeBehavior<T = ()> {
    /// The element header of the node.
    fn base(&self) -> &ElementBase;

    /// The node's own box, in page coordinates, at the instant `t`.
    ///
    /// A group answers with its own placed box: the union of its children is
    /// the layout engine's answer (D-28, ADR 0009).
    fn local_bounds(&self, cx: &MeasureCx, t: T) -> Rect;

    /// Whether the point `p`, in page coordinates, is inside the node's box.
    fn hit_test(&self, cx: &MeasureCx, p: Point, t: T) -> bool;
}

impl<T> NodeBehavior<T> for Node {
    fn base(&self) -> &ElementBase {
        &self.base
    }

    fn local_bounds(&self, cx: &MeasureCx, _t: T) -> Rect {
        if !cx.is_measurable() {
            return Rect::ZERO;
        }
        let p = &self.base.placement;
        p.to_transform()
            .transform_rect_bbox(Rect::new(0.0, 0.0, p.w, p.h))
    }

    fn hit_test(&self, cx: &MeasureCx, p: Point, t: T) -> bool {
        if !cx.is_measurable() {
            return false;
        }
        self.local_bounds(cx, t).contains(p)
    }
}

/// What an extension node's owner provides: the box it occupies and the
/// schema the inspector edits.
///
/// The domain declares no render operation and knows no handler type: the
/// render engine owns the handler and the composition root binds it, so an
/// extension is added without touching this crate (ARQ-137, ARQ-20).
pub trait NodeExtension<T = ()> {
    /// The extension's own box, in page coordinates, at the instant `t`.
    fn bounds(&self, cx: &MeasureCx, t: T) -> Rect;

    /// The schema the inspector edits.
    fn inspector_schema(&self) -> &ParamSchema;
}

/// The visitor over the node kinds that exist.
///
/// [`visit_node`](Self::visit_node) is the only method an implementor must
/// write: the kind-specific methods have a default that delegates to it
/// (ARQ-134).
pub trait NodeVisitor {
    /// What the walk collects.
    type Out;

    /// Receives every node that no kind-specific method claims.
    fn visit_node(&mut self, n: &Node) -> Self::Out;

    /// Receives a group.
    fn visit_group(&mut self, n: &Node, _g: &GroupNode) -> Self::Out {
        self.visit_node(n)
    }

    /// Receives a shape.
    fn visit_shape(&mut self, n: &Node, _s: &ShapeNode) -> Self::Out {
        self.visit_node(n)
    }

    /// Receives an extension node.
    fn visit_extension(&mut self, n: &Node, _e: &ExtensionNode) -> Self::Out {
        self.visit_node(n)
    }
}

/// Walks the page and collects what the visitor answers.
///
/// The siblings are visited in the saved order, from back to front, and every
/// reachable node is visited exactly once: a node the walk already reached is
/// not visited again, so a graph with a cycle ends instead of looping. A node
/// that does not exist in the arena is skipped. The walk does not filter by
/// `visible` or `locked`: it reaches every node, and the visitor is the one
/// that cuts (ARQ-135, ARQ-136).
pub fn walk<V: NodeVisitor>(page: &PageView, v: &mut V, order: WalkOrder) -> Vec<V::Out> {
    let mut out = Vec::new();
    let mut seen: BTreeSet<NodeId> = BTreeSet::new();
    // (node, index of the next child to visit): an explicit stack, so a deep
    // tree cannot overflow the call stack.
    let mut stack: Vec<(&Node, usize)> = Vec::new();

    for &root in page.page().roots.iter() {
        let Some(node) = page.node(root) else {
            continue;
        };
        if !seen.insert(node.base.id) {
            continue;
        }
        if order == WalkOrder::PreOrder {
            out.push(dispatch(v, node));
        }
        stack.push((node, 0));
        loop {
            let Some((node, at)) = stack.last().copied() else {
                break;
            };
            if at < node.children.len() {
                let child = node.children[at];
                if let Some(top) = stack.last_mut() {
                    top.1 = at + 1;
                }
                let Some(child) = page.node(child) else {
                    continue;
                };
                if !seen.insert(child.base.id) {
                    continue;
                }
                if order == WalkOrder::PreOrder {
                    out.push(dispatch(v, child));
                }
                stack.push((child, 0));
            } else {
                stack.pop();
                if order == WalkOrder::PostOrder {
                    out.push(dispatch(v, node));
                }
            }
        }
    }
    out
}

/// Sends a node to the one `visit_*` method that matches its kind.
///
/// The match has no catch-all arm on purpose: a new variant of `NodeKind`
/// breaks this function and forces the new kind to be dispatched here, not in
/// the implementors (ARQ-134).
fn dispatch<V: NodeVisitor>(v: &mut V, node: &Node) -> V::Out {
    match &node.kind {
        NodeKind::Group(g) => v.visit_group(node, g),
        NodeKind::Shape(s) => v.visit_shape(node, s),
        NodeKind::Extension(e) => v.visit_extension(node, e),
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use gafira_geom::{Length, Unit};

    use super::*;
    use crate::{PageId, PageSize, Placement, ShapeGeometry};

    fn size() -> PageSize {
        PageSize {
            width: Length::new(100.0, Unit::Px),
            height: Length::new(100.0, Unit::Px),
        }
    }

    fn group() -> NodeKind {
        NodeKind::Group(GroupNode {})
    }

    fn shape() -> NodeKind {
        NodeKind::Shape(ShapeNode::new(ShapeGeometry::Rect))
    }

    fn extension() -> NodeKind {
        NodeKind::Extension(ExtensionNode {
            type_id: "com.example.sticker".to_string(),
            schema_version: 1,
            payload: serde_json::Value::Null,
        })
    }

    /// A page with one node per entry of `kinds`, the parent edges of `edges`
    /// and the roots of `roots`.
    fn page(kinds: &[(u64, NodeKind)], edges: &[(u64, &[u64])], roots: &[u64]) -> Page {
        let mut p = Page::new(PageId::new(1), "P".to_string(), size());
        for (id, kind) in kinds {
            let base = ElementBase::new(NodeId::new(*id), Placement::new(0.0, 0.0, 10.0, 10.0));
            p.nodes
                .insert(NodeId::new(*id), Arc::new(Node::new(base, kind.clone())));
        }
        for &(parent, children) in edges {
            let node = Arc::make_mut(p.nodes.get_mut(&NodeId::new(parent)).unwrap());
            node.children = children.iter().map(|c| NodeId::new(*c)).collect();
        }
        p.roots = roots.iter().map(|r| NodeId::new(*r)).collect();
        p
    }

    /// A visitor that records what it receives.
    #[derive(Default)]
    struct Recorder {
        visited: Vec<NodeId>,
        groups: usize,
        shapes: usize,
        extensions: usize,
    }

    impl NodeVisitor for Recorder {
        type Out = NodeId;

        fn visit_node(&mut self, n: &Node) -> NodeId {
            self.visited.push(n.base.id);
            n.base.id
        }

        fn visit_group(&mut self, n: &Node, _g: &GroupNode) -> NodeId {
            self.groups += 1;
            self.visit_node(n)
        }

        fn visit_shape(&mut self, n: &Node, _s: &ShapeNode) -> NodeId {
            self.shapes += 1;
            self.visit_node(n)
        }

        fn visit_extension(&mut self, n: &Node, _e: &ExtensionNode) -> NodeId {
            self.extensions += 1;
            self.visit_node(n)
        }
    }

    /// A visitor that writes no kind-specific method (ARQ-134).
    struct Plain;

    impl NodeVisitor for Plain {
        type Out = NodeId;

        fn visit_node(&mut self, n: &Node) -> NodeId {
            n.base.id
        }
    }

    fn ids(list: &[NodeId]) -> Vec<u64> {
        list.iter().map(|id| id.get()).collect()
    }

    // ARQ-134: the walk calls the visitor once per node, and never twice for
    // the same node.
    #[test]
    fn arq_134_every_node_is_visited_once() {
        let p = page(
            &[(1, group()), (2, shape()), (3, extension())],
            &[],
            &[1, 2, 3],
        );
        let view = PageView::new(&p);
        let mut recorder = Recorder::default();
        let visited = walk(&view, &mut recorder, WalkOrder::PreOrder);
        assert_eq!(ids(&visited), [1, 2, 3]);
        assert_eq!(ids(&recorder.visited), [1, 2, 3]);
        assert_eq!(
            (recorder.groups, recorder.shapes, recorder.extensions),
            (1, 1, 1)
        );
    }

    // ARQ-134: a visitor that writes only `visit_node` compiles and receives
    // one call per node.
    #[test]
    fn arq_134_a_visitor_with_only_visit_node_receives_every_node() {
        let p = page(
            &[(1, group()), (2, shape()), (3, extension())],
            &[],
            &[1, 2, 3],
        );
        let view = PageView::new(&p);
        let mut plain = Plain;
        assert_eq!(
            ids(&walk(&view, &mut plain, WalkOrder::PreOrder)),
            [1, 2, 3]
        );
        assert_eq!(
            ids(&walk(&view, &mut plain, WalkOrder::PostOrder)),
            [1, 2, 3]
        );
    }

    // ARQ-135: the view borrows the page and its arena: it copies neither, and
    // the arena the walk reads is the page's own.
    #[test]
    fn arq_135_the_view_borrows_the_page_and_its_arena() {
        let p = page(&[(1, group()), (2, shape())], &[(1, &[2])], &[1]);
        let before: Vec<usize> = p.nodes.values().map(Arc::strong_count).collect();
        let view = PageView::new(&p);
        let mut plain = Plain;
        let visited = walk(&view, &mut plain, WalkOrder::PreOrder);
        assert_eq!(ids(&visited), [1, 2]);
        let after: Vec<usize> = p.nodes.values().map(Arc::strong_count).collect();
        assert_eq!(after, before, "the walk cloned or copied the arena");
        let node = view.node(NodeId::new(2)).unwrap();
        assert!(std::ptr::eq(node, p.node(NodeId::new(2)).unwrap()));
    }

    // ARQ-135: a page with no roots walks to an empty vector.
    #[test]
    fn arq_135_a_page_without_roots_walks_to_an_empty_vector() {
        let p = page(&[], &[], &[]);
        let view = PageView::new(&p);
        let mut plain = Plain;
        assert!(walk(&view, &mut plain, WalkOrder::PreOrder).is_empty());
        assert!(walk(&view, &mut plain, WalkOrder::PostOrder).is_empty());
    }

    // ARQ-135: the walk answers with owned values: the result outlives the
    // page it came from.
    #[test]
    fn arq_135_the_walk_answers_with_owned_values() {
        let visited = {
            let p = page(&[(1, group()), (2, shape())], &[(1, &[2])], &[1]);
            let view = PageView::new(&p);
            let mut plain = Plain;
            walk(&view, &mut plain, WalkOrder::PreOrder)
        };
        assert_eq!(ids(&visited), [1, 2]);
    }

    // ARQ-136: the parent comes before its children in `PreOrder`, and the
    // siblings are visited in the saved order, from back to front.
    #[test]
    fn arq_136_pre_order_visits_the_parent_before_its_children() {
        let p = page(
            &[(1, group()), (2, group()), (3, shape()), (4, extension())],
            &[(1, &[2, 3]), (2, &[4])],
            &[1],
        );
        let view = PageView::new(&p);
        let mut plain = Plain;
        assert_eq!(
            ids(&walk(&view, &mut plain, WalkOrder::PreOrder)),
            [1, 2, 4, 3]
        );
    }

    // ARQ-136: the children come before their parent in `PostOrder`, and the
    // siblings keep the saved order.
    #[test]
    fn arq_136_post_order_visits_the_parent_after_its_children() {
        let p = page(
            &[(1, group()), (2, group()), (3, shape()), (4, extension())],
            &[(1, &[2, 3]), (2, &[4])],
            &[1],
        );
        let view = PageView::new(&p);
        let mut plain = Plain;
        assert_eq!(
            ids(&walk(&view, &mut plain, WalkOrder::PostOrder)),
            [4, 2, 3, 1]
        );
    }

    // ARQ-136 and ARQ-13: a graph with a cycle ends the walk, and every node
    // is visited once.
    #[test]
    fn arq_136_a_cycle_ends_the_walk_and_visits_each_node_once() {
        // A group whose child is itself.
        let own = page(&[(1, group())], &[(1, &[1])], &[1]);
        let view = PageView::new(&own);
        let mut plain = Plain;
        assert_eq!(ids(&walk(&view, &mut plain, WalkOrder::PreOrder)), [1]);
        assert_eq!(ids(&walk(&view, &mut plain, WalkOrder::PostOrder)), [1]);

        // Two groups that point at each other.
        let mutual = page(&[(1, group()), (2, group())], &[(1, &[2]), (2, &[1])], &[1]);
        let view = PageView::new(&mutual);
        let mut plain = Plain;
        assert_eq!(ids(&walk(&view, &mut plain, WalkOrder::PreOrder)), [1, 2]);
        assert_eq!(ids(&walk(&view, &mut plain, WalkOrder::PostOrder)), [2, 1]);
    }

    // ARQ-136 and ARQ-13: a child that does not exist is skipped, the rest of
    // the children is walked, and the walk does not panic.
    #[test]
    fn arq_136_a_missing_child_is_skipped_and_the_rest_is_walked() {
        let p = page(
            &[(1, group()), (2, shape()), (3, extension())],
            &[(1, &[2, 99, 3])],
            &[1],
        );
        let view = PageView::new(&p);
        let mut plain = Plain;
        assert_eq!(
            ids(&walk(&view, &mut plain, WalkOrder::PreOrder)),
            [1, 2, 3]
        );
        assert_eq!(
            ids(&walk(&view, &mut plain, WalkOrder::PostOrder)),
            [2, 3, 1]
        );
    }

    // ARQ-136: the walk does not filter by `visible` or `locked`: it reaches
    // every node, and the visitor is the one that cuts.
    #[test]
    fn arq_136_the_walk_visits_hidden_and_locked_nodes() {
        let mut p = page(&[(1, group()), (2, shape())], &[(1, &[2])], &[1]);
        let node = Arc::make_mut(p.nodes.get_mut(&NodeId::new(2)).unwrap());
        node.base.visible = false;
        node.base.locked = true;
        let view = PageView::new(&p);
        let mut plain = Plain;
        assert_eq!(ids(&walk(&view, &mut plain, WalkOrder::PreOrder)), [1, 2]);
    }

    // ARQ-136: a DPI that is not finite, or not positive, makes the
    // measurement degenerate: a box of zero width and height, and no hit.
    #[test]
    fn arq_136_an_unusable_dpi_degenerates_the_measurement() {
        let p = page(&[(1, shape())], &[], &[1]);
        let node = p.node(NodeId::new(1)).unwrap();
        for dpi in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, 0.0, -1.0] {
            let cx = MeasureCx::new(&p, dpi);
            assert!(!cx.is_measurable(), "dpi {dpi}");
            let bounds = node.local_bounds(&cx, ());
            assert_eq!((bounds.width(), bounds.height()), (0.0, 0.0), "dpi {dpi}");
            assert!(!node.hit_test(&cx, Point::new(1.0, 1.0), ()), "dpi {dpi}");
        }
    }

    // ARQ-136: with a usable DPI the node's box is measured in page
    // coordinates, and the hit test takes the point in page coordinates.
    #[test]
    fn arq_136_a_usable_dpi_measures_the_node_box_in_page_coordinates() {
        let mut p = page(&[(1, shape())], &[], &[1]);
        {
            let node = Arc::make_mut(p.nodes.get_mut(&NodeId::new(1)).unwrap());
            node.base.placement = Placement::new(10.0, 20.0, 30.0, 40.0);
        }
        let node = p.node(NodeId::new(1)).unwrap();
        let cx = MeasureCx::new(&p, 96.0);
        assert!(cx.is_measurable());
        assert_eq!(
            node.local_bounds(&cx, ()),
            Rect::new(10.0, 20.0, 40.0, 60.0)
        );
        assert!(node.hit_test(&cx, Point::new(10.0, 20.0), ()));
        assert!(node.hit_test(&cx, Point::new(39.0, 59.0), ()));
        assert!(!node.hit_test(&cx, Point::new(40.0, 60.0), ()));
        assert!(!node.hit_test(&cx, Point::new(9.0, 20.0), ()));
    }
}
