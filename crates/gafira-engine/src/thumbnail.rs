//! The thumbnail a save stores next to the document (02 §7.1 `thumbs/`,
//! 03 §7.2, spec 033): the first page, drawn through the `PageRenderers`
//! port of the exports (spec 030) at most `THUMBNAIL_SIDE` pixels on its
//! long side. It is best effort: a page that cannot be drawn gives no
//! thumbnail, and the save goes on without one.

use gafira_doc::Document;
use gafira_doc::contract::CancelToken;
use gafira_geom::IRect;
use gafira_ports::export::{ImagePixels, PageRenderRequest, PageRenderers};
use gafira_ports::repository::{THUMBNAIL_SIDE, Thumbnail};
use gafira_time::Flicks;

/// The pixel size of the thumbnail of a page of `width` × `height` pixels,
/// and the scale that draws it: the long side becomes [`THUMBNAIL_SIDE`],
/// or stays as it is when it is shorter (a small page is not enlarged).
/// Each side is rounded and at least one pixel. `None` for a size that is
/// not finite and positive.
pub(crate) fn size(width: f64, height: f64) -> Option<(u32, u32, f64)> {
    let valid = |side: f64| side.is_finite() && side > 0.0;
    if !valid(width) || !valid(height) {
        return None;
    }
    let side = f64::from(THUMBNAIL_SIDE);
    let scale = (side / width.max(height)).min(1.0);
    // In range: the scaled side is at most `side`, which fits a `u32`.
    let px = |length: f64| (length * scale).round().clamp(1.0, side) as u32;
    Some((px(width), px(height), scale))
}

/// The thumbnail of the first page of `document`, drawn by `pages` on the
/// page background; `None` with no page, a page with no pixel size or a
/// failed render. The repository leaves out a picture it cannot store.
pub(crate) fn first_page(
    document: &Document,
    pages: &dyn PageRenderers,
    images: ImagePixels,
) -> Option<Thumbnail> {
    let page = document.pages().first()?;
    let px = page.size.to_px(document.dpi()).ok()?;
    let (width, height, scale) = size(px.width(), px.height())?;
    let request = PageRenderRequest {
        page: page.id,
        time: Flicks::default(),
        scale: scale as f32,
        region: IRect::new(0, 0, width as i32, height as i32),
        transparent: false,
    };
    let image = pages
        .for_document(document, images)
        .render(&request, &CancelToken::new())
        .ok()?;
    Some(Thumbnail {
        page: page.id,
        image,
    })
}

#[cfg(test)]
mod tests {
    use std::path::Path;
    use std::sync::Arc;
    use std::time::Duration;

    use gafira_doc::{PageId, PageSize};
    use gafira_geom::{Length, Unit};

    use super::*;
    use crate::engine::Gafira;
    use crate::fakes::*;

    const TARGET: &str = "/work/poster.gaf";
    const COPY: &str = "/work/copy.gaf";

    // S33-05: the long side becomes 512 px, the other keeps the proportion.
    #[test]
    fn s33_05_the_long_side_is_512_and_the_proportion_stays() {
        let wide = size(1920.0, 1080.0).expect("a size");
        assert_eq!((wide.0, wide.1), (512, 288));
        assert!((wide.2 - 512.0 / 1920.0).abs() < 1e-12);
        let tall = size(1080.0, 1920.0).expect("a size");
        assert_eq!((tall.0, tall.1), (288, 512));
        assert_eq!(size(512.0, 512.0), Some((512, 512, 1.0)));
        assert_eq!(size(4096.0, 4096.0), Some((512, 512, 0.125)));
    }

    // S33-05: a small page is not enlarged; a thin side keeps one pixel.
    #[test]
    fn s33_05_a_small_page_keeps_its_size_and_a_thin_side_one_pixel() {
        assert_eq!(size(100.0, 50.0), Some((100, 50, 1.0)));
        assert_eq!(size(10.4, 10.6), Some((10, 11, 1.0)));
        assert_eq!(size(16_384.0, 1.0).map(|s| (s.0, s.1)), Some((512, 1)));
    }

    #[test]
    fn s33_05_a_size_that_is_not_finite_and_positive_has_no_thumbnail() {
        for (w, h) in [
            (0.0, 10.0),
            (10.0, 0.0),
            (-1.0, 10.0),
            (f64::NAN, 10.0),
            (10.0, f64::INFINITY),
        ] {
            assert_eq!(size(w, h), None, "{w} × {h}");
        }
    }

    fn px(width: f64, height: f64) -> PageSize {
        PageSize {
            width: Length::new(width, Unit::Px),
            height: Length::new(height, Unit::Px),
        }
    }

    /// An engine drawing with `pages`, its repository and its clock, with
    /// a new document of one `width` × `height` px page.
    fn engine(
        pages: &SizedPages,
        width: f64,
        height: f64,
    ) -> (Gafira, MemoryFs, Arc<MemoryRepository>, Arc<ManualClock>) {
        let fs = MemoryFs::default();
        let repository = Arc::new(MemoryRepository::new(&fs, false));
        let clock = Arc::new(ManualClock::at(Duration::ZERO));
        let engine = engine_with_pages(
            &fs,
            &repository,
            Arc::clone(&clock) as _,
            Arc::new(FakeProcesses::default()),
            Arc::new(pages.clone()),
        );
        engine
            .projects()
            .new_document_with(px(width, height))
            .expect("a new document");
        (engine, fs, repository, clock)
    }

    fn first(engine: &mut Gafira) -> PageId {
        engine.edit().document().expect("a session").pages()[0].id
    }

    // S33-05: save as, save, save version and save copy pass the first
    // page's thumbnail, at most 512 px on the long side.
    #[test]
    fn s33_05_every_save_passes_the_thumbnail_of_the_first_page() {
        let pages = SizedPages::default();
        let (mut engine, fs, repository, _clock) = engine(&pages, 1920.0, 1080.0);
        let page = first(&mut engine);
        append_page(&mut engine);
        engine
            .projects()
            .save_as(Path::new(TARGET))
            .expect("a save as");
        engine.projects().save().expect("a save");
        engine.projects().save_version("v").expect("a version");
        engine
            .projects()
            .save_copy(Path::new(COPY))
            .expect("a copy");
        assert_eq!(
            repository.take_thumbnails(),
            vec![Some((page, 512, 288)); 4]
        );
        let requests = pages.take_requests();
        assert_eq!(requests.len(), 4);
        for request in requests {
            assert_eq!(request.page, page, "the first page");
            assert_eq!(request.region, IRect::new(0, 0, 512, 288));
            assert!((f64::from(request.scale) - 512.0 / 1920.0).abs() < 1e-6);
            assert!(!request.transparent, "on the page background");
        }
        assert!(fs.file(TARGET).is_some() && fs.file(COPY).is_some());
    }

    // S33-05: the recovery slots carry no thumbnail.
    #[test]
    fn s33_05_autosave_passes_no_thumbnail() {
        let pages = SizedPages::default();
        let (mut engine, _fs, repository, clock) = engine(&pages, 100.0, 100.0);
        append_page(&mut engine);
        clock.set(Duration::from_secs(60));
        let job = engine.projects().autosave_due().expect("a job");
        let outcome = job.run();
        assert!(outcome.result.is_ok());
        engine.projects().autosave_finished(outcome);
        assert_eq!(repository.take_thumbnails(), Vec::new());
        assert_eq!(pages.take_requests(), Vec::new(), "nothing drawn");
    }

    // S33-05: a document with no page has no thumbnail.
    #[test]
    fn s33_05_no_page_no_thumbnail() {
        let pages = SizedPages::default();
        let (engine, _fs, repository, _clock) = engine(&pages, 100.0, 100.0);
        engine.projects().new_document().expect("an empty document");
        engine
            .projects()
            .save_as(Path::new(TARGET))
            .expect("a save as");
        assert_eq!(repository.take_thumbnails(), vec![None]);
        assert_eq!(pages.take_requests(), Vec::new());
    }

    // S33-06: a render that fails leaves the thumbnail out and the save
    // succeeds.
    #[test]
    fn s33_06_a_failed_render_still_saves() {
        let pages = SizedPages::failing();
        let (engine, fs, repository, _clock) = engine(&pages, 100.0, 100.0);
        engine
            .projects()
            .save_as(Path::new(TARGET))
            .expect("the save goes on");
        assert_eq!(repository.take_thumbnails(), vec![None]);
        assert_eq!(pages.take_requests().len(), 1, "it was tried");
        assert!(fs.file(TARGET).is_some(), "the file is written");
        assert!(!engine.projects().is_modified(), "and the session is clean");
    }
}
