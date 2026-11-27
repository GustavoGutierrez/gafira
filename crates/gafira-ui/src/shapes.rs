//! The shapes of the Formas panel (specs 053 and 054, D-95): which shapes
//! there are, how the panel groups them, and the path each one draws in the
//! unit box. Pure, so it is tested without a window.
//!
//! Every path is built with `kurbo` in its own coordinates, each closed
//! subpath turned the same way so the non-zero rule fills their union (a
//! hole is turned the other way), and then scaled so its exact bounds are
//! the unit box. The aspect it had before scaling is the one a new shape
//! takes on the page.

use gafira_doc::{LineCap, PathCommand, ShapeGeometry, ShapePath};
use kurbo::{Affine, BezPath, Circle, Ellipse, PathEl, Point, RoundedRect, Shape};

/// A shape of the Formas panel.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Piece {
    /// The lines (spec 058): circles at both ends, hollow diamonds, filled
    /// diamonds, no caps, and a square start with an arrow end.
    LineCircles,
    LineDiamonds,
    LineDiamondsFilled,
    LinePlain,
    LineArrow,
    LineSquares,
    LineSquaresFilled,
    LineDashed,
    LineDotted,
    LineCirclesFilled,
    LineBars,
    LineArrowBoth,
    LineArrowEnd,
    LineArrowDotted,
    LineArrowBothDotted,
    Square,
    RoundedSquare,
    Circle,
    Triangle,
    TriangleDown,
    Diamond,
    Plus,
    Barrel,
    Ticket,
    Parallelogram,
    ParallelogramLeft,
    Trapezoid,
    TrapezoidDown,
    ArchDown,
    ArchUp,
    RightTriangle,
    HalfCircle,
    QuarterCircle,
    QuarterRing,
    HalfRing,
    Pentagon,
    Hexagon,
    HexagonFlat,
    Heptagon,
    Octagon,
    Nonagon,
    Decagon,
    Dodecagon,
    Star4,
    Star5,
    Star6,
    Star7,
    Star8,
    Star12,
    Seal,
    Burst,
    ArrowRight,
    ArrowLeft,
    ArrowUp,
    ArrowDown,
    ArrowLeftRight,
    ArrowUpDown,
    Chevron,
    ArrowNotched,
    Preparation,
    Terminator,
    Decision,
    Data,
    Document,
    Delay,
    ManualInput,
    Card,
    Database,
    StoredData,
    Display,
    MultiDocument,
    PredefinedProcess,
    InternalStorage,
    LoopLimit,
    Sort,
    Collate,
    OrGate,
    SummingJunction,
    PunchedTape,
    SequentialAccess,
    DirectData,
    Cube,
    Step,
    SpeechSquare,
    SpeechRound,
    SpeechCloud,
    SpeechRounded,
    SpeechOval,
    SpeechThought,
    SpeechCloudWide,
    SpeechCloudFlat,
    SpeechCloudRound,
    SpeechCloudThink,
    SpeechCloudThinkRight,
    SpeechCloudSoft,
    SpeechCloudCurl,
    SpeechHand,
    SpeechHandThink,
    SpeechHandWide,
    SpeechOvalCurved,
    SpeechDouble,
    Cloud,
    CloudFlat,
    CloudPuffy,
    CloudSmall,
    CloudBig,
    Heart,
    HeartWide,
    HeartNarrow,
    HeartRound,
    HeartSharp,
    HeartSoft,
    Ribbon,
    Pennant,
    Bookmark,
    LabelClassic,
    LabelScalloped,
    LabelTall,
    LabelBadge,
    RibbonCurved,
    RibbonStraight,
    RibbonSmile,
    TagRect,
    TagRounded,
    TagArch,
    TagCut,
    TagNotched,
    TagBarrel,
    TagOval,
    TagShield,
    TagEgg,
    TagHexagon,
    TagStamp,
    TagCircle,
    TagHeart,
    TagQuatrefoil,
    TagStar,
    TagScalloped,
    Drop,
    DropWide,
    DropTall,
    DropFat,
    DropDown,
    DropSide,
    DropFlame,
    DropHole,
    Gear8,
    Gear12,
    Gear10,
    Gear14Sharp,
    Gear16,
    GearSolid,
    Asterisk,
    Cross,
    Asterisk8,
    AsteriskBold8,
    AsteriskBlock5,
    Asterisk5,
    Asterisk6Bar,
    Asterisk12,
    AsteriskRound,
    AsteriskX,
    AsteriskPinwheel,
    Flower6,
    Flower8,
    FlowerThin,
    FlowerLong,
    FlowerHole,
    Clover,
    PetalCross,
    Leaf,
    WaveRipple,
    WaveBand,
    Blob,
    Pebble,
    AbstractCushion,
    AbstractStack,
    AbstractMoons,
    AbstractBlocks,
    AbstractSlant,
    AbstractPinwheel,
    AbstractSparkles,
    AbstractWindow,
    AbstractSteps,
    AbstractBean,
    /// «Comillas» (spec 098): double and single quotes in many styles.
    QuoteRound,
    QuoteRoundOpen,
    QuoteRoundSingle,
    QuoteHeavy,
    QuoteHeavyOpen,
    QuoteBlock,
    QuoteBlockOpen,
    QuoteBlockSingle,
    QuoteSoft,
    QuoteSoftOpen,
    QuoteSlant,
    QuoteWedge,
    QuoteWedgeOpen,
    QuoteThin,
    QuoteThinOpen,
    QuoteStraight,
    QuoteStraightSingle,
    QuoteAngle,
    QuoteAngleClose,
    QuoteAngleSingle,
    QuoteCircle,
    QuoteCircleOpen,
    QuoteSquare,
    QuoteSquareOpen,
    QuoteCut,
    QuoteCutClose,
    QuoteCutSingle,
    QuoteCutWide,
    QuoteCutSlim,
    QuoteCutDeep,
    /// Grunge shapes (spec 109): dry brush strokes, splatters and claw
    /// marks, by their number.
    Brush(u8),
    Splatter(u8),
    Claw(u8),
    /// The grunge shapes the owner drew (spec 109), by their number.
    Traced(u8),
}

impl Piece {
    /// Every shape, in the order of the panel.
    pub(crate) const ALL: [Self; 265] = [
        Self::LineCircles,
        Self::LineDiamonds,
        Self::LineDiamondsFilled,
        Self::LinePlain,
        Self::LineArrow,
        Self::LineSquares,
        Self::LineSquaresFilled,
        Self::LineDashed,
        Self::LineDotted,
        Self::LineCirclesFilled,
        Self::LineBars,
        Self::LineArrowBoth,
        Self::LineArrowEnd,
        Self::LineArrowDotted,
        Self::LineArrowBothDotted,
        Self::Square,
        Self::RoundedSquare,
        Self::Circle,
        Self::Triangle,
        Self::TriangleDown,
        Self::Diamond,
        Self::Plus,
        Self::Barrel,
        Self::Ticket,
        Self::Parallelogram,
        Self::ParallelogramLeft,
        Self::Trapezoid,
        Self::TrapezoidDown,
        Self::ArchDown,
        Self::ArchUp,
        Self::RightTriangle,
        Self::HalfCircle,
        Self::QuarterCircle,
        Self::QuarterRing,
        Self::HalfRing,
        Self::Pentagon,
        Self::Hexagon,
        Self::HexagonFlat,
        Self::Heptagon,
        Self::Octagon,
        Self::Nonagon,
        Self::Decagon,
        Self::Dodecagon,
        Self::Star4,
        Self::Star5,
        Self::Star6,
        Self::Star7,
        Self::Star8,
        Self::Star12,
        Self::Seal,
        Self::Burst,
        Self::ArrowRight,
        Self::ArrowLeft,
        Self::ArrowUp,
        Self::ArrowDown,
        Self::ArrowLeftRight,
        Self::ArrowUpDown,
        Self::Chevron,
        Self::ArrowNotched,
        Self::Preparation,
        Self::Terminator,
        Self::Decision,
        Self::Data,
        Self::Document,
        Self::Delay,
        Self::ManualInput,
        Self::Card,
        Self::Database,
        Self::StoredData,
        Self::Display,
        Self::MultiDocument,
        Self::PredefinedProcess,
        Self::InternalStorage,
        Self::LoopLimit,
        Self::Sort,
        Self::Collate,
        Self::OrGate,
        Self::SummingJunction,
        Self::PunchedTape,
        Self::SequentialAccess,
        Self::DirectData,
        Self::Cube,
        Self::Step,
        Self::SpeechSquare,
        Self::SpeechRound,
        Self::SpeechCloud,
        Self::SpeechRounded,
        Self::SpeechOval,
        Self::SpeechThought,
        Self::SpeechCloudWide,
        Self::SpeechCloudFlat,
        Self::SpeechCloudRound,
        Self::SpeechCloudThink,
        Self::SpeechCloudThinkRight,
        Self::SpeechCloudSoft,
        Self::SpeechCloudCurl,
        Self::SpeechHand,
        Self::SpeechHandThink,
        Self::SpeechHandWide,
        Self::SpeechOvalCurved,
        Self::SpeechDouble,
        Self::Cloud,
        Self::CloudFlat,
        Self::CloudPuffy,
        Self::CloudSmall,
        Self::CloudBig,
        Self::Heart,
        Self::HeartWide,
        Self::HeartNarrow,
        Self::HeartRound,
        Self::HeartSharp,
        Self::HeartSoft,
        Self::Ribbon,
        Self::Pennant,
        Self::Bookmark,
        Self::LabelClassic,
        Self::LabelScalloped,
        Self::LabelTall,
        Self::LabelBadge,
        Self::RibbonCurved,
        Self::RibbonStraight,
        Self::RibbonSmile,
        Self::TagRect,
        Self::TagRounded,
        Self::TagArch,
        Self::TagCut,
        Self::TagNotched,
        Self::TagBarrel,
        Self::TagOval,
        Self::TagShield,
        Self::TagEgg,
        Self::TagHexagon,
        Self::TagStamp,
        Self::TagCircle,
        Self::TagHeart,
        Self::TagQuatrefoil,
        Self::TagStar,
        Self::TagScalloped,
        Self::Drop,
        Self::DropWide,
        Self::DropTall,
        Self::DropFat,
        Self::DropDown,
        Self::DropSide,
        Self::DropFlame,
        Self::DropHole,
        Self::Gear8,
        Self::Gear12,
        Self::Gear10,
        Self::Gear14Sharp,
        Self::Gear16,
        Self::GearSolid,
        Self::Asterisk,
        Self::Cross,
        Self::Asterisk8,
        Self::AsteriskBold8,
        Self::AsteriskBlock5,
        Self::Asterisk5,
        Self::Asterisk6Bar,
        Self::Asterisk12,
        Self::AsteriskRound,
        Self::AsteriskX,
        Self::AsteriskPinwheel,
        Self::Flower6,
        Self::Flower8,
        Self::FlowerThin,
        Self::FlowerLong,
        Self::FlowerHole,
        Self::Clover,
        Self::PetalCross,
        Self::Leaf,
        Self::WaveRipple,
        Self::WaveBand,
        Self::Blob,
        Self::Pebble,
        Self::AbstractCushion,
        Self::AbstractStack,
        Self::AbstractMoons,
        Self::AbstractBlocks,
        Self::AbstractSlant,
        Self::AbstractPinwheel,
        Self::AbstractSparkles,
        Self::AbstractWindow,
        Self::AbstractSteps,
        Self::AbstractBean,
        Self::QuoteRound,
        Self::QuoteRoundOpen,
        Self::QuoteRoundSingle,
        Self::QuoteHeavy,
        Self::QuoteHeavyOpen,
        Self::QuoteBlock,
        Self::QuoteBlockOpen,
        Self::QuoteBlockSingle,
        Self::QuoteSoft,
        Self::QuoteSoftOpen,
        Self::QuoteSlant,
        Self::QuoteWedge,
        Self::QuoteWedgeOpen,
        Self::QuoteThin,
        Self::QuoteThinOpen,
        Self::QuoteStraight,
        Self::QuoteStraightSingle,
        Self::QuoteAngle,
        Self::QuoteAngleClose,
        Self::QuoteAngleSingle,
        Self::QuoteCircle,
        Self::QuoteCircleOpen,
        Self::QuoteSquare,
        Self::QuoteSquareOpen,
        Self::QuoteCut,
        Self::QuoteCutClose,
        Self::QuoteCutSingle,
        Self::QuoteCutWide,
        Self::QuoteCutSlim,
        Self::QuoteCutDeep,
        Self::Traced(0),
        Self::Traced(1),
        Self::Brush(0),
        Self::Brush(1),
        Self::Brush(2),
        Self::Brush(3),
        Self::Brush(4),
        Self::Brush(5),
        Self::Brush(6),
        Self::Brush(7),
        Self::Brush(8),
        Self::Brush(9),
        Self::Brush(10),
        Self::Brush(11),
        Self::Brush(12),
        Self::Brush(13),
        Self::Brush(14),
        Self::Brush(15),
        Self::Brush(16),
        Self::Brush(17),
        Self::Brush(18),
        Self::Brush(19),
        Self::Brush(20),
        Self::Brush(21),
        Self::Brush(22),
        Self::Brush(23),
        Self::Traced(2),
        Self::Traced(3),
        Self::Traced(7),
        Self::Traced(8),
        Self::Traced(9),
        Self::Splatter(0),
        Self::Splatter(1),
        Self::Splatter(2),
        Self::Splatter(3),
        Self::Splatter(4),
        Self::Splatter(5),
        Self::Splatter(6),
        Self::Splatter(7),
        Self::Traced(4),
        Self::Traced(5),
        Self::Traced(6),
        Self::Claw(0),
        Self::Claw(1),
        Self::Claw(2),
        Self::Claw(3),
        Self::Claw(4),
        Self::Claw(5),
        Self::Claw(6),
        Self::Claw(7),
    ];

    /// Its geometry in the document: a rectangle, an ellipse or a path.
    pub(crate) fn geometry(self) -> ShapeGeometry {
        match self {
            Self::Square | Self::RoundedSquare => ShapeGeometry::Rect,
            Self::Circle => ShapeGeometry::Ellipse,
            _ => built(self).map_or(ShapeGeometry::Rect, |(path, _)| ShapeGeometry::Path(path)),
        }
    }

    /// The caps of a line, start then end; `None` for a shape that is not
    /// a line (spec 058).
    pub(crate) fn line_caps(self) -> Option<(LineCap, LineCap)> {
        Some(match self {
            Self::LineCircles => (LineCap::Circle, LineCap::Circle),
            Self::LineDiamonds => (LineCap::Diamond, LineCap::Diamond),
            Self::LineDiamondsFilled => (LineCap::DiamondFilled, LineCap::DiamondFilled),
            Self::LinePlain => (LineCap::None, LineCap::None),
            Self::LineArrow => (LineCap::Square, LineCap::Arrow),
            Self::LineSquares => (LineCap::Square, LineCap::Square),
            Self::LineSquaresFilled => (LineCap::SquareFilled, LineCap::SquareFilled),
            Self::LineDashed | Self::LineDotted => (LineCap::None, LineCap::None),
            Self::LineCirclesFilled => (LineCap::CircleFilled, LineCap::CircleFilled),
            Self::LineBars => (LineCap::Bar, LineCap::Bar),
            Self::LineArrowBoth | Self::LineArrowBothDotted => (LineCap::Arrow, LineCap::Arrow),
            Self::LineArrowEnd | Self::LineArrowDotted => (LineCap::None, LineCap::Arrow),
            _ => return None,
        })
    }

    /// How the stroke of a line is drawn (spec 096); solid for a shape.
    pub(crate) fn line_dash(self) -> gafira_doc::LineDash {
        match self {
            Self::LineDashed => gafira_doc::LineDash::Dashed,
            Self::LineDotted | Self::LineArrowDotted | Self::LineArrowBothDotted => {
                gafira_doc::LineDash::Dotted
            }
            _ => gafira_doc::LineDash::Solid,
        }
    }

    /// Its width over its height: the box a new shape takes.
    pub(crate) fn aspect(self) -> f64 {
        built(self).map_or(1.0, |(_, aspect)| aspect)
    }

    /// Its corner radius for a side of `side` px (S53-01).
    pub(crate) fn corner_radius(self, side: f64) -> f64 {
        match self {
            Self::RoundedSquare => side / 8.0,
            _ => 0.0,
        }
    }

    /// The catalog key of its name.
    pub(crate) fn key(self) -> &'static str {
        match self {
            Self::LineCircles => "pieces.line_circles",
            Self::LineDiamonds => "pieces.line_diamonds",
            Self::LineDiamondsFilled => "pieces.line_diamonds_filled",
            Self::LinePlain => "pieces.line_plain",
            Self::LineArrow => "pieces.line_arrow",
            Self::LineSquares => "pieces.line_squares",
            Self::LineSquaresFilled => "pieces.line_squares_filled",
            Self::LineDashed => "pieces.line_dashed",
            Self::LineDotted => "pieces.line_dotted",
            Self::LineCirclesFilled => "pieces.line_circles_filled",
            Self::LineBars => "pieces.line_bars",
            Self::LineArrowBoth => "pieces.line_arrow_both",
            Self::LineArrowEnd => "pieces.line_arrow_end",
            Self::LineArrowDotted => "pieces.line_arrow_dotted",
            Self::LineArrowBothDotted => "pieces.line_arrow_both_dotted",
            Self::Square => "pieces.square",
            Self::RoundedSquare => "pieces.rounded_square",
            Self::Circle => "pieces.circle",
            Self::Triangle => "pieces.triangle",
            Self::TriangleDown => "pieces.triangle_down",
            Self::Diamond => "pieces.diamond",
            Self::Plus => "pieces.plus",
            Self::Barrel => "pieces.barrel",
            Self::Ticket => "pieces.ticket",
            Self::Parallelogram => "pieces.parallelogram",
            Self::ParallelogramLeft => "pieces.parallelogram_left",
            Self::Trapezoid => "pieces.trapezoid",
            Self::TrapezoidDown => "pieces.trapezoid_down",
            Self::ArchDown => "pieces.arch_down",
            Self::ArchUp => "pieces.arch_up",
            Self::RightTriangle => "pieces.right_triangle",
            Self::HalfCircle => "pieces.half_circle",
            Self::QuarterCircle => "pieces.quarter_circle",
            Self::QuarterRing => "pieces.quarter_ring",
            Self::HalfRing => "pieces.half_ring",
            Self::Pentagon => "pieces.pentagon",
            Self::Hexagon => "pieces.hexagon",
            Self::HexagonFlat => "pieces.hexagon_flat",
            Self::Heptagon => "pieces.heptagon",
            Self::Octagon => "pieces.octagon",
            Self::Nonagon => "pieces.nonagon",
            Self::Decagon => "pieces.decagon",
            Self::Dodecagon => "pieces.dodecagon",
            Self::Star4 => "pieces.star4",
            Self::Star5 => "pieces.star5",
            Self::Star6 => "pieces.star6",
            Self::Star7 => "pieces.star7",
            Self::Star8 => "pieces.star8",
            Self::Star12 => "pieces.star12",
            Self::Seal => "pieces.seal",
            Self::Burst => "pieces.burst",
            Self::ArrowRight => "pieces.arrow_right",
            Self::ArrowLeft => "pieces.arrow_left",
            Self::ArrowUp => "pieces.arrow_up",
            Self::ArrowDown => "pieces.arrow_down",
            Self::ArrowLeftRight => "pieces.arrow_left_right",
            Self::ArrowUpDown => "pieces.arrow_up_down",
            Self::Chevron => "pieces.chevron",
            Self::ArrowNotched => "pieces.arrow_notched",
            Self::Preparation => "pieces.preparation",
            Self::Terminator => "pieces.terminator",
            Self::Decision => "pieces.decision",
            Self::Data => "pieces.data",
            Self::Document => "pieces.document",
            Self::Delay => "pieces.delay",
            Self::ManualInput => "pieces.manual_input",
            Self::Card => "pieces.card",
            Self::Database => "pieces.database",
            Self::StoredData => "pieces.stored_data",
            Self::Display => "pieces.display",
            Self::MultiDocument => "pieces.multi_document",
            Self::PredefinedProcess => "pieces.predefined_process",
            Self::InternalStorage => "pieces.internal_storage",
            Self::LoopLimit => "pieces.loop_limit",
            Self::Sort => "pieces.sort",
            Self::Collate => "pieces.collate",
            Self::OrGate => "pieces.or_gate",
            Self::SummingJunction => "pieces.summing_junction",
            Self::PunchedTape => "pieces.punched_tape",
            Self::SequentialAccess => "pieces.sequential_access",
            Self::DirectData => "pieces.direct_data",
            Self::Cube => "pieces.cube",
            Self::Step => "pieces.step",
            Self::SpeechSquare => "pieces.speech_square",
            Self::SpeechRound => "pieces.speech_round",
            Self::SpeechCloud => "pieces.speech_cloud",
            Self::SpeechRounded => "pieces.speech_rounded",
            Self::SpeechOval => "pieces.speech_oval",
            Self::SpeechThought => "pieces.speech_thought",
            Self::SpeechCloudWide => "pieces.speech_cloud_wide",
            Self::SpeechCloudFlat => "pieces.speech_cloud_flat",
            Self::SpeechCloudRound => "pieces.speech_cloud_round",
            Self::SpeechCloudThink => "pieces.speech_cloud_think",
            Self::SpeechCloudThinkRight => "pieces.speech_cloud_think_right",
            Self::SpeechCloudSoft => "pieces.speech_cloud_soft",
            Self::SpeechCloudCurl => "pieces.speech_cloud_curl",
            Self::SpeechHand => "pieces.speech_hand",
            Self::SpeechHandThink => "pieces.speech_hand_think",
            Self::SpeechHandWide => "pieces.speech_hand_wide",
            Self::SpeechOvalCurved => "pieces.speech_oval_curved",
            Self::SpeechDouble => "pieces.speech_double",
            Self::Cloud => "pieces.cloud",
            Self::CloudFlat => "pieces.cloud_flat",
            Self::CloudPuffy => "pieces.cloud_puffy",
            Self::CloudSmall => "pieces.cloud_small",
            Self::CloudBig => "pieces.cloud_big",
            Self::Heart => "pieces.heart",
            Self::HeartWide => "pieces.heart_wide",
            Self::HeartNarrow => "pieces.heart_narrow",
            Self::HeartRound => "pieces.heart_round",
            Self::HeartSharp => "pieces.heart_sharp",
            Self::HeartSoft => "pieces.heart_soft",
            Self::Ribbon => "pieces.ribbon",
            Self::Pennant => "pieces.pennant",
            Self::Bookmark => "pieces.bookmark",
            Self::LabelClassic => "pieces.label_classic",
            Self::LabelScalloped => "pieces.label_scalloped",
            Self::LabelTall => "pieces.label_tall",
            Self::LabelBadge => "pieces.label_badge",
            Self::RibbonCurved => "pieces.ribbon_curved",
            Self::RibbonStraight => "pieces.ribbon_straight",
            Self::RibbonSmile => "pieces.ribbon_smile",
            Self::TagRect => "pieces.tag_rect",
            Self::TagRounded => "pieces.tag_rounded",
            Self::TagArch => "pieces.tag_arch",
            Self::TagCut => "pieces.tag_cut",
            Self::TagNotched => "pieces.tag_notched",
            Self::TagBarrel => "pieces.tag_barrel",
            Self::TagOval => "pieces.tag_oval",
            Self::TagShield => "pieces.tag_shield",
            Self::TagEgg => "pieces.tag_egg",
            Self::TagHexagon => "pieces.tag_hexagon",
            Self::TagStamp => "pieces.tag_stamp",
            Self::TagCircle => "pieces.tag_circle",
            Self::TagHeart => "pieces.tag_heart",
            Self::TagQuatrefoil => "pieces.tag_quatrefoil",
            Self::TagStar => "pieces.tag_star",
            Self::TagScalloped => "pieces.tag_scalloped",
            Self::Drop => "pieces.drop",
            Self::DropWide => "pieces.drop_wide",
            Self::DropTall => "pieces.drop_tall",
            Self::DropFat => "pieces.drop_fat",
            Self::DropDown => "pieces.drop_down",
            Self::DropSide => "pieces.drop_side",
            Self::DropFlame => "pieces.drop_flame",
            Self::DropHole => "pieces.drop_hole",
            Self::Gear8 => "pieces.gear8",
            Self::Gear12 => "pieces.gear12",
            Self::Gear10 => "pieces.gear10",
            Self::Gear14Sharp => "pieces.gear14_sharp",
            Self::Gear16 => "pieces.gear16",
            Self::GearSolid => "pieces.gear_solid",
            Self::Asterisk => "pieces.asterisk",
            Self::Cross => "pieces.cross",
            Self::Asterisk8 => "pieces.asterisk8",
            Self::AsteriskBold8 => "pieces.asterisk_bold8",
            Self::AsteriskBlock5 => "pieces.asterisk_block5",
            Self::Asterisk5 => "pieces.asterisk5",
            Self::Asterisk6Bar => "pieces.asterisk6_bar",
            Self::Asterisk12 => "pieces.asterisk12",
            Self::AsteriskRound => "pieces.asterisk_round",
            Self::AsteriskX => "pieces.asterisk_x",
            Self::AsteriskPinwheel => "pieces.asterisk_pinwheel",
            Self::Flower6 => "pieces.flower6",
            Self::Flower8 => "pieces.flower8",
            Self::FlowerThin => "pieces.flower_thin",
            Self::FlowerLong => "pieces.flower_long",
            Self::FlowerHole => "pieces.flower_hole",
            Self::Clover => "pieces.clover",
            Self::PetalCross => "pieces.petal_cross",
            Self::Leaf => "pieces.leaf",
            Self::WaveRipple => "pieces.wave_ripple",
            Self::WaveBand => "pieces.wave_band",
            Self::Blob => "pieces.blob",
            Self::Pebble => "pieces.pebble",
            Self::AbstractCushion => "pieces.abstract_cushion",
            Self::AbstractStack => "pieces.abstract_stack",
            Self::AbstractMoons => "pieces.abstract_moons",
            Self::AbstractBlocks => "pieces.abstract_blocks",
            Self::AbstractSlant => "pieces.abstract_slant",
            Self::AbstractPinwheel => "pieces.abstract_pinwheel",
            Self::AbstractSparkles => "pieces.abstract_sparkles",
            Self::AbstractWindow => "pieces.abstract_window",
            Self::AbstractSteps => "pieces.abstract_steps",
            Self::AbstractBean => "pieces.abstract_bean",
            Self::QuoteRound => "pieces.quote_round",
            Self::QuoteRoundOpen => "pieces.quote_round_open",
            Self::QuoteRoundSingle => "pieces.quote_round_single",
            Self::QuoteHeavy => "pieces.quote_heavy",
            Self::QuoteHeavyOpen => "pieces.quote_heavy_open",
            Self::QuoteBlock => "pieces.quote_block",
            Self::QuoteBlockOpen => "pieces.quote_block_open",
            Self::QuoteBlockSingle => "pieces.quote_block_single",
            Self::QuoteSoft => "pieces.quote_soft",
            Self::QuoteSoftOpen => "pieces.quote_soft_open",
            Self::QuoteSlant => "pieces.quote_slant",
            Self::QuoteWedge => "pieces.quote_wedge",
            Self::QuoteWedgeOpen => "pieces.quote_wedge_open",
            Self::QuoteThin => "pieces.quote_thin",
            Self::QuoteThinOpen => "pieces.quote_thin_open",
            Self::QuoteStraight => "pieces.quote_straight",
            Self::QuoteStraightSingle => "pieces.quote_straight_single",
            Self::QuoteAngle => "pieces.quote_angle",
            Self::QuoteAngleClose => "pieces.quote_angle_close",
            Self::QuoteAngleSingle => "pieces.quote_angle_single",
            Self::QuoteCircle => "pieces.quote_circle",
            Self::QuoteCircleOpen => "pieces.quote_circle_open",
            Self::QuoteSquare => "pieces.quote_square",
            Self::QuoteSquareOpen => "pieces.quote_square_open",
            Self::QuoteCut => "pieces.quote_cut",
            Self::QuoteCutClose => "pieces.quote_cut_close",
            Self::QuoteCutSingle => "pieces.quote_cut_single",
            Self::QuoteCutWide => "pieces.quote_cut_wide",
            Self::QuoteCutSlim => "pieces.quote_cut_slim",
            Self::QuoteCutDeep => "pieces.quote_cut_deep",
            Self::Brush(i) => BRUSH_KEYS[usize::from(i) % BRUSH_KEYS.len()],
            Self::Splatter(i) => SPLATTER_KEYS[usize::from(i) % SPLATTER_KEYS.len()],
            Self::Claw(i) => CLAW_KEYS[usize::from(i) % CLAW_KEYS.len()],
            Self::Traced(i) => TRACED_KEYS[usize::from(i) % TRACED_KEYS.len()],
        }
    }
}

/// The sections of the Formas panel, in the order of the reference (spec
/// 054), each with the catalog key of its title and its shapes.
pub(crate) const PIECE_SECTIONS: &[(&str, &[Piece])] = &[
    (
        "pieces.lines",
        &[
            Piece::LineCircles,
            Piece::LineDiamonds,
            Piece::LineDiamondsFilled,
            Piece::LinePlain,
            Piece::LineArrow,
            Piece::LineSquares,
            Piece::LineSquaresFilled,
            Piece::LineDashed,
            Piece::LineDotted,
            Piece::LineCirclesFilled,
            Piece::LineBars,
            Piece::LineArrowBoth,
            Piece::LineArrowEnd,
            Piece::LineArrowDotted,
            Piece::LineArrowBothDotted,
        ],
    ),
    (
        "pieces.basic",
        &[
            Piece::Square,
            Piece::RoundedSquare,
            Piece::Circle,
            Piece::Triangle,
            Piece::TriangleDown,
            Piece::Diamond,
            Piece::Plus,
            Piece::Barrel,
            Piece::Ticket,
            Piece::Parallelogram,
            Piece::ParallelogramLeft,
            Piece::Trapezoid,
            Piece::TrapezoidDown,
            Piece::ArchDown,
            Piece::ArchUp,
            Piece::RightTriangle,
            Piece::HalfCircle,
            Piece::QuarterCircle,
            Piece::QuarterRing,
            Piece::HalfRing,
        ],
    ),
    (
        "pieces.polygons",
        &[
            Piece::Pentagon,
            Piece::Hexagon,
            Piece::HexagonFlat,
            Piece::Heptagon,
            Piece::Octagon,
            Piece::Nonagon,
            Piece::Decagon,
            Piece::Dodecagon,
        ],
    ),
    (
        "pieces.stars",
        &[
            Piece::Star4,
            Piece::Star5,
            Piece::Star6,
            Piece::Star7,
            Piece::Star8,
            Piece::Star12,
            Piece::Seal,
            Piece::Burst,
        ],
    ),
    (
        "pieces.arrows",
        &[
            Piece::ArrowRight,
            Piece::ArrowLeft,
            Piece::ArrowUp,
            Piece::ArrowDown,
            Piece::ArrowLeftRight,
            Piece::ArrowUpDown,
            Piece::Chevron,
            Piece::ArrowNotched,
        ],
    ),
    (
        "pieces.flowchart",
        &[
            Piece::Preparation,
            Piece::Terminator,
            Piece::Decision,
            Piece::Data,
            Piece::Document,
            Piece::Delay,
            Piece::ManualInput,
            Piece::Card,
            Piece::Database,
            Piece::StoredData,
            Piece::Display,
            Piece::MultiDocument,
            Piece::PredefinedProcess,
            Piece::InternalStorage,
            Piece::LoopLimit,
            Piece::Sort,
            Piece::Collate,
            Piece::OrGate,
            Piece::SummingJunction,
            Piece::PunchedTape,
            Piece::SequentialAccess,
            Piece::DirectData,
            Piece::Cube,
            Piece::Step,
        ],
    ),
    (
        "pieces.speech",
        &[
            Piece::SpeechSquare,
            Piece::SpeechRound,
            Piece::SpeechCloud,
            Piece::SpeechRounded,
            Piece::SpeechOval,
            Piece::SpeechThought,
            Piece::SpeechCloudWide,
            Piece::SpeechCloudFlat,
            Piece::SpeechCloudRound,
            Piece::SpeechCloudThink,
            Piece::SpeechCloudThinkRight,
            Piece::SpeechCloudSoft,
            Piece::SpeechCloudCurl,
            Piece::SpeechHand,
            Piece::SpeechHandThink,
            Piece::SpeechHandWide,
            Piece::SpeechOvalCurved,
            Piece::SpeechDouble,
        ],
    ),
    (
        "pieces.clouds",
        &[
            Piece::Cloud,
            Piece::CloudFlat,
            Piece::CloudPuffy,
            Piece::CloudSmall,
            Piece::CloudBig,
        ],
    ),
    (
        "pieces.hearts",
        &[
            Piece::Heart,
            Piece::HeartWide,
            Piece::HeartNarrow,
            Piece::HeartRound,
            Piece::HeartSharp,
            Piece::HeartSoft,
        ],
    ),
    (
        "pieces.banners",
        &[
            Piece::Ribbon,
            Piece::Pennant,
            Piece::Bookmark,
            Piece::LabelClassic,
            Piece::LabelScalloped,
            Piece::LabelTall,
            Piece::LabelBadge,
            Piece::RibbonCurved,
            Piece::RibbonStraight,
            Piece::RibbonSmile,
        ],
    ),
    (
        "pieces.tags",
        &[
            Piece::TagRect,
            Piece::TagRounded,
            Piece::TagArch,
            Piece::TagCut,
            Piece::TagNotched,
            Piece::TagBarrel,
            Piece::TagOval,
            Piece::TagShield,
            Piece::TagEgg,
            Piece::TagHexagon,
            Piece::TagStamp,
            Piece::TagCircle,
            Piece::TagHeart,
            Piece::TagQuatrefoil,
            Piece::TagStar,
            Piece::TagScalloped,
        ],
    ),
    (
        "pieces.drops",
        &[
            Piece::Drop,
            Piece::DropWide,
            Piece::DropTall,
            Piece::DropFat,
            Piece::DropDown,
            Piece::DropSide,
            Piece::DropFlame,
            Piece::DropHole,
        ],
    ),
    (
        "pieces.gears",
        &[
            Piece::Gear8,
            Piece::Gear12,
            Piece::Gear10,
            Piece::Gear14Sharp,
            Piece::Gear16,
            Piece::GearSolid,
        ],
    ),
    (
        "pieces.asterisks",
        &[
            Piece::Asterisk,
            Piece::Cross,
            Piece::Asterisk8,
            Piece::AsteriskBold8,
            Piece::AsteriskBlock5,
            Piece::Asterisk5,
            Piece::Asterisk6Bar,
            Piece::Asterisk12,
            Piece::AsteriskRound,
            Piece::AsteriskX,
            Piece::AsteriskPinwheel,
        ],
    ),
    (
        "pieces.organic",
        &[
            Piece::Flower6,
            Piece::Flower8,
            Piece::FlowerThin,
            Piece::FlowerLong,
            Piece::FlowerHole,
            Piece::Clover,
            Piece::PetalCross,
            Piece::Leaf,
            Piece::WaveRipple,
            Piece::WaveBand,
            Piece::Blob,
            Piece::Pebble,
        ],
    ),
    (
        "pieces.abstract",
        &[
            Piece::AbstractCushion,
            Piece::AbstractStack,
            Piece::AbstractMoons,
            Piece::AbstractBlocks,
            Piece::AbstractSlant,
            Piece::AbstractPinwheel,
            Piece::AbstractSparkles,
            Piece::AbstractWindow,
            Piece::AbstractSteps,
            Piece::AbstractBean,
        ],
    ),
    (
        "pieces.quotes",
        &[
            Piece::QuoteRound,
            Piece::QuoteRoundOpen,
            Piece::QuoteRoundSingle,
            Piece::QuoteHeavy,
            Piece::QuoteHeavyOpen,
            Piece::QuoteBlock,
            Piece::QuoteBlockOpen,
            Piece::QuoteBlockSingle,
            Piece::QuoteSoft,
            Piece::QuoteSoftOpen,
            Piece::QuoteSlant,
            Piece::QuoteWedge,
            Piece::QuoteWedgeOpen,
            Piece::QuoteThin,
            Piece::QuoteThinOpen,
            Piece::QuoteStraight,
            Piece::QuoteStraightSingle,
            Piece::QuoteAngle,
            Piece::QuoteAngleClose,
            Piece::QuoteAngleSingle,
            Piece::QuoteCircle,
            Piece::QuoteCircleOpen,
            Piece::QuoteSquare,
            Piece::QuoteSquareOpen,
            Piece::QuoteCut,
            Piece::QuoteCutClose,
            Piece::QuoteCutSingle,
            Piece::QuoteCutWide,
            Piece::QuoteCutSlim,
            Piece::QuoteCutDeep,
        ],
    ),
    (
        "pieces.brushes",
        &[
            Piece::Traced(0),
            Piece::Traced(1),
            Piece::Brush(0),
            Piece::Brush(1),
            Piece::Brush(2),
            Piece::Brush(3),
            Piece::Brush(4),
            Piece::Brush(5),
            Piece::Brush(6),
            Piece::Brush(7),
            Piece::Brush(8),
            Piece::Brush(9),
            Piece::Brush(10),
            Piece::Brush(11),
            Piece::Brush(12),
            Piece::Brush(13),
            Piece::Brush(14),
            Piece::Brush(15),
            Piece::Brush(16),
            Piece::Brush(17),
            Piece::Brush(18),
            Piece::Brush(19),
            Piece::Brush(20),
            Piece::Brush(21),
            Piece::Brush(22),
            Piece::Brush(23),
        ],
    ),
    (
        "pieces.splatters",
        &[
            Piece::Traced(2),
            Piece::Traced(3),
            Piece::Traced(7),
            Piece::Traced(8),
            Piece::Traced(9),
            Piece::Splatter(0),
            Piece::Splatter(1),
            Piece::Splatter(2),
            Piece::Splatter(3),
            Piece::Splatter(4),
            Piece::Splatter(5),
            Piece::Splatter(6),
            Piece::Splatter(7),
        ],
    ),
    (
        "pieces.claws",
        &[
            Piece::Traced(4),
            Piece::Traced(5),
            Piece::Traced(6),
            Piece::Claw(0),
            Piece::Claw(1),
            Piece::Claw(2),
            Piece::Claw(3),
            Piece::Claw(4),
            Piece::Claw(5),
            Piece::Claw(6),
            Piece::Claw(7),
        ],
    ),
];

/// The names of the dry brush strokes, splatters and claw marks.
const BRUSH_KEYS: [&str; crate::brushes::STROKES] = [
    "pieces.brush_1",
    "pieces.brush_2",
    "pieces.brush_3",
    "pieces.brush_4",
    "pieces.brush_5",
    "pieces.brush_6",
    "pieces.brush_7",
    "pieces.brush_8",
    "pieces.brush_9",
    "pieces.brush_10",
    "pieces.brush_11",
    "pieces.brush_12",
    "pieces.brush_13",
    "pieces.brush_14",
    "pieces.brush_15",
    "pieces.brush_16",
    "pieces.brush_17",
    "pieces.brush_18",
    "pieces.brush_19",
    "pieces.brush_20",
    "pieces.brush_21",
    "pieces.brush_22",
    "pieces.brush_23",
    "pieces.brush_24",
];
const SPLATTER_KEYS: [&str; crate::brushes::SPLATTERS] = [
    "pieces.splatter_1",
    "pieces.splatter_2",
    "pieces.splatter_3",
    "pieces.splatter_4",
    "pieces.splatter_5",
    "pieces.splatter_6",
    "pieces.splatter_7",
    "pieces.splatter_8",
];
const TRACED_KEYS: [&str; crate::brushes::TRACED] = [
    "pieces.traced_brush_curved",
    "pieces.traced_brush_wide",
    "pieces.traced_splatter_heavy",
    "pieces.traced_splatter",
    "pieces.traced_claws",
    "pieces.traced_claws_with_claws",
    "pieces.traced_claws_thin",
    "pieces.traced_splatter_burst",
    "pieces.traced_splatter_round",
    "pieces.traced_splatter_drips",
];
const CLAW_KEYS: [&str; crate::brushes::CLAWS] = [
    "pieces.claw_1",
    "pieces.claw_2",
    "pieces.claw_3",
    "pieces.claw_4",
    "pieces.claw_5",
    "pieces.claw_6",
    "pieces.claw_7",
    "pieces.claw_8",
];

/// How many shapes a section shows in its row (spec 096).
pub(crate) const ROW: usize = 5;

/// Whether a section of `len` shapes has more than its row shows: it gets
/// «Ver todo» and arrows.
pub(crate) fn has_more(len: usize) -> bool {
    len > ROW
}

/// The first shape a row of a section of `len` shows after moving `pages`
/// rows from `first` (negative is back), never before the first shape nor
/// past the last row.
pub(crate) fn paged(len: usize, first: usize, pages: isize) -> usize {
    let last = len.saturating_sub(ROW);
    let moved = first.min(last) as isize + pages * ROW as isize;
    moved.clamp(0, last as isize) as usize
}

/// The path of `piece` in the unit box and its aspect; `None` for the
/// shapes the document draws without a path (rectangle and ellipse).
pub(crate) fn built(piece: Piece) -> Option<(ShapePath, f64)> {
    let path = match piece {
        Piece::Square | Piece::RoundedSquare | Piece::Circle => return None,
        Piece::LineCircles
        | Piece::LineDiamonds
        | Piece::LineDiamondsFilled
        | Piece::LinePlain
        | Piece::LineArrow
        | Piece::LineSquares
        | Piece::LineSquaresFilled
        | Piece::LineDashed
        | Piece::LineDotted
        | Piece::LineCirclesFilled
        | Piece::LineBars
        | Piece::LineArrowBoth
        | Piece::LineArrowEnd
        | Piece::LineArrowDotted
        | Piece::LineArrowBothDotted => return None,
        Piece::Triangle => poly(&[(0.5, 0.0), (1.0, 1.0), (0.0, 1.0)]),
        Piece::RightTriangle => poly(&[(0.0, 0.0), (1.0, 1.0), (0.0, 1.0)]),
        Piece::Pentagon => poly(&regular(5, -90.0)),
        Piece::Hexagon => poly(&regular(6, -90.0)),
        Piece::HexagonFlat => poly(&regular(6, 0.0)),
        Piece::Octagon => poly(&regular(8, -67.5)),
        Piece::Decagon => poly(&regular(10, -90.0)),
        Piece::Star4 => poly(&star(4, 0.38)),
        Piece::Star5 => poly(&star(5, 0.45)),
        Piece::Star6 => poly(&star(6, 0.58)),
        Piece::Star8 => poly(&star(8, 0.7)),
        Piece::Seal => poly(&star(16, 0.85)),
        Piece::ArrowRight => poly(&ARROW),
        Piece::ArrowLeft => poly(&mapped(&ARROW, |(x, y)| (1.0 - x, y))),
        Piece::ArrowUp => poly(&mapped(&ARROW, |(x, y)| (y, 1.0 - x))),
        Piece::ArrowDown => poly(&mapped(&ARROW, |(x, y)| (y, x))),
        Piece::ArrowLeftRight => poly(&[
            (0.0, 0.5),
            (0.45, 0.0),
            (0.45, 0.3),
            (1.15, 0.3),
            (1.15, 0.0),
            (1.6, 0.5),
            (1.15, 1.0),
            (1.15, 0.7),
            (0.45, 0.7),
            (0.45, 1.0),
        ]),
        Piece::Preparation => poly(&[
            (0.0, 0.5),
            (0.3, 0.0),
            (1.7, 0.0),
            (2.0, 0.5),
            (1.7, 1.0),
            (0.3, 1.0),
        ]),
        Piece::Terminator => RoundedRect::new(0.0, 0.0, 2.0, 1.0, 0.5).to_path(TOLERANCE),
        Piece::Decision => poly(&[(0.75, 0.0), (1.5, 0.5), (0.75, 1.0), (0.0, 0.5)]),
        Piece::Data => poly(&[(0.35, 0.0), (1.7, 0.0), (1.35, 1.0), (0.0, 1.0)]),
        Piece::Document => {
            let mut path = BezPath::new();
            path.move_to((0.0, 0.0));
            path.line_to((1.6, 0.0));
            path.line_to((1.6, 0.85));
            path.curve_to((1.2, 0.6), (0.45, 1.2), (0.0, 0.9));
            path.close_path();
            path
        }
        Piece::SpeechSquare => poly(&[
            (0.0, 0.0),
            (1.2, 0.0),
            (1.2, 0.8),
            (0.5, 0.8),
            (0.2, 1.05),
            (0.28, 0.8),
            (0.0, 0.8),
        ]),
        Piece::SpeechRound => union(vec![
            Ellipse::new((0.6, 0.42), (0.6, 0.42), 0.0).to_path(TOLERANCE),
            poly(&[(0.25, 0.6), (0.1, 1.0), (0.55, 0.78)]),
        ]),
        Piece::SpeechRounded => union(vec![
            RoundedRect::new(0.0, 0.0, 1.2, 0.8, 0.18).to_path(TOLERANCE),
            poly(&[(0.25, 0.7), (0.15, 1.05), (0.55, 0.75)]),
        ]),
        Piece::Cloud => union(vec![
            circle(0.38, 0.55, 0.26),
            circle(0.68, 0.4, 0.33),
            circle(1.0, 0.52, 0.26),
            circle(1.2, 0.66, 0.18),
            RoundedRect::new(0.15, 0.55, 1.3, 0.84, 0.14).to_path(TOLERANCE),
        ]),
        Piece::CloudFlat => union(vec![
            circle(0.42, 0.58, 0.25),
            circle(0.78, 0.42, 0.32),
            circle(1.1, 0.58, 0.23),
            poly(&[(0.17, 0.58), (1.33, 0.58), (1.33, 0.83), (0.17, 0.83)]),
        ]),
        Piece::Heart => heart(1.0),
        Piece::HeartWide => heart(1.3),
        Piece::Ribbon => poly(&[
            (0.0, 0.0),
            (1.6, 0.0),
            (1.35, 0.5),
            (1.6, 1.0),
            (0.0, 1.0),
            (0.25, 0.5),
        ]),
        Piece::Pennant => poly(&[(0.0, 0.0), (1.0, 0.0), (1.0, 0.75), (0.5, 1.0), (0.0, 0.75)]),
        Piece::Bookmark => poly(&[(0.0, 0.0), (0.7, 0.0), (0.7, 1.0), (0.35, 0.72), (0.0, 1.0)]),
        Piece::Drop => drop(1.0),
        Piece::DropWide => drop(1.3),
        Piece::Gear8 => gear(8),
        Piece::Gear12 => gear(12),
        Piece::Asterisk => union(
            [0.0, 60.0, 120.0]
                .into_iter()
                .map(|deg| {
                    poly(&turned(
                        &[(-1.0, -0.13), (1.0, -0.13), (1.0, 0.13), (-1.0, 0.13)],
                        deg,
                    ))
                })
                .collect(),
        ),
        Piece::Cross => poly(&[
            (0.35, 0.0),
            (0.65, 0.0),
            (0.65, 0.35),
            (1.0, 0.35),
            (1.0, 0.65),
            (0.65, 0.65),
            (0.65, 1.0),
            (0.35, 1.0),
            (0.35, 0.65),
            (0.0, 0.65),
            (0.0, 0.35),
            (0.35, 0.35),
        ]),
        Piece::Flower6 => flower(6),
        Piece::Flower8 => flower(8),
        Piece::TriangleDown => poly(&[(0.0, 0.0), (1.0, 0.0), (0.5, 1.0)]),
        Piece::Diamond => poly(&[(0.5, 0.0), (1.0, 0.5), (0.5, 1.0), (0.0, 0.5)]),
        Piece::Plus => poly(&[
            (0.32, 0.0),
            (0.68, 0.0),
            (0.68, 0.32),
            (1.0, 0.32),
            (1.0, 0.68),
            (0.68, 0.68),
            (0.68, 1.0),
            (0.32, 1.0),
            (0.32, 0.68),
            (0.0, 0.68),
            (0.0, 0.32),
            (0.32, 0.32),
        ]),
        Piece::Barrel => poly(&[
            (0.0, 0.08),
            (0.5, 0.0),
            (1.0, 0.08),
            (1.0, 0.92),
            (0.5, 1.0),
            (0.0, 0.92),
        ]),
        Piece::Ticket => ticket(0.12),
        Piece::Parallelogram => poly(&[(0.28, 0.0), (1.28, 0.0), (1.0, 0.72), (0.0, 0.72)]),
        Piece::ParallelogramLeft => poly(&[(0.0, 0.0), (1.0, 0.0), (1.28, 0.72), (0.28, 0.72)]),
        Piece::Trapezoid => poly(&[(0.3, 0.0), (1.0, 0.0), (1.3, 0.75), (0.0, 0.75)]),
        Piece::TrapezoidDown => poly(&[(0.0, 0.0), (1.3, 0.0), (1.0, 0.75), (0.3, 0.75)]),
        Piece::ArchDown => {
            let mut path = BezPath::new();
            path.move_to((0.0, 0.0));
            path.line_to((1.0, 0.0));
            path.line_to((1.0, 0.5));
            arc(&mut path, (0.5, 0.5), 0.5, 0.0, 180.0);
            path.close_path();
            path
        }
        Piece::ArchUp => {
            let mut path = BezPath::new();
            path.move_to((0.0, 1.0));
            path.line_to((0.0, 0.5));
            arc(&mut path, (0.5, 0.5), 0.5, 180.0, 180.0);
            path.line_to((1.0, 1.0));
            path.close_path();
            path
        }
        Piece::HalfCircle => {
            let mut path = BezPath::new();
            path.move_to((0.0, 0.5));
            arc(&mut path, (0.5, 0.5), 0.5, 180.0, 180.0);
            path.close_path();
            path
        }
        Piece::QuarterCircle => {
            let mut path = BezPath::new();
            path.move_to((1.0, 1.0));
            path.line_to((0.0, 1.0));
            arc(&mut path, (1.0, 1.0), 1.0, 180.0, 90.0);
            path.close_path();
            path
        }
        Piece::QuarterRing => {
            let mut path = BezPath::new();
            path.move_to((0.0, 1.0));
            arc(&mut path, (1.0, 1.0), 1.0, 180.0, 90.0);
            path.line_to((1.0, 0.5));
            arc(&mut path, (1.0, 1.0), 0.5, 270.0, -90.0);
            path.close_path();
            path
        }
        Piece::HalfRing => {
            let mut path = BezPath::new();
            path.move_to((0.0, 0.5));
            arc(&mut path, (0.5, 0.5), 0.5, 180.0, 180.0);
            path.line_to((0.75, 0.5));
            arc(&mut path, (0.5, 0.5), 0.25, 0.0, -180.0);
            path.close_path();
            path
        }
        Piece::Heptagon => poly(&regular(7, -90.0)),
        Piece::Nonagon => poly(&regular(9, -90.0)),
        Piece::Dodecagon => poly(&regular(12, -75.0)),
        Piece::Star7 => poly(&star(7, 0.5)),
        Piece::Star12 => poly(&star(12, 0.72)),
        Piece::Burst => poly(&star(24, 0.8)),
        Piece::ArrowUpDown => poly(&[
            (0.5, 0.0),
            (1.0, 0.45),
            (0.7, 0.45),
            (0.7, 1.15),
            (1.0, 1.15),
            (0.5, 1.6),
            (0.0, 1.15),
            (0.3, 1.15),
            (0.3, 0.45),
            (0.0, 0.45),
        ]),
        Piece::Chevron => poly(&[
            (0.0, 0.0),
            (0.6, 0.0),
            (1.0, 0.5),
            (0.6, 1.0),
            (0.0, 1.0),
            (0.4, 0.5),
        ]),
        Piece::ArrowNotched => poly(&[
            (0.0, 0.3),
            (0.55, 0.3),
            (0.55, 0.0),
            (1.0, 0.5),
            (0.55, 1.0),
            (0.55, 0.7),
            (0.0, 0.7),
            (0.15, 0.5),
        ]),
        Piece::Delay => {
            let mut path = BezPath::new();
            path.move_to((0.0, 0.0));
            path.line_to((1.0, 0.0));
            arc(&mut path, (1.0, 0.5), 0.5, 270.0, 180.0);
            path.line_to((0.0, 1.0));
            path.close_path();
            path
        }
        Piece::ManualInput => poly(&[(0.0, 0.3), (1.6, 0.0), (1.6, 1.0), (0.0, 1.0)]),
        Piece::Card => poly(&[(0.3, 0.0), (1.5, 0.0), (1.5, 1.0), (0.0, 1.0), (0.0, 0.3)]),
        Piece::SpeechCloud => {
            let mut parts: Vec<BezPath> = CLOUD_PUFFS
                .iter()
                .map(|&(x, y, r)| circle(x, y, r))
                .collect();
            let ((cx, cy), radii) = CLOUD_CORE;
            parts.push(Ellipse::new((cx, cy), radii, 0.0).to_path(TOLERANCE));
            let [from, tip, to] = CLOUD_TAIL;
            parts.push(curved_tail(from, tip, to, -0.1));
            union(parts)
        }
        Piece::SpeechOval => union(vec![
            Ellipse::new((0.65, 0.42), (0.65, 0.42), 0.0).to_path(TOLERANCE),
            poly(&[(0.85, 0.65), (1.12, 1.0), (0.6, 0.8)]),
        ]),
        Piece::SpeechThought => union(vec![
            Ellipse::new((0.68, 0.38), (0.6, 0.38), 0.0).to_path(TOLERANCE),
            circle(0.3, 0.86, 0.09),
            circle(0.13, 1.0, 0.05),
        ]),
        Piece::SpeechCloudWide => {
            let core = ((1.0, 0.6), (0.95, 0.48));
            let mut parts = cloud(core, 9, (0.42, 0.28));
            parts.push(cloud_tail(core, (0.3, 1.4), -0.15));
            union(parts)
        }
        Piece::SpeechCloudFlat => {
            let core = ((1.0, 0.5), (1.0, 0.36));
            let mut parts = cloud(core, 9, (0.34, 0.22));
            parts.push(cloud_tail(core, (0.25, 1.2), -0.12));
            union(parts)
        }
        Piece::SpeechCloudRound => {
            let core = ((0.85, 0.75), (0.78, 0.62));
            let mut parts = cloud(core, 9, (0.42, 0.3));
            parts.push(cloud_tail(core, (0.4, 1.65), -0.1));
            union(parts)
        }
        Piece::SpeechCloudThink => {
            let mut parts = cloud(((0.95, 0.7), (0.85, 0.55)), 10, (0.4, 0.26));
            parts.extend([
                circle(0.45, 1.38, 0.09),
                circle(0.28, 1.52, 0.06),
                circle(0.16, 1.62, 0.04),
            ]);
            union(parts)
        }
        Piece::SpeechCloudThinkRight => {
            let mut parts = cloud(((0.95, 0.7), (0.85, 0.55)), 10, (0.4, 0.26));
            parts.extend([
                circle(1.35, 1.38, 0.09),
                circle(1.52, 1.52, 0.06),
                circle(1.64, 1.62, 0.04),
            ]);
            union(parts)
        }
        Piece::SpeechCloudSoft => {
            let core = ((0.9, 0.7), (0.8, 0.55));
            let mut parts = cloud(core, 7, (0.46, 0.32));
            parts.push(cloud_tail(core, (0.6, 1.6), 0.1));
            union(parts)
        }
        Piece::SpeechCloudCurl => {
            let core = ((0.9, 0.72), (0.85, 0.6));
            let mut parts = cloud(core, 10, (0.4, 0.28));
            parts.push(cloud_tail(core, (0.0, 1.62), -0.3));
            union(parts)
        }
        Piece::SpeechHand => union(vec![
            soft_poly(&[(0.04, 0.1), (1.55, 0.0), (1.6, 0.86), (0.0, 0.95)], 0.12),
            curved_tail((0.25, 0.9), (0.12, 1.3), (0.48, 0.92), -0.08),
        ]),
        Piece::SpeechHandThink => union(vec![
            soft_poly(&[(0.0, 0.06), (1.5, 0.0), (1.58, 0.9), (0.06, 0.96)], 0.12),
            circle(1.3, 1.06, 0.07),
            circle(1.42, 1.17, 0.05),
            circle(1.52, 1.25, 0.035),
        ]),
        Piece::SpeechHandWide => union(vec![
            soft_poly(&[(0.0, 0.12), (2.2, 0.0), (2.16, 0.7), (0.06, 0.78)], 0.1),
            curved_tail((1.75, 0.72), (2.05, 1.05), (1.95, 0.7), 0.1),
        ]),
        Piece::SpeechOvalCurved => union(vec![
            Ellipse::new((0.85, 0.45), (0.85, 0.42), 0.0).to_path(TOLERANCE),
            curved_tail((0.35, 0.72), (0.12, 1.1), (0.55, 0.82), -0.15),
        ]),
        Piece::SpeechDouble => union(vec![
            Ellipse::new((0.72, 0.36), (0.72, 0.34), 0.0).to_path(TOLERANCE),
            Ellipse::new((0.95, 0.78), (0.6, 0.3), 0.0).to_path(TOLERANCE),
            curved_tail((0.6, 0.98), (0.4, 1.32), (0.82, 1.02), -0.1),
        ]),
        Piece::CloudPuffy => union(cloud_ring(0.75, 0.42, (0.62, 0.3), 11, 0.2)),
        Piece::CloudSmall => union(vec![
            circle(0.4, 0.62, 0.22),
            circle(0.72, 0.5, 0.3),
            circle(1.02, 0.65, 0.2),
            RoundedRect::new(0.18, 0.6, 1.22, 0.85, 0.12).to_path(TOLERANCE),
        ]),
        Piece::CloudBig => union(vec![
            circle(0.38, 0.68, 0.2),
            circle(0.72, 0.5, 0.38),
            circle(1.05, 0.7, 0.18),
            RoundedRect::new(0.18, 0.62, 1.23, 0.88, 0.13).to_path(TOLERANCE),
        ]),
        Piece::HeartNarrow => heart(0.82),
        Piece::HeartRound => heart_round(),
        Piece::HeartSharp => heart_sharp(),
        Piece::HeartSoft => heart_soft(),
        Piece::Database => {
            let mut body = BezPath::new();
            body.move_to((0.0, 0.15));
            oval(&mut body, (0.5, 0.15), (0.5, 0.15), 180.0, 180.0);
            body.line_to((1.0, 0.85));
            oval(&mut body, (0.5, 0.85), (0.5, 0.15), 0.0, 180.0);
            body.close_path();
            let mut rim = BezPath::new();
            rim.move_to((0.0, 0.15));
            oval(&mut rim, (0.5, 0.15), (0.5, 0.15), 180.0, -180.0);
            rim.line_to((1.0, 0.22));
            oval(&mut rim, (0.5, 0.22), (0.5, 0.15), 0.0, 180.0);
            rim.close_path();
            with_holes(body, vec![rim])
        }
        Piece::StoredData => {
            let mut path = BezPath::new();
            path.move_to((0.25, 0.0));
            path.line_to((1.4, 0.0));
            path.quad_to((1.15, 0.5), (1.4, 1.0));
            path.line_to((0.25, 1.0));
            path.quad_to((-0.08, 0.5), (0.25, 0.0));
            path.close_path();
            path
        }
        Piece::Display => {
            let mut path = BezPath::new();
            path.move_to((0.3, 0.0));
            path.line_to((1.25, 0.0));
            oval(&mut path, (1.25, 0.5), (0.25, 0.5), 270.0, 180.0);
            path.line_to((0.3, 1.0));
            path.line_to((0.0, 0.5));
            path.close_path();
            path
        }
        Piece::MultiDocument => {
            let mut front = BezPath::new();
            front.move_to((0.0, 0.2));
            front.line_to((1.6, 0.2));
            front.line_to((1.6, 1.0));
            front.curve_to((1.2, 0.78), (0.45, 1.3), (0.0, 1.02));
            front.close_path();
            union(vec![
                front,
                poly(&[(0.1, 0.1), (1.7, 0.1), (1.7, 0.16), (0.1, 0.16)]),
                poly(&[(1.64, 0.1), (1.7, 0.1), (1.7, 0.95), (1.64, 0.95)]),
                poly(&[(0.2, 0.0), (1.8, 0.0), (1.8, 0.06), (0.2, 0.06)]),
                poly(&[(1.74, 0.0), (1.8, 0.0), (1.8, 0.85), (1.74, 0.85)]),
            ])
        }
        Piece::PredefinedProcess => union(vec![
            poly(&[(0.0, 0.0), (0.16, 0.0), (0.16, 1.0), (0.0, 1.0)]),
            poly(&[(0.22, 0.0), (1.38, 0.0), (1.38, 1.0), (0.22, 1.0)]),
            poly(&[(1.44, 0.0), (1.6, 0.0), (1.6, 1.0), (1.44, 1.0)]),
        ]),
        Piece::InternalStorage => union(vec![
            poly(&[(0.0, 0.0), (0.18, 0.0), (0.18, 0.18), (0.0, 0.18)]),
            poly(&[(0.24, 0.0), (1.3, 0.0), (1.3, 0.18), (0.24, 0.18)]),
            poly(&[(0.0, 0.24), (0.18, 0.24), (0.18, 1.0), (0.0, 1.0)]),
            poly(&[(0.24, 0.24), (1.3, 0.24), (1.3, 1.0), (0.24, 1.0)]),
        ]),
        Piece::LoopLimit => poly(&[
            (0.2, 0.0),
            (1.4, 0.0),
            (1.6, 0.2),
            (1.6, 1.0),
            (0.0, 1.0),
            (0.0, 0.2),
        ]),
        Piece::Sort => union(vec![
            poly(&[(0.75, 0.0), (1.5, 0.47), (0.0, 0.47)]),
            poly(&[(0.0, 0.53), (1.5, 0.53), (0.75, 1.0)]),
        ]),
        Piece::Collate => union(vec![
            poly(&[(0.0, 0.0), (1.0, 0.0), (0.5, 0.5)]),
            poly(&[(0.5, 0.5), (1.0, 1.0), (0.0, 1.0)]),
        ]),
        Piece::OrGate => with_holes(circle(0.0, 0.0, 1.0), vec![poly(&cross(0.84, 0.06))]),
        Piece::SummingJunction => with_holes(
            circle(0.0, 0.0, 1.0),
            vec![poly(&turned(&cross(0.84, 0.06), 45.0))],
        ),
        Piece::PunchedTape => {
            let mut path = BezPath::new();
            path.move_to((0.0, 0.12));
            path.curve_to((0.5, -0.12), (1.1, 0.36), (1.6, 0.12));
            path.line_to((1.6, 0.88));
            path.curve_to((1.1, 1.12), (0.5, 0.64), (0.0, 0.88));
            path.close_path();
            path
        }
        Piece::SequentialAccess => union(vec![
            circle(0.5, 0.5, 0.5),
            poly(&[(0.5, 0.9), (1.05, 0.9), (1.05, 1.0), (0.5, 1.0)]),
        ]),
        Piece::DirectData => {
            let mut body = BezPath::new();
            body.move_to((0.15, 0.0));
            body.line_to((1.45, 0.0));
            oval(&mut body, (1.45, 0.5), (0.15, 0.5), 270.0, 180.0);
            body.line_to((0.15, 1.0));
            oval(&mut body, (0.15, 0.5), (0.15, 0.5), 90.0, 180.0);
            body.close_path();
            let mut rim = BezPath::new();
            rim.move_to((1.45, 0.0));
            oval(&mut rim, (1.45, 0.5), (0.15, 0.5), 270.0, -180.0);
            rim.line_to((1.38, 1.0));
            oval(&mut rim, (1.38, 0.5), (0.15, 0.5), 90.0, 180.0);
            rim.close_path();
            with_holes(body, vec![rim])
        }
        Piece::Cube => union(vec![
            poly(&[(0.0, 0.25), (0.95, 0.25), (0.95, 1.2), (0.0, 1.2)]),
            poly(&[(0.0, 0.2), (0.25, 0.0), (1.2, 0.0), (0.95, 0.2)]),
            poly(&[(1.0, 0.25), (1.2, 0.05), (1.2, 1.0), (1.0, 1.2)]),
        ]),
        Piece::Step => poly(&[(0.0, 0.0), (1.2, 0.0), (1.6, 0.5), (1.2, 1.0), (0.0, 1.0)]),
        Piece::Asterisk8 => spokes(8, -90.0, 1.0, 0.07),
        Piece::AsteriskBold8 => spokes(8, -90.0, 1.0, 0.2),
        Piece::AsteriskBlock5 => spokes(5, -90.0, 1.0, 0.3),
        Piece::Asterisk5 => spokes(5, 90.0, 1.0, 0.12),
        Piece::Asterisk6Bar => spokes(6, 0.0, 1.0, 0.08),
        Piece::Asterisk12 => spokes(12, -90.0, 1.0, 0.05),
        Piece::AsteriskRound => {
            let mut parts = vec![spokes(6, -90.0, 0.82, 0.13)];
            parts.extend(
                regular(6, -90.0)
                    .into_iter()
                    .map(|(x, y)| circle(0.82 * x, 0.82 * y, 0.18)),
            );
            union(parts)
        }
        Piece::AsteriskX => spokes(4, 45.0, 1.0, 0.15),
        Piece::AsteriskPinwheel => union(
            (0..6)
                .map(|i| {
                    poly(&turned(
                        &[(0.0, 0.0), (0.95, -0.08), (0.62, 0.42)],
                        -90.0 + 60.0 * f64::from(i),
                    ))
                })
                .collect(),
        ),
        Piece::FlowerThin => union(
            (0..8)
                .map(|i| petal(f64::from(i) * 45.0, 0.5, (0.5, 0.08)))
                .chain([circle(0.0, 0.0, 0.1)])
                .collect(),
        ),
        Piece::FlowerLong => union(
            (0..6)
                .map(|i| petal(f64::from(i) * 60.0 - 90.0, 0.5, (0.5, 0.2)))
                .collect(),
        ),
        Piece::FlowerHole => with_holes(
            union(
                regular(5, -90.0)
                    .into_iter()
                    .map(|(x, y)| circle(0.55 * x, 0.55 * y, 0.32))
                    .chain([circle(0.0, 0.0, 0.5)])
                    .collect(),
            ),
            vec![circle(0.0, 0.0, 0.15)],
        ),
        Piece::Clover => union(
            (0..4)
                .map(|i| {
                    let mut lobe = heart(1.0);
                    lobe.apply_affine(
                        Affine::rotate(f64::from(i) * std::f64::consts::FRAC_PI_2)
                            * Affine::scale(0.55)
                            * Affine::translate((-0.5, -1.0)),
                    );
                    lobe
                })
                .chain([circle(0.0, 0.0, 0.15)])
                .collect(),
        ),
        Piece::PetalCross => union(
            (0..4)
                .map(|i| {
                    let mut leaf = BezPath::new();
                    leaf.move_to((0.0, 0.0));
                    leaf.quad_to((0.5, -0.42), (1.0, 0.0));
                    leaf.quad_to((0.5, 0.42), (0.0, 0.0));
                    leaf.close_path();
                    leaf.apply_affine(Affine::rotate((45.0 + 90.0 * f64::from(i)).to_radians()));
                    leaf
                })
                .collect(),
        ),
        Piece::Leaf => {
            let mut path = BezPath::new();
            path.move_to((1.0, 1.0));
            path.curve_to((0.35, 1.0), (0.0, 0.65), (0.0, 0.0));
            path.curve_to((0.65, 0.0), (1.0, 0.35), (1.0, 1.0));
            path.close_path();
            path
        }
        Piece::WaveRipple => band(
            |x| 0.12 * (x * std::f64::consts::TAU * 2.5).sin(),
            2.0,
            0.32,
        ),
        Piece::WaveBand => band(|x| 0.2 * (x * std::f64::consts::TAU).sin(), 1.6, 0.3),
        Piece::Blob => polar(
            |t| 1.0 + 0.14 * (3.0 * t).sin() + 0.08 * (5.0 * t + 1.0).cos(),
            1.0,
        ),
        Piece::Pebble => polar(
            |t| 1.0 + 0.1 * (2.0 * t).cos() + 0.05 * (3.0 * t).sin(),
            1.35,
        ),
        Piece::AbstractCushion => union(
            [(-0.42, -0.42), (0.42, -0.42), (0.42, 0.42), (-0.42, 0.42)]
                .into_iter()
                .map(|(x, y)| circle(x, y, 0.33))
                .chain([poly(&[
                    (-0.42, -0.42),
                    (0.42, -0.42),
                    (0.42, 0.42),
                    (-0.42, 0.42),
                ])])
                .collect(),
        ),
        Piece::AbstractStack => union(vec![
            RoundedRect::from_rect(
                kurbo::Rect::new(0.08, 0.0, 1.12, 0.47),
                kurbo::RoundedRectRadii::new(0.0, 0.0, 0.23, 0.23),
            )
            .to_path(TOLERANCE),
            RoundedRect::new(0.0, 0.53, 1.2, 1.0, 0.235).to_path(TOLERANCE),
        ]),
        Piece::AbstractMoons => union(
            [0.5, 1.05, 1.6]
                .into_iter()
                .map(|x| {
                    let mut moon = BezPath::new();
                    moon.move_to((x, 0.0));
                    arc(&mut moon, (x, 0.5), 0.5, 270.0, -180.0);
                    moon.close_path();
                    moon
                })
                .collect(),
        ),
        Piece::AbstractBlocks => union(vec![
            RoundedRect::from_rect(
                kurbo::Rect::new(0.0, 0.0, 1.0, 0.46),
                kurbo::RoundedRectRadii::new(0.0, 0.0, 0.46, 0.0),
            )
            .to_path(TOLERANCE),
            RoundedRect::from_rect(
                kurbo::Rect::new(0.0, 0.54, 1.0, 1.0),
                kurbo::RoundedRectRadii::new(0.46, 0.0, 0.0, 0.0),
            )
            .to_path(TOLERANCE),
        ]),
        Piece::AbstractSlant => poly(&[
            (0.45, 0.0),
            (1.0, 0.0),
            (1.0, 0.55),
            (0.55, 1.0),
            (0.0, 1.0),
            (0.0, 0.45),
        ]),
        Piece::AbstractPinwheel => union(
            (0..4)
                .map(|i| {
                    let mut blade = BezPath::new();
                    blade.move_to((0.0, 0.0));
                    blade.line_to((1.0, 0.0));
                    arc(&mut blade, (0.5, 0.0), 0.5, 0.0, -180.0);
                    blade.close_path();
                    blade.apply_affine(Affine::rotate(f64::from(i) * std::f64::consts::FRAC_PI_2));
                    blade
                })
                .collect(),
        ),
        Piece::AbstractSparkles => union(vec![sparkle(-0.5), sparkle(0.5)]),
        // One outline (four round lobes), so the window is cut through it.
        Piece::AbstractWindow => with_holes(
            polar(
                |t| {
                    (0..4)
                        .map(|k| (t - f64::from(k) * std::f64::consts::FRAC_PI_2).cos())
                        .fold(f64::MIN, f64::max)
                },
                1.0,
            ),
            vec![poly(&[
                (-0.22, -0.22),
                (0.22, -0.22),
                (0.22, 0.22),
                (-0.22, 0.22),
            ])],
        ),
        Piece::AbstractSteps => poly(&[
            (0.0, 1.0),
            (0.0, 0.66),
            (0.33, 0.66),
            (0.33, 0.33),
            (0.66, 0.33),
            (0.66, 0.0),
            (1.0, 0.0),
            (1.0, 1.0),
        ]),
        Piece::AbstractBean => polar(|t| 1.0 + 0.28 * (2.0 * t).cos() - 0.18 * t.sin(), 1.3),
        Piece::QuoteRound => pair(|x| comma(x, 0.0)),
        Piece::QuoteRoundOpen => opening(pair(|x| comma(x, 0.0))),
        Piece::QuoteRoundSingle => comma(0.0, 0.0),
        Piece::QuoteHeavy => pair(|x| comma(x, 0.08)),
        Piece::QuoteHeavyOpen => opening(pair(|x| comma(x, 0.08))),
        Piece::QuoteBlock => pair(block_mark),
        Piece::QuoteBlockOpen => opening(pair(block_mark)),
        Piece::QuoteBlockSingle => block_mark(0.0),
        Piece::QuoteSoft => pair(soft_mark),
        Piece::QuoteSoftOpen => opening(pair(soft_mark)),
        Piece::QuoteSlant => {
            pair(|x| poly(&[(x + 0.2, 0.0), (x + 0.5, 0.0), (x + 0.3, 1.0), (x, 1.0)]))
        }
        Piece::QuoteWedge => pair(|x| poly(&[(x, 0.0), (x + 0.42, 0.0), (x + 0.1, 1.0)])),
        Piece::QuoteWedgeOpen => {
            opening(pair(|x| poly(&[(x, 0.0), (x + 0.42, 0.0), (x + 0.1, 1.0)])))
        }
        Piece::QuoteThin => pair(crescent),
        Piece::QuoteThinOpen => opening(pair(crescent)),
        Piece::QuoteStraight => {
            pair(|x| RoundedRect::new(x, 0.0, x + 0.2, 1.0, 0.1).to_path(TOLERANCE))
        }
        Piece::QuoteStraightSingle => RoundedRect::new(0.0, 0.0, 0.3, 1.0, 0.15).to_path(TOLERANCE),
        Piece::QuoteAngle => union(vec![chevron(0.0), chevron(0.42)]),
        Piece::QuoteAngleClose => opening(union(vec![chevron(0.0), chevron(0.42)])),
        Piece::QuoteAngleSingle => chevron(0.0),
        Piece::QuoteCircle => badge(false, true),
        Piece::QuoteCircleOpen => badge(true, true),
        Piece::QuoteSquare => badge(false, false),
        Piece::QuoteSquareOpen => badge(true, false),
        Piece::QuoteCut => pair(|x| cut_mark(x, 0.42, 0.45, true)),
        Piece::QuoteCutClose => opening(pair(|x| cut_mark(x, 0.42, 0.45, true))),
        Piece::QuoteCutSingle => cut_mark(0.0, 0.42, 0.45, true),
        Piece::QuoteCutWide => pair(|x| cut_mark(x, 0.54, 0.42, true)),
        Piece::QuoteCutSlim => pair(|x| cut_mark(x, 0.32, 0.5, false)),
        Piece::QuoteCutDeep => pair(|x| cut_mark(x, 0.42, 0.62, true)),
        Piece::Brush(i) => crate::brushes::stroke_path(usize::from(i)),
        Piece::Splatter(i) => crate::brushes::splatter_path(usize::from(i)),
        Piece::Claw(i) => crate::brushes::claw_path(usize::from(i)),
        Piece::Traced(i) => crate::brushes::traced_path(usize::from(i)),
        Piece::LabelClassic => label(1.6, 1.0),
        Piece::LabelTall => label(1.0, 1.4),
        Piece::LabelScalloped => {
            let (w, h, r) = (1.6, 1.0, 0.13);
            let mut parts = vec![RoundedRect::new(r, r, w - r, h - r, 0.05).to_path(TOLERANCE)];
            let nx = 7;
            let ny = 4;
            for i in 0..=nx {
                let x = r + (w - 2.0 * r) * f64::from(i) / f64::from(nx);
                parts.push(circle(x, r, r));
                parts.push(circle(x, h - r, r));
            }
            for j in 1..ny {
                let y = r + (h - 2.0 * r) * f64::from(j) / f64::from(ny);
                parts.push(circle(r, y, r));
                parts.push(circle(w - r, y, r));
            }
            union(parts)
        }
        Piece::LabelBadge => union(vec![
            RoundedRect::new(0.3, 0.0, 1.3, 1.0, 0.2).to_path(TOLERANCE),
            circle(0.3, 0.5, 0.3),
            circle(1.3, 0.5, 0.3),
            circle(0.42, 0.22, 0.2),
            circle(0.42, 0.78, 0.2),
            circle(1.18, 0.22, 0.2),
            circle(1.18, 0.78, 0.2),
        ]),
        Piece::RibbonCurved => ribbon(0.32),
        Piece::RibbonStraight => ribbon(0.0),
        Piece::RibbonSmile => ribbon(-0.32),
        Piece::TagRect => tag(poly(&[(0.0, 0.0), (0.6, 0.0), (0.6, 1.0), (0.0, 1.0)]), 0.6),
        Piece::TagRounded => tag(
            RoundedRect::new(0.0, 0.0, 0.6, 1.0, 0.1).to_path(TOLERANCE),
            0.6,
        ),
        Piece::TagArch => tag(
            RoundedRect::from_rect(
                kurbo::Rect::new(0.0, 0.0, 0.6, 1.0),
                kurbo::RoundedRectRadii::new(0.3, 0.3, 0.05, 0.05),
            )
            .to_path(TOLERANCE),
            0.6,
        ),
        Piece::TagCut => tag(
            poly(&[
                (0.15, 0.0),
                (0.45, 0.0),
                (0.6, 0.15),
                (0.6, 1.0),
                (0.0, 1.0),
                (0.0, 0.15),
            ]),
            0.6,
        ),
        Piece::TagNotched => {
            let mut path = BezPath::new();
            path.move_to((0.1, 0.0));
            path.line_to((0.5, 0.0));
            arc(&mut path, (0.6, 0.0), 0.1, 180.0, -90.0);
            path.line_to((0.6, 0.9));
            arc(&mut path, (0.6, 1.0), 0.1, 270.0, -90.0);
            path.line_to((0.1, 1.0));
            arc(&mut path, (0.0, 1.0), 0.1, 0.0, -90.0);
            path.line_to((0.0, 0.1));
            arc(&mut path, (0.0, 0.0), 0.1, 90.0, -90.0);
            path.close_path();
            tag(path, 0.6)
        }
        Piece::TagBarrel => {
            let mut path = BezPath::new();
            path.move_to((0.05, 0.0));
            path.line_to((0.55, 0.0));
            path.quad_to((0.7, 0.5), (0.55, 1.0));
            path.line_to((0.05, 1.0));
            path.quad_to((-0.1, 0.5), (0.05, 0.0));
            path.close_path();
            tag(path, 0.6)
        }
        Piece::TagOval => tag(
            RoundedRect::new(0.0, 0.0, 0.62, 1.0, 0.24).to_path(TOLERANCE),
            0.62,
        ),
        Piece::TagShield => tag(
            {
                let mut path = BezPath::new();
                path.move_to((0.0, 0.0));
                path.line_to((0.6, 0.0));
                path.line_to((0.6, 0.65));
                path.quad_to((0.6, 0.85), (0.3, 1.0));
                path.quad_to((0.0, 0.85), (0.0, 0.65));
                path.close_path();
                path
            },
            0.6,
        ),
        Piece::TagEgg => tag(
            {
                let mut path = BezPath::new();
                path.move_to((0.32, 0.0));
                path.curve_to((0.55, 0.0), (0.68, 0.45), (0.65, 0.68));
                path.curve_to((0.62, 0.9), (0.48, 1.0), (0.32, 1.0));
                path.curve_to((0.16, 1.0), (0.02, 0.9), (0.0, 0.68));
                path.curve_to((-0.03, 0.45), (0.1, 0.0), (0.32, 0.0));
                path.close_path();
                path
            },
            0.64,
        ),
        Piece::TagHexagon => tag(
            poly(&[
                (0.3, 0.0),
                (0.6, 0.15),
                (0.6, 0.85),
                (0.3, 1.0),
                (0.0, 0.85),
                (0.0, 0.15),
            ]),
            0.6,
        ),
        Piece::TagStamp => {
            let (w, h, n, m) = (0.6, 0.9, 8, 12);
            let mut points = Vec::new();
            for i in 0..n {
                let x = w * f64::from(i) / f64::from(n);
                points.push((x, 0.0));
                points.push((x + w / f64::from(2 * n), 0.03));
            }
            for j in 0..m {
                let y = h * f64::from(j) / f64::from(m);
                points.push((w, y));
                points.push((w - 0.03, y + h / f64::from(2 * m)));
            }
            for i in (1..=n).rev() {
                let x = w * f64::from(i) / f64::from(n);
                points.push((x, h));
                points.push((x - w / f64::from(2 * n), h - 0.03));
            }
            for j in (1..=m).rev() {
                let y = h * f64::from(j) / f64::from(m);
                points.push((0.0, y));
                points.push((0.03, y - h / f64::from(2 * m)));
            }
            tag(poly(&points), w)
        }
        Piece::TagCircle => tag(circle(0.5, 0.5, 0.5), 1.0),
        Piece::TagHeart => with_holes(heart(1.15), vec![circle(0.575, 0.42, 0.04)]),
        Piece::TagQuatrefoil => {
            let mut body = union(
                [(0.5, 0.28), (0.72, 0.5), (0.5, 0.72), (0.28, 0.5)]
                    .into_iter()
                    .map(|(x, y)| circle(x, y, 0.28))
                    .chain([poly(&[
                        (0.28, 0.28),
                        (0.72, 0.28),
                        (0.72, 0.72),
                        (0.28, 0.72),
                    ])])
                    .collect(),
            );
            body.apply_affine(Affine::translate((0.0, 0.0)));
            // The hole sits in the top lobe, where only it covers.
            with_holes(body, vec![circle(0.5, 0.12, 0.04)])
        }
        Piece::TagStar => {
            let mut star = poly(&star(5, 0.48));
            star.apply_affine(Affine::translate((1.0, 1.0)) * Affine::scale(1.0));
            with_holes(star, vec![circle(1.0, 0.3, 0.06)])
        }
        Piece::TagScalloped => with_holes(
            polar(|t| 1.0 + 0.07 * (14.0 * t).cos().abs().sqrt(), 1.0),
            vec![circle(0.0, -0.8, 0.07)],
        ),
        Piece::DropTall => drop(0.62),
        Piece::DropFat => {
            let mut path = BezPath::new();
            path.move_to((0.5, 0.0));
            path.curve_to((0.55, 0.2), (1.0, 0.38), (1.0, 0.66));
            path.curve_to((1.0, 0.86), (0.78, 1.0), (0.5, 1.0));
            path.curve_to((0.22, 1.0), (0.0, 0.86), (0.0, 0.66));
            path.curve_to((0.0, 0.38), (0.45, 0.2), (0.5, 0.0));
            path.close_path();
            path
        }
        Piece::DropDown => {
            let mut path = drop(1.0);
            path.apply_affine(Affine::new([1.0, 0.0, 0.0, -1.0, 0.0, 1.0]));
            path
        }
        Piece::DropSide => {
            let mut path = drop(1.0);
            path.apply_affine(Affine::rotate(std::f64::consts::FRAC_PI_2));
            path
        }
        Piece::DropFlame => {
            let mut path = BezPath::new();
            path.move_to((0.7, 0.0));
            path.curve_to((0.45, 0.25), (1.0, 0.45), (1.0, 0.7));
            path.curve_to((1.0, 0.88), (0.78, 1.0), (0.5, 1.0));
            path.curve_to((0.22, 1.0), (0.0, 0.88), (0.0, 0.68));
            path.curve_to((0.0, 0.4), (0.55, 0.3), (0.7, 0.0));
            path.close_path();
            path
        }
        Piece::DropHole => with_holes(drop(1.0), vec![circle(0.5, 0.7, 0.16)]),
        Piece::Gear10 => gear_with(10, 0.8, (0.32, 0.22), 0.55),
        Piece::Gear14Sharp => gear_with(14, 0.7, (0.4, 0.08), 0.3),
        Piece::Gear16 => gear_with(16, 0.84, (0.3, 0.2), 0.45),
        Piece::GearSolid => gear_with(12, 0.82, (0.3, 0.2), 0.16),
    };
    Some(finished(path))
}

/// How closely a curve shape follows its exact outline, in the shape's
/// own units (about 1 for a whole shape).
const TOLERANCE: f64 = 1e-3;

/// The right arrow; the other arrows are it, mirrored or turned.
const ARROW: [(f64, f64); 7] = [
    (0.0, 0.3),
    (0.55, 0.3),
    (0.55, 0.0),
    (1.0, 0.5),
    (0.55, 1.0),
    (0.55, 0.7),
    (0.0, 0.7),
];

/// A closed polygon through `points`.
pub(crate) fn poly(points: &[(f64, f64)]) -> BezPath {
    let mut path = BezPath::new();
    for (i, p) in points.iter().enumerate() {
        if i == 0 {
            path.move_to(*p);
        } else {
            path.line_to(*p);
        }
    }
    path.close_path();
    path
}

/// `points`, each moved by `f`.
fn mapped(points: &[(f64, f64)], f: impl Fn((f64, f64)) -> (f64, f64)) -> Vec<(f64, f64)> {
    points.iter().copied().map(f).collect()
}

/// `points` turned `degrees` about the origin.
fn turned(points: &[(f64, f64)], degrees: f64) -> Vec<(f64, f64)> {
    let (sin, cos) = degrees.to_radians().sin_cos();
    mapped(points, |(x, y)| (x * cos - y * sin, x * sin + y * cos))
}

/// The corners of a regular polygon of `n` sides on the unit circle, the
/// first at `start` degrees (−90 is straight up).
fn regular(n: usize, start: f64) -> Vec<(f64, f64)> {
    (0..n)
        .map(|i| {
            let a = (start + 360.0 * i as f64 / n as f64).to_radians();
            (a.cos(), a.sin())
        })
        .collect()
}

/// A star of `n` points, its inner corners at `inner` of the outer radius,
/// a point straight up.
pub(crate) fn star(n: usize, inner: f64) -> Vec<(f64, f64)> {
    (0..2 * n)
        .map(|i| {
            let r = if i % 2 == 0 { 1.0 } else { inner };
            let a = (-90.0 + 180.0 * i as f64 / n as f64).to_radians();
            (r * a.cos(), r * a.sin())
        })
        .collect()
}

/// A circle of radius `r` about `(x, y)`.
fn circle(x: f64, y: f64, r: f64) -> BezPath {
    Circle::new((x, y), r).to_path(TOLERANCE)
}

/// A heart `wide` times as wide as it is high.
fn heart(wide: f64) -> BezPath {
    let at = |x: f64, y: f64| (x * wide, y);
    let mut path = BezPath::new();
    path.move_to(at(0.5, 0.28));
    path.curve_to(at(0.5, 0.0), at(0.0, -0.02), at(0.0, 0.3));
    path.curve_to(at(0.0, 0.62), at(0.45, 0.78), at(0.5, 1.0));
    path.curve_to(at(0.55, 0.78), at(1.0, 0.62), at(1.0, 0.3));
    path.curve_to(at(1.0, -0.02), at(0.5, 0.0), at(0.5, 0.28));
    path.close_path();
    path
}

/// A teardrop with its point up, `wide` times as wide.
fn drop(wide: f64) -> BezPath {
    let at = |x: f64, y: f64| (x * wide, y);
    let mut path = BezPath::new();
    path.move_to(at(0.5, 0.0));
    path.curve_to(at(0.6, 0.15), at(1.0, 0.5), at(1.0, 0.72));
    path.curve_to(at(1.0, 0.88), at(0.78, 1.0), at(0.5, 1.0));
    path.curve_to(at(0.22, 1.0), at(0.0, 0.88), at(0.0, 0.72));
    path.curve_to(at(0.0, 0.5), at(0.4, 0.15), at(0.5, 0.0));
    path.close_path();
    path
}

/// A gear of `n` teeth with a round hole in the middle.
fn gear(n: usize) -> BezPath {
    gear_with(n, 0.78, (0.3, 0.2), 0.36)
}

/// A gear of `n` teeth whose valleys are at `root` of the outer radius,
/// each tooth `(base, top)` of a step wide on each side of its middle,
/// with a round hole of radius `hole`.
fn gear_with(n: usize, root: f64, (base, top): (f64, f64), hole: f64) -> BezPath {
    let step = 360.0 / n as f64;
    let mut points = Vec::new();
    for i in 0..n {
        let a = -90.0 + step * i as f64;
        for (r, d) in [(root, -base), (1.0, -top), (1.0, top), (root, base)] {
            let t = (a + d * step).to_radians();
            points.push((r * t.cos(), r * t.sin()));
        }
    }
    let ring = oriented(poly(&points), true);
    let hole = oriented(circle(0.0, 0.0, hole), false);
    let mut path = ring;
    path.extend(hole);
    path
}

/// Appends to `path` an arc of the circle of radius `r` about `centre`,
/// from `start` degrees turning `sweep` degrees (positive is clockwise on
/// the page, with y down; 0 points right).
fn arc(path: &mut BezPath, centre: (f64, f64), r: f64, start: f64, sweep: f64) {
    let arc = kurbo::Arc::new(centre, (r, r), start.to_radians(), sweep.to_radians(), 0.0);
    arc.append_iter(TOLERANCE).for_each(|el| path.push(el));
}

/// `n` straight arms `length` long and `half` thick on each side, from the
/// middle outwards, the first at `start` degrees, joined by a round heart.
fn spokes(n: usize, start: f64, length: f64, half: f64) -> BezPath {
    let mut parts: Vec<BezPath> = (0..n)
        .map(|i| {
            poly(&turned(
                &[(0.0, -half), (length, -half), (length, half), (0.0, half)],
                start + 360.0 * i as f64 / n as f64,
            ))
        })
        .collect();
    parts.push(circle(0.0, 0.0, half * 1.4));
    union(parts)
}

/// A petal: an ellipse of `radii` whose middle is `reach` from the
/// origin, at `degrees`.
fn petal(degrees: f64, reach: f64, radii: (f64, f64)) -> BezPath {
    let t = degrees.to_radians();
    Ellipse::new((reach * t.cos(), reach * t.sin()), radii, t).to_path(TOLERANCE)
}

/// A closed curve about the origin whose radius at angle `t` is `r(t)`,
/// `wide` times as wide as high.
fn polar(r: impl Fn(f64) -> f64, wide: f64) -> BezPath {
    let n = 144;
    let points: Vec<(f64, f64)> = (0..n)
        .map(|i| {
            let t = std::f64::consts::TAU * f64::from(i) / f64::from(n);
            let r = r(t);
            (wide * r * t.cos(), r * t.sin())
        })
        .collect();
    poly(&points)
}

/// A band `long` wide and `thick` high whose middle line is `y(x)` for
/// `x` in `0..=1`.
fn band(y: impl Fn(f64) -> f64, long: f64, thick: f64) -> BezPath {
    let n = 96;
    let line: Vec<(f64, f64)> = (0..=n)
        .map(|i| {
            let x = f64::from(i) / f64::from(n);
            (x * long, y(x))
        })
        .collect();
    let mut points: Vec<(f64, f64)> = line.iter().map(|(x, y)| (*x, y - thick / 2.0)).collect();
    points.extend(line.iter().rev().map(|(x, y)| (*x, y + thick / 2.0)));
    poly(&points)
}

/// A four-pointed sparkle with hollow sides, centred at `x`.
fn sparkle(x: f64) -> BezPath {
    let mut path = BezPath::new();
    path.move_to((x, -1.0));
    path.quad_to((x + 0.08, -0.08), (x + 0.55, 0.0));
    path.quad_to((x + 0.08, 0.08), (x, 1.0));
    path.quad_to((x - 0.08, 0.08), (x - 0.55, 0.0));
    path.quad_to((x - 0.08, -0.08), (x, -1.0));
    path.close_path();
    path
}

/// A vintage label `w` × `h`: a peak in the middle of its top and bottom,
/// its corners cut in round and its sides gently out.
fn label(w: f64, h: f64) -> BezPath {
    let r = 0.12 * w.min(h);
    let rise = 0.08 * h;
    let mut path = BezPath::new();
    path.move_to((r, rise));
    path.quad_to((w * 0.3, rise), (w / 2.0, 0.0));
    path.quad_to((w * 0.7, rise), (w - r, rise));
    arc(&mut path, (w, rise), r, 180.0, -90.0);
    path.quad_to((w + 0.05 * w, h / 2.0), (w, h - rise - r));
    arc(&mut path, (w, h - rise), r, 270.0, -90.0);
    path.quad_to((w * 0.7, h - rise), (w / 2.0, h));
    path.quad_to((w * 0.3, h - rise), (r, h - rise));
    arc(&mut path, (0.0, h - rise), r, 0.0, -90.0);
    path.quad_to((-0.05 * w, h / 2.0), (0.0, rise + r));
    arc(&mut path, (0.0, rise), r, 90.0, -90.0);
    path.close_path();
    path
}

/// A ribbon banner: a band bent up by `bend` (down when negative) with a
/// notched tail behind each end.
fn ribbon(bend: f64) -> BezPath {
    let (w, top, thick) = (2.3, 0.3, 0.6);
    let lift = bend.max(0.0);
    let mut band = BezPath::new();
    band.move_to((0.32, top + lift));
    band.curve_to(
        (0.9, top + lift - bend * 1.1),
        (w - 0.9, top + lift - bend * 1.1),
        (w - 0.32, top + lift),
    );
    band.line_to((w - 0.32, top + lift + thick));
    band.curve_to(
        (w - 0.9, top + lift + thick - bend * 1.1),
        (0.9, top + lift + thick - bend * 1.1),
        (0.32, top + lift + thick),
    );
    band.close_path();
    let y = top + lift + 0.28;
    let tail = |x0: f64, x1: f64, notch: f64| {
        poly(&[
            (x0, y),
            (x1, y),
            (x1, y + thick),
            (x0, y + thick),
            (notch, y + thick / 2.0),
        ])
    };
    union(vec![
        band,
        tail(0.0, 0.29, 0.14),
        tail(w, w - 0.29, w - 0.14),
    ])
}

/// `body` (`w` wide, from y 0) with the small round hole a hang tag hangs
/// from, near the middle of its top.
fn tag(body: BezPath, w: f64) -> BezPath {
    let top = body.bounding_box().y0;
    with_holes(body, vec![circle(w / 2.0, top + 0.08, 0.035)])
}

/// Appends to `path` an arc of the ellipse of radii `radii` about
/// `centre`, as [`arc`] does for a circle.
fn oval(path: &mut BezPath, centre: (f64, f64), radii: (f64, f64), start: f64, sweep: f64) {
    let arc = kurbo::Arc::new(centre, radii, start.to_radians(), sweep.to_radians(), 0.0);
    arc.append_iter(TOLERANCE).for_each(|el| path.push(el));
}

/// A closing quote mark («9») at `x`, one outline: a round head and a tail
/// down to the left; `heavy` thickens both.
fn comma(x: f64, heavy: f64) -> BezPath {
    let (r, w) = (0.25 + heavy / 2.0, 0.06 + heavy);
    let mut p = BezPath::new();
    p.move_to((x + r, 0.0));
    p.curve_to(
        (x + r * 1.55, 0.0),
        (x + 2.0 * r, r * 0.45),
        (x + 2.0 * r, r * 1.1),
    );
    p.curve_to((x + 2.0 * r, 0.6), (x + 1.3 * r, 0.86), (x + w, 1.0));
    p.line_to((x + w * 0.4, 0.93));
    p.curve_to(
        (x + 0.7 * r, 0.82),
        (x + 1.05 * r, 0.68),
        (x + 1.05 * r, 2.05 * r),
    );
    p.curve_to((x + 0.45 * r, 2.05 * r), (x, 1.6 * r), (x, r));
    p.curve_to((x, 0.45 * r), (x + 0.45 * r, 0.0), (x + r, 0.0));
    p.close_path();
    p
}

/// The cloud of the cloud bubble: big puffs on top, smaller ones along a
/// flatter bottom (x, y, radius), over a core that fills the gaps.
const CLOUD_PUFFS: [(f64, f64, f64); 9] = [
    (0.40, 0.62, 0.24),
    (0.68, 0.42, 0.30),
    (1.02, 0.34, 0.34),
    (1.34, 0.50, 0.26),
    (1.50, 0.74, 0.18),
    (0.30, 0.86, 0.17),
    (0.58, 0.92, 0.20),
    (0.92, 0.95, 0.21),
    (1.24, 0.93, 0.20),
];
const CLOUD_CORE: ((f64, f64), (f64, f64)) = ((0.9, 0.72), (0.55, 0.25));
/// Its tail: the two corners of its base inside the puffs, then the point,
/// so only the point leaves the cloud.
const CLOUD_TAIL: [(f64, f64); 3] = [(0.52, 0.85), (0.4, 1.32), (0.86, 0.88)];

/// A cloud about an ellipse of `centre` and `radii`: `n` puffs round it,
/// `big` on top shrinking to `small` along a flatter bottom, over a core that
/// fills the gaps; the puffs vary a little in size, as a drawn cloud does.
fn cloud(
    (centre, radii): ((f64, f64), (f64, f64)),
    n: usize,
    (big, small): (f64, f64),
) -> Vec<BezPath> {
    let (cx, cy) = centre;
    let (rx, ry) = radii;
    let mut parts: Vec<BezPath> = (0..n)
        .map(|k| {
            let a = std::f64::consts::TAU * k as f64 / n as f64 - std::f64::consts::FRAC_PI_2;
            let (sin, cos) = a.sin_cos();
            // Up is −1: the top gets the big puffs; the bottom is flatter.
            let up = (1.0 - sin) / 2.0;
            let wobble = [1.0, 0.86, 1.08, 0.92, 1.0, 0.9, 1.05][k % 7];
            let r = (small + (big - small) * up) * wobble;
            // Each puff sits half inside the ellipse: soft, round bumps.
            let (px, py) = (rx - 0.55 * r, ry - 0.55 * r);
            let y = if sin > 0.0 {
                cy + py * 0.8 * sin
            } else {
                cy + py * sin
            };
            circle(cx + px * cos, y, r)
        })
        .collect();
    parts.push(Ellipse::new(centre, (rx * 0.9, ry * 0.85), 0.0).to_path(TOLERANCE));
    parts
}

/// A cloud's tail to `tip`: its base corners are well inside the core
/// ellipse of the cloud, on each side of the way to the tip, so only the
/// point leaves the cloud and no corner sits in a notch between puffs.
fn cloud_tail(core: ((f64, f64), (f64, f64)), tip: (f64, f64), bend: f64) -> BezPath {
    let [from, to] = tail_base(core, tip);
    curved_tail(from, tip, to, bend)
}

/// The two base corners of [`cloud_tail`].
fn tail_base(((cx, cy), (rx, ry)): ((f64, f64), (f64, f64)), tip: (f64, f64)) -> [(f64, f64); 2] {
    let towards = ((tip.1 - cy) / ry).atan2((tip.0 - cx) / rx);
    let at = |a: f64| (cx + 0.65 * rx * a.cos(), cy + 0.65 * ry * a.sin());
    [at(towards - 0.22), at(towards + 0.22)]
}

/// A tail from `from` to its point `tip` and back to `to`, swept to one
/// side by `bend` (a share of its length, signed): both sides bow the same
/// way, one hollow and one full, as a drawn tail does; bowing them apart
/// would swell it round in the middle.
fn curved_tail(from: (f64, f64), tip: (f64, f64), to: (f64, f64), bend: f64) -> BezPath {
    let bow = |a: (f64, f64), b: (f64, f64), k: f64| {
        let (dx, dy) = (b.0 - a.0, b.1 - a.1);
        ((a.0 + b.0) / 2.0 - dy * k, (a.1 + b.1) / 2.0 + dx * k)
    };
    let mut p = BezPath::new();
    p.move_to(from);
    p.quad_to(bow(from, tip, bend), tip);
    // Back the other way: the opposite sign bows it to the same side.
    p.quad_to(bow(tip, to, -bend), to);
    p.close_path();
    p
}

/// A closed outline through `points` with its corners rounded by `r`.
fn soft_poly(points: &[(f64, f64)], r: f64) -> BezPath {
    let n = points.len();
    let toward = |a: (f64, f64), b: (f64, f64)| {
        let (dx, dy) = (b.0 - a.0, b.1 - a.1);
        let len = dx.hypot(dy).max(f64::EPSILON);
        let k = (r / len).min(0.5);
        (a.0 + dx * k, a.1 + dy * k)
    };
    let mut p = BezPath::new();
    for i in 0..n {
        let (prev, here, next) = (points[(i + n - 1) % n], points[i], points[(i + 1) % n]);
        let (a, b) = (toward(here, prev), toward(here, next));
        if i == 0 {
            p.move_to(a);
        } else {
            p.line_to(a);
        }
        p.quad_to(here, b);
    }
    p.close_path();
    p
}

/// A square opening mark at `x`, `w` wide: a block from `foot` down, and
/// above it a hook up to the right (the quotes of heavy grotesques). A
/// `straight` hook is a band of even width, its two edges parallel; else it
/// narrows to the block.
fn cut_mark(x: f64, w: f64, foot: f64, straight: bool) -> BezPath {
    let hook = w * 0.42;
    let inner = if straight { hook } else { w - hook * 0.55 };
    poly(&[
        (x, foot),
        (x + w - hook, 0.0),
        (x + w, 0.0),
        (x + inner, foot),
        (x + w, foot),
        (x + w, 1.0),
        (x, 1.0),
    ])
}

/// A square closing mark at `x`: a block with a slanted tail.
fn block_mark(x: f64) -> BezPath {
    poly(&[
        (x, 0.0),
        (x + 0.46, 0.0),
        (x + 0.46, 0.58),
        (x + 0.16, 1.0),
        (x + 0.16, 0.52),
        (x, 0.52),
    ])
}

/// [`block_mark`] with soft corners.
fn soft_mark(x: f64) -> BezPath {
    let mut p = BezPath::new();
    p.move_to((x + 0.08, 0.0));
    p.line_to((x + 0.38, 0.0));
    p.quad_to((x + 0.46, 0.0), (x + 0.46, 0.08));
    p.line_to((x + 0.46, 0.56));
    p.quad_to((x + 0.46, 0.64), (x + 0.41, 0.7));
    p.line_to((x + 0.24, 0.96));
    p.quad_to((x + 0.16, 1.04), (x + 0.16, 0.92));
    p.line_to((x + 0.16, 0.52));
    p.line_to((x + 0.08, 0.52));
    p.quad_to((x, 0.52), (x, 0.44));
    p.line_to((x, 0.08));
    p.quad_to((x, 0.0), (x + 0.08, 0.0));
    p.close_path();
    p
}

/// A thin closing mark at `x`: a crescent bending right.
fn crescent(x: f64) -> BezPath {
    let mut p = BezPath::new();
    p.move_to((x + 0.12, 0.0));
    p.curve_to((x + 0.52, 0.18), (x + 0.52, 0.78), (x + 0.04, 1.0));
    p.curve_to((x + 0.3, 0.72), (x + 0.3, 0.3), (x + 0.12, 0.0));
    p.close_path();
    p
}

/// An opening angle mark «‹» at `x`.
fn chevron(x: f64) -> BezPath {
    poly(&[
        (x + 0.36, 0.1),
        (x + 0.5, 0.22),
        (x + 0.24, 0.5),
        (x + 0.5, 0.78),
        (x + 0.36, 0.9),
        (x, 0.5),
    ])
}

/// Two marks side by side.
fn pair(mark: impl Fn(f64) -> BezPath) -> BezPath {
    union(vec![mark(0.0), mark(0.62)])
}

/// `path` turned half round about its middle: a closing mark opens.
fn opening(mut path: BezPath) -> BezPath {
    let middle = path.bounding_box().center();
    path.apply_affine(Affine::rotate_about(std::f64::consts::PI, middle));
    path
}

/// A circle or a rounded square with round quotes cut through it.
fn badge(open: bool, round: bool) -> BezPath {
    let body = if round {
        circle(1.0, 1.0, 1.0)
    } else {
        RoundedRect::new(0.0, 0.0, 2.0, 2.0, 0.35).to_path(TOLERANCE)
    };
    let mut marks = pair(|x| comma(x, 0.04));
    if open {
        marks = opening(marks);
    }
    let b = marks.bounding_box();
    let scale = 0.8 / b.height();
    marks.apply_affine(
        Affine::translate((1.0, 1.0))
            * Affine::scale(scale)
            * Affine::translate((-b.center().x, -b.center().y)),
    );
    let holes = marks
        .elements()
        .split(|el| matches!(el, PathEl::ClosePath))
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut hole = BezPath::from_vec(part.to_vec());
            hole.close_path();
            hole
        })
        .collect();
    with_holes(body, holes)
}

/// `body` with `holes` cut out of it: each hole turned against it.
fn with_holes(body: BezPath, holes: Vec<BezPath>) -> BezPath {
    let mut path = oriented(body, true);
    for hole in holes {
        path.extend(oriented(hole, false));
    }
    path
}

/// A plus sign about the origin, its arms `reach` long and `half` thick
/// on each side of their middle line.
fn cross(reach: f64, half: f64) -> Vec<(f64, f64)> {
    let (r, h) = (reach, half);
    vec![
        (-h, -r),
        (h, -r),
        (h, -h),
        (r, -h),
        (r, h),
        (h, h),
        (h, r),
        (-h, r),
        (-h, h),
        (-r, h),
        (-r, -h),
        (-h, -h),
    ]
}

/// A square with its corners cut in by quarter circles of radius `r`.
fn ticket(r: f64) -> BezPath {
    let mut path = BezPath::new();
    path.move_to((r, 0.0));
    path.line_to((1.0 - r, 0.0));
    arc(&mut path, (1.0, 0.0), r, 180.0, -90.0);
    path.line_to((1.0, 1.0 - r));
    arc(&mut path, (1.0, 1.0), r, 270.0, -90.0);
    path.line_to((r, 1.0));
    arc(&mut path, (0.0, 1.0), r, 0.0, -90.0);
    path.line_to((0.0, r));
    arc(&mut path, (0.0, 0.0), r, 90.0, -90.0);
    path.close_path();
    path
}

/// `n` round puffs of radius `puff` on an ellipse of radii `radii` about
/// `centre`, with the ellipse filled: a puffy cloud.
fn cloud_ring(rx: f64, ry: f64, (cx, cy): (f64, f64), n: usize, puff: f64) -> Vec<BezPath> {
    let centre = (cx + rx, cy + ry);
    let mut parts: Vec<BezPath> = regular(n, -90.0)
        .into_iter()
        .map(|(x, y)| circle(centre.0 + rx * x, centre.1 + ry * y, puff))
        .collect();
    parts.push(Ellipse::new(centre, (rx, ry), 0.0).to_path(TOLERANCE));
    parts
}

/// A heart with full, round lobes and a short point.
fn heart_round() -> BezPath {
    let mut path = BezPath::new();
    path.move_to((0.5, 0.22));
    path.curve_to((0.42, -0.05), (0.0, -0.06), (0.0, 0.32));
    path.curve_to((0.0, 0.62), (0.32, 0.8), (0.5, 0.92));
    path.curve_to((0.68, 0.8), (1.0, 0.62), (1.0, 0.32));
    path.curve_to((1.0, -0.06), (0.58, -0.05), (0.5, 0.22));
    path.close_path();
    path
}

/// A heart with tall lobes, a deep cleft and a long, sharp point.
fn heart_sharp() -> BezPath {
    let mut path = BezPath::new();
    path.move_to((0.5, 0.3));
    path.curve_to((0.46, -0.08), (0.0, -0.08), (0.02, 0.32));
    path.curve_to((0.04, 0.6), (0.4, 0.8), (0.5, 1.15));
    path.curve_to((0.6, 0.8), (0.96, 0.6), (0.98, 0.32));
    path.curve_to((1.0, -0.08), (0.54, -0.08), (0.5, 0.3));
    path.close_path();
    path
}

/// A heart with a soft, rounded point.
fn heart_soft() -> BezPath {
    let mut path = BezPath::new();
    path.move_to((0.5, 0.25));
    path.curve_to((0.48, 0.0), (0.0, -0.04), (0.0, 0.3));
    path.curve_to((0.0, 0.6), (0.3, 0.8), (0.44, 0.88));
    path.curve_to((0.48, 0.9), (0.52, 0.9), (0.56, 0.88));
    path.curve_to((0.7, 0.8), (1.0, 0.6), (1.0, 0.3));
    path.curve_to((1.0, -0.04), (0.52, 0.0), (0.5, 0.25));
    path.close_path();
    path
}

/// A flower of `n` round petals about a round heart.
fn flower(n: usize) -> BezPath {
    let mut parts: Vec<BezPath> = regular(n, -90.0)
        .into_iter()
        .map(|(x, y)| circle(0.58 * x, 0.58 * y, 0.42))
        .collect();
    parts.push(circle(0.0, 0.0, 0.5));
    union(parts)
}

/// `path` turned so its signed area is positive (or negative): with every
/// part turned the same way the non-zero rule fills their union, and a part
/// turned the other way is a hole.
fn oriented(path: BezPath, positive: bool) -> BezPath {
    if (path.area() > 0.0) == positive {
        path
    } else {
        path.reverse_subpaths()
    }
}

/// The union of `parts` under the non-zero rule: all turned the same way.
fn union(parts: Vec<BezPath>) -> BezPath {
    let mut out = BezPath::new();
    for part in parts {
        out.extend(oriented(part, true));
    }
    out
}

/// `path` scaled so its exact bounds are the unit box, as a document path,
/// with the width over the height it had.
pub(crate) fn finished(mut path: BezPath) -> (ShapePath, f64) {
    let b = path.bounding_box();
    let (w, h) = (b.width(), b.height());
    path.apply_affine(
        Affine::scale_non_uniform(1.0 / w, 1.0 / h) * Affine::translate((-b.x0, -b.y0)),
    );
    let at = |p: Point| [p.x, p.y];
    let commands = path
        .elements()
        .iter()
        .map(|el| match *el {
            PathEl::MoveTo(p) => PathCommand::MoveTo(at(p)),
            PathEl::LineTo(p) => PathCommand::LineTo(at(p)),
            PathEl::QuadTo(c, p) => PathCommand::QuadTo(at(c), at(p)),
            PathEl::CurveTo(c1, c2, p) => PathCommand::CubicTo(at(c1), at(c2), at(p)),
            PathEl::ClosePath => PathCommand::Close,
        })
        .collect();
    (ShapePath::new(commands), w / h)
}

/// The outline of `piece` as a `kurbo` path, its own proportions kept (a
/// unit square for the shapes drawn without a path).
pub(crate) fn piece_path(piece: Piece) -> kurbo::BezPath {
    match built(piece) {
        Some((path, aspect)) => bez(&path, aspect, 1.0),
        None => kurbo::Rect::new(0.0, 0.0, 1.0, 1.0).to_path(TOLERANCE),
    }
}

/// `path` as a `kurbo` path of `w` × `h`, from the top-left corner.
pub(crate) fn bez(path: &ShapePath, w: f64, h: f64) -> kurbo::BezPath {
    let at = |[x, y]: [f64; 2]| kurbo::Point::new(x * w, y * h);
    let mut out = kurbo::BezPath::new();
    for command in &path.commands {
        match *command {
            PathCommand::MoveTo(p) => out.move_to(at(p)),
            PathCommand::LineTo(p) => out.line_to(at(p)),
            PathCommand::QuadTo(c, p) => out.quad_to(at(c), at(p)),
            PathCommand::CubicTo(c1, c2, p) => out.curve_to(at(c1), at(c2), at(p)),
            PathCommand::Close => out.close_path(),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    // S96-01: a section shows five shapes in its row; the arrows move a
    // whole row at a time and stop at either end; «Ver todo» is only for
    // a section of more than five.
    #[test]
    fn s96_01_a_row_pages_by_five_and_stops_at_the_ends() {
        assert_eq!(ROW, 5);
        assert!(!has_more(5) && has_more(6));
        assert_eq!(paged(20, 0, 1), 5);
        assert_eq!(paged(20, 5, 1), 10);
        assert_eq!(paged(20, 15, 1), 15, "the last row stays");
        assert_eq!(paged(8, 0, 1), 3, "never past the last shape");
        assert_eq!(paged(8, 3, -1), 0);
        assert_eq!(paged(20, 2, -1), 0, "never before the first");
        assert_eq!(paged(4, 0, 1), 0, "a short section does not move");
        assert_eq!(paged(20, 99, 0), 15, "an old index is kept in range");
    }

    // S96-02: every section the reference asks to grow has more than five
    // shapes, so it has «Ver todo».
    // The tail of the cloud bubble leaves its body only at its point: the
    // corners of its base are inside a puff.
    #[test]
    fn the_cloud_bubble_tail_starts_inside_the_body() {
        // Inside the core, not merely a puff: a corner in the notch between
        // two puffs would stick out.
        let ((cx, cy), (rx, ry)) = CLOUD_CORE;
        let core = |(x, y): (f64, f64)| ((x - cx) / rx).powi(2) + ((y - cy) / ry).powi(2);
        assert!(core(CLOUD_TAIL[0]) < 1.0 && core(CLOUD_TAIL[2]) < 1.0);
        let in_puff = |(x, y): (f64, f64)| {
            CLOUD_PUFFS
                .iter()
                .any(|&(px, py, r)| (x - px).hypot(y - py) < r)
        };
        assert!(
            !in_puff(CLOUD_TAIL[1]) && core(CLOUD_TAIL[1]) > 1.0,
            "the point is out"
        );
    }

    // Spec 098: the tail of any cloud starts inside its core, whatever
    // the way to its point.
    #[test]
    fn a_cloud_tail_starts_inside_the_core() {
        let core = ((1.0, 0.6), (0.95, 0.48));
        for tip in [(0.3, 1.4), (0.0, 1.62), (1.9, 1.3), (1.0, 1.5), (-0.2, 0.6)] {
            for (x, y) in tail_base(core, tip) {
                let ((cx, cy), (rx, ry)) = core;
                let v = ((x - cx) / rx).powi(2) + ((y - cy) / ry).powi(2);
                assert!(v < 0.5, "{tip:?}: {v}");
            }
        }
    }

    #[test]
    fn s96_02_the_grown_sections_have_more_than_five() {
        for key in [
            "pieces.basic",
            "pieces.polygons",
            "pieces.stars",
            "pieces.arrows",
            "pieces.flowchart",
            "pieces.speech",
            "pieces.hearts",
            "pieces.gears",
        ] {
            let (_, pieces) = PIECE_SECTIONS
                .iter()
                .find(|(title, _)| *title == key)
                .expect("a section");
            assert!(has_more(pieces.len()), "{key}");
        }
        let clouds = PIECE_SECTIONS
            .iter()
            .find(|(title, _)| *title == "pieces.clouds")
            .expect("clouds");
        assert_eq!(clouds.1.len(), 5);
    }

    // S54-05: every shape drawn by a path is a valid path of the document
    // whose exact bounds are the unit box, with area; the sections follow
    // the reference, none is empty, and every shape is in one of them.
    #[test]
    fn s54_05_library_paths_are_valid_and_fill_the_box() {
        for piece in Piece::ALL {
            let geometry = piece.geometry();
            let ShapeGeometry::Path(path) = geometry else {
                assert!(
                    matches!(piece, Piece::Square | Piece::RoundedSquare | Piece::Circle)
                        || piece.line_caps().is_some(),
                    "{piece:?} draws a path"
                );
                continue;
            };
            assert!(
                matches!(path.commands.first(), Some(PathCommand::MoveTo(_))),
                "{piece:?}"
            );
            assert!(path.commands.len() <= ShapePath::MAX_COMMANDS, "{piece:?}");
            assert!(
                path.coordinates()
                    .all(|v| (ShapePath::MIN_COORDINATE..=ShapePath::MAX_COORDINATE).contains(&v)),
                "{piece:?}"
            );
            let shape = bez(&path, 1.0, 1.0);
            let b = shape.bounding_box();
            for (got, want) in [(b.x0, 0.0), (b.y0, 0.0), (b.x1, 1.0), (b.y1, 1.0)] {
                assert!((got - want).abs() < 1e-9, "{piece:?}: {b:?}");
            }
            // A spray of drops (spec 109) covers little of its box.
            let least = if matches!(piece, Piece::Splatter(_) | Piece::Traced(_)) {
                0.02
            } else {
                0.1
            };
            assert!(shape.area().abs() > least, "{piece:?} has area");
            assert!((0.25..=4.0).contains(&piece.aspect()), "{piece:?}");
        }
        let titles: Vec<&str> = PIECE_SECTIONS.iter().map(|(title, _)| *title).collect();
        assert_eq!(
            titles[..5],
            [
                "pieces.lines",
                "pieces.basic",
                "pieces.polygons",
                "pieces.stars",
                "pieces.arrows"
            ]
        );
        assert!(PIECE_SECTIONS.iter().all(|(_, pieces)| !pieces.is_empty()));
        let listed: Vec<Piece> = PIECE_SECTIONS
            .iter()
            .flat_map(|(_, p)| p.iter().copied())
            .collect();
        assert_eq!(listed, Piece::ALL, "every shape once, in order");
        let keys: BTreeSet<&str> = Piece::ALL.iter().map(|p| p.key()).collect();
        assert_eq!(keys.len(), Piece::ALL.len());
    }
}
