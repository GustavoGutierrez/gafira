//! Integer rectangle for tiles and regions of interest (C10).

/// A half-open integer rectangle: `x0 <= x < x1` and `y0 <= y < y1`.
///
/// Signed, because a region of interest can leave the image. An inverted
/// rectangle (`x1 < x0` or `y1 < y0`) is legal plain data and is empty.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct IRect {
    pub x0: i32,
    pub y0: i32,
    pub x1: i32,
    pub y1: i32,
}

impl IRect {
    #[must_use]
    pub const fn new(x0: i32, y0: i32, x1: i32, y1: i32) -> Self {
        Self { x0, y0, x1, y1 }
    }

    #[must_use]
    pub const fn width(self) -> u32 {
        if self.x1 > self.x0 {
            self.x1.abs_diff(self.x0)
        } else {
            0
        }
    }

    #[must_use]
    pub const fn height(self) -> u32 {
        if self.y1 > self.y0 {
            self.y1.abs_diff(self.y0)
        } else {
            0
        }
    }

    #[must_use]
    pub const fn area(self) -> u64 {
        self.width() as u64 * self.height() as u64
    }

    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.x1 <= self.x0 || self.y1 <= self.y0
    }

    #[must_use]
    pub const fn contains(self, x: i32, y: i32) -> bool {
        self.x0 <= x && x < self.x1 && self.y0 <= y && y < self.y1
    }

    /// The overlap: max of the lows, min of the highs. Disjoint inputs give an
    /// empty, possibly inverted, rectangle.
    #[must_use]
    pub fn intersect(self, other: IRect) -> IRect {
        IRect::new(
            self.x0.max(other.x0),
            self.y0.max(other.y0),
            self.x1.min(other.x1),
            self.y1.min(other.y1),
        )
    }

    /// Moves each side outward by `dx` or `dy`, saturating; a negative value
    /// deflates.
    #[must_use]
    pub fn inflate(self, dx: i32, dy: i32) -> IRect {
        IRect::new(
            self.x0.saturating_sub(dx),
            self.y0.saturating_sub(dy),
            self.x1.saturating_add(dx),
            self.y1.saturating_add(dy),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MIN: i32 = i32::MIN;
    const MAX: i32 = i32::MAX;

    #[test]
    fn width_and_height_are_the_extent_and_never_overflow() {
        let r = IRect::new(-3, 10, 7, 12);
        assert_eq!((r.width(), r.height()), (10, 2));
        let full = IRect::new(MIN, MIN, MAX, MAX);
        assert_eq!((full.width(), full.height()), (u32::MAX, u32::MAX));
        let half = IRect::new(MIN, 0, 0, 0);
        assert_eq!(half.width(), 1 << 31);
    }

    #[test]
    fn an_inverted_rect_is_empty_with_zero_extent_and_contains_nothing() {
        for r in [
            IRect::new(5, 0, 2, 10),
            IRect::new(0, 5, 10, 2),
            IRect::new(MAX, MAX, MIN, MIN),
        ] {
            assert_eq!(r.width().min(r.height()), 0);
            assert_eq!(r.area(), 0);
            assert!(r.is_empty());
            for (x, y) in [(3, 3), (5, 5), (2, 2), (0, 0), (MAX, MAX), (MIN, MIN)] {
                assert!(!r.contains(x, y), "{r:?} contains ({x}, {y})");
            }
        }
        let inv_x = IRect::new(5, 0, 2, 10);
        assert_eq!((inv_x.width(), inv_x.height()), (0, 10));
        let inv_y = IRect::new(0, 5, 10, 2);
        assert_eq!((inv_y.width(), inv_y.height()), (10, 0));
    }

    #[test]
    fn a_zero_extent_rect_is_empty_and_a_normal_one_is_not() {
        assert!(IRect::new(4, 0, 4, 9).is_empty());
        assert!(IRect::new(0, 4, 9, 4).is_empty());
        assert!(!IRect::new(0, 0, 1, 1).is_empty());
        assert!(!IRect::new(MIN, MIN, MAX, MAX).is_empty());
    }

    #[test]
    fn area_is_computed_in_u64() {
        assert_eq!(IRect::new(1, 2, 4, 7).area(), 15);
        let full = IRect::new(MIN, MIN, MAX, MAX);
        assert_eq!(full.area(), u64::from(u32::MAX) * u64::from(u32::MAX));
        assert_eq!(IRect::new(0, 0, 70_000, 70_000).area(), 4_900_000_000);
    }

    #[test]
    fn contains_is_half_open() {
        let r = IRect::new(2, 3, 5, 7);
        let table = [
            (2, 3, true),
            (4, 6, true),
            (1, 3, false),
            (2, 2, false),
            (5, 3, false),
            (2, 7, false),
            (5, 7, false),
            (4, 7, false),
            (5, 6, false),
            (3, 5, true),
        ];
        for (x, y, want) in table {
            assert_eq!(r.contains(x, y), want, "({x}, {y})");
        }
    }

    #[test]
    fn intersect_takes_the_max_of_lows_and_min_of_highs() {
        let a = IRect::new(0, 0, 10, 10);
        let b = IRect::new(4, 2, 14, 8);
        assert_eq!(a.intersect(b), IRect::new(4, 2, 10, 8));
        assert_eq!(b.intersect(a), IRect::new(4, 2, 10, 8));
        let inner = IRect::new(1, 1, 3, 3);
        assert_eq!(a.intersect(inner), inner);
        assert_eq!(a.intersect(a), a);
        // Abutting tiles share no pixel.
        assert!(a.intersect(IRect::new(10, 0, 20, 10)).is_empty());
        // Disjoint inputs give an empty, possibly inverted, rect.
        let far = a.intersect(IRect::new(20, 30, 25, 40));
        assert!(far.is_empty());
        assert_eq!(far, IRect::new(20, 30, 10, 10));
    }

    #[test]
    fn inflate_moves_each_side_outward_and_saturates() {
        let r = IRect::new(10, 20, 30, 40);
        assert_eq!(r.inflate(2, 5), IRect::new(8, 15, 32, 45));
        assert_eq!(r.inflate(-2, -5), IRect::new(12, 25, 28, 35));
        assert_eq!(r.inflate(0, 0), r);
        assert_eq!(r.inflate(-20, 0), IRect::new(30, 20, 10, 40));
        assert!(r.inflate(-20, 0).is_empty());
        let edge = IRect::new(MIN + 1, MIN + 1, MAX - 1, MAX - 1);
        assert_eq!(edge.inflate(5, 7), IRect::new(MIN, MIN, MAX, MAX));
        let full = IRect::new(MIN, MIN, MAX, MAX);
        assert_eq!(full.inflate(MAX, MAX), full);
        // Deflating at the extremes saturates at the other end.
        assert_eq!(
            IRect::new(MAX, MAX, MIN, MIN).inflate(MIN, MIN),
            IRect::new(MAX, MAX, MIN, MIN)
        );
        assert_eq!(
            IRect::new(MIN, MIN, MAX, MAX).inflate(MIN, MIN),
            IRect::new(0, 0, -1, -1)
        );
    }
}
