//! Spec 106: a text with parts in other faces and sizes — each part is
//! shaped in its own face and size, on the same lines.

use gafira_geom::Size;
use gafira_text::{FaceRun, TextStyle, layout_box_face, layout_box_runs};

const REGULAR: &[u8] =
    include_bytes!("../../../assets/fonts/library/opensans/OpenSans-Regular.ttf");
const BOLD: &[u8] = include_bytes!("../../../assets/fonts/library/opensans/OpenSans-Bold.ttf");

fn advance_of(layout: &gafira_text::TextLayout, range: std::ops::Range<usize>) -> f32 {
    layout
        .lines
        .iter()
        .flat_map(|l| &l.clusters)
        .filter(|c| range.contains(&c.range.start))
        .map(|c| c.advance)
        .sum()
}

// S106-05: «Agregar algo de texto» with «algo» bold and larger: that word
// is wider than in the plain layout, the rest is as it was, and its
// clusters carry its size.
#[test]
fn s106_05_parts_in_their_own_face_and_size() {
    let stack = [REGULAR, BOLD];
    let style = TextStyle {
        size_pt: 24.0,
        ..TextStyle::default()
    };
    let bounds = Size::new(2000.0, 400.0);
    let text = "Agregar algo de texto";
    let plain = layout_box_face(text, &stack, &style, (false, false), 72.0, bounds).expect("plain");
    let runs = [FaceRun {
        range: 8..12,
        size_px: 36.0,
        lead: 0,
        bold: true,
        italic: false,
    }];
    let styled = layout_box_runs(text, &stack, &style, 72.0, bounds, &runs).expect("styled");
    assert!(advance_of(&styled, 8..12) > advance_of(&plain, 8..12) * 1.4);
    assert!((advance_of(&styled, 0..7) - advance_of(&plain, 0..7)).abs() < 0.01);
    let sizes: Vec<f32> = styled
        .lines
        .iter()
        .flat_map(|l| &l.clusters)
        .filter(|c| (8..12).contains(&c.range.start))
        .map(|c| c.size_px)
        .collect();
    assert!(sizes.iter().all(|s| (*s - 36.0).abs() < 1e-3), "{sizes:?}");
    // No runs: the plain layout.
    let none = layout_box_runs(text, &stack, &style, 72.0, bounds, &[]).expect("none");
    assert_eq!(none, plain);
}
