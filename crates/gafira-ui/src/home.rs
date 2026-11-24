//! Inicio and Plantillas (03 §7.2, spec 091): one header with the title,
//! the two tabs and a search, and under it either Inicio (the shortcuts to
//! «Crear un diseño», «Recientes» and the templates by kind) or Plantillas
//! (categories, kinds and every template, loaded as the grid scrolls). A
//! search on Inicio finds design types and templates.

use std::collections::BTreeMap;
use std::path::PathBuf;

use egui::{Align, Color32, Layout, Rect, RichText, Sense, Stroke, Vec2};
use gafira_doc::msg::{Message, MessageKey};
use gafira_ports::i18n::Locale;

use crate::create::{self, Tab, Thumbs};
use crate::design_art;
use crate::design_sizes::{self, Category, DesignSize};
use crate::icons::{self, Icon, Weight};
use crate::recent::{self, RecentProject, RecentThumbnail};
use crate::shell::Go;
use crate::style::{DISPLAY, Theme};
use crate::templates::{self, Catalogue, Filter, Template};
use crate::text::Texts;

/// The widest the header's search gets.
const SEARCH_WIDTH: f32 = 680.0;
/// The height of a template card's picture in a grid and in a row.
const CARD_HEIGHT: f32 = 190.0;
/// The width of a shortcut.
const SHORTCUT_WIDTH: f32 = 104.0;
/// The space around the content.
const SIDE: f32 = 32.0;

/// The shortcuts under the search, in their order: Plantillas, the
/// categories of «Crear un diseño», then Abrir.
const SHORTCUTS: [Category; 8] = [
    Category::Social,
    Category::Presentations,
    Category::Video,
    Category::Print,
    Category::Documents,
    Category::Photos,
    Category::Web,
    Category::Custom,
];

/// The state of Inicio and Plantillas.
pub(crate) struct Home {
    tab: Tab,
    query: String,
    /// The category and kind Plantillas is narrowed to.
    filter: Filter,
    /// How many pages of the grid of Plantillas are shown.
    pages: usize,
    recent: Vec<RecentProject>,
    thumbs: BTreeMap<PathBuf, RecentThumbnail>,
    /// The texts in both languages, for a search in either.
    both: [Texts; 2],
    /// The template whose details are open, if any (spec 091).
    detail: Option<PathBuf>,
}

/// The text of `key`.
fn t(texts: &Texts, key: &'static str) -> String {
    texts.t(&Message {
        key: MessageKey(key),
        args: Vec::new(),
    })
}

impl Home {
    /// Inicio, or Plantillas on the templates of `kind` when `tab` says so.
    pub(crate) fn new(
        recent: Vec<RecentProject>,
        thumbs: BTreeMap<PathBuf, RecentThumbnail>,
        tab: Tab,
        kind: Option<&'static str>,
    ) -> Self {
        Self {
            tab,
            query: String::new(),
            filter: Filter {
                kind,
                ..Filter::default()
            },
            pages: 1,
            recent,
            thumbs,
            both: [Texts::new(Locale::Es), Texts::new(Locale::En)],
            detail: None,
        }
    }

    pub(crate) fn show(
        &mut self,
        ctx: &egui::Context,
        theme: &Theme,
        texts: &Texts,
        now: i64,
        textures: &mut Thumbs,
        catalogue: &mut Catalogue,
    ) -> Go {
        textures.keep_only(self.thumbs.keys().map(PathBuf::as_path));
        catalogue.pump(ctx);
        let mut go = create::start_topbar(ctx, theme, texts, Some(self.tab)).unwrap_or(Go::Stay);
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(theme.c("bg")))
            .show(ctx, |ui| {
                egui::ScrollArea::vertical()
                    .id_salt("start.scroll")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        self.header(ui, theme, texts);
                        ui.add_space(8.0);
                        let next = side_margins(ui, |ui| match self.tab {
                            Tab::Home => self.home(ui, theme, texts, now, textures, catalogue),
                            Tab::Templates => self.templates(ui, theme, texts, catalogue),
                            Tab::Brand => None,
                        });
                        match next {
                            // A template opens its details first.
                            Some(Go::FromTemplate(path)) => self.detail = Some(path),
                            Some(next) => go = next,
                            None => {}
                        }
                        ui.add_space(40.0);
                    });
            });
        if self.detail.is_some() {
            if let Some(next) = self.detail_modal(ctx, theme, texts, catalogue) {
                go = next;
            }
            return go;
        }
        if let Some(next) = create::start_keys(ctx) {
            go = next;
        }
        go
    }

    /// The details of a template over the screen: its picture, its name,
    /// kind and size, «Personalizar esta plantilla», and the templates like
    /// it; a click on one of them shows it here. Esc, the cross or a click
    /// outside closes it.
    fn detail_modal(
        &mut self,
        ctx: &egui::Context,
        theme: &Theme,
        texts: &Texts,
        catalogue: &mut Catalogue,
    ) -> Option<Go> {
        let path = self.detail.clone()?;
        let all = catalogue.list().map(<[Template]>::to_vec)?;
        let Some(current) = all.iter().find(|t| t.path == path).cloned() else {
            self.detail = None;
            return None;
        };
        let similar: Vec<Template> = templates::similar(&all, &current, texts.locale, 12)
            .into_iter()
            .cloned()
            .collect();
        let mut go = None;
        let mut close = ctx.input(|i| i.key_pressed(egui::Key::Escape));
        let screen = ctx.content_rect();
        // The dim behind it; a click on it closes.
        egui::Area::new(egui::Id::new("template.detail.dim"))
            .order(egui::Order::Foreground)
            .fixed_pos(screen.min)
            .show(ctx, |ui| {
                let (rect, response) = ui.allocate_exact_size(screen.size(), Sense::click());
                ui.painter()
                    .rect_filled(rect, 0.0, Color32::from_black_alpha(140));
                if response.clicked() {
                    close = true;
                }
            });
        let width = (screen.width() - 120.0).clamp(600.0, 1100.0);
        let height = (screen.height() - 100.0).clamp(420.0, 780.0);
        let window = Rect::from_center_size(screen.center(), Vec2::new(width, height));
        egui::Area::new(egui::Id::new("template.detail"))
            .order(egui::Order::Tooltip)
            .fixed_pos(window.min)
            .show(ctx, |ui| {
                egui::Frame::NONE
                    .fill(theme.c("surface"))
                    .corner_radius(16.0)
                    .inner_margin(egui::Margin::same(28))
                    .show(ui, |ui| {
                        ui.set_width(width - 56.0);
                        ui.set_height(height - 56.0);
                        // The cross.
                        let cross = Rect::from_min_size(
                            egui::pos2(window.right() - 52.0, window.top() + 16.0),
                            Vec2::splat(32.0),
                        );
                        let response = ui.interact(cross, ui.id().with("close"), Sense::click());
                        if response.hovered() {
                            ui.painter().circle_filled(
                                cross.center(),
                                16.0,
                                theme.c("secondary.bg"),
                            );
                        }
                        let c = cross.center();
                        let stroke = Stroke::new(2.0, theme.c("text"));
                        ui.painter().line_segment(
                            [c + Vec2::new(-6.0, -6.0), c + Vec2::new(6.0, 6.0)],
                            stroke,
                        );
                        ui.painter().line_segment(
                            [c + Vec2::new(6.0, -6.0), c + Vec2::new(-6.0, 6.0)],
                            stroke,
                        );
                        if response
                            .on_hover_cursor(egui::CursorIcon::PointingHand)
                            .clicked()
                        {
                            close = true;
                        }
                        let top = (height - 56.0) * 0.62;
                        ui.horizontal_top(|ui| {
                            ui.spacing_mut().item_spacing.x = 28.0;
                            // The picture, whole.
                            let picture_w = (width - 56.0) * 0.58;
                            let (tile, _) =
                                ui.allocate_exact_size(Vec2::new(picture_w, top), Sense::hover());
                            ui.painter()
                                .rect_filled(tile, 12.0, theme.c("surface.sunken"));
                            if let Some((texture, (w, h))) = catalogue.thumbnail(&current) {
                                let (fw, fh) =
                                    recent::fit(w, h, (tile.width() - 32.0, tile.height() - 32.0));
                                ui.painter().image(
                                    texture,
                                    Rect::from_center_size(tile.center(), Vec2::new(fw, fh)),
                                    Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                                    Color32::WHITE,
                                );
                            }
                            ui.vertical(|ui| {
                                ui.set_width((width - 56.0) - picture_w - 28.0 - 40.0);
                                ui.add_space(8.0);
                                ui.add(
                                    egui::Label::new(
                                        RichText::new(&current.name)
                                            .font(theme.font("h2"))
                                            .color(theme.c("text")),
                                    )
                                    .wrap(),
                                );
                                ui.add_space(10.0);
                                if let Some(kind) = current.kind {
                                    ui.label(
                                        RichText::new(t(texts, kind.key))
                                            .font(theme.font("body"))
                                            .color(theme.c("text.secondary")),
                                    );
                                    ui.label(
                                        RichText::new(kind.label(texts.locale))
                                            .font(theme.font("body-sm"))
                                            .color(theme.c("text.muted")),
                                    );
                                }
                                if let Some(category) = current.category() {
                                    ui.add_space(10.0);
                                    create::pill(ui, theme, false, &t(texts, category.key()));
                                }
                                ui.add_space(24.0);
                                let (button, response) = ui.allocate_exact_size(
                                    Vec2::new(ui.available_width(), 46.0),
                                    Sense::click(),
                                );
                                let fill = if response.hovered() {
                                    theme.c("primary.bg.hover")
                                } else {
                                    theme.c("primary.bg")
                                };
                                ui.painter().rect_filled(button, theme.radius_md(), fill);
                                ui.painter().text(
                                    button.center(),
                                    egui::Align2::CENTER_CENTER,
                                    t(texts, "templates.use"),
                                    theme.font("ui-label"),
                                    theme.c("primary.fg"),
                                );
                                if response
                                    .on_hover_cursor(egui::CursorIcon::PointingHand)
                                    .clicked()
                                {
                                    go = Some(Go::FromTemplate(current.path.clone()));
                                }
                                ui.add_space(10.0);
                                ui.label(
                                    RichText::new(t(texts, "templates.use_hint"))
                                        .font(theme.font("caption"))
                                        .color(theme.c("text.muted")),
                                );
                            });
                        });
                        ui.add_space(22.0);
                        ui.label(
                            RichText::new(t(texts, "templates.similar"))
                                .font(theme.font("ui-label"))
                                .color(theme.c("text")),
                        );
                        ui.add_space(10.0);
                        if similar.is_empty() {
                            muted(ui, theme, &t(texts, "templates.no_similar"));
                            return;
                        }
                        let row_h = (ui.available_height() - 40.0).clamp(80.0, 200.0);
                        egui::ScrollArea::horizontal()
                            .id_salt("template.detail.similar")
                            .auto_shrink([false, true])
                            .show(ui, |ui| {
                                ui.horizontal_top(|ui| {
                                    ui.spacing_mut().item_spacing.x = 14.0;
                                    for other in &similar {
                                        if template_card(ui, theme, catalogue, other, row_h) {
                                            self.detail = Some(other.path.clone());
                                        }
                                    }
                                });
                            });
                    });
            });
        if close {
            self.detail = None;
        }
        if go.is_some() {
            self.detail = None;
        }
        go
    }

    /// The band on top: the title, the tabs and the search.
    fn header(&mut self, ui: &mut egui::Ui, theme: &Theme, texts: &Texts) {
        let width = ui.available_width();
        let (band, _) = ui.allocate_exact_size(Vec2::new(width, 236.0), Sense::hover());
        soft_band(ui, theme, band);
        let mut child = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(band.shrink2(Vec2::new(SIDE, 34.0)))
                .layout(Layout::top_down(Align::Center)),
        );
        let ui = &mut child;
        ui.label(
            RichText::new(t(texts, "home.title"))
                .font(egui::FontId::new(
                    34.0,
                    egui::FontFamily::Name(DISPLAY.into()),
                ))
                .color(theme.c("text")),
        );
        ui.add_space(16.0);
        let tabs = [
            (Tab::Home, Icon::Home, "nav.home"),
            (Tab::Templates, Icon::Templates, "nav.templates"),
        ];
        let row = centred(ui, 260.0, 34.0);
        let mut tabs_ui = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(row)
                .layout(Layout::left_to_right(Align::Center)),
        );
        tabs_ui.spacing_mut().item_spacing.x = 8.0;
        for (tab, icon, key) in tabs {
            if tab_pill(&mut tabs_ui, theme, self.tab == tab, icon, &t(texts, key)).clicked()
                && self.tab != tab
            {
                self.tab = tab;
                self.pages = 1;
            }
        }
        ui.add_space(row.height() + 18.0);
        let hint = match self.tab {
            Tab::Home => "home.search_hint",
            Tab::Templates | Tab::Brand => "templates.search_hint",
        };
        let rect = centred(ui, SEARCH_WIDTH.min(ui.available_width()), 48.0);
        let mut search = ui.new_child(egui::UiBuilder::new().max_rect(rect));
        if search_field(&mut search, theme, &mut self.query, t(texts, hint)) {
            self.pages = 1;
        }
    }

    /// Inicio under the header: the search results, or the shortcuts,
    /// «Recientes» and the templates by kind.
    fn home(
        &mut self,
        ui: &mut egui::Ui,
        theme: &Theme,
        texts: &Texts,
        now: i64,
        textures: &mut Thumbs,
        catalogue: &mut Catalogue,
    ) -> Option<Go> {
        if !self.query.trim().is_empty() {
            return self.home_results(ui, theme, texts, catalogue);
        }
        let mut go = self.shortcuts(ui, theme, texts);
        ui.add_space(30.0);
        if let Some(path) = self.recents(ui, theme, texts, now, textures) {
            go = Some(Go::OpenRecent(path));
        }
        ui.add_space(30.0);
        let Some(all) = catalogue.list().map(<[Template]>::to_vec) else {
            muted(ui, theme, &t(texts, "templates.loading"));
            return go;
        };
        let mine = templates::filtered(&all, &Filter::default(), texts.locale, |_| String::new());
        for (kind, list) in templates::by_kind(&mine) {
            let title = t(texts, kind.key);
            if heading_with_more(ui, theme, &title, &t(texts, "home.see_all")) {
                go = Some(Go::Templates(Some(kind.key)));
            }
            if let Some(path) = template_row(ui, theme, catalogue, kind.key, &list) {
                go = Some(Go::FromTemplate(path));
            }
            ui.add_space(26.0);
        }
        go
    }

    /// What a search on Inicio finds: design types, then templates.
    fn home_results(
        &mut self,
        ui: &mut egui::Ui,
        theme: &Theme,
        texts: &Texts,
        catalogue: &mut Catalogue,
    ) -> Option<Go> {
        let mut go = None;
        // Either language finds a design type, as it finds a template.
        let sizes = design_sizes::search(&self.query, |s| {
            format!("{} {}", t(&self.both[0], s.key), t(&self.both[1], s.key))
        });
        let found = catalogue.list().map(|all| {
            let filter = Filter {
                query: self.query.clone(),
                ..Filter::default()
            };
            templates::filtered(all, &filter, texts.locale, |tpl| names(&self.both, tpl))
                .into_iter()
                .cloned()
                .collect::<Vec<_>>()
        });
        if sizes.is_empty() && found.as_ref().is_some_and(Vec::is_empty) {
            create::heading(ui, theme, &t(texts, "create.results"));
            muted(ui, theme, &t(texts, "create.no_results"));
            return None;
        }
        if !sizes.is_empty() {
            create::heading(ui, theme, &t(texts, "home.design_types"));
            if let Some(size) = create::grid(ui, theme, texts, &sizes) {
                go = Some(Go::Create(Box::new(size.page())));
            }
            ui.add_space(20.0);
        }
        if let Some(found) = found
            && !found.is_empty()
        {
            create::heading(ui, theme, &t(texts, "nav.templates"));
            let refs: Vec<&Template> = found.iter().collect();
            if let Some(path) = self.template_grid(ui, theme, texts, catalogue, &refs) {
                go = Some(Go::FromTemplate(path));
            }
        }
        go
    }

    /// The shortcuts: Plantillas, each category of «Crear un diseño», Abrir.
    fn shortcuts(&mut self, ui: &mut egui::Ui, theme: &Theme, texts: &Texts) -> Option<Go> {
        let mut go = None;
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = Vec2::new(10.0, 14.0);
            // Centred when they fit on one line.
            let count = SHORTCUTS.len() as f32 + 3.0;
            let row = count * SHORTCUT_WIDTH + (count - 1.0) * 10.0;
            let room = ui.available_width();
            if row < room {
                ui.add_space((room - row) / 2.0 - 10.0);
            }
            let templates = shortcut(ui, theme, &t(texts, "nav.templates"), |ui, rect| {
                icons::paint(
                    ui.painter(),
                    rect,
                    Icon::Templates,
                    Weight::Regular,
                    theme.c("text"),
                );
            });
            if templates.clicked() {
                self.tab = Tab::Templates;
                self.pages = 1;
            }
            for category in SHORTCUTS {
                let clicked = shortcut(ui, theme, &t(texts, category.key()), |ui, rect| {
                    category_icon(ui, category, rect);
                })
                .clicked();
                if clicked {
                    go = Some(Go::CreateIn(category));
                }
            }
            let open = shortcut(ui, theme, &t(texts, "home.open"), |ui, rect| {
                icons::paint(
                    ui.painter(),
                    rect,
                    Icon::Export,
                    Weight::Regular,
                    theme.c("text"),
                );
            });
            if open.clicked() {
                go = Some(Go::File(crate::files::FileAction::Open));
            }
            // Spec 097: a `.gafpack`, shown before it is installed.
            let install = shortcut(ui, theme, &t(texts, "packs.install_pack"), |ui, rect| {
                icons::paint(
                    ui.painter(),
                    rect,
                    Icon::Package,
                    Weight::Regular,
                    theme.c("text"),
                );
            });
            if install.clicked() {
                go = Some(Go::InstallPack);
            }
        });
        go
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
        create::heading(ui, theme, &t(texts, "home.recent"));
        let shown = recent::shown(&self.recent);
        if shown.is_empty() {
            muted(ui, theme, &t(texts, "home.empty"));
            return None;
        }
        let mut open = None;
        egui::ScrollArea::horizontal()
            .id_salt("start.recent")
            .auto_shrink([false, true])
            .show(ui, |ui| {
                ui.horizontal_top(|ui| {
                    ui.spacing_mut().item_spacing.x = 12.0;
                    for project in shown {
                        let picture = self.thumbs.get(&project.path).map(|thumb| {
                            let texture = textures.get(&project.path, thumb, |thumb| {
                                create::upload(ui.ctx(), &project.path, thumb)
                            });
                            (texture.clone(), (thumb.width, thumb.height))
                        });
                        if create::recent_card(ui, theme, texts, project, now, picture.as_ref()) {
                            open = Some(project.path.clone());
                        }
                    }
                });
            });
        open
    }

    /// Plantillas under the header: the categories, the kinds and every
    /// template that the search and the filters keep.
    fn templates(
        &mut self,
        ui: &mut egui::Ui,
        theme: &Theme,
        texts: &Texts,
        catalogue: &mut Catalogue,
    ) -> Option<Go> {
        let Some(all) = catalogue.list().map(<[Template]>::to_vec) else {
            muted(ui, theme, &t(texts, "templates.loading"));
            return None;
        };
        let mine = templates::filtered(&all, &Filter::default(), texts.locale, |_| String::new());
        // The chips: every category some template is of.
        let mut categories: Vec<Category> = Vec::new();
        for category in mine.iter().filter_map(|tpl| tpl.category()) {
            if !categories.contains(&category) {
                categories.push(category);
            }
        }
        categories.sort_by_key(|c| Category::ALL.iter().position(|a| a == c));
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = Vec2::new(8.0, 8.0);
            let all_on = self.filter.category.is_none() && self.filter.kind.is_none();
            if create::pill(ui, theme, all_on, &t(texts, "create.all")).clicked() {
                self.filter = Filter::default();
                self.pages = 1;
            }
            for category in &categories {
                let on = self.filter.category == Some(*category);
                if create::pill(ui, theme, on, &t(texts, category.key())).clicked() {
                    self.filter = Filter {
                        category: Some(*category),
                        ..Filter::default()
                    };
                    self.pages = 1;
                }
            }
        });
        ui.add_space(22.0);
        // «Explorar por tipo»: the kinds of the chosen category.
        if self.filter.kind.is_none() && self.query.trim().is_empty() {
            let of_category = templates::filtered(
                &all,
                &Filter {
                    category: self.filter.category,
                    ..Filter::default()
                },
                texts.locale,
                |_| String::new(),
            );
            let kinds: Vec<&'static DesignSize> = templates::by_kind(&of_category)
                .into_iter()
                .map(|(kind, _)| kind)
                .collect();
            if !kinds.is_empty() {
                create::heading(ui, theme, &t(texts, "templates.explore"));
                if let Some(kind) = kind_cards(ui, theme, texts, &kinds) {
                    self.filter.kind = Some(kind.key);
                    self.pages = 1;
                }
                ui.add_space(26.0);
            }
        }
        // The grid, under the kind chosen if any.
        let title = self
            .filter
            .kind
            .map_or_else(|| t(texts, "templates.all"), |key| t(texts, key));
        let cleared = self.filter.kind.is_some()
            && heading_with_more(ui, theme, &title, &t(texts, "templates.clear"));
        if self.filter.kind.is_none() {
            create::heading(ui, theme, &title);
        }
        if cleared {
            self.filter.kind = None;
            self.pages = 1;
        }
        let filter = Filter {
            query: self.query.clone(),
            ..self.filter.clone()
        };
        let found: Vec<Template> =
            templates::filtered(&all, &filter, texts.locale, |tpl| names(&self.both, tpl))
                .into_iter()
                .cloned()
                .collect();
        if found.is_empty() {
            muted(ui, theme, &t(texts, "templates.none"));
            return None;
        }
        let refs: Vec<&Template> = found.iter().collect();
        let shown = templates::shown(&refs, self.pages);
        let picked = self.template_grid(ui, theme, texts, catalogue, shown);
        // A page more as the end of the grid comes into view.
        if shown.len() < refs.len() {
            let (end, _) =
                ui.allocate_exact_size(Vec2::new(ui.available_width(), 24.0), Sense::hover());
            if ui.is_rect_visible(end) {
                self.pages += 1;
                ui.ctx().request_repaint();
            }
        }
        picked.map(Go::FromTemplate)
    }

    /// `list` as cards of one height that wrap; the one clicked.
    fn template_grid(
        &self,
        ui: &mut egui::Ui,
        theme: &Theme,
        texts: &Texts,
        catalogue: &mut Catalogue,
        list: &[&Template],
    ) -> Option<PathBuf> {
        let _ = texts;
        let mut picked = None;
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = Vec2::new(16.0, 18.0);
            for template in list {
                if template_card(ui, theme, catalogue, template, CARD_HEIGHT) {
                    picked = Some(template.path.clone());
                }
            }
        });
        picked
    }
}

/// The names of a template's kind and category in both languages, so a
/// search finds it whatever language it is typed in.
fn names(both: &[Texts; 2], template: &Template) -> String {
    let mut words = String::new();
    for texts in both {
        if let Some(kind) = template.kind {
            words.push_str(&t(texts, kind.key));
            words.push(' ');
        }
        if let Some(category) = template.category() {
            words.push_str(&t(texts, category.key()));
            words.push(' ');
        }
    }
    words
}

/// Runs `add` with the content's side margins.
fn side_margins<R>(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    egui::Frame::NONE
        .inner_margin(egui::Margin::symmetric(SIDE as i8, 0))
        .show(ui, add)
        .inner
}

/// A rect `width` × `height` at the horizontal centre of `ui`, from its
/// cursor down.
fn centred(ui: &egui::Ui, width: f32, height: f32) -> Rect {
    let room = ui.max_rect();
    let top = ui.cursor().top();
    Rect::from_min_size(
        egui::pos2(room.center().x - width / 2.0, top),
        Vec2::new(width, height),
    )
}

/// The header's band: a soft mint wash that fades into the background.
fn soft_band(ui: &egui::Ui, theme: &Theme, band: Rect) {
    let top = theme.c("selection.bg");
    let bottom = theme.c("bg");
    let mut mesh = egui::Mesh::default();
    mesh.colored_vertex(band.left_top(), top);
    mesh.colored_vertex(band.right_top(), top);
    mesh.colored_vertex(band.right_bottom(), bottom);
    mesh.colored_vertex(band.left_bottom(), bottom);
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(0, 2, 3);
    ui.painter().add(egui::Shape::mesh(mesh));
}

/// The search of the header: a field with a magnifier; `true` when its
/// text changed.
fn search_field(ui: &mut egui::Ui, theme: &Theme, query: &mut String, hint: String) -> bool {
    let mut changed = false;
    egui::Frame::NONE
        .fill(theme.c("surface"))
        .stroke(Stroke::new(1.0, theme.c("border")))
        .corner_radius(theme.radius_md())
        .inner_margin(egui::Margin::symmetric(14, 2))
        .show(ui, |ui| {
            let row = Vec2::new(ui.available_width(), 42.0);
            ui.allocate_ui_with_layout(row, Layout::left_to_right(Align::Center), |ui| {
                let (rect, _) = ui.allocate_exact_size(Vec2::splat(18.0), Sense::hover());
                let color = theme.c("text.secondary");
                let c = rect.center() - Vec2::new(1.5, 1.5);
                ui.painter().circle_stroke(c, 5.5, Stroke::new(1.8, color));
                ui.painter().line_segment(
                    [c + Vec2::splat(4.0), c + Vec2::splat(8.5)],
                    Stroke::new(1.8, color),
                );
                changed = ui
                    .add(
                        egui::TextEdit::singleline(query)
                            .hint_text(hint)
                            .frame(false)
                            .margin(Vec2::new(8.0, 10.0))
                            .desired_width(f32::INFINITY),
                    )
                    .changed();
            });
        });
    changed
}

/// One of the header's tabs: a pill with its icon, lit when on.
fn tab_pill(ui: &mut egui::Ui, theme: &Theme, on: bool, icon: Icon, label: &str) -> egui::Response {
    let (fill, stroke) = if on {
        (theme.c("selection.bg"), Stroke::new(1.5, theme.c("accent")))
    } else {
        (theme.c("surface"), Stroke::new(1.0, theme.c("border")))
    };
    let image = icons::image(ui.ctx(), icon, Weight::Regular, 16.0, theme.c("text"));
    ui.add(
        egui::Button::image_and_text(
            image,
            RichText::new(label)
                .font(theme.font("body-sm"))
                .color(theme.c("text")),
        )
        .fill(fill)
        .stroke(stroke)
        .corner_radius(17.0)
        .min_size(Vec2::new(120.0, 34.0)),
    )
}

/// A muted line, for the empty and loading states.
fn muted(ui: &mut egui::Ui, theme: &Theme, text: &str) {
    ui.label(
        RichText::new(text)
            .font(theme.font("body"))
            .color(theme.c("text.muted")),
    );
}

/// A title with a link on its right; `true` when the link was clicked.
fn heading_with_more(ui: &mut egui::Ui, theme: &Theme, title: &str, more: &str) -> bool {
    let mut clicked = false;
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(title)
                .font(theme.font("h3"))
                .color(theme.c("text")),
        );
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            let link = ui.add(
                egui::Button::new(
                    RichText::new(more)
                        .font(theme.font("ui-label"))
                        .color(theme.c("text")),
                )
                .frame(false),
            );
            clicked = link.clicked();
        });
    });
    ui.add_space(12.0);
    clicked
}

/// The icon of `category`, as «Crear un diseño» draws it.
fn category_icon(ui: &egui::Ui, category: Category, rect: Rect) {
    let side = (rect.width() * ui.ctx().pixels_per_point()).round() as u32;
    let name = format!("cat/{}/{side}", category.key());
    if let Some(texture) =
        design_art::texture(ui.ctx(), &name, side, || design_art::category_svg(category))
    {
        ui.painter().image(
            texture.id(),
            rect,
            Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            Color32::WHITE,
        );
    }
}

/// One shortcut: its icon on a soft tile and its name under it.
fn shortcut(
    ui: &mut egui::Ui,
    theme: &Theme,
    label: &str,
    icon: impl FnOnce(&egui::Ui, Rect),
) -> egui::Response {
    let width = SHORTCUT_WIDTH;
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, 104.0), Sense::click());
    let tile = Rect::from_center_size(
        egui::pos2(rect.center().x, rect.top() + 30.0),
        Vec2::splat(56.0),
    );
    let hover = ui.ctx().animate_bool(response.id, response.hovered());
    let fill = theme
        .c("surface")
        .lerp_to_gamma(theme.c("secondary.bg.hover"), hover);
    ui.painter().rect_filled(tile, 14.0, fill);
    ui.painter().rect_stroke(
        tile,
        14.0,
        Stroke::new(1.0, theme.c("border")),
        egui::StrokeKind::Inside,
    );
    icon(ui, Rect::from_center_size(tile.center(), Vec2::splat(28.0)));
    let galley = ui.painter().layout(
        label.to_owned(),
        theme.font("caption"),
        theme.c("text"),
        width - 4.0,
    );
    let at = egui::pos2(rect.center().x - galley.size().x / 2.0, tile.bottom() + 8.0);
    ui.painter().galley(at, galley, theme.c("text"));
    if response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    response
}

/// The cards of «Explorar por tipo»: the kind's name and its picture on a
/// soft tile; the one clicked.
fn kind_cards(
    ui: &mut egui::Ui,
    theme: &Theme,
    texts: &Texts,
    kinds: &[&'static DesignSize],
) -> Option<&'static DesignSize> {
    let mut picked = None;
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = Vec2::new(14.0, 14.0);
        for kind in kinds {
            let (rect, response) = ui.allocate_exact_size(Vec2::new(214.0, 74.0), Sense::click());
            let hover = ui.ctx().animate_bool(response.id, response.hovered());
            let fill = theme
                .c("surface.sunken")
                .lerp_to_gamma(theme.c("selection.bg"), hover);
            ui.painter().rect_filled(rect, 12.0, fill);
            let art = Rect::from_min_size(
                egui::pos2(rect.right() - 86.0, rect.top() + 8.0),
                Vec2::new(80.0, 58.0),
            );
            let side = (art.width() * ui.ctx().pixels_per_point()).round() as u32;
            let key = format!("card/{}", kind.key);
            if let Some(texture) = design_art::texture(ui.ctx(), &key, side, || {
                design_art::card_svg(kind.look, kind.aspect(), kind.key)
            }) {
                ui.painter().image(
                    texture.id(),
                    art,
                    Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    Color32::WHITE,
                );
            }
            let galley = ui.painter().layout(
                t(texts, kind.key),
                theme.font("ui-label"),
                theme.c("text"),
                rect.width() - 104.0,
            );
            let at = egui::pos2(rect.left() + 14.0, rect.center().y - galley.size().y / 2.0);
            ui.painter().galley(at, galley, theme.c("text"));
            if response.hovered() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            }
            if response.clicked() {
                picked = Some(*kind);
            }
        }
    });
    picked
}

/// The templates of one kind in a row that scrolls sideways; the one
/// clicked.
fn template_row(
    ui: &mut egui::Ui,
    theme: &Theme,
    catalogue: &mut Catalogue,
    id: &str,
    list: &[&Template],
) -> Option<PathBuf> {
    let mut picked = None;
    egui::ScrollArea::horizontal()
        .id_salt(("start.row", id))
        .auto_shrink([false, true])
        .show(ui, |ui| {
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = 16.0;
                for template in list {
                    if template_card(ui, theme, catalogue, template, CARD_HEIGHT) {
                        picked = Some(template.path.clone());
                    }
                }
            });
        });
    picked
}

/// One template: its thumbnail, whole, on a soft tile of `height` and the
/// width of its proportions, and its name under it; `true` when clicked.
/// The thumbnail is asked for only once the card is in view.
fn template_card(
    ui: &mut egui::Ui,
    theme: &Theme,
    catalogue: &mut Catalogue,
    template: &Template,
    height: f32,
) -> bool {
    let aspect = template.kind.map_or(1.0, |k| k.aspect() as f32);
    let width = (height * aspect).clamp(height * 0.56, height * 1.9);
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, height + 30.0), Sense::click());
    let tile = Rect::from_min_size(rect.min, Vec2::new(width, height));
    let hover = ui.ctx().animate_bool(response.id, response.hovered());
    ui.painter()
        .rect_filled(tile, 12.0, theme.c("surface.sunken"));
    if ui.is_rect_visible(rect)
        && let Some((texture, (w, h))) = catalogue.thumbnail(template)
    {
        let (fw, fh) = recent::fit(w, h, (tile.width() - 2.0, tile.height() - 2.0));
        let picture = Rect::from_center_size(tile.center(), Vec2::new(fw, fh));
        ui.painter().image(
            texture,
            picture,
            Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            Color32::WHITE,
        );
    }
    let stroke = if response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        Stroke::new(1.5 + hover, theme.c("accent"))
    } else {
        Stroke::new(1.0, theme.c("border"))
    };
    ui.painter()
        .rect_stroke(tile, 12.0, stroke, egui::StrokeKind::Inside);
    let galley = ui.painter().layout_no_wrap(
        template.name.clone(),
        theme.font("body-sm"),
        theme.c("text"),
    );
    let clip = Rect::from_min_size(
        egui::pos2(tile.left() + 2.0, tile.bottom() + 6.0),
        Vec2::new(width - 4.0, 22.0),
    );
    ui.painter()
        .with_clip_rect(clip)
        .galley(clip.min, galley, theme.c("text"));
    response.on_hover_text(template.name.clone()).clicked()
}
