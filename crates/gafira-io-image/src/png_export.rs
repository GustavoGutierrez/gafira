//! The PNG exporter (FD-55, spec 030).

use std::io;

use gafira_doc::contract::{ParamChoice, ParamDef, ParamKind, ParamSchema, ParamValues};
use gafira_doc::msg::MessageKey;
use gafira_ports::common::LegalClass;
use gafira_ports::export::ExportError;
use gafira_ports::format::{ByteSignature, FormatDescriptor, FormatKind};

use crate::raster::{Raster, delegate_exporter, scale_param, transparent_param};

/// The PNG format (FD-55).
pub const PNG_FORMAT: FormatDescriptor = FormatDescriptor {
    id: "png",
    extensions: &["png"],
    mime: &["image/png"],
    signatures: &[ByteSignature {
        offset: 0,
        bytes: b"\x89PNG\r\n\x1a\n",
    }],
    kind: FormatKind::Raster,
    legal: LegalClass::RoyaltyFree,
};

/// The PNG exporter: one 8-bit sRGB RGBA file per page of the scope, at a
/// scale of 0.5× to 4×, on the page background or transparent (FD-55).
pub struct PngExporter(Raster);

impl Default for PngExporter {
    fn default() -> Self {
        Self::new()
    }
}

impl PngExporter {
    /// The exporter with the options of FD-55 this cut has.
    #[must_use]
    pub fn new() -> Self {
        Self(Raster {
            format: &PNG_FORMAT,
            schema: ParamSchema::new(vec![
                scale_param(),
                ParamDef {
                    id: PNG_QUALITY,
                    label: MessageKey("export.quality"),
                    kind: ParamKind::Choice {
                        options: PNG_QUALITIES,
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

/// The option id of the PNG quality (spec 074): «Alta» or «Comprimir».
pub const PNG_QUALITY: &str = "png_quality";

/// The PNG qualities: both lossless; «Comprimir» spends more time for a
/// smaller file.
const PNG_QUALITIES: &[ParamChoice] = &[
    ParamChoice {
        id: "high",
        label: MessageKey("export.quality_high"),
    },
    ParamChoice {
        id: "compress",
        label: MessageKey("export.quality_compress"),
    },
];

delegate_exporter!(PngExporter);

/// The PNG bytes of straight 8-bit sRGB RGBA pixels, with the `sRGB` chunk
/// (FD-55, «Perfil sRGB incrustado»).
fn encode(
    width: u32,
    height: u32,
    pixels: Vec<u8>,
    options: &ParamValues,
) -> Result<Vec<u8>, ExportError> {
    let mut out = Vec::new();
    let mut encoder = png::Encoder::new(&mut out, width, height);
    // «Comprimir»: the strongest lossless compression (spec 074).
    if options.choice(PNG_QUALITY) == Some("compress") {
        encoder.set_compression(png::Compression::High);
    } else {
        encoder.set_compression(png::Compression::Fast);
    }
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.set_source_srgb(png::SrgbRenderingIntent::Perceptual);
    let mut writer = encoder.write_header().map_err(png_error)?;
    writer.write_image_data(&pixels).map_err(png_error)?;
    writer.finish().map_err(png_error)?;
    Ok(out)
}

/// An encoder error as the I/O error of the export.
fn png_error(error: png::EncodingError) -> ExportError {
    match error {
        png::EncodingError::IoError(error) => ExportError::Io(error),
        other => ExportError::Io(io::Error::other(other.to_string())),
    }
}
