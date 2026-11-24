//! `SetProperty`: one property of one or more nodes (ARQ-81, ADR-03).

use std::any::Any;

use gafira_color::Color;
use gafira_doc::msg::{CommandLabel, MessageKey};
use gafira_doc::{
    BlendMode, DocMut, Document, Dpi, GridSettings, GuideSet, Margins, Node, NodeId, NodeKind,
    Page, PageId, PageSize, Placement, RichText, ShapeGeometry, ShapeStroke, ShapeText, TextNode,
    ValidationError, check_bleed,
};
use gafira_geom::Length;

use crate::{Applied, Command, CommandError, DirtySet, GestureId, MergeKey};

/// A property value of a node, tagged with the property it belongs to.
///
/// `Fill`, `Geometry`, `Stroke` and `CornerRadius` only apply to `Shape`
/// nodes. The `Page` variants,
/// `Bleed`, `Margins`, `Guides` and `Grid` target a page and `Dpi` the
/// document; build those commands with
/// [`SetProperty::page`] and [`SetProperty::document`].
#[derive(Clone, Debug, PartialEq)]
pub enum Property {
    Name(Option<String>),
    Placement(Placement),
    Opacity(f32),
    Blend(BlendMode),
    Visible(bool),
    Locked(bool),
    Fill(Option<Color>),
    Geometry(ShapeGeometry),
    /// The outline of a shape; `None` removes it (spec 038).
    Stroke(Option<ShapeStroke>),
    /// The corner radius of a shape, in px (spec 038).
    CornerRadius(f64),
    /// The rich text of a text element (spec 027).
    TextContent(RichText),
    /// How much a text element bends (FD-44), in `[-100, 100]`.
    TextCurve(f64),
    /// The effects of a text (spec 089).
    TextEffects(Vec<gafira_doc::TextEffect>),
    /// Where the lines of a text element sit across its box.
    TextAlign(gafira_doc::TextAlign),
    /// How a text box follows its words: wide as its lines, or with a
    /// fixed width that wraps them (spec 095).
    TextSizing(gafira_doc::TextSizing),
    /// The letter spacing (thousandths of an em) and line height (`None`
    /// for the font's own) of a text element (FD-41).
    TextSpacing(f64, Option<f64>),
    /// The gradient of a text element's letters; `None` fills them with
    /// their colour again (spec 070).
    TextGradient(Option<gafira_doc::Gradient>),
    /// The text inside a shape; `None` leaves it empty (spec 055).
    ShapeText(Option<ShapeText>),
    /// A whole line: its ends, caps, connections, colour and width (spec
    /// 058).
    Line(gafira_doc::LineNode),
    /// The gradient fill of a shape; `None` fills it with its colour again
    /// (spec 063).
    Gradient(Option<gafira_doc::Gradient>),
    /// The light of a shape (spec 065); `None` takes it off.
    Effect(Option<gafira_doc::ShapeEffect>),
    /// The photo of a frame (spec 066, D-107); `None` empties it. One
    /// change, so putting, replacing or removing a photo is one undo step.
    FrameContent(Option<gafira_doc::FrameContent>),
    /// How the photo of a frame sits in it (spec 066).
    FrameFit(gafira_doc::Fit),
    /// Where the photo of a frame sits, how large and how turned:
    /// `(offset, scale, degrees)` (S66-07: crop mode).
    FrameCrop((f64, f64), f64, f64),
    /// A whole QR code (spec 080): what it says and how it looks.
    Qr(Box<gafira_doc::QrNode>),
    /// A whole table (spec 081).
    Table(Box<gafira_doc::TableNode>),
    /// What an image shows and where it comes from (spec 084).
    Image(gafira_doc::ImageNode),
    /// A whole chart (spec 083).
    Chart(Box<gafira_doc::ChartNode>),
    PageName(String),
    /// The background of a page (spec 069); `None` makes it white again.
    PageBackground(Option<gafira_doc::PageBackground>),
    PageSize(PageSize),
    PageLocked(bool),
    PageHidden(bool),
    /// The page bleed, 0 to 10 mm (FD-05).
    Bleed(Length),
    /// The page margins (FD-05).
    Margins(Margins),
    /// The whole guide set of the page, in order (FD-03, FD-04).
    Guides(GuideSet),
    /// The page grid settings (FD-07).
    Grid(GridSettings),
    Dpi(Dpi),
}

impl Property {
    /// The merge-key kind of this property.
    fn kind(&self) -> &'static str {
        match self {
            Self::Name(_) => "set_property.name",
            Self::Placement(_) => "set_property.placement",
            Self::Opacity(_) => "set_property.opacity",
            Self::Blend(_) => "set_property.blend",
            Self::Visible(_) => "set_property.visible",
            Self::Locked(_) => "set_property.locked",
            Self::Fill(_) => "set_property.fill",
            Self::Geometry(_) => "set_property.geometry",
            Self::Stroke(_) => "set_property.stroke",
            Self::CornerRadius(_) => "set_property.corner_radius",
            Self::ShapeText(_) => "set_property.shape_text",
            Self::Line(_) => "set_property.line",
            Self::Gradient(_) => "set_property.gradient",
            Self::Effect(_) => "set_property.effect",
            Self::FrameContent(_) => "set_property.frame_content",
            Self::FrameFit(_) => "set_property.frame_fit",
            Self::FrameCrop(..) => "set_property.frame_crop",
            Self::Qr(_) => "set_property.qr",
            Self::Table(_) => "set_property.table",
            Self::Chart(_) => "set_property.chart",
            Self::Image(_) => "set_property.image",
            Self::TextContent(_) => "set_property.text_content",
            Self::TextCurve(_) => "set_property.text_curve",
            Self::TextEffects(_) => "set_property.text_effects",
            Self::TextAlign(_) => "set_property.text_align",
            Self::TextSizing(_) => "set_property.text_sizing",
            Self::TextSpacing(..) => "set_property.text_spacing",
            Self::TextGradient(_) => "set_property.text_gradient",
            Self::PageName(_) => "set_property.page_name",
            Self::PageBackground(_) => "set_property.page_background",
            Self::PageSize(_) => "set_property.page_size",
            Self::PageLocked(_) => "set_property.page_locked",
            Self::PageHidden(_) => "set_property.page_hidden",
            Self::Bleed(_) => "set_property.bleed",
            Self::Margins(_) => "set_property.margins",
            Self::Guides(_) => "set_property.guides",
            Self::Grid(_) => "set_property.grid",
            Self::Dpi(_) => "set_property.dpi",
        }
    }

    /// Checks the value against the ranges `validate()` enforces.
    fn check(&self) -> Result<(), CommandError> {
        let unit = |v: f64| (0.0..=1.0).contains(&v);
        let (what, ok) = match self {
            Self::Placement(p) => (
                "placement",
                [p.x, p.y, p.w, p.h, p.rotation_deg, p.skew]
                    .iter()
                    .all(|v| v.is_finite())
                    && p.w >= 0.0
                    && p.h >= 0.0
                    && p.skew > -90.0
                    && p.skew < 90.0,
            ),
            Self::Opacity(o) => ("opacity", unit(f64::from(*o))),
            Self::Fill(Some(c)) => ("fill", c.is_finite() && unit(f64::from(c.alpha()))),
            Self::Stroke(Some(stroke)) => (
                "stroke",
                stroke.color.is_finite()
                    && unit(f64::from(stroke.color.alpha()))
                    && (0.0..=ShapeStroke::MAX_WIDTH).contains(&stroke.width),
            ),
            Self::CornerRadius(r) => ("corner_radius", r.is_finite() && *r >= 0.0),
            Self::TextContent(text) => ("text_content", spans_valid(text)),
            Self::TextSpacing(letters, lines) => (
                "text_spacing",
                TextNode::LETTER_SPACING.contains(letters)
                    && lines.is_none_or(|h| TextNode::LINE_HEIGHT.contains(&h)),
            ),
            Self::TextGradient(Some(gradient)) => ("text_gradient", gradient.problem().is_none()),
            Self::TextCurve(curve) => (
                "text_curve",
                (-TextNode::MAX_CURVE..=TextNode::MAX_CURVE).contains(curve),
            ),
            Self::TextEffects(effects) => (
                "text_effects",
                effects.len() <= gafira_doc::TextEffect::MAX
                    && effects.iter().all(gafira_doc::TextEffect::is_valid),
            ),
            Self::ShapeText(Some(text)) => ("shape_text", spans_valid(&text.content)),
            Self::Gradient(Some(gradient)) => ("gradient", gradient.problem().is_none()),
            Self::Effect(Some(effect)) => ("effect", effect.problem().is_none()),
            Self::PageBackground(Some(background)) => ("page_background", background.is_valid()),
            Self::FrameCrop(offset, scale, degrees) => (
                "frame_crop",
                offset.0.is_finite()
                    && offset.1.is_finite()
                    && scale.is_finite()
                    && *scale > 0.0
                    && degrees.is_finite(),
            ),
            Self::Qr(qr) => ("qr", qr.is_valid()),
            Self::Table(table) => ("table", table.is_valid()),
            Self::Chart(chart) => ("chart", chart.is_valid()),
            Self::FrameContent(Some(content)) => (
                "frame_content",
                content.natural.0 > 0 && content.natural.1 > 0,
            ),
            Self::Line(line) => (
                "line",
                line.color.is_finite()
                    && unit(f64::from(line.color.alpha()))
                    && (0.0..=gafira_doc::LineNode::MAX_WIDTH).contains(&line.width)
                    && line
                        .start
                        .at
                        .into_iter()
                        .chain(line.end.at)
                        .all(|v| (-1.0..=2.0).contains(&v)),
            ),
            _ => return Ok(()),
        };
        if ok {
            Ok(())
        } else {
            Err(CommandError::InvalidValue(what))
        }
    }

    /// What `node` holds for this property, or `None` for a shape property
    /// of a node that is not a shape.
    fn read(&self, node: &Node) -> Option<Property> {
        let base = &node.base;
        Some(match (self, &node.kind) {
            (Self::Name(_), _) => Self::Name(base.name.clone()),
            (Self::Placement(_), _) => Self::Placement(base.placement),
            (Self::Opacity(_), _) => Self::Opacity(base.opacity),
            (Self::Blend(_), _) => Self::Blend(base.blend),
            (Self::Visible(_), _) => Self::Visible(base.visible),
            (Self::Locked(_), _) => Self::Locked(base.locked),
            (Self::Fill(_), NodeKind::Shape(s)) => Self::Fill(s.fill),
            (Self::Geometry(_), NodeKind::Shape(s)) => Self::Geometry(s.geometry.clone()),
            (Self::Stroke(_), NodeKind::Shape(s)) => Self::Stroke(s.stroke),
            (Self::CornerRadius(_), NodeKind::Shape(s)) => Self::CornerRadius(s.corner_radius),
            (Self::TextContent(_), NodeKind::Text(t)) => Self::TextContent(t.content.clone()),
            (Self::TextCurve(_), NodeKind::Text(t)) => Self::TextCurve(t.curve),
            (Self::TextEffects(_), NodeKind::Text(t)) => Self::TextEffects(t.effects.clone()),
            (Self::TextAlign(_), NodeKind::Text(t)) => Self::TextAlign(t.align),
            (Self::TextSizing(_), NodeKind::Text(t)) => Self::TextSizing(t.sizing),
            (Self::TextSpacing(..), NodeKind::Text(t)) => {
                Self::TextSpacing(t.letter_spacing, t.line_height)
            }
            (Self::TextGradient(_), NodeKind::Text(t)) => Self::TextGradient(t.gradient.clone()),
            (Self::ShapeText(_), NodeKind::Shape(s)) => Self::ShapeText(s.text.clone()),
            (Self::Line(_), NodeKind::Line(l)) => Self::Line(l.clone()),
            (Self::Gradient(_), NodeKind::Shape(s)) => Self::Gradient(s.gradient.clone()),
            (Self::Effect(_), NodeKind::Shape(s)) => Self::Effect(s.effect.clone()),
            (Self::FrameContent(_), NodeKind::Frame(f)) => Self::FrameContent(f.content),
            (Self::FrameFit(_), NodeKind::Frame(f)) => Self::FrameFit(f.fit),
            (Self::FrameCrop(..), NodeKind::Frame(f)) => {
                Self::FrameCrop(f.content_offset, f.content_scale, f.content_rotation_deg)
            }
            (Self::Table(_), NodeKind::Table(t)) => Self::Table(t.clone()),
            (Self::Image(_), NodeKind::Image(i)) => Self::Image(i.clone()),
            (Self::Chart(_), NodeKind::Chart(c)) => Self::Chart(c.clone()),
            (Self::Qr(_), NodeKind::Qr(q)) => Self::Qr(q.clone()),
            (Self::Stroke(_), NodeKind::Frame(f)) => Self::Stroke(f.stroke),
            (Self::CornerRadius(_), NodeKind::Frame(f)) => Self::CornerRadius(f.corner_radius),
            _ => return None,
        })
    }

    /// Stores this value in `node`; a shape property skips other nodes.
    fn write(&self, node: &mut Node) {
        let base = &mut node.base;
        match (self, &mut node.kind) {
            (Self::Name(v), _) => base.name.clone_from(v),
            (Self::Placement(v), _) => base.placement = *v,
            (Self::Opacity(v), _) => base.opacity = *v,
            (Self::Blend(v), _) => base.blend = *v,
            (Self::Visible(v), _) => base.visible = *v,
            (Self::Locked(v), _) => base.locked = *v,
            (Self::Fill(v), NodeKind::Shape(s)) => s.fill = *v,
            (Self::Geometry(v), NodeKind::Shape(s)) => s.geometry = v.clone(),
            (Self::Stroke(v), NodeKind::Shape(s)) => s.stroke = *v,
            (Self::CornerRadius(v), NodeKind::Shape(s)) => s.corner_radius = *v,
            (Self::TextContent(v), NodeKind::Text(t)) => t.content.clone_from(v),
            (Self::TextCurve(v), NodeKind::Text(t)) => t.curve = *v,
            (Self::TextEffects(v), NodeKind::Text(t)) => t.effects.clone_from(v),
            (Self::TextAlign(v), NodeKind::Text(t)) => t.align = *v,
            (Self::TextSizing(v), NodeKind::Text(t)) => t.sizing = *v,
            (Self::TextSpacing(letters, lines), NodeKind::Text(t)) => {
                t.letter_spacing = *letters;
                t.line_height = *lines;
            }
            (Self::TextGradient(v), NodeKind::Text(t)) => t.gradient.clone_from(v),
            (Self::ShapeText(v), NodeKind::Shape(s)) => s.text.clone_from(v),
            (Self::Line(v), NodeKind::Line(l)) => l.clone_from(v),
            (Self::Gradient(v), NodeKind::Shape(s)) => s.gradient.clone_from(v),
            (Self::Effect(v), NodeKind::Shape(s)) => s.effect.clone_from(v),
            (Self::FrameContent(v), NodeKind::Frame(f)) => f.content = *v,
            (Self::FrameFit(v), NodeKind::Frame(f)) => f.fit = *v,
            (Self::FrameCrop(offset, scale, degrees), NodeKind::Frame(f)) => {
                f.content_offset = *offset;
                f.content_scale = *scale;
                f.content_rotation_deg = *degrees;
            }
            (Self::Table(v), NodeKind::Table(t)) => t.clone_from(v),
            (Self::Image(v), NodeKind::Image(i)) => i.clone_from(v),
            (Self::Chart(v), NodeKind::Chart(c)) => c.clone_from(v),
            (Self::Qr(v), NodeKind::Qr(q)) => q.clone_from(v),
            (Self::Stroke(v), NodeKind::Frame(f)) => f.stroke = *v,
            (Self::CornerRadius(v), NodeKind::Frame(f)) => f.corner_radius = *v,
            _ => {}
        }
    }
}

/// Whether every span of `text` has a size and a colour a document holds.
fn spans_valid(text: &RichText) -> bool {
    let unit = |v: f64| (0.0..=1.0).contains(&v);
    text.spans().all(|span| {
        let size = span.style.size;
        size.is_finite()
            && size > 0.0
            && size <= TextNode::MAX_SIZE
            && span.style.color.is_finite()
            && unit(f64::from(span.style.color.alpha()))
    })
}

/// What a property is set on.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Scope {
    Nodes,
    Page,
    Document,
}

impl Property {
    fn scope(&self) -> Scope {
        match self {
            Self::PageName(_)
            | Self::PageBackground(_)
            | Self::PageSize(_)
            | Self::PageLocked(_)
            | Self::PageHidden(_)
            | Self::Bleed(_)
            | Self::Margins(_)
            | Self::Guides(_)
            | Self::Grid(_) => Scope::Page,
            Self::Dpi(_) => Scope::Document,
            _ => Scope::Nodes,
        }
    }

    /// What `page` holds for this page property.
    fn read_page(&self, page: &Page) -> Option<Property> {
        Some(match self {
            Self::PageName(_) => Self::PageName(page.name.clone()),
            Self::PageBackground(_) => Self::PageBackground(page.background.clone()),
            Self::PageSize(_) => Self::PageSize(page.size),
            Self::PageLocked(_) => Self::PageLocked(page.locked),
            Self::PageHidden(_) => Self::PageHidden(page.hidden),
            Self::Bleed(_) => Self::Bleed(page.bleed),
            Self::Margins(_) => Self::Margins(page.margins),
            Self::Guides(_) => Self::Guides(page.guides.clone()),
            Self::Grid(_) => Self::Grid(page.grid),
            _ => return None,
        })
    }

    /// Stores this value in `page`.
    fn write_page(&self, page: &mut Page) {
        match self {
            Self::PageName(v) => page.name.clone_from(v),
            Self::PageBackground(v) => page.background.clone_from(v),
            Self::PageSize(v) => page.size = *v,
            Self::PageLocked(v) => page.locked = *v,
            Self::PageHidden(v) => page.hidden = *v,
            Self::Bleed(v) => page.bleed = *v,
            Self::Margins(v) => page.margins = *v,
            Self::Guides(v) => page.guides.clone_from(v),
            Self::Grid(v) => page.grid = *v,
            _ => {}
        }
    }
}

/// The page-layout error `validate()` would raise for `value` at `dpi`;
/// `Ok` for every other property.
fn check_layout(page: PageId, value: &Property, dpi: Dpi) -> Result<(), CommandError> {
    let checked = match value {
        Property::Bleed(bleed) => check_bleed(*bleed, dpi),
        Property::Margins(margins) => margins.check(),
        Property::Guides(guides) => guides.check(),
        Property::Grid(grid) => grid.check(),
        _ => Ok(()),
    };
    checked
        .map_err(|error| CommandError::InvalidDocument(ValidationError::PageLayout { page, error }))
}

/// The page-size error `validate()` would raise for `size` at `dpi`.
fn check_size(page: PageId, size: PageSize, dpi: Dpi) -> Result<(), CommandError> {
    size.check(dpi)
        .map_err(|error| CommandError::InvalidDocument(ValidationError::PageSize { page, error }))
}

/// Checks that every page of `doc` fits the pixel limits and keeps its
/// layout data in range at `dpi`, in the order of `validate()`: all sizes,
/// then all layouts.
fn check_pages(doc: &Document, dpi: Dpi) -> Result<(), CommandError> {
    for page in doc.pages() {
        check_size(page.id, page.size, dpi)?;
    }
    for page in doc.pages() {
        let layout = page.check_layout(dpi);
        layout.map_err(|error| {
            let page = page.id;
            CommandError::InvalidDocument(ValidationError::PageLayout { page, error })
        })?;
    }
    Ok(())
}

/// Sets one property on one or more nodes of one page (an empty list of
/// targets is refused), or one field of a page, or the document DPI.
///
/// The previous values are captured by every successful `apply`, so a
/// command stays reusable after a failed one. Commands do not check
/// `locked` (Q12).
#[derive(Clone, Debug)]
pub struct SetProperty {
    page: PageId,
    nodes: Vec<NodeId>,
    value: Property,
    gesture: Option<GestureId>,
    /// One old value per target, in target order; empty when `apply`
    /// changed nothing; `None` before the first successful `apply`.
    previous: Option<Vec<Property>>,
}

impl SetProperty {
    /// A command that sets `value` on every node of `nodes` in `page`.
    pub fn new(
        page: PageId,
        nodes: Vec<NodeId>,
        value: Property,
        gesture: Option<GestureId>,
    ) -> Self {
        Self {
            page,
            nodes,
            value,
            gesture,
            previous: None,
        }
    }

    /// A command that sets a page field (`PageName`, `PageSize`, `PageLocked`,
    /// `PageHidden`, `Bleed`, `Margins`, `Guides` or `Grid`) of `page`.
    pub fn page(page: PageId, value: Property, gesture: Option<GestureId>) -> Self {
        Self::new(page, Vec::new(), value, gesture)
    }

    /// A command that sets the document `Dpi`.
    pub fn document(value: Property, gesture: Option<GestureId>) -> Self {
        Self::new(PageId::new(0), Vec::new(), value, gesture)
    }

    fn apply_nodes(&mut self, doc: &mut DocMut<'_>) -> Result<Applied, CommandError> {
        if self.nodes.is_empty() {
            return Err(CommandError::InvalidValue("an empty list of target nodes"));
        }
        self.value.check()?;
        let page = doc
            .as_doc()
            .page(self.page)
            .ok_or(CommandError::MissingPage(self.page))?;
        let mut old = Vec::with_capacity(self.nodes.len());
        for id in &self.nodes {
            let node = page.node(*id).ok_or(CommandError::MissingNode(*id))?;
            let held = self.value.read(node).ok_or(CommandError::InvalidValue(
                "a shape property on a node that is not a shape",
            ))?;
            old.push(held);
        }
        if old.iter().all(|held| *held == self.value) {
            self.previous = Some(Vec::new());
            return Ok(Applied::Unchanged);
        }
        let value = self.value.clone();
        self.write_all(doc, self.nodes.iter().map(|id| (*id, value.clone())))?;
        self.previous = Some(old);
        Ok(Applied::Changed)
    }

    fn revert_nodes(
        &mut self,
        doc: &mut DocMut<'_>,
        old: Vec<Property>,
    ) -> Result<(), CommandError> {
        let page = doc
            .as_doc()
            .page(self.page)
            .ok_or(CommandError::MissingPage(self.page))?;
        if let Some(id) = self.nodes.iter().find(|id| page.node(**id).is_none()) {
            return Err(CommandError::MissingNode(*id));
        }
        // Back to front, so a node listed twice ends on its first old value.
        let pairs = self.nodes.iter().copied().zip(old).rev();
        self.write_all(doc, pairs)
    }

    fn apply_page(&mut self, doc: &mut DocMut<'_>) -> Result<Applied, CommandError> {
        let view = doc.as_doc();
        let page = view
            .page(self.page)
            .ok_or(CommandError::MissingPage(self.page))?;
        if let Property::PageSize(size) = &self.value {
            check_size(self.page, *size, view.dpi())?;
        }
        check_layout(self.page, &self.value, view.dpi())?;
        let held = self
            .value
            .read_page(page)
            .ok_or(CommandError::InvalidValue("a node property on a page"))?;
        if held == self.value {
            self.previous = Some(Vec::new());
            return Ok(Applied::Unchanged);
        }
        let target = doc
            .page_mut(self.page)
            .ok_or(CommandError::MissingPage(self.page))?;
        self.value.write_page(target);
        self.previous = Some(vec![held]);
        Ok(Applied::Changed)
    }

    fn revert_page(
        &mut self,
        doc: &mut DocMut<'_>,
        old: Vec<Property>,
    ) -> Result<(), CommandError> {
        let view = doc.as_doc();
        view.page(self.page)
            .ok_or(CommandError::MissingPage(self.page))?;
        for value in &old {
            if let Property::PageSize(size) = value {
                check_size(self.page, *size, view.dpi())?;
            }
        }
        let target = doc
            .page_mut(self.page)
            .ok_or(CommandError::MissingPage(self.page))?;
        for value in &old {
            value.write_page(target);
        }
        Ok(())
    }

    fn apply_dpi(&mut self, doc: &mut DocMut<'_>) -> Result<Applied, CommandError> {
        let Property::Dpi(dpi) = self.value else {
            return Err(CommandError::InvalidValue(
                "a node property on the document",
            ));
        };
        if !dpi.is_valid() {
            return Err(CommandError::InvalidValue("dpi"));
        }
        let held = doc.as_doc().dpi();
        check_pages(doc.as_doc(), dpi)?;
        if held == dpi {
            self.previous = Some(Vec::new());
            return Ok(Applied::Unchanged);
        }
        doc.set_dpi(dpi);
        self.previous = Some(vec![Property::Dpi(held)]);
        Ok(Applied::Changed)
    }

    fn revert_dpi(&mut self, doc: &mut DocMut<'_>, old: Vec<Property>) -> Result<(), CommandError> {
        for value in &old {
            if let Property::Dpi(dpi) = value {
                check_pages(doc.as_doc(), *dpi)?;
                doc.set_dpi(*dpi);
            }
        }
        Ok(())
    }

    fn write_all(
        &self,
        doc: &mut DocMut<'_>,
        values: impl Iterator<Item = (NodeId, Property)>,
    ) -> Result<(), CommandError> {
        for (id, value) in values {
            let node = doc
                .node_mut(self.page, id)
                .ok_or(CommandError::MissingNode(id))?;
            value.write(node);
        }
        Ok(())
    }
}

impl Command for SetProperty {
    fn label(&self) -> CommandLabel {
        MessageKey("edit.set_property")
    }

    fn apply(&mut self, doc: &mut DocMut<'_>) -> Result<Applied, CommandError> {
        let scope = self.value.scope();
        if scope != Scope::Nodes && !self.nodes.is_empty() {
            return Err(CommandError::InvalidValue(
                "node targets on a page or document property",
            ));
        }
        match scope {
            Scope::Nodes => self.apply_nodes(doc),
            Scope::Page => self.apply_page(doc),
            Scope::Document => self.apply_dpi(doc),
        }
    }

    fn revert(&mut self, doc: &mut DocMut<'_>) -> Result<(), CommandError> {
        let old = self.previous.clone().ok_or(CommandError::NotApplied)?;
        if old.is_empty() {
            return Ok(());
        }
        match self.value.scope() {
            Scope::Nodes => self.revert_nodes(doc, old),
            Scope::Page => self.revert_page(doc, old),
            Scope::Document => self.revert_dpi(doc, old),
        }
    }

    fn merge_key(&self) -> Option<MergeKey> {
        Some(MergeKey {
            kind: self.value.kind(),
            page: (self.value.scope() != Scope::Document).then_some(self.page),
            nodes: self.nodes.clone(),
            gesture: self.gesture,
        })
    }

    fn merge(&mut self, next: Box<dyn Command>) -> Result<(), Box<dyn Command>> {
        if !(&*next as &dyn Any).is::<SetProperty>() {
            return Err(next);
        }
        let any: Box<dyn Any + Send + Sync> = next;
        match any.downcast::<SetProperty>() {
            Ok(other) if other.merge_key() == self.merge_key() => {
                self.value = other.value;
                Ok(())
            }
            Ok(other) => Err(other),
            Err(_) => unreachable!("probed above"),
        }
    }

    fn touches(&self) -> DirtySet {
        let mut dirty = DirtySet {
            pages: Default::default(),
            nodes: self.nodes.iter().copied().collect(),
        };
        if self.value.scope() == Scope::Page {
            dirty.pages.insert(self.page);
        }
        dirty
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use gafira_color::Rgba;
    use gafira_doc::{
        Document, EditToken, ElementBase, GroupNode, Page, PageSize, ShapeNode, ShapeStroke,
    };
    use gafira_geom::{Length, Unit};

    use crate::Transaction;

    use super::*;

    const PAGE: PageId = PageId::new(1);
    const A: NodeId = NodeId::new(2);
    const B: NodeId = NodeId::new(3);
    const GROUP: NodeId = NodeId::new(4);

    fn at(x: f64) -> Placement {
        Placement::new(x, 0.0, 10.0, 10.0)
    }

    /// One page with shapes `A`, `B` and an empty group `GROUP`, all roots.
    fn fixture() -> Document {
        let token = EditToken::__new_for_gafira_edit_only();
        let mut doc = Document::default();
        let mut edit = doc.edit(&token);
        for _ in 0..4 {
            edit.alloc_node_id().expect("ids");
        }
        let size = PageSize {
            width: Length::new(800.0, Unit::Px),
            height: Length::new(600.0, Unit::Px),
        };
        let mut page = Page::new(PAGE, "p".into(), size);
        let shape = |id| {
            Node::new(
                ElementBase::new(id, at(0.0)),
                NodeKind::Shape(ShapeNode::new(ShapeGeometry::Rect)),
            )
        };
        let group = Node::new(
            ElementBase::new(GROUP, at(0.0)),
            NodeKind::Group(GroupNode {}),
        );
        for node in [shape(A), shape(B), group] {
            page.roots.push(node.base.id);
            page.nodes.insert(node.base.id, Arc::new(node));
        }
        edit.pages_mut().push(Arc::new(page));
        assert_eq!(doc.validate(), Ok(()));
        doc
    }

    fn set(nodes: &[NodeId], value: Property) -> SetProperty {
        SetProperty::new(PAGE, nodes.to_vec(), value, None)
    }

    fn apply(command: &mut impl Command, doc: &mut Document) -> Result<Applied, CommandError> {
        let token = EditToken::__new_for_gafira_edit_only();
        command.apply(&mut doc.edit(&token))
    }

    fn revert(command: &mut impl Command, doc: &mut Document) -> Result<(), CommandError> {
        let token = EditToken::__new_for_gafira_edit_only();
        command.revert(&mut doc.edit(&token))
    }

    fn node(doc: &Document, id: NodeId) -> &Node {
        doc.page(PAGE).and_then(|p| p.node(id)).expect("node")
    }

    #[test]
    fn it_moves_several_nodes_and_reverts() {
        let original = fixture();
        let mut doc = original.clone();
        let mut command = set(&[A, B], Property::Placement(at(5.0)));
        assert_eq!(apply(&mut command, &mut doc), Ok(Applied::Changed));
        assert_eq!(node(&doc, A).base.placement, at(5.0));
        assert_eq!(node(&doc, B).base.placement, at(5.0));
        assert_eq!(node(&doc, GROUP).base.placement, at(0.0));
        let touched = command.touches();
        assert_eq!(touched.nodes, [A, B].into_iter().collect());
        assert!(touched.pages.is_empty());
        revert(&mut command, &mut doc).unwrap();
        assert_eq!(doc, original);
    }

    #[test]
    fn every_property_is_set_and_restored() {
        let rgba = Color::Srgb(Rgba::new(0.1, 0.2, 0.3, 0.4));
        let values = [
            Property::Name(Some("hero".into())),
            Property::Placement(at(9.0)),
            Property::Opacity(0.5),
            Property::Blend(BlendMode::Multiply),
            Property::Visible(false),
            Property::Locked(true),
            Property::Fill(Some(rgba)),
            Property::Geometry(ShapeGeometry::Ellipse),
        ];
        for value in values {
            let original = fixture();
            let mut doc = original.clone();
            let mut command = set(&[A], value.clone());
            assert_eq!(apply(&mut command, &mut doc), Ok(Applied::Changed));
            assert_eq!(value.read(node(&doc, A)), Some(value.clone()));
            assert_eq!(doc.validate(), Ok(()));
            revert(&mut command, &mut doc).unwrap();
            assert_eq!(doc, original, "{value:?}");
        }
    }

    // S63-04: a gradient is set on a shape and taken back in one step; one
    // that is not well formed is refused and changes nothing.
    #[test]
    fn s63_04_the_gradient_is_set_and_removed() {
        use gafira_doc::{Gradient, GradientKind, GradientStop};
        let stop = |offset, r| GradientStop {
            offset,
            color: Color::Srgb(Rgba::new(r, 0.0, 0.0, 1.0)),
        };
        let gradient = Gradient {
            kind: GradientKind::Radial,
            stops: vec![stop(0.0, 1.0), stop(1.0, 0.0)],
        };
        let original = fixture();
        let mut doc = original.clone();
        let mut command = set(&[A], Property::Gradient(Some(gradient.clone())));
        assert_eq!(apply(&mut command, &mut doc), Ok(Applied::Changed));
        let NodeKind::Shape(shape) = &node(&doc, A).kind else {
            panic!("a shape")
        };
        assert_eq!(shape.gradient.as_ref(), Some(&gradient));
        let mut solid = set(&[A], Property::Gradient(None));
        assert_eq!(apply(&mut solid, &mut doc), Ok(Applied::Changed));
        assert_eq!(doc, original, "«Color sólido» takes it off");
        revert(&mut solid, &mut doc).unwrap();
        revert(&mut command, &mut doc).unwrap();
        assert_eq!(doc, original);
        let mut broken = gradient;
        broken.stops.truncate(1);
        let mut refused = set(&[A], Property::Gradient(Some(broken)));
        assert!(apply(&mut refused, &mut doc).is_err());
        assert_eq!(doc, original);
    }

    #[test]
    fn s65_01_the_effect_is_set_and_removed() {
        use gafira_doc::{Light, ShapeEffect};
        let original = fixture();
        let mut doc = original.clone();
        let effect = ShapeEffect::Sparkle(Light::SPARKLE);
        let mut command = set(&[A], Property::Effect(Some(effect.clone())));
        assert_eq!(apply(&mut command, &mut doc), Ok(Applied::Changed));
        let NodeKind::Shape(shape) = &node(&doc, A).kind else {
            panic!("a shape")
        };
        assert_eq!(shape.effect.as_ref(), Some(&effect));
        let mut off = set(&[A], Property::Effect(None));
        assert_eq!(apply(&mut off, &mut doc), Ok(Applied::Changed));
        assert_eq!(doc, original);
        revert(&mut off, &mut doc).unwrap();
        revert(&mut command, &mut doc).unwrap();
        assert_eq!(doc, original);
        let mut broken = Light::SPARKLE;
        broken.rays = 2;
        let mut refused = set(&[A], Property::Effect(Some(ShapeEffect::Sparkle(broken))));
        assert!(apply(&mut refused, &mut doc).is_err());
        assert_eq!(doc, original);
    }

    // S81-01: a table is changed as a whole and undone in one step; one that
    // a document does not hold is refused.
    // S83-01: a chart is changed whole and undone; a wrong one is refused.
    #[test]
    fn s83_01_the_chart_is_changed_and_undone() {
        use gafira_doc::{ChartKind, ChartNode, ChartText};
        let ink = Color::Srgb(Rgba::new(0.0, 0.0, 0.0, 1.0));
        let text = ChartText {
            font: "Inter".to_owned(),
            size: 9.0,
            color: ink,
            bold: false,
            italic: false,
        };
        let chart = |kind, columns| {
            let data = (columns, vec!["1".to_owned(); 4]);
            ChartNode::new(kind, data, vec![ink], text.clone())
        };
        let mut original = fixture();
        {
            let token = EditToken::__new_for_gafira_edit_only();
            let mut edit = original.edit(&token);
            let page = Arc::make_mut(&mut edit.pages_mut()[0]);
            let node = Arc::make_mut(page.nodes.get_mut(&A).expect("A"));
            node.kind = NodeKind::Chart(Box::new(chart(ChartKind::Pie, 2)));
        }
        let mut doc = original.clone();
        let next = chart(ChartKind::Radar, 4);
        let mut change = set(&[A], Property::Chart(Box::new(next.clone())));
        assert_eq!(apply(&mut change, &mut doc), Ok(Applied::Changed));
        assert_eq!(node(&doc, A).kind, NodeKind::Chart(Box::new(next)));
        revert(&mut change, &mut doc).unwrap();
        assert_eq!(doc, original);
        let mut refused = set(&[A], Property::Chart(Box::new(chart(ChartKind::Pie, 3))));
        assert!(apply(&mut refused, &mut doc).is_err());
        assert_eq!(doc, original);
    }

    #[test]
    fn s81_01_the_table_is_changed_and_undone() {
        use gafira_doc::{CellStyle, TableNode};
        let ink = Color::Srgb(Rgba::new(0.0, 0.0, 0.0, 1.0));
        let table = |rows| TableNode::new(rows, 2, (9.0, 9.0), &CellStyle::new("Inter", 9.0, ink));
        let mut original = fixture();
        {
            let token = EditToken::__new_for_gafira_edit_only();
            let mut edit = original.edit(&token);
            let page = Arc::make_mut(&mut edit.pages_mut()[0]);
            let node = Arc::make_mut(page.nodes.get_mut(&A).expect("A"));
            node.kind = NodeKind::Table(Box::new(table(2)));
        }
        let mut doc = original.clone();
        let mut change = set(&[A], Property::Table(Box::new(table(3))));
        assert_eq!(apply(&mut change, &mut doc), Ok(Applied::Changed));
        assert_eq!(node(&doc, A).kind, NodeKind::Table(Box::new(table(3))));
        revert(&mut change, &mut doc).unwrap();
        assert_eq!(doc, original);
        let mut refused = set(&[A], Property::Table(Box::new(table(51))));
        assert!(apply(&mut refused, &mut doc).is_err());
        assert_eq!(doc, original);
    }

    // S80-02: a QR code is changed as a whole and undone in one step; one
    // that a document does not hold is refused, and a shape has none.
    #[test]
    fn s80_02_the_qr_is_changed_and_undone() {
        use gafira_doc::{QrContent, QrNode};
        let grey = |v: f32| Color::Srgb(Rgba::new(v, v, v, 1.0));
        let qr = |text: &str| {
            let text = text.to_owned();
            QrNode::new(QrContent::Text { text }, grey(0.0), grey(1.0))
        };
        let mut original = fixture();
        {
            let token = EditToken::__new_for_gafira_edit_only();
            let mut edit = original.edit(&token);
            let page = Arc::make_mut(&mut edit.pages_mut()[0]);
            let node = Arc::make_mut(page.nodes.get_mut(&A).expect("A"));
            node.kind = NodeKind::Qr(Box::new(qr("a")));
        }
        let mut doc = original.clone();
        let mut change = set(&[A], Property::Qr(Box::new(qr("b"))));
        assert_eq!(apply(&mut change, &mut doc), Ok(Applied::Changed));
        assert_eq!(node(&doc, A).kind, NodeKind::Qr(Box::new(qr("b"))));
        revert(&mut change, &mut doc).unwrap();
        assert_eq!(doc, original);
        let mut wide = qr("c");
        wide.quiet_zone = QrNode::MAX_QUIET_ZONE + 1;
        let mut refused = set(&[A], Property::Qr(Box::new(wide)));
        assert!(apply(&mut refused, &mut doc).is_err());
        assert_eq!(doc, original);
        let mut elsewhere = set(&[B], Property::Qr(Box::new(qr("d"))));
        assert_ne!(apply(&mut elsewhere, &mut doc), Ok(Applied::Changed));
        assert_eq!(doc, original);
    }

    // S66-02: putting, replacing and removing the photo of a frame are one
    // change each, undone in one step; the fit changes alone.
    #[test]
    fn s66_02_set_replace_and_remove_are_one_undo_each() {
        use gafira_doc::timeline::{AssetEntry, AssetRef, AssetStorage};
        use gafira_doc::{ContentHash, Fit, FrameContent, FrameNode};
        let photo = |byte: u8| FrameContent {
            asset: AssetRef::new(ContentHash([byte; 32])),
            natural: (40, 30),
        };
        let mut original = fixture();
        {
            let token = EditToken::__new_for_gafira_edit_only();
            let mut edit = original.edit(&token);
            for byte in [1, 2] {
                edit.assets_mut().insert(
                    photo(byte).asset,
                    AssetEntry {
                        name: format!("{byte}.png"),
                        storage: AssetStorage::Embedded,
                    },
                );
            }
            let page = Arc::make_mut(&mut edit.pages_mut()[0]);
            let node = Arc::make_mut(page.nodes.get_mut(&A).expect("A"));
            node.kind = NodeKind::Frame(FrameNode::new(ShapeGeometry::Ellipse));
        }
        let frame = |doc: &Document| match &node(doc, A).kind {
            NodeKind::Frame(frame) => frame.clone(),
            _ => panic!("a frame"),
        };
        let mut doc = original.clone();
        let mut put = set(&[A], Property::FrameContent(Some(photo(1))));
        assert_eq!(apply(&mut put, &mut doc), Ok(Applied::Changed));
        let mut replace = set(&[A], Property::FrameContent(Some(photo(2))));
        assert_eq!(apply(&mut replace, &mut doc), Ok(Applied::Changed));
        assert_eq!(frame(&doc).content, Some(photo(2)));
        assert_eq!(
            frame(&doc).clip_shape,
            ShapeGeometry::Ellipse,
            "the shape stays"
        );
        let mut fit = set(&[A], Property::FrameFit(Fit::Fit));
        assert_eq!(apply(&mut fit, &mut doc), Ok(Applied::Changed));
        let mut crop = set(&[A], Property::FrameCrop((0.1, -0.2), 1.5, 20.0));
        assert_eq!(apply(&mut crop, &mut doc), Ok(Applied::Changed));
        assert_eq!(frame(&doc).content_offset, (0.1, -0.2));
        assert_eq!(frame(&doc).content_scale, 1.5);
        assert_eq!(frame(&doc).content_rotation_deg, 20.0);
        revert(&mut crop, &mut doc).unwrap();
        assert_eq!(frame(&doc).content_scale, 1.0, "the crop is one step");
        let mut bad = set(&[A], Property::FrameCrop((0.0, 0.0), 0.0, 0.0));
        assert!(apply(&mut bad, &mut doc).is_err());
        let mut remove = set(&[A], Property::FrameContent(None));
        assert_eq!(apply(&mut remove, &mut doc), Ok(Applied::Changed));
        assert_eq!(frame(&doc).content, None);
        assert_eq!(frame(&doc).fit, Fit::Fit);
        revert(&mut remove, &mut doc).unwrap();
        assert_eq!(frame(&doc).content, Some(photo(2)), "one step back");
        revert(&mut fit, &mut doc).unwrap();
        revert(&mut replace, &mut doc).unwrap();
        assert_eq!(frame(&doc).content, Some(photo(1)));
        revert(&mut put, &mut doc).unwrap();
        assert_eq!(doc, original);
    }

    #[test]
    fn the_same_value_is_unchanged_and_shares_the_page() {
        let original = fixture();
        let mut doc = original.clone();
        let mut command = set(&[A, B], Property::Visible(true));
        assert_eq!(apply(&mut command, &mut doc), Ok(Applied::Unchanged));
        assert!(Arc::ptr_eq(&doc.pages()[0], &original.pages()[0]));
        revert(&mut command, &mut doc).unwrap();
        assert!(Arc::ptr_eq(&doc.pages()[0], &original.pages()[0]));
    }

    #[test]
    fn a_merged_drag_applies_the_last_value_and_reverts_to_the_origin() {
        let original = fixture();
        let step = |x: f64| set(&[A], Property::Placement(at(x)));
        let mut doc = original.clone();
        let mut first = step(1.0);
        assert_eq!(apply(&mut first, &mut doc), Ok(Applied::Changed));
        let mut second = step(2.0);
        apply(&mut second, &mut doc).unwrap();
        let mut third = step(3.0);
        apply(&mut third, &mut doc).unwrap();
        assert!(first.merge(Box::new(second)).is_ok());
        assert!(first.merge(Box::new(third)).is_ok());
        revert(&mut first, &mut doc).unwrap();
        assert_eq!(doc, original);
        assert_eq!(apply(&mut first, &mut doc), Ok(Applied::Changed));
        assert_eq!(node(&doc, A).base.placement, at(3.0));
    }

    #[test]
    fn merge_needs_an_equal_key_and_a_set_property() {
        let value = Property::Opacity(0.5);
        let mut first = SetProperty::new(PAGE, vec![A], value.clone(), Some(GestureId(1)));
        let others = [
            SetProperty::new(PAGE, vec![A], value.clone(), Some(GestureId(2))),
            SetProperty::new(PAGE, vec![A], value.clone(), None),
            SetProperty::new(PAGE, vec![B], value.clone(), Some(GestureId(1))),
            SetProperty::new(PAGE, vec![A], Property::Visible(false), Some(GestureId(1))),
            SetProperty::new(PageId::new(9), vec![A], value, Some(GestureId(1))),
        ];
        for other in others {
            let next: Box<dyn Command> = Box::new(other);
            let before: *const dyn Command = &*next;
            let back = first.merge(next).expect_err("keys differ");
            assert!(std::ptr::addr_eq(before, &*back));
        }
        let tx: Box<dyn Command> = Box::new(Transaction::new(MessageKey("t"), Vec::new()));
        let back = first.merge(tx).expect_err("not a SetProperty");
        assert!((&*back as &dyn Any).is::<Transaction>());
        assert_eq!(first.value, Property::Opacity(0.5));
    }

    #[test]
    fn the_merge_key_names_the_property_page_nodes_and_gesture() {
        let kinds = [
            (Property::Name(None), "set_property.name"),
            (Property::Placement(at(0.0)), "set_property.placement"),
            (Property::Opacity(1.0), "set_property.opacity"),
            (Property::Blend(BlendMode::Normal), "set_property.blend"),
            (Property::Visible(true), "set_property.visible"),
            (Property::Locked(true), "set_property.locked"),
            (Property::Fill(None), "set_property.fill"),
            (
                Property::Geometry(ShapeGeometry::Rect),
                "set_property.geometry",
            ),
        ];
        for (value, kind) in kinds {
            let key = SetProperty::new(PAGE, vec![B, A], value, Some(GestureId(7)))
                .merge_key()
                .expect("always Some");
            assert_eq!(
                key,
                MergeKey {
                    kind,
                    page: Some(PAGE),
                    nodes: vec![B, A],
                    gesture: Some(GestureId(7)),
                }
            );
        }
    }

    #[test]
    fn invalid_values_fail_and_change_nothing() {
        let bad = |c: [f32; 4]| Color::Srgb(Rgba::new(c[0], c[1], c[2], c[3]));
        let wide = |f: fn(&mut Placement)| {
            let mut p = at(0.0);
            f(&mut p);
            Property::Placement(p)
        };
        let rows = [
            (Property::Opacity(f32::NAN), "opacity"),
            (Property::Opacity(1.5), "opacity"),
            (Property::Opacity(-0.1), "opacity"),
            (wide(|p| p.x = f64::NAN), "placement"),
            (wide(|p| p.y = f64::INFINITY), "placement"),
            (wide(|p| p.rotation_deg = f64::NAN), "placement"),
            (wide(|p| p.w = -1.0), "placement"),
            (wide(|p| p.h = -1.0), "placement"),
            (wide(|p| p.skew = 90.0), "placement"),
            (wide(|p| p.skew = -90.0), "placement"),
            (Property::Fill(Some(bad([f32::NAN, 0.0, 0.0, 1.0]))), "fill"),
            (Property::Fill(Some(bad([0.0, 0.0, 0.0, 1.5]))), "fill"),
            (Property::Fill(Some(bad([0.0, 0.0, 0.0, -0.5]))), "fill"),
        ];
        for (value, what) in rows {
            let original = fixture();
            let mut doc = original.clone();
            let result = apply(&mut set(&[A], value.clone()), &mut doc);
            assert_eq!(result, Err(CommandError::InvalidValue(what)), "{value:?}");
            assert_eq!(doc, original);
        }
    }

    #[test]
    fn range_edges_are_accepted() {
        let edge = |f: fn(&mut Placement)| {
            let mut p = at(0.0);
            f(&mut p);
            Property::Placement(p)
        };
        let values = [
            Property::Opacity(0.0),
            Property::Opacity(1.0),
            edge(|p| (p.w, p.h) = (0.0, 0.0)),
            edge(|p| p.skew = 89.999),
            edge(|p| p.skew = -89.999),
            Property::Fill(Some(Color::Srgb(Rgba::new(-3.0, 9.0, 0.0, 0.0)))),
            Property::Fill(Some(Color::Srgb(Rgba::new(0.0, 0.0, 0.0, 1.0)))),
        ];
        for value in values {
            let mut doc = fixture();
            let result = apply(&mut set(&[A], value.clone()), &mut doc);
            assert!(result.is_ok(), "{value:?}: {result:?}");
            assert_eq!(doc.validate(), Ok(()));
        }
    }

    #[test]
    fn a_shape_property_on_another_node_fails_without_a_partial_change() {
        let original = fixture();
        let mut doc = original.clone();
        let fill = Property::Fill(Some(Color::Srgb(Rgba::new(1.0, 0.0, 0.0, 1.0))));
        for value in [fill, Property::Geometry(ShapeGeometry::Ellipse)] {
            let result = apply(&mut set(&[A, GROUP], value), &mut doc);
            assert!(matches!(result, Err(CommandError::InvalidValue(_))));
            assert_eq!(doc, original);
        }
    }

    #[test]
    fn a_missing_page_or_node_fails_and_changes_nothing() {
        let original = fixture();
        let mut doc = original.clone();
        let value = Property::Visible(false);
        let ghost = NodeId::new(99);
        let page = SetProperty::new(PageId::new(9), vec![A], value.clone(), None);
        let rows = [
            (page, CommandError::MissingPage(PageId::new(9))),
            (set(&[A, ghost], value), CommandError::MissingNode(ghost)),
        ];
        for (mut command, error) in rows {
            assert_eq!(apply(&mut command, &mut doc), Err(error));
            assert_eq!(doc, original);
        }
    }

    #[test]
    fn a_locked_node_is_still_editable() {
        let mut doc = fixture();
        apply(&mut set(&[A], Property::Locked(true)), &mut doc).unwrap();
        let rename = Property::Name(Some("x".into()));
        assert_eq!(
            apply(&mut set(&[A], rename), &mut doc),
            Ok(Applied::Changed)
        );
    }

    #[test]
    fn a_command_is_reusable_after_a_failed_apply() {
        let original = fixture();
        let mut command = set(&[A], Property::Opacity(0.25));
        let token = EditToken::__new_for_gafira_edit_only();

        // Fails on a document without the node; nothing is recorded.
        let mut empty = Document::default();
        assert!(command.apply(&mut empty.edit(&token)).is_err());
        assert_eq!(
            revert(&mut command, &mut empty),
            Err(CommandError::NotApplied)
        );

        let mut doc = original.clone();
        assert_eq!(apply(&mut command, &mut doc), Ok(Applied::Changed));
        revert(&mut command, &mut doc).unwrap();
        assert_eq!(doc, original);
    }

    #[test]
    fn a_failed_apply_keeps_the_record_of_the_last_good_one() {
        let original = fixture();
        let mut command = set(&[A], Property::Opacity(0.25));
        let mut doc = original.clone();
        apply(&mut command, &mut doc).unwrap();
        let mut empty = Document::default();
        assert!(apply(&mut command, &mut empty).is_err());
        revert(&mut command, &mut doc).unwrap();
        assert_eq!(doc, original);
    }

    #[test]
    fn a_node_listed_twice_reverts_to_its_first_value() {
        let original = fixture();
        let mut doc = original.clone();
        let mut command = set(&[A, A], Property::Opacity(0.5));
        assert_eq!(apply(&mut command, &mut doc), Ok(Applied::Changed));
        revert(&mut command, &mut doc).unwrap();
        assert_eq!(doc, original);
    }

    #[test]
    fn reverting_with_any_target_missing_fails_and_changes_nothing() {
        for gone in [A, B] {
            let mut doc = fixture();
            let mut command = set(&[A, B], Property::Opacity(0.5));
            apply(&mut command, &mut doc).unwrap();
            let token = EditToken::__new_for_gafira_edit_only();
            let mut page = (*doc.pages()[0]).clone();
            page.nodes.remove(&gone);
            page.roots.retain(|id| *id != gone);
            doc.edit(&token).pages_mut()[0] = Arc::new(page);
            let before = doc.clone();
            assert_eq!(
                revert(&mut command, &mut doc),
                Err(CommandError::MissingNode(gone))
            );
            assert_eq!(doc, before);
        }
    }

    #[test]
    fn it_is_unchanged_only_when_every_target_holds_the_value() {
        let original = fixture();
        let mut seed = original.clone();
        apply(&mut set(&[A], Property::Visible(false)), &mut seed).unwrap();
        // One of two targets already holds the value, in either position.
        for targets in [[A, B], [B, A]] {
            let mut doc = seed.clone();
            let mut command = set(&targets, Property::Visible(false));
            assert_eq!(apply(&mut command, &mut doc), Ok(Applied::Changed));
            assert!(!node(&doc, A).base.visible && !node(&doc, B).base.visible);
            revert(&mut command, &mut doc).unwrap();
            assert_eq!(doc, seed);
        }
    }

    fn outline(width: f64) -> Option<ShapeStroke> {
        Some(ShapeStroke::new(
            Color::Srgb(Rgba::new(0.0, 0.0, 1.0, 1.0)),
            width,
        ))
    }

    // S58-01: a whole line is set and reverted; a width out of range is
    // refused; on a node that is not a line it fails.
    #[test]
    fn s58_01_a_line_is_set_and_reverted() {
        use gafira_doc::{LineCap, LineEnd, LineNode};
        let token = EditToken::__new_for_gafira_edit_only();
        let mut doc = fixture();
        let black = gafira_color::Color::Srgb(gafira_color::Rgba::new(0.0, 0.0, 0.0, 1.0));
        let line = |width: f64| LineNode {
            start: LineEnd::new([0.0, 0.0], LineCap::None),
            end: LineEnd::new([1.0, 1.0], LineCap::Arrow),
            color: black,
            width,
            dash: Default::default(),
        };
        doc.edit(&token).node_mut(PAGE, B).expect("a node").kind = NodeKind::Line(line(2.0));
        let original = doc.clone();
        let mut thick = set(&[B], Property::Line(line(6.0)));
        assert_eq!(apply(&mut thick, &mut doc), Ok(Applied::Changed));
        assert!(matches!(&node(&doc, B).kind, NodeKind::Line(l) if l.width == 6.0));
        revert(&mut thick, &mut doc).unwrap();
        assert_eq!(doc, original);
        let mut wide = set(&[B], Property::Line(line(1001.0)));
        assert!(apply(&mut wide, &mut doc).is_err());
        let mut on_group = set(&[GROUP], Property::Line(line(1.0)));
        assert!(apply(&mut on_group, &mut doc).is_err());
        assert_eq!(doc, original);
    }

    // S95-01: how a text box follows its words is set and reverted in one
    // step, and refused on a node that is not a text.
    #[test]
    fn s95_01_text_sizing_is_set_and_reverted() {
        let mut doc = fixture();
        let style = gafira_doc::SpanStyle {
            font: "Inter".to_owned(),
            size: 24.0,
            color: gafira_color::Color::Srgb(gafira_color::Rgba::new(0.0, 0.0, 0.0, 1.0)),
            bold: false,
            italic: false,
        };
        {
            let token = EditToken::__new_for_gafira_edit_only();
            let mut edit = doc.edit(&token);
            let page = edit.page_mut(PAGE).expect("page");
            let text = Node::new(
                ElementBase::new(B, at(0.0)),
                NodeKind::Text(TextNode::plain("Hola", &style)),
            );
            page.nodes.insert(B, Arc::new(text));
        }
        let original = doc.clone();
        let sizing = |doc: &Document| match &node(doc, B).kind {
            NodeKind::Text(t) => t.sizing,
            _ => panic!("not a text"),
        };
        assert_eq!(sizing(&doc), gafira_doc::TextSizing::AutoHeight);
        let mut grow = set(
            &[B],
            Property::TextSizing(gafira_doc::TextSizing::AutoWidth),
        );
        assert_eq!(apply(&mut grow, &mut doc), Ok(Applied::Changed));
        assert_eq!(sizing(&doc), gafira_doc::TextSizing::AutoWidth);
        let mut same = set(
            &[B],
            Property::TextSizing(gafira_doc::TextSizing::AutoWidth),
        );
        assert_eq!(apply(&mut same, &mut doc), Ok(Applied::Unchanged));
        revert(&mut grow, &mut doc).unwrap();
        assert_eq!(doc, original);
        let mut on_shape = set(&[A], Property::TextSizing(gafira_doc::TextSizing::Fixed));
        assert!(apply(&mut on_shape, &mut doc).is_err() || node(&doc, A) == node(&original, A));
    }

    // S55-02: the text of a shape is set, changed and taken away in one
    // step each, reverted exactly, and refused on a node that is not a
    // shape.
    #[test]
    fn s55_02_shape_text_is_set_and_reverted() {
        let original = fixture();
        let mut doc = original.clone();
        let text = |doc: &Document| match &node(doc, A).kind {
            NodeKind::Shape(s) => s.text.clone(),
            _ => panic!("not a shape"),
        };
        let style = gafira_doc::SpanStyle {
            font: "Inter".to_owned(),
            size: 24.0,
            color: gafira_color::Color::Srgb(gafira_color::Rgba::new(0.0, 0.0, 0.0, 1.0)),
            bold: false,
            italic: false,
        };
        let hola = ShapeText::plain("Hola", &style);
        let mut set_it = set(&[A], Property::ShapeText(Some(hola.clone())));
        assert_eq!(apply(&mut set_it, &mut doc), Ok(Applied::Changed));
        assert_eq!(text(&doc), Some(hola.clone()));
        assert_eq!(doc.validate(), Ok(()));
        let mut same = set(&[A], Property::ShapeText(Some(hola.clone())));
        assert_eq!(apply(&mut same, &mut doc), Ok(Applied::Unchanged));
        let mut empty = set(&[A], Property::ShapeText(None));
        assert_eq!(apply(&mut empty, &mut doc), Ok(Applied::Changed));
        assert_eq!(text(&doc), None);
        revert(&mut empty, &mut doc).unwrap();
        assert_eq!(text(&doc), Some(hola.clone()));
        revert(&mut set_it, &mut doc).unwrap();
        assert_eq!(doc, original);
        let mut on_group = set(&[GROUP], Property::ShapeText(Some(hola)));
        assert!(apply(&mut on_group, &mut doc).is_err());
        assert_eq!(doc, original);
    }

    // S38-03: the outline and the corner radius are set, undone and merged
    // like any other shape property.
    #[test]
    fn s38_03_outline_and_radius_are_set_and_reverted() {
        let original = fixture();
        let mut doc = original.clone();
        let shape = |doc: &Document| match &node(doc, A).kind {
            NodeKind::Shape(s) => s.clone(),
            _ => panic!("not a shape"),
        };
        let mut stroke = set(&[A], Property::Stroke(outline(4.0)));
        assert_eq!(apply(&mut stroke, &mut doc), Ok(Applied::Changed));
        assert_eq!(shape(&doc).stroke, outline(4.0));
        let mut radius = set(&[A], Property::CornerRadius(8.0));
        assert_eq!(apply(&mut radius, &mut doc), Ok(Applied::Changed));
        assert_eq!(shape(&doc).corner_radius, 8.0);
        assert_eq!(doc.validate(), Ok(()));
        let mut same = set(&[A], Property::CornerRadius(8.0));
        assert_eq!(apply(&mut same, &mut doc), Ok(Applied::Unchanged));
        let mut none = set(&[A], Property::Stroke(None));
        assert_eq!(apply(&mut none, &mut doc), Ok(Applied::Changed));
        assert_eq!(shape(&doc).stroke, None);
        revert(&mut none, &mut doc).unwrap();
        assert_eq!(shape(&doc).stroke, outline(4.0));
        revert(&mut radius, &mut doc).unwrap();
        revert(&mut stroke, &mut doc).unwrap();
        assert_eq!(doc, original);
        // On a node that is not a shape they fail and change nothing.
        for value in [Property::Stroke(outline(1.0)), Property::CornerRadius(1.0)] {
            let mut on_group = set(&[GROUP], value);
            assert!(apply(&mut on_group, &mut doc).is_err());
            assert_eq!(doc, original);
        }
        let kinds = [
            (Property::Stroke(None), "set_property.stroke"),
            (Property::CornerRadius(0.0), "set_property.corner_radius"),
        ];
        for (value, kind) in kinds {
            assert_eq!(set(&[A], value).merge_key().map(|k| k.kind), Some(kind));
        }
    }

    // S38-03: a value `validate()` would refuse is refused before it lands.
    #[test]
    fn s38_03_invalid_outline_and_radius_fail_and_change_nothing() {
        let colour = |a: f32| Color::Srgb(Rgba::new(0.0, 0.0, 0.0, a));
        let rows = [
            (Property::Stroke(outline(-0.5)), "stroke"),
            (
                Property::Stroke(outline(ShapeStroke::MAX_WIDTH + 0.5)),
                "stroke",
            ),
            (Property::Stroke(outline(f64::NAN)), "stroke"),
            (
                Property::Stroke(Some(ShapeStroke::new(colour(1.5), 1.0))),
                "stroke",
            ),
            (
                Property::Stroke(Some(ShapeStroke::new(colour(f32::NAN), 1.0))),
                "stroke",
            ),
            (Property::CornerRadius(-1e-9), "corner_radius"),
            (Property::CornerRadius(f64::INFINITY), "corner_radius"),
            (Property::CornerRadius(f64::NAN), "corner_radius"),
        ];
        for (value, what) in rows {
            let mut doc = fixture();
            let before = doc.clone();
            let mut command = set(&[A], value);
            assert_eq!(
                apply(&mut command, &mut doc),
                Err(CommandError::InvalidValue(what))
            );
            assert_eq!(doc, before);
        }
        for value in [
            Property::Stroke(outline(0.0)),
            Property::Stroke(outline(ShapeStroke::MAX_WIDTH)),
            Property::Stroke(Some(ShapeStroke::new(colour(0.0), 1.0))),
            Property::CornerRadius(0.0),
            Property::CornerRadius(1e9),
        ] {
            let mut doc = fixture();
            let mut command = set(&[A], value);
            assert!(apply(&mut command, &mut doc).is_ok());
            assert_eq!(doc.validate(), Ok(()));
        }
    }

    #[test]
    fn an_empty_target_list_is_refused() {
        let original = fixture();
        let mut doc = original.clone();
        let result = apply(&mut set(&[], Property::Visible(false)), &mut doc);
        assert!(matches!(result, Err(CommandError::InvalidValue(_))));
        assert_eq!(doc, original);
    }

    /// One drag frame of a multi-selection: `A` to `a` and `B` to `b`
    /// (spec 040).
    fn moves(a: f64, b: f64, gesture: Option<GestureId>) -> Box<dyn Command> {
        two(
            Property::Placement(at(a)),
            Property::Placement(at(b)),
            gesture,
        )
    }

    /// A transaction that sets `first` on `A` and `second` on `B`.
    fn two(first: Property, second: Property, gesture: Option<GestureId>) -> Box<dyn Command> {
        Box::new(Transaction::new(
            MessageKey("edit.set_property"),
            vec![
                Box::new(SetProperty::new(PAGE, vec![A], first, gesture)),
                Box::new(SetProperty::new(PAGE, vec![B], second, gesture)),
            ],
        ))
    }

    fn at_ms(ms: u64) -> std::time::Duration {
        std::time::Duration::from_millis(ms)
    }

    // S40-04: the frames of a multi-selection drag are one history entry.
    #[test]
    fn s40_04_a_gesture_of_move_transactions_is_one_entry_that_undoes_to_the_start() {
        let original = fixture();
        let mut history = crate::History::new(original.clone()).expect("valid");
        let gesture = history.begin_gesture();
        for step in 1..=3u64 {
            let d = step as f64;
            // Far apart in time: only the gesture can merge them.
            let applied = history.execute(moves(d, 10.0 + d, Some(gesture)), at_ms(step * 1000));
            assert_eq!(applied, Ok(Applied::Changed));
        }
        history.end_gesture();
        assert_eq!(history.len(), 1, "one entry for the whole drag");
        let doc = Arc::clone(history.document());
        assert_eq!(node(&doc, A).base.placement, at(3.0));
        assert_eq!(node(&doc, B).base.placement, at(13.0));
        assert_eq!(
            history.newest_touches().expect("an entry").nodes,
            [A, B].into_iter().collect()
        );
        history.undo().expect("an undo");
        assert_eq!(**history.document(), original);
        history.redo().expect("a redo");
        assert_eq!(node(history.document(), B).base.placement, at(13.0));
    }

    // S40-04: the merged transaction also reverts and re-applies by itself,
    // as a compacted entry does.
    #[test]
    fn s40_04_a_merged_transaction_reverts_to_the_first_before_and_applies_the_last_after() {
        let original = fixture();
        let mut doc = original.clone();
        let gesture = Some(GestureId(7));
        let token = EditToken::__new_for_gafira_edit_only();
        let mut first = moves(1.0, 2.0, gesture);
        assert_eq!(first.apply(&mut doc.edit(&token)), Ok(Applied::Changed));
        let mut second = moves(5.0, 6.0, gesture);
        assert_eq!(second.apply(&mut doc.edit(&token)), Ok(Applied::Changed));
        assert!(first.merge(second).is_ok());
        first.revert(&mut doc.edit(&token)).unwrap();
        assert_eq!(doc, original);
        assert_eq!(first.apply(&mut doc.edit(&token)), Ok(Applied::Changed));
        assert_eq!(node(&doc, A).base.placement, at(5.0));
        assert_eq!(node(&doc, B).base.placement, at(6.0));
    }

    // S40-04: the key of a transaction is its children's page, nodes and
    // gesture; a child without a key, or children of different pages or
    // gestures, leave it without one.
    #[test]
    fn s40_04_a_transaction_key_joins_its_children_or_is_none() {
        let gesture = Some(GestureId(1));
        let key = moves(1.0, 2.0, gesture).merge_key().expect("a key");
        assert_eq!(key.kind, "transaction");
        assert_eq!(key.page, Some(PAGE));
        assert_eq!(key.nodes, [A, B]);
        assert_eq!(key.gesture, gesture);

        let empty = Transaction::new(MessageKey("t"), Vec::new());
        assert_eq!(empty.merge_key(), None);
        let keyless = Transaction::new(
            MessageKey("t"),
            vec![
                Box::new(set(&[A], Property::Opacity(0.5))),
                Box::new(crate::RemoveNodes::new(PAGE, vec![B])),
            ],
        );
        assert_eq!(keyless.merge_key(), None);
        let gestures = Transaction::new(
            MessageKey("t"),
            vec![
                Box::new(SetProperty::new(
                    PAGE,
                    vec![A],
                    Property::Opacity(0.5),
                    gesture,
                )),
                Box::new(SetProperty::new(
                    PAGE,
                    vec![B],
                    Property::Opacity(0.5),
                    None,
                )),
            ],
        );
        assert_eq!(gestures.merge_key(), None);
        let pages = Transaction::new(
            MessageKey("t"),
            vec![
                Box::new(SetProperty::new(
                    PAGE,
                    vec![A],
                    Property::Opacity(0.5),
                    gesture,
                )),
                Box::new(SetProperty::new(
                    PageId::new(9),
                    vec![B],
                    Property::Opacity(0.5),
                    gesture,
                )),
            ],
        );
        assert_eq!(pages.merge_key(), None);
    }

    // S40-04: a transaction only absorbs one whose children pair up with
    // equal keys; anything else comes back untouched and nothing merged.
    #[test]
    fn s40_04_a_transaction_hands_back_what_does_not_pair_up() {
        let gesture = Some(GestureId(1));
        let mut first = moves(1.0, 2.0, gesture);
        let only_a: Box<dyn Command> = Box::new(Transaction::new(
            MessageKey("edit.set_property"),
            vec![Box::new(SetProperty::new(
                PAGE,
                vec![A],
                Property::Placement(at(4.0)),
                gesture,
            ))],
        ));
        let swapped: Box<dyn Command> = Box::new(Transaction::new(
            MessageKey("edit.set_property"),
            vec![
                Box::new(SetProperty::new(
                    PAGE,
                    vec![B],
                    Property::Placement(at(4.0)),
                    gesture,
                )),
                Box::new(SetProperty::new(
                    PAGE,
                    vec![A],
                    Property::Placement(at(4.0)),
                    gesture,
                )),
            ],
        ));
        let others: Vec<Box<dyn Command>> = vec![
            moves(1.0, 2.0, Some(GestureId(2))),
            only_a,
            swapped,
            two(
                Property::Placement(at(4.0)),
                Property::Opacity(0.5),
                gesture,
            ),
            Box::new(SetProperty::new(
                PAGE,
                vec![A],
                Property::Placement(at(4.0)),
                gesture,
            )),
        ];
        for other in others {
            let before: *const dyn Command = &*other;
            let back = first.merge(other).expect_err("does not pair up");
            assert!(std::ptr::addr_eq(before, &*back));
        }
        let mut doc = fixture();
        let token = EditToken::__new_for_gafira_edit_only();
        assert_eq!(first.apply(&mut doc.edit(&token)), Ok(Applied::Changed));
        assert_eq!(node(&doc, A).base.placement, at(1.0), "nothing merged");
        assert_eq!(node(&doc, B).base.placement, at(2.0));
    }
}
