//! FD-44: curved text. A line of advance `L` follows an arc of length `L`; each cluster is placed
//! by the centre of its advance and turns with the tangent.

use gafira_geom::Size;
use gafira_testkit::fonts;
use gafira_text::{
    Arc, Cluster, CurvedLine, Line, TextAlign, TextError, TextLayout, TextStyle, curve_layout,
    layout_box, layout_curved,
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

/// `L`: the advance of a line without the white space that hangs (FD-44).
fn content(line: &Line) -> f64 {
    line.clusters
        .iter()
        .filter(|c| !c.trailing)
        .map(|c| f64::from(c.advance))
        .sum()
}

/// `s` of the centre of each cluster: from the start of the content, without the advance of
/// the white space that hangs; none for that white space.
fn centres(line: &Line) -> Vec<Option<f64>> {
    let mut at = 0.0;
    line.clusters
        .iter()
        .map(|c| {
            (!c.trailing).then(|| {
                let s = at + f64::from(c.advance) / 2.0;
                at += f64::from(c.advance);
                s
            })
        })
        .collect()
}

fn dist(a: (f64, f64), b: (f64, f64)) -> f64 {
    ((a.0 - b.0).powi(2) + (a.1 - b.1).powi(2)).sqrt()
}

/// `R_ref = max_i(L_i / |theta| + sigma (b_i - b_ref))` over the lines with `L_i > 0`, with
/// `b_ref` the baseline of the first of them, and the line that gives the maximum.
fn reference(layout: &TextLayout, bend: f64) -> (f64, usize) {
    let (t, sigma) = (theta(bend).abs(), bend.signum());
    let first = layout
        .lines
        .iter()
        .find(|l| content(l) > 0.0)
        .expect("a line with content");
    let b_ref = f64::from(first.baseline);
    let mut best = (f64::MIN, 0);
    for (i, l) in layout.lines.iter().enumerate() {
        if content(l) > 0.0 {
            let r = content(l) / t + sigma * (f64::from(l.baseline) - b_ref);
            if r > best.0 {
                best = (r, i);
            }
        }
    }
    best
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
            for bend in [100.0, -100.0] {
                // (d) holds for a single line, or the line that sets the radius.
                let r = reference(&layout, bend).1;
                let l = content(&layout.lines[r]);
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
                    if content(line) <= 0.0 {
                        continue;
                    }
                    let l = content(line);
                    for (p, s) in curved.clusters.iter().zip(centres(line)) {
                        let Some(s) = s else { continue };
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
        let l = content(line);
        for (p, s) in curved.clusters.iter().zip(centres(line)) {
            assert!(
                (dist((p.x, p.y), center) - radius).abs() <= 1e-6,
                "bend {bend}"
            );
            let f = s.expect("no white space at the end") / l;
            let alpha = (f - 0.5) * theta(bend);
            assert!((p.x - (center.0 + sigma * radius * alpha.sin())).abs() <= 1e-6);
            assert!((p.y - (center.1 - sigma * radius * alpha.cos())).abs() <= 1e-6);
        }
    }
}

#[test]
fn fd_44_lines_use_concentric_arcs_with_the_same_theta() {
    // FD-44 Varias líneas: one centre, one theta, R_i = R_ref - sigma (b_i - b_ref), and
    // R_ref the maximum over the lines of L_i / |theta| + sigma (b_i - b_ref).
    for text in [
        "first line\nsecond, a longer line\nthird",
        "a longer line first\nab\nabc",
        "\nab\n \nabcdef",
        "ab\n \n \n ",
    ] {
        let layout = lay(text, 300.0, TextAlign::Left);
        for bend in [100.0_f64, 40.0, -40.0, -100.0] {
            let sigma = bend.signum();
            let lines = curve(&layout, 300.0, bend);
            let (r_ref, setter) = reference(&layout, bend);
            let b_ref = f64::from(
                layout
                    .lines
                    .iter()
                    .find(|l| content(l) > 0.0)
                    .expect("content")
                    .baseline,
            );
            let centre = (150.0, b_ref + sigma * r_ref);
            for (i, curved) in lines.iter().enumerate() {
                let line = &layout.lines[i];
                let b = f64::from(line.baseline);
                let Some(arc) = curved.arc else {
                    assert!(content(line) <= 0.0, "{text:?} line {i}");
                    continue;
                };
                assert!(dist(arc.center, centre) <= 1e-9, "{text:?}: one centre");
                assert!((arc.radius - (r_ref - sigma * (b - b_ref))).abs() <= 1e-9);
                assert!(arc.radius > 0.0, "{text:?} {bend} line {i}");
                // No line is packed below its own advance, and the setter has equality.
                let room = arc.radius * theta(bend).abs();
                assert!(room >= content(line) - 1e-9, "{text:?} {bend} line {i}");
                if i == setter {
                    assert!((room - content(line)).abs() <= 1e-9);
                }
                assert!(
                    (arc.center.1 - sigma * arc.radius - b).abs() <= 1e-9,
                    "apex on the baseline"
                );
                // The same angle: the rotations follow the line's own L.
                let last = curved.clusters.len()
                    - 1
                    - line
                        .clusters
                        .iter()
                        .rev()
                        .take_while(|c| c.trailing)
                        .count();
                let f = centres(line)[last].expect("content") / content(line);
                assert!((curved.clusters[last].rotation - (f - 0.5) * theta(bend)).abs() <= 1e-6);
            }
        }
    }
}

#[test]
fn fd_44_e_a_line_with_no_advance_does_not_change() {
    // An empty text, an empty line, a line that is only a forced break, an empty first line.
    // A line of white space that hangs, alone, has no content either.
    for (text, curved_lines) in [
        ("", 0),
        ("\n", 0),
        ("a\n\nb", 2),
        ("\nab", 1),
        ("ab\n", 1),
        (" ", 0),
        ("a\n \nb", 2),
        ("a\n   \nb", 2),
    ] {
        let layout = lay(text, 100.0, TextAlign::Left);
        let lines = curve(&layout, 100.0, 70.0);
        assert_eq!(lines.len(), layout.lines.len());
        let mut arcs = 0;
        for (curved, line) in lines.iter().zip(&layout.lines) {
            if content(line) <= 0.0 {
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
    let l = content(&layout.lines[1]);
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
                for (p, s) in curved.clusters.iter().zip(centres(line)) {
                    assert!(
                        p.x.is_finite() && p.y.is_finite() && p.rotation.is_finite(),
                        "{bend}"
                    );
                    // Almost straight: the line, centred in the box, a hair off its baseline.
                    if let (true, Some(s)) = (bend.abs() >= 0.01 && content(line) > 0.0, s) {
                        // Every line spans the angle of the longest, whose length it takes.
                        let l = content(line);
                        let longest = content(&layout.lines[reference(&layout, bend).1]);
                        let f = s / l;
                        let centred = (f - 0.5) * longest + 100.0;
                        assert!(p.rotation.abs() <= theta(bend).abs() / 2.0 + 1e-12);
                        assert!(
                            (p.x - centred).abs() <= 0.01,
                            "{text:?} {bend}: {} {centred} l={l}",
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

const WORDS: [&str; 7] = ["gafira", "text", "curve", "lorem", "ipsum", "dolor", "arc"];

fn lorem(chars: usize) -> String {
    let mut text = String::new();
    let mut i = 0;
    while text.len() < chars {
        if !text.is_empty() {
            text.push(' ');
        }
        text.push_str(WORDS[i * 3 % WORDS.len()]);
        i += 1;
    }
    text.truncate(chars);
    text
}

#[test]
fn fd_44_f_no_line_of_an_ordinary_box_has_a_radius_of_zero_or_less() {
    // 600 characters, 12 pt, 96 DPI, a box of 300 px: 17 lines, 9 of them mirrored by the
    // first rule (C24).
    let layout = lay(&lorem(600), 300.0, TextAlign::Left);
    assert!(
        layout.lines.len() >= 15,
        "many lines: {}",
        layout.lines.len()
    );
    for bend in [100.0, -100.0] {
        let lines = curve(&layout, 300.0, bend);
        let mut centres_seen: Vec<(f64, f64)> = Vec::new();
        let mut equal = 0;
        for (curved, line) in lines.iter().zip(&layout.lines) {
            let Some(arc) = curved.arc else {
                assert!(content(line) <= 0.0);
                continue;
            };
            assert!(arc.radius > 0.0, "bend {bend}: radius {}", arc.radius);
            let room = arc.radius * PI;
            assert!(
                room >= content(line) - 0.01,
                "bend {bend}: an arc of {room} px for a line of {}",
                content(line)
            );
            if (room - content(line)).abs() <= 0.01 {
                equal += 1;
            }
            centres_seen.push(arc.center);
        }
        assert!(equal >= 1, "bend {bend}: one line has the equality");
        assert!(
            centres_seen
                .iter()
                .all(|c| dist(*c, centres_seen[0]) <= 1e-6),
            "bend {bend}: one centre"
        );
    }
}

/// The placement of each cluster of the first line, by the start of its text.
fn placed(text: &str, width: f64, bend: f64) -> Vec<(usize, f64, f64, f64)> {
    let layout = lay(text, width, TextAlign::Left);
    let curved = &curve(&layout, width, bend)[0];
    let mut out: Vec<_> = layout.lines[0]
        .clusters
        .iter()
        .zip(&curved.clusters)
        .map(|(c, p)| (c.range.start, p.x, p.y, p.rotation))
        .collect();
    out.sort_by_key(|p| p.0);
    out
}

#[test]
fn fd_44_g_a_space_at_the_end_does_not_move_the_glyphs_before_it() {
    for (bare, spaced) in [
        ("Hello", "Hello "),
        ("Hello", "Hello   "),
        (
            "\u{627}\u{644}\u{639}\u{631}\u{628}\u{64a}",
            "\u{627}\u{644}\u{639}\u{631}\u{628}\u{64a} ",
        ),
    ] {
        for bend in [100.0, -100.0, 37.0] {
            let (a, b) = (placed(bare, 300.0, bend), placed(spaced, 300.0, bend));
            for (p, q) in a.iter().zip(&b) {
                assert_eq!(p.0, q.0);
                assert!(
                    (p.1 - q.1).abs() <= 0.01 && (p.2 - q.2).abs() <= 0.01,
                    "{spaced:?} {bend}: {p:?} and {q:?}"
                );
                assert!(
                    (p.3 - q.3).abs() <= 1e-6,
                    "{spaced:?} {bend}: {p:?} and {q:?}"
                );
            }
        }
    }
}

#[test]
fn fd_44_d_a_wrapped_line_ignores_the_space_that_hangs_at_its_end() {
    // "Hello " wraps before "world": the space at the end of the first line is part of it.
    let wrapped = lay("Hello world", 45.0, TextAlign::Left);
    assert!(wrapped.lines[0].clusters.iter().any(|c| c.trailing));
    let bare = lay("Hello", 45.0, TextAlign::Left);
    let (a, b) = (curve(&wrapped, 45.0, 100.0), curve(&bare, 45.0, 100.0));
    // The rotation of every glyph follows s / L of "Hello" alone; the space ends the content.
    for (p, q) in a[0].clusters.iter().zip(&b[0].clusters) {
        assert_eq!(p.rotation, q.rotation);
    }
    assert!((a[0].clusters[5].rotation - theta(100.0) / 2.0).abs() <= 1e-9);
}

#[test]
fn fd_44_d_a_space_inside_the_line_that_hangs_leaves_no_gap_in_the_arc() {
    // A hanging space of the other level can be inside the line (FD-41): the glyphs after it
    // sit where they would if it were not there, and it has no place of its own.
    let mut with = lay("abc def", 300.0, TextAlign::Left);
    let without = lay("abcdef", 300.0, TextAlign::Left);
    let space = 3;
    assert_eq!(&"abc def"[with.lines[0].clusters[space].range.clone()], " ");
    with.lines[0].clusters[space].trailing = true;
    let (a, b) = (curve(&with, 300.0, 100.0), curve(&without, 300.0, 100.0));
    let (ca, cb) = (a[0].arc.expect("arc"), b[0].arc.expect("arc"));
    assert!((ca.radius - cb.radius).abs() <= 0.01);
    let others = a[0]
        .clusters
        .iter()
        .enumerate()
        .filter(|(i, _)| *i != space);
    for ((_, p), q) in others.zip(&b[0].clusters) {
        assert!((p.x - q.x).abs() <= 0.01 && (p.y - q.y).abs() <= 0.01);
        assert!((p.rotation - q.rotation).abs() <= 1e-6);
    }
    let hang = &a[0].clusters[space];
    assert!(hang.x.is_finite() && hang.y.is_finite() && hang.rotation.is_finite());
}

/// A line by hand: one cluster of `advance` at `x` on `baseline`.
fn hand_line(baseline: f32, x: f32, advance: f32) -> Line {
    Line {
        baseline,
        advance,
        height: 10.0,
        clusters: vec![Cluster {
            range: 0..1,
            x,
            advance,
            ..Cluster::default()
        }],
    }
}

fn hand_layout(lines: Vec<Line>) -> TextLayout {
    TextLayout {
        height: 10.0,
        lines,
        overflow: false,
    }
}

// S13-02, FD-44 (e): the first line with content sets the reference baseline, not an empty
// line before it, and no line at all is not the reference either. With the content far from
// the top, a reference at 0 loses `L / |theta|` to rounding (1e20 + 32 == 1e20), so only the
// right reference gives the radius of a lone line, `L / |theta|`.
#[test]
fn fd_44_e_the_reference_baseline_is_the_first_line_with_content() {
    let far = 1e20_f32;
    let empty = Line {
        baseline: 0.0,
        ..Line::default()
    };
    for lines in [
        vec![hand_line(far, 0.0, 100.0)],
        vec![empty, hand_line(far, 0.0, 100.0)],
    ] {
        let out = curve(&hand_layout(lines), 100.0, 100.0);
        let arc = out.last().expect("a line").arc.expect("arc");
        assert!(
            (arc.radius - 100.0 / PI).abs() <= 1e-9,
            "radius {}",
            arc.radius
        );
    }
}

// S13-03, ARQ-110: each non-finite coordinate of a cluster is an error on its own. With bend
// 0 every line is straight: `x` comes from the cluster's `x` and `y` from the baseline, so a
// NaN in one reaches exactly one coordinate.
#[test]
fn arq_110_a_single_non_finite_coordinate_of_a_cluster_is_an_error() {
    let output = Err(TextError::NonFinite { field: "output" });
    for (baseline, x) in [(f32::NAN, 0.0), (0.0, f32::NAN), (f32::INFINITY, 0.0)] {
        let layout = hand_layout(vec![hand_line(baseline, x, 10.0)]);
        assert_eq!(curve_layout(&layout, 100.0, 0.0), output, "{baseline} {x}");
    }
}

// S13-03, ARQ-110: a straight line with a bad cluster is an error even though its arc (none)
// is fine, next to a curved line whose arc and clusters are finite.
#[test]
fn arq_110_a_bad_cluster_on_a_straight_line_is_an_error_beside_a_good_arc() {
    let mut bad = hand_line(20.0, f32::NAN, 0.0);
    bad.clusters[0].advance = 0.0;
    let layout = hand_layout(vec![hand_line(10.0, 0.0, 10.0), bad]);
    assert_eq!(
        curve_layout(&layout, 100.0, 50.0),
        Err(TextError::NonFinite { field: "output" })
    );
}
