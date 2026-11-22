//! Where a model is downloaded from (S84-06).

use gafira_ports::vision::VisionError;

/// Takes the bytes of a download as they come; an error stops it.
pub type Sink<'a> = dyn FnMut(&[u8]) -> Result<(), VisionError> + 'a;

/// A GET over HTTPS.
pub trait Fetch: Send + Sync {
    /// Hands the bytes at `url` over to `sink`.
    ///
    /// # Errors
    /// [`VisionError::Offline`] without a connection, the one of `sink`, or
    /// [`VisionError::Corrupt`] for another answer.
    fn fetch(&self, url: &str, sink: &mut Sink<'_>) -> Result<(), VisionError>;
}

/// No network: every download is offline.
pub struct NoNetwork;

impl Fetch for NoNetwork {
    fn fetch(&self, _url: &str, _sink: &mut Sink<'_>) -> Result<(), VisionError> {
        Err(VisionError::Offline)
    }
}
