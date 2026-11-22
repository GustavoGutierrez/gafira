//! The JPEG exporter (FD-55, spec 039).

use std::io;

use gafira_doc::contract::{ParamChoice, ParamDef, ParamKind, ParamSchema, ParamValues};
use gafira_doc::msg::MessageKey;
use gafira_ports::common::LegalClass;
use gafira_ports::export::ExportError;
use gafira_ports::format::{ByteSignature, FormatDescriptor, FormatKind};
use image::ExtendedColorType;
use image::codecs::jpeg::JpegEncoder;

use crate::raster::{Raster, delegate_exporter, scale_param};

/// The option id of the JPEG quality (FD-55, «Calidad»; spec 074: Baja,
/// Media or Alta).
pub const QUALITY: &str = "quality";

/// Baja, Media and Alta, and the JPEG quality each one writes.
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

/// The JPEG quality of a choice of [`QUALITIES`].
#[must_use]
pub fn jpeg_quality(choice: &str) -> u8 {
    match choice {
        "low" => 60,
        "medium" => 80,
        _ => 92,
    }
}

/// The JPEG format (FD-55).
pub const JPEG_FORMAT: FormatDescriptor = FormatDescriptor {
    id: "jpeg",
    extensions: &["jpg", "jpeg"],
    mime: &["image/jpeg"],
    signatures: &[ByteSignature {
        offset: 0,
        bytes: &[0xFF, 0xD8, 0xFF],
    }],
    kind: FormatKind::Raster,
    legal: LegalClass::RoyaltyFree,
};

/// The JPEG exporter: one 8-bit sRGB file per page of the scope, at a
/// scale of 0.5× to 4× and a quality of 1 to 100. JPEG has no alpha: the
/// page is drawn on its background and what is left translucent is
/// composed over white (FD-55).
pub struct JpegExporter(Raster);

impl Default for JpegExporter {
    fn default() -> Self {
        Self::new()
    }
}

impl JpegExporter {
    /// The exporter with the options of FD-55 this cut has.
    #[must_use]
    pub fn new() -> Self {
        Self(Raster {
            format: &JPEG_FORMAT,
            schema: ParamSchema::new(vec![
                scale_param(),
                ParamDef {
                    id: QUALITY,
                    label: MessageKey("export.quality"),
                    kind: ParamKind::Choice {
                        options: QUALITIES,
                        default: "high",
                    },
                    animatable: false,
                },
            ]),
            transparency: false,
            encode,
        })
    }
}

delegate_exporter!(JpegExporter);

/// Straight 8-bit RGBA pixels composed over white, as RGB: each channel is
/// `c·a + 255·(1 − a)`, rounded.
fn over_white(rgba: &[u8]) -> Vec<u8> {
    let mut rgb = Vec::with_capacity(rgba.len() / 4 * 3);
    for pixel in rgba.chunks_exact(4) {
        let alpha = u32::from(pixel[3]);
        for channel in &pixel[..3] {
            let mixed = u32::from(*channel) * alpha + 255 * (255 - alpha);
            // In range: `mixed` is at most 255 · 255, so the quotient is at
            // most 255.
            rgb.push(((mixed + 127) / 255) as u8);
        }
    }
    rgb
}

/// The JPEG bytes of straight 8-bit sRGB RGBA pixels, at the quality of
/// `options`.
fn encode(
    width: u32,
    height: u32,
    pixels: Vec<u8>,
    options: &ParamValues,
) -> Result<Vec<u8>, ExportError> {
    let quality = jpeg_quality(options.choice(QUALITY).unwrap_or("high"));
    let mut out = Vec::new();
    JpegEncoder::new_with_quality(&mut out, quality)
        .encode(&over_white(&pixels), width, height, ExtendedColorType::Rgb8)
        .map_err(|error| ExportError::Io(io::Error::other(error.to_string())))?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    // S39-01: opaque pixels keep their colour, clear ones are white and a
    // half transparent black is the grey between.
    #[test]
    fn s39_01_pixels_are_composed_over_white() {
        let rgba = [
            200, 100, 50, 255, // opaque
            10, 20, 30, 0, // clear
            0, 0, 0, 128, // half black
            255, 0, 0, 64, // a quarter red
        ];
        assert_eq!(
            over_white(&rgba),
            [200, 100, 50, 255, 255, 255, 127, 127, 127, 255, 191, 191]
        );
    }

    // S39-01: a partial pixel at the end is not read.
    #[test]
    fn s39_01_only_whole_pixels_are_composed() {
        assert_eq!(over_white(&[1, 2, 3, 255, 9, 9]), [1, 2, 3]);
        assert!(over_white(&[]).is_empty());
    }
}
