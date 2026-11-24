//! «Mejorar resolución» and «Quitar fondo» (spec 084): the work on a thread,
//! its effect over the image, the buttons of Propiedades and the panel of
//! the models.

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, PoisonError, RwLock};

use egui::{Color32, Pos2, RichText, Stroke, Ui, Vec2};
use gafira_doc::contract::{CancelToken, Progress};
use gafira_doc::{AssetRef, ContentHash, ImageNode, NodeId, PageId, Placement};
use gafira_engine::vision::{cut, remove_background};
use gafira_ports::store::ContentStore;
use gafira_ports::vision::{ModelInfo, Pixels, VisionError, VisionModels, VisionTask};
use kurbo::Affine;

use crate::charts::plain;
use crate::rotation;
use crate::style::Theme;
use crate::text::Texts;
use crate::widgets;

/// What is done to an image.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Work {
    /// «Quitar fondo».
    Cut,
    /// «Mejorar resolución», this many times larger.
    Enhance(u32),
}

/// The image worked on.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Target {
    pub page: PageId,
    pub node: NodeId,
    pub image: ImageNode,
}

/// What a thread tells.
enum Message {
    Made {
        target: Target,
        result: Result<ImageNode, VisionError>,
    },
    Opened {
        target: Target,
        result: Result<Box<(Pixels, Vec<u8>)>, VisionError>,
    },
    Installed {
        result: Result<(), VisionError>,
    },
}

/// What came of the work, for the editor.
pub(crate) enum Arrived {
    /// The image now, and the new assets it needs.
    Image(Target, ImageNode),
    /// The image and its opacity, to erase pixels of.
    Erase(Target, Box<(Pixels, Vec<u8>)>),
}

/// A piece of work going on.
struct Job {
    /// None for a download.
    target: Option<Target>,
    /// Erasing only opens or closes: no scan.
    scan: bool,
    progress: Arc<Meter>,
    cancel: CancelToken,
}

/// How far it is, for the interface to see.
struct Meter {
    share: AtomicU32,
    ctx: Option<egui::Context>,
}

impl Progress for Meter {
    fn report(&self, fraction: f32) {
        // A share of ten thousand: the cast is exact.
        let share = (fraction.clamp(0.0, 1.0) * 10_000.0) as u32;
        self.share.fetch_max(share, Ordering::Relaxed);
        if let Some(ctx) = &self.ctx {
            ctx.request_repaint();
        }
    }
}

impl Meter {
    fn share(&self) -> f32 {
        self.share.load(Ordering::Relaxed) as f32 / 10_000.0
    }
}

/// The work of the editor on images.
pub(crate) struct Studio {
    models: Option<Arc<dyn VisionModels>>,
    tx: Sender<Message>,
    rx: Receiver<Message>,
    job: Option<Job>,
    error: Option<VisionError>,
    /// The panel of the models is open; what it lists, read once.
    panel: bool,
    listed: RwLock<Option<Arc<Listed>>>,
    ctx: Option<egui::Context>,
}

struct Listed {
    models: Vec<ModelInfo>,
    runtimes: Vec<ModelInfo>,
}

impl Default for Studio {
    fn default() -> Self {
        let (tx, rx) = channel();
        Self {
            models: None,
            tx,
            rx,
            job: None,
            error: None,
            panel: false,
            listed: RwLock::default(),
            ctx: None,
        }
    }
}

/// The content `hash` of `store`, decoded.
fn decoded(store: &dyn ContentStore, asset: AssetRef) -> Result<Pixels, VisionError> {
    let io = |e: &dyn std::fmt::Display| VisionError::Io(e.to_string());
    let mut source = store.open(asset.as_hash()).map_err(|e| io(&e))?;
    let mut bytes = Vec::new();
    std::io::Read::read_to_end(&mut source, &mut bytes).map_err(|e| io(&e))?;
    let image = image::load_from_memory(&bytes).map_err(|e| io(&e))?;
    let rgba = image.into_rgba8();
    Pixels::new(rgba.width(), rgba.height(), rgba.into_raw()).ok_or(VisionError::TooLarge)
}

/// `pixels` in `store`, as a PNG.
fn stored(store: &dyn ContentStore, pixels: Pixels) -> Result<AssetRef, VisionError> {
    use image::ImageEncoder as _;
    use image::codecs::png::{CompressionType, FilterType, PngEncoder};
    let io = |e: &dyn std::fmt::Display| VisionError::Io(e.to_string());
    let (w, h) = pixels.size();
    let mut bytes = Vec::new();
    // A large image is not worth a long time.
    PngEncoder::new_with_quality(&mut bytes, CompressionType::Fast, FilterType::Adaptive)
        .write_image(pixels.rgba(), w, h, image::ExtendedColorType::Rgba8)
        .map_err(|e| io(&e))?;
    let hash: ContentHash = store
        .put(&mut std::io::Cursor::new(bytes))
        .map_err(|e| io(&e))?;
    Ok(AssetRef::new(hash))
}

/// `base` with the opacity of `cut` drawn out to its size.
fn with_opacity_of(base: &Pixels, cut: &Pixels) -> Option<Pixels> {
    let (w, h) = base.size();
    let (cw, ch) = cut.size();
    let alpha = cut.rgba().chunks_exact(4).map(|p| p[3]).collect();
    let alpha = image::GrayImage::from_raw(cw, ch, alpha)?;
    let alpha = image::imageops::resize(&alpha, w, h, image::imageops::FilterType::Triangle);
    let mut rgba = base.rgba().to_vec();
    for (pixel, a) in rgba.chunks_exact_mut(4).zip(alpha.pixels()) {
        pixel[3] = pixel[3].min(a.0[0]);
    }
    Pixels::new(w, h, rgba)
}

/// The image after `work`.
fn worked(
    models: &dyn VisionModels,
    store: &dyn ContentStore,
    image: ImageNode,
    work: Work,
    progress: &dyn Progress,
    cancel: &CancelToken,
) -> Result<ImageNode, VisionError> {
    let base = decoded(store, image.base())?;
    let check = || cancel.check().map_err(|_| VisionError::Cancelled);
    check()?;
    match work {
        Work::Cut => {
            let remover = models.remover()?;
            // Without it, the background goes all the same (S84-02).
            let upscaler = models.upscaler().ok();
            let out = remove_background(&base, &*remover, upscaler.as_deref(), progress, cancel)?;
            let whole = out.enhanced.as_ref().unwrap_or(&base);
            let made = cut(whole, &out.mask).ok_or(VisionError::TooLarge)?;
            check()?;
            let made = stored(store, made)?;
            let base = match out.enhanced {
                Some(enhanced) => stored(store, enhanced)?,
                None => image.base(),
            };
            Ok(image.edited(made, base))
        }
        Work::Enhance(factor) => {
            let better = models
                .upscaler()?
                .upscale(&base, factor, progress, cancel)?;
            check()?;
            let cut = image.is_cut().then(|| decoded(store, image.asset));
            let made = match cut {
                Some(cut) => {
                    let made = with_opacity_of(&better, &cut?).ok_or(VisionError::TooLarge)?;
                    Some(stored(store, made)?)
                }
                None => None,
            };
            let base = stored(store, better)?;
            Ok(image.edited(made.unwrap_or(base), base))
        }
    }
}

impl Studio {
    pub fn set_models(&mut self, models: Option<Arc<dyn VisionModels>>) {
        self.models = models;
        self.relist();
    }

    pub fn models(&self) -> Option<Arc<dyn VisionModels>> {
        self.models.clone()
    }

    pub fn available(&self) -> bool {
        self.models.is_some()
    }

    pub fn busy(&self) -> bool {
        self.job.is_some()
    }

    fn relist(&self) {
        *self.listed.write().unwrap_or_else(PoisonError::into_inner) = None;
    }

    /// The share of the work on `node`, if there is one to show on it.
    pub fn scanning(&self, node: NodeId) -> Option<f32> {
        let job = self.job.as_ref().filter(|job| job.scan)?;
        (job.target.as_ref()?.node == node).then(|| job.progress.share())
    }

    pub fn cancel(&self) {
        if let Some(job) = &self.job {
            job.cancel.cancel();
        }
    }

    fn start(
        &mut self,
        ctx: Option<&egui::Context>,
        (target, scan): (Option<Target>, bool),
        run: impl FnOnce(&dyn Progress, &CancelToken) -> Message + Send + 'static,
    ) {
        if self.job.is_some() {
            return;
        }
        self.error = None;
        let progress = Arc::new(Meter {
            share: AtomicU32::new(0),
            ctx: ctx.cloned(),
        });
        let cancel = CancelToken::new();
        self.job = Some(Job {
            target,
            scan,
            progress: Arc::clone(&progress),
            cancel: cancel.clone(),
        });
        let (tx, ctx) = (self.tx.clone(), ctx.cloned());
        std::thread::spawn(move || {
            let message = run(&*progress, &cancel);
            let _ = tx.send(message);
            if let Some(ctx) = ctx {
                ctx.request_repaint();
            }
        });
    }

    /// Does `work` to `target`, on a thread.
    pub fn work(&mut self, store: &Arc<dyn ContentStore>, target: Target, work: Work) {
        let Some(models) = self.models.clone() else {
            return;
        };
        let store = Arc::clone(store);
        let ctx = self.ctx.clone();
        self.start(
            ctx.as_ref(),
            (Some(target.clone()), true),
            move |progress, cancel| {
                let image = target.image.clone();
                let result = worked(&*models, &*store, image, work, progress, cancel);
                Message::Made { target, result }
            },
        );
    }

    /// Fills what is marked of `target`, on a thread.
    pub fn fill(&mut self, store: &Arc<dyn ContentStore>, target: Target, marked: Arc<Marked>) {
        let Some(models) = self.models.clone() else {
            return;
        };
        let store = Arc::clone(store);
        let ctx = self.ctx.clone();
        self.start(
            ctx.as_ref(),
            (Some(target.clone()), true),
            move |progress, cancel| {
                let image = &target.image;
                let result = filled(&*models, &*store, image, &marked, progress, cancel);
                Message::Made { target, result }
            },
        );
    }

    /// The models that fill: the one in use, or the one to download.
    pub fn inpainting(&self) -> Option<ModelInfo> {
        let listed = self.listed();
        let of = || {
            let all = listed.models.iter();
            all.filter(|m| m.task == Some(VisionTask::Inpaint))
        };
        of().find(|m| m.chosen)
            .or_else(|| of().find(|m| m.recommended))
            .or_else(|| of().next())
            .cloned()
    }

    /// How far a download is.
    pub fn downloading(&self) -> Option<f32> {
        let job = self.job.as_ref().filter(|job| job.target.is_none())?;
        Some(job.progress.share())
    }

    /// Opens `target` to erase pixels of it.
    pub fn open(&mut self, store: &Arc<dyn ContentStore>, target: Target) {
        let store = Arc::clone(store);
        let ctx = self.ctx.clone();
        self.start(ctx.as_ref(), (Some(target.clone()), false), move |_, _| {
            let result = decoded(&*store, target.image.base()).and_then(|base| {
                let cut = decoded(&*store, target.image.asset)?;
                let alpha = cut.rgba().chunks_exact(4).map(|p| p[3]).collect();
                Ok(Box::new((base, alpha)))
            });
            Message::Opened { target, result }
        });
    }

    /// Keeps the image erased by hand.
    pub fn keep(&mut self, store: &Arc<dyn ContentStore>, target: Target, made: Pixels) {
        let store = Arc::clone(store);
        let ctx = self.ctx.clone();
        self.start(ctx.as_ref(), (Some(target.clone()), false), move |_, _| {
            let image = target.image.clone();
            let result = stored(&*store, made).map(|made| image.edited(made, image.base()));
            Message::Made { target, result }
        });
    }

    /// Downloads `ids`, one after the other, and uses the models among them.
    pub fn install(&mut self, ids: Vec<&'static str>) {
        let Some(models) = self.models.clone() else {
            return;
        };
        let ctx = self.ctx.clone();
        self.start(ctx.as_ref(), (None, false), move |progress, cancel| {
            let count = ids.len().max(1) as f32;
            let one = |(i, id): (usize, &&'static str)| {
                let part = Part {
                    whole: progress,
                    from: i as f32 / count,
                    share: 1.0 / count,
                };
                models.install(id, &part, cancel)?;
                // A runtime is used by all of them.
                if models.models().iter().any(|m| m.id == *id) {
                    models.choose(id)?;
                }
                Ok(())
            };
            let result = ids.iter().enumerate().try_for_each(one);
            Message::Installed { result }
        });
    }

    /// What the threads made.
    pub fn pump(&mut self, ctx: &egui::Context) -> Vec<Arrived> {
        self.ctx.get_or_insert_with(|| ctx.clone());
        let mut arrived = Vec::new();
        while let Ok(message) = self.rx.try_recv() {
            self.job = None;
            // What is measured, or installed, changed.
            self.relist();
            let error = match message {
                Message::Made { target, result } => match result {
                    Ok(image) => {
                        arrived.push(Arrived::Image(target, image));
                        None
                    }
                    Err(error) => Some(error),
                },
                Message::Opened { target, result } => match result {
                    Ok(opened) => {
                        arrived.push(Arrived::Erase(target, opened));
                        None
                    }
                    Err(error) => Some(error),
                },
                Message::Installed { result } => result.err(),
            };
            self.error = error.filter(|e| *e != VisionError::Cancelled);
        }
        arrived
    }

    fn listed(&self) -> Arc<Listed> {
        let read = self.listed.read().unwrap_or_else(PoisonError::into_inner);
        if let Some(listed) = &*read {
            return Arc::clone(listed);
        }
        drop(read);
        let listed = Arc::new(match &self.models {
            Some(models) => Listed {
                models: models.models(),
                runtimes: models.runtimes(),
            },
            None => Listed {
                models: Vec::new(),
                runtimes: Vec::new(),
            },
        });
        *self.listed.write().unwrap_or_else(PoisonError::into_inner) = Some(Arc::clone(&listed));
        listed
    }
}

/// A part of a download of several.
struct Part<'a> {
    whole: &'a dyn Progress,
    from: f32,
    share: f32,
}

impl Progress for Part<'_> {
    fn report(&self, fraction: f32) {
        self.whole.report(self.from + self.share * fraction);
    }
}

/// What is said of an error: what happened and what to do.
fn message(error: &VisionError) -> &'static str {
    match error {
        VisionError::Cancelled => "vision.error_cancelled",
        VisionError::Offline => "vision.error_offline",
        VisionError::Corrupt => "vision.error_corrupt",
        VisionError::Unavailable(_) => "vision.error_unavailable",
        VisionError::TooLarge => "vision.error_too_large",
        VisionError::Io(_) => "vision.error_io",
        VisionError::Model(_) => "vision.error_model",
    }
}

/// What is marked of an image, at a size of its own.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Marked {
    pub alpha: Vec<u8>,
    pub size: (u32, u32),
}

/// The image without what is marked: it, and what was cut out of it, as they
/// were elsewhere.
fn filled(
    models: &dyn VisionModels,
    store: &dyn ContentStore,
    image: &ImageNode,
    marked: &Marked,
    progress: &dyn Progress,
    cancel: &CancelToken,
) -> Result<ImageNode, VisionError> {
    let base = decoded(store, image.base())?;
    let cut = image.is_cut().then(|| decoded(store, image.asset));
    let cut = cut.transpose()?;
    let (w, h) = base.size();
    let (mw, mh) = marked.size;
    let mask = image::GrayImage::from_raw(mw, mh, marked.alpha.clone());
    let mask = mask.ok_or(VisionError::TooLarge)?;
    let mask = image::imageops::resize(&mask, w, h, image::imageops::FilterType::Triangle);
    let mask = gafira_raster::AlphaMask::new(w, h, mask.into_raw());
    let mask = mask.map_err(|_| VisionError::TooLarge)?;
    let made = models.inpainter()?.fill(&base, &mask, progress, cancel)?;
    cancel.check().map_err(|_| VisionError::Cancelled)?;
    let shown = match cut {
        Some(cut) => {
            let cut = with_opacity_of(&made, &cut).ok_or(VisionError::TooLarge)?;
            Some(stored(store, cut)?)
        }
        None => None,
    };
    let base = stored(store, made)?;
    Ok(image.edited(shown.unwrap_or(base), base))
}

/// What the person asked for in Propiedades.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Ask {
    Work(Target, Work),
    Erase(Target),
    /// Fill what is marked with what is around it (spec 086).
    Fill(Target, Arc<Marked>),
    /// The image as it was before, its look as it is.
    Back(Target, ImageNode),
    Install(&'static str),
    /// The whole image back, at its better resolution.
    Background(Target),
    Original(Target),
    Cancel,
    Models,
}

/// The buttons of an image in Propiedades.
pub(crate) fn buttons(
    ui: &mut Ui,
    theme: &Theme,
    texts: &Texts,
    studio: &Studio,
    target: Target,
) -> Option<Ask> {
    if !studio.available() {
        return None;
    }
    let mut ask = None;
    ui.label(RichText::new(plain(texts, "vision.title")).font(theme.font("ui-label")));
    ui.add_space(6.0);
    let wide = Vec2::new(ui.available_width(), 34.0);
    let button = |ui: &mut Ui, on: bool, key: &'static str, hint: &'static str| {
        ui.add_enabled_ui(on, |ui| {
            ui.add_sized(wide, egui::Button::new(plain(texts, key)))
                .on_hover_text(plain(texts, hint))
                .clicked()
        })
        .inner
    };
    let free = !studio.busy();
    let key = if target.image.is_cut() {
        "vision.cut_again"
    } else {
        "vision.cut"
    };
    if button(ui, free, key, "vision.cut_hint") {
        ask = Some(Ask::Work(target.clone(), Work::Cut));
    }
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        let half = Vec2::new((wide.x - 8.0) / 2.0, 34.0);
        for factor in [2, 4] {
            let text = format!("{} ×{factor}", plain(texts, "vision.enhance"));
            let button = egui::Button::new(text).min_size(half);
            let pressed = ui
                .add_enabled(free, button)
                .on_hover_text(plain(texts, "vision.enhance_hint"));
            if pressed.clicked() {
                ask = Some(Ask::Work(target.clone(), Work::Enhance(factor)));
            }
        }
    });
    if target.image.is_cut() {
        if button(ui, free, "vision.erase", "vision.erase_hint") {
            ask = Some(Ask::Erase(target.clone()));
        }
        if button(ui, free, "vision.background", "vision.background_hint") {
            ask = Some(Ask::Background(target.clone()));
        }
    }
    if target.image.edit.is_some() && button(ui, free, "vision.original", "vision.original_hint") {
        ask = Some(Ask::Original(target.clone()));
    }
    ask = progress(ui, theme, texts, studio).or(ask);
    ui.add_space(4.0);
    let text = RichText::new(plain(texts, "vision.models")).font(theme.font("body-sm"));
    let link = ui
        .add(egui::Button::new(text).frame(false))
        .on_hover_cursor(egui::CursorIcon::PointingHand);
    if link.clicked() {
        ask = Some(Ask::Models);
    }
    ask
}

/// How far the work is, and what is said of the last one.
pub(crate) fn progress(ui: &mut Ui, theme: &Theme, texts: &Texts, studio: &Studio) -> Option<Ask> {
    let mut ask = None;
    if let Some(job) = studio.job.as_ref().filter(|job| job.target.is_some()) {
        ui.add_space(6.0);
        bar(ui, theme, job.progress.share());
        ui.horizontal(|ui| {
            let text = if job.progress.share() < 0.01 {
                plain(texts, "vision.preparing")
            } else {
                plain(texts, "vision.working")
            };
            ui.label(RichText::new(text).font(theme.font("body-sm")));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button(plain(texts, "vision.cancel")).clicked() {
                    ask = Some(Ask::Cancel);
                }
            });
        });
    }
    said(ui, theme, texts, studio);
    ask
}

/// The error, if there is one.
fn said(ui: &mut Ui, theme: &Theme, texts: &Texts, studio: &Studio) {
    if let Some(error) = &studio.error {
        ui.add_space(4.0);
        let text = RichText::new(plain(texts, message(error))).font(theme.font("body-sm"));
        crate::widgets::note(ui, crate::icons::Icon::Warning, text, theme.c("error.fg"));
    }
}

pub(crate) fn bar(ui: &mut Ui, theme: &Theme, share: f32) {
    let size = Vec2::new(ui.available_width(), 6.0);
    let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
    let painter = ui.painter();
    painter.rect_filled(rect, 3.0, theme.c("surface.sunken"));
    let mut done = rect;
    done.max.x = rect.min.x + rect.width() * share.clamp(0.02, 1.0);
    painter.rect_filled(done, 3.0, theme.c("accent"));
}

// ---- The models ----------------------------------------------------------

/// A count of bytes, for the person.
pub(crate) fn size(bytes: u64) -> String {
    // Megabytes: the cast loses nothing seen.
    let mb = bytes as f64 / 1e6;
    if mb < 10.0 {
        format!("{mb:.1} MB")
    } else {
        format!("{mb:.0} MB")
    }
}

/// What the panel of the models asks for.
enum Pick {
    Install(Vec<&'static str>),
    Use(&'static str),
    Remove(&'static str),
    Cancel,
}

impl Studio {
    pub fn open_models(&mut self) {
        self.error = None;
        self.panel = true;
    }

    /// «Modelos de imagen», over everything.
    pub fn models_panel(&mut self, ctx: &egui::Context, theme: &Theme, texts: &Texts) {
        let Some(models) = self.models.clone().filter(|_| self.panel) else {
            return;
        };
        let listed = self.listed();
        let mut pick = None;
        let mut close = false;
        let frame = egui::Frame::popup(&ctx.style()).inner_margin(egui::Margin::same(24));
        let modal = egui::Modal::new(egui::Id::new("vision_models")).frame(frame);
        let tall = (ctx.content_rect().height() - 260.0).clamp(160.0, 560.0);
        let response = modal.show(ctx, |ui| {
            ui.set_width(600.0);
            ui.label(RichText::new(plain(texts, "vision.models_title")).font(theme.font("h3")));
            ui.add_space(4.0);
            let machine = models.machine();
            let gpu = machine.gpu.as_ref().map(|gpu| gpu.name.clone());
            let gpu = gpu.unwrap_or_else(|| plain(texts, "vision.no_gpu"));
            let memory = if machine.memory_bytes > 0 {
                format!(" · {} GB", machine.memory_bytes.div_ceil(1 << 30))
            } else {
                String::new()
            };
            let threads = plain(texts, "vision.threads");
            let text = format!("{} {threads}{memory} · {gpu}", machine.threads);
            let text = RichText::new(text).font(theme.font("body-sm"));
            ui.label(text.color(theme.c("text.secondary")));
            ui.add_space(10.0);
            pick = self.recommended(ui, theme, texts, &listed);
            egui::ScrollArea::vertical()
                .max_height(tall)
                .auto_shrink([false, true])
                .show(ui, |ui| {
                    ui.set_max_width(ui.available_width() - 12.0);
                    let groups = [
                        ("vision.group_cut", Some(VisionTask::Cutout)),
                        ("vision.group_enhance", Some(VisionTask::Upscale)),
                        ("vision.group_fill", Some(VisionTask::Inpaint)),
                        ("vision.group_speed", None),
                    ];
                    for (key, task) in groups {
                        let all = listed.models.iter().chain(&listed.runtimes);
                        let of: Vec<&ModelInfo> = all.filter(|m| m.task == task).collect();
                        if of.is_empty() {
                            continue;
                        }
                        ui.add_space(8.0);
                        ui.label(RichText::new(plain(texts, key)).font(theme.font("ui-label")));
                        for model in of {
                            if let Some(asked) = row(ui, theme, texts, model, !self.busy()) {
                                pick = Some(asked);
                            }
                        }
                    }
                });
            if let Some(job) = self.job.as_ref().filter(|job| job.target.is_none()) {
                ui.add_space(8.0);
                bar(ui, theme, job.progress.share());
                ui.horizontal(|ui| {
                    let share = (job.progress.share() * 100.0).round();
                    ui.label(format!("{} {share} %", plain(texts, "vision.downloading")));
                    if ui.button(plain(texts, "vision.cancel")).clicked() {
                        pick = Some(Pick::Cancel);
                    }
                });
            }
            said(ui, theme, texts, self);
            ui.add_space(12.0);
            ui.horizontal(|ui| {
                let text = plain(texts, "vision.close");
                close = widgets::primary(ui, theme, text).clicked();
                let note = RichText::new(plain(texts, "vision.private"))
                    .font(theme.font("caption"))
                    .color(theme.c("text.secondary"));
                ui.label(note);
            });
        });
        match pick {
            Some(Pick::Install(ids)) => self.install(ids),
            Some(Pick::Use(id)) => {
                self.error = models.choose(id).err();
                self.relist();
            }
            Some(Pick::Remove(id)) => {
                self.error = models.uninstall(id).err();
                self.relist();
            }
            Some(Pick::Cancel) => self.cancel(),
            None => {}
        }
        // A download goes on with the panel closed.
        if close || response.should_close() {
            self.panel = false;
        }
    }

    /// «Preparar para este equipo»: what is right for this machine and is
    /// not in use yet.
    fn recommended(
        &self,
        ui: &mut Ui,
        theme: &Theme,
        texts: &Texts,
        listed: &Listed,
    ) -> Option<Pick> {
        let wanted: Vec<&ModelInfo> = listed
            .runtimes
            .iter()
            .chain(&listed.models)
            .filter(|m| m.recommended && !m.chosen)
            .collect();
        if wanted.is_empty() {
            let text = RichText::new(plain(texts, "vision.ready"));
            crate::widgets::note(ui, crate::icons::Icon::Check, text, theme.c("success.fg"));
            return None;
        }
        let missing = wanted.iter().filter(|m| !m.installed);
        let bytes: u64 = missing.map(|m| m.download_bytes).sum();
        let names: Vec<&str> = wanted.iter().map(|m| m.name).collect();
        ui.label(plain(texts, "vision.recommended"));
        ui.label(RichText::new(names.join(" · ")).strong());
        ui.add_space(6.0);
        let text = if bytes > 0 {
            format!("{} ({})", plain(texts, "vision.prepare"), size(bytes))
        } else {
            plain(texts, "vision.use")
        };
        let clicked = ui
            .add_enabled_ui(!self.busy(), |ui| widgets::primary(ui, theme, text))
            .inner
            .clicked();
        clicked.then(|| Pick::Install(wanted.iter().map(|m| m.id).collect()))
    }
}

fn row(ui: &mut Ui, theme: &Theme, texts: &Texts, model: &ModelInfo, free: bool) -> Option<Pick> {
    let mut pick = None;
    egui::Frame::NONE
        .fill(theme.c("surface.sunken"))
        .corner_radius(theme.radius_md())
        .inner_margin(egui::Margin::same(10))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.set_width(ui.available_width() - 200.0);
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(model.name).strong());
                        if model.recommended {
                            let text = RichText::new(plain(texts, "vision.right"));
                            let text = text.font(theme.font("caption"));
                            ui.label(text.color(theme.c("success.fg")));
                        }
                    });
                    ui.label(plain(texts, model.about));
                    let mut facts = vec![if model.bundled {
                        plain(texts, "vision.bundled")
                    } else {
                        size(model.download_bytes)
                    }];
                    facts.push(model.license.to_owned());
                    if let Some((ms, on)) = model.measured {
                        facts.push(format!("{ms} ms · {on}"));
                    }
                    let text = RichText::new(facts.join(" · ")).font(theme.font("caption"));
                    ui.label(text.color(theme.c("text.secondary")))
                        .on_hover_text(model.source);
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let button = |ui: &mut Ui, key: &'static str| {
                        ui.add_enabled(free, egui::Button::new(plain(texts, key)))
                            .clicked()
                    };
                    if !model.installed {
                        if button(ui, "vision.download") {
                            pick = Some(Pick::Install(vec![model.id]));
                        }
                        return;
                    }
                    if !model.bundled && button(ui, "vision.delete") {
                        pick = Some(Pick::Remove(model.id));
                    }
                    if model.chosen {
                        let text = RichText::new(plain(texts, "vision.in_use"));
                        let ink = theme.c("success.fg");
                        crate::widgets::note(ui, crate::icons::Icon::Check, text, ink);
                    } else if button(ui, "vision.use") {
                        pick = Some(Pick::Use(model.id));
                    }
                });
            });
        });
    ui.add_space(4.0);
    pick
}

// ---- The effect ----------------------------------------------------------

/// How long the scan takes to go across, in seconds.
const ACROSS: f64 = 1.8;

/// The share of the image its band takes.
const BAND: f64 = 0.26;

/// The scan over an image being worked on (S84-07): the image in the dark, a
/// band of the brand's mint that goes across and lights it, and a halo
/// around. `still`
/// keeps the halo alone, for those who asked for less motion.
pub(crate) fn scan(
    painter: &egui::Painter,
    theme: &Theme,
    texture: Option<egui::TextureId>,
    (turn, placement): (Affine, &Placement),
    time: f64,
    still: bool,
) {
    let (w, h) = (placement.w, placement.h);
    let mint = theme.c("accent");
    let at = |u: f64, v: f64| rotation::screen(turn, u * w, v * h);
    let quad = |texture, u0: f64, u1: f64, a: Color32, b: Color32| {
        let mut mesh = match texture {
            Some(id) => egui::Mesh::with_texture(id),
            None => egui::Mesh::default(),
        };
        for (u, v, color) in [(u0, 0.0, a), (u1, 0.0, b), (u1, 1.0, b), (u0, 1.0, a)] {
            mesh.vertices.push(egui::epaint::Vertex {
                pos: at(u, v),
                uv: if texture.is_some() {
                    Pos2::new(u as f32, v as f32)
                } else {
                    egui::epaint::WHITE_UV
                },
                color,
            });
        }
        mesh.add_triangle(0, 1, 2);
        mesh.add_triangle(0, 2, 3);
        mesh
    };
    let dark = theme.c("topbar.bg").gamma_multiply(0.62);
    painter.add(quad(None, 0.0, 1.0, dark, dark));
    let pulse = if still {
        0.7
    } else {
        0.55 + 0.45 * (time * 2.6).sin().abs()
    };
    if !still {
        // The band comes in from one side and leaves by the other.
        let head = (time / ACROSS).fract() * (1.0 + BAND);
        let (tail, end) = ((head - BAND).max(0.0), head.min(1.0));
        if end > tail {
            let share = |u: f64| ((u - (head - BAND)) / BAND).clamp(0.0, 1.0) as f32;
            let of = |color: Color32, most: f32| {
                let faded = |u: f64| color.gamma_multiply(most * share(u).powi(2));
                (faded(tail), faded(end))
            };
            if texture.is_some() {
                let (a, b) = of(Color32::WHITE, 0.9);
                painter.add(quad(texture, tail, end, a, b));
            }
            let (a, b) = of(mint, 0.72);
            painter.add(quad(None, tail, end, a, b));
            if head <= 1.0 {
                let line = Stroke::new(3.0, mint);
                painter.line_segment([at(head, 0.0), at(head, 1.0)], line);
            }
        }
    }
    let corners = [at(0.0, 0.0), at(1.0, 0.0), at(1.0, 1.0), at(0.0, 1.0)];
    for (width, share) in [(14.0, 0.10), (9.0, 0.18), (5.0, 0.32), (2.0, 1.0)] {
        // Within 0..=1: the cast loses nothing seen.
        let color = mint.gamma_multiply((share * pulse) as f32);
        let stroke = Stroke::new(width, color);
        painter.add(egui::Shape::closed_line(corners.to_vec(), stroke));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // S84-01: the opacity of a cut image goes with its better resolution;
    // what was see-through stays so.
    #[test]
    fn s84_01_the_opacity_goes_along() {
        let base = Pixels::new(4, 2, [9, 9, 9, 200].repeat(8)).unwrap();
        let cut = Pixels::new(2, 1, vec![1, 1, 1, 0, 1, 1, 1, 255]).unwrap();
        let out = with_opacity_of(&base, &cut).unwrap();
        assert_eq!((out.rgba()[3], out.rgba()[15]), (0, 200));
        assert_eq!(out.rgba()[0], 9);
    }

    /// Contents in memory.
    #[derive(Default)]
    struct Contents(std::sync::Mutex<std::collections::HashMap<ContentHash, Vec<u8>>>);

    impl ContentStore for Contents {
        fn put(
            &self,
            src: &mut dyn std::io::Read,
        ) -> Result<ContentHash, gafira_ports::store::StoreError> {
            let mut bytes = Vec::new();
            src.read_to_end(&mut bytes).expect("read");
            let hash = ContentHash(*blake3::hash(&bytes).as_bytes());
            self.0.lock().unwrap().insert(hash, bytes);
            Ok(hash)
        }

        fn open(
            &self,
            hash: ContentHash,
        ) -> Result<Box<dyn gafira_ports::system::ReadSeek + Send>, gafira_ports::store::StoreError>
        {
            let bytes = self.0.lock().unwrap()[&hash].clone();
            Ok(Box::new(std::io::Cursor::new(bytes)))
        }

        fn contains(&self, hash: ContentHash) -> Result<bool, gafira_ports::store::StoreError> {
            Ok(self.0.lock().unwrap().contains_key(&hash))
        }
    }

    /// Keeps the left half, and makes an image larger in one colour.
    struct Fake;

    impl gafira_ports::vision::BackgroundRemover for Fake {
        fn mask(
            &self,
            image: &Pixels,
            _: &dyn Progress,
            _: &CancelToken,
        ) -> Result<gafira_raster::AlphaMask, VisionError> {
            let (w, h) = image.size();
            let alpha = (0..w * h).map(|i| if i % w < w / 2 { 255 } else { 0 });
            gafira_raster::AlphaMask::new(w, h, alpha.collect()).map_err(|_| VisionError::TooLarge)
        }
    }

    impl gafira_ports::vision::Upscaler for Fake {
        fn most(&self) -> u32 {
            4
        }

        fn upscale(
            &self,
            image: &Pixels,
            factor: u32,
            _: &dyn Progress,
            _: &CancelToken,
        ) -> Result<Pixels, VisionError> {
            let (w, h) = (image.size().0 * factor, image.size().1 * factor);
            Pixels::new(w, h, [1, 2, 3, 255].repeat((w * h) as usize)).ok_or(VisionError::TooLarge)
        }
    }

    impl VisionModels for Fake {
        fn machine(&self) -> gafira_ports::vision::Machine {
            gafira_ports::vision::Machine::default()
        }
        fn models(&self) -> Vec<ModelInfo> {
            Vec::new()
        }
        fn runtimes(&self) -> Vec<ModelInfo> {
            Vec::new()
        }
        fn install(&self, _: &str, _: &dyn Progress, _: &CancelToken) -> Result<(), VisionError> {
            Ok(())
        }
        fn uninstall(&self, _: &str) -> Result<(), VisionError> {
            Ok(())
        }
        fn choose(&self, _: &str) -> Result<(), VisionError> {
            Ok(())
        }
        fn remover(&self) -> Result<Arc<dyn gafira_ports::vision::BackgroundRemover>, VisionError> {
            Ok(Arc::new(Fake))
        }
        fn upscaler(&self) -> Result<Arc<dyn gafira_ports::vision::Upscaler>, VisionError> {
            Ok(Arc::new(Fake))
        }
        fn inpainter(&self) -> Result<Arc<dyn gafira_ports::vision::Inpainter>, VisionError> {
            Ok(Arc::new(Fake))
        }
    }

    /// Fills in green.
    impl gafira_ports::vision::Inpainter for Fake {
        fn fill(
            &self,
            image: &Pixels,
            mask: &gafira_raster::AlphaMask,
            _: &dyn Progress,
            _: &CancelToken,
        ) -> Result<Pixels, VisionError> {
            let (w, h) = image.size();
            let mut rgba = image.rgba().to_vec();
            for (px, m) in rgba.chunks_exact_mut(4).zip(mask.data()) {
                if *m > 128 {
                    px[..3].copy_from_slice(&[0, 255, 0]);
                }
            }
            Pixels::new(w, h, rgba).ok_or(VisionError::TooLarge)
        }
    }

    // S84-01, S84-02: a poor image is made better and cut; the better one
    // and the original are kept, and cutting goes on from the better one.
    #[test]
    fn s84_01_what_is_kept() {
        let store = Contents::default();
        let run = |image, work| {
            let (progress, cancel) = (gafira_doc::contract::NoProgress, CancelToken::new());
            worked(&Fake, &store, image, work, &progress, &cancel).expect("made")
        };
        let photo = Pixels::new(200, 100, [9, 8, 7, 255].repeat(20_000)).unwrap();
        let start = ImageNode::new(stored(&store, photo.clone()).unwrap());
        let cut = run(start.clone(), Work::Cut);
        assert!(cut.is_cut() && cut.is_enhanced());
        assert_eq!(cut.original(), start.asset);
        assert_eq!(decoded(&store, cut.original()).unwrap(), photo);
        let made = decoded(&store, cut.asset).unwrap();
        assert_eq!(made.size(), (800, 400));
        assert_eq!((made.rgba()[3], made.rgba()[799 * 4 + 3]), (255, 0));
        assert_eq!(made.rgba()[799 * 4], 1, "its colour is kept");
        // Better still: what was cut stays cut.
        let better = run(cut.clone(), Work::Enhance(2));
        let made = decoded(&store, better.asset).unwrap();
        assert_eq!(made.size(), (1600, 800));
        assert_eq!((made.rgba()[3], made.rgba()[1599 * 4 + 3]), (255, 0));
        assert_eq!(better.original(), start.asset);
        let whole = run(start.clone(), Work::Enhance(2));
        assert!(whole.is_enhanced() && !whole.is_cut());
    }

    // S86-05: what is marked is filled in the base and in what was cut out
    // of it, which keeps its opacity; the original stays.
    #[test]
    fn s86_05_what_is_filled() {
        let store = Contents::default();
        let photo = Pixels::new(4, 2, [200, 0, 0, 255].repeat(8)).unwrap();
        let start = ImageNode::new(stored(&store, photo).unwrap());
        let run = |image: &ImageNode| {
            let marked = Marked {
                alpha: vec![255, 0],
                size: (2, 1),
            };
            let token = CancelToken::new();
            let no = gafira_doc::contract::NoProgress;
            filled(&Fake, &store, image, &marked, &no, &token).unwrap()
        };
        let whole = run(&start);
        assert_eq!(whole.original(), start.asset);
        assert!(!whole.is_cut());
        let pixels = decoded(&store, whole.asset).unwrap();
        assert_eq!(pixels.rgba()[..4], [0, 255, 0, 255]);
        assert_eq!(pixels.rgba()[12..16], [200, 0, 0, 255]);
        let half: Vec<u8> = (0..8)
            .flat_map(|i| [200, 0, 0, if i % 4 == 0 { 0 } else { 255 }])
            .collect();
        let half = stored(&store, Pixels::new(4, 2, half).unwrap()).unwrap();
        let cut = run(&start.edited(half, start.asset));
        assert!(cut.is_cut() && cut.original() == start.asset);
        let pixels = decoded(&store, cut.asset).unwrap();
        assert_eq!(pixels.rgba()[..8], [0, 255, 0, 0, 0, 255, 0, 255]);
    }

    // S84-05: sizes are read at a glance.
    #[test]
    fn s84_05_sizes() {
        assert_eq!(size(4_574_861), "4.6 MB");
        assert_eq!(size(175_997_641), "176 MB");
    }
}
