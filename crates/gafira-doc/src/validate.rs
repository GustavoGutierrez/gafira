//! `Document::validate`: the structural rules, one pass at a time.

use std::collections::{BTreeMap, BTreeSet};

use gafira_color::Color;
use serde_json::Value;

use crate::node::MAX_EXTENSION_DEPTH;
use crate::timeline::ClipSource;
use crate::{Document, Node, NodeField, NodeId, NodeKind, Page, ValidationError};

impl Document {
    /// Returns the first rule the document breaks, or `Ok(())`.
    ///
    /// Passes run in a fixed order. In P1 to P3 each check runs over all
    /// pages (in `pages` order) before the next check, in the order of the
    /// [`ValidationError`] variants; P4 goes node by node. Nothing is altered
    /// or clamped.
    pub fn validate(&self) -> Result<(), ValidationError> {
        self.check_header()?;
        self.check_ids()?;
        self.check_pages()?;
        self.check_nodes()?;
        self.check_parents()?;
        self.check_reachability()?;
        self.check_clip_sources()?;
        self.check_image_sources()?;
        self.check_timeline()?;
        self.check_data()
    }

    /// P9: the connected data of bulk create keeps its table rules (spec 104).
    fn check_data(&self) -> Result<(), ValidationError> {
        match self.data.as_deref().and_then(crate::DataSet::problem) {
            Some(problem) => Err(ValidationError::InvalidData { problem }),
            None => Ok(()),
        }
    }

    /// P1: schema version, then DPI.
    fn check_header(&self) -> Result<(), ValidationError> {
        if self.schema_version != Self::SCHEMA_VERSION {
            return Err(ValidationError::UnsupportedSchemaVersion {
                found: self.schema_version,
            });
        }
        if !self.dpi.is_valid() {
            return Err(ValidationError::InvalidDpi {
                dpi: self.dpi.get(),
            });
        }
        Ok(())
    }

    /// P2: distinct ids, arena keys, then the `next_id` bound.
    ///
    /// The counter is one for the document, so the ids of the pages, the nodes
    /// and the timeline share it and are distinct among themselves and below
    /// `next_id` (ARQ-10).
    fn check_ids(&self) -> Result<(), ValidationError> {
        let ids = || {
            let pages = self.pages.iter().flat_map(|p| {
                let nodes = p.nodes.values().map(|n| n.base.id.get());
                std::iter::once(p.id.get()).chain(nodes)
            });
            let timeline = self.sequences.iter().flat_map(|sequence| {
                let lanes = sequence.tracks().iter().flat_map(|track| {
                    let clips = track.clips().iter().map(|clip| clip.id().get());
                    std::iter::once(track.id().get()).chain(clips)
                });
                let markers = sequence.markers().iter().map(|marker| marker.id().get());
                std::iter::once(sequence.id().get())
                    .chain(lanes)
                    .chain(markers)
            });
            pages.chain(timeline)
        };
        let mut seen = BTreeSet::new();
        if let Some(id) = ids().find(|&id| !seen.insert(id)) {
            return Err(ValidationError::DuplicateId { id });
        }
        for page in &self.pages {
            if let Some((&key, node)) = page.nodes.iter().find(|(k, n)| **k != n.base.id) {
                let (page, id) = (page.id, node.base.id);
                return Err(ValidationError::ArenaKeyMismatch { page, key, id });
            }
        }
        match ids().max() {
            Some(max_id) if self.next_id <= max_id => Err(ValidationError::NextIdNotAboveMax {
                next_id: self.next_id,
                max_id,
            }),
            _ => Ok(()),
        }
    }

    /// P3: page sizes, then page layout data, then roots that do not exist, then repeated roots.
    fn check_pages(&self) -> Result<(), ValidationError> {
        // The background draws, and its image is in the table (spec 069).
        for page in &self.pages {
            if let Some(background) = &page.background
                && (!background.is_valid()
                    || background.asset().is_some_and(|a| self.asset(&a).is_none()))
            {
                return Err(ValidationError::InvalidBackground { page: page.id });
            }
        }
        for page in &self.pages {
            let size = page.size.check(self.dpi);
            size.map_err(|error| ValidationError::PageSize {
                page: page.id,
                error,
            })?;
        }
        for page in &self.pages {
            let layout = page.check_layout(self.dpi);
            layout.map_err(|error| ValidationError::PageLayout {
                page: page.id,
                error,
            })?;
        }
        for page in &self.pages {
            if let Some(&node) = page.roots.iter().find(|r| !page.nodes.contains_key(r)) {
                return Err(ValidationError::DanglingRoot {
                    page: page.id,
                    node,
                });
            }
        }
        for page in &self.pages {
            let mut seen = BTreeSet::new();
            if let Some(&node) = page.roots.iter().find(|&&r| !seen.insert(r)) {
                return Err(ValidationError::DuplicateRoot {
                    page: page.id,
                    node,
                });
            }
        }
        Ok(())
    }

    /// P4: node by node (ascending id), each node's checks in order.
    fn check_nodes(&self) -> Result<(), ValidationError> {
        for page in &self.pages {
            for node in page.nodes.values() {
                check_node(page, node)?;
            }
        }
        Ok(())
    }

    /// P5: more than one parent edge, then a root with a parent, then an orphan.
    /// Each check runs over all pages before the next one.
    fn check_parents(&self) -> Result<(), ValidationError> {
        let counts: Vec<BTreeMap<NodeId, usize>> =
            self.pages.iter().map(|p| parent_edges(p)).collect();
        for count in &counts {
            if let Some((&node, _)) = count.iter().find(|(_, n)| **n > 1) {
                return Err(ValidationError::MultipleParents { node });
            }
        }
        for (page, count) in self.pages.iter().zip(&counts) {
            let child = page
                .nodes
                .keys()
                .find(|id| page.roots.contains(id) && count.contains_key(id));
            if let Some(&node) = child {
                let parent = page.nodes.values().find(|n| n.children.contains(&node));
                if let Some(parent) = parent {
                    let parent = parent.base.id;
                    return Err(ValidationError::RootHasParent { node, parent });
                }
            }
        }
        for (page, count) in self.pages.iter().zip(&counts) {
            let alone = |id: &&NodeId| !page.roots.contains(id) && !count.contains_key(id);
            if let Some(&node) = page.nodes.keys().find(alone) {
                return Err(ValidationError::Orphan { node });
            }
        }
        Ok(())
    }

    /// P6: depth-first search from the roots; the lowest unreached id is a cycle.
    /// Iterative, so a deep tree cannot overflow the stack.
    fn check_reachability(&self) -> Result<(), ValidationError> {
        for page in &self.pages {
            let mut reached: BTreeSet<NodeId> = BTreeSet::new();
            let mut stack: Vec<NodeId> = page.roots.clone();
            while let Some(id) = stack.pop() {
                if reached.insert(id)
                    && let Some(node) = page.nodes.get(&id)
                {
                    stack.extend(&node.children);
                }
            }
            if let Some(&node) = page.nodes.keys().find(|id| !reached.contains(id)) {
                return Err(ValidationError::Cycle { node });
            }
        }
        Ok(())
    }

    /// P7: the asset each image element shows is in the asset table (spec
    /// 029). An image is never left pointing at nothing.
    fn check_image_sources(&self) -> Result<(), ValidationError> {
        for page in &self.pages {
            for (id, node) in &page.nodes {
                // What it can go back to is there too (spec 084).
                if let NodeKind::Image(image) = &node.kind
                    && image.assets().any(|asset| self.asset(&asset).is_none())
                {
                    return Err(ValidationError::DanglingImage { node: *id });
                }
                // The logo of a QR code is one too (spec 080).
                if let NodeKind::Qr(qr) = &node.kind
                    && let Some(logo) = qr.logo
                    && self.asset(&logo.asset).is_none()
                {
                    return Err(ValidationError::InvalidQr { node: *id });
                }
                // A 3D model's file is an asset of the table (spec 100 §5).
                if let NodeKind::Model3d(model) = &node.kind
                    && model.assets().any(|asset| self.asset(&asset).is_none())
                {
                    return Err(ValidationError::InvalidModel3d { node: *id });
                }
                // A frame's photo is an asset of the table too (spec 066).
                if let NodeKind::Frame(frame) = &node.kind
                    && let Some(content) = frame.content
                    && self.asset(&content.asset).is_none()
                {
                    return Err(ValidationError::InvalidFrame { node: *id });
                }
            }
        }
        Ok(())
    }

    /// P7: the source each clip of the timeline names — a `Scene` clip names a
    /// page of the document, and a `Media` or `Audio` clip names an entry of
    /// the asset table (P9 of the timeline design).
    ///
    /// A page a clip shows is never removed while the clip exists: deleting
    /// both is a command (`RemovePage` alone leaves a document that does not
    /// validate, the safe half-state), FV-05 (c).
    fn check_clip_sources(&self) -> Result<(), ValidationError> {
        for sequence in &self.sequences {
            for track in sequence.tracks() {
                for clip in track.clips() {
                    let error = match clip.source() {
                        ClipSource::Scene(page) if self.page(*page).is_none() => {
                            Some(ValidationError::DanglingScene {
                                clip: clip.id(),
                                page: *page,
                            })
                        }
                        ClipSource::Media(reference) | ClipSource::Audio(reference)
                            if self.asset(reference).is_none() =>
                        {
                            Some(ValidationError::DanglingAsset { clip: clip.id() })
                        }
                        _ => None,
                    };
                    if let Some(error) = error {
                        return Err(error);
                    }
                }
            }
        }
        Ok(())
    }

    /// P8: the laws of the timeline model, sequence by sequence — the cut on
    /// the grid, the span of at least one frame, the clip inside its sequence
    /// and inside its source, the speed that divides, the one strip per lane
    /// at any instant and the cap of the audio tracks (FV-01, FV-03, P11).
    ///
    /// The model upholds these laws as the sequence is built ([`Track::insert`],
    /// [`Sequence::add_track`]); a value that comes from bytes went through no
    /// insertion, so a document that breaks one fails to load instead of
    /// reaching the editor (ARQ-13).
    fn check_timeline(&self) -> Result<(), ValidationError> {
        for sequence in &self.sequences {
            sequence
                .check_laws()
                .map_err(|error| ValidationError::TimelineLaw {
                    sequence: sequence.id(),
                    error,
                })?;
        }
        Ok(())
    }
}

/// How many `children` lists of the page name each node.
fn parent_edges(page: &Page) -> BTreeMap<NodeId, usize> {
    let mut count = BTreeMap::new();
    for child in page.nodes.values().flat_map(|n| &n.children) {
        *count.entry(*child).or_insert(0) += 1;
    }
    count
}

type Rule = fn(f64) -> bool;

fn any(_: f64) -> bool {
    true
}

fn non_negative(v: f64) -> bool {
    v >= 0.0
}

fn below_quarter_turn(v: f64) -> bool {
    v > -90.0 && v < 90.0
}

fn unit_interval(v: f64) -> bool {
    (0.0..=1.0).contains(&v)
}

fn stroke_width(v: f64) -> bool {
    (0.0..=crate::ShapeStroke::MAX_WIDTH).contains(&v)
}

fn path_coordinate(v: f64) -> bool {
    (crate::ShapePath::MIN_COORDINATE..=crate::ShapePath::MAX_COORDINATE).contains(&v)
}

/// What is wrong with the orders of a shape path, if anything (spec 054).
fn path_problem(path: &crate::ShapePath) -> Option<crate::PathProblem> {
    use crate::{PathCommand, PathProblem};
    match path.commands.first() {
        None => Some(PathProblem::Empty),
        Some(PathCommand::MoveTo(_)) if path.commands.len() > crate::ShapePath::MAX_COMMANDS => {
            Some(PathProblem::TooLong {
                len: path.commands.len(),
            })
        }
        Some(PathCommand::MoveTo(_)) => None,
        Some(_) => Some(PathProblem::NoStart),
    }
}

fn text_curve(v: f64) -> bool {
    (-crate::TextNode::MAX_CURVE..=crate::TextNode::MAX_CURVE).contains(&v)
}

fn font_size(v: f64) -> bool {
    v > 0.0 && v <= crate::TextNode::MAX_SIZE
}

/// The four channels of a colour.
fn channels(color: Color) -> [f32; 4] {
    match color {
        Color::Srgb(c) => [c.r, c.g, c.b, c.a],
        Color::LinearSrgb(c) => [c.r, c.g, c.b, c.a],
        Color::DisplayP3(c) => [c.r, c.g, c.b, c.a],
    }
}

/// The numeric fields of a node, in reporting order, with their range rule.
fn numbers(node: &Node) -> Vec<(NodeField, f64, Rule)> {
    let p = &node.base.placement;
    let mut out: Vec<(NodeField, f64, Rule)> = vec![
        (NodeField::X, p.x, any),
        (NodeField::Y, p.y, any),
        (NodeField::W, p.w, non_negative),
        (NodeField::H, p.h, non_negative),
        (NodeField::RotationDeg, p.rotation_deg, any),
        (NodeField::Skew, p.skew, below_quarter_turn),
        (
            NodeField::Opacity,
            f64::from(node.base.opacity),
            unit_interval,
        ),
    ];
    if let NodeKind::Shape(shape) = &node.kind
        && let Some(fill) = shape.fill
    {
        let [r, g, b, a] = channels(fill);
        for channel in [r, g, b] {
            out.push((NodeField::FillChannel, f64::from(channel), any));
        }
        out.push((NodeField::FillAlpha, f64::from(a), unit_interval));
    }
    if let NodeKind::Shape(shape) = &node.kind {
        if let Some(stroke) = shape.stroke {
            let [r, g, b, a] = channels(stroke.color);
            for channel in [r, g, b] {
                out.push((NodeField::StrokeChannel, f64::from(channel), any));
            }
            out.push((NodeField::StrokeAlpha, f64::from(a), unit_interval));
            out.push((NodeField::StrokeWidth, stroke.width, stroke_width));
        }
        out.push((NodeField::CornerRadius, shape.corner_radius, non_negative));
        if let crate::ShapeGeometry::Path(path) = &shape.geometry {
            for v in path.coordinates() {
                out.push((NodeField::PathCoordinate, v, path_coordinate));
            }
        }
    }
    // A line: its colour, its width and its points in the box (spec 058).
    if let NodeKind::Line(line) = &node.kind {
        let [r, g, b, a] = channels(line.color);
        for channel in [r, g, b] {
            out.push((NodeField::StrokeChannel, f64::from(channel), any));
        }
        out.push((NodeField::StrokeAlpha, f64::from(a), unit_interval));
        out.push((NodeField::StrokeWidth, line.width, stroke_width));
        let attached = [line.start.attached, line.end.attached]
            .into_iter()
            .flatten()
            .flat_map(|a| a.at);
        for v in line.start.at.into_iter().chain(line.end.at).chain(attached) {
            out.push((NodeField::PathCoordinate, v, path_coordinate));
        }
    }
    // The words of a text element or of a shape's text (spec 055).
    let content = match &node.kind {
        NodeKind::Text(text) => Some(&text.content),
        NodeKind::Shape(shape) => shape.text.as_ref().map(|text| &text.content),
        _ => None,
    };
    if let NodeKind::Text(text) = &node.kind {
        out.push((NodeField::TextCurve, text.curve, text_curve));
        out.push((NodeField::LetterSpacing, text.letter_spacing, |v| {
            crate::TextNode::LETTER_SPACING.contains(&v)
        }));
        if let Some(height) = text.line_height {
            out.push((NodeField::LineHeight, height, |v| {
                crate::TextNode::LINE_HEIGHT.contains(&v)
            }));
        }
    }
    if let Some(content) = content {
        for span in content.spans() {
            out.push((NodeField::FontSize, span.style.size, font_size));
            let [r, g, b, a] = channels(span.style.color);
            for channel in [r, g, b] {
                out.push((NodeField::TextColorChannel, f64::from(channel), any));
            }
            out.push((NodeField::TextColorAlpha, f64::from(a), unit_interval));
        }
    }
    out
}

/// The checks 9 to 14 of one node; the first failure wins.
fn check_node(page: &Page, node: &Node) -> Result<(), ValidationError> {
    let id = node.base.id;
    let numbers = numbers(node);
    if let Some(&(field, _, _)) = numbers.iter().find(|(_, v, _)| !v.is_finite()) {
        return Err(ValidationError::NonFinite { node: id, field });
    }
    if let Some(&(field, value, _)) = numbers.iter().find(|(_, v, ok)| !ok(*v)) {
        return Err(ValidationError::OutOfRange {
            node: id,
            field,
            value,
        });
    }
    if let NodeKind::Shape(shape) = &node.kind
        && let crate::ShapeGeometry::Path(path) = &shape.geometry
        && let Some(problem) = path_problem(path)
    {
        return Err(ValidationError::InvalidPath { node: id, problem });
    }
    let gradient = match &node.kind {
        NodeKind::Shape(shape) => shape.gradient.as_ref(),
        NodeKind::Text(text) => text.gradient.as_ref(),
        _ => None,
    };
    if let Some(problem) = gradient.and_then(crate::Gradient::problem) {
        return Err(ValidationError::InvalidGradient { node: id, problem });
    }
    // The gradients of some letters (spec 106).
    let spans = match &node.kind {
        NodeKind::Text(text) => Some(&text.content),
        NodeKind::Shape(shape) => shape.text.as_ref().map(|t| &t.content),
        _ => None,
    };
    let span_problem = spans
        .into_iter()
        .flat_map(crate::RichText::spans)
        .find_map(|span| {
            span.style
                .gradient
                .as_ref()
                .and_then(crate::Gradient::problem)
        });
    if let Some(problem) = span_problem {
        return Err(ValidationError::InvalidGradient { node: id, problem });
    }
    if let NodeKind::Qr(qr) = &node.kind
        && !qr.is_valid()
    {
        return Err(ValidationError::InvalidQr { node: id });
    }
    if let NodeKind::Text(text) = &node.kind
        && (text.effects.len() > crate::TextEffect::MAX
            || !text.effects.iter().all(crate::TextEffect::is_valid))
    {
        return Err(ValidationError::InvalidText { node: id });
    }
    if let NodeKind::Image(image) = &node.kind
        && (image.look.as_ref().is_some_and(|look| !look.is_valid())
            || image.crop.as_ref().is_some_and(|crop| !crop.is_valid()))
    {
        return Err(ValidationError::InvalidImage { node: id });
    }
    if let NodeKind::Table(table) = &node.kind
        && !table.is_valid()
    {
        return Err(ValidationError::InvalidTable { node: id });
    }
    if let NodeKind::Chart(chart) = &node.kind
        && !chart.is_valid()
    {
        return Err(ValidationError::InvalidChart { node: id });
    }
    if let NodeKind::Model3d(model) = &node.kind
        && !model.is_valid()
    {
        return Err(ValidationError::InvalidModel3d { node: id });
    }
    if let NodeKind::Frame(frame) = &node.kind
        && !frame.is_valid()
    {
        return Err(ValidationError::InvalidFrame { node: id });
    }
    // The outline of a frame follows the rules of a shape path (spec 066).
    if let NodeKind::Frame(frame) = &node.kind
        && let crate::ShapeGeometry::Path(path) = &frame.clip_shape
        && let Some(problem) = path_problem(path)
    {
        return Err(ValidationError::InvalidPath { node: id, problem });
    }
    if let NodeKind::Shape(shape) = &node.kind
        && let Some(problem) = shape.effect.as_ref().and_then(crate::ShapeEffect::problem)
    {
        return Err(ValidationError::InvalidEffect { node: id, problem });
    }
    if let NodeKind::Extension(ext) = &node.kind {
        if ext.type_id.is_empty() {
            return Err(ValidationError::EmptyExtensionTypeId { node: id });
        }
        let depth = depth(&ext.payload);
        if depth > MAX_EXTENSION_DEPTH {
            return Err(ValidationError::ExtensionTooDeep { node: id, depth });
        }
    }
    if !matches!(node.kind, NodeKind::Group(_)) && !node.children.is_empty() {
        return Err(ValidationError::ChildrenOnLeaf { node: id });
    }
    match node.children.iter().find(|c| !page.nodes.contains_key(c)) {
        Some(&child) => Err(ValidationError::DanglingChild { parent: id, child }),
        None => Ok(()),
    }
}

/// Nesting depth of a JSON value: a scalar is 0, `[]` and `{}` are 1.
/// Iterative, so a hostile payload cannot overflow the stack.
fn depth(root: &Value) -> usize {
    let mut deepest = 0;
    let mut stack = vec![(root, 0usize)];
    while let Some((value, level)) = stack.pop() {
        let children: Vec<&Value> = match value {
            Value::Array(items) => items.iter().collect(),
            Value::Object(map) => map.values().collect(),
            _ => continue,
        };
        deepest = deepest.max(level + 1);
        stack.extend(children.into_iter().map(|c| (c, level + 1)));
    }
    deepest
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use gafira_color::{Color, LinearRgba, Rgba};
    use gafira_geom::{Length, Unit};
    use serde_json::{Value, json};

    use crate::ValidationError;
    use crate::document::test_document;
    use crate::{
        Axis, Document, Dpi, ElementBase, ExtensionNode, GroupNode, Node, NodeId, NodeKind, Page,
        PageId, PageSize, PageSizeError, Placement, ShapeGeometry, ShapeNode, ShapeStroke,
    };

    type Edit = fn(&mut Document);
    type EditRef<'a> = &'a dyn Fn(&mut Document);

    fn px(w: f64, h: f64) -> PageSize {
        let side = |v| Length::new(v, Unit::Px);
        PageSize {
            width: side(w),
            height: side(h),
        }
    }

    fn node(id: u64, kind: NodeKind) -> Arc<Node> {
        let base = ElementBase::new(NodeId::new(id), Placement::new(0.0, 0.0, 1.0, 1.0));
        Arc::new(Node::new(base, kind))
    }

    /// A page of 100 x 100 px whose nodes are all root groups.
    fn page(id: u64, nodes: &[u64]) -> Page {
        let mut p = Page::new(PageId::new(id), String::new(), px(100.0, 100.0));
        for &n in nodes {
            p.roots.push(NodeId::new(n));
            p.nodes
                .insert(NodeId::new(n), node(n, NodeKind::Group(GroupNode {})));
        }
        p
    }

    /// Page 1 with nodes 2 and 3, `next_id` 10, DPI 96: valid.
    fn base() -> Document {
        test_document(10, Dpi::default(), vec![Arc::new(page(1, &[2, 3]))])
    }

    fn first(d: &mut Document) -> &mut Page {
        Arc::make_mut(&mut d.pages[0])
    }

    fn push(d: &mut Document, id: u64, nodes: &[u64]) {
        d.pages.push(Arc::new(page(id, nodes)));
    }

    fn roots(p: &mut Page, ids: &[u64]) {
        p.roots = ids.iter().map(|&i| NodeId::new(i)).collect();
    }

    /// Appends page 7 (nodes 8 and 9) and edits it, so the defect is on the second page.
    fn two(d: &mut Document, edit: fn(&mut Page)) {
        push(d, 7, &[8, 9]);
        edit(Arc::make_mut(&mut d.pages[1]));
    }

    /// Moves the highest node of a page under key 5 (which must not collide with an existing id).
    fn rekey(p: &mut Page) {
        let (_, n) = p.nodes.pop_last().unwrap();
        p.nodes.insert(NodeId::new(5), n);
    }

    /// Appends page 7 holding node 10 and sets `next_id`.
    fn ten(d: &mut Document, next_id: u64) {
        push(d, 7, &[10]);
        d.next_id = next_id;
    }

    /// Appends page `page` holding `nodes` and sets `next_id` to `u64::MAX`.
    fn at_max(d: &mut Document, page: u64, nodes: &[u64]) {
        push(d, page, nodes);
        d.next_id = u64::MAX;
    }

    /// Applies each edit to a valid document and compares the `Debug` form of the result.
    fn run(rows: &[(Edit, &str)]) {
        for (i, (edit, want)) in rows.iter().enumerate() {
            let mut d = base();
            edit(&mut d);
            assert_eq!(format!("{:?}", d.validate()), *want, "row {i}");
        }
    }

    fn out(page: u64, axis: &str, px: f64) -> String {
        let err = format!("OutOfRange {{ axis: {axis}, px: {px:?} }}");
        format!("Err(PageSize {{ page: PageId({page}), error: {err} }})")
    }

    fn next(next_id: u64, max_id: u64) -> String {
        format!("Err(NextIdNotAboveMax {{ next_id: {next_id}, max_id: {max_id} }})")
    }

    fn root(kind: &str, page: u64, node: u64) -> String {
        format!("Err({kind} {{ page: PageId({page}), node: NodeId({node}) }})")
    }

    fn key(page: u64, key: u64, id: u64) -> String {
        let (page, key, id) = (format!("PageId({page})"), format!("NodeId({key})"), id);
        format!("Err(ArenaKeyMismatch {{ page: {page}, key: {key}, id: NodeId({id}) }})")
    }

    #[test]
    fn valid_documents_pass() {
        assert_eq!(Document::default().validate(), Ok(()));
        assert_eq!(base().validate(), Ok(()));
        let base = ElementBase::new(NodeId::new(2), Placement::new(0.0, 0.0, 9.0, 9.0));
        let mut group = Node::new(base, NodeKind::Group(GroupNode {}));
        group.children = vec![NodeId::new(3), NodeId::new(4)];
        let ext = ExtensionNode {
            type_id: "x".to_string(),
            schema_version: 1,
            payload: json!([1]),
        };
        let mut p = page(1, &[]);
        p.roots = vec![NodeId::new(2)];
        for n in [
            Arc::new(group),
            node(3, NodeKind::Shape(ShapeNode::new(ShapeGeometry::Rect))),
            node(4, NodeKind::Extension(ext)),
        ] {
            p.nodes.insert(n.base.id, n);
        }
        let d = test_document(5, Dpi::default(), vec![Arc::new(p)]);
        assert_eq!(d.validate(), Ok(()));
    }

    #[test]
    fn header_rows() {
        run(&[
            (
                |d| d.schema_version = 0,
                "Err(UnsupportedSchemaVersion { found: 0 })",
            ),
            (
                |d| d.schema_version = 2,
                "Err(UnsupportedSchemaVersion { found: 2 })",
            ),
            (
                |d| d.schema_version = u32::MAX,
                "Err(UnsupportedSchemaVersion { found: 4294967295 })",
            ),
            (|d| d.dpi = Dpi::new(0), "Err(InvalidDpi { dpi: 0 })"),
            (|d| d.dpi = Dpi::new(1), "Ok(())"),
            (|d| d.dpi = Dpi::new(10_000), "Ok(())"),
            (
                |d| d.dpi = Dpi::new(10_001),
                "Err(InvalidDpi { dpi: 10001 })",
            ),
            (
                |d| d.dpi = Dpi::new(u32::MAX),
                "Err(InvalidDpi { dpi: 4294967295 })",
            ),
        ]);
    }

    #[test]
    fn id_rows() {
        let max = next(u64::MAX, u64::MAX);
        run(&[
            (|d| push(d, 1, &[]), "Err(DuplicateId { id: 1 })"),
            (|d| push(d, 7, &[3]), "Err(DuplicateId { id: 3 })"),
            (|d| push(d, 2, &[]), "Err(DuplicateId { id: 2 })"),
            (|d| rekey(first(d)), &key(1, 5, 3)),
            (|d| two(d, rekey), &key(7, 5, 9)),
            (|d| ten(d, 11), "Ok(())"),
            (|d| ten(d, 10), &next(10, 10)),
            (|d| ten(d, 5), &next(5, 10)),
            (|d| ten(d, 0), &next(0, 10)),
            (
                |d| {
                    d.pages.clear();
                    d.next_id = 0;
                },
                "Ok(())",
            ),
            (|d| at_max(d, u64::MAX, &[]), &max),
            (|d| at_max(d, 7, &[u64::MAX]), &max),
        ]);
    }

    #[test]
    fn page_rows() {
        fn a0(d: &mut Document, dpi: u32) {
            let side = |v| Length::new(v, Unit::Mm);
            first(d).size = PageSize {
                width: side(841.0),
                height: side(1189.0),
            };
            d.dpi = Dpi::new(dpi);
        }
        let (low, high) = (1.0f64.next_down(), 16_384.0f64.next_up());
        let a0_px = Length::new(1189.0, Unit::Mm).to_px(351.0).unwrap();
        run(&[
            (|d| first(d).size = px(1.0, 1.0), "Ok(())"),
            (|d| first(d).size = px(16_384.0, 16_384.0), "Ok(())"),
            (
                |d| first(d).size = px(1.0f64.next_down(), 5.0),
                &out(1, "Width", low),
            ),
            (
                |d| first(d).size = px(5.0, 16_384.0f64.next_up()),
                &out(1, "Height", high),
            ),
            (|d| first(d).size = px(0.0, 5.0), &out(1, "Width", 0.0)),
            (|d| first(d).size = px(-5.0, 5.0), &out(1, "Width", -5.0)),
            (|d| a0(d, 300), "Ok(())"),
            (|d| a0(d, 350), "Ok(())"),
            (|d| a0(d, 351), &out(1, "Height", a0_px)),
            (|d| two(d, |p| p.size = px(0.0, 5.0)), &out(7, "Width", 0.0)),
        ]);
        for width in [f64::NAN, f64::INFINITY] {
            let mut d = base();
            first(&mut d).size = px(width, 5.0);
            let Err(ValidationError::PageSize {
                page,
                error: PageSizeError::Conversion { axis, .. },
            }) = d.validate()
            else {
                panic!("expected a conversion error for {width}");
            };
            assert_eq!((page, axis), (PageId::new(1), Axis::Width));
        }
    }

    #[test]
    fn root_rows() {
        fn elsewhere(d: &mut Document) {
            push(d, 7, &[9]);
            roots(first(d), &[2, 9]);
        }
        run(&[
            (|d| roots(first(d), &[42]), &root("DanglingRoot", 1, 42)),
            (
                |d| two(d, |p| roots(p, &[42])),
                &root("DanglingRoot", 7, 42),
            ),
            (|d| roots(first(d), &[3, 3]), &root("DuplicateRoot", 1, 3)),
            (
                |d| two(d, |p| roots(p, &[9, 9])),
                &root("DuplicateRoot", 7, 9),
            ),
            (
                elsewhere,
                "Err(DanglingRoot { page: PageId(1), node: NodeId(9) })",
            ),
        ]);
    }

    #[test]
    fn earlier_rules_win() {
        run(&[
            // P1 before P2.
            (
                |d| {
                    d.schema_version = 2;
                    push(d, 1, &[]);
                },
                "Err(UnsupportedSchemaVersion { found: 2 })",
            ),
            // P2 before P3.
            (
                |d| {
                    push(d, 1, &[]);
                    first(d).size = px(0.0, 5.0);
                },
                "Err(DuplicateId { id: 1 })",
            ),
            // P4 inside the pass: a lower id wins over a higher id's earlier-listed check.
            (
                |d| {
                    node_mut(d, 2, |n| n.base.opacity = 2.0);
                    node_mut(d, 3, |n| n.children = vec![NodeId::new(99)]);
                },
                &range(2, "Opacity", 2.0),
            ),
            // Inside P1: schema before DPI.
            (
                |d| {
                    d.schema_version = 2;
                    d.dpi = Dpi::new(0);
                },
                "Err(UnsupportedSchemaVersion { found: 2 })",
            ),
            // Inside P2: duplicate id before `next_id`.
            (
                |d| {
                    push(d, 1, &[]);
                    d.next_id = 0;
                },
                "Err(DuplicateId { id: 1 })",
            ),
            // Inside P2: duplicate id before arena key.
            (
                |d| {
                    push(d, 1, &[]);
                    rekey(first(d));
                },
                "Err(DuplicateId { id: 1 })",
            ),
            // Inside P3: dangling root before repeated root.
            (
                |d| roots(first(d), &[42, 42]),
                "Err(DanglingRoot { page: PageId(1), node: NodeId(42) })",
            ),
            // Inside P3, each check runs over all pages before the next one.
            (
                |d| {
                    roots(first(d), &[42]);
                    two(d, |p| p.size = px(0.0, 5.0));
                },
                &out(7, "Width", 0.0),
            ),
        ]);
    }

    fn node_mut(d: &mut Document, id: u64, edit: impl FnOnce(&mut Node)) {
        let node = first(d).nodes.get_mut(&NodeId::new(id)).unwrap();
        edit(Arc::make_mut(node));
    }

    fn non_finite(node: u64, field: &str) -> String {
        format!("Err(NonFinite {{ node: NodeId({node}), field: {field} }})")
    }

    fn range(node: u64, field: &str, value: f64) -> String {
        let (node, field) = (format!("NodeId({node})"), field);
        format!("Err(OutOfRange {{ node: {node}, field: {field}, value: {value:?} }})")
    }

    fn extension(type_id: &str, payload: Value) -> NodeKind {
        NodeKind::Extension(ExtensionNode {
            type_id: type_id.to_string(),
            schema_version: 1,
            payload,
        })
    }

    /// Nested `levels` deep: `[]` is 1, `[[]]` is 2.
    fn nested(levels: usize, object: bool) -> Value {
        (1..levels).fold(if object { json!({}) } else { json!([]) }, |inner, _| {
            if object {
                json!({ "a": inner })
            } else {
                Value::Array(vec![inner])
            }
        })
    }

    fn shape_with(fill: Color) -> NodeKind {
        let mut shape = ShapeNode::new(ShapeGeometry::Rect);
        shape.fill = Some(fill);
        NodeKind::Shape(shape)
    }

    #[test]
    fn node_value_rows() {
        type Set = fn(&mut Node, f64);
        let nan = f64::NAN;
        let rows: Vec<(Set, f64, String)> = vec![
            (|n, v| n.base.opacity = v as f32, 0.0, "Ok(())".into()),
            (|n, v| n.base.opacity = v as f32, -0.0, "Ok(())".into()),
            (|n, v| n.base.opacity = v as f32, 1.0, "Ok(())".into()),
            (|n, v| n.base.opacity = v as f32, 1e-45, "Ok(())".into()),
            (
                |n, v| n.base.opacity = v as f32,
                -0.0000001,
                range(2, "Opacity", f64::from(-0.0000001f32)),
            ),
            (
                |n, v| n.base.opacity = v as f32,
                1.0000001,
                range(2, "Opacity", f64::from(1.0000001f32)),
            ),
            (
                |n, v| n.base.opacity = v as f32,
                2.0,
                range(2, "Opacity", 2.0),
            ),
            (
                |n, v| n.base.opacity = v as f32,
                nan,
                non_finite(2, "Opacity"),
            ),
            (
                |n, v| n.base.opacity = v as f32,
                f64::INFINITY,
                non_finite(2, "Opacity"),
            ),
            (
                |n, v| n.base.opacity = v as f32,
                f64::NEG_INFINITY,
                non_finite(2, "Opacity"),
            ),
            (|n, v| n.base.placement.x = v, nan, non_finite(2, "X")),
            (
                |n, v| n.base.placement.y = v,
                f64::INFINITY,
                non_finite(2, "Y"),
            ),
            (
                |n, v| n.base.placement.rotation_deg = v,
                f64::NEG_INFINITY,
                non_finite(2, "RotationDeg"),
            ),
            (|n, v| n.base.placement.skew = v, nan, non_finite(2, "Skew")),
            (|n, v| n.base.placement.w = v, -1.0, range(2, "W", -1.0)),
            (
                |n, v| n.base.placement.h = v,
                -1e-300,
                range(2, "H", -1e-300),
            ),
            (
                |n, v| n.base.placement.skew = v,
                90.0,
                range(2, "Skew", 90.0),
            ),
            (
                |n, v| n.base.placement.skew = v,
                -90.0,
                range(2, "Skew", -90.0),
            ),
            (|n, v| n.base.placement.w = v, -0.0, "Ok(())".into()),
            (|n, v| n.base.placement.w = v, 0.0, "Ok(())".into()),
            (
                |n, v| n.base.placement.rotation_deg = v,
                720.0,
                "Ok(())".into(),
            ),
            (|n, v| n.base.placement.skew = v, 89.999, "Ok(())".into()),
            (|n, v| n.base.placement.skew = v, -89.999, "Ok(())".into()),
            (
                |n, v| n.kind = shape_with(Color::Srgb(Rgba::new(v as f32, 0.0, 0.0, 1.0))),
                nan,
                non_finite(2, "FillChannel"),
            ),
            (
                |n, v| n.kind = shape_with(Color::Srgb(Rgba::new(0.0, 0.0, 0.0, v as f32))),
                nan,
                non_finite(2, "FillAlpha"),
            ),
            (
                |n, v| n.kind = shape_with(Color::Srgb(Rgba::new(0.0, 0.0, 0.0, v as f32))),
                1.5,
                range(2, "FillAlpha", 1.5),
            ),
            (
                |n, v| n.kind = shape_with(Color::Srgb(Rgba::new(v as f32, 1.75, 0.0, 0.5))),
                -0.5,
                "Ok(())".into(),
            ),
            (
                |n, v| {
                    n.kind = shape_with(Color::LinearSrgb(LinearRgba::new(0.0, v as f32, 0.0, 1.0)))
                },
                nan,
                non_finite(2, "FillChannel"),
            ),
            (
                |n, v| {
                    n.kind = shape_with(Color::LinearSrgb(LinearRgba::new(0.0, 0.0, 0.0, v as f32)))
                },
                1.5,
                range(2, "FillAlpha", 1.5),
            ),
        ];
        for (i, (set, value, want)) in rows.iter().enumerate() {
            let mut d = base();
            node_mut(&mut d, 2, |n| set(n, *value));
            assert_eq!(format!("{:?}", d.validate()), *want, "row {i}");
        }
    }

    fn stroked(color: Color, width: f64, radius: f64) -> NodeKind {
        let mut shape = ShapeNode::new(ShapeGeometry::Rect);
        shape.stroke = Some(ShapeStroke::new(color, width));
        shape.corner_radius = radius;
        NodeKind::Shape(shape)
    }

    // S38-02: the outline and the corner radius are validated like the fill.
    #[test]
    fn s38_02_outline_and_corner_radius_rows() {
        type Set = fn(&mut Node, f64);
        let black = |a: f32| Color::Srgb(Rgba::new(0.0, 0.0, 0.0, a));
        let rows: Vec<(Set, f64, String)> = vec![
            (
                |n, v| n.kind = stroked(Color::Srgb(Rgba::new(0.0, 0.0, 0.0, 1.0)), v, 0.0),
                0.0,
                "Ok(())".into(),
            ),
            (
                |n, v| n.kind = stroked(Color::Srgb(Rgba::new(0.0, 0.0, 0.0, 1.0)), v, 0.0),
                ShapeStroke::MAX_WIDTH,
                "Ok(())".into(),
            ),
            (
                |n, v| n.kind = stroked(Color::Srgb(Rgba::new(0.0, 0.0, 0.0, 1.0)), v, 0.0),
                ShapeStroke::MAX_WIDTH + 0.5,
                range(2, "StrokeWidth", ShapeStroke::MAX_WIDTH + 0.5),
            ),
            (
                |n, v| n.kind = stroked(Color::Srgb(Rgba::new(0.0, 0.0, 0.0, 1.0)), v, 0.0),
                -0.5,
                range(2, "StrokeWidth", -0.5),
            ),
            (
                |n, v| n.kind = stroked(Color::Srgb(Rgba::new(0.0, 0.0, 0.0, 1.0)), v, 0.0),
                f64::NAN,
                non_finite(2, "StrokeWidth"),
            ),
            (
                |n, v| n.kind = stroked(Color::Srgb(Rgba::new(v as f32, 0.0, 0.0, 1.0)), 1.0, 0.0),
                f64::NAN,
                non_finite(2, "StrokeChannel"),
            ),
            (
                |n, v| n.kind = stroked(Color::Srgb(Rgba::new(v as f32, 0.0, 0.0, 1.0)), 1.0, 0.0),
                -0.5,
                "Ok(())".into(),
            ),
            (
                |n, v| n.kind = stroked(Color::Srgb(Rgba::new(0.0, 0.0, 0.0, v as f32)), 1.0, 0.0),
                1.5,
                range(2, "StrokeAlpha", 1.5),
            ),
            (
                |n, v| n.kind = stroked(Color::Srgb(Rgba::new(0.0, 0.0, 0.0, v as f32)), 1.0, 0.0),
                f64::INFINITY,
                non_finite(2, "StrokeAlpha"),
            ),
            (
                |n, v| n.kind = stroked(Color::Srgb(Rgba::new(0.0, 0.0, 0.0, 1.0)), 1.0, v),
                0.0,
                "Ok(())".into(),
            ),
            (
                |n, v| n.kind = stroked(Color::Srgb(Rgba::new(0.0, 0.0, 0.0, 1.0)), 1.0, v),
                1e9,
                "Ok(())".into(),
            ),
            (
                |n, v| n.kind = stroked(Color::Srgb(Rgba::new(0.0, 0.0, 0.0, 1.0)), 1.0, v),
                -1e-9,
                range(2, "CornerRadius", -1e-9),
            ),
            (
                |n, v| n.kind = stroked(Color::Srgb(Rgba::new(0.0, 0.0, 0.0, 1.0)), 1.0, v),
                f64::INFINITY,
                non_finite(2, "CornerRadius"),
            ),
        ];
        for (i, (set, value, want)) in rows.iter().enumerate() {
            let mut d = base();
            node_mut(&mut d, 2, |n| set(n, *value));
            assert_eq!(format!("{:?}", d.validate()), *want, "row {i}");
        }
        // A radius with no outline is checked too.
        let mut d = base();
        node_mut(&mut d, 2, |n| {
            let mut shape = ShapeNode::new(ShapeGeometry::Ellipse);
            shape.corner_radius = -1.0;
            n.kind = NodeKind::Shape(shape);
        });
        assert_eq!(
            format!("{:?}", d.validate()),
            range(2, "CornerRadius", -1.0)
        );
        // The fill is reported before the outline.
        let mut d = base();
        node_mut(&mut d, 2, |n| {
            let mut shape = ShapeNode::new(ShapeGeometry::Rect);
            shape.fill = Some(black(2.0));
            shape.stroke = Some(ShapeStroke::new(black(3.0), 1.0));
            n.kind = NodeKind::Shape(shape);
        });
        assert_eq!(format!("{:?}", d.validate()), range(2, "FillAlpha", 2.0));
    }

    #[test]
    fn validation_does_not_clamp() {
        let mut d = base();
        node_mut(&mut d, 2, |n| n.base.opacity = 1.5);
        assert!(d.validate().is_err());
        let node = first(&mut d).nodes[&NodeId::new(2)].clone();
        assert_eq!(node.base.opacity.to_bits(), 1.5f32.to_bits());
    }

    #[test]
    fn extension_rows() {
        fn ext(d: &mut Document, type_id: &str, payload: Value) {
            node_mut(d, 2, |n| n.kind = extension(type_id, payload));
        }
        let deep =
            |depth: usize| format!("Err(ExtensionTooDeep {{ node: NodeId(2), depth: {depth} }})");
        let rows: [(Value, String); 7] = [
            (nested(64, false), "Ok(())".into()),
            (nested(65, false), deep(65)),
            (nested(65, true), deep(65)),
            (json!(7), "Ok(())".into()),
            (json!([]), "Ok(())".into()),
            (nested(1_000, false), deep(1_000)),
            (json!({"a": [1, {"b": null}]}), "Ok(())".into()),
        ];
        for (i, (payload, want)) in rows.into_iter().enumerate() {
            let mut d = base();
            ext(&mut d, "x", payload);
            assert_eq!(format!("{:?}", d.validate()), want, "row {i}");
        }
        let mut d = base();
        ext(&mut d, "", json!(null));
        assert_eq!(
            format!("{:?}", d.validate()),
            "Err(EmptyExtensionTypeId { node: NodeId(2) })"
        );
    }

    #[test]
    fn child_rows() {
        let leaf = |kind: NodeKind| {
            move |d: &mut Document| {
                node_mut(d, 2, |n| {
                    n.kind = kind.clone();
                    n.children = vec![NodeId::new(3)];
                });
            }
        };
        let dangling = |d: &mut Document| {
            node_mut(d, 2, |n| n.children = vec![NodeId::new(3), NodeId::new(99)]);
        };
        let elsewhere = |d: &mut Document| {
            push(d, 7, &[8]);
            node_mut(d, 2, |n| n.children = vec![NodeId::new(8)]);
        };
        let shape = NodeKind::Shape(ShapeNode::new(ShapeGeometry::Rect));
        let rows: [(EditRef, &str); 5] = [
            (&leaf(shape), "Err(ChildrenOnLeaf { node: NodeId(2) })"),
            (
                &leaf(extension("x", json!(null))),
                "Err(ChildrenOnLeaf { node: NodeId(2) })",
            ),
            (
                &dangling,
                "Err(DanglingChild { parent: NodeId(2), child: NodeId(99) })",
            ),
            (
                &elsewhere,
                "Err(DanglingChild { parent: NodeId(2), child: NodeId(8) })",
            ),
            (&|d| set(d, &[2, 3], &[2], &[(2, &[3])]), "Ok(())"),
        ];
        for (i, (edit, want)) in rows.iter().enumerate() {
            let mut d = base();
            edit(&mut d);
            assert_eq!(format!("{:?}", d.validate()), *want, "row {i}");
        }
    }

    #[test]
    fn nodes_are_checked_in_ascending_id_and_check_order() {
        fn nan_x(n: &mut Node) {
            n.base.placement.x = f64::NAN;
        }
        let with_eight = |d: &mut Document| {
            first(d)
                .nodes
                .insert(NodeId::new(8), node(8, NodeKind::Group(GroupNode {})));
            first(d).roots.push(NodeId::new(8));
        };
        let too_deep = || extension("x", nested(65, false));
        let rows: [(EditRef, String); 7] = [
            (
                &|d| {
                    with_eight(d);
                    node_mut(d, 8, nan_x);
                    node_mut(d, 3, nan_x);
                },
                non_finite(3, "X"),
            ),
            (
                &|d| {
                    node_mut(d, 2, |n| n.children = vec![NodeId::new(99)]);
                    node_mut(d, 3, nan_x);
                },
                "Err(DanglingChild { parent: NodeId(2), child: NodeId(99) })".to_string(),
            ),
            (
                &|d| {
                    node_mut(d, 2, |n| {
                        n.base.opacity = 2.0;
                        nan_x(n);
                    });
                },
                non_finite(2, "X"),
            ),
            (
                &|d| {
                    node_mut(d, 2, |n| {
                        n.kind = extension("", json!(null));
                        n.base.opacity = 2.0;
                    });
                },
                range(2, "Opacity", 2.0),
            ),
            (
                &|d| {
                    node_mut(d, 2, |n| {
                        n.kind = extension("", nested(65, false));
                    });
                },
                "Err(EmptyExtensionTypeId { node: NodeId(2) })".to_string(),
            ),
            (
                &|d| {
                    node_mut(d, 2, |n| {
                        n.kind = too_deep();
                        n.children = vec![NodeId::new(3)];
                    });
                },
                "Err(ExtensionTooDeep { node: NodeId(2), depth: 65 })".to_string(),
            ),
            (
                &|d| {
                    node_mut(d, 2, |n| {
                        n.kind = NodeKind::Shape(ShapeNode::new(ShapeGeometry::Rect));
                        n.children = vec![NodeId::new(99)];
                    });
                },
                "Err(ChildrenOnLeaf { node: NodeId(2) })".to_string(),
            ),
        ];
        for (i, (edit, want)) in rows.iter().enumerate() {
            let mut d = base();
            edit(&mut d);
            assert_eq!(format!("{:?}", d.validate()), *want, "row {i}");
        }
    }

    /// Replaces page 1 with groups `nodes`, the given `top` roots and `(parent, children)` edges.
    fn set(d: &mut Document, nodes: &[u64], top: &[u64], edges: &[(u64, &[u64])]) {
        let mut p = page(1, nodes);
        roots(&mut p, top);
        for &(parent, kids) in edges {
            let node = p.nodes.get_mut(&NodeId::new(parent)).unwrap();
            Arc::make_mut(node).children = kids.iter().map(|&k| NodeId::new(k)).collect();
        }
        d.pages[0] = Arc::new(p);
    }

    fn multiple(node: u64) -> String {
        format!("Err(MultipleParents {{ node: NodeId({node}) }})")
    }

    fn root_parent(node: u64, parent: u64) -> String {
        format!("Err(RootHasParent {{ node: NodeId({node}), parent: NodeId({parent}) }})")
    }

    fn orphan(node: u64) -> String {
        format!("Err(Orphan {{ node: NodeId({node}) }})")
    }

    fn cycle(node: u64) -> String {
        format!("Err(Cycle {{ node: NodeId({node}) }})")
    }

    #[test]
    fn parent_and_reachability_rows() {
        let rows: Vec<(EditRef, String)> = vec![
            // Valid trees, including a chain that needs more than one hop.
            (
                &|d| set(d, &[2, 3, 4], &[2], &[(2, &[3]), (3, &[4])]),
                "Ok(())".into(),
            ),
            (
                &|d| set(d, &[2, 3, 4], &[2, 3], &[(2, &[4])]),
                "Ok(())".into(),
            ),
            // P5: two groups list the same child; one group lists it twice.
            (
                &|d| set(d, &[2, 3, 4], &[2, 3], &[(2, &[4]), (3, &[4])]),
                multiple(4),
            ),
            (&|d| set(d, &[2, 4], &[2], &[(2, &[4, 4])]), multiple(4)),
            // P5: a root listed as a child, also of itself.
            (
                &|d| set(d, &[2, 3], &[2, 3], &[(2, &[3])]),
                root_parent(3, 2),
            ),
            (&|d| set(d, &[2], &[2], &[(2, &[2])]), root_parent(2, 2)),
            // P5: neither root nor child; on the second page too.
            (&|d| set(d, &[2, 3], &[2], &[]), orphan(3)),
            (&|d| two(d, |p| p.roots.truncate(1)), orphan(9)),
            // P6: a self-parent, a ring, and a subtree below a ring (the lowest unreached id is not on the ring).
            (&|d| set(d, &[2, 3], &[2], &[(3, &[3])]), cycle(3)),
            (
                &|d| set(d, &[2, 3, 4, 5], &[2], &[(3, &[4]), (4, &[5]), (5, &[3])]),
                cycle(3),
            ),
            (
                &|d| {
                    let edges: &[(u64, &[u64])] = &[(3, &[4]), (4, &[5]), (5, &[3, 2])];
                    set(d, &[2, 3, 4, 5, 6], &[6], edges);
                },
                cycle(2),
            ),
        ];
        for (i, (edit, want)) in rows.iter().enumerate() {
            let mut d = base();
            edit(&mut d);
            assert_eq!(format!("{:?}", d.validate()), *want, "row {i}");
        }
    }

    #[test]
    fn parent_checks_and_passes_run_in_order() {
        let rows: Vec<(EditRef, String)> = vec![
            // 15 before 16 although the root with a parent has the lower id.
            (
                &|d| set(d, &[2, 3, 4], &[2, 3], &[(2, &[4]), (3, &[2, 4])]),
                multiple(4),
            ),
            // 16 before 17 although the orphan has the lower id.
            (
                &|d| set(d, &[2, 3, 4], &[3, 4], &[(3, &[4])]),
                root_parent(4, 3),
            ),
            // 15 before 17 although the orphan has the lower id.
            (
                &|d| set(d, &[2, 3, 4, 5], &[3, 4], &[(3, &[5]), (4, &[5])]),
                multiple(5),
            ),
            // Each check runs over all pages before the next one.
            (
                &|d| {
                    set(d, &[2, 3], &[2], &[]);
                    two(d, |p| {
                        let node = p.nodes.get_mut(&NodeId::new(8)).unwrap();
                        Arc::make_mut(node).children = vec![NodeId::new(9)];
                    });
                },
                root_parent(9, 8),
            ),
            // P4 before P5.
            (
                &|d| {
                    set(d, &[2, 3], &[3], &[]);
                    node_mut(d, 3, |n| n.base.opacity = 2.0);
                },
                range(3, "Opacity", 2.0),
            ),
            // P5 before P6, whatever the ids.
            (
                &|d| set(d, &[2, 3, 4], &[2], &[(3, &[4]), (4, &[3])]),
                cycle(3),
            ),
            (
                &|d| set(d, &[2, 3, 4, 5], &[2], &[(3, &[4]), (4, &[3])]),
                orphan(5),
            ),
            (
                &|d| {
                    let edges: &[(u64, &[u64])] = &[(3, &[4]), (4, &[3]), (2, &[5]), (6, &[5])];
                    set(d, &[2, 3, 4, 5, 6], &[2, 6], edges);
                },
                multiple(5),
            ),
        ];
        for (i, (edit, want)) in rows.iter().enumerate() {
            let mut d = base();
            edit(&mut d);
            assert_eq!(format!("{:?}", d.validate()), *want, "row {i}");
        }
    }

    #[test]
    fn a_deep_chain_validates_without_overflow() {
        // Deep enough that a recursive walk overflows the 2 MiB test-thread stack
        // (threads are banned in domain crates, so the stack size cannot be lowered).
        const DEPTH: u64 = 100_000;
        let mut d = base();
        let mut p = page(1, &[]);
        roots(&mut p, &[2]);
        for id in 2..2 + DEPTH {
            let mut n = Node::new(
                ElementBase::new(NodeId::new(id), Placement::new(0.0, 0.0, 1.0, 1.0)),
                NodeKind::Group(GroupNode {}),
            );
            if id + 1 < 2 + DEPTH {
                n.children = vec![NodeId::new(id + 1)];
            }
            p.nodes.insert(NodeId::new(id), Arc::new(n));
        }
        d.pages[0] = Arc::new(p);
        d.next_id = 2 + DEPTH;
        assert_eq!(d.validate(), Ok(()));
    }

    // Spec 027, S27-02: a span's size must be in (0, MAX_SIZE] and its colour
    // finite with alpha in [0, 1].
    #[test]
    fn s27_02_text_spans_are_validated() {
        use crate::{NodeField, SpanStyle, TextNode};
        use gafira_color::Rgba;
        let style = |size: f64, a: f32| SpanStyle {
            font: "Inter".to_owned(),
            size,
            color: Color::Srgb(Rgba::new(0.0, 0.0, 0.0, a)),
            bold: false,
            italic: false,
            gradient: None,
        };
        let check = |size: f64, a: f32| {
            let mut page = page(1, &[]);
            let n = node(2, NodeKind::Text(TextNode::plain("Hola", &style(size, a))));
            page.nodes.insert(NodeId::new(2), n);
            page.roots.push(NodeId::new(2));
            crate::document::test_document(3, crate::Dpi::default(), vec![Arc::new(page)])
                .validate()
        };
        assert_eq!(check(32.0, 1.0), Ok(()));
        assert_eq!(check(TextNode::MAX_SIZE, 1.0), Ok(()));
        for (size, a, field) in [
            (0.0, 1.0, NodeField::FontSize),
            (TextNode::MAX_SIZE + 1.0, 1.0, NodeField::FontSize),
            (32.0, 1.5, NodeField::TextColorAlpha),
        ] {
            assert!(
                matches!(check(size, a), Err(ValidationError::OutOfRange { field: f, .. }) if f == field),
                "{size} {a}"
            );
        }
        assert!(matches!(
            check(f64::NAN, 1.0),
            Err(ValidationError::NonFinite {
                field: NodeField::FontSize,
                ..
            })
        ));
    }

    // Spec 029, S29-04: an image must show an asset of the table.
    #[test]
    fn s29_04_an_image_without_its_asset_does_not_validate() {
        use crate::timeline::{AssetEntry, AssetRef, AssetStorage};
        use crate::{ContentHash, ImageNode};
        let asset = AssetRef::new(ContentHash([7; 32]));
        let mut page = page(1, &[]);
        page.nodes.insert(
            NodeId::new(2),
            node(2, NodeKind::Image(ImageNode::new(asset))),
        );
        page.roots.push(NodeId::new(2));
        let mut doc =
            crate::document::test_document(3, crate::Dpi::default(), vec![Arc::new(page)]);
        assert_eq!(
            doc.validate(),
            Err(ValidationError::DanglingImage {
                node: NodeId::new(2)
            })
        );
        doc.assets.insert(
            asset,
            AssetEntry {
                name: "dot.png".to_owned(),
                storage: AssetStorage::Embedded,
            },
        );
        assert_eq!(doc.validate(), Ok(()));
    }
}
