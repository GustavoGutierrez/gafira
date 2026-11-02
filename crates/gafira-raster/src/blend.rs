//! Blend modes and source-over compositing (ARQ-119), after W3C
//! *Compositing and Blending Level 1*.

use gafira_doc::BlendMode;

use crate::BufferError;

/// Backdrop colour `Cb`, source colour `Cs`, both straight, one channel.
fn separable(mode: BlendMode, cb: f64, cs: f64) -> f64 {
    let screen = |x: f64, y: f64| x + y - x * y;
    let hard_light = |b: f64, s: f64| {
        if s <= 0.5 {
            b * (2.0 * s)
        } else {
            screen(b, 2.0 * s - 1.0)
        }
    };
    match mode {
        BlendMode::Normal => cs,
        BlendMode::Multiply => cb * cs,
        BlendMode::Screen => screen(cb, cs),
        // Overlay is HardLight with the two colours swapped.
        BlendMode::Overlay => hard_light(cs, cb),
        BlendMode::Darken => cb.min(cs),
        BlendMode::Lighten => cb.max(cs),
        BlendMode::ColorDodge => {
            if cb == 0.0 {
                0.0
            } else if cs == 1.0 {
                1.0
            } else {
                (cb / (1.0 - cs)).min(1.0)
            }
        }
        BlendMode::ColorBurn => {
            if cb == 1.0 {
                1.0
            } else if cs == 0.0 {
                0.0
            } else {
                1.0 - ((1.0 - cb) / cs).min(1.0)
            }
        }
        BlendMode::HardLight => hard_light(cb, cs),
        BlendMode::SoftLight => {
            if cs <= 0.5 {
                cb - (1.0 - 2.0 * cs) * cb * (1.0 - cb)
            } else {
                let d = if cb <= 0.25 {
                    ((16.0 * cb - 12.0) * cb + 4.0) * cb
                } else {
                    cb.sqrt()
                };
                cb + (2.0 * cs - 1.0) * (d - cb)
            }
        }
        BlendMode::Difference => (cb - cs).abs(),
        BlendMode::Exclusion => cb + cs - 2.0 * cb * cs,
        // The four non-separable modes never reach here.
        BlendMode::Hue | BlendMode::Saturation | BlendMode::Color | BlendMode::Luminosity => cs,
    }
}

type Rgb = [f64; 3];

fn lum(c: Rgb) -> f64 {
    0.3 * c[0] + 0.59 * c[1] + 0.11 * c[2]
}

fn clip_color(c: Rgb) -> Rgb {
    let l = lum(c);
    let n = c[0].min(c[1]).min(c[2]);
    let x = c[0].max(c[1]).max(c[2]);
    let mut out = c;
    if n < 0.0 {
        out = out.map(|v| l + (v - l) * l / (l - n));
    }
    if x > 1.0 {
        out = out.map(|v| l + (v - l) * (1.0 - l) / (x - l));
    }
    out
}

fn set_lum(c: Rgb, l: f64) -> Rgb {
    let d = l - lum(c);
    clip_color(c.map(|v| v + d))
}

fn sat(c: Rgb) -> f64 {
    c[0].max(c[1]).max(c[2]) - c[0].min(c[1]).min(c[2])
}

fn set_sat(c: Rgb, s: f64) -> Rgb {
    let mut order = [0usize, 1, 2];
    order.sort_by(|&i, &j| c[i].total_cmp(&c[j]));
    let [i_min, i_mid, i_max] = order;
    let mut out = [0.0; 3];
    if c[i_max] > c[i_min] {
        out[i_mid] = (c[i_mid] - c[i_min]) * s / (c[i_max] - c[i_min]);
        out[i_max] = s;
    }
    out
}

fn non_separable(mode: BlendMode, cb: Rgb, cs: Rgb) -> Option<Rgb> {
    match mode {
        BlendMode::Hue => Some(set_lum(set_sat(cs, sat(cb)), lum(cb))),
        BlendMode::Saturation => Some(set_lum(set_sat(cb, sat(cs)), lum(cb))),
        BlendMode::Color => Some(set_lum(cs, lum(cb))),
        BlendMode::Luminosity => Some(set_lum(cb, lum(cs))),
        _ => None,
    }
}

/// Straight colour of a premultiplied pixel; transparent black when `a = 0`.
fn straight(p: [f64; 4]) -> Rgb {
    if p[3] > 0.0 {
        [p[0] / p[3], p[1] / p[3], p[2] / p[3]]
    } else {
        [0.0; 3]
    }
}

/// Source-over with blending on premultiplied values, in `f64`. `Normal` uses the
/// premultiplied values as they are; the others expect inputs already in `[0, 1]`.
fn composite(mode: BlendMode, b: [f64; 4], s: [f64; 4]) -> [f64; 4] {
    // A pixel with a = 0 is transparent black, whatever colour it holds.
    let b = if b[3] == 0.0 { [0.0; 4] } else { b };
    let s = if s[3] == 0.0 { [0.0; 4] } else { s };
    let (ab, a_s) = (b[3], s[3]);
    let mut out = [0.0; 4];
    out[3] = a_s + ab * (1.0 - a_s);
    if mode == BlendMode::Normal {
        for i in 0..3 {
            out[i] = s[i] + b[i] * (1.0 - a_s);
        }
        return out;
    }
    let (cb, cs) = (straight(b), straight(s));
    // W3C section 10: the result of the mixing function is clamped to [0, 1].
    let blended = non_separable(mode, cb, cs)
        .unwrap_or_else(|| [0, 1, 2].map(|i| separable(mode, cb[i], cs[i])))
        .map(|v| v.clamp(0.0, 1.0));
    for i in 0..3 {
        // αs (1 − αb) Cs + αs αb B(Cb, Cs) + (1 − αs) αb Cb
        out[i] = s[i] * (1.0 - ab) + a_s * ab * blended[i] + b[i] * (1.0 - a_s);
    }
    out
}

/// Clips to `[0, 1]`, then each colour channel to the alpha, so the straight colour
/// stays in `[0, 1]`.
fn clip(p: [f32; 4]) -> [f64; 4] {
    let [r, g, b, a] = p.map(|v| f64::from(v.clamp(0.0, 1.0)));
    [r.min(a), g.min(a), b.min(a), a]
}

/// Blends a premultiplied RGBA `f32` source over a premultiplied backdrop.
///
/// Before any mode but `Normal`, all eight inputs are clipped to `[0, 1]`, and
/// a colour channel above its alpha is clipped to the alpha. A pixel with `a = 0` counts as
/// transparent black, except a source with `a = 0`, which returns the backdrop untouched.
///
/// # Errors
///
/// [`BufferError::NonFinite`] with field `backdrop`, `source` or `result`; the
/// index is the channel. Inputs are checked first, also when `a = 0`.
pub fn blend_f32(
    mode: BlendMode,
    backdrop: [f32; 4],
    source: [f32; 4],
) -> Result<[f32; 4], BufferError> {
    for (field, p) in [("backdrop", backdrop), ("source", source)] {
        if let Some(index) = p.iter().position(|v| !v.is_finite()) {
            return Err(BufferError::NonFinite { field, index });
        }
    }
    // A source with a = 0 paints nothing: the backdrop stays as it is, bit for bit.
    if source[3] == 0.0 {
        return Ok(backdrop);
    }
    let (b, s) = if mode == BlendMode::Normal {
        (backdrop.map(f64::from), source.map(f64::from))
    } else {
        (clip(backdrop), clip(source))
    };
    let out = composite(mode, b, s).map(|v| v as f32);
    match out.iter().position(|v| !v.is_finite()) {
        Some(index) => Err(BufferError::NonFinite {
            field: "result",
            index,
        }),
        None => Ok(out),
    }
}

/// Blends premultiplied RGBA 8-bit pixels; the result is rounded to nearest.
/// A channel above its alpha is clipped to the alpha before a mode but `Normal`.
#[must_use]
pub fn blend_u8(mode: BlendMode, backdrop: [u8; 4], source: [u8; 4]) -> [u8; 4] {
    if source[3] == 0 {
        return backdrop;
    }
    let wide = |p: [u8; 4]| {
        let a = f64::from(p[3]) / 255.0;
        let c = |i: usize| {
            let v = f64::from(p[i]) / 255.0;
            if mode == BlendMode::Normal {
                v
            } else {
                v.min(a)
            }
        };
        [c(0), c(1), c(2), a]
    };
    composite(mode, wide(backdrop), wide(source))
        .map(|v| (v * 255.0 + 0.5).floor().clamp(0.0, 255.0) as u8)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::premultiply_u8;

    /// The W3C formulas, transcribed literally on straight colours and in `f64`.
    /// Kept apart from the code under test: other names, other structure.
    mod reference {
        use gafira_doc::BlendMode;

        fn sep(mode: BlendMode, cb: f64, cs: f64) -> f64 {
            let multiply = |b: f64, s: f64| b * s;
            let screen = |b: f64, s: f64| b + s - b * s;
            let hard_light = |b: f64, s: f64| {
                if s <= 0.5 {
                    multiply(b, 2.0 * s)
                } else {
                    screen(b, 2.0 * s - 1.0)
                }
            };
            match mode {
                BlendMode::Normal => cs,
                BlendMode::Multiply => multiply(cb, cs),
                BlendMode::Screen => screen(cb, cs),
                BlendMode::Overlay => hard_light(cs, cb),
                BlendMode::Darken => cb.min(cs),
                BlendMode::Lighten => cb.max(cs),
                BlendMode::ColorDodge => {
                    if cb == 0.0 {
                        0.0
                    } else if cs == 1.0 {
                        1.0
                    } else {
                        1.0_f64.min(cb / (1.0 - cs))
                    }
                }
                BlendMode::ColorBurn => {
                    if cb == 1.0 {
                        1.0
                    } else if cs == 0.0 {
                        0.0
                    } else {
                        1.0 - 1.0_f64.min((1.0 - cb) / cs)
                    }
                }
                BlendMode::HardLight => hard_light(cb, cs),
                BlendMode::SoftLight => {
                    if cs <= 0.5 {
                        cb - (1.0 - 2.0 * cs) * cb * (1.0 - cb)
                    } else {
                        let d = if cb <= 0.25 {
                            ((16.0 * cb - 12.0) * cb + 4.0) * cb
                        } else {
                            cb.sqrt()
                        };
                        cb + (2.0 * cs - 1.0) * (d - cb)
                    }
                }
                BlendMode::Difference => (cb - cs).abs(),
                BlendMode::Exclusion => cb + cs - 2.0 * cb * cs,
                _ => unreachable!("non-separable"),
            }
        }

        fn lum(c: [f64; 3]) -> f64 {
            0.3 * c[0] + 0.59 * c[1] + 0.11 * c[2]
        }

        fn clip_color(c: [f64; 3]) -> [f64; 3] {
            let l = lum(c);
            let n = c[0].min(c[1]).min(c[2]);
            let x = c[0].max(c[1]).max(c[2]);
            let mut c = c;
            if n < 0.0 {
                for v in &mut c {
                    *v = l + (((*v - l) * l) / (l - n));
                }
            }
            if x > 1.0 {
                for v in &mut c {
                    *v = l + (((*v - l) * (1.0 - l)) / (x - l));
                }
            }
            c
        }

        fn set_lum(c: [f64; 3], l: f64) -> [f64; 3] {
            let d = l - lum(c);
            clip_color([c[0] + d, c[1] + d, c[2] + d])
        }

        fn sat(c: [f64; 3]) -> f64 {
            c[0].max(c[1]).max(c[2]) - c[0].min(c[1]).min(c[2])
        }

        fn set_sat(c: [f64; 3], s: f64) -> [f64; 3] {
            let max = c[0].max(c[1]).max(c[2]);
            let min = c[0].min(c[1]).min(c[2]);
            let mid = c[0] + c[1] + c[2] - max - min;
            let mut out = [0.0; 3];
            if max > min {
                let new_mid = ((mid - min) * s) / (max - min);
                // Put each component back where it came from.
                let mut used_max = false;
                let mut used_min = false;
                for (i, v) in c.iter().enumerate() {
                    out[i] = if *v == max && !used_max {
                        used_max = true;
                        s
                    } else if *v == min && !used_min {
                        used_min = true;
                        0.0
                    } else {
                        new_mid
                    };
                }
            }
            out
        }

        pub fn b(mode: BlendMode, cb: [f64; 3], cs: [f64; 3]) -> [f64; 3] {
            match mode {
                BlendMode::Hue => set_lum(set_sat(cs, sat(cb)), lum(cb)),
                BlendMode::Saturation => set_lum(set_sat(cb, sat(cs)), lum(cb)),
                BlendMode::Color => set_lum(cs, lum(cb)),
                BlendMode::Luminosity => set_lum(cb, lum(cs)),
                _ => [0, 1, 2].map(|i| sep(mode, cb[i], cs[i])),
            }
        }

        /// `αo × Co = αs(1 − αb)Cs + αs αb B(Cb, Cs) + (1 − αs) αb Cb`, with
        /// premultiplied input and output.
        pub fn composite(mode: BlendMode, backdrop: [f64; 4], source: [f64; 4]) -> [f64; 4] {
            let (ab, a_s) = (backdrop[3], source[3]);
            let straight = |p: [f64; 4]| {
                if p[3] > 0.0 {
                    [p[0] / p[3], p[1] / p[3], p[2] / p[3]]
                } else {
                    [0.0; 3]
                }
            };
            let (cb, cs) = (straight(backdrop), straight(source));
            let bl = b(mode, cb, cs);
            let mut out = [0.0; 4];
            for i in 0..3 {
                out[i] = a_s * (1.0 - ab) * cs[i] + a_s * ab * bl[i] + (1.0 - a_s) * ab * cb[i];
            }
            out[3] = a_s + ab - a_s * ab;
            out
        }
    }

    const GRID: [f64; 7] = [0.0, 0.25, 0.3, 0.5, 0.75, 0.9, 1.0];
    const ALPHAS: [f64; 6] = [0.0, 1.0, 0.5, 0.25, 0.01, 0.9];

    fn lcg(seed: &mut u32) -> f64 {
        *seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        f64::from(*seed >> 8) / f64::from(1u32 << 24)
    }

    /// Straight colour and alpha triples: the boundary grid, then pseudo-random ones.
    fn triples() -> Vec<([f64; 3], [f64; 3], f64, f64)> {
        let mut seed = 99u32;
        let mut out = Vec::new();
        for i in 0..400 {
            let mut pick = |k: u32| {
                let v = lcg(&mut seed);
                if (i + k).is_multiple_of(3) {
                    GRID[(v * 7.0) as usize % 7]
                } else {
                    v
                }
            };
            let cb = [pick(0), pick(1), pick(2)];
            let cs = [pick(0), pick(2), pick(1)];
            let ab = ALPHAS[i as usize % 6];
            let a_s = ALPHAS[(i as usize / 6 + i as usize) % 6];
            out.push((cb, cs, ab, a_s));
        }
        out
    }

    fn premul(c: [f64; 3], a: f64) -> [f64; 4] {
        [c[0] * a, c[1] * a, c[2] * a, a]
    }

    fn to_f32(p: [f64; 4]) -> [f32; 4] {
        p.map(|v| v as f32)
    }

    fn max_err(a: [f32; 4], b: [f64; 4]) -> f64 {
        (0..4)
            .map(|i| (f64::from(a[i]) - b[i]).abs())
            .fold(0.0, f64::max)
    }

    // ARQ-119 (FD-12 c, CPU): every mode against the W3C reference, at least 256 triples,
    // f32 <= 1e-5.
    #[test]
    fn arq_119_every_mode_matches_the_reference_in_f32() {
        let t = triples();
        assert!(t.len() >= 256);
        for mode in BlendMode::ALL {
            for (cb, cs, ab, a_s) in &t {
                // Use the f32 values as the inputs of the reference too.
                let b32 = to_f32(premul(*cb, *ab));
                let s32 = to_f32(premul(*cs, *a_s));
                let want = reference::composite(mode, b32.map(f64::from), s32.map(f64::from));
                let got = blend_f32(mode, b32, s32).unwrap();
                let err = max_err(got, want);
                assert!(
                    err <= 1e-5,
                    "{mode:?} cb={cb:?} cs={cs:?} ab={ab} as={a_s}: {got:?} vs {want:?}"
                );
            }
        }
    }

    // ARQ-119 (FD-12 c, CPU): every mode in 8 bits, error <= 1/255, and the result is the
    // nearest byte.
    #[test]
    fn arq_119_every_mode_matches_the_reference_in_8_bits() {
        let to_u8 = |c: [f64; 3], a: f64| {
            let a8 = (a * 255.0).round() as u8;
            [0, 1, 2]
                .map(|i| premultiply_u8((c[i] * 255.0).round() as u8, a8))
                .into_iter()
                .chain([a8])
                .collect::<Vec<_>>()
        };
        let t = triples();
        for mode in BlendMode::ALL {
            for (cb, cs, ab, a_s) in &t {
                let b8: [u8; 4] = to_u8(*cb, *ab).try_into().unwrap();
                let s8: [u8; 4] = to_u8(*cs, *a_s).try_into().unwrap();
                let want = reference::composite(
                    mode,
                    b8.map(|v| f64::from(v) / 255.0),
                    s8.map(|v| f64::from(v) / 255.0),
                );
                let got = blend_u8(mode, b8, s8);
                for i in 0..4 {
                    let err = (f64::from(got[i]) / 255.0 - want[i]).abs();
                    assert!(
                        err <= 1.0 / 255.0,
                        "{mode:?} {b8:?} {s8:?} ch{i}: {got:?} vs {want:?}"
                    );
                    assert!(
                        err <= 0.5 / 255.0 + 1e-9,
                        "{mode:?} not rounded to nearest: ch{i} {got:?} vs {want:?}"
                    );
                }
            }
        }
    }

    // ARQ-119: known values from the formulas, worked by hand.
    #[test]
    fn arq_119_spot_values_of_the_w3c_formulas() {
        // Opaque pixels: result colour is B(Cb, Cs).
        let one = |mode, cb: f32, cs: f32| {
            blend_f32(mode, [cb, cb, cb, 1.0], [cs, cs, cs, 1.0]).unwrap()[0]
        };
        let near = |got: f32, want: f32| assert!((got - want).abs() < 1e-6, "{got} vs {want}");
        near(one(BlendMode::Multiply, 0.5, 0.5), 0.25);
        near(one(BlendMode::Screen, 0.5, 0.5), 0.75);
        near(one(BlendMode::Overlay, 0.25, 0.75), 0.375); // HardLight(0.75, 0.25) = Multiply(0.75, 0.5)
        near(one(BlendMode::Overlay, 0.75, 0.25), 0.625);
        near(one(BlendMode::HardLight, 0.25, 0.75), 0.625);
        near(one(BlendMode::Darken, 0.25, 0.75), 0.25);
        near(one(BlendMode::Lighten, 0.25, 0.75), 0.75);
        near(one(BlendMode::Difference, 0.25, 0.75), 0.5);
        near(
            one(BlendMode::Exclusion, 0.25, 0.75),
            0.25 + 0.75 - 2.0 * 0.25 * 0.75,
        );
        near(one(BlendMode::ColorDodge, 0.25, 0.5), 0.5);
        near(one(BlendMode::ColorDodge, 0.75, 0.5), 1.0);
        near(one(BlendMode::ColorBurn, 0.75, 0.5), 0.5);
        near(one(BlendMode::ColorBurn, 0.25, 0.5), 0.0);
        // SoftLight, both D(x) branches (0.25 is the join, 0.3 is past it).
        near(
            one(BlendMode::SoftLight, 0.5, 0.75),
            0.5 + 0.5 * (0.5f32.sqrt() - 0.5),
        );
        near(
            one(BlendMode::SoftLight, 0.3, 0.75),
            0.3 + 0.5 * (0.3f32.sqrt() - 0.3),
        );
        near(
            one(BlendMode::SoftLight, 0.2, 0.75),
            0.2 + 0.5 * (((16.0 * 0.2 - 12.0) * 0.2 + 4.0) * 0.2 - 0.2),
        );
        near(one(BlendMode::SoftLight, 0.5, 0.25), 0.5 - 0.5 * 0.5 * 0.5);
        // Non-separable, opaque, worked from Lum, SetSat, SetLum and ClipColor.
        let cb = [0.8, 0.2, 0.2, 1.0];
        let all = |mode, cs: [f32; 4], want: [f32; 3]| {
            let got = blend_f32(mode, cb, cs).unwrap();
            for i in 0..3 {
                near(got[i], want[i]);
            }
        };
        // Lum(Cb) = 0.38. Color: SetLum((0,0,1), 0.38) = (0.27,0.27,1.27), clipped:
        // L + (C - L) * (1 - L) / (x - L) with x = 1.27.
        all(
            BlendMode::Color,
            [0.0, 0.0, 1.0, 1.0],
            [0.303_371, 0.303_371, 1.0],
        );
        // Luminosity: Lum(Cs) = 0.5, d = 0.12, no clip.
        all(
            BlendMode::Luminosity,
            [0.5, 0.5, 0.5, 1.0],
            [0.92, 0.32, 0.32],
        );
        // Saturation of a grey source: SetSat(Cb, 0) is black, then SetLum gives grey 0.38.
        all(
            BlendMode::Saturation,
            [0.5, 0.5, 0.5, 1.0],
            [0.38, 0.38, 0.38],
        );
        // Hue: SetSat((0,0,1), Sat(Cb) = 0.6) = (0,0,0.6); Lum 0.066; d = 0.314.
        all(BlendMode::Hue, [0.0, 0.0, 1.0, 1.0], [0.314, 0.314, 0.914]);
    }

    fn bits(p: [f32; 4]) -> [u32; 4] {
        p.map(f32::to_bits)
    }

    // ARQ-119: a source with a = 0 leaves the backdrop, whatever colour it holds.
    #[test]
    fn arq_119_transparent_source_leaves_backdrop() {
        let backdrops = [
            [0.2f32, 0.4, 0.1, 0.5],
            [0.3, 0.3, 0.3, 1.0],
            [0.0, 0.0, 0.0, 0.0],
        ];
        let sources = [[0.0f32; 4], [0.9, 0.1, 0.5, 0.0], [1.0, 1.0, 1.0, -0.0]];
        for mode in BlendMode::ALL {
            for b in backdrops {
                for s in sources {
                    let got = blend_f32(mode, b, s).unwrap();
                    assert_eq!(bits(got), bits(b), "{mode:?} {b:?} {s:?}");
                }
                assert_eq!(
                    blend_u8(mode, [10, 20, 5, 100], [200, 9, 7, 0]),
                    [10, 20, 5, 100]
                );
            }
        }
    }

    // ARQ-119 (FD-12 c, CPU): a backdrop with a = 0 gives the source, bit for bit: difference 0.
    // A source with a = 0 is the other rule and is tested apart.
    #[test]
    fn arq_119_transparent_backdrop_gives_source() {
        let mut seed = 5u32;
        let mut sources = vec![
            [0.2f32, 0.4, 0.1, 0.5],
            [0.3, 0.3, 0.3, 1.0],
            [1.0e-21, 0.0, 1.0e-30, 1.0e-20],
        ];
        for _ in 0..3000 {
            // Colours whose c / a is not exact in f32: low mantissa bits are scrambled, so c
            // is not a product a * u (for those, (c / a) * a returns c anyway).
            let a = (lcg(&mut seed) as f32).max(1e-6);
            let mut channel = || {
                let c = lcg(&mut seed) as f32 * a;
                let scrambled = f32::from_bits(c.to_bits() ^ ((lcg(&mut seed) * 4.0) as u32));
                scrambled.min(a)
            };
            sources.push([channel(), channel(), channel(), a]);
        }
        let backdrops = [[0.0f32; 4], [0.9, 0.1, 0.5, 0.0], [1.0, 1.0, 1.0, -0.0]];
        for mode in BlendMode::ALL {
            for s in &sources {
                for b in backdrops {
                    let got = blend_f32(mode, b, *s).unwrap();
                    assert_eq!(bits(got), bits(*s), "{mode:?} {b:?} {s:?}");
                }
            }
            for s in [[10u8, 20, 5, 100], [255, 255, 255, 255], [1, 0, 0, 1]] {
                assert_eq!(blend_u8(mode, [200, 9, 7, 0], s), s);
            }
        }
    }

    // ARQ-119: Normal with a = 1 gives the source exactly, also outside [0, 1].
    #[test]
    fn arq_119_normal_opaque_source_is_the_source() {
        for s in [
            [0.1f32, 0.2, 0.3, 1.0],
            [2.0, -0.5, 1.5, 1.0],
            [0.0, 0.0, 0.0, 1.0],
        ] {
            for b in [[0.5f32, 0.5, 0.5, 1.0], [7.0, -3.0, 0.2, 0.4], [0.0; 4]] {
                assert_eq!(bits(blend_f32(BlendMode::Normal, b, s).unwrap()), bits(s));
            }
        }
        assert_eq!(
            blend_u8(BlendMode::Normal, [9, 8, 7, 200], [1, 2, 3, 255]),
            [1, 2, 3, 255]
        );
    }

    // ARQ-119: Normal is not clipped; other modes clip their inputs to [0, 1] first.
    #[test]
    fn arq_119_clip_applies_only_before_other_modes() {
        // Normal: 2.0 + 1.5 * (1 - 0.5) = 2.75, alpha 0.5 + 1 * 0.5.
        let n = blend_f32(
            BlendMode::Normal,
            [1.5, 0.0, 0.0, 1.0],
            [2.0, 0.0, 0.0, 0.5],
        )
        .unwrap();
        assert_eq!(n, [2.75, 0.0, 0.0, 1.0]);
        let n = blend_f32(BlendMode::Normal, [0.0; 4], [-1.0, 0.0, 0.0, 0.5]).unwrap();
        assert_eq!(n[0], -1.0);
        // Multiply: inputs clip to (1, 0, 0, 1) and (0.5, 0, 0, 1) first.
        let m = blend_f32(
            BlendMode::Multiply,
            [1.5, 0.0, 0.0, 1.0],
            [0.5, 0.0, 0.0, 1.0],
        )
        .unwrap();
        assert!((m[0] - 0.5).abs() < 1e-6, "{m:?}");
        let m = blend_f32(
            BlendMode::Multiply,
            [0.5, 0.0, 0.0, 1.0],
            [-2.0, 0.0, 0.0, 1.0],
        )
        .unwrap();
        assert_eq!(m[0], 0.0);
        // Alpha above 1 clips to 1 as well.
        let m = blend_f32(
            BlendMode::Multiply,
            [0.5, 0.5, 0.5, 3.0],
            [0.5, 0.5, 0.5, 1.0],
        )
        .unwrap();
        assert!(
            (m[3] - 1.0).abs() < 1e-6 && (m[0] - 0.25).abs() < 1e-6,
            "{m:?}"
        );
        // Premultiplied colour above its alpha clips to the alpha: straight colour 1.
        let m = blend_f32(
            BlendMode::Screen,
            [0.9, 0.0, 0.0, 0.5],
            [0.0, 0.0, 0.0, 1.0],
        )
        .unwrap();
        assert!(m.iter().all(|v| (0.0..=1.0).contains(v)), "{m:?}");
        let capped = blend_f32(
            BlendMode::Screen,
            [0.5, 0.0, 0.0, 0.5],
            [0.0, 0.0, 0.0, 1.0],
        )
        .unwrap();
        assert_eq!(bits(m), bits(capped));
        // In 8 bits a channel above its alpha is capped for the same reason, but not in Normal.
        let black = [0, 0, 0, 255];
        assert_eq!(
            blend_u8(BlendMode::Screen, [200, 0, 0, 100], black),
            blend_u8(BlendMode::Screen, [100, 0, 0, 100], black)
        );
        assert_eq!(
            blend_u8(BlendMode::Normal, [200, 0, 0, 100], [0, 0, 0, 0])[0],
            200
        );
    }

    // ARQ-119: ColorDodge and ColorBurn at both boundaries never divide by zero.
    #[test]
    fn arq_119_dodge_and_burn_boundaries() {
        let one = |mode, cb: f32, cs: f32| {
            blend_f32(mode, [cb, cb, cb, 1.0], [cs, cs, cs, 1.0]).unwrap()[0]
        };
        // Dodge: Cb = 0 gives 0 even with Cs = 1; Cs = 1 gives 1 otherwise.
        assert_eq!(one(BlendMode::ColorDodge, 0.0, 1.0), 0.0);
        assert_eq!(one(BlendMode::ColorDodge, 0.0, 0.0), 0.0);
        assert_eq!(one(BlendMode::ColorDodge, 0.5, 1.0), 1.0);
        assert_eq!(one(BlendMode::ColorDodge, 1.0, 1.0), 1.0);
        assert_eq!(one(BlendMode::ColorDodge, 1.0, 0.0), 1.0);
        // Burn: Cb = 1 gives 1 even with Cs = 0; Cs = 0 gives 0 otherwise.
        assert_eq!(one(BlendMode::ColorBurn, 1.0, 0.0), 1.0);
        assert_eq!(one(BlendMode::ColorBurn, 1.0, 1.0), 1.0);
        assert_eq!(one(BlendMode::ColorBurn, 0.5, 0.0), 0.0);
        assert_eq!(one(BlendMode::ColorBurn, 0.0, 0.0), 0.0);
        assert_eq!(one(BlendMode::ColorBurn, 0.0, 1.0), 0.0);
        // The same in 8 bits.
        assert_eq!(
            blend_u8(BlendMode::ColorDodge, [0, 0, 0, 255], [255, 255, 255, 255])[0],
            0
        );
        assert_eq!(
            blend_u8(BlendMode::ColorBurn, [255, 255, 255, 255], [0, 0, 0, 255])[0],
            255
        );
    }

    // ARQ-119, ARQ-110: boundary colours and alphas, a = 0 included, give finite output
    // in [0, 1] for every mode (no 0/0 from an unpremultiply).
    #[test]
    fn arq_119_boundary_inputs_give_finite_output_for_every_mode() {
        let vals = [0.0f32, 1.0, 0.5, 0.25];
        let alphas = [0.0f32, 1.0, 0.5, 1e-30, f32::MIN_POSITIVE, 1e-45];
        for mode in BlendMode::ALL {
            for &r in &vals {
                for &g in &vals {
                    for &ab in &alphas {
                        for &a_s in &alphas {
                            let b = [r * ab, g * ab, r * ab, ab];
                            let s = [g * a_s, r * a_s, 1.0 * a_s, a_s];
                            let got = blend_f32(mode, b, s).unwrap();
                            assert!(
                                got.iter().all(|v| v.is_finite() && (0.0..=1.0).contains(v)),
                                "{mode:?} {b:?} {s:?}: {got:?}"
                            );
                        }
                    }
                }
            }
        }
    }

    // ARQ-110: NaN or infinity in any of the eight inputs is Err naming the pixel, for all modes.
    #[test]
    fn arq_110_non_finite_input_is_err_for_every_mode() {
        for mode in BlendMode::ALL {
            for bad in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
                for side in 0..2 {
                    for k in 0..4 {
                        // Also in a pixel whose alpha is 0.
                        for alpha in [0.5f32, 0.0] {
                            let mut b = [0.2f32, 0.2, 0.2, alpha];
                            let mut s = [0.3f32, 0.3, 0.3, alpha];
                            let (p, field) = if side == 0 {
                                (&mut b, "backdrop")
                            } else {
                                (&mut s, "source")
                            };
                            p[k] = bad;
                            let err = blend_f32(mode, b, s).unwrap_err();
                            assert_eq!(
                                err,
                                BufferError::NonFinite { field, index: k },
                                "{mode:?} {bad}"
                            );
                        }
                    }
                }
            }
        }
    }

    // ARQ-110: a Normal result that overflows is Err, not an infinity.
    #[test]
    fn arq_110_overflowing_normal_result_is_err() {
        let err = blend_f32(
            BlendMode::Normal,
            [f32::MAX, 0.0, 0.0, 1.0],
            [f32::MAX, 0.0, 0.0, 0.5],
        )
        .unwrap_err();
        assert_eq!(
            err,
            BufferError::NonFinite {
                field: "result",
                index: 0
            }
        );
    }

    // ARQ-111: the same input gives the same bytes, for every mode and both widths.
    #[test]
    fn arq_111_blend_is_deterministic() {
        let t = triples();
        let run = || {
            let mut out = Vec::new();
            for mode in BlendMode::ALL {
                for (cb, cs, ab, a_s) in t.iter().take(40) {
                    out.extend(bits(
                        blend_f32(mode, to_f32(premul(*cb, *ab)), to_f32(premul(*cs, *a_s)))
                            .unwrap(),
                    ));
                    out.extend(
                        blend_u8(mode, [10, 200, 30, 255], [(cb[0] * 255.0) as u8, 9, 8, 128])
                            .map(u32::from),
                    );
                }
            }
            out
        };
        assert_eq!(run(), run());
    }

    // ARQ-119 (amended): a source with a = 0 leaves the backdrop bit for bit, in every mode,
    // with no clip, even for a backdrop outside [0, 1] or with -0.0.
    #[test]
    fn arq_119_transparent_source_leaves_the_backdrop_bit_for_bit() {
        let backdrops = [
            [1.5f32, 0.5, 0.2, 1.0],
            [-0.0, 0.5, -0.25, 1.0],
            [0.9, 0.1, 0.4, 0.5],
            [2.0, -1.0, 3.0, 7.0],
            [0.3, 0.3, 0.3, -0.0],
        ];
        let sources = [
            [0.0f32; 4],
            [0.9, 0.1, 0.5, 0.0],
            [1.5, -3.0, 9.0, -0.0],
            [0.0, -0.0, 0.0, 0.0],
        ];
        for mode in BlendMode::ALL {
            for b in backdrops {
                for s in sources {
                    let got = blend_f32(mode, b, s).unwrap();
                    assert_eq!(bits(got), bits(b), "{mode:?} {b:?} {s:?}");
                }
            }
            // 8 bits: a backdrop channel above its alpha is returned as it is, too.
            assert_eq!(
                blend_u8(mode, [200, 9, 7, 100], [1, 2, 3, 0]),
                [200, 9, 7, 100]
            );
        }
    }

    // ARQ-119 (amended): a NaN or infinity still gives Err when the source has a = 0.
    #[test]
    fn arq_119_transparent_source_still_checks_for_non_finite() {
        for mode in BlendMode::ALL {
            for bad in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
                let err = blend_f32(mode, [0.2, 0.2, 0.2, 1.0], [bad, 0.0, 0.0, 0.0]).unwrap_err();
                assert_eq!(
                    err,
                    BufferError::NonFinite {
                        field: "source",
                        index: 0
                    }
                );
                let err = blend_f32(mode, [0.2, bad, 0.2, 1.0], [0.0; 4]).unwrap_err();
                assert_eq!(
                    err,
                    BufferError::NonFinite {
                        field: "backdrop",
                        index: 1
                    }
                );
                let err = blend_f32(mode, [0.2; 4], [0.0, 0.0, 0.0, bad]).unwrap_err();
                assert_eq!(
                    err,
                    BufferError::NonFinite {
                        field: "source",
                        index: 3
                    }
                );
            }
        }
    }

    // ARQ-119, W3C section 10: the mixing result is clamped, so valid input never gives a
    // negative channel (the non-separable modes reach -4e-16 without the clamp).
    #[test]
    fn arq_119_mixing_result_is_clamped() {
        let got = blend_f32(
            BlendMode::Saturation,
            [0.723_585_6, 0.0, 0.0, 1.0],
            [0.0, 0.0, 1.0, 1.0],
        )
        .unwrap();
        assert!(got.iter().all(|v| *v >= 0.0), "{got:?}");
        let mut seed = 7u32;
        for mode in [
            BlendMode::Hue,
            BlendMode::Saturation,
            BlendMode::Color,
            BlendMode::Luminosity,
        ] {
            for _ in 0..20_000 {
                let mut px = || {
                    let a = lcg(&mut seed) as f32;
                    [
                        lcg(&mut seed) as f32 * a,
                        lcg(&mut seed) as f32 * a,
                        lcg(&mut seed) as f32 * a,
                        a,
                    ]
                };
                let (b, s) = (px(), px());
                let got = blend_f32(mode, b, s).unwrap();
                assert!(
                    got.iter().all(|v| (0.0..=1.0).contains(v)),
                    "{mode:?} {b:?} {s:?}: {got:?}"
                );
            }
        }
    }

    // ARQ-119: the SOURCE colour is capped to its alpha too (f32 and 8 bits), not only the
    // backdrop, and Normal does not cap it.
    #[test]
    fn arq_119_source_colour_is_capped_to_its_alpha() {
        let backdrop = [0.3f32, 0.2, 0.1, 0.6];
        for mode in BlendMode::ALL
            .into_iter()
            .filter(|m| *m != BlendMode::Normal)
        {
            let over = blend_f32(mode, backdrop, [0.9, 0.7, 0.5, 0.5]).unwrap();
            let capped = blend_f32(mode, backdrop, [0.5, 0.5, 0.5, 0.5]).unwrap();
            assert_eq!(bits(over), bits(capped), "{mode:?}");
            assert!(
                over.iter().all(|v| (0.0..=1.0).contains(v)),
                "{mode:?} {over:?}"
            );
            let over8 = blend_u8(mode, [80, 50, 25, 150], [230, 180, 130, 128]);
            let capped8 = blend_u8(mode, [80, 50, 25, 150], [128, 128, 128, 128]);
            assert_eq!(over8, capped8, "{mode:?}");
        }
        let n = blend_f32(BlendMode::Normal, backdrop, [0.9, 0.7, 0.5, 0.5]).unwrap();
        assert!((n[0] - (0.9 + 0.3 * 0.5)).abs() < 1e-6, "{n:?}");
        assert_eq!(
            blend_u8(BlendMode::Normal, [0, 0, 0, 0], [230, 0, 0, 128])[0],
            230
        );
    }

    // ARQ-110: a result that overflows only in alpha is Err with index 3.
    #[test]
    fn arq_110_overflowing_alpha_is_err() {
        let err = blend_f32(
            BlendMode::Normal,
            [0.0, 0.0, 0.0, f32::MAX],
            [0.0, 0.0, 0.0, -1.0],
        )
        .unwrap_err();
        assert_eq!(
            err,
            BufferError::NonFinite {
                field: "result",
                index: 3
            }
        );
    }

    // ARQ-110: ColorBurn with a source channel of -0.0 (equal to 0) is 0, never an infinity.
    #[test]
    fn arq_110_color_burn_with_negative_zero_source() {
        for cb in [0.0f32, 0.25, 0.5, 0.999] {
            let got = blend_f32(
                BlendMode::ColorBurn,
                [cb, cb, cb, 1.0],
                [-0.0, -0.0, -0.0, 1.0],
            )
            .unwrap();
            assert_eq!(got, [0.0, 0.0, 0.0, 1.0], "cb={cb}");
        }
    }

    mod property {
        use super::*;
        use proptest::prelude::*;

        proptest! {
            // ARQ-110: finite input in any range gives finite output in [0, 1] (or, for
            // Normal only, an Err on overflow).
            #[test]
            fn arq_110_finite_input_gives_finite_output(
                b in prop::array::uniform4(-1.0e30f32..1.0e30),
                s in prop::array::uniform4(-1.0e30f32..1.0e30),
                mode in 0usize..16,
            ) {
                let mode = BlendMode::ALL[mode];
                match blend_f32(mode, b, s) {
                    Ok(out) => prop_assert!(out.iter().all(|v| v.is_finite())),
                    Err(e) => {
                        prop_assert_eq!(mode, BlendMode::Normal);
                        let is_result = matches!(e, BufferError::NonFinite { field: "result", .. });
                        prop_assert!(is_result);
                    }
                }
                if mode != BlendMode::Normal {
                    let out = blend_f32(mode, b, s).unwrap();
                    prop_assert!(out.iter().all(|v| (0.0..=1.0).contains(v)));
                }
            }

            // ARQ-110: the interior and the tiny values, not only the corners: finite
            // output in [0, 1] for every non-Normal mode, and no Err.
            #[test]
            fn arq_110_interior_input_gives_output_in_unit_range(
                b in prop::array::uniform4(prop_oneof![-2.0f32..2.0, -1e-30f32..1e-30, 0.0f32..1.0]),
                s in prop::array::uniform4(prop_oneof![-2.0f32..2.0, -1e-30f32..1e-30, 0.0f32..1.0]),
                mode in 1usize..16,
            ) {
                let out = blend_f32(BlendMode::ALL[mode], b, s).unwrap();
                prop_assert!(out.iter().all(|v| (0.0..=1.0).contains(v)), "{b:?} {s:?} {out:?}");
            }

            // ARQ-119: valid premultiplied input (c <= a) in the interior stays in [0, 1].
            #[test]
            fn arq_119_valid_interior_input_stays_in_unit_range(
                bc in prop::array::uniform3(0.0f32..=1.0), ba in 1e-6f32..=1.0,
                sc in prop::array::uniform3(0.0f32..=1.0), sa in 1e-6f32..=1.0,
                mode in 1usize..16,
            ) {
                let b = [bc[0] * ba, bc[1] * ba, bc[2] * ba, ba];
                let s = [sc[0] * sa, sc[1] * sa, sc[2] * sa, sa];
                let out = blend_f32(BlendMode::ALL[mode], b, s).unwrap();
                prop_assert!(out.iter().all(|v| (0.0..=1.0).contains(v)), "{b:?} {s:?} {out:?}");
            }

            // ARQ-119: valid premultiplied 8-bit input gives a result no brighter than its alpha.
            #[test]
            fn arq_119_u8_result_is_premultiplied_valid(
                bc in prop::array::uniform3(any::<u8>()), ba in any::<u8>(),
                sc in prop::array::uniform3(any::<u8>()), sa in any::<u8>(),
                mode in 0usize..16,
            ) {
                let b = [premultiply_u8(bc[0], ba), premultiply_u8(bc[1], ba), premultiply_u8(bc[2], ba), ba];
                let s = [premultiply_u8(sc[0], sa), premultiply_u8(sc[1], sa), premultiply_u8(sc[2], sa), sa];
                let out = blend_u8(BlendMode::ALL[mode], b, s);
                prop_assert!(out[..3].iter().all(|c| *c <= out[3]), "{b:?} {s:?} {out:?}");
            }
        }
    }
}
