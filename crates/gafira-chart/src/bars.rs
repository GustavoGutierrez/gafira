//! Bars, up or across.

use kurbo::{Point, Rect, Shape as _};

use crate::axes::{Plot, Want};
use crate::data::{Colored, colored};
use crate::paths::{End, TOLERANCE, bar};
use crate::scale::ticks;
use crate::scene::{Anchor, Scene, over, with_alpha};

/// The gap between bars that touch, in px.
const BETWEEN: f64 = 1.5;

/// The least and the most of `values`, zero among them.
fn range(values: impl Iterator<Item = f64>) -> (f64, f64) {
    let (min, max) = values.fold((0.0_f64, 0.0_f64), |(a, b), v| (a.min(v), b.max(v)));
    if min == max { (0.0, 1.0) } else { (min, max) }
}

/// The labels of the rows, empty when they are not written.
fn labels(scene: &Scene<'_>) -> Vec<String> {
    (0..scene.data.labels.len())
        .map(|i| scene.label(i).to_owned())
        .collect()
}

/// The room the numbers past the bars take.
fn room(scene: &Scene<'_>, along_x: bool, whole: f64) -> f64 {
    let written: Vec<f64> = (0..scene.data.series.len())
        .flat_map(|s| (0..scene.data.labels.len()).map(move |r| (s, r)))
        .filter_map(|(s, r)| scene.number(s, r, whole))
        .map(|text| scene.width(&text))
        .collect();
    let gap = scene.size * 0.4;
    match written.iter().copied().reduce(f64::max) {
        None => 0.0,
        Some(wide) if along_x => wide + gap,
        Some(_) => scene.line() + gap,
    }
}

/// The bar from `a` to `b` along the values, from `start` across, `thick`.
fn rect(plot: &Plot, a: f64, b: f64, start: f64, thick: f64) -> Rect {
    if plot.along_x {
        Rect::new(a.min(b), start, a.max(b), start + thick)
    } else {
        Rect::new(start, a.min(b), start + thick, a.max(b))
    }
}

fn end(plot: &Plot, positive: bool) -> End {
    match (plot.along_x, positive) {
        (true, true) => End::Right,
        (true, false) => End::Left,
        (false, true) => End::Top,
        (false, false) => End::Bottom,
    }
}

/// The bars of each label side by side.
pub(crate) fn grouped(scene: &mut Scene<'_>, area: Rect, along_x: bool) {
    let all = scene.data.series.iter().flat_map(|s| &s.values);
    let (min, max) = range(all.copied().flatten());
    let positive = |v: &Option<f64>| v.filter(|v| *v > 0.0).unwrap_or(0.0);
    let values = scene.data.series.iter().flat_map(|s| &s.values);
    let whole: f64 = values.map(positive).sum();
    let names = labels(scene);
    let want = Want {
        along_x,
        min,
        max,
        labels: &names,
        to_ends: false,
        room: room(scene, along_x, whole),
        axis: scene.chart.axis,
        percent: scene.data.percent,
    };
    let plot = Plot::new(scene, area, &want);
    let count = scene.data.series.len();
    let band = plot.band();
    let inner = band * (1.0 - scene.chart.gap);
    // Few series: the cast is exact.
    let each = inner / count as f64;
    let between = if count > 1 {
        BETWEEN.min(each * 0.1)
    } else {
        0.0
    };
    let by_row = colored(scene.chart) == Colored::Rows;
    let gap = scene.size * 0.3;
    for r in 0..names.len() {
        for s in 0..count {
            let Some(v) = scene.data.series[s].values[r] else {
                continue;
            };
            let start = plot.middle(r) - inner / 2.0 + each * s as f64 + between / 2.0;
            let tip = plot.at(v);
            let shape = rect(&plot, plot.zero(), tip, start, each - between);
            let color = scene.color(if by_row { r } else { s });
            if shape.area() > 0.0 {
                let path = bar(shape, end(&plot, v >= 0.0), scene.chart.round);
                scene.fill(path, color);
            }
            let Some(text) = scene.number(s, r, whole) else {
                continue;
            };
            let across = start + (each - between) / 2.0;
            if along_x {
                let at = Point::new(shape.x1 + gap, across);
                if scene.line() <= each * 1.6 {
                    scene.words(&text, at, Anchor::Start, Anchor::Middle);
                }
            } else if scene.width(&text) <= band / count as f64 {
                let (at, v) = if v >= 0.0 {
                    (Point::new(across, shape.y0 - gap), Anchor::End)
                } else {
                    (Point::new(across, shape.y1 + gap), Anchor::Start)
                };
                scene.words(&text, at, Anchor::Middle, v);
            }
        }
    }
}

/// The bars of each label one over the other, the negative ones below zero.
pub(crate) fn stacked(scene: &mut Scene<'_>, area: Rect, along_x: bool) {
    let rows = scene.data.labels.len();
    let sum = |r: usize, positive: bool| -> f64 {
        let values = scene.data.series.iter().filter_map(|s| s.values[r]);
        values.filter(|v| (*v > 0.0) == positive).sum()
    };
    let (min, max) = range((0..rows).flat_map(|r| [sum(r, true), sum(r, false)]));
    let totals: Vec<f64> = (0..rows).map(|r| sum(r, true)).collect();
    let names = labels(scene);
    let want = Want {
        along_x,
        min,
        max,
        labels: &names,
        to_ends: false,
        room: 0.0,
        axis: scene.chart.axis,
        percent: scene.data.percent,
    };
    let plot = Plot::new(scene, area, &want);
    let thick = plot.band() * (1.0 - scene.chart.gap);
    let count = scene.data.series.len();
    for (r, total) in totals.iter().enumerate() {
        let value = |scene: &Scene<'_>, s: usize| scene.data.series[s].values[r].unwrap_or(0.0);
        let (mut up, mut down) = (0.0, 0.0);
        for s in 0..count {
            let v = value(scene, s);
            if v == 0.0 {
                continue;
            }
            let base = if v > 0.0 { &mut up } else { &mut down };
            let first = *base == 0.0;
            let (a, b) = (plot.at(*base), plot.at(*base + v));
            *base += v;
            // The gap is taken from the side the one before touches.
            let cut = if first {
                0.0
            } else {
                BETWEEN.min((b - a).abs() * 0.5)
            };
            let a = a + cut * (b - a).signum();
            let is_last = (s + 1..count).all(|s| {
                let next = value(scene, s);
                next == 0.0 || (next > 0.0) != (v > 0.0)
            });
            let start = plot.middle(r) - thick / 2.0;
            let shape = rect(&plot, a, b, start, thick);
            if shape.area() <= 0.0 {
                continue;
            }
            let tip = if is_last {
                end(&plot, v > 0.0)
            } else {
                End::None
            };
            let color = scene.color(s);
            scene.fill(bar(shape, tip, scene.chart.round), color);
            if let Some(text) = scene.number(s, r, *total)
                && scene.width(&text) <= shape.width()
                && scene.line() <= shape.height()
            {
                let middle = (Anchor::Middle, Anchor::Middle);
                scene.words_in(&text, shape.center(), middle, over(color));
            }
        }
    }
}

/// How many values fall in each of a few equal steps.
pub(crate) fn histogram(scene: &mut Scene<'_>, area: Rect) {
    let values: Vec<f64> = scene.data.series[0]
        .values
        .iter()
        .flatten()
        .copied()
        .collect();
    if values.is_empty() {
        return;
    }
    let min = values.iter().copied().fold(f64::INFINITY, f64::min);
    let max = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    // At most 201 values: the casts are exact.
    let steps = ((values.len() as f64).log2().ceil() + 1.0).clamp(1.0, 20.0) as usize;
    let edges = ticks(min, max, steps);
    let bins = edges.len().saturating_sub(1).max(1);
    let mut counts = vec![0_usize; bins];
    for v in values {
        let bin = edges[1..].iter().position(|edge| v < *edge);
        counts[bin.unwrap_or(bins - 1)] += 1;
    }
    let most = edges.iter().fold(0.0_f64, |m, v| m.max(v.abs()));
    let names: Vec<String> = if scene.chart.names {
        edges
            .windows(2)
            .map(|e| {
                let comma = scene.data.comma;
                let [a, b] = [e[0], e[1]].map(|v| crate::scale::short(v, most, comma));
                format!("{a} – {b}")
            })
            .collect()
    } else {
        vec![String::new(); bins]
    };
    let tallest = counts.iter().copied().max().unwrap_or(1) as f64;
    let numbers = scene.chart.numbers != gafira_doc::ChartLabels::None;
    let want = Want {
        along_x: false,
        min: 0.0,
        max: tallest,
        labels: &names,
        to_ends: false,
        room: if numbers { scene.line() * 1.4 } else { 0.0 },
        axis: scene.chart.axis,
        percent: false,
    };
    let plot = Plot::new(scene, area, &want);
    let thick = plot.band() * (1.0 - scene.chart.gap);
    let color = scene.color(0);
    for (i, count) in counts.iter().enumerate().filter(|(_, n)| **n > 0) {
        let start = plot.middle(i) - thick / 2.0;
        let shape = rect(&plot, plot.zero(), plot.at(*count as f64), start, thick);
        if shape.area() > 0.0 {
            scene.fill(bar(shape, End::Top, scene.chart.round), color);
        }
        if numbers {
            let at = Point::new(plot.middle(i), shape.y0 - scene.size * 0.3);
            scene.words(&count.to_string(), at, Anchor::Middle, Anchor::End);
        }
    }
}

/// A bar for each label, filled as far as it goes.
pub(crate) fn progress(scene: &mut Scene<'_>, area: Rect) {
    let rows = scene.data.labels.len();
    // Few rows: the cast is exact.
    let band = area.height() / rows as f64;
    let named = (0..rows).any(|r| !scene.label(r).trim().is_empty());
    let numbers = scene.chart.numbers != gafira_doc::ChartLabels::None;
    let head = if named || numbers {
        (scene.line() * 1.3).min(band * 0.5)
    } else {
        0.0
    };
    let by_row = colored(scene.chart) == Colored::Rows;
    for r in 0..rows {
        let top = area.y0 + band * r as f64;
        let room = band * (1.0 - scene.chart.gap * 0.5) - head;
        let tall = room.min(area.width() * 0.2).max(0.0);
        let middle = top + band / 2.0;
        let track = Rect::new(
            area.x0,
            middle - (tall - head) / 2.0,
            area.x1,
            middle + (tall + head) / 2.0,
        );
        let radius = scene.chart.round * tall / 2.0;
        let color = scene.color(if by_row { r } else { 0 });
        let rounded = |rect: Rect| rect.to_rounded_rect(radius).to_path(TOLERANCE);
        scene.fill(rounded(track), with_alpha(color, 0.2));
        let share = share(scene, r);
        if share > 0.0 {
            let mut done = track;
            done.x1 = track.x0 + (track.width() * share).max((radius * 2.0).min(track.width()));
            scene.fill(rounded(done), color);
        }
        if head <= 0.0 {
            continue;
        }
        let y = track.y0 - scene.size * 0.2;
        let number = written(scene, r, share);
        let wide = number.as_ref().map_or(0.0, |t| scene.width(t) + scene.size);
        if let Some(text) = scene.fit(scene.label(r), area.width() - wide) {
            scene.words(&text, Point::new(area.x0, y), Anchor::Start, Anchor::End);
        }
        if let Some(text) = number {
            scene.words(&text, Point::new(area.x1, y), Anchor::End, Anchor::End);
        }
    }
}

/// How far the value of `row` has gone, 0 to 1: of its total, if there is a
/// second column, or of a hundred.
pub(crate) fn share(scene: &Scene<'_>, row: usize) -> f64 {
    let value = |s: usize| {
        let series = scene.data.series.get(s)?;
        series.values.get(row).copied().flatten()
    };
    let Some(v) = value(0) else { return 0.0 };
    let percent = scene.data.series[0].words[row].ends_with('%');
    let share = match value(1).filter(|total| *total > 0.0) {
        Some(total) => v / total,
        None if percent || (v > 0.0 && v < 1.0) => v,
        None => v / 100.0,
    };
    if share.is_finite() {
        share.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

/// The number of a progress, if it is written.
pub(crate) fn written(scene: &Scene<'_>, row: usize, share: f64) -> Option<String> {
    match scene.chart.numbers {
        gafira_doc::ChartLabels::Percent => Some(scene.share(share)),
        _ => scene.number(0, row, 0.0),
    }
}
