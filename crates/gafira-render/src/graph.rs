//! The render graph of a page and the builder that walks it (02 §6.1).
//!
//! The graph holds the nodes that are drawn and the operations they emit, in
//! paint order. The builder is the visitor of the walk of `gafira-doc`
//! (02 §4.1): the walk reaches every node and the builder cuts the ones that
//! draw nothing.
//!
//! The graph holds no pixel in this cut: no effect, no mask and no image
//! source reaches it yet, so ARQ-117 has nothing to check here. The first
//! buffer arrives with the planner and the tile cache.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::sync::Arc;

use gafira_color::Color;
use gafira_doc::contract::{CancelToken, Cancelled, Progress, checkpoint};
use gafira_doc::visit::{NodeVisitor, PageView, WalkOrder, walk};
use gafira_doc::{
    BlendMode, ElementBase, ExtensionNode, GroupNode, Node, NodeId, Page, Placement, ShapeGeometry,
    ShapeNode,
};
use gafira_geom::Affine;
use gafira_ports::render::{ContentKey, RenderOp, RenderQuality};
use gafira_time::Flicks;
use kurbo::{BezPath, Ellipse, Rect, Shape};

use crate::key::{KeyParam, NodeType, content_key};
use crate::pool::WorkerPool;

/// The identifier of a node of the render graph.
///
/// The numbers are dense and a node's inputs always have a smaller number, so
/// a graph can be read from the leaves up.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GraphNodeId(pub u32);

/// What a node of the render graph is (02 §6.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GraphNodeKind {
    /// A source of pixels.
    Source(SourceKind),
    /// A container that composites its children.
    Composite,
}

/// Where the pixels of a source come from (02 §6.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceKind {
    /// An outline, filled.
    Vector,
    /// A run of text.
    Text,
    /// A still image.
    Image,
    /// A frame of a video.
    VideoFrame,
}

/// One node of the render graph (02 §6.1).
pub struct GraphNode {
    id: GraphNodeId,
    kind: GraphNodeKind,
    origin: NodeId,
    key: ContentKey,
    inputs: Vec<GraphNodeId>,
    transform: Affine,
    blend: BlendMode,
    opacity: f32,
}

impl GraphNode {
    /// The identifier of this node.
    #[must_use]
    pub fn id(&self) -> GraphNodeId {
        self.id
    }

    /// What this node is.
    #[must_use]
    pub fn kind(&self) -> GraphNodeKind {
        self.kind
    }

    /// The document node this node came from.
    #[must_use]
    pub fn origin(&self) -> NodeId {
        self.origin
    }

    /// The content key of this node (02 §6.1).
    #[must_use]
    pub fn key(&self) -> ContentKey {
        self.key
    }

    /// The nodes this one consumes.
    #[must_use]
    pub fn inputs(&self) -> &[GraphNodeId] {
        &self.inputs
    }

    /// The transform of this node in page space: the placement of the node.
    ///
    /// D-35: the placement of every node is in page space, also the one of
    /// the children of a group, so the placement of a group does not move its
    /// children.
    #[must_use]
    pub fn transform(&self) -> Affine {
        self.transform
    }

    /// The blend mode of this node.
    #[must_use]
    pub fn blend(&self) -> BlendMode {
        self.blend
    }

    /// The opacity of this node.
    #[must_use]
    pub fn opacity(&self) -> f32 {
        self.opacity
    }
}

/// The parameters that separate a preview graph from an export graph (02 §6.2).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GraphParams {
    /// Preview or export.
    pub quality: RenderQuality,
    /// The level of detail the graph resolves.
    pub lod: u8,
    /// The instant the graph is sampled at.
    ///
    /// The parameters that depend on time are sampled at this instant; the ones
    /// that do not depend on it hold the same value at every instant, so the
    /// key of a node does not change with it (02 §6.1).
    pub t: Flicks,
}

/// Why the graph could not be built (ARQ-110, ARQ-139).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GraphError {
    /// A number of the input is not finite; the field names it (ARQ-110).
    NonFinite {
        /// The field that holds the number.
        field: &'static str,
    },
    /// The build was cancelled (ARQ-139).
    Cancelled(Cancelled),
}

impl fmt::Display for GraphError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFinite { field } => write!(f, "the field `{field}` is not finite"),
            Self::Cancelled(cancelled) => write!(f, "{cancelled}"),
        }
    }
}

impl std::error::Error for GraphError {}

impl From<Cancelled> for GraphError {
    fn from(cancelled: Cancelled) -> Self {
        Self::Cancelled(cancelled)
    }
}

/// The render graph of a page (02 §6.1).
pub struct RenderGraph {
    nodes: Vec<GraphNode>,
    ops: Vec<RenderOp>,
    root_op_ranges: Vec<std::ops::Range<usize>>,
    visited: Vec<NodeId>,
    roots: Vec<GraphNodeId>,
    params: GraphParams,
}

impl RenderGraph {
    /// The nodes of the graph, in the order they were finished: a node's
    /// inputs always come before it.
    #[must_use]
    pub fn nodes(&self) -> &[GraphNode] {
        &self.nodes
    }

    /// The node with this identifier.
    #[must_use]
    pub fn node(&self, id: GraphNodeId) -> Option<&GraphNode> {
        self.nodes.get(id.0 as usize)
    }

    /// The operations of the graph, in paint order.
    #[must_use]
    pub fn ops(&self) -> &[RenderOp] {
        &self.ops
    }

    /// The operations of the subtree of every root, in the order of the
    /// roots.
    ///
    /// The planner gives each tile the operations of the root it draws
    /// (02 §6.1).
    #[must_use]
    pub fn root_ops(&self) -> impl ExactSizeIterator<Item = &[RenderOp]> {
        self.root_op_ranges
            .iter()
            .map(|range| &self.ops[range.clone()])
    }

    /// The document nodes the walk reached, in the order it reached them.
    #[must_use]
    pub fn visited(&self) -> &[NodeId] {
        &self.visited
    }

    /// The roots of the graph, back to front.
    #[must_use]
    pub fn roots(&self) -> &[GraphNodeId] {
        &self.roots
    }

    /// The parameters of the graph.
    #[must_use]
    pub fn params(&self) -> GraphParams {
        self.params
    }

    /// The content key of the node that came from the document node `origin`.
    #[must_use]
    pub fn key_of(&self, origin: NodeId) -> Option<ContentKey> {
        self.nodes
            .iter()
            .find(|node| node.origin == origin)
            .map(|node| node.key)
    }
}

/// What the visitor collected about a node.
struct Record {
    base: ElementBase,
    kind: RecordKind,
}

/// What a node contributes to the graph.
enum RecordKind {
    /// A group: it arranges its children.
    Group,
    /// A shape: it fills its own outline.
    Shape(ShapeNode),
}

/// Builds the render graph of a page by walking it (02 §6.1).
///
/// The builder is the visitor of the walk of `gafira-doc` (02 §4.1). The walk
/// reaches every node and the builder cuts the ones that draw nothing: an
/// invisible node, a node the walk did not reach and, for now, an extension
/// (the seam of ARQ-137 is open).
///
/// The pool of workers belongs to the caller (ARQ-138): the builder hands it
/// the roots of the page and takes the result back in the order of the roots,
/// so the graph never depends on the order the roots are built in (ARQ-140).
pub struct RenderGraphBuilder<'a, P: WorkerPool> {
    page: &'a Page,
    params: GraphParams,
    pool: P,
    records: BTreeMap<NodeId, Record>,
}

impl<'a, P: WorkerPool> RenderGraphBuilder<'a, P> {
    /// A builder over the page of the view.
    pub fn new(view: &PageView<'a>, params: GraphParams, pool: P) -> Self {
        Self {
            page: view.page(),
            params,
            pool,
            records: BTreeMap::new(),
        }
    }

    /// The parameters of the graph.
    #[must_use]
    pub fn params(&self) -> GraphParams {
        self.params
    }

    /// Walks the page and builds the graph.
    ///
    /// The build is a long operation: it checks the cancel token and reports
    /// the progress as it goes (ARQ-139). The fraction counts the nodes that
    /// joined the graph over the nodes planned.
    ///
    /// # Errors
    /// [`GraphError::NonFinite`] when a number of the page is not finite
    /// (ARQ-110), [`GraphError::Cancelled`] when the token is cancelled
    /// (ARQ-139).
    pub fn build(
        mut self,
        order: WalkOrder,
        cancel: &CancelToken,
        progress: &dyn Progress,
    ) -> Result<RenderGraph, GraphError> {
        let visited = walk(&PageView::new(self.page), &mut self, order);
        let roots = self.page.roots.clone();
        let built = self
            .pool
            .map(roots.len(), |at| self.subtree(roots[at], cancel));
        let mut subgraphs: Vec<SubGraph> = Vec::with_capacity(roots.len());
        for sub in built {
            subgraphs.push(sub?);
        }
        let planned = subgraphs.iter().map(|sub| sub.nodes.len()).sum::<usize>() + 1;

        let mut nodes: Vec<GraphNode> = Vec::new();
        let mut ops: Vec<RenderOp> = Vec::new();
        let mut root_op_ranges: Vec<std::ops::Range<usize>> = Vec::new();
        let mut graph_roots: Vec<GraphNodeId> = Vec::new();
        let mut done = 0usize;
        for sub in subgraphs {
            let base = nodes.len() as u32;
            let root = sub.nodes.last().map(|node| node.id);
            for mut node in sub.nodes {
                for input in &mut node.inputs {
                    *input = GraphNodeId(base + input.0);
                }
                node.id = GraphNodeId(base + node.id.0);
                nodes.push(node);
                done += 1;
                checkpoint(cancel, progress, done as f32 / planned as f32)?;
            }
            let ops_start = ops.len();
            ops.extend(sub.ops);
            if let Some(root) = root {
                graph_roots.push(GraphNodeId(base + root.0));
                root_op_ranges.push(ops_start..ops.len());
            }
        }
        checkpoint(cancel, progress, 1.0)?;
        Ok(RenderGraph {
            nodes,
            ops,
            root_op_ranges,
            visited,
            roots: graph_roots,
            params: self.params,
        })
    }

    /// The nodes and the operations of the subtree of `root`.
    fn subtree(&self, root: NodeId, cancel: &CancelToken) -> Result<SubGraph, GraphError> {
        let mut sub = SubGraph::default();
        let mut seen: BTreeSet<NodeId> = BTreeSet::new();
        // The open nodes: the ones whose children are not all visited yet.
        let mut stack: Vec<Frame> = Vec::new();
        // The node to enter next.
        let mut next = Some(root);

        loop {
            if let Some(id) = next.take() {
                let Some(record) = self.records.get(&id) else {
                    continue;
                };
                if !record.base.visible || !seen.insert(id) {
                    continue;
                }
                cancel.check()?;
                // D-35: the placement of every node is in page space, also the
                // one of the children of a group: the placement of a group does
                // not move its children.
                let transform = record.base.placement.to_transform();
                check_transform(&transform)?;
                check_opacity(record.base.opacity)?;
                let mark = sub.ops.len();
                sub.ops.push(RenderOp::PushLayer);
                match record.kind {
                    RecordKind::Shape(shape) => {
                        let Some(color) = shape.fill else {
                            // A shape without a fill draws nothing.
                            sub.ops.truncate(mark);
                            continue;
                        };
                        check_color(&color)?;
                        let path = shape_path(shape.geometry, &record.base.placement);
                        let key = self.node_key(
                            NodeType::Shape,
                            &[],
                            &record.base,
                            transform,
                            Some((shape.geometry, color)),
                        );
                        let node = GraphNode {
                            id: GraphNodeId(sub.nodes.len() as u32),
                            kind: GraphNodeKind::Source(SourceKind::Vector),
                            origin: id,
                            key,
                            inputs: Vec::new(),
                            transform,
                            blend: record.base.blend,
                            opacity: record.base.opacity,
                        };
                        let node_id = node.id;
                        sub.nodes.push(node);
                        if let Some(frame) = stack.last_mut() {
                            frame.inputs.push(node_id);
                        }
                        sub.ops.push(RenderOp::FillPath {
                            path: Arc::new(path),
                            color,
                            transform,
                        });
                        sub.ops.push(RenderOp::PopLayer {
                            blend: record.base.blend,
                            opacity: record.base.opacity,
                        });
                    }
                    RecordKind::Group => {
                        let children = self
                            .page
                            .node(id)
                            .map(|node| node.children.clone())
                            .unwrap_or_default();
                        stack.push(Frame {
                            id,
                            children,
                            at: 0,
                            inputs: Vec::new(),
                        });
                    }
                }
            }

            // Leave the open nodes whose children are done, from the top.
            let Some(frame) = stack.last_mut() else {
                break;
            };
            if frame.at < frame.children.len() {
                let child = frame.children[frame.at];
                frame.at += 1;
                next = Some(child);
                continue;
            }
            let frame = stack.pop().expect("the frame is there");
            let Some(record) = self.records.get(&frame.id) else {
                continue;
            };
            let inputs: Vec<ContentKey> = frame
                .inputs
                .iter()
                .map(|input| sub.nodes[input.0 as usize].key)
                .collect();
            let transform = record.base.placement.to_transform();
            let key = self.node_key(NodeType::Group, &inputs, &record.base, transform, None);
            let node = GraphNode {
                id: GraphNodeId(sub.nodes.len() as u32),
                kind: GraphNodeKind::Composite,
                origin: frame.id,
                key,
                inputs: frame.inputs,
                transform,
                blend: record.base.blend,
                opacity: record.base.opacity,
            };
            let node_id = node.id;
            sub.nodes.push(node);
            if let Some(parent) = stack.last_mut() {
                parent.inputs.push(node_id);
            }
            sub.ops.push(RenderOp::PopLayer {
                blend: record.base.blend,
                opacity: record.base.opacity,
            });
        }
        Ok(sub)
    }

    /// The content key of a node (02 §6.1).
    fn node_key(
        &self,
        node_type: NodeType,
        inputs: &[ContentKey],
        base: &ElementBase,
        transform: Affine,
        shape: Option<(ShapeGeometry, Color)>,
    ) -> ContentKey {
        let mut params = vec![
            KeyParam::Quality(self.params.quality),
            KeyParam::Transform(transform.as_coeffs()),
            KeyParam::Opacity(base.opacity),
            KeyParam::Blend(base.blend),
        ];
        if let Some((geometry, color)) = shape {
            params.insert(1, KeyParam::Geometry(geometry));
            params.insert(2, KeyParam::Fill(Some(color)));
        }
        content_key(node_type, self.params.lod, inputs, &params)
    }
}

impl<P: WorkerPool> NodeVisitor for RenderGraphBuilder<'_, P> {
    type Out = NodeId;

    fn visit_node(&mut self, node: &Node) -> NodeId {
        node.base.id
    }

    fn visit_group(&mut self, node: &Node, _group: &GroupNode) -> NodeId {
        self.records.insert(
            node.base.id,
            Record {
                base: node.base.clone(),
                kind: RecordKind::Group,
            },
        );
        node.base.id
    }

    fn visit_shape(&mut self, node: &Node, shape: &ShapeNode) -> NodeId {
        self.records.insert(
            node.base.id,
            Record {
                base: node.base.clone(),
                kind: RecordKind::Shape(*shape),
            },
        );
        node.base.id
    }

    fn visit_extension(&mut self, node: &Node, _extension: &ExtensionNode) -> NodeId {
        // The seam of ARQ-137 is open: an extension draws nothing in this cut.
        node.base.id
    }
}

/// An open node of the tree walk.
struct Frame {
    id: NodeId,
    children: Vec<NodeId>,
    at: usize,
    inputs: Vec<GraphNodeId>,
}

/// The nodes and the operations of the subtree of one root.
#[derive(Default)]
struct SubGraph {
    nodes: Vec<GraphNode>,
    ops: Vec<RenderOp>,
}

/// The outline of a shape, in its own box.
fn shape_path(geometry: ShapeGeometry, placement: &Placement) -> BezPath {
    let rect = Rect::new(0.0, 0.0, placement.w, placement.h).abs();
    match geometry {
        ShapeGeometry::Rect => rect.to_path(0.25),
        ShapeGeometry::Ellipse => Ellipse::from_rect(rect).to_path(0.25),
    }
}

/// The check of ARQ-110: a number that is not finite is an error that names
/// the field.
fn check_transform(transform: &Affine) -> Result<(), GraphError> {
    if transform.as_coeffs().iter().all(|c| c.is_finite()) {
        Ok(())
    } else {
        Err(GraphError::NonFinite { field: "placement" })
    }
}

/// The check of ARQ-110 for the opacity of a node.
fn check_opacity(opacity: f32) -> Result<(), GraphError> {
    if opacity.is_finite() {
        Ok(())
    } else {
        Err(GraphError::NonFinite { field: "opacity" })
    }
}

/// The check of ARQ-110 for the fill of a shape.
fn check_color(color: &Color) -> Result<(), GraphError> {
    let channels = match color {
        Color::Srgb(c) => [c.r, c.g, c.b, c.a],
        Color::LinearSrgb(c) => [c.r, c.g, c.b, c.a],
        Color::DisplayP3(c) => [c.r, c.g, c.b, c.a],
    };
    if channels.iter().all(|c| c.is_finite()) {
        Ok(())
    } else {
        Err(GraphError::NonFinite { field: "fill" })
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use gafira_color::Rgba;
    use gafira_doc::contract::NoProgress;
    use gafira_doc::{NodeKind, PageId, PageSize};
    use gafira_geom::{Length, Unit};
    use gafira_testkit::progress::check_progress_contract;

    use super::*;
    use crate::pool::InlinePool;

    /// A page of 100 by 100 pixels.
    fn new_page() -> Page {
        Page::new(
            PageId::new(1),
            "P".to_string(),
            PageSize {
                width: Length::new(100.0, Unit::Px),
                height: Length::new(100.0, Unit::Px),
            },
        )
    }

    /// Adds a node of the given kind to the page.
    fn add(page: &mut Page, id: u64, kind: NodeKind) {
        let base = ElementBase::new(NodeId::new(id), Placement::new(0.0, 0.0, 10.0, 10.0));
        page.nodes
            .insert(NodeId::new(id), Arc::new(Node::new(base, kind)));
    }

    /// Makes `parent` the parent of `children`.
    fn children_of(page: &mut Page, parent: u64, children: &[u64]) {
        let node = Arc::make_mut(
            page.nodes
                .get_mut(&NodeId::new(parent))
                .expect("the node is there"),
        );
        node.children = children.iter().map(|child| NodeId::new(*child)).collect();
    }

    /// A filled shape.
    fn shape() -> NodeKind {
        NodeKind::Shape(ShapeNode {
            geometry: ShapeGeometry::Rect,
            fill: Some(Color::Srgb(Rgba::new(0.2, 0.4, 0.6, 1.0))),
        })
    }

    /// The parameters of a graph of the given quality.
    fn params(quality: RenderQuality) -> GraphParams {
        GraphParams {
            quality,
            lod: 0,
            t: Flicks::from_ticks(0),
        }
    }

    /// Builds the graph of the page with the given pool.
    fn build_with<P: WorkerPool>(
        page: &Page,
        order: WalkOrder,
        quality: RenderQuality,
        pool: P,
    ) -> Result<RenderGraph, GraphError> {
        let view = PageView::new(page);
        RenderGraphBuilder::new(&view, params(quality), pool).build(
            order,
            &CancelToken::new(),
            &NoProgress,
        )
    }

    /// Builds the graph of the page.
    fn build(page: &Page, order: WalkOrder) -> RenderGraph {
        build_with(page, order, RenderQuality::Preview, InlinePool).expect("the page builds")
    }

    /// What an operation does, without its payload.
    fn op_kind(op: &RenderOp) -> &'static str {
        match op {
            RenderOp::PushLayer => "push",
            RenderOp::PopLayer { .. } => "pop",
            RenderOp::FillPath { .. } => "fill",
            RenderOp::DrawImage { .. } => "image",
            RenderOp::ApplyMask { .. } => "mask",
            RenderOp::ApplyEffect { .. } => "effect",
            RenderOp::ApplyTransition { .. } => "transition",
        }
    }

    /// The kinds of the operations of the graph, in order.
    fn op_kinds(graph: &RenderGraph) -> Vec<&'static str> {
        graph.ops().iter().map(|op| op_kind(op)).collect()
    }

    /// A pool that runs the jobs on `threads` threads (ARQ-140).
    struct ThreadedPool {
        threads: usize,
    }

    impl WorkerPool for ThreadedPool {
        fn map<T: Send>(&self, count: usize, work: impl Fn(usize) -> T + Sync) -> Vec<T> {
            let slots: Vec<Mutex<Option<T>>> = (0..count).map(|_| Mutex::new(None)).collect();
            let next = AtomicUsize::new(0);
            std::thread::scope(|scope| {
                for _ in 0..self.threads {
                    scope.spawn(|| {
                        loop {
                            let at = next.fetch_add(1, Ordering::SeqCst);
                            if at >= count {
                                break;
                            }
                            let value = work(at);
                            *slots[at].lock().expect("the slot is free") = Some(value);
                        }
                    });
                }
            });
            slots
                .into_iter()
                .map(|slot| {
                    slot.into_inner()
                        .expect("the slot is free")
                        .expect("the job ran")
                })
                .collect()
        }
    }

    // ARQ-141: two nodes with the same data give the same key.
    #[test]
    fn arq_141_two_nodes_with_the_same_data_give_the_same_key() {
        let mut page = new_page();
        add(&mut page, 1, shape());
        add(&mut page, 2, shape());
        page.roots = vec![NodeId::new(1), NodeId::new(2)];

        let graph = build(&page, WalkOrder::PreOrder);
        let first = graph
            .key_of(NodeId::new(1))
            .expect("the first shape is in the graph");
        let second = graph
            .key_of(NodeId::new(2))
            .expect("the second shape is in the graph");
        assert_eq!(first, second);
    }

    // ARQ-141: the content key of a node does not depend on the order of the
    // walk.
    #[test]
    fn arq_141_the_keys_do_not_depend_on_the_order_of_the_walk() {
        let mut page = new_page();
        add(&mut page, 1, NodeKind::Group(GroupNode {}));
        add(&mut page, 2, shape());
        children_of(&mut page, 1, &[2]);
        page.roots = vec![NodeId::new(1)];

        let pre = build(&page, WalkOrder::PreOrder);
        let post = build(&page, WalkOrder::PostOrder);

        assert_eq!(pre.key_of(NodeId::new(1)), post.key_of(NodeId::new(1)));
        assert_eq!(pre.key_of(NodeId::new(2)), post.key_of(NodeId::new(2)));
        // The walk reached the nodes in a different order.
        assert_eq!(pre.visited(), &[NodeId::new(1), NodeId::new(2)]);
        assert_eq!(post.visited(), &[NodeId::new(2), NodeId::new(1)]);
    }

    // ARQ-141: the graph holds the operations of the page and none of the
    // extension.
    #[test]
    fn arq_141_the_graph_holds_the_operations_of_the_page_and_none_of_the_extension() {
        let mut page = new_page();
        add(&mut page, 1, NodeKind::Group(GroupNode {}));
        add(&mut page, 2, shape());
        add(
            &mut page,
            3,
            NodeKind::Extension(ExtensionNode {
                type_id: "com.example.sticker".to_string(),
                schema_version: 1,
                payload: Default::default(),
            }),
        );
        children_of(&mut page, 1, &[2, 3]);
        page.roots = vec![NodeId::new(1)];

        let graph = build(&page, WalkOrder::PreOrder);
        assert_eq!(op_kinds(&graph), vec!["push", "push", "fill", "pop", "pop"]);
        assert_eq!(graph.nodes().len(), 2);
        assert!(graph.key_of(NodeId::new(3)).is_none());
    }

    // ARQ-141: the preview and the export graphs hold the same operations and
    // only the quality parameters differ.
    #[test]
    fn arq_141_the_preview_and_the_export_hold_the_same_operations() {
        let mut page = new_page();
        add(&mut page, 1, NodeKind::Group(GroupNode {}));
        add(&mut page, 2, shape());
        children_of(&mut page, 1, &[2]);
        page.roots = vec![NodeId::new(1)];

        let preview = build_with(
            &page,
            WalkOrder::PreOrder,
            RenderQuality::Preview,
            InlinePool,
        )
        .expect("the page builds");
        let export = build_with(
            &page,
            WalkOrder::PreOrder,
            RenderQuality::Export,
            InlinePool,
        )
        .expect("the page builds");

        assert_eq!(op_kinds(&preview), op_kinds(&export));
        assert_ne!(preview.params().quality, export.params().quality);
        assert_eq!(preview.params().lod, export.params().lod);
    }

    // ARQ-141 (D-35): the transform of a node is its own placement: the
    // placement of a group does not move its children.
    #[test]
    fn arq_141_the_transform_is_the_placement_of_the_node() {
        let mut page = new_page();
        add(&mut page, 1, NodeKind::Group(GroupNode {}));
        add(&mut page, 2, shape());
        children_of(&mut page, 1, &[2]);
        page.roots = vec![NodeId::new(1)];
        {
            let group = Arc::make_mut(
                page.nodes
                    .get_mut(&NodeId::new(1))
                    .expect("the node is there"),
            );
            group.base.placement = Placement::new(100.0, 200.0, 30.0, 40.0);
            let shape = Arc::make_mut(
                page.nodes
                    .get_mut(&NodeId::new(2))
                    .expect("the node is there"),
            );
            shape.base.placement = Placement::new(5.0, 5.0, 4.0, 8.0);
        }

        let expected = page
            .node(NodeId::new(2))
            .expect("the shape is there")
            .base
            .placement
            .to_transform();
        let graph = build(&page, WalkOrder::PreOrder);
        let node = graph
            .nodes()
            .iter()
            .find(|node| node.origin() == NodeId::new(2))
            .expect("the shape is in the graph");
        assert_eq!(node.transform().as_coeffs(), expected.as_coeffs());
    }

    // ARQ-141: the layers the graph pushes are the layers it pops.
    #[test]
    fn arq_141_the_layers_are_balanced() {
        let mut page = new_page();
        add(&mut page, 1, NodeKind::Group(GroupNode {}));
        add(&mut page, 2, shape());
        add(&mut page, 3, NodeKind::Group(GroupNode {}));
        add(&mut page, 4, shape());
        children_of(&mut page, 1, &[2, 3]);
        children_of(&mut page, 3, &[4]);
        page.roots = vec![NodeId::new(1)];

        let graph = build(&page, WalkOrder::PreOrder);
        let mut depth = 0i32;
        for op in graph.ops() {
            match op {
                RenderOp::PushLayer => depth += 1,
                RenderOp::PopLayer { .. } => {
                    depth -= 1;
                    assert!(depth >= 0, "a layer is popped before it is pushed");
                }
                _ => {}
            }
        }
        assert_eq!(depth, 0, "every layer pushed is popped");
    }

    // ARQ-141: the blend mode and the opacity of a node travel with the layer
    // it opens.
    #[test]
    fn arq_141_the_blend_and_the_opacity_are_emitted() {
        let mut page = new_page();
        add(&mut page, 1, shape());
        page.roots = vec![NodeId::new(1)];
        let node = Arc::make_mut(
            page.nodes
                .get_mut(&NodeId::new(1))
                .expect("the node is there"),
        );
        node.base.blend = BlendMode::Multiply;
        node.base.opacity = 0.25;

        let graph = build(&page, WalkOrder::PreOrder);
        let popped = graph
            .ops()
            .iter()
            .find_map(|op| match op {
                RenderOp::PopLayer { blend, opacity } => Some((*blend, *opacity)),
                _ => None,
            })
            .expect("the layer is popped");
        assert_eq!(popped, (BlendMode::Multiply, 0.25));
    }

    // ARQ-111, ARQ-140: the same input gives the same bytes and the same
    // graph, twice.
    #[test]
    fn arq_140_the_same_input_gives_the_same_graph() {
        let mut page = new_page();
        add(&mut page, 1, NodeKind::Group(GroupNode {}));
        add(&mut page, 2, shape());
        children_of(&mut page, 1, &[2]);
        page.roots = vec![NodeId::new(1)];

        let first = build(&page, WalkOrder::PreOrder);
        let second = build(&page, WalkOrder::PreOrder);
        let keys = |graph: &RenderGraph| {
            graph
                .nodes()
                .iter()
                .map(|node| node.key())
                .collect::<Vec<_>>()
        };
        assert_eq!(keys(&first), keys(&second));
        assert_eq!(op_kinds(&first), op_kinds(&second));
    }

    // ARQ-110: a number that is not finite is an error that names the field.
    #[test]
    fn arq_110_a_non_finite_number_is_an_error() {
        let mut page = new_page();
        add(&mut page, 1, shape());
        page.roots = vec![NodeId::new(1)];
        let node = Arc::make_mut(
            page.nodes
                .get_mut(&NodeId::new(1))
                .expect("the node is there"),
        );
        node.base.opacity = f32::NAN;
        let result = build_with(
            &page,
            WalkOrder::PreOrder,
            RenderQuality::Preview,
            InlinePool,
        );
        assert_eq!(
            result.err(),
            Some(GraphError::NonFinite { field: "opacity" })
        );

        let mut other = new_page();
        add(&mut other, 1, shape());
        other.roots = vec![NodeId::new(1)];
        let node = Arc::make_mut(
            other
                .nodes
                .get_mut(&NodeId::new(1))
                .expect("the node is there"),
        );
        node.base.placement.x = f64::INFINITY;
        let result = build_with(
            &other,
            WalkOrder::PreOrder,
            RenderQuality::Preview,
            InlinePool,
        );
        assert_eq!(
            result.err(),
            Some(GraphError::NonFinite { field: "placement" })
        );
    }

    // ARQ-139: the build reports a progress that never goes back and stops
    // when the token is cancelled.
    #[test]
    fn arq_139_the_build_reports_the_progress_and_honours_the_cancel() {
        let mut page = new_page();
        add(&mut page, 1, NodeKind::Group(GroupNode {}));
        for id in 2..=65 {
            add(&mut page, id, shape());
        }
        let children: Vec<u64> = (2..=65).collect();
        children_of(&mut page, 1, &children);
        page.roots = vec![NodeId::new(1)];

        let result =
            check_progress_contract(0.02, |token, progress| {
                let view = PageView::new(&page);
                match RenderGraphBuilder::new(&view, params(RenderQuality::Preview), InlinePool)
                    .build(WalkOrder::PreOrder, token, progress)
                {
                    Ok(_) => Ok(false),
                    Err(GraphError::Cancelled(_)) => Ok(true),
                    Err(other) => Err(other),
                }
            });
        assert_eq!(result, Ok(Ok(())));
    }

    // ARQ-140: the graph and the keys do not depend on the number of threads.
    // ARQ-138: the pool of workers is a parameter: the graph is built with the
    // pool the caller hands in, of one thread or of four, and the crate
    // creates no thread of its own.
    #[test]
    fn arq_140_the_graph_does_not_depend_on_the_number_of_threads() {
        let mut page = new_page();
        for id in 1..=4 {
            add(&mut page, id, shape());
            // A different placement for every shape: the keys differ and the
            // order of the graph shows.
            let node = Arc::make_mut(
                page.nodes
                    .get_mut(&NodeId::new(id))
                    .expect("the node is there"),
            );
            node.base.placement = Placement::new(id as f64 * 10.0, 0.0, 10.0, 10.0);
        }
        page.roots = (1..=4).map(NodeId::new).collect();

        let keys = |threads: usize| {
            let pool = ThreadedPool { threads };
            let graph = build_with(&page, WalkOrder::PreOrder, RenderQuality::Preview, &pool)
                .expect("the page builds");
            graph
                .nodes()
                .iter()
                .map(|node| node.key())
                .collect::<Vec<_>>()
        };
        assert_eq!(keys(1), keys(2));
        assert_eq!(keys(1), keys(4));
    }
}
