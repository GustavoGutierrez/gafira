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
    Placement, ShapeEffect, ShapeGeometry, ShapeNode, TextAlign, TextNode,
};
use gafira_geom::Affine;
use gafira_pathops::{Cap, FillRule, Join, StrokeStyle};
use gafira_ports::export::ImagePixels;
use gafira_ports::render::{ContentKey, GlyphRun, RenderOp, RenderQuality, RunGlyph};
use gafira_raster::{ImageBuf, LightSpot};
use gafira_time::Flicks;
use kurbo::{BezPath, Ellipse, Rect, RoundedRect, Shape};

use crate::fonts::FontBook;
use crate::key::{KeyParam, NodeType, content_key};
use crate::pool::WorkerPool;
use crate::text_effects::{self, TextPiece};

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
    /// An image: it draws the pixels it shows into its box (spec 032), and
    /// its shadow around it (spec 085).
    Image(AssetRef, Option<gafira_doc::ImageShadow>),
    /// A line: a stroke between its resolved ends, with its caps (spec 058).
    Line(gafira_doc::LineNode),
    /// A frame: its photo cut out by its shape, and its outline (spec 066).
    Frame(gafira_doc::FrameNode),
    /// A QR code: its background, its parts and its logo (spec 080).
    Qr(Box<gafira_doc::QrNode>),
    /// A table: its cells, their borders and their words (spec 081).
    Table(Box<gafira_doc::TableNode>),
    /// A chart: its shapes and its words (spec 083).
    Chart(Box<gafira_doc::ChartNode>),
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
    /// The size of the page in px, which «Destello de lente» spreads over
    /// (spec 065); without it the flare draws its star alone.
    page_px: Option<(f64, f64)>,
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
            page_px: None,
        }
    }

    /// The builder that knows the size of the page in px (spec 065).
    #[must_use]
    pub fn with_page_px(mut self, width: f64, height: f64) -> Self {
        self.page_px = Some((width, height));
        self
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
        let mut subgraphs: Vec<SubGraph> = Vec::with_capacity(roots.len() + 1);
        // The background of the page, under everything (spec 069).
        if let Some(background) = self.background()? {
            subgraphs.push(background);
        }
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

    /// The background of the page as a subgraph of its own (spec 069): a
    /// colour, a gradient, a pattern or an image over the page box. Needs the
    /// size of the page; without it, or without a background, nothing.
    fn background(&self) -> Result<Option<SubGraph>, GraphError> {
        use gafira_doc::{BackgroundFit, PageBackground};
        let (Some(background), Some((pw, ph))) = (&self.page.background, self.page_px) else {
            return Ok(None);
        };
        let scale = Affine::scale(self.scale);
        let rect = Rect::new(0.0, 0.0, pw, ph);
        let base = ElementBase::new(NodeId::new(0), Placement::new(0.0, 0.0, pw, ph));
        let mut ops = vec![RenderOp::PushLayer];
        match background {
            PageBackground::Solid(color) => {
                check_color(color, "background")?;
                ops.push(RenderOp::FillPath {
                    path: Arc::new(rect.to_path(0.1)),
                    color: *color,
                    transform: scale,
                });
            }
            PageBackground::Gradient(gradient) => ops.push(RenderOp::FillGradient {
                path: Arc::new(rect.to_path(0.1)),
                gradient: Arc::new(gradient.clone()),
                size: (pw, ph),
                transform: scale,
            }),
            PageBackground::Pattern { kind, color, back } => {
                // Vector shapes over the background colour, cut to the page:
                // sharp at any scale (spec 069).
                check_color(color, "background")?;
                check_color(back, "background")?;
                ops.push(RenderOp::FillPath {
                    path: Arc::new(rect.to_path(0.1)),
                    color: *back,
                    transform: scale,
                });
                ops.push(RenderOp::FillPath {
                    path: Arc::new(gafira_raster::pattern_path(
                        *kind,
                        pw,
                        ph,
                        gafira_raster::PATTERN_CELL,
                    )),
                    color: *color,
                    transform: scale,
                });
                ops.push(RenderOp::ClipPath {
                    path: Arc::new(rect.to_path(0.1)),
                    transform: scale,
                });
            }
            PageBackground::Image {
                asset,
                natural,
                fit,
            } => {
                let Some(image) = self.images.get(asset) else {
                    return Ok(None);
                };
                let (nw, nh) = (f64::from(natural.0), f64::from(natural.1));
                // Natural px to the pixels of the decoded image.
                let pixels = Affine::scale_non_uniform(
                    nw / f64::from(image.width().max(1)),
                    nh / f64::from(image.height().max(1)),
                );
                let placed = |x: f64, y: f64, k: f64| {
                    scale * Affine::translate((x, y)) * Affine::scale(k) * pixels
                };
                match fit {
                    BackgroundFit::Fill | BackgroundFit::Fit => {
                        let k = if *fit == BackgroundFit::Fill {
                            (pw / nw).max(ph / nh)
                        } else {
                            (pw / nw).min(ph / nh)
                        };
                        ops.push(RenderOp::DrawImage {
                            image: Arc::clone(image),
                            transform: placed((pw - nw * k) / 2.0, (ph - nh * k) / 2.0, k),
                        });
                    }
                    BackgroundFit::Tile => {
                        // At its own size from the top left, at most 64
                        // across and down.
                        let across = (pw / nw).ceil().clamp(1.0, 64.0) as usize;
                        let down = (ph / nh).ceil().clamp(1.0, 64.0) as usize;
                        for row in 0..down {
                            for column in 0..across {
                                ops.push(RenderOp::DrawImage {
                                    image: Arc::clone(image),
                                    transform: placed(column as f64 * nw, row as f64 * nh, 1.0),
                                });
                            }
                        }
                    }
                }
                // What overflows the page is cut.
                ops.push(RenderOp::ClipPath {
                    path: Arc::new(rect.to_path(0.1)),
                    transform: scale,
                });
            }
        }
        ops.push(RenderOp::PopLayer {
            blend: base.blend,
            opacity: 1.0,
        });
        let key = self.node_key(
            NodeType::Image,
            &[],
            &base,
            scale,
            &[KeyParam::Text(format!(
                "background {background:?} {pw}x{ph}"
            ))],
        );
        let node = GraphNode {
            id: GraphNodeId(0),
            kind: GraphNodeKind::Source(SourceKind::Image),
            origin: NodeId::new(0),
            key,
            inputs: Vec::new(),
            transform: scale,
            blend: base.blend,
            opacity: 1.0,
        };
        Ok(Some(SubGraph {
            nodes: vec![node],
            ops,
        }))
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
                    RecordKind::Shape(ref shape) if shape.effect.is_some() => {
                        // A light draws its picture instead of the shape
                        // (spec 065).
                        let Some((image, drawn)) = shape
                            .effect
                            .as_ref()
                            .and_then(|effect| self.light(effect, &record.base.placement))
                        else {
                            sub.ops.truncate(mark);
                            continue;
                        };
                        check_transform(&drawn)?;
                        let key = self.node_key(
                            NodeType::Image,
                            &[],
                            &record.base,
                            transform,
                            &[KeyParam::Text(format!(
                                "effect {:?} page {:?}",
                                shape.effect, self.page_px
                            ))],
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
                            image: Arc::new(image),
                            transform: drawn,
                        });
                        sub.ops.push(RenderOp::PopLayer {
                            blend: record.base.blend,
                            opacity: record.base.opacity,
                        });
                    }
                    RecordKind::Shape(ref shape) => {
                        let Some(drawn) =
                            DrawnShape::of(shape, &record.base.placement, &self.fonts)?
                        else {
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
                        sub.ops.extend(drawn.ops(transform));
                        sub.ops.push(RenderOp::PopLayer {
                            blend: record.base.blend,
                            opacity: record.base.opacity,
                        });
                    }
                    RecordKind::Line(ref line) => {
                        check_color(&line.color, "line")?;
                        if !line.width.is_finite() {
                            return Err(GraphError::NonFinite { field: "line" });
                        }
                        // The ends are in page px, so the line is drawn with
                        // the scale of the graph alone (spec 058).
                        let ends = line.ends(self.page, &record.base.placement);
                        let layers = line_layers(line, ends);
                        if layers.is_empty() {
                            sub.ops.truncate(mark);
                            continue;
                        }
                        let line_transform = Affine::scale(self.scale);
                        let canonical = format!(
                            "{:?}{:?}{:?}{:016x}",
                            ends,
                            line.start.cap,
                            line.end.cap,
                            line.width.to_bits()
                        );
                        let key = content_key(
                            NodeType::Shape,
                            self.params.lod,
                            &[],
                            &[
                                KeyParam::Quality(self.params.quality),
                                KeyParam::Text(canonical),
                                KeyParam::Fill(Some(line.color)),
                                KeyParam::Transform(line_transform.as_coeffs()),
                                KeyParam::Opacity(record.base.opacity),
                                KeyParam::Blend(record.base.blend),
                            ],
                        );
                        let node = GraphNode {
                            id: GraphNodeId(sub.nodes.len() as u32),
                            kind: GraphNodeKind::Source(SourceKind::Vector),
                            origin: id,
                            key,
                            inputs: Vec::new(),
                            transform: line_transform,
                            blend: record.base.blend,
                            opacity: record.base.opacity,
                        };
                        let node_id = node.id;
                        sub.nodes.push(node);
                        if let Some(frame) = stack.last_mut() {
                            frame.inputs.push(node_id);
                        }
                        for path in layers {
                            sub.ops.push(RenderOp::FillPath {
                                path: Arc::new(path),
                                color: line.color,
                                transform: line_transform,
                            });
                        }
                        sub.ops.push(RenderOp::PopLayer {
                            blend: record.base.blend,
                            opacity: record.base.opacity,
                        });
                    }
                    RecordKind::Text(ref text) => {
                        let Some((path, color, canonical, runs, parts)) =
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
                                KeyParam::Text({
                                    let mut key = canonical;
                                    if let Some(gradient) = &text.gradient {
                                        key = format!("{key}\u{0}{gradient:?}");
                                    }
                                    if !text.effects.is_empty() {
                                        key = format!("{key}\u{0}{:?}", text.effects);
                                    }
                                    key
                                }),
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
                        let placement = &record.base.placement;
                        let size = (placement.w.abs(), placement.h.abs());
                        let (path, runs) = (Arc::new(path), Arc::new(runs));
                        for piece in
                            effect_pieces(text, (&path, color), &runs, density_of(&transform))
                        {
                            // Spec 106: the letters of each colour apart.
                            if matches!(piece, TextPiece::Letters(None))
                                && text.gradient.is_none()
                                && !parts.is_empty()
                            {
                                for (part, part_color, part_runs) in &parts {
                                    sub.ops.push(RenderOp::Text {
                                        path: Arc::new(part.clone()),
                                        color: *part_color,
                                        transform,
                                        runs: Arc::new(part_runs.clone()),
                                    });
                                }
                                continue;
                            }
                            sub.ops.push(match (piece, &text.gradient) {
                                (TextPiece::Fill { path, color }, _) => RenderOp::FillPath {
                                    path: Arc::new(path),
                                    color,
                                    transform,
                                },
                                (TextPiece::Soft { image, at }, _) => RenderOp::DrawImage {
                                    image,
                                    transform: transform * at,
                                },
                                // A gradient fills the letters over the box
                                // (spec 070).
                                (TextPiece::Letters(None), Some(gradient)) => {
                                    RenderOp::FillGradient {
                                        path: Arc::clone(&path),
                                        gradient: Arc::new(gradient.clone()),
                                        size,
                                        transform,
                                    }
                                }
                                // Its glyphs go with it, for the PDF (spec
                                // 073).
                                (TextPiece::Letters(other), _) => RenderOp::Text {
                                    path: Arc::clone(&path),
                                    color: other.unwrap_or(color),
                                    transform,
                                    runs: Arc::clone(&runs),
                                },
                            });
                        }
                        sub.ops.push(RenderOp::PopLayer {
                            blend: record.base.blend,
                            opacity: record.base.opacity,
                        });
                    }
                    RecordKind::Frame(ref frame) => {
                        // The photo, cut out by the shape, then the outline
                        // over the cut edge (spec 066). An empty frame shows
                        // only its outline: it is not exported as a hole
                        // (FD-18 c).
                        let placement = &record.base.placement;
                        let rect = Rect::new(0.0, 0.0, placement.w, placement.h).abs();
                        let radius = match frame.clip_shape {
                            ShapeGeometry::Rect => drawn_radius(frame.corner_radius, rect),
                            ShapeGeometry::Ellipse | ShapeGeometry::Path(_) => 0.0,
                        };
                        let mut edge_shape = ShapeNode::new(frame.clip_shape.clone());
                        edge_shape.stroke = frame.stroke;
                        edge_shape.corner_radius = frame.corner_radius;
                        let edge = DrawnShape::of(&edge_shape, placement, &self.fonts)?;
                        let photo = frame.content.and_then(|content| {
                            let image = self.images.get(&content.asset)?;
                            let at = frame.content_transform(placement.w, placement.h)?;
                            Some((content, image, at))
                        });
                        if photo.is_none() && edge.is_none() {
                            sub.ops.truncate(mark);
                            continue;
                        }
                        let key = self.node_key(
                            NodeType::Image,
                            &[],
                            &record.base,
                            transform,
                            &[KeyParam::Text(format!("frame {frame:?}"))],
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
                        if let Some((content, image, at)) = photo {
                            // The decoded pixels may be smaller than the
                            // natural size (a preview): they span it anyway.
                            let fit = Affine::scale_non_uniform(
                                f64::from(content.natural.0) / f64::from(image.width().max(1)),
                                f64::from(content.natural.1) / f64::from(image.height().max(1)),
                            );
                            let drawn = transform * at * fit;
                            check_transform(&drawn)?;
                            sub.ops.push(RenderOp::DrawImage {
                                image: Arc::clone(image),
                                transform: drawn,
                            });
                            sub.ops.push(RenderOp::ClipPath {
                                path: Arc::new(shape_path(&frame.clip_shape, rect, radius)),
                                transform,
                            });
                        }
                        if let Some(edge) = edge {
                            sub.ops.extend(edge.ops(transform));
                        }
                        sub.ops.push(RenderOp::PopLayer {
                            blend: record.base.blend,
                            opacity: record.base.opacity,
                        });
                    }
                    RecordKind::Qr(ref qr) => {
                        // A code that cannot be made (half written, or too
                        // long) draws nothing, and so does one without area.
                        let placement = &record.base.placement;
                        let ops = gafira_qr::draw(qr).ok().and_then(|drawing| {
                            qr_ops(&drawing, placement, transform, &self.images)
                        });
                        let Some(ops) = ops else {
                            sub.ops.truncate(mark);
                            continue;
                        };
                        let key = self.node_key(
                            NodeType::Shape,
                            &[],
                            &record.base,
                            transform,
                            &[KeyParam::Text(format!("qr {qr:?}"))],
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
                        sub.ops.extend(ops);
                        sub.ops.push(RenderOp::PopLayer {
                            blend: record.base.blend,
                            opacity: record.base.opacity,
                        });
                    }
                    RecordKind::Table(ref table) => {
                        let placement = &record.base.placement;
                        let box_of = (placement.w, placement.h);
                        let ops = table_ops(table, box_of, transform, &self.fonts);
                        if ops.is_empty() {
                            sub.ops.truncate(mark);
                            continue;
                        }
                        let key = self.node_key(
                            NodeType::Shape,
                            &[],
                            &record.base,
                            transform,
                            &[KeyParam::Text(format!("table {box_of:?} {table:?}"))],
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
                        sub.ops.extend(ops);
                        sub.ops.push(RenderOp::PopLayer {
                            blend: record.base.blend,
                            opacity: record.base.opacity,
                        });
                    }
                    RecordKind::Chart(ref chart) => {
                        let placement = &record.base.placement;
                        let box_of = (placement.w, placement.h);
                        let ops = chart_ops(chart, box_of, transform, &self.fonts);
                        if ops.is_empty() {
                            sub.ops.truncate(mark);
                            continue;
                        }
                        let key = self.node_key(
                            NodeType::Shape,
                            &[],
                            &record.base,
                            transform,
                            &[KeyParam::Text(format!("chart {box_of:?} {chart:?}"))],
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
                        sub.ops.extend(ops);
                        sub.ops.push(RenderOp::PopLayer {
                            blend: record.base.blend,
                            opacity: record.base.opacity,
                        });
                    }
                    RecordKind::Image(asset, shadow) => {
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
                        // A shadow reaches past it (spec 085).
                        let size = (image.width(), image.height());
                        let (bx, by) = shadow.map_or((0.0, 0.0), |shadow| {
                            shadow.bleed(size, (placement.w, placement.h))
                        });
                        let fit = Affine::translate((-bx, -by))
                            * Affine::scale_non_uniform(
                                (placement.w + 2.0 * bx) / f64::from(size.0),
                                (placement.h + 2.0 * by) / f64::from(size.1),
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
    /// The picture of a light and the transform that lays it on the page
    /// (spec 065): the lights that span the page cover it, the others the
    /// box of the shape.
    fn light(&self, effect: &ShapeEffect, placement: &Placement) -> Option<(ImageBuf, Affine)> {
        /// The longest side of a light picture, in px: the light is soft,
        /// so a larger one adds time and no detail.
        const MAX_SIDE: f64 = 1024.0;
        if !(placement.w > 0.0 && placement.h > 0.0) {
            return None;
        }
        let fit = |w: f64, h: f64| {
            let k = (self.scale).min(MAX_SIDE / w.max(h));
            (Self::side_of(w * k), Self::side_of(h * k))
        };
        match self.page_px {
            // The flare, the streak and the rays spread over the page
            // (D-104); without its size they keep to the box.
            Some((pw, ph)) if gafira_raster::spans_page(effect) => {
                let (iw, ih) = fit(pw, ph);
                let (kx, ky) = (f64::from(iw) / pw, f64::from(ih) / ph);
                let centre = placement.to_transform()
                    * kurbo::Point::new(placement.w / 2.0, placement.h / 2.0);
                let spot = LightSpot {
                    centre: (centre.x * kx, centre.y * ky),
                    radius: placement.w.min(placement.h) / 2.0 * kx.min(ky),
                };
                let image = gafira_raster::light_picture(effect, iw, ih, spot);
                let drawn =
                    Affine::scale(self.scale) * Affine::scale_non_uniform(1.0 / kx, 1.0 / ky);
                Some((image, drawn))
            }
            _ => {
                let (iw, ih) = fit(placement.w, placement.h);
                let spot = LightSpot {
                    centre: (f64::from(iw) / 2.0, f64::from(ih) / 2.0),
                    radius: f64::from(iw.min(ih)) / 2.0,
                };
                let image = gafira_raster::light_picture(effect, iw, ih, spot);
                let to_box = Affine::scale_non_uniform(
                    placement.w / f64::from(iw),
                    placement.h / f64::from(ih),
                );
                Some((image, self.transform_of(placement) * to_box))
            }
        }
    }

    /// A side of a light picture in whole px, at least one.
    fn side_of(v: f64) -> u32 {
        // Clamped to 1..=1024 by the caller's fit: the cast is exact.
        v.round().clamp(1.0, 4096.0) as u32
    }

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

    fn visit_frame(&mut self, node: &Node, frame: &gafira_doc::FrameNode) -> NodeId {
        self.records.insert(
            node.base.id,
            Record {
                base: node.base.clone(),
                kind: RecordKind::Frame(frame.clone()),
            },
        );
        node.base.id
    }

    fn visit_qr(&mut self, node: &Node, qr: &gafira_doc::QrNode) -> NodeId {
        self.records.insert(
            node.base.id,
            Record {
                base: node.base.clone(),
                kind: RecordKind::Qr(Box::new(qr.clone())),
            },
        );
        node.base.id
    }

    fn visit_table(&mut self, node: &Node, table: &gafira_doc::TableNode) -> NodeId {
        self.records.insert(
            node.base.id,
            Record {
                base: node.base.clone(),
                kind: RecordKind::Table(Box::new(table.clone())),
            },
        );
        node.base.id
    }

    fn visit_chart(&mut self, node: &Node, chart: &gafira_doc::ChartNode) -> NodeId {
        self.records.insert(
            node.base.id,
            Record {
                base: node.base.clone(),
                kind: RecordKind::Chart(Box::new(chart.clone())),
            },
        );
        node.base.id
    }

    fn visit_line(&mut self, node: &Node, line: &gafira_doc::LineNode) -> NodeId {
        self.records.insert(
            node.base.id,
            Record {
                base: node.base.clone(),
                kind: RecordKind::Line(line.clone()),
            },
        );
        node.base.id
    }

    fn visit_image(&mut self, node: &Node, image: &ImageNode) -> NodeId {
        self.records.insert(
            node.base.id,
            Record {
                base: node.base.clone(),
                kind: RecordKind::Image(
                    image.shown(),
                    image.look.as_ref().and_then(|look| look.shadow),
                ),
            },
        );
        node.base.id
    }

    /// A 3D model draws the picture its camera sees, kept with the images (spec 100):
    /// like an image without a look, stretched to its box.
    fn visit_model3d(&mut self, node: &Node, model: &gafira_doc::Model3dNode) -> NodeId {
        self.records.insert(
            node.base.id,
            Record {
                base: node.base.clone(),
                kind: RecordKind::Image(model.shown(), None),
            },
        );
        node.base.id
    }

    fn visit_extension(&mut self, node: &Node, _extension: &ExtensionNode) -> NodeId {
        // The seam of ARQ-137 is open: an extension draws nothing in this cut.
        node.base.id
    }
}

/// The operations of a table in a box of `w` × `h` px (spec 081): the fills
/// of the cells, the borders over them and the words, each cell a text of
/// its own, so a PDF keeps it as text (S81-12).
fn table_ops(
    table: &gafira_doc::TableNode,
    (w, h): (f64, f64),
    transform: Affine,
    fonts: &FontBook,
) -> Vec<RenderOp> {
    if !(w > 0.0 && h > 0.0) {
        return Vec::new();
    }
    let layout = gafira_table::Layout::of(table, (w, h));
    let mut ops = Vec::new();
    let fill = |rect: Rect, color: Color| RenderOp::FillPath {
        path: Arc::new(rect.to_path(SHAPE_TOLERANCE)),
        color,
        transform,
    };
    for (r, c) in gafira_table::anchors(table) {
        if let Some(color) = table.cell(r, c).and_then(|cell| cell.style.fill) {
            ops.push(fill(layout.cell(table, r, c), color));
        }
    }
    for edge in gafira_table::edges(table, &layout) {
        // Half a width more at each end closes the corners.
        let half = edge.stroke.width / 2.0;
        let rect = Rect::from_points(edge.a, edge.b).inflate(half, half);
        ops.push(fill(rect, edge.stroke.color));
    }
    for (r, c) in gafira_table::anchors(table) {
        let Some(cell) = table.cell(r, c) else {
            continue;
        };
        let rect = layout.cell(table, r, c);
        if let Some(text) = cell_text(cell, (rect.width(), rect.height()), table.padding, fonts) {
            let at = rect.origin().to_vec2() + text.offset;
            ops.push(RenderOp::Text {
                path: Arc::new(text.path),
                color: cell.style.color,
                transform: transform * Affine::translate(at),
                runs: Arc::new(text.runs),
            });
        }
    }
    ops
}

/// A line of words of a chart laid out, once for all the times it is asked
/// for: its measure, then its drawing.
type ChartWords = BTreeMap<(String, u64, bool), Option<(LaidText, f64)>>;

/// The pieces of `chart` in a box of `w` × `h` px with their words laid out
/// (spec 083).
fn chart_pieces(
    chart: &gafira_doc::ChartNode,
    size: (f64, f64),
    fonts: &FontBook,
) -> Vec<(gafira_chart::Piece, Option<LaidText>, kurbo::Vec2)> {
    use gafira_chart::{Anchor, Piece};
    // Wider than any line: a label does not wrap.
    const WIDE: f64 = 1e5;
    let laid = std::cell::RefCell::new(ChartWords::new());
    let lay = |text: &str, size: f64, bold: bool| {
        let key = (text.to_owned(), size.to_bits(), bold);
        if let Some(found) = laid.borrow().get(&key) {
            return found.clone();
        }
        let style = gafira_doc::SpanStyle {
            font: chart.text.font.clone(),
            size,
            color: chart.text.color,
            bold,
            italic: chart.text.italic,
        };
        let content = gafira_doc::RichText::plain(text, &style);
        let wide = gafira_geom::Size::new(WIDE, 1.0);
        let found = laid_text(&content, Laid::plain(TextAlign::Left), wide, fonts).map(|text| {
            let width = text.path.bounding_box().x1.max(0.0);
            (text, width)
        });
        laid.borrow_mut().insert(key, found.clone());
        found
    };
    let measure = |text: &str, size: f64, bold: bool| {
        lay(text, size, bold).map_or((0.0, size), |(text, width)| (width, text.height))
    };
    gafira_chart::draw(chart, size, &measure)
        .into_iter()
        .map(|piece| {
            let Piece::Words(words) = &piece else {
                return (piece, None, kurbo::Vec2::ZERO);
            };
            let Some((text, width)) = lay(&words.text, words.size, words.bold) else {
                return (piece, None, kurbo::Vec2::ZERO);
            };
            let x = match words.h {
                Anchor::Start => 0.0,
                Anchor::Middle => width / 2.0,
                Anchor::End => width,
            };
            let y = match words.v {
                Anchor::Start => 0.0,
                Anchor::Middle => text.height / 2.0,
                Anchor::End => text.height,
            };
            let at = words.at.to_vec2() - kurbo::Vec2::new(x, y);
            (piece, Some(text), at)
        })
        .collect()
}

/// The operations of a chart in a box of `w` × `h` px, back to front (spec
/// 083): its words stay words.
fn chart_ops(
    chart: &gafira_doc::ChartNode,
    size: (f64, f64),
    transform: Affine,
    fonts: &FontBook,
) -> Vec<RenderOp> {
    chart_pieces(chart, size, fonts)
        .into_iter()
        .filter_map(|(piece, text, at)| match piece {
            gafira_chart::Piece::Fill { path, color } => Some(RenderOp::FillPath {
                path: Arc::new(path),
                color,
                transform,
            }),
            gafira_chart::Piece::Words(words) => text.map(|text| RenderOp::Text {
                path: Arc::new(text.path),
                color: words.color,
                transform: transform * Affine::translate(at),
                runs: Arc::new(text.runs),
            }),
        })
        .collect()
}

/// The outlines that draw `chart` in a box of `w` × `h` px and their
/// colours, back to front, as the export draws them (spec 083).
#[must_use]
pub fn chart_outlines(
    chart: &gafira_doc::ChartNode,
    size: (f64, f64),
    fonts: &FontBook,
) -> Vec<(BezPath, Color)> {
    chart_pieces(chart, size, fonts)
        .into_iter()
        .filter_map(|(piece, text, at)| match piece {
            gafira_chart::Piece::Fill { path, color } => Some((path, color)),
            gafira_chart::Piece::Words(words) => {
                text.map(|text| (Affine::translate(at) * text.path, words.color))
            }
        })
        .collect()
}

/// The words of a cell laid out: their outline, where it goes in the cell
/// and their height.
struct CellText {
    path: BezPath,
    offset: kurbo::Vec2,
    height: f64,
    runs: Vec<GlyphRun>,
}

/// The words of `cell` in a box of `w` × `h` px less its `padding`.
fn cell_text(
    cell: &gafira_doc::TableCell,
    (w, h): (f64, f64),
    padding: f64,
    fonts: &FontBook,
) -> Option<CellText> {
    let style = &cell.style;
    let content = gafira_doc::RichText::plain(
        &cell.text,
        &gafira_doc::SpanStyle {
            font: style.font.clone(),
            size: style.size,
            color: style.color,
            bold: style.bold,
            italic: style.italic,
        },
    );
    let pad_x = padding.min(w / 2.0 - 0.5).max(0.0);
    let inner = gafira_geom::Size::new((w - 2.0 * pad_x).max(1.0), h.max(1.0));
    let laid = laid_text(&content, Laid::plain(style.align), inner, fonts)?;
    let y = match style.valign {
        gafira_doc::VerticalAlign::Top => padding,
        gafira_doc::VerticalAlign::Middle => (h - laid.height) / 2.0,
        gafira_doc::VerticalAlign::Bottom => h - padding - laid.height,
    };
    Some(CellText {
        path: laid.path,
        offset: kurbo::Vec2::new(pad_x, y),
        height: laid.height,
        runs: laid.runs,
    })
}

/// The outline of the words of `cell` in its box, from its top left corner,
/// as the export draws it, and the height of their lines (spec 081).
#[must_use]
pub fn cell_outline(
    cell: &gafira_doc::TableCell,
    size: (f64, f64),
    padding: f64,
    fonts: &FontBook,
) -> Option<(BezPath, f64)> {
    let text = cell_text(cell, size, padding, fonts)?;
    Some((Affine::translate(text.offset) * text.path, text.height))
}

/// The operations of a QR code in the box of `placement`, back to front
/// (spec 080): each part is one path, in px of the box, and a gradient spans
/// the code without its margin. `None` for a box without area.
fn qr_ops(
    drawing: &gafira_qr::QrDrawing,
    placement: &gafira_doc::Placement,
    transform: Affine,
    images: &ImagePixels,
) -> Option<Vec<RenderOp>> {
    let (k, corner) = drawing.fit(placement.w, placement.h)?;
    let code = transform * Affine::translate(corner);
    let side = drawing.size * k;
    let mut ops = Vec::with_capacity(drawing.layers.len() + 2);
    if let Some(color) = drawing.background {
        let margin = drawing.margin * k;
        let back = Rect::new(-margin, -margin, side + margin, side + margin);
        ops.push(RenderOp::FillPath {
            path: Arc::new(back.to_path(SHAPE_TOLERANCE)),
            color,
            transform: code,
        });
    }
    for layer in &drawing.layers {
        let path = Arc::new(Affine::scale(k) * &layer.path);
        ops.push(match &layer.fill {
            gafira_doc::QrFill::Solid(color) => RenderOp::FillPath {
                path,
                color: *color,
                transform: code,
            },
            gafira_doc::QrFill::Gradient(gradient) => RenderOp::FillGradient {
                path,
                gradient: Arc::new(gradient.clone()),
                size: (side, side),
                transform: code,
            },
        });
    }
    if let Some(logo) = drawing.logo
        && let Some(image) = images.get(&logo.asset)
    {
        let rect = logo.rect;
        let fit = Affine::translate((rect.x0 * k, rect.y0 * k))
            * Affine::scale_non_uniform(
                rect.width() * k / f64::from(image.width().max(1)),
                rect.height() * k / f64::from(image.height().max(1)),
            );
        ops.push(RenderOp::DrawImage {
            image: Arc::clone(image),
            transform: code * fit,
        });
    }
    Some(ops)
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
    /// The text inside the shape, over the fill and the outline, with the
    /// canonical string of its key (spec 055).
    text: Option<(BezPath, Color, String)>,
    /// The gradient that fills the outline instead of the colour, laid in
    /// the shape's box (spec 063).
    gradient: Option<(gafira_doc::Gradient, (f64, f64))>,
    /// The radius the rectangle is drawn with; 0 for an ellipse.
    radius: f64,
}

impl DrawnShape {
    /// What `shape` draws in the box of `placement`, `None` when it draws
    /// nothing; a number that is not finite is an error (ARQ-110).
    fn of(
        shape: &ShapeNode,
        placement: &Placement,
        fonts: &FontBook,
    ) -> Result<Option<Self>, GraphError> {
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
        // A gradient fills the outline in the colour's place (spec 063).
        let gradient = shape
            .gradient
            .clone()
            .map(|gradient| (gradient, (rect.width(), rect.height())));
        let fill = match &gradient {
            Some(_) => Some((
                outline,
                Color::Srgb(gafira_color::Rgba::new(0.0, 0.0, 0.0, 0.0)),
            )),
            None => shape.fill.map(|color| (outline, color)),
        };
        let text = match &shape.text {
            Some(text) => shape_text_path(text, rect, fonts)
                .map(|(path, color, canonical)| {
                    check_color(&color, "fill").map(|()| (path, color, canonical))
                })
                .transpose()?,
            None => None,
        };
        Ok(
            (fill.is_some() || stroke.is_some() || text.is_some()).then_some(Self {
                fill,
                stroke,
                text,
                gradient,
                radius,
            }),
        )
    }

    /// The parameters of the content key that this shape draws. A shape with
    /// only a fill and square corners gives the parameters it had before
    /// spec 038, so its key does not move.
    fn key_params(&self, geometry: &ShapeGeometry) -> Vec<KeyParam> {
        let mut params = vec![
            KeyParam::Geometry(geometry.clone()),
            KeyParam::Fill(self.fill.as_ref().map(|(_, color)| *color)),
        ];
        if let Some((gradient, _)) = &self.gradient {
            // Every number of the gradient, in a canonical string.
            params.push(KeyParam::Text(format!("gradient {gradient:?}")));
        }
        if let Some((_, color, width)) = &self.stroke {
            params.push(KeyParam::Stroke(*color, *width));
        }
        if self.radius > 0.0 {
            params.push(KeyParam::CornerRadius(self.radius));
        }
        if let Some((_, color, canonical)) = &self.text {
            params.push(KeyParam::Text(canonical.clone()));
            params.push(KeyParam::Fill(Some(*color)));
        }
        params
    }

    /// The operations of the shape in the frame of `transform`, back to
    /// front: the fill (a gradient when it has one), the outline, the text.
    fn ops(self, transform: Affine) -> Vec<RenderOp> {
        let mut out = Vec::new();
        if let Some((path, color)) = self.fill {
            out.push(match self.gradient {
                Some((gradient, size)) => RenderOp::FillGradient {
                    path: Arc::new(path),
                    gradient: Arc::new(gradient),
                    size,
                    transform,
                },
                None => RenderOp::FillPath {
                    path: Arc::new(path),
                    color,
                    transform,
                },
            });
        }
        for (path, color) in self
            .stroke
            .map(|(path, color, _)| (path, color))
            .into_iter()
            .chain(self.text.map(|(path, color, _)| (path, color)))
        {
            out.push(RenderOp::FillPath {
                path: Arc::new(path),
                color,
                transform,
            });
        }
        out
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

/// The areas a line covers (spec 058), in page px: its stroke between its
/// ends, cut back where a hollow cap sits, and each cap. Every area is
/// filled on its own, so their windings never cancel. Nothing for a line of
/// width 0 or with both ends on one point.
fn line_layers(line: &gafira_doc::LineNode, [a, b]: [kurbo::Point; 2]) -> Vec<BezPath> {
    use gafira_doc::LineCap;
    let width = line.width;
    let length = (b - a).hypot();
    if width <= 0.0 || length <= f64::EPSILON {
        return Vec::new();
    }
    let dir = (b - a) / length;
    let size = (width * 2.5).max(5.0);
    // How far a cap reaches back along the line from its end.
    let reach = |cap: LineCap| match cap {
        LineCap::None | LineCap::Arrow | LineCap::Bar => 0.0,
        LineCap::Circle
        | LineCap::CircleFilled
        | LineCap::Diamond
        | LineCap::DiamondFilled
        | LineCap::Square
        | LineCap::SquareFilled => size,
    };
    let (ra, rb) = (reach(line.start.cap), reach(line.end.cap));
    let mut out = Vec::new();
    let stroke = |path: &BezPath| stroke_path(path, width);
    if ra + rb < length {
        let (from, to) = (a + dir * ra, b - dir * rb);
        for (s, e) in gafira_doc::line_pieces(line.dash, width, (to - from).hypot()) {
            if line.dash == gafira_doc::LineDash::Dotted {
                let centre = from + dir * ((s + e) / 2.0);
                out.push(kurbo::Circle::new(centre, width / 2.0).to_path(SHAPE_TOLERANCE));
            } else {
                let mut body = BezPath::new();
                body.move_to(from + dir * s);
                body.line_to(from + dir * e);
                out.extend(stroke(&body));
            }
        }
    }
    for (at, cap, out_dir) in [(a, line.start.cap, -dir), (b, line.end.cap, dir)] {
        let normal = kurbo::Vec2::new(-out_dir.y, out_dir.x);
        match cap {
            LineCap::None => {}
            LineCap::Arrow => {
                let back = at - out_dir * size * 1.6;
                let mut arrow = BezPath::new();
                arrow.move_to(back + normal * size);
                arrow.line_to(at);
                arrow.line_to(back - normal * size);
                out.extend(stroke(&arrow));
            }
            LineCap::Circle | LineCap::CircleFilled => {
                let centre = at - out_dir * size;
                let circle = kurbo::Circle::new(centre, size).to_path(SHAPE_TOLERANCE);
                if cap == LineCap::CircleFilled {
                    out.push(circle.clone());
                }
                out.extend(stroke(&circle));
            }
            LineCap::Bar => {
                let mut bar = BezPath::new();
                bar.move_to(at + normal * size);
                bar.line_to(at - normal * size);
                out.extend(stroke(&bar));
            }
            LineCap::Diamond | LineCap::DiamondFilled => {
                let centre = at - out_dir * size;
                let mut diamond = BezPath::new();
                diamond.move_to(at);
                diamond.line_to(centre + normal * size);
                diamond.line_to(centre - out_dir * size);
                diamond.line_to(centre - normal * size);
                diamond.close_path();
                if cap == LineCap::DiamondFilled {
                    out.push(diamond.clone());
                }
                out.extend(stroke(&diamond));
            }
            LineCap::Square | LineCap::SquareFilled => {
                let centre = at - out_dir * size;
                let half = size * 0.8;
                let mut square = BezPath::new();
                for (i, (u, v)) in [(1.0, 1.0), (1.0, -1.0), (-1.0, -1.0), (-1.0, 1.0)]
                    .into_iter()
                    .enumerate()
                {
                    let p = centre + out_dir * (half * u) + normal * (half * v);
                    if i == 0 {
                        square.move_to(p);
                    } else {
                        square.line_to(p);
                    }
                }
                square.close_path();
                if cap == LineCap::SquareFilled {
                    out.push(square.clone());
                }
                out.extend(stroke(&square));
            }
        }
    }
    out
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
#[allow(clippy::type_complexity)]
fn text_path(
    text: &TextNode,
    placement: &Placement,
    fonts: &FontBook,
) -> Option<(
    BezPath,
    Color,
    String,
    Vec<GlyphRun>,
    Vec<(BezPath, Color, Vec<GlyphRun>)>,
)> {
    let bounds = gafira_geom::Size::new(placement.w.abs(), placement.h.abs());
    let laid = laid_text(&text.content, Laid::of(text), bounds, fonts)?;
    Some((laid.path, laid.color, laid.canonical, laid.runs, laid.parts))
}

/// A text laid out: its outline, colour, the canonical string of its key,
/// the height of its lines and its glyph runs (spec 073; none for a curved
/// text, whose letters turn one by one).
#[derive(Clone)]
struct LaidText {
    path: BezPath,
    color: Color,
    canonical: String,
    height: f64,
    runs: Vec<GlyphRun>,
    /// A text of several styles (spec 106): its letters by colour, with
    /// their glyph runs; empty for a text of one style, all in `color`.
    parts: Vec<(BezPath, Color, Vec<GlyphRun>)>,
}

/// How a text is laid out besides its words: alignment, bend, letter
/// spacing (thousandths of an em) and line height (FD-41, FD-44).
#[derive(Clone, Copy)]
struct Laid {
    align: TextAlign,
    curve: f64,
    letter_spacing: f64,
    line_height: Option<f64>,
}

impl Laid {
    fn of(text: &TextNode) -> Self {
        Self {
            align: text.align,
            curve: text.curve,
            letter_spacing: text.letter_spacing,
            line_height: text.line_height,
        }
    }

    fn plain(align: TextAlign) -> Self {
        Self {
            align,
            curve: 0.0,
            letter_spacing: 0.0,
            line_height: None,
        }
    }
}

/// The outline of `text` in a box `width` × `height` px, from its top-left
/// corner, its colour and the height of its lines: what the export draws,
/// for the canvas to draw a curved or spaced text the same way.
#[must_use]
pub fn text_outline(
    text: &TextNode,
    (width, height): (f64, f64),
    fonts: &FontBook,
) -> Option<(BezPath, Color, f64)> {
    let bounds = gafira_geom::Size::new(width.abs(), height.abs());
    laid_text(&text.content, Laid::of(text), bounds, fonts)
        .map(|laid| (laid.path, laid.color, laid.height))
}

/// How many pixels a unit is seen at.
fn density_of(transform: &Affine) -> f64 {
    let [a, b, c, d, _, _] = transform.as_coeffs();
    a.hypot(b).max(c.hypot(d))
}

/// The pieces of a text laid out, with its effects (spec 089).
fn effect_pieces(
    text: &TextNode,
    (path, color): (&BezPath, Color),
    runs: &[GlyphRun],
    density: f64,
) -> Vec<TextPiece> {
    if text.effects.is_empty() {
        return vec![TextPiece::Letters(None)];
    }
    let spans = text.content.paragraphs.iter().flat_map(|p| &p.spans);
    let em = spans.map(|span| span.style.size).fold(0.0, f64::max);
    let lines: Vec<(f64, f64)> = runs
        .iter()
        .filter_map(|run| Some((f64::from(run.glyphs.first()?.y), f64::from(run.size))))
        .collect();
    let lines = text_effects::line_boxes(&lines, path);
    text_effects::pieces(&text.effects, (path, color), &lines, em, density)
}

/// The pieces of `text` in a box `width` × `height` px, seen at `density`
/// pixels a unit: what the export draws, for the canvas to draw the same.
#[must_use]
pub fn text_pieces(
    text: &TextNode,
    (width, height): (f64, f64),
    fonts: &FontBook,
    density: f64,
) -> Option<Vec<TextPiece>> {
    let bounds = gafira_geom::Size::new(width.abs(), height.abs());
    let laid = laid_text(&text.content, Laid::of(text), bounds, fonts)?;
    let letters = (&laid.path, laid.color);
    Some(effect_pieces(text, letters, &laid.runs, density))
}

/// The text of a shape (spec 055) in the shape's box `rect`: laid out in
/// the box less a tenth of its shorter side on each side, aligned across by
/// its `align` and down by its `valign`. With its colour and the canonical
/// string of its key, which names the vertical alignment too.
fn shape_text_path(
    text: &gafira_doc::ShapeText,
    rect: Rect,
    fonts: &FontBook,
) -> Option<(BezPath, Color, String)> {
    let pad = rect.width().min(rect.height()) / 10.0;
    let inner = gafira_geom::Size::new(
        (rect.width() - 2.0 * pad).max(1.0),
        (rect.height() - 2.0 * pad).max(1.0),
    );
    let LaidText {
        mut path,
        color,
        canonical,
        height,
        ..
    } = laid_text(&text.content, Laid::plain(text.align), inner, fonts)?;
    let (share, name) = match text.valign {
        gafira_doc::VerticalAlign::Top => (0.0, 't'),
        gafira_doc::VerticalAlign::Middle => (0.5, 'm'),
        gafira_doc::VerticalAlign::Bottom => (1.0, 'b'),
    };
    let dy = rect.y0 + pad + (inner.height() - height) * share;
    path.apply_affine(Affine::translate((rect.x0 + pad, dy)));
    Some((path, color, format!("{canonical}\u{0}{name}")))
}

/// `content` laid out in a box of `bounds` with `align`: its outline from
/// the box's top-left corner, its colour, the canonical string of its key
/// and the height of its lines; `None` when it draws nothing. With a `curve`
/// other than 0 each letter sits on the arc of its line (FD-44).
fn laid_text(
    content: &gafira_doc::RichText,
    laid: Laid,
    bounds: gafira_geom::Size,
    fonts: &FontBook,
) -> Option<LaidText> {
    let Laid {
        align,
        curve,
        letter_spacing,
        line_height,
    } = laid;
    let style = content.first_style()?;
    let words = content.plain_text();
    if words.trim().is_empty() || !style.size.is_finite() || style.size <= 0.0 {
        return None;
    }
    let mixed = content.mixed();
    let face = fonts.face(&style.font, style.bold, style.italic);
    let mut owned = fonts.stack(&face);
    // The spans by their bytes in `words`, for a text of several styles.
    let spans = if mixed {
        span_ranges(content)
    } else {
        Vec::new()
    };
    let stack: Vec<&[u8]> = owned.iter().map(AsRef::as_ref).collect();
    if stack.is_empty() {
        return None;
    }
    let size_pt = style.size.clamp(*TEXT_SIZE_PT.start(), *TEXT_SIZE_PT.end());
    let layout_style = gafira_text::TextStyle {
        size_pt,
        align: match align {
            TextAlign::Left => gafira_text::TextAlign::Left,
            TextAlign::Center => gafira_text::TextAlign::Center,
            TextAlign::Right => gafira_text::TextAlign::Right,
            TextAlign::Justify => gafira_text::TextAlign::Justify,
        },
        letter_spacing,
        line_height: line_height.unwrap_or(gafira_text::TextStyle::default().line_height),
        ..gafira_text::TextStyle::default()
    };
    let dpi = 72.0 * style.size / size_pt;
    let layout = if mixed {
        // Spec 106: every span in its own face and size; the faces of the
        // stack are those of every span, then the others of the book.
        let mut runs = Vec::with_capacity(spans.len());
        for (range, span) in &spans {
            let key = fonts.face(&span.font, span.bold, span.italic);
            let lead = fonts.stack(&key).into_iter().next()?;
            let at = match owned.iter().position(|f| Arc::ptr_eq(f, &lead)) {
                Some(at) => at,
                None => {
                    owned.push(lead);
                    owned.len() - 1
                }
            };
            if !span.size.is_finite() || span.size <= 0.0 {
                return None;
            }
            runs.push(gafira_text::FaceRun {
                range: range.clone(),
                size_px: span.size as f32,
                lead: at,
                bold: span.bold,
                italic: span.italic,
            });
        }
        let stack: Vec<&[u8]> = owned.iter().map(AsRef::as_ref).collect();
        gafira_text::layout_box_runs(&words, &stack, &layout_style, dpi, bounds, &runs).ok()?
    } else {
        fonts.lay(&face, &stack, |stack| {
            stack
                .layout_box_face(
                    &words,
                    &layout_style,
                    (style.bold, style.italic),
                    dpi,
                    bounds,
                )
                .ok()
        })?
    };
    let stack: Vec<&[u8]> = owned.iter().map(AsRef::as_ref).collect();
    let outlines = gafira_text::outline_text(&layout, &stack).ok()?;
    let curved = if curve == 0.0 {
        None
    } else {
        Some(gafira_text::curve_layout(&layout, bounds.width(), curve).ok()?)
    };
    let mut path = BezPath::new();
    // Spec 106: the letters of each colour, by the span of their cluster.
    let mut by_color: Vec<(Color, BezPath)> = Vec::new();
    let color_at = |line: usize, cluster: usize| -> Option<Color> {
        let at = layout.lines.get(line)?.clusters.get(cluster)?.range.start;
        spans
            .iter()
            .find(|(range, _)| range.contains(&at))
            .map(|(_, span)| span.color)
    };
    // The top of the straight letters: a curved text starts there too, so a
    // cup, whose ends rise, stays in its box.
    let mut straight_top = f64::MAX;
    for outline in outlines {
        let elements = outline.path.as_bez_path().elements().to_vec();
        if curved.is_some() {
            use kurbo::Shape as _;
            let glyph = BezPath::from_vec(elements.clone());
            if !glyph.elements().is_empty() {
                straight_top = straight_top.min(glyph.bounding_box().y0);
            }
        }
        let mut glyph = BezPath::from_vec(elements);
        if let Some(lines) = &curved
            && let Some(line) = layout.lines.get(outline.line)
            && let Some(cluster) = line.clusters.get(outline.cluster)
            && let Some(placed) = lines
                .get(outline.line)
                .and_then(|l| l.clusters.get(outline.cluster))
        {
            // From its place on the straight line to its place on the arc,
            // turned with it.
            let centre = f64::from(cluster.x) + f64::from(cluster.advance) / 2.0;
            glyph.apply_affine(
                Affine::translate((placed.x, placed.y))
                    * Affine::rotate(placed.rotation)
                    * Affine::translate((-centre, -f64::from(line.baseline))),
            );
        }
        if mixed && let Some(color) = color_at(outline.line, outline.cluster) {
            match by_color.iter_mut().find(|(c, _)| *c == color) {
                Some((_, part)) => part.extend(glyph.elements().iter().copied()),
                None => by_color.push((color, glyph.clone())),
            }
        }
        path.extend(glyph.elements().iter().copied());
    }
    if path.elements().is_empty() {
        return None;
    }
    if curved.is_some() && straight_top < f64::MAX {
        use kurbo::Shape as _;
        let top = path.bounding_box().y0;
        path.apply_affine(Affine::translate((0.0, straight_top - top)));
    }
    let align = match align {
        TextAlign::Left => 'l',
        TextAlign::Center => 'c',
        TextAlign::Right => 'r',
        TextAlign::Justify => 'j',
    };
    let styles = if mixed {
        format!(
            "\u{0}{:?}",
            spans.iter().map(|(r, s)| (r, s)).collect::<Vec<_>>()
        )
    } else {
        String::new()
    };
    let canonical = format!(
        "{}\u{0}{:016x}\u{0}{align}{}\u{0}{words}{styles}",
        face,
        style.size.to_bits(),
        if curve == 0.0 && letter_spacing == 0.0 && line_height.is_none() {
            String::new()
        } else {
            format!(
                "~{:016x}~{:016x}~{:?}",
                curve.to_bits(),
                letter_spacing.to_bits(),
                line_height.map(f64::to_bits)
            )
        }
    );
    // The glyphs by line and font, where the outline put them (spec 073).
    let text: Arc<str> = Arc::from(words.as_str());
    let mut runs: Vec<GlyphRun> = Vec::new();
    // The colour of each run, for a text of several styles.
    let mut run_colors: Vec<Option<Color>> = Vec::new();
    if curved.is_none() {
        for line in &layout.lines {
            let mut current: Option<GlyphRun> = None;
            for cluster in &line.clusters {
                let Some(font) = owned.get(cluster.font) else {
                    continue;
                };
                let color = if mixed {
                    spans
                        .iter()
                        .find(|(range, _)| range.contains(&cluster.range.start))
                        .map(|(_, span)| span.color)
                } else {
                    None
                };
                let same = current.as_ref().is_some_and(|run| {
                    Arc::ptr_eq(&run.font, font)
                        && run.size == cluster.size_px
                        && run_colors.last().copied().flatten() == color
                });
                if !same {
                    runs.extend(current.take());
                    run_colors.push(color);
                    current = Some(GlyphRun {
                        font: Arc::clone(font),
                        index: 0,
                        size: cluster.size_px,
                        glyphs: Vec::new(),
                        text: Arc::clone(&text),
                    });
                }
                if let Some(run) = current.as_mut() {
                    for (k, (id, (dx, dy))) in cluster
                        .glyphs
                        .iter()
                        .zip(&cluster.glyph_offsets)
                        .enumerate()
                    {
                        // The words a cluster draws go with its first glyph.
                        let range = if k == 0 {
                            cluster.range.clone()
                        } else {
                            cluster.range.end..cluster.range.end
                        };
                        run.glyphs.push(RunGlyph {
                            id: *id,
                            x: cluster.x + dx,
                            y: line.baseline + dy,
                            text: range,
                        });
                    }
                }
            }
            runs.extend(current);
        }
    }
    let parts = by_color
        .into_iter()
        .map(|(color, part)| {
            let own: Vec<GlyphRun> = runs
                .iter()
                .zip(&run_colors)
                .filter(|(_, c)| **c == Some(color))
                .map(|(run, _)| run.clone())
                .collect();
            (part, color, own)
        })
        .collect();
    Some(LaidText {
        path,
        color: style.color,
        canonical,
        height: f64::from(layout.height),
        runs,
        parts,
    })
}

/// The bytes of `content.plain_text()` each span takes, with its style;
/// paragraph breaks belong to no span.
fn span_ranges(
    content: &gafira_doc::RichText,
) -> Vec<(std::ops::Range<usize>, gafira_doc::SpanStyle)> {
    let mut out = Vec::new();
    let mut at = 0;
    for (k, paragraph) in content.paragraphs.iter().enumerate() {
        if k > 0 {
            at += 1;
        }
        for span in &paragraph.spans {
            let end = at + span.text.len();
            if end > at {
                out.push((at..end, span.style.clone()));
            }
            at = end;
        }
    }
    out
}

/// The letters of `text` by colour (spec 106), laid out in a box of
/// `width` × `height` px: one part for a text of one style.
#[must_use]
pub fn text_parts(
    text: &TextNode,
    (width, height): (f64, f64),
    fonts: &FontBook,
) -> Option<Vec<(BezPath, Color)>> {
    let bounds = gafira_geom::Size::new(width.abs(), height.abs());
    let laid = laid_text(&text.content, Laid::of(text), bounds, fonts)?;
    if laid.parts.is_empty() {
        return Some(vec![(laid.path, laid.color)]);
    }
    Some(laid.parts.into_iter().map(|(p, c, _)| (p, c)).collect())
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
            text: None,
            gradient: None,
            effect: None,
            drawn_with: None,
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
            // Text draws as its outline (spec 073).
            RenderOp::Text { .. } => "fill",
            RenderOp::FillGradient { .. } => "gradient",
            RenderOp::DrawImage { .. } => "image",
            RenderOp::ClipPath { .. } => "clip",
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

    // S58-03: a line is drawn between its resolved ends: connected to the
    // right point of a shape, its stroke reaches that point, and moving the
    // shape moves the drawn line with it.
    #[test]
    fn s58_03_a_connected_line_follows_its_shape() {
        use gafira_doc::{Anchor, Attachment, LineCap, LineEnd, LineNode};
        let page_with = |shape_x: f64| {
            let mut page = new_page();
            let mut shape = ShapeNode::new(ShapeGeometry::Rect);
            shape.fill = Some(RED);
            let base =
                ElementBase::new(NodeId::new(1), Placement::new(shape_x, 100.0, 100.0, 50.0));
            page.nodes.insert(
                NodeId::new(1),
                Arc::new(Node::new(base, NodeKind::Shape(shape))),
            );
            let mut end = LineEnd::new([1.0, 1.0], LineCap::None);
            end.attached = Some(Attachment::anchor(NodeId::new(1), Anchor::Left));
            let line = LineNode {
                start: LineEnd::new([0.0, 0.0], LineCap::None),
                end,
                color: Color::Srgb(Rgba::new(0.0, 0.0, 0.0, 1.0)),
                width: 4.0,
                dash: Default::default(),
            };
            let base = ElementBase::new(NodeId::new(2), Placement::new(10.0, 10.0, 20.0, 20.0));
            page.nodes.insert(
                NodeId::new(2),
                Arc::new(Node::new(base, NodeKind::Line(line))),
            );
            page.roots = vec![NodeId::new(1), NodeId::new(2)];
            page
        };
        let line_box = |shape_x: f64| {
            let graph = build(&page_with(shape_x), WalkOrder::PreOrder);
            let all = fills(&graph);
            assert_eq!(all.len(), 2, "the shape, then the line");
            all[1].0
        };
        let near = line_box(200.0);
        // From (10, 10) to the left point of the shape, (200, 125).
        assert!(
            (near.x0 - 10.0).abs() < 3.0 && (near.y0 - 10.0).abs() < 3.0,
            "{near:?}"
        );
        assert!(
            (near.x1 - 200.0).abs() < 3.0 && (near.y1 - 125.0).abs() < 3.0,
            "{near:?}"
        );
        let far = line_box(300.0);
        assert!((far.x1 - 300.0).abs() < 3.0, "the line follows: {far:?}");
    }

    // S96-03: a dashed line is drawn in dashes and a dotted one in dots;
    // a filled circle or square covers its middle, and a bar crosses the
    // end.
    #[test]
    fn s96_03_dashes_dots_and_new_caps() {
        use gafira_doc::{LineCap, LineDash, LineEnd, LineNode};
        use kurbo::Shape as _;
        let make = |dash, start, end| LineNode {
            start: LineEnd::new([0.0, 0.0], start),
            end: LineEnd::new([1.0, 1.0], end),
            color: Color::Srgb(Rgba::new(0.0, 0.0, 0.0, 1.0)),
            width: 4.0,
            dash,
        };
        let ends = [kurbo::Point::new(0.0, 0.0), kurbo::Point::new(200.0, 0.0)];
        let none = LineCap::None;
        assert_eq!(
            line_layers(&make(LineDash::Solid, none, none), ends).len(),
            1
        );
        let dashes = line_layers(&make(LineDash::Dashed, none, none), ends);
        let dots = line_layers(&make(LineDash::Dotted, none, none), ends);
        assert!(dashes.len() >= 10, "{}", dashes.len());
        assert!(dots.len() > dashes.len(), "{} dots", dots.len());
        for dot in &dots {
            let b = dot.bounding_box();
            assert!(b.width() <= 4.5 && b.height() <= 4.5, "{b:?}");
        }
        let size = 10.0; // 2.5 × the width
        let filled = line_layers(
            &make(
                LineDash::Solid,
                LineCap::CircleFilled,
                LineCap::SquareFilled,
            ),
            ends,
        );
        let covers = |p: kurbo::Point| filled.iter().any(|l| l.winding(p) != 0);
        assert!(covers(kurbo::Point::new(size, 0.0)), "the circle is filled");
        assert!(
            covers(kurbo::Point::new(200.0 - size, 0.0)),
            "the square is filled"
        );
        let bar = line_layers(&make(LineDash::Solid, LineCap::Bar, none), ends);
        assert!(
            bar.iter().any(|l| {
                let b = l.bounding_box();
                b.height() >= 2.0 * size - 0.5 && b.width() <= 6.0
            }),
            "a bar across the start"
        );
    }

    // S106-06: a text with «algo» bold and red draws its letters in two
    // colours, each with its glyphs for the PDF; «algo» is wider in bold.
    #[test]
    fn s106_06_a_text_of_two_styles() {
        let ink = Color::Srgb(gafira_color::Rgba::new(0.0, 0.0, 0.0, 1.0));
        let red = Color::Srgb(gafira_color::Rgba::new(1.0, 0.0, 0.0, 1.0));
        let base = gafira_doc::SpanStyle {
            font: "Inter".to_owned(),
            size: 40.0,
            color: ink,
            bold: false,
            italic: false,
        };
        let mut text = TextNode::plain("Agregar algo de texto", &base);
        let plain = text.clone();
        text.content
            .restyle(8..12, &|s: &mut gafira_doc::SpanStyle| {
                s.bold = true;
                s.color = red;
            });
        assert!(text.styled(), "drawn from its outline on the canvas");
        let placement = Placement::new(0.0, 0.0, 1000.0, 100.0);
        let fonts = fonts();
        let (_, _, key, _, parts) = text_path(&text, &placement, &fonts).expect("laid out");
        let colors: Vec<Color> = parts.iter().map(|(_, c, _)| *c).collect();
        assert_eq!(colors, vec![ink, red]);
        assert!(parts.iter().all(|(_, _, runs)| !runs.is_empty()));
        let (_, _, plain_key, _, plain_parts) =
            text_path(&plain, &placement, &fonts).expect("laid out");
        assert!(plain_parts.is_empty(), "one style draws as before");
        assert_ne!(key, plain_key, "the styles are in the key");
        let wide = |t: &TextNode| {
            text_outline(t, (1000.0, 100.0), &fonts)
                .map(|(path, _, _)| path.bounding_box().width())
                .unwrap_or_default()
        };
        assert!(wide(&text) > wide(&plain) + 1.0);
        let parts = text_parts(&text, (1000.0, 100.0), &fonts).expect("parts");
        assert_eq!(parts.len(), 2);
    }

    // S81-12: a table draws the fills, the borders and, each cell a text
    // with its glyphs, the words; a cell a merge hides draws nothing.
    #[test]
    fn s81_12_a_table_keeps_its_words_as_text() {
        use gafira_doc::{CellStyle, TableNode};
        let ink = Color::Srgb(gafira_color::Rgba::new(0.0, 0.0, 0.0, 1.0));
        let mut table = TableNode::new(2, 2, (100.0, 40.0), &CellStyle::new("Inter", 16.0, ink));
        for (cell, words) in table.cells.iter_mut().zip(["a", "b", "c", "d"]) {
            cell.text = words.to_owned();
        }
        table.cells[0].style.fill = Some(ink);
        let pen = gafira_doc::ShapeStroke::new(ink, 2.0);
        let all = gafira_table::Range::all(&table);
        gafira_table::set_borders(&mut table, all, gafira_table::BorderSet::Outer, Some(pen));
        let layout = gafira_table::Range::new((1, 0), (1, 1));
        gafira_table::merge(&mut table, layout);
        let ops = table_ops(&table, (200.0, 80.0), Affine::IDENTITY, &fonts());
        let words: Vec<String> = ops
            .iter()
            .filter_map(|op| match op {
                RenderOp::Text { runs, .. } => Some(runs[0].text.to_string()),
                _ => None,
            })
            .collect();
        assert_eq!(words, ["a", "b", "c\nd"]);
        let fills = ops.len() - words.len();
        assert_eq!(fills, 5, "one fill and four borders");
        // Every letter lies in its cell.
        for op in &ops {
            if let RenderOp::Text {
                path, transform, ..
            } = op
            {
                let bounds = (*transform * (**path).clone()).bounding_box();
                assert!(Rect::new(0.0, 0.0, 200.0, 80.0).contains_rect(bounds));
            }
        }
    }

    /// A chart of a pie with the labels and the shares.
    fn pie_chart() -> gafira_doc::ChartNode {
        use gafira_doc::{ChartKind, ChartLabels, ChartNode, ChartText};
        let ink = Color::Srgb(gafira_color::Rgba::new(0.0, 0.0, 0.0, 1.0));
        let cells = ["Equipo", "Total", "Ventas", "75", "Otros", "25"];
        let text = ChartText {
            font: "Inter".to_owned(),
            size: 14.0,
            color: ink,
            bold: false,
            italic: false,
        };
        let data = (2, cells.map(str::to_owned).to_vec());
        let mut chart = ChartNode::new(ChartKind::Pie, data, vec![ink], text);
        chart.names = true;
        chart.numbers = ChartLabels::Percent;
        chart
    }

    // S83-09: a chart is paths and words, inside its box, and the canvas
    // draws the same outlines.
    #[test]
    fn s83_09_a_chart_keeps_its_words_as_text() {
        let chart = pie_chart();
        let ops = chart_ops(&chart, (400.0, 300.0), Affine::IDENTITY, &fonts());
        let mut words: Vec<String> = Vec::new();
        for op in &ops {
            match op {
                RenderOp::Text {
                    runs,
                    path,
                    transform,
                    ..
                } => {
                    words.push(runs[0].text.to_string());
                    let bounds = (*transform * (**path).clone()).bounding_box();
                    assert!(Rect::new(0.0, 0.0, 400.0, 300.0).contains_rect(bounds));
                }
                RenderOp::FillPath { .. } => {}
                _ => panic!("neither a path nor words"),
            }
        }
        assert_eq!(words, ["Ventas", "75%", "Otros", "25%"]);
        assert_eq!(ops.len(), 6);
        let outlines = chart_outlines(&chart, (400.0, 300.0), &fonts());
        assert_eq!(outlines.len(), ops.len());
    }

    /// A filled ellipse of 200 by 100 at (10, 20) with `words` inside it,
    /// aligned down the box by `valign` (spec 055).
    fn shape_text_page(words: &str, valign: gafira_doc::VerticalAlign) -> Page {
        let mut page = new_page();
        let style = gafira_doc::SpanStyle {
            font: "Inter".to_owned(),
            size: 20.0,
            color: Color::Srgb(Rgba::new(0.1, 0.2, 0.3, 1.0)),
            bold: false,
            italic: false,
        };
        let mut shape = ShapeNode::new(ShapeGeometry::Ellipse);
        shape.fill = Some(RED);
        let mut text = gafira_doc::ShapeText::plain(words, &style);
        text.valign = valign;
        shape.text = Some(text);
        let base = ElementBase::new(NodeId::new(1), Placement::new(10.0, 20.0, 200.0, 100.0));
        page.nodes.insert(
            NodeId::new(1),
            Arc::new(Node::new(base, NodeKind::Shape(shape))),
        );
        page.roots = vec![NodeId::new(1)];
        page
    }

    // S55-03: the text of a shape is drawn over its fill, inside the box
    // less a tenth of its shorter side; top, middle and bottom place it
    // there; another text keys the shape apart.
    #[test]
    fn s55_03_shape_text_sits_where_it_is_aligned() {
        use gafira_doc::VerticalAlign;
        let text_box = |valign| {
            let graph = build_text(&shape_text_page("Hola", valign), 1.0, Some(fonts()));
            let all = fills(&graph);
            assert_eq!(all.len(), 2, "the fill, then the text");
            assert_eq!(all[0].1, RED);
            all[1].0
        };
        let (top, middle, bottom) = (
            text_box(VerticalAlign::Top),
            text_box(VerticalAlign::Middle),
            text_box(VerticalAlign::Bottom),
        );
        let pad = 10.0;
        for b in [top, middle, bottom] {
            assert!(b.x0 >= pad && b.x1 <= 200.0 - pad, "{b:?}");
            assert!(b.y0 >= pad - 1.0 && b.y1 <= 100.0 - pad + 1.0, "{b:?}");
        }
        assert!(top.y0 < middle.y0 && middle.y0 < bottom.y0);
        assert!((middle.center().x - 100.0).abs() < 2.0, "centred across");
        let key = |words| {
            build_text(
                &shape_text_page(words, VerticalAlign::Middle),
                1.0,
                Some(fonts()),
            )
            .key_of(NodeId::new(1))
        };
        assert_ne!(key("Hola"), key("Adiós"));
    }

    /// A text element of one style, in a box of 200 by 60 at (10, 20).
    fn text_page(words: &str) -> Page {
        let mut page = new_page();
        let style = gafira_doc::SpanStyle {
            font: "Inter".to_owned(),
            size: 32.0,
            color: Color::Srgb(Rgba::new(0.1, 0.2, 0.3, 1.0)),
            bold: false,
            italic: false,
        };
        let base = ElementBase::new(NodeId::new(1), Placement::new(10.0, 20.0, 200.0, 60.0));
        let kind = NodeKind::Text(TextNode::plain(words, &style));
        page.nodes
            .insert(NodeId::new(1), Arc::new(Node::new(base, kind)));
        page.roots = vec![NodeId::new(1)];
        page
    }

    // S70-04 (FD-44): an arch keeps its middle letter on the line and bends
    // its ends down; a cup bends them up and starts at the top of the box.
    #[test]
    fn s70_04_curved_text_follows_its_arc() {
        use kurbo::Shape as _;
        let style = gafira_doc::SpanStyle {
            font: "Inter".to_owned(),
            size: 32.0,
            color: Color::Srgb(Rgba::new(0.0, 0.0, 0.0, 1.0)),
            bold: false,
            italic: false,
        };
        let fonts = fonts();
        let band = |curve: f64| {
            let mut text = TextNode::plain("IIIIIIIIIIIII", &style);
            text.curve = curve;
            let (path, _, _) = text_outline(&text, (600.0, 60.0), &fonts).expect("an outline");
            // The top of the first glyph and of the middle of the line.
            let bbox = path.bounding_box();
            let mid = kurbo::Rect::new(bbox.center().x - 4.0, -1e4, bbox.center().x + 4.0, 1e4);
            let top_mid = path
                .elements()
                .iter()
                .filter_map(|e| e.end_point())
                .filter(|p| mid.contains(*p))
                .map(|p| p.y)
                .fold(f64::MAX, f64::min);
            (bbox.y0, top_mid, bbox.height())
        };
        let (straight_top, straight_mid, straight_h) = band(0.0);
        let (arch_top, arch_mid, arch_h) = band(60.0);
        let (cup_top, cup_mid, cup_h) = band(-60.0);
        assert!(arch_h > straight_h + 10.0, "an arch is taller than a line");
        assert!(cup_h > straight_h + 10.0, "and so is a cup");
        assert!(
            (arch_mid - arch_top).abs() < 1.0,
            "the middle of an arch is its top"
        );
        // A cup starts where the straight letters start, so it stays in its
        // box; its ends are above its middle.
        assert!(
            (cup_top - straight_top).abs() < 1.0,
            "a cup starts at the top"
        );
        assert!(
            cup_mid > cup_top + 5.0,
            "a cup's ends rise above its middle"
        );
        let _ = straight_mid;
        assert!(
            (straight_mid - arch_mid).abs() < 2.0,
            "the middle stays on the line"
        );
    }

    // S70-06 (FD-41): letter spacing widens the words, a line height
    // spaces the lines, both key the text apart; a gradient fills the
    // letters over the box (spec 070).
    #[test]
    fn s70_06_spacing_line_height_and_gradient_of_a_text() {
        use kurbo::Shape as _;
        let style = gafira_doc::SpanStyle {
            font: "Inter".to_owned(),
            size: 32.0,
            color: Color::Srgb(Rgba::new(0.0, 0.0, 0.0, 1.0)),
            bold: false,
            italic: false,
        };
        let fonts = fonts();
        let measure = |letters: f64, lines: Option<f64>| {
            let mut text = TextNode::plain("Hola\nmundo", &style);
            text.letter_spacing = letters;
            text.line_height = lines;
            let (path, _, height) = text_outline(&text, (600.0, 200.0), &fonts).expect("drawn");
            (path.bounding_box().width(), height)
        };
        let (plain_w, plain_h) = measure(0.0, None);
        let (spaced_w, _) = measure(200.0, None);
        let (_, tall_h) = measure(0.0, Some(2.0));
        assert!(spaced_w > plain_w + 10.0, "{spaced_w} > {plain_w}");
        assert!(tall_h > plain_h + 20.0, "{tall_h} > {plain_h}");

        let mut page = text_page("Hola");
        let plain_key = build_text(&page, 1.0, Some(fonts.clone())).key_of(NodeId::new(1));
        let node = page.nodes.get_mut(&NodeId::new(1)).expect("the text");
        let node = Arc::make_mut(node);
        let NodeKind::Text(text) = &mut node.kind else {
            panic!("a text")
        };
        text.gradient = Some(gafira_doc::Gradient {
            kind: gafira_doc::GradientKind::Linear { angle_deg: 0.0 },
            stops: vec![
                gafira_doc::GradientStop {
                    offset: 0.0,
                    color: Color::Srgb(Rgba::new(1.0, 0.0, 0.0, 1.0)),
                },
                gafira_doc::GradientStop {
                    offset: 1.0,
                    color: Color::Srgb(Rgba::new(0.0, 0.0, 1.0, 1.0)),
                },
            ],
        });
        let graph = build_text(&page, 1.0, Some(fonts));
        assert_eq!(op_kinds(&graph), ["push", "gradient", "pop"]);
        assert_ne!(graph.key_of(NodeId::new(1)), plain_key);
    }

    // S63-03: a shape with a gradient emits its fill as a gradient in its
    // box, and another gradient keys it apart.
    #[test]
    fn s63_03_a_gradient_shape_emits_a_gradient_fill() {
        use gafira_doc::{Gradient, GradientKind, GradientStop};
        let page_with = |angle: f64| {
            let mut page = new_page();
            let mut shape = ShapeNode::new(ShapeGeometry::Ellipse);
            shape.gradient = Some(Gradient {
                kind: GradientKind::Linear { angle_deg: angle },
                stops: vec![
                    GradientStop {
                        offset: 0.0,
                        color: RED,
                    },
                    GradientStop {
                        offset: 1.0,
                        color: Color::Srgb(Rgba::new(0.0, 0.0, 1.0, 1.0)),
                    },
                ],
            });
            let base = ElementBase::new(NodeId::new(1), Placement::new(10.0, 20.0, 80.0, 40.0));
            page.nodes.insert(
                NodeId::new(1),
                Arc::new(Node::new(base, NodeKind::Shape(shape))),
            );
            page.roots = vec![NodeId::new(1)];
            page
        };
        let graph = build(&page_with(0.0), WalkOrder::PreOrder);
        assert_eq!(op_kinds(&graph), ["push", "gradient", "pop"]);
        let size = graph.ops().iter().find_map(|op| match op {
            RenderOp::FillGradient { size, .. } => Some(*size),
            _ => None,
        });
        assert_eq!(size, Some((80.0, 40.0)));
        let key = |angle| build(&page_with(angle), WalkOrder::PreOrder).key_of(NodeId::new(1));
        assert_ne!(key(0.0), key(90.0));
    }

    // S65-04: a light draws a picture instead of the shape: «Brillo» over
    // the box of the shape, «Destello de lente» over the whole page; another
    // light keys apart.
    #[test]
    fn s65_04_a_light_draws_a_picture_instead_of_the_shape() {
        use gafira_doc::{Light, ShapeEffect};
        let page_with = |effect: ShapeEffect| {
            let mut page = new_page();
            let mut shape = ShapeNode::new(ShapeGeometry::Ellipse);
            shape.fill = Some(RED);
            shape.effect = Some(effect);
            let base = ElementBase::new(NodeId::new(1), Placement::new(10.0, 20.0, 80.0, 40.0));
            page.nodes.insert(
                NodeId::new(1),
                Arc::new(Node::new(base, NodeKind::Shape(shape))),
            );
            page.roots = vec![NodeId::new(1)];
            page
        };
        let built = |effect: ShapeEffect| {
            let page = page_with(effect);
            let view = PageView::new(&page);
            RenderGraphBuilder::new(&view, params(RenderQuality::Preview), InlinePool)
                .with_page_px(200.0, 100.0)
                .build(WalkOrder::PreOrder, &CancelToken::new(), &NoProgress)
                .expect("builds")
        };
        let picture = |graph: &RenderGraph| {
            graph.ops().iter().find_map(|op| match op {
                RenderOp::DrawImage { image, transform } => {
                    Some((image.width(), image.height(), *transform))
                }
                _ => None,
            })
        };
        let sparkle = built(ShapeEffect::Sparkle(Light::SPARKLE));
        assert_eq!(op_kinds(&sparkle), ["push", "image", "pop"]);
        let (w, h, at) = picture(&sparkle).expect("a picture");
        assert_eq!((w, h), (80, 40), "the box of the shape at scale 1");
        let corner = at * kurbo::Point::new(f64::from(w), f64::from(h));
        assert!((corner.x - 90.0).abs() < 1e-9 && (corner.y - 60.0).abs() < 1e-9);
        let flare = built(ShapeEffect::LensFlare(Light::LENS_FLARE));
        let (w, h, _) = picture(&flare).expect("a picture");
        assert_eq!((w, h), (200, 100), "the whole page");
        let mut turned = Light::SPARKLE;
        turned.angle_deg = 45.0;
        assert_ne!(
            sparkle.key_of(NodeId::new(1)),
            built(ShapeEffect::Sparkle(turned)).key_of(NodeId::new(1))
        );
    }

    // S66-04: a frame with its photo draws the photo and then cuts the
    // layer to its shape; an empty frame without outline draws nothing.
    #[test]
    fn s66_04_a_frame_emits_a_clip_and_an_image() {
        use gafira_doc::timeline::AssetRef;
        use gafira_doc::{ContentHash, FrameContent, FrameNode};
        let asset = AssetRef::new(ContentHash([3; 32]));
        let page_with = |content: Option<FrameContent>| {
            let mut page = new_page();
            let mut frame = FrameNode::new(ShapeGeometry::Ellipse);
            frame.content = content;
            let base = ElementBase::new(NodeId::new(1), Placement::new(10.0, 20.0, 80.0, 40.0));
            page.nodes.insert(
                NodeId::new(1),
                Arc::new(Node::new(base, NodeKind::Frame(frame))),
            );
            page.roots = vec![NodeId::new(1)];
            page
        };
        let pixels = gafira_raster::ImageBuf::new(
            2,
            2,
            gafira_raster::PixelBuf::Rgba8([255, 0, 0, 255].repeat(4)),
        )
        .expect("image");
        let images = Arc::new(ImagePixels::from([(asset, Arc::new(pixels))]));
        let built = |page: &Page| {
            let view = PageView::new(page);
            RenderGraphBuilder::new(&view, params(RenderQuality::Preview), InlinePool)
                .with_images(Arc::clone(&images))
                .build(WalkOrder::PreOrder, &CancelToken::new(), &NoProgress)
                .expect("builds")
        };
        let full = built(&page_with(Some(FrameContent {
            asset,
            natural: (200, 100),
        })));
        assert_eq!(op_kinds(&full), ["push", "image", "clip", "pop"]);
        assert!(op_kinds(&built(&page_with(None))).is_empty());
    }

    // S62-03: a bold text is drawn with the bold face of its family; a
    // family without that face falls back to its regular one.
    #[test]
    fn s62_03_bold_uses_the_bold_face() {
        let fonts = Arc::new(
            FontBook::new()
                .with(
                    "Open Sans",
                    include_bytes!("../../../assets/fonts/library/opensans/OpenSans-Regular.ttf")
                        .as_slice(),
                )
                .with(
                    "Open Sans Bold",
                    include_bytes!("../../../assets/fonts/library/opensans/OpenSans-Bold.ttf")
                        .as_slice(),
                )
                .with(
                    "Anton",
                    include_bytes!("../../../assets/fonts/library/anton/Anton-Regular.ttf")
                        .as_slice(),
                ),
        );
        let outline = |family: &str, bold: bool| {
            let mut page = text_page("Hola");
            let mut node = page.node(NodeId::new(1)).expect("the text").clone();
            if let NodeKind::Text(text) = &mut node.kind {
                for span in text
                    .content
                    .paragraphs
                    .iter_mut()
                    .flat_map(|p| p.spans.iter_mut())
                {
                    span.style.font = family.to_owned();
                    span.style.bold = bold;
                }
            }
            page.nodes.insert(NodeId::new(1), Arc::new(node));
            let graph = build_text(&page, 1.0, Some(fonts.clone()));
            fills(&graph)[0].0
        };
        assert_ne!(
            outline("Open Sans", true),
            outline("Open Sans", false),
            "the bold face"
        );
        assert_eq!(
            outline("Anton", true),
            outline("Anton", false),
            "no bold Anton: the regular"
        );
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
                // A text fills its outline too (spec 073).
                RenderOp::FillPath {
                    path,
                    color,
                    transform,
                }
                | RenderOp::Text {
                    path,
                    color,
                    transform,
                    ..
                } => Some((path.bounding_box(), *color, *transform)),
                _ => None,
            })
            .collect()
    }

    // S73-01: a text carries its glyphs, their font and its words, for the
    // PDF; a curved one does not (its letters turn one by one).
    #[test]
    fn s73_01_a_text_carries_its_glyph_runs() {
        let runs = |page: &Page| -> Vec<GlyphRun> {
            let graph = build_text(page, 1.0, Some(fonts()));
            graph
                .ops()
                .iter()
                .find_map(|op| match op {
                    RenderOp::Text { runs, .. } => Some(runs.as_ref().clone()),
                    _ => None,
                })
                .expect("a text op")
        };
        let found = runs(&text_page("Hola"));
        assert_eq!(found.len(), 1);
        assert_eq!(&*found[0].text, "Hola");
        assert_eq!(found[0].glyphs.len(), 4);
        assert!((found[0].size - 32.0).abs() < 1e-3);
        assert!(found[0].glyphs.windows(2).all(|w| w[1].x > w[0].x));
        let mut curved = text_page("Hola");
        let node = Arc::make_mut(curved.nodes.get_mut(&NodeId::new(1)).expect("text"));
        if let NodeKind::Text(text) = &mut node.kind {
            text.curve = 50.0;
        }
        assert!(runs(&curved).is_empty());
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
        let kind = NodeKind::Image(gafira_doc::ImageNode::new(asset));
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

    // S85-04: an image with a shadow keeps its box; its picture reaches past
    // it by the margin of the shadow.
    #[test]
    fn s85_04_a_shadow_reaches_past_the_box() {
        let mut shadow = gafira_doc::ImageShadow::new(gafira_doc::ShadowKind::Drop);
        (shadow.blur, shadow.distance) = (0.0, 0.195);
        let image = gafira_doc::ImageNode::new(asset(7)).looking(gafira_doc::ImageLook {
            shadow: Some(shadow),
            ..gafira_doc::ImageLook::default()
        });
        let mut page = image_page(asset(7));
        let mut node = (*page.nodes[&NodeId::new(1)]).clone();
        node.kind = NodeKind::Image(image.clone());
        page.nodes.insert(NodeId::new(1), Arc::new(node));
        // An image of 20 by 10 with its margin of 2, under its own key.
        let pixels = gafira_raster::ImageBuf::new(
            24,
            14,
            gafira_raster::PixelBuf::Rgba8(vec![255; 24 * 14 * 4]),
        )
        .expect("an image");
        let images = ImagePixels::from([(image.shown(), Arc::new(pixels))]);
        let graph = build_image(&page, 1.0, Some(Arc::new(images)));
        let [(_, transform)] = draws(&graph)[..] else {
            panic!("one image draw");
        };
        // Its box is 40 by 20 at (10, 30).
        assert_eq!(mapped(transform, 2.0, 2.0), (10.0, 30.0));
        assert_eq!(mapped(transform, 22.0, 12.0), (50.0, 50.0));
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
