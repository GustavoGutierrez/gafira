//! Page dimensions and their pixel limits.

use std::fmt;

use gafira_geom::{ConversionError, Length, Size};
use serde::{Deserialize, Serialize};

use crate::dpi::Dpi;

/// Page dimensions in any unit. Construction does not validate.
///
/// Units resolve to pixels through the document DPI (`gafira_geom` does the
/// unit math); [`PageSize::check`] enforces the pixel limits.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct PageSize {
    /// Page width.
    pub width: Length,
    /// Page height.
    pub height: Length,
}

/// A page axis, used to say which side of a [`PageSize`] failed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Axis {
    /// The horizontal side.
    Width,
    /// The vertical side.
    Height,
}

impl fmt::Display for Axis {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Width => "width",
            Self::Height => "height",
        })
    }
}

/// Why a [`PageSize`] failed [`PageSize::check`].
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PageSizeError {
    /// The side could not be converted to pixels (non-finite, overflow, bad DPI).
    Conversion {
        /// The failing side.
        axis: Axis,
        /// The conversion failure.
        error: ConversionError,
    },
    /// The unrounded pixel value lies outside `MIN_PX..=MAX_PX`.
    OutOfRange {
        /// The failing side.
        axis: Axis,
        /// The unrounded pixel value.
        px: f64,
    },
}

impl fmt::Display for PageSizeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Conversion { axis, error } => {
                write!(f, "page {axis} cannot be converted to pixels: {error}")
            }
            Self::OutOfRange { axis, px } => write!(
                f,
                "page {axis} is {px} px, outside {}..={} px",
                PageSize::MIN_PX,
                PageSize::MAX_PX
            ),
        }
    }
}

impl std::error::Error for PageSizeError {}

impl PageSize {
    /// Smallest allowed side in unrounded pixels (inclusive).
    pub const MIN_PX: f64 = 1.0;
    /// Largest allowed side in unrounded pixels (inclusive).
    pub const MAX_PX: f64 = 16_384.0;

    /// Both sides in unrounded pixels at `dpi`.
    pub fn to_px(self, dpi: Dpi) -> Result<Size, ConversionError> {
        let dpi = f64::from(dpi.get());
        Ok(Size::new(self.width.to_px(dpi)?, self.height.to_px(dpi)?))
    }

    /// Checks each side (width first) against `MIN_PX..=MAX_PX` at `dpi`.
    pub fn check(self, dpi: Dpi) -> Result<(), PageSizeError> {
        Self::check_axis(Axis::Width, self.width, dpi)?;
        Self::check_axis(Axis::Height, self.height, dpi)
    }

    fn check_axis(axis: Axis, side: Length, dpi: Dpi) -> Result<(), PageSizeError> {
        let px = side
            .to_px(f64::from(dpi.get()))
            .map_err(|error| PageSizeError::Conversion { axis, error })?;
        if (Self::MIN_PX..=Self::MAX_PX).contains(&px) {
            Ok(())
        } else {
            Err(PageSizeError::OutOfRange { axis, px })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gafira_geom::Unit;
    use proptest::prelude::*;

    fn w(length: Length) -> PageSize {
        PageSize {
            width: length,
            height: Length::new(100.0, Unit::Px),
        }
    }

    fn h(length: Length) -> PageSize {
        PageSize {
            width: Length::new(100.0, Unit::Px),
            height: length,
        }
    }

    fn px(v: f64) -> Length {
        Length::new(v, Unit::Px)
    }

    fn dpi96() -> Dpi {
        Dpi::new(96)
    }

    #[test]
    fn bounds_are_inclusive() {
        for v in [1.0, 16_384.0] {
            let size = PageSize {
                width: px(v),
                height: px(v),
            };
            assert_eq!(size.check(dpi96()), Ok(()));
            let out = size.to_px(dpi96()).unwrap();
            assert_eq!(out.width().to_bits(), v.to_bits());
            assert_eq!(out.height().to_bits(), v.to_bits());
        }
        assert_eq!((PageSize::MIN_PX, PageSize::MAX_PX), (1.0, 16_384.0));
    }

    #[test]
    fn one_step_outside_the_bounds_names_the_axis() {
        let below = 1.0_f64.next_down();
        let above = 16_384.0_f64.next_up();
        for bad in [below, above] {
            assert_eq!(
                w(px(bad)).check(dpi96()),
                Err(PageSizeError::OutOfRange {
                    axis: Axis::Width,
                    px: bad
                })
            );
            assert_eq!(
                h(px(bad)).check(dpi96()),
                Err(PageSizeError::OutOfRange {
                    axis: Axis::Height,
                    px: bad
                })
            );
        }
    }

    #[test]
    fn width_is_checked_before_height() {
        let size = PageSize {
            width: px(0.0),
            height: px(f64::NAN),
        };
        assert!(matches!(
            size.check(dpi96()),
            Err(PageSizeError::OutOfRange {
                axis: Axis::Width,
                ..
            })
        ));
    }

    #[test]
    fn bounds_reached_through_a_physical_unit() {
        let inch = |v: f64| Length::new(v, Unit::In);
        assert_eq!(w(inch(1.0)).check(Dpi::new(1)), Ok(()));
        assert!(matches!(
            w(inch(0.5)).check(Dpi::new(1)),
            Err(PageSizeError::OutOfRange {
                axis: Axis::Width,
                ..
            })
        ));
        assert_eq!(w(inch(8192.0)).check(Dpi::new(2)), Ok(()));
        assert!(matches!(
            w(inch(8192.0)).check(Dpi::new(3)),
            Err(PageSizeError::OutOfRange {
                axis: Axis::Width,
                ..
            })
        ));
    }

    #[test]
    fn a4_at_300_dpi() {
        let a4 = PageSize {
            width: Length::new(210.0, Unit::Mm),
            height: Length::new(297.0, Unit::Mm),
        };
        let size = a4.to_px(Dpi::new(300)).unwrap();
        assert_eq!(size.width().round(), 2480.0);
        assert_eq!(size.height().round(), 3508.0);
        assert_ne!(size.width(), size.width().round());
        assert_eq!(a4.width, Length::new(210.0, Unit::Mm));
        assert_ne!(size.height(), size.height().round());
        assert_eq!(a4.height, Length::new(297.0, Unit::Mm));
        assert_eq!(a4.check(Dpi::new(300)), Ok(()));
    }

    #[test]
    fn a0_follows_the_document_dpi() {
        let a0 = PageSize {
            width: Length::new(841.0, Unit::Mm),
            height: Length::new(1189.0, Unit::Mm),
        };
        assert_eq!(a0.check(Dpi::new(300)), Ok(()));
        assert_eq!(a0.check(Dpi::new(350)), Ok(()));
        let at_300 = a0.to_px(Dpi::new(300)).unwrap();
        assert_eq!(at_300.width().round(), 9933.0);
        assert_eq!(at_300.height().round(), 14043.0);
        let at_350 = a0.to_px(Dpi::new(350)).unwrap();
        assert_eq!(at_350.width().round(), 11589.0);
        assert_eq!(at_350.height().round(), 16384.0);
        match a0.check(Dpi::new(351)) {
            Err(PageSizeError::OutOfRange {
                axis: Axis::Height,
                px,
            }) => assert!((px - 16_430.7).abs() < 0.1, "{px}"),
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn zero_negative_and_non_finite_sides() {
        for v in [0.0, -0.0, -5.0] {
            assert!(matches!(
                w(px(v)).check(dpi96()),
                Err(PageSizeError::OutOfRange {
                    axis: Axis::Width,
                    ..
                })
            ));
        }
        for v in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert!(matches!(
                w(px(v)).check(dpi96()),
                Err(PageSizeError::Conversion {
                    axis: Axis::Width,
                    ..
                })
            ));
            assert!(matches!(
                h(px(v)).check(dpi96()),
                Err(PageSizeError::Conversion {
                    axis: Axis::Height,
                    ..
                })
            ));
        }
    }

    #[test]
    fn overflowing_conversion_and_invalid_dpi() {
        assert!(matches!(
            w(Length::new(1e308, Unit::In)).check(Dpi::new(10_000)),
            Err(PageSizeError::Conversion { .. })
        ));
        assert!(matches!(
            w(Length::new(10.0, Unit::Mm)).check(Dpi::new(0)),
            Err(PageSizeError::Conversion { .. })
        ));
        assert!(w(Length::new(10.0, Unit::Mm)).to_px(Dpi::new(0)).is_err());
    }

    #[test]
    fn errors_display_the_axis() {
        let out = PageSizeError::OutOfRange {
            axis: Axis::Height,
            px: 20_000.0,
        };
        assert!(out.to_string().contains("height"), "{out}");
        let conv = PageSizeError::Conversion {
            axis: Axis::Width,
            error: ConversionError::NonFiniteResult,
        };
        assert!(conv.to_string().contains("width"), "{conv}");
        let _: &dyn std::error::Error = &out;
    }

    #[test]
    fn wire_form() {
        let a4 = PageSize {
            width: Length::new(210.0, Unit::Mm),
            height: Length::new(297.0, Unit::Mm),
        };
        let json = serde_json::to_string(&a4).unwrap();
        assert_eq!(
            json,
            r#"{"width":{"value":210.0,"unit":"mm"},"height":{"value":297.0,"unit":"mm"}}"#
        );
        assert_eq!(serde_json::from_str::<PageSize>(&json).unwrap(), a4);
    }

    proptest! {
        #[test]
        fn unit_round_trip(
            target in 1.0_f64..=16_384.0,
            dpi in 1u32..=10_000,
            unit in prop::sample::select(Unit::ALL.to_vec()),
        ) {
            let dpi_f = f64::from(dpi);
            let value = px(target).convert(unit, dpi_f).unwrap();
            let size = w(value).to_px(Dpi::new(dpi)).unwrap();
            let back = px(size.width()).convert(unit, dpi_f).unwrap().value();
            let rel = ((back - value.value()) / value.value()).abs();
            prop_assert!(rel < 1e-6, "rel {rel}");
        }
    }
}
