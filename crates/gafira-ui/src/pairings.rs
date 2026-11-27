//! «Combinaciones de fuentes» of the Texto tab (owner request): small
//! typographic compositions of two or three texts in families of the
//! library (scripts with flourishes, brush, blackletter, sketched,
//! outlined, wide display...) that go together, added as one group. Each
//! keeps on the page the alignment, proportions, spacing and effects its
//! card shows.

use gafira_doc::{
    GroupNode, NodeKind, Placement, SpanStyle, TextAlign, TextEffect, TextEffectKind as K,
};
use gafira_edit::NodeSpec;

/// One text of a composition.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Line {
    pub(crate) font: &'static str,
    /// Font size, in px of page.
    pub(crate) size: f64,
    pub(crate) bold: bool,
    pub(crate) italic: bool,
    /// Its colour, 0xRRGGBB.
    pub(crate) color: u32,
    /// Its words in capitals.
    pub(crate) upper: bool,
    /// Room between letters, in thousandths of an em (FD-41).
    pub(crate) spacing: f64,
    /// Line height as a multiple of the size; `None` is the font's own.
    pub(crate) leading: Option<f64>,
    /// Room above it, as a multiple of its size; negative tucks it under
    /// the text above, as a script over a heading does.
    pub(crate) gap: f64,
    /// How much it bends (FD-44), 0 straight.
    pub(crate) curve: f64,
    /// The effects of its letters.
    pub(crate) effects: fn() -> Vec<TextEffect>,
    /// On the row of the line before, against the right edge of the group
    /// and aligned to the right, as the price of a menu item.
    pub(crate) beside: bool,
}

fn none() -> Vec<TextEffect> {
    Vec::new()
}

/// A plain line of `font` at `size` in `color`, a third of its size under
/// the one above.
const fn line(font: &'static str, size: f64, color: u32) -> Line {
    Line {
        font,
        size,
        bold: false,
        italic: false,
        color,
        upper: false,
        spacing: 0.0,
        leading: None,
        gap: 0.35,
        curve: 0.0,
        effects: none,
        beside: false,
    }
}

impl Line {
    const fn bold(self) -> Self {
        Self { bold: true, ..self }
    }

    const fn italic(self) -> Self {
        Self {
            italic: true,
            ..self
        }
    }

    /// In capitals, `spacing` thousandths of an em apart.
    const fn caps(self, spacing: f64) -> Self {
        Self {
            upper: true,
            spacing,
            ..self
        }
    }

    const fn spaced(self, spacing: f64) -> Self {
        Self { spacing, ..self }
    }

    const fn leading(self, leading: f64) -> Self {
        Self {
            leading: Some(leading),
            ..self
        }
    }

    const fn gap(self, gap: f64) -> Self {
        Self { gap, ..self }
    }

    const fn curve(self, curve: f64) -> Self {
        Self { curve, ..self }
    }

    const fn beside(self) -> Self {
        Self {
            beside: true,
            ..self
        }
    }

    const fn fx(self, effects: fn() -> Vec<TextEffect>) -> Self {
        Self { effects, ..self }
    }
}

/// A composition: its name's catalogue key, the key of its words (one text
/// per line, separated by `|`, `\n` inside one breaks it), how its lines
/// are aligned and the lines themselves.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Pairing {
    pub(crate) key: &'static str,
    pub(crate) words: &'static str,
    pub(crate) align: TextAlign,
    pub(crate) lines: &'static [Line],
}

const INK: u32 = 0x18243B;
const SOFT: u32 = 0x46526B;
const BLACK: u32 = 0x111111;
const WINE: u32 = 0x8A1C3B;
const PLUM: u32 = 0x5B3FD1;
const COFFEE: u32 = 0x4A2C1A;
const NAVY: u32 = 0x1B2A4A;

/// The effect `kind` in `color` (0xRRGGBB), changed by `change`.
fn with(kind: K, color: u32, change: impl FnOnce(&mut TextEffect)) -> TextEffect {
    let mut effect = TextEffect::new(kind);
    effect.color = crate::colors::hex(color);
    change(&mut effect);
    effect
}

const L: TextAlign = TextAlign::Left;
const C: TextAlign = TextAlign::Center;
const R: TextAlign = TextAlign::Right;

const fn pairing(
    key: &'static str,
    words: &'static str,
    align: TextAlign,
    lines: &'static [Line],
) -> Pairing {
    Pairing {
        key,
        words,
        align,
        lines,
    }
}

/// The compositions, in the order of the panel.
pub(crate) const PAIRINGS: [Pairing; 43] = [
    // Script headlines with a caps line and a paragraph: copy ready to
    // rewrite (owner request, with examples).
    pairing(
        "pairing.services",
        "pairing.services_words",
        L,
        &[
            line("Kaushan Script", 58.0, BLACK).leading(1.25),
            line("Archivo Black", 24.0, BLACK).caps(220.0).gap(0.9),
            line("Montserrat", 20.0, SOFT)
                .spaced(20.0)
                .leading(1.45)
                .gap(0.7),
        ],
    ),
    pairing(
        "pairing.modern",
        "pairing.modern_words",
        L,
        &[
            line("Montserrat", 22.0, BLACK).caps(450.0),
            line("Black Ops One", 76.0, BLACK).leading(1.05).gap(0.2),
            line("Montserrat", 26.0, BLACK)
                .italic()
                .leading(1.35)
                .gap(0.7),
        ],
    ),
    pairing(
        "pairing.poster",
        "pairing.poster_words",
        C,
        &[
            line("Kaushan Script", 92.0, BLACK).caps(40.0).leading(1.05),
            line("Questrial", 36.0, BLACK).caps(220.0).gap(0.5),
            line("Questrial", 22.0, BLACK)
                .spaced(120.0)
                .leading(1.5)
                .gap(0.8),
        ],
    ),
    pairing(
        "pairing.welcome",
        "pairing.welcome_words",
        C,
        &[
            line("Montserrat", 20.0, BLACK).bold().caps(180.0),
            line("Great Vibes", 104.0, BLACK).leading(0.95).gap(0.1),
        ],
    ),
    pairing(
        "pairing.romantic",
        "pairing.romantic_words",
        C,
        &[
            line("Monsieur La Doulaise", 104.0, BLACK),
            line("Cormorant Garamond", 24.0, BLACK)
                .bold()
                .caps(500.0)
                .gap(0.1),
            line("Cormorant Garamond", 24.0, SOFT).italic().gap(0.3),
        ],
    ),
    pairing(
        "pairing.invitation",
        "pairing.invitation_words",
        C,
        &[
            line("Lato", 16.0, WINE).caps(450.0),
            line("Allura", 104.0, WINE).leading(0.95).gap(0.2),
            line("Lato", 16.0, WINE).bold().caps(350.0).gap(0.9),
        ],
    ),
    pairing(
        "pairing.energy",
        "pairing.energy_words",
        L,
        &[
            line("Kaushan Script", 58.0, BLACK),
            line("Permanent Marker", 96.0, BLACK).caps(0.0).gap(-0.25),
            line("DM Sans", 16.0, BLACK).bold().caps(320.0).gap(0.2),
        ],
    ),
    pairing(
        "pairing.celebration",
        "pairing.celebration_words",
        C,
        &[
            line("Pacifico", 76.0, BLACK).leading(1.05),
            line("Poppins", 22.0, BLACK).bold().gap(0.5),
        ],
    ),
    pairing(
        "pairing.tattoo",
        "pairing.tattoo_words",
        C,
        &[
            line("Rye", 84.0, BLACK).caps(40.0).curve(35.0),
            line("Mrs Saint Delafield", 64.0, BLACK).gap(-0.55),
        ],
    ),
    pairing(
        "pairing.elegant",
        "pairing.elegant_words",
        C,
        &[
            line("Montserrat", 16.0, SOFT).caps(450.0),
            line("Cormorant Garamond", 66.0, BLACK)
                .caps(160.0)
                .leading(1.0)
                .gap(0.5),
            line("Lato", 13.0, SOFT).caps(350.0).gap(1.0),
        ],
    ),
    pairing(
        "pairing.sale",
        "pairing.sale_words",
        C,
        &[
            line("Lilita One", 120.0, 0xE0_0B_26)
                .caps(20.0)
                .fx(|| vec![with(K::Outline, 0xFF_D1_00, |e| e.thickness = 40.0)]),
            line("Lilita One", 92.0, 0xFF_D1_00)
                .caps(20.0)
                .gap(0.0)
                .fx(|| vec![with(K::Outline, 0xFF_E8_7A, |e| e.thickness = 40.0)]),
        ],
    ),
    pairing(
        "pairing.growth",
        "pairing.growth_words",
        C,
        &[
            line("Montserrat", 30.0, NAVY).caps(300.0),
            line("Abril Fatface", 150.0, NAVY).spaced(-20.0).gap(-0.12),
        ],
    ),
    pairing(
        "pairing.boutique",
        "pairing.boutique_words",
        C,
        &[
            line("Yeseva One", 84.0, WINE).spaced(20.0),
            line("Pinyon Script", 64.0, 0xB0_7A_4F).gap(-0.45),
            line("Lato", 14.0, SOFT).bold().caps(500.0).gap(0.5),
        ],
    ),
    pairing(
        "pairing.marker",
        "pairing.marker_words",
        L,
        &[
            line("Permanent Marker", 80.0, BLACK).leading(1.1),
            line("DM Sans", 22.0, SOFT).leading(1.45).gap(0.6),
        ],
    ),
    pairing(
        "pairing.grunge",
        "pairing.grunge_words",
        L,
        &[
            line("Rubik Dirt", 100.0, BLACK).caps(20.0).leading(0.95),
            line("DM Sans", 18.0, 0xB0_1E_23)
                .bold()
                .caps(400.0)
                .gap(0.6),
        ],
    ),
    pairing(
        "pairing.diner",
        "pairing.diner_words",
        C,
        &[
            line("Lobster", 88.0, 0xD6_2E_2E).fx(|| {
                vec![with(K::Drop, 0xFF_C8_3D, |e| {
                    (e.blur, e.opacity, e.distance) = (0.0, 100.0, 6.0);
                })]
            }),
            line("Lilita One", 30.0, INK).caps(200.0).gap(0.3),
        ],
    ),
    pairing(
        "pairing.price_list",
        "pairing.price_list_words",
        L,
        &[
            line("League Spartan", 30.0, BLACK).bold().caps(300.0),
            line("League Spartan", 30.0, BLACK).bold().beside(),
            line("Questrial", 22.0, BLACK).leading(1.5).gap(0.3),
            line("League Spartan", 30.0, BLACK)
                .bold()
                .caps(300.0)
                .gap(1.2),
            line("League Spartan", 30.0, BLACK).bold().beside(),
            line("Questrial", 22.0, BLACK).leading(1.5).gap(0.3),
            line("League Spartan", 30.0, BLACK)
                .bold()
                .caps(300.0)
                .gap(1.2),
            line("League Spartan", 30.0, BLACK).bold().beside(),
            line("Questrial", 22.0, BLACK).leading(1.5).gap(0.3),
        ],
    ),
    pairing(
        "pairing.gold",
        "pairing.gold_words",
        C,
        &[
            line("DM Serif Display", 92.0, 0xD4_A0_1E)
                .caps(60.0)
                .leading(0.95)
                .fx(|| vec![with(K::Glow, 0xFF_C8_3D, |e| e.intensity = 35.0)]),
            line("Lato", 16.0, 0x8A_6A_1C).bold().caps(500.0).gap(0.8),
        ],
    ),
    pairing(
        "pairing.signature",
        "pairing.signature_words",
        R,
        &[
            line("Herr Von Muellerhoff", 104.0, INK),
            line("EB Garamond", 18.0, SOFT).caps(500.0).gap(-0.2),
        ],
    ),
    pairing(
        "pairing.classic",
        "pairing.classic_words",
        C,
        &[
            line("Lato", 16.0, SOFT).bold().caps(400.0),
            line("DM Serif Display", 80.0, INK).gap(0.3),
            line("EB Garamond", 24.0, SOFT).italic().gap(0.1),
        ],
    ),
    pairing(
        "pairing.editorial",
        "pairing.editorial_words",
        L,
        &[
            line("Inter", 16.0, PLUM).bold().caps(300.0),
            line("DM Serif Display", 84.0, BLACK)
                .italic()
                .leading(0.95)
                .gap(0.4),
            line("Source Serif 4", 22.0, SOFT).leading(1.45).gap(0.8),
        ],
    ),
    pairing(
        "pairing.gothic",
        "pairing.gothic_words",
        C,
        &[
            line("UnifrakturMaguntia", 96.0, BLACK).leading(1.0),
            line("Anton", 28.0, 0xB0_1E_23).caps(300.0).gap(0.4),
        ],
    ),
    pairing(
        "pairing.sketch",
        "pairing.sketch_words",
        L,
        &[
            line("Cabin Sketch", 88.0, INK).bold().leading(0.95),
            line("Questrial", 22.0, SOFT)
                .spaced(40.0)
                .leading(1.45)
                .gap(0.7),
        ],
    ),
    pairing(
        "pairing.summer",
        "pairing.summer_words",
        C,
        &[
            line("Londrina Outline", 82.0, 0x0E_8A_8A).caps(150.0),
            line("Damion", 92.0, 0xF2_6B_3A).gap(-0.55),
            line("Montserrat", 16.0, 0x0E_8A_8A)
                .bold()
                .caps(400.0)
                .gap(0.4),
        ],
    ),
    pairing(
        "pairing.western",
        "pairing.western_words",
        C,
        &[
            line("Chau Philomene One", 20.0, COFFEE).caps(400.0),
            line("Rye", 70.0, COFFEE).leading(1.05).gap(0.3),
            line("Pirata One", 30.0, 0xA0_4A_1E).spaced(60.0).gap(0.4),
        ],
    ),
    pairing(
        "pairing.pirate",
        "pairing.pirate_words",
        C,
        &[
            line("Pirata One", 104.0, BLACK).spaced(20.0),
            line("Meddon", 30.0, SOFT).gap(-0.1),
        ],
    ),
    pairing(
        "pairing.luxe",
        "pairing.luxe_words",
        C,
        &[
            line("Belleza", 56.0, BLACK).caps(500.0),
            line("Alex Brush", 70.0, 0xB0_7A_4F).gap(-0.55),
            line("Belleza", 14.0, SOFT).caps(500.0).gap(0.4),
        ],
    ),
    pairing(
        "pairing.tech",
        "pairing.tech_words",
        L,
        &[
            line("Michroma", 50.0, INK).caps(250.0).leading(1.15),
            line("Inter", 20.0, 0x1E_63_E9).bold().caps(300.0).gap(0.8),
            line("Inter", 20.0, SOFT).leading(1.45).gap(0.4),
        ],
    ),
    pairing(
        "pairing.cafe",
        "pairing.cafe_words",
        C,
        &[
            line("Bree Serif", 18.0, COFFEE).caps(400.0),
            line("Merienda", 66.0, COFFEE).leading(1.05).gap(0.2),
            line("Courgette", 24.0, 0xA0_6A_3C).gap(0.5),
        ],
    ),
    pairing(
        "pairing.menu",
        "pairing.menu_words",
        C,
        &[
            line("Cormorant Garamond", 22.0, INK).bold().caps(500.0),
            line("Courgette", 64.0, INK).gap(0.2),
            line("Lato", 20.0, SOFT).italic().leading(1.6).gap(0.6),
        ],
    ),
    pairing(
        "pairing.playful",
        "pairing.playful_words",
        C,
        &[
            line("Sniglet", 84.0, 0xFF_6B_57).bold().leading(1.0),
            line("Poppins", 20.0, INK).bold().caps(220.0).gap(0.6),
        ],
    ),
    pairing(
        "pairing.casual",
        "pairing.casual_words",
        L,
        &[
            line("Satisfy", 76.0, 0x1E_63_E9).leading(1.1),
            line("Questrial", 22.0, SOFT).leading(1.45).gap(0.5),
        ],
    ),
    pairing(
        "pairing.neon",
        "pairing.neon_words",
        C,
        &[
            line("Rubik Bubbles", 92.0, 0xFF_E3_F6)
                .leading(1.0)
                .fx(|| vec![with(K::Neon, 0xFF_3F_C8, |e| e.intensity = 80.0)]),
            line("Montserrat", 18.0, 0x6A_1B_9A)
                .bold()
                .caps(400.0)
                .gap(0.8),
        ],
    ),
    pairing(
        "pairing.retro",
        "pairing.retro_words",
        C,
        &[
            line("Archivo Black", 92.0, 0xFF_6B_57)
                .caps(20.0)
                .leading(0.95)
                .fx(|| {
                    vec![with(K::Drop, INK, |e| {
                        (e.blur, e.opacity, e.distance) = (0.0, 100.0, 10.0);
                    })]
                }),
            line("Rye", 24.0, INK).caps(250.0).gap(0.8),
        ],
    ),
    pairing(
        "pairing.sticker",
        "pairing.sticker_words",
        C,
        &[
            line("Chewy", 96.0, 0xFF_FF_FF).fx(|| {
                vec![
                    with(K::Drop, PLUM, |e| {
                        (e.blur, e.opacity, e.distance) = (0.0, 100.0, 12.0);
                    }),
                    with(K::Outline, 0xB3_8C_FF, |e| e.thickness = 45.0),
                ]
            }),
            line("Poppins", 20.0, PLUM).bold().caps(250.0).gap(0.6),
        ],
    ),
    pairing(
        "pairing.screen",
        "pairing.screen_words",
        L,
        &[
            line("Bebas Neue", 110.0, INK).leading(0.9).fx(|| {
                vec![with(K::Glitch, 0x3E_E0_D0, |e| {
                    e.second = crate::colors::hex(0xFF_3B_7F);
                    e.distance = 40.0;
                })]
            }),
            line("Michroma", 16.0, SOFT).caps(400.0).gap(0.6),
        ],
    ),
    pairing(
        "pairing.hollow",
        "pairing.hollow_words",
        L,
        &[
            line("Anton", 100.0, 0xC9_8A_2E)
                .caps(40.0)
                .fx(|| vec![with(K::Hollow, 0xC9_8A_2E, |e| e.thickness = 30.0)]),
            line("Anton", 100.0, 0xC9_8A_2E).caps(40.0).gap(-0.15),
            line("Lato", 16.0, 0x8A_5A_1C).bold().caps(450.0).gap(0.4),
        ],
    ),
    pairing(
        "pairing.echo",
        "pairing.echo_words",
        C,
        &[
            line("Dancing Script", 84.0, 0xF2_6B_4A)
                .fx(|| vec![with(K::Echo, 0xF4_B8_9A, |e| e.distance = 18.0)]),
            line("Poppins", 20.0, SOFT).caps(300.0).gap(0.6),
        ],
    ),
    pairing(
        "pairing.label",
        "pairing.label_words",
        L,
        &[
            line("League Spartan", 30.0, 0xFF_FF_FF)
                .bold()
                .caps(250.0)
                .fx(|| {
                    vec![with(K::Background, 0x1E_63_E9, |e| {
                        (e.roundness, e.spread) = (30.0, 55.0);
                    })]
                }),
            line("League Spartan", 96.0, INK)
                .bold()
                .leading(0.95)
                .gap(0.6),
            line("DM Sans", 20.0, SOFT).gap(0.5),
        ],
    ),
    pairing(
        "pairing.laser",
        "pairing.laser_words",
        C,
        &[
            line("Shrikhand", 84.0, 0xD6_FF_5A)
                .leading(1.0)
                .fx(|| vec![with(K::Glow, 0x8F_E3_00, |e| e.intensity = 75.0)]),
            line("Montserrat", 18.0, 0x3A_6B_00)
                .bold()
                .caps(400.0)
                .gap(0.8),
        ],
    ),
    pairing(
        "pairing.cool",
        "pairing.cool_words",
        C,
        &[
            line("Satisfy", 88.0, 0xE8_F1_FF)
                .fx(|| vec![with(K::Neon, 0x3D_7B_FF, |e| e.intensity = 80.0)]),
            line("Questrial", 18.0, 0x1E_3A_8A).caps(400.0).gap(0.6),
        ],
    ),
    pairing(
        "pairing.quote",
        "pairing.quote_words",
        C,
        &[
            line("DM Serif Display", 120.0, 0xC9_8A_2E).leading(0.8),
            line("Alice", 34.0, INK).leading(1.35).gap(-0.2),
            line("Damion", 34.0, 0xC9_8A_2E).gap(0.4),
        ],
    ),
    pairing(
        "pairing.vintage",
        "pairing.vintage_words",
        C,
        &[
            line("UnifrakturCook", 30.0, COFFEE),
            line("Shrikhand", 78.0, 0xB0_1E_23).leading(1.0).gap(0.2),
            line("Bree Serif", 18.0, COFFEE).caps(350.0).gap(0.6),
        ],
    ),
];

/// The style of `line` in a document.
pub(crate) fn style(line: &Line) -> SpanStyle {
    SpanStyle {
        font: line.font.to_owned(),
        size: line.size,
        color: crate::colors::hex(line.color),
        bold: line.bold,
        italic: line.italic,
        gradient: None,
    }
}

/// The words of `line`: in capitals when it asks.
pub(crate) fn words(line: &Line, text: &str) -> String {
    if line.upper {
        text.to_uppercase()
    } else {
        text.to_owned()
    }
}

/// The words of each line of a composition, from its catalogue text: one
/// per line, separated by `|`; a missing one is empty.
pub(crate) fn split(pairing: &Pairing, text: &str) -> Vec<String> {
    let mut parts = text.split('|').map(|s| s.trim().replace("\\n", "\n"));
    pairing
        .lines
        .iter()
        .map(|_| parts.next().unwrap_or_default())
        .collect()
}

/// The text node of `line` saying `text`, aligned as its composition.
fn text_node(line: &Line, align: TextAlign, text: &str) -> gafira_doc::TextNode {
    let mut node = gafira_doc::TextNode::plain(&words(line, text), &style(line));
    node.sizing = gafira_doc::TextSizing::AutoWidth;
    node.align = align;
    node.letter_spacing = line.spacing;
    node.line_height = line.leading;
    node.effects = (line.effects)();
    node
}

/// `pairing` as a group of its texts, stacked as its card shows them, the
/// whole centred on `centre` in px of page. Each text is as wide as its
/// widest line (it grows sideways as it is written), on the side its
/// composition is aligned to: left edges together, centres together or
/// right edges together (owner request: the canvas keeps the card's
/// alignment and proportions).
pub(crate) fn group(
    page_w: f64,
    centre: (f64, f64),
    pairing: &Pairing,
    texts: &[String],
) -> NodeSpec {
    let mut laid = Vec::new();
    let mut y = 0.0_f64;
    let mut widest = 0.0_f64;
    // The top of the row being laid and the width of its first text.
    let mut row = (0.0_f64, 0.0_f64);
    for (index, line) in pairing.lines.iter().enumerate() {
        let text = texts.get(index).map_or("", String::as_str);
        let align = if line.beside {
            TextAlign::Right
        } else {
            pairing.align
        };
        let mut node = text_node(line, align, text);
        let paragraphs = text.split('\n').count().max(1) as f64;
        let w = crate::canvas::natural_width(&node).unwrap_or(page_w * 0.7);
        let room = line.size * line.leading.unwrap_or(1.3).max(1.3) * paragraphs * 2.0;
        let lines_h = crate::canvas::curved_height(&node, w, room)
            .filter(|h| h.is_finite() && *h > 0.0)
            .unwrap_or(line.size * 1.3 * paragraphs);
        let (w, h) = if line.curve == 0.0 {
            (w, lines_h)
        } else {
            // A bent line, as «Texto curvo» makes it: a box of its own
            // width, as high as its arc.
            node.curve = line.curve;
            node.sizing = gafira_doc::TextSizing::Fixed;
            let w = (w * 1.08).ceil();
            let h = crate::canvas::curved_height(&node, w, lines_h).unwrap_or(lines_h);
            (w, h)
        };
        if line.beside && index > 0 {
            // Beside the text before, with a gutter of twice its size.
            let at = row.0;
            widest = widest.max(row.1 + line.size * 2.0 + w);
            y = y.max(at + h);
            laid.push((node, at, w, h, true));
            continue;
        }
        if index > 0 {
            y += line.gap * line.size;
        }
        // A tucked line never climbs above the top of the group.
        y = y.max(0.0);
        row = (y, w);
        laid.push((node, y, w, h, false));
        y += h;
        widest = widest.max(w);
    }
    let total = laid
        .iter()
        .map(|(_, top, _, h, _)| top + h)
        .fold(0.0, f64::max);
    let left = centre.0 - widest / 2.0;
    let top = centre.1 - total / 2.0;
    let x_of = |w: f64| match pairing.align {
        TextAlign::Center => centre.0 - w / 2.0,
        TextAlign::Right => left + widest - w,
        TextAlign::Left | TextAlign::Justify => left,
    };
    let children = laid
        .into_iter()
        .map(|(node, at, w, h, beside)| {
            let x = if beside { left + widest - w } else { x_of(w) };
            NodeSpec::new(NodeKind::Text(node), Placement::new(x, top + at, w, h))
        })
        .collect();
    NodeSpec::new(
        NodeKind::Group(GroupNode {}),
        Placement::new(left, top, widest, total),
    )
    .with_children(children)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(pairing: &Pairing) -> Vec<String> {
        pairing
            .lines
            .iter()
            .enumerate()
            .map(|(i, _)| format!("Línea número {i}"))
            .collect()
    }

    fn texts(spec: &NodeSpec) -> Vec<&gafira_doc::TextNode> {
        spec.children
            .iter()
            .map(|c| match &c.kind {
                NodeKind::Text(t) => t,
                _ => panic!("a text"),
            })
            .collect()
    }

    // Owner request: a composition comes in as a group of two or three
    // texts, each in its family, size, colour and spacing, in capitals
    // when it asks, one under the other.
    #[test]
    fn a_pairing_is_a_group_of_its_lines() {
        let poster = PAIRINGS
            .iter()
            .find(|p| p.key == "pairing.poster")
            .expect("poster");
        let spec = group(
            1000.0,
            (500.0, 500.0),
            poster,
            &["Vacaciones".into(), "Cena".into(), "Calle 22".into()],
        );
        assert!(matches!(spec.kind, NodeKind::Group(_)));
        assert_eq!(spec.children.len(), 3);
        let lines = texts(&spec);
        let first = lines[0].content.spans().next().expect("a span");
        assert_eq!(first.style.font, "Kaushan Script");
        assert_eq!(first.text, "VACACIONES");
        assert!(lines[1].letter_spacing > 0.0);
        let ys: Vec<f64> = spec.children.iter().map(|c| c.placement.y).collect();
        assert!(ys.windows(2).all(|w| w[1] > w[0]), "{ys:?}");
        assert!((spec.placement.x + spec.placement.w / 2.0 - 500.0).abs() < 1e-9);
        assert!((spec.placement.y + spec.placement.h / 2.0 - 500.0).abs() < 1e-9);
    }

    // Owner request: what the card shows is what the page gets: a
    // composition aligned to the left keeps its lines' left edges together
    // and left-aligned, one to the right their right edges, a centred one
    // their centres; each text as wide as its words.
    #[test]
    fn a_pairing_keeps_its_alignment() {
        for pairing in &PAIRINGS {
            let spec = group(1000.0, (500.0, 500.0), pairing, &sample(pairing));
            let g = spec.placement;
            for (child, text) in spec.children.iter().zip(texts(&spec)) {
                let p = child.placement;
                let align = if text.align == TextAlign::Right && pairing.align != R {
                    R
                } else {
                    pairing.align
                };
                assert_eq!(text.align, align, "{}", pairing.key);
                let edge = match align {
                    TextAlign::Left | TextAlign::Justify => p.x - g.x,
                    TextAlign::Center => p.x + p.w / 2.0 - (g.x + g.w / 2.0),
                    TextAlign::Right => p.x + p.w - (g.x + g.w),
                };
                assert!(edge.abs() < 1e-6, "{}: {edge}", pairing.key);
                assert!(p.x >= g.x - 1e-6 && p.x + p.w <= g.x + g.w + 1e-6);
                assert!(p.y >= g.y - 1e-6 && p.y + p.h <= g.y + g.h + 1e-6);
                if text.curve == 0.0 {
                    let natural = crate::canvas::natural_width(text).expect(pairing.key);
                    assert!((p.w - natural).abs() < 1e-9, "{}", pairing.key);
                }
            }
        }
    }

    // Owner request: two or three texts each, in families of the library,
    // diverse (scripts, brush, blackletter, sketched, outlined...), with
    // valid effects and words in both languages' catalogues.
    #[test]
    fn the_pairings_are_varied_and_valid() {
        let library = gafira_theme::library_families();
        let mut families = std::collections::BTreeSet::new();
        let mut keys = std::collections::BTreeSet::new();
        for pairing in &PAIRINGS {
            assert!(keys.insert(pairing.key), "{} twice", pairing.key);
            assert!((2..=9).contains(&pairing.lines.len()), "{}", pairing.key);
            for line in pairing.lines {
                assert!(library.contains(&line.font), "{}", line.font);
                families.insert(line.font);
                assert!(gafira_doc::TextNode::LETTER_SPACING.contains(&line.spacing));
                assert!((line.effects)().iter().all(TextEffect::is_valid));
                assert!((line.effects)().len() <= TextEffect::MAX);
            }
        }
        assert!(families.len() >= 35, "{} families", families.len());
        for align in [L, C, R] {
            assert!(PAIRINGS.iter().any(|p| p.align == align), "{align:?}");
        }
        assert!(PAIRINGS.iter().filter(|p| p.lines.len() == 3).count() >= 15);
        assert!(
            PAIRINGS
                .iter()
                .filter(|p| p.lines.iter().any(|l| !(l.effects)().is_empty()))
                .count()
                >= 10
        );
    }

    // The words of a composition come one per line; a missing one is
    // empty and `\n` breaks a line.
    // Owner request (a menu): a text marked beside sits on the row of the
    // one before, its right edge on the group's, clear of the text at left.
    #[test]
    fn a_price_sits_beside_its_item() {
        let menu = PAIRINGS
            .iter()
            .find(|p| p.key == "pairing.price_list")
            .expect("menu");
        let spec = group(1000.0, (500.0, 500.0), menu, &sample(menu));
        let g = spec.placement;
        let (item, price) = (spec.children[0].placement, spec.children[1].placement);
        assert!((price.y - item.y).abs() < 1e-9);
        assert!((price.x + price.w - (g.x + g.w)).abs() < 1e-9);
        assert!(price.x > item.x + item.w);
        assert!(spec.children[2].placement.y >= item.y + item.h);
    }

    #[test]
    fn the_words_split_per_line() {
        let three = &PAIRINGS[0];
        assert_eq!(three.lines.len(), 3);
        assert_eq!(
            split(three, "Uno\\ndos | Tres"),
            vec!["Uno\ndos".to_owned(), "Tres".to_owned(), String::new()]
        );
    }
}
