//! FD-03 (a, d), ARQ-110, ARQ-111 for guide positions.

use gafira_layout::{GuideAxis, LayoutError, Length, Unit, place_guide};
use proptest::prelude::*;

const BAD: [f64; 3] = [f64::NAN, f64::INFINITY, f64::NEG_INFINITY];

fn unit() -> impl Strategy<Value = Unit> {
    prop::sample::select(Unit::ALL.to_vec())
}

proptest! {
    // FD-03 (a): stored as written, shown in the ruler unit within 0.01 units.
    #[test]
    fn fd_03_a_position_is_stored_unrounded_and_shown_within_a_hundredth(
        v in -1.0e6..1.0e6f64, u in unit(), ruler in unit(), dpi in 1.0..=10_000.0f64,
        vertical in any::<bool>(),
    ) {
        let axis = if vertical { GuideAxis::Vertical } else { GuideAxis::Horizontal };
        let written = Length::new(v, u);
        let (guide, shown) = place_guide(axis, written, ruler, dpi).unwrap();
        prop_assert_eq!(guide.axis, axis);
        prop_assert!(!guide.locked);
        prop_assert_eq!(guide.position.value().to_bits(), v.to_bits());
        prop_assert_eq!(guide.position.unit(), u);
        prop_assert_eq!(shown.unit(), ruler);
        let expected = written.convert(ruler, dpi).unwrap().value();
        prop_assert!((shown.value() - expected).abs() <= 0.01);
        if u == ruler {
            prop_assert_eq!(shown.value().to_bits(), v.to_bits());
        }
    }
}

#[test]
fn fd_03_d_non_finite_position_or_dpi_is_rejected_by_field() {
    for bad in BAD {
        assert_eq!(
            place_guide(
                GuideAxis::Vertical,
                Length::new(bad, Unit::Px),
                Unit::Mm,
                96.0
            ),
            Err(LayoutError::NonFinite { field: "position" })
        );
        assert_eq!(
            place_guide(
                GuideAxis::Vertical,
                Length::new(1.0, Unit::Px),
                Unit::Mm,
                bad
            ),
            Err(LayoutError::NonFinite { field: "dpi" })
        );
    }
    for dpi in [0.0, -96.0] {
        assert_eq!(
            place_guide(
                GuideAxis::Vertical,
                Length::new(1.0, Unit::Px),
                Unit::Mm,
                dpi
            ),
            Err(LayoutError::OutOfRange { field: "dpi" })
        );
    }
}

#[test]
fn fd_03_d_a_result_that_overflows_is_an_error_not_infinity() {
    assert_eq!(
        place_guide(
            GuideAxis::Horizontal,
            Length::new(f64::MAX, Unit::In),
            Unit::Px,
            10_000.0
        ),
        Err(LayoutError::NonFinite { field: "position" })
    );
}

#[test]
fn arq_111_placing_twice_gives_the_same_bytes() {
    let run = || {
        place_guide(
            GuideAxis::Horizontal,
            Length::new(-3.7, Unit::Pt),
            Unit::Cm,
            123.4,
        )
    };
    let (a, b) = (run().unwrap(), run().unwrap());
    assert_eq!(
        a.0.position.value().to_bits(),
        b.0.position.value().to_bits()
    );
    assert_eq!(a.1.value().to_bits(), b.1.value().to_bits());
    assert_eq!(a.0.axis, GuideAxis::Horizontal);
}
