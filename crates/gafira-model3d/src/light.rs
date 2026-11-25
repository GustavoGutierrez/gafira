//! The lighting presets of the 3D panel (Suave, Estudio, Dramática): fixed lights in view
//! space, so the light stays put while the model turns.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Lighting {
    #[default]
    Soft,
    Studio,
    Dramatic,
}

impl Lighting {
    pub const ALL: [Lighting; 3] = [Self::Soft, Self::Studio, Self::Dramatic];

    /// Stable id stored in the document.
    pub fn id(self) -> &'static str {
        match self {
            Self::Soft => "soft",
            Self::Studio => "studio",
            Self::Dramatic => "dramatic",
        }
    }

    /// The preset of a stored id; unknown ids fall back to the default.
    pub fn from_id(id: &str) -> Self {
        Self::ALL
            .into_iter()
            .find(|l| l.id() == id)
            .unwrap_or_default()
    }

    pub(crate) fn rig(self) -> Rig {
        match self {
            Self::Soft => Rig {
                sky: [0.62, 0.64, 0.68],
                ground: [0.36, 0.34, 0.33],
                lights: [
                    Light {
                        dir: [-0.45, 0.65, 0.62],
                        color: [0.78, 0.76, 0.72],
                    },
                    Light {
                        dir: [0.6, 0.15, 0.78],
                        color: [0.28, 0.29, 0.31],
                    },
                    Light {
                        dir: [0.0, 0.4, -0.9],
                        color: [0.12, 0.12, 0.12],
                    },
                ],
                specular: 0.55,
            },
            Self::Studio => Rig {
                sky: [0.42, 0.44, 0.48],
                ground: [0.2, 0.19, 0.19],
                lights: [
                    Light {
                        dir: [-0.55, 0.62, 0.56],
                        color: [1.12, 1.08, 1.0],
                    },
                    Light {
                        dir: [0.7, 0.2, 0.68],
                        color: [0.38, 0.4, 0.44],
                    },
                    Light {
                        dir: [0.15, 0.55, -0.82],
                        color: [0.55, 0.55, 0.58],
                    },
                ],
                specular: 1.0,
            },
            Self::Dramatic => Rig {
                sky: [0.1, 0.1, 0.12],
                ground: [0.04, 0.04, 0.04],
                lights: [
                    Light {
                        dir: [-0.85, 0.45, 0.28],
                        color: [1.55, 1.42, 1.25],
                    },
                    Light {
                        dir: [0.75, 0.1, 0.65],
                        color: [0.06, 0.07, 0.09],
                    },
                    Light {
                        dir: [0.65, 0.4, -0.65],
                        color: [0.85, 0.88, 1.0],
                    },
                ],
                specular: 1.25,
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
