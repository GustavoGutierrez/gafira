//! The render backend contract (02 §4.6).
//!
//! `gafira-render` builds a [`RenderPlan`]; backends read it. Both sides meet
//! here, so no backend sees `gafira-render` (ARQ-06).

use std::fmt;
use std::sync::Arc;
use std::time::Duration;

use gafira_color::Color;
use gafira_doc::contract::{CancelToken, Cancelled, ParamValues};
use gafira_doc::msg::{Message, ToMessage};
use gafira_doc::{BlendMode, msg};
use gafira_geom::{Affine, IRect};
use gafira_raster::effects::Effect;
use gafira_raster::transitions::Transition;
use gafira_raster::{AlphaMask, BufferError, EffectError, ImageBuf, PixelFormat};
use kurbo::BezPath;

use crate::common::Bytes;

/// Identifies the content of a tile; computed by `gafira-render`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ContentKey(pub u128);

/// The cache key of one tile.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TileKey {
    /// What the tile shows.
    pub content: ContentKey,
    /// Mip level.
    pub level: u8,
    /// Tile column in that level.
    pub column: u32,
    /// Tile row in that level.
    pub row: u32,
}

/// How urgent a tile is (02 §6.3). Declaration order is priority order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Priority {
    Interactive,
    Visible,
    Prefetch,
    Background,
}

/// Preview or export quality (02 §6.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RenderQuality {
    Preview,
    Export,
}

/// One drawing step of a tile. Closed on purpose: a new variant must stop a
/// backend from compiling until it handles it.
pub enum RenderOp {
    PushLayer,
    PopLayer {
        blend: BlendMode,
        opacity: f32,
    },
    FillPath {
        path: Arc<BezPath>,
        color: Color,
        transform: Affine,
    },
    DrawImage {
        image: Arc<ImageBuf>,
        transform: Affine,
    },
    ApplyMask {
        mask: Arc<AlphaMask>,
    },
    ApplyEffect {
        effect: Arc<dyn Effect>,
        params: ParamValues,
        scale: f32,
    },
    ApplyTransition {
        transition: Arc<dyn Transition>,
        params: ParamValues,
        progress: f32,
    },
}

/// The work for one tile.
pub struct TileTask {
    pub key: TileKey,
    pub priority: Priority,
    pub ops: Vec<RenderOp>,
}

/// What a backend executes: tasks in priority order (02 §6.1).
pub struct RenderPlan {
    pub quality: RenderQuality,
    pub tasks: Vec<TileTask>,
}

/// A tile cache. `gafira-render` implements it; backends only use it.
pub trait TileCache: Send {
    /// The cached tile, if any.
    fn get(&self, key: &TileKey) -> Option<Arc<ImageBuf>>;
    /// Stores a tile.
    fn put(&mut self, key: TileKey, tile: Arc<ImageBuf>);
}

/// Identifies one frame in a backend.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FrameId(pub u64);

/// A plain handle to a frame; the backend keeps its state by [`FrameId`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FrameCx {
    pub id: FrameId,
}

/// The kind of a backend.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BackendKind {
    Cpu,
    Gpu,
}

/// What a backend can do.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BackendCaps {
    pub kind: BackendKind,
    /// Largest texture side in pixels.
    pub max_texture: u32,
    /// Formats `read_back` and targets can use.
    pub formats: Vec<PixelFormat>,
    /// True when effects run on the device.
    pub gpu_effects: bool,
}

/// The surface of one frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TargetDesc {
    pub width: u32,
    pub height: u32,
    pub format: PixelFormat,
}

/// Memory a backend holds.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MemoryUsage {
    /// Host memory.
    pub host: Bytes,
    /// Device memory (zero on the CPU backend).
    pub device: Bytes,
}

/// The time limit of a render, on the monotonic clock (`Clock::monotonic`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Deadline {
    Unbounded,
    At(Duration),
}

impl Deadline {
    /// True when `now` is at or past the deadline. `Unbounded` never passes.
    #[must_use]
    pub fn has_passed(&self, now: Duration) -> bool {
        match self {
            Self::Unbounded => false,
            Self::At(t) => now >= *t,
        }
    }
}

/// What `execute` did. A late render is a report, not an error (02 §6.2).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RenderReport {
    /// Tasks finished.
    pub done: usize,
    /// Tasks left undone.
    pub left: usize,
}

impl RenderReport {
    /// True when no task is left.
    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.left == 0
    }
}

/// Why a render call failed.
#[derive(Debug)]
pub enum RenderError {
    DeviceLost,
    OutOfMemory,
    AdapterMissing,
    DriverBlocked,
    Effect(EffectError),
    Buffer(BufferError),
    Cancelled(Cancelled),
}

impl fmt::Display for RenderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DeviceLost => f.write_str("render device lost"),
            Self::OutOfMemory => f.write_str("render out of memory"),
            Self::AdapterMissing => f.write_str("no render adapter"),
            Self::DriverBlocked => f.write_str("render driver blocked"),
            Self::Effect(e) => write!(f, "effect failed: {e}"),
            Self::Buffer(e) => write!(f, "buffer error: {e}"),
            Self::Cancelled(e) => e.fmt(f),
        }
    }
}

impl std::error::Error for RenderError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Effect(e) => Some(e),
            Self::Buffer(e) => Some(e),
            Self::Cancelled(e) => Some(e),
            _ => None,
        }
    }
}

impl From<Cancelled> for RenderError {
    fn from(e: Cancelled) -> Self {
        Self::Cancelled(e)
    }
}

impl From<EffectError> for RenderError {
    fn from(e: EffectError) -> Self {
        Self::Effect(e)
    }
}

impl From<BufferError> for RenderError {
    fn from(e: BufferError) -> Self {
        Self::Buffer(e)
    }
}

impl ToMessage for RenderError {
    fn to_message(&self) -> Message {
        match self {
            Self::DeviceLost => msg!("error.render.device_lost"),
            Self::OutOfMemory => msg!("error.render.out_of_memory"),
            Self::AdapterMissing => msg!("error.render.adapter_missing"),
            Self::DriverBlocked => msg!("error.render.driver_blocked"),
            Self::Effect(e) => e.to_message(),
            Self::Buffer(e) => e.to_message(),
            Self::Cancelled(e) => e.to_message(),
        }
    }
}

/// A rendering device: CPU or GPU. Adapters implement it.
pub trait RenderBackend: Send {
    /// What the backend can do.
    fn caps(&self) -> BackendCaps;

    /// Opens a frame on `target`.
    ///
    /// # Errors
    /// [`RenderError::DeviceLost`], [`RenderError::OutOfMemory`],
    /// [`RenderError::AdapterMissing`] or [`RenderError::DriverBlocked`].
    fn begin_frame(&mut self, target: &TargetDesc) -> Result<FrameCx, RenderError>;

    /// Runs the plan until it ends or `deadline` passes. A late end is a
    /// report with `left > 0`, not an error.
    ///
    /// # Errors
    /// [`RenderError`] when the device fails or `cancel` is cancelled.
    fn execute(
        &mut self,
        cx: &mut FrameCx,
        plan: &RenderPlan,
        cache: &mut dyn TileCache,
        deadline: Deadline,
        cancel: &CancelToken,
    ) -> Result<RenderReport, RenderError>;

    /// Reads `region` back as `Rgba8` or `RgbaF32`.
    ///
    /// # Errors
    /// [`RenderError`] when the device fails.
    fn read_back(&mut self, cx: &FrameCx, region: IRect) -> Result<ImageBuf, RenderError>;

    /// Memory held now.
    fn memory_usage(&self) -> MemoryUsage;

    /// Frees cache until the backend holds at most `budget`.
    fn trim(&mut self, budget: Bytes);
}
