//! Tile grid (ARQ-121).

/// Side of a tile in pixels (FIJADA, 02 §6.1).
pub const TILE_SIZE: u32 = 256;

/// How many tiles cover an image.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TileGrid {
    /// Tiles across.
    pub columns: u32,
    /// Tiles down.
    pub rows: u32,
}

impl TileGrid {
    /// Total tiles.
    #[must_use]
    pub fn count(&self) -> u64 {
        u64::from(self.columns) * u64::from(self.rows)
    }
}

/// The tile grid of a `width × height` image. An empty image has no tiles.
#[must_use]
pub fn tile_grid(width: u32, height: u32) -> TileGrid {
    TileGrid {
        columns: width.div_ceil(TILE_SIZE),
        rows: height.div_ceil(TILE_SIZE),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ARQ-121: 256 px is one tile, 257 px is two; both axes; empty has none.
    #[test]
    fn arq_121_tile_counts_at_the_256_boundary() {
        let cases: [(u32, u32, u32, u32); 12] = [
            (256, 256, 1, 1),
            (256, 1, 1, 1),
            (257, 1, 2, 1),
            (1, 257, 1, 2),
            (1, 256, 1, 1),
            (512, 512, 2, 2),
            (513, 512, 3, 2),
            (1, 1, 1, 1),
            (255, 255, 1, 1),
            (0, 0, 0, 0),
            (0, 300, 0, 2),
            (300, 0, 2, 0),
        ];
        for (w, h, columns, rows) in cases {
            let g = tile_grid(w, h);
            assert_eq!(g, TileGrid { columns, rows }, "{w}x{h}");
            assert_eq!(g.count(), u64::from(columns) * u64::from(rows), "{w}x{h}");
        }
        assert_eq!(tile_grid(257, 1).count(), 2);
        assert_eq!(tile_grid(256, 1).count(), 1);
        assert_eq!(TILE_SIZE, 256);
    }

    // ARQ-121: the extreme size does not overflow.
    #[test]
    fn arq_121_tile_grid_of_the_largest_image() {
        let g = tile_grid(u32::MAX, u32::MAX);
        assert_eq!((g.columns, g.rows), (16_777_216, 16_777_216));
        assert_eq!(g.count(), 1u64 << 48);
    }
}
