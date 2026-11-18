//! The buttons and bars of the component catalog (03 §8) the shell uses.

use egui::{
    Button, Rect, Response, RichText, Sense, Stroke, StrokeKind, Ui, Vec2, WidgetInfo, WidgetType,
};

use crate::icons::{self, Icon, Weight};
use crate::style::{Theme, px};

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

/// A flat button on the graphite top bar.
pub(crate) fn on_topbar(ui: &mut Ui, theme: &Theme, text: String, active: bool) -> Response {
    let color = if active {
        theme.c("topbar.fg")
    } else {
        theme.c("topbar.fg.muted")
    };
    let button = Button::new(
        RichText::new(text)
            .font(theme.font("ui-label"))
            .color(color),
    )
    .frame(false)
    .min_size(Vec2::new(0.0, px(theme.tokens.size.control.md)));
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

/// A secondary action with an icon before its label (Piezas, 03 §13: an
/// icon whose meaning may be ambiguous carries a label).
pub(crate) fn secondary_icon(ui: &mut Ui, theme: &Theme, icon: Icon, text: String) -> Response {
    let color = theme.c("secondary.fg");
    let image = icons::image(ui.ctx(), icon, Weight::Regular, icon_size(theme), color);
    let button = Button::image_and_text(
        image,
        RichText::new(text)
            .font(theme.font("ui-label"))
            .color(color),
    )
    .fill(theme.c("secondary.bg"))
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
    let (rect, response) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), RAIL_TAB), Sense::click());
    let painter = ui.painter();
    if response.hovered() {
        painter.rect_filled(rect, 0.0, theme.c("topbar.hover"));
    }
    let (color, weight) = if active {
        let bar = Rect::from_min_size(rect.min, Vec2::new(3.0, rect.height()));
        painter.rect_filled(bar, 0.0, theme.c("primary.bg"));
        (theme.c("topbar.fg"), Weight::Fill)
    } else {
        (theme.c("topbar.fg.muted"), Weight::Regular)
    };
    let size = px(theme.tokens.size.icon.lg);
    let glyph = Rect::from_center_size(
        egui::pos2(rect.center().x, rect.min.y + 6.0 + size / 2.0),
        Vec2::splat(size),
    );
    icons::paint(painter, glyph, icon, weight, color);
    painter.text(
        egui::pos2(rect.center().x, rect.max.y - 6.0),
        egui::Align2::CENTER_BOTTOM,
        label,
        theme.font("caption"),
        color,
    );
    if response.has_focus() {
        focus_ring(painter, theme, rect.shrink(4.0));
    }
    response.widget_info(|| WidgetInfo::selected(WidgetType::Button, true, active, label));
    response
}

/// The height of a rail tab: icon, gap and caption.
const RAIL_TAB: f32 = 56.0;

/// The 2 px focus ring of 03 §8.1 around `rect`.
fn focus_ring(painter: &egui::Painter, theme: &Theme, rect: Rect) {
    painter.rect_stroke(
        rect,
        theme.radius_sm(),
        Stroke::new(px(theme.tokens.border.focus), theme.c("focus.ring")),
        StrokeKind::Outside,
    );
}

/// The graphite top bar of every screen (03 §7.1: 48 px).
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
