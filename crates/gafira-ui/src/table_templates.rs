//! The tables of Elementos → Tablas (spec 081, S81-07): their groups, what
//! each one holds, and the panel that shows them.

use std::collections::HashMap;

use egui::{Pos2, Rect, Response, RichText, Sense, Stroke, Ui, Vec2};
use gafira_doc::{CellStyle, NodeKind, Placement, ShapeStroke, TableNode, TextAlign};
use gafira_edit::NodeSpec;
use gafira_table::{BorderSet, Range, merge, restyle, set_borders, size};

use crate::colors::hex;
use crate::style::Theme;
use crate::tables::Outlines;
use crate::text::Texts;

fn plain(texts: &Texts, key: &'static str) -> String {
    texts.t(&gafira_doc::msg::Message {
        key: gafira_doc::msg::MessageKey(key),
        args: Vec::new(),
    })
}

/// A group of tables.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Group {
    Start,
    Present,
    Plan,
    Track,
}

impl Group {
    pub(crate) const ALL: [Self; 4] = [Self::Start, Self::Present, Self::Plan, Self::Track];

    pub(crate) fn key(self) -> &'static str {
        match self {
            Self::Start => "table.group_start",
            Self::Present => "table.group_present",
            Self::Plan => "table.group_plan",
            Self::Track => "table.group_track",
        }
    }
}

/// A table of the panel.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Template {
    Grid,
    Header,
    Zebra,
    Sided,
    Lines,
    ProsCons,
    Compare,
    Plans,
    Glossary,
    Month,
    Week,
    Hours,
    Timetable,
    Sprints,
    Log,
    Shifts,
    Tasks,
    Gantt,
    Directory,
    Budget,
}

impl Template {
    pub(crate) const ALL: [Self; 20] = [
        Self::Grid,
        Self::Header,
        Self::Zebra,
        Self::Sided,
        Self::Lines,
        Self::ProsCons,
        Self::Compare,
        Self::Plans,
        Self::Glossary,
        Self::Month,
        Self::Week,
        Self::Hours,
        Self::Timetable,
        Self::Sprints,
        Self::Log,
        Self::Shifts,
        Self::Tasks,
        Self::Gantt,
        Self::Directory,
        Self::Budget,
    ];

    pub(crate) fn group(self) -> Group {
        match self {
            Self::Grid | Self::Header | Self::Zebra | Self::Sided | Self::Lines => Group::Start,
            Self::ProsCons | Self::Compare | Self::Plans | Self::Glossary => Group::Present,
            Self::Month
            | Self::Week
            | Self::Hours
            | Self::Timetable
            | Self::Sprints
            | Self::Log
            | Self::Shifts => Group::Plan,
            Self::Tasks | Self::Gantt | Self::Directory | Self::Budget => Group::Track,
        }
    }

    pub(crate) fn key(self) -> &'static str {
        match self {
            Self::Grid => "table.tpl_grid",
            Self::Header => "table.tpl_header",
            Self::Zebra => "table.tpl_zebra",
            Self::Sided => "table.tpl_sided",
            Self::Lines => "table.tpl_lines",
            Self::ProsCons => "table.tpl_pros_cons",
            Self::Compare => "table.tpl_compare",
            Self::Plans => "table.tpl_plans",
            Self::Glossary => "table.tpl_glossary",
            Self::Month => "table.tpl_month",
            Self::Week => "table.tpl_week",
            Self::Hours => "table.tpl_hours",
            Self::Timetable => "table.tpl_timetable",
            Self::Sprints => "table.tpl_sprints",
            Self::Log => "table.tpl_log",
            Self::Shifts => "table.tpl_shifts",
            Self::Tasks => "table.tpl_tasks",
            Self::Gantt => "table.tpl_gantt",
            Self::Directory => "table.tpl_directory",
            Self::Budget => "table.tpl_budget",
        }
    }

    /// The table, in px of a page 1000 px wide, in the language of `texts`.
    pub(crate) fn table(self, texts: &Texts) -> TableNode {
        Maker::new(texts).make(self)
    }
}

const INK: u32 = 0x0018_243b;
const CORAL: u32 = 0x00ff_6b57;
const MINT: u32 = 0x003f_bfa9;
const MIST: u32 = 0x00e7_eaf2;
const WHITE: u32 = 0x00ff_ffff;
const LINE: u32 = 0x00c9_cfdc;

/// Builds the tables.
struct Maker<'a> {
    texts: &'a Texts,
}

/// A cell of a template: words of the catalogue (`table.w_…`) or as they
/// are.
type Row<'a> = &'a [&'static str];

impl<'a> Maker<'a> {
    fn new(texts: &'a Texts) -> Self {
        Self { texts }
    }

    fn words(&self, cell: &'static str) -> String {
        if cell.starts_with("table.") {
            plain(self.texts, cell)
        } else {
            cell.to_owned()
        }
    }

    /// `rows` of cells in columns of `widths` and rows of `height` px,
    /// words of 16 px in ink, thin lines between the cells.
    fn sheet(&self, widths: &[f64], height: f64, rows: &[Row<'_>]) -> TableNode {
        let style = CellStyle::new("Inter", 16.0, hex(INK));
        let mut t = TableNode::new(rows.len(), widths.len(), (1.0, height), &style);
        t.columns = widths.to_vec();
        for (r, row) in rows.iter().enumerate() {
            for (c, cell) in row.iter().enumerate() {
                if let Some(own) = t.cell_mut(r, c) {
                    own.text = self.words(cell);
                }
            }
        }
        let all = Range::all(&t);
        lines(&mut t, all, BorderSet::All, LINE, 1.0);
        t
    }

    fn make(&self, which: Template) -> TableNode {
        match which {
            Template::Grid => self.sheet(&[200.0; 3], 60.0, &[&[], &[], &[], &[]]),
            Template::Header => {
                let mut t = self.sheet(&[200.0; 3], 60.0, &[&[], &[], &[], &[]]);
                header(&mut t, CORAL, INK);
                t
            }
            Template::Zebra => {
                let mut t = self.sheet(&[200.0; 3], 54.0, &[&[], &[], &[], &[], &[]]);
                bare(&mut t);
                header(&mut t, MINT, INK);
                zebra(&mut t, 1, 0x00e6_f6f2);
                t
            }
            Template::Sided => {
                let mut t = self.sheet(&[200.0; 3], 60.0, &[&[], &[], &[], &[]]);
                header(&mut t, INK, WHITE);
                paint(&mut t, Range::new((1, 0), (3, 0)), MIST, INK, true);
                t
            }
            Template::Lines => {
                let mut t = self.sheet(&[200.0; 3], 56.0, &[&[], &[], &[], &[], &[]]);
                bare(&mut t);
                let all = Range::all(&t);
                lines(&mut t, all, BorderSet::Horizontal, LINE, 1.0);
                let first = Range::new((0, 0), (0, 2));
                lines(&mut t, first, BorderSet::Bottom, INK, 2.0);
                restyle(&mut t, first, |s| s.bold = true);
                t.header = true;
                t
            }
            Template::ProsCons => {
                let mut t = self.sheet(
                    &[320.0; 2],
                    64.0,
                    &[
                        &["table.w_pros", "table.w_cons"],
                        &["table.w_pros_1", "table.w_cons_1"],
                        &[],
                        &[],
                    ],
                );
                header(&mut t, 0x0017_7a5b, WHITE);
                paint(&mut t, Range::cell(0, 1), 0x00b3_261e, WHITE, true);
                t
            }
            Template::Compare => {
                let mut t = self.sheet(
                    &[180.0; 4],
                    64.0,
                    &[
                        &[
                            "",
                            "table.w_option_a",
                            "table.w_option_b",
                            "table.w_option_c",
                        ],
                        &["table.w_price"],
                        &["table.w_quality"],
                        &["table.w_support"],
                    ],
                );
                header(&mut t, 0x0035_b3c8, INK);
                paint(&mut t, Range::new((1, 0), (3, 0)), 0x00df_f2f5, INK, true);
                t
            }
            Template::Plans => {
                let mut t = self.sheet(
                    &[220.0, 150.0, 150.0, 150.0],
                    54.0,
                    &[
                        &["", "table.w_basic", "table.w_pro", "table.w_business"],
                        &[
                            "table.w_feature_1",
                            "table.w_yes",
                            "table.w_yes",
                            "table.w_yes",
                        ],
                        &["table.w_feature_2", "—", "table.w_yes", "table.w_yes"],
                        &["table.w_feature_3", "—", "—", "table.w_yes"],
                        &["table.w_total", "$ 10", "$ 25", "$ 60"],
                    ],
                );
                header(&mut t, 0x00f2_8c28, INK);
                zebra(&mut t, 1, 0x00fd_f0e2);
                restyle(&mut t, Range::new((4, 0), (4, 3)), |s| s.bold = true);
                left(&mut t, Range::new((1, 0), (4, 0)));
                t
            }
            Template::Glossary => {
                let mut t = self.sheet(
                    &[200.0, 460.0],
                    58.0,
                    &[
                        &["table.w_term", "table.w_definition"],
                        &["table.w_term_1", "table.w_definition_1"],
                        &[],
                        &[],
                        &[],
                    ],
                );
                header(&mut t, 0x002f_4fb5, WHITE);
                paint(&mut t, Range::new((1, 0), (4, 0)), 0x00e3_e9fa, INK, true);
                left(&mut t, Range::new((1, 1), (4, 1)));
                t
            }
            Template::Month => {
                let mut t = self.sheet(
                    &[100.0; 7],
                    62.0,
                    &[
                        &["table.w_month"],
                        &WEEK,
                        &["1", "2", "3", "4", "5", "6", "7"],
                        &["8", "9", "10", "11", "12", "13", "14"],
                        &["15", "16", "17", "18", "19", "20", "21"],
                        &["22", "23", "24", "25", "26", "27", "28"],
                        &["29", "30", "31"],
                    ],
                );
                let title = Range::new((0, 0), (0, 6));
                paint(&mut t, title, CORAL, INK, true);
                merge(&mut t, title);
                paint(&mut t, Range::new((1, 0), (1, 6)), 0x00ff_dcd6, INK, true);
                t.rows = vec![46.0, 36.0, 62.0, 62.0, 62.0, 62.0, 62.0];
                restyle(&mut t, Range::new((2, 0), (6, 6)), |s| {
                    s.align = TextAlign::Right;
                    s.valign = gafira_doc::VerticalAlign::Top;
                    s.size = 13.0;
                });
                t
            }
            Template::Week => {
                let mut t = self.sheet(&[110.0; 7], 160.0, &[&WEEK, &[]]);
                header(&mut t, 0x0035_b3c8, INK);
                t.rows = vec![44.0, 180.0];
                paint(&mut t, Range::new((1, 5), (1, 6)), 0x00df_f2f5, INK, false);
                t
            }
            Template::Hours => {
                let mut t = self.sheet(
                    &[220.0, 420.0],
                    50.0,
                    &[
                        &["table.w_hour", "table.w_activity"],
                        &["8:00 – 9:00", "table.w_activity_1"],
                        &["9:00 – 10:00"],
                        &["10:00 – 11:00"],
                        &["11:00 – 12:00"],
                    ],
                );
                bare(&mut t);
                let all = Range::all(&t);
                lines(&mut t, all, BorderSet::Horizontal, LINE, 1.0);
                header(&mut t, 0x00f2_8c28, INK);
                left(&mut t, Range::new((1, 1), (4, 1)));
                t
            }
            Template::Timetable => {
                let mut t = self.sheet(
                    &[110.0, 120.0, 120.0, 120.0, 120.0, 120.0],
                    44.0,
                    &[
                        &["", WEEK[0], WEEK[1], WEEK[2], WEEK[3], WEEK[4]],
                        &["8:00", "table.w_subject_1"],
                        &["9:00", "", "table.w_subject_2"],
                        &["10:00", "", "", "", "table.w_subject_1"],
                        &["11:00", "", "", "table.w_subject_2"],
                        &["12:00"],
                    ],
                );
                restyle(&mut t, Range::new((0, 0), (0, 5)), |s| s.bold = true);
                for cell in [(1, 1), (2, 2), (3, 4), (4, 3)] {
                    paint(&mut t, Range::cell(cell.0, cell.1), 0x0057_d6a3, INK, false);
                }
                t
            }
            Template::Sprints => {
                let mut t = self.sheet(
                    &[140.0, 180.0, 160.0, 200.0],
                    70.0,
                    &[
                        &[
                            "table.w_sprint",
                            "table.w_task",
                            "table.w_status",
                            "table.w_notes",
                        ],
                        &["table.w_sprint_1", "table.w_task_1", "table.w_doing"],
                        &["table.w_sprint_2"],
                        &["table.w_sprint_3"],
                    ],
                );
                header(&mut t, 0x006f_a8f0, INK);
                for (r, color) in [(1, 0x002f_4fb5), (2, 0x00a8_3c00), (3, 0x0017_7a5b)] {
                    paint(&mut t, Range::cell(r, 0), color, WHITE, true);
                }
                t
            }
            Template::Log => {
                let mut t = self.sheet(
                    &[70.0, 140.0, 280.0, 180.0],
                    48.0,
                    &[
                        &[
                            "table.w_number",
                            "table.w_date",
                            "table.w_subject",
                            "table.w_owner",
                        ],
                        &["1"],
                        &["2"],
                        &["3"],
                        &["4"],
                        &["5"],
                    ],
                );
                bare(&mut t);
                header(&mut t, 0x002f_4fb5, WHITE);
                zebra(&mut t, 1, 0x00e3_e9fa);
                t
            }
            Template::Shifts => {
                let mut t = self.sheet(
                    &[130.0, 180.0, 180.0, 180.0],
                    64.0,
                    &[
                        &["", "table.w_morning", "table.w_afternoon", "table.w_night"],
                        &["table.w_day_1"],
                        &["table.w_day_2"],
                        &["table.w_day_3"],
                    ],
                );
                header(&mut t, 0x0062_3cb8, WHITE);
                paint(&mut t, Range::new((1, 0), (3, 0)), 0x00ec_e6fa, INK, true);
                t
            }
            Template::Tasks => {
                let mut t = self.sheet(
                    &[280.0, 160.0, 140.0, 120.0],
                    46.0,
                    &[
                        &[
                            "table.w_task",
                            "table.w_owner",
                            "table.w_due",
                            "table.w_status",
                        ],
                        &[
                            "table.w_task_1",
                            "table.w_person_1",
                            "2026-12-01",
                            "table.w_doing",
                        ],
                        &[],
                        &[],
                        &[],
                        &[],
                    ],
                );
                bare(&mut t);
                header(&mut t, 0x0035_b3c8, INK);
                zebra(&mut t, 2, 0x00df_f2f5);
                left(&mut t, Range::new((1, 0), (5, 0)));
                t
            }
            Template::Gantt => {
                let mut t = self.sheet(
                    &[220.0, 130.0, 60.0, 60.0, 60.0, 60.0, 60.0, 60.0],
                    44.0,
                    &[
                        &[
                            "table.w_task",
                            "table.w_owner",
                            "1",
                            "2",
                            "3",
                            "4",
                            "5",
                            "6",
                        ],
                        &["table.w_phase_1"],
                        &["table.w_phase_2"],
                        &["table.w_phase_3"],
                        &[],
                    ],
                );
                header(&mut t, 0x006f_a8f0, INK);
                for (r, from, to) in [(1, 2, 3), (2, 3, 5), (3, 6, 7)] {
                    paint(
                        &mut t,
                        Range::new((r, from), (r, to)),
                        0x002f_4fb5,
                        WHITE,
                        false,
                    );
                }
                left(&mut t, Range::new((1, 0), (4, 0)));
                t
            }
            Template::Directory => {
                let mut t = self.sheet(
                    &[240.0, 220.0, 200.0],
                    46.0,
                    &[
                        &["table.w_directory"],
                        &["table.w_name", "table.w_role", "table.w_contact"],
                        &["table.w_person_1", "table.w_role_1", "table.w_contact_1"],
                        &[],
                        &[],
                    ],
                );
                let title = Range::new((0, 0), (0, 2));
                paint(&mut t, title, 0x0062_3cb8, WHITE, true);
                merge(&mut t, title);
                paint(&mut t, Range::new((1, 0), (1, 2)), 0x00ec_e6fa, INK, true);
                t
            }
            Template::Budget => {
                let mut t = self.sheet(
                    &[240.0, 140.0, 140.0, 140.0],
                    46.0,
                    &[
                        &[
                            "table.w_concept",
                            "table.w_planned",
                            "table.w_actual",
                            "table.w_difference",
                        ],
                        &["table.w_concept_1", "1200", "1150", "50"],
                        &["table.w_concept_2", "800", "920", "-120"],
                        &["table.w_concept_3", "450", "430", "20"],
                        &["table.w_total", "2450", "2500", "-50"],
                    ],
                );
                header(&mut t, INK, WHITE);
                restyle(&mut t, Range::new((1, 1), (4, 3)), |s| {
                    s.align = TextAlign::Right
                });
                left(&mut t, Range::new((1, 0), (4, 0)));
                paint(&mut t, Range::new((4, 0), (4, 3)), MIST, INK, true);
                t
            }
        }
    }
}

const WEEK: [&str; 7] = [
    "table.w_mon",
    "table.w_tue",
    "table.w_wed",
    "table.w_thu",
    "table.w_fri",
    "table.w_sat",
    "table.w_sun",
];

fn lines(t: &mut TableNode, range: Range, which: BorderSet, color: u32, width: f64) {
    set_borders(t, range, which, Some(ShapeStroke::new(hex(color), width)));
}

fn bare(t: &mut TableNode) {
    let all = Range::all(t);
    set_borders(t, all, BorderSet::All, None);
}

fn paint(t: &mut TableNode, range: Range, fill: u32, ink: u32, bold: bool) {
    restyle(t, range, |s| {
        s.fill = Some(hex(fill));
        s.color = hex(ink);
        s.bold = bold;
    });
}

fn header(t: &mut TableNode, fill: u32, ink: u32) {
    let last = t.columns.len() - 1;
    paint(t, Range::new((0, 0), (0, last)), fill, ink, true);
    t.header = true;
}

/// Every other row from `first`, lightly filled.
fn zebra(t: &mut TableNode, first: usize, fill: u32) {
    let last = t.columns.len() - 1;
    for r in (first..t.rows.len()).step_by(2) {
        restyle(t, Range::new((r, 0), (r, last)), |s| {
            s.fill = Some(hex(fill))
        });
    }
}

fn left(t: &mut TableNode, range: Range) {
    restyle(t, range, |s| s.align = TextAlign::Left);
}

/// `t` on a page of `page_px`: two thirds of its width at most, and of its
/// height, the words in proportion; centred on `at` or in the middle.
pub(crate) fn spec(
    t: TableNode,
    page_px: (f64, f64),
    at: Option<(f64, f64)>,
    name: String,
) -> NodeSpec {
    let (w, h) = size(&t);
    let k = (page_px.0 * 2.0 / 3.0 / w).min(page_px.1 * 2.0 / 3.0 / h);
    scaled(t, k, page_px, at, name)
}

/// The most of a page a table of a spreadsheet takes.
const SHEET_SHARE: f64 = 0.9;

/// A table of a spreadsheet on a page of `page_px` at `dpi` (S82-05): at its
/// own size, or smaller to fit; centred on `at`, or in the middle, and
/// inside the page.
pub(crate) fn sheet_spec(
    t: TableNode,
    page_px: (f64, f64),
    dpi: f64,
    at: Option<(f64, f64)>,
    name: String,
) -> NodeSpec {
    // The height its words take, before it shrinks with them.
    let (w, h) = size(&t);
    let all = Range::all(&t);
    let words = crate::tables::Act::Padding(t.padding);
    let t = crate::tables::change(&t, &Placement::new(0.0, 0.0, w, h), all, &words)
        .map_or(t, |(t, _, _)| t);
    let (w, h) = size(&t);
    let most = (page_px.0 * SHEET_SHARE / w).min(page_px.1 * SHEET_SHARE / h);
    let mut spec = scaled(t, (dpi / 96.0).min(most), page_px, at, name);
    let p = &mut spec.placement;
    let inside = |at: f64, side: f64, page: f64| at.min(page - side).max(0.0);
    p.x = inside(p.x, p.w, page_px.0);
    p.y = inside(p.y, p.h, page_px.1);
    spec
}

/// `t` by `k`, its words too, centred on `at` or in the middle of the page.
fn scaled(
    mut t: TableNode,
    k: f64,
    page_px: (f64, f64),
    at: Option<(f64, f64)>,
    name: String,
) -> NodeSpec {
    for side in t.columns.iter_mut().chain(&mut t.rows) {
        *side *= k;
    }
    t.padding = (t.padding * k).min(TableNode::MAX_SPACING);
    let all = Range::all(&t);
    restyle(&mut t, all, |s| {
        s.size = (s.size * k).clamp(1.0, gafira_doc::TextNode::MAX_SIZE);
        let b = &mut s.borders;
        for stroke in [&mut b.top, &mut b.right, &mut b.bottom, &mut b.left]
            .into_iter()
            .flatten()
        {
            stroke.width = (stroke.width * k).clamp(0.5, ShapeStroke::MAX_WIDTH);
        }
    });
    let (w, h) = size(&t);
    let (x, y) = at.unwrap_or((page_px.0 / 2.0, page_px.1 / 2.0));
    let placement = Placement::new(x - w / 2.0, y - h / 2.0, w, h);
    // Rows tall enough for their words, with the same fonts as the canvas.
    let words = crate::tables::Act::Padding(t.padding);
    let (t, placement) = crate::tables::change(&t, &placement, all, &words)
        .map_or((t, placement), |(t, placement, _)| (t, placement));
    NodeSpec {
        name: Some(name),
        ..NodeSpec::new(NodeKind::Table(Box::new(t)), placement)
    }
}

/// The panel: the group open with «Ver todo», and the pictures.
#[derive(Default)]
pub(crate) struct Panel {
    open: Option<Group>,
    thumbs: HashMap<(usize, String), egui::TextureHandle>,
    outlines: Outlines,
    /// How far each row is scrolled, in px.
    offsets: HashMap<usize, f32>,
}

/// The table asked for, and where it was let go (screen).
pub(crate) type Chosen = (Template, Option<Pos2>);

impl Panel {
    pub(crate) fn show(
        &mut self,
        ui: &mut Ui,
        theme: &Theme,
        texts: &Texts,
        over: &dyn Fn(Pos2) -> bool,
    ) -> Option<Chosen> {
        let mut chosen = None;
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
            let list: Vec<Template> = of(group).collect();
            let width = (ui.available_width() - 8.0) / 2.0;
            for line in list.chunks(2) {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 8.0;
                    for template in line {
                        self.tile(ui, theme, texts, (*template, width), over, &mut chosen);
                    }
                });
                ui.add_space(8.0);
            }
            return chosen;
        }
        for group in Group::ALL {
            ui.horizontal(|ui| {
                title(ui, theme, plain(texts, group.key()));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let all = ui.add(
                        egui::Button::new(
                            RichText::new(plain(texts, "illustrations.see_all"))
                                .font(theme.font("body-sm")),
                        )
                        .frame(false),
                    );
                    if all.hovered() {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                    }
                    if all.clicked() {
                        self.open = Some(group);
                    }
                });
            });
            ui.add_space(4.0);
            let width = ui.available_width() * 0.56;
            let offset = self.offsets.get(&(group as usize)).copied().unwrap_or(0.0);
            let shown = egui::ScrollArea::horizontal()
                .id_salt(("tables.row", group.key()))
                .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
                .horizontal_scroll_offset(offset)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 8.0;
                        for template in of(group) {
                            self.tile(ui, theme, texts, (template, width), over, &mut chosen);
                        }
                    });
                });
            // An arrow at each end there is more behind.
            let area = shown.inner_rect;
            let most = (shown.content_size.x - area.width()).max(0.0);
            let mut to = shown.state.offset.x;
            for (way, x, on, key) in [
                (-1.0, area.left() + 14.0, to > 1.0, "table.less"),
                (1.0, area.right() - 14.0, to < most - 1.0, "table.more"),
            ] {
                let spot = Rect::from_center_size(
                    Pos2::new(x, area.top() + width * 0.34),
                    Vec2::splat(26.0),
                );
                let id = ui.id().with((group.key(), key));
                let mark = crate::tables::Mark::Arrow(way);
                let name = plain(texts, key);
                if on && crate::tables::pill(ui, theme, id, spot, mark, &name).clicked() {
                    to = (to + way * (width + 8.0)).clamp(0.0, most);
                }
            }
            self.offsets.insert(group as usize, to);
            ui.add_space(14.0);
        }
        chosen
    }

    /// One table: a click inserts it, a drag carries it (D-105).
    fn tile(
        &mut self,
        ui: &mut Ui,
        theme: &Theme,
        texts: &Texts,
        (template, width): (Template, f32),
        over: &dyn Fn(Pos2) -> bool,
        chosen: &mut Option<Chosen>,
    ) {
        let (rect, response) =
            ui.allocate_exact_size(Vec2::new(width, width * 0.68), Sense::click_and_drag());
        if !ui.is_rect_visible(rect) {
            return;
        }
        let name = plain(texts, template.key());
        let painter = ui.painter();
        painter.rect_filled(rect, theme.radius_md(), theme.c("surface.sunken"));
        if response.hovered() || response.has_focus() {
            painter.rect_stroke(
                rect,
                theme.radius_md(),
                Stroke::new(2.0, theme.c("primary.bg")),
                egui::StrokeKind::Inside,
            );
        }
        let ctx = ui.ctx().clone();
        // The words of the picture change with the language.
        let key = (template as usize, name.clone());
        if !self.thumbs.contains_key(&key) {
            let table = template.table(texts);
            let k = 2.0 * f64::from(width) / size(&table).0;
            if let Some(pixels) =
                crate::tables::image(&table, size(&table), k as f32, None, &mut self.outlines)
            {
                let texture = ctx.load_texture("table-thumb", pixels, egui::TextureOptions::LINEAR);
                self.thumbs.insert(key.clone(), texture);
            }
        }
        if let Some(texture) = self.thumbs.get(&key) {
            let room = rect.shrink(12.0);
            let size = texture.size_vec2();
            let k = (room.width() / size.x).min(room.height() / size.y);
            let place = Rect::from_center_size(room.center(), size * k);
            let uv = Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0));
            // White paper, in the dark theme too.
            painter.rect_filled(place, 0.0, egui::Color32::WHITE);
            painter.image(texture.id(), place, uv, egui::Color32::WHITE);
            self.drag(ui, &response, (template, texture), over, chosen);
        }
        if response.hovered() && !response.dragged() {
            ctx.set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        if response.clicked() {
            *chosen = Some((template, None));
        }
        response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, &name));
        response.on_hover_text(name);
    }

    fn drag(
        &self,
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
            let layer = egui::LayerId::new(egui::Order::Tooltip, egui::Id::new("table_ghost"));
            let size = texture.size_vec2();
            let ghost = Rect::from_center_size(at, size * (180.0 / size.x));
            let uv = Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0));
            let tint = egui::Color32::WHITE.gamma_multiply(0.7);
            ui.ctx()
                .layer_painter(layer)
                .image(texture.id(), ghost, uv, tint);
        }
        if response.drag_stopped() && over(at) {
            *chosen = Some((template, Some(at)));
        }
    }
}

fn of(group: Group) -> impl Iterator<Item = Template> {
    Template::ALL
        .into_iter()
        .filter(move |t| t.group() == group)
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

    // S81-07: every table of the panel is one a document holds, on a wide
    // page and on a tall one, in both languages, and its words fit.
    #[test]
    fn s81_07_every_template_is_valid_and_fits() {
        use gafira_ports::i18n::Locale;
        for locale in [Locale::Es, Locale::En] {
            let texts = Texts::new(locale);
            for template in Template::ALL {
                let table = template.table(&texts);
                assert!(table.is_valid(), "{template:?}");
                assert!(
                    table.cells.iter().all(|c| !c.text.starts_with("table.")),
                    "{template:?}: words without a translation"
                );
                for page in [(1920.0, 1080.0), (1080.0, 1920.0)] {
                    let spec = spec(table.clone(), page, None, String::new());
                    let NodeKind::Table(t) = &spec.kind else {
                        panic!("a table")
                    };
                    assert!(t.is_valid());
                    let p = spec.placement;
                    assert!(p.w <= page.0 * 0.67, "{template:?} {p:?}");
                    let (w, h) = size(t);
                    assert!((w - p.w).abs() < 1e-6 && (h - p.h).abs() < 1e-6);
                    assert!((p.x + p.w / 2.0 - page.0 / 2.0).abs() < 1e-6);
                }
            }
        }
        for group in Group::ALL {
            assert!(of(group).count() >= 3);
        }
    }

    // S81-07: the words can be read on their cells (03 §4: 4.5 to 1).
    #[test]
    fn s81_07_the_words_stand_out() {
        let texts = Texts::new(gafira_ports::i18n::Locale::Es);
        for template in Template::ALL {
            for cell in template.table(&texts).cells {
                let fill = cell.style.fill.unwrap_or(hex(WHITE));
                let ratio = gafira_qr::contrast(cell.style.color, fill);
                assert!(ratio >= 4.5, "{template:?}: {ratio:.2} {:?}", cell.style);
            }
        }
    }
}
