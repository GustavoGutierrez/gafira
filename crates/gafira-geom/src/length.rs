//! Physical and pixel lengths with explicit-DPI conversion.

use std::fmt;

use serde::{Deserialize, Serialize};

/// Millimetres per inch.
const MM_PER_INCH: f64 = 25.4;
/// Centimetres per inch.
const CM_PER_INCH: f64 = 2.54;
/// Points per inch.
const PT_PER_INCH: f64 = 72.0;

/// A unit of length. Serialized as its lowercase name.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Unit {
    /// Device pixels; relates to inches through the DPI supplied at conversion.
    Px,
    /// Millimetres.
    Mm,
    /// Centimetres.
    Cm,
    /// Inches.
    In,
    /// Typographic points (1/72 in).
    Pt,
}

impl Unit {
    /// Every unit, in declaration order.
    pub const ALL: [Unit; 5] = [Unit::Px, Unit::Mm, Unit::Cm, Unit::In, Unit::Pt];

    /// Units of this kind per inch; `dpi` is only used by [`Unit::Px`].
    const fn per_inch(self, dpi: f64) -> f64 {
        match self {
            Unit::Px => dpi,
            Unit::Mm => MM_PER_INCH,
            Unit::Cm => CM_PER_INCH,
            Unit::In => 1.0,
            Unit::Pt => PT_PER_INCH,
        }
    }
}

/// A value together with the unit it was given in.
///
/// Never normalized implicitly. Construction does not validate: a non-finite
/// value can be built, but it cannot be converted and cannot be persisted
/// (JSON encodes it as `null`, which fails to decode as `f64`).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Length {
    value: f64,
    unit: Unit,
}

impl Length {
    /// Builds a length without validation.
    pub const fn new(value: f64, unit: Unit) -> Self {
        Self { value, unit }
    }

    /// The stored value, in [`Length::unit`].
    pub const fn value(self) -> f64 {
        self.value
    }

    /// The unit the value is expressed in.
    pub const fn unit(self) -> Unit {
        self.unit
    }

    /// Converts to `to` using the explicit `dpi` (pixels per inch).
    ///
    /// The DPI is validated on every call, including same-unit conversions.
    pub fn convert(self, to: Unit, dpi: f64) -> Result<Length, ConversionError> {
        if !dpi.is_finite() || dpi <= 0.0 {
            return Err(ConversionError::InvalidDpi(dpi));
        }
        if !self.value.is_finite() {
            return Err(ConversionError::NonFiniteValue(self.value));
        }
        if to == self.unit {
            return Ok(self);
        }
        let value = self.value / self.unit.per_inch(dpi) * to.per_inch(dpi);
        if !value.is_finite() {
            return Err(ConversionError::NonFiniteResult);
        }
        Ok(Length::new(value, to))
    }

    /// Converts to a pixel value at `dpi`, without rounding.
    pub fn to_px(self, dpi: f64) -> Result<f64, ConversionError> {
        self.convert(Unit::Px, dpi).map(Length::value)
    }
}

/// Why a [`Length`] conversion failed.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ConversionError {
    /// The DPI was zero, negative, NaN or infinite.
    InvalidDpi(f64),
    /// The length value was NaN or infinite.
    NonFiniteValue(f64),
    /// The converted value overflowed to infinity.
    NonFiniteResult,
}

impl fmt::Display for ConversionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidDpi(dpi) => {
                write!(f, "invalid dpi {dpi}: must be finite and greater than zero")
            }
            Self::NonFiniteValue(value) => {
                write!(f, "length value {value} is not finite")
            }
            Self::NonFiniteResult => f.write_str("conversion result is not finite"),
        }
    }
}

impl std::error::Error for ConversionError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn rel_close(actual: f64, expected: f64, tolerance: f64) -> bool {
        (actual - expected).abs() <= tolerance * expected.abs()
    }

    #[test]
    fn unit_set_has_exactly_five_variants() {
        assert_eq!(Unit::ALL.len(), 5);
        // Exhaustive match: adding a variant breaks compilation here.
        for unit in Unit::ALL {
            match unit {
                Unit::Px | Unit::Mm | Unit::Cm | Unit::In | Unit::Pt => {}
            }
        }
        for (i, a) in Unit::ALL.iter().enumerate() {
            for b in &Unit::ALL[i + 1..] {
                assert_ne!(a, b);
            }
        }
    }

    #[test]
    fn length_retains_value_and_unit() {
        let length = Length::new(10.0, Unit::Mm);
        assert_eq!(length.value().to_bits(), 10.0_f64.to_bits());
        assert_eq!(length.unit(), Unit::Mm);
        let other = Length::new(-2.5, Unit::Pt);
        assert_eq!(other.value().to_bits(), (-2.5_f64).to_bits());
        assert_eq!(other.unit(), Unit::Pt);
    }

    #[test]
    fn reference_conversions_of_25_4_mm() {
        let length = Length::new(25.4, Unit::Mm);

        let inches = length.convert(Unit::In, 96.0).unwrap();
        assert_eq!(inches.unit(), Unit::In);
        assert!(rel_close(inches.value(), 1.0, 1e-12), "{}", inches.value());

        let px = length.convert(Unit::Px, 96.0).unwrap();
        assert_eq!(px.unit(), Unit::Px);
        assert!(rel_close(px.value(), 96.0, 1e-12), "{}", px.value());

        let pt = length.convert(Unit::Pt, 96.0).unwrap();
        assert_eq!(pt.unit(), Unit::Pt);
        assert!(rel_close(pt.value(), 72.0, 1e-12), "{}", pt.value());
    }

    #[test]
    fn physical_ratios_hold() {
        let one_inch = Length::new(1.0, Unit::In);
        let cm = one_inch.convert(Unit::Cm, 96.0).unwrap();
        assert!(rel_close(cm.value(), 2.54, 1e-12));
        let mm = one_inch.convert(Unit::Mm, 300.0).unwrap();
        assert!(rel_close(mm.value(), 25.4, 1e-12));
        let px = one_inch.convert(Unit::Px, 300.0).unwrap();
        assert!(rel_close(px.value(), 300.0, 1e-12));
    }

    #[test]
    fn to_px_returns_the_pixel_value() {
        assert!(rel_close(
            Length::new(1.0, Unit::In).to_px(150.0).unwrap(),
            150.0,
            1e-12
        ));
        assert_eq!(Length::new(7.0, Unit::Px).to_px(300.0).unwrap(), 7.0);
        assert_eq!(
            Length::new(1.0, Unit::In).to_px(0.0),
            Err(ConversionError::InvalidDpi(0.0))
        );
    }

    #[test]
    fn invalid_dpi_is_an_error_for_every_kind() {
        let length = Length::new(10.0, Unit::Mm);
        for dpi in [0.0, -96.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            let error = length.convert(Unit::In, dpi).unwrap_err();
            assert!(
                matches!(error, ConversionError::InvalidDpi(d) if d.to_bits() == dpi.to_bits()),
                "dpi {dpi}: {error:?}"
            );
        }
    }

    #[test]
    fn invalid_dpi_is_rejected_even_for_same_unit_conversions() {
        let length = Length::new(3.0, Unit::Px);
        for dpi in [0.0, -1.0, f64::NAN, f64::INFINITY] {
            assert!(matches!(
                length.convert(Unit::Px, dpi),
                Err(ConversionError::InvalidDpi(_))
            ));
        }
    }

    #[test]
    fn non_finite_value_is_an_error() {
        for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            let error = Length::new(value, Unit::Mm)
                .convert(Unit::In, 96.0)
                .unwrap_err();
            assert!(
                matches!(error, ConversionError::NonFiniteValue(v) if v.to_bits() == value.to_bits()),
                "value {value}: {error:?}"
            );
        }
        // Same-unit conversion does not bypass the finite check.
        assert!(matches!(
            Length::new(f64::NAN, Unit::Px).convert(Unit::Px, 96.0),
            Err(ConversionError::NonFiniteValue(_))
        ));
    }

    #[test]
    fn invalid_dpi_takes_precedence_over_non_finite_value() {
        assert!(matches!(
            Length::new(f64::NAN, Unit::Px).convert(Unit::Mm, 0.0),
            Err(ConversionError::InvalidDpi(_))
        ));
    }

    #[test]
    fn overflowing_result_is_a_non_finite_result_error() {
        // 1e308 in -> px at 1e10 DPI overflows f64.
        assert_eq!(
            Length::new(1e308, Unit::In).convert(Unit::Px, 1e10),
            Err(ConversionError::NonFiniteResult)
        );
    }

    #[test]
    fn same_unit_conversion_is_bit_exact() {
        for value in [3.0, -0.0, 0.0, 5e-324, -1.234_567_890_123_456_7e300, 0.1] {
            for unit in Unit::ALL {
                let converted = Length::new(value, unit).convert(unit, 300.0).unwrap();
                assert_eq!(converted.value().to_bits(), value.to_bits());
                assert_eq!(converted.unit(), unit);
            }
        }
    }

    #[test]
    fn a4_at_300_dpi_is_2480_by_3508_pixels() {
        let width = Length::new(210.0, Unit::Mm).to_px(300.0).unwrap();
        let height = Length::new(297.0, Unit::Mm).to_px(300.0).unwrap();
        assert_eq!(width.round(), 2480.0);
        assert_eq!(height.round(), 3508.0);
    }

    #[test]
    fn nan_length_serializes_to_null_and_cannot_be_decoded() {
        let json = serde_json::to_string(&Length::new(f64::NAN, Unit::Px)).unwrap();
        assert_eq!(json, r#"{"value":null,"unit":"px"}"#);
        assert!(serde_json::from_str::<Length>(&json).is_err());

        let json = serde_json::to_string(&Length::new(f64::INFINITY, Unit::Mm)).unwrap();
        assert_eq!(json, r#"{"value":null,"unit":"mm"}"#);
        assert!(serde_json::from_str::<Length>(&json).is_err());
    }

    #[test]
    fn finite_length_serializes_with_lowercase_unit() {
        let json = serde_json::to_string(&Length::new(2.5, Unit::In)).unwrap();
        assert_eq!(json, r#"{"value":2.5,"unit":"in"}"#);
        let back: Length = serde_json::from_str(&json).unwrap();
        assert_eq!(back.value().to_bits(), 2.5_f64.to_bits());
        assert_eq!(back.unit(), Unit::In);
    }

    #[test]
    fn conversion_errors_display_their_cause() {
        let invalid = ConversionError::InvalidDpi(-96.0).to_string();
        assert!(
            invalid.contains("dpi") && invalid.contains("-96"),
            "{invalid}"
        );
        let value = ConversionError::NonFiniteValue(f64::NAN).to_string();
        assert!(value.contains("finite") && value.contains("NaN"), "{value}");
        let result = ConversionError::NonFiniteResult.to_string();
        assert!(result.contains("finite"), "{result}");
        let _: &dyn std::error::Error = &ConversionError::NonFiniteResult;
    }
}
