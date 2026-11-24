//! The preset colours of the Inspector (spec 098): round swatches in rows of
//! seven under «Colores sólidos predeterminados» and «Colores degradados
//! predeterminados», each with «Ver todo», which shows every preset in the
//! panel: a palette of ramps by hue, and the gradients by group.

use egui::{Color32, Pos2, RichText, Sense, Stroke, Ui, Vec2};
use gafira_color::Color;
use gafira_doc::{Gradient, NodeId, PageId};

use crate::canvas;
use crate::charts::plain;
use crate::colors::{TEXT_GRADIENTS, TEXT_PALETTE, gradient, hex};
use crate::icons::{self, Icon, Weight};
use crate::style::Theme;
use crate::text::Texts;

/// One colour or a gradient over the box.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Fill {
    Solid(Color),
    Gradient(Gradient),
}

/// Swatches in a row.
const COLUMNS: usize = 7;

/// What a full view of presets is for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Target {
    /// The letters of a text.
    Text(PageId, NodeId),
    /// The fill of a shape.
    Shape(PageId, NodeId),
    /// The background of a page.
    Page(PageId),
    /// The outline of a shape: solid colours only.
    Outline(PageId, NodeId),
}

/// What the preset sections of a field offer.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Options {
    pub gradients: bool,
    pub see_all: bool,
}

/// Which full view.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Full {
    Solids,
    Gradients,
}

fn open_id() -> egui::Id {
    egui::Id::new("presets.full")
}

/// The full view open now, if any.
pub(crate) fn opened(ctx: &egui::Context) -> Option<(Target, Full)> {
    ctx.data(|d| d.get_temp::<(Target, Full)>(open_id()))
}

/// Open or close the full view.
pub(crate) fn set_open(ctx: &egui::Context, open: Option<(Target, Full)>) {
    ctx.data_mut(|d| match open {
        Some(open) => d.insert_temp(open_id(), open),
        None => d.remove::<(Target, Full)>(open_id()),
    });
}

/// An sRGB colour from hue (degrees), saturation and lightness (0 to 1).
pub(crate) fn hsl(h: f64, s: f64, l: f64) -> u32 {
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let h = h.rem_euclid(360.0) / 60.0;
    let x = c * (1.0 - (h % 2.0 - 1.0).abs());
    let (r, g, b) = match h as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = l - c / 2.0;
    // Within 0 to 255 after the clamp: the cast is exact.
    let byte = |v: f64| ((v + m).clamp(0.0, 1.0) * 255.0).round() as u32;
    (byte(r) << 16) | (byte(g) << 8) | byte(b)
}

/// The hues of the rows of the full palette, from blue round to blue.
const HUES: [f64; 18] = [
    225.0, 245.0, 262.0, 280.0, 300.0, 330.0, 355.0, 5.0, 20.0, 35.0, 48.0, 75.0, 100.0, 140.0,
    160.0, 180.0, 195.0, 212.0,
];

/// Every solid preset, `COLUMNS` a row: the greys, then a ramp from dark to
/// light for each hue.
pub(crate) fn all_solids() -> Vec<u32> {
    let mut out = vec![
        0x000000, 0x404040, 0x737373, 0xA6A6A6, 0xD9D9D9, 0xF2F2F2, 0xFFFFFF,
    ];
    const LIGHT: [f64; COLUMNS] = [0.14, 0.26, 0.40, 0.54, 0.66, 0.78, 0.89];
    for hue in HUES {
        for (n, l) in LIGHT.iter().enumerate() {
            let s = if n == 0 { 0.65 } else { 0.85 };
            out.push(hsl(hue, s, *l));
        }
    }
    out
}

/// The groups of gradient presets: name and gradients, two colours and an
/// angle each.
pub(crate) const COLD: [(u32, u32, f64); 7] = [
    (0x7F7FD5, 0x2BC0E4, 45.0),
    (0x141E30, 0x2948C8, 45.0),
    (0x134E5E, 0x3FB5A3, 45.0),
    (0x43C6AC, 0xB8E6D5, 90.0),
    (0x8EA6C8, 0xDAE6F0, 90.0),
    (0x5B86E5, 0xA8D8FF, 90.0),
    (0x3A7BD5, 0x8F6CFF, 45.0),
];
pub(crate) const WARM: [(u32, u32, f64); 7] = [
    (0xFF7E5F, 0xFEB47B, 45.0),
    (0x5A1A3A, 0xA8325E, 45.0),
    (0x6B2F1F, 0xC9774F, 45.0),
    (0xFF8C42, 0xFFB26B, 90.0),
    (0x8E4A2C, 0xE0A06A, 45.0),
    (0xFF5E8A, 0xFFA6C9, 45.0),
    (0x8E54E9, 0xF7971E, 45.0),
];
pub(crate) const MONO: [(u32, u32, f64); 21] = [
    (0x3C3B33, 0x6E6A5E, 90.0),
    (0x8E8A84, 0xC9C6C1, 90.0),
    (0xE8E4DA, 0xFAF8F3, 90.0),
    (0xDDDDDD, 0xFFFFFF, 90.0),
    (0xB8BDC6, 0xE4E7EC, 90.0),
    (0x5A6070, 0x9AA0AE, 90.0),
    (0x20232B, 0x4B505C, 90.0),
    (0x8E1B1B, 0xD34040, 90.0),
    (0xB8461C, 0xE5793F, 90.0),
    (0xB88A00, 0xF2C230, 90.0),
    (0x2E8B1E, 0x5FCC3F, 90.0),
    (0x0D7C70, 0x2FC2B0, 90.0),
    (0x1A3A9A, 0x4F7BE0, 90.0),
    (0x5A1080, 0xA040D0, 90.0),
    (0xFF8A8A, 0xFFC4C4, 90.0),
    (0xFFA888, 0xFFD6C2, 90.0),
    (0xFFE39A, 0xFFF4D0, 90.0),
    (0x9BE08F, 0xD8F5D0, 90.0),
    (0x9EF5EC, 0xDDFDF8, 90.0),
    (0xA8B8FF, 0xE4EAFF, 90.0),
    (0xE8A8FF, 0xF8E4FF, 90.0),
];
const MORE: [(u32, u32, f64); 14] = [
    (0xFBD3E9, 0xBB377D, 45.0),
    (0xF7971E, 0xFFD200, 45.0),
    (0xEE0979, 0xFF6A00, 45.0),
    (0x00C9FF, 0x92FE9D, 45.0),
    (0xFC466B, 0x3F5EFB, 45.0),
    (0x11998E, 0x38EF7D, 45.0),
    (0x8360C3, 0x2EBF91, 45.0),
    (0xF953C6, 0xB91D73, 45.0),
    (0xC33764, 0x1D2671, 45.0),
    (0xFFAFBD, 0xFFC3A0, 45.0),
    (0x2193B0, 0x6DD5ED, 45.0),
    (0xCC2B5E, 0x753A88, 45.0),
    (0x42275A, 0x734B6D, 45.0),
    (0xBDC3C7, 0x2C3E50, 90.0),
];

/// Every gradient preset, each once: the ones of the text first.
pub(crate) fn all_gradients() -> Vec<(u32, u32, f64)> {
    let mut out: Vec<(u32, u32, f64)> = Vec::new();
    let lists: [&[(u32, u32, f64)]; 6] = [
        &TEXT_GRADIENTS,
        &crate::colors::GRADIENTS,
        &MORE,
        &COLD,
        &WARM,
        &MONO,
    ];
    for list in lists {
        for g in list {
            if !out.contains(g) {
                out.push(*g);
            }
        }
    }
    out
}

fn as_fills(list: &[(u32, u32, f64)]) -> Vec<Fill> {
    list.iter()
        .map(|(a, b, angle)| Fill::Gradient(gradient(*a, *b, *angle)))
        .collect()
}

fn solid_fills(list: &[u32]) -> Vec<Fill> {
    list.iter().map(|code| Fill::Solid(hex(*code))).collect()
}

/// A round swatch of `fill`, `side` across, ringed when it is the one in
/// use; a hand over it.
pub(crate) fn dot(
    ui: &mut Ui,
    theme: &Theme,
    (fill, on): (&Fill, bool),
    side: f32,
) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(side), Sense::click());
    let painter = ui.painter();
    let centre = rect.center();
    let r = side / 2.0 - 1.0;
    match fill {
        Fill::Solid(color) => {
            painter.circle_filled(centre, r, canvas::screen_color(*color));
        }
        Fill::Gradient(g) => disc(painter, g, centre, r),
    }
    let ring = if on {
        painter.circle_stroke(centre, r + 0.5, Stroke::new(2.0, theme.c("surface")));
        Stroke::new(2.0, theme.c("primary.bg"))
    } else if response.hovered() {
        Stroke::new(1.5, theme.c("text.secondary"))
    } else {
        Stroke::new(1.0, Color32::from_black_alpha(30))
    };
    let edge = if on { r + 2.0 } else { r };
    painter.circle_stroke(centre, edge, ring);
    if response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    response
}

/// A gradient as a disc: a fan whose rim takes the colour of where it falls
/// along the direction of the gradient.
fn disc(painter: &egui::Painter, g: &Gradient, centre: Pos2, r: f32) {
    let angle = match g.kind {
        gafira_doc::GradientKind::Linear { angle_deg } => angle_deg,
        _ => 90.0,
    };
    let (dy, dx) = angle.to_radians().sin_cos();
    let at = |t: f64| -> Color32 {
        let t = t.clamp(0.0, 1.0);
        let stops = &g.stops;
        if stops.is_empty() {
            return Color32::TRANSPARENT;
        }
        match stops.iter().position(|s| f64::from(s.offset) >= t) {
            Some(0) => canvas::screen_color(stops[0].color),
            None => canvas::screen_color(stops[stops.len() - 1].color),
            Some(n) => {
                let (a, b) = (&stops[n - 1], &stops[n]);
                let span = f64::from(b.offset - a.offset).max(f64::EPSILON);
                let k = ((t - f64::from(a.offset)) / span) as f32;
                canvas::screen_color(a.color).lerp_to_gamma(canvas::screen_color(b.color), k)
            }
        }
    };
    // A fan to the rim, then a band one pixel wide that fades out, so the
    // edge is smooth like egui's own circles.
    let feather = 1.0 / painter.ctx().pixels_per_point();
    let inner = r - feather / 2.0;
    let mut mesh = egui::Mesh::default();
    mesh.colored_vertex(centre, at(0.5));
    const SIDES: u32 = 64;
    for k in 0..=SIDES {
        let a = k as f32 / SIDES as f32 * std::f32::consts::TAU;
        let (sin, cos) = a.sin_cos();
        let along = (f64::from(cos) * dx + f64::from(sin) * dy + 1.0) / 2.0;
        let color = at(along);
        let dir = Vec2::new(cos, sin);
        mesh.colored_vertex(centre + dir * inner, color);
        mesh.colored_vertex(centre + dir * (inner + feather), Color32::TRANSPARENT);
        if k > 0 {
            let (i0, o0, i1, o1) = (2 * k - 1, 2 * k, 2 * k + 1, 2 * k + 2);
            mesh.add_triangle(0, i0, i1);
            mesh.add_triangle(i0, o0, i1);
            mesh.add_triangle(o0, o1, i1);
        }
    }
    painter.add(mesh);
}

/// Swatches in rows of seven across the width; the one chosen.
fn grid(
    ui: &mut Ui,
    theme: &Theme,
    fills: &[Fill],
    in_use: &dyn Fn(&Fill) -> bool,
) -> Option<Fill> {
    let gap = 6.0;
    let side =
        ((ui.available_width() - gap * (COLUMNS as f32 - 1.0)) / COLUMNS as f32).clamp(22.0, 40.0);
    let mut chosen = None;
    for row in fills.chunks(COLUMNS) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = gap;
            for fill in row {
                if dot(ui, theme, (fill, in_use(fill)), side).clicked() {
                    chosen = Some(fill.clone());
                }
            }
        });
        ui.add_space(gap - ui.spacing().item_spacing.y);
    }
    chosen
}

/// The small icon of the gradient section: a square going from dark to light.
fn gradient_icon(ui: &mut Ui, theme: &Theme) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(16.0), Sense::hover());
    let square = rect.shrink(1.5);
    let mut mesh = egui::Mesh::default();
    let (dark, light) = (theme.c("text"), theme.c("surface"));
    mesh.colored_vertex(square.left_top(), dark);
    mesh.colored_vertex(square.right_top(), light);
    mesh.colored_vertex(square.right_bottom(), light);
    mesh.colored_vertex(square.left_bottom(), dark);
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(0, 2, 3);
    ui.painter().add(mesh);
    ui.painter().rect_stroke(
        square,
        2.0,
        Stroke::new(1.0, theme.c("text")),
        egui::StrokeKind::Middle,
    );
}

/// A section title with its icon and «Ver todo»; whether «Ver todo» was
/// pressed.
fn header(
    ui: &mut Ui,
    theme: &Theme,
    texts: &Texts,
    (title, see_all): (&'static str, bool),
    full: Full,
) -> bool {
    ui.add_space(10.0);
    ui.horizontal(|ui| {
        // «Ver todo» first, at the right; the title takes what is left.
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let link = RichText::new(plain(texts, "presets.see_all"))
                .font(theme.font("body-sm"))
                .color(theme.c("text.secondary"));
            let pressed = see_all && ui.add(egui::Button::new(link).frame(false)).clicked();
            ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                match full {
                    Full::Solids => {
                        let ink = theme.c("text");
                        ui.add(icons::image(
                            ui.ctx(),
                            Icon::Identity,
                            Weight::Regular,
                            16.0,
                            ink,
                        ));
                    }
                    Full::Gradients => gradient_icon(ui, theme),
                }
                ui.add(
                    egui::Label::new(
                        RichText::new(plain(texts, title))
                            .font(theme.font("ui-label"))
                            .color(theme.c("text")),
                    )
                    .truncate(),
                );
            });
            pressed
        })
        .inner
    })
    .inner
}

/// The two preset sections of the Inspector for `target`: four rows of
/// solid colours and three of gradients (none for a page that only takes
/// solids is not a case: every target takes both). The one chosen.
pub(crate) fn sections(
    ui: &mut Ui,
    theme: &Theme,
    texts: &Texts,
    (target, options): (Target, Options),
    in_use: &dyn Fn(&Fill) -> bool,
) -> Option<Fill> {
    let mut chosen = None;
    let solids = "inspector.solid_presets";
    if header(ui, theme, texts, (solids, options.see_all), Full::Solids) {
        set_open(ui.ctx(), Some((target, Full::Solids)));
    }
    ui.add_space(4.0);
    chosen = grid(ui, theme, &solid_fills(&TEXT_PALETTE), in_use).or(chosen);
    if !options.gradients {
        return chosen;
    }
    let gradients = "inspector.gradient_presets";
    if header(
        ui,
        theme,
        texts,
        (gradients, options.see_all),
        Full::Gradients,
    ) {
        set_open(ui.ctx(), Some((target, Full::Gradients)));
    }
    ui.add_space(4.0);
    let shown: Vec<_> = all_gradients().into_iter().take(COLUMNS * 3).collect();
    chosen = grid(ui, theme, &as_fills(&shown), in_use).or(chosen);
    chosen
}

/// The full view, in place of the Inspector: «←», the title and «×» over
/// every preset, scrolled. The one chosen, and whether it was closed.
pub(crate) fn full_view(
    ui: &mut Ui,
    theme: &Theme,
    texts: &Texts,
    full: Full,
    in_use: &dyn Fn(&Fill) -> bool,
) -> Option<Fill> {
    let title = match full {
        Full::Solids => "inspector.solid_presets",
        Full::Gradients => "inspector.gradient_presets",
    };
    let mut close = false;
    ui.horizontal(|ui| {
        close |= crate::widgets::back_arrow(ui)
            .on_hover_text(plain(texts, "presets.back"))
            .clicked();
        ui.add(
            egui::Label::new(
                RichText::new(plain(texts, title))
                    .font(theme.font("ui-label"))
                    .color(theme.c("text")),
            )
            .truncate(),
        );
    });
    ui.add_space(8.0);
    let mut chosen = None;
    egui::ScrollArea::vertical()
        .id_salt("presets.full")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.set_max_width(ui.available_width() - 12.0);
            match full {
                Full::Solids => {
                    chosen = grid(ui, theme, &solid_fills(&all_solids()), in_use);
                }
                Full::Gradients => {
                    for (key, list) in [
                        ("presets.cold", COLD.to_vec()),
                        ("presets.warm", WARM.to_vec()),
                        ("presets.mono", MONO.to_vec()),
                        ("presets.all_gradients", all_gradients()),
                    ] {
                        ui.label(
                            RichText::new(plain(texts, key))
                                .font(theme.font("body-sm"))
                                .color(theme.c("text.secondary")),
                        );
                        ui.add_space(4.0);
                        chosen = grid(ui, theme, &as_fills(&list), in_use).or(chosen.take());
                        ui.add_space(10.0);
                    }
                }
            }
        });
    if close {
        set_open(ui.ctx(), None);
    }
    chosen
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn s98_10_hsl_gives_the_primaries_and_the_greys() {
        assert_eq!(hsl(0.0, 1.0, 0.5), 0xFF0000);
        assert_eq!(hsl(120.0, 1.0, 0.5), 0x00FF00);
        assert_eq!(hsl(240.0, 1.0, 0.5), 0x0000FF);
        assert_eq!(hsl(60.0, 1.0, 0.5), 0xFFFF00);
        assert_eq!(hsl(300.0, 1.0, 0.5), 0xFF00FF);
        assert_eq!(hsl(0.0, 0.0, 0.0), 0x000000);
        assert_eq!(hsl(0.0, 0.0, 1.0), 0xFFFFFF);
        assert_eq!(hsl(-120.0, 1.0, 0.5), 0x0000FF);
    }

    #[test]
    fn s98_10_the_full_palette_fills_whole_rows_from_dark_to_light() {
        let all = all_solids();
        assert_eq!(all.len(), COLUMNS * (HUES.len() + 1));
        let luma = |c: u32| ((c >> 16) & 0xFF) * 3 + ((c >> 8) & 0xFF) * 6 + (c & 0xFF);
        for row in all.chunks(COLUMNS) {
            assert!(row.windows(2).all(|w| luma(w[0]) < luma(w[1])), "{row:x?}");
        }
    }

    #[test]
    fn s98_10_every_gradient_preset_is_listed_once() {
        let all = all_gradients();
        for (n, g) in all.iter().enumerate() {
            assert!(!all[n + 1..].contains(g), "{g:x?}");
        }
        for g in COLD.iter().chain(&WARM).chain(&MONO).chain(&TEXT_GRADIENTS) {
            assert!(all.contains(g), "{g:x?}");
        }
        assert!(all.len() >= COLUMNS * 3);
    }
}
