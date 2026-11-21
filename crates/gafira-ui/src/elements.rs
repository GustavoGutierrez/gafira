//! The «Elementos» tab (spec 064, D-102): a grid of categories, each one a
//! stack of light cards with a stroke icon (03 §13, P1), and the view of the
//! category it opens. Also the state of the rail: which tab is active,
//! whether the side panel shows, and which category of Elementos is open.

use egui::emath::Rot2;
use egui::epaint::tessellator::path::rounded_rectangle;
use egui::{Align2, Pos2, Rect, Response, Sense, Shape, Stroke, Ui, Vec2, WidgetInfo, WidgetType};

use gafira_doc::{Light, ShapeEffect};

use crate::icons::{self, Icon, Weight};
use crate::style::Theme;
use crate::text::Texts;

/// The gap between the columns and rows of the grid, in logical px.
const GAP: f32 = 8.0;
/// The largest side of a card, in logical px.
const CARD_MAX: f32 = 64.0;
/// The height of the name under a card.
const LABEL: f32 = 18.0;

/// The categories of Elementos, in the order of 03 §2.3 (D-102, D-106).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Category {
    Shapes,
    /// «Foto collage» (D-106): photos placed in a collage layout.
    Collage,
    Frames,
    Icons,
    QrCodes,
    Tables,
    Charts,
    Illustrations,
    /// «Luces y efectos» (spec 065, D-103).
    Lights,
}

impl Category {
    pub(crate) const ALL: [Self; 9] = [
        Self::Shapes,
        Self::Collage,
        Self::Frames,
        Self::Icons,
        Self::QrCodes,
        Self::Tables,
        Self::Charts,
        Self::Illustrations,
        Self::Lights,
    ];

    /// The catalogue key of its name.
    pub(crate) fn key(self) -> &'static str {
        match self {
            Self::Shapes => "elements.shapes",
            Self::Collage => "elements.collage",
            Self::Frames => "elements.frames",
            Self::Icons => "elements.icons",
            Self::QrCodes => "elements.qr_codes",
            Self::Tables => "elements.tables",
            Self::Charts => "elements.charts",
            Self::Illustrations => "elements.illustrations",
            Self::Lights => "elements.lights",
        }
    }

    pub(crate) fn icon(self) -> Icon {
        match self {
            Self::Shapes => Icon::CategoryShapes,
            Self::Collage => Icon::CategoryCollage,
            Self::Frames => Icon::CategoryFrames,
            Self::Icons => Icon::CategoryIcons,
            Self::QrCodes => Icon::CategoryQrCodes,
            Self::Tables => Icon::CategoryTables,
            Self::Charts => Icon::CategoryCharts,
            Self::Illustrations => Icon::CategoryIllustrations,
            Self::Lights => Icon::CategoryLights,
        }
    }
}

/// The elements of «Luces y efectos» (spec 065, D-103, D-104).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LightItem {
    Sparkle,
    LensFlare,
    Streak,
    SunRays,
    Bokeh,
}

impl LightItem {
    pub(crate) const ALL: [Self; 5] = [
        Self::Sparkle,
        Self::LensFlare,
        Self::Streak,
        Self::SunRays,
        Self::Bokeh,
    ];

    /// The kind of `effect`.
    pub(crate) fn of(effect: &ShapeEffect) -> Self {
        match effect {
            ShapeEffect::Sparkle(_) => Self::Sparkle,
            ShapeEffect::LensFlare(_) => Self::LensFlare,
            ShapeEffect::Streak(_) => Self::Streak,
            ShapeEffect::SunRays(_) => Self::SunRays,
            ShapeEffect::Bokeh(_) => Self::Bokeh,
        }
    }

    pub(crate) fn key(self) -> &'static str {
        match self {
            Self::Sparkle => "elements.sparkle",
            Self::LensFlare => "elements.lens_flare",
            Self::Streak => "elements.streak",
            Self::SunRays => "elements.sun_rays",
            Self::Bokeh => "elements.bokeh",
        }
    }

    pub(crate) fn icon(self) -> Icon {
        match self {
            Self::Sparkle => Icon::CategoryLights,
            Self::LensFlare => Icon::LensFlare,
            Self::Streak => Icon::Streak,
            Self::SunRays => Icon::SunRays,
            Self::Bokeh => Icon::Bokeh,
        }
    }

    /// The effect of this kind with `light`.
    pub(crate) fn with(self, light: Light) -> ShapeEffect {
        match self {
            Self::Sparkle => ShapeEffect::Sparkle(light),
            Self::LensFlare => ShapeEffect::LensFlare(light),
            Self::Streak => ShapeEffect::Streak(light),
            Self::SunRays => ShapeEffect::SunRays(light),
            Self::Bokeh => ShapeEffect::Bokeh(light),
        }
    }

    /// The effect a new element of this kind carries.
    pub(crate) fn effect(self) -> ShapeEffect {
        self.with(match self {
            Self::Sparkle => Light::SPARKLE,
            Self::LensFlare => Light::LENS_FLARE,
            Self::Streak => Light::STREAK,
            Self::SunRays => Light::SUN_RAYS,
            Self::Bokeh => Light::BOKEH,
        })
    }

    /// The side of the new element as a share of the shorter side of the
    /// page: the light of the page-wide ones is small, bokeh needs room.
    pub(crate) fn share(self) -> f64 {
        match self {
            Self::LensFlare | Self::Streak | Self::SunRays => 1.0 / 8.0,
            Self::Sparkle => 1.0 / 4.0,
            Self::Bokeh => 1.0 / 2.0,
        }
    }
}

/// A move inside Elementos, asked by the side panel (S64-04, S64-05).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Move {
    Enter(Category),
    Back,
}

/// The rail and the side panel (S64-04..S64-06).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Rail {
    /// The index of the active tab.
    pub(crate) tab: usize,
    /// Whether the side panel shows.
    pub(crate) open: bool,
    /// The category of Elementos on display; `None` is the grid.
    pub(crate) category: Option<Category>,
}

impl Default for Rail {
    fn default() -> Self {
        Self {
            tab: 0,
            open: true,
            category: None,
        }
    }
}

impl Rail {
    /// A click on the tab at `index`: the active one hides or shows the
    /// panel in the same view; another one shows the panel on it.
    pub(crate) fn click_tab(&mut self, index: usize) {
        if index == self.tab {
            self.open = !self.open;
        } else {
            self.tab = index;
            self.open = true;
        }
    }

    /// A click on a card of the grid.
    pub(crate) fn enter(&mut self, category: Category) {
        self.category = Some(category);
    }

    pub(crate) fn apply(&mut self, step: Move) {
        match step {
            Move::Enter(category) => self.enter(category),
            Move::Back => self.back(),
        }
    }

    /// The arrow «Volver a Elementos».
    pub(crate) fn back(&mut self) {
        self.category = None;
    }
}

/// Where the three cards of a stack sit when `t` (0 at rest, 1 under the
/// pointer) of the way open (S64-03): offset from the stack centre as a
/// share of the card side, and turn in degrees, back card first. The back
/// cards fan out to the right; the front one lifts and tilts left.
pub(crate) fn fan(t: f32) -> [(Vec2, f32); 3] {
    let t = t.clamp(0.0, 1.0);
    let mix = |a: f32, b: f32| a + (b - a) * t;
    [
        (
            Vec2::new(mix(0.10, 0.22), mix(-0.07, -0.12)),
            mix(8.0, 18.0),
        ),
        (
            Vec2::new(mix(0.05, 0.11), mix(-0.035, -0.08)),
            mix(4.0, 9.0),
        ),
        (Vec2::new(mix(0.0, -0.06), mix(0.0, -0.08)), mix(0.0, -8.0)),
    ]
}

/// The grid of categories (S64-02, S64-03); the one clicked, if any.
pub(crate) fn grid(ui: &mut Ui, theme: &Theme, texts: &Texts) -> Option<Category> {
    let items = Category::ALL.map(|c| (c, c.icon(), plain(texts, c.key())));
    cards(ui, theme, &items, Sense::click())
        .filter(|(_, response)| response.clicked())
        .map(|(category, _)| category)
}

/// The elements of «Luces y efectos» (spec 065) and the response of the
/// one clicked, dragged or dropped, if any: like every element of the
/// panel, a click adds it and a drag carries it to the canvas (D-105).
pub(crate) fn lights(ui: &mut Ui, theme: &Theme, texts: &Texts) -> Option<(LightItem, Response)> {
    let items = LightItem::ALL.map(|i| (i, i.icon(), plain(texts, i.key())));
    cards(ui, theme, &items, Sense::click_and_drag())
}

/// Cards in rows of three, each sensing `sense`; the item and response of
/// the one clicked, dragged or let go of, if any.
fn cards<T: Copy>(
    ui: &mut Ui,
    theme: &Theme,
    items: &[(T, Icon, String)],
    sense: Sense,
) -> Option<(T, Response)> {
    let column = (ui.available_width() - 2.0 * GAP) / 3.0;
    let mut hit = None;
    for row in items.chunks(3) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = Vec2::splat(GAP);
            for (item, icon, name) in row {
                let response = card(ui, theme, column, *icon, name, sense);
                if response.clicked() || response.dragged() || response.drag_stopped() {
                    hit = Some((*item, response));
                }
            }
        });
        ui.add_space(GAP);
    }
    hit
}

/// The title of a category with the arrow «Volver a Elementos» on its left
/// (S64-04); whether the arrow was clicked.
pub(crate) fn header(ui: &mut Ui, theme: &Theme, texts: &Texts, category: Category) -> bool {
    let back = ui
        .horizontal(|ui| {
            let back = crate::widgets::IconButton::new(Icon::Back, plain(texts, "elements.back"))
                .small()
                .show(ui, theme);
            ui.label(
                egui::RichText::new(plain(texts, category.key()))
                    .font(theme.font("ui-label"))
                    .color(theme.c("text")),
            );
            back.clicked()
        })
        .inner;
    ui.add_space(12.0);
    back
}

/// The view of a category that has no library yet (S64-04).
pub(crate) fn soon(ui: &mut Ui, theme: &Theme, texts: &Texts) {
    ui.label(
        egui::RichText::new(plain(texts, "elements.soon"))
            .font(theme.font("body"))
            .color(theme.c("text.secondary")),
    );
}

/// One card of the grid: a stack of three light cards with `icon` on the
/// front one and `name` under it, opening like a fan under the pointer.
fn card(ui: &mut Ui, theme: &Theme, width: f32, icon: Icon, name: &str, sense: Sense) -> Response {
    let side = (width - 2.0 * GAP).min(CARD_MAX);
    let (rect, response) =
        ui.allocate_exact_size(Vec2::new(width, side + 2.0 * GAP + LABEL), sense);
    let duration = theme.tokens.motion.base.as_secs_f32();
    let t = ui
        .ctx()
        .animate_bool_with_time(response.id, response.hovered(), duration);
    let centre = Pos2::new(rect.center().x, rect.min.y + GAP + side / 2.0);
    let painter = ui.painter();
    let radius = theme.radius_md();
    let border = Stroke::new(1.0, theme.c("border"));
    let fills = [
        theme.c("surface.sunken"),
        theme.c("surface.sunken"),
        theme.c("surface"),
    ];
    let mut front = (centre, 0.0);
    for (index, (offset, degrees)) in fan(t).into_iter().enumerate() {
        let at = centre + offset * side;
        let angle = degrees.to_radians();
        let mut points = Vec::new();
        rounded_rectangle(
            &mut points,
            Rect::from_center_size(at, Vec2::splat(side)),
            radius.into(),
        );
        let turn = Rot2::from_angle(angle);
        for point in &mut points {
            *point = at + turn * (*point - at);
        }
        painter.add(Shape::convex_polygon(points, fills[index], border));
        front = (at, angle);
    }
    let color = theme.c("text");
    let shown = elide(ui, theme, name, width);
    icons::paint_turned(
        painter,
        front.0,
        side * 0.5,
        front.1,
        icon,
        Weight::Regular,
        color,
    );
    painter.text(
        Pos2::new(rect.center().x, rect.max.y),
        Align2::CENTER_BOTTOM,
        &shown,
        theme.font("caption"),
        color,
    );
    if response.has_focus() {
        painter.rect_stroke(
            rect,
            theme.radius_sm(),
            Stroke::new(2.0, theme.c("focus.ring")),
            egui::StrokeKind::Inside,
        );
    }
    response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, name));
    // The full name only when the caption had to be cut.
    if shown == name {
        response
    } else {
        response.on_hover_text(name)
    }
}

/// `text` cut with an ellipsis to fit `width` in the caption font.
fn elide(ui: &Ui, theme: &Theme, text: &str, width: f32) -> String {
    let font = theme.font("caption");
    let fits = |s: &str| {
        ui.fonts_mut(|f| f.layout_no_wrap(s.to_owned(), font.clone(), egui::Color32::WHITE))
            .size()
            .x
            <= width
    };
    if fits(text) {
        return text.to_owned();
    }
    let chars: Vec<char> = text.chars().collect();
    (1..chars.len())
        .rev()
        .map(|n| format!("{}…", chars[..n].iter().collect::<String>()))
        .find(|s| fits(s))
        .unwrap_or_else(|| "…".to_owned())
}

fn plain(texts: &Texts, key: &'static str) -> String {
    texts.t(&gafira_doc::msg::Message {
        key: gafira_doc::msg::MessageKey(key),
        args: Vec::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn s64_03_the_stack_rests_square_and_fans_out_under_the_pointer() {
        let rest = fan(0.0);
        assert_eq!(rest[2], (Vec2::ZERO, 0.0), "the front card rests square");
        let open = fan(1.0);
        assert!(
            open[0].1 > rest[0].1 && open[1].1 > rest[1].1,
            "the back cards turn further"
        );
        assert!(
            open[2].1 < 0.0 && open[2].0.y < 0.0,
            "the front card tilts and lifts"
        );
        assert_eq!(fan(2.0), open);
    }

    #[test]
    fn s64_06_clicking_the_active_tab_hides_and_shows_the_panel() {
        let mut rail = Rail::default();
        rail.click_tab(0);
        assert_eq!((rail.tab, rail.open), (0, false));
        rail.click_tab(0);
        assert_eq!((rail.tab, rail.open), (0, true));
    }

    #[test]
    fn s64_06_clicking_another_tab_shows_the_panel_on_it() {
        let mut rail = Rail::default();
        rail.click_tab(2);
        assert_eq!((rail.tab, rail.open), (2, true));
        rail.click_tab(2);
        rail.click_tab(4);
        assert_eq!((rail.tab, rail.open), (4, true), "hidden, then another tab");
    }

    #[test]
    fn s64_04_s64_05_a_category_is_entered_and_left() {
        let mut rail = Rail::default();
        rail.enter(Category::Shapes);
        assert_eq!(rail.category, Some(Category::Shapes));
        rail.back();
        assert_eq!(rail.category, None);
    }

    #[test]
    fn s64_06_hiding_keeps_the_category_and_reopening_returns_to_it() {
        let mut rail = Rail {
            tab: 1,
            ..Rail::default()
        };
        rail.enter(Category::Shapes);
        rail.click_tab(1);
        assert!(!rail.open);
        rail.click_tab(1);
        assert!(rail.open);
        assert_eq!(rail.category, Some(Category::Shapes));
    }
}
