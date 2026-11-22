//! «Combinaciones de fuentes» of the Texto tab (owner request): a heading
//! and a line of body text in two fonts of the library that go together,
//! added as one group.

use gafira_doc::{GroupNode, NodeKind, Placement, SpanStyle};
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

const fn face(font: &'static str, size: f64, bold: bool, color: u32) -> Face {
    Face {
        font,
        size,
        bold,
        color,
        upper: false,
    }
}

/// The pairings, in the order of the panel.
pub(crate) const PAIRINGS: [Pairing; 14] = [
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
];

/// The style of `face` in a document.
pub(crate) fn style(face: Face) -> SpanStyle {
    SpanStyle {
        font: face.font.to_owned(),
        size: face.size,
        color: crate::colors::hex(face.color),
        bold: face.bold,
        italic: false,
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

/// `pairing` as a group of its two texts, 70 % of the page across, the body
/// under the heading, centred on `centre` in px of page.
pub(crate) fn group(
    page_w: f64,
    centre: (f64, f64),
    pairing: Pairing,
    (heading, body): (&str, &str),
) -> NodeSpec {
    let w = page_w * 0.7;
    let heading_h = pairing.heading.size * 1.3;
    let body_h = pairing.body.size * 1.5;
    let gap = pairing.body.size * 0.4;
    let total = heading_h + gap + body_h;
    let (x, y) = (centre.0 - w / 2.0, centre.1 - total / 2.0);
    let text = |face: Face, words_: &str, at: f64, h: f64| {
        NodeSpec::new(
            NodeKind::Text(gafira_doc::TextNode::plain(
                &words(face, words_),
                &style(face),
            )),
            Placement::new(x, at, w, h),
        )
    };
    NodeSpec::new(
        NodeKind::Group(GroupNode {}),
        Placement::new(x, y, w, total),
    )
    .with_children(vec![
        text(pairing.heading, heading, y, heading_h),
        text(pairing.body, body, y + heading_h + gap, body_h),
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
