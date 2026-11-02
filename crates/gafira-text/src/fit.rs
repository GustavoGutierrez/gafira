//! Fit to the box (FD-46): the largest size on a grid at which the text fits.

use gafira_geom::Size;

use crate::layout::{SizeRun, Stack, check_input_with, in_range, layout_with, to_px};
use crate::{TextError, TextLayout, TextStyle};

/// The grid step of the search, in pt.
const GRID: f64 = 0.25;
/// Allowed sizes, in pt.
const SIZES: (f64, f64) = (1.0, 1000.0);

/// The range of sizes of `Autofit::FitBox` (02 §3.2).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FitBox {
    /// Smallest size, in pt, in `[1, 1000]`.
    pub min_size: f64,
    /// Largest size, in pt, in `[min_size, 1000]`.
    pub max_size: f64,
}

impl Default for FitBox {
    /// 6 to 400 pt.
    fn default() -> Self {
        Self {
            min_size: 6.0,
            max_size: 400.0,
        }
    }
}

/// A piece of the text with its size. The pieces, in order, make up the text.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FitRun<'a> {
    /// The text of the run.
    pub text: &'a str,
    /// Its size now, in pt, in `[1, 1000]`: only the proportions between runs matter.
    pub size_pt: f64,
}

/// The result of the fit.
#[derive(Clone, Debug, PartialEq)]
pub struct FitResult {
    /// The size of each run, in pt.
    pub sizes_pt: Vec<f64>,
    /// The size of the largest run, in pt: the result of the search.
    pub size_pt: f64,
    /// The text laid out at those sizes. Its cluster ranges index the concatenated runs.
    pub layout: TextLayout,
    /// True when the text does not fit at the size found, which is then `min_size`.
    pub overflow: bool,
}

/// Finds the size at which the text fits the box (FD-46).
///
/// The search runs on the grid of the multiples of 0.25 pt between `fit.min_size` and
/// `fit.max_size`, with both ends in it. It gives the largest size it can bracket by
/// bisection: one that fits while the next on the grid does not, or `max_size` when that fits.
/// Wrapping is not monotone, so this is a local maximum of the grid, not always the greatest
/// size that fits. A text fits when its height is at most the height of the box (equality fits)
/// and no cluster is wider than the box (FD-41). If it does not fit at `min_size`, the result is
/// `min_size` with `overflow`. Empty text, or no run, changes nothing: the sizes stay and there
/// is no overflow.
///
/// With several runs, the result is the size of the largest one with text, and every other run
/// has the same proportion of it as before; the range binds that run only, so a smaller run may
/// scale below `min_size` and even below 1 pt. The size in `style` is the current size, used only when
/// there is no run; the other numbers of `style` apply to every run. The checks come in the
/// order of [`crate::layout_box`], then `fit` and the sizes of the runs; the font stack is
/// checked before the empty text returns. A size found that is not finite once laid out does
/// not fit.
pub fn fit_box(
    runs: &[FitRun<'_>],
    fonts: &[&[u8]],
    style: &TextStyle,
    dpi: f64,
    bounds: Size,
    fit: &FitBox,
) -> Result<FitResult, TextError> {
    let mut finite_numbers = vec![
        ("fit.min_size", fit.min_size),
        ("fit.max_size", fit.max_size),
    ];
    finite_numbers.extend(runs.iter().map(|r| ("runs.size_pt", r.size_pt)));
    let base_px = check_input_with(style, dpi, bounds, &finite_numbers)?;
    in_range("fit.min_size", fit.min_size, SIZES)?;
    in_range("fit.max_size", fit.max_size, SIZES)?;
    if fit.min_size > fit.max_size {
        return Err(TextError::OutOfRange {
            field: "fit.min_size",
        });
    }
    for run in runs {
        in_range("runs.size_pt", run.size_pt, SIZES)?;
    }
    if fonts.is_empty() {
        return Err(TextError::NoFont);
    }
    let mut stack = Stack::new(fonts)?;
    let text: String = runs.iter().map(|r| r.text).collect();
    let largest_any = runs.iter().map(|r| r.size_pt).fold(f64::MIN, f64::max);
    let height = bounds.height();
    let width = bounds.width();

    if text.is_empty() {
        let (size_pt, px) = if runs.is_empty() {
            (style.size_pt, base_px)
        } else {
            (largest_any, to_px(largest_any, dpi)?)
        };
        let layout = layout_with(&mut stack, "", style, px, &[], width)?;
        return Ok(FitResult {
            sizes_pt: runs.iter().map(|r| r.size_pt).collect(),
            size_pt,
            layout,
            overflow: false,
        });
    }

    // The largest run with text sets the size; a run with no text has nothing to fit.
    let largest = runs
        .iter()
        .filter(|r| !r.text.is_empty())
        .map(|r| r.size_pt)
        .fold(f64::MIN, f64::max);
    let mut ranges = Vec::with_capacity(runs.len());
    let mut at = 0;
    for run in runs {
        ranges.push(at..at + run.text.len());
        at += run.text.len();
    }
    let sizes_at =
        |r: f64| -> Vec<f64> { runs.iter().map(|run| r * (run.size_pt / largest)).collect() };
    // The layout at the size `r` for the largest run, and whether the text fits it. A size
    // whose layout is not finite is too big, and has no layout.
    let mut lay = |r: f64| -> Result<TextLayout, TextError> {
        let runs: Vec<SizeRun> = sizes_at(r)
            .into_iter()
            .zip(&ranges)
            .map(|(pt, range)| {
                Ok(SizeRun {
                    range: range.clone(),
                    size_px: to_px(pt, dpi)?,
                })
            })
            .collect::<Result<_, TextError>>()?;
        layout_with(&mut stack, &text, style, to_px(r, dpi)?, &runs, width)
    };
    let mut probe = |r: f64| -> Result<(Option<TextLayout>, bool), TextError> {
        match lay(r) {
            Ok(layout) => {
                let fits = f64::from(layout.height) <= height && !layout.overflow;
                Ok((Some(layout), fits))
            }
            Err(TextError::NonFinite { .. }) => Ok((None, false)),
            Err(e) => Err(e),
        }
    };

    let grid = grid(fit);
    let last = grid.len() - 1;
    let (mut index, mut kept, mut fits) = {
        let (layout, fits) = probe(grid[last])?;
        (last, layout, fits)
    };
    if !fits && last > 0 {
        let (layout, fits_min) = probe(grid[0])?;
        (index, kept, fits) = (0, layout, fits_min);
        if fits_min {
            let (mut lo, mut hi) = (0, last);
            while hi - lo > 1 {
                let mid = (lo + hi) / 2;
                let (layout, fits_mid) = probe(grid[mid])?;
                if fits_mid {
                    (lo, kept) = (mid, layout);
                } else {
                    hi = mid;
                }
            }
            index = lo;
        }
    }
    // A smallest size with no finite layout is an error, not a fit.
    let layout = match kept {
        Some(layout) => layout,
        None => lay(grid[index])?,
    };
    Ok(FitResult {
        sizes_pt: sizes_at(grid[index]),
        size_pt: grid[index],
        layout,
        overflow: !fits,
    })
}

/// The sizes of the search: `min_size`, the multiples of 0.25 above it and below `max_size`,
/// and `max_size`.
fn grid(fit: &FitBox) -> Vec<f64> {
    let mut sizes = vec![fit.min_size];
    if fit.max_size > fit.min_size {
        let first = (fit.min_size / GRID).floor() as u64 + 1;
        let end = (fit.max_size / GRID).ceil() as u64;
        sizes.extend((first..end).map(|k| k as f64 * GRID));
        sizes.push(fit.max_size);
    }
    sizes
}
