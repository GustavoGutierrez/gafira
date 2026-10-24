//! Fields shared by every element node.

use crate::{BlendMode, NodeId, Placement};

/// The common header of a node: identity, placement and compositing state.
///
/// All fields are bare values (no wrapper) so a future untagged
/// `Animatable::Static` stays wire-compatible. Nothing is validated here.
#[derive(Clone, Debug, PartialEq)]
pub struct ElementBase {
    /// The node's id.
    pub id: NodeId,
    /// Optional display name.
    pub name: Option<String>,
    /// Position and orientation on the page.
    pub placement: Placement,
    /// Opacity, nominally `0.0..=1.0`.
    pub opacity: f32,
    /// How the element blends with what is below it.
    pub blend: BlendMode,
    /// Whether the element is drawn.
    pub visible: bool,
    /// Whether the element is protected from edits (independent of the page).
    pub locked: bool,
}

impl ElementBase {
    /// An opaque, visible, unlocked, unnamed element with `Normal` blending.
    pub fn new(id: NodeId, placement: Placement) -> Self {
        Self {
            id,
            name: None,
            placement,
            opacity: 1.0,
            blend: BlendMode::Normal,
            visible: true,
            locked: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults() {
        let e = ElementBase::new(NodeId::new(7), Placement::new(1.0, 2.0, 3.0, 4.0));
        assert_eq!(e.id, NodeId::new(7));
        assert_eq!(e.opacity.to_bits(), 1.0f32.to_bits());
        assert_eq!(e.blend, BlendMode::Normal);
        assert!(e.visible);
        assert!(!e.locked);
        assert_eq!(e.name, None);
        assert_eq!(e.placement, Placement::new(1.0, 2.0, 3.0, 4.0));
    }

    #[test]
    fn out_of_range_values_are_representable() {
        let mut e = ElementBase::new(NodeId::new(1), Placement::new(f64::NAN, 0.0, 1.0, 1.0));
        e.opacity = 1.5;
        assert_eq!(e.opacity.to_bits(), 1.5f32.to_bits());
        assert_eq!(e.placement.x.to_bits(), f64::NAN.to_bits());
        assert_eq!(crate::Dpi::new(0).get(), 0);
    }

    #[test]
    fn nan_node_is_not_equal_to_itself() {
        let e = ElementBase::new(NodeId::new(1), Placement::new(f64::NAN, 0.0, 1.0, 1.0));
        assert_ne!(e, e.clone());
        assert!(e.placement.x.is_nan());
        assert_eq!(e.placement.x.to_bits(), e.clone().placement.x.to_bits());
    }
}
