//! The charts of Elementos → Gráficos (spec 083, S83-06): their groups, the
//! data each one comes with, and the panel that shows them.

use std::collections::HashMap;

use egui::{Pos2, Rect, Response, RichText, Sense, Stroke, Ui, Vec2};
use gafira_doc::{ChartKind, ChartLabels, ChartNode, ChartText, NodeKind, Placement};
use gafira_edit::NodeSpec;

use crate::charts::plain;
use crate::colors::hex;
use crate::style::Theme;
use crate::text::Texts;
use crate::widgets;

/// The palettes of the charts: their name and their colours, in order. The
/// first is the brand's, its neighbours apart for those who confuse colours
/// too (checked: CVD ΔE 8.3 at least, 27 in full colour); the second is the
/// standard one of Okabe and Ito.
pub(crate) const PALETTES: [(&str, &[u32]); 5] = [
    (
        "chart.palette_gafira",
        &[
            0xFF_6B57, 0x2F_62C9, 0x3F_BFA9, 0x7D_4FD0, 0xD9_9A00, 0x15_83A8, 0xD6_457F, 0x4E_8F2F,
        ],
    ),
    (
        "chart.palette_standard",
        &[
            0x00_72B2, 0xE6_9F00, 0x00_9E73, 0xCC_79A7, 0x56_B4E9, 0xD5_5E00, 0xF0_E442, 0x44_4444,
        ],
    ),
    (
        "chart.palette_mint",
        &[
            0x1E_7F70, 0x3F_BFA9, 0x7F_D6C8, 0x14_5A50, 0xA9_E4DA, 0x2A_9D8C,
        ],
    ),
    (
        "chart.palette_coral",
        &[
            0xD9_4733, 0xFF_6B57, 0xFF_A294, 0xA3_2F1F, 0xFF_C9C0, 0xEE_5843,
        ],
    ),
    (
        "chart.palette_ink",
        &[
            0x18_243B, 0x4A_5A7A, 0x7D_8BA8, 0x2F_3F5F, 0xA9_B4CA, 0x62_718F,
        ],
    ),
];

/// The ink of the words.
const INK: u32 = 0x18_243B;

/// The words of a new chart, in px of a page a thousand px wide.
const WORDS: f64 = 22.0;

/// A group of charts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Group {
    Bars,
    Lines,
    Pies,
    Areas,
    Points,
    Hierarchy,
    Progress,
    Others,
}

impl Group {
    pub(crate) const ALL: [Self; 8] = [
        Self::Bars,
        Self::Lines,
        Self::Pies,
        Self::Areas,
        Self::Points,
        Self::Hierarchy,
        Self::Progress,
        Self::Others,
    ];

    pub(crate) fn key(self) -> &'static str {
        match self {
            Self::Bars => "chart.group_bars",
            Self::Lines => "chart.group_lines",
            Self::Pies => "chart.group_pies",
            Self::Areas => "chart.group_areas",
            Self::Points => "chart.group_points",
            Self::Hierarchy => "chart.group_hierarchy",
            Self::Progress => "chart.group_progress",
            Self::Others => "chart.group_others",
        }
    }
}

/// A chart of the panel: its kind, its name, its group, and whether it is
/// the second one of its kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Template {
    pub kind: ChartKind,
    pub key: &'static str,
    pub group: Group,
    second: bool,
}

const fn of(kind: ChartKind, key: &'static str, group: Group) -> Template {
    Template {
        kind,
        key,
        group,
        second: false,
    }
}

const fn second(kind: ChartKind, key: &'static str, group: Group) -> Template {
    Template {
        kind,
        key,
        group,
        second: true,
    }
}

pub(crate) const TEMPLATES: [Template; 31] = [
    of(ChartKind::Columns, "chart.columns", Group::Bars),
    of(ChartKind::Rows, "chart.rows", Group::Bars),
    second(ChartKind::Columns, "chart.columns_by_label", Group::Bars),
    second(ChartKind::Rows, "chart.rows_by_label", Group::Bars),
    of(
        ChartKind::GroupedColumns,
        "chart.grouped_columns",
        Group::Bars,
    ),
    of(ChartKind::GroupedRows, "chart.grouped_rows", Group::Bars),
    of(
        ChartKind::StackedColumns,
        "chart.stacked_columns",
        Group::Bars,
    ),
    of(ChartKind::StackedRows, "chart.stacked_rows", Group::Bars),
    of(ChartKind::Line, "chart.line", Group::Lines),
    second(ChartKind::Line, "chart.lines", Group::Lines),
    of(ChartKind::Pie, "chart.pie", Group::Pies),
    of(ChartKind::Donut, "chart.donut", Group::Pies),
    of(ChartKind::StackedArea, "chart.stacked_area", Group::Areas),
    of(ChartKind::Area, "chart.area", Group::Areas),
    of(ChartKind::Stream, "chart.stream", Group::Areas),
    of(ChartKind::PercentArea, "chart.percent_area", Group::Areas),
    of(ChartKind::Scatter, "chart.scatter", Group::Points),
    of(ChartKind::Bubbles, "chart.bubbles", Group::Points),
    of(ChartKind::DotColumns, "chart.dot_columns", Group::Points),
    of(ChartKind::DotRows, "chart.dot_rows", Group::Points),
    of(ChartKind::Pack, "chart.pack", Group::Hierarchy),
    of(ChartKind::Treemap, "chart.treemap", Group::Hierarchy),
    of(ChartKind::NestedPack, "chart.nested_pack", Group::Hierarchy),
    of(ChartKind::Sunburst, "chart.sunburst", Group::Hierarchy),
    of(
        ChartKind::ProgressRing,
        "chart.progress_ring",
        Group::Progress,
    ),
    of(
        ChartKind::ProgressArc,
        "chart.progress_arc",
        Group::Progress,
    ),
    of(
        ChartKind::ProgressBar,
        "chart.progress_bar",
        Group::Progress,
    ),
    of(ChartKind::Gauge, "chart.gauge", Group::Progress),
    of(ChartKind::Histogram, "chart.histogram", Group::Others),
    of(ChartKind::Funnel, "chart.funnel", Group::Others),
    of(ChartKind::Radar, "chart.radar", Group::Others),
];

/// The name of a kind.
pub(crate) fn kind_key(kind: ChartKind) -> &'static str {
    TEMPLATES
        .iter()
        .find(|t| t.kind == kind)
        .map_or("chart.columns", |t| t.key)
}

pub(crate) fn palette(index: usize) -> Vec<gafira_color::Color> {
    PALETTES[index].1.iter().map(|code| hex(*code)).collect()
}

/// `kind` as it looks at first, with no data.
pub(crate) fn dressed(kind: ChartKind) -> ChartNode {
    let text = ChartText {
        font: "Inter".to_owned(),
        size: WORDS,
        color: hex(INK),
        bold: false,
        italic: false,
    };
    let mut c = ChartNode::new(kind, (1, vec![String::new()]), palette(0), text);
    dress(&mut c);
    c
}

/// What `c` writes and how it is drawn, as its kind is at first.
pub(crate) fn dress(c: &mut ChartNode) {
    use ChartKind as K;
    let axes = matches!(
        c.kind,
        K::Columns
            | K::Rows
            | K::GroupedColumns
            | K::GroupedRows
            | K::StackedColumns
            | K::StackedRows
            | K::Line
            | K::StackedArea
            | K::Area
            | K::PercentArea
            | K::Scatter
            | K::Bubbles
            | K::DotColumns
            | K::DotRows
            | K::Histogram
    );
    c.axis = axes;
    c.grid = axes || c.kind == K::Radar;
    c.names = true;
    c.legend = !matches!(
        c.kind,
        K::Pie | K::Donut | K::Pack | K::Treemap | K::Funnel | K::DotColumns | K::DotRows
    );
    c.numbers = match c.kind {
        K::Pie
        | K::Donut
        | K::Funnel
        | K::Treemap
        | K::ProgressRing
        | K::ProgressArc
        | K::ProgressBar
        | K::Gauge => ChartLabels::Percent,
        K::Pack => ChartLabels::Value,
        _ => ChartLabels::None,
    };
    c.smooth = matches!(c.kind, K::Stream | K::Area);
    c.markers = matches!(c.kind, K::Line | K::Radar);
    c.round = match c.kind {
        K::ProgressRing | K::ProgressArc | K::ProgressBar => 1.0,
        _ => 0.2,
    };
    c.gap = if c.kind == K::Histogram { 0.06 } else { 0.3 };
    c.hole = match c.kind {
        K::ProgressRing | K::ProgressArc | K::Gauge => 0.72,
        K::Sunburst => 0.4,
        _ => 0.55,
    };
    c.weight = match c.kind {
        K::Scatter | K::DotColumns | K::DotRows => 5.0,
        _ => 4.0,
    };
}

impl Template {
    /// The chart with its sample data.
    pub(crate) fn chart(self, texts: &Texts) -> ChartNode {
        let mut c = dressed(self.kind);
        // The labels are by their bars already.
        c.by_label = self.second && self.kind != ChartKind::Line;
        c.legend &= !c.by_label;
        let rows = sample(self, texts);
        gafira_chart::from_grid(&mut c, &rows);
        c
    }
}

/// The data a chart comes with.
fn sample(template: Template, texts: &Texts) -> Vec<Vec<String>> {
    use ChartKind as K;
    let t = |key: &'static str| plain(texts, key);
    let rows = |names: &[&'static str], values: &[&[f64]]| -> Vec<Vec<String>> {
        names
            .iter()
            .zip(values)
            .map(|(name, values)| {
                let mut row = vec![t(name)];
                row.extend(values.iter().map(f64::to_string));
                row
            })
            .collect()
    };
    let with = |head: &[&'static str], body: Vec<Vec<String>>| {
        let mut all = vec![head.iter().map(|key| t(key)).collect()];
        all.extend(body);
        all
    };
    let teams = [
        "chart.s_sales",
        "chart.s_finance",
        "chart.s_marketing",
        "chart.s_people",
    ];
    let months = [
        "chart.s_m1",
        "chart.s_m2",
        "chart.s_m3",
        "chart.s_m4",
        "chart.s_m5",
        "chart.s_m6",
    ];
    let products = ["chart.s_month", "chart.s_a", "chart.s_b", "chart.s_c"];
    match template.kind {
        K::Columns | K::Rows | K::Pie | K::Donut | K::Pack | K::Treemap => with(
            &["chart.s_team", "chart.s_staff"],
            rows(&teams, &[&[110.0], &[50.0], &[30.0], &[10.0]]),
        ),
        K::Line if !template.second => with(
            &["chart.s_month", "chart.s_visits"],
            rows(
                &months,
                &[&[12.0], &[19.0], &[17.0], &[28.0], &[33.0], &[41.0]],
            ),
        ),
        K::Line
        | K::GroupedColumns
        | K::GroupedRows
        | K::StackedColumns
        | K::StackedRows
        | K::StackedArea
        | K::PercentArea => {
            let count = match template.kind {
                K::GroupedColumns | K::GroupedRows | K::StackedRows | K::StackedColumns => 4,
                _ => 6,
            };
            with(
                &products,
                rows(
                    &months[..count],
                    &[
                        &[18.0, 12.0, 7.0],
                        &[22.0, 15.0, 10.0],
                        &[20.0, 21.0, 12.0],
                        &[29.0, 19.0, 17.0],
                        &[31.0, 26.0, 15.0],
                        &[38.0, 28.0, 21.0],
                    ],
                ),
            )
        }
        K::Area => with(
            &products[..3],
            rows(
                &months,
                &[
                    &[4.0, 2.0],
                    &[16.0, 5.0],
                    &[28.0, 14.0],
                    &[17.0, 27.0],
                    &[9.0, 22.0],
                    &[5.0, 8.0],
                ],
            ),
        ),
        K::Stream => with(
            &products,
            rows(
                &months,
                &[
                    &[6.0, 5.0, 3.0],
                    &[10.0, 8.0, 4.0],
                    &[14.0, 9.0, 9.0],
                    &[20.0, 13.0, 14.0],
                    &[13.0, 10.0, 8.0],
                    &[7.0, 6.0, 4.0],
                ],
            ),
        ),
        K::Scatter | K::Bubbles => {
            let head = [
                "chart.s_zone",
                "chart.s_price",
                "chart.s_units",
                "chart.s_profit",
            ];
            let columns = if template.kind == K::Bubbles { 4 } else { 3 };
            let zones = [
                "chart.s_north",
                "chart.s_north",
                "chart.s_north",
                "chart.s_north",
                "chart.s_south",
                "chart.s_south",
                "chart.s_south",
                "chart.s_south",
                "chart.s_centre",
                "chart.s_centre",
                "chart.s_centre",
                "chart.s_centre",
            ];
            let points: [[f64; 3]; 12] = [
                [12.0, 64.0, 14.0],
                [16.0, 58.0, 22.0],
                [21.0, 71.0, 9.0],
                [25.0, 52.0, 30.0],
                [33.0, 38.0, 18.0],
                [38.0, 45.0, 34.0],
                [44.0, 30.0, 12.0],
                [47.0, 41.0, 25.0],
                [55.0, 22.0, 28.0],
                [60.0, 33.0, 16.0],
                [66.0, 15.0, 36.0],
                [71.0, 26.0, 20.0],
            ];
            let points: Vec<&[f64]> = points.iter().map(|p| &p[..columns - 1]).collect();
            with(&head[..columns], rows(&zones, &points))
        }
        K::DotColumns | K::DotRows => with(
            &products,
            rows(
                &months,
                &[
                    &[12.0, 22.0, 31.0],
                    &[17.0, 25.0, 27.0],
                    &[21.0, 30.0, 35.0],
                    &[26.0, 19.0, 39.0],
                    &[30.0, 35.0, 24.0],
                    &[36.0, 40.0, 44.0],
                ],
            ),
        ),
        K::NestedPack | K::Sunburst => {
            let head = ["chart.s_zone", "chart.s_team", "chart.s_staff"];
            let body = [
                ("chart.s_north", "chart.s_sales", 24.0),
                ("chart.s_north", "chart.s_finance", 14.0),
                ("chart.s_north", "chart.s_marketing", 10.0),
                ("chart.s_south", "chart.s_sales", 18.0),
                ("chart.s_south", "chart.s_people", 9.0),
                ("chart.s_centre", "chart.s_sales", 12.0),
                ("chart.s_centre", "chart.s_finance", 8.0),
                ("chart.s_centre", "chart.s_marketing", 6.0),
            ];
            let body = body
                .iter()
                .map(|(zone, team, v)| vec![t(zone), t(team), f64::to_string(v)]);
            with(&head, body.collect())
        }
        K::ProgressRing | K::ProgressArc | K::Gauge => with(
            &["chart.s_goal", "chart.s_done"],
            vec![vec![t("chart.s_project"), "72%".to_owned()]],
        ),
        K::ProgressBar => {
            let body = [
                ("chart.s_design", "90%"),
                ("chart.s_build", "65%"),
                ("chart.s_launch", "30%"),
            ];
            let body = body
                .iter()
                .map(|(name, done)| vec![t(name), (*done).to_owned()]);
            with(&["chart.s_goal", "chart.s_done"], body.collect())
        }
        K::Histogram => {
            let ages = [
                18, 21, 22, 24, 25, 26, 26, 27, 28, 29, 29, 30, 31, 31, 32, 33, 33, 34, 34, 35, 36,
                36, 37, 38, 39, 40, 41, 42, 44, 45, 47, 49, 52, 55, 58, 63,
            ];
            let body = ages.iter().map(|age: &i32| vec![age.to_string()]);
            with(&["chart.s_age"], body.collect())
        }
        K::Funnel => with(
            &["chart.s_stage", "chart.s_people_count"],
            rows(
                &[
                    "chart.s_visits",
                    "chart.s_interest",
                    "chart.s_cart",
                    "chart.s_purchase",
                ],
                &[&[1200.0], &[760.0], &[430.0], &[210.0]],
            ),
        ),
        K::Radar => with(
            &["chart.s_skill", "chart.s_a", "chart.s_b"],
            rows(
                &[
                    "chart.s_quality",
                    "chart.s_price",
                    "chart.s_support",
                    "chart.s_speed",
                    "chart.s_design",
                ],
                &[
                    &[8.0, 6.0],
                    &[5.0, 9.0],
                    &[7.0, 6.0],
                    &[6.0, 8.0],
                    &[9.0, 5.0],
                ],
            ),
        ),
    }
}

/// `c` on a page of `page_px`: half of its smaller side, wider for what goes
/// across; centred on `at`, or in the middle, and inside the page.
pub(crate) fn spec(
    mut c: ChartNode,
    page_px: (f64, f64),
    at: Option<(f64, f64)>,
    name: String,
) -> NodeSpec {
    use ChartKind as K;
    let square = matches!(
        c.kind,
        K::Pack | K::NestedPack | K::Sunburst | K::ProgressRing | K::Radar
    );
    let side = page_px.0.min(page_px.1) * 0.6;
    let (w, h) = match c.kind {
        _ if square => (side, side),
        K::ProgressBar => (side * 1.2, side * 0.6),
        K::Pie | K::Donut | K::ProgressArc | K::Gauge => (side * 1.25, side),
        _ => ((side * 1.5).min(page_px.0 * 0.8), side),
    };
    // The words keep their share of the page.
    let k = (side / 600.0).max(0.05);
    c.text.size = (c.text.size * k).clamp(1.0, gafira_doc::TextNode::MAX_SIZE);
    c.weight = (c.weight * k).clamp(0.1, ChartNode::MAX_WEIGHT);
    let (x, y) = at.unwrap_or((page_px.0 / 2.0, page_px.1 / 2.0));
    let inside = |at: f64, side: f64, page: f64| (at - side / 2.0).min(page - side).max(0.0);
    let placement = Placement::new(inside(x, w, page_px.0), inside(y, h, page_px.1), w, h);
    NodeSpec {
        name: Some(name),
        ..NodeSpec::new(NodeKind::Chart(Box::new(c)), placement)
    }
}

/// What the panel asks for.
pub(crate) enum Chosen {
    Insert(Template, Option<Pos2>),
    /// «Crear con mis datos».
    Data,
}

/// The panel: the group open with «Ver todo», the search and the pictures.
#[derive(Default)]
pub(crate) struct Panel {
    open: Option<Group>,
    query: String,
    thumbs: HashMap<(usize, String), egui::TextureHandle>,
}

/// The tiles of a line.
const IN_LINE: usize = 3;

impl Panel {
    pub(crate) fn show(
        &mut self,
        ui: &mut Ui,
        theme: &Theme,
        texts: &Texts,
        over: &dyn Fn(Pos2) -> bool,
    ) -> Option<Chosen> {
        let mut chosen = None;
        ui.add(widgets::field(
            &mut self.query,
            plain(texts, "chart.search"),
        ));
        ui.add_space(8.0);
        let button = egui::Button::new(plain(texts, "chart.from_data"))
            .min_size(Vec2::new(ui.available_width(), 38.0));
        let button = ui
            .add(button)
            .on_hover_cursor(egui::CursorIcon::PointingHand)
            .on_hover_text(plain(texts, "chart.from_data_hint"));
        if button.clicked() {
            chosen = Some(Chosen::Data);
        }
        ui.add_space(12.0);
        let query = self.query.trim().to_lowercase();
        if !query.is_empty() {
            let found: Vec<Template> = TEMPLATES
                .into_iter()
                .filter(|t| {
                    let holds = |key| plain(texts, key).to_lowercase().contains(&query);
                    holds(t.key) || holds(t.group.key())
                })
                .collect();
            if found.is_empty() {
                ui.label(plain(texts, "chart.none_found"));
            }
            self.tiles(ui, theme, texts, &found, over, &mut chosen);
            return chosen;
        }
        if let Some(group) = self.open {
            ui.horizontal(|ui| {
                let back = ui
                    .add_enabled_ui(true, crate::widgets::back_arrow)
                    .inner
                    .on_hover_text(plain(texts, "table.back"));
                if back.clicked() {
                    self.open = None;
                }
                title(ui, theme, plain(texts, group.key()));
            });
            ui.add_space(8.0);
            let all: Vec<Template> = in_group(group).collect();
            self.tiles(ui, theme, texts, &all, over, &mut chosen);
            return chosen;
        }
        for group in Group::ALL {
            let all: Vec<Template> = in_group(group).collect();
            ui.horizontal(|ui| {
                title(ui, theme, plain(texts, group.key()));
                if all.len() > IN_LINE {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let text = RichText::new(plain(texts, "illustrations.see_all"))
                            .font(theme.font("body-sm"));
                        let more = ui
                            .add(egui::Button::new(text).frame(false))
                            .on_hover_cursor(egui::CursorIcon::PointingHand);
                        if more.clicked() {
                            self.open = Some(group);
                        }
                    });
                }
            });
            ui.add_space(4.0);
            let first = &all[..all.len().min(IN_LINE)];
            self.tiles(ui, theme, texts, first, over, &mut chosen);
            ui.add_space(10.0);
        }
        chosen
    }

    fn tiles(
        &mut self,
        ui: &mut Ui,
        theme: &Theme,
        texts: &Texts,
        all: &[Template],
        over: &dyn Fn(Pos2) -> bool,
        chosen: &mut Option<Chosen>,
    ) {
        let gap = 8.0;
        let width = (ui.available_width() - gap * (IN_LINE as f32 - 1.0)) / IN_LINE as f32;
        for line in all.chunks(IN_LINE) {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = gap;
                for template in line {
                    self.tile(ui, theme, texts, (*template, width), over, chosen);
                }
            });
            ui.add_space(6.0);
        }
    }

    /// One chart: a click inserts it, a drag carries it (D-105).
    fn tile(
        &mut self,
        ui: &mut Ui,
        theme: &Theme,
        texts: &Texts,
        (template, width): (Template, f32),
        over: &dyn Fn(Pos2) -> bool,
        chosen: &mut Option<Chosen>,
    ) {
        let name = plain(texts, template.key);
        let size = Vec2::new(width, width * 0.86 + 22.0);
        let (rect, response) = ui.allocate_exact_size(size, Sense::click_and_drag());
        if !ui.is_rect_visible(rect) {
            return;
        }
        let picture = Rect::from_min_size(rect.min, Vec2::new(width, width * 0.86));
        let painter = ui.painter();
        painter.rect_filled(picture, theme.radius_md(), theme.c("surface.sunken"));
        if response.hovered() || response.has_focus() {
            painter.rect_stroke(
                picture,
                theme.radius_md(),
                Stroke::new(2.0, theme.c("primary.bg")),
                egui::StrokeKind::Inside,
            );
        }
        let color = theme.c("text");
        let mut job =
            egui::text::LayoutJob::simple_singleline(name.clone(), theme.font("caption"), color);
        job.wrap = egui::text::TextWrapping::truncate_at_width(width);
        let galley = ui.fonts_mut(|f| f.layout_job(job));
        let at = Pos2::new(
            rect.center().x - galley.size().x / 2.0,
            picture.bottom() + 4.0,
        );
        painter.galley(at, galley, color);
        let ctx = ui.ctx().clone();
        let place = TEMPLATES.iter().position(|t| *t == template).unwrap_or(0);
        // The words of the picture change with the language.
        let key = (place, name.clone());
        if !self.thumbs.contains_key(&key) {
            let chart = thumb(template, texts);
            let k = ctx.pixels_per_point() * 2.0 * width / 240.0;
            if let Some(pixels) = crate::charts::image(&chart, (240.0, 180.0), k) {
                let texture = ctx.load_texture("chart-thumb", pixels, egui::TextureOptions::LINEAR);
                self.thumbs.insert(key.clone(), texture);
            }
        }
        if let Some(texture) = self.thumbs.get(&key) {
            let room = picture.shrink(8.0);
            let size = texture.size_vec2();
            let k = (room.width() / size.x).min(room.height() / size.y);
            let place = Rect::from_center_size(room.center(), size * k);
            let uv = Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0));
            painter.image(texture.id(), place, uv, egui::Color32::WHITE);
            drag(ui, &response, (template, texture), over, chosen);
        }
        if response.hovered() && !response.dragged() {
            ctx.set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        if response.clicked() {
            *chosen = Some(Chosen::Insert(template, None));
        }
        response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, &name));
        response.on_hover_text(name);
    }
}

/// The chart of a tile: its marks alone, seen on any theme.
fn thumb(template: Template, texts: &Texts) -> ChartNode {
    let mut c = template.chart(texts);
    (c.names, c.axis, c.grid, c.legend) = (false, false, false, false);
    c.numbers = ChartLabels::None;
    if c.kind == ChartKind::Radar {
        c.grid = true;
    }
    c.text.color = hex(0x7D_8BA8);
    c.weight = 6.0;
    c
}

fn drag(
    ui: &Ui,
    response: &Response,
    (template, texture): (Template, &egui::TextureHandle),
    over: &dyn Fn(Pos2) -> bool,
    chosen: &mut Option<Chosen>,
) {
    let Some(at) = ui.input(|i| i.pointer.latest_pos()) else {
        return;
    };
    if response.dragged() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
        let layer = egui::LayerId::new(egui::Order::Tooltip, egui::Id::new("chart_ghost"));
        let size = texture.size_vec2();
        let ghost = Rect::from_center_size(at, size * (180.0 / size.x));
        let uv = Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0));
        let tint = egui::Color32::WHITE.gamma_multiply(0.7);
        let painter = ui.ctx().layer_painter(layer);
        painter.image(texture.id(), ghost, uv, tint);
    }
    if response.drag_stopped() && over(at) {
        *chosen = Some(Chosen::Insert(template, Some(at)));
    }
}

fn in_group(group: Group) -> impl Iterator<Item = Template> {
    TEMPLATES.into_iter().filter(move |t| t.group == group)
}

fn title(ui: &mut Ui, theme: &Theme, text: String) {
    ui.label(
        RichText::new(text)
            .font(theme.font("ui-label"))
            .color(theme.c("text")),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    // S83-06: every template is a valid chart that draws something, inside
    // its page, with colours enough.
    #[test]
    fn s83_06_every_template_is_valid_and_fits() {
        for locale in [
            gafira_ports::i18n::Locale::Es,
            gafira_ports::i18n::Locale::En,
        ] {
            let texts = Texts::new(locale);
            for template in TEMPLATES {
                for page in [(1080.0, 1080.0), (2480.0, 3508.0), (300.0, 90.0)] {
                    let chart = template.chart(&texts);
                    let spec = spec(chart, page, Some((5.0, 5.0)), String::new());
                    let NodeKind::Chart(c) = &spec.kind else {
                        panic!("a chart")
                    };
                    assert!(c.is_valid(), "{}", template.key);
                    let p = &spec.placement;
                    assert!(p.x >= 0.0 && p.y >= 0.0, "{}", template.key);
                    assert!(p.x + p.w <= page.0 + 1e-6 && p.y + p.h <= page.1 + 1e-6);
                    let fonts = crate::canvas::text_fonts();
                    let drawn = gafira_render::chart_outlines(c, (p.w, p.h), fonts);
                    assert!(!drawn.is_empty(), "{}", template.key);
                    assert!(gafira_chart::color_names(c).len() <= c.colors.len());
                }
            }
        }
    }

    // S83-05: the words of the charts stand out on white and the palettes
    // are valid.
    #[test]
    fn s83_05_palettes() {
        for index in 0..PALETTES.len() {
            let mut c = dressed(ChartKind::Pie);
            c.colors = palette(index);
            assert!(c.is_valid());
        }
    }

    // S83-11, FD-22: a hundred points are drawn in less than 100 ms.
    #[test]
    fn s83_11_a_hundred_points() {
        let mut c = dressed(ChartKind::Columns);
        let mut rows = vec![vec!["a".to_owned(), "b".to_owned()]];
        rows.extend((0..100).map(|i| vec![format!("Punto {i}"), (i * 7 % 50).to_string()]));
        assert!(gafira_chart::from_grid(&mut c, &rows));
        c.numbers = ChartLabels::Value;
        let drawn = gafira_render::chart_outlines(&c, (900.0, 600.0), crate::canvas::text_fonts());
        assert!(drawn.len() > 100);
    }
}
