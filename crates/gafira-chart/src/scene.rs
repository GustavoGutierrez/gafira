//! What draws a chart: filled paths and words, back to front, in px of its
//! box (S83-03).

use gafira_color::{Color, ColorSpaceId, Rgba};
use gafira_doc::{
    ChartKind, ChartLabels, ChartNode, Flip, NodeKind, PathCommand, Placement, ShapeGeometry,
    ShapeNode, ShapePath, SpanStyle, TextAlign, TextNode,
};
use kurbo::{Affine, BezPath, PathEl, Point, Rect, Shape as _, Vec2};

use crate::data::{Colored, Data, colored, plain, read};
use crate::paths::marker;
use crate::{areas, bars, lines, round};

/// Where words lie from their point.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Anchor {
    Start,
    Middle,
    End,
}

/// A line of words.
#[derive(Clone, Debug, PartialEq)]
pub struct Words {
    pub text: String,
    pub at: Point,
    /// Across: the point is at their left, middle or right.
    pub h: Anchor,
    /// Down: the point is at their top, middle or bottom.
    pub v: Anchor,
    pub size: f64,
    pub bold: bool,
    pub color: Color,
}

/// A piece of a drawing.
#[derive(Clone, Debug, PartialEq)]
pub enum Piece {
    Fill { path: BezPath, color: Color },
    Words(Words),
}

/// The width and the height of a line of words of a size, bold or not.
pub type Measure<'a> = dyn Fn(&str, f64, bool) -> (f64, f64) + 'a;

/// A row of the legend: its width, and the index, the name and the width of
/// each entry.
type LegendRow = (f64, Vec<(usize, String, f64)>);

/// A drawing being made.
pub(crate) struct Scene<'a> {
    pub chart: &'a ChartNode,
    pub data: Data,
    measure: &'a Measure<'a>,
    /// The size of the words.
    pub size: f64,
    pub out: Vec<Piece>,
}

/// The pieces of `chart` in a box of `w` × `h` px; none when there is
/// nothing to draw or nowhere to.
#[must_use]
pub fn draw(chart: &ChartNode, (w, h): (f64, f64), measure: &Measure<'_>) -> Vec<Piece> {
    let sized = |v: f64| v.is_finite() && v >= 1.0;
    if !(sized(w) && sized(h)) {
        return Vec::new();
    }
    let mut scene = Scene {
        chart,
        data: read(chart),
        measure,
        size: chart.text.size,
        out: Vec::new(),
    };
    let mut area = Rect::new(0.0, 0.0, w, h);
    scene.title(&mut area);
    if scene.data.series.is_empty() || scene.data.labels.is_empty() {
        return scene.out;
    }
    scene.legend(&mut area);
    if area.width() < 1.0 || area.height() < 1.0 {
        return scene.out;
    }
    match chart.kind {
        ChartKind::Columns | ChartKind::GroupedColumns => bars::grouped(&mut scene, area, false),
        ChartKind::Rows | ChartKind::GroupedRows => bars::grouped(&mut scene, area, true),
        ChartKind::StackedColumns => bars::stacked(&mut scene, area, false),
        ChartKind::StackedRows => bars::stacked(&mut scene, area, true),
        ChartKind::Histogram => bars::histogram(&mut scene, area),
        ChartKind::Line => lines::line(&mut scene, area),
        ChartKind::Area => lines::area(&mut scene, area),
        ChartKind::StackedArea | ChartKind::Stream | ChartKind::PercentArea => {
            lines::stacked(&mut scene, area);
        }
        ChartKind::Scatter | ChartKind::Bubbles => lines::scatter(&mut scene, area),
        ChartKind::DotColumns => lines::dots(&mut scene, area, false),
        ChartKind::DotRows => lines::dots(&mut scene, area, true),
        ChartKind::Pie | ChartKind::Donut => round::pie(&mut scene, area),
        ChartKind::Sunburst => round::sunburst(&mut scene, area),
        ChartKind::ProgressRing => round::progress(&mut scene, area, true),
        ChartKind::ProgressArc => round::progress(&mut scene, area, false),
        ChartKind::Gauge => round::gauge(&mut scene, area),
        ChartKind::Radar => round::radar(&mut scene, area),
        ChartKind::ProgressBar => bars::progress(&mut scene, area),
        ChartKind::Pack => areas::pack(&mut scene, area),
        ChartKind::NestedPack => areas::nested(&mut scene, area),
        ChartKind::Treemap => areas::treemap(&mut scene, area),
        ChartKind::Funnel => areas::funnel(&mut scene, area),
    }
    // What data and boxes out of all measure leave behind.
    scene.out.retain(|piece| match piece {
        Piece::Fill { path, .. } => path.is_finite() && !path.elements().is_empty(),
        Piece::Words(words) => words.at.is_finite(),
    });
    scene.out
}

impl Scene<'_> {
    /// The colour of the series, the label or the group at `index`.
    pub fn color(&self, index: usize) -> Color {
        let colors = &self.chart.colors;
        colors
            .get(index.checked_rem(colors.len()).unwrap_or(0))
            .copied()
            .unwrap_or(self.chart.text.color)
    }

    /// The colour of the grid: the words', faint.
    pub fn faint(&self) -> Color {
        with_alpha(self.chart.text.color, 0.15)
    }

    pub fn fill(&mut self, path: BezPath, color: Color) {
        self.out.push(Piece::Fill { path, color });
    }

    pub fn width(&self, text: &str) -> f64 {
        (self.measure)(text, self.size, self.chart.text.bold).0
    }

    /// The height of a line of words.
    pub fn line(&self) -> f64 {
        self.size * 1.25
    }

    /// Words of the chart's size in its colour.
    pub fn words(&mut self, text: &str, at: Point, h: Anchor, v: Anchor) {
        let color = self.chart.text.color;
        self.words_in(text, at, (h, v), color);
    }

    pub fn words_in(&mut self, text: &str, at: Point, (h, v): (Anchor, Anchor), color: Color) {
        self.sized(text, at, (h, v), color, self.size);
    }

    pub fn sized(
        &mut self,
        text: &str,
        at: Point,
        (h, v): (Anchor, Anchor),
        color: Color,
        size: f64,
    ) {
        // A label is one line.
        let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
        if !text.is_empty() {
            self.out.push(Piece::Words(Words {
                text,
                at,
                h,
                v,
                size,
                bold: self.chart.text.bold,
                color,
            }));
        }
    }

    /// `text` cut to `width`, with a mark; none when not even a letter fits.
    pub fn fit(&self, text: &str, width: f64) -> Option<String> {
        if self.width(text) <= width {
            return Some(text.to_owned());
        }
        let chars: Vec<char> = text.chars().collect();
        let cut = |n: usize| {
            let kept: String = chars[..n].iter().collect();
            format!("{}…", kept.trim_end())
        };
        // The longest that fits, halving.
        let (mut least, mut most) = (0, chars.len());
        while least < most {
            let middle = usize::midpoint(least, most + 1);
            if self.width(&cut(middle)) <= width {
                least = middle;
            } else {
                most = middle - 1;
            }
        }
        (least > 0).then(|| cut(least))
    }

    /// The number written on the value of `series` at `row`: as it is, or its
    /// share of `whole`.
    pub fn number(&self, series: usize, row: usize, whole: f64) -> Option<String> {
        let of = self.data.series.get(series)?;
        let v = of.values.get(row).copied().flatten()?;
        match self.chart.numbers {
            ChartLabels::None => None,
            ChartLabels::Value => of.words.get(row).cloned(),
            ChartLabels::Percent if whole > 0.0 => Some(self.share(v / whole)),
            ChartLabels::Percent => None,
        }
    }

    /// A share as it is written: whole, unless it is under one per cent.
    pub fn share(&self, share: f64) -> String {
        let whole = (share * 100.0).round() / 100.0;
        let shown = if share.abs() < 0.01 { share } else { whole };
        plain(shown, true, self.data.comma)
    }

    /// `text` at the size of the words, or down to seven tenths of it, or
    /// cut, to fit in `width`; none when hardly a letter does.
    pub fn squeezed(&self, text: &str, width: f64) -> Option<(String, f64)> {
        const LEAST: f64 = 0.7;
        let wide = self.width(text);
        if wide <= 0.0 {
            return None;
        }
        let k = (width / wide).min(1.0);
        if k >= LEAST {
            return Some((text.to_owned(), self.size * k));
        }
        let cut = self.fit(text, width / LEAST)?;
        (cut.chars().count() > 4).then_some((cut, self.size * LEAST))
    }

    /// The label of `row`, if the labels are written.
    pub fn label(&self, row: usize) -> &str {
        if self.chart.names {
            self.data.labels.get(row).map_or("", String::as_str)
        } else {
            ""
        }
    }

    /// The groups of the rows, each once, and the one of each row.
    pub fn groups(&self) -> (Vec<String>, Vec<usize>) {
        let of_rows = if self.data.groups.is_empty() {
            &self.data.labels
        } else {
            &self.data.groups
        };
        let mut names: Vec<String> = Vec::new();
        let at = of_rows
            .iter()
            .map(|name| {
                names.iter().position(|n| n == name).unwrap_or_else(|| {
                    names.push(name.clone());
                    names.len() - 1
                })
            })
            .collect();
        (names, at)
    }

    fn title(&mut self, area: &mut Rect) {
        let words = self.chart.title.trim().to_owned();
        let size = self.size * 1.3;
        let tall = size * 1.6;
        if words.is_empty() || tall > area.height() / 2.0 {
            return;
        }
        let Some(text) = self.fit(&words, area.width() / 1.3) else {
            return;
        };
        let at = Point::new(area.center().x, area.y0);
        let color = self.chart.text.color;
        self.sized(&text, at, (Anchor::Middle, Anchor::Start), color, size);
        area.y0 += tall;
    }

    /// The names of the colours, when there are two or more, in rows at the
    /// bottom.
    fn legend(&mut self, area: &mut Rect) {
        if !self.chart.legend {
            return;
        }
        let by = colored(self.chart);
        let names: Vec<String> = match by {
            Colored::Series => self.data.series.iter().map(|s| s.name.clone()).collect(),
            Colored::Rows => self.data.labels.clone(),
            Colored::Groups => self.groups().0,
        };
        // A total is not a series of its own.
        let single = matches!(
            self.chart.kind,
            ChartKind::ProgressRing
                | ChartKind::ProgressArc
                | ChartKind::ProgressBar
                | ChartKind::Gauge
        );
        if names.len() < 2 || (single && by == Colored::Series) {
            return;
        }
        let (mark, gap, tall) = (self.size * 0.8, self.size * 0.4, self.size * 1.6);
        let mut rows: Vec<LegendRow> = Vec::new();
        for (i, name) in names.iter().enumerate() {
            let room = area.width() - mark - gap;
            let Some(text) = self.fit(name, room).filter(|t| !t.trim().is_empty()) else {
                continue;
            };
            let width = mark + gap + self.width(&text);
            let wide = width + self.size;
            match rows.last_mut() {
                Some((used, row)) if *used + width <= area.width() => {
                    *used += wide;
                    row.push((i, text, wide));
                }
                _ => rows.push((wide, vec![(i, text, wide)])),
            }
        }
        // No more than a third of the box.
        rows.truncate((area.height() / 3.0 / tall).max(0.0) as usize);
        let shaped = matches!(self.chart.kind, ChartKind::Line | ChartKind::Scatter);
        // Few rows: the cast is exact.
        let top = area.y1 - tall * rows.len() as f64;
        for (r, (used, row)) in rows.iter().enumerate() {
            let y = top + tall * (r as f64 + 0.6);
            let mut x = area.center().x - (used - self.size) / 2.0;
            for (i, text, wide) in row {
                let at = Point::new(x + mark / 2.0, y);
                let path = if shaped {
                    marker(*i, at, mark / 2.0)
                } else {
                    Rect::from_center_size(at, (mark, mark))
                        .to_rounded_rect(mark * 0.25)
                        .to_path(0.1)
                };
                let color = self.color(*i);
                self.fill(path, color);
                let at = Point::new(x + mark + gap, y);
                self.words(text, at, Anchor::Start, Anchor::Middle);
                x += wide;
            }
        }
        area.y1 = top
            - if rows.is_empty() {
                0.0
            } else {
                self.size * 0.3
            };
    }
}

/// `color` with `share` of its opacity.
pub(crate) fn with_alpha(color: Color, share: f32) -> Color {
    match color.convert(ColorSpaceId::Srgb) {
        Color::Srgb(c) => Color::Srgb(Rgba::new(c.r, c.g, c.b, c.a * share)),
        other => other,
    }
}

/// `color` mixed with white, `share` of it.
pub(crate) fn lighter(color: Color, share: f32) -> Color {
    match color.convert(ColorSpaceId::Srgb) {
        Color::Srgb(c) => {
            let mix = |v: f32| v + (1.0 - v) * share;
            Color::Srgb(Rgba::new(mix(c.r), mix(c.g), mix(c.b), c.a))
        }
        other => other,
    }
}

/// The colour of words over `color`: white or ink, the one that stands out.
pub(crate) fn over(color: Color) -> Color {
    let luminance = match color.convert(ColorSpaceId::Srgb) {
        Color::Srgb(c) => {
            let linear = |v: f32| {
                let v = v.clamp(0.0, 1.0);
                if v <= 0.040_45 {
                    v / 12.92
                } else {
                    ((v + 0.055) / 1.055).powf(2.4)
                }
            };
            let l = 0.2126 * linear(c.r) + 0.7152 * linear(c.g) + 0.0722 * linear(c.b);
            // What lies behind is taken as white paper.
            l * c.a + (1.0 - c.a)
        }
        _ => 1.0,
    };
    // The contrast with white against the one with the ink of the brand.
    if 1.05 / (luminance + 0.05) >= (luminance + 0.05) / 0.066 {
        Color::Srgb(Rgba::new(1.0, 1.0, 1.0, 1.0))
    } else {
        Color::Srgb(Rgba::new(0.094, 0.141, 0.231, 1.0))
    }
}

/// `chart` in the box `placement` as shapes and texts, back to front: what
/// a format without charts takes (S83-09). The words are measured by their
/// count of letters.
#[must_use]
pub fn parts(chart: &ChartNode, placement: &Placement) -> Vec<(NodeKind, Placement)> {
    let (w, h) = (placement.w.abs(), placement.h.abs());
    let measure = |text: &str, size: f64, bold: bool| {
        let wide = if bold { 0.62 } else { 0.57 };
        // Few letters: the cast is exact.
        (text.chars().count() as f64 * size * wide, size * 1.25)
    };
    let to_page = box_to_page(placement);
    let mut out = Vec::new();
    for piece in draw(chart, (w, h), &measure) {
        match piece {
            Piece::Fill { path, color } => {
                let bounds = path.bounding_box();
                let bounds = Rect::from_center_size(
                    bounds.center(),
                    (bounds.width().max(0.01), bounds.height().max(0.01)),
                );
                for orders in pieces(&path, bounds) {
                    let mut shape = ShapeNode::new(ShapeGeometry::Path(ShapePath::new(orders)));
                    shape.fill = Some(color);
                    let at = part_of(placement, to_page, bounds);
                    out.push((NodeKind::Shape(shape), at));
                }
            }
            Piece::Words(words) => {
                let style = SpanStyle {
                    font: chart.text.font.clone(),
                    size: words.size,
                    color: words.color,
                    bold: words.bold,
                    italic: chart.text.italic,
                    gradient: None,
                };
                let mut text = TextNode::plain(&words.text, &style);
                let (wide, tall) = measure(&words.text, words.size, words.bold);
                let wide = wide + words.size;
                let x = match words.h {
                    Anchor::Start => words.at.x,
                    Anchor::Middle => words.at.x - wide / 2.0,
                    Anchor::End => words.at.x - wide,
                };
                let y = match words.v {
                    Anchor::Start => words.at.y,
                    Anchor::Middle => words.at.y - tall / 2.0,
                    Anchor::End => words.at.y - tall,
                };
                text.align = match words.h {
                    Anchor::Start => TextAlign::Left,
                    Anchor::Middle => TextAlign::Center,
                    Anchor::End => TextAlign::Right,
                };
                // Where its point says, past the box if it takes it: moved, it
                // would lie over the drawing.
                let bounds = Rect::new(x, y, x + wide, y + tall);
                let mut at = part_of(placement, to_page, bounds);
                // Words are not read backwards.
                at.flip = Flip::default();
                out.push((NodeKind::Text(text), at));
            }
        }
    }
    out
}

/// From px of the box to px of the page.
fn box_to_page(p: &Placement) -> Affine {
    let centre = Vec2::new(p.x + p.w / 2.0, p.y + p.h / 2.0);
    let flip = Affine::scale_non_uniform(
        if p.flip.x { -1.0 } else { 1.0 },
        if p.flip.y { -1.0 } else { 1.0 },
    );
    Affine::translate(centre)
        * Affine::rotate(p.rotation_deg.to_radians())
        * flip
        * Affine::translate((-p.w.abs() / 2.0, -p.h.abs() / 2.0))
}

/// The placement of the part `bounds` of the box `of`.
fn part_of(of: &Placement, to_page: Affine, bounds: Rect) -> Placement {
    let centre = to_page * bounds.center();
    Placement {
        x: centre.x - bounds.width() / 2.0,
        y: centre.y - bounds.height() / 2.0,
        w: bounds.width(),
        h: bounds.height(),
        ..*of
    }
}

/// `path` in the unit box of `bounds`, in as many paths as it takes to keep
/// each within the orders of a shape.
fn pieces(path: &BezPath, bounds: Rect) -> Vec<Vec<PathCommand>> {
    let at = |p: Point| {
        [
            (p.x - bounds.x0) / bounds.width(),
            (p.y - bounds.y0) / bounds.height(),
        ]
    };
    let mut subs: Vec<Vec<PathCommand>> = Vec::new();
    for el in path.elements() {
        let order = match *el {
            PathEl::MoveTo(p) => PathCommand::MoveTo(at(p)),
            PathEl::LineTo(p) => PathCommand::LineTo(at(p)),
            PathEl::QuadTo(c, p) => PathCommand::QuadTo(at(c), at(p)),
            PathEl::CurveTo(c, d, p) => PathCommand::CubicTo(at(c), at(d), at(p)),
            PathEl::ClosePath => PathCommand::Close,
        };
        if matches!(order, PathCommand::MoveTo(_)) {
            subs.push(Vec::new());
        }
        if let Some(sub) = subs.last_mut() {
            sub.push(order);
        }
    }
    let total: usize = subs.iter().map(Vec::len).sum();
    if total <= ShapePath::MAX_COMMANDS {
        return vec![subs.concat()];
    }
    subs.retain(|sub| sub.len() <= ShapePath::MAX_COMMANDS);
    subs
}
