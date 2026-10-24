//! Identifiers of the supported color spaces.

use serde::{Deserialize, Serialize};

/// A color space supported by this crate.
///
/// Serialized as the CSS Color 4 predefined-space identifier. The enum is
/// exhaustive on purpose: adding a space makes the compiler flag every `match`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ColorSpaceId {
    /// Encoded sRGB (`"srgb"`).
    #[serde(rename = "srgb")]
    Srgb,
    /// Linear-light sRGB primaries (`"srgb-linear"`).
    #[serde(rename = "srgb-linear")]
    LinearSrgb,
    /// Encoded Display-P3 (`"display-p3"`).
    #[serde(rename = "display-p3")]
    DisplayP3,
}

impl ColorSpaceId {
    /// Every space, in declaration order.
    pub const ALL: [ColorSpaceId; 3] = [
        ColorSpaceId::Srgb,
        ColorSpaceId::LinearSrgb,
        ColorSpaceId::DisplayP3,
    ];
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Exhaustive on purpose: adding a variant must break compilation here.
    fn wire_name(id: ColorSpaceId) -> &'static str {
        match id {
            ColorSpaceId::Srgb => "srgb",
            ColorSpaceId::LinearSrgb => "srgb-linear",
            ColorSpaceId::DisplayP3 => "display-p3",
        }
    }

    #[test]
    fn all_lists_exactly_the_three_variants_in_order() {
        assert_eq!(
            ColorSpaceId::ALL,
            [
                ColorSpaceId::Srgb,
                ColorSpaceId::LinearSrgb,
                ColorSpaceId::DisplayP3
            ]
        );
    }

    #[test]
    fn serializes_as_css_color_4_identifiers() {
        for id in ColorSpaceId::ALL {
            let json = serde_json::to_string(&id).unwrap();
            assert_eq!(json, format!("\"{}\"", wire_name(id)));
        }
        assert_eq!(
            serde_json::to_string(&ColorSpaceId::DisplayP3).unwrap(),
            r#""display-p3""#
        );
    }

    #[test]
    fn deserializes_every_wire_name() {
        for id in ColorSpaceId::ALL {
            let json = format!("\"{}\"", wire_name(id));
            assert_eq!(serde_json::from_str::<ColorSpaceId>(&json).unwrap(), id);
        }
    }

    #[test]
    fn unknown_and_case_changed_names_are_rejected() {
        for bad in [
            r#""rec2020""#,
            r#""SRGB""#,
            r#""linear-srgb""#,
            r#""display_p3""#,
            r#""""#,
            "null",
            "1",
        ] {
            assert!(
                serde_json::from_str::<ColorSpaceId>(bad).is_err(),
                "{bad} must be rejected"
            );
        }
    }

    #[test]
    fn ordering_and_hashing_are_available() {
        use std::collections::BTreeSet;
        let set: BTreeSet<ColorSpaceId> = ColorSpaceId::ALL.into_iter().collect();
        assert_eq!(set.len(), 3);
        assert!(ColorSpaceId::Srgb < ColorSpaceId::LinearSrgb);
        assert!(ColorSpaceId::LinearSrgb < ColorSpaceId::DisplayP3);
    }
}
