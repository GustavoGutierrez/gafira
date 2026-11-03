//! ARQ-113 / ADR 0010: the comparator and the mode selection.

use gafira_testkit::golden::{Channels, Image, Mismatch, Mode, compare, current_mode, mode_for};

fn gray(w: u32, h: u32, bytes: &[u8]) -> Image {
    Image::new(w, h, Channels::Gray, bytes).unwrap()
}

fn rgba(w: u32, h: u32, bytes: &[u8]) -> Image {
    Image::new(w, h, Channels::Rgba, bytes).unwrap()
}

/// 1000 gray pixels at 100, with the given `(index, delta)` changes.
fn thousand(changes: &[(usize, u8)]) -> Image {
    let mut v = vec![100u8; 1000];
    for &(i, d) in changes {
        v[i] += d;
    }
    gray(40, 25, &v)
}

#[test]
fn arq_113_one_altered_byte_fails_in_exact_mode() {
    let a = thousand(&[]);
    assert!(compare(Mode::Exact, &a, &a).is_ok());
    for i in [0, 500, 999] {
        assert!(
            compare(Mode::Exact, &a, &thousand(&[(i, 1)])).is_err(),
            "byte {i}"
        );
    }
}

#[test]
fn arq_113_cross_platform_thousand_pixels() {
    let base = thousand(&[]);
    // 1 pixel at 2/255 passes (floor(1000/1000) = 1).
    assert!(compare(Mode::CrossPlatform, &base, &thousand(&[(7, 2)])).is_ok());
    // 2 pixels at 2/255 fail.
    assert!(compare(Mode::CrossPlatform, &base, &thousand(&[(7, 2), (8, 2)])).is_err());
    // All pixels at 1/255 pass.
    let all: Vec<_> = (0..1000).map(|i| (i, 1)).collect();
    assert!(compare(Mode::CrossPlatform, &base, &thousand(&all)).is_ok());
}

#[test]
fn ac2_limit_is_floor_of_pixels_over_1000() {
    // (width, height, allowed): 999 -> 0, 1000 -> 1, 1999 -> 1, 2000 -> 2.
    for (w, h, allowed) in [
        (999u32, 1u32, 0usize),
        (1000, 1, 1),
        (1999, 1, 1),
        (2000, 1, 2),
        (1, 1, 0),
    ] {
        let n = (w * h) as usize;
        let base = gray(w, h, &vec![50; n]);
        let with = |k: usize| {
            let mut v = vec![50u8; n];
            v.iter_mut().take(k).for_each(|b| *b = 60);
            gray(w, h, &v)
        };
        assert!(
            compare(Mode::CrossPlatform, &base, &with(allowed)).is_ok(),
            "{n} px, {allowed} differing"
        );
        assert_eq!(
            compare(Mode::CrossPlatform, &base, &with(allowed + 1)),
            Err(Mismatch::Pixels {
                differing: allowed + 1,
                allowed
            }),
            "{n} px"
        );
    }
}

#[test]
fn ac2_exactly_one_step_is_not_over_and_two_is() {
    let base = gray(1, 1, &[10]);
    assert!(compare(Mode::CrossPlatform, &base, &gray(1, 1, &[11])).is_ok());
    assert!(compare(Mode::CrossPlatform, &base, &gray(1, 1, &[9])).is_ok());
    assert!(compare(Mode::CrossPlatform, &base, &gray(1, 1, &[12])).is_err());
    assert!(compare(Mode::CrossPlatform, &base, &gray(1, 1, &[8])).is_err());
}

#[test]
fn ac2_any_channel_over_one_step_counts_the_pixel() {
    // Two RGBA pixels per 2000-pixel image would be allowed 2; use 1 px images with limit 0.
    let base = rgba(1, 1, &[10, 20, 30, 40]);
    for ch in 0..4 {
        let mut v = [10, 20, 30, 40];
        v[ch] += 2;
        assert!(
            compare(Mode::CrossPlatform, &base, &rgba(1, 1, &v)).is_err(),
            "channel {ch}"
        );
    }
}

#[test]
fn ac2_a_pixel_counts_once_however_many_channels_differ() {
    // 1000 RGBA pixels, limit 1: one pixel with all four channels over passes.
    let mut v = vec![0u8; 4000];
    let base = rgba(40, 25, &v);
    v[..4].copy_from_slice(&[9, 9, 9, 9]);
    assert!(compare(Mode::CrossPlatform, &base, &rgba(40, 25, &v)).is_ok());
}

#[test]
fn ac2_exact_mode_sees_every_byte_including_the_last() {
    let base = rgba(2, 2, &[7; 16]);
    for i in 0..16 {
        let mut v = [7u8; 16];
        v[i] ^= 1;
        assert!(
            compare(Mode::Exact, &base, &rgba(2, 2, &v)).is_err(),
            "byte {i}"
        );
    }
}

#[test]
fn ac2_size_and_channel_mismatch_fail_in_both_modes() {
    for mode in [Mode::Exact, Mode::CrossPlatform] {
        assert_eq!(
            compare(mode, &gray(2, 1, &[0; 2]), &gray(1, 2, &[0; 2])),
            Err(Mismatch::Size)
        );
        assert_eq!(
            compare(mode, &gray(2, 2, &[0; 4]), &gray(4, 1, &[0; 4])),
            Err(Mismatch::Size)
        );
        assert_eq!(
            compare(mode, &gray(1, 4, &[0; 4]), &rgba(1, 1, &[0; 4])),
            Err(Mismatch::Size)
        );
        assert_eq!(
            compare(mode, &gray(1, 1, &[0]), &rgba(1, 1, &[0; 4])),
            Err(Mismatch::Channels)
        );
    }
}

#[test]
fn ac3_mode_is_exact_only_on_x86_64_linux_gnu() {
    assert_eq!(mode_for("x86_64", "linux", "gnu"), Mode::Exact);
    for (a, o, e) in [
        ("x86_64", "linux", "musl"),
        ("x86_64", "windows", "msvc"),
        ("x86_64", "macos", ""),
        ("aarch64", "linux", "gnu"),
        ("aarch64", "macos", ""),
    ] {
        assert_eq!(mode_for(a, o, e), Mode::CrossPlatform, "{a} {o} {e}");
    }
}

#[test]
fn ac3_current_mode_follows_the_build_target() {
    let expected = if cfg!(all(
        target_arch = "x86_64",
        target_os = "linux",
        target_env = "gnu"
    )) {
        Mode::Exact
    } else {
        Mode::CrossPlatform
    };
    assert_eq!(current_mode(), expected);
}

#[test]
fn arq_113_cross_platform_allowance_counts_pixels_not_bytes_on_rgba() {
    // 1000 RGBA pixels: the allowance is 1 pixel, not 4 (a byte count would give 4).
    let base = rgba(40, 25, &[100; 4000]);
    let with = |pixels: usize| {
        let mut v = vec![100u8; 4000];
        for p in 0..pixels {
            v[p * 4 + 3] = 102;
        }
        rgba(40, 25, &v)
    };
    assert!(compare(Mode::CrossPlatform, &base, &with(1)).is_ok());
    assert!(compare(Mode::CrossPlatform, &base, &with(2)).is_err());
    // 2000 RGBA pixels: allowance 2.
    let big = rgba(2000, 1, &vec![100; 8000]);
    let mut v = vec![100u8; 8000];
    v[0] = 102;
    v[4] = 102;
    assert!(compare(Mode::CrossPlatform, &big, &rgba(2000, 1, &v)).is_ok());
    v[8] = 102;
    assert!(compare(Mode::CrossPlatform, &big, &rgba(2000, 1, &v)).is_err());
}
