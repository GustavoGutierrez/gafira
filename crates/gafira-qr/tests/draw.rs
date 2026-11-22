//! The outlines of the codes, read back by a QR reader (spec 080).

use gafira_color::{Color, Rgba};
use gafira_doc::timeline::AssetRef;
use gafira_doc::{
    ContentHash, Gradient, GradientKind, GradientStop, QrBody, QrContent, QrEcc, QrEyeBall,
    QrEyeFrame, QrFill, QrLogo, QrNode,
};
use gafira_qr::{Matrix, QrDrawing, QrError, Warning, draw, payload, warnings};
use kurbo::{Affine, BezPath, ParamCurve as _, PathEl, Shape};
use proptest::prelude::*;

fn grey(v: f32) -> Color {
    Color::Srgb(Rgba::new(v, v, v, 1.0))
}

fn code(text: &str) -> QrNode {
    let text = text.to_owned();
    QrNode::new(QrContent::Text { text }, grey(0.0), grey(1.0))
}

fn logo(share: f64, natural: (u32, u32)) -> QrLogo {
    QrLogo {
        asset: AssetRef::new(ContentHash([7; 32])),
        natural,
        share,
        clear_behind: true,
    }
}

fn skia(path: &BezPath) -> Option<tiny_skia::Path> {
    let mut b = tiny_skia::PathBuilder::new();
    let f = |p: kurbo::Point| (p.x as f32, p.y as f32);
    for el in path.elements() {
        match *el {
            PathEl::MoveTo(p) => b.move_to(f(p).0, f(p).1),
            PathEl::LineTo(p) => b.line_to(f(p).0, f(p).1),
            PathEl::QuadTo(c, p) => b.quad_to(f(c).0, f(c).1, f(p).0, f(p).1),
            PathEl::CurveTo(c, d, p) => {
                b.cubic_to(f(c).0, f(c).1, f(d).0, f(d).1, f(p).0, f(p).1);
            }
            PathEl::ClosePath => b.close(),
        }
    }
    b.finish()
}

/// The code in black on white at `scale` px a module; the logo is a grey box.
fn pixels(d: &QrDrawing, scale: f64, rule: tiny_skia::FillRule) -> tiny_skia::Pixmap {
    let side = ((d.size + 2.0 * d.margin) * scale).round() as u32;
    let mut pixmap = tiny_skia::Pixmap::new(side, side).expect("a size");
    pixmap.fill(tiny_skia::Color::WHITE);
    let at = Affine::scale(scale) * Affine::translate((d.margin, d.margin));
    let mut paint = tiny_skia::Paint {
        anti_alias: true,
        ..tiny_skia::Paint::default()
    };
    paint.set_color_rgba8(0, 0, 0, 255);
    let id = tiny_skia::Transform::identity();
    for layer in &d.layers {
        if let Some(path) = skia(&(at * &layer.path)) {
            pixmap.fill_path(&path, &paint, rule, id, None);
        }
    }
    if let Some(logo) = d.logo {
        paint.set_color_rgba8(128, 128, 128, 255);
        let path = skia(&(at * logo.rect.to_path(0.1))).expect("a box");
        pixmap.fill_path(&path, &paint, rule, id, None);
    }
    pixmap
}

fn read(d: &QrDrawing) -> Option<String> {
    let scale = 8.0;
    let p = pixels(d, scale, tiny_skia::FillRule::Winding);
    let w = p.width() as usize;
    // The size is a whole number of modules.
    gafira_testkit::qr::read(d.size as usize, |x, y| {
        let at = |v: f64| (((v + d.margin) * scale).max(0.0) as usize).min(w - 1);
        p.data()[(at(y) * w + at(x)) * 4]
    })
}

fn body_path(qr: &QrNode) -> BezPath {
    draw(qr).expect("drawn").layers[0].path.clone()
}

// S80-04: the last byte that fits in each correction, and the next.
#[test]
fn s80_04_capacity_per_level() {
    for (ecc, most) in [
        (QrEcc::L, 2953),
        (QrEcc::M, 2331),
        (QrEcc::Q, 1663),
        (QrEcc::H, 1273),
    ] {
        let mut qr = code(&"a".repeat(most));
        qr.ecc = ecc;
        assert!(draw(&qr).is_ok(), "{ecc:?}");
        qr.content = QrContent::Text {
            text: "a".repeat(most + 1),
        };
        assert_eq!(draw(&qr), Err(QrError::TooLong), "{ecc:?}");
    }
}

// FD-20 b: a higher correction is a denser code.
#[test]
fn fd_20_b_the_level_changes_the_density() {
    let text = "a".repeat(60);
    let size = |ecc| Matrix::encode(&text, ecc, 1).expect("fits").size();
    assert!(size(QrEcc::L) < size(QrEcc::H));
}

// S80-05: the squares cover exactly the dark modules, at every version.
#[test]
fn s80_05_square_area_is_the_dark_modules() {
    let mut sizes = Vec::new();
    for length in [1, 20, 60, 120, 300, 700, 1300, 2331] {
        let text = "a".repeat(length);
        let m = Matrix::encode(&text, QrEcc::M, 1).expect("fits");
        sizes.push(m.size());
        let d = draw(&code(&text)).expect("drawn");
        let area: f64 = d.layers.iter().map(|l| l.path.area().abs()).sum();
        let n = m.size() as i32;
        let dark = (0..n * n).filter(|i| m.dark(i % n, i / n)).count();
        assert!((area - dark as f64).abs() < 1e-6, "{area} vs {dark}");
    }
    assert_eq!(sizes.last(), Some(&177));
}

// S80-05: every edge of the squares has a dark module on one side and a
// light one on the other.
#[test]
fn s80_05_no_inner_edges() {
    let text = "Gafira";
    let m = Matrix::encode(text, QrEcc::M, 1).expect("fits");
    let body = |x: i32, y: i32| m.dark(x, y) && !m.in_eye(x, y);
    let mut edges = 0;
    for s in body_path(&code(text)).segments() {
        let (a, b): (kurbo::Point, kurbo::Point) = (s.eval(0.0), s.eval(1.0));
        let steps = (b - a).hypot().round() as i32;
        for i in 0..steps {
            let mid = a.lerp(b, (f64::from(i) + 0.5) / f64::from(steps));
            let (x, y) = (mid.x, mid.y);
            let sides = if (a.x - b.x).abs() < 1e-9 {
                let c = x.round() as i32;
                [(c - 1, y.floor() as i32), (c, y.floor() as i32)]
            } else {
                let r = y.round() as i32;
                [(x.floor() as i32, r - 1), (x.floor() as i32, r)]
            };
            assert_ne!(body(sides[0].0, sides[0].1), body(sides[1].0, sides[1].1));
            edges += 1;
        }
    }
    assert!(edges > 0);
}

fn styles() -> Vec<(QrBody, QrEyeFrame, QrEyeBall)> {
    let mut all = Vec::new();
    for body in QrBody::ALL {
        for frame in QrEyeFrame::ALL {
            for ball in QrEyeBall::ALL {
                all.push((body, frame, ball));
            }
        }
    }
    all
}

fn styled(text: &str, (body, frame, ball): (QrBody, QrEyeFrame, QrEyeBall)) -> QrNode {
    let mut qr = code(text);
    qr.style.body = body;
    qr.style.eye_frame = frame;
    qr.style.eye_ball = ball;
    qr
}

// S80-05: the non-zero and the even-odd rule fill the same.
#[test]
fn s80_05_both_fill_rules_agree() {
    let mut each: Vec<_> = QrBody::ALL
        .iter()
        .map(|b| (*b, QrEyeFrame::Square, QrEyeBall::Square))
        .collect();
    each.extend((0..8).map(|i| (QrBody::Square, QrEyeFrame::ALL[i], QrEyeBall::ALL[i])));
    for style in each {
        let d = draw(&styled("Gafira 2026", style)).expect("drawn");
        let a = pixels(&d, 12.0, tiny_skia::FillRule::Winding);
        let b = pixels(&d, 12.0, tiny_skia::FillRule::EvenOdd);
        let far = a
            .data()
            .iter()
            .zip(b.data())
            .filter(|(x, y)| x.abs_diff(**y) > 8)
            .count();
        assert_eq!(far, 0, "{style:?}");
    }
}

// S80-06, FD-20 a: every combination is read back.
#[test]
fn s80_06_every_style_reads_back() {
    let text = "https://example.com/gafira/qr-code";
    let mut lost = Vec::new();
    for style in styles() {
        let d = draw(&styled(text, style)).expect("drawn");
        if read(&d).as_deref() != Some(text) {
            lost.push(style);
        }
    }
    assert!(lost.is_empty(), "{} lost: {lost:?}", lost.len());
}

// S80-06: every body is read back in a small, a middle and a large code.
#[test]
fn s80_06_every_body_reads_back_at_three_sizes() {
    for length in [8, 110, 900] {
        let text = "g".repeat(length);
        for body in QrBody::ALL {
            let d = draw(&styled(
                &text,
                (body, QrEyeFrame::Rounded, QrEyeBall::Rounded),
            ))
            .expect("drawn");
            assert_eq!(
                read(&d).as_deref(),
                Some(text.as_str()),
                "{body:?} {length}"
            );
        }
    }
}

// S80-10: the logo stays off the eyes, the timing and the format modules.
#[test]
fn s80_10_logo_never_covers_function_modules() {
    for length in [1, 10, 60, 300, 1000] {
        for natural in [(100, 100), (300, 100), (100, 300)] {
            let mut qr = code(&"a".repeat(length));
            qr.logo = Some(logo(QrLogo::MAX_SHARE, natural));
            let d = draw(&qr).expect("drawn");
            let rect = d.logo.expect("a logo").rect;
            assert!(d.size >= 25.0, "version 2 at least");
            assert!(rect.x0 >= 9.0 && rect.y0 >= 9.0, "{rect:?}");
            assert!(
                rect.x1 <= d.size - 9.0 && rect.y1 <= d.size - 9.0,
                "{rect:?}"
            );
            let wanted = f64::from(natural.0) / f64::from(natural.1);
            assert!((rect.width() / rect.height() - wanted).abs() < 1e-9);
            assert!((rect.center().x - d.size / 2.0).abs() < 1e-9);
            // Nothing is left behind it.
            let body = &d.layers[0].path;
            assert_eq!(body.winding(rect.center()), 0);
        }
    }
}

// S80-10: with the largest logo every version is read back.
#[test]
fn s80_10_the_largest_logo_reads_back() {
    for length in [1, 8, 20, 40, 90, 200, 400, 800, 1273] {
        for clear_behind in [true, false] {
            let text = "g".repeat(length);
            let mut qr = code(&text);
            qr.logo = Some(QrLogo {
                clear_behind,
                ..logo(QrLogo::MAX_SHARE, (100, 100))
            });
            let d = draw(&qr).expect("drawn");
            assert_eq!(
                read(&d).as_deref(),
                Some(text.as_str()),
                "{length} {clear_behind}"
            );
        }
    }
}

// S80-13: the parts of a code fill the same pixels, whatever its shapes and
// its size, and each one is a path a document holds.
#[test]
fn s80_13_the_parts_draw_the_code() {
    use gafira_doc::{NodeKind, PathCommand, Placement, ShapeGeometry, ShapePath};
    for length in [8, 300, 1000] {
        for body in QrBody::ALL {
            let mut qr = styled(
                &"g".repeat(length),
                (body, QrEyeFrame::Leaf, QrEyeBall::Flower),
            );
            qr.style.background = None;
            qr.quiet_zone = 0;
            let d = draw(&qr).expect("drawn");
            let side = d.size;
            let found =
                gafira_qr::parts(&qr, &Placement::new(0.0, 0.0, side, side)).expect("parts");
            let mut joined = d.clone();
            joined.layers = found
                .iter()
                .map(|part| {
                    let NodeKind::Shape(shape) = &part.kind else {
                        panic!("a shape")
                    };
                    let ShapeGeometry::Path(path) = &shape.geometry else {
                        panic!("a path")
                    };
                    assert!(path.commands.len() <= ShapePath::MAX_COMMANDS);
                    assert_eq!((part.placement.w, part.placement.h), (side, side));
                    let p = |[x, y]: [f64; 2]| kurbo::Point::new(x * side, y * side);
                    let mut bez = BezPath::new();
                    for c in &path.commands {
                        match *c {
                            PathCommand::MoveTo(a) => bez.move_to(p(a)),
                            PathCommand::LineTo(a) => bez.line_to(p(a)),
                            PathCommand::QuadTo(a, b) => bez.quad_to(p(a), p(b)),
                            PathCommand::CubicTo(a, b, c) => bez.curve_to(p(a), p(b), p(c)),
                            PathCommand::Close => bez.close_path(),
                        }
                    }
                    gafira_qr::Layer {
                        path: bez,
                        fill: shape.fill.map(QrFill::Solid).expect("a fill"),
                    }
                })
                .collect();
            let a = pixels(&d, 4.0, tiny_skia::FillRule::Winding);
            let b = pixels(&joined, 4.0, tiny_skia::FillRule::Winding);
            let far = a
                .data()
                .iter()
                .zip(b.data())
                .filter(|(x, y)| x.abs_diff(**y) > 40)
                .count();
            assert_eq!(far, 0, "{body:?} {length}");
        }
    }
}

// S80-07: the eyes take the colour of the body unless they have their own.
#[test]
fn s80_07_the_eyes_take_their_colour() {
    let mut qr = code("a");
    let fills = |qr: &QrNode| -> Vec<QrFill> {
        draw(qr)
            .expect("drawn")
            .layers
            .iter()
            .map(|l| l.fill.clone())
            .collect()
    };
    assert_eq!(fills(&qr), vec![QrFill::Solid(grey(0.0)); 3]);
    qr.style.eye_frame_fill = Some(QrFill::Solid(grey(0.2)));
    qr.style.eye_ball_fill = Some(QrFill::Solid(grey(0.3)));
    assert_eq!(
        fills(&qr),
        [grey(0.0), grey(0.2), grey(0.3)]
            .map(QrFill::Solid)
            .to_vec()
    );
}

// S80-08: the worst pair, at 3:1, below, with a gradient and inverted.
#[test]
fn s80_08_worst_pair_contrast() {
    let with = |dark: QrFill, light: Option<Color>| {
        let mut qr = code("a");
        qr.style.fill = dark;
        qr.style.background = light;
        warnings(&qr)
    };
    let solid = |v: f32| QrFill::Solid(grey(v));
    assert_eq!(with(solid(0.0), Some(grey(1.0))), vec![]);
    // sRGB 148 of 255 is the lightest grey at 3:1 on white; 149 is below.
    assert_eq!(with(solid(148.0 / 255.0), Some(grey(1.0))), vec![]);
    assert_eq!(
        with(solid(149.0 / 255.0), Some(grey(1.0))),
        vec![Warning::LowContrast]
    );
    assert_eq!(with(solid(1.0), Some(grey(0.0))), vec![Warning::Inverted]);
    assert_eq!(with(solid(0.0), None), vec![Warning::Transparent]);
    let fade = QrFill::Gradient(Gradient {
        kind: GradientKind::Radial,
        stops: [(0.0, 0.0), (1.0, 0.9)]
            .map(|(offset, v)| GradientStop {
                offset,
                color: grey(v),
            })
            .to_vec(),
    });
    assert_eq!(with(fade, Some(grey(1.0))), vec![Warning::LowContrast]);
    let mut qr = code("a");
    qr.style.eye_ball_fill = Some(QrFill::Solid(grey(0.95)));
    assert_eq!(warnings(&qr), vec![Warning::LowContrast]);
    let mut qr = code("a");
    qr.quiet_zone = 3;
    assert_eq!(warnings(&qr), vec![Warning::NarrowMargin]);
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(24))]
    // FD-20 a: any text comes back as it went in.
    #[test]
    fn fd_20_a_any_text_reads_back(text in "[a-zA-Z0-9 áéíñü€:/?&=]{1,120}") {
        prop_assume!(!text.trim().is_empty());
        let qr = code(&text);
        prop_assert_eq!(payload(&qr.content), Ok(text.clone()));
        prop_assert_eq!(read(&draw(&qr).expect("drawn")), Some(text));
    }
}
