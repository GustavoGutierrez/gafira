//! «Redimensionar» (spec 108): the panel of the top bar that changes the
//! size of the design's pages, with their content fitted, in the design
//! itself or in a copy; and the edit that does it.

use egui::{Align, Align2, Layout, RichText, Sense, Stroke, Vec2};
use gafira_doc::msg::{Message, MessageKey};
use gafira_doc::{Document, PageId, PageSize};
use gafira_geom::Unit;

use crate::create::{self, Custom};
use crate::design_sizes::{self, Category, DesignSize};
use crate::icons::{self, Icon, Weight};
use crate::style::Theme;
use crate::text::Texts;
use crate::widgets;

/// The sizes of «Sugerencias».
const SUGGESTED: [&str; 3] = ["size.a4_v", "size.pres_16_9", "size.ig_story"];

/// The categories of «Busca por categoría», after «Elegir el tamaño».
const CATEGORIES: [Category; 7] = [
    Category::Social,
    Category::Print,
    Category::Presentations,
    Category::Video,
    Category::Documents,
    Category::Photos,
    Category::Web,
];

/// The width of the panel, in px.
pub(crate) const WIDTH: f32 = 380.0;

/// What the panel shows under its search field.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum View {
    Main,
    Category(Category),
    Custom,
}

/// The size chosen: one of the catalogue or the custom one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Choice {
    Size(&'static str),
    Custom,
}

/// What the panel asks for: the new size, of every page or of one, in the
/// design or in a copy.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Resize {
    pub(crate) size: PageSize,
    /// Every page when `None`; else the one page.
    pub(crate) page: Option<PageId>,
    pub(crate) copy: bool,
}

/// The state of the panel while the editor is open.
pub(crate) struct Panel {
    query: String,
    view: View,
    chosen: Option<Choice>,
    /// The platform or kind chosen in a category, `None` for «Todo».
    group: Option<&'static str>,
    custom: Custom,
    every_page: bool,
}

impl Default for Panel {
    fn default() -> Self {
        Self {
            query: String::new(),
            view: View::Main,
            chosen: None,
            group: None,
            custom: Custom {
                width: String::new(),
                height: String::new(),
                unit: Unit::Px,
                locked: None,
            },
            every_page: true,
        }
    }
}

/// The text of `key`.
fn t(texts: &Texts, key: &'static str) -> String {
    texts.t(&Message {
        key: MessageKey(key),
        args: Vec::new(),
    })
}

impl Panel {
    /// The size chosen, when it is one Gafira can make.
    fn target(&self) -> Option<PageSize> {
        match self.chosen? {
            Choice::Size(key) => design_sizes::by_key(key).map(DesignSize::page_size),
            Choice::Custom => self.custom.page().map(|page| page.size),
        }
    }

    /// Chooses `choice`, or drops it when it was the one chosen.
    fn toggle(&mut self, choice: Choice) {
        self.chosen = if self.chosen == Some(choice) {
            None
        } else {
            Some(choice)
        };
    }

    /// The custom fields with `size`, in its own unit, when they are empty.
    fn fill_custom(&mut self, size: PageSize, texts: &Texts) {
        if !self.custom.width.trim().is_empty() || !self.custom.height.trim().is_empty() {
            return;
        }
        let unit = size.width.unit();
        let height = size
            .height
            .convert(unit, f64::from(gafira_doc::Dpi::DEFAULT.get()));
        let Ok(height) = height else {
            return;
        };
        self.custom.unit = if create::UNITS.contains(&unit) {
            unit
        } else {
            Unit::Px
        };
        self.custom.width = create::written(size.width.value(), texts);
        self.custom.height = create::written(height.value(), texts);
    }

    /// Draws the panel for a design of `pages` pages whose page in view is
    /// `page` of `current` size; what it asks, when a button is pressed.
    pub(crate) fn show(
        &mut self,
        ui: &mut egui::Ui,
        theme: &Theme,
        texts: &Texts,
        (page, current, pages): (PageId, PageSize, usize),
    ) -> Option<Resize> {
        ui.set_width(WIDTH);
        ui.spacing_mut().item_spacing.y = 6.0;
        let search = ui.add(
            widgets::field(&mut self.query, t(texts, "resize.search")).desired_width(f32::INFINITY),
        );
        if search.changed() && !self.query.trim().is_empty() {
            self.view = View::Main;
        }
        ui.add_space(6.0);
        egui::ScrollArea::vertical()
            .id_salt("resize.body")
            .max_height(380.0)
            .auto_shrink([false, true])
            .show(ui, |ui| {
                if !self.query.trim().is_empty() {
                    self.results(ui, theme, texts);
                    return;
                }
                match self.view {
                    View::Main => self.main(ui, theme, texts, current),
                    View::Category(category) => self.category(ui, theme, texts, category),
                    View::Custom => self.custom_size(ui, theme, texts),
                }
            });
        ui.separator();
        self.footer(ui, theme, texts, (page, pages))
    }

    /// The sizes found for the search.
    fn results(&mut self, ui: &mut egui::Ui, theme: &Theme, texts: &Texts) {
        let found = design_sizes::search(&self.query, |s| t(texts, s.key));
        if found.is_empty() {
            ui.label(
                RichText::new(t(texts, "resize.nothing_found"))
                    .font(theme.font("body-sm"))
                    .color(theme.c("text.muted")),
            );
        }
        self.grid(ui, theme, texts, &found);
    }

    /// «Sugerencias» and «Busca por categoría».
    fn main(&mut self, ui: &mut egui::Ui, theme: &Theme, texts: &Texts, current: PageSize) {
        title(ui, theme, &t(texts, "resize.suggestions"));
        let gap = 8.0;
        let side = ((ui.available_width() - 2.0 * gap) / 3.0).max(60.0);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = gap;
            for key in SUGGESTED {
                let Some(size) = design_sizes::by_key(key) else {
                    continue;
                };
                self.suggestion(ui, theme, texts, size, side);
            }
        });
        ui.add_space(8.0);
        title(ui, theme, &t(texts, "resize.by_category"));
        if row(
            ui,
            theme,
            RowIcon::Icon(Icon::Expand),
            &t(texts, "resize.custom"),
            true,
        )
        .clicked()
        {
            self.fill_custom(current, texts);
            self.view = View::Custom;
        }
        for category in CATEGORIES {
            let colour = crate::colors::hex(category.colour());
            let colour = crate::canvas::screen_color(colour);
            if row(
                ui,
                theme,
                RowIcon::Dot(colour),
                &t(texts, category.key()),
                true,
            )
            .clicked()
            {
                self.view = View::Category(category);
                self.group = None;
            }
        }
    }

    /// A card of «Sugerencias»: the picture of the size, its name and size.
    fn suggestion(
        &mut self,
        ui: &mut egui::Ui,
        theme: &Theme,
        texts: &Texts,
        size: &'static DesignSize,
        side: f32,
    ) {
        let chosen = self.chosen == Some(Choice::Size(size.key));
        if size_card(ui, theme, texts, size, side, chosen) {
            self.toggle(Choice::Size(size.key));
        }
    }

    /// `sizes` as cards, three to a row; a click chooses one.
    fn grid(
        &mut self,
        ui: &mut egui::Ui,
        theme: &Theme,
        texts: &Texts,
        sizes: &[&'static DesignSize],
    ) {
        let gap = 8.0;
        let width = ((ui.available_width() - 2.0 * gap) / 3.0).max(60.0);
        for chunk in sizes.chunks(3) {
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = gap;
                for size in chunk {
                    self.suggestion(ui, theme, texts, size, width);
                }
            });
            ui.add_space(6.0);
        }
    }

    /// The sizes of `category`: «Todo» and its platforms or kinds as chips
    /// in a row that scrolls sideways, then its sizes as cards.
    fn category(&mut self, ui: &mut egui::Ui, theme: &Theme, texts: &Texts, category: Category) {
        if back(ui, theme, &t(texts, category.key())) {
            self.view = View::Main;
        }
        let groups = design_sizes::groups(category);
        if !groups.is_empty() {
            egui::ScrollArea::horizontal()
                .id_salt("resize.groups")
                .auto_shrink([false, true])
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 8.0;
                        let all = t(texts, "create.all");
                        if create::pill(ui, theme, self.group.is_none(), &all).clicked() {
                            self.group = None;
                        }
                        for (group, key) in groups {
                            let on = self.group == Some(*group);
                            if create::pill(ui, theme, on, &t(texts, key)).clicked() {
                                self.group = Some(group);
                            }
                        }
                    });
                });
            ui.add_space(8.0);
        }
        let sizes = design_sizes::of(category, self.group);
        self.grid(ui, theme, texts, &sizes);
    }

    /// «Elegir el tamaño»: its check box, width, height, unit and lock.
    fn custom_size(&mut self, ui: &mut egui::Ui, theme: &Theme, texts: &Texts) {
        if back(ui, theme, &t(texts, "resize.custom")) {
            self.view = View::Main;
        }
        let gap = 8.0;
        let check_w = 22.0;
        let lock_w = 28.0;
        let field_w = ((ui.available_width() - check_w - lock_w - 4.0 * gap) / 3.0).max(60.0);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = gap;
            ui.add_space(check_w);
            for key in ["create.width", "create.height", "create.units"] {
                let (rect, _) = ui.allocate_exact_size(Vec2::new(field_w, 16.0), Sense::hover());
                ui.painter().text(
                    rect.left_center(),
                    Align2::LEFT_CENTER,
                    t(texts, key),
                    theme.font("caption"),
                    theme.c("text.secondary"),
                );
            }
        });
        let height = 32.0;
        let mut on = self.chosen == Some(Choice::Custom);
        ui.allocate_ui_with_layout(
            Vec2::new(ui.available_width(), height),
            Layout::left_to_right(Align::Center),
            |ui| {
                ui.spacing_mut().item_spacing.x = gap;
                if ui.checkbox(&mut on, "").changed() {
                    self.toggle(Choice::Custom);
                }
                let mut typed = false;
                let mut field = |ui: &mut egui::Ui, text: &mut String| {
                    let changed = ui
                        .add_sized(
                            Vec2::new(field_w, height),
                            egui::TextEdit::singleline(text).vertical_align(Align::Center),
                        )
                        .changed();
                    typed |= changed;
                    changed
                };
                let width_changed = field(ui, &mut self.custom.width);
                let height_changed = field(ui, &mut self.custom.height);
                if let Some(ratio) = self.custom.locked {
                    if width_changed && let Some(w) = create::number(&self.custom.width) {
                        self.custom.height = create::written(w / ratio, texts);
                    } else if height_changed && let Some(h) = create::number(&self.custom.height) {
                        self.custom.width = create::written(h * ratio, texts);
                    }
                }
                // A unit as a cycling button: a list would open outside the
                // panel and close it.
                let unit = self.custom.unit;
                let button = egui::Button::new(
                    RichText::new(format!("{}  ⌄", create::unit_name(unit)))
                        .font(theme.font("body")),
                )
                .min_size(Vec2::new(field_w, height));
                if ui.add(button).clicked() {
                    let at = create::UNITS.iter().position(|u| *u == unit).unwrap_or(0);
                    self.custom.unit = create::UNITS[(at + 1) % create::UNITS.len()];
                    typed = true;
                }
                let closed = self.custom.locked.is_some();
                let icon = if closed { Icon::Locked } else { Icon::Unlocked };
                let tint = if closed {
                    theme.c("text")
                } else {
                    theme.c("text.muted")
                };
                let lock =
                    egui::Button::image(icons::image(ui.ctx(), icon, Weight::Regular, 18.0, tint))
                        .frame(false)
                        .min_size(Vec2::new(lock_w, height));
                let hint = if closed {
                    "create.unlock_ratio"
                } else {
                    "create.lock_ratio"
                };
                if ui.add(lock).on_hover_text(t(texts, hint)).clicked() {
                    self.custom.locked = if closed {
                        None
                    } else {
                        create::number(&self.custom.width)
                            .zip(create::number(&self.custom.height))
                            .map(|(w, h)| w / h)
                    };
                }
                // Typing a size chooses it.
                if typed {
                    self.chosen = Some(Choice::Custom);
                }
            },
        );
        if self.chosen == Some(Choice::Custom) && self.custom.page().is_none() {
            ui.label(
                RichText::new(t(texts, "create.out_of_range"))
                    .font(theme.font("caption"))
                    .color(theme.c("error.fg")),
            );
        }
    }

    /// Which pages, and the two buttons.
    fn footer(
        &mut self,
        ui: &mut egui::Ui,
        theme: &Theme,
        texts: &Texts,
        (page, pages): (PageId, usize),
    ) -> Option<Resize> {
        let all = texts.t(&gafira_doc::msg!(
            "resize.all_pages",
            count = Count(pages as u64)
        ));
        ui.horizontal(|ui| {
            ui.selectable_value(
                &mut self.every_page,
                true,
                RichText::new(all).font(theme.font("ui-label")),
            );
            ui.selectable_value(
                &mut self.every_page,
                false,
                RichText::new(t(texts, "resize.this_page")).font(theme.font("ui-label")),
            );
        });
        ui.add_space(4.0);
        let target = self.target();
        let mut asked = None;
        let full = Vec2::new(ui.available_width(), 36.0);
        ui.add_enabled_ui(target.is_some(), |ui| {
            ui.spacing_mut().item_spacing.y = 6.0;
            let copy = ui.add_sized(
                full,
                egui::Button::new(
                    RichText::new(t(texts, "resize.copy")).font(theme.font("ui-label")),
                )
                .fill(theme.c("secondary.bg")),
            );
            let resize = ui.add_sized(
                full,
                egui::Button::new(
                    RichText::new(t(texts, "resize.apply"))
                        .font(theme.font("ui-label"))
                        .color(theme.c("primary.fg")),
                )
                .fill(theme.c("primary.bg")),
            );
            if let Some(size) = target {
                let page = (!self.every_page).then_some(page);
                if copy.clicked() {
                    asked = Some(Resize {
                        size,
                        page,
                        copy: true,
                    });
                } else if resize.clicked() {
                    asked = Some(Resize {
                        size,
                        page,
                        copy: false,
                    });
                }
            }
        });
        asked
    }
}

/// A section title of the panel.
fn title(ui: &mut egui::Ui, theme: &Theme, text: &str) {
    ui.label(
        RichText::new(text)
            .font(theme.font("ui-label"))
            .color(theme.c("text.secondary")),
    );
}

/// What a row of «Busca por categoría» shows before its name.
enum RowIcon {
    Icon(Icon),
    Dot(egui::Color32),
}

/// A row of «Busca por categoría»: its icon, its name and, when it opens
/// a list, an arrow.
fn row(ui: &mut egui::Ui, theme: &Theme, icon: RowIcon, text: &str, opens: bool) -> egui::Response {
    let (rect, response) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), 36.0), Sense::click());
    if response.hovered() {
        ui.painter()
            .rect_filled(rect, theme.radius_md(), theme.c("surface.sunken"));
    }
    let left = egui::Pos2::new(rect.left() + 18.0, rect.center().y);
    match icon {
        RowIcon::Icon(icon) => {
            let image = icons::image(ui.ctx(), icon, Weight::Regular, 18.0, theme.c("text"));
            image.paint_at(ui, egui::Rect::from_center_size(left, Vec2::splat(18.0)));
        }
        RowIcon::Dot(colour) => {
            ui.painter().rect_filled(
                egui::Rect::from_center_size(left, Vec2::splat(16.0)),
                4.0,
                colour,
            );
        }
    }
    ui.painter().text(
        egui::Pos2::new(rect.left() + 40.0, rect.center().y),
        Align2::LEFT_CENTER,
        text,
        theme.font("body"),
        theme.c("text"),
    );
    if opens {
        ui.painter().text(
            egui::Pos2::new(rect.right() - 12.0, rect.center().y),
            Align2::RIGHT_CENTER,
            "›",
            theme.font("body"),
            theme.c("text.secondary"),
        );
    }
    response
}

/// «‹ name», the way back to the main view; whether it was clicked.
fn back(ui: &mut egui::Ui, theme: &Theme, text: &str) -> bool {
    let clicked = ui
        .horizontal(|ui| {
            let arrow = widgets::back_arrow(ui);
            let label = ui.add(
                egui::Label::new(
                    RichText::new(text)
                        .font(theme.font("ui-label"))
                        .color(theme.c("text")),
                )
                .sense(Sense::click()),
            );
            arrow.clicked() || label.clicked()
        })
        .inner;
    ui.add_space(4.0);
    clicked
}

/// One size as a card: its picture on a soft tile, its name and size
/// under it, a check mark when `chosen`; `true` when clicked.
fn size_card(
    ui: &mut egui::Ui,
    theme: &Theme,
    texts: &Texts,
    size: &'static DesignSize,
    width: f32,
    chosen: bool,
) -> bool {
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, width + 40.0), Sense::click());
    let tile = egui::Rect::from_min_size(rect.min, Vec2::splat(width));
    let fill = if response.hovered() {
        theme.c("secondary.bg.hover")
    } else {
        theme.c("surface.sunken")
    };
    ui.painter().rect_filled(tile, 10.0, fill);
    if chosen || response.hovered() {
        let rim = if chosen {
            Stroke::new(2.0, theme.c("primary.bg"))
        } else {
            Stroke::new(1.5, theme.c("accent"))
        };
        ui.painter()
            .rect_stroke(tile, 10.0, rim, egui::StrokeKind::Inside);
    }
    // The picture of the size, 240 × 170 in its own units.
    let art_w = tile.width() - 12.0;
    let art = egui::Rect::from_center_size(tile.center(), Vec2::new(art_w, art_w * 170.0 / 240.0));
    let side = (art_w * ui.ctx().pixels_per_point()).round() as u32;
    let key = format!("card/{}", size.key);
    if let Some(texture) = crate::design_art::texture(ui.ctx(), &key, side, || {
        crate::design_art::card_svg(size.look, size.aspect(), size.key)
    }) {
        let uv = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
        ui.painter()
            .image(texture.id(), art, uv, egui::Color32::WHITE);
    }
    if chosen {
        check_mark(ui, theme, tile);
    }
    let name = t(texts, size.key);
    let painter = ui.painter().with_clip_rect(rect);
    let fits = |text: &str| {
        painter
            .layout_no_wrap(text.to_owned(), theme.font("body-sm"), theme.c("text"))
            .size()
            .x
            <= width
    };
    let shown = if fits(&name) {
        name.clone()
    } else {
        let mut cut = name.clone();
        while !cut.is_empty() && !fits(&format!("{cut}…")) {
            cut.pop();
        }
        format!("{cut}…")
    };
    painter.text(
        egui::pos2(tile.left(), tile.bottom() + 6.0),
        Align2::LEFT_TOP,
        shown,
        theme.font("body-sm"),
        theme.c("text"),
    );
    painter.text(
        egui::pos2(tile.left(), tile.bottom() + 24.0),
        Align2::LEFT_TOP,
        size.label(texts.locale),
        theme.font("caption"),
        theme.c("text.muted"),
    );
    if response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    response.on_hover_text(name).clicked()
}

/// A round check mark in the top-right corner of `rect`.
fn check_mark(ui: &egui::Ui, theme: &Theme, rect: egui::Rect) {
    let centre = egui::Pos2::new(rect.right() - 12.0, rect.top() + 12.0);
    ui.painter()
        .circle_filled(centre, 9.0, theme.c("primary.bg"));
    let image = icons::image(
        ui.ctx(),
        Icon::Check,
        Weight::Regular,
        12.0,
        theme.c("primary.fg"),
    );
    image.paint_at(ui, egui::Rect::from_center_size(centre, Vec2::splat(12.0)));
}

/// The pages of `document` at `size`, each with its content fitted: every
/// page, or `page` alone (S108-04, S108-06). `None` when a page cannot be
/// measured or there is none.
pub(crate) fn resized_pages(
    document: &Document,
    page: Option<PageId>,
    size: PageSize,
) -> Option<Vec<gafira_doc::Page>> {
    let pages: Vec<gafira_doc::Page> = document
        .pages()
        .iter()
        .filter(|p| page.is_none_or(|id| p.id == id))
        .map(|p| crate::template_fit::resized_page(p, document.dpi(), size))
        .collect::<Option<_>>()?;
    (!pages.is_empty()).then_some(pages)
}

/// The edit that resizes `document` as `resize` asks, in one undo step.
pub(crate) fn command(document: &Document, resize: Resize) -> Option<gafira_edit::ResizePages> {
    resized_pages(document, resize.page, resize.size).map(gafira_edit::ResizePages::new)
}
