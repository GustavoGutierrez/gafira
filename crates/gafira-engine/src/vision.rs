//! Taking the background of an image away (spec 084): a poor image is made
//! better first, so the model sees its subject, and what comes out keeps the
//! better pixels (S84-02).

use gafira_doc::contract::{CancelToken, Progress};
use gafira_ports::vision::{BackgroundRemover, Pixels, Upscaler, VisionError};
use gafira_raster::{AlphaMask, Quality, assess};

/// An image with its background away.
#[derive(Clone, Debug, PartialEq)]
pub struct Cutout {
    /// The image with a better resolution, when it was poor.
    pub enhanced: Option<Pixels>,
    /// The opacity of the pixels of `enhanced`, or of the image.
    pub mask: AlphaMask,
    pub quality: Quality,
}

/// The longest side a poor image is brought to, in px: what a model sees.
const ENOUGH: u32 = 1600;

/// The share of the work that making it better is.
const ENHANCING: f32 = 0.75;

/// A part of the work, from `from` to `to`.
struct Part<'a> {
    whole: &'a dyn Progress,
    from: f32,
    to: f32,
}

impl Progress for Part<'_> {
    fn report(&self, fraction: f32) {
        let fraction = fraction.clamp(0.0, 1.0);
        self.whole
            .report(self.from + (self.to - self.from) * fraction);
    }
}

/// How many times larger `image` is made before its background is taken
/// away; 1 when it is fine as it is, or large already.
fn factor(image: &Pixels, quality: Quality, most: u32) -> u32 {
    let (w, h) = image.size();
    if quality == Quality::Good {
        return 1;
    }
    (ENOUGH / w.max(h)).clamp(1, most.max(1))
}

/// Takes the background of `image` away. A poor image is made better with
/// `upscaler` first; when that fails, its background is taken away as it is.
///
/// # Errors
/// [`VisionError`] of the remover, or [`VisionError::Cancelled`].
pub fn remove_background(
    image: &Pixels,
    remover: &dyn BackgroundRemover,
    upscaler: Option<&dyn Upscaler>,
    progress: &dyn Progress,
    cancel: &CancelToken,
) -> Result<Cutout, VisionError> {
    cancel.check().map_err(|_| VisionError::Cancelled)?;
    let (w, h) = image.size();
    let quality = assess(w, h, image.rgba());
    let mut enhanced = None;
    if let Some(upscaler) = upscaler {
        let factor = factor(image, quality, upscaler.most());
        if factor > 1 {
            let part = Part {
                whole: progress,
                from: 0.0,
                to: ENHANCING,
            };
            match upscaler.upscale(image, factor, &part, cancel) {
                Ok(better) => enhanced = Some(better),
                Err(VisionError::Cancelled) => return Err(VisionError::Cancelled),
                // Its background goes all the same.
                Err(_) => {}
            }
        }
    }
    let part = Part {
        whole: progress,
        from: if enhanced.is_some() { ENHANCING } else { 0.0 },
        to: 1.0,
    };
    let mask = remover.mask(enhanced.as_ref().unwrap_or(image), &part, cancel)?;
    progress.report(1.0);
    Ok(Cutout {
        enhanced,
        mask,
        quality,
    })
}

/// `image` with the opacity of `mask` besides its own; none when they are
/// not of the same size.
#[must_use]
pub fn cut(image: &Pixels, mask: &AlphaMask) -> Option<Pixels> {
    let (w, h) = image.size();
    if (mask.width(), mask.height()) != (w, h) {
        return None;
    }
    let mut rgba = image.rgba().to_vec();
    for (pixel, alpha) in rgba.chunks_exact_mut(4).zip(mask.data()) {
        // At most 255: the cast is exact.
        pixel[3] = ((u16::from(pixel[3]) * u16::from(*alpha) + 127) / 255) as u8;
    }
    Pixels::new(w, h, rgba)
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;

    #[derive(Default)]
    struct Seen(Mutex<Vec<f32>>);

    impl Progress for Seen {
        fn report(&self, fraction: f32) {
            self.0.lock().unwrap().push(fraction);
        }
    }

    /// Takes away all but what it is told to, and keeps the sizes it saw.
    #[derive(Default)]
    struct Remover(Mutex<Vec<(u32, u32)>>);

    impl BackgroundRemover for Remover {
        fn mask(
            &self,
            image: &Pixels,
            progress: &dyn Progress,
            cancel: &CancelToken,
        ) -> Result<AlphaMask, VisionError> {
            cancel.check().map_err(|_| VisionError::Cancelled)?;
            let (w, h) = image.size();
            self.0.lock().unwrap().push((w, h));
            progress.report(1.0);
            AlphaMask::new(w, h, vec![128; (w * h) as usize]).map_err(|_| VisionError::TooLarge)
        }
    }

    struct Larger(Result<(), VisionError>);

    impl Upscaler for Larger {
        fn most(&self) -> u32 {
            4
        }

        fn upscale(
            &self,
            image: &Pixels,
            factor: u32,
            progress: &dyn Progress,
            _: &CancelToken,
        ) -> Result<Pixels, VisionError> {
            self.0.clone()?;
            progress.report(1.0);
            let (w, h) = (image.size().0 * factor, image.size().1 * factor);
            Ok(Pixels::new(w, h, vec![7; (w * h * 4) as usize]).unwrap())
        }
    }

    /// Noise, sharp at any size.
    fn image(w: u32, h: u32) -> Pixels {
        let bytes = (0..w * h).flat_map(|i| {
            let v = (i.wrapping_mul(2_654_435_761) >> 24) as u8;
            [v, v, v, 255]
        });
        Pixels::new(w, h, bytes.collect()).unwrap()
    }

    fn run(
        image: &Pixels,
        remover: &Remover,
        upscaler: Option<&dyn Upscaler>,
    ) -> (Result<Cutout, VisionError>, Vec<f32>) {
        let seen = Seen::default();
        let out = remove_background(image, remover, upscaler, &seen, &CancelToken::new());
        (out, seen.0.into_inner().unwrap())
    }

    // S84-02: a good image goes straight to the model.
    #[test]
    fn s84_02_a_good_image_is_not_made_larger() {
        let remover = Remover::default();
        let (out, seen) = run(&image(1000, 700), &remover, Some(&Larger(Ok(()))));
        let out = out.unwrap();
        assert_eq!((out.enhanced, out.quality), (None, Quality::Good));
        assert_eq!(*remover.0.lock().unwrap(), [(1000, 700)]);
        assert_eq!(seen.last(), Some(&1.0));
    }

    // S84-02: a poor one is made better, no larger than it takes, and its
    // mask is of the better one.
    #[test]
    fn s84_02_a_poor_image_is_made_better_first() {
        for (w, factor) in [(200, 4), (400, 4), (600, 2), (790, 2)] {
            let remover = Remover::default();
            let (out, seen) = run(&image(w, 100), &remover, Some(&Larger(Ok(()))));
            let out = out.unwrap();
            assert_eq!(out.enhanced.unwrap().size(), (w * factor, 100 * factor));
            assert_eq!(
                (out.mask.width(), out.quality),
                (w * factor, Quality::Small)
            );
            assert!(
                seen.windows(2).all(|p| p[0] <= p[1]) && seen.last() == Some(&1.0),
                "{seen:?}"
            );
            assert!(seen[0] < 0.9, "there is more to do: {seen:?}");
        }
    }

    // S84-02: without a way to make it better, its background goes all the
    // same; a cancelled one stops.
    #[test]
    fn s84_02_failing_to_make_it_better_is_not_the_end() {
        let remover = Remover::default();
        let fails = Larger(Err(VisionError::Model("no".into())));
        for upscaler in [None, Some(&fails as &dyn Upscaler)] {
            let (out, seen) = run(&image(200, 100), &remover, upscaler);
            assert_eq!(out.unwrap().enhanced, None);
            assert_eq!(seen.last(), Some(&1.0));
        }
        let stops = Larger(Err(VisionError::Cancelled));
        let (out, _) = run(&image(200, 100), &remover, Some(&stops));
        assert_eq!(out, Err(VisionError::Cancelled));
        let cancel = CancelToken::new();
        cancel.cancel();
        let out = remove_background(&image(900, 900), &remover, None, &Seen::default(), &cancel);
        assert_eq!(out, Err(VisionError::Cancelled));
    }

    // S84-01: the opacity of the image and of the mask, both; the colours
    // stay, so they can come back.
    #[test]
    fn s84_01_cutting_keeps_the_colours() {
        let image = Pixels::new(2, 1, vec![1, 2, 3, 255, 4, 5, 6, 128]).unwrap();
        let mask = AlphaMask::new(2, 1, vec![0, 128]).unwrap();
        assert_eq!(
            cut(&image, &mask).unwrap().rgba(),
            [1, 2, 3, 0, 4, 5, 6, 64]
        );
        let other = AlphaMask::new(1, 1, vec![0]).unwrap();
        assert_eq!(cut(&image, &other), None);
    }
}
