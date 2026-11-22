//! The real transport (D-111): HTTPS with `ureq`, rustls on ring and the
//! system's certificates, a generic Gafira `User-Agent` with nothing about
//! the person.

use std::time::Duration;

use gafira_ports::photos::PhotoError;
use ureq::tls::{RootCerts, TlsConfig, TlsProvider};

use crate::{Http, Response};

/// `ureq` over HTTPS.
pub struct UreqHttp {
    agent: ureq::Agent,
}

impl UreqHttp {
    /// A transport that names itself `Gafira/<version>`.
    #[must_use]
    pub fn new(version: &str) -> Self {
        let tls = TlsConfig::builder()
            .provider(TlsProvider::Rustls)
            .root_certs(RootCerts::PlatformVerifier)
            .build();
        let agent = ureq::Agent::config_builder()
            .tls_config(tls)
            .timeout_global(Some(Duration::from_secs(30)))
            .user_agent(format!("Gafira/{version} (desktop design editor)"))
            .http_status_as_error(false)
            .https_only(true)
            .build()
            .into();
        Self { agent }
    }
}

impl Http for UreqHttp {
    fn get(&self, url: &str, limit: u64) -> Result<Response, PhotoError> {
        let mut response = self.agent.get(url).call().map_err(|error| match error {
            ureq::Error::Timeout(_) | ureq::Error::Io(_) | ureq::Error::HostNotFound => {
                PhotoError::Offline
            }
            other => PhotoError::Invalid(other.to_string()),
        })?;
        let status = response.status().as_u16();
        let retry_after_s = response
            .headers()
            .get("retry-after")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.trim().parse().ok());
        let body = response
            .body_mut()
            .with_config()
            .limit(limit)
            .read_to_vec()
            .map_err(|error| match error {
                ureq::Error::BodyExceedsLimit(_) => PhotoError::TooLarge,
                ureq::Error::Timeout(_) | ureq::Error::Io(_) => PhotoError::Offline,
                other => PhotoError::Invalid(other.to_string()),
            })?;
        Ok(Response {
            status,
            retry_after_s,
            body,
        })
    }
}

#[cfg(test)]
mod tests {
    use gafira_ports::photos::{PhotoCatalog, PhotoQuery, PhotoSource};

    use super::*;
    use crate::Catalog;

    // D-111: one real search and one thumbnail from each source. Ignored:
    // it needs the network; run it by hand (`--ignored`), never in CI.
    #[test]
    #[ignore = "needs the network"]
    fn live_search() {
        let catalog = Catalog::new(Box::new(UreqHttp::new("test")));
        let mut answered = 0;
        for source in [PhotoSource::Openverse, PhotoSource::Wikimedia] {
            let page = match catalog.search(&PhotoQuery::first(source, "nature")) {
                Ok(page) => page,
                // A source may be down for a while (a 5xx): another answers.
                Err(gafira_ports::photos::PhotoError::Busy) => {
                    println!("{source:?}: busy");
                    continue;
                }
                Err(error) => panic!("{source:?}: {error}"),
            };
            assert!(!page.items.is_empty(), "{source:?}");
            let bytes = catalog.thumbnail(&page.items[0]).expect("a thumbnail");
            assert!(bytes.len() > 500, "{source:?}");
            println!(
                "{source:?}: {} — {}",
                page.items.len(),
                page.items[0].credit()
            );
            answered += 1;
        }
        assert!(answered > 0, "no source answered");
    }

    // Diagnosis: the image to insert of the first free photo of each source.
    #[test]
    #[ignore = "needs the network"]
    fn live_image() {
        let catalog = Catalog::new(Box::new(UreqHttp::new("test")));
        for source in [PhotoSource::Wikimedia, PhotoSource::Openverse] {
            match catalog.search(&PhotoQuery::first(source, "nature")) {
                Ok(page) => {
                    for photo in page.items.iter().take(3) {
                        let result = catalog.image(photo).map(|b| b.len());
                        println!("{source:?} {} -> {result:?}", photo.image_url);
                    }
                }
                Err(error) => println!("{source:?} search: {error}"),
            }
        }
    }

    // Spec 072: a real search, a collection and one SVG from Iconify.
    #[test]
    #[ignore = "needs the network"]
    fn live_illustrations() {
        use gafira_ports::photos::{IllustrationCatalog, IllustrationStyle};
        let catalog = Catalog::new(Box::new(UreqHttp::new("test")));
        let found = catalog
            .find("flower", IllustrationStyle::Any, 8)
            .expect("found");
        assert!(!found.is_empty());
        let mono = catalog
            .find("heart", IllustrationStyle::Mono, 8)
            .expect("found");
        assert!(
            mono.iter()
                .all(|i| i.set == "fluent-emoji-high-contrast" || i.set == "healthicons")
        );
        let weather = catalog.browse("meteocons", 8).expect("listed");
        assert_eq!(weather.len(), 8);
        let svg = catalog.svg(&found[0]).expect("an svg");
        assert!(
            svg.starts_with(b"<svg"),
            "{}",
            String::from_utf8_lossy(&svg[..40.min(svg.len())])
        );
        println!(
            "{} found, {} mono, svg {} bytes",
            found.len(),
            mono.len(),
            svg.len()
        );
    }
}
