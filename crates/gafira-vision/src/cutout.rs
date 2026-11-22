//! Taking the background away with a model that finds the subject: the image
//! goes in at the model's size, and its opacity comes back at the image's,
//! fitted to its edges.

use std::sync::Arc;

use gafira_doc::contract::{CancelToken, Progress};
use gafira_ports::vision::{BackgroundRemover, Pixels, VisionError};
use gafira_raster::AlphaMask;
use image::imageops::FilterType;
use image::{ImageBuffer, Luma, Rgb, RgbImage, imageops};

use crate::runtime::Session;

/// A plane of numbers, 0 to 1.
type Plane = ImageBuffer<Luma<f32>, Vec<f32>>;

/// The longest side the edges are fitted at, in px.
const FITTED: u32 = 1024;

pub(crate) struct Cutter {
    pub session: Arc<dyn Session>,
    pub side: usize,
    pub mean: [f32; 3],
    pub deviation: [f32; 3],
}

fn cancelled(cancel: &CancelToken) -> Result<(), VisionError> {
    cancel.check().map_err(|_| VisionError::Cancelled)
}

/// The colours of `image`, over white where it is see-through.
fn opaque(image: &Pixels) -> RgbImage {
    let (w, h) = image.size();
    let over = |c: u8, a: u8| {
        let (c, a) = (u16::from(c), u16::from(a));
        // At most 255: the cast is exact.
        ((c * a + 255 * (255 - a) + 127) / 255) as u8
    };
    RgbImage::from_fn(w, h, |x, y| {
        let at = (y as usize * w as usize + x as usize) * 4;
        let p = &image.rgba()[at..at + 4];
        Rgb([over(p[0], p[3]), over(p[1], p[3]), over(p[2], p[3])])
    })
}

/// What goes in: the channels one after the other, over the brightest of
/// them, less the mean and over the deviation.
fn input(rgb: &RgbImage, side: usize, mean: [f32; 3], deviation: [f32; 3]) -> Vec<f32> {
    // A model's side is small: the cast is exact.
    let side = side as u32;
    let small = imageops::resize(rgb, side, side, FilterType::Triangle);
    let most = small.as_raw().iter().copied().max().unwrap_or(255).max(1);
    let most = f32::from(most);
    (0..3)
        .flat_map(|c| {
            small
                .pixels()
                .map(move |p| (f32::from(p.0[c]) / most - mean[c]) / deviation[c])
                .collect::<Vec<f32>>()
        })
        .collect()
}

/// The opacity a model gives, from 0 to 1; nothing at all when it sees no
/// subject.
fn stretched(values: &[f32]) -> Vec<f32> {
    let (min, max) = values
        .iter()
        .filter(|v| v.is_finite())
        .fold((f32::INFINITY, f32::NEG_INFINITY), |(a, b), v| {
            (a.min(*v), b.max(*v))
        });
    let room = max - min;
    if !room.is_finite() || room < 1e-3 {
        return vec![0.0; values.len()];
    }
    values
        .iter()
        .map(|v| if v.is_finite() { (v - min) / room } else { 0.0 })
        .collect()
}

/// The means around each place, `radius` away, the edges as they are.
fn boxed(plane: &[f32], w: usize, h: usize, radius: usize) -> Vec<f32> {
    let line = |src: &dyn Fn(usize) -> f32, n: usize, put: &mut dyn FnMut(usize, f32)| {
        let last = n - 1;
        // Small counts: the casts are exact.
        let mut sum: f32 = src(0) * radius as f32;
        for i in 0..=radius {
            sum += src(i.min(last));
        }
        let count = (2 * radius + 1) as f32;
        for i in 0..n {
            put(i, sum / count);
            sum += src((i + radius + 1).min(last)) - src(i.saturating_sub(radius));
        }
    };
    let mut across = vec![0.0; w * h];
    for y in 0..h {
        let row = &plane[y * w..(y + 1) * w];
        line(&|x| row[x], w, &mut |x, v| across[y * w + x] = v);
    }
    let mut out = vec![0.0; w * h];
    for x in 0..w {
        line(&|y| across[y * w + x], h, &mut |y, v| out[y * w + x] = v);
    }
    out
}

/// `mask` with its edges where the ones of `light` are: a guided filter (He,
/// Sun and Tang), its lines as they are at this size.
fn guided(light: &[f32], mask: &[f32], w: usize, h: usize, radius: usize) -> (Vec<f32>, Vec<f32>) {
    const EVEN: f32 = 2e-4;
    let each = |f: &dyn Fn(usize) -> f32| -> Vec<f32> { (0..w * h).map(f).collect() };
    let mean = |plane: &[f32]| boxed(plane, w, h, radius);
    let (mean_l, mean_m) = (mean(light), mean(mask));
    let product = mean(&each(&|i| light[i] * mask[i]));
    let square = mean(&each(&|i| light[i] * light[i]));
    let slope = each(&|i| {
        let covariance = product[i] - mean_l[i] * mean_m[i];
        let variance = square[i] - mean_l[i] * mean_l[i];
        covariance / (variance.max(0.0) + EVEN)
    });
    let base = each(&|i| mean_m[i] - slope[i] * mean_l[i]);
    (mean(&slope), mean(&base))
}

fn plane(w: u32, h: u32, values: Vec<f32>) -> Result<Plane, VisionError> {
    Plane::from_raw(w, h, values).ok_or_else(|| VisionError::Model("an odd output".to_owned()))
}

fn light(rgb: &RgbImage) -> Vec<f32> {
    rgb.pixels()
        .map(|p| {
            let [r, g, b] = p.0.map(f32::from);
            (0.299 * r + 0.587 * g + 0.114 * b) / 255.0
        })
        .collect()
}

/// The opacity `mask` of `side` × `side` at the size of `rgb`, its edges
/// fitted.
fn fitted(rgb: &RgbImage, mask: Vec<f32>, side: usize) -> Result<Vec<u8>, VisionError> {
    let (w, h) = rgb.dimensions();
    let mask = plane(side as u32, side as u32, mask)?;
    // Fitted at a size that is enough; then both lines are drawn out.
    let k = (f64::from(FITTED) / f64::from(w.max(h))).min(1.0);
    let sized = |v: u32| ((f64::from(v) * k).round() as u32).max(1);
    let (sw, sh) = (sized(w), sized(h));
    let small = imageops::resize(rgb, sw, sh, FilterType::Triangle);
    let wide = imageops::resize(&mask, sw, sh, FilterType::Triangle);
    // As far as a pixel of the model reaches.
    let radius = (sw.max(sh) as usize / side).max(1) * 3;
    let (sw_, sh_) = (sw as usize, sh as usize);
    let (slope, base) = guided(&light(&small), wide.as_raw(), sw_, sh_, radius);
    let slope = imageops::resize(&plane(sw, sh, slope)?, w, h, FilterType::Triangle);
    let base = imageops::resize(&plane(sw, sh, base)?, w, h, FilterType::Triangle);
    let soft = imageops::resize(&wide, w, h, FilterType::Triangle);
    let lines = slope.as_raw().iter().zip(base.as_raw());
    Ok(light(rgb)
        .iter()
        .zip(lines)
        .zip(soft.as_raw())
        .map(|((l, (a, b)), soft)| {
            // Where the model is sure, it is right.
            let sure = ((soft - 0.5).abs() * 2.0).clamp(0.0, 1.0).powi(4);
            let v = (a * l + b) * (1.0 - sure) + soft * sure;
            let v = ((v - 0.04) / 0.92).clamp(0.0, 1.0);
            // Within 0..=255: the cast is exact.
            (v * 255.0).round() as u8
        })
        .collect())
}

impl BackgroundRemover for Cutter {
    fn mask(
        &self,
        image: &Pixels,
        progress: &dyn Progress,
        cancel: &CancelToken,
    ) -> Result<AlphaMask, VisionError> {
        cancelled(cancel)?;
        let rgb = opaque(image);
        let input = input(&rgb, self.side, self.mean, self.deviation);
        progress.report(0.1);
        cancelled(cancel)?;
        let out = self.session.run(input)?;
        progress.report(0.8);
        cancelled(cancel)?;
        if out.len() != self.side * self.side {
            return Err(VisionError::Model("an odd output".to_owned()));
        }
        let mask = fitted(&rgb, stretched(&out), self.side)?;
        cancelled(cancel)?;
        let (w, h) = image.size();
        let mask = AlphaMask::new(w, h, mask).map_err(|_| VisionError::TooLarge)?;
        progress.report(1.0);
        Ok(mask)
    }
}

#[cfg(test)]
mod tests {
    use gafira_doc::contract::NoProgress;

    use super::*;

    /// Gives back the light of its input as the subject.
    struct Light(usize);

    impl Session for Light {
        fn run(&self, input: Vec<f32>) -> Result<Vec<f32>, VisionError> {
            Ok(input[..self.0 * self.0].to_vec())
        }
    }

    fn cutter(side: usize) -> Cutter {
        Cutter {
            session: Arc::new(Light(side)),
            side,
            mean: [0.5; 3],
            deviation: [0.25; 3],
        }
    }

    /// A white disc on black.
    fn disc(w: u32, h: u32) -> Pixels {
        let (cx, cy, r) = (w as f32 / 2.0, h as f32 / 2.0, h as f32 / 4.0);
        let bytes = (0..w * h).flat_map(|i| {
            let (x, y) = ((i % w) as f32 - cx, (i / w) as f32 - cy);
            let v = if x * x + y * y < r * r { 255 } else { 0 };
            [v, v, v, 255]
        });
        Pixels::new(w, h, bytes.collect()).unwrap()
    }

    // S84-03: the subject is kept, what is around goes, at the image's size
    // and with sharp edges though the model sees little.
    #[test]
    fn s84_03_the_subject_is_kept() {
        for (w, h) in [(300, 200), (2000, 1400), (40, 600)] {
            let image = disc(w, h);
            let mask = cutter(32)
                .mask(&image, &NoProgress, &CancelToken::new())
                .unwrap();
            assert_eq!((mask.width(), mask.height()), (w, h));
            let wrong = mask
                .data()
                .iter()
                .zip(image.rgba().chunks(4))
                .filter(|(a, p)| a.abs_diff(p[0]) > 100)
                .count();
            let share = wrong as f32 / (w * h) as f32;
            assert!(share < 0.06, "{w} × {h}: {share}");
        }
    }

    // An image with no subject is all background; a wrong output is an error.
    #[test]
    fn s84_03_no_subject_and_wrong_outputs() {
        assert_eq!(stretched(&[0.3; 4]), [0.0; 4]);
        assert_eq!(stretched(&[f32::NAN, 0.0, 2.0]), [0.0, 0.0, 1.0]);
        struct Odd;
        impl Session for Odd {
            fn run(&self, _: Vec<f32>) -> Result<Vec<f32>, VisionError> {
                Ok(vec![1.0])
            }
        }
        let mut odd = cutter(16);
        odd.session = Arc::new(Odd);
        let image = disc(20, 20);
        assert!(matches!(
            odd.mask(&image, &NoProgress, &CancelToken::new()),
            Err(VisionError::Model(_))
        ));
        let cancel = CancelToken::new();
        cancel.cancel();
        let out = cutter(16).mask(&image, &NoProgress, &cancel);
        assert_eq!(out, Err(VisionError::Cancelled));
    }

    // The brightest is one, and each channel is after the one before.
    #[test]
    fn s84_03_what_goes_in() {
        let rgb = RgbImage::from_pixel(5, 3, Rgb([200, 100, 0]));
        let input = input(&rgb, 2, [0.5; 3], [0.5; 3]);
        assert_eq!(
            input,
            [
                1.0, 1.0, 1.0, 1.0, 0.0, 0.0, 0.0, 0.0, -1.0, -1.0, -1.0, -1.0
            ]
        );
        let clear = Pixels::new(1, 1, vec![0, 0, 0, 0]).unwrap();
        assert_eq!(opaque(&clear).get_pixel(0, 0).0, [255; 3]);
    }

    #[test]
    fn s84_03_means() {
        assert_eq!(boxed(&[1.0; 12], 4, 3, 5), [1.0; 12]);
        let out = boxed(&[0.0, 3.0, 0.0], 3, 1, 1);
        assert_eq!(out, [1.0, 1.0, 1.0]);
    }
}
