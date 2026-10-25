//! Public-API tests for [`Timecode`].

use gafira_time::{Flicks, FrameRate, TimeError, Timecode, TimecodeFormat, TimecodeMode};
use proptest::prelude::*;

const D24: i64 = 29_400_000;
const D25: i64 = 28_224_000;
const D29: i64 = 23_543_520;
const D5994: i64 = 11_771_760;

fn ndf(rate: FrameRate) -> TimecodeFormat {
    TimecodeFormat::new(rate, TimecodeMode::NonDrop).unwrap()
}

fn df(rate: FrameRate) -> TimecodeFormat {
    TimecodeFormat::new(rate, TimecodeMode::DropFrame).unwrap()
}

fn tc(label: i64, format: TimecodeFormat) -> Timecode {
    Timecode::new(label, format).unwrap()
}

const CANONICAL_FORMATS: usize = 6;

fn canonical_format(index: usize) -> TimecodeFormat {
    match index {
        0 => ndf(FrameRate::FPS_24),
        1 => ndf(FrameRate::FPS_120),
        2 => ndf(FrameRate::FPS_29_97),
        3 => ndf(FrameRate::FPS_25),
        4 => df(FrameRate::FPS_29_97),
        _ => df(FrameRate::FPS_59_94),
    }
}

/// Applies one edit: 0 extra leading zeros in a field, 1 toggle `-`, 2 insert,
/// 3 delete or 4 replace one character.
fn derive(text: &str, edit: usize, position: usize, zeros: usize, symbol: char) -> String {
    let mut chars: Vec<char> = text.chars().collect();
    match edit {
        0 => {
            let mut starts = vec![usize::from(chars[0] == '-')];
            for (index, c) in chars.iter().enumerate() {
                if matches!(c, ':' | ';') {
                    starts.push(index + 1);
                }
            }
            let at = starts[position % starts.len()];
            for _ in 0..zeros {
                chars.insert(at, '0');
            }
        }
        1 => {
            if chars[0] == '-' {
                chars.remove(0);
            } else {
                chars.insert(0, '-');
            }
        }
        2 => chars.insert(position % (chars.len() + 1), symbol),
        3 => {
            chars.remove(position % chars.len());
        }
        _ => {
            let at = position % chars.len();
            chars[at] = symbol;
        }
    }
    chars.into_iter().collect()
}

#[test]
fn non_drop_formatting_uses_hh_mm_ss_ff() {
    assert_eq!(tc(90_000, ndf(FrameRate::FPS_25)).format(), "01:00:00:00");
    assert_eq!(tc(0, ndf(FrameRate::FPS_24)).format(), "00:00:00:00");
    assert_eq!(tc(1, ndf(FrameRate::FPS_24)).format(), "00:00:00:01");
}

#[test]
fn non_drop_parsing_accepts_the_shape_and_sign() {
    let format = ndf(FrameRate::FPS_25);
    let parsed = Timecode::parse("01:00:00:00", format).unwrap();
    assert_eq!(parsed.label_frames(), 90_000);
    assert_eq!(parsed.to_flicks().unwrap().ticks(), 90_000 * D25);
    assert_eq!(parsed.format_spec(), format);
    assert_eq!(
        Timecode::parse("-00:00:00:01", ndf(FrameRate::FPS_24))
            .unwrap()
            .label_frames(),
        -1
    );
}

#[test]
fn negative_timecode_formats_with_a_leading_minus() {
    let format = ndf(FrameRate::FPS_24);
    let tc = Timecode::from_flicks(Flicks::from_ticks(-D24), format);
    assert_eq!(tc.format(), "-00:00:00:01");
    let parsed = Timecode::parse("-00:00:00:01", format).unwrap();
    assert_eq!(parsed.to_flicks().unwrap().ticks(), -D24);
}

#[test]
fn hours_are_a_running_count_and_never_wrap() {
    let format = ndf(FrameRate::FPS_24);
    let label = 25 * 3_600 * 24;
    let tc = tc(label, format);
    assert_eq!(tc.format(), "25:00:00:00");
    assert_eq!(Timecode::parse("25:00:00:00", format).unwrap(), tc);
    assert_eq!(tc.to_flicks().unwrap().ticks(), label * D24);
}

#[test]
fn non_drop_29_97_drift_is_explicit() {
    let parsed = Timecode::parse("01:00:00:00", ndf(FrameRate::FPS_29_97)).unwrap();
    assert_eq!(parsed.to_flicks().unwrap().ticks(), 2_542_700_160_000);
}

#[test]
fn frames_beyond_nine_hundred_are_three_digits() {
    let format = ndf(FrameRate::FPS_120);
    assert_eq!(tc(100, format).format(), "00:00:00:100");
    assert_eq!(
        Timecode::parse("00:00:00:100", format)
            .unwrap()
            .label_frames(),
        100
    );
}

#[test]
fn malformed_and_out_of_range_fields_are_rejected() {
    let format = ndf(FrameRate::FPS_24);
    for text in [
        "00:60:00:00",
        "00:00:60:00",
        "00:00:00:24",
        "00:00:00",
        "00:00:00:00:00",
        "aa:00:00:00",
        "1:00:00:00",
        "00:0:00:00",
        "00:00:0:00",
        "00:00:00:0",
        "",
        "-",
        "+00:00:00:00",
        "00:00:00:-1",
    ] {
        assert!(Timecode::parse(text, format).is_err(), "accepted {text:?}");
    }
}

#[test]
fn the_frames_separator_is_part_of_the_mode() {
    let non_drop = ndf(FrameRate::FPS_24);
    let drop = df(FrameRate::FPS_29_97);
    assert_eq!(
        Timecode::parse("00:00:00;00", non_drop),
        Err(TimeError::WrongTimecodeSeparator)
    );
    assert_eq!(
        Timecode::parse("00:00:00:00", drop),
        Err(TimeError::WrongTimecodeSeparator)
    );
    assert_eq!(tc(0, non_drop).format(), "00:00:00:00");
    assert_eq!(tc(0, drop).format(), "00:00:00;00");
}

#[test]
fn drop_frame_is_rejected_for_unsupported_rates() {
    for rate in [
        FrameRate::FPS_23_976,
        FrameRate::FPS_24,
        FrameRate::FPS_25,
        FrameRate::FPS_30,
        FrameRate::FPS_60,
        FrameRate::FPS_120,
    ] {
        assert!(
            matches!(
                TimecodeFormat::new(rate, TimecodeMode::DropFrame),
                Err(TimeError::DropFrameUnsupported { .. })
            ),
            "accepted drop-frame at {rate:?}"
        );
    }
    assert!(TimecodeFormat::new(FrameRate::FPS_29_97, TimecodeMode::DropFrame).is_ok());
    assert!(TimecodeFormat::new(FrameRate::FPS_59_94, TimecodeMode::DropFrame).is_ok());
}

#[test]
fn drop_frame_reference_labels_at_29_97() {
    let format = df(FrameRate::FPS_29_97);
    assert_eq!(
        Timecode::from_flicks(Flicks::ZERO, format).format(),
        "00:00:00;00"
    );
    assert_eq!(
        Timecode::from_flicks(Flicks::from_ticks(1_800 * D29), format).format(),
        "00:01:00;02"
    );
    assert_eq!(
        Timecode::from_flicks(Flicks::from_ticks(17_982 * D29), format).format(),
        "00:10:00;00"
    );
    assert_eq!(
        Timecode::from_flicks(Flicks::from_ticks(107_892 * D29), format).format(),
        "01:00:00;00"
    );
}

#[test]
fn drop_frame_reference_times_at_29_97() {
    let format = df(FrameRate::FPS_29_97);
    assert_eq!(
        Timecode::parse("00:01:00;02", format)
            .unwrap()
            .to_flicks()
            .unwrap()
            .ticks(),
        42_378_336_000
    );
    assert_eq!(
        Timecode::parse("00:10:00;00", format)
            .unwrap()
            .to_flicks()
            .unwrap()
            .ticks(),
        423_359_576_640
    );
    assert_eq!(
        Timecode::parse("01:00:00;00", format)
            .unwrap()
            .to_flicks()
            .unwrap()
            .ticks(),
        2_540_157_459_840
    );
}

#[test]
fn drop_frame_reference_at_59_94() {
    let format = df(FrameRate::FPS_59_94);
    assert_eq!(
        Timecode::from_flicks(Flicks::from_ticks(3_600 * D5994), format).format(),
        "00:01:00;04"
    );
    assert_eq!(
        Timecode::parse("00:01:00;04", format)
            .unwrap()
            .to_flicks()
            .unwrap()
            .ticks(),
        42_378_336_000
    );
}

#[test]
fn skipped_drop_frame_labels_are_rejected() {
    let format = df(FrameRate::FPS_29_97);
    for text in ["00:01:00;00", "00:01:00;01", "00:09:00;01"] {
        assert!(
            Timecode::parse(text, format).is_err(),
            "accepted skipped {text}"
        );
    }
    assert_eq!(
        Timecode::new(1_800, format),
        Err(TimeError::DroppedFrameLabel)
    );
    assert_eq!(
        Timecode::new(1_801, format),
        Err(TimeError::DroppedFrameLabel)
    );
    assert!(Timecode::new(1_802, format).is_ok());

    let format = df(FrameRate::FPS_59_94);
    assert!(Timecode::parse("00:01:00;03", format).is_err());
    assert_eq!(
        Timecode::new(3_600, format),
        Err(TimeError::DroppedFrameLabel)
    );
    assert!(Timecode::new(3_604, format).is_ok());
}

#[test]
fn tenth_minutes_do_not_skip() {
    let format = df(FrameRate::FPS_29_97);
    for text in ["00:10:00;00", "00:20:00;00", "00:30:00;00"] {
        assert!(
            Timecode::parse(text, format).is_ok(),
            "rejected tenth minute {text}"
        );
    }
    assert_eq!(
        Timecode::parse("00:10:00;00", format)
            .unwrap()
            .to_flicks()
            .unwrap()
            .ticks(),
        17_982 * D29
    );
}

#[test]
fn negative_drop_frame_timecodes_round_trip() {
    let format = df(FrameRate::FPS_29_97);
    let tc = Timecode::from_flicks(Flicks::from_ticks(-1_800 * D29), format);
    assert_eq!(tc.format(), "-00:01:00;02");
    assert_eq!(tc.to_flicks().unwrap().ticks(), -42_378_336_000);
    assert_eq!(Timecode::parse("-00:01:00;02", format).unwrap(), tc);
}

#[test]
fn from_flicks_floors_to_the_containing_frame() {
    let format = ndf(FrameRate::FPS_25);
    assert_eq!(
        Timecode::from_flicks(Flicks::from_ticks(D25 - 1), format).label_frames(),
        0
    );
    assert_eq!(
        Timecode::from_flicks(Flicks::from_ticks(D25), format).label_frames(),
        1
    );
    assert_eq!(
        Timecode::from_flicks(Flicks::from_ticks(-1), format).label_frames(),
        -1
    );
    assert_eq!(
        Timecode::from_flicks(Flicks::from_ticks(-D25), format).label_frames(),
        -1
    );
    assert_eq!(
        Timecode::from_flicks(Flicks::from_ticks(-D25 - 1), format).label_frames(),
        -2
    );
}

#[test]
fn to_flicks_overflow_is_an_error() {
    let tc = Timecode::new(i64::MAX, ndf(FrameRate::FPS_24)).unwrap();
    assert_eq!(tc.to_flicks(), Err(TimeError::Overflow));
}

#[test]
fn parsing_overflow_is_an_error() {
    let format = ndf(FrameRate::FPS_24);
    for text in [
        "18446744073709551615:00:00:00",
        "99999999999999999999:00:00:00",
    ] {
        assert_eq!(
            Timecode::parse(text, format),
            Err(TimeError::Overflow),
            "accepted {text}"
        );
    }
}

#[test]
fn exhaustive_drop_frame_round_trip_over_twenty_blocks() {
    for (rate, frames_per_block) in [
        (FrameRate::FPS_29_97, 17_982i64),
        (FrameRate::FPS_59_94, 35_964i64),
    ] {
        let format = df(rate);
        let duration = rate.flicks_per_frame().ticks();
        let blocks = 20;
        for real in 0..frames_per_block * blocks {
            let ticks = real * duration;
            let tc = Timecode::from_flicks(Flicks::from_ticks(ticks), format);
            assert_eq!(
                tc.to_flicks().unwrap().ticks(),
                ticks,
                "real frame {real} at {rate:?}"
            );
            assert!(
                Timecode::new(tc.label_frames(), format).is_ok(),
                "produced a skipped label {} at {rate:?}",
                tc.label_frames()
            );
        }

        let nominal = i64::from(rate.nominal_fps());
        for label in 0..nominal * 600 * blocks {
            if let Ok(tc) = Timecode::new(label, format) {
                let flicks = tc.to_flicks().unwrap();
                assert_eq!(
                    Timecode::from_flicks(flicks, format).label_frames(),
                    label,
                    "label {label} at {rate:?}"
                );
            }
        }
    }
}

#[test]
fn nominal_rates_round_trip_in_non_drop_mode() {
    for rate in [
        FrameRate::FPS_23_976,
        FrameRate::FPS_24,
        FrameRate::FPS_25,
        FrameRate::FPS_29_97,
        FrameRate::FPS_30,
        FrameRate::FPS_48,
        FrameRate::FPS_50,
        FrameRate::FPS_59_94,
        FrameRate::FPS_60,
        FrameRate::FPS_120,
    ] {
        let format = ndf(rate);
        let label = 123_456;
        let tc = tc(label, format);
        let text = tc.format();
        let parsed = Timecode::parse(&text, format).unwrap();
        assert_eq!(parsed.label_frames(), label, "{rate:?}");
        assert_eq!(parsed.to_flicks(), tc.to_flicks(), "{rate:?}");
    }
}

#[test]
fn frames_field_width_at_120_fps() {
    let format = ndf(FrameRate::FPS_120);
    for (label, text) in [
        (5, "00:00:00:05"),
        (99, "00:00:00:99"),
        (100, "00:00:00:100"),
        (119, "00:00:00:119"),
    ] {
        assert_eq!(tc(label, format).format(), text);
        assert_eq!(
            Timecode::parse(text, format).unwrap().label_frames(),
            label,
            "{text}"
        );
    }
}

#[test]
fn hours_of_one_hundred_or_more_have_no_padding() {
    let format = ndf(FrameRate::FPS_24);
    let label = 100 * 3_600 * 24;
    assert_eq!(tc(label, format).format(), "100:00:00:00");
    assert_eq!(
        Timecode::parse("100:00:00:00", format)
            .unwrap()
            .label_frames(),
        label
    );
}

#[test]
fn extra_leading_zeros_are_rejected() {
    let cases = [
        ("00:00:00:005", ndf(FrameRate::FPS_120)),
        ("00:00:00:005", ndf(FrameRate::FPS_24)),
        ("00:00:00:0023", ndf(FrameRate::FPS_24)),
        ("000:00:00:00", ndf(FrameRate::FPS_24)),
        ("001:00:00:00", ndf(FrameRate::FPS_24)),
        ("00:00:00;005", df(FrameRate::FPS_29_97)),
        ("0:00:00:00", ndf(FrameRate::FPS_24)),
        ("00:0:00:00", ndf(FrameRate::FPS_24)),
        ("00:00:000:00", ndf(FrameRate::FPS_24)),
    ];
    for (text, format) in cases {
        assert!(Timecode::parse(text, format).is_err(), "accepted {text:?}");
    }
}

#[test]
fn negative_zero_is_rejected() {
    assert!(Timecode::parse("-00:00:00:00", ndf(FrameRate::FPS_24)).is_err());
    assert!(Timecode::parse("-00:00:00;00", df(FrameRate::FPS_29_97)).is_err());
    assert_eq!(
        Timecode::parse("-00:00:00:01", ndf(FrameRate::FPS_24))
            .unwrap()
            .label_frames(),
        -1
    );
}

fn fixed_rates() -> Vec<FrameRate> {
    vec![
        FrameRate::FPS_23_976,
        FrameRate::FPS_24,
        FrameRate::FPS_25,
        FrameRate::FPS_29_97,
        FrameRate::FPS_30,
        FrameRate::FPS_59_94,
        FrameRate::FPS_120,
        FrameRate::new(245, 3).unwrap(),
    ]
}

#[test]
fn non_drop_labels_round_trip_at_every_rate() {
    for rate in fixed_rates() {
        let nominal = i64::from(rate.nominal_fps());
        let format = ndf(rate);
        // (hours, seconds into the hour, frames) of the fixed labels.
        for (hours, seconds, frames) in [
            (0, 0, 0),
            (0, 0, nominal - 1),
            (0, 59, nominal - 1),
            (1, 0, 0),
            (25, 0, 0),
        ] {
            let label = (hours * 3_600 + seconds) * nominal + frames;
            let expected = format!(
                "{hours:02}:{:02}:{:02}:{frames:02}",
                seconds / 60,
                seconds % 60
            );
            for signed in [label, -label] {
                let timecode = tc(signed, format);
                let sign = if signed < 0 { "-" } else { "" };
                assert_eq!(timecode.format(), format!("{sign}{expected}"), "{rate:?}");
                let parsed = Timecode::parse(&timecode.format(), format).unwrap();
                assert_eq!(parsed, timecode, "{rate:?} {signed}");
            }
        }
    }
}

#[test]
fn a_frames_field_equal_to_the_nominal_rate_is_rejected() {
    let r24_5 = FrameRate::new(49, 2).unwrap();
    // (format, last valid text, first rejected text).
    let cases = [
        (ndf(FrameRate::FPS_23_976), "00:00:00:23", "00:00:00:24"),
        (ndf(FrameRate::FPS_24), "00:00:00:23", "00:00:00:24"),
        (ndf(FrameRate::FPS_29_97), "00:00:00:29", "00:00:00:30"),
        (df(FrameRate::FPS_29_97), "00:00:00;29", "00:00:00;30"),
        (ndf(FrameRate::FPS_59_94), "00:00:00:59", "00:00:00:60"),
        (df(FrameRate::FPS_59_94), "00:00:00;59", "00:00:00;60"),
        (ndf(r24_5), "00:00:00:24", "00:00:00:25"),
        (ndf(FrameRate::FPS_120), "00:00:00:119", "00:00:00:120"),
    ];
    for (format, accepted, rejected) in cases {
        assert!(
            Timecode::parse(accepted, format).is_ok(),
            "rejected {accepted}"
        );
        assert_eq!(
            Timecode::parse(rejected, format),
            Err(TimeError::InvalidTimecodeField),
            "accepted {rejected}"
        );
    }
}

#[test]
fn the_label_after_the_last_frame_rolls_into_the_next_second() {
    for (format, expected) in [
        (ndf(FrameRate::FPS_23_976), "00:00:01:00"),
        (ndf(FrameRate::FPS_24), "00:00:01:00"),
        (ndf(FrameRate::FPS_29_97), "00:00:01:00"),
        (ndf(FrameRate::FPS_59_94), "00:00:01:00"),
        (ndf(FrameRate::new(49, 2).unwrap()), "00:00:01:00"),
    ] {
        let nominal = i64::from(format.rate().nominal_fps());
        assert_eq!(tc(nominal, format).format(), expected);
    }
}

#[test]
fn the_nominal_rate_bounds_the_last_label_of_a_minute() {
    assert!(Timecode::parse("00:00:59;59", df(FrameRate::FPS_59_94)).is_ok());
    assert!(Timecode::parse("00:00:59:29", ndf(FrameRate::FPS_29_97)).is_ok());
}

proptest! {
    #[test]
    fn non_drop_parse_round_trip(label in -1_000_000_000i64..=1_000_000_000) {
        let format = ndf(FrameRate::FPS_25);
        let tc = tc(label, format);
        let text = tc.format();
        let parsed = Timecode::parse(&text, format).unwrap();
        prop_assert_eq!(parsed, tc);
    }

    #[test]
    fn non_drop_from_flicks_floors(ticks in any::<i64>()) {
        let format = ndf(FrameRate::FPS_24);
        let tc = Timecode::from_flicks(Flicks::from_ticks(ticks), format);
        prop_assert_eq!(tc.label_frames(), ticks.div_euclid(D24));
    }

    #[test]
    fn drop_frame_from_flicks_round_trip(real in 0i64..359_640) {
        let format = df(FrameRate::FPS_29_97);
        let ticks = real * D29;
        let tc = Timecode::from_flicks(Flicks::from_ticks(ticks), format);
        prop_assert_eq!(tc.to_flicks().unwrap().ticks(), ticks);
        prop_assert_eq!(Timecode::parse(&tc.format(), format).unwrap(), tc);
    }

    #[test]
    fn drop_frame_from_flicks_round_trip_5994(real in 0i64..719_280) {
        let format = df(FrameRate::FPS_59_94);
        let ticks = real * D5994;
        let tc = Timecode::from_flicks(Flicks::from_ticks(ticks), format);
        prop_assert_eq!(tc.to_flicks().unwrap().ticks(), ticks);
        prop_assert_eq!(Timecode::parse(&tc.format(), format).unwrap(), tc);
    }

    #[test]
    fn accepted_text_is_canonical(
        real in -2_000_000_000i64..=2_000_000_000,
        format_index in 0usize..CANONICAL_FORMATS,
        edit in 0usize..5,
        position in any::<usize>(),
        zeros in 1usize..4,
        symbol in prop::sample::select(vec!['0', '1', '5', '9', ':', ';', '-']),
    ) {
        let format = canonical_format(format_index);
        // A real frame always has a valid label, even in drop-frame mode.
        let original = Timecode::from_flicks(
            Flicks::from_frames(real, format.rate()).unwrap(),
            format,
        );
        let text = original.format();
        let derived = derive(&text, edit, position, zeros, symbol);
        match Timecode::parse(&derived, format) {
            Err(_) => {}
            Ok(parsed) => {
                prop_assert_eq!(parsed.format(), derived.clone());
            }
        }
        if edit == 0 {
            prop_assert!(
                Timecode::parse(&derived, format).is_err(),
                "extra zeros accepted: {derived:?}"
            );
        }
        if edit == 1 && original.label_frames() == 0 {
            prop_assert!(Timecode::parse(&derived, format).is_err());
        }
    }
}
