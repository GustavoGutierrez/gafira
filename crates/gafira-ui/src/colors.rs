//! Colours to pick a page background from (spec 069): a palette, gradient
//! presets, and a search by name (Spanish or English) or by hex code.

use gafira_color::{Color, Rgba};

/// An opaque sRGB colour from its 0xRRGGBB code.
pub(crate) fn hex(code: u32) -> Color {
    let channel = |shift: u32| ((code >> shift) & 0xFF) as f32 / 255.0;
    Color::Srgb(Rgba::new(channel(16), channel(8), channel(0), 1.0))
}

/// The palette of «Fondo de la página».
pub(crate) const PALETTE: [u32; 18] = [
    0x000000, 0xFFFFFF, 0x7B61FF, 0xDCD6F7, 0x1C1C1E, 0x6E6262, 0x18243B, 0xFF5A7A, 0x2B1D14,
    0xCBC3FF, 0xFFAAB8, 0xFFC9D4, 0xFFE6A7, 0xC4E9DA, 0xE84545, 0xF57A1F, 0xF4A21B, 0xE8B90C,
];

/// The gradient presets of «Fondo de la página»: two colours and an angle.
pub(crate) const GRADIENTS: [(u32, u32, f64); 6] = [
    (0xF5576C, 0xF39C12, 45.0),
    (0x2563EB, 0x06B6D4, 45.0),
    (0xBE185D, 0x7C3AED, 45.0),
    (0x34D399, 0xD1FAE5, 90.0),
    (0x881337, 0x1E1B4B, 45.0),
    (0xFFE4E6, 0xFFF7ED, 90.0),
];

/// The solid presets of «Color del texto» (spec 070): greys, then rows of
/// reds and pinks, blues and cyans, greens and oranges.
pub(crate) const TEXT_PALETTE: [u32; 28] = [
    0x000000, 0x545454, 0x737373, 0xA6A6A6, 0xB4B4B4, 0xD9D9D9, 0xFFFFFF, 0xFF3131, 0xFF5757,
    0xFF66C4, 0xE2A9F1, 0xCB6CE6, 0x8C52FF, 0x5E17EB, 0x0097B2, 0x0CC0DF, 0x5CE1E6, 0x38B6FF,
    0x5271FF, 0x004AAD, 0x1800AD, 0x00BF63, 0x7ED957, 0xC1FF72, 0xFFDE59, 0xFFBD59, 0xFF914D,
    0xFF751F,
];

/// The gradient presets of «Color del texto» (spec 070): two colours and
/// an angle.
pub(crate) const TEXT_GRADIENTS: [(u32, u32, f64); 14] = [
    (0x000000, 0x737373, 0.0),
    (0x000000, 0xFFFFFF, 0.0),
    (0xA6A6A6, 0xFFFFFF, 0.0),
    (0xC9E265, 0x5CB85C, 45.0),
    (0x000000, 0xC89116, 0.0),
    (0x8C52FF, 0xFFDE59, 90.0),
    (0x000000, 0x5E17EB, 45.0),
    (0xA6E3F3, 0xE2A9F1, 45.0),
    (0xFF3131, 0xFF914D, 45.0),
    (0xFF5757, 0x8C52FF, 45.0),
    (0x5170FF, 0xFF66C4, 45.0),
    (0x004AAD, 0xCB6CE6, 45.0),
    (0x0097B2, 0x7ED957, 45.0),
    (0xFFDE59, 0xFF914D, 45.0),
];

/// A linear gradient from `a` to `b` at `angle_deg`.
pub(crate) fn gradient(a: u32, b: u32, angle_deg: f64) -> gafira_doc::Gradient {
    gafira_doc::Gradient {
        kind: gafira_doc::GradientKind::Linear { angle_deg },
        stops: vec![
            gafira_doc::GradientStop {
                offset: 0.0,
                color: hex(a),
            },
            gafira_doc::GradientStop {
                offset: 1.0,
                color: hex(b),
            },
        ],
    }
}

/// Named colours, Spanish and English names for each.
const NAMES: [(&str, &str, u32); 24] = [
    ("negro", "black", 0x000000),
    ("blanco", "white", 0xFFFFFF),
    ("gris", "gray", 0x808080),
    ("gris claro", "light gray", 0xD3D3D3),
    ("rojo", "red", 0xE53935),
    ("rosa", "pink", 0xFF6FA5),
    ("fucsia", "fuchsia", 0xFF00A8),
    ("morado", "purple", 0x7B3FE4),
    ("violeta", "violet", 0x8F5CFF),
    ("lila", "lilac", 0xC8A2C8),
    ("azul", "blue", 0x1E63E9),
    ("azul marino", "navy", 0x0B1F4B),
    ("celeste", "sky blue", 0x5EC8F2),
    ("turquesa", "turquoise", 0x1CC6C0),
    ("verde", "green", 0x2EAD4B),
    ("verde lima", "lime", 0x9BE33B),
    ("menta", "mint", 0x3FBFA9),
    ("amarillo", "yellow", 0xFFD400),
    ("naranja", "orange", 0xFF7A1A),
    ("coral", "coral", 0xFF6B57),
    ("marrón", "brown", 0x7B4A2A),
    ("beige", "beige", 0xEADFC8),
    ("dorado", "gold", 0xD4A017),
    ("plateado", "silver", 0xC0C0C0),
];

/// `text` without accents, in lower case, trimmed.
fn plain(text: &str) -> String {
    text.trim()
        .to_lowercase()
        .chars()
        .map(|c| match c {
            'á' => 'a',
            'é' => 'e',
            'í' => 'i',
            'ó' => 'o',
            'ú' | 'ü' => 'u',
            other => other,
        })
        .collect()
}

/// A colour from a hex code: `#RGB`, `#RRGGBB` or `#RRGGBBAA`, with or
/// without `#`.
pub(crate) fn parse_hex(text: &str) -> Option<Color> {
    let digits = text.trim().trim_start_matches('#');
    if !digits.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let value = u32::from_str_radix(digits, 16).ok()?;
    match digits.len() {
        3 => {
            let (r, g, b) = ((value >> 8) & 0xF, (value >> 4) & 0xF, value & 0xF);
            Some(hex(((r * 17) << 16) | ((g * 17) << 8) | (b * 17)))
        }
        6 => Some(hex(value)),
        8 => {
            let Color::Srgb(c) = hex(value >> 8) else {
                return None;
            };
            Some(Color::Srgb(Rgba::new(
                c.r,
                c.g,
                c.b,
                (value & 0xFF) as f32 / 255.0,
            )))
        }
        _ => None,
    }
}

/// The colours a search finds: the hex code it is, and the named colours
/// whose Spanish or English name holds it (accents and case aside).
pub(crate) fn search(query: &str) -> Vec<Color> {
    let query = plain(query);
    if query.is_empty() {
        return Vec::new();
    }
    let mut found: Vec<Color> = parse_hex(&query).into_iter().collect();
    found.extend(
        NAMES
            .iter()
            .filter(|(es, en, _)| plain(es).contains(&query) || en.contains(&query))
            .map(|(_, _, code)| hex(*code)),
    );
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    // S69-04: a hex code is read with or without «#», short or long, and
    // with transparency; anything else is not a colour.
    #[test]
    fn s69_04_hex_codes() {
        assert_eq!(parse_hex("#00c4cc"), Some(hex(0x00C4CC)));
        assert_eq!(parse_hex("00C4CC"), Some(hex(0x00C4CC)));
        assert_eq!(parse_hex("#0cc"), Some(hex(0x00CCCC)));
        assert!(matches!(parse_hex("#00c4cc80"), Some(Color::Srgb(c)) if (c.a - 0.5).abs() < 0.01));
        assert_eq!(parse_hex("azul"), None);
        assert_eq!(parse_hex("#12345"), None);
    }

    // S69-04: a word finds the colours named so, in Spanish or English,
    // accents and case aside.
    #[test]
    fn s69_04_colours_by_name() {
        assert_eq!(search("azul"), [hex(0x1E63E9), hex(0x0B1F4B)]);
        assert_eq!(search("Blue"), [hex(0x1E63E9), hex(0x5EC8F2)]);
        assert_eq!(search("marron"), [hex(0x7B4A2A)]);
        assert_eq!(search("#ff0000"), [hex(0xFF0000)]);
        assert!(search("   ").is_empty());
        assert!(search("xyz").is_empty());
    }
}
