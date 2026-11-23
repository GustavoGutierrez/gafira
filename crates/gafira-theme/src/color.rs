//! sRGB colours of the tokens and the WCAG 2.x contrast ratio (03 §4).

use std::fmt;

/// An opaque sRGB colour with 8 bits per channel, as written in the tokens file.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, PartialOrd, Ord)]
pub struct Rgb8 {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Rgb8 {
    #[must_use]
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    /// Parses `#RRGGBB` (either case). Any other form is `None`.
    #[must_use]
    pub fn parse_hex(text: &str) -> Option<Self> {
        let hex = text.strip_prefix('#')?;
        if hex.len() != 6 || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
            return None;
        }
        let channel = |i: usize| u8::from_str_radix(&hex[i..i + 2], 16).ok();
        Some(Self::new(channel(0)?, channel(2)?, channel(4)?))
    }

    /// WCAG 2.x relative luminance: linearized sRGB channels weighted by BT.709.
    #[must_use]
    pub fn relative_luminance(self) -> f64 {
        0.2126 * linear(self.r) + 0.7152 * linear(self.g) + 0.0722 * linear(self.b)
    }
}

impl fmt::Display for Rgb8 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "#{:02X}{:02X}{:02X}", self.r, self.g, self.b)
    }
}

/// The sRGB transfer function, inverted, on one 8-bit channel.
fn linear(channel: u8) -> f64 {
    let c = f64::from(channel) / 255.0;
    if c <= 0.040_45 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

/// `(L1 + 0.05) / (L2 + 0.05)` with `L1` the lighter of the two: always ≥ 1.
#[must_use]
pub fn contrast_ratio(a: Rgb8, b: Rgb8) -> f64 {
    let (la, lb) = (a.relative_luminance(), b.relative_luminance());
    let (hi, lo) = if la >= lb { (la, lb) } else { (lb, la) };
    (hi + 0.05) / (lo + 0.05)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex(text: &str) -> Rgb8 {
        Rgb8::parse_hex(text).expect("valid hex")
    }

    fn rounded(v: f64) -> f64 {
        (v * 100.0).round() / 100.0
    }

    // S14-06: the formula reproduces the brand manual ratios (03 §4).
    #[test]
    fn s14_06_formula_reproduces_the_manual_ratios() {
        let cases = [
            ("#18243B", "#FAFAF7", 14.83),
            ("#FFFFFF", "#18243B", 15.51),
            ("#18243B", "#FF6B57", 5.53),
            ("#FFFFFF", "#FF6B57", 2.80),
            ("#18243B", "#3FBFA9", 6.82),
        ];
        for (fg, bg, want) in cases {
            assert_eq!(
                rounded(contrast_ratio(hex(fg), hex(bg))),
                want,
                "{fg} on {bg}"
            );
        }
    }

    #[test]
    fn s14_06_ratio_is_symmetric_and_one_for_equal_colours() {
        let (a, b) = (hex("#46526B"), hex("#FAFAF7"));
        assert_eq!(contrast_ratio(a, b), contrast_ratio(b, a));
        assert_eq!(contrast_ratio(a, a), 1.0);
        assert_eq!(
            rounded(contrast_ratio(hex("#000000"), hex("#FFFFFF"))),
            21.0
        );
    }

    // S14-03: only `#RRGGBB`.
    #[test]
    fn s14_03_hex_parsing_accepts_only_six_digits() {
        assert_eq!(
            Rgb8::parse_hex("#18243B"),
            Some(Rgb8::new(0x18, 0x24, 0x3B))
        );
        assert_eq!(
            Rgb8::parse_hex("#ff6b57"),
            Some(Rgb8::new(0xFF, 0x6B, 0x57))
        );
        for bad in [
            "#18243", "18243B", "#GG0000", "#18243B0", "", "#", "#+1243B",
        ] {
            assert_eq!(Rgb8::parse_hex(bad), None, "{bad}");
        }
        assert_eq!(hex("#0a1120").to_string(), "#0A1120");
    }
}
