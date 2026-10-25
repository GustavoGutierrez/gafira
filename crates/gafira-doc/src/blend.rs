//! The W3C compositing blend modes.

use serde::{Deserialize, Serialize};

/// The 16 W3C Compositing blend modes, closed and exhaustive.
///
/// The wire form is the CSS kebab-case name (`color-dodge`); any other
/// spelling fails to deserialize.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BlendMode {
    /// Source replaces backdrop.
    #[default]
    Normal,
    /// Multiply.
    Multiply,
    /// Screen.
    Screen,
    /// Overlay.
    Overlay,
    /// Darken.
    Darken,
    /// Lighten.
    Lighten,
    /// Color dodge.
    ColorDodge,
    /// Color burn.
    ColorBurn,
    /// Hard light.
    HardLight,
    /// Soft light.
    SoftLight,
    /// Difference.
    Difference,
    /// Exclusion.
    Exclusion,
    /// Hue.
    Hue,
    /// Saturation.
    Saturation,
    /// Color.
    Color,
    /// Luminosity.
    Luminosity,
}

impl BlendMode {
    /// Every mode, in declaration order.
    pub const ALL: [BlendMode; 16] = [
        BlendMode::Normal,
        BlendMode::Multiply,
        BlendMode::Screen,
        BlendMode::Overlay,
        BlendMode::Darken,
        BlendMode::Lighten,
        BlendMode::ColorDodge,
        BlendMode::ColorBurn,
        BlendMode::HardLight,
        BlendMode::SoftLight,
        BlendMode::Difference,
        BlendMode::Exclusion,
        BlendMode::Hue,
        BlendMode::Saturation,
        BlendMode::Color,
        BlendMode::Luminosity,
    ];
}

#[cfg(test)]
mod tests {
    use super::*;

    const NAMES: [&str; 16] = [
        "normal",
        "multiply",
        "screen",
        "overlay",
        "darken",
        "lighten",
        "color-dodge",
        "color-burn",
        "hard-light",
        "soft-light",
        "difference",
        "exclusion",
        "hue",
        "saturation",
        "color",
        "luminosity",
    ];

    /// No wildcard arm: adding a variant breaks compilation on purpose.
    fn index(mode: BlendMode) -> usize {
        match mode {
            BlendMode::Normal => 0,
            BlendMode::Multiply => 1,
            BlendMode::Screen => 2,
            BlendMode::Overlay => 3,
            BlendMode::Darken => 4,
            BlendMode::Lighten => 5,
            BlendMode::ColorDodge => 6,
            BlendMode::ColorBurn => 7,
            BlendMode::HardLight => 8,
            BlendMode::SoftLight => 9,
            BlendMode::Difference => 10,
            BlendMode::Exclusion => 11,
            BlendMode::Hue => 12,
            BlendMode::Saturation => 13,
            BlendMode::Color => 14,
            BlendMode::Luminosity => 15,
        }
    }

    #[test]
    fn all_is_exhaustive_and_distinct() {
        assert_eq!(BlendMode::ALL.len(), 16);
        let indexes: std::collections::BTreeSet<usize> =
            BlendMode::ALL.iter().map(|m| index(*m)).collect();
        assert_eq!(indexes.len(), 16);
        for (i, mode) in BlendMode::ALL.iter().enumerate() {
            assert_eq!(index(*mode), i, "ALL is in declaration order");
        }
    }

    #[test]
    fn wire_names() {
        for (mode, name) in BlendMode::ALL.iter().zip(NAMES) {
            let json = serde_json::to_string(mode).unwrap();
            assert_eq!(json, format!("\"{name}\""));
            assert_eq!(serde_json::from_str::<BlendMode>(&json).unwrap(), *mode);
        }
    }

    #[test]
    fn unknown_or_miscased_names_are_rejected() {
        for bad in ["\"Multiply\"", "\"color_dodge\"", "\"nope\"", "\"\""] {
            assert!(serde_json::from_str::<BlendMode>(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn default_is_normal() {
        assert_eq!(BlendMode::default(), BlendMode::Normal);
    }
}
