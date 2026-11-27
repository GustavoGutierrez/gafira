//! Resizing a text box (owner rule, 2026-11-21): a corner scales the box
//! and its letters together; a side changes only the width, never below
//! what one letter needs; a text box has no handles at the middle of its
//! top and bottom.

use gafira_doc::{RichText, TextNode};

use crate::canvas::Handle;

/// The smallest letter a corner drag leaves, in px.
pub(crate) const MIN_FONT: f64 = 6.0;

/// Whether a text box shows `handle`: the corners and the middle of the
/// sides, not the middle of the top and bottom.
pub(crate) fn shown_on_text(handle: Handle) -> bool {
    handle.x != 0
}

/// The largest and smallest letter of `text`, in px.
fn sizes(text: &RichText) -> Option<(f64, f64)> {
    text.spans()
        .map(|span| span.style.size)
        .filter(|size| size.is_finite() && *size > 0.0)
        .fold(None, |range, size| match range {
            None => Some((size, size)),
            Some((lo, hi)) => Some((f64::min(lo, size), f64::max(hi, size))),
        })
}

/// The narrowest a text box of `text` may be: room for one of its largest
/// letters.
pub(crate) fn min_width(text: &RichText) -> f64 {
    sizes(text).map_or(MIN_FONT, |(_, hi)| hi * 1.2)
}

/// The scale a corner drag of `scale` gives a text box: kept so its smallest
/// letter stays at least [`MIN_FONT`] and its largest within the limit.
pub(crate) fn corner_factor(text: &RichText, scale: f64) -> f64 {
    let Some((lo, hi)) = sizes(text) else {
        return scale;
    };
    if !scale.is_finite() {
        return 1.0;
    }
    scale.clamp(MIN_FONT / lo, TextNode::MAX_SIZE / hi)
}

/// `text` with every letter `factor` times its size.
pub(crate) fn scaled(text: &RichText, factor: f64) -> RichText {
    let mut out = text.clone();
    for paragraph in &mut out.paragraphs {
        for span in &mut paragraph.spans {
            span.style.size = (span.style.size * factor).clamp(1.0, TextNode::MAX_SIZE);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use gafira_doc::{Paragraph, Span, SpanStyle};

    use super::*;

    fn text(sizes: &[f64]) -> RichText {
        RichText {
            paragraphs: vec![Paragraph {
                spans: sizes
                    .iter()
                    .map(|size| Span {
                        text: "Ab".to_owned(),
                        style: SpanStyle {
                            font: "Inter".to_owned(),
                            size: *size,
                            color: gafira_color::Color::Srgb(gafira_color::Rgba::new(
                                0.0, 0.0, 0.0, 1.0,
                            )),
                            bold: false,
                            italic: false,
                            gradient: None,
                        },
                    })
                    .collect(),
            }],
        }
    }

    // Owner rule: a text box has handles at its corners and at the middle
    // of its sides only.
    #[test]
    fn text_boxes_have_no_top_or_bottom_middle_handles() {
        let shown: Vec<(i8, i8)> = crate::canvas::HANDLES
            .into_iter()
            .filter(|h| shown_on_text(*h))
            .map(|h| (h.x, h.y))
            .collect();
        assert_eq!(shown.len(), 6);
        assert!(!shown.contains(&(0, -1)) && !shown.contains(&(0, 1)));
    }

    // Owner rule: a corner scales every letter by the same factor, never
    // below the smallest letter allowed.
    #[test]
    fn a_corner_scales_the_letters() {
        let words = text(&[20.0, 40.0]);
        let bigger = scaled(&words, 1.5);
        let new: Vec<f64> = bigger.spans().map(|s| s.style.size).collect();
        assert_eq!(new, [30.0, 60.0]);
        assert_eq!(
            corner_factor(&words, 0.1),
            MIN_FONT / 20.0,
            "the smallest stays readable"
        );
        assert_eq!(corner_factor(&words, 2.0), 2.0);
    }

    // Owner rule: a side never makes the box narrower than one letter.
    #[test]
    fn a_side_keeps_room_for_a_letter() {
        assert_eq!(min_width(&text(&[20.0, 40.0])), 48.0);
        assert_eq!(min_width(&RichText::default()), MIN_FONT);
    }
}
