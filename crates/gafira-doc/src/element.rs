//! Fields shared by every element node.

use serde::{Deserialize, Serialize};

use crate::{BlendMode, NodeId, Placement};

/// The common header of a node: identity, placement and compositing state.
///
/// All fields are bare values (no wrapper) so a future untagged
/// `Animatable::Static` stays wire-compatible. Nothing is validated here.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ElementBase {
    /// The node's id.
    pub id: NodeId,
    /// Optional display name.
    #[serde(default)]
    pub name: Option<String>,
    /// Position and orientation on the page.
    pub placement: Placement,
    /// Opacity, nominally `0.0..=1.0`.
    #[serde(default = "one")]
    pub opacity: f32,
    /// How the element blends with what is below it.
    #[serde(default)]
    pub blend: BlendMode,
    /// Whether the element is drawn.
    #[serde(default = "yes")]
    pub visible: bool,
    /// Whether the element is protected from edits (independent of the page).
    #[serde(default)]
    pub locked: bool,
}

fn one() -> f32 {
    1.0
}

fn yes() -> bool {
    true
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

    fn base_json(extra: &str) -> String {
        format!(r#"{{"id":4,"placement":{{"x":0,"y":0,"w":1,"h":1}}{extra}}}"#)
    }

    #[test]
    fn wire_form_writes_every_field_with_null_name() {
        let mut e = ElementBase::new(NodeId::new(4), Placement::new(0.0, 0.0, 1.0, 1.0));
        e.opacity = 0.5;
        let json = serde_json::to_string(&e).unwrap();
        assert_eq!(
            json,
            concat!(
                r#"{"id":4,"name":null,"placement":{"x":0.0,"y":0.0,"w":1.0,"h":1.0,"#,
                r#""rotation_deg":0.0,"skew":0.0,"flip":{"x":false,"y":false}},"#,
                r#""opacity":0.5,"blend":"normal","visible":true,"locked":false}"#
            )
        );
        assert_eq!(serde_json::from_str::<ElementBase>(&json).unwrap(), e);
    }

    #[test]
    fn omitted_fields_take_their_defaults() {
        let e: ElementBase = serde_json::from_str(&base_json("")).unwrap();
        let want = ElementBase::new(NodeId::new(4), Placement::new(0.0, 0.0, 1.0, 1.0));
        assert_eq!(e, want);
        assert_eq!(e.opacity.to_bits(), 1.0f32.to_bits());
    }

    #[test]
    fn unknown_fields_are_ignored_and_bad_vocabulary_fails() {
        assert!(serde_json::from_str::<ElementBase>(&base_json(r#","extra":1"#)).is_ok());
        for bad in [
            r#","blend":"Multiply""#,
            r#","opacity":null"#,
            r#","visible":"yes""#,
        ] {
            assert!(
                serde_json::from_str::<ElementBase>(&base_json(bad)).is_err(),
                "{bad}"
            );
        }
        assert!(
            serde_json::from_str::<ElementBase>(r#"{"placement":{"x":0,"y":0,"w":1,"h":1}}"#)
                .is_err()
        );
        assert!(serde_json::from_str::<ElementBase>(r#"{"id":1}"#).is_err());
    }

    #[test]
    fn subnormal_opacity_keeps_its_bits() {
        let mut e = ElementBase::new(NodeId::new(1), Placement::new(0.0, 0.0, 1.0, 1.0));
        e.opacity = 1e-45;
        let back: ElementBase = serde_json::from_str(&serde_json::to_string(&e).unwrap()).unwrap();
        assert_eq!(back.opacity.to_bits(), 1e-45f32.to_bits());
    }
}
