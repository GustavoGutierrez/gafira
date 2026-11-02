//! ADR 0012: every `SegError` variant has a catalogue key; wrapped errors delegate.

use gafira_doc::contract::Cancelled;
use gafira_doc::msg::ToMessage;
use gafira_raster::{BufferError, PixelFormat};
use gafira_segment::SegError;

// ARQ-102 / ADR 0012
#[test]
fn seg_error_maps_to_keys_and_delegates() {
    let b = BufferError::NonFinite {
        field: "pixels",
        index: 3,
    };
    assert_eq!(SegError::from(b).to_message(), b.to_message());
    assert_eq!(
        SegError::from(Cancelled).to_message().key.0,
        "error.job.cancelled"
    );
    assert_eq!(
        SegError::EmptyImage {
            width: 0,
            height: 4
        }
        .to_message()
        .key
        .0,
        "error.segment.empty_image"
    );
    assert_eq!(
        SegError::UnsupportedFormat {
            format: PixelFormat::Alpha8
        }
        .to_message()
        .key
        .0,
        "error.segment.unsupported_format"
    );
}
