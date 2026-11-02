//! FD-41, Subrayado y tachado: position and thickness come from `post` (underline) and
//! `OS/2` (strikethrough); a thickness of 0 or none is 0.05 em.

use gafira_geom::Size;
use gafira_testkit::fonts;
use gafira_text::{Cluster, Decoration, TextLayout, TextStyle, layout_box};

const INTER: &[u8] = fonts::INTER.bytes;
const ARABIC: &[u8] = fonts::NOTO_SANS_ARABIC.bytes;
/// 24 pt at 96 dpi.
const SIZE_PX: f32 = 32.0;

fn lay(text: &str, stack: &[&[u8]], underline: bool, strikethrough: bool) -> TextLayout {
    let s = TextStyle {
        size_pt: 24.0,
        underline,
        strikethrough,
        ..TextStyle::default()
    };
    layout_box(text, stack, &s, 96.0, Size::new(10_000.0, 1000.0)).expect("valid input")
}

fn table(font: &[u8], tag: &[u8; 4]) -> usize {
    let count = usize::from(u16::from_be_bytes([font[4], font[5]]));
    (0..count)
        .map(|i| 12 + 16 * i)
        .find(|rec| &font[*rec..*rec + 4] == tag)
        .map(|rec| {
            let b = &font[rec + 8..rec + 12];
            u32::from_be_bytes([b[0], b[1], b[2], b[3]]) as usize
        })
        .expect("table")
}

fn patched(font: &[u8], tag: &[u8; 4], edits: &[(usize, i16)]) -> Vec<u8> {
    let base = table(font, tag);
    let mut copy = font.to_vec();
    for (offset, value) in edits {
        copy[base + offset..base + offset + 2].copy_from_slice(&value.to_be_bytes());
    }
    copy
}

fn first(layout: &TextLayout) -> &Cluster {
    &layout.lines[0].clusters[0]
}

fn near(a: Decoration, offset: f32, thickness: f32) -> bool {
    (a.offset - offset).abs() <= 0.01 && (a.thickness - thickness).abs() <= 0.01
}

// Inter: upem 2048, post position -348, thickness 140; OS/2 strikeout position 671, size 140.
const INTER_UNDERLINE: (f32, f32) = (-348.0 * 32.0 / 2048.0, 140.0 * 32.0 / 2048.0);
const INTER_STRIKE: (f32, f32) = (671.0 * 32.0 / 2048.0, 140.0 * 32.0 / 2048.0);

#[test]
fn fd_41_underline_comes_from_post_and_strikethrough_from_os2() {
    let layout = lay("ab", &[INTER], true, true);
    for c in &layout.lines[0].clusters {
        let u = c.underline.expect("underline");
        let t = c.strikethrough.expect("strikethrough");
        assert!(near(u, INTER_UNDERLINE.0, INTER_UNDERLINE.1), "{u:?}");
        assert!(near(t, INTER_STRIKE.0, INTER_STRIKE.1), "{t:?}");
    }
}

#[test]
fn fd_41_decorations_are_off_unless_the_style_asks_for_them() {
    let c = lay("ab", &[INTER], false, false);
    assert!(first(&c).underline.is_none() && first(&c).strikethrough.is_none());
    let u = lay("ab", &[INTER], true, false);
    assert!(first(&u).underline.is_some() && first(&u).strikethrough.is_none());
    let t = lay("ab", &[INTER], false, true);
    assert!(first(&t).underline.is_none() && first(&t).strikethrough.is_some());
}

#[test]
fn fd_41_each_cluster_takes_the_decoration_of_its_own_font() {
    // Noto Sans Arabic: upem 1000, post -100 / 50, strikeout 322 / 50.
    let layout = lay("a\u{627}", &[INTER, ARABIC], true, true);
    let by_font = |font: usize| {
        layout.lines[0]
            .clusters
            .iter()
            .find(|c| c.font == font)
            .expect("cluster")
    };
    let arabic = by_font(1);
    assert!(near(
        arabic.underline.expect("u"),
        -100.0 * 32.0 / 1000.0,
        50.0 * 32.0 / 1000.0
    ));
    assert!(near(
        arabic.strikethrough.expect("s"),
        322.0 * 32.0 / 1000.0,
        50.0 * 32.0 / 1000.0
    ));
    assert!(near(
        by_font(0).underline.expect("u"),
        INTER_UNDERLINE.0,
        INTER_UNDERLINE.1
    ));
}

#[test]
fn fd_41_a_thickness_of_zero_is_five_hundredths_of_an_em() {
    // post thickness at 10, OS/2 strikeout size at 26.
    let post = patched(INTER, b"post", &[(10, 0)]);
    let font = patched(&post, b"OS/2", &[(26, 0)]);
    let layout = lay("a", &[font.as_slice()], true, true);
    let c = first(&layout);
    let em_05 = 0.05 * SIZE_PX;
    assert!(
        near(c.underline.expect("u"), INTER_UNDERLINE.0, em_05),
        "{:?}",
        c.underline
    );
    assert!(
        near(c.strikethrough.expect("s"), INTER_STRIKE.0, em_05),
        "{:?}",
        c.strikethrough
    );
    // A thickness of 1 unit is used as it is.
    let thin = patched(INTER, b"post", &[(10, 1)]);
    let layout = lay("a", &[thin.as_slice()], true, false);
    assert!(near(
        first(&layout).underline.expect("u"),
        INTER_UNDERLINE.0,
        32.0 / 2048.0
    ));
}

#[test]
fn fd_41_underline_and_strikethrough_do_not_read_each_others_table() {
    // Different values in post and OS/2: swapping the tables would show.
    let post = patched(INTER, b"post", &[(8, -500), (10, 200)]);
    let font = patched(&post, b"OS/2", &[(28, 900), (26, 300)]);
    let layout = lay("a", &[font.as_slice()], true, true);
    let c = first(&layout);
    let k = 32.0 / 2048.0;
    assert!(
        near(c.underline.expect("u"), -500.0 * k, 200.0 * k),
        "{:?}",
        c.underline
    );
    assert!(
        near(c.strikethrough.expect("s"), 900.0 * k, 300.0 * k),
        "{:?}",
        c.strikethrough
    );
}
