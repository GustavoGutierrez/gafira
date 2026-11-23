//! The real transport (D-115): HTTPS with `ureq`, rustls on ring and the
//! system's certificates, a generic Gafira `User-Agent` with nothing about
//! the person.

use std::io::Read;
use std::time::Duration;

use gafira_ports::vision::VisionError;
use ureq::tls::{RootCerts, TlsConfig, TlsProvider};

use crate::fetch::{Fetch, Sink};

/// `ureq` over HTTPS.
pub struct UreqFetch {
    agent: ureq::Agent,
}

impl UreqFetch {
    /// A transport that names itself `Gafira/<version>`.
    #[must_use]
    pub fn new(version: &str) -> Self {
        let tls = TlsConfig::builder()
            .provider(TlsProvider::Rustls)
            .root_certs(RootCerts::PlatformVerifier)
            .build();
        // A large file takes its time: only silence is an error.
        let agent = ureq::Agent::config_builder()
            .tls_config(tls)
            .timeout_connect(Some(Duration::from_secs(20)))
            .timeout_recv_response(Some(Duration::from_secs(30)))
            .user_agent(format!("Gafira/{version} (desktop design editor)"))
            .http_status_as_error(false)
            .https_only(true)
            .build()
            .into();
        Self { agent }
    }
}

impl Fetch for UreqFetch {
    fn fetch(&self, url: &str, sink: &mut Sink<'_>) -> Result<(), VisionError> {
        let response = self
            .agent
            .get(url)
            .call()
            .map_err(|_| VisionError::Offline)?;
        if response.status().as_u16() != 200 {
            return Err(VisionError::Corrupt);
        }
        let mut body = response.into_body().into_reader();
        let mut buffer = vec![0; 1 << 16];
        loop {
            match body.read(&mut buffer) {
                Ok(0) => return Ok(()),
                Ok(n) => sink(&buffer[..n])?,
                Err(_) => return Err(VisionError::Offline),
            }
        }
    }
}
