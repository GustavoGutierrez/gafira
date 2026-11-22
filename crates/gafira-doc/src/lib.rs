//! Layer-1 document model for Gafira: `Document -> Page -> Node`.
//!
//! Pure domain code: no I/O, no `unsafe`. Depends only on `gafira-geom`,
//! `gafira-color` and `gafira-time`, the timeline edge of ADR 0007 whose
//! trigger is `Page.duration`; geometry types are always imported through
//! `gafira_geom`.

#![forbid(unsafe_code)]

mod arena;
mod background;
mod blend;
mod chart;
mod content_hash;
pub mod contract;
mod document;
mod dpi;
mod edit;
mod effect;
mod element;
mod error;
mod frame;
mod gradient;
mod ids;
mod image;
mod image_look;
mod json;
mod layout;
mod line;
pub mod msg;
mod node;
mod page;
mod page_size;
mod placement;
mod project_id;
mod qr;
mod table;
mod text;
pub mod timeline;
mod validate;
pub mod visit;

pub use background::{BackgroundFit, PageBackground, PatternKind};
pub use blend::BlendMode;
pub use chart::{ChartCombine, ChartKind, ChartLabels, ChartNode, ChartText, MAX_CHART_COLORS};
pub use content_hash::ContentHash;
pub use document::Document;
pub use dpi::Dpi;
pub use edit::{DocMut, EditToken};
pub use effect::{EffectProblem, Light, ShapeEffect};
pub use element::ElementBase;
pub use error::{NodeField, PathProblem, ValidationError};
pub use frame::{Fit, FrameContent, FrameNode};
pub use gradient::{Gradient, GradientKind, GradientProblem, GradientStop};
pub use ids::{ClipId, IdSpaceExhausted, MarkerId, NodeId, PageId, SequenceId, TrackId};
pub use image::{ImageEdit, ImageNode};
pub use image_look::{
    ColorShift, ImageAdjust, ImageFilter, ImageLook, ImageShadow, ImageZone, ShadowKind, ZoneKind,
};
pub use json::LoadError;
pub use layout::{
    GridSettings, GridStyle, Guide, GuideAxis, GuideSet, Margins, PageField, PageLayoutError,
    check_bleed,
};
pub use line::{Anchor, Attachment, LineCap, LineEnd, LineNode};
pub use node::{
    ExtensionNode, GroupNode, MAX_EXTENSION_DEPTH, Node, NodeKind, PathCommand, ShapeGeometry,
    ShapeNode, ShapePath, ShapeStroke, ShapeText, VerticalAlign,
};
pub use page::Page;
pub use page_size::{Axis, PageSize, PageSizeError};
pub use placement::{Flip, Placement};
pub use project_id::{ProjectId, ProjectIdError};
pub use qr::{
    QrBody, QrCoin, QrContact, QrContactFormat, QrContent, QrEcc, QrEyeBall, QrEyeFrame, QrFill,
    QrLogo, QrNode, QrStyle, QrWifiSecurity,
};
pub use table::{CellBorders, CellMerge, CellStyle, TableCell, TableNode};
pub use text::{
    Paragraph, RichText, Span, SpanStyle, TextAlign, TextNode, TextSizing, face_key, nearest_face,
};
pub use timeline::{
    AssetEntry, AssetRef, AssetStorage, AssetTable, Clip, ClipSource, Marker, PixelSize, Sequence,
    Speed, TimelineError, Track, TrackKind,
};
