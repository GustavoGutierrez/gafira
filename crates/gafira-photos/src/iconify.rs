//! Vector illustrations from the Iconify public API
//! (<https://api.iconify.design>), without a key, only from collections
//! whose licence lets them be used and adapted freely, also commercially,
//! without credit in the design (MIT, Apache 2.0).

use gafira_ports::photos::{
    Illustration, IllustrationSet, IllustrationStyle, PhotoError, PhotoLicense,
};
use serde::Deserialize;

use crate::encode;

/// The collections searched: id, name, author, licence, its text and
/// whether it is in colour.
pub(crate) const SETS: [(&str, &str, &str, &str, &str, bool); 8] = [
    (
        "fluent-emoji-flat",
        "Fluent Emoji Flat",
        "Microsoft",
        "MIT",
        "https://github.com/microsoft/fluentui-emoji/blob/main/LICENSE",
        true,
    ),
    (
        "fluent-emoji",
        "Fluent Emoji",
        "Microsoft",
        "MIT",
        "https://github.com/microsoft/fluentui-emoji/blob/main/LICENSE",
        true,
    ),
    (
        "noto",
        "Noto Emoji",
        "Google",
        "Apache 2.0",
        "https://github.com/googlefonts/noto-emoji/blob/main/svg/LICENSE",
        true,
    ),
    (
        "fxemoji",
        "Firefox OS Emoji",
        "Mozilla",
        "Apache 2.0",
        "https://mozilla.github.io/fxemoji/LICENSE.md",
        true,
    ),
    (
        "flat-color-icons",
        "Flat Color Icons",
        "Icons8",
        "MIT",
        "https://github.com/icons8/flat-color-icons/blob/master/LICENSE.md",
        true,
    ),
    (
        "meteocons",
        "Meteocons",
        "Bas Milius",
        "MIT",
        "https://github.com/basmilius/weather-icons/blob/dev/LICENSE",
        true,
    ),
    // In black and white (owner request: colour or black and white).
    (
        "fluent-emoji-high-contrast",
        "Fluent Emoji High Contrast",
        "Microsoft",
        "MIT",
        "https://github.com/microsoft/fluentui-emoji/blob/main/LICENSE",
        false,
    ),
    (
        "healthicons",
        "Health Icons",
        "Resolve to Save Lives",
        "MIT",
        "https://github.com/resolvetosavelives/healthicons/blob/main/LICENSE",
        false,
    ),
];

/// The collections, as the port gives them.
pub(crate) fn sets() -> Vec<IllustrationSet> {
    SETS.iter()
        .map(|(id, name, author, license, url, color)| IllustrationSet {
            id: (*id).to_owned(),
            name: (*name).to_owned(),
            author: (*author).to_owned(),
            license: PhotoLicense {
                name: (*license).to_owned(),
                url: (*url).to_owned(),
                terms: Vec::new(),
            },
            color: *color,
        })
        .collect()
}

/// Whether `set` is one of ours.
pub(crate) fn known(set: &str) -> bool {
    SETS.iter().any(|(id, ..)| *id == set)
}

/// Whether `name` is a plain illustration name: lowercase letters, digits
/// and dashes, so it cannot change the address it goes into.
pub(crate) fn plain_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

/// The address of a search in every collection of ours of `style`.
pub(crate) fn search_url(text: &str, style: IllustrationStyle, limit: u32) -> String {
    let prefixes: Vec<&str> = SETS
        .iter()
        .filter(|set| style.allows(set.5))
        .map(|(id, ..)| *id)
        .collect();
    format!(
        "https://api.iconify.design/search?query={}&prefixes={}&limit={}",
        encode(text.trim()),
        prefixes.join(","),
        limit.clamp(32, 999)
    )
}

/// The address of the list of collection `set`.
pub(crate) fn browse_url(set: &str) -> String {
    format!(
        "https://api.iconify.design/collection?prefix={}",
        encode(set)
    )
}

/// The address of the SVG of `illustration`, when it is one of ours, at
/// 128 px so every thumbnail comes the same size.
pub(crate) fn svg_url(illustration: &Illustration) -> Option<String> {
    (known(&illustration.set) && plain_name(&illustration.name)).then(|| {
        format!(
            "https://api.iconify.design/{}/{}.svg?width=128&height=128",
            illustration.set, illustration.name
        )
    })
}

#[derive(Deserialize)]
struct Found {
    #[serde(default)]
    icons: Vec<String>,
}

/// The illustrations of a search answer, only from our collections.
pub(crate) fn parse_search(body: &[u8], limit: u32) -> Result<Vec<Illustration>, PhotoError> {
    let found: Found =
        serde_json::from_slice(body).map_err(|error| PhotoError::Invalid(error.to_string()))?;
    Ok(found
        .icons
        .iter()
        .filter_map(|id| {
            let (set, name) = id.split_once(':')?;
            (known(set) && plain_name(name)).then(|| Illustration {
                set: set.to_owned(),
                name: name.to_owned(),
            })
        })
        .take(limit as usize)
        .collect())
}

#[derive(Deserialize)]
struct Listed {
    #[serde(default)]
    uncategorized: Vec<String>,
    #[serde(default)]
    categories: std::collections::BTreeMap<String, Vec<String>>,
}

/// The first `limit` illustrations of the list of collection `set`.
pub(crate) fn parse_browse(
    body: &[u8],
    set: &str,
    limit: u32,
) -> Result<Vec<Illustration>, PhotoError> {
    let listed: Listed =
        serde_json::from_slice(body).map_err(|error| PhotoError::Invalid(error.to_string()))?;
    let mut seen = std::collections::BTreeSet::new();
    Ok(listed
        .categories
        .into_values()
        .flatten()
        .chain(listed.uncategorized)
        .filter(|name| plain_name(name) && seen.insert(name.clone()))
        .take(limit as usize)
        .map(|name| Illustration {
            set: set.to_owned(),
            name,
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    // Ilustraciones: only our collections, only plain names; the SVG
    // address cannot be steered elsewhere.
    #[test]
    fn only_our_collections_and_plain_names() {
        let body =
            br#"{"icons":["noto:sunflower","twemoji:rose","fluent-emoji-flat:tulip","noto:../x"]}"#;
        let found = parse_search(body, 10).expect("parsed");
        let names: Vec<&str> = found.iter().map(|i| i.name.as_str()).collect();
        assert_eq!(names, ["sunflower", "tulip"], "twemoji is CC BY: not ours");
        assert_eq!(
            svg_url(&found[0]).as_deref(),
            Some("https://api.iconify.design/noto/sunflower.svg?width=128&height=128")
        );
        let foreign = Illustration {
            set: "twemoji".to_owned(),
            name: "rose".to_owned(),
        };
        assert_eq!(svg_url(&foreign), None);
        assert!(search_url("flor roja", IllustrationStyle::Any, 8).contains("query=flor%20roja"));
        assert!(
            search_url("x", IllustrationStyle::Any, 8).contains("limit=32"),
            "Iconify asks 32 or more"
        );
        let mono = search_url("x", IllustrationStyle::Mono, 8);
        assert!(
            mono.contains("prefixes=fluent-emoji-high-contrast,healthicons&"),
            "{mono}"
        );
        assert!(!search_url("x", IllustrationStyle::Color, 8).contains("healthicons"));
        let listed = br#"{"prefix":"meteocons","categories":{"Weather":["sun","rain"]},"uncategorized":["sun","cloud"]}"#;
        let browsed = parse_browse(listed, "meteocons", 3).expect("parsed");
        let names: Vec<&str> = browsed.iter().map(|i| i.name.as_str()).collect();
        assert_eq!(names, ["sun", "rain", "cloud"]);
    }
}
