//! The assets of the installed packages in a panel of the editor (spec
//! 097, §13.1): in Ilustraciones, Iconos and Formas, each package under its
//! name, filtered by package and by a search on names, tags and keywords.
//! A click inserts an asset, a drag carries it to a page.

use egui::{Pos2, RichText, Sense, Vec2};
use gafira_doc::msg::{Message, MessageKey};

use crate::packs::{InstalledPack, PackAsset, PackSection, PackUse};
use crate::style::Theme;
use crate::text::Texts;

fn t(texts: &Texts, key: &'static str) -> String {
    texts.t(&Message {
        key: MessageKey(key),
        args: Vec::new(),
    })
}

/// What the person did with an asset.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum PackPick {
    /// Clicked: into the middle of the page.
    Insert(PackUse),
    /// Dropped at this screen point.
    Drop(PackUse, Pos2),
    /// Uninstall this package, once confirmed (spec 098).
    Uninstall(String),
}

/// The filter of one panel: the package shown (all when `None`) and the
/// words searched.
#[derive(Clone, Debug, Default)]
pub(crate) struct PackFilter {
    pub(crate) pack: Option<String>,
    pub(crate) words: String,
}

/// The package `id` was uninstalled: the filter no longer shows it.
pub(crate) fn forget(filter: &mut PackFilter, id: &str) {
    if filter.pack.as_deref() == Some(id) {
        filter.pack = None;
    }
}

/// Whether the packages of a panel show their assets without being opened
/// (spec 098): while words are searched or one package is chosen, so the
/// results are in sight; otherwise each package is closed until opened.
#[must_use]
pub(crate) fn open_by_filter(filter: &PackFilter) -> bool {
    !filter.words.trim().is_empty() || filter.pack.is_some()
}

/// Whether `asset` matches the words searched: each word is in its name,
/// its tags or its keywords, without case.
#[must_use]
pub(crate) fn matches(asset: &PackAsset, words: &str) -> bool {
    let haystack: Vec<String> = std::iter::once(&asset.name)
        .chain(&asset.tags)
        .chain(&asset.keywords)
        .map(|w| w.to_lowercase())
        .collect();
    words
        .split_whitespace()
        .map(str::to_lowercase)
        .all(|word| haystack.iter().any(|h| h.contains(&word)))
}

/// The assets of `pack` shown in `section` that pass `filter`.
#[cfg(test)]
#[must_use]
pub(crate) fn shown<'a>(
    pack: &'a InstalledPack,
    section: PackSection,
    filter: &PackFilter,
) -> Vec<&'a PackAsset> {
    shown_in(pack, &[section], filter)
}

/// The assets of `pack` shown in any of `sections` that pass `filter`.
#[must_use]
pub(crate) fn shown_in<'a>(
    pack: &'a InstalledPack,
    sections: &[PackSection],
    filter: &PackFilter,
) -> Vec<&'a PackAsset> {
    if filter
        .pack
        .as_ref()
        .is_some_and(|id| *id != pack.manifest.id)
    {
        return Vec::new();
    }
    pack.manifest
        .assets
        .iter()
        .filter(|a| a.section().is_some_and(|s| sections.contains(&s)) && matches(a, &filter.words))
        .collect()
}

/// The packages that have assets for `section`.
#[cfg(test)]
#[must_use]
pub(crate) fn with_section(packs: &[InstalledPack], section: PackSection) -> Vec<&InstalledPack> {
    with_any(packs, &[section])
}

/// The packages that have assets for any of `sections`.
#[must_use]
pub(crate) fn with_any<'a>(
    packs: &'a [InstalledPack],
    sections: &[PackSection],
) -> Vec<&'a InstalledPack> {
    packs
        .iter()
        .filter(|p| {
            p.manifest
                .assets
                .iter()
                .any(|a| a.section().is_some_and(|s| sections.contains(&s)))
        })
        .collect()
}

/// Draws the installed assets of `section`; what was picked. Nothing is
/// drawn when no package has assets there.
pub(crate) fn show(
    ui: &mut egui::Ui,
    theme: &Theme,
    texts: &Texts,
    (sections, packs): (&[PackSection], &[InstalledPack]),
    filter: &mut PackFilter,
    can_drop: &dyn Fn(Pos2) -> bool,
) -> Option<PackPick> {
    let spanish = texts.locale == gafira_ports::i18n::Locale::Es;
    let listed = with_any(packs, sections);
    let &section = sections.first()?;
    if listed.is_empty() {
        return None;
    }
    let mut picked = None;
    ui.label(
        RichText::new(t(texts, "packs.installed_assets"))
            .font(theme.font("ui-label"))
            .color(theme.c("text")),
    );
    ui.add_space(6.0);
    // The package filter and the search.
    let all = t(texts, "packs.all_packs");
    let current = filter
        .pack
        .as_ref()
        .and_then(|id| listed.iter().find(|p| &p.manifest.id == id))
        .map_or(all.clone(), |p| p.manifest.name.get(spanish).to_owned());
    egui::ComboBox::from_id_salt(("packs.filter", section.code()))
        .selected_text(current)
        .width(ui.available_width() - 8.0)
        .show_ui(ui, |ui| {
            ui.selectable_value(&mut filter.pack, None, all);
            for pack in &listed {
                ui.selectable_value(
                    &mut filter.pack,
                    Some(pack.manifest.id.clone()),
                    pack.manifest.name.get(spanish),
                );
            }
        });
    ui.add_space(6.0);
    ui.add(crate::widgets::field(
        &mut filter.words,
        t(texts, "packs.search"),
    ));
    ui.add_space(10.0);
    let columns = if section == PackSection::Icons {
        5.0
    } else {
        4.0
    };
    let gap = 6.0;
    let side = ((ui.available_width() - 8.0 - (columns - 1.0) * gap) / columns).max(36.0);
    let mut any = false;
    for pack in listed {
        let assets = shown_in(pack, sections, filter);
        if assets.is_empty() {
            continue;
        }
        any = true;
        // Each package is a header that opens and closes (spec 098); closed
        // by default, open while searching or filtering by a package.
        let id = ui
            .id()
            .with(("packs.open", section.code(), &pack.manifest.id));
        let opened = ui.data(|d| d.get_temp::<bool>(id)).unwrap_or(false);
        let open = opened || open_by_filter(filter);
        let (header, more) = pack_header(ui, theme, texts, pack, (spanish, assets.len(), open));
        if header.clicked() {
            ui.data_mut(|d| d.insert_temp(id, !opened));
        }
        // «⋮» and a right click offer to uninstall it, asked first under
        // the header.
        let asking = ui
            .id()
            .with(("packs.uninstall", section.code(), &pack.manifest.id));
        let mut ask = false;
        let menu = |ui: &mut egui::Ui| {
            crate::widgets::menu_style(ui, theme);
            let image = crate::icons::image(
                ui.ctx(),
                crate::icons::Icon::Delete,
                crate::icons::Weight::Regular,
                16.0,
                theme.c("text"),
            );
            let item = egui::Button::image_and_text(image, t(texts, "packs.uninstall_pack"));
            if ui.add(item).clicked() {
                ui.close();
                return true;
            }
            false
        };
        if let Some(inner) = egui::Popup::menu(&more).show(menu) {
            ask |= inner.inner;
        }
        if let Some(inner) = egui::Popup::context_menu(&header).show(menu) {
            ask |= inner.inner;
        }
        if ask {
            ui.data_mut(|d| d.insert_temp(asking, true));
        }
        if ui.data(|d| d.get_temp::<bool>(asking)).unwrap_or(false) {
            ui.add_space(6.0);
            ui.label(
                RichText::new(t(texts, "packs.uninstall_ask"))
                    .font(theme.font("body-sm"))
                    .color(theme.c("text")),
            );
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                if crate::widgets::primary(ui, theme, t(texts, "packs.uninstall")).clicked() {
                    ui.data_mut(|d| d.remove::<bool>(asking));
                    picked = Some(PackPick::Uninstall(pack.manifest.id.clone()));
                }
                if crate::widgets::secondary(ui, theme, t(texts, "packs.cancel")).clicked() {
                    ui.data_mut(|d| d.remove::<bool>(asking));
                }
            });
            ui.add_space(6.0);
        }
        if !open {
            ui.add_space(4.0);
            continue;
        }
        ui.add_space(6.0);
        for line in assets.chunks(columns as usize) {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing = Vec2::splat(gap);
                for asset in line {
                    if let Some(pick) = tile(ui, theme, pack, asset, side, can_drop) {
                        picked = Some(pick);
                    }
                }
            });
            ui.add_space(gap);
        }
        ui.add_space(8.0);
    }
    if !any {
        ui.label(
            RichText::new(t(texts, "packs.no_results"))
                .font(theme.font("body-sm"))
                .color(theme.c("text.secondary")),
        );
    }
    ui.add_space(12.0);
    ui.separator();
    ui.add_space(12.0);
    picked
}

/// The header of a package: the arrow, its picture, its name, how many
/// assets it shows and its «⋮»; the whole row opens and closes it. The row
/// and the «⋮».
fn pack_header(
    ui: &mut egui::Ui,
    theme: &Theme,
    texts: &Texts,
    pack: &InstalledPack,
    (spanish, count, open): (bool, usize, bool),
) -> (egui::Response, egui::Response) {
    let width = ui.available_width() - 8.0;
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, 40.0), Sense::click());
    let more_rect = egui::Rect::from_center_size(
        Pos2::new(rect.right() - 16.0, rect.center().y),
        Vec2::new(24.0, 28.0),
    );
    let painter = ui.painter();
    let fill = if response.hovered() {
        theme.c("secondary.bg.hover")
    } else {
        theme.c("secondary.bg")
    };
    painter.rect_filled(rect, theme.radius_sm(), fill);
    let more = ui
        .scope_builder(egui::UiBuilder::new().max_rect(more_rect), |ui| {
            crate::layer_rows::menu_button(ui, theme, &t(texts, "packs.more"), false)
        })
        .inner;
    let painter = ui.painter();
    let arrow = if open {
        crate::icons::Icon::Expand
    } else {
        crate::icons::Icon::Submenu
    };
    let arrow_rect = egui::Rect::from_center_size(
        Pos2::new(rect.left() + 16.0, rect.center().y),
        Vec2::splat(14.0),
    );
    crate::icons::paint(
        painter,
        arrow_rect,
        arrow,
        crate::icons::Weight::Regular,
        theme.c("text.secondary"),
    );
    let mut x = rect.left() + 30.0;
    if let Some(image) = crate::brand_screen::file_image(ui.ctx(), &pack.thumbnail()) {
        let picture =
            egui::Rect::from_min_size(Pos2::new(x, rect.center().y - 12.0), Vec2::splat(24.0));
        crate::brand_screen::paint_fitted(ui, image, picture);
        x += 32.0;
    }
    let count_text = count.to_string();
    let painter = ui.painter();
    let count_galley =
        painter.layout_no_wrap(count_text, theme.font("body-sm"), theme.c("text.secondary"));
    let count_at = Pos2::new(
        more_rect.left() - 6.0 - count_galley.size().x,
        rect.center().y - count_galley.size().y / 2.0,
    );
    let name_width = (count_at.x - 8.0 - x).max(0.0);
    let name = painter.layout(
        pack.manifest.name.get(spanish).to_owned(),
        theme.font("ui-label"),
        theme.c("text"),
        f32::INFINITY,
    );
    let clip = egui::Rect::from_min_max(
        Pos2::new(x, rect.top()),
        Pos2::new(x + name_width, rect.bottom()),
    );
    painter.with_clip_rect(clip).galley(
        Pos2::new(x, rect.center().y - name.size().y / 2.0),
        name,
        theme.c("text"),
    );
    painter.galley(count_at, count_galley, theme.c("text.secondary"));
    if response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    let label = pack.manifest.name.get(spanish).to_owned();
    response.widget_info(|| {
        egui::WidgetInfo::selected(egui::WidgetType::CollapsingHeader, true, open, &label)
    });
    (response, more)
}

/// One asset: its preview on a soft tile, its name on hover.
fn tile(
    ui: &mut egui::Ui,
    theme: &Theme,
    pack: &InstalledPack,
    asset: &PackAsset,
    side: f32,
    can_drop: &dyn Fn(Pos2) -> bool,
) -> Option<PackPick> {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(side), Sense::click_and_drag());
    let fill = if response.hovered() || response.dragged() {
        theme.c("secondary.bg")
    } else {
        theme.c("surface.sunken")
    };
    ui.painter().rect_filled(rect, 8.0, fill);
    let usage = pack.usage(asset)?;
    // Its preview; else the file itself, the mask of a frame, or the cells
    // of a collage drawn.
    let preview = asset
        .preview
        .as_ref()
        .map(|p| pack.dir.join(p))
        .or(match &usage {
            PackUse::File(file) => Some(file.clone()),
            PackUse::Frame { mask, .. } => Some(mask.clone()),
            PackUse::Collage { .. } | PackUse::Template(_) => None,
        });
    // Only a tile in view loads its picture: a large package never loads
    // every preview at once.
    let image = preview
        .as_ref()
        .filter(|_| ui.is_rect_visible(rect) || response.dragged())
        .and_then(|p| crate::brand_screen::file_image(ui.ctx(), p));
    if let Some(image) = image.clone() {
        crate::brand_screen::paint_fitted(ui, image, rect.shrink(side / 10.0));
    } else if let PackUse::Collage { cells, .. } = &usage {
        let inner = rect.shrink(side / 8.0);
        for &(x, y, w, h) in cells {
            let cell = egui::Rect::from_min_size(
                inner.min + Vec2::new(x as f32 * inner.width(), y as f32 * inner.height()),
                Vec2::new(w as f32 * inner.width(), h as f32 * inner.height()),
            )
            .shrink(1.5);
            ui.painter().rect_filled(cell, 2.0, theme.c("text.muted"));
        }
    }
    if response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
    }
    let at = ui.input(|i| i.pointer.latest_pos());
    if response.dragged()
        && let Some(at) = at
    {
        ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
        // What follows the pointer: the asset's tile, see-through.
        let ghost = egui::Rect::from_center_size(at, Vec2::splat(side));
        let painter = ui.ctx().layer_painter(egui::LayerId::new(
            egui::Order::Tooltip,
            egui::Id::new("pack.ghost"),
        ));
        painter.rect_filled(ghost, 8.0, theme.c("secondary.bg").gamma_multiply(0.7));
        if let Some(texture) = image.and_then(|i| {
            i.load_for_size(ui.ctx(), ghost.size())
                .ok()
                .and_then(|p| p.texture_id())
        }) {
            painter.image(
                texture,
                ghost.shrink(side / 10.0),
                egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                egui::Color32::from_white_alpha(200),
            );
        }
    }
    let name = asset.name.clone();
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, &name));
    let response = response.on_hover_text(asset.name.clone());
    if response.clicked() {
        return Some(PackPick::Insert(usage));
    }
    if response.drag_stopped()
        && let Some(at) = at
        && can_drop(at)
    {
        return Some(PackPick::Drop(usage, at));
    }
    None
}

#[cfg(test)]
mod tests {

    // Spec 098: packages are closed until opened, but searching or choosing
    // one shows its assets.
    #[test]
    fn s98_14_packages_open_only_while_searching_or_filtering() {
        let mut filter = PackFilter::default();
        assert!(!open_by_filter(&filter));
        filter.words = "  ".to_owned();
        assert!(!open_by_filter(&filter));
        filter.words = "flor".to_owned();
        assert!(open_by_filter(&filter));
        filter.words.clear();
        filter.pack = Some("acme.flowers".to_owned());
        assert!(open_by_filter(&filter));
    }

    // Spec 098: a package uninstalled from its panel no longer filters it.
    #[test]
    fn s98_15_an_uninstalled_package_leaves_the_filter() {
        let mut filter = PackFilter {
            pack: Some("acme.flowers".to_owned()),
            words: "flor".to_owned(),
        };
        forget(&mut filter, "acme.other");
        assert_eq!(filter.pack.as_deref(), Some("acme.flowers"));
        forget(&mut filter, "acme.flowers");
        assert_eq!(filter.pack, None);
        assert_eq!(filter.words, "flor");
    }

    use std::collections::BTreeSet;
    use std::path::PathBuf;

    use super::*;
    use crate::packs::{PackAsset, PackManifest};

    fn pack(id: &str, assets: Vec<PackAsset>) -> InstalledPack {
        InstalledPack {
            manifest: PackManifest {
                id: id.to_owned(),
                assets,
                ..PackManifest::default()
            },
            dir: PathBuf::from("/packs").join(id),
            installed_at: 0,
            size: 0,
        }
    }

    fn asset(id: &str, kind: &str, tags: &[&str]) -> PackAsset {
        PackAsset {
            id: id.to_owned(),
            kind: kind.to_owned(),
            name: id.to_owned(),
            file: Some(format!("assets/{id}.svg")),
            tags: tags.iter().map(|s| (*s).to_owned()).collect(),
            ..PackAsset::default()
        }
    }

    // S97-11: each section lists only its assets; the package filter and
    // the search (name, tags, keywords, any case) narrow them.
    #[test]
    fn s97_11_assets_are_filtered_by_section_pack_and_words() {
        let a = pack(
            "com.a.xmas",
            vec![
                asset("Santa", "clipart", &["navidad"]),
                asset("gift", "icon", &["regalo"]),
                asset("star", "shape", &[]),
            ],
        );
        let b = pack("com.b.icons", vec![asset("home", "icon", &["casa"])]);
        let packs = vec![a, b];
        let ids = |section, filter: &PackFilter| -> BTreeSet<String> {
            packs
                .iter()
                .flat_map(|p| shown(p, section, filter))
                .map(|a| a.id.clone())
                .collect()
        };
        let all = PackFilter::default();
        assert_eq!(
            ids(PackSection::Icons, &all),
            ["gift", "home"].map(String::from).into()
        );
        assert_eq!(
            ids(PackSection::Illustrations, &all),
            ["Santa"].map(String::from).into()
        );
        assert_eq!(
            ids(PackSection::Shapes, &all),
            ["star"].map(String::from).into()
        );
        let only_b = PackFilter {
            pack: Some("com.b.icons".to_owned()),
            words: String::new(),
        };
        assert_eq!(
            ids(PackSection::Icons, &only_b),
            ["home"].map(String::from).into()
        );
        let search = PackFilter {
            pack: None,
            words: "REGA".to_owned(),
        };
        assert_eq!(
            ids(PackSection::Icons, &search),
            ["gift"].map(String::from).into()
        );
        assert_eq!(with_section(&packs, PackSection::Shapes).len(), 1);
        assert!(with_section(&packs, PackSection::Templates).is_empty());
    }
}
