use gafira_anim::Interpolate;
use gafira_color::{Color, ColorSpaceId, DisplayP3Rgba, LinearRgba, Rgba};
use proptest::prelude::*;

fn build(space: ColorSpaceId, c: [f32; 4]) -> Color {
    match space {
        ColorSpaceId::Srgb => Rgba::new(c[0], c[1], c[2], c[3]).into(),
        ColorSpaceId::LinearSrgb => LinearRgba::new(c[0], c[1], c[2], c[3]).into(),
        ColorSpaceId::DisplayP3 => DisplayP3Rgba::new(c[0], c[1], c[2], c[3]).into(),
    }
}

fn bits(c: Color) -> [u32; 4] {
    let l = match c {
        Color::Srgb(v) => [v.r, v.g, v.b, v.a],
        Color::LinearSrgb(v) => [v.r, v.g, v.b, v.a],
        Color::DisplayP3(v) => [v.r, v.g, v.b, v.a],
    };
    l.map(f32::to_bits)
}

fn lin(c: Color) -> [f64; 4] {
    let l = c.to_linear_srgb();
    [l.r, l.g, l.b, l.a].map(f64::from)
}

/// f64 reference of the §3.4 rule, in linear sRGB. The result alpha is the f32
/// linear interpolation of the alphas (bit exact, ARQ-19) and is the divisor
/// and the branch test, as §3.4 says ("divided by its own a").
fn reference(a: Color, b: Color, t: f32) -> [f64; 3] {
    let alpha = f64::from(a.alpha() + (b.alpha() - a.alpha()) * t);
    let (a, b, t) = (lin(a), lin(b), f64::from(t));
    let mut out = [0.0; 3];
    for i in 0..3 {
        out[i] = if alpha <= 0.0 {
            a[i] + (b[i] - a[i]) * t
        } else {
            let (pa, pb) = (a[i] * a[3], b[i] * b[3]);
            (pa + (pb - pa) * t) / alpha
        };
    }
    out
}

fn assert_close(got: Color, want: [f64; 3]) {
    let got = lin(got);
    for i in 0..3 {
        let tol = 1e-5 * want[i].abs().max(1.0);
        assert!(
            (got[i] - want[i]).abs() <= tol,
            "ch {i}: {got:?} vs {want:?}"
        );
    }
}

fn space() -> impl Strategy<Value = ColorSpaceId> {
    prop::sample::select(ColorSpaceId::ALL.to_vec())
}

fn color() -> impl Strategy<Value = Color> {
    (space(), prop::array::uniform3(-0.5_f32..1.5), 0.0_f32..=1.0)
        .prop_map(|(s, c, a)| build(s, [c[0], c[1], c[2], a]))
}

proptest! {
    // ARQ-19: exact endpoints, all four channels, any pair of spaces.
    #[test]
    fn arq_19_color_endpoints_are_exact(a in color(), b in color()) {
        prop_assert_eq!(bits(a.interpolate(&b, 0.0)), bits(a));
        prop_assert_eq!(bits(a.interpolate(&b, 1.0)), bits(b));
    }

    // ARQ-19: r, g, b match the premultiplied f64 reference in linear sRGB (<= 1e-5).
    #[test]
    fn arq_19_color_matches_premultiplied_reference(a in color(), b in color(), t in -0.5_f32..1.5) {
        let r = a.interpolate(&b, t);
        let want = reference(a, b, t);
        let got = lin(r);
        for i in 0..3 {
            let tol = 1e-5 * want[i].abs().max(1.0);
            prop_assert!((got[i] - want[i]).abs() <= tol, "ch {i}: {got:?} vs {want:?}");
        }
    }

    // ARQ-19: alpha is the linear interpolation of the alphas, bit for bit in f32.
    #[test]
    fn arq_19_color_alpha_is_linear_and_bit_exact(a in color(), b in color(), t in -0.5_f32..1.5) {
        let (aa, ab) = (a.alpha(), b.alpha());
        let r = a.interpolate(&b, t);
        prop_assert_eq!(r.alpha().to_bits(), (aa + (ab - aa) * t).to_bits());
    }
}

// ARQ-19: a shared space is kept; any other pair gives linear sRGB.
#[test]
fn arq_19_color_result_space_rule() {
    let c = [0.2, 0.4, 0.6, 1.0];
    let d = [0.8, 0.1, 0.3, 1.0];
    for s in ColorSpaceId::ALL {
        assert_eq!(build(s, c).interpolate(&build(s, d), 0.5).space(), s);
    }
    for s in ColorSpaceId::ALL {
        for o in ColorSpaceId::ALL {
            if s != o {
                let r = build(s, c).interpolate(&build(o, d), 0.5);
                assert_eq!(r.space(), ColorSpaceId::LinearSrgb, "{s:?} {o:?}");
            }
        }
    }
}

// ARQ-19: interpolation happens in linear light (sRGB 0 -> 1 midpoint is 0.5 linear).
#[test]
fn arq_19_color_blends_in_linear_light() {
    let black = build(ColorSpaceId::Srgb, [0.0, 0.0, 0.0, 1.0]);
    let white = build(ColorSpaceId::Srgb, [1.0, 1.0, 1.0, 1.0]);
    let r = black.interpolate(&white, 0.25);
    assert_eq!(r.space(), ColorSpaceId::Srgb);
    assert_close(r, [0.25, 0.25, 0.25]);
    // Not the encoded-space average (0.25).
    let Color::Srgb(v) = r else { unreachable!() };
    assert!((v.r - 0.25).abs() > 0.1, "{v:?}");
}

// ARQ-19: premultiplied alpha, which differs from the straight blend.
#[test]
fn arq_19_color_premultiplies_alpha() {
    let a = build(ColorSpaceId::LinearSrgb, [1.0, 0.0, 0.0, 1.0]);
    let b = build(ColorSpaceId::LinearSrgb, [0.0, 0.0, 1.0, 0.25]);
    let r = a.interpolate(&b, 0.5);
    // alpha 0.625; r = 0.5 / 0.625 = 0.8, b = 0.125 / 0.625 = 0.2 (straight would be 0.5 / 0.5).
    assert_close(r, [0.8, 0.0, 0.2]);
    assert_eq!(r.alpha(), 0.625);
}

// ARQ-19: opaque red to transparent black keeps the red hue for every a > 0.
#[test]
fn arq_19_opaque_red_to_transparent_black_keeps_hue() {
    for space in ColorSpaceId::ALL {
        let red = build(space, [1.0, 0.0, 0.0, 1.0]);
        let black = build(space, [0.0, 0.0, 0.0, 0.0]);
        for t in [0.1_f32, 0.25, 0.5, 0.75, 0.9, 0.999] {
            let r = red.interpolate(&black, t);
            assert!(r.alpha() > 0.0);
            assert_eq!(r.space(), space);
            // The criterion holds in the result space's own channels.
            let own = match r {
                Color::Srgb(v) => [v.r, v.g, v.b],
                Color::LinearSrgb(v) => [v.r, v.g, v.b],
                Color::DisplayP3(v) => [v.r, v.g, v.b],
            };
            for (got, want) in own.into_iter().zip([1.0_f32, 0.0, 0.0]) {
                assert!((got - want).abs() <= 1e-5, "{space:?} {t}: {own:?}");
            }
        }
    }
}

// ARQ-19 (Q-31): two transparent ends blend straight, finite, alpha 0.
#[test]
fn arq_19_color_transparent_to_transparent_blends_straight() {
    for space in ColorSpaceId::ALL {
        let a = build(space, [1.0, 0.0, 0.0, 0.0]);
        let b = build(space, [0.0, 0.0, 1.0, 0.0]);
        let r = a.interpolate(&b, 0.5);
        assert!(r.is_finite());
        assert_eq!(r.alpha(), 0.0);
        assert_close(r, reference(a, b, 0.5));
    }
    let a = build(ColorSpaceId::Srgb, [1.0, 0.0, 0.0, 0.0]);
    let b = build(ColorSpaceId::Srgb, [0.0, 0.0, 1.0, 0.0]);
    let l = lin(a.interpolate(&b, 0.5));
    assert!(
        (l[0] - 0.5).abs() <= 1e-5 && (l[2] - 0.5).abs() <= 1e-5,
        "{l:?}"
    );
}

// ARQ-19 (Q-31): negative resulting alpha is kept unclamped and the channels keep their sign.
#[test]
fn arq_19_color_negative_alpha_extrapolates_straight() {
    let a = build(ColorSpaceId::Srgb, [1.0, 0.5, 0.0, 0.5]);
    let b = build(ColorSpaceId::Srgb, [0.0, 0.25, 1.0, 0.0]);
    let r = a.interpolate(&b, 1.5);
    assert_eq!(r.alpha(), -0.25);
    assert!(r.is_finite());
    assert_close(r, reference(a, b, 1.5));
    let l = lin(r);
    // Straight extrapolation: red goes below 0 in linear light, blue above 1.
    assert!(l[0] < 0.0 && l[2] > 1.0, "{l:?}");
}

// ARQ-19: no clamping of channels outside [0, 1].
#[test]
fn arq_19_color_does_not_clamp_channels() {
    let a = build(ColorSpaceId::LinearSrgb, [0.0, 0.5, 1.0, 1.0]);
    let b = build(ColorSpaceId::LinearSrgb, [1.0, 0.5, 0.0, 1.0]);
    let r = a.interpolate(&b, 2.0);
    assert_close(r, [2.0, 0.5, -1.0]);
}

// ARQ-19 / ARQ-18: is_finite is false for NaN or infinity in any channel and space.
#[test]
fn arq_19_color_is_finite_checks_every_channel() {
    for space in ColorSpaceId::ALL {
        assert!(build(space, [0.0, -1.0, 2.0, 1.0]).is_finite());
        for slot in 0..4 {
            for bad in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
                let mut c = [0.5_f32; 4];
                c[slot] = bad;
                assert!(
                    !Interpolate::is_finite(&build(space, c)),
                    "{space:?} {slot}"
                );
            }
        }
    }
}
