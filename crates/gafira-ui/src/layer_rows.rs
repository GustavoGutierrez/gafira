//! The look of a row of Capas (spec 098): the grip, the preview tile of the
//! element, the «⋮» button and the floating copy of a dragged row.

use egui::{Color32, Pos2, Rect, Response, Sense, Stroke, Ui, Vec2};
use gafira_doc::{Node, NodeKind};

use crate::canvas;
use crate::icons::{self, Icon, Weight};
use crate::style::Theme;

/// The height of a row.
pub(crate) const ROW_HEIGHT: f32 = 40.0;

/// Six dots in two columns: the row can be dragged.
pub(crate) fn grip(painter: &egui::Painter, centre: Pos2, color: Color32) {
    for column in [-2.5, 2.5] {
        for line in [-5.0, 0.0, 5.0] {
            painter.circle_filled(centre + Vec2::new(column, line), 1.4, color);
        }
    }
}

/// Three dots one above the other, the «⋮» of a menu.
fn dots(painter: &egui::Painter, centre: Pos2, color: Color32) {
    for line in [-5.0, 0.0, 5.0] {
        painter.circle_filled(centre + Vec2::new(0.0, line), 1.6, color);
    }
}

/// The «⋮» button of a row; `on` while its menu is open.
pub(crate) fn menu_button(ui: &mut Ui, theme: &Theme, name: &str, on: bool) -> Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::new(24.0, 28.0), Sense::click());
    let painter = ui.painter();
    if on {
        painter.rect_filled(rect, theme.radius_sm(), theme.c("text"));
        dots(painter, rect.center(), theme.c("surface"));
    } else {
        if response.hovered() {
            painter.rect_filled(rect, theme.radius_sm(), theme.c("secondary.bg.hover"));
        }
        dots(painter, rect.center(), theme.c("text"));
    }
    let name = name.to_owned();
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, &name));
    response.on_hover_text(name)
}

/// The icon that stands for an element without a colour of its own.
fn kind_icon(kind: &NodeKind) -> Icon {
    match kind {
        NodeKind::Group(_) => Icon::Stack,
        NodeKind::Text(_) => Icon::Text,
        NodeKind::Image(_) => Icon::Library,
        NodeKind::Frame(_) => Icon::CategoryFrames,
        NodeKind::Qr(_) => Icon::CategoryQrCodes,
        NodeKind::Table(_) => Icon::CategoryTables,
        NodeKind::Chart(_) => Icon::CategoryCharts,
        NodeKind::Model3d(_) => Icon::Cube,
        NodeKind::Shape(_) | NodeKind::Line(_) | NodeKind::Extension(_) => Icon::CategoryShapes,
    }
}

/// The tile of `node` in `rect`: a shape shows its fill, a line a stroke of
/// its colour, and the rest the icon of their kind.
pub(crate) fn tile(painter: &egui::Painter, theme: &Theme, rect: Rect, node: &Node) {
    painter.rect_filled(rect, theme.radius_sm(), theme.c("secondary.bg"));
    let inner = rect.shrink(7.0);
    match &node.kind {
        NodeKind::Shape(shape) if shape.fill.is_some() => {
            let fill = shape.fill.map(canvas::screen_color).unwrap_or_default();
            painter.rect(
                inner,
                2.0,
                fill,
                Stroke::new(1.0, theme.c("border")),
                egui::StrokeKind::Inside,
            );
        }
        NodeKind::Line(line) => {
            let color = canvas::screen_color(line.color);
            painter.line_segment(
                [inner.left_center(), inner.right_center()],
                Stroke::new(2.0, color),
            );
        }
        kind => icons::paint(
            painter,
            inner,
            kind_icon(kind),
            Weight::Regular,
            theme.c("text"),
        ),
    }
}

/// The copy of the dragged row under the pointer.
pub(crate) fn ghost(ctx: &egui::Context, theme: &Theme, at: Pos2, width: f32, label: &str) {
    let layer = egui::LayerId::new(egui::Order::Tooltip, egui::Id::new("layers.ghost"));
    let painter = ctx.layer_painter(layer);
    // Beside the pointer, so the insertion line stays in sight.
    let rect = Rect::from_min_size(at + Vec2::new(14.0, 10.0), Vec2::new(width, 34.0));
    painter.rect(
        rect,
        theme.radius_sm(),
        theme.c("surface").gamma_multiply(0.9),
        Stroke::new(1.5, theme.c("accent")),
        egui::StrokeKind::Inside,
    );
    grip(
        &painter,
        Pos2::new(rect.left() + 14.0, rect.center().y),
        theme.c("text"),
    );
    painter.text(
        Pos2::new(rect.left() + 32.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        label,
        theme.font("body-sm"),
        theme.c("text"),
    );
}
