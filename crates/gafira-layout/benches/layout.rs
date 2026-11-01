//! Benchmarks of the layer-2 layout figures (ARQ-114, 02 §6.7).
//!
//! One benchmark per row, named like the row. The budgets are checked by the
//! nightly job on the reference machine, not by `cargo test`.

use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use gafira_doc::Margins;
use gafira_layout::grid::grid_lines;
use gafira_layout::{
    GridSettings, GridStyle, Guide, GuideAxis, GuideSpec, Length, Rect, SnapInput, Unit,
    bleed_rect, generate, margin_rect, overflow, safe_rect, snap,
};

const DPI: f64 = 96.0;

fn px(v: f64) -> Length {
    Length::new(v, Unit::Px)
}

/// FD-04: the worst accepted request, on the largest page.
fn bench_guides_generate(c: &mut Criterion) {
    let spec = GuideSpec {
        columns: 16_384,
        rows: 16_384,
        gap: px(0.0),
        margin: px(0.0),
    };
    let page = Rect::new(0.0, 0.0, 16_384.0, 16_384.0);
    c.bench_function("bench_guides_generate", |b| {
        b.iter(|| generate(black_box(&spec), black_box(page), black_box(DPI)));
    });
}

/// FD-05: bleed, safe zone, margins and the overflow check of one page.
fn bench_layout_rects(c: &mut Criterion) {
    let page = Rect::new(0.0, 0.0, 1080.0, 1920.0);
    let margins = Margins {
        top: px(48.0),
        right: px(48.0),
        bottom: px(48.0),
        left: px(48.0),
        linked: true,
    };
    let bleed = Length::new(3.0, Unit::Mm);
    let element = Rect::new(-20.0, 30.0, 1100.0, 400.0);
    c.bench_function("bench_layout_rects", |b| {
        b.iter(|| {
            let bleed = bleed_rect(black_box(page), black_box(bleed), black_box(DPI));
            let safe = safe_rect(black_box(page), black_box(5.0));
            let inner = margin_rect(black_box(page), black_box(&margins), black_box(DPI));
            let over = overflow(black_box(element), black_box(page));
            (bleed, safe, inner, over)
        });
    });
}

/// FD-06: one snap against 1 000 element boxes, 20 guides, a margin rectangle and the grid.
fn bench_snap_1000(c: &mut Criterion) {
    let elements: Vec<Rect> = (0..1000)
        .map(|i| {
            let (col, row) = (f64::from(i % 40), f64::from(i / 40));
            let (x, y) = (col * 90.0, row * 90.0);
            Rect::new(x, y, x + 80.0, y + 80.0)
        })
        .collect();
    let guides: Vec<Guide> = (0..20)
        .map(|i| Guide {
            axis: if i % 2 == 0 {
                GuideAxis::Vertical
            } else {
                GuideAxis::Horizontal
            },
            position: px(f64::from(i) * 173.0),
            locked: false,
        })
        .collect();
    let grid = GridSettings {
        visible: true,
        step: px(10.0),
        subdivisions: 5,
        style: GridStyle::Lines,
        snap: true,
    };
    let page = Rect::new(0.0, 0.0, 3600.0, 2340.0);
    let input = SnapInput {
        moving: Rect::new(1234.5, 876.25, 1334.5, 976.25),
        page,
        margins: Some(Rect::new(48.0, 48.0, 3552.0, 2292.0)),
        elements: &elements,
        guides: &guides,
        guides_visible: true,
        grid: &grid,
        active: true,
        zoom: 1.0,
        threshold: 6.0,
        dpi: DPI,
    };
    c.bench_function("bench_snap_1000", |b| {
        b.iter(|| snap(black_box(&input)));
    });
}

/// FD-07: the lines of a 3840 x 2160 px view at 100 %, the 6 px step that gives the most lines.
fn bench_grid_lines(c: &mut Criterion) {
    let grid = GridSettings {
        visible: true,
        step: px(6.0),
        subdivisions: 1,
        style: GridStyle::Lines,
        snap: true,
    };
    let view = Rect::new(0.0, 0.0, 3840.0, 2160.0);
    c.bench_function("bench_grid_lines", |b| {
        b.iter(|| {
            grid_lines(
                black_box(&grid),
                black_box(view),
                black_box(1.0),
                black_box(DPI),
            )
        });
    });
}

criterion_group!(
    benches,
    bench_guides_generate,
    bench_layout_rects,
    bench_snap_1000,
    bench_grid_lines
);
criterion_main!(benches);
