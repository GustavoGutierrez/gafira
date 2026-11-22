//! S85: an image made to look as its document says.

use gafira_color::{Color, Rgba};
use gafira_doc::{
    AssetRef, ColorShift, ContentHash, ImageAdjust, ImageFilter, ImageLook, ImageShadow, ImageZone,
    ShadowKind, ZoneKind,
};
use gafira_raster::look::{FILTER_GROUPS, apply_look, cast, filter, reduced};
use gafira_raster::{AlphaMask, ImageBuf, PixelBuf, PixelData, premultiply_u8};
use proptest::prelude::*;

const MASK: AssetRef = AssetRef::new(ContentHash([5; 32]));
const KINDS: [ShadowKind; 7] = [
    ShadowKind::Glow,
    ShadowKind::Drop,
    ShadowKind::Outline,
    ShadowKind::Curved,
    ShadowKind::Floating,
    ShadowKind::Angled,
    ShadowKind::Ground,
];

fn buf(w: u32, h: u32, px: impl Fn(u32, u32) -> [u8; 4]) -> ImageBuf {
    let bytes = (0..w * h).flat_map(|i| px(i % w, i / w)).collect();
    ImageBuf::new(w, h, PixelBuf::Rgba8(bytes)).expect("an image")
}

fn bytes(image: &ImageBuf) -> Vec<u8> {
    match image.view().data() {
        PixelData::Rgba8(v) => v.to_vec(),
        _ => panic!("8 bits"),
    }
}

/// Colours of every kind, some see-through.
fn scene(w: u32, h: u32) -> ImageBuf {
    buf(w, h, |x, y| {
        let n = (x * 7919 + y * 104_729).wrapping_mul(2_654_435_761);
        let [r, g, b, a] = n.to_le_bytes();
        let a = if x % 5 == 0 { a } else { 255 };
        [
            premultiply_u8(r, a),
            premultiply_u8(g, a),
            premultiply_u8(b, a),
            a,
        ]
    })
}

fn at(image: &ImageBuf, x: u32, y: u32) -> [u8; 4] {
    let i = ((y * image.width() + x) * 4) as usize;
    bytes(image)[i..i + 4].try_into().unwrap()
}

fn run(look: &ImageLook, image: &ImageBuf) -> ImageBuf {
    let left = |asset: AssetRef| {
        assert_eq!(asset, MASK);
        AlphaMask::new(4, 1, vec![255, 255, 0, 0]).ok()
    };
    apply_look(look, image, &left, 1.0).expect("a look")
}

fn adjust(change: impl Fn(&mut ImageAdjust)) -> ImageAdjust {
    let mut a = ImageAdjust::default();
    change(&mut a);
    a
}

// S85-02: nothing changes nothing; what changes keeps the opacity.
#[test]
fn s85_02_plain_and_opacity() {
    let image = scene(40, 30);
    assert_eq!(run(&ImageLook::default(), &image), image);
    for a in [
        adjust(|a| a.temperature = 60.0),
        adjust(|a| a.contrast = 60.0),
        adjust(|a| a.saturation = -80.0),
        adjust(|a| a.hue = 90.0),
        adjust(|a| a.invert = true),
        adjust(|a| a.sharpness = 80.0),
        adjust(|a| a.vignette = -90.0),
        adjust(|a| a.clarity = 90.0),
        adjust(|a| a.blacks = 70.0),
    ] {
        let out = run(
            &ImageLook {
                adjust: a.clone(),
                ..ImageLook::default()
            },
            &image,
        );
        assert_ne!(out, image, "{a:?}");
        let alpha = |i: &ImageBuf| bytes(i).chunks(4).map(|p| p[3]).collect::<Vec<_>>();
        assert_eq!(alpha(&out), alpha(&image), "{a:?}");
    }
}

// S85-02, FD-33 (e): the values of the table, through the whole way.
#[test]
fn s85_02_known_values() {
    let gray = buf(4, 4, |_, _| [128, 128, 128, 255]);
    let of = |a: ImageAdjust| {
        at(
            &run(
                &ImageLook {
                    adjust: a,
                    ..ImageLook::default()
                },
                &gray,
            ),
            1,
            1,
        )
    };
    assert_eq!(of(adjust(|a| a.invert = true)), [127, 127, 127, 255]);
    assert_eq!(of(adjust(|a| a.contrast = 100.0))[0].abs_diff(128), 0);
    let warm = of(adjust(|a| a.temperature = 100.0));
    assert!(warm[0] > 140 && warm[2] < 115 && warm[1] == 128, "{warm:?}");
    let red = buf(4, 4, |_, _| [200, 40, 40, 255]);
    let gone = run(
        &ImageLook {
            adjust: adjust(|a| a.saturation = -100.0),
            ..ImageLook::default()
        },
        &red,
    );
    let p = at(&gone, 0, 0);
    assert!(p[0] == p[1] && p[1] == p[2], "{p:?}");
}

// FD-32 (a): a zone changes what is in it and not a byte of the rest.
#[test]
fn s85_02_zones() {
    let image = scene(40, 10);
    let zone = |inverted| ImageLook {
        zones: vec![ImageZone {
            kind: ZoneKind::Area,
            mask: MASK,
            inverted,
            adjust: adjust(|a| a.brightness = 60.0),
        }],
        ..ImageLook::default()
    };
    for (inverted, same, other) in [(false, 30..40, 0..10), (true, 0..10, 30..40)] {
        let out = run(&zone(inverted), &image);
        let mut changed = 0;
        for y in 0..10 {
            for x in same.clone() {
                assert_eq!(at(&out, x, y), at(&image, x, y));
            }
            for x in other.clone() {
                changed += u32::from(at(&out, x, y) != at(&image, x, y));
            }
        }
        assert!(changed > 50);
    }
    // A mask that is lost changes nothing.
    let lost = apply_look(&zone(false), &image, &|_| None, 1.0).expect("a look");
    assert_eq!(lost, image);
}

// FD-33: a colour is moved, and one far from it is not.
#[test]
fn s85_02_colours() {
    let image = buf(20, 4, |x, _| {
        if x < 10 {
            [220, 30, 30, 255]
        } else {
            [30, 60, 220, 255]
        }
    });
    let red = Color::Srgb(Rgba::new(220.0 / 255.0, 30.0 / 255.0, 30.0 / 255.0, 1.0));
    let look = ImageLook {
        adjust: adjust(|a| {
            a.colors = vec![ColorShift {
                color: red,
                hue: 120.0,
                saturation: 0.0,
            }]
        }),
        ..ImageLook::default()
    };
    let out = run(&look, &image);
    let moved = at(&out, 2, 2);
    assert!(moved[1] > moved[0], "{moved:?}");
    assert_eq!(at(&out, 15, 2), [30, 60, 220, 255]);
}

// S85-03: each filter is known, its own, and stronger as it is asked; none
// at 0; one that is not known does nothing.
#[test]
fn s85_03_filters() {
    let image = buf(24, 24, |x, y| [(x * 10) as u8, (y * 10) as u8, 120, 255]);
    let with = |id: &str, intensity| {
        run(
            &ImageLook {
                filter: Some(ImageFilter {
                    id: id.into(),
                    intensity,
                }),
                ..ImageLook::default()
            },
            &image,
        )
    };
    let far = |a: &ImageBuf| {
        bytes(a)
            .iter()
            .zip(bytes(&image))
            .map(|(a, b)| u64::from(a.abs_diff(b)))
            .sum::<u64>()
    };
    let mut seen: Vec<Vec<u8>> = Vec::new();
    for id in FILTER_GROUPS.iter().flat_map(|g| g.filters) {
        assert_eq!(filter(id).expect(id).id, id);
        assert_eq!(with(id, 0.0), image, "{id}");
        let (half, whole) = (with(id, 50.0), with(id, 100.0));
        assert!(far(&whole) > far(&half) && far(&half) > 0, "{id}");
        assert!(!seen.contains(&bytes(&whole)), "{id}");
        seen.push(bytes(&whole));
    }
    assert_eq!(with("what", 100.0), image);
}

// S85-04: the image is whole in the middle of its shadow, the shadow is
// around it in its colour, and what is see-through shows it.
#[test]
fn s85_04_shadows() {
    let image = buf(120, 80, |x, _| {
        if x < 60 {
            [10, 200, 10, 255]
        } else {
            [0, 0, 0, 0]
        }
    });
    for kind in KINDS {
        let mut s = ImageShadow::new(kind);
        s.color = Color::Srgb(Rgba::new(1.0, 0.0, 0.0, 1.0));
        s.opacity = 1.0;
        let out = cast(&image, &s).expect("a shadow");
        let m = (s.reach() * 80.0).ceil() as u32;
        assert_eq!(
            (out.width(), out.height()),
            (120 + 2 * m, 80 + 2 * m),
            "{kind:?}"
        );
        for (x, y) in [(0, 0), (30, 40), (59, 79)] {
            assert_eq!(at(&out, x + m, y + m), at(&image, x, y), "{kind:?}");
        }
        let data = bytes(&out);
        let shaded: Vec<&[u8]> = data.chunks(4).filter(|p| p[3] > 0 && p[1] < 100).collect();
        assert!(shaded.len() > 30, "{kind:?}");
        assert!(
            shaded
                .iter()
                .filter(|p| p[3] > 40)
                .all(|p| p[0] > 20 && p[2] < 20),
            "{kind:?}"
        );
        // Faint at the edge: nothing is cut off. An outline has a hard edge,
        // which its margin holds whole.
        let (w, h) = (out.width(), out.height());
        let faint = if kind == ShadowKind::Outline { 255 } else { 12 };
        for x in 0..w {
            assert!(
                at(&out, x, 0)[3] <= faint && at(&out, x, h - 1)[3] <= faint,
                "{kind:?}"
            );
        }
        for y in 0..h {
            assert!(
                at(&out, 0, y)[3] <= faint && at(&out, w - 1, y)[3] <= faint,
                "{kind:?}"
            );
        }
        s.opacity = 0.0;
        let clear = cast(&image, &s).expect("a shadow");
        assert!(
            bytes(&clear)
                .chunks(4)
                .all(|p| p[3] == 0 || p == [10, 200, 10, 255]),
            "{kind:?}"
        );
    }
    // FD-35 (a): with no blur, the opacity of the image, moved.
    let mut s = ImageShadow::new(ShadowKind::Drop);
    (s.blur, s.distance, s.angle_deg, s.opacity) = (0.0, 0.1, 0.0, 1.0);
    let out = cast(&image, &s).expect("a shadow");
    let m = (s.reach() * 80.0).ceil() as u32;
    assert_eq!(
        (at(&out, m + 66, m + 40)[3], at(&out, m + 69, m + 40)[3]),
        (255, 0)
    );
}

// S85-02: a smaller one looks as the whole one.
#[test]
fn s85_02_a_thumbnail_looks_the_same() {
    let image = buf(200, 100, |x, y| [(x + 20) as u8, (y * 2) as u8, 90, 255]);
    let small = buf(100, 50, |x, y| [(x * 2 + 20) as u8, (y * 4) as u8, 90, 255]);
    let look = ImageLook {
        adjust: adjust(|a| (a.contrast, a.temperature, a.vignette) = (30.0, 20.0, -50.0)),
        filter: Some(ImageFilter {
            id: "ocaso".into(),
            intensity: 70.0,
        }),
        ..ImageLook::default()
    };
    let whole = apply_look(&look, &image, &|_| None, 1.0).expect("a look");
    let half = apply_look(&look, &small, &|_| None, 0.5).expect("a look");
    for (x, y) in [(10, 10), (50, 25), (90, 40)] {
        let (a, b) = (at(&whole, x * 2, y * 2), at(&half, x, y));
        assert!(
            a.iter().zip(b).all(|(a, b)| a.abs_diff(b) < 6),
            "{a:?} {b:?}"
        );
    }
}

// S85-07: one to be seen small is made smaller first, by a whole number, and
// never grown.
#[test]
fn s85_07_reduced() {
    let image = buf(10, 7, |x, _| {
        if x < 4 {
            [200, 0, 0, 255]
        } else {
            [0, 0, 100, 255]
        }
    });
    let (small, scale) = reduced(&image, 5).expect("smaller");
    assert_eq!((small.width(), small.height(), scale), (5, 4, 0.5));
    assert_eq!(at(&small, 0, 0), [200, 0, 0, 255]);
    assert_eq!(at(&small, 4, 3), [0, 0, 100, 255]);
    assert!(reduced(&image, 10).is_none() && reduced(&image, 0).is_none());
}

proptest! {
    // S85-02: any look, any image: no failure, the same size, bytes in order.
    #[test]
    fn s85_02_any_look(w in 1_u32..24, h in 1_u32..24, v in prop::collection::vec(-100.0..100.0_f64, 12), kind in 0_usize..7, lengths in prop::collection::vec(0.0..0.3_f64, 3), angle in -180.0..180.0_f64) {
        let image = scene(w, h);
        let a = adjust(|a| {
            (a.temperature, a.tint, a.brightness, a.contrast) = (v[0], v[1], v[2], v[3]);
            (a.highlights, a.shadows, a.whites, a.blacks) = (v[4], v[5], v[6], v[7]);
            (a.saturation, a.vibrance, a.vignette, a.sharpness) = (v[8], v[9], v[10], v[11].abs());
        });
        let mut s = ImageShadow::new(KINDS[kind]);
        (s.blur, s.distance, s.size, s.angle_deg) = (lengths[0], lengths[1], lengths[2], angle);
        let look = ImageLook { adjust: a, shadow: Some(s), ..ImageLook::default() };
        prop_assert!(look.is_valid());
        let out = apply_look(&look, &image, &|_| None, 1.0).expect("a look");
        let m = (s.reach() * f64::from(w.min(h))).ceil() as u32;
        prop_assert_eq!((out.width(), out.height()), (w + 2 * m, h + 2 * m));
        prop_assert!(bytes(&out).chunks(4).all(|p| p[..3].iter().all(|c| *c <= p[3])), "premultiplied");
    }
}
