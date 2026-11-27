//! How a page fits the canvas, and the starting formats the tests open
//! documents with (Inicio's sizes are in `design_sizes`, spec 076).

#[cfg(test)]
use gafira_doc::PageSize;
#[cfg(test)]
use gafira_geom::{Length, Unit};

/// A starting format the tests open documents with.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg(test)]
pub(crate) enum StartFormat {
    Social,
    Presentation,
    Print,
    VideoVertical,
    VideoHorizontal,
    Custom,
}

#[cfg(test)]
impl StartFormat {
    /// The size of the first page: A4 in millimetres, the rest in pixels; a
    /// custom size starts square until Cambiar tamaño exists (03 §7.6).
    pub(crate) fn page_size(self) -> PageSize {
        let px = |w, h| PageSize {
            width: Length::new(w, Unit::Px),
            height: Length::new(h, Unit::Px),
        };
        match self {
            Self::Social | Self::Custom => px(1080.0, 1080.0),
            Self::Presentation | Self::VideoHorizontal => px(1920.0, 1080.0),
            Self::Print => PageSize {
                width: Length::new(210.0, Unit::Mm),
                height: Length::new(297.0, Unit::Mm),
            },
            Self::VideoVertical => px(1080.0, 1920.0),
        }
    }
}

/// The zoom that fits a page of `page` px inside `area` px leaving `margin`
/// px around it, capped at 100 % (a small page is not blown up) and never
/// under 5 % (03 §7.3, zoom 5-3200 %).
pub(crate) fn fit_zoom(page: (f32, f32), area: (f32, f32), margin: f32) -> f32 {
    let room_w = (area.0 - 2.0 * margin).max(1.0);
    let room_h = (area.1 - 2.0 * margin).max(1.0);
    (room_w / page.0).min(room_h / page.1).clamp(0.05, 1.0)
}

/// The zoom at which a page `page` px wide fills the width of `area` less
/// `margin` on each side (owner request: «Ajustar al ancho»).
pub(crate) fn fit_width(page: f32, area: f32, margin: f32) -> f32 {
    (area - 2.0 * margin).max(1.0) / page.max(1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn s17_04_every_format_has_a_valid_page_with_its_aspect() {
        let dpi = gafira_doc::Dpi::DEFAULT;
        let aspect = |format: StartFormat| {
            let size = format.page_size().to_px(dpi).expect("converts");
            size.height() / size.width()
        };
        for format in [
            StartFormat::Social,
            StartFormat::Presentation,
            StartFormat::Print,
            StartFormat::VideoVertical,
            StartFormat::VideoHorizontal,
            StartFormat::Custom,
        ] {
            assert!(format.page_size().check(dpi).is_ok(), "{format:?}");
        }
        assert!((aspect(StartFormat::VideoVertical) - 16.0 / 9.0).abs() < 1e-9);
        assert!((aspect(StartFormat::Print) - 297.0 / 210.0).abs() < 1e-9);
    }

    #[test]
    fn s17_04_the_page_fits_the_canvas_with_its_margin() {
        let page = (1920.0, 1080.0);
        let zoom = fit_zoom(page, (1000.0, 800.0), 40.0);
        assert!(page.0 * zoom <= 1000.0 - 80.0 + 1e-3);
        assert!(page.1 * zoom <= 800.0 - 80.0 + 1e-3);
        assert!((page.0 * zoom - 920.0).abs() < 1e-3, "width binds");
        let tall = fit_zoom((1080.0, 1920.0), (1000.0, 800.0), 40.0);
        assert!((1920.0 * tall - 720.0).abs() < 1e-3, "height binds");
    }

    #[test]
    fn s17_04_zoom_stays_between_5_and_100_percent() {
        assert_eq!(fit_zoom((100.0, 100.0), (4000.0, 4000.0), 0.0), 1.0);
        assert_eq!(fit_zoom((100_000.0, 100_000.0), (100.0, 100.0), 0.0), 0.05);
        assert_eq!(fit_zoom((100.0, 100.0), (10.0, 10.0), 40.0), 0.05);
        // «Ajustar al ancho»: the width less the margins, past 100 % too.
        assert_eq!(fit_width(500.0, 1080.0, 40.0), 2.0);
    }
}
