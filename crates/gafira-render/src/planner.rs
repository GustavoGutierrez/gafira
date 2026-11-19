//! The planner: the tile grid, the level of detail, the priority order of the
//! tasks and the plan a backend executes (02 §6.1, §6.3, ARQ-121).
//!
//! The planner turns a render graph into the tiles that cover the viewport:
//! it resolves the level of detail from the zoom, lays the tiles of the grid
//! of that level over the viewport, clips them to it and orders them by the
//! priority of 02 §6.3.
//!
//! The planner is pure in the sense of ARQ-02 (ADR 0016): it reads no clock
//! and opens no file. The level of detail comes from the zoom, so the planner
//! needs no instant; the deadline of the frame comes with the caller (RE4).

use std::sync::Arc;

use gafira_doc::contract::{CancelToken, Cancelled, Progress, checkpoint};
use gafira_geom::IRect;
use gafira_ports::render::{ContentKey, Priority, RenderOp, RenderQuality, TileKey};

use crate::graph::RenderGraph;
use crate::key::composite_key;

/// The size of a tile, in pixels (FIJADA, 02 §6.1 and ARQ-121).
pub const TILE_SIZE: u32 = 256;

/// The size of the page, in pixels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PageExtent {
    /// The width of the page, in pixels.
    pub width: u32,
    /// The height of the page, in pixels.
    pub height: u32,
}

/// The visible region of the page and the zoom of the view (02 §6.1).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Viewport {
    /// The visible region, in page pixels.
    pub rect: IRect,
    /// The zoom of the view: `1.0` is 100 %.
    pub zoom: f64,
    /// The point the user acts on, in page pixels; the centre of `rect` when
    /// there is none.
    pub focus: Option<(i32, i32)>,
}

/// The mip chain of a page and its tile grid (ARQ-121, 02 §6.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TileGrid {
    width: u32,
    height: u32,
}

/// One tile of the plan.
pub struct PlannedTile {
    /// The key of the tile in the cache (02 §4.6, ARQ-142).
    pub key: TileKey,
    /// The urgency of the tile (02 §6.3).
    pub priority: Priority,
    /// The region of the page the tile shows, clipped to the viewport.
    pub region: IRect,
    /// The operations that draw the tile.
    pub ops: Vec<RenderOp>,
}

/// The plan of a frame: the task of every tile, in priority order.
pub struct RenderPlan {
    quality: RenderQuality,
    level: u8,
    tiles: Vec<PlannedTile>,
}

/// Why the plan could not be built (ARQ-139).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlanError {
    /// The planning was cancelled (ARQ-139).
    Cancelled(Cancelled),
}

impl From<Cancelled> for PlanError {
    fn from(cancelled: Cancelled) -> Self {
        Self::Cancelled(cancelled)
    }
}

impl std::fmt::Display for PlanError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Cancelled(cancelled) => write!(f, "{cancelled}"),
        }
    }
}

impl std::error::Error for PlanError {}

impl RenderPlan {
    /// The quality the plan was built for.
    #[must_use]
    pub fn quality(&self) -> RenderQuality {
        self.quality
    }

    /// The level of detail the plan was built at (02 §6.1).
    #[must_use]
    pub fn level(&self) -> u8 {
        self.level
    }

    /// The tiles of the plan, in priority order.
    #[must_use]
    pub fn tiles(&self) -> &[PlannedTile] {
        &self.tiles
    }

    /// The plan a backend executes (02 §4.6).
    ///
    /// The tasks keep the order of the plan: the most urgent tile first.
    #[must_use]
    pub fn into_ports_plan(self) -> gafira_ports::render::RenderPlan {
        gafira_ports::render::RenderPlan {
            quality: self.quality,
            tasks: self
                .tiles
                .into_iter()
                .map(|tile| gafira_ports::render::TileTask {
                    key: tile.key,
                    priority: tile.priority,
                    region: tile.region,
                    ops: tile.ops,
                })
                .collect(),
        }
    }
}

impl TileGrid {
    /// The grid of a page of the given size, in pixels.
    #[must_use]
    pub fn new(extent: PageExtent) -> Self {
        Self {
            width: extent.width,
            height: extent.height,
        }
    }

    /// The number of levels of the mip chain (ARQ-121):
    /// `⌊log2(max(width, height))⌋ + 1`.
    #[must_use]
    pub fn level_count(&self) -> u32 {
        32 - self.width.max(self.height).leading_zeros()
    }

    /// The size of the page at a level (ARQ-121): each side divided by two
    /// with the floor, with a minimum of one.
    #[must_use]
    pub fn level_size(&self, level: u8) -> (u32, u32) {
        (
            level_side(self.width, level),
            level_side(self.height, level),
        )
    }

    /// The number of tiles in each direction at a level.
    #[must_use]
    pub fn tile_count(&self, level: u8) -> (u32, u32) {
        let (width, height) = self.level_size(level);
        (width.div_ceil(TILE_SIZE), height.div_ceil(TILE_SIZE))
    }

    /// The level of detail of a zoom (02 §6.1): the nearest mip of the chain
    /// of ARQ-121.
    #[must_use]
    pub fn level_for(&self, zoom: f64) -> u8 {
        level_for_zoom(zoom, self.level_count())
    }

    /// The region of the page a tile of a level covers, in page pixels.
    ///
    /// The tiles of a level cover the whole page: the last row and the last
    /// column of a level absorb the odd pixel of the side, so no pixel is
    /// lost (ARQ-121).
    #[must_use]
    pub fn tile_region(&self, level: u8, column: u32, row: u32) -> IRect {
        let (width, height) = self.level_size(level);
        let x0 = column * TILE_SIZE;
        let x1 = ((column + 1) * TILE_SIZE).min(width);
        let y0 = row * TILE_SIZE;
        let y1 = ((row + 1) * TILE_SIZE).min(height);
        let (page_x0, page_x1) = self.page_span(x0, x1, level, self.width);
        let (page_y0, page_y1) = self.page_span(y0, y1, level, self.height);
        IRect::new(page_x0, page_y0, page_x1, page_y1)
    }

    /// The page pixels that the level columns `from..to` cover.
    ///
    /// The last column of a level reaches the end of the side: it covers
    /// three source pixels instead of two when the side is odd (ARQ-121).
    fn page_span(&self, from: u32, to: u32, level: u8, size: u32) -> (i32, i32) {
        let step = 1u32 << level.min(31);
        let start = from * step;
        let end = if to >= level_side(size, level) {
            size
        } else {
            to * step
        };
        (start as i32, end as i32)
    }
}

/// One side of the page at a level (ARQ-121): divided by two with the floor,
/// with a minimum of one. A side of one does not divide.
fn level_side(size: u32, level: u8) -> u32 {
    (size >> level.min(31)).max(1)
}

/// The level of detail of a zoom (02 §6.1): the nearest mip of a chain of
/// `levels` levels.
///
/// The zoom of 100 % is the level zero; a zoom of 200 % is the level one; the
/// zoom of 3200 % is the level five. A zoom below 100 % stays in the level
/// zero: there is no mip finer than the page.
#[must_use]
pub fn level_for_zoom(zoom: f64, levels: u32) -> u8 {
    if levels == 0 || zoom.is_nan() || zoom <= 0.0 {
        return 0;
    }
    let last = f64::from(levels - 1);
    zoom.log2().round().clamp(0.0, last) as u8
}

/// Turns a render graph into the plan of a frame (02 §6.1).
#[derive(Clone, Copy, Debug, Default)]
pub struct Planner;

impl Planner {
    /// Plans the tiles that cover the viewport.
    ///
    /// The plan holds the task of every tile of the viewport, at the level of
    /// detail the zoom resolves, in the priority order of 02 §6.3: the tile
    /// the user acts on is `Interactive` and the rest are `Visible`.
    ///
    /// The graph must be built at the level the planner resolves
    /// ([`TileGrid::level_for`]), because the level is part of the content
    /// key of a node (02 §6.1).
    ///
    /// The planning is a long operation: it checks the cancel token and
    /// reports the progress once per task (ARQ-139).
    ///
    /// # Errors
    ///
    /// [`PlanError::Cancelled`] when the token is cancelled (ARQ-139).
    pub fn plan(
        graph: &RenderGraph,
        extent: PageExtent,
        viewport: &Viewport,
        cancel: &CancelToken,
        progress: &dyn Progress,
    ) -> Result<RenderPlan, PlanError> {
        let grid = TileGrid::new(extent);
        let level = grid.level_for(viewport.zoom);
        let focus = viewport.focus.unwrap_or_else(|| centre(viewport.rect));

        // The roots of the graph, with the operations of their subtree, in
        // paint order.
        let roots: Vec<(ContentKey, &[RenderOp])> = graph
            .roots()
            .iter()
            .zip(graph.root_ops())
            .filter_map(|(id, ops)| graph.node(*id).map(|node| (node.key(), ops)))
            .collect();

        // O12: a region is one task. The roots of the page are composed in
        // paint order, because the backend writes a task's tile over the
        // surface as the whole content of its region (`Surface::write`), so a
        // task per root would erase the roots painted before it.
        let root_keys: Vec<ContentKey> = roots.iter().map(|(key, _)| *key).collect();
        let content = composite_key(&root_keys);
        let merged: Vec<RenderOp> = roots
            .iter()
            .flat_map(|(_, ops)| ops.iter())
            .map(copy_op)
            .collect();

        // The tiles of the viewport, clipped to it.
        let mut regions: Vec<(u32, u32, IRect)> = Vec::new();
        if !viewport.rect.is_empty() {
            let columns = columns_of(viewport.rect.x0, viewport.rect.x1, level, extent.width);
            let rows = columns_of(viewport.rect.y0, viewport.rect.y1, level, extent.height);
            for row in rows {
                for column in columns.iter().copied() {
                    let region = grid
                        .tile_region(level, column, row)
                        .intersect(viewport.rect);
                    if !region.is_empty() {
                        regions.push((column, row, region));
                    }
                }
            }
        }

        let planned = regions.len() + 1;
        let mut tiles: Vec<PlannedTile> = Vec::with_capacity(planned);
        let mut done = 0usize;
        for (column, row, region) in regions {
            let priority = if region.contains(focus.0, focus.1) {
                Priority::Interactive
            } else {
                Priority::Visible
            };
            tiles.push(PlannedTile {
                key: TileKey {
                    content,
                    level,
                    column,
                    row,
                },
                priority,
                region,
                ops: merged.iter().map(copy_op).collect(),
            });
            done += 1;
            checkpoint(cancel, progress, done as f32 / planned as f32)?;
        }
        checkpoint(cancel, progress, 1.0)?;

        // The priority order of 02 §6.3: the most urgent tile first.
        tiles.sort_by_key(|tile| (tile.priority, tile.key.row, tile.key.column));
        Ok(RenderPlan {
            quality: graph.params().quality,
            level,
            tiles,
        })
    }
}

/// The tiles of a level that the page columns `from..to` touch.
fn columns_of(from: i32, to: i32, level: u8, size: u32) -> Vec<u32> {
    let last = level_side(size, level) - 1;
    let first = (from.max(0) as u32 >> level.min(31)).min(last);
    let final_ = ((to.saturating_sub(1)).max(0) as u32 >> level.min(31)).min(last);
    (first / TILE_SIZE..=final_ / TILE_SIZE).collect()
}

/// The centre of a rectangle.
fn centre(rect: IRect) -> (i32, i32) {
    (
        ((i64::from(rect.x0) + i64::from(rect.x1)) / 2) as i32,
        ((i64::from(rect.y0) + i64::from(rect.y1)) / 2) as i32,
    )
}

/// A copy of an operation: the same operation in another tile.
///
/// `RenderOp` is not `Clone` (the port is closed on purpose), so the copy is
/// built field by field. A new variant of the port breaks this function, and
/// that is the point.
fn copy_op(op: &RenderOp) -> RenderOp {
    match op {
        RenderOp::PushLayer => RenderOp::PushLayer,
        RenderOp::PopLayer { blend, opacity } => RenderOp::PopLayer {
            blend: *blend,
            opacity: *opacity,
        },
        RenderOp::FillPath {
            path,
            color,
            transform,
        } => RenderOp::FillPath {
            path: Arc::clone(path),
            color: *color,
            transform: *transform,
        },
        RenderOp::FillGradient {
            path,
            gradient,
            size,
            transform,
        } => RenderOp::FillGradient {
            path: Arc::clone(path),
            gradient: Arc::clone(gradient),
            size: *size,
            transform: *transform,
        },
        RenderOp::DrawImage { image, transform } => RenderOp::DrawImage {
            image: Arc::clone(image),
            transform: *transform,
        },
        RenderOp::ApplyMask { mask } => RenderOp::ApplyMask {
            mask: Arc::clone(mask),
        },
        RenderOp::ApplyEffect {
            effect,
            params,
            scale,
        } => RenderOp::ApplyEffect {
            effect: Arc::clone(effect),
            params: params.clone(),
            scale: *scale,
        },
        RenderOp::ApplyTransition {
            transition,
            params,
            progress,
        } => RenderOp::ApplyTransition {
            transition: Arc::clone(transition),
            params: params.clone(),
            progress: *progress,
        },
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use gafira_color::{Color, Rgba};
    use gafira_doc::contract::NoProgress;
    use gafira_doc::visit::{PageView, WalkOrder};
    use gafira_doc::{
        ElementBase, Node, NodeId, NodeKind, Page, PageId, PageSize, Placement, ShapeGeometry,
        ShapeNode,
    };
    use gafira_geom::{Affine, Length, Unit};
    use gafira_ports::render::Priority;
    use gafira_testkit::progress::check_progress_contract;
    use gafira_time::Flicks;

    use super::*;
    use crate::graph::{GraphParams, RenderGraphBuilder};
    use crate::pool::InlinePool;

    /// A page of the given size, in pixels, with one filled shape.
    fn new_page(width: u32, height: u32) -> Page {
        let mut page = Page::new(
            PageId::new(1),
            "P".to_string(),
            PageSize {
                width: Length::new(f64::from(width), Unit::Px),
                height: Length::new(f64::from(height), Unit::Px),
            },
        );
        let base = ElementBase::new(NodeId::new(1), Placement::new(0.0, 0.0, 10.0, 10.0));
        let kind = NodeKind::Shape(ShapeNode {
            geometry: ShapeGeometry::Rect,
            fill: Some(Color::Srgb(Rgba::new(0.2, 0.4, 0.6, 1.0))),
            stroke: None,
            corner_radius: 0.0,
            text: None,
            gradient: None,
        });
        page.nodes
            .insert(NodeId::new(1), Arc::new(Node::new(base, kind)));
        page.roots = vec![NodeId::new(1)];
        page
    }

    /// A top-level shape of `page`, 10 × 10 at `(x, y)`, filled with `fill`.
    fn add_root(page: &mut Page, id: u64, x: f64, y: f64, fill: Color) {
        let base = ElementBase::new(NodeId::new(id), Placement::new(x, y, 10.0, 10.0));
        let kind = NodeKind::Shape(ShapeNode {
            geometry: ShapeGeometry::Rect,
            fill: Some(fill),
            stroke: None,
            corner_radius: 0.0,
            text: None,
            gradient: None,
        });
        page.nodes
            .insert(NodeId::new(id), Arc::new(Node::new(base, kind)));
    }

    /// A page with two top-level shapes, so the graph has two roots.
    fn two_root_page() -> Page {
        let mut page = Page::new(
            PageId::new(1),
            "P".to_string(),
            PageSize {
                width: Length::new(100.0, Unit::Px),
                height: Length::new(100.0, Unit::Px),
            },
        );
        add_root(&mut page, 1, 0.0, 0.0, red());
        add_root(&mut page, 2, 20.0, 0.0, blue());
        page.roots = vec![NodeId::new(1), NodeId::new(2)];
        page
    }

    fn red() -> Color {
        Color::Srgb(Rgba::new(1.0, 0.0, 0.0, 1.0))
    }

    fn blue() -> Color {
        Color::Srgb(Rgba::new(0.0, 0.0, 1.0, 1.0))
    }

    /// The plan of the whole page at 100 %, one tile of the grid.
    fn merged_plan(page: &Page) -> RenderPlan {
        plan(
            &build(page),
            PageExtent {
                width: 100,
                height: 100,
            },
            &Viewport {
                rect: IRect::new(0, 0, 100, 100),
                zoom: 1.0,
                focus: None,
            },
        )
    }

    /// The fill colors of a tile, in paint order.
    fn fill_colors(tile: &PlannedTile) -> Vec<Color> {
        tile.ops
            .iter()
            .filter_map(|op| match op {
                RenderOp::FillPath { color, .. } => Some(*color),
                _ => None,
            })
            .collect()
    }

    /// The parameters of a graph of the given quality.
    fn params(quality: RenderQuality) -> GraphParams {
        GraphParams {
            quality,
            lod: 0,
            t: Flicks::from_ticks(0),
        }
    }

    /// Builds the graph of the page.
    fn build(page: &Page) -> RenderGraph {
        let view = PageView::new(page);
        RenderGraphBuilder::new(&view, params(RenderQuality::Preview), InlinePool)
            .build(WalkOrder::PreOrder, &CancelToken::new(), &NoProgress)
            .expect("the page builds")
    }

    /// Plans the viewport of the graph.
    fn plan(graph: &RenderGraph, extent: PageExtent, viewport: &Viewport) -> RenderPlan {
        Planner::plan(graph, extent, viewport, &CancelToken::new(), &NoProgress).expect("the plan")
    }

    /// What a tile of the plan holds, without the payload of its operations.
    fn tiles_of(plan: &RenderPlan) -> Vec<(TileKey, Priority, IRect, Vec<&'static str>)> {
        plan.tiles()
            .iter()
            .map(|tile| {
                (
                    tile.key,
                    tile.priority,
                    tile.region,
                    tile.ops.iter().map(|op| op_kind(op)).collect(),
                )
            })
            .collect()
    }

    /// The regions of the port plan, by tile position.
    fn port_regions(plan: RenderPlan) -> Vec<((u32, u32), IRect)> {
        let mut tasks: Vec<((u32, u32), IRect)> = plan
            .into_ports_plan()
            .tasks
            .into_iter()
            .map(|task| ((task.key.row, task.key.column), task.region))
            .collect();
        tasks.sort_by_key(|(position, _)| *position);
        tasks
    }

    /// The transform of every fill of the port plan, in plan order.
    fn fill_transforms(plan: RenderPlan) -> Vec<Affine> {
        plan.into_ports_plan()
            .tasks
            .iter()
            .flat_map(|task| task.ops.iter())
            .filter_map(|op| match op {
                RenderOp::FillPath { transform, .. } => Some(*transform),
                _ => None,
            })
            .collect()
    }

    /// What an operation does, without its payload.
    fn op_kind(op: &RenderOp) -> &'static str {
        match op {
            RenderOp::PushLayer => "push",
            RenderOp::PopLayer { .. } => "pop",
            RenderOp::FillPath { .. } => "fill",
            RenderOp::FillGradient { .. } => "gradient",
            RenderOp::DrawImage { .. } => "image",
            RenderOp::ApplyMask { .. } => "mask",
            RenderOp::ApplyEffect { .. } => "effect",
            RenderOp::ApplyTransition { .. } => "transition",
        }
    }

    // ARQ-121: the chain of a page has the levels of the section.
    #[test]
    fn arq_121_the_chain_of_a_page_has_the_levels_of_the_section() {
        let grid = TileGrid::new(PageExtent {
            width: 1000,
            height: 600,
        });
        assert_eq!(grid.level_count(), 10);
        assert_eq!(grid.level_size(0), (1000, 600));
        assert_eq!(grid.level_size(9), (1, 1));
        // A page of one pixel has one level.
        let tiny = TileGrid::new(PageExtent {
            width: 1,
            height: 1,
        });
        assert_eq!(tiny.level_count(), 1);
        assert_eq!(tiny.level_size(0), (1, 1));
    }

    // ARQ-121: a side of one does not divide.
    #[test]
    fn arq_121_a_side_of_one_does_not_divide() {
        let grid = TileGrid::new(PageExtent {
            width: 3,
            height: 1,
        });
        assert_eq!(grid.level_size(1), (1, 1));
        assert_eq!(grid.level_size(7), (1, 1));
    }

    // ARQ-121: a page of 257 pixels is split in two tiles.
    #[test]
    fn arq_121_a_page_of_257_pixels_is_split_in_two_tiles() {
        let grid = TileGrid::new(PageExtent {
            width: 257,
            height: 1,
        });
        assert_eq!(grid.tile_count(0), (2, 1));
        assert_eq!(grid.tile_region(0, 0, 0), IRect::new(0, 0, 256, 1));
        assert_eq!(grid.tile_region(0, 1, 0), IRect::new(256, 0, 257, 1));
    }

    // ARQ-121: the tiles of a level cover the whole page and no pixel is lost.
    #[test]
    fn arq_121_the_tiles_of_a_level_cover_the_whole_page() {
        for (width, height) in [(257, 1), (3, 1), (1000, 600), (256, 256), (513, 257)] {
            let grid = TileGrid::new(PageExtent { width, height });
            for level in 0..grid.level_count() {
                let level = level as u8;
                let (columns, rows) = grid.tile_count(level);
                let mut x0 = i32::MAX;
                let mut y0 = i32::MAX;
                let mut x1 = i32::MIN;
                let mut y1 = i32::MIN;
                for row in 0..rows {
                    for column in 0..columns {
                        let region = grid.tile_region(level, column, row);
                        x0 = x0.min(region.x0);
                        y0 = y0.min(region.y0);
                        x1 = x1.max(region.x1);
                        y1 = y1.max(region.y1);
                    }
                }
                assert_eq!(
                    (x0, y0, x1, y1),
                    (0, 0, width as i32, height as i32),
                    "{width}x{height} at level {level}"
                );
            }
        }
    }

    // ARQ-121, ARQ-41: the level of detail is the nearest mip of the chain.
    #[test]
    fn arq_121_the_level_of_detail_is_the_nearest_mip() {
        let grid = TileGrid::new(PageExtent {
            width: 1000,
            height: 600,
        });
        let table = [
            (1.0, 0u8),
            (2.0, 1),
            (3.0, 2),
            (0.75, 0),
            (0.05, 0),
            (32.0, 5),
            (100.0, 7),
            (1000.0, 9),
        ];
        for (zoom, want) in table {
            assert_eq!(grid.level_for(zoom), want, "zoom {zoom}");
        }
        // The chain of a small page clamps the level.
        let small = TileGrid::new(PageExtent {
            width: 100,
            height: 100,
        });
        assert_eq!(small.level_for(1000.0), 6);
    }

    // ARQ-41, ARQ-121: the plan holds the tiles that cover the viewport.
    #[test]
    fn arq_41_the_plan_holds_the_tiles_that_cover_the_viewport() {
        let page = new_page(1000, 1000);
        let graph = build(&page);
        let extent = PageExtent {
            width: 1000,
            height: 1000,
        };
        let viewport = Viewport {
            rect: IRect::new(0, 0, 600, 600),
            zoom: 1.0,
            focus: None,
        };
        let plan = plan(&graph, extent, &viewport);
        assert_eq!(plan.tiles().len(), 9);
        assert_eq!(plan.level(), 0);
        // The tiles cover the viewport.
        let mut x0 = i32::MAX;
        let mut y0 = i32::MAX;
        let mut x1 = i32::MIN;
        let mut y1 = i32::MIN;
        for tile in plan.tiles() {
            x0 = x0.min(tile.region.x0);
            y0 = y0.min(tile.region.y0);
            x1 = x1.max(tile.region.x1);
            y1 = y1.max(tile.region.y1);
        }
        assert_eq!((x0, y0, x1, y1), (0, 0, 600, 600));
        // Every tile carries the operations of the root it draws.
        assert_eq!(plan.tiles()[0].ops.len(), graph.ops().len());
    }

    // ARQ-41: the tasks of the plan come in the priority order of 02 §6.3.
    #[test]
    fn arq_41_the_tasks_of_the_plan_come_in_the_priority_order() {
        let page = new_page(1000, 1000);
        let graph = build(&page);
        let extent = PageExtent {
            width: 1000,
            height: 1000,
        };
        // The focus is in the tile of the second column of the second row.
        let viewport = Viewport {
            rect: IRect::new(0, 0, 600, 600),
            zoom: 1.0,
            focus: Some((300, 300)),
        };
        let plan = plan(&graph, extent, &viewport);
        let first = &plan.tiles()[0];
        assert_eq!(first.priority, Priority::Interactive);
        assert_eq!((first.key.column, first.key.row), (1, 1));
        // The rest of the tiles are visible.
        assert!(
            plan.tiles()[1..]
                .iter()
                .all(|tile| tile.priority == Priority::Visible)
        );
        // The order inside a priority is the raster order.
        let positions: Vec<(u32, u32)> = plan
            .tiles()
            .iter()
            .map(|tile| (tile.key.row, tile.key.column))
            .collect();
        let mut sorted = positions[1..].to_vec();
        sorted.sort();
        assert_eq!(&positions[1..], &sorted[..]);
    }

    // ARQ-121: an empty viewport holds no tile and does not panic.
    #[test]
    fn arq_121_the_plan_holds_no_tile_for_an_empty_viewport() {
        let page = new_page(1000, 1000);
        let graph = build(&page);
        let extent = PageExtent {
            width: 1000,
            height: 1000,
        };
        let viewport = Viewport {
            rect: IRect::new(10, 10, 10, 10),
            zoom: 1.0,
            focus: None,
        };
        let plan = plan(&graph, extent, &viewport);
        assert!(plan.tiles().is_empty());
    }

    // ARQ-121: the tiles of the plan lie inside the viewport.
    #[test]
    fn arq_121_the_tiles_of_the_plan_lie_inside_the_viewport() {
        let page = new_page(1000, 1000);
        let graph = build(&page);
        let extent = PageExtent {
            width: 1000,
            height: 1000,
        };
        // The viewport starts in the middle of a tile.
        let viewport = Viewport {
            rect: IRect::new(100, 100, 700, 700),
            zoom: 1.0,
            focus: None,
        };
        let plan = plan(&graph, extent, &viewport);
        assert_eq!(plan.tiles().len(), 9);
        for tile in plan.tiles() {
            assert!(tile.region.x0 >= viewport.rect.x0);
            assert!(tile.region.y0 >= viewport.rect.y0);
            assert!(tile.region.x1 <= viewport.rect.x1);
            assert!(tile.region.y1 <= viewport.rect.y1);
            assert!(!tile.region.is_empty());
        }
        // The first tile of the grid starts at the edge of the viewport.
        let first = plan
            .tiles()
            .iter()
            .find(|tile| tile.key.column == 0 && tile.key.row == 0)
            .expect("the first tile is there");
        assert_eq!(first.region, IRect::new(100, 100, 256, 256));
    }

    // ARQ-149: the plan carries the place of every tile, in page pixels
    // clipped to the viewport, and no backend re-derives it from the key.
    #[test]
    fn arq_149_the_plan_carries_the_clipped_region_of_every_tile() {
        let page = new_page(1000, 1000);
        let graph = build(&page);
        let extent = PageExtent {
            width: 1000,
            height: 1000,
        };
        // The viewport starts and ends in the middle of a tile, so every tile
        // of the plan is clipped.
        let viewport = Viewport {
            rect: IRect::new(100, 100, 356, 356),
            zoom: 1.0,
            focus: None,
        };
        assert_eq!(
            port_regions(plan(&graph, extent, &viewport)),
            vec![
                ((0, 0), IRect::new(100, 100, 256, 256)),
                ((0, 1), IRect::new(256, 100, 356, 256)),
                ((1, 0), IRect::new(100, 256, 256, 356)),
                ((1, 1), IRect::new(256, 256, 356, 356)),
            ]
        );
    }

    // ARQ-149: a viewport smaller than a tile clips its only tile.
    #[test]
    fn arq_149_a_viewport_smaller_than_a_tile_clips_its_only_tile() {
        let page = new_page(1000, 1000);
        let graph = build(&page);
        let extent = PageExtent {
            width: 1000,
            height: 1000,
        };
        let viewport = Viewport {
            rect: IRect::new(0, 0, 100, 100),
            zoom: 1.0,
            focus: None,
        };
        assert_eq!(
            port_regions(plan(&graph, extent, &viewport)),
            vec![((0, 0), IRect::new(0, 0, 100, 100))]
        );
    }

    // ARQ-149: the level of a tile does not change the unit of its region: a
    // full tile of level one covers 512 page pixels per side, not 256.
    #[test]
    fn arq_149_a_full_tile_of_level_one_covers_512_page_pixels() {
        let page = new_page(1024, 1024);
        let graph = build(&page);
        let extent = PageExtent {
            width: 1024,
            height: 1024,
        };
        let viewport = Viewport {
            rect: IRect::new(0, 0, 1024, 1024),
            zoom: 2.0,
            focus: None,
        };
        let planned = plan(&graph, extent, &viewport);
        assert_eq!(planned.level(), 1);
        let regions = port_regions(planned);
        assert_eq!(regions.len(), 4);
        assert_eq!(regions[0], ((0, 0), IRect::new(0, 0, 512, 512)));
    }

    // ARQ-149: the operations stay in page coordinates; the plan does not
    // translate them to the surface of the request.
    #[test]
    fn arq_149_the_plan_does_not_translate_the_operations() {
        let page = new_page(1000, 1000);
        let graph = build(&page);
        let extent = PageExtent {
            width: 1000,
            height: 1000,
        };
        let at_origin = Viewport {
            rect: IRect::new(0, 0, 356, 356),
            zoom: 1.0,
            focus: None,
        };
        let offset = Viewport {
            rect: IRect::new(100, 100, 456, 456),
            zoom: 1.0,
            focus: None,
        };
        let first = fill_transforms(plan(&graph, extent, &at_origin));
        let second = fill_transforms(plan(&graph, extent, &offset));
        assert!(!first.is_empty(), "the graph emits a fill");
        assert_eq!(first, second, "the plan translates no operation");
        // The two plans really are different plans of the same page.
        assert_ne!(
            port_regions(plan(&graph, extent, &at_origin)),
            port_regions(plan(&graph, extent, &offset))
        );
    }

    // ARQ-149: no task of the plan has an empty region, and a region never
    // leaves the page or the viewport.
    #[test]
    fn arq_149_no_task_of_the_plan_has_an_empty_region() {
        let page = new_page(1000, 1000);
        let graph = build(&page);
        let extent = PageExtent {
            width: 1000,
            height: 1000,
        };
        let page_rect = IRect::new(0, 0, 1000, 1000);
        for rect in [
            page_rect,
            IRect::new(0, 0, 100, 100),
            IRect::new(900, 900, 1600, 1600),
            IRect::new(1000, 1000, 1200, 1200),
            IRect::new(2000, 2000, 2100, 2100),
        ] {
            let viewport = Viewport {
                rect,
                zoom: 1.0,
                focus: None,
            };
            let regions = port_regions(plan(&graph, extent, &viewport));
            assert!(
                regions.iter().all(|(_, region)| !region.is_empty()),
                "{rect:?} plans an empty region"
            );
            for (_, region) in &regions {
                assert_eq!(
                    *region,
                    region.intersect(rect),
                    "{rect:?} plans {region:?}, outside the viewport"
                );
                assert_eq!(
                    *region,
                    region.intersect(page_rect),
                    "{rect:?} plans {region:?}, outside the page"
                );
            }
            if rect.intersect(page_rect).is_empty() {
                assert!(regions.is_empty(), "{rect:?} plans {regions:?}");
            }
        }
    }

    // ARQ-140: the same graph gives the same plan.
    #[test]
    fn arq_140_the_same_graph_gives_the_same_plan() {
        let page = new_page(1000, 1000);
        let graph = build(&page);
        let extent = PageExtent {
            width: 1000,
            height: 1000,
        };
        let viewport = Viewport {
            rect: IRect::new(0, 0, 600, 600),
            zoom: 1.0,
            focus: Some((300, 300)),
        };
        let first = plan(&graph, extent, &viewport);
        let second = plan(&graph, extent, &viewport);
        assert_eq!(tiles_of(&first), tiles_of(&second));
    }

    // ARQ-139: the planning reports the progress and honours the cancel.
    #[test]
    fn arq_139_the_planning_reports_the_progress_and_honours_the_cancel() {
        let page = new_page(2000, 2000);
        let graph = build(&page);
        let extent = PageExtent {
            width: 2000,
            height: 2000,
        };
        let viewport = Viewport {
            rect: IRect::new(0, 0, 2000, 2000),
            zoom: 1.0,
            focus: None,
        };
        let result = check_progress_contract(0.02, |token, progress| -> Result<bool, PlanError> {
            match Planner::plan(&graph, extent, &viewport, token, progress) {
                Ok(_) => Ok(false),
                Err(PlanError::Cancelled(_)) => Ok(true),
            }
        });
        assert_eq!(result, Ok(Ok(())));
    }

    // ARQ-42, O12: a region is one task, and the task carries the operations of
    // every root in paint order; the second root no longer gets a task of its
    // own that would erase the first.
    #[test]
    fn arq_42_a_region_is_one_task_with_the_ops_of_every_root_in_paint_order() {
        let page = two_root_page();
        let graph = build(&page);
        assert_eq!(graph.roots().len(), 2, "the page has two top-level shapes");
        let plan = merged_plan(&page);

        assert_eq!(plan.tiles().len(), 1, "one region, one task");
        let tile = &plan.tiles()[0];
        assert_eq!(
            tile.region,
            IRect::new(0, 0, 100, 100),
            "the task keeps the region of its tile"
        );
        assert_eq!(
            fill_colors(tile),
            vec![red(), blue()],
            "both roots, in the order the page paints them"
        );
    }

    // ARQ-42, O12: the one content key of the region changes when either root
    // changes, so a stale tile can never be served for a changed root, and the
    // order of the roots is part of the key.
    #[test]
    fn arq_42_the_merged_key_changes_when_either_root_changes() {
        let base = merged_plan(&two_root_page()).tiles()[0].key.content;

        let mut first_changed = two_root_page();
        add_root(&mut first_changed, 1, 0.0, 0.0, blue());
        let changed_first = merged_plan(&first_changed).tiles()[0].key.content;
        assert_ne!(base, changed_first, "the first root is in the key");

        let mut second_changed = two_root_page();
        add_root(&mut second_changed, 2, 20.0, 0.0, red());
        let changed_second = merged_plan(&second_changed).tiles()[0].key.content;
        assert_ne!(base, changed_second, "the second root is in the key");
        assert_ne!(
            changed_first, changed_second,
            "each root keeps its own contribution"
        );

        // Paint order is content: the same two shapes back to front are another
        // image, so they cannot share a key.
        let mut swapped = two_root_page();
        swapped.roots = vec![NodeId::new(2), NodeId::new(1)];
        let swapped_key = merged_plan(&swapped).tiles()[0].key.content;
        assert_ne!(base, swapped_key, "the order of the roots is in the key");
    }
}
