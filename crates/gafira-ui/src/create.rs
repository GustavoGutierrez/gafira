//! «Crear un diseño» (spec 076), opened from Inicio on a category (spec
//! 091): the categories on the
//! left (Para ti, Redes sociales, Presentaciones, Video, Imprimir,
//! Documentos, Fotos, Web y correo, Elegir el tamaño), a search over every
//! size, the groups of a category as chips, every size as a card drawn in
//! its own proportions, a custom size with its unit and a locked aspect,
//! and the recent projects with their thumbnails (spec 033). A card opens
//! the editor on a page of its size with its guides.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use egui::{Align, Color32, Layout, Rect, RichText, Sense, Stroke, Vec2};
use gafira_doc::msg::{Message, MessageKey};
use gafira_geom::Unit;

use crate::design_art;
use crate::design_sizes::{self, Category, DesignSize};
use crate::icons::{self, Icon, Weight};
use crate::recent::{self, RecentProject, RecentThumbnail, ThumbCache};
use crate::shell::Go;
use crate::style::{DISPLAY, Theme};
use crate::text::Texts;
use crate::widgets;

/// The units «Elegir el tamaño» offers.
const UNITS: [Unit; 4] = [Unit::Px, Unit::In, Unit::Mm, Unit::Cm];

/// The fields of «Elegir el tamaño».
struct Custom {
    width: String,
    height: String,
    unit: Unit,
    /// Width over height while the lock is closed.
    locked: Option<f64>,
}

/// The state of «Crear un diseño»: the category, group and search the
/// person is on, the custom size, and the recent projects with their
/// thumbnails, read as it appears.
pub(crate) struct Create {
    category: Category,
    group: Option<&'static str>,
    query: String,
    custom: Custom,
    recent: Vec<RecentProject>,
    thumbs: BTreeMap<PathBuf, RecentThumbnail>,
}

/// The textures of the thumbnails Inicio shows; the shell keeps them from
/// one visit of Inicio to the next (S33-07).
pub(crate) type Thumbs = ThumbCache<egui::TextureHandle>;

/// The text of `key`.
fn t(texts: &Texts, key: &'static str) -> String {
    texts.t(&Message {
        key: MessageKey(key),
        args: Vec::new(),
    })
}

/// A number typed with a comma or a point, if it is one above zero.
fn number(text: &str) -> Option<f64> {
    text.trim()
        .replace(',', ".")
        .parse::<f64>()
        .ok()
        .filter(|v| v.is_finite() && *v > 0.0)
}

/// `v` written short, with the decimal mark of the language.
fn written(v: f64, texts: &Texts) -> String {
    let text = format!("{v:.2}");
    let text = text.trim_end_matches('0').trim_end_matches('.').to_owned();
    if matches!(texts.locale, gafira_ports::i18n::Locale::Es) {
        text.replace('.', ",")
    } else {
        text
    }
}

/// The least and the greatest side of a custom size, in px (spec 076).
const SIDE_PX: (f64, f64) = (40.0, 8000.0);

/// Px per one `unit` at Gafira's 96 dpi.
fn px_per(unit: Unit) -> f64 {
    gafira_geom::Length::new(1.0, unit)
        .to_px(f64::from(gafira_doc::Dpi::DEFAULT.get()))
        .unwrap_or(1.0)
}

/// The least and the greatest side in `unit`, to two decimals, rounded
/// inwards so both ends are allowed.
fn limits(unit: Unit) -> (f64, f64) {
    let k = px_per(unit);
    (
        (SIDE_PX.0 / k * 100.0).ceil() / 100.0,
        (SIDE_PX.1 / k * 100.0).floor() / 100.0,
    )
}

/// `v` with up to two decimals and thousands grouped, as the language
/// writes it: «8.000», «2.116,66» in Spanish; «8,000», «2,116.66» in
/// English.
fn grouped(v: f64, texts: &Texts) -> String {
    let es = matches!(texts.locale, gafira_ports::i18n::Locale::Es);
    let (group, mark) = if es { ('.', ',') } else { (',', '.') };
    let text = format!("{v:.2}");
    let (whole, cents) = text.split_once('.').unwrap_or((&text, "00"));
    let mut out = String::new();
    for (i, digit) in whole.chars().enumerate() {
        if i > 0 && (whole.len() - i) % 3 == 0 {
            out.push(group);
        }
        out.push(digit);
    }
    let cents = cents.trim_end_matches('0');
    if !cents.is_empty() {
        out.push(mark);
        out.push_str(cents);
    }
    out
}

impl Custom {
    /// Whether a side typed as `v` in the chosen unit is allowed.
    fn allowed(&self, v: f64) -> bool {
        let (least, most) = limits(self.unit);
        (least..=most).contains(&v)
    }

    /// The page the fields describe, when it is one Gafira can make.
    fn page(&self) -> Option<gafira_doc::Page> {
        let (w, h) = (number(&self.width)?, number(&self.height)?);
        if !self.allowed(w) || !self.allowed(h) {
            return None;
        }
        let size = design_sizes::page_size(w, h, self.unit);
        let page = gafira_doc::Page::new(gafira_doc::PageId::new(1), String::new(), size);
        page.check_layout(gafira_doc::Dpi::DEFAULT).ok()?;
        Some(page)
    }
}

impl Create {
    /// «Crear un diseño» on `category`.
    pub(crate) fn new(
        recent: Vec<RecentProject>,
        thumbs: BTreeMap<PathBuf, RecentThumbnail>,
        category: Category,
    ) -> Self {
        Self {
            category,
            group: None,
            query: String::new(),
            custom: Custom {
                width: String::new(),
                height: String::new(),
                unit: Unit::Px,
                locked: None,
            },
            recent,
            thumbs,
        }
    }

    /// «Crear un diseño» at `now` (seconds since the Unix epoch, for the
    /// relative times of «Recientes»), with the thumbnails' textures in
    /// `textures`.
    pub(crate) fn show(
        &mut self,
        ctx: &egui::Context,
        theme: &Theme,
        texts: &Texts,
        now: i64,
        textures: &mut Thumbs,
    ) -> Go {
        textures.keep_only(self.thumbs.keys().map(PathBuf::as_path));
        let mut go = Go::Stay;
        if let Some(next) = start_topbar(ctx, theme, texts, Some(Tab::Home)) {
            go = next;
        }
        egui::SidePanel::left("create.categories")
            .exact_width(252.0)
            .resizable(false)
            .show_separator_line(false)
            .frame(
                egui::Frame::NONE
                    .fill(theme.c("bg"))
                    .inner_margin(egui::Margin {
                        left: 20,
                        right: 12,
                        top: 28,
                        bottom: 20,
                    }),
            )
            .show(ctx, |ui| {
                if let Some(next) = self.side(ui, theme, texts) {
                    go = next;
                }
            });
        egui::CentralPanel::default()
            .frame(
                egui::Frame::NONE
                    .fill(theme.c("bg"))
                    .inner_margin(egui::Margin {
                        left: 16,
                        right: 28,
                        top: 28,
                        bottom: 0,
                    }),
            )
            .show(ctx, |ui| {
                self.search_box(ui, theme, texts);
                ui.add_space(16.0);
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        if let Some(next) = self.body(ui, theme, texts, now, textures) {
                            go = next;
                        }
                        ui.add_space(28.0);
                    });
            });
        if let Some(next) = start_keys(ctx) {
            go = next;
        }
        go
    }

    /// The side list: back to Inicio, the title, the categories,
    /// Plantillas and «Abrir»; where a click on one of the last goes.
    fn side(&mut self, ui: &mut egui::Ui, theme: &Theme, texts: &Texts) -> Option<Go> {
        let mut go = None;
        let back = egui::Button::image_and_text(
            icons::image(
                ui.ctx(),
                Icon::Back,
                Weight::Regular,
                16.0,
                theme.c("text.secondary"),
            ),
            RichText::new(t(texts, "create.back_home"))
                .font(theme.font("body-sm"))
                .color(theme.c("text.secondary")),
        )
        .frame(false);
        if ui.add(back).clicked() {
            go = Some(Go::Home);
        }
        ui.add_space(10.0);
        ui.label(
            RichText::new(t(texts, "create.title"))
                .font(egui::FontId::new(
                    28.0,
                    egui::FontFamily::Name(DISPLAY.into()),
                ))
                .color(theme.c("text")),
        );
        ui.add_space(20.0);
        ui.spacing_mut().item_spacing.y = 2.0;
        for category in Category::ALL {
            if category == Category::Custom {
                ui.add_space(10.0);
            }
            let on = self.category == category && self.query.is_empty();
            let icon = |ui: &egui::Ui, rect: Rect| {
                let side = (20.0 * ui.ctx().pixels_per_point()).round() as u32;
                let name = format!("cat/{}", category.key());
                if let Some(texture) = design_art::texture(ui.ctx(), &name, side, || {
                    design_art::category_svg(category)
                }) {
                    ui.painter().image(
                        texture.id(),
                        rect,
                        Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                        Color32::WHITE,
                    );
                }
            };
            if side_row(ui, theme, on, &t(texts, category.key()), icon).clicked() {
                self.category = category;
                self.group = None;
                self.query.clear();
            }
        }
        ui.add_space(10.0);
        let templates = side_row(
            ui,
            theme,
            false,
            &t(texts, "nav.templates"),
            |ui: &egui::Ui, rect| {
                icons::paint(
                    ui.painter(),
                    rect,
                    Icon::Templates,
                    Weight::Regular,
                    theme.c("text"),
                );
            },
        );
        if templates.clicked() {
            go = Some(Go::Templates(None));
        }
        let open = side_row(
            ui,
            theme,
            false,
            &t(texts, "home.open"),
            |ui: &egui::Ui, rect| {
                icons::paint(
                    ui.painter(),
                    rect,
                    Icon::Export,
                    Weight::Regular,
                    theme.c("text"),
                );
            },
        );
        if open.clicked() {
            go = Some(Go::File(crate::files::FileAction::Open));
        }
        go
    }

    /// The search over every size.
    fn search_box(&mut self, ui: &mut egui::Ui, theme: &Theme, texts: &Texts) {
        egui::Frame::NONE
            .fill(theme.c("surface"))
            .stroke(Stroke::new(1.0, theme.c("border")))
            .corner_radius(theme.radius_md())
            .inner_margin(egui::Margin::symmetric(12, 2))
            .show(ui, |ui| {
                let row = Vec2::new(ui.available_width(), 38.0);
                ui.allocate_ui_with_layout(row, Layout::left_to_right(Align::Center), |ui| {
                    let (rect, _) = ui.allocate_exact_size(Vec2::splat(18.0), Sense::hover());
                    let color = theme.c("text.secondary");
                    let c = rect.center() - Vec2::new(1.5, 1.5);
                    ui.painter().circle_stroke(c, 5.5, Stroke::new(1.8, color));
                    ui.painter().line_segment(
                        [c + Vec2::splat(4.0), c + Vec2::splat(8.5)],
                        Stroke::new(1.8, color),
                    );
                    ui.add(
                        egui::TextEdit::singleline(&mut self.query)
                            .hint_text(t(texts, "create.search_hint"))
                            .frame(false)
                            .margin(Vec2::new(6.0, 9.0))
                            .desired_width(f32::INFINITY),
                    );
                });
            });
    }

    /// What the category, or the search, shows; what to do when a size or
    /// a recent project is clicked.
    fn body(
        &mut self,
        ui: &mut egui::Ui,
        theme: &Theme,
        texts: &Texts,
        now: i64,
        textures: &mut Thumbs,
    ) -> Option<Go> {
        let mut go = None;
        let mut picked: Option<&'static DesignSize> = None;
        if !self.query.trim().is_empty() {
            let found = design_sizes::search(&self.query, |s| t(texts, s.key));
            heading(ui, theme, &t(texts, "create.results"));
            if found.is_empty() {
                ui.label(
                    RichText::new(t(texts, "create.no_results"))
                        .font(theme.font("body"))
                        .color(theme.c("text.muted")),
                );
            }
            picked = picked.or(grid(ui, theme, texts, &found));
            return picked.map(|size| Go::Create(Box::new(size.page())));
        }
        match self.category {
            Category::ForYou => {
                for (title, keys) in design_sizes::FOR_YOU {
                    heading(ui, theme, &t(texts, title));
                    let sizes: Vec<_> = keys
                        .iter()
                        .filter_map(|k| design_sizes::by_key(k))
                        .collect();
                    picked = picked.or(row(ui, theme, texts, title, &sizes));
                    ui.add_space(22.0);
                }
                if let Some(path) = self.recents(ui, theme, texts, now, textures) {
                    go = Some(Go::OpenRecent(path));
                }
            }
            Category::Custom => {
                if let Some(page) = self.custom_form(ui, theme, texts) {
                    go = Some(Go::Create(Box::new(page)));
                }
                ui.add_space(24.0);
                heading(ui, theme, &t(texts, "create.popular_layouts"));
                let sizes: Vec<_> = design_sizes::POPULAR_LAYOUTS
                    .iter()
                    .filter_map(|k| design_sizes::by_key(k))
                    .collect();
                picked = picked.or(grid(ui, theme, texts, &sizes));
            }
            category => {
                let groups = design_sizes::groups(category);
                if !groups.is_empty() {
                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing = Vec2::new(8.0, 8.0);
                        if pill(ui, theme, self.group.is_none(), &t(texts, "create.all")).clicked()
                        {
                            self.group = None;
                        }
                        for (group, key) in groups {
                            if pill(ui, theme, self.group == Some(*group), &t(texts, key)).clicked()
                            {
                                self.group = Some(group);
                            }
                        }
                    });
                    ui.add_space(18.0);
                }
                let title = groups
                    .iter()
                    .find(|(g, _)| Some(*g) == self.group)
                    .map_or_else(|| t(texts, category.key()), |(_, key)| t(texts, key));
                heading(ui, theme, &title);
                picked = picked.or(grid(
                    ui,
                    theme,
                    texts,
                    &design_sizes::of(category, self.group),
                ));
            }
        }
        picked.map(|size| Go::Create(Box::new(size.page()))).or(go)
    }

    /// «Elegir el tamaño»: width, height, unit, the aspect lock and
    /// «Crear un diseño»; the page when it is clicked.
    fn custom_form(
        &mut self,
        ui: &mut egui::Ui,
        theme: &Theme,
        texts: &Texts,
    ) -> Option<gafira_doc::Page> {
        heading(ui, theme, &t(texts, "create.custom"));
        let page = self.custom.page();
        let mut create = false;
        // One height for every control of the row: the button's.
        let height = crate::style::px(theme.tokens.size.control.lg);
        let gap = 10.0;
        let field_w =
            ((ui.available_width() - 2.0 * height - 4.0 * gap - 140.0) / 3.0).clamp(110.0, 220.0);
        // The labels, over their controls.
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = gap;
            for key in ["create.width", "create.height", "create.units"] {
                let (rect, _) = ui.allocate_exact_size(Vec2::new(field_w, 16.0), Sense::hover());
                ui.painter().text(
                    rect.left_center(),
                    egui::Align2::LEFT_CENTER,
                    t(texts, key),
                    theme.font("caption"),
                    theme.c("text.secondary"),
                );
            }
        });
        ui.add_space(4.0);
        let row = Vec2::new(ui.available_width(), height);
        ui.allocate_ui_with_layout(row, Layout::left_to_right(Align::Center), |ui| {
            ui.spacing_mut().item_spacing.x = gap;
            ui.spacing_mut().interact_size.y = height;
            let text_h = ui.text_style_height(&egui::TextStyle::Body);
            let margin = Vec2::new(10.0, ((height - text_h) / 2.0).max(2.0));
            let field = |ui: &mut egui::Ui, text: &mut String| {
                ui.add_sized(
                    Vec2::new(field_w, height),
                    egui::TextEdit::singleline(text)
                        .margin(margin)
                        .vertical_align(Align::Center),
                )
                .changed()
            };
            let width_changed = field(ui, &mut self.custom.width);
            let height_changed = field(ui, &mut self.custom.height);
            if let Some(ratio) = self.custom.locked {
                if width_changed && let Some(w) = number(&self.custom.width) {
                    self.custom.height = written(w / ratio, texts);
                } else if height_changed && let Some(h) = number(&self.custom.height) {
                    self.custom.width = written(h * ratio, texts);
                }
            }
            let unit = &mut self.custom.unit;
            ui.allocate_ui_with_layout(
                Vec2::new(field_w, height),
                Layout::left_to_right(Align::Center),
                |ui| {
                    ui.spacing_mut().button_padding.y = (height - text_h) / 2.0;
                    egui::ComboBox::from_id_salt("create.unit")
                        .width(field_w)
                        .height(240.0)
                        .selected_text(unit_name(*unit))
                        .show_ui(ui, |ui| {
                            for u in UNITS {
                                ui.selectable_value(unit, u, unit_name(u));
                            }
                        });
                },
            );
            let closed = self.custom.locked.is_some();
            let icon = if closed { Icon::Locked } else { Icon::Unlocked };
            let tint = if closed {
                theme.c("text")
            } else {
                theme.c("text.muted")
            };
            let button =
                egui::Button::image(icons::image(ui.ctx(), icon, Weight::Regular, 18.0, tint))
                    .frame(false)
                    .min_size(Vec2::splat(height));
            let hint = if closed {
                "create.unlock_ratio"
            } else {
                "create.lock_ratio"
            };
            if ui.add(button).on_hover_text(t(texts, hint)).clicked() {
                self.custom.locked = if closed {
                    None
                } else {
                    number(&self.custom.width)
                        .zip(number(&self.custom.height))
                        .map(|(w, h)| w / h)
                };
            }
            ui.add_enabled_ui(page.is_some(), |ui| {
                if widgets::primary(ui, theme, t(texts, "create.create")).clicked() {
                    create = true;
                }
            });
        });
        // A side out of bounds says the bounds in the chosen unit; text
        // that is not a number says so.
        let sides = [&self.custom.width, &self.custom.height];
        let typed: Vec<&String> = sides.into_iter().filter(|s| !s.trim().is_empty()).collect();
        let warning = if typed.iter().any(|s| number(s).is_none()) {
            Some(t(texts, "create.out_of_range"))
        } else if typed
            .iter()
            .filter_map(|s| number(s))
            .any(|v| !self.custom.allowed(v))
        {
            let (least, most) = limits(self.custom.unit);
            let unit = unit_name(self.custom.unit);
            Some(texts.t(&gafira_doc::msg!(
                "create.size_limits",
                min = Text(format!("{} {unit}", grouped(least, texts))),
                max = Text(format!("{} {unit}", grouped(most, texts)))
            )))
        } else {
            None
        };
        if let Some(warning) = warning {
            ui.add_space(6.0);
            ui.label(
                RichText::new(warning)
                    .font(theme.font("caption"))
                    .color(theme.c("error.fg")),
            );
        }
        if create { page } else { None }
    }

    /// «Recientes»: the projects opened last; the one clicked.
    fn recents(
        &self,
        ui: &mut egui::Ui,
        theme: &Theme,
        texts: &Texts,
        now: i64,
        textures: &mut Thumbs,
    ) -> Option<PathBuf> {
        heading(ui, theme, &t(texts, "home.recent"));
        let shown = recent::shown(&self.recent);
        if shown.is_empty() {
            ui.label(
                RichText::new(t(texts, "home.empty"))
                    .font(theme.font("body"))
                    .color(theme.c("text.muted")),
            );
            return None;
        }
        let mut open = None;
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = Vec2::splat(12.0);
            for project in shown {
                let picture = self.thumbs.get(&project.path).map(|thumb| {
                    let texture = textures.get(&project.path, thumb, |thumb| {
                        upload(ui.ctx(), &project.path, thumb)
                    });
                    (texture.clone(), (thumb.width, thumb.height))
                });
                if recent_card(ui, theme, texts, project, now, picture.as_ref()) {
                    open = Some(project.path.clone());
                }
            }
        });
        open
    }
}

/// Which screen of the topbar is on: Inicio or Plantillas; `None` for
/// «Crear un diseño», which hangs from Inicio.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Tab {
    Home,
    Templates,
    /// Marca (spec 093).
    Brand,
}

/// The topbar of the start screens (spec 092): the logo, Inicio and
/// Plantillas with their icons, and Ajustes on the right; where a click goes.
pub(crate) fn start_topbar(
    ctx: &egui::Context,
    theme: &Theme,
    texts: &Texts,
    on: Option<Tab>,
) -> Option<Go> {
    let mut go = None;
    widgets::topbar(ctx, theme, "start.topbar", |ui| {
        ui.add_space(4.0);
        if widgets::topbar_logo(ui, 26.0)
            .on_hover_text(t(texts, "nav.home"))
            .clicked()
        {
            go = Some(Go::Home);
        }
        ui.add_space(28.0);
        ui.spacing_mut().item_spacing.x = 6.0;
        let home = t(texts, "nav.home");
        if widgets::nav_item(ui, theme, Icon::Home, &home, on == Some(Tab::Home)).clicked() {
            go = Some(Go::Home);
        }
        let templates = t(texts, "nav.templates");
        if widgets::nav_item(
            ui,
            theme,
            Icon::Templates,
            &templates,
            on == Some(Tab::Templates),
        )
        .clicked()
        {
            go = Some(Go::Templates(None));
        }
        let brand = t(texts, "nav.brand");
        if widgets::nav_item(ui, theme, Icon::Identity, &brand, on == Some(Tab::Brand)).clicked() {
            go = Some(Go::Brand(None));
        }
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            let settings = t(texts, "nav.settings");
            if widgets::nav_item(ui, theme, Icon::Settings, &settings, false).clicked() {
                go = Some(Go::Settings);
            }
        });
    });
    go
}

/// Ctrl+N and Ctrl+O on the start screens.
pub(crate) fn start_keys(ctx: &egui::Context) -> Option<Go> {
    if ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::N)) {
        // Ctrl+N: a square post, the size Gafira always started with.
        if let Some(size) = design_sizes::by_key("size.ig_square") {
            return Some(Go::Create(Box::new(size.page())));
        }
    }
    if ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::O)) {
        return Some(Go::File(crate::files::FileAction::Open));
    }
    None
}

fn unit_name(unit: Unit) -> &'static str {
    match unit {
        Unit::In => "in",
        Unit::Mm => "mm",
        Unit::Cm => "cm",
        Unit::Pt => "pt",
        Unit::Px => "px",
    }
}

/// A title over a row or a grid of cards.
pub(crate) fn heading(ui: &mut egui::Ui, theme: &Theme, text: &str) {
    ui.label(
        RichText::new(text)
            .font(theme.font("h3"))
            .color(theme.c("text")),
    );
    ui.add_space(12.0);
}

/// One line of the side list: its icon and name, lit when chosen.
pub(crate) fn side_row(
    ui: &mut egui::Ui,
    theme: &Theme,
    on: bool,
    label: &str,
    icon: impl FnOnce(&egui::Ui, Rect),
) -> egui::Response {
    let width = ui.available_width();
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, 38.0), Sense::click());
    let fill = if on {
        theme.c("selection.bg")
    } else if response.hovered() {
        theme.c("secondary.bg")
    } else {
        Color32::TRANSPARENT
    };
    ui.painter().rect_filled(rect, 8.0, fill);
    let icon_rect = Rect::from_center_size(
        egui::pos2(rect.left() + 22.0, rect.center().y),
        Vec2::splat(20.0),
    );
    icon(ui, icon_rect);
    ui.painter().text(
        egui::pos2(rect.left() + 44.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        label,
        theme.font("body"),
        theme.c("text"),
    );
    if response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    response
}

/// A filter chip: outlined, lit when chosen.
pub(crate) fn pill(ui: &mut egui::Ui, theme: &Theme, on: bool, label: &str) -> egui::Response {
    let (fill, stroke) = if on {
        (theme.c("selection.bg"), Stroke::new(1.5, theme.c("accent")))
    } else {
        (theme.c("surface"), Stroke::new(1.0, theme.c("border")))
    };
    let response = ui.add(
        egui::Button::new(
            RichText::new(label)
                .font(theme.font("body-sm"))
                .color(theme.c("text")),
        )
        .fill(fill)
        .stroke(stroke)
        .corner_radius(18.0)
        .min_size(Vec2::new(0.0, 34.0)),
    );
    if response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    response
}

/// The cards of `sizes` in as many columns as fit; the one clicked.
pub(crate) fn grid(
    ui: &mut egui::Ui,
    theme: &Theme,
    texts: &Texts,
    sizes: &[&'static DesignSize],
) -> Option<&'static DesignSize> {
    let gap = 16.0;
    let room = ui.available_width();
    let columns = ((room + gap) / (200.0 + gap)).floor().max(1.0);
    let width = ((room - gap * (columns - 1.0)) / columns).min(280.0);
    let mut picked = None;
    for chunk in sizes.chunks(columns as usize) {
        ui.horizontal_top(|ui| {
            ui.spacing_mut().item_spacing.x = gap;
            for size in chunk {
                if card(ui, theme, texts, size, width) {
                    picked = Some(*size);
                }
            }
        });
        ui.add_space(14.0);
    }
    picked
}

/// The cards of `sizes` in one row that scrolls sideways; the one clicked.
fn row(
    ui: &mut egui::Ui,
    theme: &Theme,
    texts: &Texts,
    id: &str,
    sizes: &[&'static DesignSize],
) -> Option<&'static DesignSize> {
    let mut picked = None;
    egui::ScrollArea::horizontal()
        .id_salt(id)
        .auto_shrink([false, true])
        .show(ui, |ui| {
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = 16.0;
                for size in sizes {
                    if card(ui, theme, texts, size, 212.0) {
                        picked = Some(*size);
                    }
                }
            });
        });
    picked
}

/// One size: its picture on a soft tile that lifts on hover, its name and
/// its size; `true` when clicked.
fn card(ui: &mut egui::Ui, theme: &Theme, texts: &Texts, size: &DesignSize, width: f32) -> bool {
    let name = t(texts, size.key);
    let label = size.label(texts.locale);
    let tile_h = (width * 0.7).round();
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, tile_h + 50.0), Sense::click());
    let tile = Rect::from_min_size(rect.min, Vec2::new(width, tile_h));
    let hover = ui.ctx().animate_bool(response.id, response.hovered());
    let fill = theme
        .c("surface.sunken")
        .lerp_to_gamma(theme.c("secondary.bg.hover"), hover);
    ui.painter().rect_filled(tile, 12.0, fill);
    if response.hovered() {
        ui.painter().rect_stroke(
            tile,
            12.0,
            Stroke::new(1.5, theme.c("accent")),
            egui::StrokeKind::Inside,
        );
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    // The picture, 240 × 170 in its own units, fills the tile less a margin
    // and lifts a little on hover.
    let art_w = (width - 24.0).min((tile_h - 16.0) * 240.0 / 170.0);
    let art = Rect::from_center_size(
        tile.center() - Vec2::new(0.0, 3.0 * hover),
        Vec2::new(art_w, art_w * 170.0 / 240.0),
    );
    let side = (art_w * ui.ctx().pixels_per_point()).round() as u32;
    let key = format!("card/{}", size.key);
    if let Some(texture) = design_art::texture(ui.ctx(), &key, side, || {
        design_art::card_svg(size.look, size.aspect(), size.key)
    }) {
        ui.painter().image(
            texture.id(),
            art,
            Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            Color32::WHITE,
        );
    }
    let text_w = width - 4.0;
    let galley = ui
        .painter()
        .layout(name.clone(), theme.font("body-sm"), theme.c("text"), text_w);
    let name_at = egui::pos2(tile.left() + 2.0, tile.bottom() + 8.0);
    let name_h = galley.size().y.min(34.0);
    ui.painter().galley(name_at, galley, theme.c("text"));
    ui.painter().text(
        egui::pos2(name_at.x, name_at.y + name_h + 2.0),
        egui::Align2::LEFT_TOP,
        label,
        theme.font("caption"),
        theme.c("text.muted"),
    );
    response.on_hover_text(name).clicked()
}

/// The texture of the thumbnail of the project at `path`.
pub(crate) fn upload(
    ctx: &egui::Context,
    path: &Path,
    thumb: &RecentThumbnail,
) -> egui::TextureHandle {
    let size = [thumb.width as usize, thumb.height as usize];
    ctx.load_texture(
        format!("recent:{}", path.display()),
        egui::ColorImage::from_rgba_unmultiplied(size, &thumb.rgba),
        egui::TextureOptions::LINEAR,
    )
}

/// One recent project: its thumbnail, whole and centred on the
/// placeholder, or the placeholder alone when it has none (spec 033); its
/// name and how long ago it was opened; `true` when clicked.
pub(crate) fn recent_card(
    ui: &mut egui::Ui,
    theme: &Theme,
    texts: &Texts,
    project: &RecentProject,
    now: i64,
    picture: Option<&(egui::TextureHandle, (u32, u32))>,
) -> bool {
    let frame = egui::Frame::NONE
        .fill(theme.c("bg"))
        .stroke(Stroke::new(1.0, theme.c("border")))
        .corner_radius(theme.radius_md())
        .inner_margin(egui::Margin::same(8));
    frame
        .show(ui, |ui| {
            ui.set_width(176.0);
            ui.vertical(|ui| {
                let (thumb, _) = ui.allocate_exact_size(Vec2::new(176.0, 110.0), Sense::hover());
                ui.painter()
                    .rect_filled(thumb, theme.radius_md(), theme.c("surface"));
                if let Some((texture, (width, height))) = picture {
                    let (w, h) = recent::fit(*width, *height, (thumb.width(), thumb.height()));
                    ui.painter().image(
                        texture.id(),
                        egui::Rect::from_center_size(thumb.center(), Vec2::new(w, h)),
                        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                        egui::Color32::WHITE,
                    );
                }
                ui.add_space(4.0);
                ui.label(
                    RichText::new(&project.name)
                        .font(theme.font("ui-label"))
                        .color(theme.c("text")),
                );
                ui.label(
                    RichText::new(texts.relative(recent::age(project.opened_at, now)))
                        .font(theme.font("caption"))
                        .color(theme.c("text.muted")),
                );
            });
        })
        .response
        .interact(Sense::click())
        .clicked()
}

#[cfg(test)]
mod size_limits {
    use super::*;

    // Spec 076: a custom side lies between 40 px and 8000 px, told in the
    // chosen unit; both ends are allowed.
    #[test]
    fn sides_are_bounded_in_every_unit() {
        assert_eq!(limits(Unit::Px), (40.0, 8000.0));
        assert_eq!(limits(Unit::In), (0.42, 83.33));
        assert_eq!(limits(Unit::Cm), (1.06, 211.66));
        assert_eq!(limits(Unit::Mm), (10.59, 2116.66));
        let es = Texts::new(gafira_ports::i18n::Locale::Es);
        assert_eq!(grouped(8000.0, &es), "8.000");
        assert_eq!(grouped(2116.66, &es), "2.116,66");
        assert_eq!(
            grouped(0.42, &Texts::new(gafira_ports::i18n::Locale::En)),
            "0.42"
        );
        let mut custom = Custom {
            width: "39".to_owned(),
            height: "500".to_owned(),
            unit: Unit::Px,
            locked: None,
        };
        assert!(custom.page().is_none());
        custom.width = "40".to_owned();
        assert!(custom.page().is_some());
        custom.height = "8001".to_owned();
        assert!(custom.page().is_none());
        let message = es.t(&gafira_doc::msg!(
            "create.size_limits",
            min = Text("40 px".to_owned()),
            max = Text("8.000 px".to_owned())
        ));
        assert_eq!(
            message,
            "El tamaño no puede ser menor a 40 px ni mayor a 8.000 px."
        );
    }
}
