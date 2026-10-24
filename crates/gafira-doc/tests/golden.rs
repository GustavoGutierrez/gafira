//! Canonical goldens: loaded without validation in D3, re-saved byte for byte.

use gafira_doc::Document;

const MINIMAL: &str = include_str!("golden/minimal.json");
const FULL: &str = include_str!("golden/full.json");
const EXTENSION: &str = include_str!("golden/extension.json");
const NONCANONICAL: &str = include_str!("golden/extension_noncanonical.json");

fn resave(text: &str) -> String {
    Document::from_json(text.as_bytes())
        .unwrap()
        .to_json()
        .unwrap()
}

#[test]
fn canonical_goldens_round_trip_byte_for_byte() {
    for (name, golden) in [
        ("minimal", MINIMAL),
        ("full", FULL),
        ("extension", EXTENSION),
    ] {
        assert_eq!(resave(golden), golden, "{name}");
    }
}

#[test]
fn noncanonical_golden_normalizes_to_the_canonical_one() {
    assert_ne!(NONCANONICAL, EXTENSION);
    let once = resave(NONCANONICAL);
    assert_eq!(once, EXTENSION);
    assert_eq!(resave(&once), once);
    assert_eq!(
        Document::from_json(NONCANONICAL.as_bytes()).unwrap(),
        Document::from_json(EXTENSION.as_bytes()).unwrap()
    );
}

#[test]
fn goldens_have_no_newline_carriage_return_or_bom() {
    for (name, golden) in [
        ("minimal", MINIMAL),
        ("full", FULL),
        ("extension", EXTENSION),
        ("extension_noncanonical", NONCANONICAL),
    ] {
        assert!(!golden.is_empty(), "{name}");
        assert!(!golden.contains(['\n', '\r']), "{name}");
        assert!(!golden.starts_with('\u{feff}'), "{name}");
        assert!(golden.starts_with('{') && golden.ends_with('}'), "{name}");
    }
}
