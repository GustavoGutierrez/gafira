//! The images with a look (spec 085) as the canvas shows them: worked out on
//! a thread, small at first and then whole, so a control answers at once
//! (S85-07); the thumbnails of the filters and the shadows, and the masks of
//! the zones.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex, PoisonError};

use egui::{ColorImage, TextureHandle, TextureId, TextureOptions};
use gafira_color::Color;
use gafira_doc::{
    AssetRef, ImageCrop, ImageFilter, ImageLook, ImageNode, ImageShadow, NodeId, ShadowKind,
};
use gafira_ports::store::ContentStore;
use gafira_raster::effects::{AutoAdjust, auto_adjust};
use gafira_raster::look::{FILTER_GROUPS, apply_look, cropped, cropped_mask, mask_of};
use gafira_raster::{AlphaMask, ImageBuf, PixelBuf, dominant_colors, premultiply_u8};

/// The longest side of an image while a control moves, and at rest.
const QUICK: u32 = 480;
const WHOLE: u32 = 2048;

/// The smaller side of a thumbnail.
const THUMB: u32 = 140;

/// The strength of «Ajuste automático» (FD-33).
const AUTO: f64 = 60.0;

/// The most images worked out at once, and decoded ones kept.
const MOST_JOBS: usize = 3;
const MOST_SOURCES: usize = 6;

/// The shadows, in the order they are shown.
pub(crate) const SHADOWS: [ShadowKind; 7] = [
    ShadowKind::Glow,
    ShadowKind::Drop,
    ShadowKind::Outline,
    ShadowKind::Curved,
    ShadowKind::Floating,
    ShadowKind::Angled,
    ShadowKind::Ground,
];

/// What the panel needs to know of an image.
pub(crate) struct Facts {
    /// The size of the image.
    pub size: (u32, u32),
    /// It, small.
    pub small: ImageBuf,
    /// Its colours, the most used first.
    pub colors: Vec<Color>,
    pub auto: Option<AutoAdjust>,
}

/// An image decoded.
struct Source {
    whole: ImageBuf,
    /// How much smaller than the image each one is.
    whole_scale: f32,
    small_scale: f32,
    facts: Arc<Facts>,
}

#[derive(Default)]
struct Cache {
    sources: Vec<(AssetRef, Arc<Source>)>,
    masks: BTreeMap<AssetRef, AlphaMask>,
}

enum Message {
    Drawn {
        node: NodeId,
        key: AssetRef,
        whole: bool,
        image: Option<ColorImage>,
    },
    Facts(AssetRef, Option<Arc<Facts>>),
    Thumbs(Set, AssetRef, Vec<(String, ColorImage)>),
}

/// A set of thumbnails.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Set {
    Filters,
    Shadows,
}

#[derive(Default)]
struct Thumbs {
    key: Option<AssetRef>,
    busy: bool,
    textures: BTreeMap<String, TextureHandle>,
}

struct Shown {
    texture: TextureHandle,
    whole: bool,
}

/// What is drawn for an image.
pub(crate) struct Picture {
    pub texture: TextureId,
    /// Its size, the margin of its shadow with it.
    pub size: (u32, u32),
    pub shadow: Option<ImageShadow>,
}

impl Picture {
    /// How far past a box of `w` × `h` it is drawn.
    pub fn bleed(&self, (w, h): (f64, f64)) -> (f64, f64) {
        self.shadow
            .map_or((0.0, 0.0), |shadow| shadow.bleed(self.size, (w, h)))
    }
}

pub(crate) struct Looks {
    tx: Sender<Message>,
    rx: Receiver<Message>,
    cache: Arc<Mutex<Cache>>,
    shown: BTreeMap<AssetRef, Shown>,
    /// What was last drawn for each element.
    last: BTreeMap<NodeId, (AssetRef, Option<ImageShadow>)>,
    busy: BTreeSet<NodeId>,
    /// What could not be drawn: not tried again.
    failed: BTreeSet<AssetRef>,
    /// Asked for since the last frame.
    wanted: BTreeSet<AssetRef>,
    seen: BTreeSet<NodeId>,
    thumbs: BTreeMap<Set, Thumbs>,
    facts: BTreeMap<AssetRef, Option<Arc<Facts>>>,
    asked_facts: BTreeSet<AssetRef>,
    masks: BTreeMap<(AssetRef, bool), Option<TextureHandle>>,
    /// The element shown as this asset, for «Comparar».
    pub compare: Option<(NodeId, AssetRef)>,
}

impl Default for Looks {
    fn default() -> Self {
        let (tx, rx) = channel();
        Self {
            tx,
            rx,
            cache: Arc::default(),
            shown: BTreeMap::new(),
            last: BTreeMap::new(),
            busy: BTreeSet::new(),
            failed: BTreeSet::new(),
            wanted: BTreeSet::new(),
            seen: BTreeSet::new(),
            thumbs: BTreeMap::new(),
            facts: BTreeMap::new(),
            asked_facts: BTreeSet::new(),
            masks: BTreeMap::new(),
            compare: None,
        }
    }
}

fn bytes_of(store: &dyn ContentStore, asset: AssetRef) -> Option<Vec<u8>> {
    let mut bytes = Vec::new();
    let mut source = store.open(asset.as_hash()).ok()?;
    std::io::Read::read_to_end(&mut source, &mut bytes).ok()?;
    Some(bytes)
}

/// `image` no longer than `most`, premultiplied.
pub(crate) fn fitted(image: &image::DynamicImage, most: u32) -> Option<ImageBuf> {
    let rgba = if image.width().max(image.height()) > most {
        image.thumbnail(most, most).into_rgba8()
    } else {
        image.to_rgba8()
    };
    let (w, h) = rgba.dimensions();
    let mut bytes = rgba.into_raw();
    for px in bytes.chunks_exact_mut(4) {
        let a = px[3];
        for c in &mut px[..3] {
            *c = premultiply_u8(*c, a);
        }
    }
    ImageBuf::new(w, h, PixelBuf::Rgba8(bytes)).ok()
}

/// How much smaller than `whole` `image` is.
fn scale(image: &ImageBuf, whole: (u32, u32)) -> f32 {
    // Sizes of images: the cast loses nothing that counts.
    let most = whole.0.max(whole.1).max(1) as f32;
    image.width().max(image.height()) as f32 / most
}

/// The image `asset`, decoded once.
fn source(cache: &Mutex<Cache>, store: &dyn ContentStore, asset: AssetRef) -> Option<Arc<Source>> {
    let find = |cache: &Cache| {
        let found = cache.sources.iter().find(|(a, _)| *a == asset);
        found.map(|(_, source)| Arc::clone(source))
    };
    if let Some(found) = find(&cache.lock().unwrap_or_else(PoisonError::into_inner)) {
        return Some(found);
    }
    let image = image::load_from_memory(&bytes_of(store, asset)?).ok()?;
    let size = (image.width(), image.height());
    let whole = fitted(&image, WHOLE)?;
    let small = fitted(&image, QUICK)?;
    let colors = dominant_colors(small.view()).unwrap_or_default();
    let facts = Facts {
        size,
        colors: colors.into_iter().map(|entry| entry.color).collect(),
        auto: auto_adjust(small.view(), AUTO).ok(),
        small,
    };
    let made = Arc::new(Source {
        whole_scale: scale(&whole, size),
        small_scale: scale(&facts.small, size),
        whole,
        facts: Arc::new(facts),
    });
    let mut cache = cache.lock().unwrap_or_else(PoisonError::into_inner);
    if let Some(found) = find(&cache) {
        return Some(found);
    }
    cache.sources.push((asset, Arc::clone(&made)));
    if cache.sources.len() > MOST_SOURCES {
        cache.sources.remove(0);
    }
    Some(made)
}

/// The mask `asset`, decoded once.
fn mask(cache: &Mutex<Cache>, store: &dyn ContentStore, asset: AssetRef) -> Option<AlphaMask> {
    let lock = || cache.lock().unwrap_or_else(PoisonError::into_inner);
    if let Some(found) = lock().masks.get(&asset) {
        return Some(found.clone());
    }
    let image = image::load_from_memory(&bytes_of(store, asset)?).ok()?;
    let made = mask_of(&fitted(&image, u32::MAX)?)?;
    let mut cache = lock();
    if cache.masks.len() > 32 {
        cache.masks.clear();
    }
    cache.masks.insert(asset, made.clone());
    Some(made)
}

/// The part `part` of the image `asset`, no longer than [`WHOLE`], and how
/// much smaller than the image it is.
fn part_of(store: &dyn ContentStore, asset: AssetRef, part: &ImageCrop) -> Option<(ImageBuf, f32)> {
    let image = image::load_from_memory(&bytes_of(store, asset)?).ok()?;
    let pixels = cropped(&fitted(&image, u32::MAX)?, part);
    let longest = pixels.width().max(pixels.height());
    if longest <= WHOLE {
        return Some((pixels, 1.0));
    }
    let PixelBuf::Rgba8(bytes) = pixels.data() else {
        return None;
    };
    let pixels = image::RgbaImage::from_raw(pixels.width(), pixels.height(), bytes.clone())?;
    let pixels = image::DynamicImage::ImageRgba8(pixels).thumbnail(WHOLE, WHOLE);
    let pixels = pixels.into_rgba8();
    let (w, h) = pixels.dimensions();
    let made = ImageBuf::new(w, h, PixelBuf::Rgba8(pixels.into_raw())).ok()?;
    // Sizes of images: the casts lose nothing that counts.
    Some((made, WHOLE as f32 / longest as f32))
}

fn color_image(image: &ImageBuf) -> Option<ColorImage> {
    let PixelBuf::Rgba8(bytes) = image.data() else {
        return None;
    };
    let size = [image.width() as usize, image.height() as usize];
    Some(ColorImage::from_rgba_premultiplied(size, bytes))
}

/// `small` as a thumbnail.
fn thumbnail(small: &ImageBuf) -> Option<(ImageBuf, f32)> {
    let PixelBuf::Rgba8(bytes) = small.data() else {
        return None;
    };
    let (w, h) = (small.width(), small.height());
    let share = f64::from(THUMB) / f64::from(w.min(h).max(1));
    if share >= 1.0 {
        return Some((small.clone(), 1.0));
    }
    // Smaller than the image: within a u32.
    let to = |v: u32| ((f64::from(v) * share).round() as u32).max(1);
    let image = image::RgbaImage::from_raw(w, h, bytes.clone())?;
    let image = image::imageops::thumbnail(&image, to(w), to(h));
    let (w, h) = image.dimensions();
    let made = ImageBuf::new(w, h, PixelBuf::Rgba8(image.into_raw())).ok()?;
    Some((made, share as f32))
}

/// The name of the thumbnail of a shadow.
pub(crate) fn shadow_id(kind: ShadowKind) -> String {
    format!("{kind:?}")
}

impl Looks {
    /// What the threads made; what nothing shows any more goes.
    pub fn pump(&mut self, ctx: &egui::Context) {
        let options = TextureOptions::LINEAR;
        while let Ok(message) = self.rx.try_recv() {
            match message {
                Message::Drawn {
                    node,
                    key,
                    whole,
                    image,
                } => {
                    self.busy.remove(&node);
                    match image {
                        Some(image) => {
                            let name = format!("look-{}", key.hex());
                            let texture = ctx.load_texture(name, image, options);
                            self.shown.insert(key, Shown { texture, whole });
                        }
                        None => {
                            self.failed.insert(key);
                        }
                    }
                }
                Message::Facts(asset, facts) => {
                    self.facts.insert(asset, facts);
                }
                Message::Thumbs(set, key, images) => {
                    let thumbs = self.thumbs.entry(set).or_default();
                    thumbs.busy = false;
                    thumbs.key = Some(key);
                    thumbs.textures = images
                        .into_iter()
                        .map(|(id, image)| {
                            let texture = ctx.load_texture(format!("thumb-{id}"), image, options);
                            (id, texture)
                        })
                        .collect();
                }
            }
        }
        let seen = std::mem::take(&mut self.seen);
        self.last.retain(|node, _| seen.contains(node));
        let wanted = std::mem::take(&mut self.wanted);
        let last: BTreeSet<AssetRef> = self.last.values().map(|(key, _)| *key).collect();
        self.shown
            .retain(|key, _| wanted.contains(key) || last.contains(key));
    }

    /// Has `image`, of `node`, drawn with its look: small at once and whole
    /// when `settled`.
    pub fn want(
        &mut self,
        ctx: &egui::Context,
        store: &Arc<dyn ContentStore>,
        (node, image): (NodeId, &ImageNode),
        settled: bool,
    ) {
        if image.is_plain() {
            return;
        }
        let shadow = image.look.as_ref().and_then(|look| look.shadow);
        let key = image.shown();
        self.wanted.insert(key);
        self.seen.insert(node);
        let whole = match self.shown.get(&key) {
            Some(shown) => {
                self.last.insert(node, (key, shadow));
                if shown.whole || !settled {
                    return;
                }
                true
            }
            None => false,
        };
        if self.failed.contains(&key) || self.busy.contains(&node) || self.busy.len() >= MOST_JOBS {
            return;
        }
        self.busy.insert(node);
        let image = image.clone();
        let (store, cache) = (Arc::clone(store), Arc::clone(&self.cache));
        self.spawn(ctx, move || {
            let part = image.part();
            let look = image.look.as_deref().cloned().unwrap_or_default();
            let image = source(&cache, &*store, image.asset).and_then(|source| {
                let (pixels, scale) = match (whole, image.crop.is_some()) {
                    // A part of a large image has pixels to spare.
                    (true, true) => part_of(&*store, image.asset, &part)?,
                    (true, false) => (source.whole.clone(), source.whole_scale),
                    (false, _) => (cropped(&source.facts.small, &part), source.small_scale),
                };
                let mask = |asset| Some(cropped_mask(&mask(&cache, &*store, asset)?, &part));
                let made = apply_look(&look, &pixels, &mask, scale).ok()?;
                color_image(&made)
            });
            Message::Drawn {
                node,
                key,
                whole,
                image,
            }
        });
    }

    fn spawn(&self, ctx: &egui::Context, run: impl FnOnce() -> Message + Send + 'static) {
        let (tx, ctx) = (self.tx.clone(), ctx.clone());
        std::thread::spawn(move || {
            let _ = tx.send(run());
            ctx.request_repaint();
        });
    }

    /// What to draw for `image` of `node`: it with its look, or the last one
    /// there was while it comes; none to draw it as it is.
    pub fn picture(&self, node: NodeId, image: &ImageNode) -> Option<Picture> {
        if image.is_plain() {
            return None;
        }
        let shadow = image.look.as_ref().and_then(|look| look.shadow);
        if self.compare.is_some_and(|(of, _)| of == node) {
            return None;
        }
        let of = |key: &AssetRef, shadow| {
            let texture = &self.shown.get(key)?.texture;
            let [w, h] = texture.size();
            Some(Picture {
                texture: texture.id(),
                size: (u32::try_from(w).ok()?, u32::try_from(h).ok()?),
                shadow,
            })
        };
        of(&image.shown(), shadow).or_else(|| {
            let (key, shadow) = self.last.get(&node)?;
            of(key, *shadow)
        })
    }

    /// What there is to know of `asset`, once it is read.
    pub fn facts(
        &mut self,
        ctx: &egui::Context,
        store: &Arc<dyn ContentStore>,
        asset: AssetRef,
    ) -> Option<Arc<Facts>> {
        if self.asked_facts.insert(asset) {
            let (store, cache) = (Arc::clone(store), Arc::clone(&self.cache));
            self.spawn(ctx, move || {
                let facts = source(&cache, &*store, asset);
                Message::Facts(asset, facts.map(|source| Arc::clone(&source.facts)))
            });
        }
        self.facts.get(&asset).cloned().flatten()
    }

    /// The picture of `image` as it is now and no other; it is drawn when it
    /// is wanted.
    pub fn exact(&self, image: &ImageNode) -> Option<TextureId> {
        Some(self.shown.get(&image.shown())?.texture.id())
    }

    /// The thumbnail `id` of `set` for `image`, the last one while it comes.
    pub fn thumb(&self, set: Set, id: &str) -> Option<&TextureHandle> {
        self.thumbs.get(&set)?.textures.get(id)
    }

    /// Has the thumbnails of `set` drawn for `image`.
    pub fn want_thumbs(
        &mut self,
        ctx: &egui::Context,
        store: &Arc<dyn ContentStore>,
        set: Set,
        image: &ImageNode,
    ) {
        let mut look = image.look.as_deref().cloned().unwrap_or_default();
        look.shadow = None;
        if set == Set::Filters {
            look.filter = None;
        }
        let part = image.part();
        let key = look.key(image.asset, image.crop.as_deref());
        let thumbs = self.thumbs.entry(set).or_default();
        if thumbs.busy || thumbs.key == Some(key) {
            return;
        }
        thumbs.busy = true;
        let asset = image.asset;
        let (store, cache) = (Arc::clone(store), Arc::clone(&self.cache));
        self.spawn(ctx, move || {
            let made = source(&cache, &*store, asset).and_then(|source| {
                let (small, share) = thumbnail(&cropped(&source.facts.small, &part))?;
                let mask = |asset| Some(cropped_mask(&mask(&cache, &*store, asset)?, &part));
                let scale = share * source.small_scale;
                let base = apply_look(&look, &small, &mask, scale).ok()?;
                Some(thumbs_of(set, &base, scale))
            });
            Message::Thumbs(set, key, made.unwrap_or_default())
        });
    }

    /// The mask `asset`, as white of its opacity.
    pub fn mask(
        &mut self,
        ctx: &egui::Context,
        store: &dyn ContentStore,
        asset: AssetRef,
        inverted: bool,
    ) -> Option<TextureId> {
        let cache = &self.cache;
        let made = self.masks.entry((asset, inverted)).or_insert_with(|| {
            let mask = mask(cache, store, asset)?;
            let image = white(mask.data(), (mask.width(), mask.height()), inverted);
            Some(ctx.load_texture(
                format!("mask-{}", asset.hex()),
                image,
                TextureOptions::LINEAR,
            ))
        });
        made.as_ref().map(TextureHandle::id)
    }

    /// The pixels of the mask `asset`.
    pub fn mask_pixels(&self, store: &dyn ContentStore, asset: AssetRef) -> Option<AlphaMask> {
        mask(&self.cache, store, asset)
    }
}

/// `alpha` as white of that opacity.
pub(crate) fn white(alpha: &[u8], (w, h): (u32, u32), inverted: bool) -> ColorImage {
    let pixels = alpha
        .iter()
        .map(|a| egui::Color32::from_white_alpha(if inverted { 255 - a } else { *a }))
        .collect();
    ColorImage::new([w as usize, h as usize], pixels)
}

fn thumbs_of(set: Set, base: &ImageBuf, scale: f32) -> Vec<(String, ColorImage)> {
    let draw = |look: ImageLook| {
        let made = apply_look(&look, base, &|_| None, scale).ok()?;
        color_image(&made)
    };
    match set {
        Set::Filters => FILTER_GROUPS
            .iter()
            .flat_map(|group| group.filters)
            .filter_map(|id| {
                let filter = ImageFilter {
                    id: id.to_owned(),
                    intensity: 100.0,
                };
                let look = ImageLook {
                    filter: Some(filter),
                    ..ImageLook::default()
                };
                Some((id.to_owned(), draw(look)?))
            })
            .collect(),
        Set::Shadows => SHADOWS
            .iter()
            .filter_map(|kind| {
                let look = ImageLook {
                    shadow: Some(ImageShadow::new(*kind)),
                    ..ImageLook::default()
                };
                Some((shadow_id(*kind), draw(look)?))
            })
            .collect(),
    }
}
