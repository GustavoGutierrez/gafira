//! The effects of a text drawn (spec 089, FD-43): the pieces around its
//! letters, from the back to the front. What is not out of focus is vector,
//! and the letters are one of the pieces, so they can stay text.

use std::sync::Arc;

use gafira_color::{Color, LinearRgba};
use gafira_doc::{TextEffect, TextEffectKind};
use gafira_geom::{Affine, Point, Rect};
use gafira_pathops::{FillRule, Path};
use gafira_raster::{ImageBuf, soft_shape};
use kurbo::{BezPath, Join, Shape, Stroke, StrokeOpts};

/// A piece of a text with effects, in its box.
#[derive(Clone, Debug)]
pub enum TextPiece {
    Fill {
        path: BezPath,
        color: Color,
    },
    /// A picture: its pixels are laid with the transform.
    Soft {
        image: Arc<ImageBuf>,
        at: Affine,
    },
    /// The letters, in a colour instead of theirs when there is one; clear
    /// when they only stay as words.
    Letters(Option<Color>),
}

/// Parts of the size of the letters at 100.
const FAR: f64 = 0.4;
const BLUR: f64 = 0.15;
const AROUND: f64 = 0.16;
const LINE: f64 = 0.07;
const ROOM: f64 = 0.4;
const APART: f64 = 0.12;

/// `color`, `share` of it.
fn faint(color: Color, share: f64) -> Color {
    let mut c = color.to_linear_srgb();
    // Between 0 and 1: the cast loses nothing seen.
    c.a *= share.clamp(0.0, 1.0) as f32;
    Color::LinearSrgb(c)
}

/// `color` as far to white as `share` says.
fn lit(color: Color, share: f32) -> Color {
    let c = color.to_linear_srgb();
    let to = |v: f32| v + (1.0 - v) * share;
    Color::LinearSrgb(LinearRgba::new(to(c.r), to(c.g), to(c.b), c.a))
}

/// The outline of letters that may be made of pieces one over another, as
/// those of a variable font are: one silhouette, with its holes.
fn silhouette(path: &BezPath, em: f64) -> BezPath {
    let whole = Path::new(path.clone(), FillRule::NonZero).close_subpaths();
    // With itself: with nothing it is left as it is.
    whole
        .union(&whole, (em / 2000.0).max(1e-4))
        // It cannot be put together: as it is.
        .map_or_else(|_| path.clone(), |one| one.as_bez_path().clone())
}

/// The line along the outline of `path`, `width` wide.
fn line(path: &BezPath, width: f64, em: f64) -> BezPath {
    let path = silhouette(path, em);
    let style = Stroke::new(width).with_join(Join::Round);
    kurbo::stroke(
        path.iter(),
        &style,
        &StrokeOpts::default(),
        (em / 400.0).max(1e-3),
    )
}

/// The boxes of the lines, as one shape.
fn boxes(lines: &[Rect], grow: f64, share: f64) -> BezPath {
    let mut out = BezPath::new();
    for rect in lines {
        // Wider than tall: the room across and half of it down.
        let rect = rect.inflate(grow, grow / 2.0);
        let radius = share * rect.width().min(rect.height()) / 2.0;
        out.extend(rect.to_rounded_rect(radius).to_path(0.05));
    }
    out
}

/// The pieces of the letters `path` of `color`, whose lines are `lines` and
/// whose size is `em`; `density` is how many pixels a unit is seen at.
pub(crate) fn pieces(
    effects: &[TextEffect],
    (path, color): (&BezPath, Color),
    lines: &[Rect],
    em: f64,
    density: f64,
) -> Vec<TextPiece> {
    use TextEffectKind as K;
    let mut out = Vec::new();
    if effects.is_empty() {
        return vec![TextPiece::Letters(None)];
    }
    let share = |v: f64| v / 100.0;
    let away = |effect: &TextEffect, far: f64| {
        let turn = effect.angle_deg.to_radians();
        let d = share(effect.distance) * far * em;
        Affine::translate((turn.cos() * d, turn.sin() * d))
    };
    let soft = |out: &mut Vec<TextPiece>, path: &BezPath, color: Color, sigma: f64| {
        if let Some((image, at)) = soft_shape(path, color, sigma, density) {
            let image = Arc::new(image);
            out.push(TextPiece::Soft { image, at });
        }
    };
    let of = |kind: K| effects.iter().filter(move |effect| effect.kind == kind);
    for effect in of(K::Background) {
        out.push(TextPiece::Fill {
            path: boxes(
                lines,
                share(effect.spread) * ROOM * em,
                share(effect.roundness),
            ),
            color: faint(effect.color, share(effect.opacity)),
        });
    }
    // A shadow is of all there is in front of it: the line around too.
    let widest = of(K::Outline)
        .map(|effect| share(effect.thickness) * AROUND * em)
        .fold(0.0, f64::max);
    let body = || {
        let mut body = path.clone();
        if widest > 0.0 {
            body.extend(line(path, 2.0 * widest, em));
        }
        body
    };
    for effect in effects {
        match effect.kind {
            K::Drop => {
                let moved = away(effect, FAR) * body();
                let color = faint(effect.color, share(effect.opacity));
                let sigma = share(effect.blur) * BLUR * em;
                if sigma > 0.0 {
                    soft(&mut out, &moved, color, sigma);
                } else {
                    out.push(TextPiece::Fill { path: moved, color });
                }
            }
            K::Glow => {
                let strength = share(effect.intensity);
                let wide = (0.04 + 0.16 * strength) * em;
                soft(&mut out, &body(), faint(effect.color, 0.9), wide);
                let near = faint(effect.color, 0.4 + 0.5 * strength);
                soft(&mut out, &body(), near, wide / 3.0);
            }
            K::Neon => {
                let strength = share(effect.intensity);
                for (far, seen) in [(0.22, 0.75), (0.08, 0.85), (0.025, 0.9)] {
                    let sigma = far * (0.4 + 0.8 * strength) * em;
                    soft(&mut out, &body(), faint(effect.color, seen), sigma);
                }
            }
            K::Echo => {
                let step = away(effect, FAR / 2.0);
                for (times, seen) in [(3, 0.2), (2, 0.4), (1, 0.6)] {
                    let mut moved = body();
                    for _ in 0..times {
                        moved = step * moved;
                    }
                    let color = faint(effect.color, seen);
                    out.push(TextPiece::Fill { path: moved, color });
                }
            }
            K::Glitch => {
                let step = away(effect, APART);
                let mut other = body();
                other.apply_affine(step.inverse());
                out.push(TextPiece::Fill {
                    path: step * body(),
                    color: effect.color,
                });
                out.push(TextPiece::Fill {
                    path: other,
                    color: effect.second,
                });
            }
            K::Offset => out.push(TextPiece::Fill {
                path: away(effect, FAR / 2.0) * path.clone(),
                color: effect.color,
            }),
            K::Outline | K::Background | K::Hollow => {}
        }
    }
    for effect in of(K::Outline) {
        let width = 2.0 * share(effect.thickness) * AROUND * em;
        if width > 0.0 {
            out.push(TextPiece::Fill {
                path: line(path, width, em),
                color: effect.color,
            });
        }
    }
    let hidden = effects.iter().any(|effect| effect.kind.hides_fill());
    let neon = of(K::Neon).next();
    out.push(TextPiece::Letters(if hidden {
        Some(faint(color, 0.0))
    } else {
        neon.map(|effect| lit(effect.color, 0.8))
    }));
    for effect in effects {
        if !effect.kind.hides_fill() {
            continue;
        }
        let width = (0.012 + LINE * share(effect.thickness)) * em;
        out.push(TextPiece::Fill {
            path: line(path, width, em),
            color: neon.map_or(color, |effect| lit(effect.color, 0.8)),
        });
    }
    out
}

/// The boxes of the lines of a text: the runs of each, as `(baseline, size)`,
/// and where the letters on it begin and end. All of `path` when it is not
/// known.
pub(crate) fn line_boxes(runs: &[(f64, f64)], path: &BezPath) -> Vec<Rect> {
    let mut lines: Vec<(f64, f64)> = Vec::new();
    for (baseline, size) in runs {
        match lines
            .iter_mut()
            .find(|line| (line.0 - baseline).abs() < 0.5)
        {
            Some(line) => line.1 = line.1.max(*size),
            None => lines.push((*baseline, *size)),
        }
    }
    if lines.is_empty() {
        let bounds = path.bounding_box();
        return if bounds.width() > 0.0 {
            vec![bounds]
        } else {
            Vec::new()
        };
    }
    let mut across: Vec<Option<(f64, f64)>> = vec![None; lines.len()];
    for piece in path.subpaths() {
        let b = piece.bounding_box();
        let middle = b.center().y;
        // The line whose middle it is nearest to.
        let nearest = lines
            .iter()
            .enumerate()
            .map(|(i, (baseline, size))| (i, (baseline - 0.35 * size - middle).abs()))
            .min_by(|a, b| a.1.total_cmp(&b.1));
        if let Some((i, _)) = nearest {
            let now = across[i].unwrap_or((b.x0, b.x1));
            across[i] = Some((now.0.min(b.x0), now.1.max(b.x1)));
        }
    }
    lines
        .iter()
        .zip(across)
        .filter_map(|((baseline, size), across)| {
            let (x0, x1) = across?;
            Some(Rect::new(
                x0,
                baseline - 0.82 * size,
                x1,
                baseline + 0.26 * size,
            ))
        })
        .collect()
}

/// The box all of `pieces` and the letters `path` take.
#[must_use]
pub fn pieces_bounds(pieces: &[TextPiece], path: &BezPath) -> Rect {
    pieces
        .iter()
        .fold(path.bounding_box(), |all, piece| match piece {
            TextPiece::Fill { path, .. } => all.union(path.bounding_box()),
            TextPiece::Soft { image, at } => {
                let corner = Point::new(f64::from(image.width()), f64::from(image.height()));
                all.union(Rect::from_points(*at * Point::ZERO, *at * corner))
            }
            TextPiece::Letters(_) => all,
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use gafira_color::Rgba;

    const EM: f64 = 100.0;

    fn letters() -> BezPath {
        Rect::new(0.0, 0.0, 60.0, 70.0).to_path(0.1)
    }

    fn ink() -> Color {
        Color::Srgb(Rgba::new(0.1, 0.1, 0.3, 1.0))
    }

    fn of(effects: &[TextEffect]) -> Vec<TextPiece> {
        let lines = [Rect::new(0.0, -5.0, 60.0, 90.0)];
        pieces(effects, (&letters(), ink()), &lines, EM, 1.0)
    }

    fn with(kind: TextEffectKind, change: impl Fn(&mut TextEffect)) -> TextEffect {
        let mut effect = TextEffect::new(kind);
        change(&mut effect);
        effect
    }

    /// Which pieces there are, in a letter each.
    fn kinds(list: &[TextPiece]) -> String {
        list.iter()
            .map(|piece| match piece {
                TextPiece::Fill { .. } => 'f',
                TextPiece::Soft { .. } => 's',
                TextPiece::Letters(None) => 'L',
                TextPiece::Letters(Some(c)) if c.alpha() == 0.0 => '-',
                TextPiece::Letters(Some(_)) => 'l',
            })
            .collect()
    }

    // S89-02: the pieces of each effect, behind the letters or over them;
    // only what is out of focus is pixels.
    #[test]
    fn s89_02_the_pieces_of_each_effect() {
        use TextEffectKind as K;
        let want = [
            (K::Drop, "sL"),
            (K::Glow, "ssL"),
            (K::Echo, "fffL"),
            (K::Outline, "fL"),
            (K::Background, "fL"),
            (K::Offset, "f-f"),
            (K::Hollow, "-f"),
            (K::Neon, "sssl"),
            (K::Glitch, "ffL"),
        ];
        for (kind, want) in want {
            assert_eq!(kinds(&of(&[TextEffect::new(kind)])), want, "{kind:?}");
        }
        assert_eq!(kinds(&of(&[])), "L");
        let sharp = with(K::Drop, |e| e.blur = 0.0);
        assert_eq!(kinds(&of(&[sharp])), "fL");
        let both = [TextEffect::new(K::Outline), TextEffect::new(K::Background)];
        assert_eq!(kinds(&of(&both)), "ffL");
    }

    // S89-02: each number does what it says, as a share of the size.
    #[test]
    fn s89_02_the_numbers() {
        use TextEffectKind as K;
        let bounds = |effect: TextEffect| pieces_bounds(&of(&[effect]), &letters());
        let sharp = |distance, angle| {
            with(K::Drop, |e| {
                (e.blur, e.distance, e.angle_deg) = (0.0, distance, angle);
            })
        };
        let moved = bounds(sharp(100.0, 0.0));
        assert!((moved.x1 - 100.0).abs() < 1e-6 && (moved.y1 - 70.0).abs() < 1e-6);
        let moved = bounds(sharp(100.0, 90.0));
        assert!((moved.y1 - 110.0).abs() < 1e-6 && (moved.x1 - 60.0).abs() < 1e-6);
        let thick = bounds(with(K::Outline, |e| e.thickness = 100.0));
        assert!(
            (thick.x0 + 16.0).abs() < 0.2 && (thick.y1 - 86.0).abs() < 0.2,
            "{thick:?}"
        );
        assert_eq!(
            bounds(with(K::Outline, |e| e.thickness = 0.0)),
            letters().bounding_box()
        );
        let wide = bounds(with(K::Background, |e| e.spread = 100.0));
        assert!(
            (wide.x0 + 40.0).abs() < 1e-6 && (wide.y1 - 110.0).abs() < 1e-6,
            "{wide:?}"
        );
        // A shadow is of the outline too.
        let alone = bounds(sharp(50.0, 0.0)).x1;
        let pair = of(&[sharp(50.0, 0.0), with(K::Outline, |e| e.thickness = 100.0)]);
        assert!(pieces_bounds(&pair, &letters()).x1 > alone + 10.0);
        // Faint as it says.
        let TextPiece::Fill { color, .. } = &of(&[sharp(10.0, 0.0)])[0] else {
            panic!("a fill")
        };
        assert!((color.alpha() - 0.4).abs() < 1e-5);
    }

    // S89-02: the line goes around the outline of the letters, not around the
    // pieces they are made of: a letter of a font may be of pieces one over
    // another, and its holes stay.
    #[test]
    fn s89_02_the_line_is_of_the_outline() {
        use kurbo::Shape as _;
        // A cross of two bars, and a ring.
        let mut path = Rect::new(0.0, 40.0, 100.0, 60.0).to_path(0.1);
        path.extend(Rect::new(40.0, 0.0, 60.0, 100.0).to_path(0.1));
        path.extend(Rect::new(200.0, 0.0, 300.0, 100.0).to_path(0.1));
        path.extend(
            Rect::new(230.0, 30.0, 270.0, 70.0)
                .to_path(0.1)
                .reverse_subpaths(),
        );
        for kind in [TextEffectKind::Hollow, TextEffectKind::Offset] {
            let mut effect = TextEffect::new(kind);
            effect.thickness = 10.0;
            let made = pieces(&[effect], (&path, ink()), &[], EM, 1.0);
            let Some(TextPiece::Fill { path: line, .. }) = made.last() else {
                panic!("a line")
            };
            let on = |x: f64, y: f64| line.winding(Point::new(x, y)) != 0;
            // Around it, and around the hole.
            assert!(on(0.0, 50.0) && on(50.0, 0.0) && on(40.0, 20.0), "{kind:?}");
            assert!(on(230.0, 50.0) && on(300.0, 50.0), "{kind:?}");
            // Not where one bar crosses the other, nor inside.
            for (x, y) in [
                (40.0, 50.0),
                (60.0, 50.0),
                (50.0, 40.0),
                (50.0, 60.0),
                (50.0, 50.0),
            ] {
                assert!(!on(x, y), "{kind:?} {x},{y}");
            }
        }
    }

    // S89-02: a box for each line, as wide as its letters.
    #[test]
    fn s89_02_the_lines() {
        let mut path = Rect::new(10.0, 10.0, 90.0, 40.0).to_path(0.1);
        path.extend(Rect::new(30.0, 70.0, 60.0, 98.0).to_path(0.1));
        let lines = line_boxes(&[(40.0, 40.0), (40.2, 30.0), (100.0, 40.0)], &path);
        assert_eq!(lines.len(), 2);
        assert_eq!((lines[0].x0, lines[0].x1), (10.0, 90.0));
        assert_eq!((lines[1].x0, lines[1].x1), (30.0, 60.0));
        assert!(lines[0].y0 < 10.0 && lines[0].y1 > 40.0);
        assert_eq!(line_boxes(&[], &path), vec![path.bounding_box()]);
        assert!(line_boxes(&[], &BezPath::new()).is_empty());
    }
}
