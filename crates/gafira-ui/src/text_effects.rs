//! «Efectos» of a text (spec 089): the panel with the effects, the numbers of
//! the one chosen under its row, «Curvar» and the styles that are ready.

use std::cell::RefCell;
use std::collections::BTreeMap;

use egui::{Pos2, Rect, Response, RichText, Sense, Stroke, StrokeKind, Ui, Vec2};
use gafira_color::Color;
use gafira_doc::{
    NodeId, PageId, Placement, SpanStyle, TextEffect, TextEffectKind as K, TextEffectNumber as N,
    TextNode,
};

use crate::canvas;
use crate::charts::plain;
use crate::colors::hex;
use crate::image_panel::{Track, heading, main_button, slider};
use crate::style::Theme;
use crate::text::Texts;
use crate::widgets;

/// The text being given effects.
pub(crate) struct Editing {
    pub page: PageId,
    pub node: NodeId,
    samples: RefCell<BTreeMap<String, Option<(egui::TextureHandle, Vec2)>>>,
}

/// What the panel asks for.
#[derive(Default)]
pub(crate) struct Asks {
    /// The effects, while a control moves one.
    pub effects: Option<Vec<TextEffect>>,
    pub curve: Option<f64>,
    /// A style: its font, its colour and its effects, in one step.
    pub style: Option<(SpanStyle, Vec<TextEffect>)>,
    pub close: bool,
}

pub(crate) fn key(kind: K) -> &'static str {
    match kind {
        K::Drop => "effects.drop",
        K::Glow => "effects.glow",
        K::Echo => "effects.echo",
        K::Outline => "effects.outline",
        K::Background => "effects.background",
        K::Offset => "effects.offset",
        K::Hollow => "effects.hollow",
        K::Neon => "effects.neon",
        K::Glitch => "effects.glitch",
    }
}

fn number_key(number: N) -> &'static str {
    match number {
        N::Thickness => "effects.thickness",
        N::Distance => "effects.distance",
        N::Angle => "effects.angle",
        N::Blur => "effects.blur",
        N::Opacity => "effects.opacity",
        N::Roundness => "effects.roundness",
        N::Spread => "effects.spread",
        N::Intensity => "effects.intensity",
    }
}

/// A style that is ready.
struct Ready {
    key: &'static str,
    font: &'static str,
    color: u32,
    effects: fn() -> Vec<TextEffect>,
}

fn with(kind: K, color: u32, change: impl FnOnce(&mut TextEffect)) -> TextEffect {
    let mut effect = TextEffect::new(kind);
    effect.color = hex(color);
    change(&mut effect);
    effect
}

const READY: [Ready; 12] = [
    Ready {
        key: "effects.s_sign",
        font: "Pacifico",
        color: 0xFF_8A_7A,
        effects: || vec![with(K::Neon, 0xFF_5A_46, |e| e.intensity = 70.0)],
    },
    Ready {
        key: "effects.s_screen",
        font: "Archivo Black",
        color: 0x18_24_3B,
        effects: || {
            vec![with(K::Glitch, 0x3E_E0_D0, |e| {
                e.second = hex(0xFF_3B_7F);
                e.distance = 45.0;
            })]
        },
    },
    Ready {
        key: "effects.s_retro",
        font: "Shrikhand",
        color: 0xFF_C8_3D,
        effects: || vec![with(K::Echo, 0xB8_4A_1F, |e| e.distance = 18.0)],
    },
    Ready {
        key: "effects.s_laser",
        font: "Bebas Neue",
        color: 0xD6_FF_5A,
        effects: || vec![with(K::Glow, 0x8F_E3_00, |e| e.intensity = 75.0)],
    },
    Ready {
        key: "effects.s_print",
        font: "Anton",
        color: 0xFF_FF_FF,
        effects: || {
            vec![
                with(K::Echo, 0xFF_6B_57, |e| e.distance = 30.0),
                with(K::Outline, 0x18_24_3B, |e| e.thickness = 22.0),
            ]
        },
    },
    Ready {
        key: "effects.s_frontier",
        font: "Kaushan Script",
        color: 0xC9_5B_3E,
        effects: || {
            vec![
                with(K::Drop, 0x5A_22_14, |e| {
                    (e.blur, e.opacity, e.distance) = (0.0, 100.0, 9.0);
                }),
                with(K::Outline, 0xFF_E9_D6, |e| e.thickness = 14.0),
            ]
        },
    },
    Ready {
        key: "effects.s_wall",
        font: "Chewy",
        color: 0xF2_F5_F0,
        effects: || {
            vec![
                with(K::Drop, 0x0E_3B_2E, |e| {
                    (e.blur, e.opacity, e.distance) = (0.0, 100.0, 24.0);
                }),
                with(K::Outline, 0x14_53_40, |e| e.thickness = 45.0),
            ]
        },
    },
    Ready {
        key: "effects.s_bubble",
        font: "Chewy",
        color: 0x3E_BF_A8,
        effects: || {
            vec![
                with(K::Drop, 0x18_24_3B, |e| {
                    (e.blur, e.opacity, e.distance) = (14.0, 35.0, 22.0);
                }),
                with(K::Outline, 0x18_24_3B, |e| e.thickness = 60.0),
            ]
        },
    },
    Ready {
        key: "effects.s_breeze",
        font: "Alex Brush",
        color: 0xFF_B8_C8,
        effects: || vec![with(K::Glow, 0xFF_8F_A8, |e| e.intensity = 55.0)],
    },
    Ready {
        key: "effects.s_console",
        font: "Bebas Neue",
        color: 0x1F_D6_5A,
        effects: || {
            vec![with(K::Drop, 0x0B_3D_1E, |e| {
                (e.blur, e.opacity, e.distance, e.angle_deg) = (0.0, 100.0, 16.0, 45.0);
            })]
        },
    },
    Ready {
        key: "effects.s_cosmos",
        font: "Satisfy",
        color: 0xE8_F1_FF,
        effects: || vec![with(K::Neon, 0x3D_7B_FF, |e| e.intensity = 80.0)],
    },
    Ready {
        key: "effects.s_poster",
        font: "League Spartan",
        color: 0xFF_FF_FF,
        effects: || {
            vec![with(K::Background, 0xFF_6B_57, |e| {
                (e.roundness, e.spread) = (20.0, 60.0);
            })]
        },
    },
];

/// The effect `kind`, put on a text of `color`: what shines does in its
/// colour.
fn start(kind: K, color: Color) -> TextEffect {
    let mut effect = TextEffect::new(kind);
    let c = color.to_linear_srgb();
    // What is dark does not shine.
    let dark = 0.2126 * c.r + 0.7152 * c.g + 0.0722 * c.b < 0.1;
    if kind == K::Echo || (matches!(kind, K::Glow | K::Neon) && !dark) {
        effect.color = color;
    }
    effect
}

/// The room between tiles.
const GAP: f32 = 8.0;

impl Editing {
    pub fn new(page: PageId, node: NodeId) -> Self {
        Self {
            page,
            node,
            samples: RefCell::default(),
        }
    }

    pub fn panel(&self, ui: &mut Ui, theme: &Theme, texts: &Texts, text: &TextNode) -> Asks {
        let mut asks = Asks::default();
        ui.horizontal(|ui| {
            ui.label(RichText::new(plain(texts, "effects.title")).font(theme.font("h3")));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let close = ui
                    .add(egui::Button::new(RichText::new("×").size(22.0)).frame(false))
                    .on_hover_text(plain(texts, "effects.close"));
                asks.close = close.clicked();
            });
        });
        ui.add_space(8.0);
        let footer = egui::Frame::NONE.inner_margin(egui::Margin {
            top: 12,
            bottom: 8,
            ..egui::Margin::ZERO
        });
        egui::TopBottomPanel::bottom("effects.actions")
            .show_separator_line(false)
            .frame(footer)
            .show_inside(ui, |ui| {
                let on = !text.effects.is_empty();
                ui.add_enabled_ui(on, |ui| {
                    let remove = plain(texts, "effects.remove");
                    if widgets::secondary(ui, theme, remove).clicked() {
                        asks.effects = Some(Vec::new());
                    }
                });
            });
        egui::ScrollArea::vertical()
            .id_salt("effects.panel")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                // Room for the scroll bar, beside the controls.
                ui.set_max_width(ui.available_width() - 14.0);
                self.effects(ui, theme, texts, text, &mut asks);
                self.curve(ui, theme, texts, text, &mut asks);
                self.styles(ui, theme, texts, text, &mut asks);
                ui.add_space(16.0);
            });
        asks
    }

    fn effects(&self, ui: &mut Ui, theme: &Theme, texts: &Texts, text: &TextNode, asks: &mut Asks) {
        let color = text.content.first_style().map(|style| style.color);
        let color = color.unwrap_or_else(|| hex(0x18_24_3B));
        let side = tile_side(ui);
        for row in K::ALL.chunks(3) {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = GAP;
                for kind in row {
                    let on = text.effects.iter().any(|effect| effect.kind == *kind);
                    let name = plain(texts, key(*kind));
                    let sample = self.sample(ui, name.clone(), || {
                        let ink = hex(0x18_24_3B);
                        sample("Ag", "Inter", ink, vec![TextEffect::new(*kind)], 0.0)
                    });
                    if tile(ui, theme, (sample, name), (side, on)).clicked() {
                        // The one chosen goes; another is put in its place.
                        asks.effects = Some(if on {
                            Vec::new()
                        } else {
                            vec![start(*kind, color)]
                        });
                    }
                }
            });
            for (at, effect) in text.effects.iter().enumerate() {
                if row.contains(&effect.kind) {
                    ui.push_id(("effects.numbers", at), |ui| {
                        numbers(ui, theme, texts, (text, at), asks);
                    });
                }
            }
            ui.add_space(8.0);
        }
    }

    fn curve(&self, ui: &mut Ui, theme: &Theme, texts: &Texts, text: &TextNode, asks: &mut Asks) {
        heading(ui, theme, plain(texts, "effects.shape"));
        let side = tile_side(ui);
        let on = text.curve != 0.0;
        let name = plain(texts, "effects.curve");
        let sample = self.sample(ui, name.clone(), || {
            sample("ABCD", "Inter", hex(0x18_24_3B), Vec::new(), 45.0)
        });
        if tile(ui, theme, (sample, name), (side, on)).clicked() {
            asks.curve = Some(if on { 0.0 } else { 40.0 });
        }
        if on {
            ui.add_space(6.0);
            let mut curve = text.curve;
            let most = TextNode::MAX_CURVE;
            let what = (&mut curve, (-most, most), 40.0);
            let name = plain(texts, "effects.curve");
            if slider(ui, theme, name, what, Track::Plain) {
                asks.curve = Some(curve);
            }
        }
    }

    fn styles(&self, ui: &mut Ui, theme: &Theme, texts: &Texts, text: &TextNode, asks: &mut Asks) {
        heading(ui, theme, plain(texts, "effects.styles"));
        let side = tile_side(ui);
        for row in READY.chunks(3) {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = GAP;
                for ready in row {
                    let name = plain(texts, ready.key);
                    let effects = (ready.effects)();
                    let style = text.content.first_style();
                    let on = text.effects == effects
                        && style.is_some_and(|style| style.font == ready.font);
                    let sample = self.sample(ui, ready.key.to_owned(), || {
                        sample("Ag", ready.font, hex(ready.color), effects.clone(), 0.0)
                    });
                    if tile(ui, theme, (sample, name), (side, on)).clicked()
                        && let Some(style) = style
                    {
                        let style = SpanStyle {
                            font: ready.font.to_owned(),
                            color: hex(ready.color),
                            bold: false,
                            italic: false,
                            ..style.clone()
                        };
                        asks.style = Some((style, effects));
                    }
                }
            });
            ui.add_space(8.0);
        }
    }

    /// The picture of a sample and its size, made once.
    fn sample(
        &self,
        ui: &Ui,
        id: String,
        make: impl FnOnce() -> Option<(egui::ColorImage, Vec2)>,
    ) -> Option<(egui::TextureId, Vec2)> {
        let mut samples = self.samples.borrow_mut();
        let made = samples.entry(id).or_insert_with_key(|id| {
            let (image, size) = make()?;
            let name = format!("effect-{id}");
            let options = egui::TextureOptions::LINEAR;
            Some((ui.ctx().load_texture(name, image, options), size))
        });
        made.as_ref().map(|(texture, size)| (texture.id(), *size))
    }
}

/// `words` with effects, for a tile.
fn sample(
    words: &str,
    font: &str,
    color: Color,
    effects: Vec<TextEffect>,
    curve: f64,
) -> Option<(egui::ColorImage, Vec2)> {
    let style = SpanStyle {
        font: font.to_owned(),
        size: if curve == 0.0 { 48.0 } else { 26.0 },
        color,
        bold: font == "Inter",
        italic: false,
    };
    let mut text = TextNode::plain(words, &style);
    text.align = gafira_doc::TextAlign::Center;
    (text.effects, text.curve) = (effects, curve);
    let placement = Placement::new(0.0, 0.0, 110.0, 64.0);
    let (image, cover) = canvas::curved_text_image(&text, &placement, 2.0)?;
    // Small sizes: the casts lose nothing seen.
    Some((image, Vec2::new(cover.w as f32, cover.h as f32)))
}

fn tile_side(ui: &Ui) -> f32 {
    ((ui.available_width() - 2.0 * GAP) / 3.0).floor().max(40.0)
}

/// A square with a sample, and its name under it.
fn tile(
    ui: &mut Ui,
    theme: &Theme,
    (sample, name): (Option<(egui::TextureId, Vec2)>, String),
    (side, on): (f32, bool),
) -> Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::new(side, side + 22.0), Sense::click());
    let square = Rect::from_min_size(rect.min, Vec2::splat(side));
    let (edge, width) = if on {
        (theme.c("accent"), 2.5)
    } else if response.hovered() {
        (theme.c("text.secondary"), 1.5)
    } else {
        (theme.c("border"), 1.0)
    };
    let painter = ui.painter();
    let stroke = Stroke::new(width, edge);
    painter.rect(
        square,
        theme.radius_md(),
        theme.c("surface"),
        stroke,
        StrokeKind::Inside,
    );
    if let Some((texture, size)) = sample {
        // All at one size: what is around the letters may be cut.
        let place = Rect::from_center_size(square.center(), size * side / 100.0);
        let uv = Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0));
        painter
            .with_clip_rect(square.shrink(3.0).intersect(ui.clip_rect()))
            .image(texture, place, uv, egui::Color32::WHITE);
    }
    let ink = theme.c("text");
    let galley = painter.layout_no_wrap(name.clone(), theme.font("caption"), ink);
    let x = square.center().x - (galley.size().x / 2.0).min(side / 2.0);
    painter
        .with_clip_rect(rect.intersect(ui.clip_rect()))
        .galley(Pos2::new(x, square.bottom() + 4.0), galley, ink);
    response.widget_info(|| {
        egui::WidgetInfo::selected(egui::WidgetType::Button, true, on, name.clone())
    });
    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// A colour, at the right of its name.
fn swatch(ui: &mut Ui, theme: &Theme, name: String, color: &mut Color) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.set_min_height(34.0);
        ui.label(RichText::new(name).font(theme.font("body-sm")));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let mut rgba = canvas::picker_rgba(Some(*color));
            if ui.color_edit_button_srgba_unmultiplied(&mut rgba).changed() {
                *color = canvas::picked_color(Some(*color), rgba);
                changed = true;
            }
        });
    });
    changed
}

/// The numbers and the colours of the effect `at` of `text`.
fn numbers(
    ui: &mut Ui,
    theme: &Theme,
    texts: &Texts,
    (text, at): (&TextNode, usize),
    asks: &mut Asks,
) {
    let Some(now) = text.effects.get(at) else {
        return;
    };
    let mut next = *now;
    let mut rest = TextEffect::new(now.kind);
    ui.add_space(8.0);
    egui::Frame::NONE
        .fill(theme.c("surface.sunken"))
        .corner_radius(theme.radius_md())
        .inner_margin(egui::Margin::symmetric(12, 10))
        .show(ui, |ui| {
            // The fields stand out of the card.
            let white = theme.c("surface");
            let widgets = &mut ui.visuals_mut().widgets;
            for state in [
                &mut widgets.inactive,
                &mut widgets.hovered,
                &mut widgets.active,
            ] {
                state.weak_bg_fill = white;
                state.bg_fill = white;
            }
            for number in now.kind.numbers() {
                let range = match number {
                    N::Angle => (-180.0, 180.0),
                    _ => (0.0, 100.0),
                };
                let back = *rest.number(*number);
                let what = (next.number(*number), range, back);
                slider(
                    ui,
                    theme,
                    plain(texts, number_key(*number)),
                    what,
                    Track::Plain,
                );
            }
            let names = ["effects.color", "effects.second"];
            let colors = [&mut next.color, &mut next.second];
            for (name, color) in names.into_iter().zip(colors).take(now.kind.colors()) {
                swatch(ui, theme, plain(texts, name), color);
            }
        });
    if next != *now {
        let mut effects = text.effects.clone();
        effects[at] = next;
        asks.effects = Some(effects);
    }
}

/// The button that opens the panel, in Propiedades.
pub(crate) fn button(ui: &mut Ui, theme: &Theme, texts: &Texts) -> bool {
    main_button(ui, theme, plain(texts, "effects.title"), true)
}

#[cfg(test)]
mod tests {
    use super::*;

    // S89-04, S89-05: every effect and style has a name and a sample of its
    // own, in a font that is there; the styles are valid.
    #[test]
    fn s89_05_names_and_samples() {
        let mut names: Vec<&str> = K::ALL
            .into_iter()
            .map(key)
            .chain(READY.iter().map(|ready| ready.key))
            .collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), 21);
        let mut seen: Vec<egui::ColorImage> = Vec::new();
        let ink = hex(0x18_24_3B);
        let effects = K::ALL.map(|kind| ("Inter", ink, vec![TextEffect::new(kind)]));
        let styles = READY.iter().map(|r| (r.font, hex(r.color), (r.effects)()));
        for (font, color, effects) in effects.into_iter().chain(styles) {
            assert!(effects.len() <= TextEffect::MAX, "{font}");
            assert!(effects.iter().all(TextEffect::is_valid), "{font}");
            let (made, _) = sample("Ag", font, color, effects.clone(), 0.0).expect(font);
            assert!(!seen.contains(&made), "{font} {effects:?}");
            seen.push(made);
        }
        assert!(sample("ABCD", "Inter", ink, Vec::new(), 45.0).is_some());
    }

    // S89-02: the picture of the canvas has all of the effect: nothing is cut
    // at its edges, and what is hollow is clear inside.
    #[test]
    fn s89_02_the_picture_is_whole() {
        for kind in K::ALL {
            let made = sample(
                "Ag",
                "Inter",
                hex(0x18_24_3B),
                vec![TextEffect::new(kind)],
                0.0,
            );
            let (made, _) = made.expect("a picture");
            let [w, h] = made.size;
            let edge = (0..w * h).filter(|i| {
                let (x, y) = (i % w, i / w);
                x == 0 || y == 0 || x + 1 == w || y + 1 == h
            });
            for i in edge {
                assert!(made.pixels[i].a() < 20, "{kind:?}");
            }
        }
        let ink = |kinds: Vec<TextEffect>| {
            let (made, _) = sample("I", "Inter", hex(0x18_24_3B), kinds, 0.0).expect("a picture");
            made.pixels.iter().map(|p| u64::from(p.a())).sum::<u64>()
        };
        assert!(ink(vec![TextEffect::new(K::Hollow)]) < ink(Vec::new()));
    }
}
