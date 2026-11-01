//! FD-05 (b, d, e, f), FD-09 (d), ARQ-110, ARQ-111 for the rectangle primitives.

use gafira_doc::Margins;
use gafira_layout::{
    LayoutError, Length, Overflow, Rect, Unit, bleed_rect, margin_rect, overflow, safe_rect,
};
use proptest::prelude::*;

const BAD: [f64; 3] = [f64::NAN, f64::INFINITY, f64::NEG_INFINITY];

fn px(v: f64) -> Length {
    Length::new(v, Unit::Px)
}

fn margins(top: f64, right: f64, bottom: f64, left: f64) -> Margins {
    Margins {
        top: px(top),
        right: px(right),
        bottom: px(bottom),
        left: px(left),
        linked: false,
    }
}

fn square(side: f64) -> Rect {
    Rect::new(0.0, 0.0, side, side)
}

fn near(a: Rect, b: Rect) {
    for (x, y) in [(a.x0, b.x0), (a.y0, b.y0), (a.x1, b.x1), (a.y1, b.y1)] {
        assert!((x - y).abs() <= 1e-9, "{a:?} vs {b:?}");
    }
}

#[test]
fn fd_05_b_overflow_at_the_safe_edge_boundary() {
    let safe = safe_rect(square(1080.0), 5.0).unwrap();
    near(safe, Rect::new(54.0, 54.0, 1026.0, 1026.0));
    let at = |x: f64| overflow(Rect::new(x, 100.0, x + 100.0, 200.0), safe).unwrap();
    assert!(!at(926.0).exceeds());
    let o = at(926.01);
    assert!(o.exceeds());
    assert!((o.right - 0.01).abs() <= 1e-9);
    assert_eq!((o.left, o.top, o.bottom), (0.0, 0.0, 0.0));
}

#[test]
fn fd_05_d_bleed_expands_all_four_sides_from_the_dpi_input() {
    let page = Rect::new(10.0, 20.0, 110.0, 220.0);
    let b = bleed_rect(page, Length::new(3.0, Unit::Mm), 300.0).unwrap();
    let side: f64 = 3.0 / 25.4 * 300.0;
    assert!((side - 35.43).abs() <= 0.01);
    near(
        b,
        Rect::new(10.0 - side, 20.0 - side, 110.0 + side, 220.0 + side),
    );
    let b96 = bleed_rect(page, Length::new(3.0, Unit::Mm), 96.0).unwrap();
    assert!((b96.x0 - (10.0 - 3.0 / 25.4 * 96.0)).abs() <= 1e-9);
    assert_eq!(
        bleed_rect(page, Length::new(0.0, Unit::Mm), 300.0),
        Ok(page)
    );
}

#[test]
fn fd_05_d_bleed_range_is_zero_to_ten_mm_inclusive() {
    let page = square(100.0);
    let bleed = |v: f64, u: Unit| bleed_rect(page, Length::new(v, u), 300.0);
    assert!(bleed(10.0, Unit::Mm).is_ok());
    assert_eq!(
        bleed(10.01, Unit::Mm),
        Err(LayoutError::OutOfRange { field: "bleed" })
    );
    assert_eq!(
        bleed(-0.01, Unit::Mm),
        Err(LayoutError::OutOfRange { field: "bleed" })
    );
    assert!(bleed(1.0, Unit::Cm).is_ok());
    assert!(bleed(1.01, Unit::Cm).is_err());
    assert_eq!(
        bleed_rect(page, px(1.0), 0.0),
        Err(LayoutError::OutOfRange { field: "dpi" })
    );
}

#[test]
fn fd_05_e_safe_rect_insets_each_axis_by_its_own_size() {
    near(
        safe_rect(Rect::new(0.0, 0.0, 1080.0, 1920.0), 5.0).unwrap(),
        Rect::new(54.0, 96.0, 1026.0, 1824.0),
    );
    near(
        safe_rect(Rect::new(100.0, 50.0, 300.0, 250.0), 10.0).unwrap(),
        Rect::new(120.0, 70.0, 280.0, 230.0),
    );
    assert_eq!(safe_rect(square(10.0), 0.0), Ok(square(10.0)));
    assert!(safe_rect(square(10.0), 49.99).is_ok());
    for p in [50.0, -0.01] {
        assert_eq!(
            safe_rect(square(10.0), p),
            Err(LayoutError::OutOfRange { field: "percent" })
        );
    }
}

#[test]
fn fd_05_f_margins_that_reach_the_size_leave_no_area() {
    let page = Rect::new(0.0, 0.0, 100.0, 80.0);
    near(
        margin_rect(page, &margins(1.0, 2.0, 3.0, 4.0), 96.0).unwrap(),
        Rect::new(4.0, 1.0, 98.0, 77.0),
    );
    let cases = [
        (margins(0.0, 50.0, 0.0, 50.0), true),
        (margins(0.0, 50.0, 0.0, 49.99), false),
        (margins(40.0, 0.0, 40.0, 0.0), true),
        (margins(40.0, 0.0, 39.99, 0.0), false),
    ];
    for (m, no_area) in cases {
        assert_eq!(
            margin_rect(page, &m, 96.0) == Err(LayoutError::NoArea),
            no_area,
            "{m:?}"
        );
    }
    assert_eq!(
        margin_rect(page, &margins(-1.0, 0.0, 0.0, 0.0), 96.0),
        Err(LayoutError::OutOfRange {
            field: "margins.top"
        })
    );
    // Margins in another unit use the dpi input: 1 in = 96 px.
    let m = Margins {
        left: Length::new(1.0, Unit::In),
        ..margins(0.0, 0.0, 0.0, 0.0)
    };
    assert_eq!(margin_rect(page, &m, 96.0).unwrap().x0, 96.0);
}

#[test]
fn fd_05_f_fd_09_d_overflow_edges_and_outside_rule() {
    let rect = Rect::new(0.0, 0.0, 1080.0, 1080.0);
    // FD-09(d): x = 1000, width 104 on a 1080 page.
    let o = overflow(Rect::new(1000.0, 10.0, 1104.0, 50.0), rect).unwrap();
    assert_eq!(
        o,
        Overflow {
            left: 0.0,
            top: 0.0,
            right: 24.0,
            bottom: 0.0,
            fully_outside: false
        }
    );
    // x = 1080 is fully outside; so is a box touching only by an edge, on any side.
    let outside = [
        Rect::new(1080.0, 10.0, 1180.0, 50.0),
        Rect::new(-100.0, 10.0, 0.0, 50.0),
        Rect::new(10.0, 1080.0, 50.0, 1180.0),
        Rect::new(10.0, -100.0, 50.0, 0.0),
    ];
    for b in outside {
        assert!(overflow(b, rect).unwrap().fully_outside, "{b:?}");
    }
    // A box sharing area, or inside, is not outside; a zero-size box inside has no excess.
    assert!(
        !overflow(Rect::new(1079.0, 10.0, 1180.0, 50.0), rect)
            .unwrap()
            .fully_outside
    );
    let dot = overflow(Rect::new(5.0, 5.0, 5.0, 5.0), rect).unwrap();
    assert_eq!(
        dot,
        Overflow {
            left: 0.0,
            top: 0.0,
            right: 0.0,
            bottom: 0.0,
            fully_outside: false
        }
    );
    // Excess on every side.
    let all = overflow(Rect::new(-1.0, -2.0, 1083.0, 1084.0), rect).unwrap();
    assert_eq!(
        (all.left, all.top, all.right, all.bottom),
        (1.0, 2.0, 3.0, 4.0)
    );
}

#[test]
fn arq_110_inverted_boxes_are_errors() {
    let inv = Rect::new(10.0, 0.0, 0.0, 10.0);
    assert!(overflow(inv, square(10.0)).is_err());
    assert!(overflow(square(10.0), inv).is_err());
    assert!(safe_rect(inv, 5.0).is_err());
    assert!(bleed_rect(inv, px(1.0), 96.0).is_err());
    assert!(margin_rect(inv, &Margins::default(), 96.0).is_err());
}

#[test]
fn arq_110_every_numeric_input_rejects_non_finite_values() {
    let ok = square(100.0);
    let rect_with = |i: usize, v: f64| {
        let mut c = [0.0, 0.0, 100.0, 100.0];
        c[i] = v;
        Rect::new(c[0], c[1], c[2], c[3])
    };
    for v in BAD {
        for i in 0..4 {
            let r = rect_with(i, v);
            let nf = Err(LayoutError::NonFinite { field: "page" });
            assert_eq!(safe_rect(r, 5.0), nf);
            assert_eq!(bleed_rect(r, px(1.0), 96.0), nf);
            assert_eq!(margin_rect(r, &Margins::default(), 96.0), nf);
            assert_eq!(
                overflow(r, ok),
                Err(LayoutError::NonFinite { field: "bounds" })
            );
            assert_eq!(
                overflow(ok, r),
                Err(LayoutError::NonFinite { field: "rect" })
            );
        }
        assert_eq!(
            safe_rect(ok, v),
            Err(LayoutError::NonFinite { field: "percent" })
        );
        assert_eq!(
            bleed_rect(ok, px(v), 96.0),
            Err(LayoutError::NonFinite { field: "bleed" })
        );
        assert_eq!(
            bleed_rect(ok, px(1.0), v),
            Err(LayoutError::NonFinite { field: "dpi" })
        );
        assert_eq!(
            margin_rect(ok, &Margins::default(), v),
            Err(LayoutError::NonFinite { field: "dpi" })
        );
        let sides = [
            (margins(v, 0.0, 0.0, 0.0), "margins.top"),
            (margins(0.0, v, 0.0, 0.0), "margins.right"),
            (margins(0.0, 0.0, v, 0.0), "margins.bottom"),
            (margins(0.0, 0.0, 0.0, v), "margins.left"),
        ];
        for (m, field) in sides {
            assert_eq!(
                margin_rect(ok, &m, 96.0),
                Err(LayoutError::NonFinite { field })
            );
        }
    }
    // Finiteness is checked before ranges: a bad range earlier does not hide a NaN later.
    assert_eq!(
        margin_rect(ok, &margins(-1.0, f64::NAN, 0.0, 0.0), 96.0),
        Err(LayoutError::NonFinite {
            field: "margins.right"
        })
    );
    assert_eq!(
        bleed_rect(ok, px(f64::NAN), 0.0),
        Err(LayoutError::NonFinite { field: "bleed" })
    );
}

const MAX: f64 = f64::MAX;

fn is_non_finite<T: std::fmt::Debug>(r: Result<T, LayoutError>) {
    assert!(matches!(r, Err(LayoutError::NonFinite { .. })), "{r:?}");
}

#[test]
fn arq_110_a_computed_non_finite_value_is_an_error_even_when_the_result_looks_finite() {
    // Bleed: the sum `x1 + bleed` overflows (the bleed itself is in range).
    let page = Rect::new(0.0, 0.0, MAX, MAX);
    is_non_finite(bleed_rect(page, Length::new(10.0, Unit::Mm), 1e306));
    // Safe zone: the width `+inf` reaches the sum.
    is_non_finite(safe_rect(Rect::new(-MAX, 0.0, MAX, 100.0), 5.0));
    // Margin rectangle: a width or a margin sum that is `+inf`.
    let wide = Rect::new(-MAX, 0.0, MAX, 100.0);
    is_non_finite(margin_rect(wide, &margins(10.0, 10.0, 10.0, 10.0), 96.0));
    let tall = Rect::new(0.0, -MAX, 100.0, MAX);
    is_non_finite(margin_rect(tall, &margins(10.0, 10.0, 10.0, 10.0), 96.0));
    let small = square(100.0);
    is_non_finite(margin_rect(small, &margins(0.0, MAX, 0.0, MAX), 96.0));
    is_non_finite(margin_rect(small, &margins(MAX, 0.0, MAX, 0.0), 96.0));
    // Overflow: every side, `+inf` and `-inf`, with the other sides finite.
    // Each case is (bounds, rect) on one axis, as (lo, hi) pairs.
    let axis = [
        ((-MAX, 0.0), (MAX, MAX)),
        ((MAX, MAX), (-MAX, 0.0)),
        ((0.0, MAX), (-MAX, -MAX)),
        ((-MAX, -MAX), (0.0, MAX)),
    ];
    for ((b0, b1), (r0, r1)) in axis {
        is_non_finite(overflow(
            Rect::new(b0, 0.0, b1, 1.0),
            Rect::new(r0, 0.0, r1, 1.0),
        ));
        is_non_finite(overflow(
            Rect::new(0.0, b0, 1.0, b1),
            Rect::new(0.0, r0, 1.0, r1),
        ));
    }
}

#[test]
fn fd_05_f_rounding_that_erases_an_axis_is_no_area() {
    // Above 2^53 a px has no resolution: `x0 + 3` rounds to `x1`.
    let page = Rect::new(1e16, 0.0, 1e16 + 4.0, 100.0);
    assert_eq!(
        margin_rect(page, &margins(0.0, 0.0, 0.0, 3.0), 96.0),
        Err(LayoutError::NoArea)
    );
    assert_eq!(safe_rect(page, 49.0), Err(LayoutError::NoArea));
    let tall = Rect::new(0.0, 1e16, 100.0, 1e16 + 4.0);
    assert_eq!(
        margin_rect(tall, &margins(3.0, 0.0, 0.0, 0.0), 96.0),
        Err(LayoutError::NoArea)
    );
    assert_eq!(safe_rect(tall, 49.0), Err(LayoutError::NoArea));
}

#[test]
fn fd_05_f_margins_are_decided_per_axis_and_a_zero_sum_keeps_the_box() {
    let flat = Rect::new(0.0, 0.0, 0.0, 100.0);
    assert_eq!(
        margin_rect(flat, &margins(0.0, 0.0, 0.0, 0.0), 96.0),
        Ok(flat)
    );
    assert_eq!(
        margin_rect(flat, &margins(0.0, 0.0, 0.0, 5.0), 96.0),
        Err(LayoutError::NoArea)
    );
    let flat_y = Rect::new(0.0, 0.0, 100.0, 0.0);
    assert_eq!(
        margin_rect(flat_y, &margins(0.0, 5.0, 0.0, 0.0), 96.0)
            .unwrap()
            .y1,
        0.0
    );
    assert_eq!(
        margin_rect(flat_y, &margins(5.0, 0.0, 0.0, 0.0), 96.0),
        Err(LayoutError::NoArea)
    );
    // A zero sum on one axis does not hide a sum that reaches the size on the other.
    let page = Rect::new(0.0, 0.0, 100.0, 80.0);
    assert_eq!(
        margin_rect(page, &margins(0.0, 100.0, 0.0, 0.0), 96.0),
        Err(LayoutError::NoArea)
    );
    assert_eq!(
        margin_rect(page, &margins(40.0, 0.0, 40.0, 0.0), 96.0),
        Err(LayoutError::NoArea)
    );
    assert_eq!(
        margin_rect(page, &margins(0.0, 99.0, 0.0, 0.0), 96.0)
            .unwrap()
            .x1,
        1.0
    );
    // A margin of 0 on the top is in range, one below 0 names the field.
    assert!(margin_rect(page, &margins(0.0, 1.0, 1.0, 1.0), 96.0).is_ok());
    assert_eq!(
        margin_rect(page, &margins(-0.5, 1.0, 1.0, 1.0), 96.0),
        Err(LayoutError::OutOfRange {
            field: "margins.top"
        })
    );
}

#[test]
fn fd_05_a_dpi_that_is_not_positive_is_out_of_range_even_for_px() {
    let dpi = Err(LayoutError::OutOfRange { field: "dpi" });
    for d in [0.0, -96.0] {
        assert_eq!(bleed_rect(square(100.0), px(1.0), d), dpi);
        assert_eq!(
            margin_rect(square(100.0), &margins(1.0, 1.0, 1.0, 1.0), d),
            dpi
        );
    }
}

fn corners(r: Rect) -> Vec<f64> {
    vec![r.x0, r.y0, r.x1, r.y1]
}

fn sides(o: Overflow) -> Vec<f64> {
    vec![
        o.left,
        o.top,
        o.right,
        o.bottom,
        f64::from(u8::from(o.fully_outside)),
    ]
}

proptest! {
    // ARQ-110 (property) and ARQ-111: finite input in the domain gives finite output,
    // and a second run of each operation gives the same bytes (`-0.0` and `0.0` differ).
    #[test]
    fn arq_110_arq_111_finite_input_gives_finite_repeatable_output(
        x in -1e6..1e6f64, y in -1e6..1e6f64, w in 0.0..1e6f64, h in 0.0..1e6f64,
        pct in 0.0..49.9f64, mm in 0.0..10.0f64, dpi in 1.0..10_000.0f64,
        fa in 0.0..0.49f64, fb in 0.0..0.49f64,
    ) {
        let page = Rect::new(x, y, x + w, y + h);
        let m = margins(h * fa, w * fb, h * fb, w * fa);
        let safe = safe_rect(page, pct).unwrap();
        let ops: [&dyn Fn() -> Vec<f64>; 5] = [
            &|| corners(safe_rect(page, pct).unwrap()),
            &|| corners(bleed_rect(page, Length::new(mm, Unit::Mm), dpi).unwrap()),
            &|| corners(margin_rect(page, &m, dpi).unwrap()),
            &|| sides(overflow(page, safe).unwrap()),
            &|| sides(overflow(safe, page).unwrap()),
        ];
        for op in ops {
            let (a, b) = (op(), op());
            prop_assert_eq!(a.iter().map(|v| v.to_bits()).collect::<Vec<_>>(), b.iter().map(|v| v.to_bits()).collect::<Vec<_>>());
            prop_assert!(a.iter().all(|v| v.is_finite()));
        }
    }
}
