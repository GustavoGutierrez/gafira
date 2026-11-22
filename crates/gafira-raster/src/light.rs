//! The pixels of the lights of spec 065 (D-103): «Brillo», a star of light,
//! and «Destello de lente», the same star plus the flare a lens casts from
//! the light through the centre of the picture.
//!
//! Both are drawn the same way by the export (as an image op) and by the
//! editor canvas (as a texture). The pixels are sRGB, premultiplied, and the
//! alpha of each one is its brightest channel: drawn with plain
//! source-over, light adds to a dark background almost like the «Trama»
//! (screen) blend, keeps a valid premultiplied pixel on a transparent one
//! and turns white on white.

use std::f64::consts::TAU;

use gafira_color::{Color, ColorSpaceId};
use gafira_doc::{Light, ShapeEffect};

use crate::{ImageBuf, PixelBuf};

/// Where the light sits in the picture, in its pixels: the centre and the
/// radius of the box of the shape.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LightSpot {
    pub centre: (f64, f64),
    pub radius: f64,
}

/// The halo of a lens flare without a colour of its own: warm red.
const FLARE_DEFAULT: [f64; 3] = [0.9, 0.3, 0.25];

/// A ghost of the flare: where on the line from the light through the
/// centre (0 the light, 1 the centre, 2 the mirror point), its size as a
/// share of the light radius, its colour and whether it is a rainbow ring.
const GHOSTS: [(f64, f64, [f64; 3], bool); 7] = [
    (0.55, 0.35, [0.35, 0.55, 0.95], false),
    (0.80, 0.18, [0.95, 0.55, 0.25], false),
    (1.05, 0.10, [0.40, 0.95, 0.60], false),
    (1.45, 0.60, [0.95, 0.60, 0.20], false),
    (1.60, 0.25, [0.90, 0.70, 0.30], false),
    (2.00, 0.45, [0.30, 0.80, 0.85], false),
    (2.30, 1.40, [1.0, 1.0, 1.0], true),
];

/// Whether `effect` is drawn over the whole page (its light sits in it) or
/// over the box of its shape alone.
#[must_use]
pub fn spans_page(effect: &ShapeEffect) -> bool {
    matches!(
        effect,
        ShapeEffect::LensFlare(_) | ShapeEffect::Streak(_) | ShapeEffect::SunRays(_)
    )
}

/// The picture of `effect`, `width` × `height`, with its light at `spot`:
/// the page when [`spans_page`], else the box of the shape.
#[must_use]
pub fn light_picture(effect: &ShapeEffect, width: u32, height: u32, spot: LightSpot) -> ImageBuf {
    match effect {
        ShapeEffect::Sparkle(light) => sparkle(light, width, height, spot),
        ShapeEffect::LensFlare(light) => lens_flare(light, width, height, spot),
        ShapeEffect::Streak(light) => streak(light, width, height, spot),
        ShapeEffect::SunRays(light) => sun_rays(light, width, height, spot),
        ShapeEffect::Bokeh(light) => bokeh(light, width, height, spot),
    }
}

/// «Destello de cine» (D-104): a white core and a long thin streak along
/// the angle, tinted by the halo away from the core, fading towards the
/// ends of the picture.
#[must_use]
pub fn streak(light: &Light, width: u32, height: u32, spot: LightSpot) -> ImageBuf {
    let r = spot.radius.max(1.0);
    let tint = light.halo.map_or([0.35, 0.6, 1.0], rgb);
    let reach = f64::from(width.max(height)) * 0.6;
    let thick = r * (0.06 + 0.3 * f64::from(light.ray_width.clamp(0.0, 1.0)));
    let strength = f64::from(light.intensity.clamp(0.0, 1.0));
    let (uy, ux) = light.angle_deg.to_radians().sin_cos();
    picture(width, height, |x, y| {
        let (dx, dy) = (x - spot.centre.0, y - spot.centre.1);
        let d = dx.hypot(dy);
        let along = (dx * ux + dy * uy).abs();
        let across = (dx * uy - dy * ux).abs();
        let line = gauss(across, thick) * (-along / reach).exp() * strength;
        let thin = 0.5 * gauss(across, thick * 0.25) * (-along / (reach * 0.4)).exp() * strength;
        // A wide soft glow round the streak, as a real lens spreads it.
        let wide = 0.3 * gauss(across, thick * 5.0) * (-along / (reach * 0.5)).exp() * strength;
        let core = gauss(d, 0.15 * r) + 0.5 * gauss(d, 0.5 * r);
        // White near the core, the tint further out.
        let white = smoothstep(0.6 * r, 0.0, along);
        let mut out = [0.0; 3];
        for (c, t) in out.iter_mut().zip(tint) {
            let colour = white + (1.0 - white) * t;
            *c = core + (line + thin + wide) * colour;
        }
        out
    })
}

/// «Rayos de sol» (D-104): beams fanning out from the light, of uneven
/// width and strength, fading with the distance; a soft glow round the
/// light, in the halo colour.
#[must_use]
pub fn sun_rays(light: &Light, width: u32, height: u32, spot: LightSpot) -> ImageBuf {
    let r = spot.radius.max(1.0);
    let tint = light.halo.map_or([1.0, 0.85, 0.55], rgb);
    let beams = u64::from(light.rays.max(1)) * 2;
    let spread = 0.02 + 0.08 * f64::from(light.ray_width.clamp(0.0, 1.0));
    let strength = f64::from(light.intensity.clamp(0.0, 1.0));
    let reach = f64::from(width).hypot(f64::from(height)) * 0.7;
    let turn = light.angle_deg.to_radians();
    let shape: Vec<(f64, f64, f64)> = (0..beams)
        .map(|k| {
            let at = turn + (k as f64 + 0.5 * random(k, 1)) * TAU / beams as f64;
            let wide = spread * (0.5 + random(k, 2));
            let bright = 0.35 + 0.65 * random(k, 3);
            (at, wide, bright)
        })
        .collect();
    picture(width, height, |x, y| {
        let (dx, dy) = (x - spot.centre.0, y - spot.centre.1);
        let d = dx.hypot(dy);
        let angle = dy.atan2(dx);
        let mut beam = 0.0;
        for &(at, wide, bright) in &shape {
            let off = (angle - at + TAU / 2.0).rem_euclid(TAU) - TAU / 2.0;
            beam += bright * gauss(off, wide);
        }
        // No beam at the very light: it starts as it leaves the glow.
        let rays = beam * strength * 0.55 * (-d / reach).exp() * smoothstep(0.0, 0.8 * r, d);
        let glow = gauss(d, 0.5 * r) + 0.35 * gauss(d, 1.6 * r);
        let mut out = [0.0; 3];
        for (c, t) in out.iter_mut().zip(tint) {
            *c = glow * (0.5 + 0.5 * t) + rays * t;
        }
        out
    })
}

/// «Luces bokeh» (D-104): as many soft discs as the rays, in places and
/// sizes that depend on their number only, turned round the centre by the
/// angle; each a little brighter at its rim, in shades of the halo.
#[must_use]
pub fn bokeh(light: &Light, width: u32, height: u32, spot: LightSpot) -> ImageBuf {
    let r = spot.radius.max(1.0);
    let tint = light.halo.map_or([1.0, 0.75, 0.4], rgb);
    let strength = f64::from(light.intensity.clamp(0.0, 1.0));
    let size = 0.1 + 0.25 * f64::from(light.ray_width.clamp(0.0, 1.0));
    let (sin, cos) = light.angle_deg.to_radians().sin_cos();
    let discs: Vec<Disc> = (0..u64::from(light.rays))
        .map(|k| {
            // A point of the disc of radius `r`, evenly by area.
            let (a, b) = (random(k, 4) * TAU, random(k, 5).sqrt() * 0.8);
            let (px, py) = (a.cos() * b * r, a.sin() * b * r);
            let at = (
                spot.centre.0 + px * cos - py * sin,
                spot.centre.1 + px * sin + py * cos,
            );
            let radius = r * size * (0.5 + random(k, 6));
            // Shades of the tint: towards white or towards its strongest
            // channel.
            let shade = random(k, 7);
            let colour = tint.map(|t| t + (1.0 - t) * shade * 0.6);
            let bright = 0.35 + 0.45 * random(k, 8);
            (at, radius, colour, bright)
        })
        .collect();
    picture(width, height, |x, y| {
        let mut out = [0.0; 3];
        for &(at, radius, colour, bright) in &discs {
            let dist = (x - at.0).hypot(y - at.1);
            if dist > radius * 1.2 {
                continue;
            }
            let edge = smoothstep(radius, radius * 0.85, dist);
            let v = edge * bright * strength * (0.7 + 0.3 * dist / radius);
            for (c, k) in out.iter_mut().zip(colour) {
                *c += v * k;
            }
        }
        out
    })
}

/// A disc of «Luces bokeh»: its centre, radius, colour and brightness.
type Disc = ((f64, f64), f64, [f64; 3], f64);

/// A number in 0–1 that depends on `k` and `salt` only (splitmix64), so a
/// light looks the same on every machine and every redraw.
fn random(k: u64, salt: u64) -> f64 {
    let mut z = k
        .wrapping_mul(0x9E37_79B9_7F4A_7C15)
        .wrapping_add(salt.wrapping_mul(0xBF58_476D_1CE4_E5B9));
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^= z >> 31;
    (z >> 11) as f64 / (1_u64 << 53) as f64
}

/// «Brillo» drawn into a `width` × `height` picture.
#[must_use]
pub fn sparkle(light: &Light, width: u32, height: u32, spot: LightSpot) -> ImageBuf {
    picture(width, height, |x, y| sparkle_at(light, spot, x, y))
}

/// «Destello de lente» drawn into a `width` × `height` picture: the star at
/// `spot` and its flare across the whole picture, through its centre.
#[must_use]
pub fn lens_flare(light: &Light, width: u32, height: u32, spot: LightSpot) -> ImageBuf {
    let centre = (f64::from(width) / 2.0, f64::from(height) / 2.0);
    let halo = light.halo.map_or(FLARE_DEFAULT, rgb);
    picture(width, height, |x, y| {
        let star = sparkle_at(light, spot, x, y);
        let flare = flare_at(light, spot, centre, halo, x, y);
        [star[0] + flare[0], star[1] + flare[1], star[2] + flare[2]]
    })
}

/// The light at the centre of the pixel (`x`, `y`).
fn sparkle_at(light: &Light, spot: LightSpot, x: f64, y: f64) -> [f64; 3] {
    let r = spot.radius.max(1.0);
    let (dx, dy) = (x - spot.centre.0, y - spot.centre.1);
    let d = dx.hypot(dy);
    let core = gauss(d, 0.10 * r) + 0.45 * gauss(d, 0.28 * r);
    let mut rays = 0.0;
    if d < r {
        let n = light.rays.max(1);
        let width = f64::from(light.ray_width.clamp(0.0, 1.0));
        for k in 0..n {
            let angle = light.angle_deg.to_radians() + f64::from(k) * TAU / f64::from(n);
            let (uy, ux) = angle.sin_cos();
            let along = dx * ux + dy * uy;
            if along <= 0.0 {
                continue;
            }
            let across = (dx * uy - dy * ux).abs();
            let left = 1.0 - along / r;
            let thick = (0.012 + 0.05 * width) * r * left + 0.6;
            rays += gauss(across, thick) * left.powf(1.5);
        }
        rays *= f64::from(light.intensity.clamp(0.0, 1.0));
    }
    let white = core + rays;
    let halo = light.halo.map_or([0.0; 3], rgb);
    let glow = 0.6 * gauss(d, 0.55 * r);
    [
        white + glow * halo[0],
        white + glow * halo[1],
        white + glow * halo[2],
    ]
}

/// The flare at (`x`, `y`): a wide glow and a ring round the light, and the
/// ghosts along the line from the light through `centre`.
fn flare_at(
    light: &Light,
    spot: LightSpot,
    centre: (f64, f64),
    halo: [f64; 3],
    x: f64,
    y: f64,
) -> [f64; 3] {
    let r = spot.radius.max(1.0);
    let strength = 0.4 + 0.6 * f64::from(light.intensity.clamp(0.0, 1.0));
    let d = (x - spot.centre.0).hypot(y - spot.centre.1);
    let around = 0.28 * gauss(d, 2.0 * r) + 0.05 * gauss(d, 6.0 * r);
    let ring = 0.7 * gauss(d - 1.15 * r, 0.09 * r);
    let mut out = [0.0; 3];
    for (c, h) in out.iter_mut().zip(halo) {
        *c = (around + ring) * h * strength;
    }
    let axis = (centre.0 - spot.centre.0, centre.1 - spot.centre.1);
    for (t, size, colour, rainbow) in GHOSTS {
        let at = (spot.centre.0 + t * axis.0, spot.centre.1 + t * axis.1);
        let size = size * r;
        let dist = (x - at.0).hypot(y - at.1);
        if dist > size * 1.3 {
            continue;
        }
        let add = if rainbow {
            let band = (dist - size) / (0.2 * size);
            let hue = rainbow_at(band);
            let v = 0.09 * gauss(band, 1.0);
            [hue[0] * v, hue[1] * v, hue[2] * v]
        } else {
            // A soft disc, a little brighter towards its edge.
            let edge = smoothstep(size, size * 0.75, dist);
            let v = 0.18 * edge * (0.6 + 0.4 * dist / size);
            [colour[0] * v, colour[1] * v, colour[2] * v]
        };
        for (c, a) in out.iter_mut().zip(add) {
            *c += a * strength;
        }
    }
    out
}

/// The picture whose pixel (`x`, `y`) gets the light `at` its centre.
fn picture(width: u32, height: u32, at: impl Fn(f64, f64) -> [f64; 3]) -> ImageBuf {
    let mut data = Vec::with_capacity(width as usize * height as usize * 4);
    for y in 0..height {
        for x in 0..width {
            let [r, g, b] = at(f64::from(x) + 0.5, f64::from(y) + 0.5);
            let (r, g, b) = (unit(r), unit(g), unit(b));
            let a = r.max(g).max(b);
            data.extend([byte(r), byte(g), byte(b), byte(a)]);
        }
    }
    ImageBuf::new(width, height, PixelBuf::Rgba8(data))
        .unwrap_or_else(|_| unreachable!("the length is width × height × 4"))
}

fn gauss(d: f64, sigma: f64) -> f64 {
    let t = d / sigma.max(1e-9);
    (-t * t).exp()
}

/// 1 at `edge1` and beyond, 0 at `edge0` and before (either order).
fn smoothstep(edge0: f64, edge1: f64, x: f64) -> f64 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Red outside to violet inside across a band of -1..1.
fn rainbow_at(band: f64) -> [f64; 3] {
    let h = ((band + 1.0) / 2.0).clamp(0.0, 1.0) * 0.8;
    let channel = |offset: f64| (0.5 + 0.5 * (TAU * (h + offset)).cos()).clamp(0.0, 1.0);
    [channel(0.0), channel(2.0 / 3.0), channel(1.0 / 3.0)]
}

fn rgb(color: Color) -> [f64; 3] {
    match color.convert(ColorSpaceId::Srgb) {
        Color::Srgb(c) => {
            [f64::from(c.r), f64::from(c.g), f64::from(c.b)].map(|v| v * f64::from(c.a))
        }
        _ => [0.0; 3],
    }
}

fn unit(v: f64) -> f64 {
    if v.is_finite() {
        v.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

fn byte(v: f64) -> u8 {
    // In 0–1 after `unit`: the cast is exact.
    (v * 255.0).round() as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pixel(image: &ImageBuf, x: u32, y: u32) -> [u8; 4] {
        let PixelBuf::Rgba8(data) = image.data() else {
            panic!("rgba8")
        };
        let i = (y * image.width() + x) as usize * 4;
        [data[i], data[i + 1], data[i + 2], data[i + 3]]
    }

    fn spot() -> LightSpot {
        LightSpot {
            centre: (50.0, 50.0),
            radius: 50.0,
        }
    }

    // S65-02: white at the centre, dark in a corner, a ray reaches out along
    // its direction and not between two rays; alpha is the brightest channel.
    #[test]
    fn s65_02_a_sparkle_is_white_in_the_middle_with_rays() {
        let light = Light::SPARKLE;
        let image = sparkle(&light, 100, 100, spot());
        assert_eq!(pixel(&image, 50, 50), [255; 4]);
        assert!(pixel(&image, 2, 2)[3] < 8, "the corner stays dark");
        // The first ray points right (0°); with six rays, 30° lies between
        // two of them.
        let on = pixel(&image, 85, 50)[3];
        let between = pixel(&image, 80, 33)[3];
        assert!(on > between + 20, "ray {on}, between {between}");
        for p in [pixel(&image, 85, 50), pixel(&image, 60, 60)] {
            assert_eq!(
                p[3],
                p[0].max(p[1]).max(p[2]),
                "alpha is the brightest channel"
            );
        }
    }

    // S65-02: the angle turns the rays and the intensity dims them.
    #[test]
    fn s65_02_angle_and_intensity() {
        let mut light = Light::SPARKLE;
        light.angle_deg = 30.0;
        let turned = sparkle(&light, 100, 100, spot());
        assert!(pixel(&turned, 80, 33)[3] > pixel(&turned, 85, 50)[3]);
        light.intensity = 0.0;
        let dim = sparkle(&light, 100, 100, spot());
        assert!(pixel(&dim, 85, 50)[3] < 10, "no rays without intensity");
    }

    // S65-02: a halo colour tints the glow round the core.
    #[test]
    fn s65_02_the_halo_tints_the_glow() {
        let mut light = Light::SPARKLE;
        light.angle_deg = 45.0;
        light.halo = Some(Color::Srgb(gafira_color::Rgba::new(1.0, 0.0, 0.0, 1.0)));
        let image = sparkle(&light, 100, 100, spot());
        let p = pixel(&image, 50, 70);
        assert!(p[0] > p[2] + 20, "{p:?}");
    }

    // D-104: the streak runs along its angle, far from the light, and not
    // across it.
    #[test]
    fn s65_08_the_streak_runs_along_its_angle() {
        let light = Light::STREAK;
        let spot = LightSpot {
            centre: (200.0, 100.0),
            radius: 20.0,
        };
        let image = streak(&light, 400, 200, spot);
        assert_eq!(pixel(&image, 200, 100)[3], 255);
        let along = pixel(&image, 330, 100);
        let across = pixel(&image, 200, 170);
        assert!(
            along[3] > across[3] + 30,
            "along {along:?}, across {across:?}"
        );
        assert!(
            along[2] > along[0],
            "tinted blue away from the core: {along:?}"
        );
    }

    // D-104: the sun rays reach far from the light, and the beams leave
    // darker gaps between them at the same distance.
    #[test]
    fn s65_09_sun_rays_fan_out_with_gaps() {
        let light = Light::SUN_RAYS;
        let spot = LightSpot {
            centre: (50.0, 50.0),
            radius: 20.0,
        };
        let image = sun_rays(&light, 400, 400, spot);
        let ring: Vec<u8> = (0..360)
            .map(|deg| {
                let a = f64::from(deg).to_radians();
                let x = 50.0 + 150.0 * a.cos();
                let y = 50.0 + 150.0 * a.sin();
                if (0.0..400.0).contains(&x) && (0.0..400.0).contains(&y) {
                    pixel(&image, x as u32, y as u32)[3]
                } else {
                    0
                }
            })
            .collect();
        let (lo, hi) = ring[5..85]
            .iter()
            .fold((255, 0), |(lo, hi), &v| (lo.min(v), hi.max(v)));
        assert!(hi > 25, "a beam reaches 150 px away: {hi}");
        assert!(hi > lo + 15, "gaps between beams: {lo}..{hi}");
    }

    // D-104: as many discs as the rays, inside the box, the same every
    // time; the angle turns them.
    #[test]
    fn s65_10_bokeh_discs_stay_in_the_box_and_repeat() {
        let light = Light::BOKEH;
        let image = bokeh(&light, 100, 100, spot());
        assert_eq!(image, bokeh(&light, 100, 100, spot()), "deterministic");
        let lit = |image: &ImageBuf| {
            let PixelBuf::Rgba8(data) = image.data() else {
                panic!("rgba8")
            };
            data.chunks(4).filter(|p| p[3] > 40).count()
        };
        assert!(lit(&image) > 300, "the discs light part of the box");
        let mut turned = light;
        turned.angle_deg = 90.0;
        assert_ne!(image, bokeh(&turned, 100, 100, spot()));
        let mut none = light;
        none.intensity = 0.0;
        assert_eq!(lit(&bokeh(&none, 100, 100, spot())), 0);
    }

    // D-104: the page-wide lights are the flare, the streak and the rays.
    #[test]
    fn s65_04_which_lights_span_the_page() {
        assert!(!spans_page(&ShapeEffect::Sparkle(Light::SPARKLE)));
        assert!(!spans_page(&ShapeEffect::Bokeh(Light::BOKEH)));
        assert!(spans_page(&ShapeEffect::LensFlare(Light::LENS_FLARE)));
        assert!(spans_page(&ShapeEffect::Streak(Light::STREAK)));
        assert!(spans_page(&ShapeEffect::SunRays(Light::SUN_RAYS)));
    }

    // S65-03: a light outside the picture, like a sun out of frame, still
    // casts its ghosts across it, through the centre.
    #[test]
    fn s65_03_a_light_out_of_frame_still_casts_its_flare() {
        let light = Light::LENS_FLARE;
        let spot = LightSpot {
            centre: (-50.0, -50.0),
            radius: 20.0,
        };
        let image = lens_flare(&light, 400, 400, spot);
        // The ghost at 1.45 of the way to the centre: -50 + 1.45 * 250.
        let ghost = pixel(&image, 312, 312)[3];
        let off = pixel(&image, 312, 60)[3];
        assert!(ghost > off + 10, "ghost {ghost}, off {off}");
    }

    // S65-03: the flare reaches far from the light, along the line through
    // the centre of the picture, and is brighter there than off that line.
    #[test]
    fn s65_03_the_flare_runs_through_the_centre() {
        let light = Light::LENS_FLARE;
        let spot = LightSpot {
            centre: (40.0, 40.0),
            radius: 20.0,
        };
        let image = lens_flare(&light, 400, 400, spot);
        assert_eq!(pixel(&image, 40, 40)[3], 255);
        // The ghost at 1.45 of the way: (40 + 1.45 * 160)^2 = (272, 272).
        let ghost = pixel(&image, 272, 272)[3];
        let off = pixel(&image, 272, 60)[3];
        assert!(ghost > off + 10, "ghost {ghost}, off {off}");
        let star = sparkle(&light, 400, 400, spot);
        assert!(
            pixel(&star, 272, 272)[3] < ghost,
            "the sparkle alone has no ghost"
        );
    }
}
