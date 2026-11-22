//! The plot of the charts with axes (S83-04).

use kurbo::{Point, Rect, Shape as _};

use crate::scale::{short, ticks};
use crate::scene::{Anchor, Scene};

/// The room the values take in the plot.
pub(crate) struct Plot {
    pub area: Rect,
    /// Whether the values go along x.
    pub along_x: bool,
    min: f64,
    max: f64,
    /// How many labels share the other side.
    pub count: usize,
}

/// What the plot writes.
pub(crate) struct Want<'a> {
    pub along_x: bool,
    /// The values the axis covers.
    pub min: f64,
    pub max: f64,
    pub labels: &'a [String],
    /// Whether the labels lie at the ends of the side, not in bands.
    pub to_ends: bool,
    /// Room past the largest value, for the numbers on the bars.
    pub room: f64,
    /// Whether the axis is written.
    pub axis: bool,
    pub percent: bool,
}

/// The most of a side the margins take.
const MOST_MARGIN: f64 = 0.45;

impl Plot {
    pub fn new(scene: &mut Scene<'_>, area: Rect, want: &Want<'_>) -> Self {
        let (line, gap) = (scene.line(), scene.size * 0.4);
        let steps = if want.along_x {
            area.width() / (scene.size * 6.0)
        } else {
            area.height() / (scene.size * 3.0)
        };
        // Two to six: the cast is exact.
        let marks = ticks(want.min, want.max, steps.clamp(2.0, 6.0) as usize);
        let (min, max) = match (marks.first(), marks.last()) {
            (Some(a), Some(b)) => (*a, *b),
            _ => (0.0, 1.0),
        };
        let most = min.abs().max(max.abs());
        let written: Vec<String> = marks
            .iter()
            .map(|v| {
                if want.percent {
                    crate::data::plain(*v, true, scene.data.comma)
                } else {
                    short(*v, most, scene.data.comma)
                }
            })
            .collect();
        let widest = |scene: &Scene<'_>, list: &[String]| {
            list.iter().map(|t| scene.width(t)).fold(0.0, f64::max)
        };
        let axis = want.axis;
        let named = want.labels.iter().any(|l| !l.trim().is_empty());
        let (mut left, mut bottom, mut top, mut right) = (0.0, 0.0, 0.0, 0.0);
        if want.along_x {
            if named {
                left = widest(scene, want.labels).min(area.width() * 0.35) + gap;
            }
            if axis {
                bottom = line + gap;
                right = written.last().map_or(0.0, |t| scene.width(t) / 2.0);
            }
            right = f64::max(right, want.room);
        } else {
            if axis {
                left = widest(scene, &written) + gap;
                top = line / 2.0;
            }
            if named {
                bottom = line + gap;
            }
            top = f64::max(top, want.room);
        }
        let x = |v: f64| v.min(area.width() * MOST_MARGIN);
        let y = |v: f64| v.min(area.height() * MOST_MARGIN);
        let plot = Self {
            area: Rect::new(
                area.x0 + x(left),
                area.y0 + y(top),
                area.x1 - x(right),
                area.y1 - y(bottom),
            ),
            along_x: want.along_x,
            min,
            max,
            count: want.labels.len().max(1),
        };
        plot.write_axis(scene, &marks, &written, axis);
        if named {
            plot.write_labels(scene, area, want);
        }
        plot
    }

    fn write_axis(&self, scene: &mut Scene<'_>, marks: &[f64], written: &[String], axis: bool) {
        let gap = scene.size * 0.4;
        let thin = (scene.size / 14.0).clamp(0.5, 2.0);
        let faint = scene.faint();
        for (v, text) in marks.iter().zip(written) {
            let at = self.at(*v);
            if scene.chart.grid {
                let a = self.area;
                let rect = if self.along_x {
                    Rect::new(at - thin / 2.0, a.y0, at + thin / 2.0, a.y1)
                } else {
                    Rect::new(a.x0, at - thin / 2.0, a.x1, at + thin / 2.0)
                };
                scene.fill(rect.to_path(0.1), faint);
            }
            if axis && self.along_x {
                let at = Point::new(at, self.area.y1 + gap);
                scene.words(text, at, Anchor::Middle, Anchor::Start);
            } else if axis {
                let at = Point::new(self.area.x0 - gap, at);
                scene.words(text, at, Anchor::End, Anchor::Middle);
            }
        }
    }

    fn write_labels(&self, scene: &mut Scene<'_>, area: Rect, want: &Want<'_>) {
        let gap = scene.size * 0.4;
        let labels = want.labels;
        if self.along_x {
            let step = (scene.line() / self.band()).ceil().max(1.0) as usize;
            for (i, label) in labels.iter().enumerate().step_by(step) {
                let room = self.area.x0 - gap - area.x0;
                if let Some(text) = scene.fit(label, room) {
                    let at = Point::new(self.area.x0 - gap, self.middle(i));
                    scene.words(&text, at, Anchor::End, Anchor::Middle);
                }
            }
            return;
        }
        // Each label has the room of its band; when that is too little, one
        // in every few is written.
        let band = if want.to_ends {
            self.area.width() / (self.count.max(2) - 1) as f64
        } else {
            self.band()
        };
        let widest = labels.iter().map(|l| scene.width(l)).fold(0.0, f64::max);
        let wanted = (widest + gap).min(scene.size * 7.0);
        // At most one step for each label: the cast is exact.
        let step = (wanted / band).ceil().clamp(1.0, self.count as f64) as usize;
        for (i, label) in labels.iter().enumerate().step_by(step) {
            let Some(text) = scene.fit(label, band * step as f64 - gap) else {
                continue;
            };
            let half = scene.width(&text) / 2.0;
            let x = if want.to_ends {
                self.end(i)
            } else {
                self.middle(i)
            };
            // Inside the box of the chart.
            let x = x.min(area.x1 - half).max(area.x0 + half);
            let at = Point::new(x, self.area.y1 + gap);
            scene.words(&text, at, Anchor::Middle, Anchor::Start);
        }
    }

    /// Where `v` lies along the axis of the values.
    pub fn at(&self, v: f64) -> f64 {
        let share = ((v - self.min) / (self.max - self.min)).clamp(0.0, 1.0);
        if self.along_x {
            self.area.x0 + share * self.area.width()
        } else {
            self.area.y1 - share * self.area.height()
        }
    }

    /// Where the bars start.
    pub fn zero(&self) -> f64 {
        self.at(0.0)
    }

    /// The room of a label across the values.
    pub fn band(&self) -> f64 {
        let side = if self.along_x {
            self.area.height()
        } else {
            self.area.width()
        };
        // Few labels: the cast is exact.
        side / self.count as f64
    }

    /// The middle of the room of the label at `index`.
    pub fn middle(&self, index: usize) -> f64 {
        let start = if self.along_x {
            self.area.y0
        } else {
            self.area.x0
        };
        start + self.band() * (index as f64 + 0.5)
    }

    /// Where the label at `index` lies when they go from end to end.
    pub fn end(&self, index: usize) -> f64 {
        if self.count < 2 {
            return self.area.center().x;
        }
        self.area.x0 + self.area.width() * index as f64 / (self.count - 1) as f64
    }
}
