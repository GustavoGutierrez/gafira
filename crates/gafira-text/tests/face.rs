//! The face of a family a layout asks for (spec 062): bold and italic.

use gafira_geom::Size;
use gafira_text::{TextStyle, layout_box, layout_box_face};

const REGULAR: &[u8] =
    include_bytes!("../../../assets/fonts/library/opensans/OpenSans-Regular.ttf");
const BOLD: &[u8] = include_bytes!("../../../assets/fonts/library/opensans/OpenSans-Bold.ttf");

fn width(layout: &gafira_text::TextLayout) -> f32 {
    layout
        .lines
        .iter()
        .map(|line| line.advance)
        .fold(0.0, f32::max)
}

// S62-03: with the regular and the bold face of a family in the stack, a
// bold layout uses the bold face (wider), a plain one the regular face, and
// `layout_box` stays the plain one.
#[test]
fn s62_03_a_bold_layout_uses_the_bold_face() {
    let stack = [REGULAR, BOLD];
    let style = TextStyle {
        size_pt: 24.0,
        ..TextStyle::default()
    };
    let bounds = Size::new(1000.0, 200.0);
    let plain = layout_box_face("Hola mundo", &stack, &style, (false, false), 72.0, bounds)
        .expect("lays out");
    let heavy = layout_box_face("Hola mundo", &stack, &style, (true, false), 72.0, bounds)
        .expect("lays out");
    assert!(
        width(&heavy) > width(&plain) + 1.0,
        "{} vs {}",
        width(&heavy),
        width(&plain)
    );
    let old = layout_box("Hola mundo", &stack, &style, 72.0, bounds).expect("lays out");
    assert_eq!(width(&old), width(&plain));
}
