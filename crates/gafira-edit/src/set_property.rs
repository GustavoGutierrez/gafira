//! `SetProperty`: one property of one or more nodes (ARQ-81, ADR-03).

use std::any::Any;

use gafira_color::Color;
use gafira_doc::msg::{CommandLabel, MessageKey};
use gafira_doc::{
    BlendMode, DocMut, Document, Dpi, Node, NodeId, NodeKind, Page, PageId, PageSize, Placement,
    ShapeGeometry, ValidationError,
};

use crate::{Applied, Command, CommandError, DirtySet, GestureId, MergeKey};

/// A property value of a node, tagged with the property it belongs to.
///
/// `Fill` and `Geometry` only apply to `Shape` nodes. The `Page` variants
/// target a page and `Dpi` the document; build those commands with
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
    PageName(String),
    PageSize(PageSize),
    PageLocked(bool),
    PageHidden(bool),
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
            Self::PageName(_) => "set_property.page_name",
            Self::PageSize(_) => "set_property.page_size",
            Self::PageLocked(_) => "set_property.page_locked",
            Self::PageHidden(_) => "set_property.page_hidden",
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
            (Self::Geometry(_), NodeKind::Shape(s)) => Self::Geometry(s.geometry),
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
            (Self::Geometry(v), NodeKind::Shape(s)) => s.geometry = *v,
            _ => {}
        }
    }
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
            Self::PageName(_) | Self::PageSize(_) | Self::PageLocked(_) | Self::PageHidden(_) => {
                Scope::Page
            }
            Self::Dpi(_) => Scope::Document,
            _ => Scope::Nodes,
        }
    }

    /// What `page` holds for this page property.
    fn read_page(&self, page: &Page) -> Option<Property> {
        Some(match self {
            Self::PageName(_) => Self::PageName(page.name.clone()),
            Self::PageSize(_) => Self::PageSize(page.size),
            Self::PageLocked(_) => Self::PageLocked(page.locked),
            Self::PageHidden(_) => Self::PageHidden(page.hidden),
            _ => return None,
        })
    }

    /// Stores this value in `page`.
    fn write_page(&self, page: &mut Page) {
        match self {
            Self::PageName(v) => page.name.clone_from(v),
            Self::PageSize(v) => page.size = *v,
            Self::PageLocked(v) => page.locked = *v,
            Self::PageHidden(v) => page.hidden = *v,
            _ => {}
        }
    }
}

/// The page-size error `validate()` would raise for `size` at `dpi`.
fn check_size(page: PageId, size: PageSize, dpi: Dpi) -> Result<(), CommandError> {
    size.check(dpi)
        .map_err(|error| CommandError::InvalidDocument(ValidationError::PageSize { page, error }))
}

/// Checks that every page of `doc` fits the pixel limits at `dpi`.
fn check_pages(doc: &Document, dpi: Dpi) -> Result<(), CommandError> {
    doc.pages()
        .iter()
        .try_for_each(|page| check_size(page.id, page.size, dpi))
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

    /// A command that sets a page field (`PageName`, `PageSize`, `PageLocked`
    /// or `PageHidden`) of `page`.
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
    use gafira_doc::{Document, EditToken, ElementBase, GroupNode, Page, PageSize, ShapeNode};
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

    #[test]
    fn an_empty_target_list_is_refused() {
        let original = fixture();
        let mut doc = original.clone();
        let result = apply(&mut set(&[], Property::Visible(false)), &mut doc);
        assert!(matches!(result, Err(CommandError::InvalidValue(_))));
        assert_eq!(doc, original);
    }
}
