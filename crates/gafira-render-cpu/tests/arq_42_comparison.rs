//! ARQ-42, ARQ-113: the preview and the export of one page agree in the
//! comparison space of 02 §10 — 8-bit sRGB **premultiplied** — and the export
//! is encoded with ARQ-117's rules before the two buffers are compared (D9).
//!
//! This file is the deliverable of `CB3`: the comparator is its critical code,
//! so it lives where a reader can check it and not behind the crate's own
//! helpers. The preview's bytes come from the backend's `Rgba8` read-back; the
//! export's `RgbaF32` surface is converted here, once, by the test. Quantising
//! the preview's `f32` surface with the same function the export's pixels go
//! through would be one code path grading itself, which is the fake this
//! delivery exists to prevent.

use gafira_color::LinearRgba;

/// One 8-bit premultiplied sample as `(c × a + 127) / 255`, integer division
/// (ARQ-117). The comparator carries its own copy on purpose: a hand-computed
/// byte has to be able to contradict it.
fn premultiply8(c: u8, a: u8) -> u8 {
    let value = (u32::from(c) * u32::from(a) + 127) / 255;
    u8::try_from(value).unwrap_or(u8::MAX)
}

/// A channel of `[0, 1]` as 8 bits, `⌊v × 255 + 0,5⌋` clamped to `[0, 255]`
/// (ARQ-118's rounding, which ARQ-117's encode step shares).
fn quantise8(value: f32) -> u8 {
    let scaled = (value.clamp(0.0, 1.0) * 255.0 + 0.5).floor();
    u8::try_from(scaled.clamp(0.0, 255.0) as u32).unwrap_or(u8::MAX)
}

/// One `RgbaF32` export pixel as premultiplied sRGB 8-bit, in the order ARQ-117
/// fixes and `D9` restates for the comparator: unpremultiply in linear light,
/// encode with the sRGB curve, quantise, premultiply with `(c × a + 127) / 255`.
///
/// `a = 0` answers four zeroes and never divides by the alpha (ARQ-117,
/// FD-55 (b)). The alpha is quantised first, so an alpha that rounds to zero
/// takes the same branch the preview's own read-back takes.
fn encode_export_pixel(pixel: [f32; 4]) -> [u8; 4] {
    let alpha = pixel[3].clamp(0.0, 1.0);
    let a = quantise8(alpha);
    if a == 0 {
        // ARQ-117: with a = 0 the answer is (0, 0, 0, 0), and the alpha is
        // never a divisor.
        return [0, 0, 0, 0];
    }
    let straight = [pixel[0] / alpha, pixel[1] / alpha, pixel[2] / alpha];
    let encoded = LinearRgba::new(straight[0], straight[1], straight[2], alpha).to_srgb();
    [
        premultiply8(quantise8(encoded.r), a),
        premultiply8(quantise8(encoded.g), a),
        premultiply8(quantise8(encoded.b), a),
        a,
    ]
}

/// How strict a comparison is (ARQ-113).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mode {
    /// Every byte equal: a stored golden on the platform that generated it.
    Exact,
    /// ARQ-42's budget: at most `pixels / 1000` pixels over 1/255 on a channel.
    Tolerant,
}

/// The target a stored golden is generated on (ARQ-113).
fn mode_for(arch: &str, os: &str, env: &str) -> Mode {
    if (arch, os, env) == ("x86_64", "linux", "gnu") {
        Mode::Exact
    } else {
        Mode::Tolerant
    }
}

/// One channel of an sRGB8 pixel.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Channel {
    Red,
    Green,
    Blue,
    Alpha,
}

impl Channel {
    const fn at(index: usize) -> Self {
        match index {
            0 => Self::Red,
            1 => Self::Green,
            2 => Self::Blue,
            _ => Self::Alpha,
        }
    }
}

/// What a comparison found, whether it passed or not (AC6 asks the failure
/// report to name the worst channel and how many pixels are over 1/255).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Report {
    /// Pixels in each buffer.
    pixels: usize,
    /// Pixels outside the tolerance of the mode.
    differing: usize,
    /// How many of them the mode allows.
    allowed: usize,
    /// The channel with the largest difference anywhere in the image.
    worst: Channel,
    /// That difference, in 1/255.
    max_delta: u8,
}

impl Report {
    /// True when the comparison meets the mode's rule.
    const fn within_budget(self) -> bool {
        self.differing <= self.allowed
    }
}

/// Compares two premultiplied sRGB8 buffers of the same region.
///
/// The tolerance of §10 is a per-channel difference of more than 1/255, and
/// ARQ-42 lets one pixel in a thousand be over it; `Mode::Exact` lets none be
/// over a difference of zero. Both sides keep the bytes the renderers
/// produced, so the count is of *pixels*, never of channels.
fn compare(mode: Mode, reference: &[u8], actual: &[u8]) -> Report {
    assert_eq!(
        reference.len(),
        actual.len(),
        "both sides cover the same region of the same image"
    );
    assert_eq!(
        reference.len() % 4,
        0,
        "an sRGB8 premultiplied buffer is four bytes per pixel"
    );
    let pixels = reference.len() / 4;
    let (mut differing, mut worst, mut max_delta) = (0usize, Channel::Red, 0u8);
    for (expected, got) in reference.chunks_exact(4).zip(actual.chunks_exact(4)) {
        let mut over = false;
        for (index, (&expected, &got)) in expected.iter().zip(got).enumerate() {
            let delta = expected.abs_diff(got);
            if delta > max_delta {
                max_delta = delta;
                worst = Channel::at(index);
            }
            let over_here = match mode {
                Mode::Exact => delta != 0,
                Mode::Tolerant => delta > 1,
            };
            over |= over_here;
        }
        if over {
            differing += 1;
        }
    }
    let allowed = match mode {
        Mode::Exact => 0,
        // ARQ-42: at most one pixel in a thousand may be over 1/255.
        Mode::Tolerant => pixels / 1000,
    };
    Report {
        pixels,
        differing,
        allowed,
        worst,
        max_delta,
    }
}

/// [`compare`], and a panic that names the worst channel and the count when the
/// comparison is outside the mode's budget (AC6).
fn assert_agrees(mode: Mode, label: &str, reference: &[u8], actual: &[u8]) -> Report {
    let report = compare(mode, reference, actual);
    assert!(
        report.within_budget(),
        "{label}: {} of {} pixels differ by 2/255 or more (allowed {}); worst channel {:?} at {}/255",
        report.differing,
        report.pixels,
        report.allowed,
        report.worst,
        report.max_delta
    );
    report
}

/// One sRGB8 buffer of `pixels` copies of `pixel`.
fn buffer(pixel: [u8; 4], pixels: usize) -> Vec<u8> {
    pixel.iter().copied().cycle().take(pixels * 4).collect()
}

// ARQ-117, SP6 (02 §10): the comparator encodes the export's `RgbaF32` pixel in
// the order the comparison space fixes, and a hand-computed pixel matches byte
// for byte. The third row is where truncating instead of rounding shows.
#[test]
fn arq_117_the_export_pixel_is_encoded_in_the_order_of_the_comparison_space() {
    let cases: &[([f32; 4], [u8; 4], &str)] = &[
        ([0.5, 0.25, 0.1, 1.0], [188, 137, 89, 255], "opaque"),
        ([0.25, 0.1, 0.05, 0.5], [94, 62, 45, 128], "half alpha"),
        (
            [0.05, 0.125, 0.0125, 0.25],
            [31, 47, 16, 64],
            "quarter alpha",
        ),
    ];
    for (pixel, expected, label) in cases {
        assert_eq!(
            &encode_export_pixel(*pixel),
            expected,
            "{label}: linear premultiplied {pixel:?} as premultiplied sRGB8"
        );
    }
}

// ARQ-117, FD-55 (b): a pixel whose alpha is 0 answers four zeroes.
#[test]
fn arq_117_a_zero_alpha_pixel_answers_four_zeroes() {
    assert_eq!(
        encode_export_pixel([0.5, 0.25, 1.0, 0.0]),
        [0, 0, 0, 0],
        "a colour under zero alpha is not divided by it"
    );
    assert_eq!(
        encode_export_pixel([0.0, 0.0, 0.0, 0.0]),
        [0, 0, 0, 0],
        "the empty pixel"
    );
    assert_eq!(
        encode_export_pixel([0.5, 0.25, 1.0, 0.001]),
        [0, 0, 0, 0],
        "an alpha that quantises to 0 takes the same branch"
    );
}

// ARQ-113: the budget on 1 000 pixels — one pixel at 2/255 passes (99,9 %), two
// fail, and every pixel at 1/255 passes.
#[test]
fn arq_113_the_budget_is_one_pixel_in_a_thousand() {
    let reference = buffer([100, 100, 100, 255], 1000);

    let mut within = reference.clone();
    within[0] = 101;
    within[1] = 99;
    within[2] = 101;
    within[3] = 254;
    let report = compare(Mode::Tolerant, &reference, &within);
    assert_eq!(report.differing, 0, "every pixel at 1/255 stays inside");
    assert!(report.within_budget(), "1/255 is the tolerance, not a miss");

    let mut one_over = reference.clone();
    one_over[0] = 102;
    let report = compare(Mode::Tolerant, &reference, &one_over);
    assert_eq!(report.differing, 1, "one pixel at 2/255");
    assert_eq!(report.allowed, 1, "one in a thousand");
    assert!(report.within_budget(), "1 pixel of 1 000 is 99,9 %");

    let mut two_over = one_over;
    two_over[4] = 98;
    let report = compare(Mode::Tolerant, &reference, &two_over);
    assert_eq!(report.differing, 2, "two pixels at 2/255");
    assert!(!report.within_budget(), "2 of 1 000 is 99,8 %");
}

// ARQ-42, AC6: the report names the worst channel and counts the pixels over
// the tolerance, not the channels.
#[test]
fn arq_42_the_report_names_the_worst_channel_and_the_count() {
    let reference = [10u8; 16];
    let mut actual = reference;
    actual[5] = 12;
    actual[10] = 13;
    let report = compare(Mode::Tolerant, &reference, &actual);
    assert_eq!(report.pixels, 4, "four pixels, four channels each");
    assert_eq!(report.differing, 2, "two pixels are over the tolerance");
    assert_eq!(report.worst, Channel::Blue, "3/255 on blue beats 2/255");
    assert_eq!(report.max_delta, 3);
    assert_eq!(report.allowed, 0, "four pixels allow none");
}

#[test]
#[should_panic(expected = "worst channel Blue at 3/255")]
fn arq_42_the_failure_report_quotes_the_worst_channel() {
    let reference = [10u8; 16];
    let mut actual = reference;
    actual[5] = 12;
    actual[10] = 13;
    assert_agrees(Mode::Tolerant, "a synthetic pair", &reference, &actual);
}

// ARQ-113: a golden is exact on the platform that generated it and tolerant
// elsewhere, so the CI matrix does not turn a last-bit difference into a red.
#[test]
fn arq_113_the_golden_is_exact_only_on_the_platform_that_generates_it() {
    assert_eq!(mode_for("x86_64", "linux", "gnu"), Mode::Exact);
    assert_eq!(mode_for("aarch64", "macos", "other"), Mode::Tolerant);
    assert_eq!(mode_for("x86_64", "windows", "msvc"), Mode::Tolerant);
}
