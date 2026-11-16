//! The content key of a node of the render graph (02 §6.1).
//!
//! The key is the BLAKE3 hash of a canonical description, truncated to 16
//! bytes and read as a big-endian `u128`. The description is, in this order
//! and without separators: the domain byte, the type byte of the node, the
//! byte of the level of detail, the number of input keys as a big-endian
//! `u32`, that many input keys of 16 big-endian bytes each, and the sampled
//! parameters, each as a big-endian `u32` with the length of the value and the
//! bytes of the value; the bytes of a parameter open with the tag of its kind,
//! so two different kinds never give the same description.

use gafira_color::{Color, ColorSpaceId};
use gafira_doc::{BlendMode, ShapeGeometry};
use gafira_ports::render::{ContentKey, RenderQuality};

/// The domain byte of the render graph (02 §6.1).
pub const DOMAIN: u8 = b'R';

/// The domain byte of the composition of the roots of a region (O12).
///
/// It is distinct from [`DOMAIN`], so a composed key never collides with the
/// key of a node of the graph.
pub const COMPOSITE_DOMAIN: u8 = b'T';

/// The key of the tile that composes every root of a region (O12).
///
/// The roots arrive in paint order, and the key is the BLAKE3 hash of the
/// domain byte, their number and their keys in that order, truncated and read
/// as a [`content_key`] is. It changes when any root changes and when the
/// order changes, which is what a region's tile needs: the backend writes the
/// tile over the surface as the whole content of the region, so a stale tile
/// would serve an image of a page that no longer exists.
#[must_use]
pub fn composite_key(root_keys: &[ContentKey]) -> ContentKey {
    let mut description = Vec::with_capacity(5 + 16 * root_keys.len());
    description.push(COMPOSITE_DOMAIN);
    description.extend_from_slice(&(root_keys.len() as u32).to_be_bytes());
    for key in root_keys {
        description.extend_from_slice(&key.0.to_be_bytes());
    }
    let hash = blake3::hash(&description);
    let mut bytes = [0u8; 16];
    bytes.copy_from_slice(&hash.as_bytes()[..16]);
    ContentKey(u128::from_be_bytes(bytes))
}

/// The type of a node in the canonical description (02 §6.1).
///
/// The numbers are fixed: `1` is a group, `2` a shape, `3` an extension and
/// `4` a text. A kind added later takes the next free number and reuses none.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NodeType {
    /// A group: it arranges its children.
    Group = 1,
    /// A shape: it fills its own outline.
    Shape = 2,
    /// An extension. The byte is reserved by §6.1; the extension itself joins
    /// the graph when the handler seam of ARQ-137 lands.
    Extension = 3,
    /// A text element: it fills the outlines of its glyphs (spec 030).
    Text = 4,
    /// An image element: it draws the pixels of its asset (spec 032).
    Image = 5,
}

/// One parameter of a node, sampled at the instant of the graph (02 §6.1).
///
/// A parameter that does not depend on time holds the same value at every
/// instant and is written all the same. The value of a parameter opens with
/// the tag byte of its kind, so two different kinds never encode the same
/// bytes (ARQ-141).
#[derive(Clone, Debug, PartialEq)]
pub enum KeyParam {
    /// The quality of the graph: preview or export (02 §6.2).
    Quality(RenderQuality),
    /// The outline of a shape.
    Geometry(ShapeGeometry),
    /// The fill of a shape; `None` draws nothing.
    Fill(Option<Color>),
    /// The transform of the node in page space: the six coefficients.
    Transform([f64; 6]),
    /// The opacity of the node.
    Opacity(f32),
    /// The blend mode of the node.
    Blend(BlendMode),
    /// What a text draws: its family, size, alignment and words, in one
    /// canonical string (spec 030).
    Text(String),
    /// The content hash of the asset an image draws (spec 032).
    Asset([u8; 32]),
}

/// The canonical description of a node (02 §6.1).
///
/// The bytes are the ones the hash is taken over: the domain byte, the type
/// byte of the node, the byte of the level of detail, the number of input keys
/// as a big-endian `u32`, that many input keys of 16 big-endian bytes each,
/// and the sampled parameters, each as a big-endian `u32` with the length of
/// the value and the bytes of the value; the bytes of a parameter open with the
/// tag of its kind (ARQ-141).
#[must_use]
pub fn canonical_description(
    node_type: NodeType,
    lod: u8,
    input_keys: &[ContentKey],
    params: &[KeyParam],
) -> Vec<u8> {
    let mut out = Vec::with_capacity(32);
    out.push(DOMAIN);
    out.push(node_type as u8);
    out.push(lod);
    out.extend_from_slice(&(input_keys.len() as u32).to_be_bytes());
    for key in input_keys {
        out.extend_from_slice(&key.0.to_be_bytes());
    }
    for param in params {
        let mut value = Vec::new();
        param.write(&mut value);
        out.extend_from_slice(&(value.len() as u32).to_be_bytes());
        out.extend_from_slice(&value);
    }
    out
}

/// The content key of a node: the hash of its canonical description,
/// truncated to 16 bytes and read as a big-endian `u128` (02 §6.1).
#[must_use]
pub fn content_key(
    node_type: NodeType,
    lod: u8,
    input_keys: &[ContentKey],
    params: &[KeyParam],
) -> ContentKey {
    let hash = blake3::hash(&canonical_description(node_type, lod, input_keys, params));
    let mut bytes = [0u8; 16];
    bytes.copy_from_slice(&hash.as_bytes()[..16]);
    ContentKey(u128::from_be_bytes(bytes))
}

impl KeyParam {
    /// The tag byte of the kind of this parameter (02 §6.1).
    ///
    /// Like the node types of the section, the numbers are fixed: a kind added
    /// later takes the next free number and reuses none, and `0` stays free so
    /// that no tag can be confused with an empty value. The tag leads the value
    /// of the parameter, inside the length-prefixed block §6.1 describes, so
    /// that two different kinds never give the same bytes (ARQ-141).
    const fn tag(&self) -> u8 {
        match self {
            Self::Quality(_) => 1,
            Self::Geometry(_) => 2,
            Self::Fill(_) => 3,
            Self::Transform(_) => 4,
            Self::Opacity(_) => 5,
            Self::Blend(_) => 6,
            Self::Text(_) => 7,
            Self::Asset(_) => 8,
        }
    }

    /// The bytes of the value of this parameter: its tag and its payload.
    fn write(&self, out: &mut Vec<u8>) {
        out.push(self.tag());
        match self {
            Self::Quality(quality) => out.push(match quality {
                RenderQuality::Preview => 0,
                RenderQuality::Export => 1,
            }),
            Self::Geometry(geometry) => out.push(match geometry {
                ShapeGeometry::Rect => 0,
                ShapeGeometry::Ellipse => 1,
            }),
            Self::Fill(color) => match color {
                None => out.push(0),
                Some(color) => {
                    out.push(1);
                    out.push(space_byte(color.space()));
                    for channel in channels(*color) {
                        out.extend_from_slice(&f32_bytes(channel));
                    }
                }
            },
            Self::Transform(coefficients) => {
                for coefficient in coefficients {
                    out.extend_from_slice(&f64_bytes(*coefficient));
                }
            }
            Self::Opacity(opacity) => out.extend_from_slice(&f32_bytes(*opacity)),
            Self::Blend(blend) => out.push(blend_byte(*blend)),
            Self::Text(text) => out.extend_from_slice(text.as_bytes()),
            Self::Asset(hash) => out.extend_from_slice(hash),
        }
    }
}

/// The byte of a color space, in the order of [`ColorSpaceId::ALL`].
fn space_byte(space: ColorSpaceId) -> u8 {
    ColorSpaceId::ALL
        .iter()
        .position(|candidate| *candidate == space)
        .unwrap_or(0) as u8
}

/// The four channels of a color, in the order red, green, blue, alpha.
fn channels(color: Color) -> [f32; 4] {
    match color {
        Color::Srgb(c) => [c.r, c.g, c.b, c.a],
        Color::LinearSrgb(c) => [c.r, c.g, c.b, c.a],
        Color::DisplayP3(c) => [c.r, c.g, c.b, c.a],
    }
}

/// The byte of a blend mode, in the order of [`BlendMode::ALL`].
fn blend_byte(blend: BlendMode) -> u8 {
    BlendMode::ALL
        .iter()
        .position(|candidate| *candidate == blend)
        .unwrap_or(0) as u8
}

/// The bytes of an `f32`.
///
/// A NaN becomes the quiet NaN with a zero payload, so that two NaNs give the
/// same key; an infinity keeps its sign, so that `+∞` and `−∞` give different
/// keys (02 §6.1).
fn f32_bytes(value: f32) -> [u8; 4] {
    let bits = if value.is_nan() {
        0x7FC0_0000
    } else {
        value.to_bits()
    };
    bits.to_be_bytes()
}

/// The bytes of an `f64`, with the same rule as [`f32_bytes`].
fn f64_bytes(value: f64) -> [u8; 8] {
    let bits = if value.is_nan() {
        0x7FF8_0000_0000_0000
    } else {
        value.to_bits()
    };
    bits.to_be_bytes()
}

#[cfg(test)]
mod tests {
    use gafira_color::Rgba;

    use super::*;

    /// The parameters of a node that does not depend on time.
    fn params() -> Vec<KeyParam> {
        vec![
            KeyParam::Quality(RenderQuality::Preview),
            KeyParam::Geometry(ShapeGeometry::Rect),
            KeyParam::Fill(Some(Color::Srgb(Rgba::new(0.25, 0.5, 0.75, 1.0)))),
            KeyParam::Transform([1.0, 0.0, 0.0, 1.0, 10.0, 20.0]),
            KeyParam::Opacity(0.5),
            KeyParam::Blend(BlendMode::Multiply),
        ]
    }

    // ARQ-141: two nodes with the same data give the same key.
    #[test]
    fn arq_141_the_same_data_gives_the_same_key() {
        let first = content_key(NodeType::Shape, 0, &[], &params());
        let second = content_key(NodeType::Shape, 0, &[], &params());
        assert_eq!(first, second);
    }

    // ARQ-141: a different parameter, level, input key or type gives another
    // key.
    #[test]
    fn arq_141_a_different_input_gives_another_key() {
        let base = content_key(NodeType::Shape, 0, &[], &params());

        let mut other = params();
        other[4] = KeyParam::Opacity(0.25);
        assert_ne!(base, content_key(NodeType::Shape, 0, &[], &other));

        assert_ne!(base, content_key(NodeType::Shape, 1, &[], &params()));
        assert_ne!(base, content_key(NodeType::Group, 0, &[], &params()));

        let input = ContentKey(7);
        assert_ne!(base, content_key(NodeType::Shape, 0, &[input], &params()));
    }

    // ARQ-141: the kind of a parameter is part of the description. Two
    // different kinds never give the same key, and the same kind with the same
    // value still gives the same key.
    #[test]
    fn arq_141_a_different_parameter_kind_gives_another_key() {
        // The four kinds whose value is a single byte of zero, and their two
        // neighbours: the tag has to tell them apart.
        let parameters = [
            KeyParam::Quality(RenderQuality::Preview),
            KeyParam::Geometry(ShapeGeometry::Rect),
            KeyParam::Fill(None),
            KeyParam::Transform([0.0; 6]),
            KeyParam::Opacity(0.0),
            KeyParam::Blend(BlendMode::Normal),
        ];
        let keys: Vec<ContentKey> = parameters
            .iter()
            .map(|param| content_key(NodeType::Shape, 0, &[], std::slice::from_ref(param)))
            .collect();

        for (left, first) in keys.iter().enumerate() {
            for (right, second) in keys.iter().enumerate().skip(left + 1) {
                assert_ne!(
                    first, second,
                    "the parameters {left} and {right} give the same key"
                );
            }
        }

        assert_eq!(
            keys[2],
            content_key(NodeType::Shape, 0, &[], &[KeyParam::Fill(None)])
        );
    }

    // ARQ-141 / 02 §6.1: the domain byte and the node type bytes are the ones
    // the section fixes by name: `R`, and `1` = Group, `2` = Shape,
    // `3` = Extension.
    #[test]
    fn arq_141_the_domain_and_the_node_types_are_the_ones_of_the_section() {
        assert_eq!(DOMAIN, b'R');
        assert_eq!(NodeType::Group as u8, 1);
        assert_eq!(NodeType::Shape as u8, 2);
        assert_eq!(NodeType::Extension as u8, 3);

        let group = canonical_description(NodeType::Group, 0, &[], &[]);
        let shape = canonical_description(NodeType::Shape, 0, &[], &[]);
        let extension = canonical_description(NodeType::Extension, 0, &[], &[]);
        assert_eq!(group[..2], [b'R', 1]);
        assert_eq!(shape[..2], [b'R', 2]);
        assert_eq!(extension[..2], [b'R', 3]);
    }

    // ARQ-141: the key is the hash of the canonical description, truncated to
    // 16 bytes and read big-endian.
    #[test]
    fn arq_141_the_key_is_the_hash_of_the_description() {
        let description = canonical_description(NodeType::Shape, 0, &[], &params());
        let hash = blake3::hash(&description);
        let mut expected = [0u8; 16];
        expected.copy_from_slice(&hash.as_bytes()[..16]);
        assert_eq!(
            content_key(NodeType::Shape, 0, &[], &params()),
            ContentKey(u128::from_be_bytes(expected))
        );
    }

    // ARQ-141: the description holds the bytes §6.1 states, in that order.
    #[test]
    fn arq_141_the_description_holds_the_bytes_of_the_section() {
        let input = ContentKey(0x0102_0304_0506_0708_090A_0B0C_0D0E_0F10);
        let parameters = params();
        let description = canonical_description(NodeType::Shape, 2, &[input], &parameters);
        // The domain byte `R` and the type byte `2` of a shape, as §6.1 names
        // them, not through the constants of this module.
        assert_eq!(&description[0..3], &[b'R', 2, 2]);
        assert_eq!(&description[3..7], &1u32.to_be_bytes());
        assert_eq!(&description[7..23], &input.0.to_be_bytes());
        // The first parameter: its length and its bytes.
        let mut first = Vec::new();
        parameters[0].write(&mut first);
        assert_eq!(&description[23..27], &(first.len() as u32).to_be_bytes());
        assert_eq!(&description[27..27 + first.len()], &first[..]);
        // The last parameter ends the description.
        let mut last = Vec::new();
        parameters[parameters.len() - 1].write(&mut last);
        assert!(description.ends_with(&last));
    }

    // ARQ-141: a NaN is written with the canonical pattern, and an infinity
    // keeps its sign.
    #[test]
    fn arq_141_the_non_finite_pattern_of_the_section_is_used() {
        let nan = KeyParam::Opacity(f32::NAN);
        let other_nan = KeyParam::Opacity(-f32::NAN);
        let positive = KeyParam::Opacity(f32::INFINITY);
        let negative = KeyParam::Opacity(f32::NEG_INFINITY);

        assert_eq!(
            content_key(NodeType::Shape, 0, &[], std::slice::from_ref(&nan)),
            content_key(NodeType::Shape, 0, &[], &[other_nan])
        );
        assert_ne!(
            content_key(NodeType::Shape, 0, &[], std::slice::from_ref(&positive)),
            content_key(NodeType::Shape, 0, &[], std::slice::from_ref(&negative))
        );

        // The tag leads the value; the payload is the canonical pattern.
        let mut bytes = Vec::new();
        nan.write(&mut bytes);
        assert_eq!(&bytes[1..], &0x7FC0_0000u32.to_be_bytes()[..]);
        let mut bytes = Vec::new();
        positive.write(&mut bytes);
        assert_eq!(&bytes[1..], &0x7F80_0000u32.to_be_bytes()[..]);
        let mut bytes = Vec::new();
        negative.write(&mut bytes);
        assert_eq!(&bytes[1..], &0xFF80_0000u32.to_be_bytes()[..]);
    }

    // ARQ-141: a node without inputs is valid: the count of input keys may be
    // zero.
    #[test]
    fn arq_141_a_node_without_inputs_is_valid() {
        let description = canonical_description(NodeType::Shape, 0, &[], &params());
        assert_eq!(&description[3..7], &0u32.to_be_bytes());
        assert_ne!(
            ContentKey(0),
            content_key(NodeType::Shape, 0, &[], &params())
        );
    }
}
