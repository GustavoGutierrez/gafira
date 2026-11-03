//! FD-11 (b, d, e), D-17, D-28, ARQ-110 and ARQ-111 for `distribute`.

use gafira_layout::{DistributeAxis, DistributeMode, Item, LayoutError, Offset, Rect, distribute};
use proptest::prelude::*;

const BAD: [f64; 3] = [f64::NAN, f64::INFINITY, f64::NEG_INFINITY];
const MODES: [DistributeMode; 2] = [DistributeMode::EqualGap, DistributeMode::EqualCentres];
const AXES: [DistributeAxis; 2] = [DistributeAxis::Horizontal, DistributeAxis::Vertical];

/// A box `[start, end]` on x and `[0, 10]` on y.
fn span(start: f64, end: f64) -> Item {
    Item {
        rect: Rect::new(start, 0.0, end, 10.0),
        locked: false,
    }
}

fn lock(i: Item) -> Item {
    Item { locked: true, ..i }
}

fn flip(i: Item) -> Item {
    Item {
        rect: Rect::new(i.rect.y0, i.rect.x0, i.rect.y1, i.rect.x1),
        locked: i.locked,
    }
}

fn shift(i: Item, dx: f64, dy: f64) -> Item {
    Item {
        rect: Rect::new(
            i.rect.x0 + dx,
            i.rect.y0 + dy,
            i.rect.x1 + dx,
            i.rect.y1 + dy,
        ),
        locked: i.locked,
    }
}

/// The offsets along the axis of `items` laid out on x, checked on both axes: the same
/// items turned on their side give the same numbers on y, and the other axis is `+0.0`.
fn run(mode: DistributeMode, items: &[Item]) -> Vec<f64> {
    let h = distribute(DistributeAxis::Horizontal, mode, items).unwrap();
    let flipped: Vec<Item> = items.iter().copied().map(flip).collect();
    let v = distribute(DistributeAxis::Vertical, mode, &flipped).unwrap();
    assert_eq!(h.len(), items.len());
    for (a, b) in h.iter().zip(&v) {
        assert_eq!(a.dx.to_bits(), b.dy.to_bits());
        assert_eq!(a.dy.to_bits(), 0.0f64.to_bits());
        assert_eq!(b.dx.to_bits(), 0.0f64.to_bits());
    }
    h.iter().map(|o| o.dx).collect()
}

fn gap(items: &[Item]) -> Vec<f64> {
    run(DistributeMode::EqualGap, items)
}

fn centres(items: &[Item]) -> Vec<f64> {
    run(DistributeMode::EqualCentres, items)
}

#[test]
fn fd_11_e_three_boxes_of_width_100_over_250_give_a_gap_of_minus_25_and_no_error() {
    let items = [span(0.0, 100.0), span(40.0, 140.0), span(150.0, 250.0)];
    // the gap is (150 - 100 - 100) / 2 = -25, so the middle box starts at 75
    assert_eq!(gap(&items), [0.0, 35.0, 0.0]);
}

#[test]
fn fd_11_b_the_gaps_are_equal_and_the_first_and_the_last_do_not_move() {
    let items = [
        span(0.0, 10.0),
        span(20.0, 50.0),
        span(55.0, 95.0),
        span(200.0, 230.0),
    ];
    // (200 - 10 - (30 + 40)) / 3 = 40: B starts at 50, C at 50 + 30 + 40
    assert_eq!(gap(&items), [0.0, 30.0, 65.0, 0.0]);
}

#[test]
fn d_28_the_elements_are_sorted_by_their_position_on_the_axis_and_the_output_keeps_the_input_order()
{
    let [a, b, c, d] = [
        span(0.0, 10.0),
        span(20.0, 50.0),
        span(55.0, 95.0),
        span(200.0, 230.0),
    ];
    assert_eq!(gap(&[c, a, d, b]), [65.0, 0.0, 0.0, 30.0]);
    assert_eq!(gap(&[d, c, b, a]), [0.0, 65.0, 30.0, 0.0]);
}

#[test]
fn fd_11_equal_centres_spaces_the_centres_and_not_the_edges() {
    let items = [
        span(0.0, 10.0),
        span(20.0, 50.0),
        span(55.0, 95.0),
        span(200.0, 230.0),
    ];
    // centres 5, 35, 75, 215: the step is 70, so the middle ones go to 75 and 145
    assert_eq!(centres(&items), [0.0, 40.0, 70.0, 0.0]);
    let [a, b, c, d] = [items[0], items[1], items[2], items[3]];
    assert_eq!(centres(&[d, b, a, c]), [0.0, 40.0, 0.0, 70.0]);
    // the mode defaults to equal gap
    assert_eq!(DistributeMode::default(), DistributeMode::EqualGap);
    // the sort of equal centres is by centre, not by start: a wide box with an early start
    let wide = span(0.0, 100.0); // centre 50
    let small = span(10.0, 20.0); // centre 15
    let first = span(-10.0, 10.0); // centre 0
    let last = span(90.0, 110.0); // centre 100
    // sorted by centre: first (0), small (15), wide (50), last (100): step 100 / 3
    let got = centres(&[first, wide, small, last]);
    let step = 100.0 / 3.0;
    assert_eq!(got[0], 0.0);
    assert_eq!(got[3], 0.0);
    assert!((got[2] - (step - 15.0)).abs() <= 1e-9);
    assert!((got[1] - (2.0 * step - 50.0)).abs() <= 1e-9);
}

#[test]
fn d_28_equal_positions_keep_the_input_order() {
    // B and C start together; B comes first in the input
    let [a, b, c, d] = [
        span(0.0, 10.0),
        span(20.0, 30.0),
        span(20.0, 50.0),
        span(101.0, 111.0),
    ];
    // gap (101 - 10 - 40) / 3 = 17
    assert_eq!(gap(&[a, b, c, d]), [0.0, 7.0, 34.0, 0.0]);
    // swapped in the input, swapped in the result: C goes first now
    assert_eq!(gap(&[a, c, b, d]), [0.0, 7.0, 54.0, 0.0]);
    // -0.0 and 0.0 are the same position: the input order decides
    let p = span(0.0, 10.0);
    let q = span(-0.0, 20.0);
    let r = span(100.0, 110.0);
    assert_eq!(gap(&[p, q, r]), [0.0, 45.0, 0.0]);
    assert_eq!(gap(&[q, p, r]), [0.0, 55.0, 0.0]);
    // the same with centres: S and T have the same centre, 5; the step to U (105) is 50
    let s = span(0.0, 10.0);
    let t = span(-5.0, 15.0);
    let u = span(100.0, 110.0);
    assert_eq!(centres(&[s, t, u]), [0.0, 50.0, 0.0]);
    assert_eq!(centres(&[t, s, u]), [0.0, 50.0, 0.0]);
    assert_eq!(centres(&[u, s, t]), [0.0, 0.0, 50.0]);
}

#[test]
fn d_28_a_long_shuffled_run_of_equal_positions_keeps_the_input_order() {
    // 300 boxes in a shuffled order, three on every one of 100 positions, with sizes that
    // differ: an unstable sort would reorder the ties
    let key = |i: u32| i * 37 % 100;
    let items: Vec<Item> = (0..300)
        .map(|i| {
            let start = f64::from(key(i)) * 10.0;
            span(start, start + 1.0 + f64::from(i % 7))
        })
        .collect();
    let got = gap(&items);
    // the order that the spec describes: by position, then by input index
    let mut order: Vec<u32> = (0..300).collect();
    order.sort_by_key(|&i| (key(i), i));
    assert_eq!(got[order[0] as usize], 0.0);
    assert_eq!(got[order[299] as usize], 0.0);
    let moved = |i: u32| {
        let r = items[i as usize].rect;
        (r.x0 + got[i as usize], r.x1 + got[i as usize])
    };
    let gaps: Vec<f64> = order
        .windows(2)
        .map(|w| moved(w[1]).0 - moved(w[0]).1)
        .collect();
    assert!(gaps.iter().all(|g| (g - gaps[0]).abs() <= 1e-6), "{gaps:?}");
}

#[test]
fn d_17_locked_elements_do_not_count_and_do_not_move() {
    let a = span(0.0, 10.0);
    let b = span(20.0, 50.0);
    let c = span(200.0, 230.0);
    let l = lock(span(60.0, 70.0));
    // 4 boxes, one locked: the three free ones are spread, the locked one stays
    assert_eq!(gap(&[a, l, b, c]), [0.0, 0.0, 70.0, 0.0]);
    // wherever the locked one is, even outside the span or first or last in the input
    for far in [
        lock(span(-100.0, -90.0)),
        lock(span(500.0, 600.0)),
        lock(span(30.0, 40.0)),
    ] {
        assert_eq!(gap(&[far, a, b, c]), [0.0, 0.0, 70.0, 0.0]);
        assert_eq!(gap(&[a, b, c, far]), [0.0, 70.0, 0.0, 0.0]);
    }
    // 3 boxes, one locked: nothing changes
    assert_eq!(gap(&[a, l, c]), [0.0, 0.0, 0.0]);
    assert_eq!(centres(&[a, l, c]), [0.0, 0.0, 0.0]);
    // a locked element may be covered by a free one: no error, no avoidance
    let covered = gap(&[a, lock(span(60.0, 120.0)), b, c]);
    assert_eq!(covered, [0.0, 0.0, 70.0, 0.0]);
}

// FD-11 (d)
#[test]
fn fd_11_d_four_elements_with_one_locked_distribute_the_three_free_ones() {
    let a = span(0.0, 10.0);
    let b = span(20.0, 50.0);
    let c = span(200.0, 230.0);
    let l = lock(span(60.0, 70.0));
    // the locked element is not counted and does not move, wherever it sits in the input
    assert_eq!(gap(&[a, l, b, c]), [0.0, 0.0, 70.0, 0.0]);
    assert_eq!(gap(&[l, a, b, c]), [0.0, 0.0, 70.0, 0.0]);
    assert_eq!(gap(&[a, b, c, l]), [0.0, 70.0, 0.0, 0.0]);
    // three elements with one locked: two free ones, nothing changes
    for mode in MODES {
        assert_eq!(run(mode, &[a, l, c]), [0.0; 3]);
    }
}

#[test]
fn d_17_with_fewer_than_three_free_elements_nothing_moves() {
    let a = span(0.0, 10.0);
    let b = span(20.0, 50.0);
    let c = span(200.0, 230.0);
    for mode in MODES {
        assert_eq!(run(mode, &[]), [] as [f64; 0]);
        assert_eq!(run(mode, &[a]), [0.0]);
        assert_eq!(run(mode, &[a, c]), [0.0, 0.0]);
        // 5 elements, 3 locked
        assert_eq!(run(mode, &[a, lock(b), lock(c), lock(a), c]), [0.0; 5]);
        // 3 locked and nothing else
        assert_eq!(run(mode, &[lock(a), lock(b), lock(c)]), [0.0; 3]);
    }
}

#[test]
fn d_17_two_wide_overlapping_free_elements_give_no_error_and_no_movement() {
    // With two free boxes the gap is never computed: here it would be
    // `(-MAX - MAX - 0) / 1`, which overflows. A third box is locked, or the two are all.
    let wide = span(-f64::MAX, f64::MAX);
    let locked = lock(span(-f64::MAX, f64::MAX));
    for mode in MODES {
        assert_eq!(run(mode, &[wide, wide]), [0.0, 0.0]);
        assert_eq!(run(mode, &[wide, locked, wide]), [0.0; 3]);
        assert_eq!(run(mode, &[locked, wide, wide]), [0.0; 3]);
    }
}

#[test]
fn fd_11_a_zero_offset_is_positive_zero_also_when_the_gap_underflows() {
    // The gap is `-5e-324 / 2 = -0.0`, so the middle box lands on `-0.0 + -0.0`, and
    // `-0.0 - 0.0` would be `-0.0`.
    let items = [span(-1.0, -0.0), span(0.0, 5e-324), span(0.0, 1.0)];
    for mode in MODES {
        for offsets in [
            distribute(DistributeAxis::Horizontal, mode, &items).unwrap(),
            distribute(DistributeAxis::Vertical, mode, &items.map(flip)).unwrap(),
        ] {
            for o in offsets {
                assert_eq!(o.dx.to_bits(), 0.0f64.to_bits(), "{mode:?}");
                assert_eq!(o.dy.to_bits(), 0.0f64.to_bits(), "{mode:?}");
            }
        }
    }
}

#[test]
fn fd_11_distribute_works_away_from_the_origin_and_on_negative_coordinates() {
    let items = [
        span(0.0, 10.0),
        span(20.0, 50.0),
        span(55.0, 95.0),
        span(200.0, 230.0),
    ];
    for (dx, dy) in [(12340.0, 0.0), (-5000.0, 1.0), (-300.0, -2000.0)] {
        let moved: Vec<Item> = items.iter().map(|&i| shift(i, dx, dy)).collect();
        assert_eq!(gap(&moved), [0.0, 30.0, 65.0, 0.0]);
        assert_eq!(centres(&moved), [0.0, 40.0, 70.0, 0.0]);
    }
}

#[test]
fn the_other_axis_does_not_change_and_every_zero_is_positive() {
    let items = [
        span(0.0, 10.0),
        lock(span(20.0, 50.0)),
        span(-0.0, 95.0),
        span(100.0, 230.0),
    ];
    for mode in MODES {
        for axis in AXES {
            let input: Vec<Item> = match axis {
                DistributeAxis::Horizontal => items.to_vec(),
                DistributeAxis::Vertical => items.iter().copied().map(flip).collect(),
            };
            let got = distribute(axis, mode, &input).unwrap();
            for o in got {
                let other = match axis {
                    DistributeAxis::Horizontal => o.dy,
                    DistributeAxis::Vertical => o.dx,
                };
                assert_eq!(other.to_bits(), 0.0f64.to_bits());
            }
        }
    }
}

#[test]
fn the_result_has_one_offset_per_box_in_the_input_order_also_when_nothing_moves() {
    let items = [
        span(0.0, 10.0),
        span(20.0, 50.0),
        lock(span(55.0, 95.0)),
        lock(span(200.0, 230.0)),
    ];
    for mode in MODES {
        let got = distribute(DistributeAxis::Horizontal, mode, &items).unwrap();
        assert_eq!(got, vec![Offset { dx: 0.0, dy: 0.0 }; 4]);
    }
}

// ---------------------------------------------------------------------------
// ARQ-110

fn setters() -> [fn(&mut Rect, f64); 4] {
    [
        |r, v| r.x0 = v,
        |r, v| r.y0 = v,
        |r, v| r.x1 = v,
        |r, v| r.y1 = v,
    ]
}

fn five() -> Vec<Item> {
    vec![
        span(0.0, 10.0),
        span(20.0, 50.0),
        span(55.0, 95.0),
        span(100.0, 130.0),
        span(200.0, 230.0),
    ]
}

#[test]
fn arq_110_every_number_of_every_box_must_be_finite_also_on_the_other_axis() {
    for mode in MODES {
        for axis in AXES {
            for set in setters() {
                for v in BAD {
                    for at in 0..5 {
                        for lk in [false, true] {
                            // five boxes, then two (fewer than three free), then one
                            for keep in [5, 2, 1] {
                                if at >= keep {
                                    continue;
                                }
                                let mut items = five();
                                items.truncate(keep);
                                items[at].locked = lk;
                                set(&mut items[at].rect, v);
                                assert_eq!(
                                    distribute(axis, mode, &items),
                                    Err(LayoutError::NonFinite { field: "item" }),
                                    "{mode:?} {axis:?} {v} at {at} locked {lk} of {keep}"
                                );
                            }
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn arq_110_a_box_with_a_negative_size_is_an_error_on_either_axis_and_zero_is_valid() {
    for mode in MODES {
        for axis in AXES {
            for bad in [
                Rect::new(5.0, 0.0, 4.0, 10.0),
                Rect::new(0.0, 5.0, 10.0, 4.0),
            ] {
                for lk in [false, true] {
                    for keep in [5, 2] {
                        let mut items = five();
                        items.truncate(keep);
                        items[1].rect = bad;
                        items[1].locked = lk;
                        assert_eq!(
                            distribute(axis, mode, &items),
                            Err(LayoutError::InvalidBox { field: "item" })
                        );
                    }
                }
            }
            let zero = [span(3.0, 3.0), span(5.0, 5.0), span(9.0, 9.0)];
            assert!(distribute(axis, mode, &zero).is_ok());
        }
    }
    // every number is checked before any box: a NaN after an inverted box still wins
    let mut items = five();
    items[0].rect = Rect::new(5.0, 0.0, 4.0, 0.0);
    items[3].rect.x0 = f64::NAN;
    assert_eq!(
        distribute(DistributeAxis::Horizontal, DistributeMode::EqualGap, &items),
        Err(LayoutError::NonFinite { field: "item" })
    );
}

#[test]
fn arq_110_a_centre_or_a_sum_that_overflows_is_an_error() {
    let wide = Rect::new(f64::MAX, 0.0, f64::MAX, 10.0);
    for mode in MODES {
        for lk in [false, true] {
            for keep in [5, 1] {
                let mut items = five();
                items.truncate(keep);
                items[0] = Item {
                    rect: wide,
                    locked: lk,
                };
                assert_eq!(
                    distribute(DistributeAxis::Horizontal, mode, &items),
                    Err(LayoutError::NonFinite { field: "item" })
                );
            }
        }
    }
    // two middle boxes of width MAX: their sizes add up to infinity
    let items = [
        span(0.0, 1.0),
        span(5.0, f64::MAX),
        span(6.0, f64::MAX),
        span(10.0, 11.0),
    ];
    assert_eq!(
        distribute(DistributeAxis::Horizontal, DistributeMode::EqualGap, &items),
        Err(LayoutError::NonFinite { field: "gap" })
    );
    // an offset that overflows: the middle box moves from -MAX / 2 to 0.56 MAX
    let items = [
        span(-f64::MAX, f64::MAX),
        span(-f64::MAX / 2.0, -f64::MAX / 2.0),
        span(f64::MAX / 8.0, f64::MAX / 8.0),
    ];
    assert_eq!(
        distribute(DistributeAxis::Horizontal, DistributeMode::EqualGap, &items),
        Err(LayoutError::NonFinite { field: "offset" })
    );
    // the centres of the same boxes are fine
    assert!(
        distribute(
            DistributeAxis::Horizontal,
            DistributeMode::EqualCentres,
            &items
        )
        .is_ok()
    );
}

// ---------------------------------------------------------------------------
// ARQ-111 and the properties

#[test]
fn arq_111_the_same_input_gives_the_same_bytes() {
    let a = span(0.0, 10.0);
    let cases: [Vec<Item>; 4] = [
        five(),
        vec![
            span(-0.0, 10.0),
            lock(span(0.0, 5.0)),
            span(20.0, 50.0),
            span(0.0, 230.0),
        ],
        vec![a, a, a, a, a],
        vec![a, span(20.0, 50.0)],
    ];
    for items in &cases {
        for mode in MODES {
            for axis in AXES {
                let bits = |o: Vec<Offset>| -> Vec<[u64; 2]> {
                    o.iter().map(|o| [o.dx.to_bits(), o.dy.to_bits()]).collect()
                };
                let first = bits(distribute(axis, mode, items).unwrap());
                for _ in 0..3 {
                    assert_eq!(bits(distribute(axis, mode, items).unwrap()), first);
                }
            }
        }
    }
}

proptest! {
    /// In the order of the positions, the free boxes end with equal gaps (or equal
    /// centre steps), the first and last free ones have not moved and the locked ones neither.
    #[test]
    fn fd_11_b_the_result_is_evenly_spaced_with_the_ends_fixed(
        raw in prop::collection::vec((-4000i32..4000, 0i32..400, any::<bool>()), 0..12),
        centres_mode in any::<bool>(),
        vertical in any::<bool>(),
    ) {
        let mode = if centres_mode { DistributeMode::EqualCentres } else { DistributeMode::EqualGap };
        let items: Vec<Item> = raw
            .iter()
            .map(|&(s, w, l)| Item { locked: l, ..span(f64::from(s) / 2.0, f64::from(s + w) / 2.0) })
            .collect();
        let input: Vec<Item> = if vertical { items.iter().copied().map(flip).collect() } else { items.clone() };
        let axis = if vertical { DistributeAxis::Vertical } else { DistributeAxis::Horizontal };
        let zero = Offset { dx: 0.0, dy: 0.0 };
        let got = distribute(axis, mode, &input).unwrap();
        prop_assert_eq!(got.len(), items.len());
        let along = |o: &Offset| if vertical { o.dy } else { o.dx };
        // the free ones, stably ordered by position
        let mut free: Vec<usize> = (0..items.len()).filter(|&i| !items[i].locked).collect();
        let key = |i: usize| if centres_mode { (items[i].rect.x0 + items[i].rect.x1) / 2.0 } else { items[i].rect.x0 };
        free.sort_by(|&a, &b| key(a).partial_cmp(&key(b)).unwrap());
        for (i, o) in got.iter().enumerate() {
            if items[i].locked {
                prop_assert_eq!(*o, zero);
            }
        }
        if free.len() < 3 {
            prop_assert!(got.iter().all(|o| *o == zero));
        } else {
            prop_assert_eq!(along(&got[free[0]]), 0.0);
            prop_assert_eq!(along(&got[*free.last().unwrap()]), 0.0);
            let moved = |i: usize| (items[i].rect.x0 + along(&got[i]), items[i].rect.x1 + along(&got[i]));
            let steps: Vec<f64> = free
                .windows(2)
                .map(|w| {
                    let (a, b) = (moved(w[0]), moved(w[1]));
                    if centres_mode { (b.0 + b.1) / 2.0 - (a.0 + a.1) / 2.0 } else { b.0 - a.1 }
                })
                .collect();
            for s in &steps {
                prop_assert!((s - steps[0]).abs() <= 0.01, "{:?}", steps);
            }
            // the other axis never changes
            for o in &got {
                prop_assert_eq!(if vertical { o.dx } else { o.dy }, 0.0);
            }
        }
    }
}
