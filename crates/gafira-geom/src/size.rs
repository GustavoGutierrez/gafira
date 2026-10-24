//! A width and height pair.

use serde::{Deserialize, Serialize};

/// A width and height in caller-defined units. Not validated.
#[derive(Clone, Copy, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct Size {
    width: f64,
    height: f64,
}

impl Size {
    /// Builds a size without validation.
    pub const fn new(width: f64, height: f64) -> Self {
        Self { width, height }
    }

    /// The width.
    pub const fn width(self) -> f64 {
        self.width
    }

    /// The height.
    pub const fn height(self) -> f64 {
        self.height
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn size_holds_width_and_height() {
        let size = Size::new(210.0, 297.5);
        assert_eq!(size.width().to_bits(), 210.0_f64.to_bits());
        assert_eq!(size.height().to_bits(), 297.5_f64.to_bits());
        let other = Size::new(-0.0, 5e-324);
        assert_eq!(other.width().to_bits(), (-0.0_f64).to_bits());
        assert_eq!(other.height().to_bits(), 5e-324_f64.to_bits());
    }

    #[test]
    fn size_does_not_validate() {
        let size = Size::new(f64::NAN, -3.0);
        assert!(size.width().is_nan());
        assert_eq!(size.height(), -3.0);
    }

    #[test]
    fn default_size_is_zero_by_zero() {
        assert_eq!(Size::default(), Size::new(0.0, 0.0));
    }

    #[test]
    fn size_serializes_as_width_and_height() {
        let json = serde_json::to_string(&Size::new(1.5, 2.0)).unwrap();
        assert_eq!(json, r#"{"width":1.5,"height":2.0}"#);
        let back: Size = serde_json::from_str(&json).unwrap();
        assert_eq!(back, Size::new(1.5, 2.0));
        assert!(serde_json::from_str::<Size>(r#"{"width":1.5}"#).is_err());
    }
}
