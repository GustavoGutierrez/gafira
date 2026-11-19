//! The render contract (design P2, P8): `RenderBackend` double, `Deadline`,
//! `RenderReport`, `RenderError`.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;

use gafira_doc::contract::{CancelToken, Cancelled, ParamValues};
use gafira_doc::msg::{Message, MessageKey, ToMessage, is_valid_key};
use gafira_geom::IRect;
use gafira_ports::common::Bytes;
use gafira_ports::render::{
    BackendCaps, BackendKind, ContentKey, Deadline, EffectRegistry, FrameCx, FrameId, MemoryUsage,
    Priority, RenderBackend, RenderError, RenderOp, RenderPlan, RenderQuality, RenderReport,
    TargetDesc, TileCache, TileKey, TileTask, TransitionRegistry,
};
use gafira_raster::effects::ColorAdjust;
use gafira_raster::transitions::Cut;
use gafira_raster::{BufferError, EffectError, ImageBuf, PixelBuf, PixelFormat};

/// A cache double over a `BTreeMap`.
#[derive(Default)]
struct MapCache(BTreeMap<TileKey, Arc<ImageBuf>>);

impl TileCache for MapCache {
    fn get(&self, key: &TileKey) -> Option<Arc<ImageBuf>> {
        self.0.get(key).cloned()
    }
    fn put(&mut self, key: TileKey, tile: Arc<ImageBuf>) {
        self.0.insert(key, tile);
    }
    fn bytes(&self) -> Bytes {
        Bytes(self.0.values().map(|tile| tile_bytes(tile)).sum())
    }
    fn trim(&mut self, budget: Bytes) {
        while self.bytes() > budget {
            let Some(last) = self.0.keys().next_back().copied() else {
                break;
            };
            self.0.remove(&last);
        }
    }
}

/// A backend double. Each task costs one tick of its own clock; it stops when
/// the deadline has passed and counts the tasks left.
struct TickBackend {
    ticks: Duration,
    format: PixelFormat,
    lose_device: bool,
}

impl TickBackend {
    fn new(format: PixelFormat) -> Self {
        Self {
            ticks: Duration::ZERO,
            format,
            lose_device: false,
        }
    }
}

fn tile() -> Arc<ImageBuf> {
    Arc::new(ImageBuf::new(1, 1, PixelBuf::Rgba8(vec![0, 0, 0, 255])).unwrap())
}

/// The bytes a tile holds.
fn tile_bytes(image: &ImageBuf) -> u64 {
    match image.data() {
        PixelBuf::Rgba8(d) => d.len() as u64,
        PixelBuf::RgbaF32(d) => (d.len() * 4) as u64,
        PixelBuf::Alpha8(d) => d.len() as u64,
    }
}

/// The key of the tile in `column`.
fn tile_key(column: u32) -> TileKey {
    TileKey {
        content: ContentKey(1),
        level: 0,
        column,
        row: 0,
    }
}

/// The page region of the tile in `column` (ARQ-149): 256 px per side at the
/// level zero.
fn tile_region(column: u32) -> IRect {
    let x0 = column as i32 * 256;
    IRect::new(x0, 0, x0 + 256, 256)
}

impl RenderBackend for TickBackend {
    fn caps(&self) -> BackendCaps {
        BackendCaps {
            kind: BackendKind::Cpu,
            max_texture: 8192,
            formats: vec![PixelFormat::Rgba8, PixelFormat::RgbaF32],
            gpu_effects: false,
        }
    }
    fn begin_frame(&mut self, target: &TargetDesc) -> Result<FrameCx, RenderError> {
        if self.lose_device {
            return Err(RenderError::DeviceLost);
        }
        self.format = target.format;
        Ok(FrameCx { id: FrameId(7) })
    }
    fn execute(
        &mut self,
        _cx: &mut FrameCx,
        plan: &RenderPlan,
        cache: &mut dyn TileCache,
        deadline: Deadline,
        cancel: &CancelToken,
    ) -> Result<RenderReport, RenderError> {
        let mut done = 0;
        for task in &plan.tasks {
            cancel.check()?;
            if deadline.has_passed(self.ticks) {
                break;
            }
            if cache.get(&task.key).is_none() {
                cache.put(task.key, tile());
            }
            self.ticks += Duration::from_millis(1);
            done += 1;
        }
        Ok(RenderReport {
            done,
            left: plan.tasks.len() - done,
        })
    }
    fn read_back(&mut self, _cx: &FrameCx, region: IRect) -> Result<ImageBuf, RenderError> {
        let (w, h) = (1, 1);
        let _ = region;
        let data = match self.format {
            PixelFormat::RgbaF32 => PixelBuf::RgbaF32(vec![0.0; 4]),
            _ => PixelBuf::Rgba8(vec![0; 4]),
        };
        Ok(ImageBuf::new(w, h, data)?)
    }
    fn memory_usage(&self) -> MemoryUsage {
        MemoryUsage::default()
    }
    fn trim(&mut self, _budget: Bytes) {}
}

fn plan(n: u32) -> RenderPlan {
    RenderPlan {
        quality: RenderQuality::Preview,
        tasks: (0..n)
            .map(|column| TileTask {
                key: tile_key(column),
                priority: Priority::Visible,
                region: tile_region(column),
                ops: vec![RenderOp::PushLayer],
            })
            .collect(),
    }
}

fn target(format: PixelFormat) -> TargetDesc {
    TargetDesc {
        width: 1,
        height: 1,
        origin: (0, 0),
        page: (1, 1),
        format,
    }
}

// ARQ-06, ARQ-20: a backend double built from ports and lower crates only
// runs through `dyn`, and the cache is a trait object.
#[test]
fn arq_06_backend_double_runs_through_dyn() {
    let mut backend: Box<dyn RenderBackend> = Box::new(TickBackend::new(PixelFormat::Rgba8));
    let mut cache = MapCache::default();
    let mut cx = backend.begin_frame(&target(PixelFormat::Rgba8)).unwrap();
    let report = backend
        .execute(
            &mut cx,
            &plan(3),
            &mut cache,
            Deadline::Unbounded,
            &CancelToken::new(),
        )
        .unwrap();
    assert_eq!(report, RenderReport { done: 3, left: 0 });
    assert!(report.is_complete());
    assert_eq!(cache.0.len(), 3);
    assert_eq!(backend.caps().kind, BackendKind::Cpu);
}

// 02 §6.2: a late render is a report with `left`, not an error.
#[test]
fn p2_late_render_is_a_report_not_an_error() {
    let mut backend = TickBackend::new(PixelFormat::Rgba8);
    let mut cache = MapCache::default();
    let mut cx = backend.begin_frame(&target(PixelFormat::Rgba8)).unwrap();
    let report = backend
        .execute(
            &mut cx,
            &plan(5),
            &mut cache,
            Deadline::At(Duration::from_millis(2)),
            &CancelToken::new(),
        )
        .unwrap();
    assert_eq!(report, RenderReport { done: 2, left: 3 });
    assert!(!report.is_complete());
}

#[test]
fn p2_cancel_is_an_error_carrying_cancelled() {
    let mut backend = TickBackend::new(PixelFormat::Rgba8);
    let mut cache = MapCache::default();
    let mut cx = backend.begin_frame(&target(PixelFormat::Rgba8)).unwrap();
    let cancel = CancelToken::new();
    cancel.cancel();
    let err = backend
        .execute(&mut cx, &plan(1), &mut cache, Deadline::Unbounded, &cancel)
        .unwrap_err();
    assert!(matches!(err, RenderError::Cancelled(Cancelled)));
}

// S13: `read_back` gives `Rgba8` or `RgbaF32`.
#[test]
fn s13_read_back_gives_rgba8_or_rgba_f32() {
    for format in [PixelFormat::Rgba8, PixelFormat::RgbaF32] {
        let mut backend = TickBackend::new(format);
        let cx = backend.begin_frame(&target(format)).unwrap();
        let img = backend.read_back(&cx, IRect::new(0, 0, 1, 1)).unwrap();
        let got = match img.data() {
            PixelBuf::Rgba8(_) => PixelFormat::Rgba8,
            PixelBuf::RgbaF32(_) => PixelFormat::RgbaF32,
            PixelBuf::Alpha8(_) => PixelFormat::Alpha8,
        };
        assert_eq!(got, format);
    }
}

// ARQ-06 (02 §4.6): `RenderOp` is closed. This `match` has no wildcard, so it
// stops compiling if a variant is added or `#[non_exhaustive]` is put on it.
#[test]
fn arq_06_render_op_is_closed() {
    fn name(op: &RenderOp) -> &'static str {
        match op {
            RenderOp::PushLayer => "push_layer",
            RenderOp::PopLayer { .. } => "pop_layer",
            RenderOp::FillPath { .. } => "fill_path",
            // Spec 063: added on purpose, with every backend handling it.
            RenderOp::FillGradient { .. } => "fill_gradient",
            RenderOp::DrawImage { .. } => "draw_image",
            RenderOp::ApplyMask { .. } => "apply_mask",
            RenderOp::ApplyEffect { .. } => "apply_effect",
            RenderOp::ApplyTransition { .. } => "apply_transition",
        }
    }
    assert_eq!(name(&RenderOp::PushLayer), "push_layer");
}

#[test]
fn p2_priority_orders_interactive_first() {
    assert!(Priority::Interactive < Priority::Visible);
    assert!(Priority::Visible < Priority::Prefetch);
    assert!(Priority::Prefetch < Priority::Background);
}

// P8 Deadline::has_passed (02 §6.2). Times in nanoseconds.
#[test]
fn p8_deadline_has_passed_table() {
    let ns = Duration::from_nanos;
    let at = Deadline::At(ns(1_000));
    let cases = [
        (at, ns(999), false),
        (at, ns(1_000), true),
        (at, ns(1_001), true),
        (Deadline::Unbounded, ns(0), false),
        (Deadline::Unbounded, Duration::MAX, false),
    ];
    for (deadline, now, expected) in cases {
        assert_eq!(
            deadline.has_passed(now),
            expected,
            "{deadline:?} at {now:?}"
        );
    }
}

// P8 RenderReport::is_complete: true only when `left == 0`.
#[test]
fn p8_report_is_complete_only_when_nothing_is_left() {
    for (done, left, expected) in [(0, 0, true), (5, 0, true), (4, 1, false), (0, 3, false)] {
        assert_eq!(
            RenderReport { done, left }.is_complete(),
            expected,
            "left {left}"
        );
    }
    assert!(RenderReport::default().is_complete());
}

fn all_errors() -> Vec<(RenderError, &'static str)> {
    vec![
        (RenderError::DeviceLost, "error.render.device_lost"),
        (RenderError::OutOfMemory, "error.render.out_of_memory"),
        (RenderError::AdapterMissing, "error.render.adapter_missing"),
        (RenderError::DriverBlocked, "error.render.driver_blocked"),
        (
            RenderError::Effect(EffectError::UnsupportedFormat {
                format: PixelFormat::Alpha8,
            }),
            "error.raster.unsupported_format",
        ),
        (
            RenderError::Buffer(BufferError::SizeOverflow {
                width: 1,
                height: 2,
            }),
            "error.raster.size_overflow",
        ),
        (RenderError::Cancelled(Cancelled), "error.job.cancelled"),
    ]
}

// ARQ-22: the four failures a selector falls back on are told apart, by
// variant, by message key and by display text.
#[test]
fn arq_22_fallback_failures_are_told_apart() {
    let four = [
        RenderError::DeviceLost,
        RenderError::OutOfMemory,
        RenderError::AdapterMissing,
        RenderError::DriverBlocked,
    ];
    let mut keys: Vec<_> = four.iter().map(|e| e.to_message().key).collect();
    let mut texts: Vec<_> = four.iter().map(ToString::to_string).collect();
    keys.sort_by_key(|k| k.0);
    keys.dedup();
    texts.sort();
    texts.dedup();
    assert_eq!(keys.len(), 4);
    assert_eq!(texts.len(), 4);
    let mut backend = TickBackend::new(PixelFormat::Rgba8);
    backend.lose_device = true;
    assert!(matches!(
        backend.begin_frame(&target(PixelFormat::Rgba8)),
        Err(RenderError::DeviceLost)
    ));
}

// ARQ-102 (ADR 0012, 0014): one fixture per variant.
#[test]
fn arq_102_render_error_messages() {
    for (err, key) in all_errors() {
        let m: Message = err.to_message();
        assert!(is_valid_key(m.key.0), "{:?}", m.key);
        assert_eq!(m.key, MessageKey(key), "{err}");
        if matches!(
            err,
            RenderError::DeviceLost
                | RenderError::OutOfMemory
                | RenderError::AdapterMissing
                | RenderError::DriverBlocked
        ) {
            assert!(m.key.0.starts_with("error.render."));
            assert!(m.args.is_empty());
        }
    }
}

// ADR 0012 rule 4: `?` works from `Cancelled`, and wrapped errors delegate.
#[test]
fn arq_102_wrapped_errors_delegate_and_convert() {
    let from_cancel: RenderError = Cancelled.into();
    assert_eq!(from_cancel.to_message(), Cancelled.to_message());
    let effect = EffectError::UnsupportedFormat {
        format: PixelFormat::Alpha8,
    };
    assert_eq!(
        RenderError::from(effect).to_message(),
        EffectError::UnsupportedFormat {
            format: PixelFormat::Alpha8
        }
        .to_message()
    );
    let buffer = BufferError::SizeOverflow {
        width: 3,
        height: 4,
    };
    assert_eq!(RenderError::from(buffer).to_message(), buffer.to_message());
}

// P4: the aliases hold raster traits, register by id and refuse a repeat; what
// they hold goes straight into a plan op.
#[test]
fn p4_effect_and_transition_registries_register_by_id() {
    let mut effects = EffectRegistry::new();
    effects.register(Arc::new(ColorAdjust::new())).unwrap();
    let id = effects.iter().next().unwrap().descriptor().id;
    assert!(effects.register(Arc::new(ColorAdjust::new())).is_err());
    assert_eq!(effects.len(), 1);
    let effect = Arc::clone(effects.get(id).unwrap());
    let op = RenderOp::ApplyEffect {
        effect,
        params: ParamValues::default(),
        scale: 1.0,
    };
    assert!(matches!(op, RenderOp::ApplyEffect { .. }));

    let mut transitions = TransitionRegistry::new();
    transitions.register(Arc::new(Cut::new())).unwrap();
    assert!(transitions.register(Arc::new(Cut::new())).is_err());
    assert!(transitions.get("transition.cut").is_some());
    assert!(transitions.get(id).is_none());
}

// ARQ-44: a cache tells the governor how much it holds and can be trimmed to
// a budget; both are in bytes. The calls go through the trait, so this also
// proves the trait declares them.
#[test]
fn arq_44_tile_cache_reports_bytes_and_trims_to_a_budget() {
    let mut cache: Box<dyn TileCache> = Box::new(MapCache::default());
    assert_eq!(cache.bytes(), Bytes(0));

    for column in 0..3 {
        cache.put(tile_key(column), tile());
    }
    // Three 1x1 Rgba8 tiles: four bytes each.
    assert_eq!(cache.bytes(), Bytes(12));

    // A budget above the current size changes nothing.
    cache.trim(Bytes(1_000));
    assert_eq!(cache.bytes(), Bytes(12));

    // A trim to half the current size leaves at most half of it.
    cache.trim(Bytes(6));
    assert!(cache.bytes() <= Bytes(6), "left {}", cache.bytes().0);

    // A trim to zero empties the cache.
    cache.trim(Bytes(0));
    assert_eq!(cache.bytes(), Bytes(0));
}
