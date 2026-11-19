//! The render graph of a page and the builder that walks it (02 §6.1).
//!
//! The graph holds the nodes that are drawn and the operations they emit, in
//! paint order. The builder is the visitor of the walk of `gafira-doc`
//! (02 §4.1): the walk reaches every node and the builder cuts the ones that
//! draw nothing.
//!
//! The graph holds no pixel of its own: an image element draws the decoded
//! pixels the caller hands over (spec 032), shared, never copied; no effect
//! and no mask reaches it yet.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::sync::Arc;

use gafira_color::Color;
use gafira_doc::contract::{CancelToken, Cancelled, Progress, checkpoint};
use gafira_doc::visit::{NodeVisitor, PageView, WalkOrder, walk};
use gafira_doc::{
    AssetRef, BlendMode, ElementBase, ExtensionNode, GroupNode, ImageNode, Node, NodeId, Page,
    Placement, ShapeGeometry, ShapeNode, TextAlign, TextNode,
};
use gafira_geom::Affine;
use gafira_pathops::{Cap, FillRule, Join, StrokeStyle};
use gafira_ports::export::ImagePixels;
use gafira_ports::render::{ContentKey, RenderOp, RenderQuality};
use gafira_time::Flicks;
use kurbo::{BezPath, Ellipse, Rect, RoundedRect, Shape};

use crate::fonts::FontBook;
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
    /// A text: it fills the outlines of its glyphs (spec 030).
    Text(TextNode),
    /// An image: it draws the pixels of its asset into its box (spec 032).
    Image(AssetRef),
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
///
/// A text is laid out and outlined with the fonts of [`Self::with_fonts`]
/// (02 §4.6, spec 030); without them it draws nothing. The scale of
/// [`Self::scaled`] goes in front of the placement of every node, so a page
/// can be drawn at a multiple of its size (FD-55).
///
/// An image draws the pixels of its asset from [`Self::with_images`]
/// (spec 032); an asset the map does not hold draws nothing, which is what
/// the preview gets, because only an export resolves the assets.
pub struct RenderGraphBuilder<'a, P: WorkerPool> {
    page: &'a Page,
    params: GraphParams,
    pool: P,
    records: BTreeMap<NodeId, Record>,
    scale: f64,
    fonts: Arc<FontBook>,
    images: Arc<ImagePixels>,
}

impl<'a, P: WorkerPool> RenderGraphBuilder<'a, P> {
    /// A builder over the page of the view.
    pub fn new(view: &PageView<'a>, params: GraphParams, pool: P) -> Self {
        Self {
            page: view.page(),
            params,
            pool,
            records: BTreeMap::new(),
            scale: 1.0,
            fonts: Arc::new(FontBook::new()),
            images: Arc::new(ImagePixels::new()),
        }
    }

    /// The builder that draws the page `scale` times its size (FD-55): every
    /// placement is scaled from the page origin. The caller checks that the
    /// scale is finite and above zero; a scale that is not finite reaches
    /// every transform and the build answers [`GraphError::NonFinite`].
    #[must_use]
    pub fn scaled(mut self, scale: f64) -> Self {
        self.scale = scale;
        self
    }

    /// The builder that lays text out with `fonts` (spec 030).
    #[must_use]
    pub fn with_fonts(mut self, fonts: Arc<FontBook>) -> Self {
        self.fonts = fonts;
        self
    }

    /// The builder that draws each image with the pixels of its asset in
    /// `images` (spec 032).
    #[must_use]
    pub fn with_images(mut self, images: Arc<ImagePixels>) -> Self {
        self.images = images;
        self
    }

    /// The transform of a node: the scale of the build in front of its
    /// placement (D-35: every placement is in page space).
    fn transform_of(&self, placement: &Placement) -> Affine {
        Affine::scale(self.scale) * placement.to_transform()
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
                let transform = self.transform_of(&record.base.placement);
                check_transform(&transform)?;
                check_opacity(record.base.opacity)?;
                let mark = sub.ops.len();
                sub.ops.push(RenderOp::PushLayer);
                match record.kind {
                    RecordKind::Shape(ref shape) => {
                        let Some(drawn) = DrawnShape::of(shape, &record.base.placement)? else {
                            // A shape without a fill or an outline draws nothing.
                            sub.ops.truncate(mark);
                            continue;
                        };
                        let key = self.node_key(
                            NodeType::Shape,
                            &[],
                            &record.base,
                            transform,
                            &drawn.key_params(&shape.geometry),
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
                        for (path, color) in drawn.layers() {
                            sub.ops.push(RenderOp::FillPath {
                                path: Arc::new(path),
                                color,
                                transform,
                            });
                        }
                        sub.ops.push(RenderOp::PopLayer {
                            blend: record.base.blend,
                            opacity: record.base.opacity,
                        });
                    }
                    RecordKind::Text(ref text) => {
                        let Some((path, color, canonical)) =
                            text_path(text, &record.base.placement, &self.fonts)
                        else {
                            // No words, no fonts or no glyph: nothing to draw.
                            sub.ops.truncate(mark);
                            continue;
                        };
                        check_color(&color, "fill")?;
                        let key = content_key(
                            NodeType::Text,
                            self.params.lod,
                            &[],
                            &[
                                KeyParam::Quality(self.params.quality),
                                KeyParam::Text(canonical),
                                KeyParam::Fill(Some(color)),
                                KeyParam::Transform(transform.as_coeffs()),
                                KeyParam::Opacity(record.base.opacity),
                                KeyParam::Blend(record.base.blend),
                            ],
                        );
                        let node = GraphNode {
                            id: GraphNodeId(sub.nodes.len() as u32),
                            kind: GraphNodeKind::Source(SourceKind::Text),
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
                    RecordKind::Image(asset) => {
                        let placement = &record.base.placement;
                        let Some(image) = self.images.get(&asset) else {
                            // No pixels for the asset (the preview): nothing.
                            sub.ops.truncate(mark);
                            continue;
                        };
                        if placement.w == 0.0 || placement.h == 0.0 {
                            // A box with no area shows nothing.
                            sub.ops.truncate(mark);
                            continue;
                        }
                        // The image space maps onto the box of the node:
                        // its pixels are stretched to the box (spec 032).
                        let fit = Affine::scale_non_uniform(
                            placement.w / f64::from(image.width()),
                            placement.h / f64::from(image.height()),
                        );
                        let drawn = transform * fit;
                        check_transform(&drawn)?;
                        let key = content_key(
                            NodeType::Image,
                            self.params.lod,
                            &[],
                            &[
                                KeyParam::Quality(self.params.quality),
                                KeyParam::Asset(asset.as_hash().0),
                                KeyParam::Transform(drawn.as_coeffs()),
                                KeyParam::Opacity(record.base.opacity),
                                KeyParam::Blend(record.base.blend),
                            ],
                        );
                        let node = GraphNode {
                            id: GraphNodeId(sub.nodes.len() as u32),
                            kind: GraphNodeKind::Source(SourceKind::Image),
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
                        sub.ops.push(RenderOp::DrawImage {
                            image: Arc::clone(image),
                            transform: drawn,
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
            let transform = self.transform_of(&record.base.placement);
            let key = self.node_key(NodeType::Group, &inputs, &record.base, transform, &[]);
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
        shape: &[KeyParam],
    ) -> ContentKey {
        let mut params = vec![KeyParam::Quality(self.params.quality)];
        params.extend_from_slice(shape);
        params.extend([
            KeyParam::Transform(transform.as_coeffs()),
            KeyParam::Opacity(base.opacity),
            KeyParam::Blend(base.blend),
        ]);
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
                kind: RecordKind::Shape(shape.clone()),
            },
        );
        node.base.id
    }

    fn visit_text(&mut self, node: &Node, text: &TextNode) -> NodeId {
        self.records.insert(
            node.base.id,
            Record {
                base: node.base.clone(),
                kind: RecordKind::Text(text.clone()),
            },
        );
        node.base.id
    }

    fn visit_image(&mut self, node: &Node, image: &ImageNode) -> NodeId {
        self.records.insert(
            node.base.id,
            Record {
                base: node.base.clone(),
                kind: RecordKind::Image(image.asset),
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

/// The flattening tolerance of shape outlines, in px of the node's box.
const SHAPE_TOLERANCE: f64 = 0.25;

/// What a shape draws, in its own box: its fill and its outline (spec 038).
struct DrawnShape {
    fill: Option<(BezPath, Color)>,
    stroke: Option<(BezPath, Color, f64)>,
    /// The radius the rectangle is drawn with; 0 for an ellipse.
    radius: f64,
}

impl DrawnShape {
    /// What `shape` draws in the box of `placement`, `None` when it draws
    /// nothing; a number that is not finite is an error (ARQ-110).
    fn of(shape: &ShapeNode, placement: &Placement) -> Result<Option<Self>, GraphError> {
        if let Some(color) = &shape.fill {
            check_color(color, "fill")?;
        }
        if let Some(stroke) = &shape.stroke {
            check_color(&stroke.color, "stroke")?;
            if !stroke.width.is_finite() {
                return Err(GraphError::NonFinite { field: "stroke" });
            }
        }
        if !shape.corner_radius.is_finite() {
            return Err(GraphError::NonFinite {
                field: "corner_radius",
            });
        }
        let rect = Rect::new(0.0, 0.0, placement.w, placement.h).abs();
        let radius = match shape.geometry {
            ShapeGeometry::Rect => drawn_radius(shape.corner_radius, rect),
            ShapeGeometry::Ellipse | ShapeGeometry::Path(_) => 0.0,
        };
        let outline = shape_path(&shape.geometry, rect, radius);
        let stroke = shape
            .stroke
            .filter(|stroke| stroke.width > 0.0)
            .and_then(|stroke| {
                stroke_path(&outline, stroke.width).map(|path| (path, stroke.color, stroke.width))
            });
        let fill = shape.fill.map(|color| (outline, color));
        Ok((fill.is_some() || stroke.is_some()).then_some(Self {
            fill,
            stroke,
            radius,
        }))
    }

    /// The parameters of the content key that this shape draws. A shape with
    /// only a fill and square corners gives the parameters it had before
    /// spec 038, so its key does not move.
    fn key_params(&self, geometry: &ShapeGeometry) -> Vec<KeyParam> {
        let mut params = vec![
            KeyParam::Geometry(geometry.clone()),
            KeyParam::Fill(self.fill.as_ref().map(|(_, color)| *color)),
        ];
        if let Some((_, color, width)) = &self.stroke {
            params.push(KeyParam::Stroke(*color, *width));
        }
        if self.radius > 0.0 {
            params.push(KeyParam::CornerRadius(self.radius));
        }
        params
    }

    /// The paths to fill, back to front: the fill, then the outline over it.
    fn layers(self) -> impl Iterator<Item = (BezPath, Color)> {
        self.fill
            .into_iter()
            .chain(self.stroke.map(|(path, color, _)| (path, color)))
    }
}

/// The radius a rectangle is drawn with: the stored one, limited to half
/// its shorter side so the corners never overlap (FD-36 a).
fn drawn_radius(radius: f64, rect: Rect) -> f64 {
    radius.clamp(0.0, rect.width().min(rect.height()) / 2.0)
}

/// The outline of a shape, in its own box.
fn shape_path(geometry: &ShapeGeometry, rect: Rect, radius: f64) -> BezPath {
    match geometry {
        ShapeGeometry::Path(path) => unit_path(path, rect),
        ShapeGeometry::Rect if radius > 0.0 => {
            RoundedRect::from_rect(rect, radius).to_path(SHAPE_TOLERANCE)
        }
        ShapeGeometry::Rect => rect.to_path(SHAPE_TOLERANCE),
        ShapeGeometry::Ellipse => Ellipse::from_rect(rect).to_path(SHAPE_TOLERANCE),
    }
}

/// A shape path (spec 054) in the box `rect`: each unit-box point scaled
/// to the box. Its interior is the non-zero one, as the fill rule of every
/// shape.
pub fn unit_path(path: &gafira_doc::ShapePath, rect: Rect) -> BezPath {
    use gafira_doc::PathCommand;
    let at = |[x, y]: [f64; 2]| {
        kurbo::Point::new(rect.x0 + x * rect.width(), rect.y0 + y * rect.height())
    };
    let mut out = BezPath::new();
    for command in &path.commands {
        match *command {
            PathCommand::MoveTo(p) => out.move_to(at(p)),
            PathCommand::LineTo(p) => out.line_to(at(p)),
            PathCommand::QuadTo(c, p) => out.quad_to(at(c), at(p)),
            PathCommand::CubicTo(c1, c2, p) => out.curve_to(at(c1), at(c2), at(p)),
            PathCommand::Close => out.close_path(),
        }
    }
    out
}

/// The area an outline of `width` covers along `outline`, centred on it,
/// with mitred corners; `None` when `gafira-pathops` refuses the path (a
/// coordinate beyond its limit), so the shape keeps its fill.
fn stroke_path(outline: &BezPath, width: f64) -> Option<BezPath> {
    let style = StrokeStyle {
        width,
        cap: Cap::Butt,
        join: Join::Miter,
    };
    gafira_pathops::Path::new(outline.clone(), FillRule::NonZero)
        .stroke_outline(&style, SHAPE_TOLERANCE)
        .ok()
        .map(|path| path.as_bez_path().clone())
}

/// The size range of `gafira_text::TextStyle::size_pt` (FD-41).
const TEXT_SIZE_PT: std::ops::RangeInclusive<f64> = 1.0..=1000.0;

/// The outline of a text in its own box, its colour and the canonical string
/// of its key (spec 030), or `None` when it draws nothing.
///
/// The first slice of `TextNode` keeps one style per element (spec 027), so
/// the whole text is laid out with the style of its first span. The size of
/// the document is in px; `gafira-text` takes points in `1..=1000` and a dpi,
/// so the size is kept in that range and the dpi carries the rest: the glyphs
/// come out at the size in px of the document. A text that does not lay out
/// (no font covers it, a number out of range) draws nothing.
fn text_path(
    text: &TextNode,
    placement: &Placement,
    fonts: &FontBook,
) -> Option<(BezPath, Color, String)> {
    let style = text.content.first_style()?;
    let words = text.content.plain_text();
    if words.trim().is_empty() || !style.size.is_finite() || style.size <= 0.0 {
        return None;
    }
    let stack = fonts.stack(&style.font);
    if stack.is_empty() {
        return None;
    }
    let size_pt = style.size.clamp(*TEXT_SIZE_PT.start(), *TEXT_SIZE_PT.end());
    let layout_style = gafira_text::TextStyle {
        size_pt,
        align: match text.align {
            TextAlign::Left => gafira_text::TextAlign::Left,
            TextAlign::Center => gafira_text::TextAlign::Center,
            TextAlign::Right => gafira_text::TextAlign::Right,
            TextAlign::Justify => gafira_text::TextAlign::Justify,
        },
        ..gafira_text::TextStyle::default()
    };
    let dpi = 72.0 * style.size / size_pt;
    let bounds = gafira_geom::Size::new(placement.w.abs(), placement.h.abs());
    let layout = gafira_text::layout_box(&words, &stack, &layout_style, dpi, bounds).ok()?;
    let outlines = gafira_text::outline_text(&layout, &stack).ok()?;
    let mut path = BezPath::new();
    for outline in outlines {
        path.extend(outline.path.as_bez_path().elements().iter().copied());
    }
    if path.elements().is_empty() {
        return None;
    }
    let align = match text.align {
        TextAlign::Left => 'l',
        TextAlign::Center => 'c',
        TextAlign::Right => 'r',
        TextAlign::Justify => 'j',
    };
    let canonical = format!(
        "{}\u{0}{:016x}\u{0}{align}\u{0}{words}",
        style.font,
        style.size.to_bits()
    );
    Some((path, style.color, canonical))
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

/// The check of ARQ-110 for a colour; `field` names it.
fn check_color(color: &Color, field: &'static str) -> Result<(), GraphError> {
    let channels = match color {
        Color::Srgb(c) => [c.r, c.g, c.b, c.a],
        Color::LinearSrgb(c) => [c.r, c.g, c.b, c.a],
        Color::DisplayP3(c) => [c.r, c.g, c.b, c.a],
    };
    if channels.iter().all(|c| c.is_finite()) {
        Ok(())
    } else {
        Err(GraphError::NonFinite { field })
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

    // S54-03: a shape path is scaled into the shape's box, fills its inside
    // and not the rest of the box, and two different paths key apart.
    #[test]
    fn s54_03_a_path_triangle_fills_its_inside() {
        use gafira_doc::{PathCommand, ShapePath};
        let triangle = ShapePath::new(vec![
            PathCommand::MoveTo([0.5, 0.0]),
            PathCommand::LineTo([1.0, 1.0]),
            PathCommand::LineTo([0.0, 1.0]),
            PathCommand::Close,
        ]);
        let rect = Rect::new(0.0, 0.0, 200.0, 100.0);
        let path = shape_path(&ShapeGeometry::Path(triangle.clone()), rect, 0.0);
        assert!(path.contains(kurbo::Point::new(100.0, 70.0)), "inside");
        assert!(
            !path.contains(kurbo::Point::new(10.0, 10.0)),
            "a corner of the box"
        );
        assert_eq!(path.bounding_box(), rect);
        let mut other = triangle.clone();
        other.commands[0] = PathCommand::MoveTo([0.4, 0.0]);
        let bytes = |path: ShapePath| {
            let mut out = Vec::new();
            KeyParam::Geometry(ShapeGeometry::Path(path)).write_for_test(&mut out);
            out
        };
        assert_eq!(bytes(triangle.clone()), bytes(triangle.clone()));
        assert_ne!(bytes(triangle.clone()), bytes(other));
        // The same points with other orders key apart too.
        let mut moved = triangle.clone();
        moved.commands[1] = PathCommand::MoveTo([1.0, 1.0]);
        assert_ne!(bytes(triangle), bytes(moved));
    }

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
            stroke: None,
            corner_radius: 0.0,
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

    /// A text element of one style, in a box of 200 by 60 at (10, 20).
    fn text_page(words: &str) -> Page {
        let mut page = new_page();
        let style = gafira_doc::SpanStyle {
            font: "Inter".to_owned(),
            size: 32.0,
            color: Color::Srgb(Rgba::new(0.1, 0.2, 0.3, 1.0)),
        };
        let base = ElementBase::new(NodeId::new(1), Placement::new(10.0, 20.0, 200.0, 60.0));
        let kind = NodeKind::Text(TextNode::plain(words, &style));
        page.nodes
            .insert(NodeId::new(1), Arc::new(Node::new(base, kind)));
        page.roots = vec![NodeId::new(1)];
        page
    }

    /// The fonts of the tests: the Inter subset of the testkit.
    fn fonts() -> Arc<FontBook> {
        Arc::new(FontBook::new().with("Inter", gafira_testkit::fonts::INTER.bytes))
    }

    /// Builds the graph of the page with fonts and a scale.
    fn build_text(page: &Page, scale: f64, fonts: Option<Arc<FontBook>>) -> RenderGraph {
        let view = PageView::new(page);
        let mut builder =
            RenderGraphBuilder::new(&view, params(RenderQuality::Export), InlinePool).scaled(scale);
        if let Some(fonts) = fonts {
            builder = builder.with_fonts(fonts);
        }
        builder
            .build(WalkOrder::PreOrder, &CancelToken::new(), &NoProgress)
            .expect("the page builds")
    }

    /// The fills of a graph: their path's bounding box in the node's own box,
    /// their colour and their transform.
    fn fills(graph: &RenderGraph) -> Vec<(Rect, Color, Affine)> {
        graph
            .ops()
            .iter()
            .filter_map(|op| match op {
                RenderOp::FillPath {
                    path,
                    color,
                    transform,
                } => Some((path.bounding_box(), *color, *transform)),
                _ => None,
            })
            .collect()
    }

    // S30-01: a text element fills the outlines of its glyphs, in its colour,
    // inside its box and moved by its placement.
    #[test]
    fn s30_01_a_text_fills_its_glyph_outlines_inside_its_box() {
        let page = text_page("Hola");
        let graph = build_text(&page, 1.0, Some(fonts()));
        assert_eq!(op_kinds(&graph), ["push", "fill", "pop"]);
        let [(bounds, color, transform)] = fills(&graph)[..] else {
            panic!("one fill");
        };
        assert_eq!(color, Color::Srgb(Rgba::new(0.1, 0.2, 0.3, 1.0)));
        assert_eq!(
            transform,
            Placement::new(10.0, 20.0, 200.0, 60.0).to_transform()
        );
        assert!(
            bounds.width() > 10.0 && bounds.height() > 10.0,
            "{bounds:?}"
        );
        assert!(
            bounds.x0 >= 0.0 && bounds.y0 >= 0.0 && bounds.x1 <= 200.0 && bounds.y1 <= 60.0,
            "the glyphs stay in the box: {bounds:?}"
        );
        let node = graph
            .nodes()
            .iter()
            .find(|node| node.origin() == NodeId::new(1))
            .expect("the text is a node");
        assert_eq!(node.kind(), GraphNodeKind::Source(SourceKind::Text));
    }

    // S30-01: no fonts, no words or only spaces draw nothing; other words give
    // another key.
    #[test]
    fn s30_01_a_text_without_fonts_or_words_draws_nothing_and_its_words_key_it() {
        assert!(build_text(&text_page("Hola"), 1.0, None).ops().is_empty());
        assert!(
            build_text(&text_page(""), 1.0, Some(fonts()))
                .ops()
                .is_empty()
        );
        assert!(
            build_text(&text_page("   "), 1.0, Some(fonts()))
                .ops()
                .is_empty()
        );
        let hola = build_text(&text_page("Hola"), 1.0, Some(fonts()));
        let adios = build_text(&text_page("Adiós"), 1.0, Some(fonts()));
        assert_ne!(hola.key_of(NodeId::new(1)), adios.key_of(NodeId::new(1)));
    }

    // S30-02: the scale of an export goes in front of the placement of every
    // node, so the key of a scaled node differs from the unscaled one.
    #[test]
    fn s30_02_the_scale_goes_in_front_of_every_placement() {
        let mut page = new_page();
        add(&mut page, 1, shape());
        page.roots = vec![NodeId::new(1)];
        let one = build_text(&page, 1.0, None);
        let two = build_text(&page, 2.0, None);
        let placement = Placement::new(0.0, 0.0, 10.0, 10.0).to_transform();
        assert_eq!(fills(&one)[0].2, placement);
        assert_eq!(fills(&two)[0].2, Affine::scale(2.0) * placement);
        assert_ne!(one.key_of(NodeId::new(1)), two.key_of(NodeId::new(1)));
        let text = build_text(&text_page("Hola"), 2.0, Some(fonts()));
        assert_eq!(
            fills(&text)[0].2,
            Affine::scale(2.0) * Placement::new(10.0, 20.0, 200.0, 60.0).to_transform()
        );
    }

    /// The asset of the image tests.
    fn asset(byte: u8) -> gafira_doc::AssetRef {
        gafira_doc::AssetRef::new(gafira_doc::ContentHash([byte; 32]))
    }

    /// An image element of `asset` in a box of 40 by 20 at (10, 30), at half
    /// opacity, multiplied.
    fn image_page(asset: gafira_doc::AssetRef) -> Page {
        let mut page = new_page();
        let mut base = ElementBase::new(NodeId::new(1), Placement::new(10.0, 30.0, 40.0, 20.0));
        base.opacity = 0.5;
        base.blend = BlendMode::Multiply;
        let kind = NodeKind::Image(gafira_doc::ImageNode { asset });
        page.nodes
            .insert(NodeId::new(1), Arc::new(Node::new(base, kind)));
        page.roots = vec![NodeId::new(1)];
        page
    }

    /// Pixels of 4 by 2 for `asset`.
    fn pixels_of(asset: gafira_doc::AssetRef) -> Arc<ImagePixels> {
        let image = gafira_raster::ImageBuf::new(
            4,
            2,
            gafira_raster::PixelBuf::Rgba8(vec![255; 4 * 2 * 4]),
        )
        .expect("an image");
        Arc::new(ImagePixels::from([(asset, Arc::new(image))]))
    }

    /// Builds the graph of the page with images and a scale.
    fn build_image(page: &Page, scale: f64, images: Option<Arc<ImagePixels>>) -> RenderGraph {
        let view = PageView::new(page);
        let mut builder =
            RenderGraphBuilder::new(&view, params(RenderQuality::Export), InlinePool).scaled(scale);
        if let Some(images) = images {
            builder = builder.with_images(images);
        }
        builder
            .build(WalkOrder::PreOrder, &CancelToken::new(), &NoProgress)
            .expect("the page builds")
    }

    /// The image draws of a graph: their size and their transform.
    fn draws(graph: &RenderGraph) -> Vec<((u32, u32), Affine)> {
        graph
            .ops()
            .iter()
            .filter_map(|op| match op {
                RenderOp::DrawImage { image, transform } => {
                    Some(((image.width(), image.height()), *transform))
                }
                _ => None,
            })
            .collect()
    }

    /// `point` mapped by `transform`, rounded to 1e-9.
    fn mapped(transform: Affine, x: f64, y: f64) -> (f64, f64) {
        let p = transform * kurbo::Point::new(x, y);
        let round = |v: f64| (v * 1e9).round() / 1e9;
        (round(p.x), round(p.y))
    }

    // S32-01: an image draws its pixels inside its layer, which carries the
    // opacity and the blend of the node, and its corners land on the corners
    // of its box.
    #[test]
    fn s32_01_an_image_draws_its_pixels_into_its_box_inside_its_layer() {
        let page = image_page(asset(7));
        let graph = build_image(&page, 1.0, Some(pixels_of(asset(7))));
        assert_eq!(op_kinds(&graph), ["push", "image", "pop"]);
        let popped = graph.ops().iter().find_map(|op| match op {
            RenderOp::PopLayer { blend, opacity } => Some((*blend, *opacity)),
            _ => None,
        });
        assert_eq!(popped, Some((BlendMode::Multiply, 0.5)));
        let [(size, transform)] = draws(&graph)[..] else {
            panic!("one image draw");
        };
        assert_eq!(size, (4, 2));
        assert_eq!(mapped(transform, 0.0, 0.0), (10.0, 30.0));
        assert_eq!(mapped(transform, 4.0, 0.0), (50.0, 30.0));
        assert_eq!(mapped(transform, 4.0, 2.0), (50.0, 50.0));
        assert_eq!(mapped(transform, 0.0, 2.0), (10.0, 50.0));
        let node = graph
            .nodes()
            .iter()
            .find(|node| node.origin() == NodeId::new(1))
            .expect("the image is a node");
        assert_eq!(node.kind(), GraphNodeKind::Source(SourceKind::Image));
        assert_eq!(node.opacity(), 0.5);
    }

    // S32-01, S30-02: the export scale goes in front of the placement of an
    // image too.
    #[test]
    fn s32_01_the_scale_goes_in_front_of_an_image() {
        let graph = build_image(&image_page(asset(7)), 2.0, Some(pixels_of(asset(7))));
        let [(_, transform)] = draws(&graph)[..] else {
            panic!("one image draw");
        };
        assert_eq!(mapped(transform, 0.0, 0.0), (20.0, 60.0));
        assert_eq!(mapped(transform, 4.0, 2.0), (100.0, 100.0));
    }

    // S32-01: an image without pixels, or with a side of zero, draws nothing;
    // the asset keys the node.
    #[test]
    fn s32_01_an_image_without_pixels_draws_nothing_and_its_asset_keys_it() {
        assert!(
            build_image(&image_page(asset(7)), 1.0, None)
                .ops()
                .is_empty()
        );
        assert!(
            build_image(&image_page(asset(7)), 1.0, Some(pixels_of(asset(8))))
                .ops()
                .is_empty()
        );
        let mut flat = image_page(asset(7));
        Arc::make_mut(flat.nodes.get_mut(&NodeId::new(1)).expect("the node"))
            .base
            .placement
            .h = 0.0;
        assert!(
            build_image(&flat, 1.0, Some(pixels_of(asset(7))))
                .ops()
                .is_empty()
        );
        let seven = build_image(&image_page(asset(7)), 1.0, Some(pixels_of(asset(7))));
        let eight = build_image(&image_page(asset(8)), 1.0, Some(pixels_of(asset(8))));
        assert!(seven.key_of(NodeId::new(1)).is_some());
        assert_ne!(seven.key_of(NodeId::new(1)), eight.key_of(NodeId::new(1)));
    }

    const RED: Color = Color::Srgb(Rgba::new(1.0, 0.0, 0.0, 1.0));
    const BLUE: Color = Color::Srgb(Rgba::new(0.0, 0.0, 1.0, 1.0));

    /// A page with one shape of `kind` in the box `(10, 20, 40, 20)`.
    fn outlined_page(shape: ShapeNode) -> Page {
        let mut page = new_page();
        let base = ElementBase::new(NodeId::new(1), Placement::new(10.0, 20.0, 40.0, 20.0));
        page.nodes.insert(
            NodeId::new(1),
            Arc::new(Node::new(base, NodeKind::Shape(shape))),
        );
        page.roots = vec![NodeId::new(1)];
        page
    }

    fn outlined(
        geometry: ShapeGeometry,
        fill: Option<Color>,
        width: f64,
        radius: f64,
    ) -> ShapeNode {
        let mut shape = ShapeNode::new(geometry);
        shape.fill = fill;
        shape.stroke = Some(gafira_doc::ShapeStroke::new(BLUE, width));
        shape.corner_radius = radius;
        shape
    }

    fn close(a: Rect, b: Rect) -> bool {
        (a.x0 - b.x0).abs() < 1e-6
            && (a.y0 - b.y0).abs() < 1e-6
            && (a.x1 - b.x1).abs() < 1e-6
            && (a.y1 - b.y1).abs() < 1e-6
    }

    // S38-04: the outline is filled over the fill, in its colour, centred on
    // the edge: half its width falls outside the box.
    #[test]
    fn s38_04_the_outline_is_drawn_over_the_fill_centred_on_the_edge() {
        let graph = build(
            &outlined_page(outlined(ShapeGeometry::Rect, Some(RED), 4.0, 0.0)),
            WalkOrder::PreOrder,
        );
        assert_eq!(op_kinds(&graph), ["push", "fill", "fill", "pop"]);
        let fills = fills(&graph);
        assert_eq!(fills[0].1, RED);
        assert!(close(fills[0].0, Rect::new(0.0, 0.0, 40.0, 20.0)));
        assert_eq!(fills[1].1, BLUE);
        assert!(
            close(fills[1].0, Rect::new(-2.0, -2.0, 42.0, 22.0)),
            "{:?}",
            fills[1].0
        );
        assert_eq!(fills[1].2, fills[0].2);
        // The outline is a ring: the middle of the box is not covered.
        let RenderOp::FillPath { path, .. } = &graph.ops()[2] else {
            panic!("the outline")
        };
        assert_eq!(path.winding(kurbo::Point::new(20.0, 10.0)), 0);
        assert_ne!(path.winding(kurbo::Point::new(0.5, 10.0)), 0);
        assert_ne!(path.winding(kurbo::Point::new(-1.5, 10.0)), 0);
    }

    // S38-04: an outline alone draws; a zero width or no outline and no fill
    // draws nothing.
    #[test]
    fn s38_04_an_outline_without_a_fill_draws_and_a_zero_width_does_not() {
        let alone = build(
            &outlined_page(outlined(ShapeGeometry::Ellipse, None, 2.0, 0.0)),
            WalkOrder::PreOrder,
        );
        assert_eq!(op_kinds(&alone), ["push", "fill", "pop"]);
        assert_eq!(fills(&alone)[0].1, BLUE);
        assert!(close(fills(&alone)[0].0, Rect::new(-1.0, -1.0, 41.0, 21.0)));
        let zero = build(
            &outlined_page(outlined(ShapeGeometry::Rect, None, 0.0, 0.0)),
            WalkOrder::PreOrder,
        );
        assert!(zero.ops().is_empty());
        assert!(zero.key_of(NodeId::new(1)).is_none());
        let filled = build(
            &outlined_page(outlined(ShapeGeometry::Rect, Some(RED), 0.0, 0.0)),
            WalkOrder::PreOrder,
        );
        assert_eq!(op_kinds(&filled), ["push", "fill", "pop"]);
    }

    // S38-05: the corner radius rounds a rectangle, limited to half its
    // shorter side (FD-36 a); an ellipse ignores it.
    #[test]
    fn s38_05_the_corner_radius_rounds_the_rectangle_up_to_half_the_short_side() {
        let path_of = |shape: ShapeNode| {
            let graph = build(&outlined_page(shape), WalkOrder::PreOrder);
            let RenderOp::FillPath { path, .. } = &graph.ops()[1] else {
                panic!("the fill")
            };
            (**path).clone()
        };
        let fill_only = |geometry, radius| {
            let mut shape = ShapeNode::new(geometry);
            shape.fill = Some(RED);
            shape.corner_radius = radius;
            shape
        };
        let corner = kurbo::Point::new(0.5, 0.5);
        let square = path_of(fill_only(ShapeGeometry::Rect, 0.0));
        assert_ne!(square.winding(corner), 0);
        let round = path_of(fill_only(ShapeGeometry::Rect, 6.0));
        assert_eq!(round.winding(corner), 0);
        assert_ne!(round.winding(kurbo::Point::new(6.0, 0.5)), 0);
        assert!(close(round.bounding_box(), Rect::new(0.0, 0.0, 40.0, 20.0)));
        // Above half the short side (10) it is drawn at 10, never deformed.
        let huge = path_of(fill_only(ShapeGeometry::Rect, 1e9));
        let half = path_of(fill_only(ShapeGeometry::Rect, 10.0));
        assert_eq!(huge, half);
        assert!(close(huge.bounding_box(), Rect::new(0.0, 0.0, 40.0, 20.0)));
        // The flat top between the two corners is still there.
        assert_ne!(huge.winding(kurbo::Point::new(20.0, 0.5)), 0);
        assert_eq!(huge.winding(kurbo::Point::new(1.0, 1.0)), 0);
        let ellipse = path_of(fill_only(ShapeGeometry::Ellipse, 6.0));
        assert_eq!(ellipse, path_of(fill_only(ShapeGeometry::Ellipse, 0.0)));
        // The outline follows the rounded corner.
        let graph = build(
            &outlined_page(outlined(ShapeGeometry::Rect, Some(RED), 2.0, 6.0)),
            WalkOrder::PreOrder,
        );
        let RenderOp::FillPath { path, .. } = &graph.ops()[2] else {
            panic!("the outline")
        };
        assert_eq!(path.winding(kurbo::Point::new(-0.5, -0.5)), 0);
        assert_ne!(path.winding(kurbo::Point::new(-0.5, 10.0)), 0);
    }

    // S38-06: the outline and the drawn radius enter the content key; a shape
    // without them keeps the key it had before spec 038.
    #[test]
    fn s38_06_the_outline_and_the_radius_key_the_shape() {
        let key = |shape: ShapeNode| {
            build(&outlined_page(shape), WalkOrder::PreOrder).key_of(NodeId::new(1))
        };
        let mut plain = ShapeNode::new(ShapeGeometry::Rect);
        plain.fill = Some(RED);
        let placement = Placement::new(10.0, 20.0, 40.0, 20.0);
        let before = content_key(
            NodeType::Shape,
            0,
            &[],
            &[
                KeyParam::Quality(RenderQuality::Preview),
                KeyParam::Geometry(ShapeGeometry::Rect),
                KeyParam::Fill(Some(RED)),
                KeyParam::Transform(placement.to_transform().as_coeffs()),
                KeyParam::Opacity(1.0),
                KeyParam::Blend(BlendMode::Normal),
            ],
        );
        assert_eq!(key(plain.clone()), Some(before));
        let thin = key(outlined(ShapeGeometry::Rect, Some(RED), 1.0, 0.0));
        let thick = key(outlined(ShapeGeometry::Rect, Some(RED), 2.0, 0.0));
        assert_ne!(thin, Some(before));
        assert_ne!(thin, thick);
        let mut green = outlined(ShapeGeometry::Rect, Some(RED), 1.0, 0.0);
        green.stroke = Some(gafira_doc::ShapeStroke::new(
            Color::Srgb(Rgba::new(0.0, 1.0, 0.0, 1.0)),
            1.0,
        ));
        assert_ne!(key(green), thin);
        let mut round = plain.clone();
        round.corner_radius = 4.0;
        assert_ne!(key(round.clone()), Some(before));
        // Radii drawn the same give the same key.
        round.corner_radius = 10.0;
        let mut rounder = plain;
        rounder.corner_radius = 500.0;
        assert_eq!(key(round), key(rounder));
        // A zero-width outline draws nothing and keys nothing.
        assert_eq!(
            key(outlined(ShapeGeometry::Rect, Some(RED), 0.0, 0.0)),
            Some(before)
        );
    }

    // ARQ-110: a non-finite outline width or radius is an error naming it.
    #[test]
    fn s38_04_a_non_finite_outline_or_radius_is_an_error() {
        let fail = |shape: ShapeNode| {
            build_with(
                &outlined_page(shape),
                WalkOrder::PreOrder,
                RenderQuality::Preview,
                InlinePool,
            )
            .err()
        };
        assert_eq!(
            fail(outlined(ShapeGeometry::Rect, None, f64::NAN, 0.0)),
            Some(GraphError::NonFinite { field: "stroke" })
        );
        let mut colour = outlined(ShapeGeometry::Rect, None, 1.0, 0.0);
        colour.stroke = Some(gafira_doc::ShapeStroke::new(
            Color::Srgb(Rgba::new(f32::NAN, 0.0, 0.0, 1.0)),
            1.0,
        ));
        assert_eq!(
            fail(colour),
            Some(GraphError::NonFinite { field: "stroke" })
        );
        assert_eq!(
            fail(outlined(ShapeGeometry::Rect, Some(RED), 1.0, f64::INFINITY)),
            Some(GraphError::NonFinite {
                field: "corner_radius"
            })
        );
    }
}
