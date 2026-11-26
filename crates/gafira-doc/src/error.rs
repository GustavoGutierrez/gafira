//! The error type of `Document::validate`.

use std::fmt;

use crate::node::MAX_EXTENSION_DEPTH;
use crate::timeline::TimelineError;
use crate::{ClipId, Document, Dpi, NodeId, PageId, PageLayoutError, PageSizeError, SequenceId};

/// What is wrong with a shape path (spec 054).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PathProblem {
    /// It has no order.
    Empty,
    /// Its first order is not a `MoveTo`.
    NoStart,
    /// It has more than `ShapePath::MAX_COMMANDS` orders.
    TooLong { len: usize },
}

impl fmt::Display for PathProblem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => f.write_str("the path has no order"),
            Self::NoStart => f.write_str("the path does not start with move_to"),
            Self::TooLong { len } => write!(f, "the path has {len} orders, more than allowed"),
        }
    }
}

/// A numeric field of a node, named in value errors.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NodeField {
    /// `placement.x`.
    X,
    /// `placement.y`.
    Y,
    /// `placement.w`.
    W,
    /// `placement.h`.
    H,
    /// `placement.rotation_deg`.
    RotationDeg,
    /// `placement.skew`.
    Skew,
    /// `opacity`.
    Opacity,
    /// A red, green or blue channel of a shape fill.
    FillChannel,
    /// The alpha channel of a shape fill.
    FillAlpha,
    /// A red, green or blue channel of a shape outline (spec 038).
    StrokeChannel,
    /// The alpha channel of a shape outline.
    StrokeAlpha,
    /// The width of a shape outline.
    StrokeWidth,
    /// The corner radius of a shape.
    CornerRadius,
    /// The font size of a text span (spec 027).
    FontSize,
    /// A red, green or blue channel of a text span colour.
    TextColorChannel,
    /// The alpha channel of a text span colour.
    TextColorAlpha,
    /// A coordinate of a point of a shape path (spec 054).
    PathCoordinate,
    /// How much a text bends (FD-44).
    TextCurve,
    /// The letter spacing of a text (FD-41).
    LetterSpacing,
    /// The line height of a text (FD-41).
    LineHeight,
}

impl fmt::Display for NodeField {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::X => "x",
            Self::Y => "y",
            Self::W => "w",
            Self::H => "h",
            Self::RotationDeg => "rotation_deg",
            Self::Skew => "skew",
            Self::Opacity => "opacity",
            Self::FillChannel => "fill channel",
            Self::FillAlpha => "fill alpha",
            Self::StrokeChannel => "stroke channel",
            Self::StrokeAlpha => "stroke alpha",
            Self::StrokeWidth => "stroke width",
            Self::PathCoordinate => "path coordinate",
            Self::TextCurve => "text curve",
            Self::LetterSpacing => "letter spacing",
            Self::LineHeight => "line height",
            Self::CornerRadius => "corner radius",
            Self::FontSize => "font size",
            Self::TextColorChannel => "text color channel",
            Self::TextColorAlpha => "text color alpha",
        })
    }
}

/// The first rule a document breaks, in the fixed pass order P1 to P6.
///
/// Variants are listed in pass order; `Document::validate` returns the
/// earliest one.
///
/// The enum is `#[non_exhaustive]`: a downstream `match` needs a wildcard arm,
/// so this does not compile outside the crate:
///
/// ```compile_fail
/// use gafira_doc::ValidationError::*;
/// fn rule(e: gafira_doc::ValidationError) {
///     match e {
///         UnsupportedSchemaVersion { .. } | InvalidDpi { .. } | DuplicateId { .. }
///         | ArenaKeyMismatch { .. } | NextIdNotAboveMax { .. } | PageSize { .. }
///         | PageLayout { .. } | DanglingRoot { .. } | DuplicateRoot { .. } | NonFinite { .. }
///         | OutOfRange { .. } | EmptyExtensionTypeId { .. } | ExtensionTooDeep { .. }
///         | ChildrenOnLeaf { .. } | DanglingChild { .. } | MultipleParents { .. }
///         | RootHasParent { .. } | Orphan { .. } | Cycle { .. } | DanglingScene { .. }
///         | DanglingAsset { .. } | DanglingImage { .. } | TimelineLaw { .. } => {}
///     }
/// }
/// ```
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq)]
pub enum ValidationError {
    /// P1: `schema_version` is not the supported one.
    UnsupportedSchemaVersion { found: u32 },
    /// P1: the document DPI is outside `Dpi::MIN..=Dpi::MAX`.
    InvalidDpi { dpi: u32 },
    /// P2: a page or node id is used twice in the document.
    DuplicateId { id: u64 },
    /// P2: an arena key differs from the id of the node stored under it.
    ArenaKeyMismatch {
        page: PageId,
        key: NodeId,
        id: NodeId,
    },
    /// P2: `next_id` is not above the largest id in use.
    NextIdNotAboveMax { next_id: u64, max_id: u64 },
    /// P3: a page side is outside the pixel limits at the document DPI.
    PageSize { page: PageId, error: PageSizeError },
    /// P3: bleed, margins, guides or grid of a page break their ranges.
    PageLayout {
        page: PageId,
        error: PageLayoutError,
    },
    /// P3: a root id is not in its page's arena.
    DanglingRoot { page: PageId, node: NodeId },
    /// P3: a root id is listed twice.
    DuplicateRoot { page: PageId, node: NodeId },
    /// P4: a number is NaN or infinite.
    NonFinite { node: NodeId, field: NodeField },
    /// P4: a finite number is outside its allowed range.
    OutOfRange {
        node: NodeId,
        field: NodeField,
        value: f64,
    },
    /// P4: an extension node has an empty `type_id`.
    EmptyExtensionTypeId { node: NodeId },
    /// P4: an extension payload nests deeper than `MAX_EXTENSION_DEPTH`.
    ExtensionTooDeep { node: NodeId, depth: usize },
    /// P4: a shape path is not well formed (spec 054).
    InvalidPath { node: NodeId, problem: PathProblem },
    /// P3/P7: a page background cannot be drawn or shows an image missing
    /// from the asset table (spec 069).
    InvalidBackground { page: PageId },
    /// P4/P7: a frame has numbers it cannot draw or a photo missing from
    /// the asset table (spec 066).
    InvalidFrame { node: NodeId },
    /// P4/P7: a QR code has numbers or colours it cannot draw, or a logo
    /// missing from the asset table (spec 080).
    InvalidQr { node: NodeId },
    /// P4: a table whose sizes, cells and merges do not agree (spec 081).
    InvalidTable { node: NodeId },
    /// P4: a text whose effects are too many or out of range (spec 089).
    InvalidText { node: NodeId },
    /// P4: an image whose look has numbers out of their range (spec 085).
    InvalidImage { node: NodeId },
    /// P4: a chart whose data or numbers cannot be drawn (spec 083).
    InvalidChart { node: NodeId },
    /// P4/P7: a 3D model whose pose or camera cannot be drawn, or whose model file is not
    /// in the asset table (spec 100).
    InvalidModel3d { node: NodeId },
    /// P4: an effect is not well formed (spec 065).
    InvalidEffect {
        node: NodeId,
        problem: crate::EffectProblem,
    },
    /// P4: a gradient fill is not well formed (spec 063).
    InvalidGradient {
        node: NodeId,
        problem: crate::GradientProblem,
    },
    /// P4: a shape or extension lists children.
    ChildrenOnLeaf { node: NodeId },
    /// P4: a child id is not in the parent's arena.
    DanglingChild { parent: NodeId, child: NodeId },
    /// P5: a node is listed as a child more than once.
    MultipleParents { node: NodeId },
    /// P5: a root is also listed as a child.
    RootHasParent { node: NodeId, parent: NodeId },
    /// P5: a node is neither a root nor a child.
    Orphan { node: NodeId },
    /// P6: a node cannot be reached from any root, so it is in or below a cycle.
    Cycle { node: NodeId },
    /// P7: a `Scene` clip names a page the document does not have.
    DanglingScene { clip: ClipId, page: PageId },
    /// P7: a `Media` or `Audio` clip names an entry the asset table does not
    /// have (P9 of the timeline design).
    DanglingAsset { clip: ClipId },
    /// P7: an image element names an entry the asset table does not have
    /// (spec 029).
    DanglingImage { node: NodeId },
    /// P8: a sequence breaks a law of the timeline model (FV-01, FV-03, P11).
    TimelineLaw {
        sequence: SequenceId,
        error: TimelineError,
    },
    /// P9: the connected data breaks a rule of its table (spec 104).
    InvalidData { problem: crate::DataProblem },
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedSchemaVersion { found } => write!(
                f,
                "unsupported schema version {found} (expected {})",
                Document::SCHEMA_VERSION
            ),
            Self::InvalidDpi { dpi } => {
                write!(f, "dpi {dpi} is outside {}..={}", Dpi::MIN, Dpi::MAX)
            }
            Self::DuplicateId { id } => write!(f, "id {id} is used more than once"),
            Self::ArenaKeyMismatch { page, key, id } => {
                write!(f, "page {page}: arena key {key} holds node {id}")
            }
            Self::NextIdNotAboveMax { next_id, max_id } => {
                write!(f, "next_id {next_id} is not above the largest id {max_id}")
            }
            Self::PageSize { page, error } => {
                write!(f, "page {page} has an invalid size: {error}")
            }
            Self::PageLayout { page, error } => write!(f, "page {page}: {error}"),
            Self::DanglingRoot { page, node } => {
                write!(f, "page {page}: root {node} is not in the page")
            }
            Self::DuplicateRoot { page, node } => {
                write!(f, "page {page}: root {node} is listed twice")
            }
            Self::NonFinite { node, field } => write!(f, "node {node}: {field} is not finite"),
            Self::OutOfRange { node, field, value } => {
                write!(f, "node {node}: {field} is {value}, out of range")
            }
            Self::EmptyExtensionTypeId { node } => {
                write!(f, "node {node}: extension type_id is empty")
            }
            Self::ExtensionTooDeep { node, depth } => write!(
                f,
                "node {node}: extension payload is {depth} levels deep, the limit is {MAX_EXTENSION_DEPTH}"
            ),
            Self::InvalidPath { node, problem } => write!(f, "node {node}: {problem}"),
            Self::InvalidBackground { page } => {
                write!(f, "page {page}: the background is not valid")
            }
            Self::InvalidQr { node } => {
                write!(f, "node {node}: the QR code is not valid")
            }
            Self::InvalidTable { node } => {
                write!(f, "node {node}: the table is not valid")
            }
            Self::InvalidText { node } => {
                write!(f, "node {node}: the effects of the text are not valid")
            }
            Self::InvalidImage { node } => {
                write!(f, "node {node}: the look of the image is not valid")
            }
            Self::InvalidChart { node } => {
                write!(f, "node {node}: the chart is not valid")
            }
            Self::InvalidModel3d { node } => {
                write!(f, "node {node}: the 3D model is not valid")
            }
            Self::InvalidFrame { node } => {
                write!(f, "node {node}: the frame is not valid")
            }
            Self::InvalidEffect { node, problem } => {
                write!(f, "node {node}: the effect is not valid ({problem:?})")
            }
            Self::InvalidGradient { node, problem } => {
                write!(f, "node {node}: the gradient is not valid ({problem:?})")
            }
            Self::ChildrenOnLeaf { node } => {
                write!(f, "node {node}: only groups can have children")
            }
            Self::DanglingChild { parent, child } => {
                write!(f, "node {parent}: child {child} is not in the page")
            }
            Self::MultipleParents { node } => write!(f, "node {node}: has more than one parent"),
            Self::RootHasParent { node, parent } => {
                write!(f, "node {node}: is a root but has parent {parent}")
            }
            Self::Orphan { node } => write!(f, "node {node}: is not a root and has no parent"),
            Self::Cycle { node } => write!(f, "node {node}: is in or below a cycle"),
            Self::DanglingScene { clip, page } => {
                write!(
                    f,
                    "clip {clip}: the page {page} it shows is not in the document"
                )
            }
            Self::DanglingAsset { clip } => {
                write!(f, "clip {clip}: the asset it names is not in the table")
            }
            Self::DanglingImage { node } => {
                write!(f, "image {node}: the asset it shows is not in the table")
            }
            Self::TimelineLaw { sequence, error } => {
                write!(f, "sequence {sequence}: {error}")
            }
            Self::InvalidData { problem } => {
                write!(f, "the connected data is not valid ({problem:?})")
            }
        }
    }
}

impl std::error::Error for ValidationError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::PageSize { error, .. } => Some(error),
            Self::PageLayout { error, .. } => Some(error),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use super::*;
    use crate::{Axis, NodeId, PageField, PageId, PageSizeError};

    fn node(v: u64) -> NodeId {
        NodeId::new(v)
    }

    fn page(v: u64) -> PageId {
        PageId::new(v)
    }

    fn too_tall() -> PageSizeError {
        PageSizeError::OutOfRange {
            axis: Axis::Height,
            px: 20000.5,
        }
    }

    #[test]
    fn every_variant_has_a_distinct_message_naming_ids_and_field() {
        use ValidationError::*;
        let (n, f) = (node(4), NodeField::Opacity);
        let errors = [
            UnsupportedSchemaVersion { found: 7 },
            InvalidDpi { dpi: 0 },
            DuplicateId { id: 5 },
            ArenaKeyMismatch {
                page: page(1),
                key: node(5),
                id: node(6),
            },
            NextIdNotAboveMax {
                next_id: 3,
                max_id: 9,
            },
            PageSize {
                page: page(1),
                error: too_tall(),
            },
            PageLayout {
                page: page(1),
                error: PageLayoutError::OutOfRange {
                    field: PageField::Bleed,
                    value: 11.0,
                },
            },
            DanglingRoot {
                page: page(1),
                node: node(2),
            },
            DuplicateRoot {
                page: page(1),
                node: node(2),
            },
            NonFinite { node: n, field: f },
            OutOfRange {
                node: n,
                field: f,
                value: 1.5,
            },
            EmptyExtensionTypeId { node: n },
            ExtensionTooDeep { node: n, depth: 65 },
            ChildrenOnLeaf { node: n },
            DanglingChild {
                parent: node(2),
                child: node(99),
            },
            MultipleParents { node: n },
            RootHasParent {
                node: n,
                parent: node(2),
            },
            Orphan { node: n },
            Cycle { node: n },
        ];
        let texts = [
            "unsupported schema version 7 (expected 1)",
            "dpi 0 is outside 1..=10000",
            "id 5 is used more than once",
            "page 1: arena key 5 holds node 6",
            "next_id 3 is not above the largest id 9",
            "page 1 has an invalid size: page height is 20000.5 px, outside 1..=16384 px",
            "page 1: bleed is 11, out of range",
            "page 1: root 2 is not in the page",
            "page 1: root 2 is listed twice",
            "node 4: opacity is not finite",
            "node 4: opacity is 1.5, out of range",
            "node 4: extension type_id is empty",
            "node 4: extension payload is 65 levels deep, the limit is 64",
            "node 4: only groups can have children",
            "node 2: child 99 is not in the page",
            "node 4: has more than one parent",
            "node 4: is a root but has parent 2",
            "node 4: is not a root and has no parent",
            "node 4: is in or below a cycle",
        ];
        let mut kinds = Vec::new();
        for (e, text) in errors.iter().zip(texts) {
            assert_eq!(e.to_string(), text);
            let kind = std::mem::discriminant(e);
            assert!(!kinds.contains(&kind), "{e:?} repeats a variant");
            kinds.push(kind);
        }
        assert_eq!(kinds.len(), 19);
    }

    #[test]
    fn node_fields_have_names() {
        use NodeField::*;
        let rows = [
            (X, "x"),
            (Y, "y"),
            (W, "w"),
            (H, "h"),
            (RotationDeg, "rotation_deg"),
            (Skew, "skew"),
            (Opacity, "opacity"),
            (FillChannel, "fill channel"),
            (FillAlpha, "fill alpha"),
            (StrokeChannel, "stroke channel"),
            (StrokeAlpha, "stroke alpha"),
            (StrokeWidth, "stroke width"),
            (CornerRadius, "corner radius"),
            (FontSize, "font size"),
            (TextColorChannel, "text color channel"),
            (TextColorAlpha, "text color alpha"),
        ];
        for (field, name) in rows {
            assert_eq!(field.to_string(), name);
        }
    }

    #[test]
    fn errors_are_comparable_clone_and_expose_the_page_size_cause() {
        let e = ValidationError::PageSize {
            page: page(1),
            error: too_tall(),
        };
        assert_eq!(e.clone(), e);
        assert_ne!(e, ValidationError::Cycle { node: node(1) });
        let dynamic: &dyn Error = &e;
        assert_eq!(
            dynamic.source().unwrap().to_string(),
            too_tall().to_string()
        );
        assert!(ValidationError::Cycle { node: node(1) }.source().is_none());
        assert!(ValidationError::InvalidDpi { dpi: 0 }.source().is_none());
    }

    #[test]
    fn a_page_layout_error_exposes_its_cause() {
        let cause = PageLayoutError::OutOfRange {
            field: PageField::Bleed,
            value: 11.0,
        };
        let e = ValidationError::PageLayout {
            page: page(1),
            error: cause,
        };
        let dynamic: &dyn Error = &e;
        assert_eq!(dynamic.source().unwrap().to_string(), cause.to_string());
    }
}
