#![forbid(unsafe_code)]

//! Remote photo catalogues (spec 071, D-111): Openverse and Wikimedia
//! Commons, without an API key. Implements [`PhotoCatalog`] over an
//! [`Http`] transport; the real one ([`online::UreqHttp`]) is behind the
//! `online` feature, which only the binary turns on.

mod bundled;
mod bundled_files;
mod iconify;
mod openverse;
mod wikimedia;

#[cfg(feature = "online")]
pub mod online;

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use gafira_ports::photos::{
    Illustration, IllustrationCatalog, IllustrationSet, IllustrationStyle, Photo,
    PhotoCapabilities, PhotoCatalog, PhotoError, PhotoOrientation, PhotoPage, PhotoQuery,
    PhotoSource,
};

/// The most bytes a thumbnail may have.
pub const THUMBNAIL_LIMIT: u64 = 5 * 1024 * 1024;

/// The most bytes an image to insert may have (spec 071 §10).
pub const IMAGE_LIMIT: u64 = 30 * 1024 * 1024;

/// How long a source rests after a 429 that does not say (spec 071 §8).
const DEFAULT_WAIT: Duration = Duration::from_secs(60);

/// The most searches kept for the session.
const CACHE_SIZE: usize = 128;

/// An answer of the transport.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Response {
    pub status: u16,
    /// `Retry-After`, in seconds, when it came.
    pub retry_after_s: Option<u64>,
    pub body: Vec<u8>,
}

/// A GET over HTTPS, at most `limit` bytes of body; no connection is
/// [`PhotoError::Offline`], more bytes [`PhotoError::TooLarge`].
pub trait Http: Send + Sync {
    fn get(&self, url: &str, limit: u64) -> Result<Response, PhotoError>;
}

/// Openverse and Wikimedia Commons over `http`.
pub struct Catalog {
    http: Box<dyn Http>,
    cache: Mutex<HashMap<PhotoQuery, PhotoPage>>,
    /// When each service may be searched again, after a 429.
    resting: Mutex<HashMap<&'static str, Instant>>,
}

impl Catalog {
    #[must_use]
    pub fn new(http: Box<dyn Http>) -> Self {
        Self {
            http,
            cache: Mutex::new(HashMap::new()),
            resting: Mutex::new(HashMap::new()),
        }
    }

    /// `url` fetched, when it is HTTPS: a 429 to a search (`search`) rests
    /// `source`, a 429 to an image only fails that image; a 404 or 410 is
    /// gone, a 5xx busy, another status an error.
    fn fetch(
        &self,
        service: &'static str,
        url: &str,
        (limit, search): (u64, bool),
    ) -> Result<Vec<u8>, PhotoError> {
        if !url.starts_with("https://") {
            return Err(PhotoError::Invalid("not https".to_owned()));
        }
        if search
            && let Some(until) = self
                .resting
                .lock()
                .ok()
                .and_then(|r| r.get(service).copied())
            && Instant::now() < until
        {
            let left = until.saturating_duration_since(Instant::now()).as_secs();
            return Err(PhotoError::RateLimited {
                retry_after_s: Some(left.max(1)),
            });
        }
        let response = self.http.get(url, limit)?;
        match response.status {
            200..=299 => Ok(response.body),
            429 => {
                let wait = response
                    .retry_after_s
                    .map_or(DEFAULT_WAIT, Duration::from_secs);
                if search && let Ok(mut resting) = self.resting.lock() {
                    resting.insert(service, Instant::now() + wait);
                }
                Err(PhotoError::RateLimited {
                    retry_after_s: Some(wait.as_secs()),
                })
            }
            404 | 410 => Err(PhotoError::Unavailable),
            500..=599 => Err(PhotoError::Busy),
            status => Err(PhotoError::Invalid(format!("status {status}"))),
        }
    }
}

impl PhotoCatalog for Catalog {
    fn sources(&self) -> Vec<(PhotoSource, PhotoCapabilities)> {
        vec![
            (
                PhotoSource::Openverse,
                PhotoCapabilities {
                    orientations: vec![
                        PhotoOrientation::Landscape,
                        PhotoOrientation::Portrait,
                        PhotoOrientation::Square,
                    ],
                },
            ),
            (
                PhotoSource::Wikimedia,
                PhotoCapabilities {
                    orientations: Vec::new(),
                },
            ),
        ]
    }

    fn search(&self, query: &PhotoQuery) -> Result<PhotoPage, PhotoError> {
        if let Some(page) = self.cache.lock().ok().and_then(|c| c.get(query).cloned()) {
            return Ok(page);
        }
        let url = match query.source {
            PhotoSource::Openverse => openverse::search_url(query),
            PhotoSource::Wikimedia => wikimedia::search_url(query),
        };
        let body = self.fetch(service(query.source), &url, (IMAGE_LIMIT, true))?;
        let page = match query.source {
            PhotoSource::Openverse => openverse::parse(&body, query)?,
            PhotoSource::Wikimedia => wikimedia::parse(&body, query)?,
        };
        if let Ok(mut cache) = self.cache.lock() {
            if cache.len() >= CACHE_SIZE {
                cache.clear();
            }
            cache.insert(query.clone(), page.clone());
        }
        Ok(page)
    }

    fn thumbnail(&self, photo: &Photo) -> Result<Vec<u8>, PhotoError> {
        self.fetch(
            service(photo.source),
            &photo.thumbnail_url,
            (THUMBNAIL_LIMIT, false),
        )
    }

    fn image(&self, photo: &Photo) -> Result<Vec<u8>, PhotoError> {
        self.fetch(
            service(photo.source),
            &photo.image_url,
            (IMAGE_LIMIT, false),
        )
    }
}

/// The service a source rests as after a 429.
fn service(source: PhotoSource) -> &'static str {
    match source {
        PhotoSource::Openverse => "openverse",
        PhotoSource::Wikimedia => "wikimedia",
    }
}

/// The service of the illustrations.
const ICONIFY: &str = "iconify";

/// The most bytes an illustration's SVG may have.
pub const SVG_LIMIT: u64 = 2 * 1024 * 1024;

impl IllustrationCatalog for Catalog {
    fn sets(&self) -> Vec<IllustrationSet> {
        let mut sets = bundled::sets();
        sets.extend(iconify::sets());
        sets
    }

    /// The bundled drawings first, then those of Iconify; without network
    /// the bundled ones still come.
    fn find(
        &self,
        text: &str,
        style: IllustrationStyle,
        limit: u32,
    ) -> Result<Vec<Illustration>, PhotoError> {
        if text.trim().is_empty() {
            return Ok(Vec::new());
        }
        let mut found = bundled::find(text, |color| style.allows(color));
        found.truncate(limit as usize);
        let left = limit.saturating_sub(found.len() as u32);
        if left == 0 {
            return Ok(found);
        }
        let url = iconify::search_url(text, style, left);
        match self
            .fetch(ICONIFY, &url, (SVG_LIMIT, true))
            .and_then(|body| iconify::parse_search(&body, left))
        {
            Ok(remote) => {
                found.extend(remote);
                Ok(found)
            }
            Err(_) if !found.is_empty() => Ok(found),
            Err(error) => Err(error),
        }
    }

    fn browse(&self, set: &str, limit: u32) -> Result<Vec<Illustration>, PhotoError> {
        if bundled::has(set) {
            return Ok(bundled::browse(set, limit));
        }
        if !iconify::known(set) {
            return Err(PhotoError::Unavailable);
        }
        let url = iconify::browse_url(set);
        let body = self.fetch(ICONIFY, &url, (IMAGE_LIMIT, true))?;
        iconify::parse_browse(&body, set, limit)
    }

    fn svg(&self, illustration: &Illustration) -> Result<Vec<u8>, PhotoError> {
        if let Some(bytes) = bundled::svg(illustration) {
            return Ok(bytes);
        }
        let url = iconify::svg_url(illustration).ok_or(PhotoError::Unavailable)?;
        self.fetch(ICONIFY, &url, (SVG_LIMIT, false))
    }
}

/// `text` with `%` escapes for a query string.
fn encode(text: &str) -> String {
    let mut out = String::new();
    for byte in text.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(char::from(byte));
            }
            b' ' => out.push_str("%20"),
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

/// The words of a search, or a default one when there are none (spec 071
/// §4.2: no empty searches).
fn words(query: &PhotoQuery) -> &str {
    let text = query.text.trim();
    if text.is_empty() { "nature" } else { text }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::*;

    /// A transport that answers `status` with `body` and counts its calls.
    struct Canned {
        status: u16,
        body: &'static str,
        calls: Arc<AtomicUsize>,
    }

    impl Http for Canned {
        fn get(&self, _url: &str, _limit: u64) -> Result<Response, PhotoError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(Response {
                status: self.status,
                retry_after_s: Some(30),
                body: self.body.as_bytes().to_vec(),
            })
        }
    }

    fn catalog(status: u16, body: &'static str) -> (Catalog, Arc<AtomicUsize>) {
        let calls = Arc::new(AtomicUsize::new(0));
        let http = Canned {
            status,
            body,
            calls: Arc::clone(&calls),
        };
        (Catalog::new(Box::new(http)), calls)
    }

    // S71: a search is asked once and then answered from the session cache.
    #[test]
    fn a_search_is_cached() {
        let (catalog, calls) = catalog(200, openverse::tests::SAMPLE);
        let query = PhotoQuery::first(PhotoSource::Openverse, "nature");
        let first = catalog.search(&query).expect("a page");
        assert_eq!(first.items.len(), 2);
        let again = catalog.search(&query).expect("a page");
        assert_eq!(first, again);
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    // Spec 071 §8: a 429 rests the source for what it asked, without
    // calling it again meanwhile.
    #[test]
    fn a_429_rests_the_source() {
        let (catalog, calls) = catalog(429, "");
        let query = PhotoQuery::first(PhotoSource::Openverse, "nature");
        assert_eq!(
            catalog.search(&query),
            Err(PhotoError::RateLimited {
                retry_after_s: Some(30)
            })
        );
        assert!(matches!(
            catalog.search(&query),
            Err(PhotoError::RateLimited { .. })
        ));
        assert_eq!(calls.load(Ordering::SeqCst), 1, "not asked while resting");
    }

    // A 429 to an image fails that image only: the source keeps searching
    // (the second drag from Wikimedia failed when it rested the source).
    #[test]
    fn a_429_to_an_image_does_not_rest_the_source() {
        let (catalog, calls) = catalog(429, "");
        let photo = openverse::parse(openverse::tests::SAMPLE.as_bytes(), &{
            let mut any = PhotoQuery::first(PhotoSource::Openverse, "x");
            any.rights = gafira_ports::photos::PhotoRights::Any;
            any
        })
        .expect("parsed")
        .items
        .remove(0);
        assert!(matches!(
            catalog.image(&photo),
            Err(PhotoError::RateLimited { .. })
        ));
        let _ = catalog.search(&PhotoQuery::first(PhotoSource::Openverse, "nature"));
        assert_eq!(
            calls.load(Ordering::SeqCst),
            2,
            "the search was still asked"
        );
    }

    // Spec 071 §7.2: only HTTPS addresses are fetched; a gone photo says so.
    #[test]
    fn only_https_and_gone_photos() {
        let (catalog, calls) = catalog(404, "");
        let mut photo = openverse::parse(openverse::tests::SAMPLE.as_bytes(), &{
            let mut any = PhotoQuery::first(PhotoSource::Openverse, "x");
            any.rights = gafira_ports::photos::PhotoRights::Any;
            any
        })
        .expect("parsed")
        .items
        .remove(0);
        assert_eq!(catalog.image(&photo), Err(PhotoError::Unavailable));
        photo.image_url = "http://example.org/a.jpg".to_owned();
        assert!(matches!(catalog.image(&photo), Err(PhotoError::Invalid(_))));
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn words_are_escaped() {
        assert_eq!(encode("café con leche"), "caf%C3%A9%20con%20leche");
        assert_eq!(encode("a&b=c"), "a%26b%3Dc");
    }
}
