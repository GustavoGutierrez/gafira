//! The zoom of the canvas (spec 037, 03 §7.3 and §9.2): 5 % to 3200 %,
//! «Ajustar» to fit the first page, fixed steps for the buttons and the
//! shortcuts, and a smooth factor for Ctrl + wheel.

use egui::Key;

/// 5 % (03 §7.3).
pub(crate) const MIN: f32 = 0.05;
/// 3200 % (03 §7.3).
pub(crate) const MAX: f32 = 32.0;

/// The fixed choices of the zoom menu after «Ajustar» (03 §7.3).
pub(crate) const PRESETS: [f32; 3] = [0.5, 1.0, 2.0];

/// The stops of «Acercar» and «Alejar».
const STEPS: [f32; 19] = [
    0.05, 0.1, 0.25, 0.33, 0.5, 0.67, 0.75, 1.0, 1.25, 1.5, 2.0, 3.0, 4.0, 6.0, 8.0, 12.0, 16.0,
    24.0, 32.0,
];

/// How the canvas zoom is chosen.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) enum Zoom {
    /// The first page fits the canvas (`format::fit_zoom`).
    #[default]
    Fit,
    /// A fixed scale, 1.0 being 100 %.
    Fixed(f32),
}

impl Zoom {
    /// The scale in force, given the one that fits.
    pub(crate) fn resolve(self, fit: f32) -> f32 {
        match self {
            Self::Fit => clamp(fit),
            Self::Fixed(scale) => clamp(scale),
        }
    }
}

/// What the person asked of the zoom.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum ZoomAsk {
    In,
    Out,
    Fit,
    To(f32),
}

impl ZoomAsk {
    /// The zoom that follows from `current`, the scale on display.
    pub(crate) fn apply(self, current: f32) -> Zoom {
        match self {
            Self::In => Zoom::Fixed(step_in(current)),
            Self::Out => Zoom::Fixed(step_out(current)),
            Self::Fit => Zoom::Fit,
            Self::To(scale) => Zoom::Fixed(clamp(scale)),
        }
    }

    /// The shortcut of 03 §9.2 for `key` with Ctrl/Cmd held: Ctrl+= (or +)
    /// acercar, Ctrl+- alejar, Ctrl+0 ajustar, Ctrl+1 100 %.
    pub(crate) fn from_shortcut(command: bool, key: Key) -> Option<Self> {
        if !command {
            return None;
        }
        match key {
            Key::Equals | Key::Plus => Some(Self::In),
            Key::Minus => Some(Self::Out),
            Key::Num0 => Some(Self::Fit),
            Key::Num1 => Some(Self::To(1.0)),
            _ => None,
        }
    }
}

/// `scale` within 5-3200 % (an infinite one goes to its limit); a value
/// that is not a number is 100 %.
pub(crate) fn clamp(scale: f32) -> f32 {
    if scale.is_nan() {
        1.0
    } else {
        scale.clamp(MIN, MAX)
    }
}

/// The first stop above `current` (a value within 0.5 % of a stop counts as on it).
pub(crate) fn step_in(current: f32) -> f32 {
    let current = clamp(current);
    STEPS
        .into_iter()
        .find(|s| *s > current * 1.005)
        .unwrap_or(MAX)
}

/// The first stop below `current`.
pub(crate) fn step_out(current: f32) -> f32 {
    let current = clamp(current);
    STEPS
        .into_iter()
        .rev()
        .find(|s| *s < current * 0.995)
        .unwrap_or(MIN)
}

/// The zoom after a pinch or Ctrl + wheel of `factor` (S59-01): the
/// current one times it, within the limits.
pub(crate) fn by_factor(current: f32, factor: f32) -> f32 {
    clamp(current * factor)
}

/// Where page `index` of pages `sizes` (px of page) sits in the content of
/// the canvas at `zoom`, with `view` the size of the canvas on screen: the
/// pages stack top to bottom `gap` apart, centred across, after a `margin`,
/// and the whole stack is centred down when it is shorter than the view.
/// The canvas draws them there and the zoom keeps a point under the
/// pointer with it (spec 059).
pub(crate) fn page_origin(
    sizes: &[egui::Vec2],
    index: usize,
    zoom: f32,
    view: egui::Vec2,
    (margin, gap): (f32, f32),
) -> egui::Vec2 {
    let widest = sizes.iter().map(|s| s.x).fold(0.0, f32::max);
    let content_w = (widest * zoom + 2.0 * margin).max(view.x);
    let height: f32 = sizes.iter().map(|s| s.y * zoom + gap).sum::<f32>() + margin;
    let top = margin.max((view.y - height).max(0.0) / 2.0);
    let above: f32 = sizes.iter().take(index).map(|s| s.y * zoom + gap).sum();
    let size = sizes.get(index).copied().unwrap_or_default();
    egui::Vec2::new(content_w / 2.0 - size.x * zoom / 2.0, top + above)
}

#[cfg(test)]
mod tests {
    use super::*;

    // S59-02: the pages are centred across the content, stacked down from
    // the margin with the gap between them, and centred down when short.
    #[test]
    fn s59_02_page_origin() {
        let sizes = [egui::Vec2::new(100.0, 50.0), egui::Vec2::new(200.0, 100.0)];
        let view = egui::Vec2::new(1000.0, 100.0);
        // Content 1000 wide (the view is wider than 200 * 2 + 2 * 48).
        assert_eq!(
            page_origin(&sizes, 0, 2.0, view, (48.0, 56.0)),
            egui::Vec2::new(400.0, 48.0)
        );
        assert_eq!(
            page_origin(&sizes, 1, 2.0, view, (48.0, 56.0)),
            egui::Vec2::new(300.0, 48.0 + 100.0 + 56.0)
        );
        // A tall view centres the stack down: height 50 + 56 + 100 + 56 + 48 = 310.
        let tall = egui::Vec2::new(1000.0, 1000.0);
        assert_eq!(
            page_origin(&sizes, 0, 1.0, tall, (48.0, 56.0)).y,
            (1000.0 - 310.0) / 2.0
        );
        // A narrow view: the content is as wide as the widest page plus margins.
        let narrow = egui::Vec2::new(100.0, 100.0);
        assert_eq!(
            page_origin(&sizes, 0, 1.0, narrow, (48.0, 56.0)).x,
            (296.0 - 100.0) / 2.0
        );
    }

    // S59-01: a pinch or Ctrl + wheel multiplies the zoom, within 5-3200 %.
    #[test]
    fn s59_01_zoom_by_factor() {
        assert_eq!(by_factor(1.0, 1.1), 1.1);
        assert_eq!(by_factor(2.0, 0.5), 1.0);
        assert_eq!(by_factor(MAX, 2.0), MAX);
        assert_eq!(by_factor(MIN, 0.5), MIN);
    }

    #[test]
    fn s37_06_steps_go_to_the_next_stop_and_stop_at_the_limits() {
        assert_eq!(step_in(1.0), 1.25);
        assert_eq!(step_out(1.0), 0.75);
        // Between two stops: the next one in each direction.
        assert_eq!(step_in(0.8), 1.0);
        assert_eq!(step_out(0.8), 0.75);
        // A fitted zoom just off a stop does not repeat that stop.
        assert_eq!(step_in(0.999), 1.25);
        assert_eq!(step_in(MAX), MAX);
        assert_eq!(step_out(MIN), MIN);
        assert_eq!(step_in(1000.0), MAX);
        assert_eq!(step_out(0.0), MIN);
    }

    #[test]
    fn s37_06_every_scale_stays_within_5_and_3200_percent() {
        for value in [
            -1.0,
            0.0,
            0.01,
            0.05,
            1.0,
            32.0,
            99.0,
            f32::NAN,
            f32::INFINITY,
        ] {
            let scale = clamp(value);
            assert!((MIN..=MAX).contains(&scale), "{value} -> {scale}");
            for zoom in [Zoom::Fit, Zoom::Fixed(value)] {
                assert!((MIN..=MAX).contains(&zoom.resolve(value)), "{zoom:?}");
            }
        }
        assert_eq!(clamp(f32::NAN), 1.0);
        assert_eq!(by_factor(1.0, 1.0e6), MAX);
        assert_eq!(by_factor(1.0, 1.0e-6), MIN);
        assert!(by_factor(1.0, 1.1) > 1.0 && by_factor(1.0, 0.9) < 1.0);
    }

    #[test]
    fn s37_06_fit_follows_the_window_and_fixed_does_not() {
        assert_eq!(Zoom::Fit.resolve(0.4), 0.4);
        assert_eq!(Zoom::Fixed(2.0).resolve(0.4), 2.0);
        assert_eq!(Zoom::default(), Zoom::Fit);
        assert_eq!(ZoomAsk::Fit.apply(3.0), Zoom::Fit);
        assert_eq!(ZoomAsk::To(0.5).apply(3.0), Zoom::Fixed(0.5));
        assert_eq!(ZoomAsk::To(100.0).apply(3.0), Zoom::Fixed(MAX));
        assert_eq!(ZoomAsk::In.apply(0.4), Zoom::Fixed(0.5));
        assert_eq!(ZoomAsk::Out.apply(0.4), Zoom::Fixed(0.33));
    }

    #[test]
    fn s37_07_the_shortcuts_of_03_9_2_need_ctrl() {
        assert_eq!(ZoomAsk::from_shortcut(true, Key::Equals), Some(ZoomAsk::In));
        assert_eq!(ZoomAsk::from_shortcut(true, Key::Plus), Some(ZoomAsk::In));
        assert_eq!(ZoomAsk::from_shortcut(true, Key::Minus), Some(ZoomAsk::Out));
        assert_eq!(ZoomAsk::from_shortcut(true, Key::Num0), Some(ZoomAsk::Fit));
        assert_eq!(
            ZoomAsk::from_shortcut(true, Key::Num1),
            Some(ZoomAsk::To(1.0))
        );
        assert_eq!(ZoomAsk::from_shortcut(false, Key::Num1), None);
        assert_eq!(ZoomAsk::from_shortcut(true, Key::A), None);
    }
}
