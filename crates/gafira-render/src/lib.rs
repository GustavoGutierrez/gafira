//! Layer-3 render engine crate `gafira-render`: the render graph, the content
//! key of every node, the emission of the render operations, the planner, the
//! tile cache, the frame budget and the page renderer the exporters reach
//! (02 §4.5, §4.17.7, §6.1, §6.2, §6.3).
//!
//! The crate is pure in the sense of ARQ-02 (ADR 0016): it opens no file,
//! opens no socket, spawns no thread and reads no clock of its own. The
//! instant comes from the injected clock (`ports::Clock`, §4.6) and the pool
//! of workers comes as a parameter too (ARQ-138, ARQ-143).

#![forbid(unsafe_code)]

mod budget;
mod cache;
mod fonts;
mod frame;
mod graph;
mod key;
mod page;
mod planner;
mod pool;
mod scope;

pub use budget::{FRAME, FrameBudget, GRAPH_READY, MARGIN, Stage, VISIBLE_DONE};
pub use cache::ContentTileCache;
pub use fonts::FontBook;
pub use frame::{BackendSelector, Degradation, FrameLoop, FrameOutcome, FrameRequest};
pub use graph::{
    GraphError, GraphNode, GraphNodeId, GraphNodeKind, GraphParams, RenderGraph,
    RenderGraphBuilder, SourceKind,
};
pub use key::{DOMAIN, KeyParam, NodeType, canonical_description, content_key};
pub use page::{BackendFactory, GraphPageRenderer, GraphPageRenderers, PAGE_BACKGROUND};
pub use planner::{
    PageExtent, PlanError, PlannedTile, Planner, RenderPlan, TILE_SIZE, TileGrid, Viewport,
    level_for_zoom,
};
pub use pool::{InlinePool, WorkerPool};
pub use scope::{PageScope, ScopeError};
