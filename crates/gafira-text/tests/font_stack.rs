//! Fonts kept ready lay text out as ones registered anew (spec 081).

use gafira_geom::Size;
use gafira_testkit::fonts::INTER;
use gafira_text::{FontStack, TextError, TextStyle, layout_box_face};

// S81-03: the same stack, text after text, in any face and width, gives what
// a call of its own gives.
#[test]
fn s81_03_a_kept_stack_lays_out_the_same() {
    let fonts: [&[u8]; 1] = [INTER.bytes];
    let mut stack = FontStack::new(&fonts).expect("fonts");
    let style = TextStyle::default();
    for round in 0..2 {
        for words in ["Hola", "", "una frase larga que se parte\nen dos", "año €"] {
            for width in [30.0, 400.0] {
                for face in [(false, false), (true, false), (false, true)] {
                    let bounds = Size::new(width, 50.0);
                    let kept = stack.layout_box_face(words, &style, face, 96.0, bounds);
                    let anew = layout_box_face(words, &fonts, &style, face, 96.0, bounds);
                    assert_eq!(kept, anew, "{round} {words:?} {width} {face:?}");
                }
            }
        }
    }
    let wrong = stack.layout_box_face("a", &style, (false, false), f64::NAN, Size::new(1.0, 1.0));
    assert!(wrong.is_err(), "the numbers are checked too");
    assert!(matches!(FontStack::new(&[]), Err(TextError::NoFont)));
}
