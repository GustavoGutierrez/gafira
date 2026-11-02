//! ARQ-110 (non-finite input), FD-41 (d) (ranges and their bounds) for `layout_box`.

use gafira_geom::Size;
use gafira_testkit::fonts;
use gafira_text::{TextError, TextLayout, TextStyle, layout_box};

/// One call input; each test changes the field it studies.
#[derive(Clone, Copy)]
struct Input {
    style: TextStyle,
    dpi: f64,
    width: f64,
    height: f64,
}

fn base() -> Input {
    Input {
        style: TextStyle {
            size_pt: 16.0,
            letter_spacing: 0.0,
            line_height: 1.0,
        },
        dpi: 96.0,
        width: 300.0,
        height: 200.0,
    }
}

fn run(i: Input) -> Result<TextLayout, TextError> {
    layout_box(
        "Hi",
        &[fonts::INTER.bytes],
        &i.style,
        i.dpi,
        Size::new(i.width, i.height),
    )
}

type Setter = fn(&mut Input, f64);

/// Every numeric input, with the field name the error must carry.
fn numeric_fields() -> Vec<(&'static str, Setter)> {
    vec![
        ("style.size_pt", |i, v| i.style.size_pt = v),
        ("style.letter_spacing", |i, v| i.style.letter_spacing = v),
        ("style.line_height", |i, v| i.style.line_height = v),
        ("dpi", |i, v| i.dpi = v),
        ("bounds.width", |i, v| i.width = v),
        ("bounds.height", |i, v| i.height = v),
    ]
}

#[test]
fn arq_110_nan_and_infinities_in_any_number_give_err_naming_the_field() {
    for (field, set) in numeric_fields() {
        for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            let mut i = base();
            set(&mut i, bad);
            assert_eq!(
                run(i).err(),
                Some(TextError::NonFinite { field }),
                "{field} = {bad}"
            );
        }
    }
}

#[test]
fn arq_110_a_computed_size_that_is_not_finite_gives_err() {
    // 1000 pt at 1e308 dpi overflows f64; 1 pt at 1e300 dpi is finite in f64 and not in f32.
    for (size_pt, dpi) in [(1000.0, 1e308), (1.0, 1e300)] {
        let mut i = base();
        i.style.size_pt = size_pt;
        i.dpi = dpi;
        assert_eq!(
            run(i).err(),
            Some(TextError::NonFinite { field: "size_px" }),
            "{size_pt} pt at {dpi} dpi"
        );
    }
}

type Range = (&'static str, Setter, f64, f64, f64, f64);

/// Field, setter, lowest and highest allowed value, and the nearest values outside.
fn ranges() -> Vec<Range> {
    vec![
        (
            "style.size_pt",
            |i, v| i.style.size_pt = v,
            1.0,
            1000.0,
            0.99,
            1000.01,
        ),
        (
            "style.letter_spacing",
            |i, v| i.style.letter_spacing = v,
            -100.0,
            500.0,
            -100.01,
            500.01,
        ),
        (
            "style.line_height",
            |i, v| i.style.line_height = v,
            0.5,
            4.0,
            0.49,
            4.01,
        ),
    ]
}

#[test]
fn fd_41_d_values_at_their_bounds_are_accepted() {
    for (field, set, lo, hi, _, _) in ranges() {
        for ok in [lo, hi] {
            let mut i = base();
            set(&mut i, ok);
            assert!(run(i).is_ok(), "{field} = {ok} must be accepted");
        }
    }
}

#[test]
fn fd_41_d_values_just_outside_their_bounds_give_out_of_range() {
    for (field, set, _, _, below, above) in ranges() {
        for bad in [below, above] {
            let mut i = base();
            set(&mut i, bad);
            assert_eq!(
                run(i).err(),
                Some(TextError::OutOfRange { field }),
                "{field} = {bad}"
            );
        }
    }
}

#[test]
fn arq_110_numbers_without_a_range_row_never_panic() {
    // A finite value outside every row is not rechecked and its result is not
    // specified (02 section 4.17, Rangos): this only requires no panic.
    for dpi in [0.0, -96.0, 5e-324, f64::MIN_POSITIVE, 1e6] {
        let mut i = base();
        i.dpi = dpi;
        let _ = run(i);
    }
    for (w, h) in [
        (-1.0, -1.0),
        (0.0, 0.0),
        (f64::MAX, f64::MAX),
        (-0.0, 5e-324),
    ] {
        let mut i = base();
        i.width = w;
        i.height = h;
        let _ = run(i);
    }
}
