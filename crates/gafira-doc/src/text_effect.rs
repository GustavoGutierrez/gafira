//! The effects of a text (spec 089, FD-43): data over words that stay words.

use gafira_color::{Color, Rgba};
use serde::{Deserialize, Serialize};

/// What an effect does to the letters.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TextEffectKind {
    /// «Paralela»: a shadow.
    Drop,
    /// «Brillo».
    Glow,
    /// «Eco»: copies behind, each fainter.
    Echo,
    /// «Contorno», around the letters.
    Outline,
    /// «Fondo»: a box behind each line.
    Background,
    /// «Desalineado»: the inside away from the outline.
    Offset,
    /// «Sin relleno»: the outline alone.
    Hollow,
    /// «Neón».
    Neon,
    /// «Distorsión»: two copies apart, of two colours.
    Glitch,
}

/// A number of an effect.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextEffectNumber {
    Thickness,
    Distance,
    Angle,
    Blur,
    Opacity,
    Roundness,
    Spread,
    Intensity,
}

impl TextEffectKind {
    pub const ALL: [Self; 9] = [
        Self::Drop,
        Self::Glow,
        Self::Echo,
        Self::Outline,
        Self::Background,
        Self::Offset,
        Self::Hollow,
        Self::Neon,
        Self::Glitch,
    ];

    /// The numbers that change it, in the order they are shown.
    #[must_use]
    pub const fn numbers(self) -> &'static [TextEffectNumber] {
        use TextEffectNumber as N;
        match self {
            Self::Drop => &[N::Angle, N::Distance, N::Blur, N::Opacity],
            Self::Glow | Self::Neon => &[N::Intensity],
            Self::Echo | Self::Glitch => &[N::Angle, N::Distance],
            Self::Outline | Self::Hollow => &[N::Thickness],
            Self::Background => &[N::Roundness, N::Spread, N::Opacity],
            Self::Offset => &[N::Thickness, N::Distance, N::Angle],
        }
    }

    /// How many colours of its own it has: the rest is of the text's.
    #[must_use]
    pub const fn colors(self) -> usize {
        match self {
            Self::Hollow => 0,
            Self::Glitch => 2,
            _ => 1,
        }
    }

    /// Whether the inside of the letters is not drawn.
    #[must_use]
    pub const fn hides_fill(self) -> bool {
        matches!(self, Self::Offset | Self::Hollow)
    }
}

fn none(v: &f64) -> bool {
    *v == 0.0
}

/// An effect of a text: numbers from 0 to 100, shares of the size of its
/// letters, and the direction in degrees, −180 to 180, clockwise from the
/// right. Those its kind does not use are 0.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct TextEffect {
    pub kind: TextEffectKind,
    pub color: Color,
    /// The second one of «Distorsión».
    pub second: Color,
    #[serde(default, skip_serializing_if = "none")]
    pub thickness: f64,
    #[serde(default, skip_serializing_if = "none")]
    pub distance: f64,
    #[serde(default, skip_serializing_if = "none")]
    pub angle_deg: f64,
    #[serde(default, skip_serializing_if = "none")]
    pub blur: f64,
    #[serde(default, skip_serializing_if = "none")]
    pub opacity: f64,
    #[serde(default, skip_serializing_if = "none")]
    pub roundness: f64,
    #[serde(default, skip_serializing_if = "none")]
    pub spread: f64,
    #[serde(default, skip_serializing_if = "none")]
    pub intensity: f64,
}

impl TextEffect {
    /// The most effects of a text.
    pub const MAX: usize = 4;

    /// The effect `kind` starts with.
    #[must_use]
    pub fn new(kind: TextEffectKind) -> Self {
        const fn rgb(r: f32, g: f32, b: f32) -> Color {
            Color::Srgb(Rgba::new(r / 255.0, g / 255.0, b / 255.0, 1.0))
        }
        let ink = rgb(24.0, 36.0, 59.0);
        let mint = rgb(62.0, 191.0, 168.0);
        let coral = rgb(255.0, 107.0, 87.0);
        let start = Self {
            kind,
            color: ink,
            second: coral,
            thickness: 0.0,
            distance: 0.0,
            angle_deg: 0.0,
            blur: 0.0,
            opacity: 0.0,
            roundness: 0.0,
            spread: 0.0,
            intensity: 0.0,
        };
        use TextEffectKind as K;
        match kind {
            K::Drop => Self {
                distance: 20.0,
                angle_deg: 45.0,
                blur: 10.0,
                opacity: 40.0,
                ..start
            },
            K::Glow => Self {
                color: coral,
                intensity: 50.0,
                ..start
            },
            K::Echo => Self {
                distance: 25.0,
                angle_deg: 45.0,
                ..start
            },
            K::Outline => Self {
                color: mint,
                thickness: 40.0,
                ..start
            },
            K::Background => Self {
                color: mint,
                roundness: 50.0,
                spread: 50.0,
                opacity: 100.0,
                ..start
            },
            K::Offset => Self {
                color: mint,
                thickness: 30.0,
                distance: 40.0,
                angle_deg: 45.0,
                ..start
            },
            K::Hollow => Self {
                thickness: 30.0,
                ..start
            },
            K::Neon => Self {
                color: coral,
                intensity: 60.0,
                ..start
            },
            K::Glitch => Self {
                color: mint,
                distance: 30.0,
                ..start
            },
        }
    }

    /// A number of it.
    #[must_use]
    pub fn number(&mut self, number: TextEffectNumber) -> &mut f64 {
        use TextEffectNumber as N;
        match number {
            N::Thickness => &mut self.thickness,
            N::Distance => &mut self.distance,
            N::Angle => &mut self.angle_deg,
            N::Blur => &mut self.blur,
            N::Opacity => &mut self.opacity,
            N::Roundness => &mut self.roundness,
            N::Spread => &mut self.spread,
            N::Intensity => &mut self.intensity,
        }
    }

    #[must_use]
    pub fn is_valid(&self) -> bool {
        let color = |c: Color| c.is_finite() && (0.0..=1.0).contains(&c.alpha());
        let shares = [
            self.thickness,
            self.distance,
            self.blur,
            self.opacity,
            self.roundness,
            self.spread,
            self.intensity,
        ];
        color(self.color)
            && color(self.second)
            && shares.iter().all(|v| (0.0..=100.0).contains(v))
            && (-180.0..=180.0).contains(&self.angle_deg)
    }
}
