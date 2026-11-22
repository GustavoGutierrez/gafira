//! Wikimedia Commons (<https://commons.wikimedia.org/w/api.php>): bitmap
//! files of the File namespace, 20 a page, with their licence and author.

#[cfg(test)]
use gafira_ports::photos::PhotoRights;
use gafira_ports::photos::{Photo, PhotoError, PhotoLicense, PhotoPage, PhotoQuery, PhotoSource};
use serde::Deserialize;

use crate::{encode, words};

/// Results a page.
const PAGE: u32 = 20;

/// The widths Commons keeps thumbnails at, largest first: the one inserted
/// is the largest no wider than the original.
const WIDTHS: [u32; 5] = [1920, 1280, 960, 500, 330];

/// The address of the version of a photo to insert: a thumbnail of
/// `thumb.wikimedia.org` at the largest kept width that fits `width`.
/// `upload.wikimedia.org` answers 429 to HTTP/1.1 clients (D-111), so the
/// original is used only when there is no thumbnail.
fn insert_url(thumbnail: &str, original: &str, width: u32) -> String {
    match WIDTHS.iter().find(|w| **w <= width) {
        Some(w) if thumbnail.contains("/330px-") => {
            thumbnail.replace("/330px-", &format!("/{w}px-"))
        }
        _ => original.to_owned(),
    }
}

/// The address of a search.
pub(crate) fn search_url(query: &PhotoQuery) -> String {
    format!(
        "https://commons.wikimedia.org/w/api.php?action=query&format=json&formatversion=2\
         &generator=search&gsrnamespace=6&gsrsearch={}%20filetype:bitmap&gsrlimit={PAGE}\
         &gsroffset={}&prop=imageinfo&iiprop=url|size|extmetadata&iiurlwidth=330\
         &iiextmetadatafilter=Artist|LicenseShortName|LicenseUrl|ObjectName",
        encode(words(query)),
        (query.page.max(1) - 1) * PAGE
    )
}

#[derive(Deserialize)]
struct Answer {
    #[serde(default, rename = "continue")]
    more: Option<serde_json::Value>,
    #[serde(default)]
    query: Option<Query>,
}

#[derive(Deserialize)]
struct Query {
    #[serde(default)]
    pages: Vec<Page>,
}

#[derive(Deserialize)]
struct Page {
    pageid: u64,
    title: String,
    #[serde(default)]
    index: u32,
    #[serde(default)]
    imageinfo: Vec<Info>,
}

#[derive(Deserialize)]
struct Info {
    url: String,
    #[serde(default)]
    thumburl: Option<String>,
    #[serde(default)]
    descriptionurl: Option<String>,
    #[serde(default)]
    width: u32,
    #[serde(default)]
    height: u32,
    #[serde(default)]
    extmetadata: Option<Meta>,
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Meta {
    artist: Option<Field>,
    license_short_name: Option<Field>,
    license_url: Option<Field>,
    object_name: Option<Field>,
}

#[derive(Deserialize)]
struct Field {
    value: serde_json::Value,
}

impl Field {
    fn text(&self) -> String {
        match &self.value {
            serde_json::Value::String(text) => plain(text),
            other => other.to_string(),
        }
    }
}

/// `html` as plain text: tags dropped, the usual entities read, spaces
/// joined.
fn plain(html: &str) -> String {
    let mut out = String::new();
    let mut in_tag = false;
    for c in html.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            c if !in_tag => out.push(c),
            _ => {}
        }
    }
    let out = out
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&nbsp;", " ");
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// The name of a file without `File:` and its extension.
fn title_of(file: &str) -> String {
    let name = file.strip_prefix("File:").unwrap_or(file);
    match name.rsplit_once('.') {
        Some((stem, _)) => stem.to_owned(),
        None => name.to_owned(),
    }
}

/// A page of an answer: in the order of the search, only the licences the
/// query asks for (Commons has no licence filter in its search, so it is
/// each page that is filtered; a page may come shorter).
pub(crate) fn parse(body: &[u8], query: &PhotoQuery) -> Result<PhotoPage, PhotoError> {
    let answer: Answer =
        serde_json::from_slice(body).map_err(|error| PhotoError::Invalid(error.to_string()))?;
    let mut pages = answer.query.map(|q| q.pages).unwrap_or_default();
    pages.sort_by_key(|page| page.index);
    let items = pages
        .into_iter()
        .filter_map(|page| {
            let info = page.imageinfo.into_iter().next()?;
            let meta = info.extmetadata.unwrap_or_default();
            let text = |field: &Option<Field>| field.as_ref().map(Field::text).unwrap_or_default();
            let license =
                PhotoLicense::named(&text(&meta.license_short_name), &text(&meta.license_url));
            let thumbnail = info.thumburl.clone().unwrap_or_else(|| info.url.clone());
            let image_url = insert_url(&thumbnail, &info.url, info.width);
            let title = match text(&meta.object_name) {
                name if name.is_empty() => title_of(&page.title),
                name => name,
            };
            Some(Photo {
                source: PhotoSource::Wikimedia,
                id: page.pageid.to_string(),
                title,
                author: text(&meta.artist),
                page_url: info.descriptionurl.unwrap_or_default(),
                license,
                thumbnail_url: thumbnail,
                image_url,
                width: info.width,
                height: info.height,
            })
        })
        .filter(|photo| query.rights.allows(&photo.license))
        .collect();
    let page = query.page.max(1);
    Ok(PhotoPage {
        items,
        next_page: answer.more.is_some().then_some(page + 1),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"{"batchcomplete":true,"continue":{"gsroffset":20},"query":{"pages":[
{"pageid":2,"ns":6,"title":"File:Lake Tekapo 01.jpg","index":2,"imageinfo":[{"width":4568,"height":2886,
"thumburl":"https://upload.wikimedia.org/thumb/e/e2/Lake.jpg/330px-Lake.jpg","url":"https://upload.wikimedia.org/e/e2/Lake.jpg",
"descriptionurl":"https://commons.wikimedia.org/wiki/File:Lake.jpg","extmetadata":{
"Artist":{"value":"<a href=\"//commons.wikimedia.org/wiki/User:A\">Ana &amp; Luis</a>"},
"LicenseShortName":{"value":"CC BY-SA 4.0"},"LicenseUrl":{"value":"https://creativecommons.org/licenses/by-sa/4.0"}}}]},
{"pageid":1,"ns":6,"title":"File:Old map.png","index":1,"imageinfo":[{"width":800,"height":600,
"thumburl":"https://upload.wikimedia.org/thumb/a/a1/Map.png/330px-Map.png","url":"https://upload.wikimedia.org/a/a1/Map.png",
"descriptionurl":"https://commons.wikimedia.org/wiki/File:Map.png","extmetadata":{
"LicenseShortName":{"value":"GFDL"}}}]}]}}"#;

    // S71: Commons pages become photos in search order, the author as plain
    // text, a large original as its 1920 px version; with the commercial
    // filter only those free to use stay.
    #[test]
    fn an_answer_becomes_photos() {
        let mut query = PhotoQuery::first(PhotoSource::Wikimedia, "lake");
        query.rights = PhotoRights::Any;
        let page = parse(SAMPLE.as_bytes(), &query).expect("parsed");
        assert_eq!(page.next_page, Some(2));
        assert_eq!(page.items[0].title, "Old map");
        let lake = &page.items[1];
        assert_eq!(lake.author, "Ana & Luis");
        assert_eq!(lake.license.name, "CC BY-SA 4.0");
        assert!(
            lake.image_url.contains("/1920px-Lake.jpg"),
            "{}",
            lake.image_url
        );
        assert!(
            page.items[0].image_url.contains("/500px-Map.png"),
            "an 800 px original comes in at 500: {}",
            page.items[0].image_url
        );
        assert_eq!(
            insert_url("https://x/330px-a.jpg", "https://upload/a.jpg", 200),
            "https://upload/a.jpg",
            "smaller than every thumbnail"
        );
        let mut commercial = query.clone();
        commercial.rights = PhotoRights::Commercial;
        let page = parse(SAMPLE.as_bytes(), &commercial).expect("parsed");
        assert_eq!(page.items.len(), 1, "GFDL is not marked free");
        let free = PhotoQuery::first(PhotoSource::Wikimedia, "lake");
        let page = parse(SAMPLE.as_bytes(), &free).expect("parsed");
        assert!(page.items.is_empty(), "CC BY-SA is not free of rights");
        assert!(search_url(&query).contains("gsroffset=0"));
    }
}
