//! The WebP exporter (FD-55, spec 039): lossless only, because `image-webp`
//! encodes no lossy WebP.

use std::io;

use gafira_doc::contract::{ParamSchema, ParamValues};
use gafira_ports::common::LegalClass;
use gafira_ports::export::ExportError;
use gafira_ports::format::{ByteSignature, FormatDescriptor, FormatKind};
use image_webp::{ColorType, WebPEncoder};

use crate::raster::{Raster, delegate_exporter, scale_param, transparent_param};

/// The WebP format (FD-55).
pub const WEBP_FORMAT: FormatDescriptor = FormatDescriptor {
    id: "webp",
    extensions: &["webp"],
    mime: &["image/webp"],
    signatures: &[ByteSignature {
        offset: 8,
        bytes: b"WEBP",
    }],
    kind: FormatKind::Raster,
    legal: LegalClass::RoyaltyFree,
};

/// The WebP exporter: one lossless 8-bit RGBA file per page of the scope,
/// at a scale of 0.5× to 4×, on the page background or transparent
/// (FD-55).
pub struct WebpExporter(Raster);

impl Default for WebpExporter {
    fn default() -> Self {
        Self::new()
    }
}

impl WebpExporter {
    /// The exporter with the options of FD-55 this cut has.
    #[must_use]
    pub fn new() -> Self {
        Self(Raster {
            format: &WEBP_FORMAT,
            schema: ParamSchema::new(vec![scale_param(), transparent_param()]),
            transparency: true,
            encode,
        })
    }
}

delegate_exporter!(WebpExporter);

/// The lossless WebP bytes of straight 8-bit sRGB RGBA pixels.
fn encode(
    width: u32,
    height: u32,
    pixels: Vec<u8>,
    _options: &ParamValues,
) -> Result<Vec<u8>, ExportError> {
    let mut out = Vec::new();
    WebPEncoder::new(&mut out)
        .encode(&pixels, width, height, ColorType::Rgba8)
        .map_err(|error| ExportError::Io(io::Error::other(error.to_string())))?;
    Ok(out)
}
