//! Distribute a selection with equal gaps or equal centres (FD-11, D-17).

use std::cmp::Ordering;

use crate::LayoutError;
use crate::align::{Item, Offset, check_boxes};
use crate::error::finite;

/// The axis along which elements are spread.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DistributeAxis {
    /// Left to right.
    Horizontal,
    /// Top to bottom.
    Vertical,
}

/// What is made equal.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DistributeMode {
    /// The space between neighbours (the default of FD-11).
    #[default]
    EqualGap,
    /// The distance between neighbouring centres.
    EqualCentres,
}

/// The start, the end and the centre of a box along `axis`.
fn along(axis: DistributeAxis, item: &Item) -> (f64, f64, f64) {
    let r = item.rect;
    let (start, end) = match axis {
        DistributeAxis::Horizontal => (r.x0, r.x1),
        DistributeAxis::Vertical => (r.y0, r.y1),
    };
    (start, end, (start + end) / 2.0)
}

/// The offset of each item that spreads the free ones evenly along `axis`.
///
/// Only free (unlocked) items count (D-17). They are ordered by their position,
/// the start edge for [`DistributeMode::EqualGap`] and the centre for
/// [`DistributeMode::EqualCentres`], compared exactly, so `-0.0` and `0.0` tie
/// and a tie keeps the input order. The first and the last do not move, and with
/// fewer than three free items none does. Equal gap is
/// `(start of the last - end of the first - sum of the sizes between) / (n - 1)`;
/// it may be negative, and the boxes then overlap evenly. Equal centres spaces
/// the centres evenly between those of the first and the last.
///
/// The result has one offset per input item, in input order, `+0.0` on the
/// other axis and on every item that stays. Every box is checked, locked or not,
/// free or not, on both axes: all numbers, then `x1 >= x0` and `y1 >= y0`, then
/// every centre. An intermediate or an offset that is not finite is `NonFinite`
/// naming `gap` (the gap) or `offset`.
pub fn distribute(
    axis: DistributeAxis,
    mode: DistributeMode,
    items: &[Item],
) -> Result<Vec<Offset>, LayoutError> {
    check_boxes(None, items)?;
    let mut out = vec![Offset { dx: 0.0, dy: 0.0 }; items.len()];
    let mut free: Vec<usize> = (0..items.len()).filter(|&i| !items[i].locked).collect();
    let n = free.len();
    if n < 3 {
        return Ok(out);
    }
    let position = |i: usize| {
        let (start, _, centre) = along(axis, &items[i]);
        match mode {
            DistributeMode::EqualGap => start,
            DistributeMode::EqualCentres => centre,
        }
    };
    // The sort is stable: equal positions keep the input order.
    free.sort_by(|&a, &b| {
        position(a)
            .partial_cmp(&position(b))
            .unwrap_or(Ordering::Equal)
    });
    let (first, last) = (free[0], free[n - 1]);
    let (_, first_end, first_centre) = along(axis, &items[first]);
    let (last_start, _, last_centre) = along(axis, &items[last]);
    let spans = (n - 1) as f64;
    // `+ 0.0` turns a -0.0 into 0.0: `-0.0 - 0.0` is `-0.0`, and a gap that underflows is `-0.0`.
    let moved = |v: f64| finite("offset", v).map(|v| v + 0.0);
    let middle = &free[1..n - 1];
    match mode {
        DistributeMode::EqualGap => {
            // An infinite sum makes the numerator below non-finite.
            let mut sizes = 0.0;
            for &i in middle {
                let (start, end, _) = along(axis, &items[i]);
                sizes += end - start;
            }
            let gap = finite("gap", (last_start - first_end - sizes) / spans)?;
            let mut cursor = first_end;
            for &i in middle {
                let (start, end, _) = along(axis, &items[i]);
                // A sum that overflows reaches `moved` as an infinity.
                let new_start = cursor + gap;
                set(&mut out[i], axis, moved(new_start - start)?);
                cursor = new_start + (end - start);
            }
        }
        DistributeMode::EqualCentres => {
            // The centres lie within half of `f64::MAX` (checked above), so this is finite.
            let step = (last_centre - first_centre) / spans;
            for (k, &i) in middle.iter().enumerate() {
                let (_, _, centre) = along(axis, &items[i]);
                let target = first_centre + (k + 1) as f64 * step;
                set(&mut out[i], axis, moved(target - centre)?);
            }
        }
    }
    Ok(out)
}

fn set(slot: &mut Offset, axis: DistributeAxis, v: f64) {
    match axis {
        DistributeAxis::Horizontal => slot.dx = v,
        DistributeAxis::Vertical => slot.dy = v,
    }
}
