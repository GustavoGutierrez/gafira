//! The tokens of `gafira-theme` as egui colours, fonts and visuals (03 §3).

use egui::{Color32, CornerRadius, FontData, FontDefinitions, FontFamily, FontId, Stroke};
use gafira_theme::{Rgb8, ThemeTokens};

/// The family egui knows Manrope by (`type.family.display`).
pub(crate) const DISPLAY: &str = "display";

/// Shown when a token is missing, so the gap is visible and not hidden behind
/// a plausible colour. The tests check that every token the UI names exists.
const MISSING: Color32 = Color32::from_rgb(255, 0, 255);

/// The validated tokens and the mode in force.
pub(crate) struct Theme {
    pub(crate) tokens: ThemeTokens,
    pub(crate) dark: bool,
}

pub(crate) fn color32(rgb: Rgb8) -> Color32 {
    Color32::from_rgb(rgb.r, rgb.g, rgb.b)
}

impl Theme {
    pub(crate) fn new(tokens: ThemeTokens) -> Self {
        Self {
            tokens,
            dark: false,
        }
    }

    /// The colour of `token` in the mode in force (`"primary.bg"`).
    pub(crate) fn c(&self, token: &str) -> Color32 {
        self.tokens
            .mode(self.dark)
            .get(token)
            .map_or(MISSING, color32)
    }

    /// The font of a step of the type scale (`"h1"`, `"body"`...).
    pub(crate) fn font(&self, step: &str) -> FontId {
        let Some(style) = self.tokens.typography.scale.get(step) else {
            return FontId::proportional(14.0);
        };
        let family = if style.family == self.tokens.typography.display {
            FontFamily::Name(DISPLAY.into())
        } else {
            FontFamily::Proportional
        };
        FontId::new(px(style.size), family)
    }

    pub(crate) fn radius_md(&self) -> CornerRadius {
        CornerRadius::same(clamp_u8(self.tokens.radius.md))
    }

    pub(crate) fn radius_sm(&self) -> CornerRadius {
        CornerRadius::same(clamp_u8(self.tokens.radius.sm))
    }

    /// Install the fonts and the visuals on `ctx`.
    pub(crate) fn apply(&self, ctx: &egui::Context) {
        ctx.set_fonts(fonts(&self.tokens));
        let mut visuals = if self.dark {
            egui::Visuals::dark()
        } else {
            egui::Visuals::light()
        };
        visuals.panel_fill = self.c("bg");
        visuals.window_fill = self.c("surface");
        visuals.extreme_bg_color = self.c("surface.sunken");
        visuals.faint_bg_color = self.c("surface.raised");
        visuals.override_text_color = Some(self.c("text"));
        visuals.hyperlink_color = self.c("link");
        visuals.selection.bg_fill = self.c("selection.bg");
        visuals.selection.stroke = Stroke::new(1.0, self.c("focus.ring"));
        visuals.window_corner_radius = self.radius_md();
        for widget in [
            &mut visuals.widgets.inactive,
            &mut visuals.widgets.hovered,
            &mut visuals.widgets.active,
            &mut visuals.widgets.noninteractive,
        ] {
            widget.corner_radius = self.radius_sm();
        }
        visuals.widgets.inactive.weak_bg_fill = self.c("secondary.bg");
        visuals.widgets.inactive.bg_fill = self.c("secondary.bg");
        visuals.widgets.hovered.weak_bg_fill = self.c("secondary.bg.hover");
        visuals.widgets.active.weak_bg_fill = self.c("secondary.bg.pressed");
        visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, self.c("border"));
        ctx.set_visuals(visuals);
        ctx.style_mut(|style| {
            style
                .text_styles
                .insert(egui::TextStyle::Body, self.font("body-sm"));
            style
                .text_styles
                .insert(egui::TextStyle::Button, self.font("ui-label"));
            style
                .text_styles
                .insert(egui::TextStyle::Small, self.font("caption"));
            style
                .text_styles
                .insert(egui::TextStyle::Heading, self.font("h3"));
            style.spacing.button_padding = egui::vec2(12.0, 6.0);
        });
    }
}

/// Inter for the interface and Manrope for headings (03 §3), from the files
/// `gafira-theme` embeds.
fn fonts(tokens: &ThemeTokens) -> FontDefinitions {
    let mut defs = FontDefinitions::empty();
    for font in gafira_theme::fonts() {
        defs.font_data.insert(
            font.family.to_owned(),
            FontData::from_static(font.bytes).into(),
        );
    }
    // The font library (spec 062): every face under its name, falling back
    // to the interface face for what it lacks.
    let ui = tokens.typography.ui.clone();
    for face in gafira_theme::library() {
        let key = gafira_theme::face_key(face.family, face.bold, face.italic);
        defs.font_data
            .insert(key.clone(), FontData::from_static(face.bytes).into());
        defs.families
            .insert(FontFamily::Name(key.clone().into()), vec![key, ui.clone()]);
    }
    let display = tokens.typography.display.clone();
    defs.families
        .insert(FontFamily::Proportional, vec![ui.clone()]);
    defs.families
        .insert(FontFamily::Monospace, vec![ui.clone()]);
    defs.families
        .insert(FontFamily::Name(DISPLAY.into()), vec![display, ui]);
    defs
}

/// Logical px as egui's `f32`; every token is a small integer.
pub(crate) fn px(value: u32) -> f32 {
    f32::from(u16::try_from(value).unwrap_or(u16::MAX))
}

fn clamp_u8(value: u32) -> u8 {
    u8::try_from(value).unwrap_or(u8::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every colour token the shell names.
    pub(crate) const USED: &[&str] = &[
        "bg",
        "surface",
        "surface.raised",
        "surface.sunken",
        "workspace",
        "border",
        "text",
        "text.secondary",
        "text.muted",
        "text.disabled",
        "primary.bg",
        "primary.bg.hover",
        "primary.fg",
        "secondary.bg",
        "secondary.bg.hover",
        "secondary.bg.pressed",
        "secondary.fg",
        "accent",
        "focus.ring",
        "selection.bg",
        "link",
        "topbar.bg",
        "topbar.fg",
        "topbar.fg.muted",
        "topbar.hover",
    ];

    #[test]
    fn s17_02_every_colour_the_shell_names_exists_in_both_modes() {
        let tokens = ThemeTokens::embedded().expect("the embedded tokens load");
        for dark in [false, true] {
            for token in USED {
                assert!(
                    tokens.mode(dark).get(token).is_some(),
                    "{token} (dark: {dark})"
                );
            }
        }
    }

    #[test]
    fn s17_02_every_type_step_the_shell_names_exists() {
        let tokens = ThemeTokens::embedded().expect("the embedded tokens load");
        for step in [
            "h1", "h3", "body-lg", "body", "body-sm", "caption", "ui-label",
        ] {
            assert!(tokens.typography.scale.contains_key(step), "{step}");
        }
    }

    #[test]
    fn s17_02_the_display_family_resolves_to_manrope_and_ui_to_inter() {
        let theme = Theme::new(ThemeTokens::embedded().expect("tokens"));
        assert_eq!(theme.font("h1").family, FontFamily::Name(DISPLAY.into()));
        assert_eq!(theme.font("body").family, FontFamily::Proportional);
        let defs = fonts(&theme.tokens);
        assert_eq!(defs.families[&FontFamily::Proportional], ["Inter"]);
        assert_eq!(
            defs.families[&FontFamily::Name(DISPLAY.into())][0],
            "Manrope"
        );
    }

    #[test]
    fn s17_02_a_missing_token_shows_the_warning_colour() {
        let theme = Theme::new(ThemeTokens::embedded().expect("tokens"));
        assert_eq!(theme.c("no.such.token"), MISSING);
        assert_ne!(theme.c("bg"), MISSING);
    }
}
