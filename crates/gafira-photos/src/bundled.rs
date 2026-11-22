//! The illustrations bundled with Gafira (spec 072): Open Peeps and Open
//! Doodles by Pablo Stanley, CC0 1.0. They are searched first and need no
//! network.

use gafira_ports::photos::{Illustration, IllustrationSet, PhotoLicense};

use crate::bundled_files::{OPEN_DOODLES, OPEN_PEEPS};

/// The collection ids.
pub(crate) const PEEPS: &str = "open-peeps";
pub(crate) const DOODLES: &str = "open-doodles";

/// The words that find every drawing of a collection, besides its names.
const PEEP_WORDS: [&str; 8] = [
    "peep",
    "peeps",
    "person",
    "people",
    "persona",
    "personas",
    "gente",
    "personaje",
];
const DOODLE_WORDS: [&str; 7] = [
    "doodle", "doodles", "dibujo", "person", "people", "persona", "gente",
];

/// The collections, CC0, both line drawings in black and white (Open
/// Doodles with a few colours).
pub(crate) fn sets() -> Vec<IllustrationSet> {
    let cc0 = || PhotoLicense {
        name: "CC0 1.0".to_owned(),
        url: "https://creativecommons.org/publicdomain/zero/1.0/".to_owned(),
        terms: vec![gafira_ports::photos::LicenseTerm::Zero],
    };
    vec![
        IllustrationSet {
            id: PEEPS.to_owned(),
            name: "Open Peeps".to_owned(),
            author: "Pablo Stanley".to_owned(),
            license: cc0(),
            color: false,
        },
        IllustrationSet {
            id: DOODLES.to_owned(),
            name: "Open Doodles".to_owned(),
            author: "Pablo Stanley".to_owned(),
            license: cc0(),
            color: true,
        },
    ]
}

/// The drawings of collection `set`, or none when it is not bundled.
fn files(set: &str) -> &'static [(&'static str, &'static [u8])] {
    match set {
        PEEPS => &OPEN_PEEPS,
        DOODLES => &OPEN_DOODLES,
        _ => &[],
    }
}

/// Whether `set` is bundled.
pub(crate) fn has(set: &str) -> bool {
    !files(set).is_empty()
}

/// The first `limit` drawings of `set`.
pub(crate) fn browse(set: &str, limit: u32) -> Vec<Illustration> {
    files(set)
        .iter()
        .take(limit as usize)
        .map(|(name, _)| Illustration {
            set: set.to_owned(),
            name: (*name).to_owned(),
        })
        .collect()
}

/// The bundled drawings `text` finds in the collections `allowed` lets in:
/// a word of it in a drawing's name, or a word of its collection.
pub(crate) fn find(text: &str, allowed: impl Fn(bool) -> bool) -> Vec<Illustration> {
    let words: Vec<String> = text
        .split_whitespace()
        .map(str::to_lowercase)
        .filter(|w| w.len() > 1)
        .collect();
    if words.is_empty() {
        return Vec::new();
    }
    let mut out = Vec::new();
    for set in sets().into_iter().filter(|s| allowed(s.color)) {
        let keys: &[&str] = if set.id == PEEPS {
            &PEEP_WORDS
        } else {
            &DOODLE_WORDS
        };
        let whole = words.iter().any(|w| keys.contains(&w.as_str()));
        for (name, _) in files(&set.id) {
            if whole || words.iter().any(|w| name.contains(w.as_str())) {
                out.push(Illustration {
                    set: set.id.clone(),
                    name: (*name).to_owned(),
                });
            }
        }
    }
    out
}

/// The SVG of a bundled drawing.
pub(crate) fn svg(illustration: &Illustration) -> Option<Vec<u8>> {
    files(&illustration.set)
        .iter()
        .find(|(name, _)| *name == illustration.name)
        .map(|(_, bytes)| bytes.to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    // Spec 072: Open Peeps and Open Doodles are bundled, found by their
    // names or by «persona», and their SVGs are there without network.
    #[test]
    fn bundled_drawings_are_found_offline() {
        assert_eq!(OPEN_PEEPS.len(), 93);
        assert_eq!(OPEN_DOODLES.len(), 33);
        let coffee = find("coffee", |_| true);
        assert_eq!(coffee.len(), 1);
        assert_eq!(coffee[0].set, DOODLES);
        let bytes = svg(&coffee[0]).expect("bundled");
        assert!(bytes.windows(4).any(|w| w == b"<svg"));
        let people = find("persona", |_| true);
        assert_eq!(people.len(), 93 + 33);
        assert!(
            find("persona", |color| !color)
                .iter()
                .all(|i| i.set == PEEPS)
        );
        assert_eq!(browse(PEEPS, 5).len(), 5);
        assert!(
            svg(&Illustration {
                set: PEEPS.to_owned(),
                name: "../x".to_owned()
            })
            .is_none()
        );
    }
}
