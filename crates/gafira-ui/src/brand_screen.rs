//! The Marca screen (spec 093): every brand kit as a card, «Añadir uno
//! nuevo» with its name first, and the page of one kit, section by
//! section (logos, colours, fonts, photos, graphics, icons and its
//! brand templates); every change is kept at once (DIS-75).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use egui::{Align, Color32, Layout, Rect, RichText, Sense, Stroke, Vec2};
use gafira_doc::msg::{Message, MessageKey};

use crate::brand::{
    self, BrandColor, BrandFiles, BrandKit, BrandPalette, BrandStore, BrandTextStyle,
};
use crate::create::{self, Tab};
use crate::icons::{self, Icon, Weight};
use crate::recent;
use crate::shell::Go;
use crate::style::{DISPLAY, Theme};
use crate::text::Texts;
use crate::widgets;

fn t(texts: &Texts, key: &'static str) -> String {
    texts.t(&Message {
        key: MessageKey(key),
        args: Vec::new(),
    })
}

/// A thumbnail as a texture and its size, or none when it has none.
pub(crate) type Thumb = Option<(egui::TextureHandle, (u32, u32))>;

/// What the screen shows.
#[derive(Clone, Debug, PartialEq, Eq)]
enum View {
    Kits,
    Kit(String),
    Templates,
}

/// The colour being edited: its palette and place, and the text of its
/// fields.
struct ColorEdit {
    palette: usize,
    color: usize,
    hex: String,
    /// «C,M,Y,K» as typed, while CMYK is shown.
    cmyk: Option<String>,
    /// Hue, saturation and value, kept apart so a grey keeps its hue.
    hsv: [f32; 3],
    /// The tile the picker floats under, on screen.
    anchor: Option<Rect>,
}

impl ColorEdit {
    fn of(palette: usize, color: usize, value: &BrandColor) -> Self {
        let h = egui::ecolor::Hsva::from_srgb(rgb_bytes(value.rgb));
        Self {
            palette,
            color,
            hex: format!("#{:06X}", value.rgb),
            cmyk: value.cmyk.map(cmyk_text),
            hsv: [h.h, h.s, h.v],
            anchor: None,
        }
    }
}

/// The three bytes of `0xRRGGBB`.
fn rgb_bytes(c: u32) -> [u8; 3] {
    [(c >> 16) as u8, (c >> 8) as u8, c as u8]
}

pub(crate) struct BrandScreen {
    view: View,
    kits: Vec<BrandKit>,
    /// The name typed in «Crear kit de marca», while it is open.
    creating: Option<String>,
    /// The kit whose name is being changed, and the new name.
    renaming: Option<(String, String)>,
    color: Option<ColorEdit>,
    /// «Extraer colores de una imagen», while it is open.
    extract: Option<Extract>,
    /// The style being edited.
    style: Option<StyleDraft>,
    /// Whether «Administrar fuentes subidas» is open.
    fonts_open: bool,
    /// The section the left list asked to show.
    jump: Option<&'static str>,
    thumbs: BTreeMap<PathBuf, Thumb>,
    /// A message for the person, such as a template that could not be added.
    notice: Option<String>,
}

/// «Extraer colores de una imagen» for a palette: the image chosen, the
/// colours found in it and which of them the person keeps.
#[derive(Default)]
struct Extract {
    palette: usize,
    image: Option<PathBuf>,
    found: Vec<u32>,
    chosen: Vec<bool>,
}

impl Extract {
    /// The colours of the image at `path`: all chosen but those the
    /// palette already has.
    fn pick(&mut self, path: PathBuf, palette: &BrandPalette) {
        self.found = std::fs::read(&path)
            .map(|bytes| brand::colors_of_image(&bytes))
            .unwrap_or_default();
        self.chosen = self
            .found
            .iter()
            .map(|rgb| palette.colors.iter().all(|c| c.rgb != *rgb))
            .collect();
        self.image = Some(path);
    }
}

/// The file kinds of a kit and the keys of their titles.
const FILE_SECTIONS: [(BrandFiles, &str); 4] = [
    (BrandFiles::Logos, "brand.logos"),
    (BrandFiles::Photos, "brand.photos"),
    (BrandFiles::Graphics, "brand.graphics"),
    (BrandFiles::Icons, "brand.icons"),
];

/// The image files a kit takes.
const IMAGE_FILES: [&str; 6] = ["png", "jpg", "jpeg", "webp", "svg", "avif"];

impl BrandScreen {
    /// The kits of `store`, on the page of `kit` when given.
    pub(crate) fn new(store: &dyn BrandStore, kit: Option<String>) -> Self {
        Self {
            view: kit.map_or(View::Kits, View::Kit),
            kits: store.kits(),
            creating: None,
            renaming: None,
            color: None,
            extract: None,
            style: None,
            fonts_open: false,
            jump: None,
            thumbs: BTreeMap::new(),
            notice: None,
        }
    }

    pub(crate) fn show(
        &mut self,
        ctx: &egui::Context,
        theme: &Theme,
        texts: &Texts,
        store: &dyn BrandStore,
        window: Option<&eframe::Frame>,
    ) -> Go {
        let mut go = create::start_topbar(ctx, theme, texts, Some(Tab::Brand)).unwrap_or(Go::Stay);
        egui::SidePanel::left("brand.nav")
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
                if let Some(next) = self.nav(ui, theme, texts) {
                    go = next;
                }
            });
        egui::CentralPanel::default()
            .frame(
                egui::Frame::NONE
                    .fill(theme.c("bg"))
                    .inner_margin(egui::Margin {
                        left: 16,
                        right: 32,
                        top: 28,
                        bottom: 0,
                    }),
            )
            .show(ctx, |ui| {
                egui::ScrollArea::vertical()
                    .id_salt("brand.scroll")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        let next = match self.view.clone() {
                            View::Kits => self.kits_view(ui, theme, texts, store),
                            View::Kit(id) => self.kit_view(ui, theme, texts, store, &id, window),
                            View::Templates => self.templates_view(ui, theme, texts, store),
                        };
                        if let Some(next) = next {
                            go = next;
                        }
                        ui.add_space(40.0);
                    });
            });
        if self.creating.is_some() {
            self.create_modal(ctx, theme, texts, store);
        }
        if self.renaming.is_some() {
            self.rename_modal(ctx, theme, texts, store);
        }
        go
    }

    /// The list on the left.
    fn nav(&mut self, ui: &mut egui::Ui, theme: &Theme, texts: &Texts) -> Option<Go> {
        let mut go = None;
        let row = |ui: &mut egui::Ui, on: bool, icon: Icon, label: &str| {
            create::side_row(ui, theme, on, label, |ui, rect| {
                icons::paint(ui.painter(), rect, icon, Weight::Regular, theme.c("text"));
            })
            .clicked()
        };
        ui.spacing_mut().item_spacing.y = 2.0;
        if let View::Kit(id) = &self.view {
            if row(ui, false, Icon::Back, &t(texts, "brand.back_kits")) {
                self.view = View::Kits;
                return None;
            }
            ui.add_space(14.0);
            let name = self
                .kits
                .iter()
                .find(|k| &k.id == id)
                .map(|k| k.name.clone())
                .unwrap_or_default();
            ui.label(
                RichText::new(name)
                    .font(theme.font("ui-label"))
                    .color(theme.c("text")),
            );
            ui.add_space(8.0);
            for (key, _) in sections() {
                if create::side_row(ui, theme, false, &t(texts, key), |_, _| {}).clicked() {
                    self.jump = Some(key);
                }
            }
            return None;
        }
        if row(ui, false, Icon::Back, &t(texts, "create.back_home")) {
            go = Some(Go::Home);
        }
        ui.add_space(14.0);
        if row(
            ui,
            self.view == View::Kits,
            Icon::Identity,
            &t(texts, "brand.kits"),
        ) {
            self.view = View::Kits;
        }
        if row(
            ui,
            self.view == View::Templates,
            Icon::Templates,
            &t(texts, "brand.templates"),
        ) {
            self.view = View::Templates;
        }
        go
    }

    /// Every kit as a card, and «Añadir uno nuevo».
    fn kits_view(
        &mut self,
        ui: &mut egui::Ui,
        theme: &Theme,
        texts: &Texts,
        store: &dyn BrandStore,
    ) -> Option<Go> {
        if title_row(
            ui,
            theme,
            &t(texts, "brand.kits"),
            Some(&t(texts, "brand.add_new")),
        ) {
            self.creating = Some(String::new());
        }
        if self.kits.is_empty() {
            empty_state(
                ui,
                theme,
                &t(texts, "brand.no_kits"),
                &t(texts, "brand.no_kits_hint"),
            );
            return None;
        }
        let gap = 18.0;
        let width = ((ui.available_width() + gap) / 4.0 - gap).clamp(200.0, 300.0);
        let mut open = None;
        let mut delete = None;
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = Vec2::new(gap, gap);
            for kit in &self.kits {
                match kit_card(ui, theme, texts, kit, width) {
                    Some(CardAction::Open) => open = Some(kit.id.clone()),
                    Some(CardAction::Rename) => {
                        self.renaming = Some((kit.id.clone(), kit.name.clone()))
                    }
                    Some(CardAction::Delete) => delete = Some(kit.id.clone()),
                    None => {}
                }
            }
        });
        if let Some(id) = delete {
            store.delete(&id);
            self.kits = store.kits();
        }
        if let Some(id) = open {
            store.set_active(&id);
            self.view = View::Kit(id);
        }
        None
    }

    /// Scroll to here when the left list asked for `key`.
    fn jump_here(&mut self, ui: &mut egui::Ui, key: &'static str) {
        if self.jump == Some(key) {
            ui.scroll_to_cursor(Some(Align::TOP));
            self.jump = None;
        }
    }

    /// The page of one kit.
    fn kit_view(
        &mut self,
        ui: &mut egui::Ui,
        theme: &Theme,
        texts: &Texts,
        store: &dyn BrandStore,
        id: &str,
        window: Option<&eframe::Frame>,
    ) -> Option<Go> {
        let Some(index) = self.kits.iter().position(|k| k.id == id) else {
            self.view = View::Kits;
            return None;
        };
        let mut kit = self.kits[index].clone();
        let before = kit.clone();
        let mut go = None;
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(&kit.name)
                    .font(egui::FontId::new(
                        30.0,
                        egui::FontFamily::Name(DISPLAY.into()),
                    ))
                    .color(theme.c("text")),
            );
            if icon_button(ui, theme, Icon::Drawing, &t(texts, "brand.rename")) {
                self.renaming = Some((kit.id.clone(), kit.name.clone()));
            }
        });
        ui.add_space(18.0);
        // Logos first, then colours and fonts, and the other files.
        self.files_section(
            ui,
            theme,
            texts,
            (store, window),
            &mut kit,
            FILE_SECTIONS[0],
        );
        self.colors_section(ui, theme, texts, &mut kit);
        self.extract_modal(ui.ctx(), theme, texts, &mut kit, window);
        self.styles_section(ui, theme, texts, store, &mut kit, window);
        for files in &FILE_SECTIONS[1..] {
            self.files_section(ui, theme, texts, (store, window), &mut kit, *files);
        }
        if let Some(next) = self.kit_templates(ui, theme, texts, store, &kit, window) {
            go = Some(next);
        }
        if kit != before {
            store.save(&kit);
            self.kits[index] = kit;
        }
        go
    }

    /// A section of files: their pictures, a cross to take one out, and
    /// «Añadir uno nuevo» with the file dialog.
    fn files_section(
        &mut self,
        ui: &mut egui::Ui,
        theme: &Theme,
        texts: &Texts,
        (store, window): (&dyn BrandStore, Option<&eframe::Frame>),
        kit: &mut BrandKit,
        (kind, key): (BrandFiles, &'static str),
    ) {
        self.jump_here(ui, key);
        let count = kit.files(kind).len();
        let title = format!("{} ({count})", t(texts, key));
        let add = section(ui, theme, &title, Some(&t(texts, "brand.add_new")), |ui| {
            let mut remove = None;
            if kit.files(kind).is_empty() {
                drop_zone(ui, theme, &t(texts, "brand.add_files_hint"))
            } else {
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = Vec2::splat(12.0);
                    for (i, path) in kit.files(kind).iter().enumerate() {
                        if file_tile(ui, theme, path, 120.0, true).1 {
                            remove = Some(i);
                        }
                    }
                });
                if let Some(i) = remove {
                    kit.files_mut(kind).remove(i);
                }
                false
            }
        });
        if add
            && let Some(files) = crate::files::picker(window)
                .add_filter(t(texts, "brand.images"), &IMAGE_FILES)
                .pick_files()
        {
            for file in files {
                if let Some(copied) = store.add_file(&kit.id, kind, &file) {
                    kit.files_mut(kind).push(copied);
                }
            }
        }
    }

    /// The palettes: their names, their colours with a picker each, «+».
    fn colors_section(
        &mut self,
        ui: &mut egui::Ui,
        theme: &Theme,
        texts: &Texts,
        kit: &mut BrandKit,
    ) {
        self.jump_here(ui, "brand.colors");
        let title = format!("{} ({})", t(texts, "brand.colors"), kit.colors().len());
        let add = section(ui, theme, &title, Some(&t(texts, "brand.add_new")), |ui| {
            if kit.palettes.is_empty() {
                ui.label(
                    RichText::new(t(texts, "brand.no_colors"))
                        .font(theme.font("body-sm"))
                        .color(theme.c("text.muted")),
                );
                return false;
            }
            let mut remove_palette = None;
            for (p, palette) in kit.palettes.iter_mut().enumerate() {
                egui::Frame::NONE
                    .fill(theme.c("surface.sunken"))
                    .corner_radius(10.0)
                    .inner_margin(egui::Margin::same(14))
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.horizontal(|ui| {
                            ui.add(
                                egui::TextEdit::singleline(&mut palette.name)
                                    .font(theme.font("ui-label"))
                                    .frame(false)
                                    .desired_width(260.0),
                            );
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                if widgets::secondary(ui, theme, t(texts, "brand.delete_palette"))
                                    .clicked()
                                {
                                    remove_palette = Some(p);
                                }
                                if widgets::secondary(ui, theme, t(texts, "brand.extract"))
                                    .clicked()
                                {
                                    self.color = None;
                                    self.extract = Some(Extract {
                                        palette: p,
                                        ..Extract::default()
                                    });
                                }
                            });
                        });
                        ui.add_space(8.0);
                        ui.horizontal_wrapped(|ui| {
                            ui.spacing_mut().item_spacing = Vec2::splat(10.0);
                            ui.style_mut().wrap_mode = None;
                            for (c, color) in palette.colors.iter().enumerate() {
                                let on = self
                                    .color
                                    .as_ref()
                                    .is_some_and(|e| e.palette == p && e.color == c);
                                let response = ui
                                    .vertical(|ui| {
                                        let response = color_tile(ui, theme, color.rgb, on);
                                        ui.label(
                                            RichText::new(format!("#{:06x}", color.rgb))
                                                .font(theme.font("caption"))
                                                .color(theme.c("text.secondary")),
                                        );
                                        response
                                    })
                                    .inner;
                                if response.clicked() {
                                    self.color = Some(ColorEdit::of(p, c, color));
                                }
                                if on && let Some(edit) = self.color.as_mut() {
                                    edit.anchor = Some(response.rect);
                                }
                            }
                            // The «+» sits like a colour: the tile on top,
                            // a line under it where the others show HEX.
                            let response = ui
                                .vertical(|ui| {
                                    let (plus, response) =
                                        ui.allocate_exact_size(Vec2::splat(56.0), Sense::click());
                                    ui.painter().rect_filled(plus, 8.0, theme.c("surface"));
                                    ui.painter().rect_stroke(
                                        plus,
                                        8.0,
                                        Stroke::new(1.0, theme.c("border")),
                                        egui::StrokeKind::Outside,
                                    );
                                    ui.painter().text(
                                        plus.center(),
                                        egui::Align2::CENTER_CENTER,
                                        "+",
                                        theme.font("h3"),
                                        theme.c("text"),
                                    );
                                    ui.label(RichText::new(" ").font(theme.font("caption")));
                                    response
                                })
                                .inner;
                            if response
                                .on_hover_text(t(texts, "brand.add_color"))
                                .on_hover_cursor(egui::CursorIcon::PointingHand)
                                .clicked()
                            {
                                palette.colors.push(BrandColor::rgb(0xFFFFFF));
                                let last = palette.colors.len() - 1;
                                self.color = Some(ColorEdit::of(p, last, &palette.colors[last]));
                            }
                        });
                        if let Some(edit) = self.color.as_mut()
                            && edit.palette == p
                            && let Some(color) = palette.colors.get_mut(edit.color)
                        {
                            let done = color_editor(ui, theme, texts, color, edit);
                            if done == EditorEnd::Remove {
                                palette.colors.remove(edit.color);
                                self.color = None;
                            } else if done == EditorEnd::Close {
                                self.color = None;
                            }
                        }
                    });
                ui.add_space(10.0);
            }
            if let Some(p) = remove_palette {
                kit.palettes.remove(p);
                self.color = None;
            }
            false
        });
        if add {
            kit.palettes.push(BrandPalette {
                name: t(texts, "brand.palette_name"),
                colors: Vec::new(),
            });
        }
    }

    /// «Extraer colores de una imagen»: an image of the kit, or one from
    /// the computer, then its colours, the person choosing which go into
    /// the palette.
    fn extract_modal(
        &mut self,
        ctx: &egui::Context,
        theme: &Theme,
        texts: &Texts,
        kit: &mut BrandKit,
        window: Option<&eframe::Frame>,
    ) {
        let Some(extract) = self.extract.as_mut() else {
            return;
        };
        let Some(palette) = kit.palettes.get(extract.palette).cloned() else {
            self.extract = None;
            return;
        };
        let images: Vec<PathBuf> = [
            BrandFiles::Logos,
            BrandFiles::Photos,
            BrandFiles::Graphics,
            BrandFiles::Icons,
        ]
        .into_iter()
        .flat_map(|kind| kit.files(kind).iter().cloned())
        .collect();
        let mut close = false;
        let mut add = false;
        modal_of(ctx, theme, &t(texts, "brand.extract_title"), 620.0, |ui| {
            ui.label(
                RichText::new(t(texts, "brand.extract_pick"))
                    .font(theme.font("body-sm"))
                    .color(theme.c("text.secondary")),
            );
            ui.add_space(10.0);
            egui::ScrollArea::horizontal()
                .id_salt("brand.extract.images")
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 10.0;
                        for path in &images {
                            let on = extract.image.as_ref() == Some(path);
                            let (response, _) = file_tile(ui, theme, path, 84.0, false);
                            if on {
                                ui.painter().rect_stroke(
                                    response.rect,
                                    10.0,
                                    Stroke::new(3.0, theme.c("accent")),
                                    egui::StrokeKind::Inside,
                                );
                            }
                            if response
                                .on_hover_cursor(egui::CursorIcon::PointingHand)
                                .clicked()
                            {
                                extract.pick(path.clone(), &palette);
                            }
                        }
                    });
                });
            ui.add_space(10.0);
            if widgets::secondary(ui, theme, t(texts, "brand.extract_upload")).clicked()
                && let Some(path) = crate::files::picker(window)
                    .add_filter(
                        t(texts, "brand.images"),
                        &["png", "jpg", "jpeg", "webp", "avif", "svg"],
                    )
                    .pick_file()
            {
                extract.pick(path, &palette);
            }
            if extract.image.is_some() {
                ui.add_space(16.0);
                if extract.found.is_empty() {
                    ui.label(
                        RichText::new(t(texts, "brand.extract_none"))
                            .font(theme.font("body-sm"))
                            .color(theme.c("text.muted")),
                    );
                } else {
                    ui.label(
                        RichText::new(t(texts, "brand.extract_choose"))
                            .font(theme.font("ui-label"))
                            .color(theme.c("text")),
                    );
                    ui.add_space(8.0);
                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing = Vec2::splat(10.0);
                        for (rgb, chosen) in extract.found.iter().zip(extract.chosen.iter_mut()) {
                            let response = ui
                                .vertical(|ui| {
                                    let response = color_tile(ui, theme, *rgb, *chosen);
                                    if *chosen {
                                        let mark = Rect::from_center_size(
                                            response.rect.right_top() + Vec2::new(-10.0, 10.0),
                                            Vec2::splat(18.0),
                                        );
                                        ui.painter().circle_filled(
                                            mark.center(),
                                            9.0,
                                            theme.c("accent"),
                                        );
                                        ui.painter().circle_stroke(
                                            mark.center(),
                                            9.0,
                                            Stroke::new(1.5, Color32::WHITE),
                                        );
                                        // A tick, drawn: two strokes.
                                        let c = mark.center();
                                        ui.painter().line(
                                            vec![
                                                c + Vec2::new(-4.5, 0.0),
                                                c + Vec2::new(-1.5, 3.0),
                                                c + Vec2::new(4.5, -3.5),
                                            ],
                                            Stroke::new(2.0, theme.c("accent.fg")),
                                        );
                                    }
                                    ui.label(
                                        RichText::new(format!("#{rgb:06x}"))
                                            .font(theme.font("caption"))
                                            .color(theme.c("text.secondary")),
                                    );
                                    response
                                })
                                .inner;
                            if response
                                .on_hover_cursor(egui::CursorIcon::PointingHand)
                                .clicked()
                            {
                                *chosen = !*chosen;
                            }
                        }
                    });
                }
            }
            ui.add_space(18.0);
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let count = extract.chosen.iter().filter(|c| **c).count();
                let label = t(texts, "brand.extract_add").replace("{n}", &count.to_string());
                if ui
                    .add_enabled_ui(count > 0, |ui| widgets::primary(ui, theme, label))
                    .inner
                    .clicked()
                {
                    add = true;
                }
                if widgets::secondary(ui, theme, t(texts, "brand.cancel")).clicked() {
                    close = true;
                }
            });
        });
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            close = true;
        }
        if add && let Some(palette) = kit.palettes.get_mut(extract.palette) {
            for (rgb, chosen) in extract.found.iter().zip(&extract.chosen) {
                if *chosen && palette.colors.iter().all(|c| c.rgb != *rgb) {
                    palette.colors.push(BrandColor::rgb(*rgb));
                }
            }
            close = true;
        }
        if close {
            self.extract = None;
        }
    }

    /// The text styles: each drawn in itself, a pencil and a bin; the
    /// pencil opens its fields in place (font, size, bold, italic) with ✓
    /// and ✕. Under them, the fonts uploaded to the kit.
    fn styles_section(
        &mut self,
        ui: &mut egui::Ui,
        theme: &Theme,
        texts: &Texts,
        store: &dyn BrandStore,
        kit: &mut BrandKit,
        window: Option<&eframe::Frame>,
    ) {
        self.jump_here(ui, "brand.fonts");
        let uploaded = uploaded_families(kit);
        let add = section(
            ui,
            theme,
            &t(texts, "brand.fonts"),
            Some(&t(texts, "brand.add_new")),
            |ui| {
                let mut remove = None;
                let mut commit = None;
                for (i, style) in kit.styles.iter().enumerate() {
                    egui::Frame::NONE
                        .fill(theme.c("surface.sunken"))
                        .stroke(if self.style.as_ref().is_some_and(|d| d.index == i) {
                            Stroke::new(1.5, theme.c("accent"))
                        } else {
                            Stroke::NONE
                        })
                        .corner_radius(8.0)
                        .inner_margin(egui::Margin::symmetric(16, 10))
                        .show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            if let Some(draft) = self.style.as_mut().filter(|d| d.index == i) {
                                match style_editor(ui, theme, texts, draft, &uploaded) {
                                    Some(true) => commit = Some((i, draft.style.clone())),
                                    Some(false) => commit = Some((i, style.clone())),
                                    None => {}
                                }
                                return;
                            }
                            ui.horizontal(|ui| {
                                preview(ui, theme, style);
                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                    if icon_button(
                                        ui,
                                        theme,
                                        Icon::Delete,
                                        &t(texts, "brand.delete"),
                                    ) {
                                        remove = Some(i);
                                    }
                                    if icon_button(
                                        ui,
                                        theme,
                                        Icon::Drawing,
                                        &t(texts, "brand.edit"),
                                    ) {
                                        self.style = Some(StyleDraft {
                                            index: i,
                                            style: style.clone(),
                                            search: String::new(),
                                            picking: false,
                                            size: format!("{:.0}", style.size),
                                        });
                                    }
                                    ui.label(
                                        RichText::new(format!(
                                            "{} · {:.0} px",
                                            style.font, style.size
                                        ))
                                        .font(theme.font("caption"))
                                        .color(theme.c("text.muted")),
                                    );
                                });
                            });
                        });
                    ui.add_space(8.0);
                }
                if let Some((i, style)) = commit {
                    kit.styles[i] = style;
                    self.style = None;
                }
                if let Some(i) = remove {
                    kit.styles.remove(i);
                    self.style = None;
                }
                // «Administrar fuentes subidas (n)».
                let label = format!("{} ({})", t(texts, "brand.manage_fonts"), kit.fonts.len());
                if ui
                    .link(
                        RichText::new(label)
                            .font(theme.font("body-sm"))
                            .color(theme.c("link")),
                    )
                    .clicked()
                {
                    self.fonts_open = !self.fonts_open;
                }
                if self.fonts_open {
                    ui.add_space(8.0);
                    let mut drop_font = None;
                    for (k, path) in kit.fonts.iter().enumerate() {
                        ui.horizontal(|ui| {
                            let family = uploaded
                                .iter()
                                .find(|(p, _)| p == path)
                                .map_or_else(String::new, |(_, f)| f.clone());
                            ui.label(
                                RichText::new(&family)
                                    .font(theme.font("ui-label"))
                                    .color(theme.c("text")),
                            );
                            ui.label(
                                RichText::new(
                                    path.file_name()
                                        .map(|n| n.to_string_lossy().into_owned())
                                        .unwrap_or_default(),
                                )
                                .font(theme.font("caption"))
                                .color(theme.c("text.muted")),
                            );
                            if icon_button(ui, theme, Icon::Delete, &t(texts, "brand.delete")) {
                                drop_font = Some(k);
                            }
                        });
                    }
                    if let Some(k) = drop_font {
                        kit.fonts.remove(k);
                    }
                    if add_button(ui, theme, &t(texts, "brand.upload_font")).clicked()
                        && let Some(files) = crate::files::picker(window)
                            .add_filter(t(texts, "brand.font_files"), &["ttf", "otf"])
                            .pick_files()
                    {
                        for file in files {
                            if let Some(copied) = store.add_file(&kit.id, BrandFiles::Fonts, &file)
                            {
                                kit.fonts.push(copied);
                            }
                        }
                    }
                }
                false
            },
        );
        if add {
            kit.styles.push(BrandTextStyle {
                name: t(texts, "brand.style_name"),
                font: "Inter".to_owned(),
                size: 18.0,
                bold: false,
                italic: false,
            });
            let index = kit.styles.len() - 1;
            self.style = Some(StyleDraft {
                index,
                style: kit.styles[index].clone(),
                search: String::new(),
                picking: true,
                size: format!("{:.0}", kit.styles[index].size),
            });
        }
    }

    /// The brand templates of the kit, «Subir plantilla» and a click to use
    /// one.
    fn kit_templates(
        &mut self,
        ui: &mut egui::Ui,
        theme: &Theme,
        texts: &Texts,
        store: &dyn BrandStore,
        kit: &BrandKit,
        window: Option<&eframe::Frame>,
    ) -> Option<Go> {
        self.jump_here(ui, "brand.templates");
        let listings = store.templates(&kit.id);
        let title = format!("{} ({})", t(texts, "brand.templates"), listings.len());
        let mut go = None;
        let mut remove = None;
        let upload = section(
            ui,
            theme,
            &title,
            Some(&t(texts, "brand.upload_template")),
            |ui| {
                if listings.is_empty() {
                    return drop_zone(ui, theme, &t(texts, "brand.no_templates"));
                }
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = Vec2::splat(14.0);
                    for listing in &listings {
                        match self.template_tile(ui, theme, texts, store, listing) {
                            Some(true) => go = Some(Go::FromTemplate(listing.path.clone())),
                            Some(false) => remove = Some(listing.path.clone()),
                            None => {}
                        }
                    }
                });
                false
            },
        );
        if let Some(path) = remove {
            store.remove_template(&path);
            self.thumbs.remove(&path);
        }
        if upload
            && let Some(file) = crate::files::picker(window)
                .add_filter("Gafira", &["gaf"])
                .pick_file()
        {
            let name = file
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_default();
            let meta = brand::template_meta(&kit.id, &name, None);
            if let Err(error) = store.add_template(&kit.id, &file, &meta) {
                eprintln!("brand template: {error}");
                self.notice = Some(t(texts, "brand.template_failed"));
            }
        }
        if let Some(notice) = &self.notice {
            ui.label(
                RichText::new(notice)
                    .font(theme.font("body-sm"))
                    .color(theme.c("error.fg")),
            );
        }
        go
    }

    /// One brand template: its thumbnail, its name, «Usar» and a cross;
    /// `Some(true)` to use it, `Some(false)` to remove it.
    fn template_tile(
        &mut self,
        ui: &mut egui::Ui,
        theme: &Theme,
        texts: &Texts,
        store: &dyn BrandStore,
        listing: &crate::TemplateListing,
    ) -> Option<bool> {
        let name = listing
            .meta
            .tags
            .iter()
            .find_map(|t| t.strip_prefix("title:"))
            .unwrap_or_default()
            .to_owned();
        let thumb = self
            .thumbs
            .entry(listing.path.clone())
            .or_insert_with(|| {
                store.thumbnail(&listing.path).map(|thumb| {
                    let texture = ui.ctx().load_texture(
                        format!("brand-template:{}", listing.path.display()),
                        egui::ColorImage::from_rgba_unmultiplied(
                            [thumb.width as usize, thumb.height as usize],
                            &thumb.rgba,
                        ),
                        egui::TextureOptions::LINEAR,
                    );
                    (texture, (thumb.width, thumb.height))
                })
            })
            .clone();
        let mut out = None;
        ui.vertical(|ui| {
            let (tile, response) = ui.allocate_exact_size(Vec2::new(180.0, 140.0), Sense::click());
            ui.painter()
                .rect_filled(tile, 10.0, theme.c("surface.sunken"));
            if let Some((texture, (w, h))) = thumb {
                let (fw, fh) = recent::fit(w, h, (tile.width() - 12.0, tile.height() - 12.0));
                ui.painter().image(
                    texture.id(),
                    Rect::from_center_size(tile.center(), Vec2::new(fw, fh)),
                    Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    Color32::WHITE,
                );
            }
            if response.hovered() {
                ui.painter().rect_stroke(
                    tile,
                    10.0,
                    Stroke::new(2.0, theme.c("accent")),
                    egui::StrokeKind::Inside,
                );
            }
            if response
                .on_hover_text(t(texts, "brand.use_template"))
                .on_hover_cursor(egui::CursorIcon::PointingHand)
                .clicked()
            {
                out = Some(true);
            }
            ui.horizontal(|ui| {
                ui.set_width(180.0);
                ui.label(
                    RichText::new(&name)
                        .font(theme.font("body-sm"))
                        .color(theme.c("text")),
                );
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if icon_button(ui, theme, Icon::Delete, &t(texts, "brand.delete")) {
                        out = Some(false);
                    }
                });
            });
        });
        out
    }

    /// Every brand template of every kit, by kit.
    fn templates_view(
        &mut self,
        ui: &mut egui::Ui,
        theme: &Theme,
        texts: &Texts,
        store: &dyn BrandStore,
    ) -> Option<Go> {
        title_row(ui, theme, &t(texts, "brand.templates"), None);
        let kits = self.kits.clone();
        let mut any = false;
        let mut go = None;
        for kit in &kits {
            let listings = store.templates(&kit.id);
            if listings.is_empty() {
                continue;
            }
            any = true;
            ui.label(
                RichText::new(&kit.name)
                    .font(theme.font("h3"))
                    .color(theme.c("text")),
            );
            ui.add_space(10.0);
            let mut remove = None;
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = Vec2::splat(14.0);
                for listing in &listings {
                    match self.template_tile(ui, theme, texts, store, listing) {
                        Some(true) => go = Some(Go::FromTemplate(listing.path.clone())),
                        Some(false) => remove = Some(listing.path.clone()),
                        None => {}
                    }
                }
            });
            if let Some(path) = remove {
                store.remove_template(&path);
                self.thumbs.remove(&path);
            }
            ui.add_space(24.0);
        }
        if !any {
            empty_state(
                ui,
                theme,
                &t(texts, "brand.no_templates_title"),
                &t(texts, "brand.no_templates"),
            );
        }
        go
    }

    /// «Crear kit de marca»: its name, Cancelar and Crear.
    fn create_modal(
        &mut self,
        ctx: &egui::Context,
        theme: &Theme,
        texts: &Texts,
        store: &dyn BrandStore,
    ) {
        let Some(name) = self.creating.as_mut() else {
            return;
        };
        let mut close = false;
        let mut create = false;
        modal(ctx, theme, &t(texts, "brand.create_title"), |ui| {
            ui.label(
                RichText::new(t(texts, "brand.name"))
                    .font(theme.font("ui-label"))
                    .color(theme.c("text")),
            );
            ui.add_space(6.0);
            let field = ui.add(widgets::field(name, t(texts, "brand.name_hint")));
            field.request_focus();
            let enter = field.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
            ui.add_space(18.0);
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let ready = !name.trim().is_empty();
                if ui
                    .add_enabled_ui(ready, |ui| {
                        widgets::primary(ui, theme, t(texts, "brand.create"))
                    })
                    .inner
                    .clicked()
                    || (enter && ready)
                {
                    create = true;
                }
                if widgets::secondary(ui, theme, t(texts, "brand.cancel")).clicked() {
                    close = true;
                }
            });
        });
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            close = true;
        }
        if create {
            let kit = BrandKit::with_styles(
                brand::new_id(&self.kits),
                name.trim().to_owned(),
                [
                    &t(texts, "brand.style_title"),
                    &t(texts, "brand.style_subtitle"),
                    &t(texts, "brand.style_heading"),
                    &t(texts, "brand.style_body"),
                ],
            );
            store.save(&kit);
            store.set_active(&kit.id);
            self.view = View::Kit(kit.id.clone());
            self.kits = store.kits();
            close = true;
        }
        if close {
            self.creating = None;
        }
    }

    /// A new name for a kit.
    fn rename_modal(
        &mut self,
        ctx: &egui::Context,
        theme: &Theme,
        texts: &Texts,
        store: &dyn BrandStore,
    ) {
        let Some((id, name)) = self.renaming.as_mut() else {
            return;
        };
        let mut close = ctx.input(|i| i.key_pressed(egui::Key::Escape));
        let mut save = false;
        modal(ctx, theme, &t(texts, "brand.rename"), |ui| {
            ui.add(widgets::field(name, t(texts, "brand.name_hint")));
            ui.add_space(18.0);
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui
                    .add_enabled_ui(!name.trim().is_empty(), |ui| {
                        widgets::primary(ui, theme, t(texts, "brand.save"))
                    })
                    .inner
                    .clicked()
                {
                    save = true;
                }
                if widgets::secondary(ui, theme, t(texts, "brand.cancel")).clicked() {
                    close = true;
                }
            });
        });
        if save && let Some(kit) = self.kits.iter_mut().find(|k| &k.id == id) {
            kit.name = name.trim().to_owned();
            store.save(kit);
            close = true;
        }
        if close {
            self.renaming = None;
        }
    }
}

/// The sections of a kit page and their keys, in their order.
fn sections() -> [(&'static str, ()); 7] {
    [
        ("brand.logos", ()),
        ("brand.colors", ()),
        ("brand.fonts", ()),
        ("brand.photos", ()),
        ("brand.graphics", ()),
        ("brand.icons", ()),
        ("brand.templates", ()),
    ]
}

/// The document style of a kit's text style.
pub(crate) fn span_style(style: &BrandTextStyle) -> gafira_doc::SpanStyle {
    gafira_doc::SpanStyle {
        font: style.font.clone(),
        size: style.size,
        color: crate::colors::hex(0x18243B),
        bold: style.bold,
        italic: style.italic,
    }
}

/// A big title with an action on its right; `true` when it was clicked.
fn title_row(ui: &mut egui::Ui, theme: &Theme, title: &str, action: Option<&str>) -> bool {
    let mut clicked = false;
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(title)
                .font(egui::FontId::new(
                    30.0,
                    egui::FontFamily::Name(DISPLAY.into()),
                ))
                .color(theme.c("text")),
        );
        if let Some(action) = action {
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                clicked = add_button(ui, theme, action).clicked();
            });
        }
    });
    ui.add_space(18.0);
    clicked
}

/// «+ Añadir uno nuevo»: the secondary button of the interface.
fn add_button(ui: &mut egui::Ui, theme: &Theme, label: &str) -> egui::Response {
    widgets::secondary(ui, theme, format!("+  {label}"))
        .on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// A section of a kit page: its title and action, and `add` under them;
/// `true` when the action, or `add`, asked for a new item.
fn section(
    ui: &mut egui::Ui,
    theme: &Theme,
    title: &str,
    action: Option<&str>,
    add: impl FnOnce(&mut egui::Ui) -> bool,
) -> bool {
    let mut clicked = false;
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(title)
                .font(theme.font("h3"))
                .color(theme.c("text")),
        );
        if let Some(action) = action {
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                clicked = add_button(ui, theme, action).clicked();
            });
        }
    });
    ui.add_space(10.0);
    if add(ui) {
        clicked = true;
    }
    ui.add_space(26.0);
    clicked
}

/// A dashed zone that says how to add the first item; `true` when clicked.
fn drop_zone(ui: &mut egui::Ui, theme: &Theme, text: &str) -> bool {
    let (rect, response) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), 72.0), Sense::click());
    let hover = response.hovered();
    ui.painter().rect_filled(
        rect,
        10.0,
        if hover {
            theme.c("selection.bg")
        } else {
            theme.c("surface.sunken")
        },
    );
    let stroke = Stroke::new(1.0, theme.c("border.strong"));
    let dash = egui::Shape::dashed_line(
        &[
            rect.left_top(),
            rect.right_top(),
            rect.right_bottom(),
            rect.left_bottom(),
            rect.left_top(),
        ],
        stroke,
        6.0,
        4.0,
    );
    ui.painter().extend(dash);
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        text,
        theme.font("body-sm"),
        theme.c("text.secondary"),
    );
    response
        .on_hover_cursor(egui::CursorIcon::PointingHand)
        .clicked()
}

/// The state when there is nothing yet: a title and a line.
fn empty_state(ui: &mut egui::Ui, theme: &Theme, title: &str, line: &str) {
    egui::Frame::NONE
        .stroke(Stroke::new(1.0, theme.c("border")))
        .corner_radius(12.0)
        .inner_margin(egui::Margin::same(32))
        .show(ui, |ui| {
            ui.set_width(ui.available_width().min(560.0));
            ui.vertical_centered(|ui| {
                let (rect, _) = ui.allocate_exact_size(Vec2::splat(56.0), Sense::hover());
                icons::paint(
                    ui.painter(),
                    rect,
                    Icon::Identity,
                    Weight::Fill,
                    theme.c("accent"),
                );
                ui.add_space(10.0);
                ui.label(
                    RichText::new(title)
                        .font(theme.font("h3"))
                        .color(theme.c("text")),
                );
                ui.label(
                    RichText::new(line)
                        .font(theme.font("body-sm"))
                        .color(theme.c("text.secondary")),
                );
            });
        });
}

/// What a click on a kit card asks.
enum CardAction {
    Open,
    Rename,
    Delete,
}

/// A kit: its first logo (or its name) on a soft tile, its colours in a
/// strip under it, its name and «…» with Renombrar and Eliminar.
fn kit_card(
    ui: &mut egui::Ui,
    theme: &Theme,
    texts: &Texts,
    kit: &BrandKit,
    width: f32,
) -> Option<CardAction> {
    let mut action = None;
    ui.vertical(|ui| {
        ui.set_width(width);
        let height = (width * 0.5).round();
        let (tile, response) = ui.allocate_exact_size(Vec2::new(width, height), Sense::click());
        ui.painter()
            .rect_filled(tile, 10.0, theme.c("surface.sunken"));
        let picture = Rect::from_min_max(
            tile.min + Vec2::splat(16.0),
            egui::pos2(tile.max.x - 16.0, tile.max.y - 30.0),
        );
        match kit
            .logos
            .first()
            .and_then(|logo| file_image(ui.ctx(), logo))
        {
            Some(image) => {
                paint_fitted(ui, image, picture);
            }
            None => {
                ui.painter().text(
                    picture.center(),
                    egui::Align2::CENTER_CENTER,
                    &kit.name,
                    theme.font("ui-label"),
                    theme.c("text.secondary"),
                );
            }
        }
        let colors = kit.colors();
        let strip = Rect::from_min_max(egui::pos2(tile.left(), tile.bottom() - 14.0), tile.max);
        if colors.is_empty() {
            ui.painter().rect_filled(
                strip,
                egui::CornerRadius {
                    nw: 0,
                    ne: 0,
                    sw: 10,
                    se: 10,
                },
                theme.c("border"),
            );
        } else {
            let w = strip.width() / colors.len() as f32;
            for (i, c) in colors.iter().enumerate() {
                let r = Rect::from_min_size(
                    egui::pos2(strip.left() + w * i as f32, strip.top()),
                    Vec2::new(w, strip.height()),
                );
                ui.painter().rect_filled(r, 0.0, rgb(*c));
            }
        }
        if response.hovered() {
            ui.painter().rect_stroke(
                tile,
                10.0,
                Stroke::new(2.0, theme.c("accent")),
                egui::StrokeKind::Inside,
            );
        }
        if response
            .on_hover_cursor(egui::CursorIcon::PointingHand)
            .clicked()
        {
            action = Some(CardAction::Open);
        }
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(&kit.name)
                    .font(theme.font("ui-label"))
                    .color(theme.c("text")),
            );
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.menu_button("···", |ui| {
                    if ui.button(t(texts, "brand.open")).clicked() {
                        action = Some(CardAction::Open);
                        ui.close();
                    }
                    if ui.button(t(texts, "brand.rename")).clicked() {
                        action = Some(CardAction::Rename);
                        ui.close();
                    }
                    if ui.button(t(texts, "brand.delete")).clicked() {
                        action = Some(CardAction::Delete);
                        ui.close();
                    }
                });
            });
        });
    });
    action
}

/// The egui colour of `0xRRGGBB`.
pub(crate) fn rgb(c: u32) -> Color32 {
    Color32::from_rgb((c >> 16) as u8, (c >> 8) as u8, c as u8)
}

/// The picture of the file at `path`, read once and kept for the session:
/// egui's loaders read it from its bytes (no file loader is built in).
/// `None` when the file cannot be read.
pub(crate) fn file_image(ctx: &egui::Context, path: &Path) -> Option<egui::Image<'static>> {
    type Cache = std::collections::HashMap<PathBuf, Option<std::sync::Arc<[u8]>>>;
    let id = egui::Id::new("brand.file_bytes");
    let bytes = ctx.data_mut(|d| {
        d.get_temp_mut_or_default::<Cache>(id)
            .entry(path.to_path_buf())
            .or_insert_with(|| std::fs::read(path).ok().map(std::sync::Arc::from))
            .clone()
    })?;
    // The extension stays at the end of the URI: it picks the loader.
    let uri = format!("bytes://{}", path.display());
    Some(egui::Image::from_bytes(
        uri,
        egui::load::Bytes::Shared(bytes),
    ))
}

/// `image` as large as fits in `rect`, centred, its proportions kept
/// (a photo or an SVG is never stretched to the tile).
pub(crate) fn paint_fitted(ui: &egui::Ui, image: egui::Image<'_>, rect: Rect) {
    let size = image
        .load_for_size(ui.ctx(), rect.size())
        .ok()
        .and_then(|poll| poll.size())
        .filter(|s| s.x > 0.0 && s.y > 0.0);
    let at = size.map_or(rect, |s| {
        Rect::from_center_size(
            rect.center(),
            s * (rect.width() / s.x).min(rect.height() / s.y),
        )
    });
    image.paint_at(ui, at);
}

/// A file of a kit: its picture on a tile and, when `removable`, a cross on
/// hover; the response and whether the cross was clicked. The cross shows
/// while the pointer is anywhere on the tile, the cross included, so it
/// stays put to be clicked.
pub(crate) fn file_tile(
    ui: &mut egui::Ui,
    theme: &Theme,
    path: &Path,
    side: f32,
    removable: bool,
) -> (egui::Response, bool) {
    let (tile, response) = ui.allocate_exact_size(Vec2::splat(side), Sense::click());
    ui.painter()
        .rect_filled(tile, 10.0, theme.c("surface.sunken"));
    let inner = tile.shrink(10.0);
    match file_image(ui.ctx(), path) {
        Some(image) => paint_fitted(ui, image, inner),
        None => {
            ui.painter().text(
                inner.center(),
                egui::Align2::CENTER_CENTER,
                path.file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default(),
                theme.font("caption"),
                theme.c("text.muted"),
            );
        }
    }
    let over = ui.rect_contains_pointer(tile);
    let mut removed = false;
    if over {
        ui.painter().rect_stroke(
            tile,
            10.0,
            Stroke::new(2.0, theme.c("accent")),
            egui::StrokeKind::Inside,
        );
    }
    if over && removable {
        let cross =
            Rect::from_center_size(tile.right_top() + Vec2::new(-14.0, 14.0), Vec2::splat(24.0));
        let hit = ui
            .interact(cross, response.id.with("remove"), Sense::click())
            .on_hover_cursor(egui::CursorIcon::PointingHand);
        let fill = if hit.hovered() {
            theme.c("error.bg")
        } else {
            theme.c("surface")
        };
        ui.painter().circle_filled(cross.center(), 12.0, fill);
        ui.painter()
            .circle_stroke(cross.center(), 12.0, Stroke::new(1.0, theme.c("border")));
        icons::paint(
            ui.painter(),
            Rect::from_center_size(cross.center(), Vec2::splat(14.0)),
            Icon::Delete,
            Weight::Regular,
            if hit.hovered() {
                theme.c("error.fg")
            } else {
                theme.c("text")
            },
        );
        removed = hit.clicked();
    }
    (response, removed)
}

/// A colour of a palette, ringed when it is the one being edited.
fn color_tile(ui: &mut egui::Ui, theme: &Theme, c: u32, on: bool) -> egui::Response {
    let (tile, response) = ui.allocate_exact_size(Vec2::splat(56.0), Sense::click());
    ui.painter().rect_filled(tile, 8.0, rgb(c));
    let stroke = if on {
        Stroke::new(3.0, theme.c("accent"))
    } else {
        Stroke::new(1.0, theme.c("border"))
    };
    ui.painter()
        .rect_stroke(tile, 8.0, stroke, egui::StrokeKind::Outside);
    response
        .on_hover_text(format!("#{c:06X}"))
        .on_hover_cursor(egui::CursorIcon::PointingHand)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum EditorEnd {
    Open,
    Close,
    Remove,
}

/// The picker of one colour, floating under its tile so the card keeps
/// its height: the saturation and brightness square, the hue bar, HEX and,
/// with «Añadir CMYK», its CMYK for print; Listo and Eliminar. A click
/// outside or Esc closes it.
fn color_editor(
    ui: &mut egui::Ui,
    theme: &Theme,
    texts: &Texts,
    color: &mut BrandColor,
    edit: &mut ColorEdit,
) -> EditorEnd {
    let mut end = EditorEnd::Open;
    let Some(anchor) = edit.anchor else {
        return end;
    };
    let area = egui::Area::new(
        ui.id()
            .with(("brand.color.picker", edit.palette, edit.color)),
    )
    .order(egui::Order::Foreground)
    .fixed_pos(anchor.left_bottom() + Vec2::new(0.0, 8.0))
    .show(ui.ctx(), |ui| {
        egui::Frame::NONE
            .fill(theme.c("surface"))
            .stroke(Stroke::new(1.0, theme.c("border")))
            .corner_radius(12.0)
            .shadow(egui::epaint::Shadow {
                offset: [0, 6],
                blur: 20,
                spread: 0,
                color: Color32::from_black_alpha(45),
            })
            .inner_margin(egui::Margin::same(14))
            .show(ui, |ui| {
                let width = 260.0;
                ui.set_width(width);
                let before = edit.hsv;
                sv_square(ui, &mut edit.hsv, Vec2::new(width, 150.0));
                ui.add_space(10.0);
                hue_bar(ui, &mut edit.hsv, Vec2::new(width, 14.0));
                if edit.hsv != before {
                    let [r, g, b] =
                        egui::ecolor::Hsva::new(edit.hsv[0], edit.hsv[1], edit.hsv[2], 1.0)
                            .to_srgb();
                    color.rgb = (u32::from(r) << 16) | (u32::from(g) << 8) | u32::from(b);
                    edit.hex = format!("#{:06X}", color.rgb);
                }
                ui.add_space(12.0);
                let hex = ui.add(
                    widgets::field(&mut edit.hex, "#RRGGBB".to_owned())
                        .horizontal_align(Align::Center),
                );
                if hex.changed()
                    && let Some(code) = brand::parse_hex(&edit.hex)
                {
                    color.rgb = code;
                    let h = egui::ecolor::Hsva::from_srgb(rgb_bytes(code));
                    edit.hsv = [h.h, h.s, h.v];
                }
                ui.add_space(10.0);
                match edit.cmyk.as_mut() {
                    None => {
                        if widgets::secondary(
                            ui,
                            theme,
                            format!("+  {}", t(texts, "brand.add_cmyk")),
                        )
                        .clicked()
                        {
                            edit.cmyk = Some(String::new());
                        }
                    }
                    Some(text) => {
                        ui.label(
                            RichText::new("CMYK")
                                .font(theme.font("caption"))
                                .color(theme.c("text.secondary")),
                        );
                        ui.horizontal(|ui| {
                            let (dot, _) =
                                ui.allocate_exact_size(Vec2::splat(22.0), Sense::hover());
                            ui.painter().rect_filled(dot, 5.0, rgb(color.rgb));
                            let changed = ui
                                .add(widgets::field(text, "0,11,18,1".to_owned()))
                                .changed();
                            if changed && let Some(values) = parse_cmyk(text) {
                                color.cmyk = Some(values);
                                color.rgb = BrandColor::rgb_of_cmyk(values);
                                edit.hex = format!("#{:06X}", color.rgb);
                                let h = egui::ecolor::Hsva::from_srgb(rgb_bytes(color.rgb));
                                edit.hsv = [h.h, h.s, h.v];
                            }
                        });
                        if widgets::secondary(ui, theme, t(texts, "brand.remove_cmyk")).clicked() {
                            color.cmyk = None;
                            edit.cmyk = None;
                        }
                    }
                }
                ui.add_space(12.0);
                ui.horizontal(|ui| {
                    if widgets::primary(ui, theme, t(texts, "brand.done")).clicked() {
                        end = EditorEnd::Close;
                    }
                    if widgets::secondary(ui, theme, t(texts, "brand.delete")).clicked() {
                        end = EditorEnd::Remove;
                    }
                });
            });
    });
    let outside = ui.input(|i| {
        i.pointer.any_click()
            && i.pointer
                .interact_pos()
                .is_some_and(|p| !area.response.rect.contains(p) && !anchor.contains(p))
    });
    if end == EditorEnd::Open && (outside || ui.input(|i| i.key_pressed(egui::Key::Escape))) {
        end = EditorEnd::Close;
    }
    end
}

/// The saturation (across) and brightness (down) of the hue in `hsv`,
/// with a ring where it is; a press or a drag picks.
fn sv_square(ui: &mut egui::Ui, hsv: &mut [f32; 3], size: Vec2) {
    let (rect, response) = ui.allocate_exact_size(size, Sense::click_and_drag());
    let hue = Color32::from(egui::ecolor::Hsva::new(hsv[0], 1.0, 1.0, 1.0));
    let mut mesh = egui::Mesh::default();
    // White to the hue across, then a black wash down.
    for (pos, color) in [
        (rect.left_top(), Color32::WHITE),
        (rect.right_top(), hue),
        (rect.right_bottom(), hue),
        (rect.left_bottom(), Color32::WHITE),
    ] {
        mesh.colored_vertex(pos, color);
    }
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(0, 2, 3);
    for (pos, color) in [
        (rect.left_top(), Color32::TRANSPARENT),
        (rect.right_top(), Color32::TRANSPARENT),
        (rect.right_bottom(), Color32::BLACK),
        (rect.left_bottom(), Color32::BLACK),
    ] {
        mesh.colored_vertex(pos, color);
    }
    mesh.add_triangle(4, 5, 6);
    mesh.add_triangle(4, 6, 7);
    ui.painter().add(egui::Shape::mesh(mesh));
    if let Some(p) = response.interact_pointer_pos() {
        hsv[1] = ((p.x - rect.left()) / rect.width()).clamp(0.0, 1.0);
        hsv[2] = 1.0 - ((p.y - rect.top()) / rect.height()).clamp(0.0, 1.0);
    }
    let at = egui::pos2(
        rect.left() + hsv[1] * rect.width(),
        rect.top() + (1.0 - hsv[2]) * rect.height(),
    );
    ui.painter()
        .circle_stroke(at, 7.0, Stroke::new(2.5, Color32::WHITE));
    ui.painter()
        .circle_stroke(at, 8.5, Stroke::new(1.0, Color32::from_black_alpha(90)));
    response.on_hover_cursor(egui::CursorIcon::Crosshair);
}

/// The hue bar, with a handle where the hue is; a press or a drag picks.
fn hue_bar(ui: &mut egui::Ui, hsv: &mut [f32; 3], size: Vec2) {
    let (rect, response) = ui.allocate_exact_size(size, Sense::click_and_drag());
    let steps = 36;
    let mut mesh = egui::Mesh::default();
    for k in 0..=steps {
        let f = k as f32 / steps as f32;
        let x = rect.left() + f * rect.width();
        let c = Color32::from(egui::ecolor::Hsva::new(f, 1.0, 1.0, 1.0));
        mesh.colored_vertex(egui::pos2(x, rect.top()), c);
        mesh.colored_vertex(egui::pos2(x, rect.bottom()), c);
        if k > 0 {
            let i = (2 * k) as u32;
            mesh.add_triangle(i - 2, i - 1, i);
            mesh.add_triangle(i - 1, i + 1, i);
        }
    }
    ui.painter().add(egui::Shape::mesh(mesh));
    if let Some(p) = response.interact_pointer_pos() {
        hsv[0] = ((p.x - rect.left()) / rect.width()).clamp(0.0, 0.999);
    }
    let x = rect.left() + hsv[0] * rect.width();
    ui.painter().circle_filled(
        egui::pos2(x, rect.center().y),
        rect.height() * 0.6,
        Color32::WHITE,
    );
    ui.painter().circle_stroke(
        egui::pos2(x, rect.center().y),
        rect.height() * 0.6,
        Stroke::new(1.0, Color32::from_black_alpha(90)),
    );
    response.on_hover_cursor(egui::CursorIcon::PointingHand);
}

/// A style being edited: a copy that becomes the style with Listo.
pub(crate) struct StyleDraft {
    index: usize,
    style: BrandTextStyle,
    search: String,
    picking: bool,
    /// The size as typed.
    size: String,
}

/// The sizes the size list offers, in px.
const SIZES: [f64; 18] = [
    8.0, 10.0, 12.0, 14.0, 16.0, 18.0, 20.0, 24.0, 28.0, 32.0, 36.0, 40.0, 48.0, 56.0, 64.0, 72.0,
    96.0, 120.0,
];

/// The size of a style: a white field to type it in and an arrow that
/// opens the usual sizes.
fn size_combo(ui: &mut egui::Ui, theme: &Theme, texts: &Texts, draft: &mut StyleDraft) {
    let field = ui.add(
        widgets::field(&mut draft.size, "px".to_owned())
            .desired_width(56.0)
            .background_color(theme.c("surface"))
            .horizontal_align(Align::Center),
    );
    if field.changed()
        && let Ok(size) = draft.size.trim().replace(',', ".").parse::<f64>()
        && (6.0..=400.0).contains(&size)
    {
        draft.style.size = size;
    }
    if field.lost_focus() {
        draft.size = format!("{:.0}", draft.style.size);
    }
    let (arrow, response) = ui.allocate_exact_size(Vec2::new(28.0, 34.0), Sense::click());
    ui.painter().rect(
        arrow,
        8.0,
        theme.c("surface"),
        Stroke::new(1.0, theme.c("border")),
        egui::StrokeKind::Inside,
    );
    let c = arrow.center();
    ui.painter().add(egui::Shape::convex_polygon(
        vec![
            c + Vec2::new(-5.0, -2.5),
            c + Vec2::new(5.0, -2.5),
            c + Vec2::new(0.0, 3.5),
        ],
        theme.c("text"),
        Stroke::NONE,
    ));
    let response = response
        .on_hover_text(t(texts, "brand.size"))
        .on_hover_cursor(egui::CursorIcon::PointingHand);
    egui::Popup::menu(&response).show(|ui| {
        ui.set_min_width(90.0);
        egui::ScrollArea::vertical()
            .max_height(260.0)
            .show(ui, |ui| {
                for size in SIZES {
                    let on = (draft.style.size - size).abs() < 0.5;
                    if ui.selectable_label(on, format!("{size:.0} px")).clicked() {
                        draft.style.size = size;
                        draft.size = format!("{size:.0}");
                    }
                }
            });
    });
}

/// The families of the fonts uploaded to `kit`, by file.
fn uploaded_families(kit: &BrandKit) -> Vec<(PathBuf, String)> {
    let added = crate::system_fonts::catalogue().added();
    kit.fonts
        .iter()
        .filter_map(|path| {
            added
                .iter()
                .find(|f| &f.path == path)
                .map(|f| (path.clone(), f.family.clone()))
        })
        .collect()
}

/// A style drawn in its own font, size (up to 120 px here) and slant.
fn preview(ui: &mut egui::Ui, theme: &Theme, style: &BrandTextStyle) {
    let family = crate::editor::text_family(ui.ctx(), theme, &span_style(style));
    let size = (style.size as f32).clamp(12.0, 120.0);
    let mut text = RichText::new(&style.name)
        .font(egui::FontId::new(size, family))
        .color(theme.c("text"));
    if style.italic {
        text = text.italics();
    }
    ui.label(text);
}

/// A small icon button; `true` when clicked.
fn icon_button(ui: &mut egui::Ui, theme: &Theme, icon: Icon, name: &str) -> bool {
    widgets::IconButton::new(icon, name.to_owned())
        .tooltip(name.to_owned())
        .small()
        .show(ui, theme)
        .clicked()
}

/// The fields of a style in place: «Escoge una fuente», its name, size, B
/// and I, Listo and Cancelar; the style shows in its row as it changes.
/// The fonts open in a floating list under the button (the uploaded ones
/// first, then Gafira's), so the page does not move. `Some(true)` keeps
/// the changes, `Some(false)` drops them.
fn style_editor(
    ui: &mut egui::Ui,
    theme: &Theme,
    texts: &Texts,
    draft: &mut StyleDraft,
    uploaded: &[(PathBuf, String)],
) -> Option<bool> {
    let mut out = None;
    let control = |ui: &mut egui::Ui, text: RichText, on: bool, width: f32| {
        ui.add(
            egui::Button::new(text.color(theme.c("text")))
                .fill(if on {
                    theme.c("selection.bg")
                } else {
                    theme.c("surface")
                })
                .stroke(Stroke::new(
                    1.0,
                    if on {
                        theme.c("accent")
                    } else {
                        theme.c("border")
                    },
                ))
                .corner_radius(8.0)
                .min_size(Vec2::new(width, 34.0)),
        )
        .on_hover_cursor(egui::CursorIcon::PointingHand)
    };
    let row = ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        let font = control(
            ui,
            RichText::new(&draft.style.font).font(theme.font("ui-label")),
            draft.picking,
            200.0,
        )
        .on_hover_text(t(texts, "brand.font"));
        if font.clicked() {
            draft.picking = !draft.picking;
        }
        ui.add(
            widgets::field(&mut draft.style.name, t(texts, "brand.name"))
                .desired_width(170.0)
                .background_color(theme.c("surface")),
        );
        size_combo(ui, theme, texts, draft);
        if widgets::IconButton::new(Icon::Bold, t(texts, "brand.bold"))
            .active(draft.style.bold)
            .tooltip(t(texts, "brand.bold"))
            .show(ui, theme)
            .clicked()
        {
            draft.style.bold = !draft.style.bold;
        }
        if widgets::IconButton::new(Icon::Italic, t(texts, "brand.italic"))
            .active(draft.style.italic)
            .tooltip(t(texts, "brand.italic"))
            .show(ui, theme)
            .clicked()
        {
            draft.style.italic = !draft.style.italic;
        }
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if widgets::secondary(ui, theme, t(texts, "brand.cancel")).clicked() {
                out = Some(false);
            }
            if widgets::primary(ui, theme, t(texts, "brand.done")).clicked() {
                out = Some(true);
            }
        });
        font.rect
    });
    let button = row.inner;
    ui.add_space(8.0);
    preview(ui, theme, &draft.style);
    if !draft.picking {
        return out;
    }
    // The floating list of fonts.
    let list = egui::Area::new(ui.id().with(("brand.fonts.list", draft.index)))
        .order(egui::Order::Foreground)
        .fixed_pos(button.left_bottom() + Vec2::new(0.0, 6.0))
        .show(ui.ctx(), |ui| {
            egui::Frame::NONE
                .fill(theme.c("surface"))
                .stroke(Stroke::new(1.0, theme.c("border")))
                .corner_radius(10.0)
                .shadow(egui::epaint::Shadow {
                    offset: [0, 4],
                    blur: 16,
                    spread: 0,
                    color: Color32::from_black_alpha(40),
                })
                .inner_margin(egui::Margin::same(12))
                .show(ui, |ui| {
                    ui.set_width(340.0);
                    ui.add(widgets::field(
                        &mut draft.search,
                        t(texts, "brand.font_search"),
                    ));
                    let words = draft.search.trim().to_lowercase();
                    let keep =
                        |family: &str| words.is_empty() || family.to_lowercase().contains(&words);
                    let mut own: Vec<String> = uploaded.iter().map(|(_, f)| f.clone()).collect();
                    own.sort();
                    own.dedup();
                    let mut gafira: Vec<&str> =
                        gafira_theme::fonts().iter().map(|f| f.family).collect();
                    gafira.extend(gafira_theme::library().iter().map(|f| f.family));
                    gafira.sort_unstable();
                    gafira.dedup();
                    egui::ScrollArea::vertical()
                        .max_height(300.0)
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            let mut group =
                                |ui: &mut egui::Ui, title: String, families: Vec<String>| {
                                    if families.is_empty() {
                                        return;
                                    }
                                    ui.add_space(6.0);
                                    ui.label(
                                        RichText::new(title)
                                            .font(theme.font("caption"))
                                            .color(theme.c("text.secondary")),
                                    );
                                    for family in families {
                                        let sample = BrandTextStyle {
                                            font: family.clone(),
                                            name: family.clone(),
                                            size: 18.0,
                                            bold: false,
                                            italic: false,
                                        };
                                        let face = crate::editor::text_family(
                                            ui.ctx(),
                                            theme,
                                            &span_style(&sample),
                                        );
                                        let on = draft.style.font == family;
                                        let item = ui
                                            .with_layout(
                                                Layout::top_down_justified(Align::Min),
                                                |ui| {
                                                    ui.selectable_label(
                                                        on,
                                                        RichText::new(&family)
                                                            .font(egui::FontId::new(17.0, face)),
                                                    )
                                                },
                                            )
                                            .inner;
                                        if item.clicked() {
                                            draft.style.font = family;
                                            draft.picking = false;
                                        }
                                    }
                                };
                            group(
                                ui,
                                t(texts, "brand.uploaded_fonts"),
                                own.into_iter().filter(|f| keep(f)).collect(),
                            );
                            group(
                                ui,
                                t(texts, "brand.gafira_fonts"),
                                gafira
                                    .into_iter()
                                    .filter(|f| keep(f))
                                    .map(str::to_owned)
                                    .collect(),
                            );
                        });
                });
        });
    // A click outside the list and its button closes it.
    let clicked_outside = ui.input(|i| {
        i.pointer.any_click()
            && i.pointer
                .interact_pos()
                .is_some_and(|p| !list.response.rect.contains(p) && !button.contains(p))
    });
    if clicked_outside || ui.input(|i| i.key_pressed(egui::Key::Escape)) {
        draft.picking = false;
    }
    out
}

/// A dimmed window in the middle of the screen with `title` and `add`.
pub(crate) fn modal(
    ctx: &egui::Context,
    theme: &Theme,
    title: &str,
    add: impl FnOnce(&mut egui::Ui),
) {
    modal_of(ctx, theme, title, 420.0, add);
}

/// [`modal`] `width` px wide, centred on the window whatever its height.
pub(crate) fn modal_of(
    ctx: &egui::Context,
    theme: &Theme,
    title: &str,
    width: f32,
    add: impl FnOnce(&mut egui::Ui),
) {
    let screen = ctx.content_rect();
    egui::Area::new(egui::Id::new("brand.modal.dim"))
        .order(egui::Order::Foreground)
        .fixed_pos(screen.min)
        .show(ctx, |ui| {
            let (rect, _) = ui.allocate_exact_size(screen.size(), Sense::click());
            ui.painter()
                .rect_filled(rect, 0.0, Color32::from_black_alpha(120));
        });
    let size = Vec2::new(width, 0.0);
    egui::Area::new(egui::Id::new("brand.modal"))
        .order(egui::Order::Tooltip)
        .pivot(egui::Align2::CENTER_CENTER)
        .fixed_pos(screen.center())
        .show(ctx, |ui| {
            egui::Frame::NONE
                .fill(theme.c("surface"))
                .corner_radius(14.0)
                .inner_margin(egui::Margin::same(24))
                .show(ui, |ui| {
                    ui.set_width(size.x - 48.0);
                    ui.label(
                        RichText::new(title)
                            .font(theme.font("h3"))
                            .color(theme.c("text")),
                    );
                    ui.add_space(14.0);
                    add(ui);
                });
        });
}

/// «C,M,Y,K», each part in percent.
fn cmyk_text([c, m, y, k]: [u8; 4]) -> String {
    format!("{c},{m},{y},{k}")
}

/// The four parts of «C,M,Y,K» (commas, spaces or semicolons between
/// them), each from 0 to 100.
fn parse_cmyk(text: &str) -> Option<[u8; 4]> {
    let parts: Vec<u8> = text
        .split(|c: char| c == ',' || c == ';' || c.is_whitespace())
        .filter(|p| !p.is_empty())
        .map(|p| p.parse::<u8>().ok().filter(|v| *v <= 100))
        .collect::<Option<_>>()?;
    parts.try_into().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn s93_03_cmyk_is_typed_as_four_numbers() {
        assert_eq!(parse_cmyk("0,11,18,01"), Some([0, 11, 18, 1]));
        assert_eq!(parse_cmyk(" 10; 20 30 , 40 "), Some([10, 20, 30, 40]));
        assert_eq!(parse_cmyk("0,11,18"), None, "four parts");
        assert_eq!(parse_cmyk("0,11,18,101"), None, "at most 100");
        assert_eq!(cmyk_text([0, 11, 18, 1]), "0,11,18,1");
    }
}
