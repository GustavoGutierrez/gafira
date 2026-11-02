//! ARQ-111 and ARQ-110 for the styles of FD-41: the same input gives the same bytes, and
//! the extremes of every style number give finite output.

use gafira_geom::Size;
use gafira_testkit::fonts;
use gafira_text::{ListItem, ListKind, TextAlign, TextStyle, layout_box, layout_list};

const STACK: [&[u8]; 3] = [
    fonts::INTER.bytes,
    fonts::NOTO_SANS_ARABIC.bytes,
    fonts::NOTO_EMOJI.bytes,
];
const TEXT: &str =
    "Stra\u{df}e fi fl \u{627}\u{628}\u{62c} \u{1f600}\nsecond line, wrapped and wide";

fn styles() -> Vec<TextStyle> {
    let mut all = Vec::new();
    for (spacing, line_height) in [(0.0, 1.0), (-100.0, 0.5), (500.0, 4.0), (37.5, 1.25)] {
        for align in [
            TextAlign::Left,
            TextAlign::Center,
            TextAlign::Right,
            TextAlign::Justify,
        ] {
            all.push(TextStyle {
                size_pt: 18.0,
                letter_spacing: spacing,
                line_height,
                uppercase: spacing > 0.0,
                align,
                underline: true,
                strikethrough: line_height > 1.0,
            });
        }
    }
    all
}

#[test]
fn arq_111_styled_layouts_give_the_same_bytes_twice() {
    for style in styles() {
        let run = || {
            format!(
                "{:?}",
                layout_box(TEXT, &STACK, &style, 96.0, Size::new(220.0, 400.0))
            )
        };
        let first = run();
        assert!(first.starts_with("Ok("), "{style:?}: {first}");
        assert_eq!(first, run(), "{style:?}");
    }
}

#[test]
fn arq_111_a_list_gives_the_same_bytes_twice() {
    let items = [
        ListItem {
            text: "one",
            level: 1,
            kind: ListKind::Numbered,
        },
        ListItem {
            text: "two \u{627}\u{628}",
            level: 2,
            kind: ListKind::Bullet,
        },
    ];
    let run = || {
        format!(
            "{:?}",
            layout_list(&items, &STACK, &styles()[5], 96.0, Size::new(300.0, 400.0))
        )
    };
    assert!(run().starts_with("Ok("));
    assert_eq!(run(), run());
}

#[test]
fn arq_110_the_extremes_of_every_style_number_give_finite_output() {
    for size_pt in [1.0, 1000.0] {
        for style in styles() {
            let style = TextStyle { size_pt, ..style };
            for width in [0.0, 1.0, 5000.0] {
                let out = format!(
                    "{:?}",
                    layout_box(TEXT, &STACK, &style, 96.0, Size::new(width, 10.0))
                );
                assert!(
                    out.starts_with("Ok("),
                    "{style:?} {width}: {}",
                    &out[..60.min(out.len())]
                );
                assert!(
                    !out.contains("NaN") && !out.contains("inf"),
                    "{style:?} {width}"
                );
            }
        }
    }
}
