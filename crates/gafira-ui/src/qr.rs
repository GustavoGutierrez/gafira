//! «Códigos QR» (spec 080, FD-20): the panel of Elementos, the fields it
//! shares with Propiedades, and the pixels of a code on the canvas.

use std::collections::HashMap;
use std::hash::{Hash, Hasher};

use egui::{ColorImage, Pos2, Rect, Response, RichText, Sense, Stroke, Ui, Vec2};
use gafira_doc::{
    GradientKind, NodeKind, Placement, QrBody, QrCoin, QrContact, QrContactFormat, QrContent,
    QrEcc, QrEyeBall, QrEyeFrame, QrFill, QrLogo, QrNode, QrWifiSecurity,
};
use gafira_edit::NodeSpec;
use gafira_qr::{ContentError, Field, QrError, Warning};

use crate::canvas;
use crate::colors;
use crate::style::Theme;
use crate::text::Texts;
use crate::widgets;

fn plain(texts: &Texts, key: &'static str) -> String {
    texts.t(&gafira_doc::msg::Message {
        key: gafira_doc::msg::MessageKey(key),
        args: Vec::new(),
    })
}

/// A new code takes this share of the page's shorter side.
pub(crate) const SHARE: f64 = 0.25;
/// The longest side of the texture of a code, in px.
const TEXTURE_MAX: f64 = 2048.0;
/// The side of the preview of the panel, in logical px.
const PREVIEW: f32 = 200.0;
/// The side of a tile of a shape, in logical px.
const TILE: f32 = 40.0;

/// What a code holds (S80-03).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Kind {
    Url,
    Text,
    Email,
    Phone,
    Sms,
    Wifi,
    Contact,
    Event,
    Location,
    Social,
    Crypto,
}

impl Kind {
    pub(crate) const ALL: [Self; 11] = [
        Self::Url,
        Self::Text,
        Self::Email,
        Self::Phone,
        Self::Sms,
        Self::Wifi,
        Self::Contact,
        Self::Event,
        Self::Location,
        Self::Social,
        Self::Crypto,
    ];

    pub(crate) fn key(self) -> &'static str {
        match self {
            Self::Url => "qr.kind_url",
            Self::Text => "qr.kind_text",
            Self::Email => "qr.kind_email",
            Self::Phone => "qr.kind_phone",
            Self::Sms => "qr.kind_sms",
            Self::Wifi => "qr.kind_wifi",
            Self::Contact => "qr.kind_contact",
            Self::Event => "qr.kind_event",
            Self::Location => "qr.kind_location",
            Self::Social => "qr.kind_social",
            Self::Crypto => "qr.kind_crypto",
        }
    }

    pub(crate) fn of(content: &QrContent) -> Self {
        match content {
            QrContent::Url { .. } => Self::Url,
            QrContent::Text { .. } => Self::Text,
            QrContent::Email { .. } => Self::Email,
            QrContent::Phone { .. } => Self::Phone,
            QrContent::Sms { .. } => Self::Sms,
            QrContent::Wifi { .. } => Self::Wifi,
            QrContent::Contact(_) => Self::Contact,
            QrContent::Event { .. } => Self::Event,
            QrContent::Location { .. } => Self::Location,
            QrContent::Social { .. } => Self::Social,
            QrContent::Crypto { .. } => Self::Crypto,
        }
    }

    /// An empty content of this kind.
    pub(crate) fn empty(self) -> QrContent {
        let none = String::new;
        match self {
            Self::Url => QrContent::Url { url: none() },
            Self::Text => QrContent::Text { text: none() },
            Self::Email => QrContent::Email {
                address: none(),
                subject: none(),
                body: none(),
            },
            Self::Phone => QrContent::Phone { number: none() },
            Self::Sms => QrContent::Sms {
                number: none(),
                text: none(),
            },
            Self::Wifi => QrContent::Wifi {
                ssid: none(),
                password: none(),
                security: QrWifiSecurity::Wpa,
                hidden: false,
            },
            Self::Contact => QrContent::Contact(Box::default()),
            Self::Event => QrContent::Event {
                title: none(),
                place: none(),
                start: none(),
                end: none(),
            },
            Self::Location => QrContent::Location {
                latitude: none(),
                longitude: none(),
            },
            Self::Social => QrContent::Social { url: none() },
            Self::Crypto => QrContent::Crypto {
                coin: QrCoin::Bitcoin,
                address: none(),
                amount: none(),
            },
        }
    }
}

/// The code the panel starts with.
pub(crate) fn fresh() -> QrNode {
    QrNode::new(Kind::Url.empty(), colors::hex(0), colors::hex(0x00ff_ffff))
}

/// A new code on a page of `page_px`, centred on `at` or in its middle.
pub(crate) fn spec(qr: QrNode, page_px: (f64, f64), at: Option<(f64, f64)>) -> NodeSpec {
    let side = page_px.0.min(page_px.1) * SHARE;
    let (x, y) = at.unwrap_or((page_px.0 / 2.0, page_px.1 / 2.0));
    NodeSpec::new(
        NodeKind::Qr(Box::new(qr)),
        Placement::new(x - side / 2.0, y - side / 2.0, side, side),
    )
}

/// The group «Convertir en formas» puts in the place of a code (S80-13).
pub(crate) fn as_shapes(qr: &QrNode, placement: &Placement) -> Option<NodeSpec> {
    let parts = gafira_qr::parts(qr, placement).ok()?;
    let group = parts.first()?.placement;
    let children = parts
        .into_iter()
        .map(|part| NodeSpec::new(part.kind, part.placement))
        .collect();
    Some(
        NodeSpec::new(
            NodeKind::Group(gafira_doc::GroupNode {}),
            Placement::new(group.x, group.y, group.w, group.h),
        )
        .with_children(children),
    )
}

// ---- Pixels --------------------------------------------------------------

/// The key of the pixels of `qr`: what it says and how it looks.
pub(crate) fn key(qr: &QrNode, rest: impl Hash) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    format!("{qr:?}").hash(&mut hasher);
    rest.hash(&mut hasher);
    hasher.finish()
}

fn solid(color: gafira_color::Color) -> tiny_skia::Paint<'static> {
    let [r, g, b, a] = canvas::picker_rgba(Some(color));
    let mut paint = tiny_skia::Paint {
        anti_alias: true,
        ..tiny_skia::Paint::default()
    };
    paint.set_color_rgba8(r, g, b, a);
    paint
}

/// The pixels of `qr` in a box of `w` × `h` px at `zoom`: the same drawing
/// as the export. `logo` holds the decoded pixels of its image.
pub(crate) fn image(
    qr: &QrNode,
    (w, h): (f64, f64),
    zoom: f32,
    logo: Option<&ColorImage>,
) -> Option<ColorImage> {
    let drawing = gafira_qr::draw(qr).ok()?;
    let scale = f64::from(zoom).min(TEXTURE_MAX / w.max(h));
    if !(scale.is_finite() && scale > 0.0) {
        return None;
    }
    // Within 1..=2048: the casts are exact.
    let (pw, ph) = (
        (w * scale).round().max(1.0) as u32,
        (h * scale).round().max(1.0) as u32,
    );
    let mut pixmap = tiny_skia::Pixmap::new(pw, ph)?;
    let (k, (x, y)) = drawing.fit(f64::from(pw), f64::from(ph))?;
    let at = tiny_skia::Transform::from_translate(x as f32, y as f32);
    let side = drawing.size * k;
    if let Some(color) = drawing.background {
        let margin = drawing.margin * k;
        let whole = (side + 2.0 * margin) as f32;
        let back = tiny_skia::Rect::from_xywh(-margin as f32, -margin as f32, whole, whole)?;
        pixmap.fill_rect(back, &solid(color), at, None);
    }
    for layer in &drawing.layers {
        let Some(path) = canvas::skia_path(&(kurbo::Affine::scale(k) * &layer.path)) else {
            continue;
        };
        let paint = match &layer.fill {
            QrFill::Solid(color) => solid(*color),
            QrFill::Gradient(gradient) => tiny_skia::Paint {
                shader: canvas::gradient_shader(gradient, (side, side), (0.0, 1.0))?,
                anti_alias: true,
                ..tiny_skia::Paint::default()
            },
        };
        pixmap.fill_path(&path, &paint, tiny_skia::FillRule::Winding, at, None);
    }
    if let (Some(place), Some(photo)) = (drawing.logo, logo) {
        draw_logo(&mut pixmap, photo, place.rect, k, (x, y));
    }
    Some(ColorImage::from_rgba_premultiplied(
        [pw as usize, ph as usize],
        pixmap.data(),
    ))
}

fn draw_logo(
    pixmap: &mut tiny_skia::Pixmap,
    photo: &ColorImage,
    rect: kurbo::Rect,
    k: f64,
    (x, y): (f64, f64),
) -> Option<()> {
    let [iw, ih] = photo.size;
    let bytes: Vec<u8> = photo.pixels.iter().flat_map(|p| p.to_array()).collect();
    let size = tiny_skia::IntSize::from_wh(u32::try_from(iw).ok()?, u32::try_from(ih).ok()?)?;
    let source = tiny_skia::Pixmap::from_vec(bytes, size)?;
    let (left, top) = (x + rect.x0 * k, y + rect.y0 * k);
    let (w, h) = (rect.width() * k, rect.height() * k);
    let shader = tiny_skia::Pattern::new(
        source.as_ref(),
        tiny_skia::SpreadMode::Pad,
        tiny_skia::FilterQuality::Bilinear,
        1.0,
        tiny_skia::Transform::from_row(
            (w / iw as f64) as f32,
            0.0,
            0.0,
            (h / ih as f64) as f32,
            left as f32,
            top as f32,
        ),
    );
    let paint = tiny_skia::Paint {
        shader,
        anti_alias: true,
        ..tiny_skia::Paint::default()
    };
    let area = tiny_skia::Rect::from_xywh(left as f32, top as f32, w as f32, h as f32)?;
    pixmap.fill_rect(area, &paint, tiny_skia::Transform::identity(), None);
    Some(())
}

/// What a tile shows, in white: the part of `path` within `region`, in
/// modules.
fn tile_image(path: &kurbo::BezPath, region: kurbo::Rect) -> Option<ColorImage> {
    let px = (TILE * 2.0) as u32;
    let mut pixmap = tiny_skia::Pixmap::new(px, px)?;
    let k = f64::from(px) / region.width();
    let at = kurbo::Affine::scale(k) * kurbo::Affine::translate((-region.x0, -region.y0));
    pixmap.fill_path(
        &canvas::skia_path(&(at * path))?,
        &solid(colors::hex(0x00ff_ffff)),
        tiny_skia::FillRule::Winding,
        tiny_skia::Transform::identity(),
        None,
    );
    Some(ColorImage::from_rgba_premultiplied(
        [px as usize, px as usize],
        pixmap.data(),
    ))
}

// ---- The styles of the panel (S80-14) ------------------------------------

/// A style of the panel: its shapes and colours.
struct Preset {
    key: &'static str,
    body: QrBody,
    frame: QrEyeFrame,
    ball: QrEyeBall,
    dark: u32,
    /// The other end of a gradient.
    end: Option<u32>,
    eyes: Option<u32>,
    light: u32,
}

const PRESETS: [Preset; 8] = {
    const fn preset(
        key: &'static str,
        (body, frame, ball): (QrBody, QrEyeFrame, QrEyeBall),
        (dark, end, eyes, light): (u32, Option<u32>, Option<u32>, u32),
    ) -> Preset {
        Preset {
            key,
            body,
            frame,
            ball,
            dark,
            end,
            eyes,
            light,
        }
    }
    use QrBody as B;
    use QrEyeBall as C;
    use QrEyeFrame as F;
    [
        preset(
            "qr.style_classic",
            (B::Square, F::Square, C::Square),
            (0x0000_0000, None, None, 0x00ff_ffff),
        ),
        preset(
            "qr.style_soft",
            (B::Fluid, F::Rounded, C::Rounded),
            (0x0018_243b, None, None, 0x00ff_ffff),
        ),
        preset(
            "qr.style_dots",
            (B::Dot, F::Circle, C::Circle),
            (0x0018_243b, None, Some(0x00c0_3a2b), 0x00ff_ffff),
        ),
        preset(
            "qr.style_night",
            (B::FluidCorners, F::Drop, C::Circle),
            (0x0018_243b, Some(0x004a_2c82), None, 0x00ff_ffff),
        ),
        preset(
            "qr.style_forest",
            (B::Leaf, F::Leaf, C::Leaf),
            (0x0014_532d, Some(0x0016_6534), None, 0x00f3_faf4),
        ),
        preset(
            "qr.style_bars",
            (B::BarsVertical, F::Rounded, C::Bars),
            (0x001e_3a8a, None, Some(0x000f_172a), 0x00ff_ffff),
        ),
        preset(
            "qr.style_gem",
            (B::Diamond, F::Octagon, C::Diamond),
            (0x007c_1d4d, None, None, 0x00ff_f7fb),
        ),
        preset(
            "qr.style_garden",
            (B::SmallDot, F::Dotted, C::Flower),
            (0x0036_2f78, None, Some(0x009d_174d), 0x00ff_ffff),
        ),
    ]
};

impl Preset {
    /// `qr` with this style; what it says, its logo and its correction stay.
    fn on(&self, qr: &QrNode) -> QrNode {
        let mut out = qr.clone();
        out.style.body = self.body;
        out.style.eye_frame = self.frame;
        out.style.eye_ball = self.ball;
        out.style.fill = match self.end {
            Some(end) => QrFill::Gradient(colors::gradient(self.dark, end, 90.0)),
            None => QrFill::Solid(colors::hex(self.dark)),
        };
        let eyes = self.eyes.map(|c| QrFill::Solid(colors::hex(c)));
        out.style.eye_frame_fill = None;
        out.style.eye_ball_fill = eyes;
        out.style.background = Some(colors::hex(self.light));
        out
    }
}

// ---- The fields ----------------------------------------------------------

/// What the fields ask for.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Outcome {
    pub(crate) changed: bool,
    pub(crate) pick_logo: bool,
}

/// The textures the fields draw from.
#[derive(Default)]
pub(crate) struct Tiles {
    cache: HashMap<String, egui::TextureHandle>,
}

impl Tiles {
    fn texture(
        &mut self,
        ctx: &egui::Context,
        name: String,
        make: impl FnOnce() -> Option<ColorImage>,
    ) -> Option<egui::TextureId> {
        if let Some(texture) = self.cache.get(&name) {
            return Some(texture.id());
        }
        let texture = ctx.load_texture(format!("qr-{name}"), make()?, egui::TextureOptions::LINEAR);
        let id = texture.id();
        self.cache.insert(name, texture);
        Some(id)
    }
}

fn caption(ui: &mut Ui, theme: &Theme, text: String) {
    ui.label(
        RichText::new(text)
            .font(theme.font("body-sm"))
            .color(theme.c("text.secondary")),
    );
}

/// The space between a field and the next one.
const ROW_GAP: f32 = 12.0;
/// The space between a field and its label or message.
const LABEL_GAP: f32 = 4.0;
/// The height of a control that sits beside its label.
const ROW_HEIGHT: f32 = 28.0;

/// The fields of what a code says, with what is wrong with them now
/// (S80-17).
struct Form<'a> {
    theme: &'a Theme,
    texts: &'a Texts,
    problems: Vec<ContentError>,
    changed: bool,
}

/// What a field asks for.
#[derive(Clone, Copy)]
struct Ask {
    key: &'static str,
    /// What can be wrong with it.
    field: Option<Field>,
    required: bool,
    /// The key of an example, shown while it is empty; empty for none.
    hint: &'static str,
}

impl Ask {
    const fn plain(key: &'static str) -> Self {
        Self {
            key,
            field: None,
            required: false,
            hint: "",
        }
    }

    const fn checked(key: &'static str, field: Field, hint: &'static str) -> Self {
        Self {
            key,
            field: Some(field),
            required: false,
            hint,
        }
    }

    const fn required(key: &'static str, field: Field, hint: &'static str) -> Self {
        Self {
            key,
            field: Some(field),
            required: true,
            hint,
        }
    }
}

impl Form<'_> {
    /// The problem to tell about `ask`: a wrong value as it is written, a
    /// missing one once the field was used, so an untouched form does not
    /// scold.
    fn problem(&self, ask: &Ask, used: bool) -> Option<ContentError> {
        let field = ask.field?;
        self.problems.iter().copied().find(|problem| match problem {
            ContentError::Invalid(of) => *of == field,
            ContentError::Empty(of) => *of == field && used,
        })
    }

    /// A field under its label, the width of its column, its message below.
    fn text(&mut self, ui: &mut Ui, ask: Ask, text: &mut String, rows: usize) {
        let theme = self.theme;
        let name = plain(self.texts, ask.key);
        let id = ui.make_persistent_id(ask.key);
        let used_id = id.with("used");
        let used = ui.data(|d| d.get_temp::<bool>(used_id)).unwrap_or(false);
        let problem = self.problem(&ask, used);
        let hint = if ask.hint.is_empty() {
            String::new()
        } else {
            plain(self.texts, ask.hint)
        };
        // An example never looks like something written.
        let hint = RichText::new(hint).color(theme.c("text.muted"));
        let label = ui
            .horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 2.0;
                let label = ui.label(
                    RichText::new(&name)
                        .font(theme.font("body-sm"))
                        .color(theme.c("text.secondary")),
                );
                if ask.required {
                    ui.label(
                        RichText::new("*")
                            .font(theme.font("body-sm"))
                            .color(theme.c("error.fg")),
                    );
                }
                label
            })
            .inner;
        ui.add_space(LABEL_GAP);
        let response = ui
            .scope(|ui| {
                if problem.is_some() {
                    let stroke = Stroke::new(1.5, theme.c("error.border"));
                    let visuals = ui.visuals_mut();
                    visuals.selection.stroke = stroke;
                    for widget in [
                        &mut visuals.widgets.inactive,
                        &mut visuals.widgets.hovered,
                        &mut visuals.widgets.active,
                        &mut visuals.widgets.open,
                        &mut visuals.widgets.noninteractive,
                    ] {
                        widget.bg_stroke = stroke;
                    }
                }
                let edit = if rows > 1 {
                    egui::TextEdit::multiline(text)
                        .margin(Vec2::new(10.0, 8.0))
                        .desired_rows(rows)
                        .desired_width(f32::INFINITY)
                        .hint_text(hint)
                } else {
                    widgets::field(text, String::new()).hint_text(hint)
                };
                ui.add(edit.id(id))
            })
            .inner
            .labelled_by(label.id);
        self.changed |= response.changed();
        if !used && (response.changed() || response.lost_focus()) {
            ui.data_mut(|d| d.insert_temp(used_id, true));
            // The message shows on the next pass, at once.
            ui.ctx().request_repaint();
        }
        if let Some(problem) = problem {
            ui.add_space(LABEL_GAP);
            message(ui, theme, problem_text(self.texts, problem));
        }
        ui.add_space(ROW_GAP);
    }

    fn line(&mut self, ui: &mut Ui, ask: Ask, text: &mut String) {
        self.text(ui, ask, text, 1);
    }

    fn block(&mut self, ui: &mut Ui, ask: Ask, text: &mut String) {
        self.text(ui, ask, text, 3);
    }

    /// Two short fields side by side, in columns of the same width.
    fn pair(
        &mut self,
        ui: &mut Ui,
        (left, left_text): (Ask, &mut String),
        (right, right_text): (Ask, &mut String),
    ) {
        ui.columns(2, |columns| {
            self.line(&mut columns[0], left, left_text);
            self.line(&mut columns[1], right, right_text);
        });
    }

    /// A list under its label.
    fn choice<T: Copy + PartialEq>(
        &mut self,
        ui: &mut Ui,
        key: &'static str,
        value: &mut T,
        options: &[(T, &'static str)],
    ) {
        caption(ui, self.theme, plain(self.texts, key));
        ui.add_space(LABEL_GAP);
        self.changed |= choice(ui, self.texts, key, value, options);
        ui.add_space(ROW_GAP);
    }
}

/// A message under a field: a mark and its words, so it does not depend on
/// the colour alone (S80-15).
fn message(ui: &mut Ui, theme: &Theme, text: String) {
    ui.horizontal_top(|ui| {
        ui.spacing_mut().item_spacing.x = 6.0;
        let (rect, _) = ui.allocate_exact_size(Vec2::new(14.0, 18.0), Sense::hover());
        let centre = rect.center();
        ui.painter().circle_filled(centre, 7.0, theme.c("error.fg"));
        let mark = Stroke::new(1.5, theme.c("surface"));
        ui.painter().line_segment(
            [centre - Vec2::new(0.0, 3.5), centre + Vec2::new(0.0, 0.5)],
            mark,
        );
        ui.painter()
            .circle_filled(centre + Vec2::new(0.0, 3.5), 1.0, theme.c("surface"));
        ui.add(
            egui::Label::new(
                RichText::new(text)
                    .font(theme.font("caption"))
                    .color(theme.c("error.fg")),
            )
            .wrap(),
        );
    });
}

/// A warning about the whole code: its words over a tinted box.
fn message_box(ui: &mut Ui, theme: &Theme, text: String) {
    egui::Frame::NONE
        .fill(theme.c("warning.bg"))
        .corner_radius(theme.radius_md())
        .inner_margin(egui::Margin::same(10))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.add(
                egui::Label::new(
                    RichText::new(text)
                        .font(theme.font("body-sm"))
                        .color(theme.c("warning.fg")),
                )
                .wrap(),
            );
        });
    ui.add_space(8.0);
}

/// A label on the left and its control at the right edge, every row of the
/// same height, so the controls of a section line up.
fn row(ui: &mut Ui, theme: &Theme, label: String, add: impl FnOnce(&mut Ui)) {
    let size = Vec2::new(ui.available_width(), ROW_HEIGHT);
    ui.allocate_ui_with_layout(
        size,
        egui::Layout::right_to_left(egui::Align::Center),
        |ui| {
            ui.set_min_size(size);
            add(ui);
            ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                ui.add(
                    egui::Label::new(
                        RichText::new(label)
                            .font(theme.font("body-sm"))
                            .color(theme.c("text.secondary")),
                    )
                    .truncate(),
                );
            });
        },
    );
}

/// A switch at the right edge of its row.
fn switch(ui: &mut Ui, theme: &Theme, label: String, on: &mut bool) -> bool {
    let mut changed = false;
    row(ui, theme, label, |ui| {
        changed = widgets::toggle(ui, theme, on).changed();
    });
    changed
}

/// A list of one choice, the width of its column.
fn choice<T: Copy + PartialEq>(
    ui: &mut Ui,
    texts: &Texts,
    id: &str,
    value: &mut T,
    options: &[(T, &'static str)],
) -> bool {
    let before = *value;
    let shown = options
        .iter()
        .find(|(option, _)| option == value)
        .map_or_else(String::new, |(_, key)| plain(texts, key));
    egui::ComboBox::from_id_salt(id)
        .width(ui.available_width())
        .selected_text(shown)
        .show_ui(ui, |ui| {
            for (option, key) in options {
                ui.selectable_value(value, *option, plain(texts, key));
            }
        });
    *value != before
}

/// The label of a field, for the summary of the form.
fn field_key(field: Field) -> &'static str {
    match field {
        Field::Url => "qr.url",
        Field::Text => "qr.text",
        Field::Email => "qr.email",
        Field::Phone => "qr.phone",
        Field::WorkPhone => "qr.work_phone",
        Field::Ssid => "qr.ssid",
        Field::Password => "qr.password",
        Field::Name => "qr.first_name",
        Field::Title => "qr.title",
        Field::Start => "qr.start",
        Field::End => "qr.end",
        Field::Latitude => "qr.latitude",
        Field::Longitude => "qr.longitude",
        Field::Address => "qr.address",
        Field::Amount => "qr.amount",
    }
}

/// What a problem of a field says under it, with how to mend it.
fn problem_key(problem: ContentError) -> &'static str {
    match problem {
        ContentError::Empty(Field::Name) => "qr.need_name",
        ContentError::Empty(_) => "qr.need",
        ContentError::Invalid(field) => match field {
            Field::Url => "qr.bad_url",
            Field::Email => "qr.bad_email",
            Field::Phone | Field::WorkPhone => "qr.bad_phone",
            Field::Password => "qr.bad_password",
            Field::Start => "qr.bad_moment",
            Field::End => "qr.bad_end",
            Field::Latitude => "qr.bad_latitude",
            Field::Longitude => "qr.bad_longitude",
            Field::Address => "qr.bad_address",
            Field::Amount => "qr.bad_amount",
            Field::Text | Field::Ssid | Field::Name | Field::Title => "qr.need",
        },
    }
}

fn problem_text(texts: &Texts, problem: ContentError) -> String {
    plain(texts, problem_key(problem))
}

/// Why «Añadir al diseño» waits: the fields still to fill in or to mend, or
/// the length of the content.
fn waiting_text(texts: &Texts, qr: &QrNode) -> Option<String> {
    let problems = gafira_qr::problems(&qr.content);
    if problems.is_empty() {
        return match gafira_qr::draw(qr) {
            Err(QrError::TooLong) => Some(plain(texts, "qr.too_long")),
            _ => None,
        };
    }
    let mut names: Vec<String> = Vec::new();
    for problem in problems {
        let (ContentError::Empty(field) | ContentError::Invalid(field)) = problem;
        let name = plain(texts, field_key(field));
        if !names.contains(&name) {
            names.push(name);
        }
    }
    Some(format!(
        "{}: {}",
        plain(texts, "qr.waiting"),
        names.join(", ")
    ))
}

fn content_fields(ui: &mut Ui, theme: &Theme, texts: &Texts, content: &mut QrContent) -> bool {
    let mut form = Form {
        theme,
        texts,
        problems: gafira_qr::problems(content),
        changed: false,
    };
    let mut kind = Kind::of(content);
    let kinds = Kind::ALL.map(|k| (k, k.key()));
    form.choice(ui, "qr.kind", &mut kind, &kinds);
    if form.changed {
        *content = kind.empty();
        form.problems = gafira_qr::problems(content);
    }
    // Each kind keeps its own «used» marks.
    ui.push_id(kind.key(), |ui| {
        // The labels are not controls: a click goes to the fields.
        ui.style_mut().interaction.selectable_labels = false;
        form_of(ui, &mut form, content);
    });
    form.changed
}

fn form_of(ui: &mut Ui, form: &mut Form<'_>, content: &mut QrContent) {
    const URL: &str = "qr.hint_url";
    const EMAIL: &str = "qr.hint_email";
    const PHONE: &str = "qr.hint_phone";
    const MOMENT: &str = "qr.hint_moment";
    match content {
        QrContent::Url { url } | QrContent::Social { url } => {
            form.line(ui, Ask::required("qr.url", Field::Url, URL), url);
        }
        QrContent::Text { text } => {
            form.block(ui, Ask::required("qr.text", Field::Text, ""), text);
        }
        QrContent::Email {
            address,
            subject,
            body,
        } => {
            form.line(ui, Ask::required("qr.email", Field::Email, EMAIL), address);
            form.line(ui, Ask::plain("qr.subject"), subject);
            form.block(ui, Ask::plain("qr.message"), body);
        }
        QrContent::Phone { number } => {
            form.line(ui, Ask::required("qr.phone", Field::Phone, PHONE), number);
        }
        QrContent::Sms { number, text } => {
            form.line(ui, Ask::required("qr.phone", Field::Phone, PHONE), number);
            form.block(ui, Ask::plain("qr.message"), text);
        }
        QrContent::Wifi {
            ssid,
            password,
            security,
            hidden,
        } => {
            form.line(ui, Ask::required("qr.ssid", Field::Ssid, ""), ssid);
            form.choice(
                ui,
                "qr.security",
                security,
                &[
                    (QrWifiSecurity::Wpa, "qr.wpa"),
                    (QrWifiSecurity::Wep, "qr.wep"),
                    (QrWifiSecurity::Open, "qr.open"),
                ],
            );
            if *security != QrWifiSecurity::Open {
                let ask = Ask::required("qr.password", Field::Password, "");
                form.line(ui, ask, password);
                caption(ui, form.theme, plain(form.texts, "qr.password_note"));
                ui.add_space(ROW_GAP);
            }
            let name = plain(form.texts, "qr.hidden");
            form.changed |= switch(ui, form.theme, name, hidden);
        }
        QrContent::Contact(c) => contact_form(ui, form, c),
        QrContent::Event {
            title,
            place,
            start,
            end,
        } => {
            form.line(ui, Ask::required("qr.title", Field::Title, ""), title);
            form.line(ui, Ask::plain("qr.place"), place);
            form.line(ui, Ask::required("qr.start", Field::Start, MOMENT), start);
            form.line(ui, Ask::required("qr.end", Field::End, MOMENT), end);
        }
        QrContent::Location {
            latitude,
            longitude,
        } => {
            form.pair(
                ui,
                (
                    Ask::required("qr.latitude", Field::Latitude, "qr.hint_latitude"),
                    latitude,
                ),
                (
                    Ask::required("qr.longitude", Field::Longitude, "qr.hint_longitude"),
                    longitude,
                ),
            );
        }
        QrContent::Crypto {
            coin,
            address,
            amount,
        } => {
            form.choice(
                ui,
                "qr.coin",
                coin,
                &[
                    (QrCoin::Bitcoin, "qr.bitcoin"),
                    (QrCoin::BitcoinCash, "qr.bitcoin_cash"),
                    (QrCoin::Ethereum, "qr.ethereum"),
                    (QrCoin::Litecoin, "qr.litecoin"),
                    (QrCoin::Dash, "qr.dash"),
                ],
            );
            form.line(ui, Ask::required("qr.address", Field::Address, ""), address);
            if *coin != QrCoin::Ethereum {
                form.line(
                    ui,
                    Ask::checked("qr.amount", Field::Amount, "qr.hint_amount"),
                    amount,
                );
            }
        }
    }
}

fn contact_form(ui: &mut Ui, form: &mut Form<'_>, c: &mut QrContact) {
    form.choice(
        ui,
        "qr.format",
        &mut c.format,
        &[
            (QrContactFormat::Vcard, "qr.vcard"),
            (QrContactFormat::Mecard, "qr.mecard"),
        ],
    );
    form.pair(
        ui,
        (
            Ask::required("qr.first_name", Field::Name, ""),
            &mut c.first_name,
        ),
        (Ask::plain("qr.last_name"), &mut c.last_name),
    );
    form.line(ui, Ask::plain("qr.organization"), &mut c.organization);
    if c.format == QrContactFormat::Vcard {
        form.line(ui, Ask::plain("qr.job"), &mut c.job);
    }
    form.line(
        ui,
        Ask::checked("qr.phone", Field::Phone, "qr.hint_phone"),
        &mut c.phone,
    );
    form.line(
        ui,
        Ask::checked("qr.work_phone", Field::WorkPhone, "qr.hint_phone"),
        &mut c.work_phone,
    );
    form.line(
        ui,
        Ask::checked("qr.email", Field::Email, "qr.hint_email"),
        &mut c.email,
    );
    form.line(
        ui,
        Ask::checked("qr.url", Field::Url, "qr.hint_url"),
        &mut c.url,
    );
    form.line(ui, Ask::plain("qr.street"), &mut c.street);
    form.pair(
        ui,
        (Ask::plain("qr.city"), &mut c.city),
        (Ask::plain("qr.postcode"), &mut c.postcode),
    );
    form.pair(
        ui,
        (Ask::plain("qr.region"), &mut c.region),
        (Ask::plain("qr.country"), &mut c.country),
    );
}

/// A colour picker over `color`.
fn color_pick(ui: &mut Ui, color: &mut gafira_color::Color) -> bool {
    let mut rgba = canvas::picker_rgba(Some(*color));
    let changed = ui.color_edit_button_srgba_unmultiplied(&mut rgba).changed();
    if changed {
        *color = canvas::picked_color(Some(*color), rgba);
    }
    changed
}

/// The colour of the body: one, or a gradient of two or more.
fn fill_fields(ui: &mut Ui, theme: &Theme, texts: &Texts, fill: &mut QrFill) -> bool {
    let mut changed = false;
    let is_solid = matches!(fill, QrFill::Solid(_));
    row(ui, theme, plain(texts, "qr.front"), |ui| {
        if ui
            .selectable_label(!is_solid, plain(texts, "qr.gradient"))
            .clicked()
            && let QrFill::Solid(color) = *fill
        {
            *fill = QrFill::Gradient(gafira_doc::Gradient {
                kind: GradientKind::Linear { angle_deg: 90.0 },
                stops: vec![
                    gafira_doc::GradientStop { offset: 0.0, color },
                    gafira_doc::GradientStop {
                        offset: 1.0,
                        color: colors::hex(0x0001_5f96),
                    },
                ],
            });
            changed = true;
        }
        if ui
            .selectable_label(is_solid, plain(texts, "qr.solid"))
            .clicked()
            && let QrFill::Gradient(gradient) = &*fill
            && let Some(first) = gradient.stops.first()
        {
            *fill = QrFill::Solid(first.color);
            changed = true;
        }
    });
    match fill {
        QrFill::Solid(color) => {
            row(ui, theme, plain(texts, "qr.color"), |ui| {
                changed |= color_pick(ui, color);
            });
        }
        QrFill::Gradient(gradient) => {
            let last = gradient.stops.len().saturating_sub(1);
            for index in [0, last] {
                let key = if index == 0 { "qr.from" } else { "qr.to" };
                if let Some(stop) = gradient.stops.get_mut(index) {
                    row(ui, theme, plain(texts, key), |ui| {
                        ui.push_id(key, |ui| changed |= color_pick(ui, &mut stop.color));
                    });
                }
            }
            let radial = matches!(gradient.kind, GradientKind::Radial);
            row(ui, theme, plain(texts, "qr.direction"), |ui| {
                if ui
                    .selectable_label(radial, plain(texts, "qr.radial"))
                    .clicked()
                    && !radial
                {
                    gradient.kind = GradientKind::Radial;
                    changed = true;
                }
                if ui
                    .selectable_label(!radial, plain(texts, "qr.linear"))
                    .clicked()
                    && radial
                {
                    gradient.kind = GradientKind::Linear { angle_deg: 90.0 };
                    changed = true;
                }
            });
            if let GradientKind::Linear { angle_deg } = &mut gradient.kind {
                row(ui, theme, plain(texts, "qr.angle"), |ui| {
                    let field = egui::DragValue::new(angle_deg)
                        .range(0.0..=360.0)
                        .max_decimals(0)
                        .suffix("°");
                    changed |= ui.add(field).changed();
                });
            }
            row(ui, theme, String::new(), |ui| {
                if ui.button(plain(texts, "qr.swap")).clicked() {
                    let mut colors: Vec<_> = gradient.stops.iter().map(|s| s.color).collect();
                    colors.reverse();
                    for (stop, color) in gradient.stops.iter_mut().zip(colors) {
                        stop.color = color;
                    }
                    changed = true;
                }
            });
        }
    }
    changed
}

/// The colour of a part of the eyes, its own or the one of the body.
fn eye_fill(
    ui: &mut Ui,
    theme: &Theme,
    texts: &Texts,
    key: &'static str,
    (own, body): (&mut Option<QrFill>, &QrFill),
) -> bool {
    let mut changed = false;
    row(ui, theme, plain(texts, key), |ui| {
        let mut on = own.is_some();
        if widgets::toggle(ui, theme, &mut on).changed() {
            *own = on
                .then(|| body.colors().first().copied())
                .flatten()
                .map(QrFill::Solid);
            changed = true;
        }
        if let Some(QrFill::Solid(color)) = own {
            changed |= color_pick(ui, color);
        }
    });
    changed
}

fn color_fields(ui: &mut Ui, theme: &Theme, texts: &Texts, qr: &mut QrNode) -> bool {
    let mut changed = false;
    ui.push_id("qr.front", |ui| {
        changed |= fill_fields(ui, theme, texts, &mut qr.style.fill);
    });
    let body = qr.style.fill.clone();
    for (key, own) in [
        ("qr.own_frame", &mut qr.style.eye_frame_fill),
        ("qr.own_ball", &mut qr.style.eye_ball_fill),
    ] {
        ui.push_id(key, |ui| {
            changed |= eye_fill(ui, theme, texts, key, (own, &body));
        });
    }
    row(ui, theme, plain(texts, "qr.back"), |ui| {
        let mut on = qr.style.background.is_some();
        if widgets::toggle(ui, theme, &mut on).changed() {
            qr.style.background = on.then(|| colors::hex(0x00ff_ffff));
            changed = true;
        }
        if let Some(color) = &mut qr.style.background {
            ui.push_id("qr.back", |ui| changed |= color_pick(ui, color));
        }
    });
    changed
}

/// A tile that draws a shape; its click chooses it.
fn tile(
    ui: &mut Ui,
    theme: &Theme,
    texture: Option<egui::TextureId>,
    chosen: bool,
    name: String,
) -> Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(TILE), Sense::click());
    let fill = if response.hovered() {
        theme.c("secondary.bg")
    } else {
        theme.c("surface.sunken")
    };
    ui.painter().rect_filled(rect, theme.radius_md(), fill);
    if let Some(id) = texture {
        let uv = Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0));
        ui.painter()
            .image(id, rect.shrink(6.0), uv, theme.c("text"));
    }
    let (width, color) = if chosen || response.has_focus() {
        (2.0, theme.c("canvas.selection"))
    } else {
        (1.0, theme.c("border"))
    };
    ui.painter().rect_stroke(
        rect,
        theme.radius_md(),
        Stroke::new(width, color),
        egui::StrokeKind::Inside,
    );
    if response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    response.widget_info(|| {
        egui::WidgetInfo::selected(egui::WidgetType::RadioButton, true, chosen, &name)
    });
    response.on_hover_text(name)
}

/// A code in one shape, for its tile: `"GAFIRA"` in version 1.
fn sample() -> QrNode {
    let mut qr = fresh();
    qr.content = QrContent::Text {
        text: "GAFIRA".to_owned(),
    };
    qr
}

fn shape_fields(
    ui: &mut Ui,
    theme: &Theme,
    texts: &Texts,
    qr: &mut QrNode,
    tiles: &mut Tiles,
) -> bool {
    let mut changed = false;
    let ctx = ui.ctx().clone();
    let gap = Vec2::splat(6.0);
    caption(ui, theme, plain(texts, "qr.body"));
    ui.add_space(LABEL_GAP);
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = gap;
        for body in QrBody::ALL {
            let texture = tiles.texture(&ctx, format!("body-{body:?}"), || {
                tile_image(
                    &gafira_qr::swatch(body),
                    kurbo::Rect::new(0.0, 0.0, 5.0, 5.0),
                )
            });
            let name = plain(texts, body_key(body));
            if tile(ui, theme, texture, qr.style.body == body, name).clicked() {
                qr.style.body = body;
                changed = true;
            }
        }
    });
    ui.add_space(ROW_GAP);
    caption(ui, theme, plain(texts, "qr.eye_frame"));
    ui.add_space(LABEL_GAP);
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = gap;
        for frame in QrEyeFrame::ALL {
            let texture = tiles.texture(&ctx, format!("frame-{frame:?}"), || {
                let mut qr = sample();
                qr.style.eye_frame = frame;
                let drawing = gafira_qr::draw(&qr).ok()?;
                tile_image(
                    &drawing.layers.get(1)?.path,
                    kurbo::Rect::new(0.0, 0.0, 7.0, 7.0),
                )
            });
            let name = plain(texts, frame_key(frame));
            if tile(ui, theme, texture, qr.style.eye_frame == frame, name).clicked() {
                qr.style.eye_frame = frame;
                changed = true;
            }
        }
    });
    ui.add_space(ROW_GAP);
    caption(ui, theme, plain(texts, "qr.eye_ball"));
    ui.add_space(LABEL_GAP);
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = gap;
        for ball in QrEyeBall::ALL {
            let texture = tiles.texture(&ctx, format!("ball-{ball:?}"), || {
                let mut qr = sample();
                qr.style.eye_ball = ball;
                let drawing = gafira_qr::draw(&qr).ok()?;
                tile_image(
                    &drawing.layers.get(2)?.path,
                    kurbo::Rect::new(1.5, 1.5, 5.5, 5.5),
                )
            });
            let name = plain(texts, ball_key(ball));
            if tile(ui, theme, texture, qr.style.eye_ball == ball, name).clicked() {
                qr.style.eye_ball = ball;
                changed = true;
            }
        }
    });
    ui.add_space(6.0);
    changed |= switch(
        ui,
        theme,
        plain(texts, "qr.mirror"),
        &mut qr.style.mirror_eyes,
    );
    changed
}

fn body_key(body: QrBody) -> &'static str {
    match body {
        QrBody::Square => "qr.shape_square",
        QrBody::Separate => "qr.shape_separate",
        QrBody::Dot => "qr.shape_dot",
        QrBody::SmallDot => "qr.shape_small_dot",
        QrBody::Diamond => "qr.shape_diamond",
        QrBody::Star => "qr.shape_star",
        QrBody::Cross => "qr.shape_cross",
        QrBody::Fluid => "qr.shape_fluid",
        QrBody::FluidCorners => "qr.shape_fluid_corners",
        QrBody::BarsHorizontal => "qr.shape_bars_horizontal",
        QrBody::BarsVertical => "qr.shape_bars_vertical",
        QrBody::Bevel => "qr.shape_bevel",
        QrBody::Leaf => "qr.shape_leaf",
    }
}

fn frame_key(frame: QrEyeFrame) -> &'static str {
    match frame {
        QrEyeFrame::Square => "qr.shape_square",
        QrEyeFrame::Rounded => "qr.shape_rounded",
        QrEyeFrame::Circle => "qr.shape_circle",
        QrEyeFrame::Leaf => "qr.shape_leaf",
        QrEyeFrame::Drop => "qr.shape_drop",
        QrEyeFrame::Bevel => "qr.shape_bevel",
        QrEyeFrame::Dotted => "qr.shape_dotted",
        QrEyeFrame::Octagon => "qr.shape_octagon",
    }
}

fn ball_key(ball: QrEyeBall) -> &'static str {
    match ball {
        QrEyeBall::Square => "qr.shape_square",
        QrEyeBall::Rounded => "qr.shape_rounded",
        QrEyeBall::Circle => "qr.shape_circle",
        QrEyeBall::Leaf => "qr.shape_leaf",
        QrEyeBall::Diamond => "qr.shape_diamond",
        QrEyeBall::Bars => "qr.shape_bars_vertical",
        QrEyeBall::Cushion => "qr.shape_cushion",
        QrEyeBall::Flower => "qr.shape_flower",
    }
}

fn logo_fields(ui: &mut Ui, theme: &Theme, texts: &Texts, qr: &mut QrNode, out: &mut Outcome) {
    ui.horizontal(|ui| {
        let key = if qr.logo.is_some() {
            "qr.logo_replace"
        } else {
            "qr.logo_add"
        };
        out.pick_logo |= ui.button(plain(texts, key)).clicked();
        if qr.logo.is_some() && ui.button(plain(texts, "qr.logo_remove")).clicked() {
            qr.logo = None;
            out.changed = true;
        }
    });
    if let Some(logo) = &mut qr.logo {
        ui.add_space(ROW_GAP);
        let mut percent = logo.share * 100.0;
        let range = QrLogo::MIN_SHARE * 100.0..=QrLogo::MAX_SHARE * 100.0;
        if slider(ui, theme, plain(texts, "qr.logo_size"), &mut percent, range) {
            logo.share = (percent / 100.0).clamp(QrLogo::MIN_SHARE, QrLogo::MAX_SHARE);
            out.changed = true;
        }
        out.changed |= switch(
            ui,
            theme,
            plain(texts, "qr.logo_clear"),
            &mut logo.clear_behind,
        );
    }
}

/// A slider under its label, the width of its column, its value at the
/// right edge.
fn slider(
    ui: &mut Ui,
    theme: &Theme,
    label: String,
    value: &mut f64,
    range: std::ops::RangeInclusive<f64>,
) -> bool {
    row(ui, theme, label, |ui| {
        ui.label(RichText::new(format!("{value:.0}")).font(theme.font("body-sm")));
    });
    ui.spacing_mut().slider_width = ui.available_width();
    let changed = ui
        .add(
            egui::Slider::new(value, range)
                .step_by(1.0)
                .show_value(false),
        )
        .changed();
    ui.add_space(ROW_GAP);
    changed
}

fn quality_fields(ui: &mut Ui, theme: &Theme, texts: &Texts, qr: &mut QrNode) -> bool {
    let mut changed = false;
    caption(ui, theme, plain(texts, "qr.correction"));
    ui.add_space(LABEL_GAP);
    if qr.logo.is_some() {
        caption(ui, theme, plain(texts, "qr.correction_logo"));
    } else {
        changed |= choice(
            ui,
            texts,
            "qr.correction",
            &mut qr.ecc,
            &[
                (QrEcc::L, "qr.ecc_l"),
                (QrEcc::M, "qr.ecc_m"),
                (QrEcc::Q, "qr.ecc_q"),
                (QrEcc::H, "qr.ecc_h"),
            ],
        );
    }
    ui.add_space(ROW_GAP);
    let mut margin = f64::from(qr.quiet_zone);
    let most = f64::from(QrNode::MAX_QUIET_ZONE);
    if slider(
        ui,
        theme,
        plain(texts, "qr.margin"),
        &mut margin,
        0.0..=most,
    ) {
        // Within 0..=10: the cast is exact.
        qr.quiet_zone = margin.round().clamp(0.0, most) as u8;
        changed = true;
    }
    changed
}

/// The fields of a code, in sections; those of what it says are left to the
/// caller, which shows them first.
pub(crate) fn fields(
    ui: &mut Ui,
    theme: &Theme,
    texts: &Texts,
    qr: &mut QrNode,
    tiles: &mut Tiles,
) -> Outcome {
    let mut out = Outcome::default();
    let section = |ui: &mut Ui, key: &'static str, open: bool, add: &mut dyn FnMut(&mut Ui)| {
        egui::CollapsingHeader::new(
            RichText::new(plain(texts, key))
                .font(theme.font("ui-label"))
                .color(theme.c("text")),
        )
        .id_salt(key)
        .default_open(open)
        .show_unindented(ui, |ui| add(ui));
        ui.add_space(4.0);
    };
    section(ui, "qr.content", true, &mut |ui| {
        out.changed |= content_fields(ui, theme, texts, &mut qr.content);
    });
    if gafira_qr::draw(qr) == Err(QrError::TooLong) {
        message(ui, theme, plain(texts, "qr.too_long"));
        ui.add_space(ROW_GAP);
    }
    section(ui, "qr.styles", false, &mut |ui| {
        let ctx = ui.ctx().clone();
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = Vec2::splat(6.0);
            for preset in &PRESETS {
                let texture = tiles.texture(&ctx, preset.key.to_owned(), || {
                    let mut qr = preset.on(&sample());
                    qr.quiet_zone = 1;
                    image(&qr, (160.0, 160.0), 1.0, None)
                });
                let size = Vec2::splat(56.0);
                let (rect, response) = ui.allocate_exact_size(size, Sense::click());
                if let Some(id) = texture {
                    let uv = Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0));
                    ui.painter().image(id, rect, uv, egui::Color32::WHITE);
                }
                let color = if response.hovered() || response.has_focus() {
                    theme.c("canvas.selection")
                } else {
                    theme.c("border")
                };
                let stroke = Stroke::new(1.0, color);
                let inside = egui::StrokeKind::Inside;
                ui.painter()
                    .rect_stroke(rect, theme.radius_md(), stroke, inside);
                let name = plain(texts, preset.key);
                response.widget_info(|| {
                    egui::WidgetInfo::labeled(egui::WidgetType::Button, true, &name)
                });
                if response.on_hover_text(name).clicked() {
                    *qr = preset.on(qr);
                    out.changed = true;
                }
            }
        });
    });
    section(ui, "qr.colors", true, &mut |ui| {
        out.changed |= color_fields(ui, theme, texts, qr);
    });
    section(ui, "qr.shapes", true, &mut |ui| {
        out.changed |= shape_fields(ui, theme, texts, qr, tiles);
    });
    section(ui, "qr.logo", false, &mut |ui| {
        logo_fields(ui, theme, texts, qr, &mut out)
    });
    section(ui, "qr.reading", false, &mut |ui| {
        out.changed |= quality_fields(ui, theme, texts, qr);
    });
    for warning in gafira_qr::warnings(qr) {
        let key = match warning {
            Warning::LowContrast => "qr.warn_contrast",
            Warning::Inverted => "qr.warn_inverted",
            Warning::Transparent => "qr.warn_transparent",
            Warning::NarrowMargin => "qr.warn_margin",
        };
        message_box(ui, theme, plain(texts, key));
    }
    out
}

// ---- The panel -----------------------------------------------------------

/// What the panel asks for.
pub(crate) enum Chosen {
    /// «Añadir al diseño», or a drop at a screen point.
    Insert(Box<QrNode>, Option<Pos2>),
    PickLogo,
}

/// A code being changed in Propiedades: what is written, which may not make
/// a code yet, over the code it started from (S80-04).
struct Editing {
    node: gafira_doc::NodeId,
    base: QrNode,
    written: QrNode,
}

/// The state of Elementos → Códigos QR and of the fields of Propiedades.
pub(crate) struct QrPanel {
    pub(crate) draft: QrNode,
    /// What was written of the other kinds.
    kept: Vec<QrContent>,
    tiles: Tiles,
    preview: Option<(u64, egui::TextureHandle)>,
    editing: Option<Editing>,
}

impl Default for QrPanel {
    fn default() -> Self {
        Self {
            draft: fresh(),
            kept: Vec::new(),
            tiles: Tiles::default(),
            preview: None,
            editing: None,
        }
    }
}

impl QrPanel {
    /// The panel (S80-11): the preview, «Añadir al diseño» and the fields.
    /// `logo` holds the pixels of the draft's image; `over` says whether a
    /// point of the screen is on a page.
    pub(crate) fn show(
        &mut self,
        ui: &mut Ui,
        theme: &Theme,
        texts: &Texts,
        logo: Option<&ColorImage>,
        over: &dyn Fn(Pos2) -> bool,
    ) -> Option<Chosen> {
        let mut chosen = None;
        let ready = gafira_qr::draw(&self.draft).is_ok();
        let key = key(&self.draft, logo.is_some());
        if ready && self.preview.as_ref().is_none_or(|(k, _)| *k != key) {
            let side = f64::from(PREVIEW);
            if let Some(pixels) = image(&self.draft, (side, side), 2.0, logo) {
                match &mut self.preview {
                    Some((k, texture)) => {
                        *k = key;
                        texture.set(pixels, egui::TextureOptions::LINEAR);
                    }
                    None => {
                        let options = egui::TextureOptions::LINEAR;
                        let texture = ui.ctx().load_texture("qr-preview", pixels, options);
                        self.preview = Some((key, texture));
                    }
                }
            }
        }
        ui.vertical_centered(|ui| {
            let sense = if ready {
                Sense::click_and_drag()
            } else {
                Sense::hover()
            };
            let (rect, response) = ui.allocate_exact_size(Vec2::splat(PREVIEW), sense);
            ui.painter()
                .rect_filled(rect, theme.radius_md(), theme.c("surface.sunken"));
            let uv = Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0));
            if let Some((_, texture)) = self.preview.as_ref().filter(|_| ready) {
                let white = egui::Color32::WHITE;
                ui.painter().image(texture.id(), rect, uv, white);
                if response.hovered() {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
                }
                let at = ui.input(|i| i.pointer.latest_pos());
                if response.dragged()
                    && let Some(at) = at
                {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
                    let layer =
                        egui::LayerId::new(egui::Order::Tooltip, egui::Id::new("editor.qr_ghost"));
                    let ghost = Rect::from_center_size(at, Vec2::splat(PREVIEW / 2.0));
                    let tint = white.gamma_multiply(0.6);
                    ui.ctx()
                        .layer_painter(layer)
                        .image(texture.id(), ghost, uv, tint);
                }
                if response.drag_stopped()
                    && let Some(at) = at
                    && over(at)
                {
                    chosen = Some(Chosen::Insert(Box::new(self.draft.clone()), Some(at)));
                }
            } else {
                ui.painter().text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    plain(texts, "qr.empty"),
                    theme.font("body-sm"),
                    theme.c("text.muted"),
                );
            }
            ui.add_space(8.0);
            ui.add_enabled_ui(ready, |ui| {
                if widgets::primary(ui, theme, plain(texts, "qr.add")).clicked() {
                    chosen = Some(Chosen::Insert(Box::new(self.draft.clone()), None));
                }
            });
            // Always there, so the form below does not jump as it is filled.
            ui.add_space(LABEL_GAP);
            let (words, color) = match waiting_text(texts, &self.draft) {
                Some(waiting) => (waiting, theme.c("text.secondary")),
                None => (plain(texts, "qr.ready"), theme.c("success.fg")),
            };
            ui.add(
                egui::Label::new(
                    RichText::new(words)
                        .font(theme.font("caption"))
                        .color(color),
                )
                .wrap(),
            );
        });
        ui.add_space(8.0);
        let before = Kind::of(&self.draft.content);
        let written = self.draft.content.clone();
        let out = fields(ui, theme, texts, &mut self.draft, &mut self.tiles);
        let now = Kind::of(&self.draft.content);
        if now != before {
            // What was written of a kind comes back with it.
            self.kept.retain(|c| Kind::of(c) != before);
            self.kept.push(written);
            if let Some(back) = self.kept.iter().find(|c| Kind::of(c) == now) {
                self.draft.content = back.clone();
            }
        }
        if out.pick_logo {
            chosen = Some(Chosen::PickLogo);
        }
        chosen
    }

    /// The fields of the code `node` in Propiedades (S80-12): the code to
    /// set, if it changed and can be made, and whether to pick a logo. What
    /// cannot be made yet stays in the fields and off the document.
    pub(crate) fn inspector(
        &mut self,
        ui: &mut Ui,
        theme: &Theme,
        texts: &Texts,
        (node, qr): (gafira_doc::NodeId, &QrNode),
    ) -> (Option<Box<QrNode>>, bool) {
        let mut written = match &self.editing {
            Some(e) if e.node == node && e.base == *qr => e.written.clone(),
            _ => qr.clone(),
        };
        let out = fields(ui, theme, texts, &mut written, &mut self.tiles);
        let set = self.written(node, qr, written, out.changed);
        (set, out.pick_logo)
    }

    /// Keeps what is written of `node`; the code to set, if it makes one.
    fn written(
        &mut self,
        node: gafira_doc::NodeId,
        qr: &QrNode,
        written: QrNode,
        changed: bool,
    ) -> Option<Box<QrNode>> {
        let set = changed && written.is_valid() && gafira_qr::draw(&written).is_ok();
        self.editing = Some(Editing {
            node,
            base: if set { written.clone() } else { qr.clone() },
            written: written.clone(),
        });
        set.then(|| Box::new(written))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(words: &str) -> QrContent {
        QrContent::Text {
            text: words.to_owned(),
        }
    }

    // S80-04: what cannot be made yet stays in the fields; the code keeps
    // the last one that could.
    #[test]
    fn s80_04_an_unfinished_field_keeps_the_last_code() {
        let node = gafira_doc::NodeId::new(5);
        let mut panel = QrPanel::default();
        let mut code = fresh();
        code.content = text("a");
        let mut empty = code.clone();
        empty.content = text("");
        assert_eq!(panel.written(node, &code, empty.clone(), true), None);
        let kept = panel.editing.as_ref().expect("kept");
        assert_eq!((&kept.written, &kept.base), (&empty, &code));
        let mut longer = code.clone();
        longer.content = text("ab");
        let set = panel.written(node, &code, longer.clone(), true);
        assert_eq!(set.as_deref(), Some(&longer));
        assert_eq!(panel.written(node, &longer, longer.clone(), false), None);
    }

    // S80-14: a style changes the looks alone, and every one can be read
    // without a warning.
    #[test]
    fn s80_14_a_style_keeps_what_the_code_says() {
        let code = sample();
        for preset in &PRESETS {
            let styled = preset.on(&code);
            assert_eq!(styled.content, code.content);
            assert_eq!((styled.ecc, styled.logo), (code.ecc, code.logo));
            assert!(styled.is_valid());
            assert!(gafira_qr::warnings(&styled).is_empty(), "{}", preset.key);
        }
    }

    // S80-03: each kind has an empty content of its own.
    #[test]
    fn s80_03_each_kind_has_its_content() {
        for kind in Kind::ALL {
            assert_eq!(Kind::of(&kind.empty()), kind);
            assert!(gafira_qr::payload(&kind.empty()).is_err());
        }
    }

    // S80-01: 300 characters in less than 50 ms, even in a debug build.
    #[test]
    fn s80_01_300_characters_are_quick() {
        let mut code = sample();
        code.content = text(&"g".repeat(300));
        code.style.body = QrBody::FluidCorners;
        code.style.eye_ball = QrEyeBall::Flower;
        let start = std::time::Instant::now();
        for _ in 0..4 {
            gafira_qr::draw(&code).expect("drawn");
        }
        let each = start.elapsed() / 4;
        assert!(each.as_millis() < 50, "{each:?}");
    }

    // S80-12: the canvas draws the background, the code and the logo.
    #[test]
    fn s80_12_the_canvas_draws_the_code() {
        let mut code = sample();
        code.logo = Some(QrLogo {
            asset: gafira_doc::AssetRef::new(gafira_doc::ContentHash([1; 32])),
            natural: (10, 10),
            share: 0.2,
            clear_behind: true,
        });
        let red = ColorImage::filled([10, 10], egui::Color32::RED);
        let drawn = image(&code, (200.0, 100.0), 2.0, Some(&red)).expect("pixels");
        assert_eq!(drawn.size, [400, 200]);
        assert_eq!(drawn[(200, 100)], egui::Color32::RED, "the logo");
        assert_eq!(drawn[(5, 5)].a(), 0, "beside the square");
        assert_eq!(drawn[(105, 5)], egui::Color32::WHITE, "the margin");
        let first = 100.0 + 200.0 * 4.5 / 33.0;
        assert_eq!(drawn[(first as usize, 27)], egui::Color32::BLACK, "an eye");
    }

    /// A form drawn over and over, as in the panel: 248 px wide.
    struct Drawn {
        ctx: egui::Context,
        theme: Theme,
        content: QrContent,
        words: Vec<String>,
        rects: Vec<Rect>,
    }

    impl Drawn {
        fn new(content: QrContent) -> Self {
            let tokens = gafira_theme::ThemeTokens::embedded().expect("tokens");
            let (ctx, theme) = (egui::Context::default(), Theme::new(tokens));
            theme.apply(&ctx);
            let mut drawn = Self {
                ctx,
                theme,
                content,
                words: Vec::new(),
                rects: Vec::new(),
            };
            drawn.pass(Vec::new());
            drawn.pass(Vec::new());
            drawn
        }

        fn pass(&mut self, events: Vec<egui::Event>) {
            let texts = Texts::new(gafira_ports::i18n::Locale::Es);
            let input = egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(248.0, 1400.0))),
                events,
                ..egui::RawInput::default()
            };
            let (theme, content) = (&self.theme, &mut self.content);
            let output = self.ctx.run(input, |ctx| {
                egui::CentralPanel::default()
                    .frame(egui::Frame::NONE)
                    .show(ctx, |ui| content_fields(ui, theme, &texts, content));
            });
            self.words = output
                .shapes
                .iter()
                .filter_map(|clipped| match &clipped.shape {
                    egui::Shape::Text(text) => Some(text.galley.text().to_owned()),
                    _ => None,
                })
                .collect();
            self.rects = self.ctx.viewport(|v| {
                let layers = v.prev_pass.widgets.layers();
                let all = layers.flat_map(|(_, widgets)| widgets.iter());
                all.filter(|w| w.sense.senses_click() && w.rect.width() > 60.0)
                    .map(|w| w.rect)
                    .filter(|rect| (24.0..100.0).contains(&rect.height()))
                    .collect()
            });
        }

        fn says(&self, words: &str) -> bool {
            self.words.iter().any(|w| w == words)
        }
    }

    const REQUIRED: &str = "Este dato es obligatorio.";

    // S80-17: an untouched form marks what it needs and scolds about
    // nothing; a field used and left empty says it is required, at once.
    #[test]
    fn s80_17_a_required_field_says_so_once_used() {
        let mut form = Drawn::new(Kind::Url.empty());
        assert!(form.says("*"), "marked as required: {:?}", form.words);
        assert!(!form.says(REQUIRED));
        let field = form.rects.last().copied().expect("the field").center();
        let button = |pressed| egui::Event::PointerButton {
            pos: field,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        };
        form.pass(vec![egui::Event::PointerMoved(field)]);
        form.pass(vec![button(true)]);
        form.pass(vec![button(false)]);
        form.pass(vec![egui::Event::Text("a".to_owned())]);
        assert_eq!(
            form.content,
            QrContent::Url {
                url: "a".to_owned()
            }
        );
        form.pass(vec![egui::Event::Key {
            key: egui::Key::Backspace,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        }]);
        form.pass(Vec::new());
        assert!(form.says(REQUIRED), "{:?}", form.words);
    }

    // S80-17: a mistyped value is told as it is written, with an example;
    // mended, the message goes.
    #[test]
    fn s80_17_a_mistyped_value_says_how_to_mend_it() {
        let url = |url: &str| QrContent::Url {
            url: url.to_owned(),
        };
        let email = |address: &str| QrContent::Email {
            address: address.to_owned(),
            subject: String::new(),
            body: String::new(),
        };
        let bad_url = "Escribe una dirección válida, por ejemplo https://ejemplo.com";
        let bad_email = "Escribe un correo válido, por ejemplo nombre@ejemplo.com";
        assert!(Drawn::new(url("htp://a.com")).says(bad_url));
        assert!(!Drawn::new(url("https://a.com")).says(bad_url));
        assert!(Drawn::new(email("ana@")).says(bad_email));
        assert!(!Drawn::new(email("ana@a.com")).says(bad_email));
    }

    // S80-17: every problem has words of its own in both languages.
    #[test]
    fn s80_17_the_button_says_what_is_missing() {
        let texts = Texts::new(gafira_ports::i18n::Locale::Es);
        let mut code = fresh();
        code.content = Kind::Wifi.empty();
        assert_eq!(
            waiting_text(&texts, &code).as_deref(),
            Some("Para añadirlo, completa: Nombre de la red, Clave")
        );
        code.content = text(&"a".repeat(3000));
        let long = plain(&texts, "qr.too_long");
        assert_eq!(waiting_text(&texts, &code), Some(long));
        code.content = text("a");
        assert_eq!(waiting_text(&texts, &code), None);
    }

    // S80-18: every field of every kind lines up: the whole column, or one
    // of two halves of the same width, side by side.
    #[test]
    fn s80_18_the_fields_line_up() {
        for kind in Kind::ALL {
            let form = Drawn::new(kind.empty());
            assert!(form.rects.len() >= 2, "{kind:?}: {:?}", form.rects);
            let near = |a: f32, b: f32| (a - b).abs() < 1.0;
            let mut halves: Vec<Rect> = Vec::new();
            for rect in &form.rects {
                if near(rect.left(), 0.0) && near(rect.right(), 248.0) {
                    continue;
                }
                halves.push(*rect);
            }
            for pair in halves.chunks(2) {
                let [left, right] = pair else {
                    panic!("{kind:?}: a field out of line: {pair:?}")
                };
                assert!(near(left.left(), 0.0), "{kind:?}: {left:?}");
                assert!(near(right.right(), 248.0), "{kind:?}: {right:?}");
                assert!(near(left.width(), right.width()), "{kind:?}: {pair:?}");
                assert!(near(left.top(), right.top()), "{kind:?}: {pair:?}");
                assert!(near(left.height(), right.height()), "{kind:?}: {pair:?}");
            }
        }
    }
}
