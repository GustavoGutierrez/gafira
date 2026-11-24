//! Plantillas in the rail of the editor (spec 091): a search over the
//! templates that suit the proportions of the page in view, the ones used
//! last, and every other one in two columns that load as they scroll. A
//! click puts the template on the page (`ApplyTemplate`).

use std::path::{Path, PathBuf};

use egui::{Color32, Rect, RichText, Sense, Stroke, Vec2};
use gafira_doc::msg::{Message, MessageKey};
use gafira_doc::{Document, Page};
use gafira_ports::i18n::Locale;

use crate::design_sizes;
use crate::recent;
use crate::style::Theme;
use crate::template_fit;
use crate::templates::{self, Catalogue, Filter, Template};
use crate::text::Texts;
use crate::widgets;

/// How many templates «Usado recién» keeps.
const USED: usize = 6;

/// What the panel keeps between frames.
#[derive(Default)]
pub(crate) struct Panel {
    query: String,
    /// The templates put on a page, the last first.
    used: Vec<PathBuf>,
    /// How many pages of the grid are shown.
    pages: usize,
    /// The texts in both languages, for a search in either.
    both: Option<[Texts; 2]>,
}

impl Panel {
    /// Remember that `path` was put on a page.
    pub(crate) fn used(&mut self, path: &Path) {
        self.used.retain(|p| p != path);
        self.used.insert(0, path.to_path_buf());
        self.used.truncate(USED);
    }
}

fn t(texts: &Texts, key: &'static str) -> String {
    texts.t(&Message {
        key: MessageKey(key),
        args: Vec::new(),
    })
}

/// How the page is named in the search's hint: the size of «Crear un
/// diseño» it has, else its size in px.
fn size_name(texts: &Texts, document: &Document, page: &Page) -> String {
    let known = design_sizes::SIZES
        .iter()
        .find(|size| size.page_size() == page.size)
        .map(|size| t(texts, size.key));
    known.unwrap_or_else(|| match page.size.to_px(document.dpi()) {
        Ok(px) => format!("{:.0} × {:.0} px", px.width(), px.height()),
        Err(_) => String::new(),
    })
}

/// The panel for `page` of `document`; the template clicked, if any.
pub(crate) fn show(
    ui: &mut egui::Ui,
    theme: &Theme,
    texts: &Texts,
    panel: &mut Panel,
    catalogue: &mut Catalogue,
    (document, page): (&Document, &Page),
) -> Option<PathBuf> {
    catalogue.pump(ui.ctx());
    let aspect = page
        .size
        .to_px(document.dpi())
        .map(|px| px.width() / px.height())
        .unwrap_or(1.0);
    let hint = texts.t(&gafira_doc::msg!(
        "templates.search_for",
        size = Text(size_name(texts, document, page))
    ));
    let changed = ui
        .add_sized(
            [ui.available_width(), 36.0],
            widgets::field(&mut panel.query, hint),
        )
        .changed();
    if changed || panel.pages == 0 {
        panel.pages = 1;
    }
    ui.add_space(12.0);
    let Some(all) = catalogue.list().map(<[Template]>::to_vec) else {
        muted(ui, theme, &t(texts, "templates.loading"));
        return None;
    };
    let both = panel
        .both
        .get_or_insert_with(|| [Texts::new(Locale::Es), Texts::new(Locale::En)]);
    let filter = Filter {
        query: panel.query.clone(),
        ..Filter::default()
    };
    let suited: Vec<Template> = templates::filtered(&all, &filter, texts.locale, |tpl| {
        let mut words = String::new();
        for texts in both.iter() {
            if let Some(kind) = tpl.kind {
                words.push_str(&t(texts, kind.key));
                words.push(' ');
            }
            if let Some(category) = tpl.category() {
                words.push_str(&t(texts, category.key()));
                words.push(' ');
            }
        }
        words
    })
    .into_iter()
    .filter(|tpl| {
        tpl.kind
            .is_some_and(|kind| template_fit::fits(aspect, kind.aspect()))
    })
    .cloned()
    .collect();
    if suited.is_empty() {
        muted(ui, theme, &t(texts, "templates.none_for_size"));
        return None;
    }
    let used: Vec<&Template> = panel
        .used
        .iter()
        .filter_map(|path| suited.iter().find(|tpl| &tpl.path == path))
        .collect();
    let rest: Vec<&Template> = suited
        .iter()
        .filter(|tpl| !panel.used.contains(&tpl.path))
        .collect();
    egui::ScrollArea::vertical()
        .id_salt("editor.templates")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let mut picked = None;
            if !used.is_empty() && panel.query.trim().is_empty() {
                title(ui, theme, &t(texts, "templates.used"));
                picked = picked.or(columns(ui, theme, catalogue, &used, aspect));
                ui.add_space(14.0);
            }
            title(ui, theme, &t(texts, "templates.for_you"));
            let shown = templates::shown(&rest, panel.pages);
            picked = picked.or(columns(ui, theme, catalogue, shown, aspect));
            if shown.len() < rest.len() {
                let (end, _) =
                    ui.allocate_exact_size(Vec2::new(ui.available_width(), 24.0), Sense::hover());
                if ui.is_rect_visible(end) {
                    panel.pages += 1;
                    ui.ctx().request_repaint();
                }
            }
            picked
        })
        .inner
}

fn title(ui: &mut egui::Ui, theme: &Theme, text: &str) {
    ui.label(
        RichText::new(text)
            .font(theme.font("ui-label"))
            .color(theme.c("text")),
    );
    ui.add_space(8.0);
}

fn muted(ui: &mut egui::Ui, theme: &Theme, text: &str) {
    ui.label(
        RichText::new(text)
            .font(theme.font("body-sm"))
            .color(theme.c("text.muted")),
    );
}

/// `list` in two columns, each card in the page's proportions; the one
/// clicked.
fn columns(
    ui: &mut egui::Ui,
    theme: &Theme,
    catalogue: &mut Catalogue,
    list: &[&Template],
    aspect: f64,
) -> Option<PathBuf> {
    let gap = 10.0;
    let width = ((ui.available_width() - gap) / 2.0).max(60.0);
    let height = (width / aspect.clamp(0.4, 3.0) as f32).round();
    let mut picked = None;
    for pair in list.chunks(2) {
        ui.horizontal_top(|ui| {
            ui.spacing_mut().item_spacing.x = gap;
            for template in pair {
                if card(ui, theme, catalogue, template, Vec2::new(width, height)) {
                    picked = Some(template.path.clone());
                }
            }
        });
        ui.add_space(gap);
    }
    picked
}

/// One template: its thumbnail, whole, on a soft tile; its name on hover.
fn card(
    ui: &mut egui::Ui,
    theme: &Theme,
    catalogue: &mut Catalogue,
    template: &Template,
    size: Vec2,
) -> bool {
    let (tile, response) = ui.allocate_exact_size(size, Sense::click());
    ui.painter()
        .rect_filled(tile, 8.0, theme.c("surface.sunken"));
    if ui.is_rect_visible(tile)
        && let Some((texture, (w, h))) = catalogue.thumbnail(template)
    {
        let (fw, fh) = recent::fit(w, h, (tile.width(), tile.height()));
        ui.painter().image(
            texture,
            Rect::from_center_size(tile.center(), Vec2::new(fw, fh)),
            Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            Color32::WHITE,
        );
    }
    let stroke = if response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        Stroke::new(2.0, theme.c("accent"))
    } else {
        Stroke::new(1.0, theme.c("border"))
    };
    ui.painter()
        .rect_stroke(tile, 8.0, stroke, egui::StrokeKind::Inside);
    response.on_hover_text(template.name.clone()).clicked()
}
