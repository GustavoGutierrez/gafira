//! The render doubles of the testkit, seen from the engine that injects them
//! (ARQ-112, 02 §4.5, §4.6).

use std::sync::Arc;

use gafira_doc::PageId;
use gafira_doc::contract::CancelToken;
use gafira_geom::IRect;
use gafira_ports::common::Bytes;
use gafira_ports::export::{PageRenderRequest, PageRenderer};
use gafira_ports::render::{
    BackendKind, ContentKey, Deadline, Priority, RenderBackend, RenderError, RenderPlan,
    RenderQuality, TargetDesc, TileCache, TileKey, TileTask,
};
use gafira_raster::{ImageBuf, PixelFormat};
use gafira_testkit::ports::page_renderer::FakePageRenderer;
use gafira_testkit::ports::render_backend::FakeRenderBackend;
use gafira_time::Flicks;

/// A cache that keeps nothing.
struct NullCache;

impl TileCache for NullCache {
    fn get(&self, _key: &TileKey) -> Option<Arc<ImageBuf>> {
        None
    }

    fn put(&mut self, _key: TileKey, _tile: Arc<ImageBuf>) {}

    fn bytes(&self) -> Bytes {
        Bytes(0)
    }

    fn trim(&mut self, _budget: Bytes) {}
}

/// One task with no operations.
fn plan() -> RenderPlan {
    RenderPlan {
        quality: RenderQuality::Preview,
        tasks: vec![TileTask {
            key: TileKey {
                content: ContentKey(1),
                level: 0,
                column: 0,
                row: 0,
            },
            priority: Priority::Visible,
            region: IRect::new(0, 0, 256, 256),
            ops: Vec::new(),
        }],
    }
}

// ARQ-112: the render doubles answer instead of panicking.
#[test]
fn arq_112_the_render_doubles_answer() {
    let mut backend = FakeRenderBackend;
    assert_eq!(backend.caps().kind, BackendKind::Cpu);
    assert!(!backend.caps().gpu_effects);

    let target = TargetDesc {
        width: 4,
        height: 4,
        origin: (0, 0),
        format: PixelFormat::Rgba8,
    };
    let mut cx = backend.begin_frame(&target).expect("the frame opens");
    let mut cache = NullCache;
    let report = backend
        .execute(
            &mut cx,
            &plan(),
            &mut cache,
            Deadline::Unbounded,
            &CancelToken::new(),
        )
        .expect("the plan runs");
    assert_eq!(report.done, 1);
    assert_eq!(report.left, 0);

    let image = backend
        .read_back(&cx, IRect::new(0, 0, 4, 4))
        .expect("the region reads back");
    assert_eq!(image.width(), 4);
    assert_eq!(image.height(), 4);

    let request = PageRenderRequest {
        page: PageId::new(1),
        time: Flicks::from_ticks(0),
        scale: 1.0,
        region: IRect::new(0, 0, 8, 8),
        transparent: true,
    };
    let image = FakePageRenderer
        .render(&request, &CancelToken::new())
        .expect("the page renders");
    assert_eq!(image.width(), 8);
    assert_eq!(image.height(), 8);
}

// ARQ-112: a cancelled token stops the doubles with the cancelled error.
#[test]
fn arq_112_a_cancelled_token_stops_the_doubles() {
    let token = CancelToken::new();
    token.cancel();

    let mut backend = FakeRenderBackend;
    let target = TargetDesc {
        width: 4,
        height: 4,
        origin: (0, 0),
        format: PixelFormat::Rgba8,
    };
    let mut cx = backend.begin_frame(&target).expect("the frame opens");
    let mut cache = NullCache;
    let result = backend.execute(&mut cx, &plan(), &mut cache, Deadline::Unbounded, &token);
    assert!(matches!(result, Err(RenderError::Cancelled(_))));

    let request = PageRenderRequest {
        page: PageId::new(1),
        time: Flicks::from_ticks(0),
        scale: 1.0,
        region: IRect::new(0, 0, 8, 8),
        transparent: true,
    };
    let result = FakePageRenderer.render(&request, &token);
    assert!(matches!(result, Err(RenderError::Cancelled(_))));
}
