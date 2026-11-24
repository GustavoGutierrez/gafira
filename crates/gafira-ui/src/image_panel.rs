//! «Editar imagen» (spec 085): the panel with where it applies, the tools,
//! the adjustments, the filters and the shadows of an image, and the zone
//! chosen over it on the canvas.

use std::sync::Arc;

use egui::{Color32, Pos2, Rect, Response, RichText, Sense, Stroke, StrokeKind, Ui, Vec2};
use gafira_doc::{
    AssetRef, ColorShift, CropShape, ImageAdjust, ImageFilter, ImageLook, ImageNode, ImageShadow,
    NodeId, PageId, Placement, ShadowKind, ZoneKind,
};
use gafira_ports::store::ContentStore;
use gafira_ports::vision::VisionModels;
use gafira_raster::look::FILTER_GROUPS;
use kurbo::Affine;

use crate::canvas;
use crate::charts::plain;
use crate::crop::{Cropper, RATIOS};
use crate::eraser::{Mode, Paint};
use crate::icons::Icon;
use crate::looks::{Facts, Looks, SHADOWS, Set, shadow_id, white};
use crate::rotation;
use crate::style::Theme;
use crate::text::Texts;
use crate::vision::{self, Ask, Studio, Target, Work};
use crate::widgets::{self, IconButton};
use crate::zones::{Made, Masks, PAINTED, Purpose, Trouble, stored, with_zone};

/// The pages of the panel.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum View {
    #[default]
    Home,
    Adjust,
    Filters,
    Shadows,
    /// «Borrador mágico» (spec 086).
    Magic,
    /// «Recortar» (spec 088).
    Crop,
}

/// The sizes of the brush, in px of the screen.
const BRUSH: std::ops::RangeInclusive<f32> = 4.0..=200.0;

/// How strong the zone is seen over the image.
const VEIL: f32 = 0.55;

/// What the pointer does on the image.
enum Tool {
    /// A click takes the element under it.
    Pick,
    Brush(Box<Brush>),
}

/// A zone painted by hand.
struct Brush {
    paint: Paint,
    size: (u32, u32),
    /// The mask of the document it was read from, or left as.
    of: Option<AssetRef>,
    texture: Option<egui::TextureHandle>,
    from: Option<(f32, f32)>,
    /// The stroke, in shares of the sides of the image.
    stroke: Vec<[f32; 2]>,
}

/// What is marked to be erased (spec 086). It is not of the document.
struct Magic {
    /// The image when the page was opened.
    before: ImageNode,
    /// It was opened to take a watermark off.
    stamp: bool,
    /// A click takes an element.
    pick: bool,
    brush: Box<Brush>,
    /// The image being worked on: another one by now clears what is marked.
    sent: Option<AssetRef>,
}

/// The image being edited.
pub(crate) struct Editing {
    pub page: PageId,
    pub node: NodeId,
    view: View,
    /// The zone the adjustments are of; none for the whole image.
    zone: Option<usize>,
    /// Whether the zone is seen over the image.
    veil: bool,
    tool: Option<Tool>,
    magic: Option<Magic>,
    crop: Option<Cropper>,
    /// «Recortar» opens as soon as there is an image to crop.
    to_crop: bool,
    cropped: Option<(ImageNode, Placement)>,
    /// The size of the box all of the image has, and «Aplicar».
    whole: (f64, f64),
    accept: bool,
    brush_size: f32,
    adding: bool,
    detect: bool,
    /// The colour of the image that is moved.
    color: Option<usize>,
    /// Masks on their way, and what is said of the last one.
    masks: Masks,
    note: Option<&'static str>,
    /// A change that is a step of its own.
    step: Option<ImageNode>,
}

/// What the panel asks for.
#[derive(Default)]
pub(crate) struct Asks {
    /// The image now, while a control changes it.
    pub image: Option<ImageNode>,
    pub vision: Option<Ask>,
    pub close: bool,
}

impl Asks {
    /// The look to change: the one changed already, or the one of `of`.
    fn edit(&mut self, of: &ImageNode, change: impl FnOnce(&mut ImageLook)) {
        let now = self.image.as_ref().unwrap_or(of);
        let mut look = now.look.as_deref().cloned().unwrap_or_default();
        change(&mut look);
        let next = now.looking(look);
        if next != *now {
            self.image = Some(next);
        }
    }
}

/// What the panel is drawn with.
pub(crate) struct With<'a> {
    pub theme: &'a Theme,
    pub texts: &'a Texts,
    pub looks: &'a mut Looks,
    pub studio: &'a Studio,
    pub store: &'a Arc<dyn ContentStore>,
    /// The texture of all of the image, as it was brought in.
    pub plain: Option<egui::TextureId>,
}

impl With<'_> {
    fn t(&self, key: &'static str) -> String {
        plain(self.texts, key)
    }
}

/// The adjustments of `look` for `zone`.
fn adjust_of(look: &ImageLook, zone: Option<usize>) -> Option<&ImageAdjust> {
    match zone {
        None => Some(&look.adjust),
        Some(at) => look.zones.get(at).map(|zone| &zone.adjust),
    }
}

fn adjust_mut(look: &mut ImageLook, zone: Option<usize>) -> Option<&mut ImageAdjust> {
    match zone {
        None => Some(&mut look.adjust),
        Some(at) => look.zones.get_mut(at).map(|zone| &mut zone.adjust),
    }
}

/// The name of each kind of zone.
fn zone_key(kind: ZoneKind) -> &'static str {
    match kind {
        ZoneKind::Element => "image.element",
        ZoneKind::Area => "image.area",
        ZoneKind::Foreground => "image.foreground",
        ZoneKind::Background => "image.background",
    }
}

fn shadow_key(kind: ShadowKind) -> &'static str {
    match kind {
        ShadowKind::Glow => "image.shadow_glow",
        ShadowKind::Drop => "image.shadow_drop",
        ShadowKind::Outline => "image.shadow_outline",
        ShadowKind::Curved => "image.shadow_curved",
        ShadowKind::Floating => "image.shadow_floating",
        ShadowKind::Angled => "image.shadow_angled",
        ShadowKind::Ground => "image.shadow_ground",
    }
}

/// The names of the filters and of their groups, as in the catalogue.
fn filter_key(id: &str) -> &'static str {
    match id {
        "alba" => "image.f_alba",
        "brisa" => "image.f_brisa",
        "prado" => "image.f_prado",
        "lino" => "image.f_lino",
        "rocio" => "image.f_rocio",
        "sauce" => "image.f_sauce",
        "ambar" => "image.f_ambar",
        "miel" => "image.f_miel",
        "terracota" => "image.f_terracota",
        "ocaso" => "image.f_ocaso",
        "canela" => "image.f_canela",
        "duna" => "image.f_duna",
        "marea" => "image.f_marea",
        "glaciar" => "image.f_glaciar",
        "niebla" => "image.f_niebla",
        "cobalto" => "image.f_cobalto",
        "pizarra" => "image.f_pizarra",
        "invierno" => "image.f_invierno",
        "carnaval" => "image.f_carnaval",
        "tropico" => "image.f_tropico",
        "neon" => "image.f_neon",
        "arrecife" => "image.f_arrecife",
        "fiesta" => "image.f_fiesta",
        "selva" => "image.f_selva",
        "nube" => "image.f_nube",
        "algodon" => "image.f_algodon",
        "petalo" => "image.f_petalo",
        "perla" => "image.f_perla",
        "susurro" => "image.f_susurro",
        "crema" => "image.f_crema",
        "sepia" => "image.f_sepia",
        "postal" => "image.f_postal",
        "cinta" => "image.f_cinta",
        "recuerdo" => "image.f_recuerdo",
        "album" => "image.f_album",
        _ => "image.f_mono",
    }
}

fn group_key(id: &str) -> &'static str {
    match id {
        "natural" => "image.g_natural",
        "warm" => "image.g_warm",
        "cold" => "image.g_cold",
        "vivid" => "image.g_vivid",
        "soft" => "image.g_soft",
        _ => "image.g_classic",
    }
}

/// The controls of a group: what it is called, its least and most, its
/// track, and the number it moves.
type Field = fn(&mut ImageAdjust) -> &mut f64;
struct Control {
    key: &'static str,
    range: (f64, f64),
    track: Track,
    field: Field,
}

pub(crate) use crate::slider::Track;

const fn control(key: &'static str, range: (f64, f64), track: Track, field: Field) -> Control {
    Control {
        key,
        range,
        track,
        field,
    }
}

const BOTH: (f64, f64) = (-100.0, 100.0);
const UP: (f64, f64) = (0.0, 100.0);
const TURN: (f64, f64) = (-180.0, 180.0);

const WHITE_BALANCE: [Control; 2] = [
    control("image.temperature", BOTH, Track::Temperature, |a| {
        &mut a.temperature
    }),
    control("image.tint", BOTH, Track::Tint, |a| &mut a.tint),
];

const LIGHT: [Control; 6] = [
    control("image.brightness", BOTH, Track::Plain, |a| {
        &mut a.brightness
    }),
    control("image.contrast", BOTH, Track::Plain, |a| &mut a.contrast),
    control("image.highlights", BOTH, Track::Plain, |a| {
        &mut a.highlights
    }),
    control("image.shadows_light", BOTH, Track::Plain, |a| {
        &mut a.shadows
    }),
    control("image.whites", BOTH, Track::Plain, |a| &mut a.whites),
    control("image.blacks", BOTH, Track::Plain, |a| &mut a.blacks),
];

const COLOR: [Control; 3] = [
    control("image.hue", TURN, Track::Hue, |a| &mut a.hue),
    control("image.saturation", BOTH, Track::Saturation, |a| {
        &mut a.saturation
    }),
    control("image.vibrance", BOTH, Track::Saturation, |a| {
        &mut a.vibrance
    }),
];

const TEXTURE: [Control; 3] = [
    control("image.sharpness", UP, Track::Plain, |a| &mut a.sharpness),
    control("image.clarity", UP, Track::Plain, |a| &mut a.clarity),
    control("image.vignette", BOTH, Track::Plain, |a| &mut a.vignette),
];

/// A slider with its name and its number, in whole steps; a double click on
/// it goes back to `rest` (spec 098: [`crate::slider::Slider`]).
pub(crate) fn slider(
    ui: &mut Ui,
    theme: &Theme,
    name: String,
    (value, range, rest): (&mut f64, (f64, f64), f64),
    track: Track,
) -> bool {
    crate::slider::Slider::new(name, value, range)
        .rest(rest)
        .track(track)
        .show(ui, theme)
}

/// A choice in a row of them.
fn chip(ui: &mut Ui, theme: &Theme, text: String, on: bool) -> Response {
    chip_if(ui, theme, text, (on, true))
}

/// [`chip`], which may not be there to press. Not in a scope of its own: it
/// goes to the next row when it does not fit.
fn chip_if(ui: &mut Ui, theme: &Theme, text: String, (on, enabled): (bool, bool)) -> Response {
    let (fill, ink, edge) = if on {
        (
            theme.c("accent").gamma_multiply(0.25),
            theme.c("text"),
            theme.c("accent"),
        )
    } else {
        (theme.c("surface"), theme.c("text"), theme.c("border"))
    };
    let text = RichText::new(text).font(theme.font("ui-label")).color(ink);
    ui.add_enabled(
        enabled,
        egui::Button::new(text)
            .fill(fill)
            .stroke(Stroke::new(1.0, edge))
            .corner_radius(16.0)
            // Whole in the next row instead of broken in this one.
            .wrap_mode(egui::TextWrapMode::Extend)
            .min_size(Vec2::new(0.0, 32.0)),
    )
}

/// The name of a part of the panel.
pub(crate) fn heading(ui: &mut Ui, theme: &Theme, text: String) {
    ui.add_space(12.0);
    ui.label(RichText::new(text).font(theme.font("ui-label")));
    ui.add_space(6.0);
}

/// [`heading`] with a link at its right; whether it was pressed.
fn heading_with(ui: &mut Ui, theme: &Theme, (text, link): (String, String), on: bool) -> bool {
    ui.add_space(12.0);
    let pressed = ui
        .horizontal(|ui| {
            ui.label(RichText::new(text).font(theme.font("ui-label")));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let link = RichText::new(link).font(theme.font("body-sm"));
                on && ui
                    .add(egui::Button::new(link).frame(false))
                    .on_hover_cursor(egui::CursorIcon::PointingHand)
                    .clicked()
            })
            .inner
        })
        .inner;
    ui.add_space(6.0);
    pressed
}

/// A square with a shape to cut to, and its name under it.
fn shape_tile(
    ui: &mut Ui,
    theme: &Theme,
    (shape, name): (&CropShape, String),
    (side, on): (f32, bool),
) -> Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::new(side, side + 22.0), Sense::click());
    let square = Rect::from_min_size(rect.min, Vec2::splat(side));
    let (fill, edge) = if on {
        (theme.c("accent").gamma_multiply(0.25), theme.c("accent"))
    } else if response.hovered() {
        (theme.c("surface.sunken"), theme.c("border"))
    } else {
        (theme.c("surface"), theme.c("border"))
    };
    let stroke = Stroke::new(if on { 2.0 } else { 1.0 }, edge);
    let painter = ui.painter();
    painter.rect(square, theme.radius_md(), fill, stroke, StrokeKind::Inside);
    let icon = Rect::from_center_size(square.center(), Vec2::splat(side * 0.42));
    crate::crop::shape_icon(painter, icon, shape, theme.c("text"));
    let galley = painter.layout_no_wrap(name.clone(), theme.font("caption"), theme.c("text"));
    let x = square.center().x - galley.size().x / 2.0;
    painter.galley(Pos2::new(x, square.bottom() + 4.0), galley, theme.c("text"));
    response.widget_info(|| {
        egui::WidgetInfo::selected(egui::WidgetType::Button, true, on, name.clone())
    });
    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// The room between two tiles of a row of three, and the side of each.
const GAP: f32 = 8.0;

fn tile_side(ui: &Ui) -> f32 {
    ((ui.available_width() - 2.0 * GAP) / 3.0).floor().max(40.0)
}

/// A square picture with its name under it.
fn tile(
    ui: &mut Ui,
    theme: &Theme,
    (picture, cover): (Option<&egui::TextureHandle>, bool),
    name: String,
    (side, on): (f32, bool),
) -> Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::new(side, side + 20.0), Sense::click());
    let square = Rect::from_min_size(rect.min, Vec2::splat(side));
    let painter = ui.painter();
    painter.rect_filled(square, theme.radius_md(), theme.c("surface.sunken"));
    if let Some(texture) = picture {
        let [w, h] = texture.size();
        let (w, h) = (w as f32, h as f32);
        if cover {
            let most = w.min(h).max(1.0);
            let half = Vec2::new(most / w / 2.0, most / h / 2.0);
            let middle = Pos2::new(0.5, 0.5);
            let uv = Rect::from_min_max(middle - half, middle + half);
            let inner = square.shrink(2.0);
            egui::Image::new((texture.id(), inner.size()))
                .uv(uv)
                .corner_radius(theme.radius_md())
                .paint_at(ui, inner);
        } else {
            let room = square.shrink(8.0);
            let share = (room.width() / w).min(room.height() / h);
            let place = Rect::from_center_size(room.center(), Vec2::new(w, h) * share);
            let uv = Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0));
            painter.image(texture.id(), place, uv, Color32::WHITE);
        }
    }
    let painter = ui.painter();
    if on || response.hovered() {
        let color = if on {
            theme.c("accent")
        } else {
            theme.c("border")
        };
        let stroke = Stroke::new(if on { 2.5 } else { 1.5 }, color);
        painter.rect_stroke(square, theme.radius_md(), stroke, StrokeKind::Inside);
    }
    let galley = painter.layout_no_wrap(name.clone(), theme.font("caption"), theme.c("text"));
    let x = square.center().x - (galley.size().x / 2.0).min(side / 2.0);
    painter
        .with_clip_rect(rect.intersect(ui.clip_rect()))
        .galley(Pos2::new(x, square.bottom() + 3.0), galley, theme.c("text"));
    response.widget_info(|| {
        egui::WidgetInfo::selected(egui::WidgetType::Button, true, on, name.clone())
    });
    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// A tool: its icon over its name.
fn tool_tile(
    ui: &mut Ui,
    theme: &Theme,
    (icon, name): (Icon, String),
    (side, enabled): (f32, bool),
) -> bool {
    // Room for a name of two rows, whole.
    let size = Vec2::new(side, 84.0);
    // Not in a scope of its own: it goes to the next row when it does not fit.
    let sense = if enabled {
        Sense::click()
    } else {
        Sense::hover()
    };
    {
        let (rect, response) = ui.allocate_exact_size(size, sense);
        let fill = if response.hovered() && enabled {
            theme.c("surface.sunken")
        } else {
            theme.c("surface")
        };
        let ink = if enabled {
            theme.c("text")
        } else {
            theme.c("text.secondary").gamma_multiply(0.5)
        };
        ui.painter().rect(
            rect,
            theme.radius_md(),
            fill,
            Stroke::new(1.0, theme.c("border")),
            StrokeKind::Inside,
        );
        let at = Rect::from_center_size(
            Pos2::new(rect.center().x, rect.top() + 23.0),
            Vec2::splat(22.0),
        );
        crate::icons::paint(ui.painter(), at, icon, crate::icons::Weight::Regular, ink);
        let mut job =
            egui::text::LayoutJob::simple(name.clone(), theme.font("caption"), ink, side - 10.0);
        job.halign = egui::Align::Center;
        let galley = ui.painter().layout_job(job);
        // In the middle of what is left under the icon.
        let middle = rect.top() + 60.0;
        let top = (middle - galley.size().y / 2.0).max(rect.top() + 38.0);
        let at = Pos2::new(rect.center().x, top);
        ui.painter().galley(at, galley, ink);
        response.widget_info(|| {
            egui::WidgetInfo::labeled(egui::WidgetType::Button, enabled, name.clone())
        });
        enabled
            && response
                .on_hover_cursor(egui::CursorIcon::PointingHand)
                .clicked()
    }
}

impl Editing {
    pub fn new(page: PageId, node: NodeId) -> Self {
        Self {
            page,
            node,
            view: View::Home,
            zone: None,
            veil: true,
            tool: None,
            magic: None,
            crop: None,
            to_crop: false,
            cropped: None,
            whole: (1.0, 1.0),
            accept: false,
            brush_size: 40.0,
            adding: true,
            detect: false,
            color: None,
            masks: Masks::default(),
            note: None,
            step: None,
        }
    }

    /// One that opens on «Recortar».
    pub fn cropping(page: PageId, node: NodeId) -> Self {
        Self {
            to_crop: true,
            ..Self::new(page, node)
        }
    }

    pub fn is_cropping(&self) -> bool {
        self.crop.is_some()
    }

    /// The image cropped and its box, to set in one step.
    pub fn take_crop(&mut self) -> Option<(ImageNode, Placement)> {
        self.cropped.take()
    }

    /// «Aplicar», when it crops.
    pub fn accept(&mut self) {
        self.accept = self.crop.is_some();
    }

    /// «Aplicar» of «Recortar».
    pub fn apply_crop(&mut self, image: &ImageNode, placement: &Placement) {
        let Some(cropper) = self.crop.take() else {
            return;
        };
        let whole = image.part().whole_box(placement);
        let next = image.cropped(cropper.crop.clone());
        if next != *image {
            self.cropped = Some((next, cropper.crop.part_box(&whole)));
        }
        self.home_again();
    }

    /// The zone chosen goes when the document has it no more; one on its way
    /// is not there yet.
    fn settle(&mut self, image: &ImageNode) {
        let zones = image.look.as_ref().map_or(0, |look| look.zones.len());
        if self.step.is_none() {
            self.zone = self.zone.filter(|at| *at < zones);
        }
    }

    /// Whether the pointer chooses a zone instead of moving the image.
    pub fn choosing(&self) -> bool {
        match &self.magic {
            _ if self.crop.is_some() => true,
            Some(magic) => magic.sent.is_none(),
            None => self.tool.is_some(),
        }
    }

    /// Leaves the tool; whether there was one.
    pub fn leave(&mut self) -> bool {
        if self.crop.is_some() {
            self.home_again();
            return true;
        }
        match &mut self.magic {
            Some(magic) if magic.sent.is_some() => true,
            Some(magic) if magic.brush.is_marked() => {
                magic.brush.clear();
                true
            }
            Some(_) => {
                self.home_again();
                true
            }
            None => self.tool.take().is_some(),
        }
    }

    fn home_again(&mut self) {
        self.view = View::Home;
        self.tool = None;
        self.magic = None;
        self.crop = None;
        self.note = None;
    }

    /// Takes back, or does again, a stroke of what is marked to be erased;
    /// whether there was one: the document's step otherwise.
    pub fn unmark(&mut self, again: bool) -> bool {
        let Some(magic) = self.magic.as_mut().filter(|m| m.sent.is_none()) else {
            return false;
        };
        let changed = if again {
            magic.brush.paint.redo()
        } else {
            magic.brush.paint.undo()
        };
        if changed.is_some() {
            magic.brush.texture = None;
        }
        changed.is_some()
    }

    /// The brush there is.
    fn brush(&mut self) -> Option<&mut Brush> {
        match (&mut self.magic, &mut self.tool) {
            (Some(magic), _) => Some(&mut magic.brush),
            (None, Some(Tool::Brush(brush))) => Some(brush),
            _ => None,
        }
    }

    /// A change that is a step of its own, to make.
    pub fn take_step(&mut self) -> Option<ImageNode> {
        self.step.take()
    }

    /// The panel, over what the document has now.
    pub fn panel(&mut self, ui: &mut Ui, with: &mut With<'_>, image: &ImageNode) -> Asks {
        let mut asks = Asks::default();
        // A step undone may have taken the zone with it.
        self.settle(image);
        if std::mem::take(&mut self.to_crop) {
            self.open_crop(image);
        }
        let facts = with.looks.facts(ui.ctx(), with.store, image.asset);
        self.header(ui, with, &mut asks);
        ui.add_space(8.0);
        egui::TopBottomPanel::bottom("image.actions")
            .show_separator_line(false)
            .frame(egui::Frame::NONE.inner_margin(egui::Margin {
                top: 12,
                bottom: 8,
                ..egui::Margin::ZERO
            }))
            .show_inside(ui, |ui| self.footer(ui, with, image, &mut asks));
        egui::ScrollArea::vertical()
            .id_salt(("image.panel", self.view as u8))
            .auto_shrink([false, false])
            .show(ui, |ui| {
                // Room for the scroll bar, beside the controls.
                ui.set_max_width(ui.available_width() - 14.0);
                match self.view {
                    View::Home => self.home(ui, with, (image, facts.as_deref()), &mut asks),
                    View::Adjust => self.adjust(ui, with, (image, facts.as_deref()), &mut asks),
                    View::Filters => filters(ui, with, image, &mut asks),
                    View::Shadows => shadows(ui, with, image, &mut asks),
                    View::Magic => self.magic(ui, with, image, &mut asks),
                    View::Crop => self.crop_page(ui, with, facts.as_deref()),
                }
                ui.add_space(16.0);
            });
        asks
    }

    fn header(&mut self, ui: &mut Ui, with: &With<'_>, asks: &mut Asks) {
        let theme = with.theme;
        let title = match self.view {
            View::Home => "image.title",
            View::Adjust => "image.adjust",
            View::Filters => "image.filters",
            View::Shadows => "image.shadows",
            View::Magic if self.magic.as_ref().is_some_and(|m| m.stamp) => "image.stamp",
            View::Magic => "image.magic",
            View::Crop => "image.crop",
        };
        ui.horizontal(|ui| {
            if self.view != View::Home {
                let back = IconButton::new(Icon::Back, with.t("image.back"));
                let free = self.magic.as_ref().is_none_or(|m| m.sent.is_none());
                if back.small().show(ui, theme).clicked() && free {
                    self.home_again();
                }
            }
            ui.label(RichText::new(with.t(title)).font(theme.font("h3")));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let close = ui
                    .add(egui::Button::new(RichText::new("×").size(22.0)).frame(false))
                    .on_hover_text(with.t("image.close"));
                asks.close = close.clicked();
            });
        });
    }

    /// «Comparar», and what takes off what the page puts on.
    fn footer(&mut self, ui: &mut Ui, with: &mut With<'_>, image: &ImageNode, asks: &mut Asks) {
        let theme = with.theme;
        let look = image.look.as_deref();
        let (key, on) = match self.view {
            View::Home => ("image.reset_all", look.is_some()),
            View::Adjust => (
                "image.reset",
                look.and_then(|l| adjust_of(l, self.zone))
                    .is_some_and(|a| !a.is_plain()),
            ),
            View::Filters => ("image.remove", look.is_some_and(|l| l.filter.is_some())),
            View::Shadows => ("image.remove", look.is_some_and(|l| l.shadow.is_some())),
            View::Magic => ("image.back_to", self.changed(image) && !with.studio.busy()),
            View::Crop => (
                "image.crop_whole",
                self.crop.as_ref().is_some_and(|c| !c.crop.is_whole()),
            ),
        };
        // What «Comparar» shows: the image without its look, or as it was
        // when it began to be erased.
        let against = match (&self.magic, look) {
            (Some(magic), _) => self.changed(image).then_some(magic.before.asset),
            (None, Some(_)) if self.crop.is_none() => Some(image.asset),
            _ => None,
        };
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = GAP;
            let pressed = ui.add_enabled_ui(on, |ui| {
                widgets::secondary(ui, theme, with.t(key)).clicked()
            });
            if pressed.inner {
                let zone = self.zone;
                match self.view {
                    View::Home => {
                        self.zone = None;
                        self.tool = None;
                        asks.edit(image, |l| *l = ImageLook::default());
                    }
                    View::Adjust => asks.edit(image, |l| {
                        if let Some(adjust) = adjust_mut(l, zone) {
                            *adjust = ImageAdjust::default();
                        }
                    }),
                    View::Filters => asks.edit(image, |l| l.filter = None),
                    View::Shadows => asks.edit(image, |l| l.shadow = None),
                    View::Crop => {
                        if let Some(cropper) = &mut self.crop {
                            cropper.whole();
                        }
                    }
                    View::Magic => {
                        if let Some(magic) = &mut self.magic {
                            magic.brush.clear();
                            let target = Target {
                                page: self.page,
                                node: self.node,
                                image: image.clone(),
                            };
                            asks.vision = Some(Ask::Back(target, magic.before.clone()));
                        }
                    }
                }
            }
            ui.add_enabled_ui(against.is_some(), |ui| {
                let held = widgets::secondary(ui, theme, with.t("image.compare"))
                    .on_hover_text(with.t("image.compare_hint"))
                    .is_pointer_button_down_on();
                with.looks.compare = against.filter(|_| held).map(|a| (self.node, a));
            });
        });
    }

    /// Where what is adjusted applies: the whole image, or a part of it.
    fn choose(
        &mut self,
        ui: &mut Ui,
        with: &mut With<'_>,
        (image, facts): (&ImageNode, Option<&Facts>),
        asks: &mut Asks,
    ) {
        let theme = with.theme;
        let zones = image.look.as_ref().map_or(&[][..], |look| &look.zones);
        heading(ui, theme, with.t("image.select"));
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = Vec2::new(6.0, 6.0);
            let free = !self.masks.busy();
            let all = self.zone.is_none() && self.tool.is_none();
            if chip(ui, theme, with.t("image.all"), all).clicked() {
                self.zone = None;
                self.tool = None;
                self.note = None;
            }
            let picking = matches!(self.tool, Some(Tool::Pick));
            let pick = chip(ui, theme, with.t("image.element"), picking);
            if pick.on_hover_text(with.t("image.element_hint")).clicked() {
                self.tool = (!picking).then_some(Tool::Pick);
                self.zone = None;
                self.note = None;
                self.view = View::Adjust;
            }
            let painting = matches!(self.tool, Some(Tool::Brush(_)));
            let area = chip(ui, theme, with.t("image.area"), painting);
            if area.on_hover_text(with.t("image.area_hint")).clicked()
                && let Some(facts) = facts
            {
                self.note = None;
                self.view = View::Adjust;
                if painting {
                    self.tool = None;
                } else {
                    self.zone = self
                        .zone
                        .filter(|at| zones.get(*at).is_some_and(|z| z.kind == ZoneKind::Area));
                    self.begin_brush(with, image, facts.size);
                }
            }
            for kind in [ZoneKind::Foreground, ZoneKind::Background] {
                let there = zones.iter().position(|zone| zone.kind == kind);
                let on = there.is_some() && self.zone == there;
                let name = with.t(zone_key(kind));
                let able = free || there.is_some();
                let pressed = chip_if(ui, theme, name, (on, able)).clicked();
                if pressed {
                    self.tool = None;
                    self.note = None;
                    self.view = View::Adjust;
                    self.subject(ui.ctx(), with, image, kind);
                }
            }
        });
        self.finding(ui, with, asks);
        match &self.tool {
            Some(Tool::Pick) => {
                ui.add_space(8.0);
                hint(ui, theme, with.t("image.pick_hint"));
            }
            Some(Tool::Brush(_)) => self.brush_controls(ui, with),
            None => {}
        }
        self.zone_list(ui, with, image, asks);
    }

    /// A mask on its way, and what is said of the last one.
    fn finding(&mut self, ui: &mut Ui, with: &With<'_>, asks: &mut Asks) {
        let theme = with.theme;
        if self.masks.busy() {
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.add(egui::Spinner::new().size(18.0).color(theme.c("accent")));
                ui.label(RichText::new(with.t("image.finding")).font(theme.font("body-sm")));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button(with.t("vision.cancel")).clicked() {
                        self.masks.cancel();
                    }
                });
            });
        }
        if let Some(note) = self.note {
            ui.add_space(8.0);
            let color = theme.c("error.fg");
            let text = RichText::new(with.t(note)).font(theme.font("body-sm"));
            widgets::note(ui, Icon::Warning, text, color);
            if note == "image.no_model" && ui.button(with.t("vision.models")).clicked() {
                asks.vision = Some(Ask::Models);
            }
        }
    }

    /// The zones there are: to choose one, or take it off.
    fn zone_list(&mut self, ui: &mut Ui, with: &With<'_>, image: &ImageNode, asks: &mut Asks) {
        let theme = with.theme;
        let zones = image.look.as_ref().map_or(&[][..], |look| &look.zones);
        if zones.is_empty() {
            return;
        }
        heading(ui, theme, with.t("image.zones"));
        let mut gone = None;
        for (at, zone) in zones.iter().enumerate() {
            ui.push_id(("image.zone", at), |ui| {
                ui.horizontal(|ui| {
                    let count = zones[..=at].iter().filter(|z| z.kind == zone.kind).count();
                    let single = matches!(zone.kind, ZoneKind::Foreground | ZoneKind::Background);
                    let mut name = with.t(zone_key(zone.kind));
                    if !single {
                        name = format!("{name} {count}");
                    }
                    if chip(ui, theme, name, self.zone == Some(at)).clicked() {
                        self.zone = Some(at);
                        self.tool = None;
                        self.veil = true;
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let delete = IconButton::new(Icon::Delete, with.t("image.delete_zone"));
                        if delete.small().show(ui, theme).clicked() {
                            gone = Some(at);
                        }
                    });
                });
            });
        }
        if let Some(gone) = gone {
            self.zone = match self.zone {
                Some(at) if at == gone => None,
                Some(at) if at > gone => Some(at - 1),
                other => other,
            };
            self.tool = None;
            asks.edit(image, |look| {
                look.zones.remove(gone);
            });
        }
        if self.zone.is_some() {
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                widgets::toggle(ui, theme, &mut self.veil);
                ui.label(RichText::new(with.t("image.veil")).font(theme.font("body-sm")));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button(with.t("image.deselect")).clicked() {
                        self.zone = None;
                        self.tool = None;
                    }
                });
            });
        }
    }

    fn brush_controls(&mut self, ui: &mut Ui, with: &With<'_>) {
        let theme = with.theme;
        ui.add_space(10.0);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = GAP;
            let half = (ui.available_width() - GAP) / 2.0;
            for (adding, key) in [(true, "image.add"), (false, "image.subtract")] {
                let button = egui::Button::selectable(self.adding == adding, with.t(key))
                    .min_size(Vec2::new(half, 34.0));
                if ui.add(button).clicked() {
                    self.adding = adding;
                }
            }
        });
        ui.add_space(8.0);
        let mut size = f64::from(self.brush_size);
        let range = (f64::from(*BRUSH.start()), f64::from(*BRUSH.end()));
        let name = with.t("image.brush_size");
        if slider(ui, theme, name, (&mut size, range, 40.0), Track::Plain) {
            self.brush_size = size as f32;
        }
        ui.horizontal(|ui| {
            ui.label(RichText::new(with.t("image.detect")).font(theme.font("body-sm")));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                widgets::toggle(ui, theme, &mut self.detect)
                    .on_hover_text(with.t("image.detect_hint"));
            });
        });
    }

    fn home(
        &mut self,
        ui: &mut Ui,
        with: &mut With<'_>,
        (image, facts): (&ImageNode, Option<&Facts>),
        asks: &mut Asks,
    ) {
        let theme = with.theme;
        self.choose(ui, with, (image, facts), asks);
        heading(ui, theme, with.t("image.tools"));
        let side = tile_side(ui);
        let target = Target {
            page: self.page,
            node: self.node,
            image: image.clone(),
        };
        let free = !with.studio.busy();
        let models = with.studio.available();
        let mut opened = None;
        let mut crop = false;
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = Vec2::splat(GAP);
            let tool = |ui: &mut Ui, icon, key, enabled| {
                tool_tile(ui, theme, (icon, with.t(key)), (side, enabled))
            };
            if tool(ui, Icon::Identity, "image.adjust", true) {
                self.view = View::Adjust;
            }
            if tool(ui, Icon::Crop, "image.crop", true) {
                crop = true;
            }
            if models {
                if tool(ui, Icon::CategoryPhotos, "image.cut", free) {
                    asks.vision = Some(Ask::Work(target.clone(), Work::Cut));
                }
                if tool(ui, Icon::CategoryLights, "image.enhance", free) {
                    asks.vision = Some(Ask::Work(target.clone(), Work::Enhance(2)));
                }
                if image.is_cut() && tool(ui, Icon::Pieces, "image.erase", free) {
                    asks.vision = Some(Ask::Erase(target.clone()));
                }
            }
            if models {
                if tool(ui, Icon::Drawing, "image.magic", free) {
                    opened = Some(false);
                }
                if tool(ui, Icon::CategoryFrames, "image.stamp", free) {
                    opened = Some(true);
                }
            }
            if image.edit.is_some() && tool(ui, Icon::Versions, "image.original", free) {
                asks.vision = Some(Ask::Original(target.clone()));
            }
        });
        if crop {
            self.open_crop(image);
        }
        if let (Some(stamp), Some(facts)) = (opened, facts) {
            self.open_magic(image, facts.size, stamp);
        }
        if let Some(ask) = vision::progress(ui, theme, with.texts, with.studio) {
            asks.vision = Some(ask);
        }
        let link = (with.t("image.filters"), with.t("image.see_all"));
        if heading_with(ui, theme, link, true) {
            self.view = View::Filters;
        }
        with.looks
            .want_thumbs(ui.ctx(), with.store, Set::Filters, image);
        let first: Vec<&str> = FILTER_GROUPS.iter().take(3).map(|g| g.filters[0]).collect();
        filter_row(ui, with, image, &first, asks);
        let link = (with.t("image.shadows"), with.t("image.see_all"));
        if heading_with(ui, theme, link, true) {
            self.view = View::Shadows;
        }
        with.looks
            .want_thumbs(ui.ctx(), with.store, Set::Shadows, image);
        let row = [None, Some(ShadowKind::Glow), Some(ShadowKind::Drop)];
        shadow_row(ui, with, image, &row, asks);
    }

    fn adjust(
        &mut self,
        ui: &mut Ui,
        with: &mut With<'_>,
        (image, facts): (&ImageNode, Option<&Facts>),
        asks: &mut Asks,
    ) {
        let theme = with.theme;
        self.choose(ui, with, (image, facts), asks);
        let zone = self.zone;
        let Some(now) = image
            .look
            .as_deref()
            .map_or(Some(ImageAdjust::default()), |l| {
                adjust_of(l, zone).cloned()
            })
        else {
            return;
        };
        let mut next = now.clone();
        ui.add_space(14.0);
        let auto = facts.and_then(|facts| facts.auto);
        ui.add_enabled_ui(auto.is_some(), |ui| {
            let wide = Vec2::new(ui.available_width(), 36.0);
            let button = egui::Button::new(with.t("image.auto"));
            if ui.add_sized(wide, button).clicked()
                && let Some(auto) = auto
            {
                (next.temperature, next.tint) = (auto.temperature, auto.tint);
                (next.brightness, next.whites) = (auto.brightness, auto.whites);
                next.blacks = auto.blacks;
            }
        });
        group(ui, with, ("image.white_balance", &WHITE_BALANCE), &mut next);
        group(ui, with, ("image.light", &LIGHT), &mut next);
        let plain_color = next.invert || COLOR.iter().any(|c| *(c.field)(&mut next) != 0.0);
        let link = (with.t("image.color"), with.t("image.reset"));
        if heading_with(ui, theme, link, plain_color) {
            next.invert = false;
            COLOR.iter().for_each(|c| *(c.field)(&mut next) = 0.0);
        }
        ui.horizontal(|ui| {
            ui.label(RichText::new(with.t("image.invert")).font(theme.font("body-sm")));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                widgets::toggle(ui, theme, &mut next.invert);
            });
        });
        ui.add_space(6.0);
        controls(ui, with, &COLOR, &mut next);
        if let Some(facts) = facts {
            self.colors(ui, with, facts, &mut next);
        }
        group(ui, with, ("image.texture", &TEXTURE), &mut next);
        if next != now {
            // What is done is seen, not where.
            self.veil = false;
            asks.edit(image, |look| {
                if let Some(adjust) = adjust_mut(look, zone) {
                    *adjust = next;
                }
            });
        }
    }

    /// The colours of the image, each moved on its own.
    fn colors(&mut self, ui: &mut Ui, with: &With<'_>, facts: &Facts, next: &mut ImageAdjust) {
        let theme = with.theme;
        let colors: Vec<_> = facts.colors.iter().take(6).copied().collect();
        if colors.is_empty() {
            return;
        }
        let link = (with.t("image.color_edit"), with.t("image.reset"));
        if heading_with(ui, theme, link, !next.colors.is_empty()) {
            next.colors.clear();
        }
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = Vec2::splat(10.0);
            for (at, color) in colors.iter().enumerate() {
                let (rect, response) = ui.allocate_exact_size(Vec2::splat(34.0), Sense::click());
                let on = self.color == Some(at);
                let painter = ui.painter();
                painter.circle_filled(rect.center(), 13.0, canvas::screen_color(*color));
                painter.circle_stroke(rect.center(), 13.0, Stroke::new(1.0, theme.c("border")));
                if on {
                    painter.circle_stroke(rect.center(), 16.0, Stroke::new(2.0, theme.c("accent")));
                }
                if next.colors.iter().any(|shift| shift.color == *color) {
                    painter.circle_filled(
                        rect.right_top() + Vec2::new(-6.0, 6.0),
                        4.0,
                        theme.c("primary.bg"),
                    );
                }
                if response
                    .on_hover_cursor(egui::CursorIcon::PointingHand)
                    .clicked()
                {
                    self.color = (!on).then_some(at);
                }
            }
        });
        let Some(color) = self.color.and_then(|at| colors.get(at)).copied() else {
            ui.add_space(6.0);
            hint(ui, theme, with.t("image.color_hint"));
            return;
        };
        let found = next.colors.iter().position(|shift| shift.color == color);
        let mut shift = found.map_or(
            ColorShift {
                color,
                hue: 0.0,
                saturation: 0.0,
            },
            |at| next.colors[at],
        );
        ui.add_space(8.0);
        let hue = (&mut shift.hue, TURN, 0.0);
        let mut changed = slider(ui, theme, with.t("image.hue"), hue, Track::Hue);
        let saturation = (&mut shift.saturation, BOTH, 0.0);
        let name = with.t("image.saturation");
        changed |= slider(ui, theme, name, saturation, Track::Saturation);
        if !changed {
            return;
        }
        let plain = shift.hue == 0.0 && shift.saturation == 0.0;
        match found {
            Some(at) if plain => {
                next.colors.remove(at);
            }
            Some(at) => next.colors[at] = shift,
            None if plain || next.colors.len() >= ImageAdjust::MAX_COLORS => {}
            None => next.colors.push(shift),
        }
    }

    /// What was marked is erased: it begins again.
    pub fn erased(&mut self) {
        if let Some(magic) = &mut self.magic
            && magic.sent.take().is_some()
        {
            magic.brush.clear();
        }
    }

    fn open_crop(&mut self, image: &ImageNode) {
        self.view = View::Crop;
        self.tool = None;
        self.zone = None;
        self.magic = None;
        self.note = None;
        self.crop = Some(Cropper::new(image.part()));
    }

    fn crop_page(&mut self, ui: &mut Ui, with: &mut With<'_>, facts: Option<&Facts>) {
        let theme = with.theme;
        let Some(cropper) = &mut self.crop else {
            return;
        };
        // The proportions of the box are the ones that are seen.
        let size = self.whole;
        heading(ui, theme, with.t("image.crop_shape"));
        let side = ((ui.available_width() - 3.0 * GAP) / 4.0).floor().max(40.0);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = GAP;
            let shapes = [
                ("image.crop_rect", CropShape::Rect),
                ("image.crop_rounded", CropShape::Rounded(0.15)),
                ("image.crop_circle", CropShape::Ellipse),
                ("image.crop_hand", CropShape::Path(Vec::new())),
            ];
            for (key, shape) in shapes {
                let now = &cropper.crop.shape;
                let same = std::mem::discriminant(now) == std::mem::discriminant(&shape);
                let by_hand = matches!(shape, CropShape::Path(_));
                let on = if cropper.drawing { by_hand } else { same };
                if shape_tile(ui, theme, (&shape, with.t(key)), (side, on)).clicked() {
                    cropper.cut(shape, size);
                }
            }
        });
        if cropper.drawing {
            ui.add_space(8.0);
            hint(ui, theme, with.t("image.crop_draw"));
        }
        if let CropShape::Rounded(radius) = &mut cropper.crop.shape {
            ui.add_space(10.0);
            let mut shown = (*radius * 200.0).round();
            let what = (&mut shown, UP, 30.0);
            if slider(ui, theme, with.t("image.crop_radius"), what, Track::Plain) {
                *radius = shown / 200.0;
            }
        }
        heading(ui, theme, with.t("image.crop_ratio"));
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = Vec2::new(6.0, 6.0);
            for (at, (name, _)) in RATIOS.iter().enumerate() {
                let name = match at {
                    0 | 1 => with.t(name),
                    _ => (*name).to_owned(),
                };
                if chip(ui, theme, name, cropper.ratio == at).clicked() {
                    cropper.keep(at, size);
                }
            }
        });
        if let Some(facts) = facts {
            let of = |side: u32, share: f64| (f64::from(side) * share).round();
            let (w, h) = facts.size;
            let text = format!("{} × {} px", of(w, cropper.crop.w), of(h, cropper.crop.h));
            ui.add_space(12.0);
            hint(ui, theme, text);
        }
        ui.add_space(14.0);
        self.accept |= main_button(ui, theme, with.t("image.crop_apply"), true);
        ui.add_space(8.0);
        let wide = Vec2::new(ui.available_width(), 38.0);
        let cancel = egui::Button::new(with.t("vision.cancel"));
        if ui.add_sized(wide, cancel).clicked() {
            self.home_again();
        }
        ui.add_space(8.0);
        hint(ui, theme, with.t("image.crop_hint"));
    }

    /// Whether it is not as it was when it began to be erased.
    fn changed(&self, image: &ImageNode) -> bool {
        self.magic.as_ref().is_some_and(|magic| {
            (magic.before.asset, magic.before.edit) != (image.asset, image.edit)
        })
    }

    /// «Borrador mágico», or «Quitar marca de agua».
    fn open_magic(&mut self, image: &ImageNode, size: (u32, u32), stamp: bool) {
        let Some(brush) = Brush::blank(size) else {
            return;
        };
        self.view = View::Magic;
        self.tool = None;
        self.zone = None;
        self.note = None;
        self.veil = true;
        self.adding = true;
        self.brush_size = 24.0;
        self.magic = Some(Magic {
            before: image.clone(),
            stamp,
            pick: false,
            brush: Box::new(brush),
            sent: None,
        });
    }

    fn magic(&mut self, ui: &mut Ui, with: &mut With<'_>, image: &ImageNode, asks: &mut Asks) {
        let theme = with.theme;
        if let Some(magic) = &mut self.magic
            && magic.sent.is_some()
            && !with.studio.busy()
        {
            // It failed, or was cancelled: what is marked stays.
            magic.sent = None;
        }
        let Some((stamp, pick, marked, working)) = self.magic.as_ref().map(|magic| {
            let marked = magic.brush.is_marked();
            (magic.stamp, magic.pick, marked, magic.sent.is_some())
        }) else {
            return;
        };
        if working {
            ui.add_space(8.0);
            if let Some(ask) = vision::progress(ui, theme, with.texts, with.studio) {
                asks.vision = Some(ask);
            }
            ui.add_space(8.0);
            hint(ui, theme, with.t("image.magic_working"));
            return;
        }
        let link = (with.t("image.magic_choose"), with.t("image.deselect"));
        if heading_with(ui, theme, link, marked)
            && let Some(magic) = &mut self.magic
        {
            magic.brush.clear();
        }
        let free = !self.masks.busy() && !with.studio.busy();
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = Vec2::new(6.0, 6.0);
            let area = chip(ui, theme, with.t("image.area"), !pick).clicked();
            let element = chip(ui, theme, with.t("image.element"), pick).clicked();
            if (area || element)
                && let Some(magic) = &mut self.magic
            {
                magic.pick = element;
                self.note = None;
            }
            let name = with.t("image.foreground");
            if chip_if(ui, theme, name, (false, free)).clicked() {
                self.note = None;
                let what = (self.node, image.asset, ZoneKind::Foreground);
                let models = with.studio.models();
                self.masks.subject(ui.ctx(), what, with.store, models);
            }
        });
        self.finding(ui, with, asks);
        if pick {
            ui.add_space(8.0);
            hint(ui, theme, with.t("image.magic_pick"));
        } else {
            self.brush_controls(ui, with);
        }
        ui.add_space(14.0);
        self.erase_button(ui, with, (image, marked && free), asks);
        ui.add_space(8.0);
        let key = if stamp {
            "image.stamp_note"
        } else {
            "image.magic_note"
        };
        hint(ui, theme, with.t(key));
    }

    /// «Borrar», or what is missing to.
    fn erase_button(
        &mut self,
        ui: &mut Ui,
        with: &With<'_>,
        (image, able): (&ImageNode, bool),
        asks: &mut Asks,
    ) {
        let theme = with.theme;
        let model = with.studio.inpainting();
        let Some(model) = model.filter(|model| !model.installed) else {
            if main_button(ui, theme, with.t("image.magic_erase"), able)
                && let Some(magic) = &mut self.magic
            {
                magic.sent = Some(image.asset);
                let target = Target {
                    page: self.page,
                    node: self.node,
                    image: image.clone(),
                };
                let marked = vision::Marked {
                    alpha: magic.brush.paint.alpha().to_vec(),
                    size: magic.brush.size,
                };
                asks.vision = Some(Ask::Fill(target, Arc::new(marked)));
            }
            // Nothing runs: what was said of the last one.
            vision::progress(ui, theme, with.texts, with.studio);
            return;
        };
        ui.label(RichText::new(with.t("image.magic_model")).font(theme.font("body-sm")));
        ui.add_space(8.0);
        if let Some(share) = with.studio.downloading() {
            vision::bar(ui, theme, share);
            ui.add_space(6.0);
            if ui.button(with.t("vision.cancel")).clicked() {
                asks.vision = Some(Ask::Cancel);
            }
            return;
        }
        let name = format!(
            "{} {} ({})",
            with.t("vision.download"),
            model.name,
            vision::size(model.download_bytes)
        );
        if main_button(ui, theme, name, !with.studio.busy()) {
            asks.vision = Some(Ask::Install(model.id));
        }
        ui.add_space(6.0);
        if ui.button(with.t("vision.models")).clicked() {
            asks.vision = Some(Ask::Models);
        }
        vision::progress(ui, theme, with.texts, with.studio);
    }

    /// The zone of the subject, or of what is not it.
    fn subject(&mut self, ctx: &egui::Context, with: &With<'_>, image: &ImageNode, kind: ZoneKind) {
        let look = image.look.as_deref().cloned().unwrap_or_default();
        if let Some(at) = look.zones.iter().position(|zone| zone.kind == kind) {
            self.zone = Some(at);
            self.veil = true;
            return;
        }
        // The other one has the same mask.
        let other = look
            .zones
            .iter()
            .find(|z| matches!(z.kind, ZoneKind::Foreground | ZoneKind::Background));
        if let Some(other) = other {
            self.put_zone(image, kind, other.mask);
            return;
        }
        let models: Option<Arc<dyn VisionModels>> = with.studio.models();
        let what = (self.node, image.asset, kind);
        self.masks.subject(ctx, what, with.store, models);
    }

    /// A new zone of `kind`, chosen.
    fn put_zone(&mut self, image: &ImageNode, kind: ZoneKind, mask: AssetRef) {
        let look = image.look.as_deref().cloned().unwrap_or_default();
        match with_zone(&look, kind, mask) {
            Some((look, at)) => {
                self.zone = Some(at);
                self.veil = true;
                self.step = Some(image.looking(look));
            }
            None => self.note = Some("image.too_many"),
        }
    }

    /// Begins to paint: the zone chosen, or a new one.
    fn begin_brush(&mut self, with: &With<'_>, image: &ImageNode, (w, h): (u32, u32)) {
        let zones = image.look.as_ref().map_or(&[][..], |look| &look.zones);
        let of = self.zone.and_then(|at| zones.get(at)).map(|z| z.mask);
        let read = of.and_then(|asset| with.looks.mask_pixels(&**with.store, asset));
        let (size, alpha) = match read {
            Some(mask) => ((mask.width(), mask.height()), mask.data().to_vec()),
            None => {
                let size = mask_size((w, h));
                (size, vec![0; size.0 as usize * size.1 as usize])
            }
        };
        let whole = vec![255; alpha.len()];
        self.veil = true;
        let paint = Paint::new(size, alpha, whole);
        self.tool = paint.map(|paint| {
            Tool::Brush(Box::new(Brush {
                paint,
                size,
                of,
                texture: None,
                from: None,
                stroke: Vec::new(),
            }))
        });
    }

    /// What the threads made of the image it is now.
    pub fn arrived(&mut self, store: &dyn ContentStore, image: &ImageNode) {
        for made in self.masks.pump() {
            if made.node == self.node && made.asset == image.asset {
                self.take(store, image, made);
            }
        }
    }

    fn take(&mut self, store: &dyn ContentStore, image: &ImageNode, made: Made) {
        use gafira_ports::vision::VisionError;
        let mask = match made.result {
            Ok(mask) => mask,
            Err(trouble) => {
                self.note = match trouble {
                    Trouble::Vision(VisionError::Cancelled) => None,
                    Trouble::Vision(VisionError::Unavailable(_)) => Some("image.no_model"),
                    Trouble::Vision(_) => Some("image.not_made"),
                    Trouble::Nothing => Some("image.not_found"),
                };
                return;
            }
        };
        self.note = (!mask.reliable).then_some("image.unsure");
        let size = (mask.mask.width(), mask.mask.height());
        match made.purpose {
            // What is found is marked to be erased.
            Purpose::Zone(_) if self.magic.is_some() => {
                self.mark(mask.mask.data(), size, true);
            }
            Purpose::Zone(kind) => {
                if let Some(asset) = stored(store, mask.mask.data(), size) {
                    self.tool = None;
                    self.put_zone(image, kind, asset);
                }
            }
            Purpose::Stroke(adding) => {
                if self.mark(mask.mask.data(), size, adding) && self.magic.is_none() {
                    self.keep_brush(store, image);
                }
            }
        }
    }

    /// Adds `found` to what is painted, or takes it away; whether it could.
    fn mark(&mut self, found: &[u8], size: (u32, u32), adding: bool) -> bool {
        let Some(brush) = self.brush() else {
            return false;
        };
        let Some(found) = stretched(found, size, brush.size) else {
            return false;
        };
        brush.paint.put(&found, adding);
        brush.texture = None;
        true
    }

    /// What is painted goes to the document.
    fn keep_brush(&mut self, store: &dyn ContentStore, image: &ImageNode) {
        let Some(Tool::Brush(brush)) = &mut self.tool else {
            return;
        };
        let Some(asset) = stored(store, brush.paint.alpha(), brush.size) else {
            return;
        };
        brush.of = Some(asset);
        let mut look = image.look.as_deref().cloned().unwrap_or_default();
        match self.zone.and_then(|at| look.zones.get_mut(at)) {
            Some(zone) => {
                zone.mask = asset;
                self.step = Some(image.looking(look));
            }
            None => self.put_zone(image, ZoneKind::Area, asset),
        }
    }

    /// The zone over the image, and the pointer that chooses it; whether the
    /// pointer is taken.
    pub fn overlay(
        &mut self,
        ui: &Ui,
        painter: &egui::Painter,
        with: &mut With<'_>,
        (response, turn, placement): (&Response, Affine, &Placement),
        image: &ImageNode,
    ) -> bool {
        let part = image.part();
        if let Some(cropper) = &mut self.crop {
            let whole = part.whole_box(placement);
            self.whole = (whole.w, whole.h);
            let turn = turn * placement.to_transform().inverse() * whole.to_transform();
            let mut all = image.clone();
            all.crop = None;
            if let Some(look) = &mut all.look {
                look.shadow = None;
            }
            let ctx = ui.ctx();
            let settled = !ui.input(|i| i.pointer.any_down());
            with.looks.want(ctx, with.store, (self.node, &all), settled);
            let picture = with.looks.exact(&all).or(with.plain);
            let what = (response, turn, &whole);
            let done = cropper.overlay(ui, painter, with.theme, what, picture);
            if std::mem::take(&mut self.accept) || done.is_some() {
                self.apply_crop(image, placement);
            }
            return true;
        }
        let zones = image.look.as_ref().map_or(&[][..], |look| &look.zones);
        self.settle(image);
        self.sync(
            with,
            zones.get(self.zone.unwrap_or(usize::MAX)).map(|z| z.mask),
        );
        let mint = with.theme.c("accent").gamma_multiply(VEIL);
        let veil = self.veil && with.looks.compare.is_none();
        let zone = self.zone;
        let texture = match self.brush() {
            Some(brush) => veil.then(|| brush.texture(ui.ctx())),
            None => zone
                .filter(|_| veil)
                .and_then(|at| zones.get(at))
                .and_then(|zone| {
                    let store = &**with.store;
                    with.looks.mask(ui.ctx(), store, zone.mask, zone.inverted)
                }),
        };
        if let Some(texture) = texture {
            let seen = ((part.x, part.y), (part.w, part.h));
            let mesh = canvas::image_mesh_part(texture, turn, placement, seen, mint);
            painter.add(mesh);
        }
        let hover = response.hover_pos();
        let share = |at: Pos2| {
            let p = rotation::local(turn, at)?;
            // Shares of the sides: small numbers.
            let of = |v: f64, side: f64, from: f64, size: f64| (from + v / side * size) as f32;
            Some([
                of(p.x, placement.w, part.x, part.w),
                of(p.y, placement.h, part.y, part.h),
            ])
        };
        let inside = |p: &[f32; 2]| p.iter().all(|v| (0.0..=1.0).contains(v));
        if self.magic.as_ref().is_some_and(|m| m.sent.is_some()) {
            // At work: nothing is chosen, and nothing moves.
            return true;
        }
        let picking = match &self.magic {
            Some(magic) => magic.pick,
            None => matches!(self.tool, Some(Tool::Pick)),
        };
        if picking {
            if hover.and_then(share).is_some_and(|p| inside(&p)) {
                ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair);
            }
            if response.clicked()
                && let Some(at) = response.interact_pointer_pos().and_then(share)
                && inside(&at)
            {
                let what = (self.node, image.asset, Purpose::Zone(ZoneKind::Element));
                let seed = (vec![at], 0.012);
                self.masks.element(ui.ctx(), what, with.store, seed);
            }
            return true;
        }
        let (brush_size, adding) = (self.brush_size, self.adding);
        let Some(brush) = self.brush() else {
            return false;
        };
        // The size of the brush on the mask, as large as it is seen.
        let side = |x: f64, y: f64| rotation::screen(turn, x, y);
        let wide = side(0.0, 0.0).distance(side(placement.w, 0.0)).max(1.0);
        // As wide as all of the image would be.
        let wide = wide / part.w as f32;
        let size = brush_size * brush.size.0 as f32 / wide;
        let mode = if adding { Mode::Restore } else { Mode::Erase };
        if let Some(at) = hover.or(response.interact_pointer_pos()) {
            let ring = brush_size / 2.0;
            let light = Stroke::new(2.5, Color32::from_white_alpha(200));
            painter.circle_stroke(at, ring, light);
            painter.circle_stroke(at, ring, Stroke::new(1.0, with.theme.c("topbar.bg")));
            ui.ctx().set_cursor_icon(egui::CursorIcon::None);
        }
        if response.is_pointer_button_down_on() && ui.input(|i| i.pointer.primary_down()) {
            let at = ui.input(|i| i.pointer.latest_pos()).and_then(share);
            if let Some(at) = at {
                let to = (at[0] * brush.size.0 as f32, at[1] * brush.size.1 as f32);
                if brush.from.is_none() {
                    brush.paint.begin();
                    brush.stroke.clear();
                }
                let from = brush.from.unwrap_or(to);
                if brush.paint.line(mode, from, to, size).is_some() {
                    brush.texture = None;
                }
                brush.from = Some(to);
                if inside(&at) {
                    brush.stroke.push(at);
                }
                self.veil = true;
            }
            ui.ctx().request_repaint();
        } else if brush.from.take().is_some() {
            brush.paint.end();
            let stroke = std::mem::take(&mut brush.stroke);
            if !stroke.is_empty() && self.detect {
                let radius = brush_size / 2.0 / wide;
                let what = (self.node, image.asset, Purpose::Stroke(adding));
                self.masks
                    .element(ui.ctx(), what, with.store, (stroke, radius));
            }
            if self.magic.is_none() {
                self.keep_brush(&**with.store, image);
            }
        }
        true
    }

    /// The brush follows the document: a step undone or done again.
    fn sync(&mut self, with: &With<'_>, mask: Option<AssetRef>) {
        let waits = self.step.is_some() || self.magic.is_some();
        let Some(Tool::Brush(brush)) = &mut self.tool else {
            return;
        };
        if waits || brush.from.is_some() || brush.of == mask {
            return;
        }
        let read = mask.and_then(|asset| with.looks.mask_pixels(&**with.store, asset));
        let (size, alpha) = match read {
            Some(read) => ((read.width(), read.height()), read.data().to_vec()),
            None => (brush.size, vec![0; brush.paint.alpha().len()]),
        };
        let whole = vec![255; alpha.len()];
        if let Some(paint) = Paint::new(size, alpha, whole) {
            (brush.paint, brush.size, brush.of) = (paint, size, mask);
            brush.texture = None;
        }
    }
}

impl Brush {
    /// One with nothing painted, for an image of `size`.
    fn blank(size: (u32, u32)) -> Option<Self> {
        let size = mask_size(size);
        let count = size.0 as usize * size.1 as usize;
        Some(Self {
            paint: Paint::new(size, vec![0; count], vec![255; count])?,
            size,
            of: None,
            texture: None,
            from: None,
            stroke: Vec::new(),
        })
    }

    fn is_marked(&self) -> bool {
        self.paint.alpha().iter().any(|a| *a >= 64)
    }

    fn clear(&mut self) {
        let all = vec![255; self.paint.alpha().len()];
        self.paint.put(&all, false);
        self.texture = None;
    }

    fn texture(&mut self, ctx: &egui::Context) -> egui::TextureId {
        let (alpha, size) = (self.paint.alpha(), self.size);
        self.texture
            .get_or_insert_with(|| {
                let image = white(alpha, size, false);
                ctx.load_texture("image.brush", image, egui::TextureOptions::LINEAR)
            })
            .id()
    }
}

/// The size of the mask painted over an image of `w` × `h`.
fn mask_size((w, h): (u32, u32)) -> (u32, u32) {
    let most = w.max(h).max(1);
    // No longer than it was, or than a mask: within a u32.
    let to = |v: u32| {
        let v = u64::from(v) * u64::from(PAINTED.min(most)) / u64::from(most);
        u32::try_from(v).unwrap_or(PAINTED).max(1)
    };
    (to(w), to(h))
}

/// `alpha` drawn out to another size.
fn stretched(alpha: &[u8], (w, h): (u32, u32), to: (u32, u32)) -> Option<Vec<u8>> {
    if (w, h) == to {
        return Some(alpha.to_vec());
    }
    let image = image::GrayImage::from_raw(w, h, alpha.to_vec())?;
    let filter = image::imageops::FilterType::Triangle;
    Some(image::imageops::resize(&image, to.0, to.1, filter).into_raw())
}

/// The one thing to do in a page, from side to side.
pub(crate) fn main_button(ui: &mut Ui, theme: &Theme, text: String, enabled: bool) -> bool {
    let text = RichText::new(text)
        .font(theme.font("ui-label"))
        .color(theme.c("primary.fg"));
    let button = egui::Button::new(text)
        .fill(theme.c("primary.bg"))
        .corner_radius(theme.radius_md());
    let wide = Vec2::new(ui.available_width(), 40.0);
    ui.add_enabled_ui(enabled, |ui| ui.add_sized(wide, button).clicked())
        .inner
}

fn hint(ui: &mut Ui, theme: &Theme, text: String) {
    let text = RichText::new(text)
        .font(theme.font("caption"))
        .color(theme.c("text.secondary"));
    ui.label(text);
}

/// The controls of `list`.
fn controls(ui: &mut Ui, with: &With<'_>, list: &[Control], adjust: &mut ImageAdjust) {
    for control in list {
        let value = (control.field)(adjust);
        let what = (value, control.range, 0.0);
        slider(ui, with.theme, with.t(control.key), what, control.track);
    }
}

/// A group of controls, with what takes them all back.
fn group(
    ui: &mut Ui,
    with: &With<'_>,
    (key, list): (&'static str, &[Control]),
    adjust: &mut ImageAdjust,
) {
    let changed = list.iter().any(|c| *(c.field)(adjust) != 0.0);
    let link = (with.t(key), with.t("image.reset"));
    if heading_with(ui, with.theme, link, changed) {
        list.iter().for_each(|c| *(c.field)(adjust) = 0.0);
    }
    controls(ui, with, list, adjust);
}

/// A row of filters.
fn filter_row(ui: &mut Ui, with: &With<'_>, image: &ImageNode, ids: &[&str], asks: &mut Asks) {
    let now = image.look.as_ref().and_then(|look| look.filter.as_ref());
    let side = tile_side(ui);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = GAP;
        for id in ids {
            let picture = (with.looks.thumb(Set::Filters, id), true);
            let on = now.is_some_and(|filter| filter.id == **id);
            let name = with.t(filter_key(id));
            if tile(ui, with.theme, picture, name, (side, on)).clicked() {
                asks.edit(image, |look| {
                    look.filter = (!on).then(|| ImageFilter {
                        id: (*id).to_owned(),
                        intensity: now.map_or(100.0, |filter| filter.intensity),
                    });
                });
            }
        }
    });
}

fn filters(ui: &mut Ui, with: &mut With<'_>, image: &ImageNode, asks: &mut Asks) {
    let theme = with.theme;
    with.looks
        .want_thumbs(ui.ctx(), with.store, Set::Filters, image);
    if let Some(filter) = image.look.as_ref().and_then(|look| look.filter.as_ref()) {
        ui.add_space(6.0);
        let mut intensity = filter.intensity;
        let what = (&mut intensity, UP, 100.0);
        if slider(ui, theme, with.t("image.intensity"), what, Track::Plain) {
            asks.edit(image, |look| {
                if let Some(filter) = &mut look.filter {
                    filter.intensity = intensity;
                }
            });
        }
    }
    for group in &FILTER_GROUPS {
        heading(ui, theme, with.t(group_key(group.id)));
        for row in group.filters.chunks(3) {
            filter_row(ui, with, image, row, asks);
            ui.add_space(6.0);
        }
    }
}

/// A row of shadows; the first one may be none.
fn shadow_row(
    ui: &mut Ui,
    with: &With<'_>,
    image: &ImageNode,
    kinds: &[Option<ShadowKind>],
    asks: &mut Asks,
) {
    let now = image.look.as_ref().and_then(|look| look.shadow);
    let side = tile_side(ui);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = GAP;
        for kind in kinds {
            let on = now.map(|shadow| shadow.kind) == *kind;
            let name = with.t(kind.map_or("image.shadow_none", shadow_key));
            let id = kind.map(shadow_id);
            let picture = id.and_then(|id| with.looks.thumb(Set::Shadows, &id));
            let response = tile(ui, with.theme, (picture, false), name, (side, on));
            if kind.is_none() {
                // A line across: none.
                let square = Rect::from_min_size(response.rect.min, Vec2::splat(side));
                let c = square.center();
                let r = side * 0.18;
                let stroke = Stroke::new(2.0, with.theme.c("text.secondary"));
                ui.painter().circle_stroke(c, r, stroke);
                let d = Vec2::new(r, r) * std::f32::consts::FRAC_1_SQRT_2;
                ui.painter().line_segment([c - d, c + d], stroke);
            }
            if response.clicked() {
                // Its colour stays.
                asks.edit(image, |look| {
                    look.shadow = kind.map(|kind| ImageShadow {
                        color: now.map_or(ImageShadow::new(kind).color, |s| s.color),
                        ..ImageShadow::new(kind)
                    });
                });
            }
        }
    });
}

fn shadows(ui: &mut Ui, with: &mut With<'_>, image: &ImageNode, asks: &mut Asks) {
    let theme = with.theme;
    with.looks
        .want_thumbs(ui.ctx(), with.store, Set::Shadows, image);
    let all: Vec<Option<ShadowKind>> = std::iter::once(None)
        .chain(SHADOWS.into_iter().map(Some))
        .collect();
    for row in all.chunks(3) {
        shadow_row(ui, with, image, row, asks);
        ui.add_space(6.0);
    }
    let Some(now) = image.look.as_ref().and_then(|look| look.shadow) else {
        return;
    };
    let rest = ImageShadow::new(now.kind);
    let mut next = now;
    heading(ui, theme, with.t("image.shadow_settings"));
    ui.horizontal(|ui| {
        ui.label(RichText::new(with.t("image.shadow_color")).font(theme.font("body-sm")));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let mut rgba = canvas::picker_rgba(Some(next.color));
            if crate::eyedropper::color_edit(ui, &mut rgba).changed() {
                next.color = canvas::picked_color(Some(next.color), rgba);
            }
        });
    });
    ui.add_space(6.0);
    let share = |ui: &mut Ui, key, value: &mut f64, rest: f64, most: f64| {
        let mut shown = (*value / most * 100.0).round();
        let what = (&mut shown, UP, (rest / most * 100.0).round());
        if slider(ui, theme, with.t(key), what, Track::Plain) {
            *value = shown / 100.0 * most;
        }
    };
    let most = ImageShadow::MAX_LENGTH;
    share(
        ui,
        "image.shadow_opacity",
        &mut next.opacity,
        rest.opacity,
        1.0,
    );
    share(ui, "image.shadow_blur", &mut next.blur, rest.blur, most);
    let (far, large, turned) = match now.kind {
        ShadowKind::Glow | ShadowKind::Outline => (false, true, false),
        ShadowKind::Drop | ShadowKind::Curved => (true, false, true),
        ShadowKind::Floating | ShadowKind::Angled | ShadowKind::Ground => (true, true, true),
    };
    if far {
        share(
            ui,
            "image.shadow_distance",
            &mut next.distance,
            rest.distance,
            most,
        );
    }
    if large {
        share(ui, "image.shadow_size", &mut next.size, rest.size, most);
    }
    if turned {
        let what = (&mut next.angle_deg, TURN, rest.angle_deg);
        slider(ui, theme, with.t("image.shadow_angle"), what, Track::Plain);
    }
    if next != now {
        asks.edit(image, |look| look.shadow = Some(next));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gafira_doc::ContentHash;

    fn asset(n: u8) -> AssetRef {
        AssetRef::new(ContentHash([n; 32]))
    }

    // S85-06: each control moves a number of its own, within its range, and
    // changes nothing at rest.
    #[test]
    fn s85_06_each_control_is_its_own() {
        let all = || {
            WHITE_BALANCE
                .iter()
                .chain(&LIGHT)
                .chain(&COLOR)
                .chain(&TEXTURE)
        };
        let mut seen: Vec<ImageAdjust> = Vec::new();
        for control in all() {
            for value in [control.range.0, control.range.1] {
                let mut adjust = ImageAdjust::default();
                *(control.field)(&mut adjust) = value;
                let look = ImageLook {
                    adjust: adjust.clone(),
                    ..ImageLook::default()
                };
                assert!(look.is_valid(), "{}", control.key);
                assert!(value == 0.0 || !seen.contains(&adjust), "{}", control.key);
                seen.push(adjust);
            }
        }
        assert_eq!(all().count(), 14);
    }

    // S85-06: what changes is of the zone chosen, and what is asked together
    // is kept together; nothing changed asks for nothing.
    #[test]
    fn s85_06_the_zone_takes_the_change() {
        let (look, at) = with_zone(&ImageLook::default(), ZoneKind::Area, asset(5)).unwrap();
        let image = ImageNode::new(asset(1)).looking(look);
        let mut asks = Asks::default();
        asks.edit(&image, |_| ());
        assert!(asks.image.is_none());
        asks.edit(&image, |look| {
            adjust_mut(look, Some(at)).unwrap().contrast = 20.0;
        });
        asks.edit(&image, |look| {
            look.shadow = Some(ImageShadow::new(ShadowKind::Drop))
        });
        let next = asks.image.expect("a change").look.expect("a look");
        assert!(next.adjust.is_plain() && next.shadow.is_some());
        assert_eq!(next.zones[0].adjust.contrast, 20.0);
        assert!(adjust_of(&next, Some(3)).is_none());
    }

    // S85-03, S85-04: every filter and shadow has a name of its own.
    #[test]
    fn s85_03_names() {
        let mut names: Vec<&str> = FILTER_GROUPS
            .iter()
            .flat_map(|g| g.filters)
            .map(filter_key)
            .chain(FILTER_GROUPS.iter().map(|g| group_key(g.id)))
            .chain(SHADOWS.into_iter().map(shadow_key))
            .collect();
        let count = names.len();
        names.sort_unstable();
        names.dedup();
        assert_eq!((names.len(), count), (49, 49));
    }

    // S85-05: a mask keeps the shape of the image, and no side is lost.
    #[test]
    fn s85_05_the_size_of_a_mask() {
        assert_eq!(mask_size((4000, 2000)), (1024, 512));
        assert_eq!(mask_size((300, 200)), (300, 200));
        assert_eq!(mask_size((90_000, 2)), (1024, 1));
    }
}
