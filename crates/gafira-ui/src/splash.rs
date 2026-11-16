//! The Startup screen (03 §7.10, D-09), drawn inside the main window: the
//! fallback §7.10.2 allows when a borderless, centred window is not
//! available (Wayland places windows itself).

use egui::{Align2, Pos2, Rect, Vec2};

use crate::style::{DISPLAY, Theme, color32, px};
use crate::text::Texts;

const IMAGE: &[u8] = include_bytes!("../../../assets/splash_screen.jpg");
const IMAGE_SIZE: (f32, f32) = (1672.0, 941.0);
const SYMBOL: &[u8] = include_bytes!("../../../assets/gafira-logo-symbol.svg");

/// The shortest time the screen stays up, in seconds (03 §7.10, D-09).
pub(crate) const MIN_VISIBLE: f32 = 1.2;

/// The share of the bar filled after `elapsed` seconds. Nothing is loaded
/// yet, so the bar follows the minimum visible time.
pub(crate) fn progress(elapsed: f32) -> f32 {
    (elapsed / MIN_VISIBLE).clamp(0.0, 1.0)
}

/// The rule «cubrir» of 03 §7.10.3: the image covers the window without
/// deforming, centred horizontally and centred vertically on the zone above
/// the panel. `window` and `image` are sizes; `panel` is the panel height.
pub(crate) fn cover(window: Vec2, panel: f32, image: (f32, f32)) -> Rect {
    let k = (window.x / image.0).max(window.y / image.1);
    let (w, h) = (image.0 * k, image.1 * k);
    let x = (window.x - w) / 2.0;
    let visible = window.y - panel;
    let y = (visible / 2.0 - h / 2.0).clamp(window.y - h, 0.0);
    Rect::from_min_size(Pos2::new(x, y), Vec2::new(w, h))
}

pub(crate) fn show(ctx: &egui::Context, theme: &Theme, texts: &Texts, version: &str, done: f32) {
    let splash = &theme.tokens.splash;
    egui::CentralPanel::default()
        .frame(egui::Frame::NONE.fill(color32(splash.panel)))
        .show(ctx, |ui| {
            let full = ui.max_rect();
            let panel_h = px(splash.panel_height);
            let image_zone =
                Rect::from_min_max(full.min, Pos2::new(full.max.x, full.max.y - panel_h));
            if image_zone.height() >= px(splash.image_min_visible_height) {
                let at = cover(full.size(), panel_h, IMAGE_SIZE).translate(full.min.to_vec2());
                ui.scope(|ui| {
                    ui.set_clip_rect(image_zone);
                    egui::Image::from_bytes("bytes://splash_screen.jpg", IMAGE).paint_at(ui, at);
                });
            }
            let pad = px(splash.panel_padding);
            let panel = Rect::from_min_max(Pos2::new(full.min.x, full.max.y - panel_h), full.max);
            let logo =
                Rect::from_min_size(panel.min + Vec2::new(pad, pad - 4.0), Vec2::splat(44.0));
            egui::Image::from_bytes("bytes://gafira-logo-symbol.svg", SYMBOL).paint_at(ui, logo);
            let painter = ui.painter();
            painter.text(
                Pos2::new(logo.max.x + 12.0, logo.center().y),
                Align2::LEFT_CENTER,
                texts.t(&gafira_doc::msg!("app.name")),
                egui::FontId::new(30.0, egui::FontFamily::Name(DISPLAY.into())),
                theme.c("text"),
            );
            painter.text(
                Pos2::new(panel.max.x - pad, logo.center().y),
                Align2::RIGHT_CENTER,
                texts.t(&gafira_doc::msg!("brand.slogan")),
                theme.font("body-lg"),
                theme.c("text"),
            );
            let bar_h = px(splash.progress_height);
            let track = Rect::from_min_max(
                Pos2::new(panel.min.x + pad, panel.max.y - pad),
                Pos2::new(panel.max.x - pad - 140.0, panel.max.y - pad + bar_h),
            );
            painter.rect_filled(track, 2.0, color32(splash.progress_track));
            let mut fill = track;
            fill.set_width(track.width() * done);
            painter.rect_filled(fill, 2.0, color32(splash.progress_fill));
            painter.text(
                Pos2::new(panel.max.x - pad, track.center().y),
                Align2::RIGHT_CENTER,
                texts.t(&gafira_doc::msg!(
                    "splash.version",
                    version = Text(version.to_owned())
                )),
                theme.font("caption"),
                theme.c("text.secondary"),
            );
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn s17_01_the_bar_fills_over_the_minimum_visible_time() {
        assert_eq!(progress(0.0), 0.0);
        assert!((progress(0.6) - 0.5).abs() < 1e-6);
        assert_eq!(progress(MIN_VISIBLE), 1.0);
        assert_eq!(progress(10.0), 1.0);
        assert_eq!(progress(-1.0), 0.0);
    }

    #[test]
    fn s17_01_cover_fills_the_window_without_deforming() {
        let window = Vec2::new(800.0, 450.0);
        let at = cover(window, 128.0, IMAGE_SIZE);
        assert!(at.min.x <= 0.0 && at.max.x >= 800.0, "covers the width");
        assert!(at.min.y <= 0.0 && at.max.y >= 450.0, "covers the height");
        assert!((at.width() / at.height() - IMAGE_SIZE.0 / IMAGE_SIZE.1).abs() < 1e-4);
        assert!((at.center().x - 400.0).abs() < 1e-3, "centred horizontally");
    }

    #[test]
    fn s17_01_a_wide_window_centres_the_image_on_the_zone_above_the_panel() {
        // 3:1 is wider than the image, so the width sets the scale and the
        // centre goes to the middle of the 1000 - 128 px above the panel.
        let at = cover(Vec2::new(3000.0, 1000.0), 128.0, IMAGE_SIZE);
        assert!((at.width() - 3000.0).abs() < 1e-3);
        assert!((at.center().y - 436.0).abs() < 1e-3);
        // At 2:1 the image is barely taller than the window: the clamp keeps
        // its bottom edge on the window's, so no gap opens under it.
        let at = cover(Vec2::new(2000.0, 1000.0), 128.0, IMAGE_SIZE);
        assert!((at.max.y - 1000.0).abs() < 1e-3);
    }
}
