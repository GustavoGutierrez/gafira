//! FD-06 (a, b, d to h), D-16, D-28, ARQ-110 and ARQ-111 for `snap`.

use gafira_layout::{
    AxisSnap, GridSettings, GridStyle, Guide, GuideAxis, LayoutError, Length, Rect, SnapInput,
    SnapKind, SnapResult, SnapTarget, Unit, snap,
};
use proptest::prelude::*;

const BAD: [f64; 3] = [f64::NAN, f64::INFINITY, f64::NEG_INFINITY];
const NONE: AxisSnap = AxisSnap {
    offset: 0.0,
    target: None,
};

#[derive(Clone)]
struct Case {
    moving: Rect,
    page: Rect,
    margins: Option<Rect>,
    elements: Vec<Rect>,
    guides: Vec<Guide>,
    guides_visible: bool,
    grid: GridSettings,
    active: bool,
    zoom: f64,
    threshold: f64,
    dpi: f64,
}

fn grid(visible: bool, snap: bool, step: f64, subdivisions: u8) -> GridSettings {
    GridSettings {
        visible,
        step: Length::new(step, Unit::Px),
        subdivisions,
        style: GridStyle::Lines,
        snap,
    }
}

fn vg(x: f64) -> Guide {
    Guide {
        axis: GuideAxis::Vertical,
        position: Length::new(x, Unit::Px),
        locked: false,
    }
}

fn hg(y: f64) -> Guide {
    Guide {
        axis: GuideAxis::Horizontal,
        position: Length::new(y, Unit::Px),
        locked: false,
    }
}

fn flip(r: Rect) -> Rect {
    Rect::new(r.y0, r.x0, r.y1, r.x1)
}

impl Case {
    /// A zero-size box at the origin, with every other source of targets far away or off.
    fn new() -> Self {
        Self {
            moving: Rect::new(0.0, 0.0, 0.0, 0.0),
            page: Rect::new(1e6, 1e6, 2e6, 2e6),
            margins: None,
            elements: Vec::new(),
            guides: Vec::new(),
            guides_visible: true,
            grid: grid(false, true, 10.0, 1),
            active: true,
            zoom: 1.0,
            threshold: 6.0,
            dpi: 96.0,
        }
    }

    fn at(x: f64) -> Self {
        let mut c = Self::new();
        c.moving = Rect::new(x, 0.0, x, 0.0);
        c
    }

    fn run(&self) -> Result<SnapResult, LayoutError> {
        snap(&SnapInput {
            moving: self.moving,
            page: self.page,
            margins: self.margins,
            elements: &self.elements,
            guides: &self.guides,
            guides_visible: self.guides_visible,
            grid: &self.grid,
            active: self.active,
            zoom: self.zoom,
            threshold: self.threshold,
            dpi: self.dpi,
        })
    }

    /// The same case with x and y exchanged everywhere.
    fn flipped(&self) -> Self {
        let mut c = self.clone();
        c.moving = flip(self.moving);
        c.page = flip(self.page);
        c.margins = self.margins.map(flip);
        c.elements = self.elements.iter().copied().map(flip).collect();
        for g in &mut c.guides {
            g.axis = match g.axis {
                GuideAxis::Vertical => GuideAxis::Horizontal,
                GuideAxis::Horizontal => GuideAxis::Vertical,
            };
        }
        c
    }

    /// The x axis of this case, which must equal the y axis of the flipped case. Unless the
    /// grid attracts (it has lines on both axes), the other axis has no target.
    fn solve(&self) -> AxisSnap {
        let a = self.run().unwrap();
        let b = self.flipped().run().unwrap();
        assert_eq!(a.x, b.y, "x of the case differs from y of the flipped case");
        assert_eq!(a.y, b.x, "y of the case differs from x of the flipped case");
        if !(self.grid.visible && self.grid.snap) {
            assert_eq!(a.y, NONE, "the y axis has no target in this case");
        }
        a.x
    }

    /// Puts one target of `kind` on x = `c`; everything else of that kind lies far away.
    fn put(&mut self, kind: SnapKind, c: f64) {
        match kind {
            SnapKind::Guide => self.guides.push(vg(c)),
            SnapKind::Margin => self.margins = Some(Rect::new(c, 1e5, c + 1e5, 2e5)),
            SnapKind::Page => self.page = Rect::new(c, 1e6, c + 1e6, 2e6),
            SnapKind::Element => self
                .elements
                .push(Rect::new(c, 3e5, c + 1000.0, 3e5 + 10.0)),
            SnapKind::Grid => {
                assert!(
                    c % 10.0 == 0.0,
                    "grid lines of step 10 sit on multiples of 10"
                );
                self.grid = grid(true, true, 10.0, 1);
            }
        }
    }
}

fn target(coordinate: f64, kind: SnapKind) -> Option<SnapTarget> {
    Some(SnapTarget { coordinate, kind })
}

fn bits(r: &SnapResult) -> [u64; 4] {
    [
        r.x.offset.to_bits(),
        r.y.offset.to_bits(),
        r.x.target.map_or(0, |t| t.coordinate.to_bits()),
        r.y.target.map_or(0, |t| t.coordinate.to_bits()),
    ]
}

// ---------------------------------------------------------------------------
// FD-06 (a), (b), (h)

#[test]
fn fd_06_a_five_px_from_a_guide_the_edge_lands_on_it() {
    let mut c = Case::new();
    c.moving = Rect::new(20.0, 30.0, 70.0, 80.0);
    c.guides = vec![vg(25.0), hg(88.0)];
    let r = c.run().unwrap();
    // left edge 20 -> 25; bottom edge 80 -> 88 is 8 away, so y stays
    assert!((c.moving.x0 + r.x.offset - 25.0).abs() <= 1e-9);
    assert_eq!(r.x.target, target(25.0, SnapKind::Guide));
    assert_eq!(r.y, NONE);
    c.guides = vec![vg(25.0), hg(85.0)];
    let r = c.run().unwrap();
    assert!((c.moving.y1 + r.y.offset - 85.0).abs() <= 1e-9);
    assert_eq!(r.y.target, target(85.0, SnapKind::Guide));
}

#[test]
fn fd_06_b_an_inactive_magnet_gives_no_offset_but_still_checks_its_input() {
    let mut c = Case::new();
    c.active = false;
    c.guides = vec![vg(1.0)];
    assert_eq!(c.run().unwrap(), SnapResult { x: NONE, y: NONE });
    c.guides = vec![vg(f64::NAN)];
    assert_eq!(c.run(), Err(LayoutError::NonFinite { field: "guide" }));
    // the ranges are checked too
    c.guides = vec![];
    c.zoom = 33.0;
    assert_eq!(c.run(), Err(LayoutError::OutOfRange { field: "zoom" }));
    c.zoom = 1.0;
    c.threshold = 0.0;
    assert_eq!(c.run(), Err(LayoutError::OutOfRange { field: "threshold" }));
    c.threshold = 6.0;
    c.moving = Rect::new(1.0, 0.0, 0.0, 0.0);
    assert_eq!(c.run(), Err(LayoutError::InvalidBox { field: "moving" }));
    // active again, the same guide snaps
    c.moving = Rect::new(0.0, 0.0, 0.0, 0.0);
    c.guides = vec![vg(1.0)];
    c.active = true;
    assert_eq!(c.solve().target, target(1.0, SnapKind::Guide));
}

#[test]
fn fd_06_h_without_targets_both_offsets_are_zero() {
    let mut c = Case::at(-300.0);
    c.moving = Rect::new(-300.0, 40.0, -200.0, 90.0);
    assert_eq!(c.run().unwrap(), SnapResult { x: NONE, y: NONE });
    // targets that exist but lie beyond the threshold change nothing
    c.guides = vec![vg(-293.5), hg(97.0)];
    assert_eq!(c.run().unwrap(), SnapResult { x: NONE, y: NONE });
    // the offset of a no-target axis is +0.0, never -0.0
    let r = c.run().unwrap();
    assert_eq!(r.x.offset.to_bits(), 0.0f64.to_bits());
    assert_eq!(r.y.offset.to_bits(), 0.0f64.to_bits());
}

// ---------------------------------------------------------------------------
// FD-06 (e) and the threshold row: exact comparison of `distance <= threshold / zoom`

/// The offset on x of a zero-size box at `r` with one guide at `g`, or `None` without a snap.
fn guide_hit(r: f64, g: f64, zoom: f64, threshold: f64) -> Option<f64> {
    let mut c = Case::at(r);
    c.guides = vec![vg(g)];
    c.zoom = zoom;
    c.threshold = threshold;
    let res = c.run().unwrap();
    res.x.target.map(|_| res.x.offset)
}

#[test]
fn fd_06_e_distance_6_snaps_and_6_01_does_not() {
    assert_eq!(guide_hit(100.0, 106.0, 1.0, 6.0), Some(6.0));
    assert_eq!(guide_hit(100.0, 106.01, 1.0, 6.0), None);
    assert_eq!(guide_hit(100.0, 94.0, 1.0, 6.0), Some(-6.0));
    assert_eq!(guide_hit(100.0, 93.99, 1.0, 6.0), None);
    // zoom 200 %: 3 snaps, 3.01 does not
    assert_eq!(guide_hit(1000.0, 1003.0, 2.0, 6.0), Some(3.0));
    assert_eq!(guide_hit(1000.0, 1003.01, 2.0, 6.0), None);
    assert_eq!(guide_hit(-1000.0, -1003.0, 2.0, 6.0), Some(-3.0));
    assert_eq!(guide_hit(-1000.0, -1003.01, 2.0, 6.0), None);
}

#[test]
fn fd_06_the_threshold_is_compared_exactly_and_divided_by_the_zoom() {
    // zoom 0.9: the page threshold is 6 / 0.9 computed in f64
    let q = 6.0 / 0.9;
    assert_eq!(guide_hit(0.0, q, 0.9, 6.0), Some(q));
    assert_eq!(guide_hit(0.0, q.next_up(), 0.9, 6.0), None);
    assert_eq!(guide_hit(0.0, -q, 0.9, 6.0), Some(-q));
    assert_eq!(guide_hit(0.0, -q.next_up(), 0.9, 6.0), None);
    // the same distance away from the origin: the reference is -q, the guide on 0
    assert_eq!(guide_hit(-q, 0.0, 0.9, 6.0), Some(q));
    assert_eq!(guide_hit(-q.next_up(), 0.0, 0.9, 6.0), None);
    // 6.5 is under the page threshold 6.67 but over the screen threshold 6
    assert_eq!(guide_hit(0.0, 6.5, 0.9, 6.0), Some(6.5));
    // zoom 2: 4 is under the screen threshold 6 but over the page threshold 3
    assert_eq!(guide_hit(0.0, 4.0, 2.0, 6.0), None);
    assert_eq!(guide_hit(0.0, 3.0, 2.0, 6.0), Some(3.0));
    assert_eq!(guide_hit(0.0, 3.0f64.next_up(), 2.0, 6.0), None);
    // away from the origin, where the subtraction is exact: threshold = distance
    let r = 1000.0;
    let t = 1000.0 + 0.1;
    let d = t - r;
    assert_eq!(guide_hit(r, t, 1.0, d), Some(d));
    assert_eq!(guide_hit(r, t, 1.0, d.next_down()), None);
    // thresholds the spec names: 0.5 and 20 are valid
    assert_eq!(guide_hit(0.0, 0.5, 1.0, 0.5), Some(0.5));
    assert_eq!(guide_hit(0.0, 20.0, 1.0, 20.0), Some(20.0));
}

// ---------------------------------------------------------------------------
// The checks of the threshold, zoom and DPI rows

#[test]
fn threshold_must_be_finite_and_above_zero() {
    for t in [0.0, -1.0, -0.0, -f64::MIN_POSITIVE] {
        let mut c = Case::new();
        c.threshold = t;
        assert_eq!(
            c.run(),
            Err(LayoutError::OutOfRange { field: "threshold" }),
            "{t}"
        );
    }
    for t in [0.5, 20.0, f64::MIN_POSITIVE, 1e300] {
        let mut c = Case::new();
        c.threshold = t;
        assert!(c.run().is_ok(), "{t}");
    }
    // the quotient overflows
    let mut c = Case::new();
    c.threshold = 1e308;
    c.zoom = 0.05;
    assert_eq!(c.run(), Err(LayoutError::NonFinite { field: "threshold" }));
}

#[test]
fn zoom_range_is_closed_and_checked_without_targets_and_with_the_magnet_off() {
    for active in [true, false] {
        for z in [0.05, 32.0, 1.0] {
            let mut c = Case::new();
            c.active = active;
            c.zoom = z;
            assert!(c.run().is_ok(), "{z} {active}");
        }
        for z in [
            0.049,
            33.0,
            0.05f64.next_down(),
            32.0f64.next_up(),
            0.0,
            -1.0,
        ] {
            let mut c = Case::new();
            c.active = active;
            c.zoom = z;
            assert_eq!(
                c.run(),
                Err(LayoutError::OutOfRange { field: "zoom" }),
                "{z} {active}"
            );
        }
    }
}

#[test]
fn dpi_must_be_above_zero_even_when_nothing_uses_it() {
    for dpi in [0.0, -96.0, -0.0] {
        let mut c = Case::new();
        c.dpi = dpi;
        assert_eq!(
            c.run(),
            Err(LayoutError::OutOfRange { field: "dpi" }),
            "{dpi}"
        );
    }
    let mut c = Case::new();
    c.dpi = f64::MIN_POSITIVE;
    assert!(c.run().is_ok());
}

#[test]
fn the_order_of_checks_is_numbers_and_boxes_then_dpi_then_ranges() {
    let dpi0 = || {
        let mut c = Case::new();
        c.dpi = 0.0;
        c
    };
    let mut c = dpi0();
    c.guides = vec![vg(f64::NAN)];
    assert_eq!(c.run(), Err(LayoutError::NonFinite { field: "guide" }));
    let mut c = dpi0();
    c.grid.step = Length::new(f64::NAN, Unit::Px);
    assert_eq!(c.run(), Err(LayoutError::NonFinite { field: "step" }));
    let mut c = dpi0();
    c.moving = Rect::new(f64::NAN, 0.0, 0.0, 0.0);
    assert_eq!(c.run(), Err(LayoutError::NonFinite { field: "moving" }));
    let mut c = dpi0();
    c.elements = vec![Rect::new(f64::NAN, 0.0, 0.0, 0.0)];
    assert_eq!(c.run(), Err(LayoutError::NonFinite { field: "element" }));
    let mut c = dpi0();
    c.moving = Rect::new(1.0, 0.0, 0.0, 0.0);
    assert_eq!(c.run(), Err(LayoutError::InvalidBox { field: "moving" }));
    // a guide that does not attract is checked in the same pass
    let mut c = dpi0();
    c.guides_visible = false;
    c.guides = vec![vg(f64::NAN)];
    assert_eq!(c.run(), Err(LayoutError::NonFinite { field: "guide" }));
    // every number is checked before any box: a NaN after an inverted box still wins
    let mut c = Case::new();
    c.moving = Rect::new(1.0, 0.0, 0.0, 0.0);
    c.page = Rect::new(f64::NAN, 0.0, 0.0, 0.0);
    assert_eq!(c.run(), Err(LayoutError::NonFinite { field: "page" }));
    // after the dpi rule come the ranges
    for set in [
        (|c: &mut Case| c.zoom = 33.0) as fn(&mut Case),
        |c| c.threshold = 0.0,
        |c| c.grid.subdivisions = 0,
        |c| c.grid.step = Length::new(0.5, Unit::Px),
    ] {
        let mut c = dpi0();
        set(&mut c);
        assert_eq!(c.run(), Err(LayoutError::OutOfRange { field: "dpi" }));
    }
}

#[test]
fn grid_ranges_are_checked_also_when_the_grid_does_not_attract() {
    let bad: [(GridSettings, &str); 5] = [
        (grid(true, true, 500.01, 1), "step"),
        (grid(false, false, 0.99, 1), "step"),
        (grid(false, true, 10.0, 0), "subdivisions"),
        (grid(true, false, 10.0, 11), "subdivisions"),
        (grid(false, false, 0.0, 1), "step"),
    ];
    for (g, field) in bad {
        for active in [true, false] {
            let mut c = Case::new();
            c.grid = g;
            c.active = active;
            assert_eq!(c.run(), Err(LayoutError::OutOfRange { field }), "{field}");
        }
    }
    // the ends are valid
    for g in [grid(false, false, 1.0, 1), grid(true, true, 500.0, 10)] {
        let mut c = Case::new();
        c.grid = g;
        assert!(c.run().is_ok());
    }
    // a step that is 0 on screen
    let mut c = Case::new();
    c.grid.step = Length::new(1.0, Unit::In);
    c.grid.visible = false;
    c.dpi = 5e-324;
    c.zoom = 0.05;
    assert_eq!(c.run(), Err(LayoutError::OutOfRange { field: "step" }));
}

// ---------------------------------------------------------------------------
// ARQ-110: every float input, intermediates included

type Setter = fn(&mut Case, f64);

fn float_inputs() -> Vec<(&'static str, Setter)> {
    let mut v: Vec<(&'static str, Setter)> = Vec::new();
    let moving: [Setter; 4] = [
        |c, x| c.moving.x0 = x,
        |c, x| c.moving.y0 = x,
        |c, x| c.moving.x1 = x,
        |c, x| c.moving.y1 = x,
    ];
    for s in moving {
        v.push(("moving", s));
    }
    let page: [Setter; 4] = [
        |c, x| c.page.x0 = x,
        |c, x| c.page.y0 = x,
        |c, x| c.page.x1 = x,
        |c, x| c.page.y1 = x,
    ];
    for s in page {
        v.push(("page", s));
    }
    let margins: [Setter; 4] = [
        |c, x| c.margins.as_mut().unwrap().x0 = x,
        |c, x| c.margins.as_mut().unwrap().y0 = x,
        |c, x| c.margins.as_mut().unwrap().x1 = x,
        |c, x| c.margins.as_mut().unwrap().y1 = x,
    ];
    for s in margins {
        v.push(("margins", s));
    }
    let elements: [Setter; 4] = [
        |c, x| c.elements[1].x0 = x,
        |c, x| c.elements[1].y0 = x,
        |c, x| c.elements[1].x1 = x,
        |c, x| c.elements[1].y1 = x,
    ];
    for s in elements {
        v.push(("element", s));
    }
    v.push(("guide", |c, x| {
        c.guides[1].position = Length::new(x, Unit::Px)
    }));
    v.push(("guide", |c, x| {
        c.guides[2].position = Length::new(x, Unit::In)
    }));
    v.push(("step", |c, x| c.grid.step = Length::new(x, Unit::Px)));
    v.push(("zoom", |c, x| c.zoom = x));
    v.push(("threshold", |c, x| c.threshold = x));
    v.push(("dpi", |c, x| c.dpi = x));
    v
}

/// A valid case in which most inputs cannot change the result.
fn rich() -> Case {
    let mut c = Case::new();
    c.moving = Rect::new(0.0, 0.0, 10.0, 10.0);
    c.page = Rect::new(-100.0, -100.0, 1000.0, 1000.0);
    c.margins = Some(Rect::new(10.0, 10.0, 900.0, 900.0));
    c.elements = vec![
        Rect::new(300.0, 300.0, 400.0, 400.0),
        Rect::new(5000.0, 5000.0, 5100.0, 5100.0),
    ];
    c.guides = vec![vg(5.0), vg(9000.0), vg(2.0)];
    c.guides[2].position = Length::new(100.0, Unit::In);
    c.grid = grid(true, true, 10.0, 2);
    c
}

#[test]
fn arq_110_every_float_input_rejects_non_finite_values_naming_its_field() {
    let inputs = float_inputs();
    assert_eq!(inputs.len(), 4 * 4 + 6);
    for active in [true, false] {
        for guides_visible in [true, false] {
            for (field, set) in &inputs {
                for v in BAD {
                    let mut c = rich();
                    c.active = active;
                    c.guides_visible = guides_visible;
                    set(&mut c, v);
                    assert_eq!(
                        c.run(),
                        Err(LayoutError::NonFinite { field }),
                        "{field} = {v}, active {active}, guides visible {guides_visible}"
                    );
                }
            }
        }
    }
    // The step counts when the grid does not attract; so does a hidden guide.
    for g in [grid(false, false, 10.0, 1), grid(true, false, 10.0, 1)] {
        let mut c = rich();
        c.grid = g;
        c.grid.step = Length::new(f64::NAN, Unit::Px);
        assert_eq!(c.run(), Err(LayoutError::NonFinite { field: "step" }));
    }
    // Without margins there is nothing to check there.
    let mut c = rich();
    c.margins = None;
    assert!(c.run().is_ok());
}

#[test]
fn arq_110_a_result_that_is_not_finite_is_an_error_even_if_the_value_would_be_finite() {
    // the distance from -MAX to MAX is +inf
    let mut c = Case::new();
    c.moving = Rect::new(-f64::MAX, 0.0, 0.0, 0.0);
    c.guides = vec![vg(f64::MAX)];
    assert_eq!(c.run(), Err(LayoutError::NonFinite { field: "distance" }));
    // the same pair with the guides hidden: the guide does not attract, so no distance
    c.guides_visible = false;
    assert!(c.run().is_ok());
    // a centre that overflows
    let mut c = Case::new();
    c.moving = Rect::new(f64::MAX, 0.0, f64::MAX, 1.0);
    assert_eq!(c.run(), Err(LayoutError::NonFinite { field: "moving" }));
    let mut c = Case::new();
    c.page = Rect::new(f64::MAX, 0.0, f64::MAX, 1.0);
    assert_eq!(c.run(), Err(LayoutError::NonFinite { field: "page" }));
    let mut c = Case::new();
    c.elements = vec![Rect::new(f64::MAX, 0.0, f64::MAX, 1.0)];
    assert_eq!(c.run(), Err(LayoutError::NonFinite { field: "element" }));
    // a margin rectangle has no centre
    let mut c = Case::new();
    c.margins = Some(Rect::new(f64::MAX, 0.0, f64::MAX, 1.0));
    assert!(c.run().is_ok());
    // a guide in inches that overflows when converted, also when it does not attract
    for visible in [true, false] {
        let mut c = Case::new();
        c.guides_visible = visible;
        c.guides = vec![Guide {
            axis: GuideAxis::Vertical,
            position: Length::new(f64::MAX, Unit::In),
            locked: false,
        }];
        assert_eq!(c.run(), Err(LayoutError::NonFinite { field: "guide" }));
    }
}

#[test]
fn arq_110_boxes_with_a_negative_size_are_errors_and_a_size_of_zero_is_valid() {
    let cases: [(&str, Setter); 4] = [
        ("moving", |c, _| c.moving = Rect::new(2.0, 0.0, 1.0, 0.0)),
        ("page", |c, _| c.page = Rect::new(0.0, 2.0, 0.0, 1.0)),
        ("margins", |c, _| {
            c.margins = Some(Rect::new(2.0, 0.0, 1.0, 0.0));
        }),
        ("element", |c, _| {
            c.elements.push(Rect::new(0.0, 0.0, 0.0, -1e-300));
        }),
    ];
    for active in [true, false] {
        for (field, set) in cases {
            let mut c = Case::new();
            c.active = active;
            set(&mut c, 0.0);
            assert_eq!(c.run(), Err(LayoutError::InvalidBox { field }), "{field}");
        }
    }
    let mut c = Case::new();
    c.margins = Some(Rect::new(3.0, 3.0, 3.0, 3.0));
    c.elements = vec![Rect::new(7.0, 7.0, 7.0, 7.0)];
    assert!(c.run().is_ok());
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(512))]

    #[test]
    fn arq_110_finite_input_gives_finite_output_or_an_error(
        m in prop::array::uniform4(prop::num::f64::NORMAL | prop::num::f64::ZERO),
        g in prop::num::f64::NORMAL | prop::num::f64::ZERO,
        e in prop::array::uniform4(prop::num::f64::NORMAL | prop::num::f64::ZERO),
        zoom in 0.05f64..32.0,
        threshold in 0.001f64..1e300,
    ) {
        let mut c = Case::new();
        c.moving = Rect::new(m[0].min(m[2]), m[1].min(m[3]), m[0].max(m[2]), m[1].max(m[3]));
        c.guides = vec![vg(g), hg(g)];
        c.elements = vec![Rect::new(e[0].min(e[2]), e[1].min(e[3]), e[0].max(e[2]), e[1].max(e[3]))];
        c.zoom = zoom;
        c.threshold = threshold;
        if let Ok(r) = c.run() {
            for a in [r.x, r.y] {
                prop_assert!(a.offset.is_finite());
                if let Some(t) = a.target {
                    prop_assert!(t.coordinate.is_finite());
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// ARQ-111

#[test]
fn arq_111_the_same_input_gives_the_same_bytes() {
    // distinct results on the two axes, plus -0.0 and zero offsets, so a sign or an
    // order that changes between calls cannot cancel out
    let mut a = rich();
    a.moving = Rect::new(2.5, -0.0, 12.5, 7.25);
    let mut b = Case::at(-0.0);
    b.guides = vec![vg(0.0), vg(-0.0)];
    let mut c = Case::at(0.0);
    c.guides = vec![vg(-0.0)];
    let mut d = rich();
    d.moving = Rect::new(-37.0, 411.0, 21.0, 450.0);
    d.elements.push(Rect::new(-40.0, 405.0, -39.0, 409.0));
    for case in [a, b, c, d] {
        let runs: Vec<_> = (0..4).map(|_| bits(&case.run().unwrap())).collect();
        assert!(runs.windows(2).all(|w| w[0] == w[1]), "{runs:?}");
    }
    // a zero offset is +0.0 even when the target is -0.0 or the reference is
    let mut r = Case::at(0.0);
    r.guides = vec![vg(-0.0)];
    assert_eq!(r.run().unwrap().x.offset.to_bits(), 0.0f64.to_bits());
}

// ---------------------------------------------------------------------------
// D-16: which target wins

/// The kinds in D-16 priority order, best first.
const ORDER: [SnapKind; 5] = [
    SnapKind::Guide,
    SnapKind::Margin,
    SnapKind::Page,
    SnapKind::Element,
    SnapKind::Grid,
];
/// The kinds that are boxes or lines of the page, which can sit anywhere.
const FREE: [SnapKind; 4] = [
    SnapKind::Guide,
    SnapKind::Margin,
    SnapKind::Page,
    SnapKind::Element,
];

#[test]
fn fd_06_f_at_equal_distance_each_kind_beats_every_lower_one() {
    for (i, &hi) in ORDER.iter().enumerate() {
        for &lo in &ORDER[i + 1..] {
            for base in [-2000.0, 0.0, 12340.0] {
                for first in [hi, lo] {
                    for dir in [1.0, -1.0] {
                        let second = if first == hi { lo } else { hi };
                        if second == SnapKind::Grid {
                            continue;
                        }
                        let mut c = Case::at(base + dir * 3.0);
                        c.put(first, base);
                        c.put(second, base + dir * 6.0);
                        let (won, at) = if hi == first {
                            (hi, base)
                        } else {
                            (hi, base + dir * 6.0)
                        };
                        let got = c.solve();
                        assert_eq!(
                            got.target,
                            target(at, won),
                            "{hi:?} over {lo:?}, first {first:?} at {base}, dir {dir}"
                        );
                        assert_eq!(got.offset, at - (base + dir * 3.0));
                    }
                }
            }
        }
    }
}

#[test]
fn fd_06_f_a_nearer_target_of_a_lower_kind_beats_a_farther_higher_one() {
    for (i, &hi) in FREE.iter().enumerate() {
        for &lo in &FREE[i + 1..] {
            for base in [-2000.0, 0.0, 12340.0] {
                for dir in [1.0, -1.0] {
                    let mut c = Case::at(base);
                    c.put(hi, base + dir * 3.0);
                    c.put(lo, base - dir * 2.0);
                    let got = c.solve();
                    assert_eq!(got.target, target(base - dir * 2.0, lo), "{hi:?} {lo:?}");
                    assert_eq!(got.offset, -dir * 2.0);
                }
            }
        }
    }
}

#[test]
fn fd_06_after_the_kind_the_smaller_coordinate_wins() {
    for kind in FREE {
        for base in [-2000.0, 0.0, 12340.0] {
            let mut c = Case::at(base);
            // the same kind on both sides at the same distance: the smaller coordinate
            match kind {
                SnapKind::Guide => c.guides = vec![vg(base + 2.0), vg(base - 2.0)],
                SnapKind::Element => {
                    c.elements = vec![
                        Rect::new(base + 2.0, 3e5, base + 100.0, 3e5 + 1.0),
                        Rect::new(base - 2.0, 3e5, base + 100.0, 3e5 + 1.0),
                    ];
                }
                SnapKind::Margin => {
                    c.margins = Some(Rect::new(base - 2.0, 1e5, base + 2.0, 2e5));
                }
                // the centre of this page sits on base + 2, as far as the right edge's neighbour
                _ => c.page = Rect::new(base - 2.0, 1e6, base + 6.0, 2e6),
            }
            let got = c.solve();
            assert_eq!(got.target, target(base - 2.0, kind), "{kind:?}");
            assert_eq!(got.offset, -2.0);
        }
    }
}

#[test]
fn fd_06_after_the_coordinate_the_reference_order_is_left_centre_right() {
    for base in [-2000.0, 0.0, 12340.0] {
        // box [8, 12] has its centre on 10: a guide on 9 is 1 from the left edge and 1 from the centre
        let mut c = Case::new();
        c.moving = Rect::new(base + 8.0, 0.0, base + 12.0, 0.0);
        c.guides = vec![vg(base + 9.0)];
        let got = c.solve();
        assert_eq!(got.target, target(base + 9.0, SnapKind::Guide));
        assert_eq!(got.offset, 1.0, "left before centre");
        // a guide on 11 is 1 from the centre and 1 from the right edge
        c.guides = vec![vg(base + 11.0)];
        let got = c.solve();
        assert_eq!(got.offset, 1.0, "centre before right");
        // the centre alone: a guide on 10 is 0 away
        c.guides = vec![vg(base + 10.0)];
        assert_eq!(c.solve().offset, 0.0);
    }
}

#[test]
fn fd_06_each_reference_reaches_targets_on_its_own() {
    for base in [-2000.0, 0.0, 12340.0] {
        let mut c = Case::new();
        c.moving = Rect::new(base, 0.0, base + 100.0, 0.0);
        for (guide, offset) in [
            (-4.0, -4.0),
            (54.0, 4.0),
            (104.0, 4.0),
            (47.0, -3.0),
            (95.0, -5.0),
        ] {
            c.guides = vec![vg(base + guide)];
            let got = c.solve();
            assert_eq!(got.offset, offset, "guide at {guide}");
            assert_eq!(got.target, target(base + guide, SnapKind::Guide));
        }
        // 20 from every reference
        c.guides = vec![vg(base + 20.0)];
        assert_eq!(c.solve(), NONE);
    }
}

#[test]
fn fd_06_g_each_axis_is_resolved_on_its_own() {
    let mut c = Case::new();
    c.moving = Rect::new(100.0, 200.0, 140.0, 260.0);
    // x snaps to a guide, y to an element, in the same call
    c.guides = vec![vg(103.0)];
    c.elements = vec![Rect::new(500.0, 195.0, 600.0, 400.0)];
    let r = c.run().unwrap();
    assert_eq!(r.x.target, target(103.0, SnapKind::Guide));
    assert_eq!(r.x.offset, 3.0);
    assert_eq!(r.y.target, target(195.0, SnapKind::Element));
    assert_eq!(r.y.offset, -5.0);
    // a vertical guide never attracts y, nor a horizontal one x
    let mut c = Case::new();
    c.moving = Rect::new(100.0, 200.0, 140.0, 260.0);
    c.guides = vec![vg(202.0), hg(102.0)];
    assert_eq!(c.run().unwrap(), SnapResult { x: NONE, y: NONE });
    // y references are y values: the x edge 100 is 3 from a horizontal guide on 103
    c.guides = vec![hg(103.0)];
    assert_eq!(c.run().unwrap(), SnapResult { x: NONE, y: NONE });
    c.guides = vec![hg(203.0)];
    assert_eq!(c.run().unwrap().y.offset, 3.0);
}

// ---------------------------------------------------------------------------
// The sources of targets

#[test]
fn the_page_attracts_by_both_edges_and_the_centre() {
    for base in [-2000.0, 0.0, 12340.0] {
        for (ref_x, want) in [(3.0, 0.0), (497.0, 500.0), (1003.0, 1000.0), (497.5, 500.0)] {
            let mut c = Case::at(base + ref_x);
            c.page = Rect::new(base, 1e6, base + 1000.0, 2e6);
            let got = c.solve();
            assert_eq!(got.target, target(base + want, SnapKind::Page), "{ref_x}");
            assert_eq!(got.offset, want - ref_x);
        }
    }
}

#[test]
fn an_element_attracts_by_both_edges_and_the_centre() {
    for base in [-2000.0, 0.0, 12340.0] {
        for (ref_x, want) in [(-3.0, 0.0), (97.0, 100.0), (203.0, 200.0)] {
            let mut c = Case::at(base + ref_x);
            c.elements = vec![
                Rect::new(base, 3e5, base + 200.0, 3e5 + 1.0),
                Rect::new(base + 50000.0, 3e5, base + 50001.0, 3e5 + 1.0),
            ];
            let got = c.solve();
            assert_eq!(
                got.target,
                target(base + want, SnapKind::Element),
                "{ref_x}"
            );
        }
    }
    // an empty list of elements is valid
    assert_eq!(Case::new().solve(), NONE);
}

#[test]
fn the_margin_rectangle_attracts_by_its_two_edges_and_not_by_its_centre() {
    for base in [-2000.0, 0.0, 12340.0] {
        for (ref_x, want) in [(53.0, 50.0), (947.0, 950.0)] {
            let mut c = Case::at(base + ref_x);
            c.margins = Some(Rect::new(base + 50.0, 1e5, base + 950.0, 2e5));
            let got = c.solve();
            assert_eq!(got.target, target(base + want, SnapKind::Margin));
        }
        let mut c = Case::at(base + 500.0);
        c.margins = Some(Rect::new(base + 50.0, 1e5, base + 950.0, 2e5));
        assert_eq!(c.solve(), NONE);
    }
}

#[test]
fn d_16_zero_margins_give_the_margin_kind_and_no_margin_rectangle_the_page_kind() {
    let page = Rect::new(0.0, 0.0, 1000.0, 800.0);
    let mut c = Case::new();
    c.moving = Rect::new(1.0, 100.0, 50.0, 200.0);
    c.page = page;
    c.margins = Some(page);
    let r = c.run().unwrap();
    assert_eq!(r.x.target, target(0.0, SnapKind::Margin));
    assert_eq!(r.x.offset, -1.0);
    // no rectangle: no error, the same edge is a page edge
    c.margins = None;
    let r = c.run().unwrap();
    assert_eq!(r.x.target, target(0.0, SnapKind::Page));
    // the page centre is a page target even when the margin rectangle equals the page
    c.margins = Some(page);
    c.moving = Rect::new(499.0, 100.0, 499.0, 100.0);
    assert_eq!(c.run().unwrap().x.target, target(500.0, SnapKind::Page));
    // and a guide 1 px from the edge still wins over the page when it is as near
    c.margins = None;
    c.moving = Rect::new(1.0, 100.0, 50.0, 200.0);
    c.guides = vec![vg(2.0)];
    let r = c.run().unwrap();
    assert_eq!(r.x.target, target(2.0, SnapKind::Guide));
    assert_eq!(r.x.offset, 1.0);
}

#[test]
fn guides_convert_to_px_with_the_dpi_and_hidden_ones_do_not_attract() {
    let mut c = Case::new();
    c.moving = Rect::new(93.0, 0.0, 93.0, 0.0);
    c.guides = vec![Guide {
        axis: GuideAxis::Vertical,
        position: Length::new(1.0, Unit::In),
        locked: false,
    }];
    let got = c.solve();
    assert_eq!(got.target, target(96.0, SnapKind::Guide));
    assert_eq!(got.offset, 3.0);
    // the same inch at another dpi is another position
    c.dpi = 72.0;
    c.moving = Rect::new(70.0, 0.0, 70.0, 0.0);
    let got = c.solve();
    assert_eq!(got.target, target(72.0, SnapKind::Guide));
    assert_eq!(got.offset, 2.0);
    c.moving = Rect::new(93.0, 0.0, 93.0, 0.0);
    // visibility off: no snap, and a locked guide attracts like any other
    c.dpi = 96.0;
    c.guides_visible = false;
    assert_eq!(c.run().unwrap().x, NONE);
    c.guides_visible = true;
    c.guides[0].locked = true;
    assert_eq!(c.run().unwrap().x.offset, 3.0);
}

// ---------------------------------------------------------------------------
// Properties

proptest! {
    #![proptest_config(ProptestConfig::with_cases(512))]

    #[test]
    fn fd_06_d_the_result_never_moves_more_than_the_threshold(
        m in prop::array::uniform4(-500.0f64..500.0),
        gs in prop::collection::vec(-600.0f64..600.0, 0..6),
        es in prop::collection::vec(prop::array::uniform4(-600.0f64..600.0), 0..4),
        zoom in 0.05f64..32.0,
        threshold in 2.0f64..16.0,
        step in 1.0f64..500.0,
        sub in 1u8..=10,
    ) {
        let mut c = Case::new();
        c.moving = Rect::new(m[0].min(m[2]), m[1].min(m[3]), m[0].max(m[2]), m[1].max(m[3]));
        c.page = Rect::new(-50.0, -50.0, 550.0, 450.0);
        c.guides = gs.iter().flat_map(|&g| [vg(g), hg(g)]).collect();
        c.elements = es.iter().map(|e| Rect::new(e[0].min(e[2]), e[1].min(e[3]), e[0].max(e[2]), e[1].max(e[3]))).collect();
        c.zoom = zoom;
        c.threshold = threshold;
        c.grid = grid(true, true, step, sub);
        let r = c.run().unwrap();
        let limit = threshold / zoom;
        for (a, refs) in [
            (r.x, [c.moving.x0, (c.moving.x0 + c.moving.x1) / 2.0, c.moving.x1]),
            (r.y, [c.moving.y0, (c.moving.y0 + c.moving.y1) / 2.0, c.moving.y1]),
        ] {
            prop_assert!(a.offset.abs() <= limit);
            match a.target {
                None => prop_assert_eq!(a.offset, 0.0),
                Some(t) => prop_assert!(
                    refs.iter().any(|&x| (x + a.offset - t.coordinate).abs() <= 1e-9 * (1.0 + t.coordinate.abs()))
                ),
            }
        }
    }

    /// A plain model of the rule: the smallest (distance, kind, coordinate, reference).
    #[test]
    fn d_16_the_winner_is_the_smallest_distance_kind_coordinate_and_reference(
        m in prop::array::uniform2(0i32..40),
        gs in prop::collection::vec(0i32..40, 0..5),
        es in prop::collection::vec(prop::array::uniform2(0i32..40), 0..4),
        mg in prop::option::of(prop::array::uniform2(0i32..40)),
        pg in prop::array::uniform2(0i32..40),
        threshold in 1i32..12,
    ) {
        let (x0, x1) = (f64::from(m[0].min(m[1])), f64::from(m[0].max(m[1])));
        let mut c = Case::new();
        c.moving = Rect::new(x0, 0.0, x1, 0.0);
        c.guides = gs.iter().map(|&g| vg(f64::from(g))).collect();
        c.elements = es.iter().map(|e| Rect::new(f64::from(e[0].min(e[1])), 3e5, f64::from(e[0].max(e[1])), 3e5 + 1.0)).collect();
        c.margins = mg.map(|g| Rect::new(f64::from(g[0].min(g[1])), 1e5, f64::from(g[0].max(g[1])), 2e5));
        c.page = Rect::new(f64::from(pg[0].min(pg[1])), 1e6, f64::from(pg[0].max(pg[1])), 2e6);
        c.threshold = f64::from(threshold);
        let refs = [x0, (x0 + x1) / 2.0, x1];
        let mut targets: Vec<(SnapKind, f64)> = gs.iter().map(|&g| (SnapKind::Guide, f64::from(g))).collect();
        if let Some(g) = mg {
            targets.push((SnapKind::Margin, f64::from(g[0].min(g[1]))));
            targets.push((SnapKind::Margin, f64::from(g[0].max(g[1]))));
        }
        let (p0, p1) = (f64::from(pg[0].min(pg[1])), f64::from(pg[0].max(pg[1])));
        targets.extend([(SnapKind::Page, p0), (SnapKind::Page, (p0 + p1) / 2.0), (SnapKind::Page, p1)]);
        for e in &es {
            let (a, b) = (f64::from(e[0].min(e[1])), f64::from(e[0].max(e[1])));
            targets.extend([(SnapKind::Element, a), (SnapKind::Element, (a + b) / 2.0), (SnapKind::Element, b)]);
        }
        let mut best: Option<((f64, SnapKind, f64, usize), f64)> = None;
        for (ri, &r) in refs.iter().enumerate() {
            for &(kind, t) in &targets {
                let d = (t - r).abs();
                if d > f64::from(threshold) {
                    continue;
                }
                let key = (d, kind, t, ri);
                let better = best.is_none_or(|(b, _)| {
                    key.0 < b.0
                        || (key.0 == b.0 && (key.1 < b.1
                            || (key.1 == b.1 && (key.2 < b.2 || (key.2 == b.2 && key.3 < b.3)))))
                });
                if better {
                    best = Some((key, t - r));
                }
            }
        }
        let got = c.run().unwrap().x;
        match best {
            None => prop_assert_eq!(got, NONE),
            Some((key, offset)) => {
                prop_assert_eq!(got.target, target(key.2, key.1));
                prop_assert_eq!(got.offset, offset);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// The grid as a target (FD-06, FD-07, "Destino en la Cuadrícula")

fn with_grid(step: f64, sub: u8, zoom: f64, at: f64) -> Case {
    let mut c = Case::at(at);
    c.grid = grid(true, true, step, sub);
    c.zoom = zoom;
    c
}

/// The grid target of a zero-size box at `at`, or `None`.
fn grid_hit(step: f64, sub: u8, zoom: f64, at: f64) -> Option<f64> {
    let got = with_grid(step, sub, zoom, at).solve();
    got.target.map(|t| {
        assert_eq!(t.kind, SnapKind::Grid);
        assert_eq!(got.offset, t.coordinate - at);
        t.coordinate
    })
}

#[test]
fn fd_06_the_grid_attracts_to_the_nearest_line_on_either_side_and_a_tie_goes_down() {
    assert_eq!(grid_hit(10.0, 1, 1.0, 14.0), Some(10.0));
    assert_eq!(grid_hit(10.0, 1, 1.0, 15.0), Some(10.0));
    assert_eq!(grid_hit(10.0, 1, 1.0, -14.0), Some(-10.0));
    // the line above
    assert_eq!(grid_hit(10.0, 1, 1.0, 16.0), Some(20.0));
    assert_eq!(grid_hit(10.0, 1, 1.0, 18.0), Some(20.0));
    assert_eq!(grid_hit(10.0, 1, 1.0, -16.0), Some(-20.0));
    // negative ties go to the smaller, more negative line
    assert_eq!(grid_hit(10.0, 1, 1.0, -15.0), Some(-20.0));
    assert_eq!(grid_hit(10.0, 1, 1.0, -25.0), Some(-30.0));
    assert_eq!(grid_hit(10.0, 1, 1.0, 25.0), Some(20.0));
    // on a line, and next to it, away from the origin
    assert_eq!(grid_hit(10.0, 1, 1.0, 5000.0), Some(5000.0));
    assert_eq!(grid_hit(10.0, 1, 1.0, 5004.5), Some(5000.0));
    assert_eq!(grid_hit(10.0, 1, 1.0, -5004.5), Some(-5000.0));
    // beyond the threshold of 6 screen px there is no snap
    assert_eq!(grid_hit(20.0, 1, 1.0, 7.0), None);
    assert_eq!(grid_hit(20.0, 1, 1.0, 6.0), Some(0.0));
    // -0.0 is a reference on the line 0
    let got = with_grid(10.0, 1, 1.0, -0.0).solve();
    assert_eq!(got.offset.to_bits(), 0.0f64.to_bits());
}

#[test]
fn fd_06_the_grid_follows_the_levels_that_grid_lines_shows() {
    // five subdivisions: at 100 % only the major lines, so 13 is 3 from 10 and not 1 from 12
    assert_eq!(grid_hit(10.0, 5, 1.0, 13.0), Some(10.0));
    // at 500 % the minor lines show every 2 px; the threshold is 1.2 page units
    assert_eq!(grid_hit(10.0, 5, 5.0, 3.2), Some(4.0));
    assert_eq!(grid_hit(10.0, 5, 5.0, 1.1), Some(2.0));
    assert_eq!(grid_hit(10.0, 5, 5.0, 0.9), Some(0.0));
    // 5 is as far from 4 as from 6: the smaller line
    assert_eq!(grid_hit(10.0, 5, 5.0, 5.0), Some(4.0));
    // 24.9 %: neither level fits, the step becomes 10 * 2^2 = 40
    assert_eq!(grid_hit(10.0, 5, 0.249, 25.0), Some(40.0));
    // a zoom under 25 % shows no minor lines even if they would fit
    assert_eq!(grid_hit(500.0, 10, 0.2, 40.0), None);
    assert_eq!(grid_hit(500.0, 10, 0.2, 480.0), Some(500.0));
    // the first zoom with minor lines
    assert_eq!(grid_hit(100.0, 2, 0.25, 40.0), Some(50.0));
    assert_eq!(grid_hit(100.0, 2, 0.2499, 40.0), None);
    // very fine grid at the largest page: no error, no view limit
    let mut c = with_grid(1.0, 1, 32.0, 16000.01);
    c.page = Rect::new(0.0, 0.0, 16384.0, 16384.0);
    assert_eq!(c.solve().target, target(16000.0, SnapKind::Grid));
}

#[test]
fn fd_06_a_major_line_is_exactly_k_times_the_step() {
    // 11 * 4.8 has its own bits; 33 * 4.8 / 3 can differ
    let want: f64 = 11.0 * 4.8;
    let got = grid_hit(4.8, 3, 4.0, 52.81).unwrap();
    assert_eq!(got.to_bits(), want.to_bits());
    // minor lines are k * step / subdivisions
    let minor = grid_hit(4.8, 3, 4.0, 3.3).unwrap();
    assert_eq!(minor.to_bits(), (2.0f64 * 4.8 / 3.0).to_bits());
    // negative k
    let neg = grid_hit(4.8, 3, 4.0, -52.81).unwrap();
    assert_eq!(neg.to_bits(), (-11.0f64 * 4.8).to_bits());
}

#[test]
fn fd_06_the_grid_step_is_converted_with_the_dpi() {
    let mut c = with_grid(1.0, 1, 1.0, 100.0);
    c.grid.step = Length::new(1.0, Unit::In);
    assert_eq!(c.solve().target, target(96.0, SnapKind::Grid));
    c.dpi = 72.0;
    c.moving = Rect::new(70.0, 0.0, 70.0, 0.0);
    assert_eq!(c.solve().target, target(72.0, SnapKind::Grid));
    // millimetres
    let mut c = with_grid(1.0, 1, 1.0, 0.0);
    c.grid.step = Length::new(25.4, Unit::Mm);
    c.moving = Rect::new(97.0, 0.0, 97.0, 0.0);
    assert_eq!(c.solve().target.unwrap().kind, SnapKind::Grid);
}

#[test]
fn fd_06_the_grid_attracts_only_when_visible_and_set_to_snap() {
    for (visible, snap_on, active, want) in [
        (true, true, true, true),
        (false, true, true, false),
        (true, false, true, false),
        (false, false, true, false),
        (true, true, false, false),
    ] {
        let mut c = with_grid(10.0, 1, 1.0, 14.0);
        c.grid.visible = visible;
        c.grid.snap = snap_on;
        c.active = active;
        let got = c.run().unwrap().x.target;
        assert_eq!(got.is_some(), want, "{visible} {snap_on} {active}");
    }
}

#[test]
fn fd_06_f_the_grid_loses_a_tie_to_every_other_kind_and_wins_when_nearer() {
    for hi in FREE {
        for base in [-2000.0, 0.0, 12340.0] {
            for dir in [1.0, -1.0] {
                // equal distance: the grid line on `base`, the other target 6 away on the far side
                let mut c = Case::at(base + dir * 3.0);
                c.put(SnapKind::Grid, base);
                c.put(hi, base + dir * 6.0);
                let got = c.solve();
                assert_eq!(got.target, target(base + dir * 6.0, hi), "{hi:?}");
                // the grid 2 away, the other 3 away: the grid is nearer and wins
                let mut c = Case::at(base + dir * 2.0);
                c.put(SnapKind::Grid, base);
                c.put(hi, base + dir * 5.0);
                let got = c.solve();
                assert_eq!(got.target, target(base, SnapKind::Grid), "{hi:?}");
                assert_eq!(got.offset, -dir * 2.0);
            }
        }
    }
}

#[test]
fn fd_06_the_grid_is_reached_from_each_reference_of_the_box() {
    // lines every 10; the reference nearest to any line moves, whichever it is
    for base in [-2000.0, 0.0, 12340.0] {
        for (x0, x1, offset, line) in [
            (13.0, 19.0, 1.0, 20.0),  // the right edge is 1 from 20
            (3.0, 17.0, 0.0, 10.0),   // the centre is on 10
            (11.0, 17.0, -1.0, 10.0), // the left edge is 1 from 10
            (13.0, 25.0, 1.0, 20.0),  // the centre 19 is 1 from 20 and the left 13 is 3 from 10
            (11.0, 25.0, -1.0, 10.0), // the left edge is 1 from 10, the centre 18 is 2 from 20
        ] {
            let mut c = with_grid(10.0, 1, 1.0, 0.0);
            c.moving = Rect::new(base + x0, 0.0, base + x1, 0.0);
            let got = c.solve();
            assert_eq!(got.offset, offset, "{x0} {x1}");
            assert_eq!(got.target, target(base + line, SnapKind::Grid));
        }
    }
}

/// The nearest line of the grid_lines output around `at`, with a tie going down.
fn nearest_in_view(g: &GridSettings, zoom: f64, at: f64) -> Option<f64> {
    let half = 1920.0 / zoom;
    let view = Rect::new(at - half, 0.0, at + half, 1.0);
    let lines = gafira_layout::grid::grid_lines(g, view, zoom, 96.0)
        .unwrap()
        .vertical;
    let below = lines
        .iter()
        .map(|l| l.position)
        .filter(|&p| p <= at)
        .fold(None, |m: Option<f64>, p| Some(m.map_or(p, |m| m.max(p))));
    let above = lines
        .iter()
        .map(|l| l.position)
        .filter(|&p| p >= at)
        .fold(None, |m: Option<f64>, p| Some(m.map_or(p, |m| m.min(p))));
    match (below, above) {
        (Some(b), Some(a)) => Some(if at - b <= a - at { b } else { a }),
        // A line on one side only: any line on the other side is more than half a view away.
        (Some(p), None) | (None, Some(p)) => Some(p),
        (None, None) => None,
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1024))]

    /// The snap target is the line grid_lines would draw nearest to the reference.
    #[test]
    fn fd_06_the_grid_target_has_the_bits_of_the_nearest_line_of_grid_lines(
        step in prop_oneof![(1u32..=500).prop_map(f64::from), 1.0f64..500.0],
        sub in 1u8..=10,
        zoom in prop_oneof![
            0.05f64..32.0,
            Just(0.25), Just(0.2499), Just(0.249), Just(1.0), Just(32.0), Just(0.05),
        ],
        around in -5000.0f64..5000.0,
        midpoint in any::<bool>(),
        threshold in 2.0f64..16.0,
    ) {
        let g = grid(true, true, step, sub);
        // half the cases put the reference exactly between two drawn lines, to hit ties
        let at = if midpoint {
            let half = 1920.0 / zoom;
            let lines = gafira_layout::grid::grid_lines(&g, Rect::new(around - half, 0.0, around + half, 1.0), zoom, 96.0).unwrap().vertical;
            match lines.windows(2).nth(lines.len().saturating_sub(2) / 2) {
                Some(w) => (w[0].position + w[1].position) / 2.0,
                None => around,
            }
        } else {
            around
        };
        let mut c = Case::at(at);
        c.grid = g;
        c.zoom = zoom;
        c.threshold = threshold;
        let got = c.run().unwrap().x;
        let limit = threshold / zoom;
        match nearest_in_view(&g, zoom, at) {
            Some(n) if (n - at).abs() <= limit => {
                let t = got.target.expect("a line within the threshold");
                prop_assert_eq!(t.kind, SnapKind::Grid);
                prop_assert_eq!(t.coordinate.to_bits(), n.to_bits());
            }
            _ => prop_assert_eq!(got.target, None),
        }
    }
}

#[test]
fn arq_110_the_grid_does_not_loop_over_a_huge_reference() {
    // the reference is far out; the target is computed, not searched for
    let mut c = Case::at(8e307);
    let r = c.run().unwrap();
    assert!(r.x.offset.is_finite());
    c.grid = grid(true, true, 10.0, 1);
    let r = c.run().unwrap();
    assert!(r.x.offset.is_finite());
    // a quotient that overflows is a non-finite result: 8.9e307 / 0.2
    let mut c = Case::at(8.9e307);
    c.grid = grid(true, true, 2.0, 10);
    c.zoom = 32.0;
    assert_eq!(c.run(), Err(LayoutError::NonFinite { field: "step" }));
    // the same reference with a grid that does not attract is not computed
    c.grid.visible = false;
    assert!(c.run().is_ok());
}
