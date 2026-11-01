//! Guide positions and the guide generator (FD-03, FD-04).

use gafira_doc::{Guide, GuideAxis};
use gafira_geom::{Length, Unit};

use crate::LayoutError;
use crate::error::finite;
use crate::rects::convert;

/// A guide at a typed position, and that position shown in the ruler unit.
///
/// The guide keeps `written` as typed: same number, same unit, never rounded
/// (FD-03). The second value is the same distance in `ruler`, converted with
/// `dpi`, for the on-screen readout. A non-finite position or DPI, or a
/// conversion that overflows, is an error and no guide is made.
pub fn place_guide(
    axis: GuideAxis,
    written: Length,
    ruler: Unit,
    dpi: f64,
) -> Result<(Guide, Length), LayoutError> {
    finite("position", written.value())?;
    finite("dpi", dpi)?;
    let shown = convert("position", written, ruler, dpi)?;
    let guide = Guide {
        axis,
        position: written,
        locked: false,
    };
    Ok((guide, Length::new(shown, ruler)))
}
