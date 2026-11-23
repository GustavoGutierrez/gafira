//! The page scope of an export (FD-57, FD-58).
//!
//! `ExportScope` says what an export covers; this module resolves it against
//! the document, before any render starts, and turns it into one
//! [`PageRenderRequest`] per page. A page the document does not hold is
//! rejected here and not in the renderer: `RenderError` has no variant that
//! can carry that, the port documents [`PageRenderer::render`](gafira_ports::export::PageRenderer::render) as answering
//! only [`RenderError::Cancelled`](gafira_ports::render::RenderError::Cancelled), and the double of ARQ-112 reads `render`
//! as total over [`PageId`]. FD-58 wants the check before the export begins,
//! which is this mapping.

use gafira_doc::{Document, PageId};
use gafira_ports::export::{ExportScope, PageRenderRequest};

/// Why a page scope cannot be mapped to render requests (FD-57, FD-58).
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScopeError {
    /// The scope names a page the document does not hold (FD-58).
    OutOfRange {
        /// The page the scope named.
        page: PageId,
        /// How many pages the document holds.
        pages: usize,
    },
}

impl std::fmt::Display for ScopeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::OutOfRange { page, pages } => write!(
                f,
                "page {page} is outside the {pages} page(s) of the document"
            ),
        }
    }
}

impl std::error::Error for ScopeError {}

/// The pages an export covers, checked against the document before any render
/// starts (FD-57, FD-58).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PageScope {
    pages: Vec<PageId>,
}

impl PageScope {
    /// Resolves the scope of an export against the document.
    ///
    /// A scope of pages holds a single page or a range the caller already
    /// resolved (02 §4.5); a scope of nodes holds the page of the selection.
    /// Every page is checked here, so no render starts for a page the document
    /// does not hold (FD-58).
    ///
    /// # Errors
    /// [`ScopeError::OutOfRange`] when the document does not hold a page.
    pub fn resolve(document: &Document, scope: &ExportScope) -> Result<Self, ScopeError> {
        let pages = match scope {
            ExportScope::Pages(pages) => pages.clone(),
            ExportScope::Nodes { page, .. } => vec![*page],
        };
        let total = document.pages().len();
        for page in &pages {
            if document.page(*page).is_none() {
                return Err(ScopeError::OutOfRange {
                    page: *page,
                    pages: total,
                });
            }
        }
        Ok(Self { pages })
    }

    /// The pages of the scope, in the order the caller named them (FD-57 (a)).
    #[must_use]
    pub fn pages(&self) -> &[PageId] {
        &self.pages
    }

    /// One request per page, with the options of `template` (FD-57).
    ///
    /// The template carries what does not change from page to page: the
    /// instant, the scale, the region and the transparency. The page is the
    /// one of the scope.
    #[must_use]
    pub fn requests(&self, template: &PageRenderRequest) -> Vec<PageRenderRequest> {
        self.pages
            .iter()
            .map(|page| PageRenderRequest {
                page: *page,
                ..*template
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use gafira_doc::{
        Document, Dpi, ElementBase, Node, NodeId, NodeKind, Page, PageId, PageSize, Placement,
        ShapeGeometry, ShapeNode,
    };
    use gafira_geom::{IRect, Length, Unit};
    use gafira_time::Flicks;

    use super::*;

    /// A page of the given size in pixels, with one filled rectangle.
    fn page(id: PageId, node: NodeId, width: u32, height: u32) -> Page {
        let mut page = Page::new(
            id,
            "P".to_string(),
            PageSize {
                width: Length::new(f64::from(width), Unit::Px),
                height: Length::new(f64::from(height), Unit::Px),
            },
        );
        let base = ElementBase::new(node, Placement::new(0.0, 0.0, 10.0, 10.0));
        let kind = NodeKind::Shape(ShapeNode {
            geometry: ShapeGeometry::Rect,
            fill: None,
            stroke: None,
            corner_radius: 0.0,
            text: None,
            gradient: None,
            effect: None,
            drawn_with: None,
        });
        page.nodes.insert(node, Arc::new(Node::new(base, kind)));
        page.roots = vec![node];
        page
    }

    /// A document of the given page sizes, each page with one shape.
    fn document(sizes: &[(u32, u32)]) -> Document {
        let mut next_id = 1u64;
        let mut pages = Vec::with_capacity(sizes.len());
        for (width, height) in sizes {
            let id = PageId::new(next_id);
            next_id += 1;
            let node = NodeId::new(next_id);
            next_id += 1;
            pages.push(Arc::new(page(id, node, *width, *height)));
        }
        Document::from_parts(next_id, Dpi::default(), pages).expect("the document validates")
    }

    /// One request over a region of the page.
    fn request(page: PageId, region: IRect, scale: f32, transparent: bool) -> PageRenderRequest {
        PageRenderRequest {
            page,
            time: Flicks::from_ticks(0),
            scale,
            region,
            transparent,
        }
    }

    // FD-57 (a), AC4: a single page and a range map to one request per page,
    // in the order the caller named them.
    #[test]
    fn fd_57_a_single_page_and_a_range_map_to_requests() {
        let document = document(&[(10, 10); 5]);
        let ids: Vec<PageId> = document.pages().iter().map(|page| page.id).collect();
        let template = request(PageId::new(0), IRect::new(0, 0, 10, 10), 1.0, true);

        let single = PageScope::resolve(&document, &ExportScope::Pages(vec![ids[0]]))
            .expect("one page is in range");
        assert_eq!(single.pages(), &[ids[0]]);
        let requests = single.requests(&template);
        assert_eq!(requests.len(), 1);
        assert_eq!(
            requests[0],
            PageRenderRequest {
                page: ids[0],
                ..template
            }
        );

        // The range «1-3, 5» of FD-57 exports exactly the pages 1, 2, 3 and 5.
        let range = vec![ids[0], ids[1], ids[2], ids[4]];
        let scope = PageScope::resolve(&document, &ExportScope::Pages(range.clone()))
            .expect("the range is in range");
        assert_eq!(scope.pages(), range.as_slice());
        let pages: Vec<PageId> = scope
            .requests(&template)
            .iter()
            .map(|request| request.page)
            .collect();
        assert_eq!(pages, range);

        // A selection lives on one page (FD-57).
        let selection = PageScope::resolve(
            &document,
            &ExportScope::Nodes {
                page: ids[3],
                nodes: Vec::new(),
            },
        )
        .expect("the page of the selection is in range");
        assert_eq!(selection.pages(), &[ids[3]]);
    }

    // FD-58, AC4: a page outside the document is rejected before any render
    // starts, and the mapping answers no request at all.
    #[test]
    fn fd_58_an_out_of_range_page_is_rejected_before_any_render() {
        let document = document(&[(10, 10); 3]);
        let ids: Vec<PageId> = document.pages().iter().map(|page| page.id).collect();
        let missing = PageId::new(1_000);

        let result = PageScope::resolve(&document, &ExportScope::Pages(vec![ids[0], missing]));
        assert_eq!(
            result,
            Err(ScopeError::OutOfRange {
                page: missing,
                pages: 3,
            })
        );
        // A selection on a page outside the document is rejected the same way.
        assert_eq!(
            PageScope::resolve(
                &document,
                &ExportScope::Nodes {
                    page: missing,
                    nodes: Vec::new(),
                },
            ),
            Err(ScopeError::OutOfRange {
                page: missing,
                pages: 3,
            })
        );
    }
}
