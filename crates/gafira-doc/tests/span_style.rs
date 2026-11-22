//! Bold and italic spans (spec 062, D-100).

use gafira_color::{Color, Rgba};
use gafira_doc::{RichText, SpanStyle};

fn style(bold: bool, italic: bool) -> SpanStyle {
    SpanStyle {
        font: "Open Sans".to_owned(),
        size: 24.0,
        color: Color::Srgb(Rgba::new(0.0, 0.0, 0.0, 1.0)),
        bold,
        italic,
    }
}

// S62-01: a bold, italic span round-trips; a plain one writes no `bold`
// or `italic`, so its bytes are the ones it had before spec 062; a span
// without them loads plain.
#[test]
fn s62_01_bold_italic_round_trip() {
    let rich = RichText::plain("Hola", &style(true, true));
    let json = serde_json::to_string(&rich).expect("json");
    assert!(
        json.contains(r#""bold":true"#) && json.contains(r#""italic":true"#),
        "{json}"
    );
    assert_eq!(
        serde_json::from_str::<RichText>(&json).expect("loads"),
        rich
    );
    let plain =
        serde_json::to_string(&RichText::plain("Hola", &style(false, false))).expect("json");
    assert!(
        !plain.contains("bold") && !plain.contains("italic"),
        "{plain}"
    );
    let old = plain.clone();
    assert_eq!(
        serde_json::from_str::<RichText>(&old).expect("loads"),
        RichText::plain("Hola", &style(false, false))
    );
}
