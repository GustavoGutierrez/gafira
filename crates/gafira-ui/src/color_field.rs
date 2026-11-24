//! The colour field of the editor (spec 098): one component wherever a
//! colour is chosen in the Inspector (the letters of a text, the fill and the
//! outline of a shape, the background of a page), so they all work alike:
//!
//! - the title, and «Sin …» when the colour can be taken away;
//! - the picker with its eyedropper ([`crate::eyedropper::color_edit`]) and
//!   «Color personalizado»;
//! - the colours of the brand kit in use and those the design already uses;
//! - the preset colours, solid and (where they apply) gradients, with «Ver
//!   todo» ([`crate::presets`]).

use egui::{RichText, Ui, Vec2};

use crate::canvas;
use crate::charts::plain;
use crate::presets::{self, Fill, Target};
use crate::style::Theme;
use crate::text::Texts;

/// What was chosen in a colour field.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Picked {
    Fill(Fill),
    /// «Sin relleno», «Sin contorno».
    Clear,
}

/// A colour field; see the module.
pub(crate) struct ColorField<'a> {
    title: String,
    current: Option<Fill>,
    target: Target,
    gradients: bool,
    see_all: bool,
    design: &'a [Fill],
    clear: Option<String>,
}

impl<'a> ColorField<'a> {
    /// The field `title` of `target`, showing `current`.
    pub(crate) fn new(title: String, target: Target, current: Option<Fill>) -> Self {
        Self {
            title,
            current,
            target,
            gradients: true,
            see_all: true,
            design: &[],
            clear: None,
        }
    }

    /// Solid colours only (an outline).
    pub(crate) fn solid_only(mut self) -> Self {
        self.gradients = false;
        self
    }

    /// No «Ver todo» (a field for several elements at once).
    pub(crate) fn without_see_all(mut self) -> Self {
        self.see_all = false;
        self
    }

    /// The colours the design already uses.
    pub(crate) fn design(mut self, design: &'a [Fill]) -> Self {
        self.design = design;
        self
    }

    /// «Sin relleno»: the colour can be taken away.
    pub(crate) fn clear(mut self, text: String) -> Self {
        self.clear = Some(text);
        self
    }

    /// Draw it; what was chosen this frame.
    pub(crate) fn show(self, ui: &mut Ui, theme: &Theme, texts: &Texts) -> Option<Picked> {
        let mut picked = None;
        let current = self.current.clone();
        let in_use = |f: &Fill| current.as_ref() == Some(f);
        let caption = |ui: &mut Ui, text: String| {
            ui.add_space(8.0);
            ui.label(
                RichText::new(text)
                    .font(theme.font("body-sm"))
                    .color(theme.c("text.secondary")),
            );
            ui.add_space(4.0);
        };
        // The title, and at its right «Sin …» when there is a colour to take
        // away.
        ui.horizontal(|ui| {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if let Some(clear) = &self.clear
                    && self.current.is_some()
                {
                    let link = RichText::new(clear.as_str())
                        .font(theme.font("body-sm"))
                        .color(theme.c("text.secondary"));
                    if ui.add(egui::Button::new(link).frame(false)).clicked() {
                        picked = Some(Picked::Clear);
                    }
                }
                ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    ui.add(
                        egui::Label::new(
                            RichText::new(self.title.as_str())
                                .font(theme.font("ui-label"))
                                .color(theme.c("text")),
                        )
                        .truncate(),
                    );
                });
            });
        });
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            // The picker shows the colour in use, or the first of its
            // gradient; with none, white.
            let shown = match &self.current {
                Some(Fill::Solid(c)) => Some(*c),
                Some(Fill::Gradient(g)) => g.stops.first().map(|s| s.color),
                None => None,
            };
            let mut rgba = canvas::picker_rgba(shown.or(Some(crate::colors::hex(0xFFFFFF))));
            if crate::eyedropper::color_edit(ui, &mut rgba).changed() {
                picked = Some(Picked::Fill(Fill::Solid(canvas::picked_color(shown, rgba))));
            }
            ui.label(plain(texts, "inspector.custom_color"));
        });
        let brand: Vec<Fill> = crate::brand_panel::colors(ui.ctx())
            .into_iter()
            .map(|c| Fill::Solid(crate::colors::hex(c)))
            .collect();
        for (key, fills) in [
            ("brand.colors_of_brand", brand),
            ("inspector.design_colors", self.design.to_vec()),
        ] {
            let fills: Vec<Fill> = fills
                .into_iter()
                .filter(|f| self.gradients || matches!(f, Fill::Solid(_)))
                .collect();
            if fills.is_empty() {
                continue;
            }
            caption(ui, plain(texts, key));
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = Vec2::splat(6.0);
                for fill in fills {
                    if presets::dot(ui, theme, (&fill, in_use(&fill)), 28.0).clicked() {
                        picked = Some(Picked::Fill(fill));
                    }
                }
            });
        }
        let options = presets::Options {
            gradients: self.gradients,
            see_all: self.see_all,
        };
        if let Some(fill) = presets::sections(ui, theme, texts, (self.target, options), &in_use) {
            picked = Some(Picked::Fill(fill));
        }
        picked
    }
}
