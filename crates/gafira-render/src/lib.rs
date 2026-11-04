//! Layer-3 render engine crate `gafira-render`: the render graph, the content
//! key of every node and the emission of the render operations (02 §4.17.7,
//! §6.1).
//!
//! The crate is pure in the sense of ARQ-02 (ADR 0016): it opens no file,
//! opens no socket, spawns no thread and reads no clock. The instant comes
//! with the parameters of the graph and the pool of workers comes as a
//! parameter too (ARQ-138).

#![forbid(unsafe_code)]

mod graph;
mod key;
mod pool;

pub use graph::{
    GraphError, GraphNode, GraphNodeId, GraphNodeKind, GraphParams, RenderGraph,
    RenderGraphBuilder, SourceKind,
};
pub use key::{DOMAIN, KeyParam, NodeType, canonical_description, content_key};
pub use pool::{InlinePool, WorkerPool};
