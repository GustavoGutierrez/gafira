//! «Combinaciones de fuentes» of the Texto tab (owner request): a heading
//! and a line of body text in two fonts of the library that go together,
//! added as one group.

use gafira_doc::{GroupNode, NodeKind, Placement, SpanStyle, TextEffect, TextEffectKind as K};
use gafira_edit::NodeSpec;

/// One text of a pairing: its family, size, weight, colour (0xRRGGBB) and
/// whether its words are in capitals.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Face {
    pub(crate) font: &'static str,
    pub(crate) size: f64,
    pub(crate) bold: bool,
    pub(crate) color: u32,
    pub(crate) upper: bool,
    /// The effects of its letters (owner request: pairings with effects).
    pub(crate) effects: fn() -> Vec<TextEffect>,
}

/// A pairing: its name's catalogue key, its heading and its body.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Pairing {
    pub(crate) key: &'static str,
    pub(crate) heading: Face,
    pub(crate) body: Face,
}

const INK: u32 = 0x18243B;
const SOFT: u32 = 0x46526B;

fn none() -> Vec<TextEffect> {
    Vec::new()
}

const fn face(font: &'static str, size: f64, bold: bool, color: u32) -> Face {
    Face {
        font,
        size,
        bold,
        color,
        upper: false,
        effects: none,
    }
}

/// A heading with `effects`.
const fn lit(font: &'static str, size: f64, color: u32, effects: fn() -> Vec<TextEffect>) -> Face {
    Face {
        font,
        size,
        bold: false,
        color,
        upper: false,
        effects,
    }
}

/// A line in capitals, small and bold, as many headings with effects have.
const fn caps(font: &'static str, size: f64, color: u32) -> Face {
    Face {
        font,
        size,
        bold: true,
        color,
        upper: true,
        effects: none,
    }
}

/// The effect `kind` in `color` (0xRRGGBB), changed by `change`.
fn with(kind: K, color: u32, change: impl FnOnce(&mut TextEffect)) -> TextEffect {
    let mut effect = TextEffect::new(kind);
    effect.color = crate::colors::hex(color);
    change(&mut effect);
    effect
}

/// The pairings, in the order of the panel.
pub(crate) const PAIRINGS: [Pairing; 24] = [
    Pairing {
        key: "pairing.classic",
        heading: face("DM Serif Display", 64.0, false, INK),
        body: face("Lato", 26.0, false, SOFT),
    },
    Pairing {
        key: "pairing.modern",
        heading: face("Montserrat", 64.0, true, INK),
        body: face("Inter", 26.0, false, SOFT),
    },
    Pairing {
        key: "pairing.editorial",
        heading: face("DM Serif Display", 64.0, false, 0x5B3FD1),
        body: face("Source Serif 4", 26.0, false, SOFT),
    },
    Pairing {
        key: "pairing.playful",
        heading: face("Pacifico", 60.0, false, 0xE0457B),
        body: face("Poppins", 26.0, false, SOFT),
    },
    Pairing {
        key: "pairing.poster",
        heading: Face {
            font: "Bebas Neue",
            size: 80.0,
            bold: false,
            color: INK,
            upper: true,
            effects: none,
        },
        body: face("DM Sans", 26.0, false, SOFT),
    },
    Pairing {
        key: "pairing.elegant",
        heading: face("Cormorant Garamond", 66.0, false, 0x8A1C3B),
        body: face("Lato", 24.0, false, SOFT),
    },
    // Script headings over a plain body line (owner request, with
    // examples): welcome cards, invitations, menus and signatures.
    Pairing {
        key: "pairing.welcome",
        heading: face("Kaushan Script", 66.0, false, INK),
        body: Face {
            font: "Montserrat",
            size: 22.0,
            bold: true,
            color: INK,
            upper: true,
            effects: none,
        },
    },
    Pairing {
        key: "pairing.romantic",
        heading: face("Great Vibes", 76.0, false, INK),
        body: face("Cormorant Garamond", 28.0, false, SOFT),
    },
    Pairing {
        key: "pairing.invitation",
        heading: face("Allura", 76.0, false, 0x8A1C3B),
        body: face("Lato", 24.0, false, SOFT),
    },
    Pairing {
        key: "pairing.celebration",
        heading: face("Dancing Script", 68.0, false, 0xE0457B),
        body: face("Poppins", 24.0, false, SOFT),
    },
    Pairing {
        key: "pairing.signature",
        heading: face("Alex Brush", 76.0, false, INK),
        body: Face {
            font: "EB Garamond",
            size: 24.0,
            bold: false,
            color: SOFT,
            upper: true,
            effects: none,
        },
    },
    Pairing {
        key: "pairing.casual",
        heading: face("Satisfy", 68.0, false, 0x1E63E9),
        body: face("Questrial", 24.0, false, SOFT),
    },
    Pairing {
        key: "pairing.cafe",
        heading: face("Merienda", 58.0, false, 0x2B1D14),
        body: face("Open Sans", 24.0, false, SOFT),
    },
    Pairing {
        key: "pairing.menu",
        heading: face("Courgette", 60.0, false, INK),
        body: face("Lato", 24.0, false, SOFT),
    },
    // With effects (owner request, with examples of the kind): neon signs,
    // gold, retro, stickers, screens and labels, in families of the library.
    Pairing {
        key: "pairing.neon",
        heading: lit("Pacifico", 72.0, 0xFF_D6_F2, || {
            vec![with(K::Neon, 0xE0_3F_D8, |e| e.intensity = 80.0)]
        }),
        body: caps("Montserrat", 20.0, 0x6A_1B_9A),
    },
    Pairing {
        key: "pairing.gold",
        heading: lit("Shrikhand", 72.0, 0xFF_D2_4A, || {
            vec![with(K::Glow, 0xFF_B0_00, |e| e.intensity = 70.0)]
        }),
        body: caps("Lato", 20.0, 0x8A_6A_1C),
    },
    Pairing {
        key: "pairing.retro",
        heading: lit("Archivo Black", 70.0, 0xFF_6B_57, || {
            vec![with(K::Drop, 0x18_24_3B, |e| {
                (e.blur, e.opacity, e.distance) = (0.0, 100.0, 10.0);
            })]
        }),
        body: caps("DM Sans", 20.0, INK),
    },
    Pairing {
        key: "pairing.sticker",
        heading: lit("Chewy", 74.0, 0xFF_FF_FF, || {
            vec![
                with(K::Drop, 0x5B_3F_D1, |e| {
                    (e.blur, e.opacity, e.distance) = (0.0, 100.0, 12.0);
                }),
                with(K::Outline, 0xB3_8C_FF, |e| e.thickness = 45.0),
            ]
        }),
        body: caps("Poppins", 20.0, 0x5B_3F_D1),
    },
    Pairing {
        key: "pairing.screen",
        heading: lit("Bebas Neue", 84.0, INK, || {
            vec![with(K::Glitch, 0x3E_E0_D0, |e| {
                e.second = crate::colors::hex(0xFF_3B_7F);
                e.distance = 40.0;
            })]
        }),
        body: face("Inter", 22.0, false, SOFT),
    },
    Pairing {
        key: "pairing.hollow",
        heading: lit("Anton", 80.0, 0xC9_8A_2E, || {
            vec![with(K::Hollow, 0xC9_8A_2E, |e| e.thickness = 30.0)]
        }),
        body: caps("Lato", 20.0, 0x8A_5A_1C),
    },
    Pairing {
        key: "pairing.echo",
        heading: lit("Dancing Script", 72.0, 0xFF_9E_7A, || {
            vec![with(K::Echo, 0xF4_B8_9A, |e| e.distance = 18.0)]
        }),
        body: face("Poppins", 22.0, false, SOFT),
    },
    Pairing {
        key: "pairing.label",
        heading: lit("League Spartan", 64.0, 0xFF_FF_FF, || {
            vec![with(K::Background, 0x1E_63_E9, |e| {
                (e.roundness, e.spread) = (30.0, 55.0);
            })]
        }),
        body: face("DM Sans", 22.0, false, SOFT),
    },
    Pairing {
        key: "pairing.laser",
        heading: lit("Kaushan Script", 72.0, 0xD6_FF_5A, || {
            vec![with(K::Glow, 0x8F_E3_00, |e| e.intensity = 75.0)]
        }),
        body: caps("Montserrat", 20.0, 0x3A_6B_00),
    },
    Pairing {
        key: "pairing.cool",
        heading: lit("Satisfy", 76.0, 0xE8_F1_FF, || {
            vec![with(K::Neon, 0x3D_7B_FF, |e| e.intensity = 80.0)]
        }),
        body: caps("Questrial", 20.0, 0x1E_3A_8A),
    },
];

/// The style of `face` in a document.
pub(crate) fn style(face: Face) -> SpanStyle {
    SpanStyle {
        font: face.font.to_owned(),
        size: face.size,
        color: crate::colors::hex(face.color),
        bold: face.bold,
        italic: false,
        gradient: None,
    }
}

/// The words of `face`: in capitals when it asks.
pub(crate) fn words(face: Face, text: &str) -> String {
    if face.upper {
        text.to_uppercase()
    } else {
        text.to_owned()
    }
}

/// `pairing` as a group of its two texts, the body under the heading, both
/// centred on `centre` in px of page and each as wide as its widest line
/// (owner request), so they grow sideways as they are written.
pub(crate) fn group(
    page_w: f64,
    centre: (f64, f64),
    pairing: Pairing,
    (heading, body): (&str, &str),
) -> NodeSpec {
    let heading_h = pairing.heading.size * 1.3;
    let body_h = pairing.body.size * 1.5;
    let gap = pairing.body.size * 0.4;
    let total = heading_h + gap + body_h;
    let top = centre.1 - total / 2.0;
    let node = |face: Face, words_: &str| {
        let mut node = gafira_doc::TextNode::plain(&words(face, words_), &style(face));
        node.sizing = gafira_doc::TextSizing::AutoWidth;
        node.align = gafira_doc::TextAlign::Center;
        node.effects = (face.effects)();
        // As wide as its words; a text that cannot be measured keeps 70 %
        // of the page.
        let w = crate::canvas::natural_width(&node).unwrap_or(page_w * 0.7);
        (node, w)
    };
    let (first, first_w) = node(pairing.heading, heading);
    let (second, second_w) = node(pairing.body, body);
    let w = first_w.max(second_w);
    let place = |width: f64, at: f64, h: f64| Placement::new(centre.0 - width / 2.0, at, width, h);
    NodeSpec::new(NodeKind::Group(GroupNode {}), place(w, top, total)).with_children(vec![
        NodeSpec::new(NodeKind::Text(first), place(first_w, top, heading_h)),
        NodeSpec::new(
            NodeKind::Text(second),
            place(second_w, top + heading_h + gap, body_h),
        ),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    // Owner request: a pairing comes in as a group of a heading over a body
    // line, each in its family, size and colour, the poster in capitals.
    #[test]
    fn a_pairing_is_a_group_of_two_texts() {
        let spec = group(
            1000.0,
            (500.0, 500.0),
            PAIRINGS[4],
            ("Añade un título", "Una línea"),
        );
        assert!(matches!(spec.kind, NodeKind::Group(_)));
        assert_eq!(spec.children.len(), 2);
        let NodeKind::Text(heading) = &spec.children[0].kind else {
            panic!("a text")
        };
        let first = heading.content.spans().next().expect("a span");
        assert_eq!(first.style.font, "Bebas Neue");
        assert_eq!(first.text, "AÑADE UN TÍTULO");
        assert!(spec.children[1].placement.y > spec.children[0].placement.y);
        assert!((spec.placement.x + spec.placement.w / 2.0 - 500.0).abs() < 1e-9);
        // Owner request: each text as wide as its words, centred, growing
        // sideways; the group as wide as the widest.
        for child in &spec.children {
            let NodeKind::Text(text) = &child.kind else {
                panic!("a text")
            };
            assert_eq!(text.sizing, gafira_doc::TextSizing::AutoWidth);
            let natural = crate::canvas::natural_width(text).expect("measured");
            assert!((child.placement.w - natural).abs() < 1e-9);
            assert!((child.placement.x + child.placement.w / 2.0 - 500.0).abs() < 1e-9);
            assert!(child.placement.w < 700.0, "narrower than 70 % of the page");
        }
        let widest = spec
            .children
            .iter()
            .map(|c| c.placement.w)
            .fold(0.0, f64::max);
        assert!((spec.placement.w - widest).abs() < 1e-9);
        // Pairings with effects carry them, and they are valid.
        let lit: Vec<_> = PAIRINGS
            .iter()
            .filter(|p| !(p.heading.effects)().is_empty())
            .collect();
        assert!(lit.len() >= 10);
        for pairing in lit {
            let spec = group(1000.0, (500.0, 500.0), *pairing, ("Hola", "Una línea"));
            let NodeKind::Text(heading) = &spec.children[0].kind else {
                panic!("a text")
            };
            assert!(!heading.effects.is_empty(), "{}", pairing.key);
            assert!(
                heading.effects.iter().all(gafira_doc::TextEffect::is_valid),
                "{}",
                pairing.key
            );
        }
        // Every family is one of the library.
        let library = gafira_theme::library_families();
        for pairing in PAIRINGS {
            assert!(
                library.contains(&pairing.heading.font),
                "{}",
                pairing.heading.font
            );
            assert!(
                library.contains(&pairing.body.font),
                "{}",
                pairing.body.font
            );
        }
    }
}
