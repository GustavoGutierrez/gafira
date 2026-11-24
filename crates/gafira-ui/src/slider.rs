//! The slider of the editor (spec 098), first drawn for «Editar imagen»
//! (spec 085): a thin track with the part done in the accent colour, a round
//! knob, and, above it, the name with its number in a box at the right. A
//! double click goes back to the resting value. The compact form (the zoom
//! of the bottom bar) is only the track and the knob.

use egui::{Color32, Pos2, Rect, RichText, Sense, Stroke, Ui, Vec2};

use crate::style::Theme;
use crate::text::Texts;

/// The colours along a track.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Track {
    Plain,
    Temperature,
    Tint,
    Hue,
    Saturation,
}

impl Track {
    fn colors(self) -> Vec<Color32> {
        let rgb = Color32::from_rgb;
        match self {
            Self::Plain => Vec::new(),
            Self::Temperature => vec![rgb(47, 128, 237), rgb(221, 226, 230), rgb(242, 153, 74)],
            Self::Tint => vec![rgb(39, 174, 96), rgb(221, 226, 230), rgb(214, 70, 160)],
            Self::Saturation => vec![rgb(150, 150, 150), rgb(235, 87, 87)],
            // Once around, from the opposite of red.
            Self::Hue => (0..=12)
                .map(|n| egui::ecolor::Hsva::new((n as f32 / 12.0 + 0.5).fract(), 0.75, 0.9, 1.0))
                .map(Color32::from)
                .collect(),
        }
    }
}

/// Where along `range` the value `v` sits, from 0 to 1; on a logarithmic
/// scale each doubling is as long.
pub(crate) fn share_of(v: f64, (lo, hi): (f64, f64), log: bool) -> f64 {
    let share = if log && lo > 0.0 && hi > lo {
        (v.max(lo) / lo).ln() / (hi / lo).ln()
    } else {
        (v - lo) / (hi - lo).max(f64::EPSILON)
    };
    if share.is_finite() {
        share.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

/// The value at `share` (0 to 1) along `range`, on the steps of `step` from
/// its start (none when `step` is 0) and within it.
pub(crate) fn value_at(share: f64, (lo, hi): (f64, f64), log: bool, step: f64) -> f64 {
    let share = share.clamp(0.0, 1.0);
    let raw = if log && lo > 0.0 && hi > lo {
        lo * (hi / lo).powf(share)
    } else {
        lo + share * (hi - lo)
    };
    let stepped = if step > 0.0 {
        lo + ((raw - lo) / step).round() * step
    } else {
        raw
    };
    stepped.clamp(lo, hi)
}

/// The unit after a percentage, as the catalog writes it (« %», «%»;
/// ARQ-97).
pub(crate) fn percent(texts: &Texts) -> String {
    texts.t(&gafira_doc::msg!(
        "format.percent",
        value = Text(String::new())
    ))
}

/// The unit after degrees, as the catalog writes it («°»; ARQ-97).
pub(crate) fn degrees(texts: &Texts) -> String {
    texts.t(&gafira_doc::msg!("unit.deg", value = Text(String::new())))
}

/// A slider of the editor; see the module.
pub(crate) struct Slider<'a> {
    value: &'a mut f64,
    range: (f64, f64),
    rest: f64,
    step: f64,
    decimals: usize,
    suffix: String,
    track: Track,
    log: bool,
    width: Option<f32>,
    name: String,
    compact: bool,
    named: bool,
}

impl<'a> Slider<'a> {
    /// `value` within `range`, named `name`; it rests at the start of the
    /// range, moves in whole steps and shows no decimals.
    pub(crate) fn new(name: String, value: &'a mut f64, range: (f64, f64)) -> Self {
        Self {
            value,
            range,
            rest: range.0,
            step: 1.0,
            decimals: 0,
            suffix: String::new(),
            track: Track::Plain,
            log: false,
            width: None,
            name,
            compact: false,
            named: true,
        }
    }

    /// The value a double click goes back to, where the accent starts.
    pub(crate) fn rest(mut self, rest: f64) -> Self {
        self.rest = rest;
        self
    }

    /// The steps it moves in (0 for none) and the decimals it shows.
    pub(crate) fn step(mut self, step: f64, decimals: usize) -> Self {
        self.step = step;
        self.decimals = decimals;
        self
    }

    /// The unit after the number: «%», «°», «×».
    pub(crate) fn suffix(mut self, suffix: impl Into<String>) -> Self {
        self.suffix = suffix.into();
        self
    }

    pub(crate) fn track(mut self, track: Track) -> Self {
        self.track = track;
        self
    }

    /// Each doubling as long (the zoom, sizes).
    pub(crate) fn logarithmic(mut self) -> Self {
        self.log = true;
        self
    }

    /// The width of the track; the whole row by default.
    pub(crate) fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    /// Only the track and the knob, with no name or number; the name is
    /// still its accessible name.
    pub(crate) fn compact(mut self) -> Self {
        self.compact = true;
        self
    }

    /// The number without the name beside it, when the name is shown
    /// elsewhere (a row of a form); it is still the accessible name.
    pub(crate) fn unnamed(mut self) -> Self {
        self.named = false;
        self
    }

    /// Draw it; whether the value changed.
    pub(crate) fn show(self, ui: &mut Ui, theme: &Theme) -> bool {
        let Self {
            value,
            range,
            rest,
            step,
            decimals,
            suffix,
            track,
            log,
            width,
            name,
            compact,
            named,
        } = self;
        let before = *value;
        if !compact {
            ui.horizontal(|ui| {
                if named {
                    ui.label(RichText::new(name.as_str()).font(theme.font("body-sm")));
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let speed = if step > 0.0 {
                        step.max(0.01) * 0.5
                    } else {
                        (range.1 - range.0) / 400.0
                    };
                    let mut drag = egui::DragValue::new(value)
                        .range(range.0..=range.1)
                        .max_decimals(decimals)
                        .speed(speed);
                    if !suffix.is_empty() {
                        drag = drag.suffix(suffix.as_str());
                    }
                    let box_width = if suffix.is_empty() { 56.0 } else { 64.0 };
                    ui.add_sized(Vec2::new(box_width, 26.0), drag);
                });
            });
        }
        let (height, knob_r) = if compact { (18.0, 6.0) } else { (22.0, 8.0) };
        let wide = width.unwrap_or_else(|| ui.available_width()).max(40.0);
        let (rect, response) =
            ui.allocate_exact_size(Vec2::new(wide, height), Sense::click_and_drag());
        // The knob stays whole inside.
        let room = rect.shrink2(Vec2::new(knob_r + 1.0, 0.0));
        if response.double_clicked() {
            *value = rest;
        } else if (response.dragged() || response.clicked())
            && let Some(at) = response.interact_pointer_pos()
        {
            let share = f64::from((at.x - room.left()) / room.width().max(1.0));
            *value = value_at(share, range, log, step);
        }
        let x_of = |v: f64| room.left() + room.width() * share_of(v, range, log) as f32;
        let thick = if compact { 4.0 } else { 5.0 };
        let line = Rect::from_center_size(rect.center(), Vec2::new(room.width(), thick));
        let painter = ui.painter();
        let colors = track.colors();
        if colors.len() < 2 {
            painter.rect_filled(line, thick / 2.0, theme.c("border"));
            let (a, b) = (x_of(rest), x_of(*value));
            let done = Rect::from_x_y_ranges(a.min(b)..=a.max(b), line.y_range());
            painter.rect_filled(done, thick / 2.0, theme.c("accent"));
        } else {
            let mut mesh = egui::Mesh::default();
            // A handful of colours: the casts are exact.
            let last = (colors.len() - 1) as f32;
            for (n, color) in colors.iter().enumerate() {
                let x = line.left() + line.width() * n as f32 / last;
                mesh.colored_vertex(Pos2::new(x, line.top()), *color);
                mesh.colored_vertex(Pos2::new(x, line.bottom()), *color);
                if n > 0 {
                    let at = n as u32 * 2;
                    mesh.add_triangle(at - 2, at - 1, at);
                    mesh.add_triangle(at - 1, at, at + 1);
                }
            }
            painter.add(mesh);
        }
        let knob = Pos2::new(x_of(*value), rect.center().y);
        let edge = if response.hovered() || response.dragged() {
            theme.c("accent")
        } else {
            theme.c("text.secondary")
        };
        painter.circle(knob, knob_r, theme.c("surface"), Stroke::new(1.5, edge));
        if response.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
        }
        let shown = *value;
        response.widget_info(|| egui::WidgetInfo::slider(true, shown, name.as_str()));
        if !compact {
            ui.add_space(4.0);
        }
        *value != before
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn s98_06_a_linear_track_places_and_reads_values_evenly() {
        let range = (-100.0, 100.0);
        assert_eq!(share_of(0.0, range, false), 0.5);
        assert_eq!(share_of(-200.0, range, false), 0.0);
        assert_eq!(share_of(300.0, range, false), 1.0);
        assert_eq!(value_at(0.25, range, false, 1.0), -50.0);
        assert_eq!(value_at(2.0, range, false, 1.0), 100.0);
    }

    #[test]
    fn s98_06_values_fall_on_the_steps_from_the_start_of_the_range() {
        // Line height from 0.5 to 3 in steps of 0.05.
        let v = value_at(0.1234, (0.5, 3.0), false, 0.05);
        assert!((v - 0.8).abs() < 1e-9, "{v}");
        // No step: the exact value.
        assert!((value_at(0.3, (0.0, 1.0), false, 0.0) - 0.3).abs() < 1e-12);
        // The last step never goes past the end.
        assert_eq!(value_at(1.0, (0.0, 10.0), false, 4.0), 10.0);
    }

    #[test]
    fn s98_06_a_logarithmic_track_gives_each_doubling_the_same_length() {
        // The zoom, 5 % to 3200 %.
        let range = (0.05, 32.0);
        let a = share_of(0.1, range, true) - share_of(0.05, range, true);
        let b = share_of(16.0, range, true) - share_of(8.0, range, true);
        assert!((a - b).abs() < 1e-9);
        assert!((value_at(share_of(1.0, range, true), range, true, 0.0) - 1.0).abs() < 1e-9);
        assert_eq!(share_of(0.0, range, true), 0.0);
    }
}
