//! ARQ-113 / ADR 0010: quantization and the failing-golden path.

use std::path::{Component, Path, PathBuf};

use gafira_testkit::golden::{
    Channels, CheckError, Image, Mismatch, Mode, check, check_in, decode_png, encode_png, quantize,
};

fn tmp() -> PathBuf {
    PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
}

fn gray(bytes: &[u8]) -> Image {
    Image::new(bytes.len() as u32, 1, Channels::Gray, bytes).unwrap()
}

#[test]
fn ac6_quantize_rounds_to_nearest() {
    // Every byte value maps back to itself, so a scale other than 255 shows.
    let all: Vec<u8> = (0..=255).collect();
    let floats: Vec<f32> = all.iter().map(|&b| f32::from(b) / 255.0).collect();
    assert_eq!(quantize(&floats), all);
    // c = 0.5/255 + epsilon rounds up; just below it rounds down.
    assert_eq!(
        quantize(&[0.0, 1.0, 0.5, 1.0 / 255.0]),
        vec![0, 255, 128, 1]
    );
    assert_eq!(
        quantize(&[0.51 / 255.0, 0.49 / 255.0, 1.51 / 255.0, 254.6 / 255.0]),
        vec![1, 0, 2, 255]
    );
}

#[test]
fn ac6_quantize_bounds_to_zero_one() {
    assert_eq!(
        quantize(&[-1.0, -0.001, 1.001, 2.0, f32::INFINITY, f32::NEG_INFINITY]),
        vec![0, 0, 255, 255, 255, 0]
    );
}

fn has_component(p: &Path, parts: [&str; 2]) -> bool {
    let c: Vec<_> = p
        .components()
        .filter_map(|c| match c {
            Component::Normal(s) => s.to_str(),
            _ => None,
        })
        .collect();
    c.windows(2).any(|w| w == parts)
}

#[test]
fn ac4_a_failing_golden_writes_the_candidate_under_the_target_tmpdir() {
    let golden = encode_png(&gray(&[1, 2, 3, 4])).unwrap();
    let actual = gray(&[1, 2, 3, 9]);
    let dir = tmp();
    let _ = std::fs::remove_file(dir.join("ac4_fail.png"));
    let err = check_in(Mode::Exact, "ac4_fail", &golden, &actual, &dir).unwrap_err();
    let CheckError::Mismatch {
        mismatch,
        candidate,
    } = err
    else {
        panic!("expected a mismatch")
    };
    assert_eq!(
        mismatch,
        Mismatch::Pixels {
            differing: 1,
            allowed: 0
        }
    );
    assert!(candidate.starts_with(&dir), "{candidate:?}");
    assert!(!has_component(&candidate, ["tests", "golden"]));
    assert_eq!(
        decode_png(&std::fs::read(&candidate).unwrap()).unwrap(),
        actual
    );
    // Nothing is written next to the stored goldens.
    assert!(
        !Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/golden")
            .exists()
    );
}

#[test]
fn ac4_a_matching_golden_writes_nothing() {
    let img = gray(&[5, 6, 7]);
    let golden = encode_png(&img).unwrap();
    let dir = tmp();
    let _ = std::fs::remove_file(dir.join("ac4_ok.png"));
    check_in(Mode::Exact, "ac4_ok", &golden, &img, &dir).unwrap();
    assert!(!dir.join("ac4_ok.png").exists());
}

#[test]
fn arq_113_a_tolerated_difference_passes_only_in_cross_platform_mode() {
    let golden = encode_png(&gray(&[10, 10])).unwrap();
    let actual = gray(&[11, 10]);
    let dir = tmp();
    assert!(check_in(Mode::CrossPlatform, "ac4_tol", &golden, &actual, &dir).is_ok());
    assert!(matches!(
        check_in(Mode::Exact, "ac4_tol", &golden, &actual, &dir),
        Err(CheckError::Mismatch { .. })
    ));
}

#[test]
fn ac4_a_failing_candidate_write_is_still_a_failure() {
    let golden = encode_png(&gray(&[1])).unwrap();
    let missing = tmp().join("no-such-dir").join("deeper");
    let r = check_in(Mode::Exact, "ac4_io", &golden, &gray(&[2]), &missing);
    assert!(matches!(r, Err(CheckError::Io(_))));
}

#[test]
fn ac4_an_unreadable_golden_is_an_error_not_a_pass() {
    let r = check_in(
        Mode::CrossPlatform,
        "ac4_bad",
        b"not a png",
        &gray(&[1]),
        &tmp(),
    );
    assert!(matches!(r, Err(CheckError::Golden(_))));
}

#[test]
fn ac3_check_uses_the_mode_of_the_build_target() {
    // `check` is the entry for tests: exact where the target is exact, tolerant elsewhere.
    let golden = encode_png(&gray(&[10, 10])).unwrap();
    let off_by_one = gray(&[11, 10]);
    let r = check("ac3_target_mode", &golden, &off_by_one, &tmp());
    if cfg!(all(
        target_arch = "x86_64",
        target_os = "linux",
        target_env = "gnu"
    )) {
        assert!(matches!(r, Err(CheckError::Mismatch { .. })));
    } else {
        assert!(r.is_ok());
    }
}

#[test]
fn ac4_check_rejects_a_different_image_and_accepts_the_same_one() {
    let golden = encode_png(&gray(&[10, 10])).unwrap();
    assert!(check("ac4_check_same", &golden, &gray(&[10, 10]), &tmp()).is_ok());
    assert!(check("ac4_check_diff", &golden, &gray(&[10, 50]), &tmp()).is_err());
}

#[test]
fn ac4_a_size_or_channel_mismatch_still_writes_a_candidate() {
    let golden = encode_png(&gray(&[1, 2, 3, 4])).unwrap();
    let rgba = Image::new(1, 1, Channels::Rgba, &[1, 2, 3, 4]).unwrap();
    for (name, actual, expected) in [
        ("ac4_size", gray(&[1, 2, 3]), Mismatch::Size),
        ("ac4_chan", rgba, Mismatch::Size),
    ] {
        let dir = tmp();
        let _ = std::fs::remove_file(dir.join(format!("{name}.png")));
        let err = check_in(Mode::Exact, name, &golden, &actual, &dir).unwrap_err();
        let CheckError::Mismatch {
            mismatch,
            candidate,
        } = err
        else {
            panic!("expected a mismatch")
        };
        assert_eq!(mismatch, expected);
        assert_eq!(
            decode_png(&std::fs::read(&candidate).unwrap()).unwrap(),
            actual
        );
    }
    // Same dimensions, other channel layout.
    let golden = encode_png(&Image::new(1, 1, Channels::Gray, &[1]).unwrap()).unwrap();
    let actual = Image::new(1, 1, Channels::Rgba, &[1, 1, 1, 1]).unwrap();
    let err = check_in(Mode::Exact, "ac4_layout", &golden, &actual, &tmp()).unwrap_err();
    assert!(matches!(
        err,
        CheckError::Mismatch {
            mismatch: Mismatch::Channels,
            ..
        }
    ));
}
