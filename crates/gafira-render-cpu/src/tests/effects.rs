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
};

use super::*;

/// What the probe effect does to its input.
#[derive(Clone, Copy)]
enum Shape {
    /// `dst(x, y) = src(x − d, y − d)`, clamped at the top-left: it reads its
    /// input from the far side of the tile's own edge.
    Shift(i32),
    /// The identity over a region `p` larger than `out` on every side.
    Pad(i32),
}

/// An effect that records how it was called.
struct Probe {
    descriptor: Descriptor,
    schema: ParamSchema,
    shape: Shape,
    calls: Mutex<usize>,
    seen: Mutex<Vec<(u32, u32)>>,
    cancel: Option<CancelToken>,
}

impl Probe {
    fn new(shape: Shape) -> Self {
        Self {
            descriptor: Descriptor::new("test.probe", MessageKey("test.probe"), 1),
            schema: ParamSchema::new(Vec::new()),
            shape,
            calls: Mutex::new(0),
            seen: Mutex::new(Vec::new()),
            cancel: None,
        }
    }

    fn cancelling(shape: Shape, cancel: CancelToken) -> Self {
        Self {
            cancel: Some(cancel),
            ..Self::new(shape)
        }
    }

    fn calls(&self) -> usize {
        *self.calls.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// The `src` sizes the effect was handed, in call order.
    fn seen(&self) -> Vec<(u32, u32)> {
        self.seen.lock().unwrap_or_else(|e| e.into_inner()).clone()
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
        }
    }

    fn cost_hint(&self, _cx: &EffectCx, _p: &ParamValues, _px: u64) -> Cost {
        Cost { units: 0 }
    }

    fn run_cpu(
        &self,
        _cx: &EffectCx,
        _p: &ParamValues,
        src: ImageView<'_>,
        mut dst: ImageViewMut<'_>,
    ) -> Result<(), EffectError> {
        *self.calls.lock().unwrap_or_else(|e| e.into_inner()) += 1;
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
            Shape::Pad(_) => 0,
        };
        let target = match dst.data_mut() {
            PixelDataMut::RgbaF32(data) => data,
            _ => panic!("the effect writes premultiplied f32"),
        };
        for y in 0..height {
            for x in 0..width {
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

fn effect_op(probe: &Arc<Probe>) -> RenderOp {
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
    let target_desc = target(16, 16, (0, 0), PixelFormat::RgbaF32);
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

// ARQ-120: the grown rectangle is the effect's request, clipped to the page.
#[test]
fn arq_120_the_grown_rectangle_is_clipped_to_the_page() {
    let target_desc = target(16, 16, (0, 0), PixelFormat::RgbaF32);
    let probe = Arc::new(Probe::new(Shape::Pad(100)));
    let scene = quadrant(vec![fill(0.0, 0.0, 16.0, 16.0, white()), effect_op(&probe)]);
    let got = run(target_desc, &scene);

    assert_eq!(probe.calls(), 1);
    assert_eq!(
        probe.seen(),
        vec![(16, 16)],
        "an asked-for 100 px pad is clipped to the page"
    );
    assert!(is_white(&got, 8, 8), "the identity leaves the tile alone");
    assert!(is_white(&got, 15, 15), "the crop is the whole tile");
    assert!(is_clear(&got, 7, 7), "outside the tile stays unwritten");
}

// ---------------------------------------------------------------------------
// AC5 — a cancelled effect leaves nothing behind
// ---------------------------------------------------------------------------

// ARQ-112, ARQ-118: the effect receives no token, so the checkpoint is between
// the run and the commit. A cancel raised inside the effect aborts the tile
// before it reaches the surface or the cache, and the layer keeps what it had.
#[test]
fn arq_112_a_cancelled_effect_aborts_the_tile_before_it_is_committed() {
    let target_desc = target(16, 16, (0, 0), PixelFormat::RgbaF32);
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
