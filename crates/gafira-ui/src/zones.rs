//! The zones of an image (S85-05): how their masks are made, by the model
//! that tells a subject from its background, by a click on an element or by
//! hand.

use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender, channel};

use gafira_doc::contract::{CancelToken, NoProgress};
use gafira_doc::{AssetRef, ContentHash, ImageLook, ImageZone, NodeId, ZoneKind};
use gafira_ports::store::ContentStore;
use gafira_ports::vision::{Pixels, VisionError, VisionModels};
use gafira_raster::{AlphaMask, ImageBuf};
use gafira_segment::{ClassicSegmenter, SeedStroke, SegHints, SegZone, Segmenter};

/// The longest side of a mask painted or of an element, and of a subject.
pub(crate) const PAINTED: u32 = 1024;
const SUBJECT: u32 = 2048;

/// A mask and whether it can be trusted.
pub(crate) struct Mask {
    pub mask: AlphaMask,
    pub reliable: bool,
}

/// Why there is no mask.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Trouble {
    Vision(VisionError),
    /// Nothing was told apart.
    Nothing,
}

/// What a thread made.
pub(crate) struct Made {
    pub node: NodeId,
    /// The image it was made of: another one by now does not take it.
    pub asset: AssetRef,
    pub purpose: Purpose,
    pub result: Result<Mask, Trouble>,
}

/// What a mask is for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Purpose {
    Zone(ZoneKind),
    /// What is under a stroke, added to the one painted or taken from it.
    Stroke(bool),
}

/// The threads that make masks, one at a time.
pub(crate) struct Masks {
    tx: Sender<Made>,
    rx: Receiver<Made>,
    cancel: Option<CancelToken>,
}

impl Default for Masks {
    fn default() -> Self {
        let (tx, rx) = channel();
        Self {
            tx,
            rx,
            cancel: None,
        }
    }
}

impl Masks {
    pub fn busy(&self) -> bool {
        self.cancel.is_some()
    }

    pub fn cancel(&mut self) {
        if let Some(cancel) = &self.cancel {
            cancel.cancel();
        }
    }

    pub fn pump(&mut self) -> Vec<Made> {
        let made: Vec<Made> = self.rx.try_iter().collect();
        if !made.is_empty() {
            self.cancel = None;
        }
        made
    }

    fn start(
        &mut self,
        ctx: &egui::Context,
        (node, asset, purpose): (NodeId, AssetRef, Purpose),
        run: impl FnOnce(&CancelToken) -> Result<Mask, Trouble> + Send + 'static,
    ) {
        if self.busy() {
            return;
        }
        let cancel = CancelToken::new();
        self.cancel = Some(cancel.clone());
        let (tx, ctx) = (self.tx.clone(), ctx.clone());
        std::thread::spawn(move || {
            let result = run(&cancel);
            let _ = tx.send(Made {
                node,
                asset,
                purpose,
                result,
            });
            ctx.request_repaint();
        });
    }

    /// The subject of the image `of`, for a zone of `kind`.
    pub fn subject(
        &mut self,
        ctx: &egui::Context,
        (node, asset, kind): (NodeId, AssetRef, ZoneKind),
        store: &Arc<dyn ContentStore>,
        models: Option<Arc<dyn VisionModels>>,
    ) {
        let store = Arc::clone(store);
        self.start(ctx, (node, asset, Purpose::Zone(kind)), move |cancel| {
            let whole = decoded(&*store, asset).map_err(Trouble::Vision)?;
            subject(&whole, || match &models {
                Some(models) => models.remover()?.mask(&whole, &NoProgress, cancel),
                None => Err(VisionError::Unavailable(String::new())),
            })
        });
    }

    /// The element of `asset` under `points`, in shares of its sides.
    pub fn element(
        &mut self,
        ctx: &egui::Context,
        (node, asset, purpose): (NodeId, AssetRef, Purpose),
        store: &Arc<dyn ContentStore>,
        (points, radius): (Vec<[f32; 2]>, f32),
    ) {
        let store = Arc::clone(store);
        self.start(ctx, (node, asset, purpose), move |cancel| {
            let whole = decoded(&*store, asset).map_err(Trouble::Vision)?;
            let image = shrunk(&whole, PAINTED).ok_or(Trouble::Nothing)?;
            element(&image, &points, radius, cancel)
        });
    }
}

/// The content of `asset`, decoded.
fn decoded(store: &dyn ContentStore, asset: AssetRef) -> Result<Pixels, VisionError> {
    let io = |e: &dyn std::fmt::Display| VisionError::Io(e.to_string());
    let mut source = store.open(asset.as_hash()).map_err(|e| io(&e))?;
    let mut bytes = Vec::new();
    std::io::Read::read_to_end(&mut source, &mut bytes).map_err(|e| io(&e))?;
    let rgba = image::load_from_memory(&bytes)
        .map_err(|e| io(&e))?
        .into_rgba8();
    Pixels::new(rgba.width(), rgba.height(), rgba.into_raw()).ok_or(VisionError::TooLarge)
}

/// `image` no longer than `most`, premultiplied.
fn shrunk(image: &Pixels, most: u32) -> Option<ImageBuf> {
    let (w, h) = image.size();
    let image = image::RgbaImage::from_raw(w, h, image.rgba().to_vec())?;
    crate::looks::fitted(&image::DynamicImage::ImageRgba8(image), most)
}

/// `mask` no longer than `most`.
fn shrunk_mask(mask: AlphaMask, most: u32) -> Option<AlphaMask> {
    let (w, h) = (mask.width(), mask.height());
    if w.max(h) <= most {
        return Some(mask);
    }
    let image = image::GrayImage::from_raw(w, h, mask.data().to_vec())?;
    let image = image::DynamicImage::ImageLuma8(image).thumbnail(most, most);
    let image = image.into_luma8();
    AlphaMask::new(image.width(), image.height(), image.into_raw()).ok()
}

/// The subject of `image`: what is left of it when its background is already
/// gone, or what `model` says.
fn subject(
    image: &Pixels,
    model: impl FnOnce() -> Result<AlphaMask, VisionError>,
) -> Result<Mask, Trouble> {
    let (w, h) = image.size();
    let alpha: Vec<u8> = image.rgba().chunks_exact(4).map(|px| px[3]).collect();
    let clear = alpha.iter().filter(|a| **a < 16).count();
    let mask = if clear * 100 > alpha.len() {
        AlphaMask::new(w, h, alpha).map_err(|_| Trouble::Nothing)?
    } else {
        model().map_err(Trouble::Vision)?
    };
    let covered = mask.data().iter().filter(|a| **a >= 128).count();
    let reliable = plausible(covered, mask.data().len());
    if covered == 0 {
        return Err(Trouble::Nothing);
    }
    let mask = shrunk_mask(mask, SUBJECT).ok_or(Trouble::Nothing)?;
    Ok(Mask { mask, reliable })
}

/// Neither next to nothing nor nearly all of it (ARQ-131).
fn plausible(covered: usize, total: usize) -> bool {
    covered * 100 >= total && covered * 100 <= 95 * total
}

/// The element of `image` under `points`, in shares of its sides, `radius` a
/// share of the smaller one.
pub(crate) fn element(
    image: &ImageBuf,
    points: &[[f32; 2]],
    radius: f32,
    cancel: &CancelToken,
) -> Result<Mask, Trouble> {
    // Sizes of at most 1024: the casts are exact.
    let (w, h) = (image.width(), image.height());
    let (fw, fh) = (w as f32, h as f32);
    let points: Vec<[f32; 2]> = points.iter().map(|p| [p[0] * fw, p[1] * fh]).collect();
    let radius = (radius * fw.min(fh)).max(2.0);
    let hints = SegHints {
        zone: SegZone::Foreground,
        foreground: vec![SeedStroke {
            points: points.clone(),
            radius,
        }],
        background: Vec::new(),
    };
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(4)
        .build()
        .map_err(|e| Trouble::Vision(VisionError::Io(e.to_string())))?;
    let found = ClassicSegmenter::new(Arc::new(pool))
        .segment(image.view(), &hints, &NoProgress, cancel)
        .map_err(|_| Trouble::Nothing)?;
    let mut data = found.mask.data().to_vec();
    keep_touching(&mut data, (w as usize, h as usize), &points);
    let covered = data.iter().filter(|a| **a >= 128).count();
    if covered == 0 {
        return Err(Trouble::Nothing);
    }
    let reliable = found.reliable && plausible(covered, data.len());
    let mask = AlphaMask::new(w, h, data).map_err(|_| Trouble::Nothing)?;
    Ok(Mask { mask, reliable })
}

/// Leaves of `data` only the pieces that hold one of `points`.
fn keep_touching(data: &mut [u8], (w, h): (usize, usize), points: &[[f32; 2]]) {
    let mut kept = vec![false; data.len()];
    let mut next: Vec<usize> = points
        .iter()
        .filter(|p| p[0] >= 0.0 && p[1] >= 0.0)
        // Not negative: the casts drop the fraction.
        .map(|p| (p[0] as usize, p[1] as usize))
        .filter(|(x, y)| *x < w && *y < h)
        .map(|(x, y)| y * w + x)
        .collect();
    while let Some(at) = next.pop() {
        if kept[at] || data[at] < 64 {
            continue;
        }
        kept[at] = true;
        let (x, y) = (at % w, at / w);
        if x > 0 {
            next.push(at - 1);
        }
        if x + 1 < w {
            next.push(at + 1);
        }
        if y > 0 {
            next.push(at - w);
        }
        if y + 1 < h {
            next.push(at + w);
        }
    }
    for (value, kept) in data.iter_mut().zip(kept) {
        if !kept {
            *value = 0;
        }
    }
}

/// `alpha` in `store`, as a grey PNG.
pub(crate) fn stored(
    store: &dyn ContentStore,
    alpha: &[u8],
    (w, h): (u32, u32),
) -> Option<AssetRef> {
    use image::ImageEncoder as _;
    let mut bytes = Vec::new();
    image::codecs::png::PngEncoder::new(&mut bytes)
        .write_image(alpha, w, h, image::ExtendedColorType::L8)
        .ok()?;
    let hash: ContentHash = store.put(&mut std::io::Cursor::new(bytes)).ok()?;
    Some(AssetRef::new(hash))
}

/// `look` with a zone of `kind` made of `mask`, and which one it is: the one
/// there is of a kind there is one of, or a new one, if it fits.
pub(crate) fn with_zone(
    look: &ImageLook,
    kind: ZoneKind,
    mask: AssetRef,
) -> Option<(ImageLook, usize)> {
    let mut look = look.clone();
    let single = matches!(kind, ZoneKind::Foreground | ZoneKind::Background);
    let same = look.zones.iter().position(|zone| zone.kind == kind);
    if let Some(at) = same.filter(|_| single) {
        look.zones[at].mask = mask;
        return Some((look, at));
    }
    if look.zones.len() >= ImageLook::MAX_ZONES {
        return None;
    }
    look.zones.push(ImageZone {
        kind,
        mask,
        inverted: kind == ZoneKind::Background,
        adjust: gafira_doc::ImageAdjust::default(),
    });
    let at = look.zones.len() - 1;
    Some((look, at))
}

#[cfg(test)]
mod tests {
    use super::*;
    use gafira_raster::PixelBuf;

    fn disc(w: u32, h: u32) -> ImageBuf {
        // A red disc on the left and a blue one on the right, on grey.
        let bytes = (0..w * h)
            .flat_map(|i| {
                let (x, y) = ((i % w) as f32, (i / w) as f32);
                let near = |cx: f32| (x - cx).hypot(y - 60.0) < 30.0;
                if near(60.0) {
                    [210, 40, 40, 255]
                } else if near(180.0) {
                    [40, 60, 210, 255]
                } else {
                    [200, 200, 196, 255]
                }
            })
            .collect();
        ImageBuf::new(w, h, PixelBuf::Rgba8(bytes)).expect("an image")
    }

    // S85-05: a click on an element takes it and not the other one.
    #[test]
    fn s85_05_the_element_under_a_click() {
        let image = disc(240, 120);
        let token = CancelToken::new();
        let made = element(&image, &[[0.25, 0.5]], 0.03, &token).expect("an element");
        let at = |x: usize, y: usize| made.mask.data()[y * 240 + x];
        assert!(at(60, 60) > 200 && at(45, 60) > 200);
        assert!(at(180, 60) < 40 && at(120, 20) < 40 && at(5, 5) < 40);
        assert!(made.reliable);
    }

    // S85-05: what is already cut out is its own subject; a model that finds
    // nothing, or everything, is not to be trusted.
    #[test]
    fn s85_05_the_subject() {
        let cut: Vec<u8> = (0..100)
            .flat_map(|i| [9, 9, 9, if i < 50 { 255 } else { 0 }])
            .collect();
        let cut = Pixels::new(10, 10, cut).expect("pixels");
        let made = subject(&cut, || panic!("no model")).expect("a mask");
        assert!(made.reliable && made.mask.data()[0] == 255 && made.mask.data()[99] == 0);
        let whole = Pixels::new(10, 10, vec![255; 400]).expect("pixels");
        let model = |v: u8| subject(&whole, || Ok(AlphaMask::new(10, 10, vec![v; 100]).unwrap()));
        assert!(matches!(model(0), Err(Trouble::Nothing)));
        assert!(model(255).is_ok_and(|m| !m.reliable));
        let none = subject(&whole, || Err(VisionError::Offline));
        assert!(matches!(none, Err(Trouble::Vision(VisionError::Offline))));
    }

    // S85-05: one subject and one background, as many areas as fit.
    #[test]
    fn s85_05_zones_are_added() {
        let mask = |n| AssetRef::new(ContentHash([n; 32]));
        let mut look = ImageLook::default();
        for (kind, at) in [
            (ZoneKind::Foreground, 0),
            (ZoneKind::Background, 1),
            (ZoneKind::Foreground, 0),
        ] {
            let (next, index) = with_zone(&look, kind, mask(7)).expect("it fits");
            assert_eq!(index, at);
            look = next;
        }
        assert!(look.zones[1].inverted && !look.zones[0].inverted);
        for n in 0..6 {
            look = with_zone(&look, ZoneKind::Area, mask(n))
                .expect("it fits")
                .0;
        }
        assert!(look.is_valid() && with_zone(&look, ZoneKind::Area, mask(9)).is_none());
    }
}
