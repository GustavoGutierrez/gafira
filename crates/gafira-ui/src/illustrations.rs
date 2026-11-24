//! Elementos → Ilustraciones (spec 072): vector illustrations from
//! collections free to use, also commercially and without credit in the
//! design, in rows by theme as in the reference, a search, and «Ver todo».
//! A click inserts one as vector shapes; a drag carries it (D-105). Every
//! call to the catalogue runs on its own thread.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender, channel};

use egui::{Align2, Color32, Pos2, Rect, Sense, Stroke, Vec2};
use gafira_ports::photos::{
    Illustration, IllustrationCatalog, IllustrationStyle, Photo, PhotoCatalog, PhotoError,
    PhotoPage, PhotoQuery, PhotoSource,
};

use crate::style::Theme;
use crate::text::Texts;

/// A row of the panel: its name's catalogue key and what fills it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum Row {
    /// Illustrations whose names match these words.
    Words(&'static str, &'static str),
    /// The first ones of a whole collection.
    Set(&'static str, &'static str),
}

impl Row {
    fn key(self) -> &'static str {
        match self {
            Self::Words(key, _) | Self::Set(key, _) => key,
        }
    }
}

/// The rows, in the order of the panel.
pub(crate) const ROWS: [Row; 17] = [
    // Bundled with Gafira (CC0): there without network.
    Row::Set("illustrations.open_peeps", "open-peeps"),
    Row::Set("illustrations.open_doodles", "open-doodles"),
    Row::Words("illustrations.flowers", "flower"),
    Row::Words("illustrations.leaves", "leaf"),
    Row::Words("illustrations.trees", "tree"),
    Row::Words("illustrations.hearts", "heart"),
    Row::Words("illustrations.stars", "star"),
    Row::Set("illustrations.weather", "meteocons"),
    Row::Words("illustrations.cats", "cat"),
    Row::Words("illustrations.dogs", "dog"),
    Row::Words("illustrations.sweets", "cake"),
    Row::Words("illustrations.music", "music"),
    Row::Words("illustrations.books", "book"),
    Row::Words("illustrations.cars", "car"),
    Row::Set("illustrations.office", "flat-color-icons"),
    // In black and white.
    Row::Set("illustrations.line_art", "fluent-emoji-high-contrast"),
    Row::Set("illustrations.health", "healthicons"),
];

/// How many a row shows, and «Ver todo» or a search.
const ROW_SIZE: u32 = 8;
const ALL_SIZE: u32 = 96;

/// The longer side of a thumbnail, in px.
const THUMB_SIDE: f32 = 160.0;

/// The most SVGs on their way at once.
const AT_ONCE: usize = 8;

/// How long the words rest before they are searched.
const DEBOUNCE_S: f64 = 0.4;

/// What fills a list: a row, or the words of a search.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum List {
    Row(Row),
    Search(String),
}

/// A list on its way, here, or failed.
enum Listed {
    Loading,
    Ready(Vec<Illustration>),
    Failed(PhotoError),
}

/// An illustration's drawing: on its way, ready (its SVG and thumbnail),
/// or failed.
enum Svg {
    Loading,
    Ready(Arc<Vec<u8>>, egui::TextureHandle),
    Failed,
}

enum Message {
    List(
        (List, IllustrationStyle, u32),
        Result<Vec<Illustration>, PhotoError>,
    ),
    Svg(Illustration, Option<(Vec<u8>, egui::ColorImage)>),
    Cutouts(&'static str, Result<PhotoPage, PhotoError>),
    CutoutThumb(String, Option<egui::ColorImage>),
}

/// What to insert, and where it was let go (screen).
pub(crate) enum Chosen {
    /// A vector illustration: its SVG, inserted as shapes.
    Drawing {
        illustration: Illustration,
        svg: Arc<Vec<u8>>,
        at: Option<Pos2>,
    },
    /// A cut-out illustration of rawpixel, inserted as an image.
    Cutout { photo: Photo, at: Option<Pos2> },
}

/// The rows of «Ilustraciones detalladas» (owner request, with examples):
/// rawpixel's public-domain cut-outs on a transparent ground, through
/// Openverse. Their names' keys and the words searched.
pub(crate) const CUTOUTS: [(&str, &str); 8] = [
    ("illustrations.cut_plants", "fern"),
    ("illustrations.cut_tropical", "monstera"),
    ("illustrations.cut_trees", "tree"),
    ("illustrations.cut_flowers", "flower"),
    ("illustrations.cut_rocks", "rock"),
    ("illustrations.cut_mushrooms", "mushroom"),
    ("illustrations.cut_birds", "bird"),
    ("illustrations.cut_butterflies", "butterfly"),
];

/// A thumbnail of a cut-out: on its way, here, or failed.
enum CutThumb {
    Loading,
    Ready(egui::TextureHandle),
    Failed,
}

/// The state of the panel for the editor's session.
pub(crate) struct IllustrationsPanel {
    catalog: Option<Arc<dyn IllustrationCatalog>>,
    /// The photo catalogue the cut-outs come from.
    photos: Option<Arc<dyn PhotoCatalog>>,
    cutouts: HashMap<&'static str, Result<Vec<Photo>, PhotoError>>,
    cutouts_asked: std::collections::HashSet<&'static str>,
    cut_thumbs: HashMap<String, CutThumb>,
    ctx: Option<egui::Context>,
    text: String,
    typed_at: Option<f64>,
    /// The row open with «Ver todo», if any.
    open: Option<Row>,
    /// Colour, black and white, or both (owner request).
    style: IllustrationStyle,
    lists: HashMap<(List, IllustrationStyle, u32), Listed>,
    svgs: HashMap<Illustration, Svg>,
    svgs_loading: usize,
    tx: Sender<Message>,
    rx: Receiver<Message>,
}

impl Default for IllustrationsPanel {
    fn default() -> Self {
        let (tx, rx) = channel();
        Self {
            catalog: None,
            photos: None,
            cutouts: HashMap::new(),
            cutouts_asked: std::collections::HashSet::new(),
            cut_thumbs: HashMap::new(),
            ctx: None,
            text: String::new(),
            typed_at: None,
            open: None,
            style: IllustrationStyle::Any,
            lists: HashMap::new(),
            svgs: HashMap::new(),
            svgs_loading: 0,
            tx,
            rx,
        }
    }
}

impl IllustrationsPanel {
    pub(crate) fn set_catalog(&mut self, catalog: Option<Arc<dyn IllustrationCatalog>>) {
        self.catalog = catalog;
    }

    /// The photo catalogue of «Ilustraciones detalladas».
    pub(crate) fn set_photos(&mut self, photos: Option<Arc<dyn PhotoCatalog>>) {
        self.photos = photos;
    }

    pub(crate) fn available(&self) -> bool {
        self.catalog.is_some()
    }

    /// Takes what the threads sent.
    pub(crate) fn pump(&mut self) {
        while let Ok(message) = self.rx.try_recv() {
            match message {
                Message::List(key, result) => {
                    let listed = match result {
                        Ok(found) => Listed::Ready(found),
                        Err(error) => Listed::Failed(error),
                    };
                    self.lists.insert(key, listed);
                }
                Message::Svg(illustration, made) => {
                    self.svgs_loading = self.svgs_loading.saturating_sub(1);
                    let svg = match (made, &self.ctx) {
                        (Some((bytes, image)), Some(ctx)) => Svg::Ready(
                            Arc::new(bytes),
                            ctx.load_texture(
                                format!("illustration-{}-{}", illustration.set, illustration.name),
                                image,
                                egui::TextureOptions::LINEAR,
                            ),
                        ),
                        _ => Svg::Failed,
                    };
                    self.svgs.insert(illustration, svg);
                }
                Message::Cutouts(words, result) => {
                    self.cutouts.insert(words, result.map(|page| page.items));
                }
                Message::CutoutThumb(url, image) => {
                    self.svgs_loading = self.svgs_loading.saturating_sub(1);
                    let thumb = match (image, &self.ctx) {
                        (Some(image), Some(ctx)) => CutThumb::Ready(ctx.load_texture(
                            format!("cutout-{url}"),
                            image,
                            egui::TextureOptions::LINEAR,
                        )),
                        _ => CutThumb::Failed,
                    };
                    self.cut_thumbs.insert(url, thumb);
                }
            }
        }
    }

    /// The illustrations of `list`, asking for them the first time.
    fn list(&mut self, list: &List, size: u32) -> Option<&Listed> {
        let key = (list.clone(), self.style, size);
        if !self.lists.contains_key(&key) {
            let catalog = self.catalog.clone()?;
            self.lists.insert(key.clone(), Listed::Loading);
            let (tx, ctx) = (self.tx.clone(), self.ctx.clone());
            std::thread::spawn(move || {
                let style = key.1;
                let result = match &key.0 {
                    List::Row(Row::Words(_, words)) => catalog.find(words, style, size),
                    List::Row(Row::Set(_, set)) => catalog.browse(set, size),
                    List::Search(words) => catalog.find(words, style, size),
                };
                let _ = tx.send(Message::List(key, result));
                if let Some(ctx) = ctx {
                    ctx.request_repaint();
                }
            });
        }
        self.lists.get(&(list.clone(), self.style, size))
    }

    /// The SVG and thumbnail of `illustration`, asking for them when they
    /// are not on their way yet (a few at once).
    fn svg(&mut self, illustration: &Illustration) -> Option<(Arc<Vec<u8>>, egui::TextureHandle)> {
        match self.svgs.get(illustration) {
            Some(Svg::Ready(bytes, texture)) => return Some((Arc::clone(bytes), texture.clone())),
            Some(Svg::Loading | Svg::Failed) => return None,
            None => {}
        }
        if self.svgs_loading >= AT_ONCE {
            return None;
        }
        let catalog = self.catalog.clone()?;
        self.svgs_loading += 1;
        self.svgs.insert(illustration.clone(), Svg::Loading);
        let (tx, ctx, wanted) = (self.tx.clone(), self.ctx.clone(), illustration.clone());
        std::thread::spawn(move || {
            let made = catalog.svg(&wanted).ok().and_then(|bytes| {
                let (rgba, w, h) = gafira_io_vector::svg_thumbnail(&bytes, THUMB_SIDE).ok()?;
                let image =
                    egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &rgba);
                Some((bytes, image))
            });
            let _ = tx.send(Message::Svg(wanted, made));
            if let Some(ctx) = ctx {
                ctx.request_repaint();
            }
        });
        None
    }

    /// The panel: the search, then the rows (or the row open, or the
    /// results), and the licences. Returns the illustration to insert.
    pub(crate) fn show(
        &mut self,
        ui: &mut egui::Ui,
        theme: &Theme,
        texts: &Texts,
        can_drop: &dyn Fn(Pos2) -> bool,
    ) -> Option<Chosen> {
        if self.ctx.is_none() {
            self.ctx = Some(ui.ctx().clone());
        }
        let now = ui.input(|i| i.time);
        let field = ui.add(crate::widgets::field(
            &mut self.text,
            plain(texts, "illustrations.search"),
        ));
        if field.changed() {
            self.typed_at = Some(now);
        }
        if let Some(at) = self.typed_at
            && now - at < DEBOUNCE_S
        {
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_secs_f64(DEBOUNCE_S));
        } else {
            self.typed_at = None;
        }
        ui.add_space(6.0);
        // Colour, black and white, or both, as one small list.
        let style_key = |style: IllustrationStyle| match style {
            IllustrationStyle::Any => "illustrations.style_any",
            IllustrationStyle::Color => "illustrations.style_color",
            IllustrationStyle::Mono => "illustrations.style_mono",
        };
        egui::ComboBox::from_id_salt("illustrations.style")
            .selected_text(plain(texts, style_key(self.style)))
            .show_ui(ui, |ui| {
                for style in [
                    IllustrationStyle::Any,
                    IllustrationStyle::Color,
                    IllustrationStyle::Mono,
                ] {
                    ui.selectable_value(&mut self.style, style, plain(texts, style_key(style)));
                }
            });
        ui.add_space(6.0);
        note(ui, theme, plain(texts, "illustrations.free"));
        ui.add_space(8.0);
        let mut chosen = None;
        let words = self.text.trim().to_owned();
        let look = Look {
            theme,
            texts,
            can_drop,
        };
        if !words.is_empty() && self.typed_at.is_none() {
            self.grid(ui, &look, &List::Search(words), &mut chosen);
        } else if let Some(row) = self.open {
            ui.horizontal(|ui| {
                let back = crate::widgets::back_arrow(ui);
                if back.hovered() {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                }
                if back.clicked() {
                    self.open = None;
                }
                ui.label(
                    egui::RichText::new(plain(texts, row.key()))
                        .font(theme.font("ui-label"))
                        .color(theme.c("text")),
                );
            });
            ui.add_space(6.0);
            self.grid(ui, &look, &List::Row(row), &mut chosen);
        } else {
            for row in ROWS {
                if self.shows(row) {
                    self.row(ui, &look, row, &mut chosen);
                }
            }
            // Cut-outs are in colour: hidden when black and white is asked.
            if self.photos.is_some() && self.style != IllustrationStyle::Mono {
                ui.add_space(6.0);
                ui.label(
                    egui::RichText::new(plain(texts, "illustrations.detailed"))
                        .font(theme.font("ui-label"))
                        .color(theme.c("text")),
                );
                note(ui, theme, plain(texts, "illustrations.detailed_note"));
                ui.add_space(8.0);
                for (key, words) in CUTOUTS {
                    self.cutout_row(ui, &look, (key, words), &mut chosen);
                }
            }
        }
        ui.add_space(8.0);
        note(ui, theme, plain(texts, "illustrations.sources"));
        chosen
    }

    /// One row of cut-outs: its title and its first results in a line that
    /// scrolls sideways, asked for once it is in view.
    fn cutout_row(
        &mut self,
        ui: &mut egui::Ui,
        look: &Look<'_>,
        (key, words): (&'static str, &'static str),
        chosen: &mut Option<Chosen>,
    ) {
        ui.label(
            egui::RichText::new(plain(look.texts, key))
                .font(look.theme.font("body-sm"))
                .color(look.theme.c("text")),
        );
        let side = ((ui.available_width() - 3.0 * 6.0) / 4.0).max(40.0);
        let (strip, _) =
            ui.allocate_exact_size(Vec2::new(ui.available_width(), side), Sense::hover());
        if !ui.is_rect_visible(strip) {
            ui.add_space(10.0);
            return;
        }
        if self.cutouts_asked.insert(words)
            && let Some(photos) = self.photos.clone()
        {
            let (tx, ctx) = (self.tx.clone(), self.ctx.clone());
            std::thread::spawn(move || {
                let mut query = PhotoQuery::first(PhotoSource::Openverse, words);
                query.cutouts = true;
                let _ = tx.send(Message::Cutouts(words, photos.search(&query)));
                if let Some(ctx) = ctx {
                    ctx.request_repaint();
                }
            });
        }
        let found = match self.cutouts.get(words) {
            Some(Ok(found)) => found.clone(),
            Some(Err(error)) => {
                ui.painter().text(
                    strip.left_center(),
                    Align2::LEFT_CENTER,
                    plain(look.texts, error_key(error)),
                    look.theme.font("body-sm"),
                    look.theme.c("text.secondary"),
                );
                ui.add_space(10.0);
                return;
            }
            None => Vec::new(),
        };
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(strip));
        egui::ScrollArea::horizontal()
            .id_salt(("illustrations.cutouts", key))
            .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
            .show(&mut child, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 6.0;
                    if found.is_empty() {
                        for _ in 0..4 {
                            let (rect, _) =
                                ui.allocate_exact_size(Vec2::splat(side), Sense::hover());
                            ui.painter()
                                .rect_filled(rect, 8.0, look.theme.c("surface.sunken"));
                        }
                    }
                    for photo in &found {
                        self.cutout_tile(ui, look, photo, side, chosen);
                    }
                });
            });
        ui.add_space(10.0);
    }

    /// One cut-out: its thumbnail on a light ground, its proportions kept;
    /// a click inserts it, a drag carries it, and its licence on hover.
    fn cutout_tile(
        &mut self,
        ui: &mut egui::Ui,
        look: &Look<'_>,
        photo: &Photo,
        side: f32,
        chosen: &mut Option<Chosen>,
    ) {
        let (rect, response) = ui.allocate_exact_size(Vec2::splat(side), Sense::click_and_drag());
        let texture = if ui.is_rect_visible(rect) {
            self.cutout_thumb(photo)
        } else {
            None
        };
        let painter = ui.painter();
        painter.rect_filled(rect, 8.0, Color32::from_gray(246));
        if let Some(texture) = &texture {
            let inner = Rect::from_center_size(rect.center(), fit(texture.size_vec2(), side * 0.9));
            painter.image(
                texture.id(),
                inner,
                Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                Color32::WHITE,
            );
        }
        if response.hovered() && !response.dragged() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            painter.rect_stroke(
                rect,
                8.0,
                Stroke::new(1.5, look.theme.c("primary.bg")),
                egui::StrokeKind::Inside,
            );
        }
        let response = response.on_hover_text(format!(
            "{}\n{} · rawpixel\n{}",
            photo.title.trim(),
            photo.license.name,
            plain(look.texts, "illustrations.how")
        ));
        if response.clicked() {
            *chosen = Some(Chosen::Cutout {
                photo: photo.clone(),
                at: None,
            });
        }
        let at = ui.input(|i| i.pointer.latest_pos());
        if response.dragged()
            && let Some(at) = at
        {
            ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
            if let Some(texture) = &texture {
                let ghost = ui.ctx().layer_painter(egui::LayerId::new(
                    egui::Order::Tooltip,
                    egui::Id::new("illustrations.ghost"),
                ));
                ghost.image(
                    texture.id(),
                    Rect::from_center_size(at, fit(texture.size_vec2(), 96.0)),
                    Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                    Color32::WHITE.gamma_multiply(0.8),
                );
            }
        }
        if response.drag_stopped()
            && let Some(at) = at
            && (look.can_drop)(at)
        {
            *chosen = Some(Chosen::Cutout {
                photo: photo.clone(),
                at: Some(at),
            });
        }
    }

    /// The thumbnail of a cut-out, asking for it once (a few at once).
    fn cutout_thumb(&mut self, photo: &Photo) -> Option<egui::TextureHandle> {
        match self.cut_thumbs.get(&photo.thumbnail_url) {
            Some(CutThumb::Ready(texture)) => return Some(texture.clone()),
            Some(CutThumb::Loading | CutThumb::Failed) => return None,
            None => {}
        }
        if self.svgs_loading >= AT_ONCE {
            return None;
        }
        let photos = self.photos.clone()?;
        self.svgs_loading += 1;
        self.cut_thumbs
            .insert(photo.thumbnail_url.clone(), CutThumb::Loading);
        let (tx, ctx, photo) = (self.tx.clone(), self.ctx.clone(), photo.clone());
        std::thread::spawn(move || {
            let image = photos.thumbnail(&photo).ok().and_then(|bytes| {
                let image = image::load_from_memory(&bytes).ok()?.thumbnail(200, 200);
                let rgba = image.to_rgba8();
                Some(egui::ColorImage::from_rgba_unmultiplied(
                    [rgba.width() as usize, rgba.height() as usize],
                    rgba.as_raw(),
                ))
            });
            let _ = tx.send(Message::CutoutThumb(photo.thumbnail_url.clone(), image));
            if let Some(ctx) = ctx {
                ctx.request_repaint();
            }
        });
        None
    }

    /// Whether `row` fits the style: a whole collection only when it is in
    /// the colours asked for.
    fn shows(&self, row: Row) -> bool {
        match row {
            Row::Words(..) => true,
            Row::Set(_, set) => self
                .catalog
                .as_ref()
                .and_then(|c| c.sets().into_iter().find(|s| s.id == set))
                .is_none_or(|s| self.style.allows(s.color)),
        }
    }

    /// Whether `set` is in black and white, to draw it on a light ground.
    fn mono(&self, set: &str) -> bool {
        self.catalog
            .as_ref()
            .and_then(|c| c.sets().into_iter().find(|s| s.id == set))
            .is_some_and(|s| !s.color)
    }

    /// One row: its title, «Ver todo» and its first illustrations in a
    /// line. Asked for only once it is in view.
    fn row(&mut self, ui: &mut egui::Ui, look: &Look<'_>, row: Row, chosen: &mut Option<Chosen>) {
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new(plain(look.texts, row.key()))
                    .font(look.theme.font("ui-label"))
                    .color(look.theme.c("text")),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let all = ui.add(
                    egui::Button::new(
                        egui::RichText::new(plain(look.texts, "illustrations.see_all"))
                            .font(look.theme.font("body-sm")),
                    )
                    .frame(false),
                );
                if all.hovered() {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                }
                if all.clicked() {
                    self.open = Some(row);
                }
            });
        });
        let side = ((ui.available_width() - 3.0 * 6.0) / 4.0).max(40.0);
        let (strip, _) =
            ui.allocate_exact_size(Vec2::new(ui.available_width(), side), Sense::hover());
        if ui.is_rect_visible(strip) {
            let list = List::Row(row);
            let found = match self.list(&list, ROW_SIZE) {
                Some(Listed::Ready(found)) => found.clone(),
                Some(Listed::Failed(error)) => {
                    ui.painter().text(
                        strip.left_center(),
                        Align2::LEFT_CENTER,
                        plain(look.texts, error_key(error)),
                        look.theme.font("body-sm"),
                        look.theme.c("text.secondary"),
                    );
                    Vec::new()
                }
                _ => Vec::new(),
            };
            let mut child = ui.new_child(egui::UiBuilder::new().max_rect(strip));
            egui::ScrollArea::horizontal()
                .id_salt(("illustrations.row", row.key()))
                .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
                .show(&mut child, |ui| {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 6.0;
                        if found.is_empty() {
                            for _ in 0..4 {
                                let (rect, _) =
                                    ui.allocate_exact_size(Vec2::splat(side), Sense::hover());
                                ui.painter()
                                    .rect_filled(rect, 8.0, look.theme.c("surface.sunken"));
                            }
                        }
                        for illustration in &found {
                            self.tile(ui, look, illustration, side, chosen);
                        }
                    });
                });
        }
        ui.add_space(12.0);
    }

    /// A list as a grid of four columns.
    fn grid(
        &mut self,
        ui: &mut egui::Ui,
        look: &Look<'_>,
        list: &List,
        chosen: &mut Option<Chosen>,
    ) {
        let found = match self.list(list, ALL_SIZE) {
            Some(Listed::Ready(found)) => found.clone(),
            Some(Listed::Failed(error)) => {
                note(ui, look.theme, plain(look.texts, error_key(error)));
                return;
            }
            Some(Listed::Loading) => {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label(plain(look.texts, "illustrations.loading"));
                });
                return;
            }
            None => return,
        };
        if found.is_empty() {
            note(ui, look.theme, plain(look.texts, "illustrations.empty"));
            return;
        }
        let side = ((ui.available_width() - 3.0 * 6.0) / 4.0).max(40.0);
        for line in found.chunks(4) {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                for illustration in line {
                    self.tile(ui, look, illustration, side, chosen);
                }
            });
            ui.add_space(6.0);
        }
    }

    /// One illustration: its thumbnail; a click inserts it, a drag carries
    /// it; its name, collection and licence on hover.
    fn tile(
        &mut self,
        ui: &mut egui::Ui,
        look: &Look<'_>,
        illustration: &Illustration,
        side: f32,
        chosen: &mut Option<Chosen>,
    ) {
        let (rect, response) = ui.allocate_exact_size(Vec2::splat(side), Sense::click_and_drag());
        let ready = if ui.is_rect_visible(rect) {
            self.svg(illustration)
        } else {
            None
        };
        let mono = self.mono(&illustration.set);
        let painter = ui.painter();
        if mono {
            // Black lines on a light ground, also in the dark theme.
            painter.rect_filled(rect, 8.0, Color32::from_gray(246));
        } else if response.hovered() {
            painter.rect_filled(rect, 8.0, look.theme.c("surface.sunken"));
        }
        match &ready {
            Some((_, texture)) => {
                // Its own proportions, fitted in the tile (a tall character of
                // Open Peeps is not squashed into a square).
                let inner =
                    Rect::from_center_size(rect.center(), fit(texture.size_vec2(), side * 0.82));
                painter.image(
                    texture.id(),
                    inner,
                    Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                    Color32::WHITE,
                );
            }
            None => {
                painter.rect_filled(rect.shrink(4.0), 8.0, look.theme.c("surface.sunken"));
            }
        }
        if response.hovered() && !response.dragged() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            painter.rect_stroke(
                rect,
                8.0,
                Stroke::new(1.5, look.theme.c("primary.bg")),
                egui::StrokeKind::Inside,
            );
        }
        let response = response.on_hover_text(hint(look.texts, illustration));
        let Some((svg, texture)) = ready else {
            return;
        };
        if response.clicked() {
            *chosen = Some(Chosen::Drawing {
                illustration: illustration.clone(),
                svg: Arc::clone(&svg),
                at: None,
            });
        }
        let at = ui.input(|i| i.pointer.latest_pos());
        if response.dragged()
            && let Some(at) = at
        {
            ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
            let ghost = ui.ctx().layer_painter(egui::LayerId::new(
                egui::Order::Tooltip,
                egui::Id::new("illustrations.ghost"),
            ));
            ghost.image(
                texture.id(),
                Rect::from_center_size(at, fit(texture.size_vec2(), 96.0)),
                Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                Color32::WHITE.gamma_multiply(0.8),
            );
        }
        if response.drag_stopped()
            && let Some(at) = at
            && (look.can_drop)(at)
        {
            *chosen = Some(Chosen::Drawing {
                illustration: illustration.clone(),
                svg,
                at: Some(at),
            });
        }
    }

    /// The name, collection and licence of `illustration`, for its group.
    pub(crate) fn credit(&self, illustration: &Illustration) -> String {
        let set = self
            .catalog
            .as_ref()
            .and_then(|c| c.sets().into_iter().find(|s| s.id == illustration.set));
        match set {
            Some(set) => format!(
                "{} — {} ({}, {})",
                illustration.name, set.name, set.author, set.license.name
            ),
            None => illustration.name.clone(),
        }
    }
}

/// `size` scaled to fit a square of `side`, its proportions kept.
fn fit(size: Vec2, side: f32) -> Vec2 {
    let longest = size.x.max(size.y).max(1.0);
    size * (side / longest)
}

/// What every tile draws with.
#[derive(Clone, Copy)]
struct Look<'a> {
    theme: &'a Theme,
    texts: &'a Texts,
    can_drop: &'a dyn Fn(Pos2) -> bool,
}

/// The catalogue key of what to say when a list failed.
fn error_key(error: &PhotoError) -> &'static str {
    match error {
        PhotoError::Offline => "illustrations.offline",
        PhotoError::RateLimited { .. } => "photos.limited",
        PhotoError::Busy => "illustrations.busy",
        _ => "illustrations.failed",
    }
}

/// The hover of a tile.
fn hint(texts: &Texts, illustration: &Illustration) -> String {
    format!(
        "{}\n{}\n{}",
        illustration.name.replace('-', " "),
        illustration.set,
        plain(texts, "illustrations.how")
    )
}

fn note(ui: &mut egui::Ui, theme: &Theme, text: String) {
    ui.label(
        egui::RichText::new(text)
            .font(theme.font("body-sm"))
            .color(theme.c("text.secondary")),
    );
}

fn plain(texts: &Texts, key: &'static str) -> String {
    texts.t(&gafira_doc::msg::Message {
        key: gafira_doc::msg::MessageKey(key),
        args: Vec::new(),
    })
}

#[cfg(test)]
mod tests {
    use gafira_ports::photos::IllustrationSet;

    use super::*;

    /// A catalogue of one drawing for any words.
    struct Fake;

    const SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="128" height="128" viewBox="0 0 128 128"><circle cx="64" cy="64" r="40" fill="#e33"/></svg>"##;

    impl IllustrationCatalog for Fake {
        fn sets(&self) -> Vec<IllustrationSet> {
            Vec::new()
        }

        fn find(
            &self,
            text: &str,
            _style: IllustrationStyle,
            _limit: u32,
        ) -> Result<Vec<Illustration>, PhotoError> {
            Ok(vec![Illustration {
                set: "noto".to_owned(),
                name: text.to_owned(),
            }])
        }

        fn browse(&self, _set: &str, _limit: u32) -> Result<Vec<Illustration>, PhotoError> {
            Err(PhotoError::Busy)
        }

        fn svg(&self, _illustration: &Illustration) -> Result<Vec<u8>, PhotoError> {
            Ok(SVG.as_bytes().to_vec())
        }
    }

    fn settle(panel: &mut IllustrationsPanel) {
        for _ in 0..100 {
            panel.pump();
            let busy = panel.lists.values().any(|l| matches!(l, Listed::Loading))
                || panel.svgs.values().any(|s| matches!(s, Svg::Loading));
            if !busy {
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }

    /// A photo catalogue that remembers what it was asked.
    struct Photos(std::sync::Mutex<Vec<PhotoQuery>>);

    impl PhotoCatalog for Photos {
        fn sources(&self) -> Vec<(PhotoSource, gafira_ports::photos::PhotoCapabilities)> {
            Vec::new()
        }

        fn search(&self, query: &PhotoQuery) -> Result<PhotoPage, PhotoError> {
            self.0.lock().expect("lock").push(query.clone());
            Ok(PhotoPage::default())
        }

        fn thumbnail(&self, _photo: &Photo) -> Result<Vec<u8>, PhotoError> {
            Err(PhotoError::Unavailable)
        }

        fn image(&self, _photo: &Photo) -> Result<Vec<u8>, PhotoError> {
            Err(PhotoError::Unavailable)
        }
    }

    // Owner request: a row of «Ilustraciones detalladas» asks Openverse for
    // rawpixel's cut-outs, free of rights, once.
    #[test]
    fn a_cutout_row_asks_for_free_cutouts() {
        let photos = Arc::new(Photos(std::sync::Mutex::new(Vec::new())));
        let mut panel = IllustrationsPanel {
            ctx: Some(egui::Context::default()),
            ..IllustrationsPanel::default()
        };
        panel.set_photos(Some(Arc::clone(&photos) as Arc<dyn PhotoCatalog>));
        let ctx = egui::Context::default();
        let theme =
            crate::style::Theme::new(gafira_theme::ThemeTokens::embedded().expect("tokens"));
        let texts = Texts::new(gafira_ports::i18n::Locale::Es);
        let never = |_: Pos2| false;
        let look = Look {
            theme: &theme,
            texts: &texts,
            can_drop: &never,
        };
        for _ in 0..2 {
            let _ = ctx.run(egui::RawInput::default(), |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    let mut chosen = None;
                    panel.cutout_row(ui, &look, CUTOUTS[0], &mut chosen);
                });
            });
            std::thread::sleep(std::time::Duration::from_millis(50));
            panel.pump();
        }
        let asked = photos.0.lock().expect("lock").clone();
        assert_eq!(asked.len(), 1, "asked once");
        assert!(asked[0].cutouts);
        assert_eq!(asked[0].text, "fern");
        assert_eq!(asked[0].rights, gafira_ports::photos::PhotoRights::Free);
        assert!(matches!(panel.cutouts.get("fern"), Some(Ok(found)) if found.is_empty()));
    }

    // Owner report: a tall drawing keeps its proportions in its tile.
    #[test]
    fn a_thumbnail_keeps_its_proportions() {
        let tall = fit(Vec2::new(80.0, 160.0), 100.0);
        assert_eq!(tall, Vec2::new(50.0, 100.0));
        let wide = fit(Vec2::new(160.0, 40.0), 100.0);
        assert_eq!(wide, Vec2::new(100.0, 25.0));
    }

    // Spec 072: a row is asked once and its drawings come with their SVG,
    // drawn as a thumbnail; a failed collection says so.
    #[test]
    fn a_row_and_its_drawings_arrive() {
        let mut panel = IllustrationsPanel {
            ctx: Some(egui::Context::default()),
            ..IllustrationsPanel::default()
        };
        panel.set_catalog(Some(Arc::new(Fake)));
        let row = List::Row(Row::Words("illustrations.flowers", "flower"));
        assert!(matches!(panel.list(&row, ROW_SIZE), Some(Listed::Loading)));
        settle(&mut panel);
        let Some(Listed::Ready(found)) = panel.list(&row, ROW_SIZE) else {
            panic!("the row arrived")
        };
        let found = found.clone();
        assert_eq!(found[0].name, "flower");
        assert!(panel.svg(&found[0]).is_none(), "on its way");
        settle(&mut panel);
        let (svg, texture) = panel.svg(&found[0]).expect("ready");
        assert_eq!(svg.as_slice(), SVG.as_bytes());
        assert_eq!(texture.size(), [160, 160], "a thumbnail is 160 px across");
        let weather = List::Row(Row::Set("illustrations.weather", "meteocons"));
        panel.list(&weather, ROW_SIZE);
        settle(&mut panel);
        assert!(matches!(
            panel.list(&weather, ROW_SIZE),
            Some(Listed::Failed(PhotoError::Busy))
        ));
    }
}
