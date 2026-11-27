//! The buttons and bars of the component catalog (03 §8) the shell uses.

use egui::{
    Button, Rect, Response, RichText, Sense, Stroke, StrokeKind, Ui, Vec2, WidgetInfo, WidgetType,
};

use crate::icons::{self, Icon, Weight};
use crate::style::{Theme, px};

/// A one-line text field of the interface (03, DIS-88): 10 px of
/// room left and right and 8 px above and below, so the text and its hint
/// never touch the border, the width of its column. Every text field of the
/// interface is built with it.
pub(crate) fn field(text: &mut String, hint: String) -> egui::TextEdit<'_> {
    egui::TextEdit::singleline(text)
        .hint_text(hint)
        .margin(Vec2::new(10.0, 8.0))
        .desired_width(f32::INFINITY)
}

/// An on/off switch (spec 073): a pill that slides its knob to the right
/// and fills with the brand colour when on; a hand over it.
pub(crate) fn toggle(ui: &mut Ui, theme: &Theme, on: &mut bool) -> Response {
    let size = Vec2::new(36.0, 20.0);
    let (rect, mut response) = ui.allocate_exact_size(size, Sense::click());
    if response.clicked() {
        *on = !*on;
        response.mark_changed();
    }
    let t = ui.ctx().animate_bool(response.id, *on);
    let painter = ui.painter();
    let fill = if *on {
        theme.c("primary.bg")
    } else {
        theme.c("border")
    };
    painter.rect_filled(rect, rect.height() / 2.0, fill);
    let r = rect.height() / 2.0 - 2.0;
    let x = egui::lerp(rect.left() + r + 2.0..=rect.right() - r - 2.0, t);
    painter.circle_filled(egui::pos2(x, rect.center().y), r, egui::Color32::WHITE);
    if response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    response.widget_info(|| WidgetInfo::selected(WidgetType::Checkbox, true, *on, ""));
    response
}

/// The coral button: one per view (DIS-15).
pub(crate) fn primary(ui: &mut Ui, theme: &Theme, text: String) -> Response {
    let height = px(theme.tokens.size.control.lg);
    let button = Button::new(
        RichText::new(text)
            .font(theme.font("ui-label"))
            .color(theme.c("primary.fg")),
    )
    .fill(theme.c("primary.bg"))
    .corner_radius(theme.radius_md())
    .min_size(Vec2::new(0.0, height));
    ui.add(button)
}

/// A secondary action.
pub(crate) fn secondary(ui: &mut Ui, theme: &Theme, text: String) -> Response {
    let height = px(theme.tokens.size.control.lg);
    let button = Button::new(
        RichText::new(text)
            .font(theme.font("ui-label"))
            .color(theme.c("secondary.fg")),
    )
    .fill(theme.c("secondary.bg"))
    .corner_radius(theme.radius_md())
    .min_size(Vec2::new(0.0, height));
    ui.add(button)
}

/// The coral button with an icon before its label («Exportar», 03 §2.2).
pub(crate) fn primary_icon(ui: &mut Ui, theme: &Theme, icon: Icon, text: String) -> Response {
    let color = theme.c("primary.fg");
    let image = icons::image(ui.ctx(), icon, Weight::Regular, icon_size(theme), color);
    let button = Button::image_and_text(
        image,
        RichText::new(text)
            .font(theme.font("ui-label"))
            .color(color),
    )
    .fill(theme.c("primary.bg"))
    .corner_radius(theme.radius_md())
    .min_size(Vec2::new(0.0, px(theme.tokens.size.control.lg)));
    ui.add(button)
}

/// A flat button with an icon and a label on the graphite top bar
/// («Versiones», 03 §7.3).
pub(crate) fn on_topbar_icon(ui: &mut Ui, theme: &Theme, icon: Icon, text: String) -> Response {
    let color = theme.c("topbar.fg");
    let image = icons::image(ui.ctx(), icon, Weight::Regular, icon_size(theme), color);
    let button = Button::image_and_text(
        image,
        RichText::new(text)
            .font(theme.font("ui-label"))
            .color(theme.c("topbar.fg.muted")),
    )
    .frame(false)
    .min_size(Vec2::new(0.0, px(theme.tokens.size.control.md)));
    ui.add(button)
}

/// The icon of a «Botón de icono» (03 §8.1: 20 px).
fn icon_size(theme: &Theme) -> f32 {
    px(theme.tokens.size.icon.md)
}

/// Where an icon button sits; its colours are that surface's tokens.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Surface {
    /// The graphite top bar (`topbar.*`).
    Topbar,
    /// Panels and bars on `bg` or `surface` (`text`, `secondary.*`).
    Panel,
}

/// The «Botón de icono» of 03 §8.1: no fill at rest, a 20 px icon in a
/// 32 px zone (16 in 28 when small), a tooltip and an accessible name from
/// the catalog.
pub(crate) struct IconButton {
    icon: Icon,
    name: String,
    tooltip: Option<String>,
    surface: Surface,
    small: bool,
    /// A switch that is on (Imán, spec 044): drawn on `selection.bg`.
    active: Option<bool>,
}

impl IconButton {
    /// `name` is the accessible name and, unless replaced, the tooltip.
    pub(crate) fn new(icon: Icon, name: String) -> Self {
        Self {
            icon,
            name,
            tooltip: None,
            surface: Surface::Panel,
            small: false,
            active: None,
        }
    }

    /// A switch, on or off; its accessible state says which.
    pub(crate) fn active(mut self, on: bool) -> Self {
        self.active = Some(on);
        self
    }

    pub(crate) fn on(mut self, surface: Surface) -> Self {
        self.surface = surface;
        self
    }

    /// A tooltip other than the name («Deshacer · Ctrl+Z»).
    pub(crate) fn tooltip(mut self, text: String) -> Self {
        self.tooltip = Some(text);
        self
    }

    pub(crate) fn small(mut self) -> Self {
        self.small = true;
        self
    }

    pub(crate) fn show(self, ui: &mut Ui, theme: &Theme) -> Response {
        let size = &theme.tokens.size;
        let (zone, icon) = if self.small {
            (px(size.control.sm), px(size.icon.sm))
        } else {
            (px(size.control.md), px(size.icon.md))
        };
        let (rect, response) = ui.allocate_exact_size(Vec2::splat(zone), Sense::click());
        let enabled = ui.is_enabled();
        let (fg, disabled, hover, pressed) = match self.surface {
            Surface::Topbar => (
                theme.c("topbar.fg"),
                theme.c("topbar.fg.muted").gamma_multiply(0.5),
                theme.c("topbar.hover"),
                theme.c("topbar.hover"),
            ),
            Surface::Panel => (
                theme.c("text"),
                theme.c("text.disabled"),
                theme.c("secondary.bg"),
                theme.c("secondary.bg.hover"),
            ),
        };
        let fill = if !enabled {
            None
        } else if response.is_pointer_button_down_on() {
            Some(pressed)
        } else if response.hovered() {
            Some(hover)
        } else if self.active == Some(true) {
            Some(theme.c("selection.bg"))
        } else {
            None
        };
        let painter = ui.painter();
        if let Some(fill) = fill {
            painter.rect_filled(rect, theme.radius_sm(), fill);
        }
        if response.has_focus() {
            focus_ring(painter, theme, rect);
        }
        let color = if enabled { fg } else { disabled };
        icons::paint(
            painter,
            Rect::from_center_size(rect.center(), Vec2::splat(icon)),
            self.icon,
            Weight::Regular,
            color,
        );
        let name = self.name;
        match self.active {
            Some(on) => response
                .widget_info(|| WidgetInfo::selected(WidgetType::Checkbox, enabled, on, &name)),
            None => {
                response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, enabled, &name))
            }
        }
        let tooltip = self.tooltip.unwrap_or_else(|| name.clone());
        response
            .on_hover_text(tooltip.clone())
            .on_disabled_hover_text(tooltip)
    }
}

/// The «Pestaña del riel» of 03 §8.1: a 24 px icon over its label on the
/// graphite rail; the active one draws the Fill weight (D-87) in
/// `topbar.fg` with the coral indicator, the others Regular in
/// `topbar.fg.muted`.
pub(crate) fn rail_tab(
    ui: &mut Ui,
    theme: &Theme,
    icon: Icon,
    label: &str,
    active: bool,
) -> Response {
    tab_of(ui, theme, (icon, label), active, true)
}

/// A tab of the rail that is still to come: faint, and nothing to press.
pub(crate) fn rail_tab_off(ui: &mut Ui, theme: &Theme, icon: Icon, label: &str) -> Response {
    tab_of(ui, theme, (icon, label), false, false)
}

fn tab_of(
    ui: &mut Ui,
    theme: &Theme,
    (icon, label): (Icon, &str),
    active: bool,
    enabled: bool,
) -> Response {
    // The label a size under the caption, with 6 px of room each side so it
    // never touches the rail's edges (owner request); one too long for a
    // line takes two, and the tab grows for it.
    let mut font = theme.font("caption");
    font.size = (font.size - 1.0).max(9.0);
    let width = ui.available_width();
    let room_width = width - 2.0 * RAIL_LABEL_ROOM;
    let measure = |text: &str| {
        ui.fonts_mut(|f| {
            f.layout_no_wrap(text.to_owned(), font.clone(), egui::Color32::WHITE)
                .size()
                .x
        })
    };
    let lines = rail_lines(label, room_width, &measure);
    let extra = (lines.len() as f32 - 1.0) * (font.size + 2.0);
    let (rect, response) = ui.allocate_exact_size(
        Vec2::new(width, RAIL_TAB + extra),
        if enabled {
            Sense::click()
        } else {
            Sense::hover()
        },
    );
    let painter = ui.painter();
    if response.hovered() && enabled {
        painter.rect_filled(rect, 0.0, theme.c("topbar.hover"));
    }
    let (color, weight) = if active {
        let bar = Rect::from_min_size(rect.min, Vec2::new(3.0, rect.height()));
        painter.rect_filled(bar, 0.0, theme.c("primary.bg"));
        (theme.c("topbar.fg"), Weight::Fill)
    } else {
        (theme.c("topbar.fg.muted"), Weight::Regular)
    };
    let color = if enabled {
        color
    } else {
        color.gamma_multiply(0.45)
    };
    let size = px(theme.tokens.size.icon.lg);
    let glyph = Rect::from_center_size(
        egui::pos2(rect.center().x, rect.min.y + 6.0 + size / 2.0),
        Vec2::splat(size),
    );
    icons::paint(painter, glyph, icon, weight, color);
    let room = rect.shrink2(Vec2::new(RAIL_LABEL_ROOM, 0.0));
    let step = font.size + 2.0;
    for (k, line) in lines.iter().rev().enumerate() {
        painter.with_clip_rect(room).text(
            egui::pos2(rect.center().x, rect.max.y - 6.0 - step * k as f32),
            egui::Align2::CENTER_BOTTOM,
            line,
            font.clone(),
            color,
        );
    }
    if response.has_focus() {
        focus_ring(painter, theme, rect.shrink(4.0));
    }
    response.widget_info(|| WidgetInfo::selected(WidgetType::Button, enabled, active, label));
    response
}

/// The lines of a rail label: one when it fits `room` px, else two cut at
/// the space that leaves the longer line shortest; a single word stays one
/// line (and is cut by the clip).
fn rail_lines(label: &str, room: f32, width: &dyn Fn(&str) -> f32) -> Vec<String> {
    if width(label) <= room {
        return vec![label.to_owned()];
    }
    let cuts = label.match_indices(' ').map(|(at, _)| at);
    let best = cuts.min_by(|a, b| {
        let longer = |at: usize| width(&label[..at]).max(width(&label[at + 1..]));
        longer(*a).total_cmp(&longer(*b))
    });
    match best {
        Some(at) => vec![label[..at].to_owned(), label[at + 1..].to_owned()],
        None => vec![label.to_owned()],
    }
}

/// The height of a rail tab: icon, gap and caption.
const RAIL_TAB: f32 = 56.0;

/// The room left and right of a rail tab's label, in px.
const RAIL_LABEL_ROOM: f32 = 6.0;

/// The 2 px focus ring of 03 §8.1 around `rect`.
fn focus_ring(painter: &egui::Painter, theme: &Theme, rect: Rect) {
    painter.rect_stroke(
        rect,
        theme.radius_sm(),
        Stroke::new(px(theme.tokens.border.focus), theme.c("focus.ring")),
        StrokeKind::Outside,
    );
}

/// A menu of the graphite top bar (Archivo, Edición, Ver): no fill at rest
/// and `topbar.hover` when pointed at or open, with the `topbar.*` text, in
/// both themes. egui's own menu button takes the panel colours of the
/// light theme, a light chip under light text on the graphite bar. The
/// menu itself opens with the window's style.
pub(crate) fn topbar_menu(
    ui: &mut Ui,
    theme: &Theme,
    text: String,
    add: impl FnOnce(&mut Ui),
) -> Response {
    ui.scope(|ui| {
        topbar_menu_visuals(ui.visuals_mut(), theme);
        let label = RichText::new(text).font(theme.font("ui-label"));
        ui.menu_button(label, |ui| {
            *ui.style_mut() = (*ui.ctx().style()).clone();
            menu_style(ui, theme);
            add(ui);
        })
        .response
    })
    .inner
}

/// A button of the top bar that looks like its menus (owner request:
/// «Redimensionar» like Archivo, Edición, Ver): the same hover and, while
/// what it opens is open, the same lit state.
pub(crate) fn topbar_button(ui: &mut Ui, theme: &Theme, text: String, open: bool) -> Response {
    ui.scope(|ui| {
        topbar_menu_visuals(ui.visuals_mut(), theme);
        if open {
            let lit = ui.visuals().widgets.open;
            ui.visuals_mut().widgets.inactive = lit;
        }
        let label = RichText::new(text).font(theme.font("ui-label"));
        ui.button(label)
    })
    .inner
}

/// The widget colours of [`topbar_menu`]'s button.
fn topbar_menu_visuals(visuals: &mut egui::Visuals, theme: &Theme) {
    // The window's text colour would win over the states' own colours.
    visuals.override_text_color = None;
    let widgets = &mut visuals.widgets;
    let rest = &mut widgets.inactive;
    rest.weak_bg_fill = egui::Color32::TRANSPARENT;
    rest.bg_fill = egui::Color32::TRANSPARENT;
    rest.bg_stroke = Stroke::NONE;
    rest.fg_stroke.color = theme.c("topbar.fg.muted");
    for state in [&mut widgets.hovered, &mut widgets.active, &mut widgets.open] {
        state.weak_bg_fill = theme.c("topbar.hover");
        state.bg_fill = theme.c("topbar.hover");
        state.bg_stroke = Stroke::NONE;
        state.fg_stroke.color = theme.c("topbar.fg");
        state.expansion = 0.0;
    }
}

/// The pointing hand over anything that is only clicked (buttons, menus,
/// tabs, rows), like the tool rail: run once at the end of every pass.
/// Widgets that are also dragged (the canvas, fields, sliders) and any
/// cursor a widget already chose (resize arrows, grab) are left alone.
pub(crate) fn pointing_hand(ctx: &egui::Context) {
    if ctx.output(|o| o.cursor_icon) != egui::CursorIcon::Default {
        return;
    }
    let hovered = ctx.interaction_snapshot(|s| s.hovered.clone());
    let clickable = hovered.into_iter().any(|id| {
        ctx.read_response(id)
            .is_some_and(|r| r.enabled() && r.sense.senses_click() && !r.sense.senses_drag())
    });
    if clickable {
        ctx.set_cursor_icon(egui::CursorIcon::PointingHand);
    }
}

/// The graphite top bar of every screen (03 §7.1: 48 px).
/// The horizontal logo for dark backgrounds (brand manual), `height`
/// points tall, on the graphite top bar.
pub(crate) fn topbar_logo(ui: &mut Ui, height: f32) -> Response {
    const LOGO: &str = include_str!("../../../assets/gafira-logo-dark-background.svg");
    // The logo's own proportions: 1325 × 425.
    let size = Vec2::new(height * 1325.0 / 425.0, height);
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
    let side = (size.x * ui.ctx().pixels_per_point()).round() as u32;
    if let Some(texture) =
        crate::design_art::texture(ui.ctx(), "logo/dark", side, || LOGO.to_owned())
    {
        ui.painter().image(
            texture.id(),
            rect,
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            egui::Color32::WHITE,
        );
    }
    response
}

/// A place of the top bar of the start screens: its icon and name, lit
/// with a soft fill and a coral line under it when it is the one open.
pub(crate) fn nav_item(
    ui: &mut Ui,
    theme: &Theme,
    icon: Icon,
    text: &str,
    active: bool,
) -> Response {
    let font = theme.font("ui-label");
    let galley = ui
        .painter()
        .layout_no_wrap(text.to_owned(), font, theme.c("topbar.fg"));
    let size = Vec2::new(galley.size().x + 18.0 + 8.0 + 24.0, 34.0);
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
    let hover = response.hovered();
    if active || hover {
        ui.painter().rect_filled(rect, 8.0, theme.c("topbar.hover"));
    }
    let color = if active || hover {
        theme.c("topbar.fg")
    } else {
        theme.c("topbar.fg.muted")
    };
    let weight = if active {
        Weight::Fill
    } else {
        Weight::Regular
    };
    let icon_rect = egui::Rect::from_center_size(
        egui::pos2(rect.left() + 12.0 + 9.0, rect.center().y),
        Vec2::splat(18.0),
    );
    icons::paint(ui.painter(), icon_rect, icon, weight, color);
    ui.painter().galley(
        egui::pos2(
            icon_rect.right() + 8.0,
            rect.center().y - galley.size().y / 2.0,
        ),
        galley,
        color,
    );
    if active {
        let line = egui::Rect::from_min_size(
            egui::pos2(rect.left() + 10.0, rect.bottom() + 5.0),
            Vec2::new(rect.width() - 20.0, 3.0),
        );
        ui.painter().rect_filled(line, 1.5, theme.c("primary.bg"));
    }
    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}

pub(crate) fn topbar(ctx: &egui::Context, theme: &Theme, id: &str, add: impl FnOnce(&mut Ui)) {
    egui::TopBottomPanel::top(id.to_owned())
        .exact_height(px(theme.tokens.size.topbar))
        .frame(
            egui::Frame::NONE
                .fill(theme.c("topbar.bg"))
                .inner_margin(egui::Margin::symmetric(12, 0)),
        )
        .show(ctx, |ui| {
            ui.horizontal_centered(add);
        });
    // The tabs of the open projects, right under the menus (spec 111);
    // Ajustes covers the window and shows none.
    if id != "settings.topbar" {
        crate::tabs::under_topbar(ctx, theme);
    }
}

#[cfg(test)]
mod tests {
    use egui::accesskit::{Node, Role, Toggled};
    use gafira_theme::ThemeTokens;

    use super::*;

    /// The accessibility tree of one frame that runs `add` in a panel.
    fn tree(mut add: impl FnMut(&mut Ui)) -> Vec<Node> {
        let ctx = egui::Context::default();
        ctx.enable_accesskit();
        let output = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| add(ui));
        });
        output
            .platform_output
            .accesskit_update
            .expect("an accessibility tree")
            .nodes
            .into_iter()
            .map(|(_, node)| node)
            .collect()
    }

    #[test]
    fn s37_05_an_icon_button_is_a_button_named_from_the_catalog() {
        let theme = Theme::new(ThemeTokens::embedded().expect("tokens"));
        let nodes = tree(|ui| {
            IconButton::new(Icon::Undo, "Deshacer".to_owned())
                .tooltip("Deshacer · Ctrl+Z".to_owned())
                .show(ui, &theme);
            ui.add_enabled_ui(false, |ui| {
                IconButton::new(Icon::Redo, "Rehacer".to_owned()).show(ui, &theme)
            });
        });
        let undo = nodes
            .iter()
            .find(|n| n.label() == Some("Deshacer"))
            .expect("the name is the catalog text, not the tooltip");
        assert_eq!(undo.role(), Role::Button);
        assert!(!undo.is_disabled());
        let redo = nodes
            .iter()
            .find(|n| n.label() == Some("Rehacer"))
            .expect("a disabled button keeps its name");
        assert!(redo.is_disabled());
    }

    #[test]
    fn s37_05_a_rail_tab_says_whether_it_is_the_active_one() {
        let theme = Theme::new(ThemeTokens::embedded().expect("tokens"));
        let nodes = tree(|ui| {
            rail_tab(ui, &theme, Icon::Pieces, "Piezas", true);
            rail_tab(ui, &theme, Icon::Text, "Texto", false);
        });
        let toggled = |label: &str| {
            nodes
                .iter()
                .find(|n| n.label() == Some(label))
                .and_then(Node::toggled)
        };
        assert_eq!(toggled("Piezas"), Some(Toggled::True));
        assert_eq!(toggled("Texto"), Some(Toggled::False));
    }
}

#[cfg(test)]
mod topbar_menu_tests {
    use super::*;

    // The menus of the graphite bar never draw a light chip under their
    // light text: no fill at rest, `topbar.hover` when pointed at or open,
    // and `topbar.*` text, in the light theme as in the dark one.
    #[test]
    fn topbar_menus_take_the_bar_colours_in_both_themes() {
        let tokens = gafira_theme::ThemeTokens::embedded().expect("tokens");
        for dark in [false, true] {
            let mut theme = Theme::new(tokens.clone());
            theme.dark = dark;
            let mut visuals = if dark {
                egui::Visuals::dark()
            } else {
                egui::Visuals::light()
            };
            topbar_menu_visuals(&mut visuals, &theme);
            let w = &visuals.widgets;
            assert_eq!(w.inactive.weak_bg_fill, egui::Color32::TRANSPARENT);
            assert_eq!(w.inactive.fg_stroke.color, theme.c("topbar.fg.muted"));
            assert_eq!(visuals.override_text_color, None);
            for state in [&w.hovered, &w.active, &w.open] {
                assert_eq!(state.weak_bg_fill, theme.c("topbar.hover"));
                assert_eq!(state.fg_stroke.color, theme.c("topbar.fg"));
            }
        }
    }
}

#[cfg(test)]
mod pointing_hand_tests {
    use super::*;

    fn cursor_over(widget: impl Fn(&mut Ui)) -> egui::CursorIcon {
        let ctx = egui::Context::default();
        let mut cursor = egui::CursorIcon::Default;
        for _ in 0..3 {
            let mut input = egui::RawInput::default();
            input
                .events
                .push(egui::Event::PointerMoved(egui::pos2(20.0, 15.0)));
            let output = ctx.run(input, |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| widget(ui));
                pointing_hand(ctx);
            });
            cursor = output.platform_output.cursor_icon;
        }
        cursor
    }

    // A button shows the hand; a widget that is also dragged keeps its own
    // cursor, and so does a cursor some widget already set.
    #[test]
    fn buttons_show_the_hand_and_dragged_widgets_do_not() {
        assert_eq!(
            cursor_over(|ui| {
                ui.add_sized([200.0, 100.0], Button::new("Abrir"));
            }),
            egui::CursorIcon::PointingHand
        );
        assert_eq!(
            cursor_over(|ui| {
                ui.allocate_exact_size(Vec2::new(200.0, 100.0), Sense::click_and_drag());
            }),
            egui::CursorIcon::Default
        );
        assert_eq!(
            cursor_over(|ui| {
                ui.add_sized([200.0, 100.0], Button::new("Abrir"))
                    .on_hover_cursor(egui::CursorIcon::ResizeNwSe);
            }),
            egui::CursorIcon::ResizeNwSe
        );
        assert_eq!(
            cursor_over(|ui| {
                ui.add_enabled_ui(false, |ui| {
                    ui.add_sized([200.0, 100.0], Button::new("Abrir"));
                });
            }),
            egui::CursorIcon::Default
        );
    }

    // Owner request: every rail label, in Spanish and English, fits the
    // rail with its room on both sides.
    #[test]
    fn rail_labels_keep_their_room() {
        let theme = Theme::new(gafira_theme::ThemeTokens::embedded().expect("tokens"));
        let ctx = egui::Context::default();
        theme.apply(&ctx);
        let room = px(theme.tokens.size.rail) - 2.0 * RAIL_LABEL_ROOM;
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            let mut font = theme.font("caption");
            font.size -= 1.0;
            for label in [
                "Plantillas",
                "Elementos",
                "Texto",
                "Marca",
                "Biblioteca",
                "Dibujo",
                "Templates",
                "Elements",
                "Text",
                "Brand",
                "Library",
                "Drawing",
            ] {
                let w = ctx.fonts_mut(|f| {
                    f.layout_no_wrap(label.to_owned(), font.clone(), egui::Color32::WHITE)
                        .size()
                        .x
                });
                assert!(w <= room, "{label}: {w} > {room}");
            }
        });
    }
}

/// The arrow of an item that opens a submenu (spec 098): an icon, as the
/// interface font has no «⏵» or «›».
pub(crate) fn submenu_arrow(ui: &Ui) -> egui::Image<'static> {
    icons::image(
        ui.ctx(),
        Icon::Submenu,
        Weight::Regular,
        12.0,
        ui.visuals().text_color(),
    )
}

/// A frameless «Volver» arrow, drawn as an icon (spec 098).
pub(crate) fn back_arrow(ui: &mut Ui) -> Response {
    let image = icons::image(
        ui.ctx(),
        Icon::Back,
        Weight::Regular,
        16.0,
        ui.visuals().text_color(),
    );
    ui.add(egui::Button::image(image).frame(false))
}

/// `text` after the icon of a state (ready, in use, an error), in `color`;
/// the words wrap beside it.
pub(crate) fn note(ui: &mut Ui, icon: Icon, text: egui::RichText, color: egui::Color32) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 6.0;
        ui.add(icons::image(ui.ctx(), icon, Weight::Regular, 14.0, color));
        ui.add(egui::Label::new(text.color(color)).wrap());
    });
}

/// The look of a menu (spec 098): its items have no fill at rest, a soft
/// one under the pointer, and room to be hit, like a list and not a column
/// of buttons. Its submenus inherit it.
pub(crate) fn menu_style(ui: &mut Ui, theme: &Theme) {
    let style = ui.style_mut();
    style.spacing.button_padding = egui::Vec2::new(10.0, 7.0);
    style.spacing.item_spacing.y = 2.0;
    style.spacing.icon_spacing = 10.0;
    let widgets = &mut style.visuals.widgets;
    // The separators keep the stroke of `noninteractive`.
    widgets.inactive.weak_bg_fill = egui::Color32::TRANSPARENT;
    widgets.inactive.bg_stroke = Stroke::NONE;
    for state in [&mut widgets.hovered, &mut widgets.active, &mut widgets.open] {
        state.weak_bg_fill = theme.c("secondary.bg");
        state.bg_stroke = Stroke::NONE;
        state.expansion = 0.0;
    }
}

/// An item of a menu that is on or off (Reglas, Guías, Modo oscuro): a tick
/// on its left while on, no fill, its shortcut on the right (spec 098).
pub(crate) fn check_item(ui: &mut Ui, on: bool, text: String, keys: Option<String>) -> Response {
    let ink = if on {
        ui.visuals().text_color()
    } else {
        egui::Color32::TRANSPARENT
    };
    let tick = icons::image(ui.ctx(), Icon::Check, Weight::Regular, 16.0, ink);
    let mut item = egui::Button::image_and_text(tick, text.clone());
    if let Some(keys) = keys {
        item = item.shortcut_text(keys);
    }
    let response = ui.add(item);
    response
        .widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::Checkbox, true, on, &text));
    response
}

#[cfg(test)]
mod glyph_tests {
    use std::path::Path;

    /// The characters of `text` the interface font cannot draw.
    fn missing(ctx: &egui::Context, text: &str) -> Vec<char> {
        let font = egui::FontId::proportional(14.0);
        text.chars()
            .filter(|c| !c.is_ascii())
            .filter(|c| !ctx.fonts_mut(|f| f.has_glyph(&font, *c)))
            .collect()
    }

    /// The string literals of `source` outside its tests.
    fn literals(source: &str) -> Vec<String> {
        let code = source.split("#[cfg(test)]").next().unwrap_or_default();
        let mut out = Vec::new();
        for line in code.lines() {
            let line = line.split("//").next().unwrap_or_default();
            let mut rest = line;
            while let Some(start) = rest.find('"') {
                let after = &rest[start + 1..];
                let Some(end) = after.find('"') else { break };
                out.push(after[..end].to_owned());
                rest = &after[end + 1..];
            }
        }
        out
    }

    // Spec 098: the interface font is a subset (spec 014); a sign it lacks
    // shows as «?». Every text of the catalogs and of the interface code
    // has a glyph.
    #[test]
    fn s98_09_every_character_the_interface_writes_has_a_glyph() {
        let ctx = egui::Context::default();
        crate::style::Theme::new(gafira_theme::ThemeTokens::embedded().expect("tokens"))
            .apply(&ctx);
        let _ = ctx.run(egui::RawInput::default(), |_| {});
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let mut bad = Vec::new();
        for lang in ["es", "en"] {
            let path = root.join(format!("../../assets/i18n/{lang}.toml"));
            let catalog = std::fs::read_to_string(&path).expect("catalog");
            for (n, line) in catalog.lines().enumerate() {
                let lost = missing(&ctx, line);
                if !lost.is_empty() {
                    bad.push(format!("{lang}.toml:{}: {lost:?}", n + 1));
                }
            }
        }
        let mut files = vec![root.join("src")];
        while let Some(dir) = files.pop() {
            for entry in std::fs::read_dir(&dir).expect("source dir").flatten() {
                let path = entry.path();
                if path.is_dir() {
                    if path.file_name().is_some_and(|n| n != "tests") {
                        files.push(path);
                    }
                } else if path.extension().is_some_and(|e| e == "rs") {
                    let source = std::fs::read_to_string(&path).expect("source");
                    for text in literals(&source) {
                        let lost = missing(&ctx, &text);
                        if !lost.is_empty() {
                            bad.push(format!("{}: {lost:?}", path.display()));
                        }
                    }
                }
            }
        }
        assert!(bad.is_empty(), "{bad:#?}");
    }
}

#[cfg(test)]
mod rail_label_tests {
    use super::rail_lines;

    // Owner request (spec 104): «Crear en lote» does not fit the rail on one
    // line; it takes two, cut where the lines are most alike.
    #[test]
    fn s104_59_a_long_rail_label_takes_two_lines() {
        let width = |text: &str| text.chars().count() as f32 * 6.0;
        assert_eq!(rail_lines("Texto", 60.0, &width), ["Texto"]);
        assert_eq!(
            rail_lines("Crear en lote", 60.0, &width),
            ["Crear", "en lote"]
        );
        assert_eq!(rail_lines("Bulk create", 60.0, &width), ["Bulk", "create"]);
        assert_eq!(rail_lines("Elementosss", 60.0, &width), ["Elementosss"]);
    }
}
