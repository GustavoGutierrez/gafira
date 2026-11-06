//! The memory governor of 02 §6.7 (line 1515) and design P7, P17.
//!
//! Every trimmable cache reports its `bytes()` and obeys `trim(budget)`
//! (ARQ-44); the governor owns the order and the arithmetic. It does not own
//! the caches: the frame loop passes them to [`MemoryGovernor::enforce`] on
//! each call, and the governor counts only what it is given (not the history,
//! not the document).
//!
//! The governor is pure (P1): no thread, no timer, no wait. A cache that cannot
//! shrink is tried at most [`MAX_TRIM_PASSES`] times per call; the next try is
//! the host's next `enforce`.

use gafira_ports::common::Bytes;
use gafira_ports::render::{RenderBackend, TileCache};

use crate::engine::{EngineConfig, EngineError};
use crate::event::{EngineEvent, EventBus};

/// The most passes one [`MemoryGovernor::enforce`] runs over the caches, so at
/// most `MAX_TRIM_PASSES * caches.len()` `trim` calls per call (P17).
pub const MAX_TRIM_PASSES: usize = 2;

/// A cache the governor can shrink (ARQ-44).
pub trait Trimmable {
    /// Memory held now, in bytes.
    fn bytes(&self) -> Bytes;
    /// Free memory until the cache holds at most `budget` bytes. A cache may
    /// stop short; the governor measures what it actually freed.
    fn trim(&mut self, budget: Bytes);
}

/// A [`TileCache`] seen as a [`Trimmable`].
pub struct TileCacheTrim<'a> {
    cache: &'a mut dyn TileCache,
}

impl<'a> TileCacheTrim<'a> {
    /// Wrap `cache` for one `enforce`.
    #[must_use]
    pub fn new(cache: &'a mut dyn TileCache) -> Self {
        Self { cache }
    }
}

impl Trimmable for TileCacheTrim<'_> {
    fn bytes(&self) -> Bytes {
        self.cache.bytes()
    }

    fn trim(&mut self, budget: Bytes) {
        self.cache.trim(budget);
    }
}

/// A [`RenderBackend`] seen as a [`Trimmable`]: its host bytes only; device
/// memory is not governed here (P17, SP8).
pub struct BackendTrim<'a> {
    backend: &'a mut dyn RenderBackend,
}

impl<'a> BackendTrim<'a> {
    /// Wrap `backend` for one `enforce`.
    #[must_use]
    pub fn new(backend: &'a mut dyn RenderBackend) -> Self {
        Self { backend }
    }
}

impl Trimmable for BackendTrim<'_> {
    fn bytes(&self) -> Bytes {
        self.backend.memory_usage().host
    }

    fn trim(&mut self, budget: Bytes) {
        self.backend.trim(budget);
    }
}

/// The trim order (P7, P17): frames first (ARQ-44, ARQ-145), then tiles,
/// which are content-addressed and rebuilt on demand (ARQ-142), then the
/// backend's textures, the most expensive to rebuild.
const TRIM_ORDER: [CacheKind; 3] = [CacheKind::Frames, CacheKind::Tiles, CacheKind::Backend];

/// What a cache is, which fixes when it is trimmed ([`TRIM_ORDER`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CacheKind {
    /// Decoded frames.
    Frames,
    /// Rendered tiles.
    Tiles,
    /// The backend's textures.
    Backend,
}

/// Whether the governed caches fit the limit after an `enforce`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PressureLevel {
    /// Within the limit.
    Normal,
    /// Still above the limit after every pass.
    Critical,
}

/// What one `enforce` did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GovernorOutcome {
    /// The saturating sum of every `bytes()` before any trim.
    pub before: Bytes,
    /// The same sum after the last pass.
    pub after: Bytes,
    /// `Critical` when `after` is still above the limit.
    pub level: PressureLevel,
}

/// Keeps the caches it is given within the declared memory limit.
pub struct MemoryGovernor {
    limit: Bytes,
    /// The last level published; `Normal`, unpublished, at the start.
    level: PressureLevel,
    bus: EventBus,
}

impl MemoryGovernor {
    /// A governor for the limit of `config`, publishing
    /// [`EngineEvent::MemoryPressure`] into `bus`.
    ///
    /// Returns [`EngineError::InvalidConfig`] when `config` declares no memory
    /// limit (design P5, P7).
    pub fn new(config: &EngineConfig, bus: EventBus) -> Result<Self, EngineError> {
        let limit = config.memory_limit().ok_or(EngineError::InvalidConfig {
            detail: "no memory limit is declared",
        })?;
        Ok(Self {
            limit,
            level: PressureLevel::Normal,
            bus,
        })
    }

    /// The limit this governor enforces.
    #[must_use]
    pub fn limit(&self) -> Bytes {
        self.limit
    }

    /// Trim `caches` until their total is within the limit, or until
    /// [`MAX_TRIM_PASSES`] passes are spent.
    ///
    /// The caches are visited in [`TRIM_ORDER`], stably: two caches of one kind
    /// keep the caller's order. A total exactly at the limit is within it and
    /// calls no `trim`; a zero limit trims every cache to 0. Every sum
    /// saturates at `u64::MAX`. A pass after the first runs only when a fresh
    /// sum of every `bytes()` is still above the limit, so this call makes at
    /// most `MAX_TRIM_PASSES * caches.len()` `trim` calls and never waits.
    pub fn enforce(&mut self, caches: &mut [(CacheKind, &mut dyn Trimmable)]) -> GovernorOutcome {
        let mut order: Vec<usize> = (0..caches.len()).collect();
        order.sort_by_key(|&at| rank(caches[at].0));

        let before = total(caches);
        let mut after = before;
        for _ in 0..MAX_TRIM_PASSES {
            if after <= self.limit {
                break;
            }
            trim_pass(caches, &order, after.0 - self.limit.0);
            after = total(caches);
        }
        let level = if after > self.limit {
            PressureLevel::Critical
        } else {
            PressureLevel::Normal
        };
        // Only a change is news: a host calling `enforce` every frame must not
        // flood the bus (P17). "Trimmed and now within" is in the outcome.
        if level != self.level {
            self.level = level;
            self.bus.publish(EngineEvent::MemoryPressure(level));
        }
        GovernorOutcome {
            before,
            after,
            level,
        }
    }
}

/// Where `kind` stands in [`TRIM_ORDER`].
fn rank(kind: CacheKind) -> usize {
    TRIM_ORDER
        .iter()
        .position(|&listed| listed == kind)
        .unwrap_or(TRIM_ORDER.len())
}

/// The saturating sum of every `bytes()`.
fn total(caches: &[(CacheKind, &mut dyn Trimmable)]) -> Bytes {
    Bytes(caches.iter().fold(0, |sum: u64, (_, cache)| {
        sum.saturating_add(cache.bytes().0)
    }))
}

/// One pass in `order` over a positive `excess`: each cache is asked to keep
/// `bytes() - excess`, and the excess drops by what it actually freed; a cache
/// that grew frees 0. The pass stops once the excess reaches 0. Whether a pass
/// runs at all is decided by the caller alone, against the budget.
fn trim_pass(caches: &mut [(CacheKind, &mut dyn Trimmable)], order: &[usize], excess: u64) {
    let mut excess = excess;
    for &at in order {
        let cache = &mut caches[at].1;
        let held = cache.bytes().0;
        cache.trim(Bytes(held.saturating_sub(excess)));
        let freed = held.saturating_sub(cache.bytes().0);
        excess = excess.saturating_sub(freed);
        if excess == 0 {
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;

    use gafira_doc::contract::CancelToken;
    use gafira_geom::IRect;
    use gafira_ports::render::{
        BackendCaps, BackendKind, Deadline, FrameCx, FrameId, MemoryUsage, RenderError, RenderPlan,
        RenderReport, TargetDesc, TileKey,
    };
    use gafira_raster::ImageBuf;

    use super::*;
    use crate::fakes::{config, drain, engine_over, quiet_channel};

    use CacheKind::{Backend, Frames, Tiles};

    const MAX: u64 = u64::MAX;

    /// One `trim` a fake received: the cache's name and target.
    type Trim = (&'static str, u64);

    /// Every `trim` the fakes received, in order.
    type Log = Rc<RefCell<Vec<Trim>>>;

    /// A cache double. With a `script`, `bytes()` answers its readings in order
    /// (the last one repeats) and `trim` only records; otherwise it holds
    /// `held` and frees at most `frees_at_most` per `trim`: `u64::MAX` is an
    /// honest cache, a smaller step one that stops short, `0` one that cannot
    /// shrink at all.
    struct Fake {
        name: &'static str,
        held: u64,
        frees_at_most: u64,
        script: Vec<u64>,
        read: Cell<usize>,
        log: Log,
    }

    impl Fake {
        fn honest(name: &'static str, held: u64, log: &Log) -> Self {
            Self::lying(name, held, MAX, log)
        }

        fn lying(name: &'static str, held: u64, frees_at_most: u64, log: &Log) -> Self {
            Self {
                name,
                held,
                frees_at_most,
                script: Vec::new(),
                read: Cell::new(0),
                log: Rc::clone(log),
            }
        }

        fn stuck(name: &'static str, held: u64, log: &Log) -> Self {
            Self::lying(name, held, 0, log)
        }

        fn scripted(name: &'static str, readings: &[u64], log: &Log) -> Self {
            Self {
                script: readings.to_vec(),
                ..Self::stuck(name, 0, log)
            }
        }
    }

    impl Trimmable for Fake {
        fn bytes(&self) -> Bytes {
            if self.script.is_empty() {
                return Bytes(self.held);
            }
            let at = self.read.get().min(self.script.len() - 1);
            self.read.set(self.read.get() + 1);
            Bytes(self.script[at])
        }

        fn trim(&mut self, budget: Bytes) {
            self.log.borrow_mut().push((self.name, budget.0));
            if self.script.is_empty() {
                let freed = self.held.saturating_sub(budget.0).min(self.frees_at_most);
                self.held -= freed;
            }
        }
    }

    fn log() -> Log {
        Rc::new(RefCell::new(Vec::new()))
    }

    fn governor(limit: u64) -> MemoryGovernor {
        governor_over(&engine_over(quiet_channel()), limit)
    }

    /// A governor that publishes into `engine`'s bus.
    fn governor_over(engine: &crate::Gafira, limit: u64) -> MemoryGovernor {
        let config = config().with_memory_limit(Bytes(limit));
        MemoryGovernor::new(&config, engine.bus().clone()).expect("a governor")
    }

    fn enforce(governor: &mut MemoryGovernor, caches: &mut [(CacheKind, Fake)]) -> GovernorOutcome {
        let mut slots: Vec<(CacheKind, &mut dyn Trimmable)> = caches
            .iter_mut()
            .map(|(kind, cache)| (*kind, cache as &mut dyn Trimmable))
            .collect();
        governor.enforce(&mut slots)
    }

    fn outcome(before: u64, after: u64, level: PressureLevel) -> GovernorOutcome {
        GovernorOutcome {
            before: Bytes(before),
            after: Bytes(after),
            level,
        }
    }

    fn taken(log: &Log) -> Vec<Trim> {
        std::mem::take(&mut *log.borrow_mut())
    }

    /// AC1: frames, then tiles, then backend, whatever the caller's order; two
    /// caches of one kind keep the caller's order.
    #[test]
    fn ac1_caches_are_trimmed_frames_tiles_backend_and_stable_within_a_kind() {
        let log = log();
        let mut caches = [
            (Backend, Fake::honest("backend", 3, &log)),
            (Tiles, Fake::honest("tiles-a", 5, &log)),
            (Frames, Fake::honest("frames-a", 7, &log)),
            (Tiles, Fake::honest("tiles-b", 11, &log)),
            (Frames, Fake::honest("frames-b", 13, &log)),
        ];

        let done = enforce(&mut governor(0), &mut caches);

        assert_eq!(
            taken(&log),
            [
                ("frames-a", 0),
                ("frames-b", 0),
                ("tiles-a", 0),
                ("tiles-b", 0),
                ("backend", 0),
            ]
        );
        assert_eq!(done, outcome(39, 0, PressureLevel::Normal));
    }

    /// AC1: the order within a kind is the caller's even with many caches, where
    /// an unstable sort would reorder caches of equal kind (P17, "sorted
    /// stably").
    #[test]
    fn ac1_many_caches_of_one_kind_keep_the_caller_order() {
        let log = log();
        let kinds = [Backend, Frames, Tiles];
        let mut caches: Vec<(CacheKind, Fake)> = (0..64_usize)
            .map(|at| {
                let name: &'static str = Box::leak(format!("cache-{at}").into_boxed_str());
                (kinds[(at * 7 + at / 5) % 3], Fake::honest(name, 1, &log))
            })
            .collect();
        let expected: Vec<Trim> = [Frames, Tiles, Backend]
            .into_iter()
            .flat_map(|kind| {
                caches
                    .iter()
                    .filter(move |(listed, _)| *listed == kind)
                    .map(|(_, cache)| (cache.name, 0))
            })
            .collect();

        let done = enforce(&mut governor(0), &mut caches);

        assert_eq!(taken(&log), expected);
        assert_eq!(done, outcome(64, 0, PressureLevel::Normal));
    }

    /// AC2: exactly at the budget is within it; one byte above trims exactly
    /// one byte from the first cache in order.
    #[test]
    fn ac2_exactly_at_the_limit_trims_nothing_and_one_byte_above_trims_one_byte() {
        let cases: [(u64, &[Trim], u64); 2] = [(100, &[], 100), (99, &[("frames", 59)], 99)];
        for (limit, trims, after) in cases {
            let log = log();
            let mut caches = [
                (Tiles, Fake::honest("tiles", 40, &log)),
                (Frames, Fake::honest("frames", 60, &log)),
            ];

            let done = enforce(&mut governor(limit), &mut caches);

            assert_eq!(taken(&log), trims, "limit {limit}");
            assert_eq!(
                done,
                outcome(100, after, PressureLevel::Normal),
                "limit {limit}"
            );
        }
    }

    /// AC3: each target is `bytes() - excess` (saturating), the excess drops by
    /// what the cache freed, and nothing is trimmed once it reaches 0.
    #[test]
    fn ac3_each_target_is_the_cache_bytes_minus_the_running_excess() {
        let log = log();
        let mut caches = [
            (Backend, Fake::honest("backend", 40, &log)),
            (Tiles, Fake::honest("tiles", 50, &log)),
            (Frames, Fake::honest("frames", 30, &log)),
        ];

        let done = enforce(&mut governor(60), &mut caches);

        // Excess 60: frames 30 -> 0 (excess 30), tiles 50 -> 20 (excess 0).
        assert_eq!(taken(&log), [("frames", 0), ("tiles", 20)]);
        assert_eq!(done, outcome(120, 60, PressureLevel::Normal));
    }

    /// AC4: the total saturates at `u64::MAX` instead of wrapping.
    #[test]
    fn ac4_a_total_near_u64_max_saturates_instead_of_wrapping() {
        let log = log();
        let mut caches = [
            (Tiles, Fake::honest("tiles", 10, &log)),
            (Frames, Fake::honest("frames", MAX, &log)),
        ];

        let done = enforce(&mut governor(100), &mut caches);

        // Saturated total MAX, excess MAX - 100: frames -> 100; the fresh
        // total 110 asks for a second pass: frames -> 90.
        assert_eq!(taken(&log), [("frames", 100), ("frames", 90)]);
        assert_eq!(done, outcome(MAX, 100, PressureLevel::Normal));
    }

    /// AC4: a cache that reports more after its trim freed 0, not a negative
    /// amount, so the next cache still receives the whole excess.
    #[test]
    fn ac4_a_cache_that_grows_after_its_trim_freed_nothing() {
        let log = log();
        let mut caches = [
            (Frames, Fake::scripted("frames", &[10, 10, 15], &log)),
            (Tiles, Fake::honest("tiles", 10, &log)),
        ];

        let done = enforce(&mut governor(15), &mut caches);

        assert_eq!(
            taken(&log),
            [("frames", 5), ("tiles", 5), ("frames", 10), ("tiles", 0)]
        );
        assert_eq!(done, outcome(20, 15, PressureLevel::Normal));
    }

    /// AC5: a zero budget is a budget: every cache is trimmed to 0, in order.
    #[test]
    fn ac5_a_zero_limit_trims_every_cache_to_zero() {
        let log = log();
        let mut caches = [
            (Tiles, Fake::honest("tiles", 5, &log)),
            (Backend, Fake::honest("backend", 3, &log)),
            (Frames, Fake::honest("frames", 7, &log)),
        ];

        let done = enforce(&mut governor(0), &mut caches);

        assert_eq!(taken(&log), [("frames", 0), ("tiles", 0), ("backend", 0)]);
        assert_eq!(done, outcome(15, 0, PressureLevel::Normal));
    }

    /// AC6: a cache that stops short leaves the excess to the next cache and,
    /// still above the limit, to a second pass, which reaches it.
    #[test]
    fn ac6_a_cache_that_stops_short_is_finished_by_the_second_pass() {
        let log = log();
        let mut caches = [
            (Tiles, Fake::honest("tiles", 20, &log)),
            (Frames, Fake::lying("frames", 100, 30, &log)),
        ];

        let done = enforce(&mut governor(50), &mut caches);

        // Pass 1, excess 70: frames 100 -> 70 (asked 30), tiles 20 -> 0.
        // Pass 2, fresh excess 20: frames 70 -> 50.
        assert_eq!(taken(&log), [("frames", 30), ("tiles", 0), ("frames", 50)]);
        assert_eq!(done, outcome(120, 50, PressureLevel::Normal));
    }

    /// AC6: a cache that stops short every time gets exactly two passes.
    #[test]
    fn ac6_never_more_than_two_passes() {
        let log = log();
        let mut caches = [(Frames, Fake::lying("frames", 100, 10, &log))];

        let done = enforce(&mut governor(50), &mut caches);

        assert_eq!(taken(&log), [("frames", 50); MAX_TRIM_PASSES]);
        assert_eq!(done, outcome(100, 80, PressureLevel::Critical));
    }

    /// AC7: caches that cannot shrink at all cost at most `2 * caches.len()`
    /// trims per `enforce`, every call, and end `Critical`.
    #[test]
    fn ac7_caches_that_cannot_shrink_cost_two_trims_each_per_call() {
        let log = log();
        let mut caches = [
            (Tiles, Fake::stuck("tiles", 50, &log)),
            (Frames, Fake::stuck("frames", 100, &log)),
        ];
        let mut governor = governor(120);

        for call in 0..3 {
            let done = enforce(&mut governor, &mut caches);

            let trims = taken(&log);
            assert!(trims.len() <= 2 * caches.len(), "call {call}");
            assert_eq!(
                trims,
                [("frames", 70), ("tiles", 20), ("frames", 70), ("tiles", 20)],
                "call {call}"
            );
            assert_eq!(
                done,
                outcome(150, 150, PressureLevel::Critical),
                "call {call}"
            );
        }
    }

    /// AC9: an empty slice is within even a zero limit, and says nothing.
    #[test]
    fn ac9_an_empty_slice_trims_nothing_and_stays_normal() {
        let engine = engine_over(quiet_channel());
        let done = enforce(&mut governor_over(&engine, 0), &mut []);
        assert_eq!(done, outcome(0, 0, PressureLevel::Normal));
        assert_eq!(drain(&engine), []);
    }

    /// AC8: `MemoryPressure` is published only on a change of level: never at
    /// the start, once on `Normal -> Critical`, not again while `Critical`,
    /// once on `Critical -> Normal`.
    #[test]
    fn ac8_memory_pressure_is_published_only_on_a_change_of_level() {
        use PressureLevel::{Critical, Normal};
        let engine = engine_over(quiet_channel());
        let mut governor = governor_over(&engine, 10);
        let log = log();
        let mut caches = [(Frames, Fake::stuck("frames", 5, &log))];
        let pressure = |level| vec![EngineEvent::MemoryPressure(level)];
        // (bytes held before the call, the level, the events it publishes)
        let steps = [
            (5, Normal, vec![]),
            (20, Critical, pressure(Critical)),
            (20, Critical, vec![]),
            (30, Critical, vec![]),
            (10, Normal, pressure(Normal)),
            (0, Normal, vec![]),
            (11, Critical, pressure(Critical)),
        ];

        for (call, (held, level, events)) in steps.into_iter().enumerate() {
            caches[0].1.held = held;
            let done = enforce(&mut governor, &mut caches);
            assert_eq!(done, outcome(held, held, level), "call {call}");
            assert_eq!(drain(&engine), events, "call {call}");
        }
    }

    /// AC9: no declared limit, no governor (P5, P7).
    #[test]
    fn ac9_a_config_without_a_memory_limit_refuses_the_governor() {
        let engine = engine_over(quiet_channel());
        let refused = MemoryGovernor::new(&config(), engine.bus().clone());
        assert!(matches!(refused, Err(EngineError::InvalidConfig { .. })));

        let config = config().with_memory_limit(Bytes(7));
        let built = MemoryGovernor::new(&config, engine.bus().clone()).expect("a governor");
        assert_eq!(built.limit(), Bytes(7));
    }

    /// AC10: the second pass is decided by a fresh sum of every `bytes()`, not
    /// by the running excess.
    #[test]
    fn ac10_a_cache_that_grew_back_gets_a_second_pass() {
        let log = log();
        // Total 20, before trim 20, after trim 10 (running excess 0), then the
        // fresh sum reads 20 again; the second pass frees it for good.
        let mut caches = [(
            Frames,
            Fake::scripted("frames", &[20, 20, 10, 20, 20, 10], &log),
        )];

        let done = enforce(&mut governor(10), &mut caches);

        assert_eq!(taken(&log), [("frames", 10), ("frames", 10)]);
        assert_eq!(done, outcome(20, 10, PressureLevel::Normal));
    }

    /// A tile cache that records its trims.
    struct RecordingTiles {
        held: u64,
        trims: Vec<u64>,
    }

    impl TileCache for RecordingTiles {
        fn get(&self, _key: &TileKey) -> Option<std::sync::Arc<ImageBuf>> {
            None
        }
        fn put(&mut self, _key: TileKey, _tile: std::sync::Arc<ImageBuf>) {}
        fn bytes(&self) -> Bytes {
            Bytes(self.held)
        }
        fn trim(&mut self, budget: Bytes) {
            self.trims.push(budget.0);
            self.held = self.held.min(budget.0);
        }
    }

    /// A backend that holds host and device memory and records its trims.
    struct Device {
        usage: MemoryUsage,
        trims: Vec<u64>,
    }

    impl RenderBackend for Device {
        fn caps(&self) -> BackendCaps {
            BackendCaps {
                kind: BackendKind::Cpu,
                max_texture: 0,
                formats: Vec::new(),
                gpu_effects: false,
            }
        }
        fn begin_frame(&mut self, _target: &TargetDesc) -> Result<FrameCx, RenderError> {
            Ok(FrameCx { id: FrameId(0) })
        }
        fn execute(
            &mut self,
            _cx: &mut FrameCx,
            _plan: &RenderPlan,
            _cache: &mut dyn TileCache,
            _deadline: Deadline,
            _cancel: &CancelToken,
        ) -> Result<RenderReport, RenderError> {
            Err(RenderError::DeviceLost)
        }
        fn read_back(&mut self, _cx: &FrameCx, _region: IRect) -> Result<ImageBuf, RenderError> {
            Err(RenderError::DeviceLost)
        }
        fn memory_usage(&self) -> MemoryUsage {
            self.usage
        }
        fn trim(&mut self, budget: Bytes) {
            self.trims.push(budget.0);
        }
    }

    /// The wrappers: a tile cache's bytes and trim, a backend's host bytes
    /// only (P17, SP8) and its trim.
    #[test]
    fn scope_the_port_wrappers_report_host_bytes_and_forward_the_target() {
        let mut tiles = RecordingTiles {
            held: 30,
            trims: Vec::new(),
        };
        let mut device = Device {
            usage: MemoryUsage {
                host: Bytes(50),
                device: Bytes(1_000),
            },
            trims: Vec::new(),
        };
        let mut tile_slot = TileCacheTrim::new(&mut tiles);
        let mut backend_slot = BackendTrim::new(&mut device);
        let mut caches: [(CacheKind, &mut dyn Trimmable); 2] =
            [(Backend, &mut backend_slot), (Tiles, &mut tile_slot)];

        let done = governor(20).enforce(&mut caches);

        // Excess 60: tiles 30 -> 0 (excess 30), backend asked 50 - 30 = 20 and
        // frees nothing; pass 2, excess 30: tiles 0, backend 20 again.
        assert_eq!(done, outcome(80, 50, PressureLevel::Critical));
        assert_eq!(tiles.trims, [0, 0]);
        assert_eq!(device.trims, [20, 20]);
    }
}
