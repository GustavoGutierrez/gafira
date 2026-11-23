//! Pies, rings, gauges and radars.

use gafira_doc::ChartKind;
use kurbo::{BezPath, Point, Rect};

use crate::bars::{share, written};
use crate::data::plain;
use crate::paths::{marker, polar, slice, stroke, through};
use crate::scene::{Anchor, Scene, lighter, over, with_alpha};

/// The positive values of the first series, zero in place of the others, and
/// their sum.
pub(crate) fn positive(scene: &Scene<'_>) -> (Vec<f64>, f64) {
    let values: Vec<f64> = scene.data.series[0]
        .values
        .iter()
        .map(|v| v.filter(|v| *v > 0.0).unwrap_or(0.0))
        .collect();
    let whole = values.iter().sum();
    (values, whole)
}

/// Where words lie around a centre, at `turns` from the top.
fn around(turns: f64) -> (Anchor, Anchor) {
    let (sin, cos) = (turns * std::f64::consts::TAU).sin_cos();
    let h = match sin {
        s if s > 0.2 => Anchor::Start,
        s if s < -0.2 => Anchor::End,
        _ => Anchor::Middle,
    };
    let v = match cos {
        c if c > 0.75 => Anchor::End,
        c if c < -0.75 => Anchor::Start,
        _ => Anchor::Middle,
    };
    (h, v)
}

/// The box lines of words take.
fn taken(at: Point, (h, v): (Anchor, Anchor), wide: f64, tall: f64) -> Rect {
    let x = match h {
        Anchor::Start => at.x,
        Anchor::Middle => at.x - wide / 2.0,
        Anchor::End => at.x - wide,
    };
    let y = match v {
        Anchor::Start => at.y,
        Anchor::Middle => at.y - tall / 2.0,
        Anchor::End => at.y - tall,
    };
    Rect::new(x, y, x + wide, y + tall)
}

pub(crate) fn pie(scene: &mut Scene<'_>, area: Rect) {
    let (values, whole) = positive(scene);
    if whole <= 0.0 {
        return;
    }
    // The label of each slice, and its number under it.
    let lines: Vec<Vec<String>> = (0..values.len())
        .map(|r| {
            let name = scene.label(r).to_owned();
            let number = scene.number(0, r, whole).unwrap_or_default();
            [name, number]
                .into_iter()
                .filter(|t| !t.trim().is_empty())
                .collect()
        })
        .collect();
    let widest = lines
        .iter()
        .flatten()
        .map(|t| scene.width(t))
        .fold(0.0, f64::max);
    // At most two lines: the cast is exact.
    let tall = lines.iter().map(Vec::len).max().unwrap_or(0) as f64 * scene.line();
    let gap = if tall > 0.0 { scene.size * 0.6 } else { 0.0 };
    let least = area.width().min(area.height()) * 0.25;
    let side = area.width() / 2.0 - widest - gap;
    let up = area.height() / 2.0 - tall - gap;
    let radius = side.min(up).max(least);
    let centre = area.center();
    let inner = if scene.chart.kind == ChartKind::Donut {
        radius * scene.chart.hole
    } else {
        0.0
    };
    let mut start = 0.0;
    let mut spots = Vec::new();
    for (r, v) in values.iter().enumerate() {
        let turns = v / whole;
        if turns > 0.0 {
            let color = scene.color(r);
            scene.fill(slice(centre, radius, inner, start, turns), color);
            spots.push((turns, start + turns / 2.0, r));
        }
        start += turns;
    }
    // The larger slices are named first; a name over another is left out.
    spots.sort_by(|a, b| b.0.total_cmp(&a.0));
    let mut placed: Vec<Rect> = Vec::new();
    let room = (area.width() / 2.0 - radius - gap).max(0.0);
    for (_, turns, r) in spots {
        let shown: Vec<String> = lines[r]
            .iter()
            .filter_map(|t| scene.fit(t, room.max(scene.size * 2.0)))
            .collect();
        if shown.is_empty() {
            continue;
        }
        let at = polar(centre, radius + gap, turns);
        let anchor = around(turns);
        let wide = shown.iter().map(|t| scene.width(t)).fold(0.0, f64::max);
        let tall = scene.line() * shown.len() as f64;
        let spot = taken(at, anchor, wide, tall);
        if placed.iter().any(|other| other.overlaps(spot)) {
            continue;
        }
        placed.push(spot);
        for (i, text) in shown.iter().enumerate() {
            // Both lines share the middle of their box.
            let at = Point::new(spot.center().x, spot.y0 + scene.line() * i as f64);
            scene.words(text, at, Anchor::Middle, Anchor::Start);
        }
    }
}

/// The groups in a ring and their rows in another around it.
pub(crate) fn sunburst(scene: &mut Scene<'_>, area: Rect) {
    let (values, whole) = positive(scene);
    if whole <= 0.0 {
        return;
    }
    let (names, group_of) = scene.groups();
    let nested = !scene.data.groups.is_empty();
    let centre = area.center();
    let radius = area.width().min(area.height()) / 2.0;
    let hole = radius * scene.chart.hole * 0.5;
    let middle = if nested {
        f64::midpoint(hole, radius)
    } else {
        radius
    };
    let mut start = 0.0;
    for (g, name) in names.iter().enumerate() {
        let rows: Vec<usize> = (0..values.len()).filter(|r| group_of[*r] == g).collect();
        let sum: f64 = rows.iter().map(|r| values[*r]).sum();
        let turns = sum / whole;
        if turns <= 0.0 {
            continue;
        }
        let color = scene.color(g);
        scene.fill(slice(centre, middle, hole, start, turns), color);
        if scene.chart.names {
            inside(scene, name, (centre, hole, middle), start, turns, color);
        }
        let mut at = start;
        for (i, r) in rows.iter().enumerate().filter(|_| nested) {
            let turns = values[*r] / whole;
            // Each row a little lighter than the one before.
            let light = 0.25 + 0.4 * (i as f32 / rows.len().max(1) as f32);
            let tone = lighter(color, light);
            if turns > 0.0 {
                scene.fill(slice(centre, radius, middle + 1.0, at, turns), tone);
                let name = scene.label(*r).to_owned();
                inside(scene, &name, (centre, middle, radius), at, turns, tone);
            }
            at += turns;
        }
        start += turns;
    }
}

/// Words in a part of a ring, when they fit in it.
fn inside(
    scene: &mut Scene<'_>,
    text: &str,
    (centre, inner, outer): (Point, f64, f64),
    start: f64,
    turns: f64,
    color: gafira_color::Color,
) {
    let middle = f64::midpoint(inner, outer);
    let arc = turns * std::f64::consts::TAU * middle;
    let room = (outer - inner).min(arc.max(0.0) * 1.5);
    if arc < scene.line() || outer - inner < scene.line() {
        return;
    }
    if let Some((text, size)) = scene.squeezed(text, room) {
        let at = polar(centre, middle, start + turns / 2.0);
        let anchor = (Anchor::Middle, Anchor::Middle);
        scene.sized(&text, at, anchor, over(color), size);
    }
}

/// The word and the number in the middle of a ring.
fn middle(scene: &mut Scene<'_>, at: Point, room: f64, row: usize, done: f64) {
    let number = written(scene, row, done);
    let name = scene.label(row).to_owned();
    let color = scene.chart.text.color;
    let mut y = at.y;
    if let Some(text) = number {
        // As large as the hole takes, up to three times the words.
        let wide = scene.width(&text).max(0.1);
        let k = (room * 0.9 / wide).clamp(0.2, 3.0);
        let named = !name.trim().is_empty();
        let size = scene.size * k;
        let v = if named { Anchor::End } else { Anchor::Middle };
        y += if named { size * 0.3 } else { 0.0 };
        scene.sized(&text, Point::new(at.x, y), (Anchor::Middle, v), color, size);
    }
    if let Some(text) = scene.fit(&name, room) {
        let v = if y > at.y {
            Anchor::Start
        } else {
            Anchor::Middle
        };
        scene.words(&text, Point::new(at.x, y), Anchor::Middle, v);
    }
}

/// A ring, or half of one, filled as far as it goes.
pub(crate) fn progress(scene: &mut Scene<'_>, area: Rect, whole: bool) {
    let (radius, centre, start, turns) = if whole {
        let radius = area.width().min(area.height()) / 2.0;
        (radius, area.center(), 0.0, 1.0)
    } else {
        let radius = (area.width() / 2.0).min(area.height());
        let centre = Point::new(area.center().x, area.center().y + radius / 2.0);
        (radius, centre, -0.25, 0.5)
    };
    let thick = radius * (1.0 - scene.chart.hole).max(0.02);
    let round = scene.chart.round > 0.0;
    let way = radius - thick / 2.0;
    let ring = |from: f64, turns: f64| -> BezPath {
        if !round || turns >= 1.0 {
            return slice(centre, radius, radius - thick, from, turns);
        }
        // The round ends stay within the turns.
        let cap = thick / 2.0 / way / std::f64::consts::TAU;
        let (a, b) = if whole {
            (from, from + turns)
        } else {
            (from + cap, (from + turns - cap).max(from + cap))
        };
        let steps = 64;
        let points: Vec<Point> = (0..=steps)
            .map(|i| polar(centre, way, a + (b - a) * f64::from(i) / f64::from(steps)))
            .collect();
        let mut path = BezPath::new();
        through(&mut path, &points, false);
        stroke(&path, thick)
    };
    let done = share(scene, 0);
    let color = scene.color(0);
    scene.fill(ring(start, turns), with_alpha(color, 0.2));
    if done > 0.0 {
        scene.fill(ring(start, turns * done), color);
    }
    let room = (radius - thick) * 1.6;
    let at = if whole {
        centre
    } else {
        Point::new(centre.x, centre.y - scene.size * 1.2)
    };
    middle(scene, at, room, 0, done);
}

/// A scale in colours and a needle.
pub(crate) fn gauge(scene: &mut Scene<'_>, area: Rect) {
    let foot = scene.line() * 2.2;
    let foot = foot.min(area.height() * 0.4);
    let radius = (area.width() / 2.0).min(area.height() - foot).max(0.5);
    let top = area.center().y - (radius + foot) / 2.0;
    let centre = Point::new(area.center().x, top + radius);
    let inner = radius * scene.chart.hole;
    let zones = scene.chart.colors.len().min(5);
    let each = 0.5 / zones.max(1) as f64;
    for i in 0..zones {
        let color = scene.color(i);
        let start = -0.25 + each * i as f64;
        scene.fill(slice(centre, radius, inner, start, each * 0.985), color);
    }
    let done = share(scene, 0);
    let turns = -0.25 + 0.5 * done;
    let ink = scene.chart.text.color;
    let base = (radius * 0.045).max(0.5);
    let mut needle = BezPath::new();
    needle.move_to(polar(centre, radius * 0.92, turns));
    needle.line_to(polar(centre, base, turns + 0.25));
    needle.line_to(polar(centre, base, turns - 0.25));
    needle.close_path();
    scene.fill(needle, ink);
    scene.fill(marker(0, centre, base * 1.6), ink);
    let name = scene.label(0).to_owned();
    let number = written(scene, 0, done).unwrap_or_default();
    let text = format!("{name} {number}");
    if let Some(text) = scene.fit(text.trim(), area.width()) {
        let at = Point::new(centre.x, centre.y + base * 1.6 + scene.size * 0.4);
        scene.words(&text, at, Anchor::Middle, Anchor::Start);
    }
}

pub(crate) fn radar(scene: &mut Scene<'_>, area: Rect) {
    let sides = scene.data.labels.len();
    let all = scene.data.series.iter().flat_map(|s| &s.values);
    let most = all.copied().flatten().fold(0.0, f64::max);
    if sides < 3 || most <= 0.0 {
        return;
    }
    let names: Vec<String> = (0..sides).map(|r| scene.label(r).to_owned()).collect();
    let named = names.iter().any(|n| !n.trim().is_empty());
    let (tall, gap) = if named {
        (scene.line(), scene.size * 0.5)
    } else {
        (0.0, 0.0)
    };
    let widest = names.iter().map(|t| scene.width(t)).fold(0.0, f64::max);
    let widest = widest.min(area.width() * 0.25);
    let radius = (area.width() / 2.0 - widest - gap)
        .min(area.height() / 2.0 - tall - gap)
        .max(area.width().min(area.height()) * 0.2);
    let centre = area.center();
    let marks = crate::scale::ticks(0.0, most, 4);
    let top = marks.last().copied().unwrap_or(most);
    // Few sides: the casts are exact.
    let turn = |i: usize| i as f64 / sides as f64;
    let outline = |of: &dyn Fn(usize) -> f64| {
        let points: Vec<Point> = (0..sides)
            .map(|i| polar(centre, radius * of(i).clamp(0.0, 1.0), turn(i)))
            .collect();
        let mut path = BezPath::new();
        through(&mut path, &points, false);
        path.close_path();
        path
    };
    let thin = (scene.size / 14.0).clamp(0.5, 2.0);
    let faint = scene.faint();
    if scene.chart.grid {
        for mark in marks.iter().filter(|m| **m > 0.0) {
            let path = outline(&|_| mark / top);
            scene.fill(stroke(&path, thin), faint);
        }
        for i in 0..sides {
            let mut path = BezPath::new();
            path.move_to(centre);
            path.line_to(polar(centre, radius, turn(i)));
            scene.fill(stroke(&path, thin), faint);
        }
    }
    let weight = scene.chart.weight.min(radius * 0.1);
    for s in 0..scene.data.series.len() {
        let values = scene.data.series[s].values.clone();
        let path = outline(&|i| values[i].unwrap_or(0.0) / top);
        let color = scene.color(s);
        scene.fill(path.clone(), with_alpha(color, 0.2));
        scene.fill(stroke(&path, weight), color);
        if scene.chart.markers {
            for (i, v) in values.iter().enumerate() {
                let share = (v.unwrap_or(0.0) / top).clamp(0.0, 1.0);
                let at = polar(centre, radius * share, turn(i));
                scene.fill(marker(s, at, weight * 1.3), color);
            }
        }
    }
    for (i, name) in names.iter().enumerate() {
        if let Some(text) = scene.fit(name, widest.max(scene.size * 2.0)) {
            let at = polar(centre, radius + gap, turn(i));
            let (h, v) = around(turn(i));
            scene.words(&text, at, h, v);
        }
    }
    if scene.chart.axis {
        for mark in &marks {
            let at = polar(centre, radius * mark / top, 0.0);
            let text = if scene.data.percent {
                plain(*mark, true, scene.data.comma)
            } else {
                crate::scale::short(*mark, top, scene.data.comma)
            };
            let at = Point::new(at.x + scene.size * 0.3, at.y);
            scene.words(&text, at, Anchor::Start, Anchor::Middle);
        }
    }
}
