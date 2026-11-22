//! Areas in proportion: circles, a tree map, a funnel.

use gafira_color::Color;
use kurbo::{BezPath, Circle, Point, Rect, Shape as _, Vec2};

use crate::paths::{TOLERANCE, marker};
use crate::round::positive;
use crate::scene::{Anchor, Scene, over, with_alpha};

/// Circles of `radii` beside one another around the origin, none over
/// another, after the front chain of Wang and others.
fn front_chain(radii: &[f64]) -> Vec<Circle> {
    let mut out: Vec<Circle> = radii.iter().map(|r| Circle::new((0.0, 0.0), *r)).collect();
    let n = out.len();
    if n < 2 {
        return out;
    }
    out[0].center.x = -out[1].radius;
    out[1].center.x = out[0].radius;
    if n == 2 {
        return out;
    }
    let third = beside(&out[1], &out[0], out[2].radius);
    out[2].center = third;
    let mut next = vec![0; n];
    let mut previous = vec![0; n];
    let mut link = |a: usize, b: usize| {
        next[a] = b;
        previous[b] = a;
    };
    link(0, 1);
    link(1, 2);
    link(2, 0);
    let (mut a, mut b) = (0, 1);
    let mut i = 3;
    // Each failed try shortens the chain, so there are few.
    let mut tries = 0;
    'place: while i < n && tries < n * 64 {
        tries += 1;
        let centre = beside(&out[a], &out[b], out[i].radius);
        out[i].center = centre;
        let (mut j, mut k) = (next[b], previous[a]);
        let (mut sj, mut sk) = (out[b].radius, out[a].radius);
        loop {
            if sj <= sk {
                if meet(&out[j], &out[i]) {
                    b = j;
                    next[a] = b;
                    previous[b] = a;
                    continue 'place;
                }
                sj += out[j].radius;
                j = next[j];
            } else {
                if meet(&out[k], &out[i]) {
                    a = k;
                    next[a] = b;
                    previous[b] = a;
                    continue 'place;
                }
                sk += out[k].radius;
                k = previous[k];
            }
            if j == next[k] {
                break;
            }
        }
        previous[i] = a;
        next[i] = b;
        next[a] = i;
        previous[b] = i;
        b = i;
        // The pair nearest the origin is next.
        let score = |of: usize| {
            let (p, q) = (&out[of], &out[next[of]]);
            let both = p.radius + q.radius;
            let x = (p.center.x * q.radius + q.center.x * p.radius) / both;
            let y = (p.center.y * q.radius + q.center.y * p.radius) / both;
            x * x + y * y
        };
        let mut best = score(a);
        let mut at = next[i];
        while at != b {
            let s = score(at);
            if s < best {
                a = at;
                best = s;
            }
            at = next[at];
        }
        b = next[a];
        i += 1;
    }
    out.truncate(i);
    out
}

/// The centre of a circle of `radius` that touches `b` and `a`.
fn beside(b: &Circle, a: &Circle, radius: f64) -> Point {
    let d = b.center - a.center;
    let d2 = d.hypot2();
    if d2 == 0.0 {
        return Point::new(a.center.x + radius, a.center.y);
    }
    let a2 = (a.radius + radius).powi(2);
    let b2 = (b.radius + radius).powi(2);
    if a2 > b2 {
        let x = (d2 + b2 - a2) / (2.0 * d2);
        let y = (b2 / d2 - x * x).max(0.0).sqrt();
        Point::new(
            b.center.x - x * d.x - y * d.y,
            b.center.y - x * d.y + y * d.x,
        )
    } else {
        let x = (d2 + a2 - b2) / (2.0 * d2);
        let y = (a2 / d2 - x * x).max(0.0).sqrt();
        Point::new(
            a.center.x + x * d.x - y * d.y,
            a.center.y + x * d.y + y * d.x,
        )
    }
}

fn meet(a: &Circle, b: &Circle) -> bool {
    let room = a.radius + b.radius - 1e-6;
    room > 0.0 && room * room > (b.center - a.center).hypot2()
}

/// A circle around all of `list`, near the smallest.
fn enclosing(list: &[Circle]) -> Circle {
    let Some(first) = list.first() else {
        return Circle::new((0.0, 0.0), 0.0);
    };
    let mut centre = first.center;
    let far = |centre: Point| {
        let reach = |c: &Circle| c.center.distance(centre) + c.radius;
        let found = list.iter().max_by(|a, b| reach(a).total_cmp(&reach(b)));
        found.map_or((centre, 0.0), |c| (c.center, reach(c)))
    };
    for step in 1..=400 {
        let (to, _) = far(centre);
        centre += (to - centre) / f64::from(step + 1);
    }
    Circle::new(centre, far(centre).1)
}

/// The circles of `values`, their areas in proportion; the place of each in
/// `values` with it.
fn packed(values: &[f64]) -> (Vec<(usize, Circle)>, Circle) {
    let most = values.iter().copied().fold(0.0, f64::max);
    let mut order: Vec<usize> = (0..values.len()).filter(|i| values[*i] > 0.0).collect();
    order.sort_by(|a, b| values[*b].total_cmp(&values[*a]));
    let radii: Vec<f64> = order
        .iter()
        .map(|i| (values[*i] / most).sqrt().max(0.01))
        .collect();
    let circles = front_chain(&radii);
    let around = enclosing(&circles);
    (order.into_iter().zip(circles).collect(), around)
}

/// Words in the middle of a room over `color`, the number under the label.
fn caption(scene: &mut Scene<'_>, row: usize, room: Rect, color: Color, whole: f64) {
    let name = scene.label(row).to_owned();
    let number = scene.number(0, row, whole).unwrap_or_default();
    let shown: Vec<(String, f64)> = [name, number]
        .iter()
        .filter(|t| !t.trim().is_empty())
        .filter_map(|t| scene.squeezed(t, room.width()))
        .collect();
    let line = scene.line();
    // At most two lines: the casts are exact.
    let fit = ((room.height() / line).max(0.0) as usize).min(shown.len());
    let top = room.center().y - line * fit as f64 / 2.0;
    for (i, (text, size)) in shown[..fit].iter().enumerate() {
        let at = Point::new(room.center().x, top + line * i as f64);
        let anchor = (Anchor::Middle, Anchor::Start);
        scene.sized(text, at, anchor, over(color), *size);
    }
}

/// The room for words in a circle.
fn within(circle: Circle) -> Rect {
    Rect::from_center_size(circle.center, (circle.radius * 1.7, circle.radius * 1.1))
}

/// From the circles of a drawing to as much of `area` as they fill.
fn to_area(circles: &[Circle], area: Rect) -> impl Fn(Circle) -> Circle + use<> {
    let of = |c: &Circle| Rect::from_center_size(c.center, (c.radius * 2.0, c.radius * 2.0));
    let bounds = circles
        .iter()
        .map(of)
        .reduce(|a, b| a.union(b))
        .unwrap_or(area);
    let k =
        (area.width() / bounds.width().max(1e-9)).min(area.height() / bounds.height().max(1e-9));
    move |c| {
        let centre = area.center() + (c.center - bounds.center()) * k;
        Circle::new(centre, c.radius * k)
    }
}

pub(crate) fn pack(scene: &mut Scene<'_>, area: Rect) {
    let (values, whole) = positive(scene);
    if whole <= 0.0 {
        return;
    }
    let (circles, _) = packed(&values);
    let place = to_area(&circles.iter().map(|(_, c)| *c).collect::<Vec<_>>(), area);
    for (row, circle) in circles {
        let circle = place(circle);
        let color = scene.color(row);
        scene.fill(marker(0, circle.center, circle.radius), color);
        caption(scene, row, within(circle), color, whole);
    }
}

/// The circles of each group inside one of their own.
pub(crate) fn nested(scene: &mut Scene<'_>, area: Rect) {
    let (values, whole) = positive(scene);
    if whole <= 0.0 {
        return;
    }
    if scene.data.groups.is_empty() {
        pack(scene, area);
        return;
    }
    let (names, group_of) = scene.groups();
    let most = values.iter().copied().fold(0.0, f64::max);
    let mut groups = Vec::new();
    for g in 0..names.len() {
        let rows: Vec<usize> = (0..values.len()).filter(|r| group_of[*r] == g).collect();
        let own: Vec<f64> = rows.iter().map(|r| values[*r]).collect();
        let largest = own.iter().copied().fold(0.0, f64::max);
        if largest <= 0.0 {
            continue;
        }
        let (circles, around) = packed(&own);
        // The same scale in every group.
        let k = (largest / most).sqrt();
        let circles: Vec<(usize, Circle)> = circles
            .into_iter()
            .map(|(i, c)| {
                let centre = ((c.center - around.center) * k).to_point();
                (rows[i], Circle::new(centre, c.radius * k))
            })
            .collect();
        groups.push((g, circles, around.radius * k * 1.12));
    }
    groups.sort_by(|a, b| b.2.total_cmp(&a.2));
    let radii: Vec<f64> = groups.iter().map(|g| g.2).collect();
    let outer = front_chain(&radii);
    let place = to_area(&outer, area);
    for ((g, circles, _), circle) in groups.into_iter().zip(outer) {
        let color = scene.color(g);
        let shown = place(circle);
        let path = marker(0, shown.center, shown.radius);
        scene.fill(path, with_alpha(color, 0.25));
        for (row, inner) in circles {
            let moved = Circle::new(circle.center + inner.center.to_vec2(), inner.radius);
            let inner = place(moved);
            scene.fill(marker(0, inner.center, inner.radius * 0.96), color);
            caption(scene, row, within(inner), color, whole);
        }
    }
}

/// Rectangles of `areas` (that add up to the one of `room`), as square as
/// can be, after Bruls, Huizing and van Wijk.
fn squares(areas: &[f64], room: Rect) -> Vec<Rect> {
    let mut out = Vec::with_capacity(areas.len());
    let (mut left, mut at) = (room, 0);
    while at < areas.len() {
        let side = left.width().min(left.height()).max(1e-9);
        let worst = |row: &[f64]| {
            let sum: f64 = row.iter().sum();
            let thick = sum / side;
            row.iter()
                .map(|a| {
                    let long = a / thick;
                    (long / thick).max(thick / long)
                })
                .fold(0.0, f64::max)
        };
        let mut end = at + 1;
        while end < areas.len() && worst(&areas[at..=end]) <= worst(&areas[at..end]) {
            end += 1;
        }
        let row = &areas[at..end];
        let sum: f64 = row.iter().sum();
        let thick = sum / side;
        let mut along = 0.0;
        for a in row {
            let long = a / thick;
            let rect = if left.width() >= left.height() {
                Rect::new(
                    left.x0,
                    left.y0 + along,
                    left.x0 + thick,
                    left.y0 + along + long,
                )
            } else {
                Rect::new(
                    left.x0 + along,
                    left.y0,
                    left.x0 + along + long,
                    left.y0 + thick,
                )
            };
            out.push(rect);
            along += long;
        }
        if left.width() >= left.height() {
            left.x0 = (left.x0 + thick).min(left.x1);
        } else {
            left.y0 = (left.y0 + thick).min(left.y1);
        }
        at = end;
    }
    out
}

pub(crate) fn treemap(scene: &mut Scene<'_>, area: Rect) {
    let (values, whole) = positive(scene);
    if whole <= 0.0 {
        return;
    }
    let mut order: Vec<usize> = (0..values.len())
        .filter(|i| values[*i] / whole > 1e-6)
        .collect();
    order.sort_by(|a, b| values[*b].total_cmp(&values[*a]));
    let shown: f64 = order.iter().map(|i| values[*i]).sum();
    let areas: Vec<f64> = order
        .iter()
        .map(|i| values[*i] / shown * area.area())
        .collect();
    for (row, rect) in order.iter().zip(squares(&areas, area)) {
        let rect = rect.intersect(area);
        let gap = rect.width().min(rect.height()).min(6.0) / 6.0;
        let rect = rect.inset(-gap);
        let side = rect.width().min(rect.height()).max(0.0);
        let radius = scene.chart.round * side * 0.15;
        let color = scene.color(*row);
        scene.fill(rect.to_rounded_rect(radius).to_path(TOLERANCE), color);
        let room = rect.inset(-scene.size * 0.3);
        if room.width() > 0.0 && room.height() > 0.0 {
            caption(scene, *row, room, color, whole);
        }
    }
}

/// Steps as wide as their values, one under the other.
pub(crate) fn funnel(scene: &mut Scene<'_>, area: Rect) {
    let (values, whole) = positive(scene);
    let most = values.iter().copied().fold(0.0, f64::max);
    let rows: Vec<usize> = (0..values.len()).filter(|r| values[*r] > 0.0).collect();
    if rows.is_empty() || whole <= 0.0 {
        return;
    }
    // Few rows: the casts are exact.
    let tall = area.height() / rows.len() as f64;
    let gap = 1.5_f64.min(tall * 0.1);
    for (i, row) in rows.iter().enumerate() {
        let half = |v: f64| v / most * area.width() / 2.0;
        let top = half(values[*row]);
        let bottom = match rows.get(i + 1) {
            Some(next) => half(values[*next]),
            None => top * 0.7,
        };
        let y = area.y0 + tall * i as f64;
        let centre = area.center().x;
        let mut path = BezPath::new();
        path.move_to((centre - top, y));
        path.line_to((centre + top, y));
        path.line_to((centre + bottom, y + tall - gap));
        path.line_to((centre - bottom, y + tall - gap));
        path.close_path();
        let color = scene.color(*row);
        scene.fill(path, color);
        let wide = (top + bottom).min(area.width()) - scene.size * 0.5;
        let room = Rect::from_center_size(
            Point::new(centre, y + (tall - gap) / 2.0),
            (wide.max(0.0), tall - gap),
        );
        let label = scene.label(*row).to_owned();
        let number = scene.number(0, *row, most).unwrap_or_default();
        let text = format!("{label} {number}");
        let text = text.trim();
        if scene.width(text) <= room.width() && scene.line() <= room.height() {
            let anchor = (Anchor::Middle, Anchor::Middle);
            scene.words_in(text, room.center(), anchor, over(color));
        } else if scene.line() <= tall {
            // Beside the step, where there is room.
            let x = centre + top.max(bottom) + scene.size * 0.4;
            if let Some(text) = scene.fit(text, area.x1 - x) {
                let at = Point::new(x, room.center().y) + Vec2::ZERO;
                scene.words(&text, at, Anchor::Start, Anchor::Middle);
            }
        }
    }
}
