//! Align a selection to the page or to its own box (FD-11, D-29).

use gafira_geom::Rect;

use crate::LayoutError;
use crate::error::finite;

/// One box of the selection, in px of page.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Item {
    /// The box (see [`crate::node_bounds`]).
    pub rect: Rect,
    /// Whether the element is locked: it never moves.
    pub locked: bool,
}

/// How far one box moves, in px of page. Zero on a box that stays.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Offset {
    /// Along x.
    pub dx: f64,
    /// Along y.
    pub dy: f64,
}

/// The six actions of Align (FD-11).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AlignAction {
    /// Left edges to the left of the reference.
    Left,
    /// Centres to the horizontal centre.
    Center,
    /// Right edges to the right of the reference.
    Right,
    /// Top edges to the top of the reference.
    Top,
    /// Centres to the vertical centre.
    Middle,
    /// Bottom edges to the bottom of the reference.
    Bottom,
}

/// Checks every number, then every box, then every centre.
///
/// The page is read by `align` only, so `distribute` passes `None`. All numbers
/// come first, whatever box they sit in; a centre that overflows is a result that
/// is not finite, also for a box the action does not use.
pub(crate) fn check_boxes(page: Option<Rect>, items: &[Item]) -> Result<(), LayoutError> {
    let numbers = |field: &'static str, r: Rect| -> Result<(), LayoutError> {
        for v in [r.x0, r.y0, r.x1, r.y1] {
            finite(field, v)?;
        }
        Ok(())
    };
    let order = |field: &'static str, r: Rect| -> Result<(), LayoutError> {
        if r.x1 < r.x0 || r.y1 < r.y0 {
            return Err(LayoutError::InvalidBox { field });
        }
        Ok(())
    };
    let centres = |field: &'static str, r: Rect| -> Result<(), LayoutError> {
        finite(field, (r.x0 + r.x1) / 2.0)?;
        finite(field, (r.y0 + r.y1) / 2.0)?;
        Ok(())
    };
    if let Some(p) = page {
        numbers("page", p)?;
    }
    for i in items {
        numbers("item", i.rect)?;
    }
    if let Some(p) = page {
        order("page", p)?;
    }
    for i in items {
        order("item", i.rect)?;
    }
    if let Some(p) = page {
        centres("page", p)?;
    }
    for i in items {
        centres("item", i.rect)?;
    }
    Ok(())
}

/// The box that holds all the boxes of `items`, which must not be empty.
fn union(items: &[Item]) -> Option<Rect> {
    items.iter().map(|i| i.rect).reduce(|a, b| a.union(b))
}

/// The offset of each item that aligns the selection, in input order.
///
/// A selection of exactly one free element aligns to `page`. Two or more align
/// to the box that holds all of them, locked ones included (D-29), even when
/// only one is free. A locked element never moves, so nothing moves when the
/// selection is empty or all locked. Each action
/// changes one axis only; the other offset is `+0.0`.
///
/// Every box (and `page`) is checked, locked or not and used or not: all
/// numbers, then `x1 >= x0` and `y1 >= y0`, then every centre. An offset that
/// is not finite is `NonFinite` naming `offset`.
pub fn align(action: AlignAction, page: Rect, items: &[Item]) -> Result<Vec<Offset>, LayoutError> {
    check_boxes(Some(page), items)?;
    let mut out = vec![Offset { dx: 0.0, dy: 0.0 }; items.len()];
    let reference = match items {
        [_] => page,
        _ => match union(items) {
            Some(u) => u,
            // No box: nothing to align.
            None => return Ok(out),
        },
    };
    // Finite: `check_boxes` has checked the centre of the page and of every box, and
    // the box that holds them all cannot have a centre outside theirs.
    let centre_x = (reference.x0 + reference.x1) / 2.0;
    let centre_y = (reference.y0 + reference.y1) / 2.0;
    for (slot, item) in out.iter_mut().zip(items) {
        if item.locked {
            continue;
        }
        let r = item.rect;
        // `+ 0.0` turns a -0.0 into 0.0.
        let moved = |v: f64| finite("offset", v).map(|v| v + 0.0);
        *slot = match action {
            AlignAction::Left => Offset {
                dx: moved(reference.x0 - r.x0)?,
                dy: 0.0,
            },
            AlignAction::Center => Offset {
                dx: moved(centre_x - (r.x0 + r.x1) / 2.0)?,
                dy: 0.0,
            },
            AlignAction::Right => Offset {
                dx: moved(reference.x1 - r.x1)?,
                dy: 0.0,
            },
            AlignAction::Top => Offset {
                dx: 0.0,
                dy: moved(reference.y0 - r.y0)?,
            },
            AlignAction::Middle => Offset {
                dx: 0.0,
                dy: moved(centre_y - (r.y0 + r.y1) / 2.0)?,
            },
            AlignAction::Bottom => Offset {
                dx: 0.0,
                dy: moved(reference.y1 - r.y1)?,
            },
        };
    }
    Ok(out)
}
