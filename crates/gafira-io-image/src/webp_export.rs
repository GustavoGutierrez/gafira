//! The WebP exporter (FD-55, spec 039): lossless only, because `image-webp`
//! encodes no lossy WebP.

use std::io;

use gafira_doc::contract::{ParamChoice, ParamDef, ParamKind, ParamSchema, ParamValues};
use gafira_doc::msg::MessageKey;
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
            schema: ParamSchema::new(vec![
                scale_param(),
                ParamDef {
                    id: WEBP_QUALITY,
                    label: MessageKey("export.quality"),
                    kind: ParamKind::Choice {
                        options: QUALITIES,
                        default: "high",
                    },
                    animatable: false,
                },
                transparent_param(),
            ]),
            transparency: true,
            encode,
        })
    }
}

/// The option id of the WebP quality (spec 074).
pub const WEBP_QUALITY: &str = "webp_quality";

/// Baja, Media and Alta: the encoder writes only lossless WebP, so the
/// lower qualities keep fewer bits of each colour channel (5, 6), which the
/// lossless coder packs into a smaller file; Alta keeps them all.
const QUALITIES: &[ParamChoice] = &[
    ParamChoice {
        id: "low",
        label: MessageKey("export.quality_low"),
    },
    ParamChoice {
        id: "medium",
        label: MessageKey("export.quality_medium"),
    },
    ParamChoice {
        id: "high",
        label: MessageKey("export.quality_high"),
    },
];

/// `pixels` with `bits` kept of each colour channel, rounded to the
/// nearest level and spread back over 0..=255; alpha untouched.
fn fewer_bits(mut pixels: Vec<u8>, bits: u32) -> Vec<u8> {
    let levels = (1_u32 << bits) - 1;
    for pixel in pixels.chunks_exact_mut(4) {
        for channel in &mut pixel[..3] {
            let level = (u32::from(*channel) * levels + 127) / 255;
            *channel = ((level * 255 + levels / 2) / levels) as u8;
        }
    }
    pixels
}

delegate_exporter!(WebpExporter);

/// The lossless WebP bytes of straight 8-bit sRGB RGBA pixels.
fn encode(
    width: u32,
    height: u32,
    pixels: Vec<u8>,
    options: &ParamValues,
) -> Result<Vec<u8>, ExportError> {
    let pixels = match options.choice(WEBP_QUALITY) {
        Some("low") => fewer_bits(pixels, 5),
        Some("medium") => fewer_bits(pixels, 6),
        _ => pixels,
    };
    let mut out = Vec::new();
    WebPEncoder::new(&mut out)
        .encode(&pixels, width, height, ColorType::Rgba8)
        .map_err(|error| ExportError::Io(io::Error::other(error.to_string())))?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    // Spec 074: fewer bits keep black, white and alpha, and round the rest
    // to the nearest of their levels.
    #[test]
    fn fewer_bits_round_to_their_levels() {
        let out = super::fewer_bits(vec![0, 255, 128, 77, 10, 200, 33, 255], 5);
        assert_eq!(&out[..4], &[0, 255, 132, 77], "alpha untouched");
        assert_eq!(super::fewer_bits(vec![1, 2, 3, 4], 8), vec![1, 2, 3, 4]);
    }
}
