//! FD-04 (a, d, e, f), ARQ-110, ARQ-111 for the guide generator.

use gafira_layout::guides::generate;
use gafira_layout::{GuideAxis, GuideSpec, LayoutError, Length, Rect, Unit};
use proptest::prelude::*;

const BAD: [f64; 3] = [f64::NAN, f64::INFINITY, f64::NEG_INFINITY];

fn px(v: f64) -> Length {
    Length::new(v, Unit::Px)
}

fn spec(columns: u32, rows: u32, gap: f64, margin: f64) -> GuideSpec {
    GuideSpec {
        columns,
        rows,
        gap: px(gap),
        margin: px(margin),
    }
}

fn page(w: f64, h: f64) -> Rect {
    Rect::new(0.0, 0.0, w, h)
}

/// Positions of the guides on one axis, in output order.
fn on(axis: GuideAxis, guides: &[gafira_layout::Guide]) -> Vec<f64> {
    guides
        .iter()
        .filter(|g| g.axis == axis)
        .map(|g| g.position.value())
        .collect()
}

fn run(s: &GuideSpec, w: f64, h: f64) -> Vec<gafira_layout::Guide> {
    generate(s, page(w, h), 96.0).unwrap()
}

#[test]
fn fd_04_a_gap_of_negative_zero_is_a_gap_of_zero() {
    for (columns, rows) in [(1, 1), (3, 2)] {
        let neg = run(&spec(columns, rows, -0.0, 10.0), 300.0, 200.0);
        let zero = run(&spec(columns, rows, 0.0, 10.0), 300.0, 200.0);
        let bits = |g: &[gafira_layout::Guide]| -> Vec<u64> {
            g.iter().map(|g| g.position.value().to_bits()).collect()
        };
        assert_eq!(bits(&neg), bits(&zero));
    }
}

#[test]
fn fd_04_a_twelve_columns_in_1080() {
    let g = run(&spec(12, 0, 24.0, 24.0), 1080.0, 1080.0);
    let v = on(GuideAxis::Vertical, &g);
    assert_eq!(g.len(), 24);
    assert_eq!(&v[..6], &[24.0, 88.0, 112.0, 176.0, 200.0, 264.0]);
    assert_eq!(&v[18..], &[816.0, 880.0, 904.0, 968.0, 992.0, 1056.0]);
    assert!(g.iter().all(|g| !g.locked && g.position.unit() == Unit::Px));
    // margin 0: columns of 68 px, first guides 0, 68, 92
    let v0 = on(
        GuideAxis::Vertical,
        &run(&spec(12, 0, 24.0, 0.0), 1080.0, 1080.0),
    );
    assert_eq!(&v0[..3], &[0.0, 68.0, 92.0]);
    // margin 40: 12 c + 11 * 24 = 1000, c unrounded
    let v40 = on(
        GuideAxis::Vertical,
        &run(&spec(12, 0, 24.0, 40.0), 1080.0, 1080.0),
    );
    let c = v40[1] - v40[0];
    assert!((12.0 * c + 11.0 * 24.0 - 1000.0).abs() <= 1e-9);
    assert!(
        (c * 100.0).round() / 100.0 != c,
        "c = {c} must not be rounded"
    );
}

#[test]
fn fd_04_e_three_by_three_in_1080() {
    let g = run(&spec(3, 3, 24.0, 24.0), 1080.0, 1080.0);
    let want = [24.0, 352.0, 376.0, 704.0, 728.0, 1056.0];
    assert_eq!(g.len(), 12);
    assert_eq!(on(GuideAxis::Vertical, &g), want);
    assert_eq!(on(GuideAxis::Horizontal, &g), want);
}

#[test]
fn fd_04_f_zero_gap_emits_each_position_once_per_axis() {
    let g = run(&spec(3, 0, 0.0, 0.0), 900.0, 900.0);
    assert_eq!(on(GuideAxis::Vertical, &g), [0.0, 300.0, 600.0, 900.0]);
    assert_eq!(g.len(), 4);
    // equal means the same f64: two guides 1e-7 px apart stay two guides
    assert_eq!(run(&spec(2, 0, 1e-7, 0.0), 100.0, 100.0).len(), 4);
    // the same positions on the other axis are not merged with these
    let both = run(&spec(3, 3, 0.0, 0.0), 900.0, 900.0);
    assert_eq!(both.len(), 8);
    assert!(both[..4].iter().all(|g| g.axis == GuideAxis::Vertical));
    assert!(both[4..].iter().all(|g| g.axis == GuideAxis::Horizontal));
}

#[test]
fn fd_04_d_column_width_boundary_is_one_px_inclusive() {
    let ok = run(&spec(2, 0, 8.0, 0.0), 10.0, 10.0);
    assert_eq!(on(GuideAxis::Vertical, &ok), [0.0, 1.0, 9.0, 10.0]);
    let short =
        |w: f64, h: f64, cols, rows| generate(&spec(cols, rows, 8.0, 0.0), page(w, h), 96.0);
    assert_eq!(
        short(9.999, 99.0, 2, 0),
        Err(LayoutError::OutOfRange { field: "columns" })
    );
    assert_eq!(
        short(99.0, 9.999, 0, 2),
        Err(LayoutError::OutOfRange { field: "rows" })
    );
    assert!(short(99.0, 10.0, 0, 2).is_ok());
    // the value right below 1 px is an error, 1 px itself is not
    let one = |w: f64| generate(&spec(1, 0, 0.0, 0.0), page(w, 1.0), 96.0);
    assert_eq!(
        one(1.0f64.next_down()),
        Err(LayoutError::OutOfRange { field: "columns" })
    );
    assert!(one(1.0).is_ok());
    assert_eq!(
        short(10.0f64.next_down(), 99.0, 2, 0),
        Err(LayoutError::OutOfRange { field: "columns" })
    );
    // a margin that eats the whole side is the same error, not a silent empty result
    assert_eq!(
        generate(&spec(1, 0, 0.0, 50.0), page(100.0, 100.0), 96.0),
        Err(LayoutError::OutOfRange { field: "columns" })
    );
}

#[test]
fn fd_04_zero_divisions_give_no_guides_on_that_axis() {
    assert_eq!(run(&spec(0, 0, 24.0, 24.0), 1080.0, 1080.0), []);
    let rows_only = run(&spec(0, 3, 24.0, 24.0), 5.0, 1080.0);
    assert_eq!(on(GuideAxis::Horizontal, &rows_only).len(), 6);
    assert_eq!(rows_only.len(), 6);
    let cols_only = run(&spec(2, 0, 1.0, 0.0), 100.0, 0.5);
    assert_eq!(cols_only.len(), 4);
}

#[test]
fn fd_04_columns_use_the_width_and_rows_the_height() {
    let g = run(&spec(2, 2, 0.0, 0.0), 1000.0, 600.0);
    assert_eq!(on(GuideAxis::Vertical, &g), [0.0, 500.0, 1000.0]);
    assert_eq!(on(GuideAxis::Horizontal, &g), [0.0, 300.0, 600.0]);
}

#[test]
fn fd_03_guides_are_measured_from_the_top_left_corner_of_the_page() {
    // the same 1080 x 1080 page at the origin and away from it: same positions
    let s = spec(3, 3, 24.0, 24.0);
    let origin = generate(&s, page(1080.0, 1080.0), 96.0).unwrap();
    for corner in [(500.0, -300.0), (-2000.0, 4000.0), (0.0, 7.0)] {
        let p = Rect::new(corner.0, corner.1, corner.0 + 1080.0, corner.1 + 1080.0);
        assert_eq!(generate(&s, p, 96.0).unwrap(), origin);
    }
}

#[test]
fn fd_04_gap_and_margin_follow_the_dpi_input() {
    let s = GuideSpec {
        columns: 1,
        rows: 0,
        gap: px(0.0),
        margin: Length::new(1.0, Unit::In),
    };
    for dpi in [96.0, 300.0] {
        let g = generate(&s, page(1000.0, 1000.0), dpi).unwrap();
        assert_eq!(on(GuideAxis::Vertical, &g), [dpi, 1000.0 - dpi]);
    }
    let mm = GuideSpec {
        margin: Length::new(25.4, Unit::Mm),
        ..s
    };
    let g = generate(&mm, page(1000.0, 1000.0), 96.0).unwrap();
    assert!((on(GuideAxis::Vertical, &g)[0] - 96.0).abs() <= 1e-9);
    // positions are px whatever the unit of gap and margin: 2 columns, 1 in each, 960 px at 96 dpi
    let inch = Length::new(1.0, Unit::In);
    let two = GuideSpec {
        columns: 2,
        gap: inch,
        margin: inch,
        ..s
    };
    let g = generate(&two, page(960.0, 960.0), 96.0).unwrap();
    assert_eq!(on(GuideAxis::Vertical, &g), [96.0, 432.0, 528.0, 864.0]);
    assert!(g.iter().all(|g| g.position.unit() == Unit::Px));
}

#[test]
fn fd_04_f_negative_gap_or_margin_is_an_error_and_zero_is_not() {
    let p = page(1080.0, 1080.0);
    assert_eq!(
        generate(&spec(3, 3, -0.01, 24.0), p, 96.0),
        Err(LayoutError::OutOfRange { field: "gap" })
    );
    assert_eq!(
        generate(&spec(3, 3, 24.0, -0.01), p, 96.0),
        Err(LayoutError::OutOfRange { field: "margin" })
    );
    // unused axes still validate the shared inputs
    assert!(generate(&spec(0, 0, -1.0, 0.0), p, 96.0).is_err());
    assert_eq!(
        generate(&spec(1, 0, 0.0, 0.0), p, 0.0),
        Err(LayoutError::OutOfRange { field: "dpi" })
    );
    // -0.0 is zero, and the guide lands on +0.0
    let g = run(&spec(1, 0, -0.0, -0.0), 100.0, 100.0);
    assert_eq!(on(GuideAxis::Vertical, &g)[0].to_bits(), 0.0f64.to_bits());
}

#[test]
fn arq_110_every_float_input_is_checked_even_when_unused() {
    let ok = spec(3, 0, 24.0, 24.0);
    let p = page(1080.0, 1080.0);
    for bad in BAD {
        for (spec, page, dpi, field) in [
            (GuideSpec { gap: px(bad), ..ok }, p, 96.0, "gap"),
            (
                GuideSpec {
                    margin: px(bad),
                    ..ok
                },
                p,
                96.0,
                "margin",
            ),
            (ok, p, bad, "dpi"),
            (ok, Rect::new(bad, 0.0, 10.0, 10.0), 96.0, "page"),
            (ok, Rect::new(0.0, bad, 10.0, 10.0), 96.0, "page"),
            (ok, Rect::new(0.0, 0.0, bad, 10.0), 96.0, "page"),
            (ok, Rect::new(0.0, 0.0, 10.0, bad), 96.0, "page"),
        ] {
            assert_eq!(
                generate(&spec, page, dpi),
                Err(LayoutError::NonFinite { field }),
                "{field} = {bad}"
            );
        }
    }
    // intermediates: an overflowing size, an overflowing gap total, an overflowing unit conversion
    let huge = |g: GuideSpec, r: Rect| generate(&g, r, 96.0);
    let max = f64::MAX;
    assert!(matches!(
        huge(spec(2, 0, 0.0, 0.0), Rect::new(-max, 0.0, max, 1.0)),
        Err(LayoutError::NonFinite { .. })
    ));
    assert!(matches!(
        huge(spec(3, 0, max, 0.0), page(1000.0, 1.0)),
        Err(LayoutError::NonFinite { .. })
    ));
    assert!(matches!(
        huge(
            GuideSpec {
                margin: Length::new(max, Unit::In),
                ..ok
            },
            p
        ),
        Err(LayoutError::NonFinite { .. })
    ));
}

#[test]
fn arq_111_generating_twice_gives_the_same_bytes_and_axes_do_not_swap() {
    let s = spec(5, 2, 7.5, 13.25);
    let a = run(&s, 977.0, 411.0);
    let b = run(&s, 977.0, 411.0);
    assert_eq!(a.len(), b.len());
    for (x, y) in a.iter().zip(&b) {
        assert_eq!(x.axis, y.axis);
        assert_eq!(x.position.value().to_bits(), y.position.value().to_bits());
    }
    // asymmetric on purpose: swapping axes, or reversing a sort, changes the bytes
    assert_eq!(on(GuideAxis::Vertical, &a).len(), 10);
    assert_eq!(on(GuideAxis::Horizontal, &a).len(), 4);
    assert_eq!(on(GuideAxis::Vertical, &a)[0], 13.25);
    assert_eq!(on(GuideAxis::Horizontal, &a)[3], 411.0 - 13.25);
}

proptest! {
    // FD-04 rules for any n, gap, margin: 2n guides, ascending, mirrored, spaced c then gap.
    #[test]
    fn fd_04_every_axis_has_2n_ordered_mirrored_guides(
        cols in 1u32..30, rows in 0u32..30,
        gap in 0.5..50.0f64, margin in 0.0..60.0f64,
        extra_w in 0.0..500.0f64, extra_h in 0.0..500.0f64,
    ) {
        let need = |n: u32| 2.0 * margin + f64::from(n) + f64::from(n - 1) * gap;
        let w = need(cols) + extra_w;
        let h = if rows == 0 { 7.0 } else { need(rows) + extra_h };
        let g = generate(&spec(cols, rows, gap, margin), page(w, h), 96.0).unwrap();
        prop_assert_eq!(g.len(), 2 * (cols + rows) as usize);
        let first_h = g.iter().position(|g| g.axis == GuideAxis::Horizontal).unwrap_or(g.len());
        prop_assert!(g[..first_h].iter().all(|g| g.axis == GuideAxis::Vertical));
        prop_assert!(g[first_h..].iter().all(|g| g.axis == GuideAxis::Horizontal));
        for (axis, n, side) in [(GuideAxis::Vertical, cols, w), (GuideAxis::Horizontal, rows, h)] {
            let s = on(axis, &g);
            if n == 0 { prop_assert!(s.is_empty()); continue; }
            let c = (side - 2.0 * margin - f64::from(n - 1) * gap) / f64::from(n);
            prop_assert!(s.windows(2).all(|p| p[0] < p[1]));
            prop_assert!((s[0] - margin).abs() <= 1e-9);
            for j in 0..n as usize {
                prop_assert!((s[2 * j + 1] - s[2 * j] - c).abs() <= 1e-9);
                if j + 1 < n as usize {
                    prop_assert!((s[2 * j + 2] - s[2 * j + 1] - gap).abs() <= 1e-9);
                }
            }
            prop_assert!((s[s.len() - 1] - (side - margin)).abs() <= 1e-9);
        }
    }

    // ARQ-110 property: finite in the domain gives finite out.
    #[test]
    fn arq_110_finite_input_gives_finite_output(
        cols in 0u32..20, rows in 0u32..20, gap in 0.0..100.0f64, margin in 0.0..100.0f64,
    ) {
        let side = 2.0 * margin + 20.0 * (1.0 + gap) + 1.0;
        let g = generate(&spec(cols, rows, gap, margin), page(side, side), 96.0).unwrap();
        prop_assert!(g.iter().all(|g| g.position.value().is_finite()));
    }
}

/// A count far above the 16 384 that fit; `u32::MAX` itself is in `guides_huge`.
const HUGE: u32 = 100_000;

/// The `page` side limit, in px (ARQ-110, FD-04).
const MAX_SIDE: f64 = 16_384.0;

#[test]
fn fd_04_page_side_limit_is_inclusive_and_checked_per_side() {
    let out = |field| Err(LayoutError::OutOfRange { field });
    let gen_ = |s: &GuideSpec, w: f64, h: f64| generate(s, page(w, h), 96.0);
    // the maximum is valid: 16 384 columns of 1 px give 16 385 distinct positions
    let g = run(&spec(MAX_SIDE as u32, 0, 0.0, 0.0), MAX_SIDE, MAX_SIDE);
    assert_eq!(on(GuideAxis::Vertical, &g).len(), 16_385);
    assert!(gen_(&spec(0, 1, 0.0, 0.0), 1.0, MAX_SIDE).is_ok());
    // just above, on each side alone, also when that axis has n = 0
    assert_eq!(
        gen_(&spec(1, 0, 0.0, 0.0), MAX_SIDE.next_up(), 1.0),
        out("page")
    );
    assert_eq!(
        gen_(&spec(1, 0, 0.0, 0.0), 1.0, MAX_SIDE.next_up()),
        out("page")
    );
    assert_eq!(gen_(&spec(0, 1, 0.0, 0.0), 16_385.0, 1.0), out("page"));
    assert_eq!(gen_(&spec(0, 0, 0.0, 0.0), 16_385.0, 1.0), out("page"));
    assert_eq!(gen_(&spec(0, 0, 0.0, 0.0), 1.0, 16_385.0), out("page"));
}

#[test]
fn fd_04_page_side_is_checked_after_non_finite_and_before_the_column_width() {
    let out = |field| Err(LayoutError::OutOfRange { field });
    let big = page(16_385.0, 16_385.0);
    let ok = page(MAX_SIDE, MAX_SIDE);
    // a valid side: an enormous n is the column width rule
    assert_eq!(generate(&spec(HUGE, 0, 0.0, 0.0), ok, 96.0), out("columns"));
    assert_eq!(generate(&spec(0, HUGE, 0.0, 0.0), ok, 96.0), out("rows"));
    // a side out of range: the same n is the page rule
    assert_eq!(generate(&spec(HUGE, 0, 0.0, 0.0), big, 96.0), out("page"));
    // non-finite inputs win over the side limit
    assert_eq!(
        generate(&spec(1, 1, 0.0, 0.0), big, f64::NAN),
        Err(LayoutError::NonFinite { field: "dpi" })
    );
    assert_eq!(
        generate(&spec(1, 1, f64::NAN, 0.0), big, 96.0),
        Err(LayoutError::NonFinite { field: "gap" })
    );
    assert_eq!(
        generate(&spec(1, 1, 0.0, 0.0), page(f64::NAN, 20_000.0), 96.0),
        Err(LayoutError::NonFinite { field: "page" })
    );
}

#[test]
fn fd_04_dpi_is_checked_after_non_finite_and_before_the_other_range_rules() {
    let dpi = Err(LayoutError::OutOfRange { field: "dpi" });
    let p = page(1080.0, 1080.0);
    for d in [0.0, -96.0] {
        assert_eq!(generate(&spec(0, 0, 0.0, 0.0), p, d), dpi);
        assert_eq!(generate(&spec(1, 1, -1.0, 0.0), p, d), dpi);
        assert_eq!(generate(&spec(1, 1, 0.0, -1.0), p, d), dpi);
        assert_eq!(generate(&spec(1, 1, 0.0, 0.0), page(16_385.0, 1.0), d), dpi);
    }
    for (gap, margin, field) in [(f64::NAN, 0.0, "gap"), (0.0, f64::NAN, "margin")] {
        assert_eq!(
            generate(&spec(1, 1, gap, margin), p, 0.0),
            Err(LayoutError::NonFinite { field })
        );
    }
}

proptest! {
    // FD-04 bound: with c >= 1 px, n <= 16 384, so every Ok has at most 32 768 guides per axis.
    #[test]
    fn fd_04_every_ok_has_at_most_32768_guides_per_axis(
        cols in prop_oneof![0u32..40, 16_000u32..17_000, Just(HUGE)],
        rows in prop_oneof![0u32..40, 16_000u32..17_000, Just(HUGE)],
        w in prop_oneof![1.0..100.0f64, 16_000.0..16_400.0f64],
        h in prop_oneof![1.0..100.0f64, 16_000.0..16_400.0f64],
        margin in prop_oneof![Just(0.0), 0.0..10.0f64],
    ) {
        if let Ok(g) = generate(&spec(cols, rows, 0.0, margin), page(w, h), 96.0) {
            for axis in [GuideAxis::Vertical, GuideAxis::Horizontal] {
                prop_assert!(on(axis, &g).len() <= 32_768);
            }
        }
    }
}
