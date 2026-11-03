//! FD-41 (g) and D-19: the font of each cluster across a fallback list, through parley
//! alone (XP12). Inter and Noto Sans Arabic are partial subsets, so they split clusters.

use gafira_geom::Size;
use gafira_testkit::fonts;
use gafira_text::{Cluster, TextLayout, TextStyle, layout_box};

const INTER: &[u8] = fonts::INTER.bytes;
const ARABIC: &[u8] = fonts::NOTO_SANS_ARABIC.bytes;

fn lay(text: &str, stack: &[&[u8]]) -> TextLayout {
    let s = TextStyle {
        size_pt: 24.0,
        ..TextStyle::default()
    };
    layout_box(text, stack, &s, 96.0, Size::new(10_000.0, 1000.0)).expect("valid input")
}

fn clusters(layout: &TextLayout) -> Vec<&Cluster> {
    layout.lines.iter().flat_map(|l| &l.clusters).collect()
}

/// The font and `.notdef` flag of the only cluster of `text`.
fn only(text: &str, stack: &[&[u8]]) -> (usize, bool) {
    let layout = lay(text, stack);
    let all = clusters(&layout);
    assert_eq!(all.len(), 1, "{text:?} is one cluster");
    (all[0].font, all[0].notdef)
}

const DOTLESS_I: &str = "\u{131}"; // Inter only
const SHADDA: &str = "\u{651}"; // Arabic only
const FATHATAN: &str = "\u{64b}"; // Arabic only

#[test]
fn fd_41_g_a_mixed_text_uses_each_font_for_its_own_clusters() {
    let layout = lay("ab \u{627}\u{628}", &[INTER, ARABIC]);
    let fonts: Vec<usize> = clusters(&layout).iter().map(|c| c.font).collect();
    // Visual order: a, b, space, then the Arabic letters from the right.
    assert_eq!(fonts, [0, 0, 0, 1, 1]);
    assert!(clusters(&layout).iter().all(|c| !c.notdef));
}

#[test]
fn fd_41_g_without_a_fallback_the_arabic_clusters_are_notdef_and_flagged() {
    let layout = lay("ab \u{627}\u{628}", &[INTER]);
    let flags: Vec<bool> = clusters(&layout).iter().map(|c| c.notdef).collect();
    assert_eq!(flags, [false, false, false, true, true]);
}

#[test]
fn fd_41_g_a_cluster_uses_the_first_font_that_covers_all_its_code_points() {
    // Space + shadda: Inter covers the space only, Arabic covers both.
    let text = format!(" {SHADDA}");
    assert_eq!(only(&text, &[INTER, ARABIC]), (1, false));
    assert_eq!(only(&text, &[ARABIC, INTER]), (0, false));
    // Both fonts cover the space and the cluster is whole in either: the first wins.
    assert_eq!(only(" ", &[INTER, ARABIC]), (0, false));
    assert_eq!(only(" ", &[ARABIC, INTER]), (0, false));
}

#[test]
fn fd_41_g_with_no_full_cover_the_font_with_most_code_points_gives_the_notdef() {
    // dotless i (Inter) + shadda (Arabic) + fathatan (Arabic): Arabic covers 2 of 3.
    let text = format!("{DOTLESS_I}{SHADDA}{FATHATAN}");
    assert_eq!(only(&text, &[INTER, ARABIC]), (1, true));
    assert_eq!(only(&text, &[ARABIC, INTER]), (0, true));
}

#[test]
fn fd_41_g_in_a_tie_the_first_font_of_the_list_gives_the_notdef() {
    // dotless i (Inter) + shadda (Arabic): 1 of 2 each.
    let text = format!("{DOTLESS_I}{SHADDA}");
    assert_eq!(only(&text, &[INTER, ARABIC]), (0, true));
    assert_eq!(only(&text, &[ARABIC, INTER]), (0, true));
}

#[test]
fn fd_41_g_a_font_that_covers_nothing_of_the_cluster_does_not_win_a_tie() {
    // Neither font covers CJK: the first one gives the .notdef.
    assert_eq!(only("\u{4e2d}", &[INTER, ARABIC]), (0, true));
    assert_eq!(only("\u{4e2d}", &[ARABIC, INTER]), (0, true));
}
