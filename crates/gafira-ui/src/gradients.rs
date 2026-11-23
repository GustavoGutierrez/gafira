//! What the «Degradado» tab of the Inspector does to a gradient (spec 063):
//! start one from the fill, pick a style, change, add or remove a colour.

use gafira_color::{Color, Rgba};
use gafira_doc::{Gradient, GradientKind, GradientStop};

/// The styles the tab offers, in order: left to right, right to left, top
/// to bottom, bottom to top, the two diagonals and circular.
pub(crate) const STYLES: [GradientKind; 7] = [
    GradientKind::Linear { angle_deg: 0.0 },
    GradientKind::Linear { angle_deg: 180.0 },
    GradientKind::Linear { angle_deg: 90.0 },
    GradientKind::Linear { angle_deg: 270.0 },
    GradientKind::Linear { angle_deg: 45.0 },
    GradientKind::Linear { angle_deg: 135.0 },
    GradientKind::Radial,
];

/// The gradient the tab starts with: from the fill (light grey without
/// one) to a darker shade of it, left to right.
pub(crate) fn start(fill: Option<Color>) -> Gradient {
    let from = fill.unwrap_or(srgb(200.0 / 255.0, 209.0 / 255.0, 217.0 / 255.0));
    let c = match from.convert(gafira_color::ColorSpaceId::Srgb) {
        Color::Srgb(c) => c,
        _ => Rgba::new(0.5, 0.5, 0.5, 1.0),
    };
    let darker = Color::Srgb(Rgba::new(c.r * 0.45, c.g * 0.45, c.b * 0.45, c.a));
    Gradient {
        kind: STYLES[0],
        stops: vec![stop(0.0, from), stop(1.0, darker)],
    }
}

/// `gradient` with colour `index` set to `color`.
pub(crate) fn with_color(gradient: &Gradient, index: usize, color: Color) -> Gradient {
    let mut out = gradient.clone();
    if let Some(stop) = out.stops.get_mut(index) {
        stop.color = color;
    }
    out
}

/// `gradient` with one more colour, the last one again, the colours spread
/// evenly; `None` when it holds the most it can.
pub(crate) fn with_stop_added(gradient: &Gradient) -> Option<Gradient> {
    if gradient.stops.len() >= Gradient::MAX_STOPS {
        return None;
    }
    let last = gradient.stops.last()?.color;
    let colors: Vec<Color> = gradient
        .stops
        .iter()
        .map(|s| s.color)
        .chain([last])
        .collect();
    Some(spread(gradient.kind, colors))
}

/// `gradient` without colour `index`, the rest spread evenly; `None` when
/// it would hold fewer than two.
pub(crate) fn with_stop_removed(gradient: &Gradient, index: usize) -> Option<Gradient> {
    if gradient.stops.len() <= Gradient::MIN_STOPS || index >= gradient.stops.len() {
        return None;
    }
    let colors = gradient
        .stops
        .iter()
        .enumerate()
        .filter(|(i, _)| *i != index)
        .map(|(_, s)| s.color)
        .collect();
    Some(spread(gradient.kind, colors))
}

/// `colors` spread evenly from 0 to 1.
fn spread(kind: GradientKind, colors: Vec<Color>) -> Gradient {
    let last = colors.len().saturating_sub(1).max(1) as f32;
    Gradient {
        kind,
        stops: colors
            .into_iter()
            .enumerate()
            .map(|(i, color)| stop(i as f32 / last, color))
            .collect(),
    }
}

fn srgb(r: f32, g: f32, b: f32) -> Color {
    Color::Srgb(Rgba::new(r, g, b, 1.0))
}

fn stop(offset: f32, color: Color) -> GradientStop {
    GradientStop { offset, color }
}

#[cfg(test)]
mod tests {
    use super::*;

    // S63-04: the tab starts from the fill to a darker shade; a style, a
    // colour, one more colour or one less give a valid gradient each.
    #[test]
    fn s63_04_gradient_edits() {
        let red = srgb(1.0, 0.0, 0.0);
        let first = start(Some(red));
        assert_eq!(first.kind, GradientKind::Linear { angle_deg: 0.0 });
        assert_eq!(first.stops.len(), 2);
        assert_eq!(first.stops[0], stop(0.0, red));
        assert_eq!(first.stops[1].offset, 1.0);
        assert_ne!(first.stops[1].color, red, "a darker shade");
        assert_eq!(first.problem(), None);
        assert_eq!(start(None).problem(), None);

        let blue = srgb(0.0, 0.0, 1.0);
        let coloured = with_color(&first, 1, blue);
        assert_eq!(coloured.stops[1], stop(1.0, blue));
        assert_eq!(with_color(&first, 9, blue), first, "no such colour");

        let three = with_stop_added(&coloured).expect("room for one more");
        assert_eq!(
            three.stops,
            [stop(0.0, red), stop(0.5, blue), stop(1.0, blue)]
        );
        let two = with_stop_removed(&three, 0).expect("two are left");
        assert_eq!(two.stops, [stop(0.0, blue), stop(1.0, blue)]);
        assert_eq!(with_stop_removed(&two, 0), None, "two at least");

        let mut full = first.clone();
        while let Some(next) = with_stop_added(&full) {
            full = next;
        }
        assert_eq!(full.stops.len(), Gradient::MAX_STOPS);
        assert_eq!(full.problem(), None);
        for kind in STYLES {
            let styled = Gradient {
                kind,
                ..first.clone()
            };
            assert_eq!(styled.problem(), None);
        }
    }
}
