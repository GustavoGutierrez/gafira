//! Elementos → Iconos (spec 103): the Tabler icon set (MIT) that ships with Gafira, in its
//! line and filled styles, grouped by category with a search. A click adds an icon to the
//! page as vector shapes; a drag drops it where it is let go. Brand logos are left out.
//!
//! The set is `assets/icons/tabler/tabler.json.gz`, made by `tools/gen_tabler_icons.py`;
//! it is read the first time the panel opens.

use std::collections::BTreeMap;
use std::io::Read;
use std::sync::OnceLock;

use egui::{Pos2, Rect, RichText, Sense, Stroke, Vec2};
use serde::Deserialize;

use crate::charts::plain;
use crate::style::Theme;
use crate::text::Texts;

const DATA: &[u8] = include_bytes!("../../../assets/icons/tabler/tabler.json.gz");

/// The icons of a category shown before «Ver todo».
const PREVIEW: usize = 10;
/// The most icons a search shows.
const RESULTS: usize = 150;
/// New icon pictures drawn in one frame, so scrolling stays smooth.
const PER_FRAME: usize = 24;
/// The colour an inserted icon takes.
const INK: &str = "#18243B";

#[derive(Deserialize)]
struct Raw {
    categories: Vec<String>,
    /// Name, category, tags, line drawing, filled drawing.
    icons: Vec<(String, usize, String, String, String)>,
}

/// One icon of the set.
pub(crate) struct LibIcon {
    pub(crate) name: String,
    pub(crate) category: usize,
    tags: String,
    line: String,
    fill: String,
}

pub(crate) struct Library {
    pub(crate) categories: Vec<String>,
    pub(crate) icons: Vec<LibIcon>,
}

/// The set, read once.
pub(crate) fn library() -> &'static Library {
    static LIBRARY: OnceLock<Library> = OnceLock::new();
    LIBRARY.get_or_init(|| {
        let mut json = Vec::new();
        let raw: Option<Raw> = flate2::read::GzDecoder::new(DATA)
            .read_to_end(&mut json)
            .ok()
            .and_then(|_| serde_json::from_slice(&json).ok());
        let raw = raw.unwrap_or(Raw {
            categories: Vec::new(),
            icons: Vec::new(),
        });
        Library {
            categories: raw.categories,
            icons: raw
                .icons
                .into_iter()
                .map(|(name, category, tags, line, fill)| LibIcon {
                    name,
                    category,
                    tags,
                    line,
                    fill,
                })
                .collect(),
        }
    })
}

/// The two styles of the set.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Style {
    #[default]
    Line,
    Fill,
}

impl LibIcon {
    fn drawing(&self, style: Style) -> &str {
        match style {
            Style::Line => &self.line,
            Style::Fill => &self.fill,
        }
    }

    /// The icon as a whole SVG in `color`.
    pub(crate) fn svg(&self, style: Style, color: &str) -> Option<String> {
        let body = self.drawing(style);
        if body.is_empty() {
            return None;
        }
        let paint = match style {
            Style::Line => format!(
                r#"fill="none" stroke="{color}" stroke-width="2" stroke-linecap="round" stroke-linejoin="round""#
            ),
            Style::Fill => format!(r#"fill="{color}" stroke="none""#),
        };
        Some(format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" {paint}>{body}</svg>"#
        ))
    }
}

/// The category key of the catalogs: «Version control» → `icons.cat_version_control`.
fn category_key(name: &str) -> String {
    let slug: String = name
        .to_ascii_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect();
    format!("icons.cat_{slug}")
}

/// The indices of the icons of `style` whose name, tags or category hold every word of
/// `query` (any case), in the order of the set.
pub(crate) fn search(lib: &Library, style: Style, query: &str, names: &[String]) -> Vec<usize> {
    let words: Vec<String> = query.split_whitespace().map(str::to_lowercase).collect();
    lib.icons
        .iter()
        .enumerate()
        .filter(|(_, icon)| !icon.drawing(style).is_empty())
        .filter(|(_, icon)| {
            let cat = names
                .get(icon.category)
                .map(String::as_str)
                .unwrap_or_default()
                .to_lowercase();
            let english = lib
                .categories
                .get(icon.category)
                .map(String::as_str)
                .unwrap_or_default()
                .to_lowercase();
            words.iter().all(|w| {
                icon.name.contains(w.as_str())
                    || icon.tags.to_lowercase().contains(w.as_str())
                    || cat.contains(w.as_str())
                    || english.contains(w.as_str())
            })
        })
        .map(|(i, _)| i)
        .collect()
}

/// What the person did: an icon as SVG, its name, and where it was dropped.
pub(crate) struct Picked {
    pub(crate) svg: String,
    pub(crate) name: String,
    pub(crate) at: Option<Pos2>,
}

/// The state of the panel.
#[derive(Default)]
pub(crate) struct IconPanel {
    style: Style,
    query: String,
    open: Option<usize>,
    pictures: BTreeMap<(usize, Style), Option<egui::TextureHandle>>,
}

impl IconPanel {
    /// The panel. `can_drop` tells whether a screen point is over a page.
    pub(crate) fn show(
        &mut self,
        ui: &mut egui::Ui,
        theme: &Theme,
        texts: &Texts,
        can_drop: &dyn Fn(Pos2) -> bool,
    ) -> Option<Picked> {
        let lib = library();
        let names: Vec<String> = lib
            .categories
            .iter()
            .map(|c| texts.t(&dynamic(&category_key(c))))
            .collect();
        let mut picked = None;
        let mut drawn = 0;

        // Style: Línea or Relleno.
        ui.horizontal(|ui| {
            for (style, key) in [(Style::Line, "icons.line"), (Style::Fill, "icons.fill")] {
                let on = self.style == style;
                let label = RichText::new(plain(texts, key))
                    .font(theme.font("ui-label"))
                    .color(if on {
                        theme.c("text")
                    } else {
                        theme.c("text.muted")
                    });
                let response = ui.add(egui::Button::new(label).frame(false));
                if on {
                    let r = response.rect;
                    ui.painter().line_segment(
                        [r.left_bottom(), r.right_bottom()],
                        Stroke::new(2.0, theme.c("primary.bg")),
                    );
                }
                if response.clicked() {
                    self.style = style;
                }
                ui.add_space(8.0);
            }
        });
        ui.add_space(8.0);
        ui.add(crate::widgets::field(
            &mut self.query,
            plain(texts, "icons.search"),
        ));
        ui.add_space(10.0);

        let columns = 5;
        let gap = 6.0;
        let side =
            ((ui.available_width() - 12.0 - gap * (columns as f32 - 1.0)) / columns as f32).floor();
        let style = self.style;
        let mut grid = |ui: &mut egui::Ui, items: &[usize], me: &mut Self, drawn: &mut usize| {
            for row in items.chunks(columns) {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing = Vec2::splat(gap);
                    for &i in row {
                        let icon = &lib.icons[i];
                        let (rect, response) =
                            ui.allocate_exact_size(Vec2::splat(side), Sense::click_and_drag());
                        let painter = ui.painter();
                        let hovered = response.hovered() || response.dragged();
                        painter.rect_filled(
                            rect,
                            6.0,
                            if hovered {
                                theme.c("secondary.bg.hover")
                            } else {
                                theme.c("surface")
                            },
                        );
                        if ui.is_rect_visible(rect)
                            && let Some(texture) = me.picture(ui.ctx(), i, style, drawn)
                        {
                            let at =
                                Rect::from_center_size(rect.center(), Vec2::splat(side * 0.55));
                            let uv = Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0));
                            painter.image(texture, at, uv, theme.c("text"));
                        }
                        if hovered {
                            ui.ctx().set_cursor_icon(if response.dragged() {
                                egui::CursorIcon::Grabbing
                            } else {
                                egui::CursorIcon::PointingHand
                            });
                        }
                        let response = response.on_hover_text(icon.name.replace('-', " "));
                        let pointer = ui.input(|i| i.pointer.latest_pos());
                        if response.dragged()
                            && let (Some(p), Some(texture)) =
                                (pointer, me.picture(ui.ctx(), i, style, drawn))
                        {
                            let ghost = ui.ctx().layer_painter(egui::LayerId::new(
                                egui::Order::Tooltip,
                                egui::Id::new("icons.ghost"),
                            ));
                            let uv = Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0));
                            ghost.image(
                                texture,
                                Rect::from_center_size(p, Vec2::splat(56.0)),
                                uv,
                                theme.c("text").gamma_multiply(0.8),
                            );
                        }
                        let insert = |at| {
                            icon.svg(style, INK).map(|svg| Picked {
                                svg,
                                name: icon.name.replace('-', " "),
                                at,
                            })
                        };
                        if response.clicked() {
                            picked = insert(None);
                        }
                        if response.drag_stopped()
                            && let Some(p) = pointer
                            && can_drop(p)
                        {
                            picked = insert(Some(p));
                        }
                    }
                });
                ui.add_space(gap);
            }
        };

        egui::ScrollArea::vertical()
            .id_salt("icons.library")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.set_max_width(ui.available_width() - 12.0);
                if !self.query.trim().is_empty() {
                    let found = search(lib, style, &self.query, &names);
                    if found.is_empty() {
                        ui.label(
                            RichText::new(plain(texts, "icons.none"))
                                .font(theme.font("body-sm"))
                                .color(theme.c("text.secondary")),
                        );
                    }
                    let shown: Vec<usize> = found.into_iter().take(RESULTS).collect();
                    grid(ui, &shown, self, &mut drawn);
                    return;
                }
                if let Some(open) = self.open {
                    let back = ui.add(
                        egui::Button::new(
                            RichText::new(format!(
                                "← {}",
                                names.get(open).cloned().unwrap_or_default()
                            ))
                            .font(theme.font("ui-label")),
                        )
                        .frame(false),
                    );
                    if back.clicked() {
                        self.open = None;
                    }
                    ui.add_space(8.0);
                    let mut all: Vec<usize> = (0..lib.icons.len())
                        .filter(|&i| {
                            lib.icons[i].category == open && !lib.icons[i].drawing(style).is_empty()
                        })
                        .collect();
                    all.sort_by_key(|&i| (lib.icons[i].name.len(), i));
                    grid(ui, &all, self, &mut drawn);
                    return;
                }
                for (c, name) in names.iter().enumerate() {
                    let mut items: Vec<usize> = (0..lib.icons.len())
                        .filter(|&i| {
                            lib.icons[i].category == c && !lib.icons[i].drawing(style).is_empty()
                        })
                        .collect();
                    // The plain ones first: shorter names are the basic icons
                    // («arrow-left», «heart»), the long ones their variants.
                    items.sort_by_key(|&i| (lib.icons[i].name.len(), i));
                    if items.is_empty() {
                        continue;
                    }
                    let mut see_all = false;
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(name)
                                .font(theme.font("ui-label"))
                                .color(theme.c("text")),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if items.len() > PREVIEW {
                                let link = RichText::new(plain(texts, "icons.see_all"))
                                    .font(theme.font("caption"))
                                    .color(theme.c("text.secondary"));
                                see_all = ui.add(egui::Button::new(link).frame(false)).clicked();
                            }
                        });
                    });
                    ui.add_space(4.0);
                    grid(ui, &items[..items.len().min(PREVIEW)], self, &mut drawn);
                    ui.add_space(10.0);
                    if see_all {
                        self.open = Some(c);
                    }
                }
            });
        if drawn >= PER_FRAME {
            ui.ctx().request_repaint();
        }
        picked
    }

    /// The picture of icon `i` in `style`, white on transparent so the painter tints it;
    /// drawn at most [`PER_FRAME`] a frame.
    fn picture(
        &mut self,
        ctx: &egui::Context,
        i: usize,
        style: Style,
        drawn: &mut usize,
    ) -> Option<egui::TextureId> {
        if let Some(found) = self.pictures.get(&(i, style)) {
            return found.as_ref().map(egui::TextureHandle::id);
        }
        if *drawn >= PER_FRAME {
            return None;
        }
        *drawn += 1;
        let svg = library().icons.get(i)?.svg(style, "#FFFFFF")?;
        let texture = gafira_io_vector::svg_pixels(svg.as_bytes(), 4.0)
            .ok()
            .map(|(rgba, w, h)| {
                let image =
                    egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &rgba);
                ctx.load_texture(
                    format!("icon-{i}-{style:?}"),
                    image,
                    egui::TextureOptions::LINEAR,
                )
            });
        let id = texture.as_ref().map(egui::TextureHandle::id);
        self.pictures.insert((i, style), texture);
        id
    }
}

/// A message by a key built at run time.
fn dynamic(key: &str) -> gafira_doc::msg::Message {
    gafira_doc::msg::Message {
        key: gafira_doc::msg::MessageKey(leak(key)),
        args: Vec::new(),
    }
}

/// The catalog keys are `&'static str`; the category keys are few and fixed, made once.
fn leak(key: &str) -> &'static str {
    static KEYS: OnceLock<std::sync::Mutex<BTreeMap<String, &'static str>>> = OnceLock::new();
    let mut keys = KEYS
        .get_or_init(Default::default)
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if let Some(k) = keys.get(key) {
        return k;
    }
    let k: &'static str = Box::leak(key.to_owned().into_boxed_str());
    keys.insert(key.to_owned(), k);
    k
}

#[cfg(test)]
mod tests {
    use super::*;

    // S103-01: the set loads with its categories, no brand logos, and icons in both styles.
    #[test]
    fn s103_01_the_set_loads() {
        let lib = library();
        assert!(lib.icons.len() > 5000, "{}", lib.icons.len());
        assert_eq!(lib.categories.len(), 40);
        assert!(!lib.categories.iter().any(|c| c == "Brand"));
        assert!(lib.icons.iter().all(|i| i.category < lib.categories.len()));
        let home = lib.icons.iter().find(|i| i.name == "home").expect("home");
        let line = home.svg(Style::Line, INK).expect("line");
        assert!(line.contains(r##"stroke="#18243B""##) && line.contains("<path"));
        assert!(home.svg(Style::Fill, INK).is_some());
        // The drawing becomes Gafira shapes.
        let drawing = gafira_io_vector::svg_drawing_outlined(line.as_bytes()).expect("shapes");
        assert!(
            drawing
                .shapes
                .iter()
                .all(|s| s.fill.is_some() && s.stroke.is_none())
        );
    }

    // S103-02: the search matches names, tags and the category in either language.
    #[test]
    fn s103_02_the_search_finds_icons() {
        let lib = library();
        let names: Vec<String> = lib
            .categories
            .iter()
            .map(|c| {
                if c == "Food" {
                    "Comida".to_owned()
                } else {
                    c.clone()
                }
            })
            .collect();
        let found = search(lib, Style::Line, "home", &names);
        assert!(found.iter().any(|&i| lib.icons[i].name == "home"));
        assert!(!search(lib, Style::Line, "comida", &names).is_empty());
        assert!(search(lib, Style::Line, "zzzqqq", &names).is_empty());
        // Every key of a category exists in the catalogs.
        for c in &lib.categories {
            assert!(gafira_doc::msg::is_valid_key(leak(&category_key(c))), "{c}");
        }
    }
}
