//! Document resolution in pixels per inch.

use serde::{Deserialize, Serialize};

/// Pixels per inch, stored once per document.
///
/// Construction does not validate: any `u32` can be carried, and
/// `Document::validate` rejects values outside [`Dpi::MIN`]..=[`Dpi::MAX`].
/// The wire form is a bare integer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Dpi(u32);

impl Dpi {
    /// The default resolution, 96 pixels per inch.
    pub const DEFAULT: Dpi = Dpi(96);
    /// Smallest valid value.
    pub const MIN: u32 = 1;
    /// Largest valid value.
    pub const MAX: u32 = 10_000;

    /// Wraps a raw value without validation.
    pub const fn new(value: u32) -> Self {
        Self(value)
    }

    /// The raw value.
    pub const fn get(self) -> u32 {
        self.0
    }

    /// True exactly for `MIN..=MAX`.
    pub const fn is_valid(self) -> bool {
        self.0 >= Self::MIN && self.0 <= Self::MAX
    }
}

impl Default for Dpi {
    fn default() -> Self {
        Self::DEFAULT
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_96_and_valid() {
        assert_eq!(Dpi::default().get(), 96);
        assert_eq!(Dpi::DEFAULT.get(), 96);
        assert!(Dpi::default().is_valid());
        assert!(Dpi::DEFAULT.is_valid());
    }

    #[test]
    fn range_edges() {
        let cases = [
            (0, false),
            (1, true),
            (10_000, true),
            (10_001, false),
            (u32::MAX, false),
        ];
        for (v, valid) in cases {
            assert_eq!(Dpi::new(v).is_valid(), valid, "dpi {v}");
            assert_eq!(Dpi::new(v).get(), v);
        }
        assert_eq!((Dpi::MIN, Dpi::MAX), (1, 10_000));
    }

    #[test]
    fn wire_form() {
        let json = serde_json::to_string(&Dpi::new(300)).unwrap();
        assert_eq!(json, "300");
        assert_eq!(serde_json::from_str::<Dpi>(&json).unwrap(), Dpi::new(300));
        assert_eq!(
            serde_json::from_str::<Dpi>("4294967295").unwrap(),
            Dpi::new(u32::MAX)
        );
        assert_eq!(serde_json::from_str::<Dpi>("0").unwrap(), Dpi::new(0));
    }

    #[test]
    fn non_integer_wire_values_are_rejected() {
        for bad in ["-1", "96.0", "1e2", "4294967296", "null", "\"96\""] {
            assert!(serde_json::from_str::<Dpi>(bad).is_err(), "{bad}");
        }
    }
}
