//! Lines, areas, points.

use gafira_doc::{ChartKind, ChartLabels};
use kurbo::{BezPath, Point, Rect};

use crate::axes::{Plot, Want};
use crate::paths::{marker, stroke, through};
use crate::scene::{Anchor, Scene, with_alpha};

fn labels(scene: &Scene<'_>) -> Vec<String> {
    (0..scene.data.labels.len())
        .map(|i| scene.label(i).to_owned())
        .collect()
}

/// The least and the most of `values`; `zero` among them.
fn range(values: impl Iterator<Item = f64>, zero: bool) -> (f64, f64) {
    let start = if zero {
        (0.0, 0.0)
    } else {
        (f64::INFINITY, f64::NEG_INFINITY)
    };
    let (min, max) = values.fold(start, |(a, b), v| (a.min(v), b.max(v)));
    if !(min.is_finite() && max.is_finite()) {
        (0.0, 1.0)
    } else if min == max {
        (
            min.min(0.0),
            if max > 0.0 { max } else { min.min(0.0) + 1.0 },
        )
    } else {
        (min, max)
    }
}

fn of_values<'a>(scene: &Scene<'_>, (min, max): (f64, f64), labels: &'a [String]) -> Want<'a> {
    Want {
        along_x: false,
        min,
        max,
        labels,
        to_ends: false,
        room: if scene.chart.numbers == ChartLabels::None {
            0.0
        } else {
            scene.line() * 1.3
        },
        axis: scene.chart.axis,
        percent: scene.data.percent,
    }
}

/// The plot of the series from end to end of the labels.
fn plot_of(scene: &mut Scene<'_>, area: Rect, values: (f64, f64)) -> Plot {
    let names = labels(scene);
    let want = Want {
        to_ends: true,
        ..of_values(scene, values, &names)
    };
    // The marks at both ends and the lines stay inside the box.
    let edge = (scene.chart.weight * 1.6)
        .min(area.width() * 0.1)
        .min(area.height() * 0.1);
    Plot::new(scene, area.inset(-edge), &want)
}

/// The runs of points of a series: a missing value parts them.
fn runs(scene: &Scene<'_>, plot: &Plot, series: usize) -> Vec<Vec<(usize, Point)>> {
    let mut out: Vec<Vec<(usize, Point)>> = vec![Vec::new()];
    for (i, v) in scene.data.series[series].values.iter().enumerate() {
        match (v, out.last_mut()) {
            (Some(v), Some(run)) => run.push((i, Point::new(plot.end(i), plot.at(*v)))),
            (None, Some(run)) if !run.is_empty() => out.push(Vec::new()),
            _ => {}
        }
    }
    out.retain(|run| !run.is_empty());
    out
}

fn values(scene: &Scene<'_>) -> Vec<f64> {
    let all = scene.data.series.iter().flat_map(|s| &s.values);
    all.copied().flatten().collect()
}

/// The lines, marks and numbers of every series.
fn trace(scene: &mut Scene<'_>, plot: &Plot) {
    let weight = scene
        .chart
        .weight
        .min(plot.area.width().min(plot.area.height()) * 0.2);
    for s in 0..scene.data.series.len() {
        let color = scene.color(s);
        for run in runs(scene, plot, s) {
            let points: Vec<Point> = run.iter().map(|(_, p)| *p).collect();
            if points.len() > 1 {
                let mut path = BezPath::new();
                through(&mut path, &points, scene.chart.smooth);
                scene.fill(stroke(&path, weight), color);
            }
            for (i, p) in &run {
                if scene.chart.markers || points.len() == 1 {
                    scene.fill(marker(s, *p, weight * 1.3), color);
                }
                if let Some(text) = scene.number(s, *i, 0.0) {
                    let at = Point::new(p.x, p.y - weight * 1.6 - scene.size * 0.2);
                    scene.words(&text, at, Anchor::Middle, Anchor::End);
                }
            }
        }
    }
}

pub(crate) fn line(scene: &mut Scene<'_>, area: Rect) {
    let all = values(scene);
    let (min, max) = range(all.iter().copied(), false);
    // A line near zero shows it.
    let (min, max) = if min > 0.0 && min < max * 0.6 {
        (0.0, max)
    } else {
        (min, max)
    };
    let plot = plot_of(scene, area, (min, max));
    trace(scene, &plot);
}

/// Areas one before the other.
pub(crate) fn area(scene: &mut Scene<'_>, area: Rect) {
    let all = values(scene);
    let plot = plot_of(scene, area, range(all.into_iter(), true));
    for s in 0..scene.data.series.len() {
        let color = with_alpha(scene.color(s), 0.35);
        for run in runs(scene, &plot, s) {
            let points: Vec<Point> = run.iter().map(|(_, p)| *p).collect();
            let (Some(first), Some(last)) = (points.first(), points.last()) else {
                continue;
            };
            let mut path = BezPath::new();
            path.move_to((first.x, plot.zero()));
            through(&mut path, &points, scene.chart.smooth);
            path.line_to((last.x, plot.zero()));
            path.close_path();
            scene.fill(path, color);
        }
    }
    trace(scene, &plot);
}

/// Areas one over the other: from zero, as shares of the whole, or around
/// the middle.
pub(crate) fn stacked(scene: &mut Scene<'_>, area: Rect) {
    let rows = scene.data.labels.len();
    let count = scene.data.series.len();
    let value = |s: usize, r: usize| {
        let v = scene.data.series[s].values[r];
        v.filter(|v| *v > 0.0).unwrap_or(0.0)
    };
    let totals: Vec<f64> = (0..rows)
        .map(|r| (0..count).map(|s| value(s, r)).sum())
        .collect();
    let most = totals.iter().copied().fold(0.0, f64::max);
    if most <= 0.0 {
        return;
    }
    let shares = scene.chart.kind == ChartKind::PercentArea;
    let stream = scene.chart.kind == ChartKind::Stream;
    // The edges of the areas, from the bottom up.
    let mut edges: Vec<Vec<f64>> = vec![Vec::new(); count + 1];
    for (r, total) in totals.iter().enumerate() {
        let k = match (shares, *total > 0.0) {
            (true, true) => 1.0 / total,
            (true, false) => 0.0,
            _ => 1.0,
        };
        let mut at = if stream { (most - total) / 2.0 } else { 0.0 };
        edges[0].push(at);
        for s in 0..count {
            at += value(s, r) * k;
            edges[s + 1].push(at);
        }
    }
    let names = labels(scene);
    let want = Want {
        to_ends: true,
        room: 0.0,
        // The height of a stream is not a value.
        axis: scene.chart.axis && !stream,
        percent: shares || scene.data.percent,
        ..of_values(scene, (0.0, if shares { 1.0 } else { most }), &names)
    };
    let plot = Plot::new(scene, area, &want);
    let points = |edge: &[f64]| -> Vec<Point> {
        let at = |(i, v): (usize, &f64)| Point::new(plot.end(i), plot.at(*v));
        let mut all: Vec<Point> = edge.iter().enumerate().map(at).collect();
        // A lone label takes the whole width.
        if let [only] = all[..] {
            all = vec![
                Point::new(plot.area.x0, only.y),
                Point::new(plot.area.x1, only.y),
            ];
        }
        all
    };
    let smooth = scene.chart.smooth || stream;
    for s in 0..count {
        let mut path = BezPath::new();
        through(&mut path, &points(&edges[s + 1]), smooth);
        let mut below = points(&edges[s]);
        below.reverse();
        through(&mut path, &below, smooth);
        path.close_path();
        let color = scene.color(s);
        scene.fill(path, color);
    }
}

/// A point for each row, at its x and its y; a bubble, as large as its
/// third value.
pub(crate) fn scatter(scene: &mut Scene<'_>, area: Rect) {
    if scene.data.series.len() < 2 {
        return;
    }
    let column = |s: usize| scene.data.series.get(s).map(|of| of.values.clone());
    let (Some(xs), Some(ys)) = (column(0), column(1)) else {
        return;
    };
    let sizes = column(2).filter(|_| scene.chart.kind == ChartKind::Bubbles);
    let side = area.width().min(area.height());
    let largest = match &sizes {
        Some(_) => side * 0.09,
        None => (scene.chart.weight * 1.6).min(side * 0.05),
    };
    let most = sizes
        .iter()
        .flatten()
        .flatten()
        .copied()
        .fold(0.0, f64::max);
    let area = area.inset(-largest);
    let x_range = range(xs.iter().copied().flatten(), false);
    let y_range = range(ys.iter().copied().flatten(), false);
    // The axis of x is written by a plot that lies on the other's.
    let none: [String; 0] = [];
    let (axis, gap) = (scene.chart.axis, scene.size * 0.4);
    let foot = if axis { scene.line() + gap } else { 0.0 };
    let foot = foot.min(area.height() * 0.4);
    let upper = Rect::new(area.x0, area.y0, area.x1, area.y1 - foot);
    let want = Want {
        room: 0.0,
        percent: false,
        ..of_values(scene, y_range, &none)
    };
    let down = Plot::new(scene, upper, &want);
    let want = Want {
        along_x: true,
        room: 0.0,
        percent: false,
        ..of_values(scene, x_range, &none)
    };
    let lower = Rect::new(down.area.x0, down.area.y0, down.area.x1, area.y1);
    let across = Plot::new(scene, lower, &want);
    let (_, group_of) = scene.groups();
    for (i, (x, y)) in xs.iter().zip(&ys).enumerate() {
        let (Some(x), Some(y)) = (x, y) else { continue };
        let at = Point::new(across.at(*x), down.at(*y));
        // With no words, the points are of one kind.
        let group = if scene.data.labels[i].is_empty() && scene.data.groups.is_empty() {
            0
        } else {
            group_of[i]
        };
        let radius = match &sizes {
            None => largest,
            Some(sizes) => {
                let v = sizes[i].filter(|v| *v > 0.0).unwrap_or(0.0);
                if v <= 0.0 || most <= 0.0 {
                    continue;
                }
                (v / most).sqrt() * largest
            }
        };
        let color = scene.color(group);
        let shape = if sizes.is_some() { 0 } else { group };
        scene.fill(marker(shape, at, radius), with_alpha(color, 0.8));
        if sizes.is_some()
            && let Some(text) = scene.number(2, i, 0.0)
        {
            scene.words(&text, at, Anchor::Middle, Anchor::Middle);
        }
    }
}

/// The values as points over their label, or their series when there are
/// several.
pub(crate) fn dots(scene: &mut Scene<'_>, area: Rect, along_x: bool) {
    let by_series = scene.data.series.len() > 1;
    let (groups, group_of) = scene.groups();
    let names: Vec<String> = if by_series {
        scene.data.series.iter().map(|s| s.name.clone()).collect()
    } else {
        groups
    };
    let names: Vec<String> = if scene.chart.names {
        names
    } else {
        vec![String::new(); names.len()]
    };
    let all = values(scene);
    let edge = (scene.chart.weight * 1.6).min(area.width().min(area.height()) * 0.05);
    let want = Want {
        along_x,
        room: 0.0,
        ..of_values(scene, range(all.into_iter(), false), &names)
    };
    let plot = Plot::new(scene, area.inset(-edge), &want);
    let radius = edge.min(plot.band() / 2.0);
    for s in 0..scene.data.series.len() {
        for (i, group) in group_of.iter().enumerate() {
            let Some(v) = scene.data.series[s].values[i] else {
                continue;
            };
            let band = if by_series { s } else { *group };
            let (along, across) = (plot.at(v), plot.middle(band));
            let at = if along_x {
                Point::new(along, across)
            } else {
                Point::new(across, along)
            };
            let color = with_alpha(scene.color(band), 0.75);
            scene.fill(marker(0, at, radius), color);
        }
    }
}
