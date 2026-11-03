//! FD-11 (a, d, e, f), D-28, D-29, ARQ-110 and ARQ-111 for `align`.

use std::sync::Arc;

use gafira_doc::{
    ElementBase, Node, NodeId, NodeKind, Page, PageId, PageSize, Placement, ShapeGeometry,
    ShapeNode,
};
use gafira_layout::{
    AlignAction, Item, LayoutError, Length, Offset, Rect, Unit, align, node_bounds,
};
use proptest::prelude::*;

const BAD: [f64; 3] = [f64::NAN, f64::INFINITY, f64::NEG_INFINITY];
const ACTIONS: [AlignAction; 6] = [
    AlignAction::Left,
    AlignAction::Center,
    AlignAction::Right,
    AlignAction::Top,
    AlignAction::Middle,
    AlignAction::Bottom,
];

fn item(x0: f64, y0: f64, x1: f64, y1: f64) -> Item {
    Item {
        rect: Rect::new(x0, y0, x1, y1),
        locked: false,
    }
}

fn locked(x0: f64, y0: f64, x1: f64, y1: f64) -> Item {
    Item {
        locked: true,
        ..item(x0, y0, x1, y1)
    }
}

fn page() -> Rect {
    Rect::new(0.0, 0.0, 1000.0, 800.0)
}

fn off(dx: f64, dy: f64) -> Offset {
    Offset { dx, dy }
}

fn bits(o: &[Offset]) -> Vec<[u64; 2]> {
    o.iter().map(|o| [o.dx.to_bits(), o.dy.to_bits()]).collect()
}

/// Three boxes whose union is (-40, 5) to (130, 130), centre (45, 67.5).
fn three() -> [Item; 3] {
    [
        item(10.0, 20.0, 50.0, 60.0),
        item(30.0, 100.0, 130.0, 130.0),
        item(-40.0, 5.0, 0.0, 35.0),
    ]
}

#[test]
fn fd_11_each_action_aligns_a_selection_to_the_box_of_the_selection() {
    let items = three();
    // dx per box, then dy per box, for each action
    let cases: [(AlignAction, [f64; 3], bool); 6] = [
        (AlignAction::Left, [-50.0, -70.0, 0.0], true),
        (AlignAction::Center, [15.0, -35.0, 65.0], true),
        (AlignAction::Right, [80.0, 0.0, 130.0], true),
        (AlignAction::Top, [-15.0, -95.0, 0.0], false),
        (AlignAction::Middle, [27.5, -47.5, 47.5], false),
        (AlignAction::Bottom, [70.0, 0.0, 95.0], false),
    ];
    // the page is far from the boxes and must not matter with two or more
    let far = Rect::new(5000.0, 5000.0, 9000.0, 9000.0);
    for page in [page(), far] {
        for (action, d, horizontal) in cases {
            let got = align(action, page, &items).unwrap();
            for (o, d) in got.iter().zip(d) {
                // the other axis does not change, not even by a signed zero
                let (moved, other) = if horizontal {
                    (o.dx, o.dy)
                } else {
                    (o.dy, o.dx)
                };
                assert_eq!(moved, d, "{action:?}");
                assert_eq!(other.to_bits(), 0.0f64.to_bits(), "{action:?}");
            }
        }
    }
}

#[test]
fn fd_11_f_one_free_element_aligns_to_the_page() {
    let a = [item(10.0, 20.0, 50.0, 60.0)];
    for (action, want) in [
        (AlignAction::Left, off(-10.0, 0.0)),
        (AlignAction::Center, off(470.0, 0.0)),
        (AlignAction::Right, off(950.0, 0.0)),
        (AlignAction::Top, off(0.0, -20.0)),
        (AlignAction::Middle, off(0.0, 360.0)),
        (AlignAction::Bottom, off(0.0, 740.0)),
    ] {
        assert_eq!(align(action, page(), &a).unwrap(), [want], "{action:?}");
    }
    // away from the origin and on negative coordinates
    let p = Rect::new(-2000.0, 12340.0, -1000.0, 13140.0);
    let a = [item(-3000.0, 12000.0, -2960.0, 12040.0)];
    assert_eq!(align(AlignAction::Left, p, &a).unwrap(), [off(1000.0, 0.0)]);
    assert_eq!(
        align(AlignAction::Middle, p, &a).unwrap(),
        [off(0.0, 720.0)]
    );
}

#[test]
fn fd_11_f_an_empty_selection_or_a_single_locked_element_changes_nothing() {
    for action in ACTIONS {
        assert_eq!(align(action, page(), &[]).unwrap(), []);
        let one = [locked(10.0, 20.0, 50.0, 60.0)];
        assert_eq!(align(action, page(), &one).unwrap(), [off(0.0, 0.0)]);
    }
}

#[test]
fn d_29_locked_elements_count_in_the_box_but_never_move() {
    // two locked: nothing moves
    for action in ACTIONS {
        let two = [locked(0.0, 0.0, 10.0, 10.0), locked(50.0, 50.0, 60.0, 60.0)];
        assert_eq!(align(action, page(), &two).unwrap(), [off(0.0, 0.0); 2]);
        // three locked, four locked: still nothing
        let three = three().map(|i| Item { locked: true, ..i });
        assert_eq!(align(action, page(), &three).unwrap(), [off(0.0, 0.0); 3]);
    }
    // one free and one locked: aligned to their union, not to the page
    let a = item(10.0, 20.0, 50.0, 60.0);
    let c = locked(-40.0, 5.0, 0.0, 35.0);
    let got = align(AlignAction::Left, page(), &[a, c]).unwrap();
    assert_eq!(got, [off(-50.0, 0.0), off(0.0, 0.0)]);
    // the locked box is part of the union on the other side, too
    let b = locked(30.0, 100.0, 130.0, 130.0);
    let got = align(AlignAction::Right, page(), &[a, b]).unwrap();
    assert_eq!(got, [off(80.0, 0.0), off(0.0, 0.0)]);
    let got = align(AlignAction::Bottom, page(), &[a, b]).unwrap();
    assert_eq!(got, [off(0.0, 70.0), off(0.0, 0.0)]);
    // a locked box at the end of a longer selection
    let got = align(AlignAction::Center, page(), &[a, a, b]).unwrap();
    assert_eq!(got, [off(40.0, 0.0), off(40.0, 0.0), off(0.0, 0.0)]);
}

#[test]
fn fd_11_the_result_has_one_offset_per_box_in_the_input_order() {
    let items = [
        item(0.0, 0.0, 10.0, 10.0),
        locked(100.0, 100.0, 110.0, 110.0),
        item(30.0, 30.0, 40.0, 40.0),
        item(60.0, 60.0, 70.0, 70.0),
    ];
    let got = align(AlignAction::Left, page(), &items).unwrap();
    assert_eq!(
        got,
        [
            off(0.0, 0.0),
            off(0.0, 0.0),
            off(-30.0, 0.0),
            off(-60.0, 0.0)
        ]
    );
    // reversing the input reverses the output
    let mut rev = items;
    rev.reverse();
    let mut want = got.clone();
    want.reverse();
    assert_eq!(align(AlignAction::Left, page(), &rev).unwrap(), want);
}

#[test]
fn fd_11_f_a_rotated_element_aligns_by_its_box_aligned_to_the_axes() {
    let mut p = Page::new(
        PageId::new(1),
        String::new(),
        PageSize {
            width: Length::new(1000.0, Unit::Px),
            height: Length::new(1000.0, Unit::Px),
        },
    );
    let mut pl = Placement::new(100.0, 100.0, 100.0, 100.0);
    pl.rotation_deg = 45.0;
    let node = Node::new(
        ElementBase::new(NodeId::new(1), pl),
        NodeKind::Shape(ShapeNode::new(ShapeGeometry::Rect)),
    );
    p.nodes.insert(NodeId::new(1), Arc::new(node));
    let aabb = node_bounds(&p, NodeId::new(1)).unwrap();
    let half = 100.0 * 2f64.sqrt() / 2.0;
    assert!((aabb.x0 - (150.0 - half)).abs() <= 1e-9);
    let got = align(
        AlignAction::Left,
        Rect::new(0.0, 0.0, 1000.0, 1000.0),
        &[Item {
            rect: aabb,
            locked: false,
        }],
    )
    .unwrap();
    // the left edge of the box aligned to the axes lands on the page edge
    assert!((aabb.x0 + got[0].dx).abs() <= 1e-9);
    assert_eq!(got[0].dy, 0.0);
}

#[test]
fn fd_11_zero_size_boxes_are_valid_and_a_zero_offset_is_positive_zero() {
    let p = Rect::new(-0.0, -0.0, 0.0, 0.0);
    let got = align(AlignAction::Left, p, &[item(-0.0, -0.0, -0.0, -0.0)]).unwrap();
    assert_eq!(bits(&got), [[0.0f64.to_bits(), 0.0f64.to_bits()]]);
    // a reference at -0.0 and a box at +0.0 would subtract to -0.0
    let got = align(
        AlignAction::Left,
        Rect::new(-0.0, 0.0, 10.0, 10.0),
        &[item(0.0, 0.0, 0.0, 0.0)],
    )
    .unwrap();
    assert_eq!(bits(&got), [[0.0f64.to_bits(), 0.0f64.to_bits()]]);
    let got = align(
        AlignAction::Middle,
        Rect::new(0.0, -0.0, 10.0, 0.0),
        &[item(0.0, 0.0, 0.0, 0.0)],
    )
    .unwrap();
    assert_eq!(bits(&got), [[0.0f64.to_bits(), 0.0f64.to_bits()]]);
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

#[test]
fn arq_110_every_number_of_the_page_and_of_every_box_must_be_finite() {
    for action in ACTIONS {
        for set in setters() {
            for v in BAD {
                // the page, with an empty selection, one free, one locked, and two or more
                let mut p = page();
                set(&mut p, v);
                let page_err = Err(LayoutError::NonFinite { field: "page" });
                assert_eq!(align(action, p, &[]), page_err, "{action:?} {v}");
                assert_eq!(align(action, p, &three()), page_err);
                assert_eq!(align(action, p, &[locked(0.0, 0.0, 1.0, 1.0)]), page_err);
                // each position in the list, locked or not, first, middle or last
                for at in 0..3 {
                    for lock in [false, true] {
                        let mut items = three();
                        items[at].locked = lock;
                        set(&mut items[at].rect, v);
                        assert_eq!(
                            align(action, page(), &items),
                            Err(LayoutError::NonFinite { field: "item" }),
                            "{action:?} {v} at {at} locked {lock}"
                        );
                    }
                }
                let mut one = [item(0.0, 0.0, 1.0, 1.0)];
                set(&mut one[0].rect, v);
                assert_eq!(
                    align(action, page(), &one),
                    Err(LayoutError::NonFinite { field: "item" })
                );
            }
        }
    }
}

#[test]
fn arq_110_a_box_with_a_negative_size_is_an_error_and_a_size_of_zero_is_not() {
    for action in ACTIONS {
        let err = |field| Err(LayoutError::InvalidBox { field });
        assert_eq!(
            align(action, Rect::new(1.0, 0.0, 0.0, 0.0), &[]),
            err("page")
        );
        assert_eq!(
            align(action, Rect::new(0.0, 1.0, 0.0, 0.0), &three()),
            err("page")
        );
        for lock in [false, true] {
            let mut items = three();
            items[1].locked = lock;
            items[1].rect = Rect::new(5.0, 0.0, 4.0, 0.0);
            assert_eq!(align(action, page(), &items), err("item"));
            items[1].rect = Rect::new(0.0, 5.0, 0.0, 4.9);
            assert_eq!(align(action, page(), &items), err("item"));
        }
        assert!(
            align(
                action,
                Rect::new(2.0, 2.0, 2.0, 2.0),
                &[item(1.0, 1.0, 1.0, 1.0)]
            )
            .is_ok()
        );
    }
    // every number is checked before any box: a NaN after an inverted box still wins
    let items = [item(5.0, 0.0, 4.0, 0.0), item(f64::NAN, 0.0, 0.0, 0.0)];
    assert_eq!(
        align(AlignAction::Left, page(), &items),
        Err(LayoutError::NonFinite { field: "item" })
    );
}

#[test]
fn arq_110_a_centre_or_an_offset_that_overflows_is_an_error_even_if_the_action_ignores_it() {
    for action in ACTIONS {
        // a centre that overflows, on the page, on a free box and on a locked one
        let wide = Rect::new(f64::MAX, f64::MAX, f64::MAX, f64::MAX);
        assert_eq!(
            align(action, wide, &[]),
            Err(LayoutError::NonFinite { field: "page" })
        );
        for lock in [false, true] {
            let items = [
                item(0.0, 0.0, 1.0, 1.0),
                Item {
                    rect: wide,
                    locked: lock,
                },
            ];
            assert_eq!(
                align(action, page(), &items),
                Err(LayoutError::NonFinite { field: "item" })
            );
        }
    }
    // an offset that overflows: the union starts at -MAX, the free box lies at MAX / 4
    let items = [
        locked(-f64::MAX, 0.0, f64::MAX / 8.0, 1.0),
        item(f64::MAX / 4.0, 0.0, f64::MAX / 4.0 + 1e290, 1.0),
    ];
    assert_eq!(
        align(AlignAction::Left, page(), &items),
        Err(LayoutError::NonFinite { field: "offset" })
    );
}

#[test]
fn arq_110_a_centre_that_is_finite_is_not_an_error_however_large_the_box() {
    // The sum of the two edges lies between MAX / 2 and MAX: the centre is finite, but twice
    // the sum is not. Checked on each axis and on every action.
    let big = (0.3 * f64::MAX, 0.4 * f64::MAX);
    let x = Rect::new(big.0, 0.0, big.1, 1.0);
    let y = Rect::new(0.0, big.0, 1.0, big.1);
    for action in ACTIONS {
        for rect in [x, y] {
            let items = [Item { rect, locked: true }];
            assert!(align(action, page(), &items).is_ok(), "{action:?} {rect:?}");
        }
    }
}

// ---------------------------------------------------------------------------
// ARQ-111 and the property

#[test]
fn arq_111_the_same_input_gives_the_same_bytes() {
    let mut zeros = three();
    zeros[0].rect = Rect::new(-0.0, 0.0, 0.0, 0.0);
    let cases: [(Vec<Item>, Rect); 3] = [
        (three().to_vec(), page()),
        (vec![item(10.0, 20.0, 50.0, 60.0)], page()),
        (zeros.to_vec(), Rect::new(-0.0, 0.0, 5.0, 5.0)),
    ];
    for (items, p) in &cases {
        for action in ACTIONS {
            let first = bits(&align(action, *p, items).unwrap());
            for _ in 0..3 {
                assert_eq!(bits(&align(action, *p, items).unwrap()), first);
            }
        }
    }
}

proptest! {
    /// After the move, every free edge of the action sits on the reference, the locked
    /// boxes did not move and the other axis did not change.
    #[test]
    fn fd_11_a_free_edges_land_on_the_reference_and_the_other_axis_is_unchanged(
        raw in prop::collection::vec((-4000i32..4000, -4000i32..4000, 0i32..2000, 0i32..2000, any::<bool>()), 2..8),
        which in 0usize..6,
    ) {
        let items: Vec<Item> = raw
            .iter()
            .map(|&(x, y, w, h, l)| Item {
                rect: Rect::new(f64::from(x) / 2.0, f64::from(y) / 2.0, f64::from(x + w) / 2.0, f64::from(y + h) / 2.0),
                locked: l,
            })
            .collect();
        let action = ACTIONS[which];
        let got = align(action, page(), &items).unwrap();
        prop_assert_eq!(got.len(), items.len());
        // the reference is the box of all of them
        let ux0 = items.iter().map(|i| i.rect.x0).fold(f64::INFINITY, f64::min);
        let ux1 = items.iter().map(|i| i.rect.x1).fold(f64::NEG_INFINITY, f64::max);
        let uy0 = items.iter().map(|i| i.rect.y0).fold(f64::INFINITY, f64::min);
        let uy1 = items.iter().map(|i| i.rect.y1).fold(f64::NEG_INFINITY, f64::max);
        for (o, i) in got.iter().zip(&items) {
            let moved = Rect::new(i.rect.x0 + o.dx, i.rect.y0 + o.dy, i.rect.x1 + o.dx, i.rect.y1 + o.dy);
            if i.locked {
                prop_assert_eq!(*o, off(0.0, 0.0));
                continue;
            }
            let (edge, want, other) = match action {
                AlignAction::Left => (moved.x0, ux0, o.dy),
                AlignAction::Right => (moved.x1, ux1, o.dy),
                AlignAction::Center => ((moved.x0 + moved.x1) / 2.0, (ux0 + ux1) / 2.0, o.dy),
                AlignAction::Top => (moved.y0, uy0, o.dx),
                AlignAction::Bottom => (moved.y1, uy1, o.dx),
                AlignAction::Middle => ((moved.y0 + moved.y1) / 2.0, (uy0 + uy1) / 2.0, o.dx),
            };
            prop_assert!((edge - want).abs() <= 0.01);
            prop_assert_eq!(other, 0.0);
        }
    }
}
