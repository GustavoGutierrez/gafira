//! The lighting presets of the 3D panel (Estudio, Cálida, Fría, Oscura, Suave, Natural): fixed lights in view
//! space, so the light stays put while the model turns.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Lighting {
    /// Bright and neutral.
    #[default]
    Studio,
    Warm,
    Cool,
    /// Low key with a strong side light.
    Dark,
    Soft,
    /// Daylight: a warm sun and a blue sky.
    Natural,
}

const fn light(dir: [f32; 3], color: [f32; 3]) -> Light {
    Light { dir, color }
}

impl Lighting {
    pub const ALL: [Lighting; 6] = [
        Self::Studio,
        Self::Warm,
        Self::Cool,
        Self::Dark,
        Self::Soft,
        Self::Natural,
    ];

    /// Stable id stored in the document.
    pub fn id(self) -> &'static str {
        match self {
            Self::Studio => "studio",
            Self::Warm => "warm",
            Self::Cool => "cool",
            Self::Dark => "dark",
            Self::Soft => "soft",
            Self::Natural => "natural",
        }
    }

    /// The preset of a stored id; unknown ids fall back to the default.
    pub fn from_id(id: &str) -> Self {
        if id == "dramatic" {
            return Self::Dark;
        }
        Self::ALL
            .into_iter()
            .find(|l| l.id() == id)
            .unwrap_or_default()
    }

    pub(crate) fn rig(self) -> Rig {
        match self {
            // A bright key from the upper left, a soft fill and a rim: the model reads
            // clearly with no dark side.
            Self::Studio => Rig {
                sky: [0.66, 0.68, 0.72],
                ground: [0.38, 0.37, 0.36],
                lights: [
                    light([-0.5, 0.62, 0.6], [1.25, 1.22, 1.16]),
                    light([0.7, 0.18, 0.69], [0.55, 0.56, 0.6]),
                    light([0.2, 0.55, -0.81], [0.6, 0.6, 0.62]),
                ],
                specular: 0.9,
            },
            Self::Warm => Rig {
                sky: [0.62, 0.52, 0.4],
                ground: [0.34, 0.25, 0.18],
                lights: [
                    light([-0.55, 0.6, 0.58], [1.42, 1.08, 0.72]),
                    light([0.7, 0.15, 0.7], [0.42, 0.3, 0.22]),
                    light([0.3, 0.5, -0.81], [0.7, 0.48, 0.3]),
                ],
                specular: 0.9,
            },
            Self::Cool => Rig {
                sky: [0.46, 0.56, 0.72],
                ground: [0.2, 0.25, 0.32],
                lights: [
                    light([-0.5, 0.62, 0.6], [0.98, 1.12, 1.38]),
                    light([0.7, 0.18, 0.69], [0.3, 0.38, 0.52]),
                    light([0.2, 0.55, -0.81], [0.5, 0.66, 0.92]),
                ],
                specular: 1.0,
            },
            Self::Dark => Rig {
                sky: [0.1, 0.1, 0.12],
                ground: [0.04, 0.04, 0.04],
                lights: [
                    light([-0.85, 0.45, 0.28], [1.55, 1.42, 1.25]),
                    light([0.75, 0.1, 0.65], [0.06, 0.07, 0.09]),
                    light([0.65, 0.4, -0.65], [0.85, 0.88, 1.0]),
                ],
                specular: 1.25,
            },
            Self::Soft => Rig {
                sky: [0.62, 0.64, 0.68],
                ground: [0.36, 0.34, 0.33],
                lights: [
                    light([-0.45, 0.65, 0.62], [0.78, 0.76, 0.72]),
                    light([0.6, 0.15, 0.78], [0.28, 0.29, 0.31]),
                    light([0.0, 0.4, -0.9], [0.12, 0.12, 0.12]),
                ],
                specular: 0.55,
            },
            // A high sun from the upper right, a blue sky above and warm ground light below.
            Self::Natural => Rig {
                sky: [0.4, 0.54, 0.86],
                ground: [0.36, 0.3, 0.2],
                lights: [
                    light([0.45, 0.8, 0.4], [1.62, 1.4, 1.02]),
                    light([-0.7, 0.2, 0.68], [0.3, 0.36, 0.48]),
                    light([-0.2, 0.3, -0.93], [0.35, 0.32, 0.26]),
                ],
                specular: 0.85,
            },
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct Light {
    /// From the surface towards the light, view space (normalized when used).
    pub dir: [f32; 3],
    pub color: [f32; 3],
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct Rig {
    pub sky: [f32; 3],
    pub ground: [f32; 3],
    pub lights: [Light; 3],
    pub specular: f32,
}
