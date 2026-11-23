//! The pixels of an AVIF: its AV1 frames, from their planes to sRGB bytes.

use re_rav1d::dav1d::{Decoder, Error, PixelLayout, PlanarImageComponent as Part, Settings, pixel};

use super::MOST_PIXELS;
use super::container::{Avif, Bad, Cicp, Picture, Turn};

/// A plane of numbers, row by row.
struct Plane {
    w: usize,
    h: usize,
    data: Vec<u16>,
}

impl Plane {
    fn at(&self, x: usize, y: usize) -> f32 {
        f32::from(self.data[y.min(self.h - 1) * self.w + x.min(self.w - 1)])
    }

    /// Between its samples, where they are fewer than those of the image.
    fn between(&self, (x, y): (usize, usize), (across, down): (bool, bool)) -> f32 {
        let place = |v: usize, half: bool, most: usize| {
            if half {
                // Sizes of an image: the casts lose nothing seen.
                let at = ((v as f32 + 0.5) / 2.0 - 0.5).clamp(0.0, (most - 1) as f32);
                (at as usize, at.fract())
            } else {
                (v, 0.0)
            }
        };
        let (x, tx) = place(x, across, self.w);
        let (y, ty) = place(y, down, self.h);
        let mix = |a: f32, b: f32, t: f32| a + (b - a) * t;
        let top = mix(self.at(x, y), self.at(x + 1, y), tx);
        if ty == 0.0 {
            return top;
        }
        mix(top, mix(self.at(x, y + 1), self.at(x + 1, y + 1), tx), ty)
    }
}

/// A frame: its light, and its colours when it has them.
struct Frame {
    light: Plane,
    colors: Option<(Plane, Plane)>,
    /// The colours have half the samples, across and down.
    half: (bool, bool),
    bits: u32,
    cicp: Cicp,
}

fn frame(bytes: &[u8]) -> Result<Frame, Bad> {
    let failed = |_: Error| Bad("not an AV1 frame");
    let mut settings = Settings::new();
    // One frame: it is given as soon as it is ready.
    settings.set_max_frame_delay(1);
    settings.set_frame_size_limit(u32::try_from(MOST_PIXELS).unwrap_or(u32::MAX));
    let mut decoder = Decoder::with_settings(&settings).map_err(failed)?;
    let mut pending = match decoder.send_data(bytes.to_vec(), None, None, None) {
        Ok(()) => false,
        Err(error) if error.is_again() => true,
        Err(error) => return Err(failed(error)),
    };
    let picture = loop {
        match decoder.get_picture() {
            Ok(picture) => break picture,
            Err(error) if error.is_again() && pending => {
                pending = match decoder.send_pending_data() {
                    Ok(()) => false,
                    Err(error) if error.is_again() => true,
                    Err(error) => return Err(failed(error)),
                };
            }
            Err(error) => return Err(failed(error)),
        }
    };
    let (w, h) = (picture.width() as usize, picture.height() as usize);
    if w == 0 || h == 0 {
        return Err(Bad("an empty frame"));
    }
    let layout = picture.pixel_layout();
    let half = match layout {
        PixelLayout::I420 => (true, true),
        PixelLayout::I422 => (true, false),
        PixelLayout::I400 | PixelLayout::I444 => (false, false),
    };
    let wide = picture.bit_depth() > 8;
    let bits = picture.bits_per_component().map_or(8, |b| b.0);
    let plane = |part: Part, (w, h): (usize, usize)| -> Result<Plane, Bad> {
        let bytes = picture.plane(part);
        let stride = picture.stride(part) as usize;
        let mut data = Vec::with_capacity(w * h);
        for row in bytes.chunks(stride.max(1)).take(h) {
            if wide {
                let row = row.get(..w * 2).ok_or(Bad("an odd frame"))?;
                data.extend(
                    row.chunks_exact(2)
                        .map(|v| u16::from_le_bytes([v[0], v[1]])),
                );
            } else {
                let row = row.get(..w).ok_or(Bad("an odd frame"))?;
                data.extend(row.iter().copied().map(u16::from));
            }
        }
        if data.len() != w * h {
            return Err(Bad("an odd frame"));
        }
        Ok(Plane { w, h, data })
    };
    let part = |v: usize, half: bool| if half { v.div_ceil(2) } else { v };
    let small = (part(w, half.0), part(h, half.1));
    let colors = match layout {
        PixelLayout::I400 => None,
        _ => Some((plane(Part::U, small)?, plane(Part::V, small)?)),
    };
    Ok(Frame {
        light: plane(Part::Y, (w, h))?,
        colors,
        half,
        // 8, 10 or 12: the cast is exact.
        bits: bits as u32,
        cicp: Cicp {
            matrix: picture.matrix_coefficients() as u16,
            full: picture.color_range() == pixel::YUVRange::Full,
        },
    })
}

/// From a frame's numbers to 0..=1 of light and −0.5..=0.5 of colour.
struct Scale {
    light: (f32, f32),
    color: (f32, f32),
}

impl Scale {
    fn new(bits: u32, full: bool) -> Self {
        // At most 12 bits: the casts are exact.
        let most = ((1_u32 << bits) - 1) as f32;
        let step = (1_u32 << (bits - 8)) as f32;
        let middle = (1_u32 << (bits - 1)) as f32;
        if full {
            Self {
                light: (0.0, most),
                color: (middle, most),
            }
        } else {
            Self {
                light: (16.0 * step, 219.0 * step),
                color: (middle, 224.0 * step),
            }
        }
    }

    fn light(&self, v: f32) -> f32 {
        (v - self.light.0) / self.light.1
    }

    fn color(&self, v: f32) -> f32 {
        (v - self.color.0) / self.color.1
    }
}

fn byte(v: f32) -> u8 {
    // Clamped: the cast drops the fraction alone.
    (v.clamp(0.0, 1.0) * 255.0 + 0.5) as u8
}

/// The shares of red and blue in the light, by the code of the matrix.
fn shares(matrix: u16) -> (f32, f32) {
    match matrix {
        1 => (0.2126, 0.0722),
        4 => (0.30, 0.11),
        7 => (0.212, 0.087),
        9 | 10 => (0.2627, 0.0593),
        // BT.601, and what is not said.
        _ => (0.299, 0.114),
    }
}

/// The sRGB bytes of a frame, opaque.
fn colors(frame: &Frame, told: Option<Cicp>) -> Vec<u8> {
    let cicp = told.unwrap_or(frame.cicp);
    let scale = Scale::new(frame.bits, cicp.full);
    let (kr, kb) = shares(cicp.matrix);
    let kg = 1.0 - kr - kb;
    let (w, h) = (frame.light.w, frame.light.h);
    let mut out = Vec::with_capacity(w * h * 4);
    for y in 0..h {
        for x in 0..w {
            let light = scale.light(frame.light.at(x, y));
            let rgb = match &frame.colors {
                None => [light; 3],
                Some((u, v)) => {
                    let (u, v) = (u.between((x, y), frame.half), v.between((x, y), frame.half));
                    match cicp.matrix {
                        // The planes are green, blue and red.
                        0 => [scale.light(v), light, scale.light(u)],
                        8 => {
                            let (g, o) = (scale.color(u), scale.color(v));
                            [light - g + o, light + g, light - g - o]
                        }
                        _ => {
                            let (u, v) = (scale.color(u), scale.color(v));
                            let r = light + 2.0 * (1.0 - kr) * v;
                            let b = light + 2.0 * (1.0 - kb) * u;
                            [r, (light - kr * r - kb * b) / kg, b]
                        }
                    }
                }
            };
            out.extend(rgb.map(byte));
            out.push(255);
        }
    }
    out
}

/// A picture of `n` bytes for each pixel.
struct Pixels {
    w: usize,
    h: usize,
    data: Vec<u8>,
}

/// A picture, its pieces side by side, each made into `n` bytes a pixel.
fn pieced(picture: &Picture, n: usize, make: impl Fn(&Frame) -> Vec<u8>) -> Result<Pixels, Bad> {
    let first = frame(&picture.tiles[0])?;
    let (tw, th) = (first.light.w, first.light.h);
    let rows = picture.tiles.len() / picture.columns.max(1);
    let (w, h) = if picture.tiles.len() == 1 {
        (tw, th)
    } else {
        let (w, h) = picture.size.ok_or(Bad("no size"))?;
        (w as usize, h as usize)
    };
    let room = (tw as u64 * picture.columns as u64, th as u64 * rows as u64);
    if w as u64 * h as u64 > MOST_PIXELS || room.0 < w as u64 || room.1 < h as u64 {
        return Err(Bad("an odd mosaic"));
    }
    if picture.tiles.len() == 1 {
        return Ok(Pixels {
            w,
            h,
            data: make(&first),
        });
    }
    let mut data = vec![0; w * h * n];
    for (at, bytes) in picture.tiles.iter().enumerate() {
        let tile = if at == 0 { None } else { Some(frame(bytes)?) };
        let tile = tile.as_ref().unwrap_or(&first);
        if (tile.light.w, tile.light.h) != (tw, th) {
            return Err(Bad("an odd mosaic"));
        }
        let pixels = make(tile);
        let (x, y) = (at % picture.columns * tw, at / picture.columns * th);
        if x >= w || y >= h {
            continue;
        }
        let shown = tw.min(w - x) * n;
        for (row, line) in pixels.chunks_exact(tw * n).enumerate().take(h - y) {
            let to = ((y + row) * w + x) * n;
            data[to..to + shown].copy_from_slice(&line[..shown]);
        }
    }
    Ok(Pixels { w, h, data })
}

fn turned(p: Pixels, turn: Turn) -> Pixels {
    let (w, h) = (p.w, p.h);
    let size = match turn {
        Turn::Quarters(n) if n % 2 == 1 => (h, w),
        _ => (w, h),
    };
    let mut data = Vec::with_capacity(p.data.len());
    for y in 0..size.1 {
        for x in 0..size.0 {
            let (sx, sy) = match turn {
                Turn::Quarters(0) => (x, y),
                Turn::Quarters(1) => (w - 1 - y, x),
                Turn::Quarters(2) => (w - 1 - x, h - 1 - y),
                Turn::Quarters(_) => (y, h - 1 - x),
                Turn::Upside => (x, h - 1 - y),
                Turn::Sideways => (w - 1 - x, y),
            };
            let at = (sy * w + sx) * 4;
            data.extend_from_slice(&p.data[at..at + 4]);
        }
    }
    Pixels {
        w: size.0,
        h: size.1,
        data,
    }
}

/// The straight sRGB bytes of what `avif` shows, and their size.
pub(super) fn rgba(avif: &Avif) -> Result<((u32, u32), Vec<u8>), Bad> {
    let mut shown = pieced(&avif.color, 4, |frame| colors(frame, avif.color.cicp))?;
    if let Some(alpha) = &avif.alpha {
        let alpha = pieced(alpha, 1, |frame| {
            let full = alpha.cicp.unwrap_or(frame.cicp).full;
            let scale = Scale::new(frame.bits, full);
            let each = frame.light.data.iter();
            each.map(|v| byte(scale.light(f32::from(*v)))).collect()
        })?;
        if (alpha.w, alpha.h) != (shown.w, shown.h) {
            return Err(Bad("an odd opacity"));
        }
        for (px, a) in shown.data.chunks_exact_mut(4).zip(&alpha.data) {
            if avif.premultiplied && *a > 0 {
                for c in &mut px[..3] {
                    let v = u32::from(*c) * 255 / u32::from(*a);
                    *c = u8::try_from(v).unwrap_or(u8::MAX);
                }
            }
            px[3] = *a;
        }
    }
    for turn in &avif.turns {
        shown = turned(shown, *turn);
    }
    let size = |v: usize| u32::try_from(v).map_err(|_| Bad("too large"));
    Ok(((size(shown.w)?, size(shown.h)?), shown.data))
}
