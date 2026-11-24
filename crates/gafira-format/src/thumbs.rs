//! The `thumbs/` folder of a `.gaf` (02 §7.1, D-68, spec 033): one
//! `thumbs/<page_id>.webp` per page that has a thumbnail; this build writes
//! the first page's only. Each one is a lossless WebP of straight 8-bit
//! sRGB RGBA, at most `THUMBNAIL_SIDE` on each side.

use std::io::{Cursor, Read, Seek};

use gafira_doc::PageId;
use gafira_ports::repository::{RepositoryError, THUMBNAIL_SIDE, TemplateMeta, Thumbnail};
use gafira_raster::straight_srgb8;
use image_webp::{ColorType, WebPDecoder, WebPEncoder};
use serde_json::Value;
use zip::ZipArchive;
use zip::result::ZipError;

use crate::read::{check_mimetype, corrupt, json, zip_error};
use crate::{FORMAT, Limits, manifest};

/// The folder of the thumbnails.
pub(crate) const FOLDER: &str = "thumbs/";

/// What every thumbnail's name ends with.
const EXTENSION: &str = ".webp";

/// A thumbnail as a reader gets it: straight (not premultiplied) 8-bit
/// sRGB RGBA, row by row (spec 033).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ThumbnailImage {
    pub width: u32,
    pub height: u32,
    /// `width × height × 4` bytes.
    pub rgba: Vec<u8>,
}

/// The entry of the thumbnail of `page`.
pub(crate) fn name(page: PageId) -> String {
    format!("{FOLDER}{page}{EXTENSION}")
}

/// The page id an entry under `thumbs/` names: `thumbs/<digits>.webp`
/// and nothing else.
pub(crate) fn parse(name: &str) -> Option<u64> {
    let id = name.strip_prefix(FOLDER)?.strip_suffix(EXTENSION)?;
    if id.is_empty() || !id.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    id.parse().ok()
}

/// Whether a thumbnail of `width` × `height` may be in a file.
fn fits(width: u32, height: u32) -> bool {
    (1..=THUMBNAIL_SIDE).contains(&width) && (1..=THUMBNAIL_SIDE).contains(&height)
}

/// The lossless WebP of `thumbnail`, or `None` when it cannot be one: a
/// mask, an empty image or a side above [`THUMBNAIL_SIDE`].
pub(crate) fn encode(thumbnail: &Thumbnail) -> Option<Vec<u8>> {
    let image = &thumbnail.image;
    if !fits(image.width(), image.height()) {
        return None;
    }
    let pixels = straight_srgb8(image)?;
    let mut out = Vec::new();
    WebPEncoder::new(&mut out)
        .encode(&pixels, image.width(), image.height(), ColorType::Rgba8)
        .ok()?;
    Some(out)
}

/// The pixels of the WebP `bytes`; anything but one still picture of
/// 1 to [`THUMBNAIL_SIDE`] pixels a side is corrupt.
fn decode(bytes: &[u8]) -> Result<ThumbnailImage, RepositoryError> {
    let bad = |detail: String| corrupt(format!("thumbnail: {detail}"));
    let mut decoder = WebPDecoder::new(Cursor::new(bytes)).map_err(|e| bad(e.to_string()))?;
    if decoder.is_animated() {
        return Err(bad("an animation".to_owned()));
    }
    let (width, height) = decoder.dimensions();
    if !fits(width, height) {
        return Err(bad(format!("{width} × {height} px")));
    }
    let size = decoder
        .output_buffer_size()
        .ok_or_else(|| bad("too large".to_owned()))?;
    let mut pixels = vec![0; size];
    decoder
        .read_image(&mut pixels)
        .map_err(|e| bad(e.to_string()))?;
    let rgba = if decoder.has_alpha() {
        pixels
    } else {
        pixels
            .chunks_exact(3)
            .flat_map(|p| [p[0], p[1], p[2], u8::MAX])
            .collect()
    };
    Ok(ThumbnailImage {
        width,
        height,
        rgba,
    })
}

/// The thumbnail entry `name` is corrupt when it holds more than `max`.
fn too_large(name: &str, max: u64) -> RepositoryError {
    corrupt(format!("{name} is larger than {max} bytes"))
}

/// Read the thumbnail of the first page of the `.gaf` in `source`, without
/// the rest of the project (03 §7.2, spec 033): only the `mimetype`, the
/// manifest and `thumbs/<first page id>.webp` are read, with the same
/// hostile-file rules as [`crate::read_gaf`].
///
/// `None` when the manifest lists no page or the file has no thumbnail of
/// the first one.
///
/// # Errors
/// [`RepositoryError::Corrupt`] when the file is not a `.gaf` within
/// `limits` or the thumbnail is not a WebP of at most `THUMBNAIL_SIDE` a
/// side; [`RepositoryError::Io`] when the source fails.
pub fn read_thumbnail<R: Read + Seek>(
    source: R,
    limits: Limits,
) -> Result<Option<ThumbnailImage>, RepositoryError> {
    let (mut zip, manifest) = open(source, limits)?;
    first_thumbnail(&mut zip, &manifest, limits)
}

/// A template as a catalogue lists it (spec 091): its `template` and the
/// thumbnail of its first page, if it has one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TemplateInfo {
    pub meta: TemplateMeta,
    pub thumbnail: Option<ThumbnailImage>,
}

/// The template the `.gaf` in `source` is, read without the rest of the
/// project, as [`read_thumbnail`] reads: `None` when its manifest has no
/// well-formed `template` (D-81), so it is an ordinary project.
///
/// # Errors
/// Those of [`read_thumbnail`].
pub fn read_template_info<R: Read + Seek>(
    source: R,
    limits: Limits,
) -> Result<Option<TemplateInfo>, RepositoryError> {
    let (mut zip, manifest) = open(source, limits)?;
    let Some(meta) = manifest::template(&manifest) else {
        return Ok(None);
    };
    let thumbnail = first_thumbnail(&mut zip, &manifest, limits)?;
    Ok(Some(TemplateInfo { meta, thumbnail }))
}

/// The archive in `source` and its manifest, checked as a `.gaf` within
/// `limits`.
fn open<R: Read + Seek>(
    source: R,
    limits: Limits,
) -> Result<(ZipArchive<R>, Value), RepositoryError> {
    let mut zip = ZipArchive::new(source).map_err(zip_error)?;
    if zip.len() > limits.max_entries {
        return Err(corrupt(format!(
            "{} entries, more than {}",
            zip.len(),
            limits.max_entries
        )));
    }
    check_mimetype(&mut zip)?;
    let manifest = json(&mut zip, "manifest.json", limits)?;
    if manifest.get("format").and_then(Value::as_str) != Some(FORMAT) {
        return Err(corrupt("the manifest is not a gafira-project"));
    }
    Ok((zip, manifest))
}

/// The thumbnail of the first page the manifest lists, if the file has it.
fn first_thumbnail<R: Read + Seek>(
    zip: &mut ZipArchive<R>,
    manifest: &Value,
    limits: Limits,
) -> Result<Option<ThumbnailImage>, RepositoryError> {
    let pages = manifest
        .get("pages")
        .and_then(Value::as_array)
        .ok_or_else(|| corrupt("the manifest lists no pages"))?;
    let Some(first) = pages.first() else {
        return Ok(None);
    };
    let id = first
        .as_u64()
        .ok_or_else(|| corrupt("a pages id is not a number"))?;
    let name = name(PageId::new(id));
    let entry = match zip.by_name(&name) {
        Ok(entry) => entry,
        Err(ZipError::FileNotFound) => return Ok(None),
        Err(other) => return Err(zip_error(other)),
    };
    if entry.size() > limits.max_thumb_bytes {
        return Err(too_large(&name, limits.max_thumb_bytes));
    }
    let mut bytes = Vec::new();
    entry
        .take(limits.max_thumb_bytes + 1)
        .read_to_end(&mut bytes)
        .map_err(RepositoryError::Io)?;
    if bytes.len() as u64 > limits.max_thumb_bytes {
        return Err(too_large(&name, limits.max_thumb_bytes));
    }
    decode(&bytes).map(Some)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn s33_03_only_digits_dot_webp_name_a_thumbnail() {
        assert_eq!(parse("thumbs/1.webp"), Some(1));
        assert_eq!(parse(&name(PageId::new(42))), Some(42));
        for bad in [
            "thumbs/.webp",
            "thumbs/+1.webp",
            "thumbs/-1.webp",
            "thumbs/1.WEBP",
            "thumbs/1.png",
            "thumbs/a/1.webp",
            "thumbs/99999999999999999999.webp",
            "pages/1.webp",
        ] {
            assert_eq!(parse(bad), None, "{bad}");
        }
    }

    #[test]
    fn s33_02_sizes_from_one_to_the_side_fit() {
        assert!(fits(1, 1));
        assert!(fits(THUMBNAIL_SIDE, THUMBNAIL_SIDE));
        assert!(!fits(0, 1));
        assert!(!fits(1, 0));
        assert!(!fits(THUMBNAIL_SIDE + 1, 1));
        assert!(!fits(1, THUMBNAIL_SIDE + 1));
    }
}
