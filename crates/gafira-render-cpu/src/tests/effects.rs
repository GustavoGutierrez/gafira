//! The effect pass of `CB5`: the tile grown by the effect's `roi`, the effect
//! run once over it, and the tile keeping its own crop (ARQ-120, 02 §4.6).

use std::sync::{Arc, Mutex};

use gafira_doc::contract::{CancelToken, Describe, Descriptor, ParamSchema, ParamValues};
use gafira_doc::msg::MessageKey;
use gafira_geom::IRect;
use gafira_ports::render::RenderError;
use gafira_raster::effects::{Cost, Effect};
use gafira_raster::{
    EffectCx, EffectError, ImageView, ImageViewMut, PixelData, PixelDataMut, PixelFormat,
    TileWindow,
};

use super::*;

/// What the probe effect does to its input.
#[derive(Clone, Copy)]
pub(super) enum Shape {
    /// `dst(x, y) = src(x − d, y − d)`, clamped at the top-left: it reads its
    /// input from the far side of the tile's own edge.
    Shift(i32),
    /// The identity over a region `p` larger than `out` on every side.
    Pad(i32),
    /// Writes its destination's first pixel and leaves the rest as it found
    /// it: a sparse writer, which `Effect`'s contract allows (only an error
    /// must leave `dst` untouched).
    Dot,
}

/// An effect that records how it was called.
pub(super) struct Probe {
    descriptor: Descriptor,
    schema: ParamSchema,
    shape: Shape,
    calls: Mutex<usize>,
    seen: Mutex<Vec<(u32, u32)>>,
    windows: Mutex<Vec<TileWindow>>,
    cancel: Option<CancelToken>,
}

pub(super) type ProbeEffect = Probe;

impl Probe {
    pub(super) fn new(shape: Shape) -> Self {
        Self {
            descriptor: Descriptor::new("test.probe", MessageKey("test.probe"), 1),
            schema: ParamSchema::new(Vec::new()),
            shape,
            calls: Mutex::new(0),
            seen: Mutex::new(Vec::new()),
            windows: Mutex::new(Vec::new()),
            cancel: None,
        }
    }

    fn cancelling(shape: Shape, cancel: CancelToken) -> Self {
        Self {
            cancel: Some(cancel),
            ..Self::new(shape)
        }
    }

    pub(super) fn calls(&self) -> usize {
        *self.calls.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// The `src` sizes the effect was handed, in call order.
    pub(super) fn seen(&self) -> Vec<(u32, u32)> {
        self.seen.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }

    /// The windows the effect was asked with, in call order.
    pub(super) fn windows(&self) -> Vec<TileWindow> {
        self.windows
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }
}

impl Describe for Probe {
    fn descriptor(&self) -> &Descriptor {
        &self.descriptor
    }
}

impl Effect for Probe {
    fn params(&self) -> &ParamSchema {
        &self.schema
    }

    fn roi(&self, _cx: &EffectCx, _p: &ParamValues, out: IRect) -> IRect {
        match self.shape {
            Shape::Shift(d) => IRect::new(out.x0 - d, out.y0 - d, out.x1 - d, out.y1 - d),
            Shape::Pad(p) => out.inflate(p, p),
            Shape::Dot => out,
        }
    }

    fn cost_hint(&self, _cx: &EffectCx, _p: &ParamValues, _px: u64) -> Cost {
        Cost { units: 0 }
    }

    fn run_cpu(
        &self,
        cx: &EffectCx,
        _p: &ParamValues,
        src: ImageView<'_>,
        mut dst: ImageViewMut<'_>,
    ) -> Result<(), EffectError> {
        *self.calls.lock().unwrap_or_else(|e| e.into_inner()) += 1;
        if let Some(window) = cx.tile {
            self.windows
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .push(window);
        }
        self.seen
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .push((src.width(), src.height()));
        let source = match src.data() {
            PixelData::RgbaF32(data) => data.to_vec(),
            other => panic!("the effect runs on premultiplied f32, not {other:?}"),
        };
        let width = usize::try_from(dst.width()).unwrap_or(0);
        let height = usize::try_from(dst.height()).unwrap_or(0);
        let shift = match self.shape {
            Shape::Shift(d) => i64::from(d),
            Shape::Pad(_) | Shape::Dot => 0,
        };
        let target = match dst.data_mut() {
            PixelDataMut::RgbaF32(data) => data,
            _ => panic!("the effect writes premultiplied f32"),
        };
        for y in 0..height {
            for x in 0..width {
                // A sparse writer: the contract forbids only an **error** from
                // writing, so leaving the rest of `dst` alone is legal.
                if matches!(self.shape, Shape::Dot) && (x, y) != (0, 0) {
                    continue;
                }
                let read_x =
                    usize::try_from((i64::try_from(x).unwrap_or(0) - shift).max(0)).unwrap_or(0);
                let read_y =
                    usize::try_from((i64::try_from(y).unwrap_or(0) - shift).max(0)).unwrap_or(0);
                let read_x = read_x.min(width.saturating_sub(1));
                let read_y = read_y.min(height.saturating_sub(1));
                let from = (read_y * width + read_x) * 4;
                let at = (y * width + x) * 4;
                target[at..at + 4].copy_from_slice(&source[from..from + 4]);
            }
        }
        if let Some(token) = &self.cancel {
            token.cancel();
        }
        Ok(())
    }
}

pub(super) fn effect_op(probe: &Arc<Probe>) -> RenderOp {
    RenderOp::ApplyEffect {
        effect: Arc::clone(probe) as Arc<dyn Effect>,
        params: ParamValues::new(),
        scale: 1.0,
    }
}

/// A 16 × 16 page whose only tile is the bottom-right quadrant.
fn quadrant(ops: Vec<RenderOp>) -> RenderPlan {
    plan(vec![task(1, IRect::new(8, 8, 16, 16), ops)])
}

// ---------------------------------------------------------------------------
// AC4 — the effect reads the grown rectangle and the tile keeps its crop
// ---------------------------------------------------------------------------

// ARQ-120, 02 §4.6: `roi` decides how far outside the tile the effect must
// read, the tile's ops are rendered over that grown rectangle, the effect runs
// once over it, and the tile keeps its own crop. The fill ends exactly at the
// tile's top-left, so the pixel the effect pulls into the tile can only come
// from outside it.
#[test]
fn arq_120_the_effect_reads_past_the_tile_edge_and_runs_once() {
    let target_desc = target(16, 16, (0, 0), (16, 16), PixelFormat::RgbaF32);
    let probe = Arc::new(Probe::new(Shape::Shift(4)));
    let scene = quadrant(vec![fill(0.0, 0.0, 8.0, 8.0, white()), effect_op(&probe)]);
    let got = run(target_desc, &scene);

    assert_eq!(probe.calls(), 1, "the effect runs once over the grown tile");
    // The tile is (8, 8)-(16, 16) and the input region reaches (4, 4), so the
    // grown rectangle is (4, 4)-(16, 16).
    assert_eq!(probe.seen(), vec![(12, 12)], "the grown rectangle");
    // The crop's first pixel reads the grown buffer's (4, 4), which is the
    // page's (4, 4) — inside the fill and outside the tile.
    assert!(is_white(&got, 8, 8), "the crop reads outside the tile");
    assert!(is_white(&got, 8, 10), "the crop reads outside the tile");
    // The crop's own origin: a crop taken at the buffer's top-left instead of
    // at the tile's offset inside it would paint the fill here.
    assert!(
        is_clear(&got, 12, 12),
        "the crop starts at the tile's offset in the grown buffer"
    );
    assert!(is_clear(&got, 7, 7), "outside the tile stays unwritten");

    // The same scene with no effect at all: the tile holds nothing, so the
    // pixels above are the effect's and not the fill's.
    let without = run(
        target_desc,
        &quadrant(vec![fill(0.0, 0.0, 8.0, 8.0, white())]),
    );
    assert!(
        is_clear(&without, 8, 8),
        "without the effect the tile is empty"
    );
}

// ARQ-120: an effect's `roi` is asked against the buffer it filters, so a
// chain of effects composes. The first asks the tile and grows the buffer; the
// second asks **that** buffer, so its input covers what the first produced.
// Measuring every `roi` against the bare tile instead leaves the second one
// two pixels short on each side of the first one's growth.
#[test]
fn arq_120_two_effects_grow_by_the_buffer_the_first_produced() {
    let target_desc = target(32, 32, (0, 0), (32, 32), PixelFormat::RgbaF32);
    let first = Arc::new(Probe::new(Shape::Pad(2)));
    let second = Arc::new(Probe::new(Shape::Shift(4)));
    let scene = plan(vec![task(
        1,
        IRect::new(8, 8, 16, 16),
        vec![
            fill(0.0, 0.0, 32.0, 32.0, white()),
            effect_op(&first),
            effect_op(&second),
        ],
    )]);
    let got = run(target_desc, &scene);

    // The first effect asks `roi` over the tile (8, 8)-(16, 16) and pads it by
    // 2, so the buffer is (6, 6)-(18, 18). The second asks `roi` over that
    // buffer and shifts it by 4, reaching (2, 2)-(14, 14); the union is
    // (2, 2)-(18, 18), 16 x 16. From the tile alone the second asks
    // (4, 4)-(12, 12) and the union is 14 x 14 — the crop this fixes.
    assert_eq!(
        first.seen(),
        vec![(16, 16)],
        "the buffer the first effect produced"
    );
    assert_eq!(
        second.seen(),
        vec![(16, 16)],
        "the second effect reads what the first produced, not the tile"
    );
    assert_eq!(first.calls(), 1, "each effect runs once");
    assert_eq!(second.calls(), 1, "each effect runs once");
    assert!(is_white(&got, 8, 8), "the tile keeps its crop");
    assert!(is_white(&got, 15, 15), "the tile keeps its crop");
}

// ARQ-120: the grown rectangle is the effect's request, clipped to the surface.
#[test]
fn arq_120_the_grown_rectangle_is_clipped_to_the_surface() {
    let target_desc = target(16, 16, (0, 0), (16, 16), PixelFormat::RgbaF32);
    let probe = Arc::new(Probe::new(Shape::Pad(100)));
    let scene = quadrant(vec![fill(0.0, 0.0, 16.0, 16.0, white()), effect_op(&probe)]);
    let got = run(target_desc, &scene);

    assert_eq!(probe.calls(), 1);
    assert_eq!(
        probe.seen(),
        vec![(16, 16)],
        "an asked-for 100 px pad is clipped to the surface"
    );
    assert!(is_white(&got, 8, 8), "the identity leaves the tile alone");
    assert!(is_white(&got, 15, 15), "the crop is the whole tile");
    assert!(is_clear(&got, 7, 7), "outside the tile stays unwritten");
}

// 02 §6.2, ARQ-120: on a grown tile the cull's rectangle is the **grown** one,
// and that is a pixel rule, not only work skipped. A fill strictly inside the
// margin — outside the tile and inside what the effect reads — must still be
// drawn, or the effect pulls transparency into the tile. Culling at
// `task.region` drops it and the tile loses the pixel it was grown for.
#[test]
fn arq_120_a_fill_inside_the_margin_reaches_the_tile_through_the_effect() {
    let target_desc = target(16, 16, (0, 0), (16, 16), PixelFormat::RgbaF32);
    let probe = Arc::new(Probe::new(Shape::Shift(4)));
    // The tile is (8, 8)-(16, 16) and the effect reads 4 px back, so the grown
    // rectangle is (4, 4)-(16, 16). The fill is (4, 4)-(6, 6): wholly outside
    // the tile and wholly inside the growth.
    let scene = quadrant(vec![fill(4.0, 4.0, 6.0, 6.0, white()), effect_op(&probe)]);
    let got = run(target_desc, &scene);

    assert_eq!(probe.seen(), vec![(12, 12)], "the grown rectangle");
    assert!(
        is_white(&got, 8, 8),
        "the fill inside the margin reaches the tile through the effect"
    );
    // The control: the same scene with the fill outside the grown rectangle
    // leaves the pixel clear, so the pixel above is the fill's and not a
    // default the effect writes.
    let without = run(
        target_desc,
        &quadrant(vec![fill(0.0, 0.0, 2.0, 2.0, white()), effect_op(&probe)]),
    );
    assert!(
        is_clear(&without, 8, 8),
        "outside the grown rectangle the tile keeps the transparent page"
    );
}

// ---------------------------------------------------------------------------
// O18 — the window an effect is handed is page space
// ---------------------------------------------------------------------------

// ARQ-120, ARQ-149, O18, 02 §4.4: `image_width`/`image_height` are the whole
// **page's** extent and `x`/`y` are the page pixel coordinates of `src`'s first
// pixel. A surface that is a sub-region of the page is the case the surface's
// own rectangle gets wrong on both counts; a full-page surface at `(0, 0)` is
// the case where the two readings coincide, so the export path's window is the
// one `CB5` produced.
#[test]
fn arq_120_the_window_is_the_page_when_the_surface_is_a_sub_region() {
    // A 16 x 16 viewport at (8, 8) of a 32 x 32 page. The surface's rectangle
    // is (8, 8)-(24, 24); the page's is (0, 0)-(32, 32).
    let viewport = target(16, 16, (8, 8), (32, 32), PixelFormat::RgbaF32);
    let probe = Arc::new(Probe::new(Shape::Pad(2)));
    let scene = plan(vec![task(
        1,
        IRect::new(8, 8, 24, 24),
        vec![fill(0.0, 0.0, 32.0, 32.0, white()), effect_op(&probe)],
    )]);
    run(viewport, &scene);

    assert_eq!(
        probe.windows(),
        vec![TileWindow {
            image_width: 32,
            image_height: 32,
            x: 8,
            y: 8,
        }],
        "the window names the 32 x 32 page, and src starts at its (8, 8)"
    );

    // The full-page surface of the export: the surface is the page, so the
    // window is exactly what it was before the page extent reached the port.
    let full = target(16, 16, (0, 0), (16, 16), PixelFormat::RgbaF32);
    let probe = Arc::new(Probe::new(Shape::Pad(2)));
    let scene = plan(vec![task(
        1,
        IRect::new(0, 0, 16, 16),
        vec![fill(0.0, 0.0, 16.0, 16.0, white()), effect_op(&probe)],
    )]);
    run(full, &scene);

    assert_eq!(
        probe.windows(),
        vec![TileWindow {
            image_width: 16,
            image_height: 16,
            x: 0,
            y: 0,
        }],
        "the export path's window does not move"
    );
}

// ---------------------------------------------------------------------------
// O17 — the effect's destination is the backend's, accounted and dropable
// ---------------------------------------------------------------------------

// O17, ARQ-143, ARQ-44 (partial): an effect's destination is one backend-owned
// scratch, reused by the ops of the tile, counted in `memory_usage` beside the
// surface and the layer pool, and dropped by `trim` — `trim` runs between
// frames, which is the only moment a lever can exist. The plan carries two
// effects, so a per-op buffer would double the term.
#[test]
fn arq_143_the_effect_scratch_is_accounted_and_trim_drops_it() {
    let target_desc = target(32, 32, (0, 0), (32, 32), PixelFormat::RgbaF32);
    let first = Arc::new(Probe::new(Shape::Pad(2)));
    let second = Arc::new(Probe::new(Shape::Shift(4)));
    let scene = plan(vec![task(
        1,
        IRect::new(8, 8, 16, 16),
        vec![
            fill(0.0, 0.0, 32.0, 32.0, white()),
            effect_op(&first),
            effect_op(&second),
        ],
    )]);
    let mut frame = Frame::open(CpuBackend::new(), target_desc);
    let mut cache = RecordingCache::default();
    frame
        .execute(&scene, &mut cache, &CancelToken::new())
        .expect("the plan runs");

    let surface = 32 * 32 * 16;
    let cached = 8 * 8 * 16;
    // The two effects compose to a 16 x 16 buffer, and the one scratch serves
    // both of them: a destination per op would be 2 x this term.
    let scratch = 16 * 16 * 16;
    assert_eq!(
        frame.backend.memory_usage().host,
        Bytes(surface + cached + scratch),
        "the surface, the cache and the one effect destination"
    );
    assert_eq!(frame.backend.memory_usage().device, Bytes(0));

    // A budget that covers what is held keeps the scratch; a tighter one drops
    // it with the frame and the accounting of the cache (ARQ-44).
    frame.backend.trim(Bytes(surface + cached + scratch));
    assert_eq!(
        frame.backend.memory_usage().host,
        Bytes(surface + cached + scratch),
        "a budget that covers what is held drops nothing"
    );
    frame.backend.trim(Bytes(0));
    assert_eq!(
        frame.backend.memory_usage().host,
        Bytes(0),
        "trim drops the scratch with the surface and the pool"
    );
}

// O17, D-03, ARQ-44 (partial): the peak — the surface, the layer bytes and the
// effect's destination — is refused **before** the buffer is allocated, and the
// cap is a ceiling: exactly at `SURFACE_BUDGET` fits, one byte over does not.
#[test]
fn arq_44_the_effect_peak_is_refused_before_the_scratch_is_allocated() {
    use crate::surface::SURFACE_BUDGET;

    let pixels = 16usize;
    let bytes = 64u64;
    // The boundary fits: the frame's held bytes plus the new buffer are exactly
    // the cap, and the cap is inclusive.
    let mut scratch = Vec::new();
    crate::effect::reserve_scratch(
        &mut scratch,
        pixels,
        SURFACE_BUDGET.0 - bytes,
        SURFACE_BUDGET,
    )
    .expect("the cap is inclusive");
    assert_eq!(scratch.len(), pixels, "the buffer is grown at the cap");

    // One byte further is over, and the refusal precedes the allocation.
    let mut refused = Vec::new();
    assert!(
        matches!(
            crate::effect::reserve_scratch(
                &mut refused,
                pixels,
                SURFACE_BUDGET.0 - bytes + 1,
                SURFACE_BUDGET
            ),
            Err(RenderError::OutOfMemory)
        ),
        "a peak past the cap is refused"
    );
    assert!(refused.is_empty(), "the refusal precedes the allocation");

    // The case O17 names: the 8192 x 8192 surface of `MAX_SIDE` holds 1.07 GB,
    // and a page-sized destination would double it past the cap. The numbers
    // are the guard's; nothing is allocated to refuse them.
    let page = 8192usize * 8192;
    let mut big = Vec::new();
    assert!(
        matches!(
            crate::effect::reserve_scratch(&mut big, page * 4, page as u64 * 16, SURFACE_BUDGET),
            Err(RenderError::OutOfMemory)
        ),
        "the doubled page is refused"
    );
    assert!(
        big.is_empty(),
        "the 2.14 GB peak is refused before it exists"
    );
}

// O17, ARQ-111: the destination is the backend's and is reused by the ops of a
// tile, so nothing a previous op wrote may reach this one — the effect starts
// from transparent black, as a freshly allocated buffer would. The second
// effect writes its first pixel and leaves the rest, which is exactly where the
// first one's bytes would surface; removing the clearing turns the tile white.
#[test]
fn arq_111_a_reused_destination_starts_cleared_for_the_next_op() {
    let target_desc = target(16, 16, (0, 0), (16, 16), PixelFormat::RgbaF32);
    let writer = Arc::new(Probe::new(Shape::Pad(0)));
    let partial = Arc::new(Probe::new(Shape::Dot));
    let scene = quadrant(vec![
        fill(0.0, 0.0, 16.0, 16.0, white()),
        effect_op(&writer),
        effect_op(&partial),
    ]);
    let got = run(target_desc, &scene);

    // The first effect writes the whole tile-sized destination; the second
    // reuses the same scratch and writes one pixel of it.
    assert_eq!(
        writer.seen(),
        vec![(8, 8)],
        "the first fills the destination"
    );
    assert_eq!(partial.seen(), vec![(8, 8)], "the second reuses it");
    assert!(is_white(&got, 8, 8), "the pixel the second op wrote");
    assert!(
        is_clear(&got, 9, 9),
        "the pixel the second op left holds no byte of the first"
    );
    assert!(
        is_clear(&got, 15, 15),
        "and the tile's far corner is clear too"
    );
}

// O17, D-03, ARQ-44: the refusal is the backend's own path, not only the
// reservation's. `execute` puts the surface and the open layers into `held` and
// hands the effect's peak to the same comparison; dropping either term leaves
// the peak under the cap and the tile runs, which is what `CB5`'s round could
// not see. The cap is the one thing lowered here, so the frame reaches it with
// real buffers instead of a gibibyte of them.
#[test]
fn arq_44_execute_refuses_the_effect_peak_near_the_cap() {
    let target_desc = target(16, 16, (0, 0), (16, 16), PixelFormat::RgbaF32);
    // One tile half the page wide, its fill inside a layer, and the effect on
    // the layer: the frame holds the surface, the open layer and the
    // destination at the effect op.
    let scene = |probe: &Arc<Probe>| {
        plan(vec![task(
            1,
            IRect::new(0, 0, 8, 16),
            vec![
                push(),
                fill(0.0, 0.0, 8.0, 16.0, white()),
                effect_op(probe),
                pop(BlendMode::Normal, 1.0),
            ],
        )])
    };
    // (16 x 16 surface + 8 x 16 layer + 8 x 16 destination) x 16 bytes.
    let peak = 16 * 16 * 16 + 8 * 16 * 16 * 2;

    // Exactly at the cap the frame runs: the comparison is inclusive.
    let probe = Arc::new(Probe::new(Shape::Pad(0)));
    let mut cache = RecordingCache::default();
    let mut frame = Frame::open(CpuBackend::new().with_budget(Bytes(peak)), target_desc);
    let outcome = frame.execute(&scene(&probe), &mut cache, &CancelToken::new());
    assert!(outcome.is_ok(), "the cap is inclusive: {outcome:?}");
    assert_eq!(probe.calls(), 1, "the effect runs at the cap");

    // One byte over, and `execute` answers the refusal itself — before the
    // effect is reached.
    let probe = Arc::new(Probe::new(Shape::Pad(0)));
    let mut cache = RecordingCache::default();
    let mut frame = Frame::open(CpuBackend::new().with_budget(Bytes(peak - 1)), target_desc);
    let outcome = frame.execute(&scene(&probe), &mut cache, &CancelToken::new());
    assert!(
        matches!(outcome, Err(RenderError::OutOfMemory)),
        "the backend answers the refusal: {outcome:?}"
    );
    assert_eq!(probe.calls(), 0, "the refusal precedes the effect");
}

// ---------------------------------------------------------------------------
// AC5 — a cancelled effect leaves nothing behind
// ---------------------------------------------------------------------------

// ARQ-112, ARQ-118: the effect receives no token, so the checkpoint is between
// the run and the commit. A cancel raised inside the effect aborts the tile
// before it reaches the surface or the cache, and the layer keeps what it had.
#[test]
fn arq_112_a_cancelled_effect_aborts_the_tile_before_it_is_committed() {
    let target_desc = target(16, 16, (0, 0), (16, 16), PixelFormat::RgbaF32);
    let token = CancelToken::new();
    let probe = Arc::new(Probe::cancelling(Shape::Shift(4), token.clone()));
    let scene = quadrant(vec![fill(0.0, 0.0, 16.0, 16.0, white()), effect_op(&probe)]);

    let mut cache = RecordingCache::default();
    let mut frame = Frame::open(CpuBackend::new(), target_desc);
    let outcome = frame.execute(&scene, &mut cache, &token);
    assert!(
        matches!(outcome, Err(RenderError::Cancelled(_))),
        "the tile answers Cancelled: {outcome:?}"
    );
    assert!(
        cache.puts().is_empty(),
        "a cancelled tile never reaches the cache"
    );

    // The control: the same scene with a live token commits one tile.
    let live = CancelToken::new();
    let probe = Arc::new(Probe::new(Shape::Shift(4)));
    let scene = quadrant(vec![fill(0.0, 0.0, 16.0, 16.0, white()), effect_op(&probe)]);
    let mut cache = RecordingCache::default();
    let mut frame = Frame::open(CpuBackend::new(), target_desc);
    let outcome = frame.execute(&scene, &mut cache, &live);
    assert!(outcome.is_ok(), "the control runs");
    assert_eq!(cache.puts().len(), 1, "the control commits one tile");
}
