//! S69-03: page backgrounds through the whole export path (graph, planner,
//! CPU backend).

use std::sync::Arc;
use std::time::Duration;

use gafira_color::{Color, Rgba};
use gafira_doc::contract::CancelToken;
use gafira_doc::timeline::{AssetEntry, AssetRef, AssetStorage, AssetTable};
use gafira_doc::{
    BackgroundFit, ContentHash, Document, Dpi, Page, PageBackground, PageId, PageSize, PatternKind,
};
use gafira_geom::{IRect, Length, Unit};
use gafira_ports::common::{Bytes, UnixTime};
use gafira_ports::export::{ImagePixels, PageRenderRequest, PageRenderer};
use gafira_ports::render::RenderQuality;
use gafira_ports::system::Clock;
use gafira_raster::{ImageBuf, PixelBuf};
use gafira_render::{BackendSelector, ContentTileCache, GraphPageRenderer};
use gafira_render_cpu::CpuBackend;

const SIDE: u32 = 64;
const PHOTO: AssetRef = AssetRef::new(ContentHash([6; 32]));

struct FixedClock;

impl Clock for FixedClock {
    fn now(&self) -> UnixTime {
        UnixTime(0)
    }

    fn monotonic(&self) -> Duration {
        Duration::ZERO
    }
}

fn rgb(r: f32, g: f32, b: f32) -> Color {
    Color::Srgb(Rgba::new(r, g, b, 1.0))
}

fn render(background: PageBackground) -> Vec<u8> {
    let mut page = Page::new(
        PageId::new(1),
        "S69".to_owned(),
        PageSize {
            width: Length::new(f64::from(SIDE), Unit::Px),
            height: Length::new(f64::from(SIDE), Unit::Px),
        },
    );
    page.background = Some(background);
    let mut assets = AssetTable::new();
    assets.insert(
        PHOTO,
        AssetEntry {
            name: "tile.png".to_owned(),
            storage: AssetStorage::Embedded,
        },
    );
    let document =
        Document::from_parts_with_assets(2, Dpi::default(), vec![Arc::new(page)], assets)
            .expect("valid");
    // A 16 × 16 tile: red on its left half, blue on its right.
    let mut tile = Vec::new();
    for _ in 0..16 {
        for x in 0..16 {
            tile.extend(if x < 8 {
                [255, 0, 0, 255]
            } else {
                [0, 0, 255, 255]
            });
        }
    }
    let image = ImageBuf::new(16, 16, PixelBuf::Rgba8(tile)).expect("image");
    let renderer = GraphPageRenderer::new(
        document,
        RenderQuality::Preview,
        BackendSelector::new(None, Box::new(CpuBackend::new())),
        ContentTileCache::new(Bytes(4 * 1024 * 1024)),
        Box::new(FixedClock),
    )
    .with_images(ImagePixels::from([(PHOTO, Arc::new(image))]));
    let request = PageRenderRequest {
        page: PageId::new(1),
        time: Default::default(),
        scale: 1.0,
        region: IRect::new(0, 0, SIDE as i32, SIDE as i32),
        transparent: false,
    };
    match renderer
        .render(&request, &CancelToken::new())
        .expect("renders")
        .data()
    {
        PixelBuf::Rgba8(bytes) => bytes.clone(),
        other => panic!("the preview answers Rgba8, not {other:?}"),
    }
}

fn at(pixels: &[u8], x: u32, y: u32) -> [u8; 4] {
    let i = ((y * SIDE + x) * 4) as usize;
    [pixels[i], pixels[i + 1], pixels[i + 2], pixels[i + 3]]
}

// S69-03: a colour fills the whole page.
#[test]
fn s69_03_a_colour_fills_the_page() {
    let pixels = render(PageBackground::Solid(rgb(0.0, 0.0, 1.0)));
    for (x, y) in [(0, 0), (32, 32), (63, 63)] {
        assert_eq!(at(&pixels, x, y), [0, 0, 255, 255], "({x}, {y})");
    }
}

// S69-03: a pattern of checks alternates its two colours every half cell.
#[test]
fn s69_03_a_pattern_alternates_its_colours() {
    let pixels = render(PageBackground::Pattern {
        kind: PatternKind::Checks,
        color: rgb(1.0, 0.0, 0.0),
        back: rgb(0.0, 1.0, 0.0),
    });
    assert_eq!(at(&pixels, 8, 8), [255, 0, 0, 255]);
    assert_eq!(at(&pixels, 40, 8), [0, 255, 0, 255]);
}

// S69-03: a tiled image repeats at its own size.
#[test]
fn s69_03_a_tiled_image_repeats() {
    let pixels = render(PageBackground::Image {
        asset: PHOTO,
        natural: (16, 16),
        fit: BackgroundFit::Tile,
    });
    for x in [2, 18, 50] {
        assert_eq!(
            at(&pixels, x, 40)[..3],
            [255, 0, 0],
            "left half of a tile at {x}"
        );
    }
    for x in [12, 28, 60] {
        assert_eq!(
            at(&pixels, x, 40)[..3],
            [0, 0, 255],
            "right half of a tile at {x}"
        );
    }
}
