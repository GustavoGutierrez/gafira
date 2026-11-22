//! What may keep a code from being read (S80-08, S80-09).

use gafira_color::Color;
use gafira_doc::{QrFill, QrNode};

/// A warning about a code; it never blocks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Warning {
    /// Less than 3:1 between the code and its background.
    LowContrast,
    /// The code is lighter than its background.
    Inverted,
    /// No background: it depends on what lies behind.
    Transparent,
    /// Less margin than the standard asks for.
    NarrowMargin,
}

/// The luminance of WCAG 2 of a colour over `under`, 0 to 1.
fn luminance(color: Color, under: f64) -> f64 {
    let c = color.to_linear_srgb();
    let channel = |v: f32| f64::from(v).clamp(0.0, 1.0);
    let own = 0.2126 * channel(c.r) + 0.7152 * channel(c.g) + 0.0722 * channel(c.b);
    let alpha = channel(c.a);
    own * alpha + under * (1.0 - alpha)
}

fn ratio(a: f64, b: f64) -> f64 {
    (a.max(b) + 0.05) / (a.min(b) + 0.05)
}

/// The contrast between two colours, 1 to 21.
#[must_use]
pub fn contrast(a: Color, b: Color) -> f64 {
    let b = luminance(b, 1.0);
    ratio(luminance(a, b), b)
}

/// The least contrast of a code (FD-20).
const LEAST_CONTRAST: f64 = 3.0;

/// The warnings of `qr`.
#[must_use]
pub fn warnings(qr: &QrNode) -> Vec<Warning> {
    let mut out = Vec::new();
    let style = &qr.style;
    match style.background {
        // A see-through background depends on what lies behind it.
        Some(back) if back.alpha() >= 1.0 => {
            let under = luminance(back, 1.0);
            let fronts: Vec<f64> = [
                Some(&style.fill),
                style.eye_frame_fill.as_ref(),
                style.eye_ball_fill.as_ref(),
            ]
            .into_iter()
            .flatten()
            .flat_map(QrFill::colors)
            .map(|color| luminance(color, under))
            .collect();
            if fronts
                .iter()
                .any(|front| ratio(*front, under) < LEAST_CONTRAST)
            {
                out.push(Warning::LowContrast);
            }
            if fronts.iter().any(|front| *front > under) {
                out.push(Warning::Inverted);
            }
        }
        _ => out.push(Warning::Transparent),
    }
    if qr.quiet_zone < QrNode::QUIET_ZONE {
        out.push(Warning::NarrowMargin);
    }
    out
}
