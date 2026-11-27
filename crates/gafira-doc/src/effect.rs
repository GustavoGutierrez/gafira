//! The effects a shape can carry instead of its fill (spec 065, D-103,
//! D-104): lights drawn from the box of the shape, all with the same
//! settings ([`Light`]).

use gafira_color::{Color, Rgba};
use serde::{Deserialize, Serialize};

/// The effect of a shape (spec 065). A shape with an effect does not draw
/// its fill, gradient, outline or text.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShapeEffect {
    /// A star of light: white core, rays and an optional coloured halo.
    Sparkle(Light),
    /// The sparkle plus the flare a camera lens casts across the page, from
    /// the light through the centre of the page.
    LensFlare(Light),
    /// «Destello de cine»: the long streak of light of an anamorphic lens,
    /// across the page along the angle, white in the middle and tinted by
    /// the halo. The rays are not used.
    Streak(Light),
    /// «Rayos de sol»: beams of light fanning out from the light across the
    /// page, twice as many as the rays, turned by the angle.
    SunRays(Light),
    /// «Luces bokeh»: as many soft discs of light as the rays, scattered
    /// over the box of the shape; the width sets their size and the angle
    /// turns the arrangement.
    Bokeh(Light),
    /// «Luz de luna»: a glowing moon over the box of the shape, its craters
    /// as many as the rays; the width is its phase (0 full, 1 a thin
    /// crescent), the angle where its dark side lies, the halo its glow.
    Moon(Light),
    /// «Foco»: a stage spotlight, a cone of light from the light across the
    /// page along the angle; the width opens the cone.
    Spotlight(Light),
    /// «Resplandor»: a soft round glow over the box of the shape, in the
    /// halo colour; the width makes it softer.
    Glow(Light),
    /// «Estrellas»: small four-pointed stars scattered over the box of the
    /// shape, three times as many as the rays; the width sets their size.
    Stars(Light),
    /// «Polvo de estrellas»: tiny specks of light, twelve times the rays,
    /// gathered along a band through the box along the angle.
    Stardust(Light),
    /// «Estela de cometa»: a bright head towards the angle with a tail of
    /// light streaming back; the width opens the tail.
    Comet(Light),
    /// «Aurora boreal»: waving curtains of green, blue and violet light
    /// across the page; the width makes them taller, the angle tilts them.
    Aurora(Light),
    /// «Fuga de luz»: warm patches of light coming in from the edge of the
    /// page the angle points to.
    LightLeak(Light),
    /// «Anillo luminoso»: a thin ring of light like an eclipse, with a
    /// corona and a bright bead where the angle points.
    Ring(Light),
}

/// The settings of a light (spec 065): the box of the shape gives its
/// centre and size.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Light {
    /// How many rays, four to sixteen.
    pub rays: u8,
    /// How thick the rays are, 0 thin to 1 thick.
    pub ray_width: f32,
    /// How bright the rays and the flare are, 0 to 1.
    pub intensity: f32,
    /// The direction of the first ray, clockwise from the right.
    pub angle_deg: f64,
    /// The colour of the outer halo; none, only the white core shows.
    /// Written only when set.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub halo: Option<Color>,
}

/// What is wrong with an effect (spec 065).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EffectProblem {
    /// Fewer than four or more than sixteen rays.
    Rays { count: u8 },
    /// A width or an intensity out of 0–1.
    OutOfRange,
    /// A number or a colour is not finite.
    NotFinite,
}

impl Light {
    pub const MIN_RAYS: u8 = 4;
    pub const MAX_RAYS: u8 = 16;

    /// The light «Brillo» is inserted with.
    pub const SPARKLE: Self = Self {
        rays: 6,
        ray_width: 0.3,
        intensity: 0.8,
        angle_deg: 0.0,
        halo: None,
    };

    /// The light «Destello de cine» is inserted with: a cold blue streak.
    pub const STREAK: Self = Self {
        rays: 6,
        ray_width: 0.3,
        intensity: 0.8,
        angle_deg: 0.0,
        halo: Some(Color::Srgb(Rgba::new(0.35, 0.6, 1.0, 1.0))),
    };

    /// The light «Rayos de sol» is inserted with: warm, twelve rays.
    pub const SUN_RAYS: Self = Self {
        rays: 12,
        ray_width: 0.4,
        intensity: 0.7,
        angle_deg: 0.0,
        halo: Some(Color::Srgb(Rgba::new(1.0, 0.85, 0.55, 1.0))),
    };

    /// The light «Luces bokeh» is inserted with: ten warm discs.
    pub const BOKEH: Self = Self {
        rays: 10,
        ray_width: 0.5,
        intensity: 0.7,
        angle_deg: 0.0,
        halo: Some(Color::Srgb(Rgba::new(1.0, 0.75, 0.4, 1.0))),
    };

    /// The light «Luz de luna» is inserted with: a waxing moon, cold glow.
    pub const MOON: Self = Self {
        rays: 6,
        ray_width: 0.55,
        intensity: 0.7,
        angle_deg: 200.0,
        halo: Some(Color::Srgb(Rgba::new(0.55, 0.7, 1.0, 1.0))),
    };

    /// The light «Foco» is inserted with: a warm white cone downwards.
    pub const SPOTLIGHT: Self = Self {
        rays: 6,
        ray_width: 0.35,
        intensity: 0.75,
        angle_deg: 90.0,
        halo: Some(Color::Srgb(Rgba::new(1.0, 0.92, 0.75, 1.0))),
    };

    /// The light «Resplandor» is inserted with: a soft pink glow.
    pub const GLOW: Self = Self {
        rays: 6,
        ray_width: 0.5,
        intensity: 0.8,
        angle_deg: 0.0,
        halo: Some(Color::Srgb(Rgba::new(1.0, 0.45, 0.75, 1.0))),
    };

    /// The light «Estrellas» is inserted with: a sky of cold white stars.
    pub const STARS: Self = Self {
        rays: 10,
        ray_width: 0.4,
        intensity: 0.85,
        angle_deg: 0.0,
        halo: Some(Color::Srgb(Rgba::new(0.8, 0.88, 1.0, 1.0))),
    };

    /// The light «Polvo de estrellas» is inserted with: cold white specks.
    pub const STARDUST: Self = Self {
        rays: 12,
        ray_width: 0.4,
        intensity: 0.85,
        angle_deg: -25.0,
        halo: Some(Color::Srgb(Rgba::new(0.85, 0.85, 1.0, 1.0))),
    };

    /// The light «Estela de cometa» is inserted with: a blue tail.
    pub const COMET: Self = Self {
        rays: 6,
        ray_width: 0.4,
        intensity: 0.85,
        angle_deg: -30.0,
        halo: Some(Color::Srgb(Rgba::new(0.5, 0.75, 1.0, 1.0))),
    };

    /// The light «Aurora boreal» is inserted with: violet at the top.
    pub const AURORA: Self = Self {
        rays: 6,
        ray_width: 0.5,
        intensity: 0.75,
        angle_deg: 0.0,
        halo: Some(Color::Srgb(Rgba::new(0.6, 0.3, 0.9, 1.0))),
    };

    /// The light «Fuga de luz» is inserted with: warm orange from the right.
    pub const LIGHT_LEAK: Self = Self {
        rays: 6,
        ray_width: 0.5,
        intensity: 0.8,
        angle_deg: -20.0,
        halo: Some(Color::Srgb(Rgba::new(1.0, 0.45, 0.15, 1.0))),
    };

    /// The light «Anillo luminoso» is inserted with: a warm ring.
    pub const RING: Self = Self {
        rays: 6,
        ray_width: 0.35,
        intensity: 0.9,
        angle_deg: -45.0,
        halo: Some(Color::Srgb(Rgba::new(1.0, 0.85, 0.6, 1.0))),
    };

    /// The light «Destello de lente» is inserted with: a warm red halo.
    pub const LENS_FLARE: Self = Self {
        rays: 6,
        ray_width: 0.3,
        intensity: 0.8,
        angle_deg: 15.0,
        halo: Some(Color::Srgb(Rgba::new(0.95, 0.35, 0.3, 1.0))),
    };

    fn problem(&self) -> Option<EffectProblem> {
        if !(Self::MIN_RAYS..=Self::MAX_RAYS).contains(&self.rays) {
            return Some(EffectProblem::Rays { count: self.rays });
        }
        let finite = self.ray_width.is_finite()
            && self.intensity.is_finite()
            && self.angle_deg.is_finite()
            && self.halo.is_none_or(Color::is_finite);
        if !finite {
            return Some(EffectProblem::NotFinite);
        }
        let unit = 0.0..=1.0;
        if !unit.contains(&self.ray_width) || !unit.contains(&self.intensity) {
            return Some(EffectProblem::OutOfRange);
        }
        None
    }
}

impl ShapeEffect {
    /// What is wrong with it, if anything.
    #[must_use]
    pub fn problem(&self) -> Option<EffectProblem> {
        match self {
            Self::Sparkle(light)
            | Self::LensFlare(light)
            | Self::Streak(light)
            | Self::SunRays(light)
            | Self::Bokeh(light)
            | Self::Moon(light)
            | Self::Spotlight(light)
            | Self::Glow(light)
            | Self::Stars(light)
            | Self::Stardust(light)
            | Self::Comet(light)
            | Self::Aurora(light)
            | Self::LightLeak(light)
            | Self::Ring(light) => light.problem(),
        }
    }

    /// The same effect with `light`.
    #[must_use]
    pub fn with_light(&self, light: Light) -> Self {
        match self {
            Self::Sparkle(_) => Self::Sparkle(light),
            Self::LensFlare(_) => Self::LensFlare(light),
            Self::Streak(_) => Self::Streak(light),
            Self::SunRays(_) => Self::SunRays(light),
            Self::Bokeh(_) => Self::Bokeh(light),
            Self::Moon(_) => Self::Moon(light),
            Self::Spotlight(_) => Self::Spotlight(light),
            Self::Glow(_) => Self::Glow(light),
            Self::Stars(_) => Self::Stars(light),
            Self::Stardust(_) => Self::Stardust(light),
            Self::Comet(_) => Self::Comet(light),
            Self::Aurora(_) => Self::Aurora(light),
            Self::LightLeak(_) => Self::LightLeak(light),
            Self::Ring(_) => Self::Ring(light),
        }
    }

    /// The light of the effect.
    #[must_use]
    pub fn light(&self) -> &Light {
        match self {
            Self::Sparkle(light)
            | Self::LensFlare(light)
            | Self::Streak(light)
            | Self::SunRays(light)
            | Self::Bokeh(light)
            | Self::Moon(light)
            | Self::Spotlight(light)
            | Self::Glow(light)
            | Self::Stars(light)
            | Self::Stardust(light)
            | Self::Comet(light)
            | Self::Aurora(light)
            | Self::LightLeak(light)
            | Self::Ring(light) => light,
        }
    }
}
