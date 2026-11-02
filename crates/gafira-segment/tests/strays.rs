//! D-47, ARQ-131, ARQ-133: a band colour set aside by the stray rule clears
//! `reliable` and never changes the mask. The masks are compared by hash with
//! the ones the rule gave before the condition existed (recorded from that code).

use std::sync::Arc;

use gafira_doc::contract::{CancelToken, NoProgress};
use gafira_raster::{ImageView, PixelData};
use gafira_segment::{ClassicSegmenter, SegHints, SegZone, Segmentation, Segmenter};

fn seg() -> ClassicSegmenter {
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(2)
        .build()
        .unwrap();
    ClassicSegmenter::new(Arc::new(pool))
}

fn run(w: u32, h: u32, data: &[u8], zone: SegZone) -> Segmentation {
    let view = ImageView::new(w, h, PixelData::Rgba8(data)).unwrap();
    let hints = SegHints {
        zone,
        ..SegHints::default()
    };
    seg()
        .segment(view, &hints, &NoProgress, &CancelToken::new())
        .unwrap()
}

fn fnv(data: &[u8]) -> u64 {
    data.iter().fold(0xcbf2_9ce4_8422_2325u64, |h, b| {
        (h ^ u64::from(*b)).wrapping_mul(0x0100_0000_01b3)
    })
}

fn paint(w: u32, h: u32, colour: impl Fn(u32, u32) -> [u8; 3]) -> Vec<u8> {
    (0..w * h)
        .flat_map(|i| {
            let [r, g, b] = colour(i % w, i / w);
            [r, g, b, 255]
        })
        .collect()
}

fn disk(cx: f64, cy: f64, r: f64) -> impl Fn(u32, u32) -> bool {
    move |x, y| {
        let (dx, dy) = (f64::from(x) - cx, f64::from(y) - cy);
        dx * dx + dy * dy <= r * r
    }
}

const PAIRS: [([u8; 3], [u8; 3]); 2] = [
    ([120, 160, 210], [90, 140, 70]),
    ([205, 200, 190], [120, 85, 60]),
];

/// 1600x1200, a red disc of radius 240 on two flat colours split at `pct` %.
fn two_colour(pair: usize, vertical: bool, pct: u32) -> Vec<u8> {
    let (a, b) = PAIRS[pair];
    let on_disc = disk(800.0, 500.0, 240.0);
    paint(1600, 1200, |x, y| {
        if on_disc(x, y) {
            return [190, 50, 50];
        }
        let at = if vertical {
            x * 100 / 1600
        } else {
            y * 100 / 1200
        };
        if at < pct { a } else { b }
    })
}

/// 1200x900, a subject of this shape on a flat background.
fn touching(inside: impl Fn(u32, u32) -> bool) -> Vec<u8> {
    paint(1200, 900, |x, y| {
        if inside(x, y) {
            [190, 70, 60]
        } else {
            [70, 110, 150]
        }
    })
}

/// (name, hash of the Primer plano mask, hash of the Elemento mask) the stray
/// rule gave before D-47.
const TWO_BEFORE: [(&str, u64, u64); 20] = [
    ("two_p0_h_50_disc", 0x54a3c66e10299085, 0x54a3c66e10299085),
    ("two_p0_h_60_disc", 0x4fa9d3695e0cd125, 0x4fa9d3695e0cd125),
    ("two_p0_h_70_disc", 0x8ee08d9d4751e599, 0xd017e8ca218db2b2),
    ("two_p0_h_80_disc", 0x88c5e22912ae28e9, 0xd017e8ca218db2b2),
    ("two_p0_h_90_disc", 0x39e1d47697b0800b, 0xd017e8ca218db2b2),
    ("two_p0_v_50_disc", 0xb97e89295d9a9d61, 0xb97e89295d9a9d61),
    ("two_p0_v_60_disc", 0xef67ede60053b255, 0xef67ede60053b255),
    ("two_p0_v_70_disc", 0x77993b5c3c4ed841, 0xd017e8ca218db2b2),
    ("two_p0_v_80_disc", 0x79e09137bb5c7d41, 0xd017e8ca218db2b2),
    ("two_p0_v_90_disc", 0xf34a4f718b4f7bc1, 0xd017e8ca218db2b2),
    ("two_p1_h_50_disc", 0xca3bbf3fe36c5bd3, 0xca3bbf3fe36c5bd3),
    ("two_p1_h_60_disc", 0x0d2fdf5d6370d9eb, 0x0d2fdf5d6370d9eb),
    ("two_p1_h_70_disc", 0xa67b065381472374, 0x991b875b96b2fa7e),
    ("two_p1_h_80_disc", 0xb1aa8bfb329f2ee4, 0x991b875b96b2fa7e),
    ("two_p1_h_90_disc", 0x2b07b2a4ae8c4396, 0x991b875b96b2fa7e),
    ("two_p1_v_50_disc", 0xe2b940a6237717e3, 0xe2b940a6237717e3),
    ("two_p1_v_60_disc", 0x0b570c512ecd759b, 0x0b570c512ecd759b),
    ("two_p1_v_70_disc", 0x80b73cf9e3c0fd9c, 0x991b875b96b2fa7e),
    ("two_p1_v_80_disc", 0xa75511f17d89ff5c, 0x991b875b96b2fa7e),
    ("two_p1_v_90_disc", 0x6ecfdfd8a11ee61c, 0x991b875b96b2fa7e),
];
const TOUCH_BEFORE: [(&str, u64, u64); 6] = [
    ("touch_left", 0x0a77238fc56b9c2c, 0x0a77238fc56b9c2c),
    ("touch_corner", 0x4265b1775fe75cd1, 0x4265b1775fe75cd1),
    ("touch_top", 0x6287cab9572e997a, 0x6287cab9572e997a),
    ("touch_side", 0x39522957de63e69c, 0x39522957de63e69c),
    ("touch_wide_edge", 0x663f345d76ea59a0, 0x663f345d76ea59a0),
    ("touch_narrow_edge", 0x1f1b500349466d47, 0x1f1b500349466d47),
];

/// Primer plano, Elemento and Fondo of one input: the masks are the ones before
/// the condition and Fondo is the exact complement. Returns the three `reliable`.
fn check(name: &str, (w, h): (u32, u32), data: &[u8], before: (u64, u64)) -> [bool; 3] {
    let fg = run(w, h, data, SegZone::Foreground);
    let el = run(w, h, data, SegZone::Element);
    let bg = run(w, h, data, SegZone::Background);
    assert_eq!(fnv(fg.mask.data()), before.0, "{name}: Primer plano mask");
    assert_eq!(fnv(el.mask.data()), before.1, "{name}: Elemento mask");
    let complement: Vec<u8> = fg.mask.data().iter().map(|a| 255 - a).collect();
    assert_eq!(bg.mask.data(), complement.as_slice(), "{name}: Fondo mask");
    [fg.reliable, el.reliable, bg.reliable]
}

fn two_colour_case(pair: usize, vertical: bool) {
    for pct in [50, 60, 70, 80, 90] {
        let name = format!(
            "two_p{pair}_{}_{pct}_disc",
            if vertical { "v" } else { "h" }
        );
        let before = TWO_BEFORE.iter().find(|r| r.0 == name).unwrap();
        let data = two_colour(pair, vertical, pct);
        let got = check(&name, (1600, 1200), &data, (before.1, before.2));
        assert_eq!(got, [false; 3], "{name}: reliable in fg, el, bg");
    }
}

// ARQ-131, D-47 (AC1, AC4): two flat background colours split at 50 to 90 %:
// `Ok`, not reliable in the three zones, the mask as before.
macro_rules! two_colour_tests {
    ($($name:ident: $pair:expr, $vertical:expr),* $(,)?) => {$(
        #[test]
        fn $name() {
            two_colour_case($pair, $vertical);
        }
    )*};
}

two_colour_tests! {
    arq_131_two_colours_pair0_horizontal_not_reliable: 0, false,
    arq_131_two_colours_pair0_vertical_not_reliable: 0, true,
    arq_131_two_colours_pair1_horizontal_not_reliable: 1, false,
    arq_131_two_colours_pair1_vertical_not_reliable: 1, true,
}

/// Share of the subject's pixels kept in Primer plano.
fn kept(w: u32, h: u32, data: &[u8], inside: impl Fn(u32, u32) -> bool) -> f64 {
    let out = run(w, h, data, SegZone::Foreground);
    let (mut hit, mut all) = (0.0, 0.0);
    for (i, a) in out.mask.data().iter().enumerate() {
        if inside(i as u32 % w, i as u32 / w) {
            all += 1.0;
            hit += f64::from(u8::from(*a >= 128));
        }
    }
    hit / all
}

fn touch_case(name: &str, inside: impl Fn(u32, u32) -> bool, want_reliable: bool) {
    let before = TOUCH_BEFORE.iter().find(|r| r.0 == name).unwrap();
    let data = touching(&inside);
    let got = check(name, (1200, 900), &data, (before.1, before.2));
    assert_eq!(got, [want_reliable; 3], "{name}: reliable in fg, el, bg");
}

// ARQ-133, ARQ-131, D-47 (AC2, AC4): the four subjects touching the border of
// ARQ-133, and a disc with a contact of about 5.6 % of the band, are not
// reliable and keep their inner part; the mask is the one before.
#[test]
fn arq_133_a_wide_contact_with_the_border_is_not_reliable() {
    touch_case("touch_left", disk(0.0, 450.0, 300.0), false);
    touch_case("touch_corner", disk(1200.0, 900.0, 480.0), false);
    touch_case("touch_top", disk(600.0, 0.0, 240.0), false);
    touch_case("touch_side", |_, y| y >= 620, false);
    let inside = disk(290.0, 450.0, 300.0);
    touch_case("touch_wide_edge", &inside, false);
    let data = touching(&inside);
    assert!(kept(1200, 900, &data, &inside) > 0.75, "inner part kept");
}

// ARQ-133, ARQ-131, D-47 (AC2, AC4): the same disc moved 5 px outward touches
// about 3.9 % of the band: `reliable` as before, and the mask too.
#[test]
fn arq_133_a_narrow_contact_with_the_border_stays_reliable() {
    touch_case("touch_narrow_edge", disk(295.0, 450.0, 300.0), true);
}
