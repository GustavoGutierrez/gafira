//! Pixel goldens (ARQ-113, ADR 0010): PNG read and write, a pure comparator and
//! the candidate output used when a golden fails.
//!
//! A pixel criterion uses an analytic value first. A stored golden is the
//! second choice, used only when no closed form exists. For `f32` output,
//! [`quantize`] first turns the buffer into bytes with `round(c * 255)` on
//! `[0, 1]`.

use std::fmt;

/// Largest stored golden, per axis, in pixels.
pub const MAX_SIDE: u32 = 256;

/// Bytes per pixel of a golden.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Channels {
    /// One 8-bit channel (a mask).
    Gray,
    /// Four 8-bit channels, stored verbatim (premultiplied values included).
    Rgba,
}

/// An 8-bit image buffer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Image {
    width: u32,
    height: u32,
    channels: Channels,
    bytes: Vec<u8>,
}

/// Why a golden operation failed.
#[derive(Debug)]
pub enum GoldenError {
    /// The byte length does not match `width * height * channels`.
    BadLength,
    /// A side is zero or above [`MAX_SIDE`].
    BadSize { width: u32, height: u32 },
    /// The PNG is not 8-bit grayscale or RGBA, or it is malformed.
    BadPng(String),
}

impl fmt::Display for GoldenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BadLength => write!(f, "buffer length does not match the dimensions"),
            Self::BadSize { width, height } => {
                write!(f, "golden size {width}x{height} is outside 1..={MAX_SIDE}")
            }
            Self::BadPng(why) => write!(f, "unsupported or malformed PNG: {why}"),
        }
    }
}

impl std::error::Error for GoldenError {}

impl Channels {
    const fn count(self) -> usize {
        match self {
            Self::Gray => 1,
            Self::Rgba => 4,
        }
    }
}

fn check_size(width: u32, height: u32) -> Result<(), GoldenError> {
    if (1..=MAX_SIDE).contains(&width) && (1..=MAX_SIDE).contains(&height) {
        Ok(())
    } else {
        Err(GoldenError::BadSize { width, height })
    }
}

impl Image {
    /// Builds an image from `width`, `height`, `channels` and the bytes.
    pub fn new(
        width: u32,
        height: u32,
        channels: Channels,
        bytes: &[u8],
    ) -> Result<Self, GoldenError> {
        let pixels = u64::from(width) * u64::from(height);
        if u64::try_from(bytes.len()) != Ok(pixels * channels.count() as u64) {
            return Err(GoldenError::BadLength);
        }
        Ok(Self {
            width,
            height,
            channels,
            bytes: bytes.to_vec(),
        })
    }

    /// Width in pixels.
    pub fn width(&self) -> u32 {
        self.width
    }

    /// Height in pixels.
    pub fn height(&self) -> u32 {
        self.height
    }

    /// Channel layout.
    pub fn channels(&self) -> Channels {
        self.channels
    }

    /// The channel bytes, row-major, as stored.
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}

/// Encodes an image as an 8-bit PNG with no colour chunks.
pub fn encode_png(image: &Image) -> Result<Vec<u8>, GoldenError> {
    check_size(image.width, image.height)?;
    let mut out = Vec::new();
    let mut encoder = png::Encoder::new(&mut out, image.width, image.height);
    encoder.set_color(match image.channels {
        Channels::Gray => png::ColorType::Grayscale,
        Channels::Rgba => png::ColorType::Rgba,
    });
    encoder.set_depth(png::BitDepth::Eight);
    let bad = |e: png::EncodingError| GoldenError::BadPng(e.to_string());
    let mut writer = encoder.write_header().map_err(bad)?;
    writer.write_image_data(&image.bytes).map_err(bad)?;
    writer.finish().map_err(bad)?;
    Ok(out)
}

/// Decodes an 8-bit grayscale or RGBA PNG without any transformation.
pub fn decode_png(png: &[u8]) -> Result<Image, GoldenError> {
    let bad = |e: png::DecodingError| GoldenError::BadPng(e.to_string());
    let mut decoder = png::Decoder::new(std::io::Cursor::new(png));
    decoder.set_transformations(png::Transformations::IDENTITY);
    let mut reader = decoder.read_info().map_err(bad)?;
    let info = reader.info();
    let (width, height) = (info.width, info.height);
    check_size(width, height)?;
    let channels = match (info.color_type, info.bit_depth) {
        (png::ColorType::Grayscale, png::BitDepth::Eight) => Channels::Gray,
        (png::ColorType::Rgba, png::BitDepth::Eight) => Channels::Rgba,
        (color, depth) => return Err(GoldenError::BadPng(format!("{color:?} at {depth:?}"))),
    };
    let mut bytes = vec![
        0;
        reader
            .output_buffer_size()
            .ok_or_else(|| GoldenError::BadPng("size overflow".into()))?
    ];
    let frame = reader.next_frame(&mut bytes).map_err(bad)?;
    bytes.truncate(frame.buffer_size());
    Image::new(width, height, channels, &bytes)
}

/// How strict a comparison is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// Every byte equal.
    Exact,
    /// At most `floor(pixels / 1000)` pixels differ by more than 1/255 on any channel.
    CrossPlatform,
}

/// Why two images do not match.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mismatch {
    /// Width or height differ.
    Size,
    /// The channel layout differs.
    Channels,
    /// Pixels differ beyond what the mode allows.
    Pixels { differing: usize, allowed: usize },
}

/// The mode for a target: exact on `x86_64-unknown-linux-gnu`, tolerant elsewhere.
pub fn mode_for(arch: &str, os: &str, env: &str) -> Mode {
    if (arch, os, env) == ("x86_64", "linux", "gnu") {
        Mode::Exact
    } else {
        Mode::CrossPlatform
    }
}

/// The mode for the target this crate was built for.
pub fn current_mode() -> Mode {
    let env = if cfg!(target_env = "gnu") {
        "gnu"
    } else {
        "other"
    };
    mode_for(std::env::consts::ARCH, std::env::consts::OS, env)
}

/// Compares `actual` with the `expected` golden under `mode`.
pub fn compare(mode: Mode, expected: &Image, actual: &Image) -> Result<(), Mismatch> {
    if (expected.width, expected.height) != (actual.width, actual.height) {
        return Err(Mismatch::Size);
    }
    if expected.channels != actual.channels {
        return Err(Mismatch::Channels);
    }
    let step = expected.channels.count();
    let pixels = expected
        .bytes
        .chunks_exact(step)
        .zip(actual.bytes.chunks_exact(step));
    let (differing, allowed) = match mode {
        Mode::Exact => (pixels.filter(|(e, a)| e != a).count(), 0),
        Mode::CrossPlatform => (
            pixels
                .filter(|(e, a)| e.iter().zip(*a).any(|(x, y)| x.abs_diff(*y) > 1))
                .count(),
            expected.bytes.len() / step / 1000,
        ),
    };
    if differing > allowed {
        Err(Mismatch::Pixels { differing, allowed })
    } else {
        Ok(())
    }
}
