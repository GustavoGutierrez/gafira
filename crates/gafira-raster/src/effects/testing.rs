//! Test helpers shared by the effect modules: images, runners and the checks every
//! one-input effect must pass (ARQ-110, ARQ-111, FD-33 (a, f, h)).

use gafira_doc::contract::{ParamError, ParamKind, ParamValue, ParamValues};

use super::Effect;
use crate::{EffectCx, EffectError, ImageView, ImageViewMut, PixelData, PixelDataMut, PixelFormat};

pub(crate) const SENTINEL: u8 = 0xAB;
pub(crate) const SENTINEL_F32: u32 = 0x7FC0_1234;

pub(crate) fn cx(scale: f32) -> EffectCx {
    EffectCx { scale }
}

/// Parameter values from `(id, number)` pairs; the rest take their defaults.
pub(crate) fn pv(items: &[(&str, f64)]) -> ParamValues {
    let mut v = ParamValues::new();
    for (id, n) in items {
        v.set(*id, ParamValue::Float(*n));
    }
    v
}

/// Premultiplied pixels: random alpha with zeros and 255s, colour never above alpha, and
/// transparent pixels that keep a colour.
pub(crate) fn img_u8(w: usize, h: usize, seed: u32) -> Vec<u8> {
    let mut v = Vec::with_capacity(w * h * 4);
    for i in 0..w * h {
        let x = (i as u32 ^ seed)
            .wrapping_mul(2_654_435_761)
            .wrapping_add(seed);
        let a = match x >> 29 {
            0 | 1 => 0,
            2..=4 => 255,
            _ => (x >> 8) as u8 | 1,
        };
        let c = |s: u32| ((x >> s) as u8).min(a);
        v.extend([c(3), c(11), c(17), a]);
        if a == 0 && x & 1 == 1 {
            let n = v.len();
            v[n - 4] = 7;
        }
    }
    v
}

/// `f32` premultiplied linear pixels, some channels beyond `[0, 1]`, a few `-0.0`.
pub(crate) fn img_f32(w: usize, h: usize, seed: u32) -> Vec<f32> {
    img_u8(w, h, seed)
        .chunks(4)
        .flat_map(|p| {
            let a = f32::from(p[3]) / 255.0;
            [
                f32::from(p[0]) / 255.0 * 1.2,
                f32::from(p[1]) / 255.0,
                -0.0 + f32::from(p[2]) / 300.0,
                a,
            ]
        })
        .collect()
}

pub(crate) fn bits(v: &[f32]) -> Vec<u32> {
    v.iter().map(|x| x.to_bits()).collect()
}

pub(crate) fn run8(
    e: &dyn Effect,
    c: &EffectCx,
    p: &ParamValues,
    src: &[u8],
    size: (u32, u32),
) -> Result<Vec<u8>, EffectError> {
    let mut dst = vec![SENTINEL; src.len()];
    e.run_cpu(
        c,
        p,
        ImageView::new(size.0, size.1, PixelData::Rgba8(src)).unwrap(),
        ImageViewMut::new(size.0, size.1, PixelDataMut::Rgba8(&mut dst)).unwrap(),
    )?;
    Ok(dst)
}

pub(crate) fn runf(
    e: &dyn Effect,
    c: &EffectCx,
    p: &ParamValues,
    src: &[f32],
    size: (u32, u32),
) -> Result<Vec<f32>, EffectError> {
    let mut dst = vec![f32::from_bits(SENTINEL_F32); src.len()];
    e.run_cpu(
        c,
        p,
        ImageView::new(size.0, size.1, PixelData::RgbaF32(src)).unwrap(),
        ImageViewMut::new(size.0, size.1, PixelDataMut::RgbaF32(&mut dst)).unwrap(),
    )?;
    Ok(dst)
}

/// Runs and leaves `dst` full of the sentinel when the effect says `Err`.
pub(crate) fn run8_keep(
    e: &dyn Effect,
    c: &EffectCx,
    p: &ParamValues,
    src: &[u8],
    size: (u32, u32),
) -> (Result<(), EffectError>, Vec<u8>) {
    let mut dst = vec![SENTINEL; src.len()];
    let r = e.run_cpu(
        c,
        p,
        ImageView::new(size.0, size.1, PixelData::Rgba8(src)).unwrap(),
        ImageViewMut::new(size.0, size.1, PixelDataMut::Rgba8(&mut dst)).unwrap(),
    );
    (r, dst)
}

pub(crate) fn runf_keep(
    e: &dyn Effect,
    c: &EffectCx,
    p: &ParamValues,
    src: &[f32],
    size: (u32, u32),
) -> (Result<(), EffectError>, Vec<f32>) {
    let mut dst = vec![f32::from_bits(SENTINEL_F32); src.len()];
    let r = e.run_cpu(
        c,
        p,
        ImageView::new(size.0, size.1, PixelData::RgbaF32(src)).unwrap(),
        ImageViewMut::new(size.0, size.1, PixelDataMut::RgbaF32(&mut dst)).unwrap(),
    );
    (r, dst)
}

/// sRGB byte to linear light, written here apart from the crate.
pub(crate) fn lin(byte: f64) -> f64 {
    let v = byte / 255.0;
    if v <= 0.04045 {
        v / 12.92
    } else {
        ((v + 0.055) / 1.055).powf(2.4)
    }
}

/// Encoded value of a linear one, in `f64`, odd about zero.
pub(crate) fn enc_f(linear: f64) -> f64 {
    let a = linear.abs();
    let e = if a <= 0.003_130_8 {
        12.92 * a
    } else {
        1.055 * a.powf(1.0 / 2.4) - 0.055
    };
    e.copysign(linear)
}

/// Linear value of an encoded one, in `f64`, odd about zero.
pub(crate) fn dec_f(encoded: f64) -> f64 {
    let a = encoded.abs();
    let l = if a <= 0.04045 {
        a / 12.92
    } else {
        ((a + 0.055) / 1.055).powf(2.4)
    };
    l.copysign(encoded)
}

/// Smoothstep from `a` to `b` (FD-33 `w(a, b, x)`).
pub(crate) fn smooth(a: f64, b: f64, x: f64) -> f64 {
    let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Linear light to the nearest sRGB byte, written here apart from the crate.
pub(crate) fn enc(linear: f64) -> u8 {
    let l = linear.clamp(0.0, 1.0);
    let v = if l <= 0.003_130_8 {
        12.92 * l
    } else {
        1.055 * l.powf(1.0 / 2.4) - 0.055
    };
    (v * 255.0).round() as u8
}

/// Every valid premultiplied pair `(c, a)` in 8 bits, the three channels offset from each other.
pub(crate) fn all_pairs_u8() -> Vec<u8> {
    let mut v = Vec::new();
    for a in 0..=255u16 {
        for c in 0..=a {
            let k = |d: u16| ((c + d) % (a + 1)) as u8;
            v.extend([c as u8, k(37), k(101), a as u8]);
        }
    }
    v
}

/// FD-33 (a): every parameter at its default is the identity, difference 0, both formats,
/// whether the values are empty or resolved.
pub(crate) fn assert_defaults_are_identity(e: &dyn Effect) {
    for (w, h) in [(7usize, 5usize), (1, 1), (300, 3)] {
        let s8 = img_u8(w, h, 3);
        let sf = img_f32(w, h, 5);
        for p in [ParamValues::new(), e.params().defaults()] {
            let size = (w as u32, h as u32);
            assert_eq!(run8(e, &cx(1.0), &p, &s8, size).unwrap(), s8, "{w}x{h}");
            let all = all_pairs_u8();
            let n = all.len() as u32 / 4;
            assert_eq!(run8(e, &cx(1.0), &p, &all, (n, 1)).unwrap(), all);
            assert_eq!(
                bits(&runf(e, &cx(1.0), &p, &sf, size).unwrap()),
                bits(&sf),
                "{w}x{h}"
            );
        }
    }
}

/// FD-33 (h), ARQ-110: NaN, infinities, just outside and far outside the range give the
/// `Err` of the parameter, and `dst` is untouched; the two ends are accepted.
pub(crate) fn assert_bad_parameters_are_err_and_ends_pass(e: &dyn Effect) {
    let s8 = img_u8(5, 4, 9);
    let sf = img_f32(5, 4, 9);
    let size = (5, 4);
    for def in e.params().params() {
        let ParamKind::Float { min, max, .. } = def.kind else {
            continue;
        };
        for end in [min, max] {
            let p = pv(&[(def.id, end)]);
            run8(e, &cx(1.0), &p, &s8, size).unwrap_or_else(|x| panic!("{} {end}: {x:?}", def.id));
            runf(e, &cx(1.0), &p, &sf, size).unwrap_or_else(|x| panic!("{} {end}: {x:?}", def.id));
        }
        let outside = [
            (min - (min.abs() * 1e-9).max(1e-9), false),
            (max + (max.abs() * 1e-9).max(1e-9), false),
            (min - 1.0, false),
            (max + 1.0, false),
            (f64::MIN, false),
            (f64::MAX, false),
            (f64::NAN, true),
            (f64::INFINITY, true),
            (f64::NEG_INFINITY, true),
        ];
        for (v, non_finite) in outside {
            let p = pv(&[(def.id, v)]);
            let want = if non_finite {
                ParamError::NonFinite { param: def.id }
            } else {
                ParamError::OutOfRange { param: def.id }
            };
            let (r, dst) = run8_keep(e, &cx(1.0), &p, &s8, size);
            assert_eq!(r, Err(EffectError::Param(want)), "{} {v}", def.id);
            assert!(dst.iter().all(|b| *b == SENTINEL), "{} {v}", def.id);
            let (r, dst) = runf_keep(e, &cx(1.0), &p, &sf, size);
            assert_eq!(r, Err(EffectError::Param(want)), "{} {v}", def.id);
            assert!(
                dst.iter().all(|b| b.to_bits() == SENTINEL_F32),
                "{} {v}",
                def.id
            );
        }
    }
}

/// ARQ-110: a non-finite `cx.scale` or `f32` pixel is `Err` and `dst` is untouched.
pub(crate) fn assert_non_finite_scale_and_pixels_are_err(e: &dyn Effect) {
    let s8 = img_u8(4, 4, 2);
    let mut sf = img_f32(4, 4, 2);
    let size = (4, 4);
    for scale in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        let (r, dst) = run8_keep(e, &cx(scale), &ParamValues::new(), &s8, size);
        assert_eq!(
            r,
            Err(EffectError::Buffer(crate::BufferError::NonFinite {
                field: "cx.scale",
                index: 0
            })),
            "scale {scale}"
        );
        assert!(dst.iter().all(|b| *b == SENTINEL));
    }
    for bad in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        for index in [0usize, 3, 5, sf.len() - 1] {
            let keep = sf[index];
            sf[index] = bad;
            let (r, dst) = runf_keep(e, &cx(1.0), &ParamValues::new(), &sf, size);
            assert_eq!(
                r,
                Err(EffectError::Buffer(crate::BufferError::NonFinite {
                    field: "src",
                    index
                })),
                "{bad} at {index}"
            );
            assert!(dst.iter().all(|b| b.to_bits() == SENTINEL_F32));
            sf[index] = keep;
        }
    }
}

/// Formats and sizes: `Alpha8` is unsupported, a mix of formats or sizes is `Err`, and
/// `dst` is untouched.
pub(crate) fn assert_format_and_size_errors(e: &dyn Effect) {
    let a8 = vec![1u8; 6];
    let mut dst8 = vec![SENTINEL; 6];
    let r = e.run_cpu(
        &cx(1.0),
        &ParamValues::new(),
        ImageView::new(3, 2, PixelData::Alpha8(&a8)).unwrap(),
        ImageViewMut::new(3, 2, PixelDataMut::Alpha8(&mut dst8)).unwrap(),
    );
    assert_eq!(
        r,
        Err(EffectError::UnsupportedFormat {
            format: PixelFormat::Alpha8
        })
    );
    let s8 = img_u8(3, 2, 1);
    let mut df = vec![f32::from_bits(SENTINEL_F32); 24];
    let r = e.run_cpu(
        &cx(1.0),
        &ParamValues::new(),
        ImageView::new(3, 2, PixelData::Rgba8(&s8)).unwrap(),
        ImageViewMut::new(3, 2, PixelDataMut::RgbaF32(&mut df)).unwrap(),
    );
    assert_eq!(
        r,
        Err(EffectError::FormatMismatch {
            field: "dst",
            expected: PixelFormat::Rgba8,
            actual: PixelFormat::RgbaF32
        })
    );
    let mut small = vec![SENTINEL; 8];
    let r = e.run_cpu(
        &cx(1.0),
        &ParamValues::new(),
        ImageView::new(3, 2, PixelData::Rgba8(&s8)).unwrap(),
        ImageViewMut::new(2, 1, PixelDataMut::Rgba8(&mut small)).unwrap(),
    );
    assert_eq!(
        r,
        Err(EffectError::SizeMismatch {
            field: "dst",
            expected: (3, 2),
            actual: (2, 1)
        })
    );
    assert!(small.iter().all(|b| *b == SENTINEL));
}

/// Size 0 is `Ok` and writes nothing (ARQ-118 numeric rule).
pub(crate) fn assert_empty_images_are_ok(e: &dyn Effect) {
    for size in [(0u32, 0u32), (0, 5), (5, 0)] {
        assert_eq!(
            run8(e, &cx(1.0), &ParamValues::new(), &[], size).unwrap(),
            Vec::<u8>::new()
        );
        assert!(
            runf(e, &cx(1.0), &ParamValues::new(), &[], size)
                .unwrap()
                .is_empty()
        );
    }
}

/// FD-33 (f): alpha is never touched and a pixel with alpha 0 stays as it was, with every
/// parameter at each end of its range; the premultiplied colour never exceeds its alpha in
/// 8 bits.
pub(crate) fn assert_alpha_is_kept(e: &dyn Effect, scale: f32) {
    let (w, h) = (9usize, 7usize);
    let s8 = img_u8(w, h, 11);
    let sf = img_f32(w, h, 11);
    let size = (w as u32, h as u32);
    let mut sets: Vec<ParamValues> = Vec::new();
    for def in e.params().params() {
        if let ParamKind::Float { min, max, .. } = def.kind {
            sets.push(pv(&[(def.id, min)]));
            sets.push(pv(&[(def.id, max)]));
        }
    }
    let all_max: Vec<(&str, f64)> = e
        .params()
        .params()
        .iter()
        .filter_map(|d| match d.kind {
            ParamKind::Float { max, .. } => Some((d.id, max)),
            _ => None,
        })
        .collect();
    sets.push(pv(&all_max));
    for p in &sets {
        let o8 = run8(e, &cx(scale), p, &s8, size).unwrap();
        for (i, (a, b)) in s8.chunks(4).zip(o8.chunks(4)).enumerate() {
            assert_eq!(a[3], b[3], "alpha at {i} {p:?}");
            if a[3] == 0 {
                assert_eq!(a, b, "transparent pixel {i} {p:?}");
            }
            assert!(
                a[3] == 0 || b[..3].iter().all(|c| *c <= b[3]),
                "c > a at {i} {p:?}"
            );
        }
        let of = runf(e, &cx(scale), p, &sf, size).unwrap();
        for (i, (a, b)) in sf.chunks(4).zip(of.chunks(4)).enumerate() {
            assert_eq!(a[3].to_bits(), b[3].to_bits(), "alpha at {i} {p:?}");
            if a[3] == 0.0 {
                assert_eq!(bits(a), bits(b), "transparent pixel {i} {p:?}");
            }
        }
    }
}

/// ARQ-111: two runs give the same bytes.
pub(crate) fn assert_deterministic(e: &dyn Effect, p: &ParamValues) {
    let s8 = img_u8(8, 6, 4);
    let sf = img_f32(8, 6, 4);
    assert_eq!(
        run8(e, &cx(1.0), p, &s8, (8, 6)).unwrap(),
        run8(e, &cx(1.0), p, &s8, (8, 6)).unwrap()
    );
    assert_eq!(
        bits(&runf(e, &cx(1.0), p, &sf, (8, 6)).unwrap()),
        bits(&runf(e, &cx(1.0), p, &sf, (8, 6)).unwrap())
    );
}
