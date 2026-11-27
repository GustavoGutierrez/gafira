//! Elementos → Fotos (spec 071, D-111): search remote photo catalogues,
//! browse their thumbnails with the marks of each licence, and insert a
//! photo with a click or a drag. Every call to the catalogue runs on its own
//! thread; the answers come back through a channel the editor drains each
//! frame.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender, channel};

use egui::{Align2, Color32, Pos2, Rect, Sense, Stroke, Vec2};
use gafira_ports::photos::{
    LicenseTerm, Photo, PhotoCatalog, PhotoError, PhotoOrientation, PhotoPage, PhotoQuery,
    PhotoRights, PhotoSource,
};

use crate::style::Theme;
use crate::text::Texts;

/// How long the words rest before they are searched (spec 071 §4.2).
const DEBOUNCE_S: f64 = 0.4;

/// The most thumbnails loading at once.
const THUMBS_AT_ONCE: usize = 6;

/// The longest side a thumbnail is kept at, in px.
const THUMB_SIDE: u32 = 360;

/// The most pixels an inserted photo may have (spec 071 §10).
const MAX_PIXELS: u64 = 80_000_000;

/// The theme shortcuts (spec 071 §4.1): their catalogue key and the words
/// searched, in English, which both sources index best.
pub(crate) const THEMES: [(&str, &str); 7] = [
    ("photos.theme_nature", "nature"),
    ("photos.theme_people", "people"),
    ("photos.theme_business", "business"),
    ("photos.theme_food", "food"),
    ("photos.theme_technology", "technology"),
    ("photos.theme_backgrounds", "background"),
    ("photos.theme_textures", "texture"),
];

/// A thumbnail: on its way, shown, or failed.
enum Thumb {
    Loading,
    Ready(egui::TextureHandle),
    Failed,
}

/// What a thread sends back.
enum Message {
    Page {
        seq: u64,
        result: Result<PhotoPage, PhotoError>,
    },
    Thumb {
        url: String,
        image: Option<egui::ColorImage>,
    },
    Image {
        photo: Box<Photo>,
        at: Option<Pos2>,
        result: Result<Vec<u8>, PhotoError>,
    },
}

/// A photo whose bytes arrived, to be inserted: where it was dropped, if it
/// was, and its size.
pub(crate) struct Arrived {
    pub(crate) photo: Photo,
    pub(crate) at: Option<Pos2>,
    pub(crate) bytes: Vec<u8>,
    pub(crate) natural: (u32, u32),
}

/// What the panel asks the editor this frame.
#[derive(Default)]
pub(crate) struct Wanted {
    /// A photo to insert: in the middle of the page, or where it was let go
    /// (screen), into the frame there if any (D-105, spec 071 §5.2).
    pub(crate) insert: Option<(Photo, Option<Pos2>)>,
}

/// The state of the panel, kept for the editor's session (spec 071 §4.2).
pub(crate) struct PhotosPanel {
    catalog: Option<Arc<dyn PhotoCatalog>>,
    ctx: Option<egui::Context>,
    pub(crate) text: String,
    pub(crate) source: PhotoSource,
    pub(crate) orientation: Option<PhotoOrientation>,
    pub(crate) rights: PhotoRights,
    results: Vec<Photo>,
    next_page: Option<u32>,
    /// The search shown, or on its way.
    shown: Option<PhotoQuery>,
    loading: bool,
    error: Option<PhotoError>,
    /// The number of the newest search: an older answer is dropped.
    seq: u64,
    /// When the words last changed, to wait before searching.
    typed_at: Option<f64>,
    thumbs: HashMap<String, Thumb>,
    thumbs_loading: usize,
    /// Photos whose bytes are on their way.
    pub(crate) inserting: usize,
    /// The last insertion that failed, to say so.
    pub(crate) insert_error: Option<PhotoError>,
    tx: Sender<Message>,
    rx: Receiver<Message>,
}

impl Default for PhotosPanel {
    fn default() -> Self {
        let (tx, rx) = channel();
        Self {
            catalog: None,
            ctx: None,
            text: String::new(),
            source: PhotoSource::Openverse,
            orientation: None,
            rights: PhotoRights::Free,
            results: Vec::new(),
            next_page: None,
            shown: None,
            loading: false,
            error: None,
            seq: 0,
            typed_at: None,
            thumbs: HashMap::new(),
            thumbs_loading: 0,
            inserting: 0,
            insert_error: None,
            tx,
            rx,
        }
    }
}

impl PhotosPanel {
    /// The catalogue it searches; `None` hides the category's content.
    pub(crate) fn set_catalog(&mut self, catalog: Option<Arc<dyn PhotoCatalog>>) {
        self.catalog = catalog;
    }

    /// Whether there is a catalogue to search.
    pub(crate) fn available(&self) -> bool {
        self.catalog.is_some()
    }

    /// The query of the panel's words and filters, page `page`.
    pub(crate) fn query(&self, page: u32) -> PhotoQuery {
        let orientation = self.orientation.filter(|o| self.orientations().contains(o));
        PhotoQuery {
            source: self.source,
            text: self.text.trim().to_owned(),
            orientation,
            rights: self.rights,
            cutouts: false,
            page,
        }
    }

    /// The orientations the chosen source filters by (spec 071 §4.3).
    pub(crate) fn orientations(&self) -> Vec<PhotoOrientation> {
        self.catalog
            .as_ref()
            .and_then(|c| c.sources().into_iter().find(|(s, _)| *s == self.source))
            .map(|(_, caps)| caps.orientations)
            .unwrap_or_default()
    }

    /// Search again from the first page, or ask for the next one when
    /// `more`. A newer search makes the answer of an older one be dropped.
    pub(crate) fn search(&mut self, more: bool) {
        let Some(catalog) = self.catalog.clone() else {
            return;
        };
        let page = match (more, self.next_page) {
            (true, Some(next)) => next,
            (true, None) => return,
            (false, _) => 1,
        };
        let query = self.query(page);
        if !more {
            if self.shown.as_ref() == Some(&query) && !self.results.is_empty() {
                return;
            }
            self.results.clear();
            self.next_page = None;
        }
        self.seq += 1;
        self.loading = true;
        self.error = None;
        self.shown = Some(query.clone());
        let (seq, tx, ctx) = (self.seq, self.tx.clone(), self.ctx.clone());
        std::thread::spawn(move || {
            let result = catalog.search(&query);
            let _ = tx.send(Message::Page { seq, result });
            if let Some(ctx) = ctx {
                ctx.request_repaint();
            }
        });
    }

    /// The bytes of `photo`, fetched on a thread; [`Self::pump`] hands them
    /// over with `at` once they arrive.
    pub(crate) fn fetch(&mut self, photo: Photo, at: Option<Pos2>) {
        let Some(catalog) = self.catalog.clone() else {
            return;
        };
        self.inserting += 1;
        self.insert_error = None;
        let (tx, ctx) = (self.tx.clone(), self.ctx.clone());
        std::thread::spawn(move || {
            let result = catalog.image(&photo);
            let _ = tx.send(Message::Image {
                photo: Box::new(photo),
                at,
                result,
            });
            if let Some(ctx) = ctx {
                ctx.request_repaint();
            }
        });
    }

    /// Takes what the threads sent: pages (an older search's is dropped),
    /// thumbnails, and photos to insert, checked (spec 071 §10).
    pub(crate) fn pump(&mut self) -> Vec<Arrived> {
        let mut arrived = Vec::new();
        while let Ok(message) = self.rx.try_recv() {
            match message {
                Message::Page { seq, result } => {
                    if seq != self.seq {
                        continue;
                    }
                    self.loading = false;
                    match result {
                        Ok(page) => {
                            self.next_page = page.next_page;
                            self.results.extend(page.items);
                        }
                        Err(error) => self.error = Some(error),
                    }
                }
                Message::Thumb { url, image } => {
                    self.thumbs_loading = self.thumbs_loading.saturating_sub(1);
                    let thumb = match (image, &self.ctx) {
                        (Some(image), Some(ctx)) => Thumb::Ready(ctx.load_texture(
                            format!("photo-{url}"),
                            image,
                            egui::TextureOptions::LINEAR,
                        )),
                        _ => Thumb::Failed,
                    };
                    self.thumbs.insert(url, thumb);
                }
                Message::Image { photo, at, result } => {
                    self.inserting = self.inserting.saturating_sub(1);
                    match result.and_then(|bytes| checked(&bytes).map(|size| (bytes, size))) {
                        Ok((bytes, natural)) => arrived.push(Arrived {
                            photo: *photo,
                            at,
                            bytes,
                            natural,
                        }),
                        Err(error) => self.insert_error = Some(error),
                    }
                }
            }
        }
        arrived
    }

    /// The texture of `photo`'s thumbnail, asking for it when it is not on
    /// its way yet (a few at once).
    fn thumbnail(&mut self, photo: &Photo) -> Option<egui::TextureHandle> {
        match self.thumbs.get(&photo.thumbnail_url) {
            Some(Thumb::Ready(texture)) => return Some(texture.clone()),
            Some(Thumb::Loading | Thumb::Failed) => return None,
            None => {}
        }
        if self.thumbs_loading >= THUMBS_AT_ONCE {
            return None;
        }
        let catalog = self.catalog.clone()?;
        self.thumbs_loading += 1;
        self.thumbs
            .insert(photo.thumbnail_url.clone(), Thumb::Loading);
        let (tx, ctx, photo) = (self.tx.clone(), self.ctx.clone(), photo.clone());
        std::thread::spawn(move || {
            let image = catalog.thumbnail(&photo).ok().and_then(|b| small(&b));
            let _ = tx.send(Message::Thumb {
                url: photo.thumbnail_url.clone(),
                image,
            });
            if let Some(ctx) = ctx {
                ctx.request_repaint();
            }
        });
        None
    }

    /// The panel: search, source, licences, orientation, themes, the grid
    /// of photos and what comes next (spec 071 §4).
    pub(crate) fn show(
        &mut self,
        ui: &mut egui::Ui,
        theme: &Theme,
        texts: &Texts,
        can_drop: &dyn Fn(Pos2) -> bool,
    ) -> Wanted {
        if self.ctx.is_none() {
            self.ctx = Some(ui.ctx().clone());
        }
        let mut wanted = Wanted::default();
        // The width of the panel, read before anything is drawn: a row that
        // overflows cannot widen the grid (owner report).
        let full = ui.available_width();
        let now = ui.input(|i| i.time);
        // The words: searched 400 ms after the last key, or at once on Enter.
        let field = ui.add(crate::widgets::field(
            &mut self.text,
            plain(texts, "photos.search"),
        ));
        if field.changed() {
            self.typed_at = Some(now);
        }
        let entered = field.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
        if entered {
            self.typed_at = None;
            self.search(false);
        } else if let Some(at) = self.typed_at {
            if now - at >= DEBOUNCE_S {
                self.typed_at = None;
                self.search(false);
            } else {
                ui.ctx()
                    .request_repaint_after(std::time::Duration::from_secs_f64(DEBOUNCE_S));
            }
        }
        ui.add_space(8.0);
        let mut changed = false;
        // Source, licences and shape as three small drop-down lists on one
        // row (owner request: compact filters); each choice keeps its hint.
        let sources: Vec<PhotoSource> = self
            .catalog
            .as_ref()
            .map(|c| c.sources().into_iter().map(|(s, _)| s).collect())
            .unwrap_or_default();
        let orientations = self.orientations();
        let rights = [
            (
                PhotoRights::Free,
                "photos.rights_free_short",
                "photos.rights_free_hint",
            ),
            (
                PhotoRights::Commercial,
                "photos.rights_commercial_short",
                "photos.rights_commercial_hint",
            ),
            (
                PhotoRights::Any,
                "photos.rights_any_short",
                "photos.rights_any_hint",
            ),
        ];
        let lists = if orientations.is_empty() { 2.0 } else { 3.0 };
        // The width of each list less its arrow and padding, so the row
        // fits the panel and never widens it.
        let arrow = ui.spacing().icon_width
            + ui.spacing().icon_spacing
            + 2.0 * ui.spacing().button_padding.x;
        let width = ((ui.available_width() - 6.0 * (lists - 1.0)) / lists - arrow).max(40.0);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 6.0;
            let shown = short_source(self.source);
            egui::ComboBox::from_id_salt("photos.source")
                .wrap_mode(egui::TextWrapMode::Truncate)
                .width(width)
                .selected_text(shown)
                .show_ui(ui, |ui| {
                    for source in &sources {
                        changed |= ui
                            .selectable_value(&mut self.source, *source, source.name())
                            .changed();
                    }
                })
                .response
                .on_hover_text(plain(texts, "photos.source_hint"));
            let (_, key, _) = rights
                .iter()
                .find(|(r, _, _)| *r == self.rights)
                .copied()
                .unwrap_or(rights[0]);
            egui::ComboBox::from_id_salt("photos.rights")
                .wrap_mode(egui::TextWrapMode::Truncate)
                .width(width)
                .selected_text(plain(texts, key))
                .show_ui(ui, |ui| {
                    for (choice, key, hint) in rights {
                        changed |= ui
                            .selectable_value(&mut self.rights, choice, plain(texts, key))
                            .on_hover_text(plain(texts, hint))
                            .changed();
                    }
                })
                .response
                .on_hover_text(plain(texts, "photos.rights_hint"));
            // The shape, only where the source filters by it (spec 071 §4.3).
            if !orientations.is_empty() {
                let label = |o: Option<PhotoOrientation>| match o {
                    None => plain(texts, "photos.any_orientation"),
                    Some(PhotoOrientation::Landscape) => plain(texts, "photos.landscape"),
                    Some(PhotoOrientation::Portrait) => plain(texts, "photos.portrait"),
                    Some(PhotoOrientation::Square) => plain(texts, "photos.square"),
                };
                egui::ComboBox::from_id_salt("photos.orientation")
                    .wrap_mode(egui::TextWrapMode::Truncate)
                    .width(width)
                    .selected_text(label(self.orientation))
                    .show_ui(ui, |ui| {
                        for choice in
                            std::iter::once(None).chain(orientations.iter().copied().map(Some))
                        {
                            changed |= ui
                                .selectable_value(&mut self.orientation, choice, label(choice))
                                .changed();
                        }
                    });
            }
        });
        ui.add_space(6.0);
        // The themes, one row of small chips that scrolls sideways.
        egui::ScrollArea::horizontal()
            .id_salt("photos.themes")
            .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 4.0;
                    for (key, words) in THEMES {
                        let on = self.text.trim() == words;
                        if chip(ui, theme, on, &plain(texts, key), None).clicked() {
                            self.text = words.to_owned();
                            self.typed_at = None;
                            changed = true;
                        }
                    }
                });
            });
        if changed || (self.shown.is_none() && !self.loading) {
            self.search(false);
        }
        ui.add_space(10.0);
        if self.inserting > 0 {
            note(ui, theme, plain(texts, "photos.inserting"));
        }
        if let Some(error) = &self.insert_error {
            note(ui, theme, plain(texts, error_key(error, true)));
        }
        self.grid(ui, (theme, texts, full), can_drop, &mut wanted);
        if self.loading {
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label(plain(texts, "photos.loading"));
            });
        } else if let Some(error) = &self.error {
            note(ui, theme, plain(texts, error_key(error, false)));
        } else if self.results.is_empty() {
            note(ui, theme, plain(texts, "photos.empty"));
        } else if self.next_page.is_some() {
            ui.add_space(8.0);
            let more = ui.add_sized(
                Vec2::new(ui.available_width(), 32.0),
                egui::Button::new(plain(texts, "photos.more")),
            );
            if more.hovered() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            }
            if more.clicked() {
                self.search(true);
            }
        }
        ui.add_space(8.0);
        note(ui, theme, plain(texts, "photos.credit_note"));
        wanted
    }

    /// The photos in two columns: the thumbnail, its marks and its author;
    /// a click inserts it, a drag carries it (D-105).
    fn grid(
        &mut self,
        ui: &mut egui::Ui,
        (theme, texts, full): (&Theme, &Texts, f32),
        can_drop: &dyn Fn(Pos2) -> bool,
        wanted: &mut Wanted,
    ) {
        let gap = 8.0;
        let width = ((full.min(ui.available_width()) - gap) / 2.0).max(60.0);
        let photos = self.results.clone();
        let look = Look {
            theme,
            texts,
            can_drop,
        };
        for row in photos.chunks(2) {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = gap;
                for photo in row {
                    self.card(ui, &look, (photo, width), wanted);
                }
            });
            ui.add_space(gap);
        }
    }

    /// One photo of the grid, `width` across.
    fn card(
        &mut self,
        ui: &mut egui::Ui,
        look: &Look<'_>,
        (photo, width): (&Photo, f32),
        wanted: &mut Wanted,
    ) {
        let Look {
            theme,
            texts,
            can_drop,
        } = *look;
        let image_h = width * 0.75;
        let (rect, response) =
            ui.allocate_exact_size(Vec2::new(width, image_h + 24.0), Sense::click_and_drag());
        let picture = Rect::from_min_size(rect.min, Vec2::new(width, image_h));
        let painter = ui.painter_at(rect);
        painter.rect_filled(picture, 6.0, theme.c("surface.sunken"));
        let texture = self.thumbnail(photo);
        if let Some(texture) = &texture {
            // Cut to the card (cover), its middle kept.
            let size = texture.size_vec2();
            let uv = cover_uv(size, picture.size());
            egui::Image::new((texture.id(), size))
                .uv(uv)
                .corner_radius(6.0)
                .paint_at(ui, picture);
        }
        // The marks of its licence, top left; «Libre» or «Uso comercial»
        // top right (owner request).
        let mut x = picture.left() + 6.0;
        for term in &photo.license.terms {
            let (label, key) = term_mark(*term);
            let badge = Rect::from_center_size(
                Pos2::new(x + 11.0, picture.top() + 15.0),
                Vec2::splat(22.0),
            );
            painter.circle_filled(badge.center(), 11.0, Color32::from_black_alpha(170));
            painter.text(
                badge.center(),
                Align2::CENTER_CENTER,
                label,
                egui::FontId::proportional(9.0),
                Color32::WHITE,
            );
            let hover = ui.interact(badge, response.id.with(key), Sense::hover());
            hover.on_hover_text(plain(texts, key));
            x += 25.0;
        }
        let free = photo.license.free_of_rights();
        if free || photo.license.free_to_use() {
            let (key, hint, fill) = if free {
                ("photos.badge_free", "photos.badge_free_hint", hex(0x1F9D6B))
            } else {
                (
                    "photos.badge_commercial",
                    "photos.badge_commercial_hint",
                    hex(0x2563EB),
                )
            };
            let label = plain(texts, key);
            let font = egui::FontId::proportional(10.0);
            let w = painter
                .layout_no_wrap(label.clone(), font.clone(), Color32::WHITE)
                .size()
                .x
                + 28.0;
            let pill = Rect::from_min_size(
                Pos2::new(picture.right() - w - 6.0, picture.top() + 5.0),
                Vec2::new(w, 20.0),
            );
            painter.rect_filled(pill, 10.0, fill);
            // The tick is an icon: the interface font has no «✔» (spec 098).
            let tick = Rect::from_center_size(
                Pos2::new(pill.left() + 13.0, pill.center().y),
                Vec2::splat(11.0),
            );
            crate::icons::paint(
                &painter,
                tick,
                crate::icons::Icon::Check,
                crate::icons::Weight::Regular,
                Color32::WHITE,
            );
            painter.text(
                Pos2::new(pill.left() + 21.0, pill.center().y),
                Align2::LEFT_CENTER,
                label,
                font,
                Color32::WHITE,
            );
            let hover = ui.interact(pill, response.id.with("free"), Sense::hover());
            hover.on_hover_text(plain(texts, hint));
        }
        // The author and the licence, under it, visible without hovering
        // (spec 071 §4.4).
        let caption = if photo.author.trim().is_empty() {
            photo.license.name.clone()
        } else {
            format!("{} · {}", photo.author.trim(), photo.license.name)
        };
        painter.text(
            Pos2::new(rect.left() + 2.0, picture.bottom() + 12.0),
            Align2::LEFT_CENTER,
            caption,
            theme.font("body-sm"),
            theme.c("text.secondary"),
        );
        if response.hovered() && !response.dragged() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            painter.rect_stroke(
                picture,
                6.0,
                Stroke::new(2.0, theme.c("primary.bg")),
                egui::StrokeKind::Inside,
            );
        }
        let response = response.on_hover_text(details(texts, photo));
        if response.secondary_clicked() {
            ui.ctx().open_url(egui::OpenUrl::new_tab(&photo.page_url));
        }
        if response.clicked() {
            wanted.insert = Some((photo.clone(), None));
        }
        let at = ui.input(|i| i.pointer.latest_pos());
        if response.dragged()
            && let Some(at) = at
        {
            ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
            if let Some(texture) = &texture {
                let ghost = ui.ctx().layer_painter(egui::LayerId::new(
                    egui::Order::Tooltip,
                    egui::Id::new("photos.ghost"),
                ));
                let size = fit(texture.size_vec2(), 120.0);
                ghost.image(
                    texture.id(),
                    Rect::from_center_size(at, size),
                    Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                    Color32::WHITE.gamma_multiply(0.8),
                );
            }
        }
        if response.drag_stopped()
            && let Some(at) = at
            && can_drop(at)
        {
            wanted.insert = Some((photo.clone(), Some(at)));
        }
    }
}

/// What every card of the grid draws with.
#[derive(Clone, Copy)]
struct Look<'a> {
    theme: &'a Theme,
    texts: &'a Texts,
    can_drop: &'a dyn Fn(Pos2) -> bool,
}

/// The catalogue key of the message for `error`, inserting or searching
/// (spec 071 §9).
fn error_key(error: &PhotoError, inserting: bool) -> &'static str {
    match error {
        PhotoError::Offline => "photos.offline",
        PhotoError::RateLimited { .. } => "photos.limited",
        PhotoError::Busy => "photos.busy",
        PhotoError::Unavailable => "photos.gone",
        PhotoError::TooLarge | PhotoError::Invalid(_) if inserting => "photos.failed",
        PhotoError::TooLarge | PhotoError::Invalid(_) => "photos.search_failed",
    }
}

/// The short mark of a licence element and the key of its explanation.
fn term_mark(term: LicenseTerm) -> (&'static str, &'static str) {
    match term {
        LicenseTerm::Zero => ("CC0", "photos.term_zero"),
        LicenseTerm::PublicDomain => ("PD", "photos.term_pd"),
        LicenseTerm::By => ("BY", "photos.term_by"),
        LicenseTerm::NonCommercial => ("NC", "photos.term_nc"),
        LicenseTerm::ShareAlike => ("SA", "photos.term_sa"),
        LicenseTerm::NoDerivatives => ("ND", "photos.term_nd"),
    }
}

/// What the hover of a card says: title, author, licence, source, and how
/// to open its page.
fn details(texts: &Texts, photo: &Photo) -> String {
    let mut lines = Vec::new();
    if !photo.title.trim().is_empty() {
        lines.push(photo.title.trim().to_owned());
    }
    if !photo.author.trim().is_empty() {
        lines.push(photo.author.trim().to_owned());
    }
    lines.push(format!("{} · {}", photo.license.name, photo.source.name()));
    if photo.width > 0 && photo.height > 0 {
        lines.push(format!("{} × {} px", photo.width, photo.height));
    }
    lines.push(plain(texts, "photos.open_page"));
    lines.join("\n")
}

/// The part of an image of `size` that covers a box of `into`, centred.
fn cover_uv(size: Vec2, into: Vec2) -> Rect {
    if size.x <= 0.0 || size.y <= 0.0 || into.x <= 0.0 || into.y <= 0.0 {
        return Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0));
    }
    let (image, cell) = (size.x / size.y, into.x / into.y);
    if image > cell {
        let w = cell / image;
        Rect::from_min_max(
            Pos2::new((1.0 - w) / 2.0, 0.0),
            Pos2::new((1.0 + w) / 2.0, 1.0),
        )
    } else {
        let h = image / cell;
        Rect::from_min_max(
            Pos2::new(0.0, (1.0 - h) / 2.0),
            Pos2::new(1.0, (1.0 + h) / 2.0),
        )
    }
}

/// `size` scaled to fit a square of `side`.
fn fit(size: Vec2, side: f32) -> Vec2 {
    let k = side / size.x.max(size.y).max(1.0);
    size * k
}

/// The size of an image to insert, when it decodes as one and is not too
/// large (spec 071 §10).
fn checked(bytes: &[u8]) -> Result<(u32, u32), PhotoError> {
    let reader = image::ImageReader::new(std::io::Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|error| PhotoError::Invalid(error.to_string()))?;
    let (w, h) = reader
        .into_dimensions()
        .map_err(|error| PhotoError::Invalid(error.to_string()))?;
    if w == 0 || h == 0 {
        return Err(PhotoError::Invalid("empty image".to_owned()));
    }
    if u64::from(w) * u64::from(h) > MAX_PIXELS {
        return Err(PhotoError::TooLarge);
    }
    Ok((w, h))
}

/// A thumbnail's bytes as a small texture image.
fn small(bytes: &[u8]) -> Option<egui::ColorImage> {
    let image = image::load_from_memory(bytes).ok()?;
    let image = if image.width().max(image.height()) > THUMB_SIDE {
        image.thumbnail(THUMB_SIDE, THUMB_SIDE)
    } else {
        image
    };
    let rgba = image.to_rgba8();
    Some(egui::ColorImage::from_rgba_unmultiplied(
        [rgba.width() as usize, rgba.height() as usize],
        rgba.as_raw(),
    ))
}

/// A choice of the panel: a rounded chip, filled when chosen, a hand over
/// it, `hint` on hover.
fn chip(
    ui: &mut egui::Ui,
    theme: &Theme,
    on: bool,
    label: &str,
    hint: Option<String>,
) -> egui::Response {
    let (fill, color) = if on {
        (theme.c("primary.bg"), theme.c("primary.fg"))
    } else {
        (theme.c("surface.sunken"), theme.c("text"))
    };
    let button = egui::Button::new(
        egui::RichText::new(label)
            .font(theme.font("body-sm"))
            .color(color),
    )
    .fill(fill)
    .corner_radius(12.0)
    .min_size(Vec2::new(0.0, 24.0));
    let response = ui.add(button);
    if response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    match hint {
        Some(hint) => response.on_hover_text(hint),
        None => response,
    }
}

/// The short name of `source` for its drop-down list.
fn short_source(source: PhotoSource) -> &'static str {
    match source {
        PhotoSource::Openverse => "Openverse",
        PhotoSource::Wikimedia => "Wikimedia",
    }
}

/// A short line in the secondary colour.
fn note(ui: &mut egui::Ui, theme: &Theme, text: String) {
    ui.label(
        egui::RichText::new(text)
            .font(theme.font("body-sm"))
            .color(theme.c("text.secondary")),
    );
}

fn hex(code: u32) -> Color32 {
    Color32::from_rgb((code >> 16) as u8, (code >> 8) as u8, code as u8)
}

fn plain(texts: &Texts, key: &'static str) -> String {
    texts.t(&gafira_doc::msg::Message {
        key: gafira_doc::msg::MessageKey(key),
        args: Vec::new(),
    })
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use gafira_ports::photos::{PhotoCapabilities, PhotoLicense};

    use super::*;

    /// A catalogue that answers each search with its words as the title of
    /// one photo, slower for the words «slow».
    struct Fake {
        asked: Mutex<Vec<PhotoQuery>>,
    }

    impl PhotoCatalog for Fake {
        fn sources(&self) -> Vec<(PhotoSource, PhotoCapabilities)> {
            vec![
                (
                    PhotoSource::Openverse,
                    PhotoCapabilities {
                        orientations: vec![PhotoOrientation::Square],
                    },
                ),
                (
                    PhotoSource::Wikimedia,
                    PhotoCapabilities {
                        orientations: Vec::new(),
                    },
                ),
            ]
        }

        fn search(&self, query: &PhotoQuery) -> Result<PhotoPage, PhotoError> {
            self.asked.lock().expect("lock").push(query.clone());
            if query.text == "slow" {
                std::thread::sleep(std::time::Duration::from_millis(200));
            }
            Ok(PhotoPage {
                items: vec![photo(&query.text)],
                next_page: Some(query.page + 1),
            })
        }

        fn thumbnail(&self, _photo: &Photo) -> Result<Vec<u8>, PhotoError> {
            Err(PhotoError::Unavailable)
        }

        fn image(&self, _photo: &Photo) -> Result<Vec<u8>, PhotoError> {
            let mut png = Vec::new();
            image::RgbaImage::from_pixel(30, 20, image::Rgba([1, 2, 3, 255]))
                .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
                .map_err(|e| PhotoError::Invalid(e.to_string()))?;
            Ok(png)
        }
    }

    fn photo(title: &str) -> Photo {
        Photo {
            source: PhotoSource::Openverse,
            id: title.to_owned(),
            title: title.to_owned(),
            author: "Ana".to_owned(),
            page_url: "https://example.org/p".to_owned(),
            license: PhotoLicense::named("CC0 1.0", ""),
            thumbnail_url: format!("https://example.org/{title}.jpg"),
            image_url: format!("https://example.org/{title}-big.jpg"),
            width: 30,
            height: 20,
        }
    }

    fn panel() -> PhotosPanel {
        let mut panel = PhotosPanel::default();
        panel.set_catalog(Some(Arc::new(Fake {
            asked: Mutex::new(Vec::new()),
        })));
        panel
    }

    /// Drain the panel until nothing is on its way, or a second passes.
    fn settle(panel: &mut PhotosPanel) -> Vec<Arrived> {
        let mut arrived = Vec::new();
        for _ in 0..100 {
            arrived.extend(panel.pump());
            if !panel.loading && panel.inserting == 0 {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        arrived
    }

    // Spec 071 §4.2: the answer of an older search never replaces the
    // newer one; free of rights is the default.
    #[test]
    fn a_late_answer_is_dropped() {
        let mut panel = panel();
        assert_eq!(panel.query(1).rights, PhotoRights::Free);
        panel.text = "slow".to_owned();
        panel.search(false);
        panel.text = "fast".to_owned();
        panel.search(false);
        std::thread::sleep(std::time::Duration::from_millis(300));
        settle(&mut panel);
        let titles: Vec<&str> = panel.results.iter().map(|p| p.title.as_str()).collect();
        assert_eq!(titles, ["fast"]);
        panel.search(true);
        settle(&mut panel);
        assert_eq!(panel.results.len(), 2, "the next page is added");
    }

    // Spec 071 §4.3: an orientation the source has no filter for is not
    // sent.
    #[test]
    fn an_orientation_the_source_lacks_is_left_out() {
        let mut panel = panel();
        panel.orientation = Some(PhotoOrientation::Square);
        assert_eq!(panel.query(1).orientation, Some(PhotoOrientation::Square));
        panel.source = PhotoSource::Wikimedia;
        assert_eq!(panel.query(1).orientation, None);
    }

    // Spec 071 §5.1: a fetched photo arrives with its size, once.
    #[test]
    fn a_fetched_photo_arrives_checked() {
        let mut panel = panel();
        panel.fetch(photo("a"), None);
        let arrived = settle(&mut panel);
        assert_eq!(arrived.len(), 1);
        assert_eq!(arrived[0].natural, (30, 20));
        assert!(checked(b"not an image").is_err());
    }
}
