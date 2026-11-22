//! Openverse (<https://api.openverse.org>): anonymous search, 20 results a
//! page and the first 240 of a search.

use gafira_ports::photos::{
    Photo, PhotoError, PhotoLicense, PhotoOrientation, PhotoPage, PhotoQuery, PhotoRights,
    PhotoSource,
};
use serde::Deserialize;

use crate::{encode, words};

/// The address of a search.
pub(crate) fn search_url(query: &PhotoQuery) -> String {
    // Cut-outs: rawpixel's stickers, which come on a transparent ground.
    let words = if query.cutouts {
        format!("{} png sticker", words(query))
    } else {
        words(query).to_owned()
    };
    let mut url = format!(
        "https://api.openverse.org/v1/images/?q={}&page={}&mature=false",
        encode(&words),
        query.page.max(1)
    );
    if query.cutouts {
        url.push_str("&source=rawpixel");
    }
    url.push_str(match query.rights {
        PhotoRights::Free => "&license=cc0,pdm",
        PhotoRights::Commercial => "&license_type=commercial,modification",
        PhotoRights::Any => "",
    });
    if let Some(orientation) = query.orientation {
        url.push_str(match orientation {
            PhotoOrientation::Landscape => "&aspect_ratio=wide",
            PhotoOrientation::Portrait => "&aspect_ratio=tall",
            PhotoOrientation::Square => "&aspect_ratio=square",
        });
    }
    url
}

#[derive(Deserialize)]
struct Answer {
    #[serde(default)]
    page_count: u32,
    #[serde(default)]
    results: Vec<Item>,
}

#[derive(Deserialize)]
struct Item {
    id: String,
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    foreign_landing_url: Option<String>,
    url: String,
    #[serde(default)]
    creator: Option<String>,
    license: String,
    #[serde(default)]
    license_version: Option<String>,
    #[serde(default)]
    license_url: Option<String>,
    #[serde(default)]
    thumbnail: Option<String>,
    #[serde(default)]
    width: Option<u32>,
    #[serde(default)]
    height: Option<u32>,
}

/// The short name of an Openverse licence code and version.
fn licence_name(code: &str, version: Option<&str>) -> String {
    let name = match code {
        "cc0" => "CC0".to_owned(),
        "pdm" => "Public Domain Mark".to_owned(),
        other => format!("CC {}", other.to_uppercase()),
    };
    match version.filter(|v| !v.is_empty() && *v != "N/A") {
        Some(version) => format!("{name} {version}"),
        None => name,
    }
}

/// A page of an answer.
pub(crate) fn parse(body: &[u8], query: &PhotoQuery) -> Result<PhotoPage, PhotoError> {
    let answer: Answer =
        serde_json::from_slice(body).map_err(|error| PhotoError::Invalid(error.to_string()))?;
    let items = answer
        .results
        .into_iter()
        .map(|item| {
            let license = PhotoLicense::named(
                &licence_name(&item.license, item.license_version.as_deref()),
                item.license_url.as_deref().unwrap_or_default(),
            );
            Photo {
                source: PhotoSource::Openverse,
                thumbnail_url: item.thumbnail.unwrap_or_else(|| item.url.clone()),
                page_url: item.foreign_landing_url.unwrap_or_default(),
                title: item.title.unwrap_or_default(),
                author: item.creator.unwrap_or_default(),
                image_url: item.url,
                width: item.width.unwrap_or(0),
                height: item.height.unwrap_or(0),
                license,
                id: item.id,
            }
        })
        .collect();
    let page = query.page.max(1);
    Ok(PhotoPage {
        items,
        next_page: (page < answer.page_count).then_some(page + 1),
    })
}

#[cfg(test)]
pub(crate) mod tests {
    use gafira_ports::photos::LicenseTerm;

    use super::*;

    /// Two results as Openverse answers them (trimmed).
    pub(crate) const SAMPLE: &str = r#"{"result_count":240,"page_count":12,"page_size":20,"page":1,"results":[
{"id":"547f9a52","title":"nature","foreign_landing_url":"https://www.flickr.com/photos/1/2","url":"https://live.staticflickr.com/1/2_b.jpg","creator":"barnyz","license":"by-nc-sa","license_version":"2.0","license_url":"https://creativecommons.org/licenses/by-nc-sa/2.0/","height":681,"width":1024,"thumbnail":"https://api.openverse.org/v1/images/547f9a52/thumb/"},
{"id":"d6d8977b","title":"Lake","foreign_landing_url":"https://www.flickr.com/photos/3/4","url":"https://live.staticflickr.com/3/4_b.jpg","creator":"Ana","license":"cc0","license_version":"1.0","license_url":"https://creativecommons.org/publicdomain/zero/1.0/","height":800,"width":1200,"thumbnail":"https://api.openverse.org/v1/images/d6d8977b/thumb/"}]}"#;

    // S71: an answer becomes photos with their author, licence elements and
    // next page; filters go in the address.
    #[test]
    fn an_answer_becomes_photos() {
        let mut query = PhotoQuery::first(PhotoSource::Openverse, "nature");
        query.rights = PhotoRights::Any;
        let page = parse(SAMPLE.as_bytes(), &query).expect("parsed");
        assert_eq!(page.next_page, Some(2));
        let first = &page.items[0];
        assert_eq!(first.author, "barnyz");
        assert_eq!(first.license.name, "CC BY-NC-SA 2.0");
        assert_eq!(
            first.license.terms,
            [
                LicenseTerm::By,
                LicenseTerm::NonCommercial,
                LicenseTerm::ShareAlike
            ]
        );
        assert!(!first.license.free_to_use());
        assert!(page.items[1].license.free_to_use());
        assert_eq!((first.width, first.height), (1024, 681));
        let mut filtered = query.clone();
        filtered.rights = PhotoRights::Commercial;
        filtered.orientation = Some(PhotoOrientation::Square);
        let url = search_url(&filtered);
        assert!(
            url.contains("license_type=commercial,modification"),
            "{url}"
        );
        assert!(url.contains("aspect_ratio=square"), "{url}");
        assert!(!search_url(&query).contains("license"));
        let free = PhotoQuery::first(PhotoSource::Openverse, "nature");
        assert!(
            search_url(&free).contains("&license=cc0,pdm"),
            "free of rights by default"
        );
        let mut cutouts = free.clone();
        cutouts.cutouts = true;
        let url = search_url(&cutouts);
        assert!(url.contains("q=nature%20png%20sticker&"), "{url}");
        assert!(url.contains("&source=rawpixel"), "{url}");
        assert!(!search_url(&free).contains("rawpixel"));
    }
}
