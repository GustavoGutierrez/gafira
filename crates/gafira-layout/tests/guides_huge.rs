//! FD-04 (d), ARQ-110: the largest count is refused by the column width rule, not walked.
//!
//! Apart from `guides_generate` so that, if the rule breaks, the quick tests there fail
//! first: this one would loop over four billion divisions.

use gafira_layout::guides::generate;
use gafira_layout::{GuideSpec, LayoutError, Length, Rect, Unit};

fn spec(columns: u32, rows: u32) -> GuideSpec {
    let zero = Length::new(0.0, Unit::Px);
    GuideSpec {
        columns,
        rows,
        gap: zero,
        margin: zero,
    }
}

#[test]
fn fd_04_d_a_count_of_u32_max_is_a_column_width_error() {
    let side = 16_384.0;
    let page = Rect::new(0.0, 0.0, side, side);
    let out = |field| Err(LayoutError::OutOfRange { field });
    assert_eq!(generate(&spec(u32::MAX, 0), page, 96.0), out("columns"));
    assert_eq!(generate(&spec(0, u32::MAX), page, 96.0), out("rows"));
    let big = Rect::new(0.0, 0.0, side.next_up(), side);
    assert_eq!(generate(&spec(u32::MAX, 0), big, 96.0), out("page"));
}
