//! The AVIF exporter (spec 087, FD-55).

use std::io;

use gafira_doc::contract::{ParamChoice, ParamDef, ParamKind, ParamSchema, ParamValues};
use gafira_doc::msg::MessageKey;
use gafira_ports::common::LegalClass;
use gafira_ports::export::ExportError;
use gafira_ports::format::{ByteSignature, FormatDescriptor, FormatKind};
use ravif::{Encoder, Img, RGBA8};

use crate::raster::{Raster, delegate_exporter, scale_param, transparent_param};

/// The option id of the AVIF quality: Baja, Media or Alta.
pub const AVIF_QUALITY: &str = "avif_quality";

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

/// The AVIF format.
pub const AVIF_FORMAT: FormatDescriptor = FormatDescriptor {
    id: "avif",
    extensions: &["avif"],
    mime: &["image/avif"],
    signatures: &[ByteSignature {
        offset: 4,
        bytes: b"ftypavif",
    }],
    kind: FormatKind::Raster,
    legal: LegalClass::RoyaltyFree,
};

/// The AVIF exporter: one lossy file per page of the scope, at a scale of
/// 0.5× to 4×, on the page background or transparent.
pub struct AvifExporter(Raster);

impl Default for AvifExporter {
    fn default() -> Self {
        Self::new()
    }
}

impl AvifExporter {
    #[must_use]
    pub fn new() -> Self {
        Self(Raster {
            format: &AVIF_FORMAT,
            schema: ParamSchema::new(vec![
                scale_param(),
                ParamDef {
                    id: AVIF_QUALITY,
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

delegate_exporter!(AvifExporter);

/// How hard it tries, from 1 to 10: a page takes seconds, not minutes.
const SPEED: u8 = 8;

/// The AVIF bytes of straight 8-bit sRGB RGBA pixels.
pub(super) fn encode(
    width: u32,
    height: u32,
    pixels: Vec<u8>,
    options: &ParamValues,
) -> Result<Vec<u8>, ExportError> {
    let quality = match options.choice(AVIF_QUALITY) {
        Some("low") => 50.0,
        Some("medium") => 70.0,
        _ => 88.0,
    };
    let each: Vec<RGBA8> = pixels
        .chunks_exact(4)
        .map(|p| RGBA8::new(p[0], p[1], p[2], p[3]))
        .collect();
    if width == 0 || height == 0 || each.len() != width as usize * height as usize {
        return Err(crate::raster::invalid("no pixels"));
    }
    let image = Img::new(each.as_slice(), width as usize, height as usize);
    Encoder::new()
        .with_quality(quality)
        .with_alpha_quality(quality)
        .with_speed(SPEED)
        .encode_rgba(image)
        .map(|made| made.avif_file)
        .map_err(|error| ExportError::Io(io::Error::other(error.to_string())))
}
