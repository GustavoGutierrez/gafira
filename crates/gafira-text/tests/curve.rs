//! FD-44: curved text. A line of advance `L` follows an arc of length `L`; each cluster is placed
//! by the centre of its advance and turns with the tangent.

use gafira_geom::Size;
use gafira_testkit::fonts;
use gafira_text::{
    Arc, CurvedLine, TextAlign, TextError, TextLayout, TextStyle, curve_layout, layout_box,
    layout_curved,
};

const INTER: &[u8] = fonts::INTER.bytes;
const ARABIC: &[u8] = fonts::NOTO_SANS_ARABIC.bytes;
const STACK: [&[u8]; 2] = [INTER, ARABIC];
const DPI: f64 = 96.0;
const PI: f64 = std::f64::consts::PI;

fn style(align: TextAlign) -> TextStyle {
    TextStyle {
        size_pt: 12.0,
        align,
        ..TextStyle::default()
    }
}

fn lay(text: &str, width: f64, align: TextAlign) -> TextLayout {
    layout_box(text, &STACK, &style(align), DPI, Size::new(width, 1000.0)).expect("valid input")
}

fn curve(layout: &TextLayout, width: f64, bend: f64) -> Vec<CurvedLine> {
    curve_layout(layout, width, bend).expect("valid input")
}

/// θ for a bend: 1.8 degrees for each unit (D-26).
fn theta(bend: f64) -> f64 {
    bend * 1.8_f64.to_radians()
}

fn start_of(line: &gafira_text::Line) -> f64 {
    line.clusters
        .iter()
        .map(|c| f64::from(c.x))
        .fold(f64::MAX, f64::min)
}

fn dist(a: (f64, f64), b: (f64, f64)) -> f64 {
    ((a.0 - b.0).powi(2) + (a.1 - b.1).powi(2)).sqrt()
}

/// The longest line, the first of equals: the one that sets the radius.
fn reference(layout: &TextLayout) -> usize {
    let longest = layout.lines.iter().map(|l| l.advance).fold(0.0, f32::max);
    assert!(longest > 0.0, "a line with advance");
    layout
        .lines
        .iter()
        .position(|l| l.advance == longest)
        .expect("a longest line")
}

const TEXTS: [&str; 5] = [
    "Hello curved world",
    "abc \u{627}\u{628}\u{62c} def",
    "first line\nsecond, a longer line\nthird",
    "ab cd ef gh ij kl mn op qr st uv wx yz ab cd ef",
    "W",
];

#[test]
fn fd_44_a_bend_zero_is_the_straight_text_and_computes_no_radius() {
    for text in TEXTS.iter().copied().chain(["", "\n", "a\n\nb"]) {
        let layout = lay(text, 120.0, TextAlign::Justify);
        for bend in [0.0, -0.0] {
            let lines = curve(&layout, 120.0, bend);
            assert_eq!(lines.len(), layout.lines.len());
            for (curved, line) in lines.iter().zip(&layout.lines) {
                assert_eq!(curved.arc, None, "{text:?}");
                assert_eq!(curved.clusters.len(), line.clusters.len());
                for (p, c) in curved.clusters.iter().zip(&line.clusters) {
                    let x = f64::from(c.x) + f64::from(c.advance) / 2.0;
                    assert_eq!(p.x.to_bits(), x.to_bits(), "{text:?}");
                    assert_eq!(p.y.to_bits(), f64::from(line.baseline).to_bits());
                    assert_eq!(p.rotation.to_bits(), 0.0_f64.to_bits());
                }
            }
        }
    }
}

#[test]
fn fd_44_d_the_chord_at_100_is_two_l_over_pi() {
    for text in TEXTS {
        for (width, align) in [
            (120.0, TextAlign::Left),
            (400.0, TextAlign::Right),
            (60.0, TextAlign::Justify),
        ] {
            let layout = lay(text, width, align);
            let r = reference(&layout);
            let l = f64::from(layout.lines[r].advance);
            for bend in [100.0, -100.0] {
                let lines = curve(&layout, width, bend);
                let arc = lines[r].arc.expect("an arc");
                let chord = dist(arc.start, arc.end);
                assert!(
                    (chord - 2.0 * l / PI).abs() <= 0.01,
                    "{text:?} {bend}: chord {chord}, 2L/pi {}",
                    2.0 * l / PI
                );
                assert!(
                    (arc.radius - l / PI).abs() <= 1e-9 * l.max(1.0),
                    "radius {}",
                    arc.radius
                );
                // The arc is centred in the width of the box.
                assert!(((arc.start.0 + arc.end.0) / 2.0 - width / 2.0).abs() <= 1e-9);
                assert!((arc.center.0 - width / 2.0).abs() <= 1e-9);
            }
        }
    }
}

#[test]
fn fd_44_d_the_chord_and_radius_follow_the_bend() {
    let layout = lay(TEXTS[0], 300.0, TextAlign::Left);
    let l = f64::from(layout.lines[0].advance);
    for bend in [100.0, 75.5, 50.0, 1.0, 0.01, -0.01, -33.0, -100.0] {
        let arc = curve(&layout, 300.0, bend)[0].arc.expect("an arc");
        let t = theta(bend).abs();
        assert!(
            (arc.radius - l / t).abs() <= 1e-9 * (l / t),
            "{bend}: radius {}",
            arc.radius
        );
        let chord = 2.0 * (l / t) * (t / 2.0).sin();
        assert!((dist(arc.start, arc.end) - chord).abs() <= 0.01, "{bend}");
    }
}

#[test]
fn fd_44_d_a_cluster_at_s_turns_by_s_over_l_minus_a_half_times_theta() {
    for text in TEXTS {
        for align in [TextAlign::Left, TextAlign::Justify, TextAlign::Center] {
            let layout = lay(text, 110.0, align);
            for bend in [100.0, 50.0, 1.0, 0.01, -0.01, -50.0, -100.0, 33.3] {
                let lines = curve(&layout, 110.0, bend);
                for (curved, line) in lines.iter().zip(&layout.lines) {
                    if line.advance <= 0.0 {
                        continue;
                    }
                    let l = f64::from(line.advance);
                    let x0 = start_of(line);
                    for (p, c) in curved.clusters.iter().zip(&line.clusters) {
                        let s = f64::from(c.x) + f64::from(c.advance) / 2.0 - x0;
                        let expected = (s / l - 0.5) * theta(bend);
                        assert!(
                            (p.rotation - expected).abs() <= 1e-6,
                            "{text:?} bend {bend}: {} instead of {expected}",
                            p.rotation
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn fd_44_d_plus_and_minus_turn_opposite_ways_and_open_up_and_down() {
    let layout = lay("Mirror, mirror", 200.0, TextAlign::Left);
    let up = curve(&layout, 200.0, 50.0);
    let down = curve(&layout, 200.0, -50.0);
    let baseline = f64::from(layout.lines[0].baseline);
    let (a, b) = (up[0].arc.expect("arc"), down[0].arc.expect("arc"));
    // Positive is open downwards: the centre is below the text; negative, above.
    assert!(a.center.1 > baseline && b.center.1 < baseline);
    for (p, q) in up[0].clusters.iter().zip(&down[0].clusters) {
        assert!(
            (p.rotation + q.rotation).abs() <= 1e-12,
            "{} and {}",
            p.rotation,
            q.rotation
        );
        assert!((p.x - q.x).abs() <= 1e-9);
        // The middle of the arc is the highest point of the first and the lowest of the second.
        assert!(((p.y - baseline) + (q.y - baseline)).abs() <= 1e-9);
        assert!(p.y >= baseline - 1e-9 && q.y <= baseline + 1e-9);
    }
    // Left clusters turn counter-clockwise (negative) in an arch and clockwise in a cup.
    assert!(up[0].clusters[0].rotation < 0.0 && up[0].clusters.last().expect("c").rotation > 0.0);
    assert!(
        down[0].clusters[0].rotation > 0.0 && down[0].clusters.last().expect("c").rotation < 0.0
    );
}

#[test]
fn fd_44_each_cluster_sits_on_the_circle_at_the_angle_of_its_centre() {
    let layout = lay("Around the circle we go", 260.0, TextAlign::Center);
    for bend in [100.0_f64, 25.0, -60.0] {
        let line = &layout.lines[0];
        let curved = &curve(&layout, 260.0, bend)[0];
        let Arc {
            radius,
            center,
            start,
            end,
        } = curved.arc.expect("arc");
        let sigma = bend.signum();
        // The ends are the clusters' fraction 0 and 1, on the same circle.
        assert!((dist(start, center) - radius).abs() <= 1e-6);
        assert!((dist(end, center) - radius).abs() <= 1e-6);
        let l = f64::from(line.advance);
        let x0 = start_of(line);
        for (p, c) in curved.clusters.iter().zip(&line.clusters) {
            assert!(
                (dist((p.x, p.y), center) - radius).abs() <= 1e-6,
                "bend {bend}"
            );
            let f = (f64::from(c.x) + f64::from(c.advance) / 2.0 - x0) / l;
            let alpha = (f - 0.5) * theta(bend);
            assert!((p.x - (center.0 + sigma * radius * alpha.sin())).abs() <= 1e-6);
            assert!((p.y - (center.1 - sigma * radius * alpha.cos())).abs() <= 1e-6);
        }
    }
}

#[test]
fn fd_44_lines_use_concentric_arcs_with_the_same_theta() {
    let text = "first line\nsecond, a longer line\nthird";
    let layout = lay(text, 300.0, TextAlign::Left);
    assert_eq!(layout.lines.len(), 3);
    for bend in [100.0_f64, 40.0, -40.0, -100.0] {
        let sigma = bend.signum();
        let lines = curve(&layout, 300.0, bend);
        let r = reference(&layout);
        assert_eq!(r, 1, "the second line is the longest");
        let first = lines[r].arc.expect("arc");
        let b0 = f64::from(layout.lines[r].baseline);
        for (i, curved) in lines.iter().enumerate() {
            let arc = curved.arc.expect("arc");
            let b = f64::from(layout.lines[i].baseline);
            assert!(dist(arc.center, first.center) <= 1e-9, "one centre");
            // Each line sits on its baseline at the middle of its arc, one line spacing nearer to
            // the centre in an arch and further in a cup.
            assert!((arc.radius - (first.radius - sigma * (b - b0))).abs() <= 1e-9);
            assert!(
                (arc.center.1 - sigma * arc.radius - b).abs() <= 1e-9,
                "apex on the baseline"
            );
            // The same angle: the cluster rotations follow the line's own advance.
            let line = &layout.lines[i];
            let (p, c) = (
                curved.clusters.last().expect("p"),
                line.clusters.last().expect("c"),
            );
            let f = (f64::from(c.x) + f64::from(c.advance) / 2.0 - start_of(line))
                / f64::from(line.advance);
            assert!((p.rotation - (f - 0.5) * theta(bend)).abs() <= 1e-6);
        }
    }
}

#[test]
fn fd_44_e_a_line_with_no_advance_does_not_change() {
    // An empty text, an empty line, a line that is only a forced break, an empty first line.
    for (text, curved_lines) in [("", 0), ("\n", 0), ("a\n\nb", 2), ("\nab", 1), ("ab\n", 1)] {
        let layout = lay(text, 100.0, TextAlign::Left);
        let lines = curve(&layout, 100.0, 70.0);
        assert_eq!(lines.len(), layout.lines.len());
        let mut arcs = 0;
        for (curved, line) in lines.iter().zip(&layout.lines) {
            if line.advance <= 0.0 {
                assert_eq!(curved.arc, None, "{text:?}");
                for (p, c) in curved.clusters.iter().zip(&line.clusters) {
                    assert_eq!(p.x, f64::from(c.x) + f64::from(c.advance) / 2.0);
                    assert_eq!(p.y, f64::from(line.baseline));
                    assert_eq!(p.rotation, 0.0);
                }
            } else {
                assert!(curved.arc.is_some());
                arcs += 1;
            }
        }
        assert_eq!(arcs, curved_lines, "{text:?}");
    }
    // The empty first line does not set the radius: the first line with advance does.
    let layout = lay("\nab", 100.0, TextAlign::Left);
    let arc = curve(&layout, 100.0, 100.0)[1].arc.expect("arc");
    let l = f64::from(layout.lines[1].advance);
    assert!((arc.radius - l / PI).abs() <= 1e-9);
    assert!((arc.center.1 - arc.radius - f64::from(layout.lines[1].baseline)).abs() <= 1e-9);
}

#[test]
fn fd_44_e_the_bend_is_checked_and_its_ends_are_in() {
    let layout = lay("Hello", 100.0, TextAlign::Left);
    let call = |bend: f64| curve_layout(&layout, 100.0, bend).err();
    let range = Some(TextError::OutOfRange {
        field: "curve.bend",
    });
    for bad in [100.01, -100.01, 1e300, -1e300, 100.000_000_1] {
        assert_eq!(call(bad), range, "{bad}");
    }
    for ok in [100.0, -100.0, 0.0, 99.99, -99.99] {
        assert_eq!(call(ok), None, "{ok}");
    }
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert_eq!(
            call(bad),
            Some(TextError::NonFinite {
                field: "curve.bend"
            })
        );
    }
    assert_eq!(
        curve_layout(&layout, f64::NAN, 10.0).err(),
        Some(TextError::NonFinite {
            field: "curve.width"
        })
    );
    // Non-finite numbers come before ranges, also across the style and the bend.
    let bad_style = TextStyle {
        size_pt: 0.0,
        ..style(TextAlign::Left)
    };
    let both = layout_curved(
        "Hi",
        &STACK,
        &bad_style,
        DPI,
        Size::new(10.0, 10.0),
        f64::NAN,
    );
    assert_eq!(
        both.err(),
        Some(TextError::NonFinite {
            field: "curve.bend"
        })
    );
    let out_of_range = layout_curved("Hi", &STACK, &bad_style, DPI, Size::new(10.0, 10.0), 200.0);
    assert_eq!(
        out_of_range.err(),
        Some(TextError::OutOfRange {
            field: "style.size_pt"
        })
    );
    let no_font = layout_curved(
        "Hi",
        &[],
        &style(TextAlign::Left),
        DPI,
        Size::new(10.0, 10.0),
        200.0,
    );
    assert_eq!(
        no_font.err(),
        range,
        "the bend is checked before the font stack"
    );
}

#[test]
fn fd_44_e_a_tiny_bend_keeps_every_position_finite() {
    for text in TEXTS {
        let layout = lay(text, 200.0, TextAlign::Left);
        for bend in [0.01, -0.01, 1e-9, -1e-300] {
            let lines = curve(&layout, 200.0, bend);
            for (curved, line) in lines.iter().zip(&layout.lines) {
                for (p, c) in curved.clusters.iter().zip(&line.clusters) {
                    assert!(
                        p.x.is_finite() && p.y.is_finite() && p.rotation.is_finite(),
                        "{bend}"
                    );
                    // Almost straight: the line, centred in the box, a hair off its baseline.
                    if bend.abs() >= 0.01 && line.advance > 0.0 {
                        // Every line spans the angle of the longest, whose length it takes.
                        let l = f64::from(line.advance);
                        let longest = f64::from(layout.lines[reference(&layout)].advance);
                        let f = (f64::from(c.x) + f64::from(c.advance) / 2.0 - start_of(line)) / l;
                        let centred = (f - 0.5) * longest + 100.0;
                        assert!(p.rotation.abs() <= theta(bend).abs() / 2.0 + 1e-12);
                        assert!(
                            (p.x - centred).abs() <= 0.01,
                            "{text:?} {bend}: {} {centred} l={l} c={c:?}",
                            p.x
                        );
                        assert!((p.y - f64::from(line.baseline)).abs() <= 0.01);
                    }
                }
            }
        }
    }
}

#[test]
fn arq_110_a_bend_too_small_for_a_radius_is_never_a_nan() {
    let layout = lay("Hello world", 200.0, TextAlign::Left);
    for bend in [5e-324, -5e-324, 1e-310, 1e-306] {
        match curve_layout(&layout, 200.0, bend) {
            Ok(lines) => {
                for p in lines.iter().flat_map(|l| &l.clusters) {
                    assert!(
                        p.x.is_finite() && p.y.is_finite() && p.rotation.is_finite(),
                        "{bend}"
                    );
                }
            }
            Err(e) => assert_eq!(e, TextError::NonFinite { field: "output" }, "{bend}"),
        }
    }
    // A bend that is not 0 but gives a theta of 0 is a straight line.
    assert_eq!(curve(&layout, 200.0, 5e-324), curve(&layout, 200.0, 0.0));
}

#[test]
fn fd_44_c_a_new_text_makes_a_new_curve() {
    let style = style(TextAlign::Left);
    let run = |text: &str| {
        layout_curved(text, &STACK, &style, DPI, Size::new(300.0, 100.0), 60.0)
            .expect("valid input")
    };
    let (short, long) = (run("abc"), run("abc def ghi"));
    // The curve is the one of the new layout, not of the old.
    for (out, text) in [(&short, "abc"), (&long, "abc def ghi")] {
        let layout = layout_box(text, &STACK, &style, DPI, Size::new(300.0, 100.0)).expect("valid");
        assert_eq!(out.layout, layout);
        assert_eq!(out.lines, curve(&layout, 300.0, 60.0));
    }
    let (a, b) = (
        short.lines[0].arc.expect("arc"),
        long.lines[0].arc.expect("arc"),
    );
    let ratio = f64::from(long.layout.lines[0].advance) / f64::from(short.layout.lines[0].advance);
    assert!(
        ((b.radius / a.radius) - ratio).abs() <= 1e-9,
        "the radius follows the advance"
    );
    assert_eq!(short.lines[0].clusters.len(), 3);
    assert_eq!(long.lines[0].clusters.len(), 11);
}

#[test]
fn fd_44_one_cluster_sits_at_the_top_of_the_arc_without_turning() {
    let layout = lay("W", 100.0, TextAlign::Left);
    for bend in [100.0, -100.0, 3.0] {
        let p = &curve(&layout, 100.0, bend)[0].clusters[0];
        assert!(
            p.rotation.abs() <= 1e-12 && (p.x - 50.0).abs() <= 1e-9,
            "{bend}: {p:?}"
        );
        assert!((p.y - f64::from(layout.lines[0].baseline)).abs() <= 1e-9);
    }
}

#[test]
fn fd_44_a_negative_width_acts_as_zero() {
    let layout = lay("Hello", 0.0, TextAlign::Left);
    assert_eq!(curve(&layout, -5.0, 40.0), curve(&layout, 0.0, 40.0));
}
