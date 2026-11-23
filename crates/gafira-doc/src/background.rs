//! The background of a page (spec 069): a colour, a gradient, a pattern of
//! two colours or an image, under every element of the page.

use gafira_color::Color;
use serde::{Deserialize, Serialize};

use crate::timeline::AssetRef;

/// The patterns a background repeats (spec 069).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PatternKind {
    Dots,
    Grid,
    Stripes,
    Checks,
    Zigzag,
    Crosses,
    /// Paper patterns (spec 069, owner request): bricks, a grid with
    /// diagonals, hexagons, isometric (±30°), diamonds (±45°) and
    /// triangles.
    Bricks,
    GridDiagonal,
    Hexagons,
    Isometric,
    Diamonds,
    Triangles,
    /// Bricks twice the size, and a fine grid (quadrille).
    BricksLarge,
    GridFine,
}

impl PatternKind {
    pub const ALL: [Self; 14] = [
        Self::Dots,
        Self::Grid,
        Self::Stripes,
        Self::Checks,
        Self::Zigzag,
        Self::Crosses,
        Self::Bricks,
        Self::GridDiagonal,
        Self::Hexagons,
        Self::Isometric,
        Self::Diamonds,
        Self::Triangles,
        Self::BricksLarge,
        Self::GridFine,
    ];
}

/// How a background image covers its page (spec 069).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BackgroundFit {
    /// «Rellenar»: covers the page; what overflows is cut.
    #[default]
    Fill,
    /// «Encajar»: the whole image shows, centred.
    Fit,
    /// «Mosaico»: repeated at its own size from the top left corner.
    Tile,
}

/// The background of a page (spec 069).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PageBackground {
    Solid(Color),
    Gradient(crate::Gradient),
    Pattern {
        kind: PatternKind,
        color: Color,
        back: Color,
    },
    Image {
        asset: AssetRef,
        natural: (u32, u32),
        #[serde(default)]
        fit: BackgroundFit,
    },
}

impl PageBackground {
    /// Whether its numbers can be drawn: finite colours, a valid gradient,
    /// an image of some size. Whether the image is in the asset table is
    /// checked with the document (P7).
    #[must_use]
    pub fn is_valid(&self) -> bool {
        match self {
            Self::Solid(color) => color.is_finite(),
            Self::Gradient(gradient) => gradient.problem().is_none(),
            Self::Pattern { color, back, .. } => color.is_finite() && back.is_finite(),
            Self::Image { natural, .. } => natural.0 > 0 && natural.1 > 0,
        }
    }

    /// The asset an image background shows.
    #[must_use]
    pub fn asset(&self) -> Option<AssetRef> {
        match self {
            Self::Image { asset, .. } => Some(*asset),
            _ => None,
        }
    }
}
