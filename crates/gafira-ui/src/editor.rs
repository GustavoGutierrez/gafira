//! The regions of the Design editor (03 §7.1, §7.3): top bar, tab rail, side
//! panel, canvas with the page, inspector and bottom bar.

use std::collections::BTreeSet;
use std::sync::Arc;

use egui::{
    Align, Align2, Color32, Key, KeyboardShortcut, Layout, Modifiers, Pos2, Rect, RichText, Stroke,
    Vec2,
};
use gafira_doc::msg::MessageKey;
use gafira_doc::{Document, GuideAxis, GuideSet, NodeId, Page, PageId, Placement, ShapeGeometry};
use gafira_edit::{
    Command, GestureId, InsertNodes, InsertPage, Property, RemoveNodes, SetProperty, Transaction,
};
use gafira_engine::Gafira;
use gafira_layout::{AlignAction, DistributeAxis};
use gafira_ports::settings::ViewSettings;
use gafira_theme::Shadow;

use crate::arrange::{self, Order};
use crate::canvas::{self, Shape};
use crate::clipboard::{self, Clip};
use crate::elements::{self, Category, Move, Rail};
use crate::files::{FileAction, project_name};
use crate::format::fit_zoom;
use crate::group::{self, Change};
use crate::icons::{self, Icon, Weight};
use crate::layers::{self, Flag};
use crate::lines;
use crate::magnet::{self, Snap};
use crate::pairings;
use crate::resize;
use crate::rotation::{self, Axis};
use crate::ruler;
use crate::selection::Selection;
use crate::shell::Go;
use crate::style::{Theme, color32, px};
use crate::system_fonts;
use crate::tables;
use crate::text::Texts;
use crate::transfer;
use crate::versions::Versions;
use crate::widgets::{self, IconButton, Surface};
use crate::zoom::{self, Zoom, ZoomAsk};

/// The tabs of the rail, in the order of 03 §2.3, with their icons.
/// The tabs of the rail (owner request): Elementos first, the one open
/// when a design opens; Marca third from last and Plantillas last.
const TABS: [(&str, Icon); 6] = [
    ("tab.pieces", Icon::Pieces),
    ("tab.text", Icon::Text),
    ("tab.library", Icon::Library),
    ("tab.identity", Icon::Identity),
    ("tab.drawing", Icon::Drawing),
    ("tab.templates", Icon::Templates),
];

/// The margin around the page on the canvas, in logical px.
const CANVAS_MARGIN: f32 = 48.0;

/// The space between two pages on the canvas, in logical px (03 §7.3: the
/// pages stack vertically).
const PAGE_GAP: f32 = 56.0;
/// The room under the last page for «Añadir página» (spec 077).
const ADD_PAGE_ROOM: f32 = 48.0;

/// The shortcuts of 03 §9.2 that tooltips and the Edición menu show.
const UNDO: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::Z);
const REDO: KeyboardShortcut =
    KeyboardShortcut::new(Modifiers::COMMAND.plus(Modifiers::SHIFT), Key::Z);
const ZOOM_IN: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::Equals);
const ZOOM_OUT: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::Minus);
const ZOOM_FIT: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::Num0);
const SELECT_ALL: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::A);
const FLIP_H: KeyboardShortcut = KeyboardShortcut::new(Modifiers::SHIFT, Key::H);
const FLIP_V: KeyboardShortcut = KeyboardShortcut::new(Modifiers::SHIFT, Key::V);
/// Cortar, Copiar, Pegar, Duplicar, Agrupar and Desagrupar (03 §9.2, spec
/// 043).
const CUT: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::X);
const COPY: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::C);
const PASTE: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::V);
const DUPLICATE: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::D);
const GROUP: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::G);
const UNGROUP: KeyboardShortcut =
    KeyboardShortcut::new(Modifiers::COMMAND.plus(Modifiers::SHIFT), Key::G);
/// Alinear (03 §9.2 «Capas», D-88): Alt with one letter, never Ctrl+Alt,
/// which locks the screen on some Linux desktops and is AltGr on Windows.
const ALIGN_LEFT: KeyboardShortcut = KeyboardShortcut::new(Modifiers::ALT, Key::A);
const ALIGN_CENTER: KeyboardShortcut = KeyboardShortcut::new(Modifiers::ALT, Key::H);
const ALIGN_RIGHT: KeyboardShortcut = KeyboardShortcut::new(Modifiers::ALT, Key::D);
const ALIGN_TOP: KeyboardShortcut = KeyboardShortcut::new(Modifiers::ALT, Key::W);
const ALIGN_MIDDLE: KeyboardShortcut = KeyboardShortcut::new(Modifiers::ALT, Key::V);
const ALIGN_BOTTOM: KeyboardShortcut = KeyboardShortcut::new(Modifiers::ALT, Key::S);
/// Ver → Reglas, Guías and Imán (03 §9.2, FD-03, FD-06; spec 044).
const RULERS: KeyboardShortcut =
    KeyboardShortcut::new(Modifiers::COMMAND.plus(Modifiers::SHIFT), Key::R);
const GUIDES: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::Semicolon);
const MAGNET: KeyboardShortcut =
    KeyboardShortcut::new(Modifiers::COMMAND.plus(Modifiers::SHIFT), Key::Quote);
/// Orden de apilado (03 §9.2 «Capas», spec 042).
const FORWARD: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::CloseBracket);
const BACKWARD: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::OpenBracket);
const FRONT: KeyboardShortcut =
    KeyboardShortcut::new(Modifiers::COMMAND.plus(Modifiers::SHIFT), Key::CloseBracket);
const BACK: KeyboardShortcut =
    KeyboardShortcut::new(Modifiers::COMMAND.plus(Modifiers::SHIFT), Key::OpenBracket);

/// The six Align actions with their catalog key, icon and shortcut, in the
/// order of FD-11 (03 §9.2, D-88).
const ALIGNS: [(AlignAction, &str, Icon, Option<KeyboardShortcut>); 6] = [
    (
        AlignAction::Left,
        "arrange.align_left",
        Icon::AlignLeft,
        Some(ALIGN_LEFT),
    ),
    (
        AlignAction::Center,
        "arrange.align_center",
        Icon::AlignCenter,
        Some(ALIGN_CENTER),
    ),
    (
        AlignAction::Right,
        "arrange.align_right",
        Icon::AlignRight,
        Some(ALIGN_RIGHT),
    ),
    (
        AlignAction::Top,
        "arrange.align_top",
        Icon::AlignTop,
        Some(ALIGN_TOP),
    ),
    (
        AlignAction::Middle,
        "arrange.align_middle",
        Icon::AlignMiddle,
        Some(ALIGN_MIDDLE),
    ),
    (
        AlignAction::Bottom,
        "arrange.align_bottom",
        Icon::AlignBottom,
        Some(ALIGN_BOTTOM),
    ),
];

/// The two Distribute actions (FD-11); they have no shortcut in 03 §9.2.
const DISTRIBUTES: [(DistributeAxis, &str, Icon); 2] = [
    (
        DistributeAxis::Horizontal,
        "arrange.distribute_horizontal",
        Icon::DistributeHorizontal,
    ),
    (
        DistributeAxis::Vertical,
        "arrange.distribute_vertical",
        Icon::DistributeVertical,
    ),
];

/// The four layer-order actions (FD-12) in the order of 03 §9.2.
const ORDERS: [(Order, &str, Icon, KeyboardShortcut); 4] = [
    (
        Order::Forward,
        "arrange.forward",
        Icon::BringForward,
        FORWARD,
    ),
    (
        Order::Backward,
        "arrange.backward",
        Icon::SendBackward,
        BACKWARD,
    ),
    (Order::Front, "arrange.front", Icon::BringToFront, FRONT),
    (Order::Back, "arrange.back", Icon::SendToBack, BACK),
];

/// The text of a catalog key without parameters.
fn plain(texts: &Texts, key: &'static str) -> String {
    texts.t(&gafira_doc::msg::Message {
        key: MessageKey(key),
        args: Vec::new(),
    })
}

/// What the arrange actions can do with the selection this frame (spec
/// 042): align with one element or more, distribute with three or more,
/// and each order action only when it would change the layers (FD-12),
/// among the siblings the selection shares (spec 043). Group with one
/// element or more and ungroup when a group is selected (spec 043).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct CanArrange {
    align: bool,
    distribute: bool,
    /// By the index of [`ORDERS`].
    order: [bool; 4],
    group: bool,
    ungroup: bool,
}

impl CanArrange {
    fn of(document: Option<&Document>, selected: &Selection) -> Self {
        let Some(page) = selected.page().and_then(|page| document?.page(page)) else {
            return Self::default();
        };
        let nodes = selected.nodes();
        let siblings = group::siblings(page, nodes);
        Self {
            align: !nodes.is_empty(),
            distribute: nodes.len() >= 3,
            order: ORDERS.map(|(order, ..)| {
                siblings
                    .as_ref()
                    .is_some_and(|(_, list)| arrange::reordered(list, nodes, order) != *list)
            }),
            group: !nodes.is_empty(),
            ungroup: nodes
                .iter()
                .any(|id| group::ungroup_plan(page, *id).is_some()),
        }
    }
}

/// A shortcut as the platform writes it, with the key as its symbol: egui
/// names `]` «CloseBracket» and `=` «Equals» (spec 042).
fn keys_text(ctx: &egui::Context, keys: &KeyboardShortcut) -> String {
    let key = keys.logical_key;
    ctx.format_shortcut(keys)
        .replace(key.name(), key.symbol_or_name())
}

/// «Deshacer · Ctrl+Z»: an action and its shortcut as the platform writes it
/// (03 §8.1, Tooltip).
fn with_keys(ctx: &egui::Context, texts: &Texts, action: String, keys: KeyboardShortcut) -> String {
    texts.t(&gafira_doc::msg!(
        "tooltip.shortcut",
        action = Text(action),
        keys = Text(keys_text(ctx, &keys))
    ))
}

/// The editor's view state; the document lives in the engine and is read
/// from a snapshot every frame (02 §5.5).
pub(crate) struct Editor {
    /// The active tab of the rail, whether the side panel shows and the
    /// view of Elementos (spec 064).
    rail: Rail,
    /// How the zoom is chosen: «Ajustar» or a fixed scale (spec 037).
    view: Zoom,
    /// The zoom of the last frame, for the bottom bar.
    zoom: f32,
    /// The Inspector tab on display: 0 Posición, 1 Capas, 2 Imagen.
    inspector_tab: usize,
    /// The selected elements, all of one page (03 §9.1, spec 040).
    selected: Selection,
    /// The textures of the images on display, by content hash; `None`
    /// when the bytes could not be read or decoded (a placeholder is drawn).
    textures: std::collections::HashMap<gafira_doc::ContentHash, Option<egui::TextureHandle>>,
    /// Biblioteca (spec 068): the imported files, the panel's tab, search
    /// and list, and the previews of its files.
    pub(crate) library: Option<std::rc::Rc<dyn crate::LibraryStore>>,
    /// Elementos → Fotos (spec 071): its search, results and downloads.
    pub(crate) photo_panel: std::cell::RefCell<crate::photos::PhotosPanel>,
    /// Dibujo: the tool, the brushes' settings and the stroke being drawn
    /// (spec 079).
    draw: std::cell::RefCell<crate::draw::DrawState>,
    /// Elementos → Ilustraciones (spec 072).
    pub(crate) illustration_panel: std::cell::RefCell<crate::illustrations::IllustrationsPanel>,
    /// Elementos → Códigos QR and the QR fields of Propiedades (spec 080).
    qr_panel: std::cell::RefCell<crate::qr::QrPanel>,
    /// The changes to a QR code being made in Propiedades: one gesture, one
    /// undo step, until its fields lose the focus (S80-12).
    qr_gesture: Option<(NodeId, GestureId)>,
    /// Elementos → Tablas and the tables on the canvas (spec 081): the
    /// cells chosen, what is being dragged, the menu open and where, the
    /// cell the pointer was over last, the pen of the borders, the looks
    /// copied and the changes being made as one undo step.
    table_panel: std::cell::RefCell<crate::table_templates::Panel>,
    table_outlines: std::cell::RefCell<tables::Outlines>,
    table_pen: std::cell::Cell<tables::Pen>,
    table_style: Option<gafira_doc::CellStyle>,
    table_drag: Option<TableDrag>,
    table_menu: Option<(tables::Side, Pos2)>,
    table_hover: Option<(NodeId, (usize, usize))>,
    table_gesture: Option<GestureId>,
    cells: Option<tables::Cells>,
    library_view: std::cell::RefCell<(crate::library::View, f64)>,
    /// Marcos: its search and the collection open with «Ver todo».
    frames_view: std::cell::RefCell<(String, Option<usize>)>,
    /// The search of «Fondo de la página» (spec 069).
    background_query: std::cell::RefCell<String>,
    library_previews: std::cell::RefCell<std::collections::HashMap<std::path::PathBuf, Preview>>,
    /// The decoded pixels of the photos of frames, by content hash, cut
    /// out by each frame's shape into a texture (spec 066).
    photos: std::collections::HashMap<gafira_doc::ContentHash, Option<Arc<egui::ColorImage>>>,
    /// The drag in progress: its gesture (one undo entry, ARQ-33), where the
    /// selected elements started and how far the pointer went.
    drag: Option<Drag>,
    /// The marquee being drawn from empty canvas (spec 040).
    marquee: Option<Marquee>,
    /// The font atlas size the last turned text was mapped with.
    font_atlas: Option<[usize; 2]>,
    /// «Versiones» and «Guardar versión…» (spec 031).
    versions: Versions,
    /// What Align, Distribute and Orden can do this frame (spec 042).
    can: CanArrange,
    /// The group entered with a double click, and its page: a click picks
    /// among its children (03 §9.1, spec 043). `None` picks among the
    /// roots.
    scope: Option<(PageId, NodeId)>,
    /// The groups open in Capas (spec 043).
    open: BTreeSet<NodeId>,
    /// What Copiar or Cortar took (spec 043). The shell keeps it between
    /// projects of the session.
    clip: Option<Arc<Clip>>,
    /// A copy waits for its plain text on the system clipboard.
    copied: bool,
    /// Ver → Reglas, Guías and Imán (spec 044), read from the settings on
    /// the first frame.
    shown: ViewSettings,
    shown_loaded: bool,
    /// A guide being dragged from a ruler or along the page (spec 044).
    guide_drag: Option<GuideDrag>,
    /// What the magnet snapped to in the drag in progress, for the smart
    /// guides (spec 044).
    snapped: Option<(PageId, Snap)>,
    /// The row of Capas being renamed (spec 048): its node, the text typed
    /// so far and whether the field already took the keyboard.
    renaming: Option<(NodeId, String, bool)>,
    /// The row of Capas being dragged to a new place (spec 048).
    row_drag: Option<NodeId>,
    /// The page clicked last, drawn with a border; `None` after a click
    /// outside every page (spec 050).
    active: Option<PageId>,
    /// The page whose title is being written in its header, the words so
    /// far and whether the field took the keyboard (spec 056).
    renaming_page: Option<(PageId, String, bool)>,
    /// Each page's id and screen box in the last frame drawn: where a shape
    /// dragged from the panel lands (spec 052).
    page_rects: Vec<(PageId, Rect)>,
    /// The textures path shapes are drawn from, on the canvas and in the
    /// Formas panel, by what they show (spec 054).
    path_textures: std::cell::RefCell<std::collections::BTreeMap<u64, egui::TextureHandle>>,
    /// The box each curved text texture covers, by its key (FD-44).
    curved_covers: std::cell::RefCell<std::collections::BTreeMap<u64, Placement>>,
    /// The text being written on the canvas (spec 055).
    editing: Option<Editing>,
    /// Crop mode of a frame (S66-07): the photo moves inside it, the whole
    /// session one undo step.
    cropping: Option<Cropping>,
    /// A text paste arrived since Ctrl+V went down: its release then does
    /// not look for an image in the clipboard (spec 067).
    paste_seen: bool,
    /// SVG files waiting for «¿Cómo quieres importarlo?» (spec 067):
    /// the files and where they were dropped.
    pending_svg: Option<(Vec<std::path::PathBuf>, Option<Pos2>)>,
    /// The frame an image dragged on the canvas would drop into (spec 066):
    /// lit, with the photo previewed in it.
    frame_target: Option<FrameTarget>,
    /// What the drag of a line end shows (spec 058).
    line_view: Option<LineView>,
    /// A text whose box takes the height of its words once drawn (a paste,
    /// spec 061), and the height measured when a text is drawn.
    fit_pending: Option<(PageId, NodeId)>,
    text_measure: Option<(PageId, NodeId, f64)>,
    /// The Fuente panel takes the side panel (spec 062), with the font of
    /// the selected text and the families the document uses, as the
    /// Inspector last saw them.
    font_panel: bool,
    selection_font: Option<(String, bool, bool)>,
    document_fonts: Vec<String>,
    /// The families of the system already installed in egui.
    loaded_fonts: std::cell::RefCell<BTreeSet<String>>,
}

/// Crop mode of a frame (S66-07).
#[derive(Clone, Copy, Debug)]
struct Cropping {
    page: PageId,
    node: NodeId,
    gesture: Option<GestureId>,
}

/// An image dragged over a frame (spec 066): where it would go.
#[derive(Clone, Copy, Debug, PartialEq)]
struct FrameTarget {
    page: PageId,
    frame: NodeId,
    image: NodeId,
    asset: gafira_doc::timeline::AssetRef,
}

/// What the drag of a line end shows (spec 058): the shapes near it, whose
/// five points are drawn; the one it sticks to, whose outline is lit; and
/// both ends in page px, for the length and angle beside the pointer.
struct LineView {
    page: PageId,
    near: Vec<NodeId>,
    stuck: Option<NodeId>,
    end: gafira_geom::Point,
    other: gafira_geom::Point,
}

/// A text being written on the canvas (spec 055): whose it is, the words so
/// far and the gesture that makes the whole edit one undo step.
struct Editing {
    page: PageId,
    node: NodeId,
    words: String,
    gesture: Option<GestureId>,
    /// The field took the keyboard on its first frame.
    focused: bool,
}

/// A guide held by the pointer (FD-03, spec 044): a new one from a ruler
/// (`index` is `None`) or one of the page's.
#[derive(Clone, Copy, Debug, PartialEq)]
struct GuideDrag {
    page: PageId,
    axis: GuideAxis,
    index: Option<usize>,
}

struct Drag {
    gesture: Option<GestureId>,
    /// The page of the selection.
    page: PageId,
    /// Each selected element and where it started; a group by its box.
    origins: Vec<(NodeId, Placement)>,
    /// The document when the drag began: a group moves every node below
    /// it from where it was then (spec 043).
    start: Option<Arc<Document>>,
    /// Whether a move of the drag changed the document: one undo step to
    /// fold into a transfer (spec 051).
    moved: bool,
    offset: Vec2,
    /// The handle held, or `None` for a move. Only a single selection
    /// resizes or turns.
    hold: Option<Hold>,
}

/// A handle held on a single selection.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Hold {
    Resize(canvas::Handle),
    /// The rotate handle (spec 041): the element's centre and where the
    /// pointer was pressed, in screen px.
    Rotate {
        center: Pos2,
        from: Pos2,
    },
    /// One end of a line (spec 058): 0 the start, 1 the end.
    LineEnd(usize),
}

/// A marquee on the canvas, in screen px: where the drag started, where the
/// pointer is, and whether Shift adds to the selection (spec 040).
#[derive(Clone, Copy, Debug)]
struct Marquee {
    from: Pos2,
    to: Pos2,
    add: bool,
}

impl Default for Editor {
    fn default() -> Self {
        Self {
            rail: Rail::default(),
            view: Zoom::Fit,
            zoom: 1.0,
            inspector_tab: 0,
            selected: Selection::default(),
            textures: std::collections::HashMap::new(),
            photos: std::collections::HashMap::new(),
            library: None,
            photo_panel: std::cell::RefCell::default(),
            draw: std::cell::RefCell::default(),
            illustration_panel: std::cell::RefCell::default(),
            qr_panel: std::cell::RefCell::default(),
            qr_gesture: None,
            table_panel: std::cell::RefCell::default(),
            table_outlines: std::cell::RefCell::default(),
            table_pen: std::cell::Cell::default(),
            table_style: None,
            table_drag: None,
            table_menu: None,
            table_hover: None,
            table_gesture: None,
            cells: None,
            library_view: std::cell::RefCell::default(),
            frames_view: std::cell::RefCell::default(),
            background_query: std::cell::RefCell::default(),
            library_previews: std::cell::RefCell::default(),
            drag: None,
            marquee: None,
            versions: Versions::default(),
            font_atlas: None,
            can: CanArrange::default(),
            scope: None,
            open: BTreeSet::new(),
            clip: None,
            copied: false,
            shown: ViewSettings::default(),
            shown_loaded: false,
            guide_drag: None,
            snapped: None,
            renaming: None,
            row_drag: None,
            active: None,
            renaming_page: None,
            page_rects: Vec::new(),
            path_textures: std::cell::RefCell::default(),
            curved_covers: std::cell::RefCell::default(),
            editing: None,
            cropping: None,
            paste_seen: false,
            pending_svg: None,
            frame_target: None,
            line_view: None,
            fit_pending: None,
            text_measure: None,
            font_panel: false,
            selection_font: None,
            document_fonts: Vec::new(),
            loaded_fonts: std::cell::RefCell::default(),
        }
    }
}

/// What the person asked for this frame; applied after drawing, so no
/// panel holds the engine while another reads it.
#[derive(Default)]
struct Asked {
    /// A freehand stroke to add to its page (spec 079).
    draw_stroke: Option<(PageId, gafira_edit::NodeSpec)>,
    /// Strokes the eraser went over, to remove in one step (spec 079).
    erase: Option<(PageId, Vec<NodeId>)>,
    home: bool,
    file: Option<FileAction>,
    /// «Versiones» in the top bar, «Guardar versión…» in Archivo.
    versions: bool,
    save_version: bool,
    /// The name of a version to save, from «Guardar versión» (spec 036).
    version: Option<String>,
    undo: bool,
    redo: bool,
    add_page: bool,
    /// A new piece of this geometry on the selected element's page, or the
    /// first page.
    insert: Option<canvas::Piece>,
    /// A new light in the middle of the page (spec 065).
    insert_effect: Option<elements::LightItem>,
    /// A new selection (an empty one clears it, Esc).
    select: Option<Selection>,
    /// Ctrl+A or Edición → Seleccionar todo (spec 040).
    select_all: bool,
    /// A drag of the selection begins: a move, or a resize or a turn by
    /// this handle.
    drag_begin: Option<Option<Hold>>,
    drag_to: Option<Vec2>,
    /// Shift is held during the drag: a turn snaps to 15° (FD-06).
    snap: bool,
    /// Ctrl (Cmd on macOS) is held during the drag: the magnet lets go
    /// (FD-06, D-90).
    free: bool,
    /// Alt is held during the drag: a resize goes from the centre (03
    /// §9.1, D-90).
    centre: bool,
    /// An arrow moved the selection by this much, in px of page (03 §9.2).
    nudge: Option<(f64, f64)>,
    /// Ver → Reglas, Guías or Imán switched (spec 044).
    shown: Option<ViewSettings>,
    /// A guide created, moved or deleted: the page's new guide set.
    guides: Option<(PageId, GuideSet)>,
    /// Turn every selected element by these degrees about its centre
    /// (Edición, Inspector; spec 041).
    turn: Option<f64>,
    /// Flip every selected element about its centre (Mayús+H, Mayús+V).
    flip: Option<Axis>,
    /// Align the selection, or one element to its page (spec 042).
    align: Option<AlignAction>,
    /// Spread the selection with equal gaps (spec 042).
    distribute: Option<DistributeAxis>,
    /// Move the selection in the layer order (spec 042).
    order: Option<Order>,
    /// Agrupar and Desagrupar the selection (spec 043).
    group: bool,
    ungroup: bool,
    /// Duplicar the selection (spec 043), with the names its copies take:
    /// «Logo» and «Logo copia» (FD-13).
    duplicate: bool,
    copy_names: Vec<(String, String)>,
    /// Copiar, Cortar and Pegar (spec 043).
    copy: bool,
    cut: bool,
    paste: bool,
    /// Enter a group or leave one: the new scope of the canvas (spec 043).
    scope: Option<Option<(PageId, NodeId)>>,
    /// Esc inside a group: select the group and leave it (spec 043).
    leave: bool,
    /// Open or close a group's row in Capas (spec 043).
    toggle_open: Option<NodeId>,
    drag_end: bool,
    delete: bool,
    /// A placement typed in the Inspector for the selected element.
    place: Option<Placement>,
    /// A fill picked in the Inspector (`Some(None)` removes it).
    fill: Option<Option<gafira_color::Color>>,
    /// A gradient set in the Inspector (`Some(None)` goes back to the
    /// solid colour, spec 063).
    gradient: Option<Option<gafira_doc::Gradient>>,
    /// A light set in the Inspector (`Some(None)` takes it off, spec 065).
    effect: Option<Option<gafira_doc::ShapeEffect>>,
    /// An opacity set in the Inspector, 0 to 1.
    opacity: Option<f32>,
    /// An outline set in the Inspector (`Some(None)` removes it, spec 038).
    stroke: Option<Option<gafira_doc::ShapeStroke>>,
    /// A corner radius set in the Inspector, in px (spec 038).
    corner_radius: Option<f64>,
    /// Biblioteca → Importar archivo… (spec 029, 068).
    import_image: bool,
    /// A file of Biblioteca added: in the middle of the page, or dropped
    /// at a screen point (spec 068).
    library_add: Option<(std::path::PathBuf, bool, Option<Pos2>)>,
    /// Files taken out of Biblioteca's list (path and tab); they stay on
    /// the disk (spec 068).
    library_remove: Vec<(std::path::PathBuf, bool)>,
    /// A colour of a group changed in the Inspector: the group, the colour
    /// it had and the new one (spec 067).
    recolor: Option<(NodeId, gafira_color::Color, gafira_color::Color)>,
    /// «Fondo de la página» (spec 069): the background of the current page
    /// (`Some(None)` makes it white), the same background on every page,
    /// and «Subir una imagen».
    page_background: Option<Option<gafira_doc::PageBackground>>,
    /// «Usar como fondo de la página»: the image element becomes its page's
    /// background and leaves the page, in one step.
    image_to_background: Option<(PageId, NodeId, gafira_doc::timeline::AssetRef)>,
    background_all: bool,
    background_pick: bool,
    /// The answer to «¿Cómo quieres importarlo?» for the SVG files waiting:
    /// `Some(true)` as an image, `Some(false)` as a frame (spec 067).
    svg_as_image: Option<bool>,
    svg_cancel: bool,
    /// A new empty frame of this outline in the middle of the page, and
    /// one dropped on a page at a point in px of page (spec 066, D-105).
    insert_frame: Option<canvas::Piece>,
    drop_frame: Option<(canvas::Piece, PageId, (f64, f64))>,
    /// «Poner foto…» of the Inspector: pick an image for the selected
    /// frames (spec 066).
    pick_frame_photo: bool,
    /// «Importar forma (SVG o PDF)…» of Elementos → Marcos (S66-10).
    import_frame_shape: bool,
    /// The photo of the selected frames (`Some(None)` empties them) and how
    /// it sits (spec 066).
    frame_content: Option<Option<gafira_doc::FrameContent>>,
    frame_fit: Option<gafira_doc::Fit>,
    /// A new text element of this preset (Texto tab, spec 027).
    /// With the words it starts with, already in the interface language.
    insert_text: Option<(canvas::TextPreset, String)>,
    /// A curved text from the Texto panel, with its bend, in the middle of
    /// the page or where it was dropped (FD-44).
    insert_curved: Option<(f64, String, Option<Dropped>)>,
    /// A font pairing from the Texto panel, by its index in
    /// [`pairings::PAIRINGS`], in the middle of the page or where it was
    /// dropped: a group of a heading and a body line.
    insert_pairing: Option<(usize, (String, String), Option<Dropped>)>,
    /// «Curva» of the selected texts (FD-44).
    text_curve: Option<f64>,
    /// An illustration to insert as vector shapes (spec 072): its SVG, its
    /// credit and where it was let go (screen).
    insert_illustration: Option<(Arc<Vec<u8>>, String, Option<Pos2>)>,
    /// A QR code of Elementos, in the middle of the page or dropped at a
    /// screen point (spec 080, D-105).
    insert_qr: Option<(Box<gafira_doc::QrNode>, Option<Pos2>)>,
    /// The selected QR code changed in Propiedades, and whether its fields
    /// are at rest.
    qr: Option<Box<gafira_doc::QrNode>>,
    qr_rest: bool,
    /// «Añadir logotipo»: of the panel's code, or of the selected one.
    qr_panel_logo: bool,
    qr_logo: bool,
    /// «Convertir en formas» (S80-13).
    qr_to_shapes: bool,
    /// A table of Elementos, in the middle of the page or dropped at a
    /// screen point (spec 081, D-105).
    insert_table: Option<(crate::table_templates::Template, Option<Pos2>)>,
    /// A change of the selected table; whether the controls are at rest, so
    /// the next one is another undo step.
    table_act: Option<tables::Act>,
    table_rest: bool,
    /// The border of a column or a row dragged: the table and its box now.
    table_sized: Option<(gafira_doc::TableNode, Placement)>,
    table_menu: Option<tables::Menu>,
    /// The cells chosen from now on (`Some(None)` goes back to the table).
    cells: Option<Option<tables::Cells>>,
    /// Write in the first cell: the words it starts with, or its own.
    write_cell: Option<Option<String>>,
    /// The words written so far; and the end of the writing, with the move
    /// to the next cell.
    cell_words: Option<String>,
    end_cell: Option<CellMove>,
    /// A Foto collage layout, by its index in [`crate::collage::LAYOUTS`],
    /// filling the current page or the one it was dropped on (S66-09).
    insert_collage: Option<(usize, Option<PageId>)>,
    /// A frame of Marcos, in the middle of the current page or where it
    /// was dropped (spec 066, owner request with references).
    insert_frame_kind: Option<(crate::frames::Kind, Option<Dropped>)>,
    /// The alignment of the selected texts (spec 070).
    text_align: Option<gafira_doc::TextAlign>,
    /// The letter spacing and line height of the selected texts (FD-41).
    text_spacing: Option<(f64, Option<f64>)>,
    /// The colour or gradient of the selected texts' letters (spec 070).
    text_fill: Option<TextFill>,
    /// New rich text for the selected text element (Inspector).
    text: Option<gafira_doc::RichText>,
    /// An element to move to this index among its siblings (Capas).
    reorder: Option<(PageId, NodeId, usize)>,
    /// Ocultar or Bloquear from a row of Capas: these nodes of the page
    /// become hidden or locked (`true`) or visible or free (spec 048).
    flag: Option<(PageId, Vec<NodeId>, Flag, bool)>,
    /// Ctrl+Shift+H or Ctrl+Shift+L on the selection (S48-03).
    toggle: Option<Flag>,
    /// A name typed in Capas (S48-05).
    rename: Option<(PageId, NodeId, Option<String>)>,
    /// Alt+↑ (1) or Alt+↓ (-1): the selected element one step in the
    /// stacking order (S48-06).
    step: Option<isize>,
    /// A move dropped on another page: from, to and the shift in page px
    /// from one page's corner to the other's (spec 051).
    transfer: Option<(PageId, PageId, (f64, f64))>,
    /// A shape dragged from the panel and dropped on a page, at this point
    /// in px of page (spec 052).
    drop_piece: Option<(canvas::Piece, PageId, (f64, f64))>,
    /// A light dragged from the panel and dropped on a page, at this point
    /// in px of page (spec 065, D-105).
    drop_effect: Option<(elements::LightItem, PageId, (f64, f64))>,
    /// A double click on a text or a shape: write on it (spec 055).
    begin_edit: Option<(PageId, NodeId)>,
    /// Crop mode (S66-07): enter it on a frame, move the photo by screen
    /// px, set its offset and scale from the Inspector, leave it.
    begin_crop: Option<(PageId, NodeId)>,
    crop_drag: Option<Vec2>,
    crop_set: Option<((f64, f64), f64, f64)>,
    end_crop: bool,
    /// The words written so far on the canvas.
    edit_text: Option<String>,
    /// The writing on the canvas ends: Esc or a click elsewhere.
    end_edit: bool,
    /// The alignment of the selected shape's text, across and down.
    shape_align: Option<(
        Option<gafira_doc::TextAlign>,
        Option<gafira_doc::VerticalAlign>,
    )>,
    /// An action of a page header (spec 056).
    page_action: Option<PageAction>,
    /// Text pasted from outside Gafira: a new text box (spec 061).
    paste_text: Option<String>,
    /// Spec 067: image files pasted (their paths were the clipboard text),
    /// and image files dropped from the system with where the pointer was.
    paste_files: Vec<std::path::PathBuf>,
    drop_files: Vec<std::path::PathBuf>,
    drop_at: Option<Pos2>,
    /// Ctrl+V went up this frame, and a paste event came this frame.
    v_released: bool,
    pasted_any: bool,
    /// A text of the Texto panel dropped on a page at this point in px of
    /// page (spec 061).
    drop_text: Option<(canvas::TextPreset, String, PageId, (f64, f64))>,
    /// The height of the words written on the canvas, in px of page.
    edit_height: Option<f64>,
    /// A font for the selected text or shape text: family, bold, italic
    /// (spec 062).
    font: Option<(String, bool, bool)>,
    /// Open (true) or close the Fuente panel.
    font_panel: Option<bool>,
    /// Ver → Modo oscuro switched to this value (D-82).
    dark: Option<bool>,
    /// Archivo → Ajustes (spec 034).
    settings: bool,
    /// «Exportar» was pressed (spec 030).
    export: bool,
    /// A zoom button, menu item or shortcut (spec 037).
    zoom: Option<ZoomAsk>,
}

impl Asked {
    /// The keys held during a drag (D-90): Shift frees a corner or snaps
    /// a turn, Alt resizes from the centre and Ctrl lets the magnet go.
    fn hold_keys(&mut self, keys: resize::DragKeys) {
        self.snap = keys.shift;
        self.centre = keys.centre;
        self.free = keys.free;
    }
}

/// Where the writing in a cell goes on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CellMove {
    Stay,
    Down,
    Next,
    Before,
}

/// What is dragged on a table (spec 081): the border of a column or of a
/// row, from where and over the table as it was, or a range of cells.
#[derive(Clone, Debug)]
enum TableDrag {
    Column(usize, Box<TableStart>),
    Row(usize, Box<TableStart>),
    Cells,
}

/// What a drag on a table started from.
#[derive(Clone, Debug)]
struct TableStart {
    table: gafira_doc::TableNode,
    placement: Placement,
    at: gafira_geom::Point,
}

/// What a page header asks for (spec 056, FD-02).
#[derive(Clone, Debug, PartialEq)]
enum PageAction {
    /// One place towards the front (`-1`) or the back (`1`).
    Move(PageId, isize),
    Hide(PageId, bool),
    Lock(PageId, bool),
    Duplicate(PageId),
    Delete(PageId),
    /// An empty page of the same size right after it.
    AddAfter(PageId),
    Rename(PageId, String),
}

/// What the top bar shows about the session.
struct Header<'a> {
    can_undo: bool,
    can_redo: bool,
    modified: bool,
    name: Option<String>,
    notice: Option<&'a str>,
}

/// The selected shapes still in `document`, in selection order (an undo may
/// have taken some away). A selected group is listed by its box (spec
/// 043).
fn selected_shapes(document: Option<&Document>, selected: &Selection) -> Vec<Shape> {
    let Some(page) = selected.page().and_then(|page| document?.page(page)) else {
        return Vec::new();
    };
    chosen(page, selected)
}

/// The selected shapes of `page`, in selection order; a group by its box.
fn chosen(page: &Page, selected: &Selection) -> Vec<Shape> {
    if selected.page() != Some(page.id) {
        return Vec::new();
    }
    let shapes = canvas::shapes(page);
    selected
        .nodes()
        .iter()
        .filter_map(|node| {
            shapes
                .iter()
                .find(|shape| shape.node == *node)
                .cloned()
                .or_else(|| group_shape(page, *node))
        })
        .collect()
}

/// A drawn group as a shape: its box around its children, upright, with
/// the group's opacity (spec 043). `None` for anything else.
fn group_shape(page: &Page, node: NodeId) -> Option<Shape> {
    if !group::is_group(page, node) || !group::draws(page, node) {
        return None;
    }
    let placement = group::item_box(page, node)?;
    Some(Shape {
        page: page.id,
        node,
        placement,
        geometry: ShapeGeometry::Rect,
        fill: None,
        stroke: None,
        corner_radius: 0.0,
        gradient: None,
        effect: None,
        opacity: page.node(node)?.base.opacity,
        inherited: 1.0,
        group: true,
        text: None,
        image: None,
        locked: crate::layers::locked(page, node),
        inner: None,
        line: None,

        frame: None,
        qr: None,
        table: None,
    })
}

/// The selection a finished marquee makes (spec 040): the shapes it touches
/// on the page where it started, or else on the first page it touches.
/// With Shift they join `current`; without, they replace it. `pick` turns
/// a touched shape into what is selected: itself, or the group that holds
/// it in the scope of the canvas (spec 043).
fn marquee_selection(
    current: &Selection,
    marquee: Marquee,
    laid: &[(PageId, Rect, Vec<Shape>)],
    zoom: f32,
    pick: impl Fn(PageId, NodeId) -> Option<NodeId>,
) -> Selection {
    let rect = Rect::from_two_pos(marquee.from, marquee.to);
    let touched = |(id, page, shapes): &(PageId, Rect, Vec<Shape>)| {
        let picked = canvas::in_marquee(shapes, *page, zoom, rect)
            .into_iter()
            .filter_map(|leaf| pick(*id, leaf))
            .collect::<Vec<_>>();
        (*id, picked)
    };
    let on = laid
        .iter()
        .find(|(_, page, _)| page.contains(marquee.from))
        .map(touched)
        .or_else(|| {
            laid.iter()
                .map(touched)
                .find(|(_, nodes)| !nodes.is_empty())
        });
    let mut next = if marquee.add {
        current.clone()
    } else {
        Selection::default()
    };
    if let Some((page, nodes)) = on {
        next.add(page, nodes);
    }
    next
}

/// One step for many commands: the command itself when there is one, a
/// `Transaction` that merges by gesture when there are more (spec 040).
fn batch(mut commands: Vec<Box<dyn Command>>) -> Option<Box<dyn Command>> {
    match commands.len() {
        0 | 1 => commands.pop(),
        _ => Some(Box::new(Transaction::new(
            MessageKey("edit.set_property"),
            commands,
        ))),
    }
}

/// The pages of `document` in logical px at its dpi.
fn page_sizes(document: &Document) -> Vec<Vec2> {
    document
        .pages()
        .iter()
        .filter_map(|page| page.size.to_px(document.dpi()).ok())
        .map(|size| Vec2::new(size.width() as f32, size.height() as f32))
        .collect()
}

impl Editor {
    /// The page of the selection, or the first page.
    fn current_page<'a>(&self, document: &'a Document) -> Option<&'a Page> {
        let ids: Vec<PageId> = document.pages().iter().map(|page| page.id).collect();
        let id = current_page_id(&ids, self.selected.page(), self.active)?;
        document.page(id)
    }

    /// Drops from the selection what `document` no longer draws (an undo
    /// may have taken it away) and returns the shapes that stay, in
    /// selection order (spec 040). The scope goes when its group does
    /// (spec 043).
    fn prune(&mut self, document: Option<&Document>) -> Vec<Shape> {
        let shapes = selected_shapes(document, &self.selected);
        self.selected
            .retain(|node| shapes.iter().any(|shape| shape.node == node));
        // The cells chosen go with their table, and stay inside it.
        let single = self.selected.single();
        self.cells = self.cells.take().filter(|cells| {
            single == Some((cells.page, cells.node))
                && document
                    .and_then(|d| table_of(d, cells.page, cells.node))
                    .is_some_and(|(table, _)| {
                        table.cell(cells.from.0, cells.from.1).is_some()
                            && table.cell(cells.to.0, cells.to.1).is_some()
                    })
        });
        self.scope = self.scope.filter(|(page, node)| {
            document
                .and_then(|d| d.page(*page))
                .is_some_and(|page| group::is_group(page, *node))
        });
        shapes
    }

    /// The group entered on `page`, if any.
    fn scope_on(&self, page: PageId) -> Option<NodeId> {
        self.scope
            .filter(|(entered, _)| *entered == page)
            .map(|(_, node)| node)
    }

    pub(crate) fn show(
        &mut self,
        ctx: &egui::Context,
        engine: &mut Gafira,
        theme: &Theme,
        texts: &Texts,
        notice: Option<&str>,
        clipboard: &mut Option<Arc<Clip>>,
    ) -> Go {
        // The clipboard of the session lives in the shell, so a paste
        // reaches the next project too (spec 043). It is shared, so handing
        // it over every frame copies nothing.
        if clipboard.is_some() {
            self.clip.clone_from(clipboard);
        }
        let go = self.frame(ctx, engine, theme, texts, notice);
        if std::mem::take(&mut self.copied)
            && let Some(clip) = self.clip.as_mut()
        {
            let clip = Arc::make_mut(clip);
            clip.text =
                clipboard::plain_text(&clip.specs, |kind| plain(texts, canvas::kind_key(kind)));
            ctx.copy_text(clip.text.clone());
        }
        clipboard.clone_from(&self.clip);
        go
    }

    /// One frame of the editor: draw, then run what was asked.
    fn frame(
        &mut self,
        ctx: &egui::Context,
        engine: &mut Gafira,
        theme: &Theme,
        texts: &Texts,
        notice: Option<&str>,
    ) -> Go {
        let document = engine.edit().document();
        let (can_undo, can_redo) = engine.edit().can_undo_redo();
        let header = Header {
            can_undo,
            can_redo,
            modified: engine.projects().is_modified(),
            name: project_name(engine.projects().path().as_deref()),
            notice,
        };
        if !self.shown_loaded {
            self.shown = engine.settings().get().view;
            self.shown_loaded = true;
        }
        let mut asked = Asked::default();
        ctx.input(|i| {
            if i.modifiers.command && i.key_pressed(egui::Key::S) {
                asked.file = Some(if i.modifiers.shift {
                    FileAction::SaveAs
                } else {
                    FileAction::Save
                });
            }
            if i.modifiers.command && i.key_pressed(egui::Key::O) {
                asked.file = Some(FileAction::Open);
            }
            if i.modifiers.command && i.key_pressed(egui::Key::Z) {
                if i.modifiers.shift {
                    asked.redo = true;
                } else {
                    asked.undo = true;
                }
            }
            if i.modifiers.command && i.key_pressed(egui::Key::Y) {
                asked.redo = true;
            }
            for key in [
                egui::Key::Equals,
                egui::Key::Plus,
                egui::Key::Minus,
                egui::Key::Num0,
                egui::Key::Num1,
            ] {
                if i.key_pressed(key) {
                    asked.zoom = ZoomAsk::from_shortcut(i.modifiers.command, key).or(asked.zoom);
                }
            }
        });
        // A turned text maps its glyphs with the atlas size of the last
        // frame; if the atlas grew after that, draw once more (spec 041).
        if let Some(used) = self.font_atlas.take()
            && ctx.fonts(|f| f.font_image_size()) != used
        {
            ctx.request_repaint();
        }
        // Image files dropped from the system on the window (spec 067).
        ctx.input(|i| {
            asked.drop_files = i
                .raw
                .dropped_files
                .iter()
                .filter_map(|file| file.path.clone())
                .filter(|path| is_image_path(path) || is_vector_path(path))
                .collect();
            asked.drop_at = i.pointer.latest_pos();
        });
        let typing = ctx.wants_keyboard_input();
        // The cells chosen of a table take the keys (S81-08).
        let in_cells = !typing && self.cell_keys(ctx, document.as_deref(), &mut asked);
        if !typing && !in_cells {
            ctx.input(|i| {
                if i.key_pressed(egui::Key::Delete) || i.key_pressed(egui::Key::Backspace) {
                    asked.delete = true;
                }
                // Esc leaves the group entered, selecting it; outside one it
                // empties the selection (03 §9.1, spec 043).
                if i.key_pressed(egui::Key::Escape) {
                    if self.cropping.is_some() {
                        asked.end_crop = true;
                    } else if self.scope.is_some() {
                        asked.leave = true;
                    } else {
                        asked.select = Some(Selection::default());
                    }
                }
                if i.modifiers.command && i.key_pressed(egui::Key::A) {
                    asked.select_all = true;
                }
                // Ctrl+G groups and Ctrl+Shift+G ungroups; Ctrl+D duplicates
                // (03 §9.2, spec 043).
                if i.key_pressed(egui::Key::G) {
                    if i.modifiers.matches_exact(Modifiers::COMMAND) {
                        asked.group = true;
                    } else if i
                        .modifiers
                        .matches_exact(Modifiers::COMMAND.plus(Modifiers::SHIFT))
                    {
                        asked.ungroup = true;
                    }
                }
                if i.modifiers.matches_exact(Modifiers::COMMAND) && i.key_pressed(egui::Key::D) {
                    asked.duplicate = true;
                }
                // The window turns Ctrl+C, Ctrl+X and Ctrl+V into clipboard
                // events. A paste carries the system clipboard: it pastes the
                // clip only when that text is the one the copy put there;
                // other text is not pasted (spec 043).
                if i.modifiers.matches_exact(Modifiers::COMMAND) {
                    asked.copy |= i.key_pressed(egui::Key::C);
                    asked.cut |= i.key_pressed(egui::Key::X);
                    asked.paste |= i.key_pressed(egui::Key::V);
                }
                for event in &i.events {
                    if matches!(event, egui::Event::Paste(_)) {
                        asked.pasted_any = true;
                    }
                    // The window keeps Ctrl+V down for itself and reports
                    // only its release: with no text pasted, the clipboard
                    // may hold an image (spec 067).
                    if let egui::Event::Key {
                        key: egui::Key::V,
                        pressed: false,
                        modifiers,
                        ..
                    } = event
                        && modifiers.command
                    {
                        asked.v_released = true;
                    }
                    match event {
                        egui::Event::Copy => asked.copy = true,
                        egui::Event::Cut => asked.cut = true,
                        egui::Event::Paste(text)
                            if self.clip.as_ref().is_some_and(|clip| clip.text == *text) =>
                        {
                            asked.paste = true;
                        }
                        // Text from anywhere else becomes a text box (spec
                        // 061).
                        // Files copied in the file manager arrive as their
                        // paths: images are imported (spec 067).
                        egui::Event::Paste(text) if !image_paths(text).is_empty() => {
                            asked.paste_files = image_paths(text);
                        }
                        egui::Event::Paste(text) if !text.trim().is_empty() => {
                            asked.paste_text = Some(text.clone());
                        }
                        _ => {}
                    }
                }
                // Alt+A/H/D and Alt+W/V/S align (03 §9.2, D-88). Only Alt
                // exactly: AltGr is Ctrl+Alt on Windows and no Alt on Linux,
                // so a character typed with AltGr never aligns.
                if i.modifiers.matches_exact(Modifiers::ALT) {
                    for (action, _, _, keys) in ALIGNS {
                        if let Some(keys) = keys
                            && i.key_pressed(keys.logical_key)
                        {
                            asked.align = Some(action);
                        }
                    }
                }
                // Ctrl+] and Ctrl+[ one step, with Shift to the front or the
                // back. Shift+] reads as `}` on many layouts, so both count;
                // Alt is not looked at, because the brackets need AltGr
                // (Ctrl+Alt on Windows) on layouts such as Spanish.
                let close = i.key_pressed(egui::Key::CloseBracket)
                    || i.key_pressed(egui::Key::CloseCurlyBracket);
                let open = i.key_pressed(egui::Key::OpenBracket)
                    || i.key_pressed(egui::Key::OpenCurlyBracket);
                if i.modifiers.command {
                    let shift = i.modifiers.shift;
                    if close {
                        asked.order = Some(if shift { Order::Front } else { Order::Forward });
                    } else if open {
                        asked.order = Some(if shift { Order::Back } else { Order::Backward });
                    }
                }
                // Arrows nudge 1 px, with Shift 10 px (03 §9.2); a held key
                // repeats, and the steps within 500 ms are one undo step
                // (02 §4.3).
                if i.modifiers.matches_exact(Modifiers::NONE)
                    || i.modifiers.matches_exact(Modifiers::SHIFT)
                {
                    for key in [
                        egui::Key::ArrowLeft,
                        egui::Key::ArrowRight,
                        egui::Key::ArrowUp,
                        egui::Key::ArrowDown,
                    ] {
                        if i.key_pressed(key)
                            && let Some((dx, dy)) = magnet::nudge(key, i.modifiers.shift)
                        {
                            let (x, y) = asked.nudge.unwrap_or_default();
                            asked.nudge = Some((x + dx, y + dy));
                        }
                    }
                }
                // Ctrl+Shift+R rulers, Ctrl+; guides (Shift may be needed to
                // type `;`), Ctrl+Shift+' magnet (03 §9.2, FD-03, FD-06).
                let mut view = self.shown;
                if i.modifiers.command && i.modifiers.shift && i.key_pressed(egui::Key::R) {
                    view.rulers = !view.rulers;
                }
                if i.modifiers.command && !i.modifiers.alt && i.key_pressed(egui::Key::Semicolon) {
                    view.guides = !view.guides;
                }
                if i.modifiers.command && i.modifiers.shift && i.key_pressed(egui::Key::Quote) {
                    view.magnet = !view.magnet;
                }
                if view != self.shown {
                    asked.shown = Some(view);
                }
                // Ctrl+Shift+H hides and Ctrl+Shift+L locks the selection;
                // Alt+↑/↓ moves it one step in the stacking order (03 §11,
                // spec 048).
                if i.modifiers
                    .matches_exact(Modifiers::COMMAND.plus(Modifiers::SHIFT))
                {
                    if i.key_pressed(egui::Key::H) {
                        asked.toggle = Some(Flag::Hidden);
                    }
                    if i.key_pressed(egui::Key::L) {
                        asked.toggle = Some(Flag::Locked);
                    }
                }
                if i.modifiers.matches_exact(Modifiers::ALT) {
                    if i.key_pressed(egui::Key::ArrowUp) {
                        asked.step = Some(1);
                    }
                    if i.key_pressed(egui::Key::ArrowDown) {
                        asked.step = Some(-1);
                    }
                }
                if i.modifiers.matches_exact(Modifiers::SHIFT) {
                    if i.key_pressed(egui::Key::H) {
                        asked.flip = Some(Axis::Horizontal);
                    }
                    if i.key_pressed(egui::Key::V) {
                        asked.flip = Some(Axis::Vertical);
                    }
                }
            });
        }
        self.load_textures(ctx, engine, document.as_deref());
        let selection = self.prune(document.as_deref());
        if asked.duplicate {
            // «Logo copia» (FD-13), in the language of the interface.
            asked.copy_names = selection
                .iter()
                .filter_map(|shape| {
                    let page = document.as_deref()?.page(shape.page)?;
                    let name = page.node(shape.node)?.base.name.clone()?;
                    let copy = texts.t(&gafira_doc::msg!(
                        "editor.copy_name",
                        name = Text(name.clone())
                    ));
                    Some((name, copy))
                })
                .collect();
        }
        self.can = CanArrange::of(document.as_deref(), &self.selected);
        self.bars(ctx, theme, texts, &document, &header, &mut asked);
        if let Some(ask) = asked.zoom {
            self.view = ask.apply(self.zoom);
        }
        // «¿Cómo quieres importarlo?» for the SVG files dropped, pasted or
        // picked (spec 067).
        if let Some((files, _)) = &self.pending_svg {
            match crate::dialogs::svg_import(ctx, theme, texts, files) {
                crate::dialogs::SvgImport::Waiting => {}
                crate::dialogs::SvgImport::Image => asked.svg_as_image = Some(true),
                crate::dialogs::SvgImport::Frame => asked.svg_as_image = Some(false),
                crate::dialogs::SvgImport::Cancel => asked.svg_cancel = true,
            }
        }
        self.rail(ctx, theme, texts);
        if let Some(step) = self.side_panel(ctx, theme, texts, &mut asked) {
            self.rail.apply(step);
        }
        self.inspector(
            ctx,
            theme,
            texts,
            document.as_deref(),
            &selection,
            &mut asked,
        );
        self.canvas(ctx, theme, texts, document.as_deref(), &mut asked);
        match asked.table_menu {
            Some(tables::Menu::Duplicate) => asked.duplicate = true,
            Some(tables::Menu::Delete) => asked.delete = true,
            Some(tables::Menu::Lock) => asked.toggle = Some(Flag::Locked),
            Some(tables::Menu::Align(index)) => {
                asked.align = ALIGNS.get(index).map(|(action, ..)| *action);
            }
            Some(tables::Menu::Copy) => {
                if let Some((page, node)) = self.selected.single()
                    && let Some((table, _)) =
                        document.as_deref().and_then(|d| table_of(d, page, node))
                {
                    let range = self.cell_range(node, &table);
                    ctx.copy_text(gafira_table::copy(&table, range));
                }
            }
            _ => {}
        }
        // The controls of a table are at rest: the next change is another
        // undo step.
        asked.table_rest |= self.table_gesture.is_some()
            && self.table_drag.is_none()
            && asked.table_act.is_none()
            && asked.table_sized.is_none()
            && !ctx.input(|i| i.pointer.any_down())
            && !egui::Popup::is_any_open(ctx);
        if asked.versions {
            self.versions.open_list();
        }
        if asked.save_version {
            self.versions.ask_name();
        }
        asked.version = self.versions.show(ctx, engine, theme, texts);
        let field = |cells: &Option<tables::Cells>| {
            cells
                .as_ref()
                .filter(|c| c.writing.is_some())
                .map(|c| cell_id(c.node, c.from))
        };
        let before = field(&self.cells);
        let go = self.apply(engine, &document, &asked, theme);
        // The keyboard goes to the cell being written, and leaves the one
        // before, in this very frame: no letter typed next is lost.
        let now = field(&self.cells);
        // Tab goes from cell to cell, not to another control.
        let tabbed = ctx.input(|i| {
            i.raw.events.iter().any(|event| {
                matches!(
                    event,
                    egui::Event::Key {
                        key: egui::Key::Tab,
                        pressed: true,
                        ..
                    }
                )
            })
        });
        if tabbed && (in_cells || before.is_some()) {
            ctx.memory_mut(|m| {
                if let Some(id) = m.focused().filter(|id| Some(*id) != now) {
                    m.surrender_focus(id);
                }
            });
            ctx.request_repaint();
        }
        if let Some(old) = before.filter(|old| Some(*old) != now) {
            ctx.memory_mut(|m| m.surrender_focus(old));
            ctx.request_repaint();
        }
        if let Some(id) = now
            && let Some(writing) = self.cells.as_mut().and_then(|c| c.writing.as_mut())
            && !std::mem::replace(&mut writing.focused, true)
        {
            // What was typed to start goes on at its end.
            let mut state = egui::text_edit::TextEditState::load(ctx, id).unwrap_or_default();
            let end = egui::text::CCursor::new(writing.words.chars().count());
            state
                .cursor
                .set_char_range(Some(egui::text::CCursorRange::one(end)));
            state.store(ctx, id);
            ctx.memory_mut(|m| m.request_focus(id));
            ctx.request_repaint();
        }
        go
    }

    fn bars(
        &mut self,
        ctx: &egui::Context,
        theme: &Theme,
        texts: &Texts,
        document: &Option<Arc<Document>>,
        header: &Header<'_>,
        asked: &mut Asked,
    ) {
        let can = self.can;
        widgets::topbar(ctx, theme, "editor.topbar", |ui| {
            if IconButton::new(Icon::Home, texts.t(&gafira_doc::msg!("nav.home")))
                .on(Surface::Topbar)
                .show(ui, theme)
                .clicked()
            {
                asked.home = true;
            }
            ui.add_space(12.0);
            widgets::topbar_menu(ui, theme, texts.t(&gafira_doc::msg!("menu.file")), |ui| {
                ui.set_min_width(200.0);
                if ui.button(texts.t(&gafira_doc::msg!("file.new"))).clicked() {
                    asked.home = true;
                    ui.close();
                }
                for (key, action) in [
                    (gafira_doc::msg!("file.open"), FileAction::Open),
                    (gafira_doc::msg!("file.save"), FileAction::Save),
                    (gafira_doc::msg!("file.save_as"), FileAction::SaveAs),
                    (gafira_doc::msg!("file.save_copy"), FileAction::SaveCopy),
                ] {
                    if ui.button(texts.t(&key)).clicked() {
                        asked.file = Some(action);
                        ui.close();
                    }
                }
                if ui
                    .button(texts.t(&gafira_doc::msg!("file.save_version")))
                    .clicked()
                {
                    asked.save_version = true;
                    ui.close();
                }
                ui.separator();
                if ui
                    .button(texts.t(&gafira_doc::msg!("settings.title")))
                    .clicked()
                {
                    asked.settings = true;
                    ui.close();
                }
            });
            widgets::topbar_menu(ui, theme, texts.t(&gafira_doc::msg!("menu.edit")), |ui| {
                ui.set_min_width(200.0);
                let undo = egui::Button::new(texts.t(&gafira_doc::msg!("editor.undo")))
                    .shortcut_text(keys_text(ui.ctx(), &UNDO));
                if ui.add_enabled(header.can_undo, undo).clicked() {
                    asked.undo = true;
                    ui.close();
                }
                let redo = egui::Button::new(texts.t(&gafira_doc::msg!("editor.redo")))
                    .shortcut_text(keys_text(ui.ctx(), &REDO));
                if ui.add_enabled(header.can_redo, redo).clicked() {
                    asked.redo = true;
                    ui.close();
                }
                ui.separator();
                // Cortar, Copiar, Pegar and Duplicar (03 §9.2, spec 043).
                let some = !self.selected.is_empty();
                let paste = self.clip.is_some() && document.is_some();
                for (key, keys, enabled, flag) in [
                    ("editor.cut", CUT, some, &mut asked.cut),
                    ("editor.copy", COPY, some, &mut asked.copy),
                    ("editor.paste", PASTE, paste, &mut asked.paste),
                    ("editor.duplicate", DUPLICATE, some, &mut asked.duplicate),
                ] {
                    let item = egui::Button::new(plain(texts, key))
                        .shortcut_text(keys_text(ui.ctx(), &keys));
                    if ui.add_enabled(enabled, item).clicked() {
                        *flag = true;
                        ui.close();
                    }
                }
                let delete = egui::Button::new(texts.t(&gafira_doc::msg!("editor.delete")));
                if ui.add_enabled(!self.selected.is_empty(), delete).clicked() {
                    asked.delete = true;
                    ui.close();
                }
                let all = egui::Button::new(texts.t(&gafira_doc::msg!("editor.select_all")))
                    .shortcut_text(keys_text(ui.ctx(), &SELECT_ALL));
                if ui.add_enabled(document.is_some(), all).clicked() {
                    asked.select_all = true;
                    ui.close();
                }
                ui.separator();
                let some = !self.selected.is_empty();
                for (key, turn) in [
                    (gafira_doc::msg!("editor.rotate_left"), -90.0),
                    (gafira_doc::msg!("editor.rotate_right"), 90.0),
                ] {
                    if ui
                        .add_enabled(some, egui::Button::new(texts.t(&key)))
                        .clicked()
                    {
                        asked.turn = Some(turn);
                        ui.close();
                    }
                }
                for (key, axis, keys) in [
                    (
                        gafira_doc::msg!("editor.flip_horizontal"),
                        Axis::Horizontal,
                        FLIP_H,
                    ),
                    (
                        gafira_doc::msg!("editor.flip_vertical"),
                        Axis::Vertical,
                        FLIP_V,
                    ),
                ] {
                    let flip =
                        egui::Button::new(texts.t(&key)).shortcut_text(keys_text(ui.ctx(), &keys));
                    if ui.add_enabled(some, flip).clicked() {
                        asked.flip = Some(axis);
                        ui.close();
                    }
                }
                ui.separator();
                // Agrupar and Desagrupar (03 §9.2 «Capas», spec 043).
                for (key, keys, enabled, flag) in [
                    ("editor.group", GROUP, can.group, &mut asked.group),
                    ("editor.ungroup", UNGROUP, can.ungroup, &mut asked.ungroup),
                ] {
                    let item = egui::Button::new(plain(texts, key))
                        .shortcut_text(keys_text(ui.ctx(), &keys));
                    if ui.add_enabled(enabled, item).clicked() {
                        *flag = true;
                        ui.close();
                    }
                }
                arrange_menu(ui, texts, can, asked);
            });
            widgets::topbar_menu(ui, theme, texts.t(&gafira_doc::msg!("menu.view")), |ui| {
                ui.set_min_width(200.0);
                // Reglas, Guías and Imán (03 §2.2, spec 044).
                let now = self.shown;
                for (key, keys, on, flip) in [
                    (
                        "view.rulers",
                        RULERS,
                        now.rulers,
                        ViewSettings::new(!now.rulers, now.guides, now.magnet),
                    ),
                    (
                        "view.guides",
                        GUIDES,
                        now.guides,
                        ViewSettings::new(now.rulers, !now.guides, now.magnet),
                    ),
                    (
                        "view.magnet",
                        MAGNET,
                        now.magnet,
                        ViewSettings::new(now.rulers, now.guides, !now.magnet),
                    ),
                ] {
                    let item = egui::Button::selectable(on, plain(texts, key))
                        .shortcut_text(keys_text(ui.ctx(), &keys));
                    if ui.add(item).clicked() {
                        asked.shown = Some(flip);
                        ui.close();
                    }
                }
                ui.separator();
                let mut dark = theme.dark;
                if ui
                    .checkbox(&mut dark, texts.t(&gafira_doc::msg!("view.dark_mode")))
                    .changed()
                {
                    asked.dark = Some(dark);
                    ui.close();
                }
            });
            widgets::on_topbar(ui, theme, texts.t(&gafira_doc::msg!("menu.size")), false);
            ui.add_space(24.0);
            ui.label(
                RichText::new(
                    header
                        .name
                        .clone()
                        .unwrap_or_else(|| texts.t(&gafira_doc::msg!("editor.untitled"))),
                )
                .font(theme.font("ui-label"))
                .color(theme.c("topbar.fg")),
            );
            ui.add_space(16.0);
            for (icon, key, shortcut, enabled, flag) in [
                (
                    Icon::Undo,
                    gafira_doc::msg!("editor.undo"),
                    UNDO,
                    header.can_undo,
                    &mut asked.undo,
                ),
                (
                    Icon::Redo,
                    gafira_doc::msg!("editor.redo"),
                    REDO,
                    header.can_redo,
                    &mut asked.redo,
                ),
            ] {
                let name = texts.t(&key);
                let tooltip = with_keys(ui.ctx(), texts, name.clone(), shortcut);
                let button = ui.add_enabled_ui(enabled, |ui| {
                    IconButton::new(icon, name)
                        .on(Surface::Topbar)
                        .tooltip(tooltip)
                        .show(ui, theme)
                });
                *flag |= button.inner.clicked();
            }
            ui.add_space(16.0);
            let status = if header.modified {
                gafira_doc::msg!("editor.unsaved")
            } else {
                gafira_doc::msg!("editor.saved")
            };
            let (text, color) = match header.notice {
                Some(notice) => (notice.to_owned(), theme.c("primary.bg")),
                None => (texts.t(&status), theme.c("topbar.fg.muted")),
            };
            ui.label(RichText::new(text).font(theme.font("caption")).color(color));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                asked.export |= widgets::primary_icon(
                    ui,
                    theme,
                    Icon::Export,
                    texts.t(&gafira_doc::msg!("editor.export")),
                )
                .clicked();
                ui.add_space(8.0);
                asked.versions |= widgets::on_topbar_icon(
                    ui,
                    theme,
                    Icon::Versions,
                    texts.t(&gafira_doc::msg!("editor.versions")),
                )
                .clicked();
            });
        });
        egui::TopBottomPanel::bottom("editor.bottombar")
            .exact_height(px(theme.tokens.size.bottombar))
            .frame(
                egui::Frame::NONE
                    .fill(theme.c("bg"))
                    .stroke(Stroke::new(1.0, theme.c("border")))
                    .inner_margin(egui::Margin::symmetric(12, 0)),
            )
            .show(ctx, |ui| {
                // 03 §7.3: Notas on the left; on the right the magnet, the
                // zoom, and Páginas with «n de N» (spec 049). Añadir página
                // is under the last page (spec 077).
                let label = |text: String| RichText::new(text).font(theme.font("ui-label"));
                ui.horizontal_centered(|ui| {
                    // Notas is not a function yet (FD-02): shown, muted.
                    bar_icon(ui, theme, Icon::Notes, theme.c("text.disabled"));
                    ui.label(
                        label(texts.t(&gafira_doc::msg!("bottom.notes")))
                            .color(theme.c("text.disabled")),
                    );
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let ids: Vec<PageId> = document
                            .as_ref()
                            .map(|d| d.pages().iter().map(|page| page.id).collect())
                            .unwrap_or_default();
                        let current = document
                            .as_deref()
                            .and_then(|d| self.current_page(d))
                            .map(|page| page.id);
                        ui.label(label(texts.t(&gafira_doc::msg!(
                            "editor.page_of",
                            current = Count(page_number(&ids, current) as u64),
                            total = Count(ids.len() as u64)
                        ))));
                        ui.add_space(8.0);
                        ui.label(label(texts.t(&gafira_doc::msg!("bottom.pages"))));
                        bar_icon(ui, theme, Icon::Pages, theme.c("text"));
                        bar_separator(ui, theme);
                        asked.zoom = self.zoom_controls(ui, theme, texts).or(asked.zoom);
                        bar_separator(ui, theme);
                        // Imán on or off (03 §2.6, FD-06, spec 044).
                        let name = plain(texts, "view.magnet");
                        let tooltip = with_keys(ui.ctx(), texts, name.clone(), MAGNET);
                        let now = self.shown;
                        if IconButton::new(Icon::Magnet, name)
                            .active(now.magnet)
                            .tooltip(tooltip)
                            .small()
                            .show(ui, theme)
                            .clicked()
                        {
                            asked.shown =
                                Some(ViewSettings::new(now.rulers, now.guides, !now.magnet));
                        }
                    });
                });
            });
    }

    /// Alejar, Acercar and the zoom menu (Ajustar, 50, 100, 200 %) of the
    /// bottom bar (03 §7.3), laid out right to left.
    fn zoom_controls(&self, ui: &mut egui::Ui, theme: &Theme, texts: &Texts) -> Option<ZoomAsk> {
        let mut ask = None;
        let percent = |scale: f32| {
            texts.t(&gafira_doc::msg!(
                "format.percent",
                value = Text(format!("{:.0}", scale * 100.0))
            ))
        };
        let caret = icons::image(
            ui.ctx(),
            Icon::Expand,
            Weight::Regular,
            px(theme.tokens.size.icon.sm),
            theme.c("text"),
        );
        let label = RichText::new(percent(self.zoom)).font(theme.font("ui-label"));
        let menu = ui.menu_button((label, caret), |ui| {
            ui.set_min_width(120.0);
            let fit = ui.selectable_label(
                self.view == Zoom::Fit,
                texts.t(&gafira_doc::msg!("zoom.fit")),
            );
            if fit
                .on_hover_text(with_keys(
                    ui.ctx(),
                    texts,
                    texts.t(&gafira_doc::msg!("zoom.fit")),
                    ZOOM_FIT,
                ))
                .clicked()
            {
                ask = Some(ZoomAsk::Fit);
                ui.close();
            }
            for scale in zoom::PRESETS {
                if ui
                    .selectable_label(self.view == Zoom::Fixed(scale), percent(scale))
                    .clicked()
                {
                    ask = Some(ZoomAsk::To(scale));
                    ui.close();
                }
            }
        });
        // Named «Zoom», with the scale on display as its value.
        let name = texts.t(&gafira_doc::msg!("zoom.menu"));
        let value = percent(self.zoom);
        menu.response.widget_info(|| egui::WidgetInfo {
            current_text_value: Some(value.clone()),
            ..egui::WidgetInfo::labeled(egui::WidgetType::ComboBox, true, &name)
        });
        // Right to left: Acercar, the slider, Alejar (03 §7.3, FD-08).
        for (icon, key, shortcut, step) in [
            (
                Icon::ZoomIn,
                gafira_doc::msg!("zoom.in"),
                ZOOM_IN,
                ZoomAsk::In,
            ),
            (
                Icon::ZoomOut,
                gafira_doc::msg!("zoom.out"),
                ZOOM_OUT,
                ZoomAsk::Out,
            ),
        ] {
            if step == ZoomAsk::Out {
                // 5 to 3200 %, logarithmic, so each doubling is as long.
                let mut value = self.zoom;
                ui.spacing_mut().slider_width = 120.0;
                let slider = ui.add(
                    egui::Slider::new(&mut value, zoom::MIN..=zoom::MAX)
                        .logarithmic(true)
                        .show_value(false),
                );
                slider.widget_info(|| {
                    egui::WidgetInfo::slider(true, f64::from(value), name.as_str())
                });
                if slider.changed() {
                    ask = Some(ZoomAsk::To(value));
                }
            }
            let name = texts.t(&key);
            let tooltip = with_keys(ui.ctx(), texts, name.clone(), shortcut);
            let limit = match step {
                ZoomAsk::In => self.zoom < zoom::MAX,
                _ => self.zoom > zoom::MIN,
            };
            let button = ui.add_enabled_ui(limit, |ui| {
                IconButton::new(icon, name)
                    .tooltip(tooltip)
                    .small()
                    .show(ui, theme)
            });
            if button.inner.clicked() {
                ask = Some(step);
            }
        }
        ask
    }

    fn rail(&mut self, ctx: &egui::Context, theme: &Theme, texts: &Texts) {
        egui::SidePanel::left("editor.rail")
            .exact_width(px(theme.tokens.size.rail))
            .resizable(false)
            .frame(egui::Frame::NONE.fill(theme.c("topbar.bg")))
            .show(ctx, |ui| {
                ui.add_space(8.0);
                for (index, (key, icon)) in TABS.iter().enumerate() {
                    let label = texts.t(&gafira_doc::msg::Message {
                        key: gafira_doc::msg::MessageKey(key),
                        args: Vec::new(),
                    });
                    let active = index == self.rail.tab;
                    let response = widgets::rail_tab(ui, theme, *icon, &label, active);
                    if response.clicked() {
                        // The active tab hides or shows the panel (S64-06).
                        self.rail.click_tab(index);
                    }
                }
            });
    }

    /// The side panel; the move it asks inside Elementos, if any (spec 064).
    fn side_panel(
        &self,
        ctx: &egui::Context,
        theme: &Theme,
        texts: &Texts,
        asked: &mut Asked,
    ) -> Option<Move> {
        // Hidden from the rail (S64-06); «Fuente» shows all the same.
        if !self.rail.open && !self.font_panel {
            return None;
        }
        let mut go = None;
        egui::SidePanel::left("editor.side")
            .default_width(px(theme.tokens.size.panel_left))
            .width_range(280.0..=480.0)
            .frame(
                egui::Frame::NONE
                    .fill(theme.c("bg"))
                    .stroke(Stroke::new(1.0, theme.c("border")))
                    .inner_margin(egui::Margin::same(16)),
            )
            .show(ctx, |ui| {
                if self.font_panel {
                    self.fonts_panel(ui, theme, texts, asked);
                    return;
                }
                if TABS[self.rail.tab].0 == "tab.library" {
                    self.library_panel(ui, theme, texts, asked);
                    return;
                }
                if TABS[self.rail.tab].0 == "tab.text" {
                    // The whole panel scrolls: the pairings go past the
                    // bottom of the window.
                    egui::ScrollArea::vertical()
                        .id_salt("editor.text_panel")
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            section_title(ui, theme, texts, "text.section_add");
                            for (key, words, preset) in [
                                (
                                    gafira_doc::msg!("text.add_title"),
                                    gafira_doc::msg!("text.title"),
                                    canvas::TextPreset::Title,
                                ),
                                (
                                    gafira_doc::msg!("text.add_subtitle"),
                                    gafira_doc::msg!("text.subtitle"),
                                    canvas::TextPreset::Subtitle,
                                ),
                                (
                                    gafira_doc::msg!("text.add_body"),
                                    gafira_doc::msg!("text.body"),
                                    canvas::TextPreset::Body,
                                ),
                            ] {
                                let (size, display) = preset.size_and_display();
                                let font = if display {
                                    egui::FontId::new(
                                        size as f32 * 0.4,
                                        egui::FontFamily::Name(crate::style::DISPLAY.into()),
                                    )
                                } else {
                                    egui::FontId::proportional(size as f32 * 0.5)
                                };
                                let button = egui::Button::new(
                                    RichText::new(texts.t(&key)).font(font.clone()),
                                )
                                .min_size(Vec2::new(ui.available_width(), 48.0));
                                // A click adds it to the middle of the page; a drag
                                // carries it to where it is dropped (spec 061).
                                let button = ui.add(button).interact(egui::Sense::click_and_drag());
                                // A hand over it, a closed hand while it is carried.
                                if button.hovered() && !button.dragged() {
                                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                                }
                                if button.clicked() {
                                    asked.insert_text = Some((preset, texts.t(&words)));
                                }
                                let at = ui.input(|i| i.pointer.latest_pos());
                                if button.dragged()
                                    && let Some(at) = at
                                {
                                    ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
                                    let painter = ui.ctx().layer_painter(egui::LayerId::new(
                                        egui::Order::Tooltip,
                                        egui::Id::new("editor.text_ghost"),
                                    ));
                                    painter.text(
                                        at,
                                        Align2::CENTER_CENTER,
                                        texts.t(&key),
                                        font.clone(),
                                        theme.c("text").gamma_multiply(0.5),
                                    );
                                }
                                if button.drag_stopped()
                                    && let Some(at) = at
                                    && let Some((page, point)) =
                                        canvas::drop_point(&self.page_rects, self.zoom, at)
                                {
                                    asked.drop_text = Some((preset, texts.t(&words), page, point));
                                }
                                ui.add_space(8.0);
                            }
                            self.text_extras(ui, theme, texts, asked);
                        });
                    return;
                }
                if TABS[self.rail.tab].0 == "tab.pieces" {
                    // Elementos: the grid of categories, or the one open
                    // with its «Volver» arrow (spec 064).
                    let Some(category) = self.rail.category else {
                        go = elements::grid(ui, theme, texts).map(Move::Enter);
                        return;
                    };
                    if elements::header(ui, theme, texts, category) {
                        go = Some(Move::Back);
                    }
                    if category == Category::Collage {
                        // S66-09: a click fills the current page with the
                        // layout, a drag the page it is dropped on (D-105).
                        egui::ScrollArea::vertical()
                            .id_salt("editor.collage")
                            .auto_shrink([false, false])
                            .show(ui, |ui| {
                                ui.label(
                                    RichText::new(plain(texts, "collage.hint"))
                                        .font(theme.font("body-sm"))
                                        .color(theme.c("text.secondary")),
                                );
                                ui.add_space(8.0);
                                let side = ((ui.available_width() - 2.0 * 8.0) / 3.0).max(40.0);
                                // The sample photo of the frames in every cell.
                                let sample = canvas::sample_photo().and_then(|image| {
                                    let size =
                                        Vec2::new(image.size[0] as f32, image.size[1] as f32);
                                    self.path_texture(ui.ctx(), 0x5A3_91E, || {
                                        Some((*image).clone())
                                    })
                                    .map(|texture| (texture, size))
                                });
                                let layouts: Vec<(usize, &str)> = crate::collage::LAYOUTS
                                    .iter()
                                    .copied()
                                    .enumerate()
                                    .collect();
                                for line in layouts.chunks(3) {
                                    ui.horizontal(|ui| {
                                        ui.spacing_mut().item_spacing.x = 8.0;
                                        for (index, layout) in line {
                                            let (rect, card) = ui.allocate_exact_size(
                                                Vec2::splat(side),
                                                egui::Sense::click_and_drag(),
                                            );
                                            crate::collage::paint(
                                                ui.painter(),
                                                rect,
                                                layout,
                                                sample,
                                            );
                                            if card.hovered() && !card.dragged() {
                                                ui.ctx().set_cursor_icon(
                                                    egui::CursorIcon::PointingHand,
                                                );
                                                ui.painter().rect_stroke(
                                                    rect.expand(2.0),
                                                    4.0,
                                                    Stroke::new(2.0, theme.c("primary.bg")),
                                                    egui::StrokeKind::Outside,
                                                );
                                            }
                                            if card.clicked() {
                                                asked.insert_collage = Some((*index, None));
                                            }
                                            let at = ui.input(|i| i.pointer.latest_pos());
                                            if card.dragged()
                                                && let Some(at) = at
                                            {
                                                ui.ctx()
                                                    .set_cursor_icon(egui::CursorIcon::Grabbing);
                                                let ghost =
                                                    ui.ctx().layer_painter(egui::LayerId::new(
                                                        egui::Order::Tooltip,
                                                        egui::Id::new("collage.ghost"),
                                                    ));
                                                crate::collage::paint(
                                                    &ghost,
                                                    Rect::from_center_size(at, Vec2::splat(80.0)),
                                                    layout,
                                                    sample,
                                                );
                                            }
                                            if card.drag_stopped()
                                                && let Some(at) = at
                                                && let Some((page, _)) = canvas::drop_point(
                                                    &self.page_rects,
                                                    self.zoom,
                                                    at,
                                                )
                                            {
                                                asked.insert_collage = Some((*index, Some(page)));
                                            }
                                        }
                                    });
                                    ui.add_space(8.0);
                                }
                            });
                        return;
                    }
                    if category == Category::Illustrations {
                        // Spec 072: a click inserts the drawing as vector
                        // shapes, a drag carries it (D-105).
                        let rects = self.page_rects.clone();
                        let zoom = self.zoom;
                        let can_drop =
                            move |at: Pos2| canvas::drop_point(&rects, zoom, at).is_some();
                        egui::ScrollArea::vertical()
                            .id_salt("editor.illustrations")
                            .auto_shrink([false, false])
                            .show(ui, |ui| {
                                let mut panel = self.illustration_panel.borrow_mut();
                                if !panel.available() {
                                    ui.label(plain(texts, "photos.unavailable"));
                                    return;
                                }
                                panel.pump();
                                match panel.show(ui, theme, texts, &can_drop) {
                                    Some(crate::illustrations::Chosen::Drawing {
                                        illustration,
                                        svg,
                                        at,
                                    }) => {
                                        let credit = panel.credit(&illustration);
                                        asked.insert_illustration = Some((svg, credit, at));
                                    }
                                    // A cut-out comes in as an image, through
                                    // the downloads of Fotos (spec 071).
                                    Some(crate::illustrations::Chosen::Cutout { photo, at }) => {
                                        self.photo_panel.borrow_mut().fetch(photo, at);
                                    }
                                    None => {}
                                }
                            });
                        return;
                    }
                    if category == Category::Photos {
                        // Spec 071: a click inserts the photo, a drag
                        // carries it to the page or into a frame (D-105).
                        let rects = self.page_rects.clone();
                        let zoom = self.zoom;
                        let can_drop =
                            move |at: Pos2| canvas::drop_point(&rects, zoom, at).is_some();
                        egui::ScrollArea::vertical()
                            .id_salt("editor.photos")
                            .auto_shrink([false, false])
                            .show(ui, |ui| {
                                let mut panel = self.photo_panel.borrow_mut();
                                if !panel.available() {
                                    ui.label(plain(texts, "photos.unavailable"));
                                    return;
                                }
                                let wanted = panel.show(ui, theme, texts, &can_drop);
                                if let Some((photo, at)) = wanted.insert {
                                    panel.fetch(photo, at);
                                }
                            });
                        return;
                    }
                    if category == Category::Lights {
                        // A click adds the light to the middle of the
                        // page; a drag carries it to where it is dropped,
                        // as every element of the panel (D-105).
                        if let Some((item, card)) = elements::lights(ui, theme, texts) {
                            if card.clicked() {
                                asked.insert_effect = Some(item);
                            }
                            let at = ui.input(|i| i.pointer.latest_pos());
                            if card.dragged()
                                && let Some(at) = at
                            {
                                ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
                                self.light_ghost(ui.ctx(), theme, item, at);
                            }
                            if card.drag_stopped()
                                && let Some(at) = at
                                && let Some((page, point)) =
                                    canvas::drop_point(&self.page_rects, self.zoom, at)
                            {
                                asked.drop_effect = Some((item, page, point));
                            }
                        }
                        return;
                    }
                    if category == Category::Tables {
                        // Spec 081: a table is added with a click or a drag
                        // (D-105).
                        let rects = self.page_rects.clone();
                        let zoom = self.zoom;
                        let over = move |at: Pos2| canvas::drop_point(&rects, zoom, at).is_some();
                        egui::ScrollArea::vertical()
                            .id_salt("editor.tables")
                            .auto_shrink([false, false])
                            .show(ui, |ui| {
                                ui.set_max_width(ui.available_width() - 12.0);
                                let mut panel = self.table_panel.borrow_mut();
                                asked.insert_table = panel.show(ui, theme, texts, &over);
                            });
                        return;
                    }
                    if category == Category::QrCodes {
                        // Spec 080: the code is added with a click or
                        // dragged by its preview (D-105).
                        let rects = self.page_rects.clone();
                        let zoom = self.zoom;
                        let over = move |at: Pos2| canvas::drop_point(&rects, zoom, at).is_some();
                        egui::ScrollArea::vertical()
                            .id_salt("editor.qr")
                            .auto_shrink([false, false])
                            .show(ui, |ui| {
                                // Room for the scroll bar, beside the fields.
                                ui.set_max_width(ui.available_width() - 12.0);
                                let mut panel = self.qr_panel.borrow_mut();
                                let logo = panel
                                    .draft
                                    .logo
                                    .and_then(|l| self.photos.get(&l.asset.as_hash()))
                                    .and_then(Option::as_deref);
                                match panel.show(ui, theme, texts, logo, &over) {
                                    Some(crate::qr::Chosen::Insert(code, at)) => {
                                        asked.insert_qr = Some((code, at));
                                    }
                                    Some(crate::qr::Chosen::PickLogo) => {
                                        asked.qr_panel_logo = true;
                                    }
                                    None => {}
                                }
                            });
                        return;
                    }
                    if category == Category::Frames {
                        self.frames_panel(ui, theme, texts, asked);
                        return;
                    }
                    if category != Category::Shapes {
                        elements::soon(ui, theme, texts);
                        return;
                    }
                    // The shapes by section, each a tile that draws it
                    // (spec 053): a click adds it to the middle of the
                    // page, a drag carries it to where it is dropped
                    // (spec 052).
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        for (title, pieces) in canvas::PIECE_SECTIONS {
                            ui.label(
                                RichText::new(plain(texts, title))
                                    .font(theme.font("ui-label"))
                                    .color(theme.c("text")),
                            );
                            ui.add_space(8.0);
                            ui.horizontal_wrapped(|ui| {
                                ui.spacing_mut().item_spacing = Vec2::splat(4.0);
                                for piece in *pieces {
                                    let tile = self.piece_tile(
                                        ui,
                                        theme,
                                        *piece,
                                        plain(texts, piece.key()),
                                    );
                                    if tile.clicked() {
                                        asked.insert = Some(*piece);
                                    }
                                    let at = ui.input(|i| i.pointer.latest_pos());
                                    if tile.dragged()
                                        && let Some(at) = at
                                    {
                                        ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
                                        self.piece_ghost(ui.ctx(), theme, *piece, at);
                                    }
                                    if tile.drag_stopped()
                                        && let Some(at) = at
                                        && let Some((page, point)) =
                                            canvas::drop_point(&self.page_rects, self.zoom, at)
                                    {
                                        asked.drop_piece = Some((*piece, page, point));
                                    }
                                }
                            });
                            ui.add_space(16.0);
                        }
                    });
                    return;
                }
                if TABS[self.rail.tab].0 == "tab.drawing" {
                    crate::draw::panel(ui, theme, texts, &mut self.draw.borrow_mut());
                    return;
                }
                let mut query = String::new();
                ui.add(widgets::field(
                    &mut query,
                    texts.t(&gafira_doc::msg!("editor.search_templates")),
                ));
                ui.add_space(16.0);
                let card = Vec2::new((ui.available_width() - 12.0) / 2.0, 120.0);
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = Vec2::splat(12.0);
                    for _ in 0..6 {
                        let (rect, _) = ui.allocate_exact_size(card, egui::Sense::hover());
                        ui.painter().rect(
                            rect,
                            theme.radius_md(),
                            theme.c("surface"),
                            Stroke::new(1.0, theme.c("border")),
                            egui::StrokeKind::Inside,
                        );
                    }
                });
            });
        go
    }

    fn inspector(
        &mut self,
        ctx: &egui::Context,
        theme: &Theme,
        texts: &Texts,
        document: Option<&Document>,
        selection: &[Shape],
        asked: &mut Asked,
    ) {
        let can = self.can;
        // What the Fuente panel shows as current and as the document's.
        self.selection_font = selection.first().and_then(|shape| {
            let style = shape
                .text
                .as_ref()
                .and_then(|t| t.content.first_style())
                .or_else(|| shape.inner.as_ref().and_then(|t| t.content.first_style()))
                .cloned()
                .or_else(|| {
                    let table = shape.table.as_ref()?;
                    let range = self.cell_range(shape.node, table);
                    Some(span_of(&table.cell(range.r0, range.c0)?.style))
                })?;
            Some((style.font.clone(), style.bold, style.italic))
        });
        self.document_fonts = document.map(document_fonts).unwrap_or_default();
        self.load_system_fonts(ctx);
        egui::SidePanel::right("editor.inspector")
            .exact_width(px(theme.tokens.size.panel_right))
            .resizable(false)
            .frame(
                egui::Frame::NONE
                    .fill(theme.c("surface"))
                    .stroke(Stroke::new(1.0, theme.c("border")))
                    .inner_margin(egui::Margin::same(16)),
            )
            .show(ctx, |ui| {
                // Crop mode takes the Inspector (S66-07).
                if let Some(crop) = self.cropping
                    && let Some(shape) = selection.iter().find(|s| s.node == crop.node)
                    && let Some(frame) = &shape.frame
                {
                    crop_fields(ui, theme, texts, frame, asked);
                    return;
                }
                // «Propiedades» of what is selected (or of the page), and
                // «Capas».
                ui.horizontal(|ui| {
                    for (index, key) in [
                        gafira_doc::msg!("inspector.properties"),
                        gafira_doc::msg!("inspector.layers"),
                    ]
                    .iter()
                    .enumerate()
                    {
                        let on = index == self.inspector_tab;
                        let color = if on {
                            theme.c("text")
                        } else {
                            theme.c("text.muted")
                        };
                        let tab = egui::Button::new(
                            RichText::new(texts.t(key))
                                .font(theme.font("ui-label"))
                                .color(color),
                        )
                        .frame(false);
                        let response = ui.add(tab);
                        if on {
                            let r = response.rect;
                            ui.painter().line_segment(
                                [r.left_bottom(), r.right_bottom()],
                                Stroke::new(2.0, theme.c("primary.bg")),
                            );
                        }
                        if response.clicked() {
                            self.inspector_tab = index;
                        }
                        ui.add_space(8.0);
                    }
                });
                ui.add_space(24.0);
                if self.inspector_tab == 1 {
                    self.layers(ui, theme, texts, document, asked);
                    return;
                }
                let shape = match selection {
                    [] => {
                        // The page or the canvas: its background (spec 069).
                        let page = document.and_then(|d| {
                            self.active
                                .and_then(|id| d.page(id))
                                .or_else(|| d.pages().first().map(|p| &**p))
                        });
                        match page {
                            Some(page) => {
                                egui::ScrollArea::vertical().show(ui, |ui| {
                                    self.background_fields(ui, theme, texts, page, asked);
                                });
                            }
                            None => {
                                ui.label(
                                    RichText::new(texts.t(&gafira_doc::msg!("inspector.empty")))
                                        .font(theme.font("body-sm"))
                                        .color(theme.c("text.muted")),
                                );
                            }
                        }
                        return;
                    }
                    [shape] => shape,
                    many => {
                        many_fields(ui, theme, texts, many, can, asked);
                        return;
                    }
                };
                // What is selected, as the title of «Propiedades».
                ui.label(
                    RichText::new(plain(texts, kind_title(shape)))
                        .font(theme.font("ui-label"))
                        .color(theme.c("text")),
                );
                ui.add_space(8.0);
                // A table: the looks of its cells, then its box (spec 081).
                if let Some(table) = &shape.table {
                    let range = self.cell_range(shape.node, table);
                    egui::ScrollArea::vertical()
                        .id_salt("inspector.table")
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            ui.set_max_width(ui.available_width() - 12.0);
                            let style = table
                                .cell(range.r0, range.c0)
                                .map(|cell| span_of(&cell.style));
                            font_button(ui, theme, texts, style.as_ref(), asked);
                            ui.add_space(8.0);
                            let mut pen = self.table_pen.get();
                            if let Some(act) =
                                tables::inspector(ui, theme, texts, (table, range), &mut pen)
                            {
                                asked.table_act = Some(act);
                            }
                            self.table_pen.set(pen);
                            ui.add_space(16.0);
                            position_fields(ui, theme, texts, shape, asked);
                            ui.add_space(8.0);
                            turn_buttons(ui, theme, texts, asked);
                            ui.add_space(16.0);
                            arrange_buttons(ui, theme, texts, can, asked);
                            ui.add_space(8.0);
                            opacity_field(ui, texts, shape, asked);
                        });
                    return;
                }
                // A QR code: its own fields, then its box (spec 080).
                if let Some(qr) = &shape.qr {
                    egui::ScrollArea::vertical()
                        .id_salt("inspector.qr")
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            // Room for the scroll bar, beside the fields.
                            ui.set_max_width(ui.available_width() - 12.0);
                            let mut panel = self.qr_panel.borrow_mut();
                            let (set, pick) = panel.inspector(ui, theme, texts, (shape.node, qr));
                            asked.qr = set;
                            asked.qr_logo = pick;
                            ui.add_space(8.0);
                            if ui.button(plain(texts, "qr.to_shapes")).clicked() {
                                asked.qr_to_shapes = true;
                            }
                            ui.add_space(16.0);
                            position_fields(ui, theme, texts, shape, asked);
                            ui.add_space(8.0);
                            turn_buttons(ui, theme, texts, asked);
                            ui.add_space(16.0);
                            arrange_buttons(ui, theme, texts, can, asked);
                            ui.add_space(8.0);
                            opacity_field(ui, texts, shape, asked);
                        });
                    asked.qr_rest = ctx.memory(|m| m.focused().is_none())
                        && !ctx.input(|i| i.pointer.any_down())
                        && !egui::Popup::is_any_open(ctx);
                    return;
                }
                // An image: its file, and «Usar como fondo de la página».
                if let Some(asset) = shape.image {
                    let name = document
                        .and_then(|d| d.asset(&asset))
                        .map(|entry| entry.name.clone())
                        .unwrap_or_default();
                    if !name.is_empty() {
                        ui.label(
                            RichText::new(name)
                                .font(theme.font("body-sm"))
                                .color(theme.c("text.secondary")),
                        );
                        ui.add_space(6.0);
                    }
                    if ui.button(plain(texts, "inspector.as_background")).clicked() {
                        asked.image_to_background = Some((shape.page, shape.node, asset));
                    }
                    ui.add_space(12.0);
                }
                if let Some(text) = &shape.text {
                    font_button(ui, theme, texts, text.content.first_style(), asked);
                    let design = document
                        .and_then(|d| d.page(shape.page))
                        .map(design_colors)
                        .unwrap_or_default();
                    text_fields(ui, theme, texts, (text, &design), asked);
                    ui.add_space(16.0);
                }
                position_fields(ui, theme, texts, shape, asked);
                ui.add_space(8.0);
                turn_buttons(ui, theme, texts, asked);
                ui.add_space(16.0);
                arrange_buttons(ui, theme, texts, can, asked);
                ui.add_space(16.0);
                // A light shows its own settings instead of the fill,
                // outline and text (spec 065).
                let plain_shape = shape.text.is_none()
                    && shape.image.is_none()
                    && !shape.group
                    && shape.line.is_none();
                // The colours of a group (an SVG brought in as shapes),
                // each one changed everywhere in it (spec 067).
                if shape.group
                    && let Some(page) = document.and_then(|d| d.page(shape.page))
                {
                    group_colors(ui, texts, page, shape.node, asked);
                }
                if let Some(frame) = &shape.frame {
                    frame_fields(ui, texts, frame, asked);
                } else if let Some(effect) = &shape.effect {
                    effect_fields(ui, texts, effect, asked);
                } else if shape.text.is_none() && !shape.group && shape.line.is_none() {
                    fill_field(ui, texts, shape, asked);
                    // Any shape can become a light (spec 065, S65-06).
                    if plain_shape {
                        ui.add_space(8.0);
                        light_kind_row(ui, texts, None, asked);
                    }
                }
                if shape.effect.is_none()
                    && shape.text.is_none()
                    && shape.image.is_none()
                    && !shape.group
                    && shape.line.is_none()
                {
                    ui.add_space(8.0);
                    outline_fields(ui, texts, shape, asked);
                    shape_text_fields(ui, theme, texts, shape, asked);
                    if let Some(inner) = &shape.inner {
                        font_button(ui, theme, texts, inner.content.first_style(), asked);
                    }
                }
                ui.add_space(8.0);
                opacity_field(ui, texts, shape, asked);
            });
    }

    /// The Capas tab (03 §2.4): the elements of the selected element's page
    /// (or the first page), front first. A click selects, a double click
    /// renames and a drag moves the row among its siblings; the eye hides
    /// and the lock locks (spec 048); Subir, Bajar and Eliminar act on the
    /// one selected element.
    fn layers(
        &mut self,
        ui: &mut egui::Ui,
        theme: &Theme,
        texts: &Texts,
        document: Option<&Document>,
        asked: &mut Asked,
    ) {
        let Some(document) = document else { return };
        let Some(page) = self.current_page(document) else {
            return;
        };
        let rows = canvas::layers(page, &self.open);
        if rows.is_empty() {
            ui.label(
                RichText::new(texts.t(&gafira_doc::msg!("layers.empty")))
                    .font(theme.font("body-sm"))
                    .color(theme.c("text.muted")),
            );
            return;
        }
        let dragged = self
            .row_drag
            .and_then(|node| rows.iter().find(|row| row.node == node).cloned());
        // The rows of the dragged row's siblings, front first, for the
        // insertion line.
        let mut sibling_rows: Vec<Rect> = Vec::new();
        let mut released = false;
        for row in rows {
            let Some(node) = page.node(row.node) else {
                continue;
            };
            let last = row.siblings.saturating_sub(1);
            let selected = self.selected.contains(page.id, row.node);
            // A hidden or locked row, or one inside a hidden or locked
            // group, is dimmed (03 «Lista con arrastre»).
            let dim = layers::locked(page, row.node) || layers::hidden(page, row.node);
            let label = row.name.clone().unwrap_or_else(|| {
                texts.t(&gafira_doc::msg::Message {
                    key: gafira_doc::msg::MessageKey(row.kind),
                    args: Vec::new(),
                })
            });
            let line = ui.horizontal(|ui| {
                // A group's children sit one step in, below it (spec 043).
                ui.add_space(16.0 * row.depth as f32);
                let (_, opener) = ui.allocate_exact_size(Vec2::splat(16.0), egui::Sense::click());
                if row.opens {
                    let open = self.open.contains(&row.node);
                    egui::collapsing_header::paint_default_icon(
                        ui,
                        if open { 1.0 } else { 0.0 },
                        &opener,
                    );
                    let tip = plain(texts, if open { "layers.close" } else { "layers.open" });
                    if opener.on_hover_text(tip).clicked() {
                        asked.toggle_open = Some(row.node);
                    }
                }
                // The eye and the lock, on the right of every row; Subir,
                // Bajar and Eliminar before them on the selected one.
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let lock = node.base.locked;
                    let (icon, key) = if lock {
                        (Icon::Locked, "layers.unlock")
                    } else {
                        (Icon::Unlocked, "layers.lock")
                    };
                    let button = IconButton::new(icon, plain(texts, key))
                        .active(lock)
                        .small()
                        .show(ui, theme);
                    if button.clicked() {
                        asked.flag = Some((page.id, vec![row.node], Flag::Locked, !lock));
                    }
                    let shown = node.base.visible;
                    let (icon, key) = if shown {
                        (Icon::Visible, "layers.hide")
                    } else {
                        (Icon::Hidden, "layers.show")
                    };
                    let button = IconButton::new(icon, plain(texts, key))
                        .small()
                        .show(ui, theme);
                    if button.clicked() {
                        asked.flag = Some((page.id, vec![row.node], Flag::Hidden, shown));
                    }
                    if selected && self.selected.len() == 1 {
                        let delete = IconButton::new(
                            Icon::Delete,
                            texts.t(&gafira_doc::msg!("editor.delete")),
                        )
                        .small()
                        .show(ui, theme);
                        asked.delete |= delete.clicked();
                        let down = ui.add_enabled_ui(row.index > 0, |ui| {
                            IconButton::new(
                                Icon::SendBackward,
                                texts.t(&gafira_doc::msg!("layers.down")),
                            )
                            .small()
                            .show(ui, theme)
                        });
                        if down.inner.clicked() {
                            asked.reorder = Some((page.id, row.node, row.index - 1));
                        }
                        let up = ui.add_enabled_ui(row.index < last, |ui| {
                            IconButton::new(
                                Icon::BringForward,
                                texts.t(&gafira_doc::msg!("layers.up")),
                            )
                            .small()
                            .show(ui, theme)
                        });
                        if up.inner.clicked() {
                            asked.reorder = Some((page.id, row.node, row.index + 1));
                        }
                    }
                    ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                        self.layer_name(ui, theme, page.id, &row, (label, selected, dim), asked)
                    })
                    .inner
                })
                .inner
            });
            if let Some(name) = line.inner {
                if name.drag_started() {
                    self.row_drag = Some(row.node);
                }
                released |= self.row_drag == Some(row.node) && name.drag_stopped();
            }
            if dragged
                .as_ref()
                .is_some_and(|d| d.parent == row.parent && d.depth == row.depth)
            {
                sibling_rows.push(line.response.rect);
            }
        }
        // The insertion line while a row is dragged; dropping it reorders
        // it among its siblings in one step (S48-06).
        let Some(row) = dragged else {
            self.row_drag = None;
            return;
        };
        let Some(at) = ui.input(|i| i.pointer.latest_pos()) else {
            return;
        };
        let gap = sibling_rows.iter().filter(|r| r.center().y < at.y).count();
        let y = match sibling_rows.get(gap) {
            Some(below) => below.top(),
            None => sibling_rows.last().map_or(at.y, |r| r.bottom()),
        };
        if !released {
            let x = ui.min_rect().x_range();
            ui.painter()
                .hline(x, y, Stroke::new(2.0, theme.c("primary.bg")));
            return;
        }
        self.row_drag = None;
        if let Some(index) = layers::dropped(row.index, row.siblings, gap) {
            asked.reorder = Some((page.id, row.node, index));
        }
    }

    /// The name of a row of Capas: a label that selects on a click, starts
    /// a rename on a double click and can be dragged; or, while renamed,
    /// the field (S48-05). With the label's response.
    fn layer_name(
        &mut self,
        ui: &mut egui::Ui,
        theme: &Theme,
        page: PageId,
        row: &canvas::Layer,
        (label, selected, dim): (String, bool, bool),
        asked: &mut Asked,
    ) -> Option<egui::Response> {
        if let Some((node, text, focused)) = self.renaming.as_mut()
            && *node == row.node
        {
            let field = ui.add(
                egui::TextEdit::singleline(text)
                    .font(theme.font("body-sm"))
                    .desired_width(f32::INFINITY),
            );
            if !*focused {
                field.request_focus();
                *focused = true;
            } else if field.lost_focus() {
                // Enter or a click elsewhere keeps it; Esc does not.
                if !ui.input(|i| i.key_pressed(Key::Escape))
                    && let Some(name) = layers::renamed(row.name.as_deref(), text)
                {
                    asked.rename = Some((page, row.node, name));
                }
                self.renaming = None;
            }
            return None;
        }
        let mut text = RichText::new(label.clone());
        if dim {
            text = text.color(theme.c("text.muted"));
        }
        let name = ui
            .selectable_label(selected, text)
            .interact(egui::Sense::click_and_drag());
        if name.double_clicked() {
            self.renaming = Some((row.node, row.name.clone().unwrap_or(label), false));
        } else if name.clicked() {
            // Ctrl/Cmd or Shift + click toggles the row (spec 040).
            let toggle = ui.input(|i| i.modifiers.command || i.modifiers.shift);
            asked.select = Some(if toggle {
                let mut next = self.selected.clone();
                next.toggle(page, row.node);
                next
            } else {
                Selection::one(page, row.node)
            });
            // A child of a group enters that group on the canvas.
            asked.scope = Some(row.parent.map(|group| (page, group)));
        }
        Some(name)
    }

    /// A text of `preset` with `words` on `page`, centred on `at` (px of
    /// page) or in the middle, selected, one undo step (specs 027, 061).
    /// Alignment, spacing and colour of the selected texts (spec 070), each
    /// one undo step; spacing refits a box that grows with its words.
    fn text_styles(&mut self, engine: &mut Gafira, asked: &Asked) {
        let Some(page) = self.selected.page() else {
            return;
        };
        let Some(document) = engine.edit().document() else {
            return;
        };
        let Some(on) = document.page(page) else {
            return;
        };
        let nodes: Vec<NodeId> = self
            .selected
            .nodes()
            .iter()
            .copied()
            .filter(|id| {
                on.node(*id)
                    .is_some_and(|n| matches!(n.kind, gafira_doc::NodeKind::Text(_)))
            })
            .collect();
        if nodes.is_empty() {
            return;
        }
        let mut commands: Vec<Box<dyn Command>> = Vec::new();
        if let Some(align) = asked.text_align {
            commands.push(Box::new(SetProperty::new(
                page,
                nodes.clone(),
                Property::TextAlign(align),
                None,
            )));
        }
        if let Some((letters, lines)) = asked.text_spacing {
            commands.push(Box::new(SetProperty::new(
                page,
                nodes.clone(),
                Property::TextSpacing(letters, lines),
                None,
            )));
            self.fit_pending = nodes.first().map(|node| (page, *node));
        }
        match &asked.text_fill {
            Some(TextFill::Gradient(gradient)) => commands.push(Box::new(SetProperty::new(
                page,
                nodes.clone(),
                Property::TextGradient(Some(gradient.clone())),
                None,
            ))),
            Some(TextFill::Solid(color)) => {
                commands.push(Box::new(SetProperty::new(
                    page,
                    nodes.clone(),
                    Property::TextGradient(None),
                    None,
                )));
                for node in &nodes {
                    if let Some(found) = on.node(*node)
                        && let gafira_doc::NodeKind::Text(text) = &found.kind
                        && let Some(style) = text.content.first_style()
                    {
                        let restyled = gafira_doc::SpanStyle {
                            color: *color,
                            ..style.clone()
                        };
                        let words = text.content.plain_text();
                        commands.push(Box::new(SetProperty::new(
                            page,
                            vec![*node],
                            Property::TextContent(gafira_doc::RichText::plain(&words, &restyled)),
                            None,
                        )));
                    }
                }
            }
            None => {}
        }
        if commands.is_empty() {
            return;
        }
        let step = Transaction::new(MessageKey("edit.set_property"), commands);
        if let Err(error) = engine.edit().execute(Box::new(step)) {
            eprintln!("text style: {error}");
        }
    }

    /// «Texto curvo» and «Combinaciones de fuentes» of the Texto panel:
    /// each card is added with a click and carried with a drag (D-105).
    fn text_extras(&self, ui: &mut egui::Ui, theme: &Theme, texts: &Texts, asked: &mut Asked) {
        ui.add_space(12.0);
        section_title(ui, theme, texts, "text.curved");
        let half = ((ui.available_width() - 8.0) / 2.0).max(40.0);
        ui.horizontal(|ui| {
            for (key, bend) in [
                ("text.arc_up", CURVE_PRESET),
                ("text.arc_down", -CURVE_PRESET),
            ] {
                let (rect, response) =
                    ui.allocate_exact_size(Vec2::new(half, 72.0), egui::Sense::click_and_drag());
                card_frame(ui, theme, rect, response.hovered());
                let painter = ui.painter();
                painter.text(
                    Pos2::new(rect.center().x, rect.top() + 20.0),
                    Align2::CENTER_CENTER,
                    plain(texts, key),
                    theme.font("body-sm"),
                    theme.c("text.secondary"),
                );
                // The arc the words follow: ⌒ for an arch, ⌣ for a cup.
                let y = rect.top() + 48.0;
                let lift = if bend > 0.0 { 12.0 } else { -12.0 };
                let a = Pos2::new(rect.center().x - 22.0, y + lift / 2.0);
                let b = Pos2::new(rect.center().x + 22.0, y + lift / 2.0);
                let control = Pos2::new(rect.center().x, y - lift);
                painter.add(egui::epaint::QuadraticBezierShape::from_points_stroke(
                    [a, control, b],
                    false,
                    Color32::TRANSPARENT,
                    Stroke::new(2.0, theme.c("text")),
                ));
                let words = plain(texts, "text.curved_words");
                if response.clicked() {
                    asked.insert_curved = Some((bend, words.clone(), None));
                }
                self.carry(ui, theme, &response, &plain(texts, key), |page, point| {
                    asked.insert_curved = Some((bend, words, Some((page, point))));
                });
            }
        });
        ui.add_space(16.0);
        section_title(ui, theme, texts, "text.pairings");
        let heading_words = plain(texts, "text.pairing_heading");
        let body_words = plain(texts, "text.pairing_body");
        for (index, pairing) in pairings::PAIRINGS.iter().enumerate() {
            let (rect, response) = ui.allocate_exact_size(
                Vec2::new(ui.available_width(), 92.0),
                egui::Sense::click_and_drag(),
            );
            card_frame(ui, theme, rect, response.hovered());
            let clip = ui.painter().with_clip_rect(rect.shrink(1.0));
            let left = rect.left() + 14.0;
            clip.text(
                Pos2::new(left, rect.top() + 10.0),
                Align2::LEFT_TOP,
                plain(texts, pairing.key),
                theme.font("body-sm"),
                theme.c("text.secondary"),
            );
            for (face, words, y, size) in [
                (pairing.heading, &heading_words, rect.top() + 30.0, 24.0),
                (pairing.body, &body_words, rect.top() + 66.0, 12.0),
            ] {
                let style = pairings::style(face);
                let family = text_family(ui.ctx(), theme, &style);
                clip.text(
                    Pos2::new(left, y),
                    Align2::LEFT_TOP,
                    pairings::words(face, words),
                    egui::FontId::new(size, family),
                    canvas::screen_color(style.color),
                );
            }
            let words = (heading_words.clone(), body_words.clone());
            if response.clicked() {
                asked.insert_pairing = Some((index, words.clone(), None));
            }
            self.carry(
                ui,
                theme,
                &response,
                &plain(texts, pairing.key),
                |page, point| {
                    asked.insert_pairing = Some((index, words, Some((page, point))));
                },
            );
            ui.add_space(8.0);
        }
    }

    /// A card of a side panel being dragged: a ghost under the pointer and,
    /// when it is let go over a page, `dropped` with the page and the point.
    fn carry(
        &self,
        ui: &egui::Ui,
        theme: &Theme,
        response: &egui::Response,
        label: &str,
        dropped: impl FnOnce(PageId, (f64, f64)),
    ) {
        let at = ui.input(|i| i.pointer.latest_pos());
        // A hand over the card, a closed hand while it is carried.
        if response.hovered() && !response.dragged() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        if response.dragged()
            && let Some(at) = at
        {
            ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
            let painter = ui.ctx().layer_painter(egui::LayerId::new(
                egui::Order::Tooltip,
                egui::Id::new("editor.text_ghost"),
            ));
            painter.text(
                at,
                Align2::CENTER_CENTER,
                label,
                theme.font("ui-label"),
                theme.c("text").gamma_multiply(0.6),
            );
        }
        if response.drag_stopped()
            && let Some(at) = at
            && let Some((page, point)) = canvas::drop_point(&self.page_rects, self.zoom, at)
        {
            dropped(page, point);
        }
    }

    /// A curved text of `bend` on `page`, in its middle or centred on `at`,
    /// its box as high as its arc; selected (FD-44).
    fn add_curved(
        &mut self,
        engine: &mut Gafira,
        document: &Document,
        theme: &Theme,
        (bend, words): (f64, &str),
        (page, at): (PageId, Option<(f64, f64)>),
    ) {
        let Some(on) = document.page(page) else {
            return;
        };
        let Ok(size) = on.size.to_px(document.dpi()) else {
            return;
        };
        let style = text_style(theme, canvas::TextPreset::Subtitle);
        let px = Vec2::new(size.width() as f32, size.height() as f32);
        let mut spec = match at {
            Some(at) => canvas::text_at(px, words, &style, at),
            None => canvas::new_text(px, words, &style),
        };
        if let gafira_doc::NodeKind::Text(text) = &mut spec.kind {
            text.curve = bend;
            text.sizing = gafira_doc::TextSizing::Fixed;
            let placement = &mut spec.placement;
            // As wide as the words on a line, so they bend as one arc.
            placement.w = (words.chars().count() as f64 * style.size * 0.62).max(80.0);
            if let Some(h) = canvas::curved_height(text, placement.w, placement.h) {
                let centre_y = placement.y + placement.h / 2.0;
                placement.h = h;
                placement.y = centre_y - h / 2.0;
            }
            let centre_x = match at {
                Some((x, _)) => x,
                None => f64::from(px.x) / 2.0,
            };
            placement.x = centre_x - placement.w / 2.0;
        }
        let next = NodeId::new(document.next_id());
        let command = InsertNodes::new(page, None, on.roots.len(), vec![spec]);
        match engine.edit().execute(Box::new(command)) {
            Ok(_) => {
                self.selected = Selection::one(page, next);
                self.active = Some(page);
                self.scope = None;
            }
            Err(error) => eprintln!("insert curved text: {error}"),
        }
    }

    /// The font pairing `index` on `page`, in its middle or centred on
    /// `at`: one group of its heading and body line, selected.
    fn add_pairing(
        &mut self,
        engine: &mut Gafira,
        document: &Document,
        (index, (heading, body)): (usize, &(String, String)),
        (page, at): (PageId, Option<(f64, f64)>),
    ) {
        let (Some(on), Some(pairing)) = (document.page(page), pairings::PAIRINGS.get(index)) else {
            return;
        };
        let Ok(size) = on.size.to_px(document.dpi()) else {
            return;
        };
        let (w, h) = (size.width(), size.height());
        let centre = at.unwrap_or((w / 2.0, h / 2.0));
        let spec = pairings::group(w, centre, *pairing, (heading, body));
        let next = NodeId::new(document.next_id());
        let command = InsertNodes::new(page, None, on.roots.len(), vec![spec]);
        match engine.edit().execute(Box::new(command)) {
            Ok(_) => {
                self.selected = Selection::one(page, next);
                self.active = Some(page);
                self.scope = None;
            }
            Err(error) => eprintln!("insert font pairing: {error}"),
        }
    }

    fn add_text(
        &mut self,
        engine: &mut Gafira,
        document: &Document,
        theme: &Theme,
        (preset, words): (canvas::TextPreset, &str),
        (page, at): (PageId, Option<(f64, f64)>),
    ) -> Option<(PageId, NodeId)> {
        let on = document.page(page)?;
        let size = on.size.to_px(document.dpi()).ok()?;
        let style = text_style(theme, preset);
        let px = Vec2::new(size.width() as f32, size.height() as f32);
        let spec = match at {
            Some(at) => canvas::text_at(px, words, &style, at),
            None => canvas::new_text(px, words, &style),
        };
        let command = InsertNodes::new(page, None, on.roots.len(), vec![spec]);
        let next = NodeId::new(document.next_id());
        match engine.edit().execute(Box::new(command)) {
            Ok(_) => {
                self.selected = Selection::one(page, next);
                self.active = Some(page);
                self.scope = None;
                Some((page, next))
            }
            Err(error) => {
                eprintln!("insert text: {error}");
                None
            }
        }
    }

    /// The texture `key` names, made by `make` the first time; `None` when
    /// it cannot be made. The cache is emptied when it grows past 256, so
    /// a resize or a zoom does not keep every size it went through.
    fn path_texture(
        &self,
        ctx: &egui::Context,
        key: u64,
        make: impl FnOnce() -> Option<egui::ColorImage>,
    ) -> Option<egui::TextureId> {
        let mut cache = self.path_textures.borrow_mut();
        if let Some(texture) = cache.get(&key) {
            return Some(texture.id());
        }
        if cache.len() > 256 {
            cache.clear();
        }
        let texture = ctx.load_texture(
            format!("path-{key:x}"),
            make()?,
            egui::TextureOptions::LINEAR,
        );
        let id = texture.id();
        cache.insert(key, texture);
        Some(id)
    }

    /// `piece` filling `rect` in `color`, at its own proportions (specs 053
    /// and 054): what a tile of the Formas panel and the drag preview draw.
    fn paint_piece(
        &self,
        ctx: &egui::Context,
        painter: &egui::Painter,
        piece: canvas::Piece,
        rect: Rect,
        color: Color32,
    ) {
        // A line: across the tile with its caps (spec 058).
        if let Some((start, end)) = piece.line_caps() {
            let line = gafira_doc::LineNode {
                start: gafira_doc::LineEnd::new([0.0, 0.5], start),
                end: gafira_doc::LineEnd::new([1.0, 0.5], end),
                color: canvas::document_color(color),
                width: 2.0,
            };
            let y = rect.center().y;
            painter.extend(canvas::line_shapes(
                &line,
                Pos2::new(rect.min.x + 2.0, y),
                Pos2::new(rect.max.x - 2.0, y),
                1.0,
                color,
            ));
            return;
        }
        let aspect = piece.aspect() as f32;
        let side = rect.width().min(rect.height());
        let size = if aspect >= 1.0 {
            Vec2::new(side, side / aspect)
        } else {
            Vec2::new(side * aspect, side)
        };
        let rect = Rect::from_center_size(rect.center(), size);
        match piece.geometry() {
            ShapeGeometry::Rect => {
                let radius = piece.corner_radius(f64::from(rect.width())) as f32;
                painter.rect_filled(rect, radius, color);
            }
            ShapeGeometry::Ellipse => {
                painter.circle_filled(rect.center(), side / 2.0, color);
            }
            geometry @ ShapeGeometry::Path(_) => {
                let shape = Shape {
                    page: PageId::new(0),
                    node: NodeId::new(0),
                    placement: Placement::new(
                        f64::from(rect.min.x),
                        f64::from(rect.min.y),
                        f64::from(rect.width()),
                        f64::from(rect.height()),
                    ),
                    geometry,
                    fill: Some(canvas::document_color(color)),
                    stroke: None,
                    corner_radius: 0.0,
                    gradient: None,
                    effect: None,
                    opacity: f32::from(color.a()) / 255.0,
                    inherited: 1.0,
                    group: false,
                    text: None,
                    image: None,
                    locked: false,
                    inner: None,
                    line: None,
                    frame: None,
                    qr: None,
                    table: None,
                };
                let cover = canvas::path_cover(&shape, 1.0);
                let key = path_key(&shape, 1.0);
                if let Some(texture) =
                    self.path_texture(ctx, key, || canvas::path_image(&shape, 1.0).map(|(i, _)| i))
                {
                    let at = Rect::from_min_size(
                        Pos2::new(cover.x as f32, cover.y as f32),
                        Vec2::new(cover.w as f32, cover.h as f32),
                    );
                    let uv = Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0));
                    painter.image(
                        texture,
                        at,
                        uv,
                        Color32::WHITE.gamma_multiply(shape.opacity),
                    );
                }
            }
        }
    }

    /// A 56 px tile of the Formas panel that draws `piece` (S53-03), named
    /// `name` for assistive technology and in its tooltip; it can be clicked
    /// and dragged.
    /// «Fondo de la página» (spec 069): what the Inspector shows with the
    /// page or the canvas selected. A colour (searched by name or hex, from
    /// the palette, or picked), a gradient, a pattern with its two colours, or
    /// an image filled, fitted or tiled; «Aplicar a todas las páginas».
    fn background_fields(
        &self,
        ui: &mut egui::Ui,
        theme: &Theme,
        texts: &Texts,
        page: &gafira_doc::Page,
        asked: &mut Asked,
    ) {
        use gafira_doc::{
            BackgroundFit, Gradient, GradientKind, GradientStop, PageBackground, PatternKind,
        };
        let current = page.background.clone();
        let heading = |ui: &mut egui::Ui, key: &'static str| {
            ui.add_space(12.0);
            ui.label(
                RichText::new(plain(texts, key))
                    .font(theme.font("ui-label"))
                    .color(theme.c("text")),
            );
            ui.add_space(6.0);
        };
        ui.label(
            RichText::new(plain(texts, "background.title"))
                .font(theme.font("ui-label"))
                .color(theme.c("text")),
        );
        ui.add_space(8.0);
        // The search: «azul», «blue» or «#00c4cc».
        let found = {
            let mut query = self.background_query.borrow_mut();
            ui.add(widgets::field(
                &mut query,
                plain(texts, "background.search"),
            ));
            crate::colors::search(&query)
        };
        if !found.is_empty() {
            ui.add_space(6.0);
            ui.horizontal_wrapped(|ui| {
                for color in found {
                    if swatch(ui, theme, |p, r| {
                        p.rect_filled(r, 6.0, canvas::screen_color(color));
                    })
                    .clicked()
                    {
                        asked.page_background = Some(Some(PageBackground::Solid(color)));
                    }
                }
            });
        }
        heading(ui, "background.color");
        ui.horizontal(|ui| {
            ui.label(plain(texts, "background.custom"));
            let solid = match &current {
                Some(PageBackground::Solid(c)) => Some(*c),
                _ => None,
            };
            if let Some(color) = color_button(ui, solid.or(Some(crate::colors::hex(0xFFFFFF)))) {
                asked.page_background = Some(Some(PageBackground::Solid(color)));
            }
        });
        ui.add_space(6.0);
        ui.horizontal_wrapped(|ui| {
            for code in crate::colors::PALETTE {
                let color = crate::colors::hex(code);
                if swatch(ui, theme, |p, r| {
                    p.rect_filled(r, 6.0, canvas::screen_color(color));
                })
                .clicked()
                {
                    asked.page_background = Some(Some(PageBackground::Solid(color)));
                }
            }
        });
        heading(ui, "background.gradient");
        ui.horizontal_wrapped(|ui| {
            for (a, b, angle) in crate::colors::GRADIENTS {
                let (ca, cb) = (crate::colors::hex(a), crate::colors::hex(b));
                let clicked = swatch(ui, theme, |p, r| {
                    paint_gradient_swatch(
                        p,
                        r,
                        canvas::screen_color(ca),
                        canvas::screen_color(cb),
                        angle,
                    );
                })
                .clicked();
                if clicked {
                    asked.page_background = Some(Some(PageBackground::Gradient(Gradient {
                        kind: GradientKind::Linear { angle_deg: angle },
                        stops: vec![
                            GradientStop {
                                offset: 0.0,
                                color: ca,
                            },
                            GradientStop {
                                offset: 1.0,
                                color: cb,
                            },
                        ],
                    })));
                }
            }
        });
        heading(ui, "background.patterns");
        let (pattern_color, pattern_back) = match &current {
            Some(PageBackground::Pattern { color, back, .. }) => (*color, *back),
            // Gafira's mint (brand.mint) on white until a pattern is chosen.
            _ => (crate::colors::hex(0x3FBFA9), crate::colors::hex(0xFFFFFF)),
        };
        let current_kind = match &current {
            Some(PageBackground::Pattern { kind, .. }) => Some(*kind),
            _ => None,
        };
        ui.horizontal(|ui| {
            ui.label(plain(texts, "background.pattern_color"));
            if let Some(color) = color_button(ui, Some(pattern_color)) {
                asked.page_background = Some(Some(PageBackground::Pattern {
                    kind: current_kind.unwrap_or(PatternKind::Dots),
                    color,
                    back: pattern_back,
                }));
            }
        });
        ui.horizontal(|ui| {
            ui.label(plain(texts, "background.pattern_back"));
            if let Some(back) = color_button(ui, Some(pattern_back)) {
                asked.page_background = Some(Some(PageBackground::Pattern {
                    kind: current_kind.unwrap_or(PatternKind::Dots),
                    color: pattern_color,
                    back,
                }));
            }
        });
        ui.add_space(6.0);
        ui.horizontal_wrapped(|ui| {
            for kind in PatternKind::ALL {
                let preview = PageBackground::Pattern {
                    kind,
                    color: pattern_color,
                    back: pattern_back,
                };
                let key = {
                    use std::hash::{Hash, Hasher};
                    let mut hasher = std::collections::hash_map::DefaultHasher::new();
                    format!("pattern tile {preview:?}").hash(&mut hasher);
                    hasher.finish()
                };
                let texture = self.path_texture(ui.ctx(), key, || {
                    canvas::background_image(&preview, (96.0, 96.0), 1.0, None)
                });
                let clicked = swatch(ui, theme, |p, r| {
                    if let Some(texture) = texture {
                        let uv = Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0));
                        p.image(texture, r, uv, Color32::WHITE);
                    }
                })
                .clicked();
                if clicked {
                    asked.page_background = Some(Some(preview));
                }
            }
        });
        heading(ui, "background.image");
        if ui
            .add_sized(
                [ui.available_width(), 32.0],
                egui::Button::new(plain(texts, "background.upload")),
            )
            .clicked()
        {
            asked.background_pick = true;
        }
        if let Some(PageBackground::Image {
            asset,
            natural,
            fit,
        }) = &current
        {
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                for (option, key) in [
                    (BackgroundFit::Fill, "background.fill"),
                    (BackgroundFit::Fit, "background.fit"),
                    (BackgroundFit::Tile, "background.tile"),
                ] {
                    if ui
                        .selectable_label(*fit == option, plain(texts, key))
                        .clicked()
                    {
                        asked.page_background = Some(Some(PageBackground::Image {
                            asset: *asset,
                            natural: *natural,
                            fit: option,
                        }));
                    }
                }
            });
        }
        ui.add_space(12.0);
        if current.is_some() && ui.button(plain(texts, "background.remove")).clicked() {
            asked.page_background = Some(None);
        }
        ui.add_space(8.0);
        if ui
            .add_sized(
                [ui.available_width(), 32.0],
                egui::Button::new(plain(texts, "background.all")).fill(theme.c("secondary.bg")),
            )
            .clicked()
        {
            asked.background_all = true;
        }
    }

    /// The bar over the selection (owner request): «Agrupar» for several
    /// elements or «Desagrupar» for a group, then Bloquear or Desbloquear,
    /// Duplicar, Eliminar and «···» with Copiar and Cortar. Above the
    /// selection, or below it when there is no room.
    fn selection_bar(
        &self,
        ctx: &egui::Context,
        theme: &Theme,
        texts: &Texts,
        around: Rect,
        page: &Page,
        asked: &mut Asked,
    ) {
        let locked = self
            .selected
            .nodes()
            .iter()
            .all(|node| page.node(*node).is_some_and(|n| n.base.locked));
        let screen = ctx.content_rect();
        // Clear of the turn handle above the box.
        let above = around.top() - 84.0 > screen.top() + 56.0;
        let anchor = if above {
            Pos2::new(around.center().x, around.top() - 40.0)
        } else {
            Pos2::new(around.center().x, around.bottom() + 16.0)
        };
        let pivot = if above {
            Align2::CENTER_BOTTOM
        } else {
            Align2::CENTER_TOP
        };
        egui::Area::new(egui::Id::new("editor.selection_bar"))
            .order(egui::Order::Foreground)
            .pivot(pivot)
            .fixed_pos(anchor)
            .show(ctx, |ui| {
                egui::Frame::popup(ui.style())
                    .corner_radius(theme.radius_md())
                    .inner_margin(egui::Margin::symmetric(10, 6))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.set_min_height(32.0);
                            // Locked, the one thing to do is unlock it (D-110).
                            if locked {
                                // The closed lock, lit, «Desbloquear» on hover.
                                let unlock =
                                    IconButton::new(Icon::Locked, plain(texts, "layers.unlock"))
                                        .active(true)
                                        .small()
                                        .show(ui, theme);
                                if unlock.hovered() {
                                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                                }
                                if unlock.clicked() {
                                    asked.toggle = Some(Flag::Locked);
                                }
                                return;
                            }
                            if let Some((_, node)) = self.selected.single()
                                && let Some(gafira_doc::NodeKind::Table(table)) =
                                    page.node(node).map(|n| &n.kind)
                            {
                                self.table_bar(ui, theme, texts, (node, table), asked);
                                bar_separator(ui, theme);
                            }
                            if self.can.ungroup && self.selected.len() == 1 {
                                if ui.button(plain(texts, "editor.ungroup")).clicked() {
                                    asked.ungroup = true;
                                }
                            } else if self.can.group
                                && self.selected.len() > 1
                                && ui.button(plain(texts, "editor.group")).clicked()
                            {
                                asked.group = true;
                            }
                            let (icon, key) = if locked {
                                (Icon::Locked, "layers.unlock")
                            } else {
                                (Icon::Unlocked, "layers.lock")
                            };
                            if IconButton::new(icon, plain(texts, key))
                                .small()
                                .show(ui, theme)
                                .clicked()
                            {
                                asked.toggle = Some(Flag::Locked);
                            }
                            if IconButton::new(Icon::PageCopy, plain(texts, "editor.duplicate"))
                                .small()
                                .show(ui, theme)
                                .clicked()
                            {
                                asked.duplicate = true;
                            }
                            if IconButton::new(Icon::Delete, plain(texts, "editor.delete"))
                                .small()
                                .show(ui, theme)
                                .clicked()
                            {
                                asked.delete = true;
                            }
                            ui.menu_button("···", |ui| {
                                if ui.button(plain(texts, "editor.copy")).clicked() {
                                    asked.copy = true;
                                    ui.close();
                                }
                                if ui.button(plain(texts, "editor.cut")).clicked() {
                                    asked.cut = true;
                                    ui.close();
                                }
                            });
                        });
                    });
            });
    }

    /// The background of a page on the canvas (spec 069): a colour painted
    /// directly, anything else from its texture.
    fn paint_background(
        &self,
        ctx: &egui::Context,
        painter: &egui::Painter,
        page: Rect,
        zoom: f32,
        background: &gafira_doc::PageBackground,
    ) {
        if let gafira_doc::PageBackground::Solid(color) = background {
            painter.rect_filled(page, 0.0, canvas::screen_color(*color));
            return;
        }
        // A pattern is drawn as vector shapes, sharp at any zoom (spec 069).
        if let gafira_doc::PageBackground::Pattern { kind, color, back } = background {
            painter.rect_filled(page, 0.0, canvas::screen_color(*back));
            let fill = canvas::screen_color(*color);
            let clip = painter.with_clip_rect(page.intersect(painter.clip_rect()));
            let (pw, ph) = (
                f64::from(page.width() / zoom),
                f64::from(page.height() / zoom),
            );
            let path = gafira_raster::pattern_path(*kind, pw, ph, gafira_raster::PATTERN_CELL);
            let to_screen = |p: kurbo::Point| {
                Pos2::new(
                    page.min.x + p.x as f32 * zoom,
                    page.min.y + p.y as f32 * zoom,
                )
            };
            let mut shape: Vec<Pos2> = Vec::new();
            let mut shapes: Vec<egui::Shape> = Vec::new();
            // A quarter of a screen px, in px of page.
            let tolerance = 0.25 / f64::from(zoom.max(1e-3));
            kurbo::flatten(path.elements().iter().copied(), tolerance, |el| match el {
                kurbo::PathEl::MoveTo(p) => {
                    shape.clear();
                    shape.push(to_screen(p));
                }
                kurbo::PathEl::LineTo(p) => shape.push(to_screen(p)),
                kurbo::PathEl::ClosePath => {
                    if shape.len() >= 3 {
                        shapes.push(egui::Shape::convex_polygon(
                            std::mem::take(&mut shape),
                            fill,
                            Stroke::NONE,
                        ));
                    }
                }
                _ => {}
            });
            clip.extend(shapes);
            return;
        }
        let page_px = (
            f64::from(page.width() / zoom),
            f64::from(page.height() / zoom),
        );
        let photo = background
            .asset()
            .and_then(|asset| self.photos.get(&asset.as_hash()))
            .and_then(Option::as_ref);
        let key = {
            use std::hash::{Hash, Hasher};
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            format!("background {background:?} {page_px:?} {zoom}").hash(&mut hasher);
            hasher.finish()
        };
        if let Some(texture) = self.path_texture(ctx, key, || {
            canvas::background_image(background, page_px, zoom, photo.map(|p| &**p))
        }) {
            let uv = Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0));
            painter.image(texture, page, uv, Color32::WHITE);
        }
    }

    /// A tile of Elementos → Marcos (S66-12): the frame's shape with the
    /// sample photo inside; click and drag like the shape tiles.
    /// Marcos (spec 066, owner request with references): a search, the
    /// collections as rows with «Ver todo», «Populares» as a grid, and
    /// «Importar forma». Each frame is added with a click or a drag.
    fn frames_panel(&self, ui: &mut egui::Ui, theme: &Theme, texts: &Texts, asked: &mut Asked) {
        use crate::frames::collections;
        egui::ScrollArea::vertical()
            .id_salt("editor.frames")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let mut view = self.frames_view.borrow_mut();
                ui.add(widgets::field(&mut view.0, plain(texts, "frames.search")));
                ui.add_space(10.0);
                let query = view.0.trim().to_owned();
                let side = ((ui.available_width() - 3.0 * 6.0) / 4.0).max(40.0);
                if !query.is_empty() {
                    let found: Vec<crate::frames::Kind> = crate::frames::search(&query)
                        .iter()
                        .map(|i| i.kind)
                        .collect();
                    if found.is_empty() {
                        ui.label(
                            RichText::new(plain(texts, "frames.empty"))
                                .font(theme.font("body-sm"))
                                .color(theme.c("text.secondary")),
                        );
                    }
                    self.frame_grid(ui, theme, &found, side, asked);
                    return;
                }
                if let Some(open) = view.1 {
                    let Some(collection) = collections().get(open) else {
                        view.1 = None;
                        return;
                    };
                    ui.horizontal(|ui| {
                        let back = ui.add(egui::Button::new("←").frame(false));
                        if back.hovered() {
                            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                        }
                        if back.clicked() {
                            view.1 = None;
                        }
                        ui.label(
                            RichText::new(plain(texts, collection.key))
                                .font(theme.font("ui-label"))
                                .color(theme.c("text")),
                        );
                    });
                    ui.add_space(6.0);
                    let kinds: Vec<_> = collection.items.iter().map(|i| i.kind).collect();
                    self.frame_grid(ui, theme, &kinds, side * 4.0 / 3.0 - 2.0, asked);
                    return;
                }
                for (index, collection) in collections().iter().enumerate() {
                    let kinds: Vec<_> = collection.items.iter().map(|i| i.kind).collect();
                    let popular = collection.key == "frames.popular";
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(plain(texts, collection.key))
                                .font(theme.font("ui-label"))
                                .color(theme.c("text")),
                        );
                        if !popular {
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    let all = ui.add(
                                        egui::Button::new(
                                            RichText::new(plain(texts, "frames.see_all"))
                                                .font(theme.font("body-sm")),
                                        )
                                        .frame(false),
                                    );
                                    if all.hovered() {
                                        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                                    }
                                    if all.clicked() {
                                        view.1 = Some(index);
                                    }
                                },
                            );
                        }
                    });
                    ui.add_space(4.0);
                    if popular {
                        self.frame_grid(ui, theme, &kinds, side * 4.0 / 3.0 - 2.0, asked);
                    } else {
                        egui::ScrollArea::horizontal()
                            .id_salt(("editor.frames.row", index))
                            .scroll_bar_visibility(
                                egui::scroll_area::ScrollBarVisibility::AlwaysHidden,
                            )
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.spacing_mut().item_spacing.x = 6.0;
                                    for kind in kinds.iter().take(10) {
                                        self.frame_kind_tile(ui, theme, *kind, side, asked);
                                    }
                                });
                            });
                    }
                    ui.add_space(12.0);
                }
                if widgets::secondary(ui, theme, plain(texts, "frame.import_shape")).clicked() {
                    asked.import_frame_shape = true;
                }
                ui.add_space(8.0);
                ui.label(
                    RichText::new(plain(texts, "frame.hint"))
                        .small()
                        .color(theme.c("text.secondary")),
                );
            });
    }

    /// `kinds` as a grid of three columns of tiles `side` across.
    fn frame_grid(
        &self,
        ui: &mut egui::Ui,
        theme: &Theme,
        kinds: &[crate::frames::Kind],
        side: f32,
        asked: &mut Asked,
    ) {
        let columns = ((ui.available_width() + 6.0) / (side + 6.0))
            .floor()
            .max(1.0) as usize;
        for line in kinds.chunks(columns) {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                for kind in line {
                    self.frame_kind_tile(ui, theme, *kind, side, asked);
                }
            });
            ui.add_space(6.0);
        }
    }

    /// One frame of Marcos: the sample photo cut by its outline, or the
    /// device or film with the sample in its screens; a click adds it, a
    /// drag carries it (D-105).
    fn frame_kind_tile(
        &self,
        ui: &mut egui::Ui,
        theme: &Theme,
        kind: crate::frames::Kind,
        side: f32,
        asked: &mut Asked,
    ) {
        let (rect, tile) = ui.allocate_exact_size(Vec2::splat(side), egui::Sense::click_and_drag());
        if tile.hovered() && !tile.dragged() {
            ui.painter()
                .rect_filled(rect, theme.radius_md(), theme.c("secondary.bg"));
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        if ui.is_rect_visible(rect) {
            self.paint_frame_kind(ui.ctx(), ui.painter(), kind, rect.shrink(4.0));
        }
        if tile.clicked() {
            asked.insert_frame_kind = Some((kind, None));
        }
        let at = ui.input(|i| i.pointer.latest_pos());
        if tile.dragged()
            && let Some(at) = at
        {
            ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
            let ghost = ui.ctx().layer_painter(egui::LayerId::new(
                egui::Order::Tooltip,
                egui::Id::new("frames.ghost"),
            ));
            self.paint_frame_kind(
                ui.ctx(),
                &ghost,
                kind,
                Rect::from_center_size(at, Vec2::splat(80.0)),
            );
        }
        if tile.drag_stopped()
            && let Some(at) = at
            && let Some((page, point)) = canvas::drop_point(&self.page_rects, self.zoom, at)
        {
            asked.insert_frame_kind = Some((kind, Some((page, point))));
        }
    }

    /// The preview of `kind` in `rect`, its proportions kept.
    fn paint_frame_kind(
        &self,
        ctx: &egui::Context,
        painter: &egui::Painter,
        kind: crate::frames::Kind,
        rect: Rect,
    ) {
        // Every frame, plain or held by a device, film, stem or paper, is
        // drawn by the same preview, its proportions kept.
        let scale = ctx.pixels_per_point();
        let side = f64::from(rect.width().min(rect.height()));
        let key = {
            use std::hash::{Hash, Hasher};
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            format!("frame kind {kind:?} {side} {scale}").hash(&mut hasher);
            hasher.finish()
        };
        let made = self.path_texture(ctx, key, || {
            let spec = crate::frames::spec(kind, (side, side), (side / 2.0, side / 2.0))?;
            canvas::spec_preview(&spec, side, scale)
        });
        if let Some(texture) = made {
            let uv = Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0));
            let square = Rect::from_center_size(rect.center(), Vec2::splat(side as f32));
            painter.image(texture, square, uv, Color32::WHITE);
        }
    }

    fn piece_tile(
        &self,
        ui: &mut egui::Ui,
        theme: &Theme,
        piece: canvas::Piece,
        name: String,
    ) -> egui::Response {
        let (rect, response) =
            ui.allocate_exact_size(Vec2::splat(56.0), egui::Sense::click_and_drag());
        if response.hovered() || response.dragged() {
            ui.painter()
                .rect_filled(rect, theme.radius_md(), theme.c("secondary.bg"));
            ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
        }
        self.paint_piece(
            ui.ctx(),
            ui.painter(),
            piece,
            rect.shrink(6.0),
            theme.c("text"),
        );
        response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, &name));
        response.on_hover_text(name)
    }

    /// The shape being dragged from the panel, half transparent under the
    /// pointer, over everything, with the size it will take on the page
    /// under it (or the first page) at the zoom on display (S52-04).
    /// What follows the pointer while a light is dragged from the panel
    /// (D-105): a soft disc of its size on the page under it, with its icon.
    fn light_ghost(&self, ctx: &egui::Context, theme: &Theme, item: elements::LightItem, at: Pos2) {
        let Some((_, page)) = self
            .page_rects
            .iter()
            .find(|(_, rect)| rect.contains(at))
            .or(self.page_rects.first())
        else {
            return;
        };
        let side = page.width().min(page.height()) * item.share() as f32;
        let painter = ctx.layer_painter(egui::LayerId::new(
            egui::Order::Tooltip,
            egui::Id::new("editor.light_ghost"),
        ));
        painter.circle_filled(at, side / 2.0, theme.c("accent").gamma_multiply(0.35));
        let icon = side.clamp(16.0, 32.0);
        crate::icons::paint(
            &painter,
            Rect::from_center_size(at, Vec2::splat(icon)),
            item.icon(),
            crate::icons::Weight::Regular,
            theme.c("text"),
        );
    }

    fn piece_ghost(&self, ctx: &egui::Context, theme: &Theme, piece: canvas::Piece, at: Pos2) {
        let Some((_, page)) = self
            .page_rects
            .iter()
            .find(|(_, rect)| rect.contains(at))
            .or(self.page_rects.first())
        else {
            return;
        };
        let side = page.width().min(page.height()) / 3.0;
        let color = theme.c("accent").gamma_multiply(0.5);
        let painter = ctx.layer_painter(egui::LayerId::new(
            egui::Order::Tooltip,
            egui::Id::new("editor.piece_ghost"),
        ));
        self.paint_piece(
            ctx,
            &painter,
            piece,
            Rect::from_center_size(at, Vec2::splat(side)),
            color,
        );
    }

    /// The end of a line follows the pointer (spec 058): near a shape it
    /// sticks to one of its five points or, else, to the nearest point of
    /// its outline, unless Ctrl lets the magnet go. The line and its box
    /// change together, one undo step for the whole drag.
    fn drag_line_end(&mut self, engine: &mut Gafira, delta: Vec2, free: bool) {
        let zoom = f64::from(self.zoom);
        let Some(drag) = self.drag.as_mut() else {
            return;
        };
        drag.offset += delta;
        let Some(Hold::LineEnd(which)) = drag.hold else {
            return;
        };
        let Some((node, origin)) = drag.origins.first().copied() else {
            return;
        };
        let Some(start) = drag.start.as_ref().and_then(|d| d.page(drag.page)) else {
            return;
        };
        let Some(gafira_doc::NodeKind::Line(line)) = start.node(node).map(|n| &n.kind) else {
            return;
        };
        let Some(now) = engine.edit().document() else {
            return;
        };
        let Some(page) = now.page(drag.page) else {
            return;
        };
        let from = line.ends(start, &origin)[which.min(1)];
        let p = from
            + gafira_geom::Vec2::new(
                f64::from(drag.offset.x) / zoom,
                f64::from(drag.offset.y) / zoom,
            );
        let stuck = (!free)
            .then(|| lines::stick(page, p, lines::SNAP_PX / zoom, node))
            .flatten();
        let point = stuck.map_or(p, |s| s.point);
        let attach = stuck.map(|s| gafira_doc::Attachment {
            node: s.node,
            at: s.at,
        });
        let (next, placement) = lines::with_end(line, &origin, page, which, point, attach);
        let ends = next.ends(page, &placement);
        let commands: Vec<Box<dyn Command>> = vec![
            Box::new(SetProperty::new(
                drag.page,
                vec![node],
                Property::Placement(placement),
                drag.gesture,
            )),
            Box::new(SetProperty::new(
                drag.page,
                vec![node],
                Property::Line(next),
                drag.gesture,
            )),
        ];
        match engine.edit().execute(Box::new(Transaction::new(
            MessageKey("edit.line"),
            commands,
        ))) {
            Ok(applied) => drag.moved |= applied == gafira_edit::Applied::Changed,
            Err(error) => eprintln!("line: {error}"),
        }
        self.line_view = Some(LineView {
            page: drag.page,
            near: lines::near(page, p, lines::REACH_PX / zoom, node),
            stuck: stuck.map(|s| s.node),
            end: ends[which.min(1)],
            other: ends[1 - which.min(1)],
        });
    }

    /// Move `nodes` from one page to the other after a drag (spec 051), as
    /// one undo step with the drag: the drag is undone and the transfer,
    /// built from where the drag left them, takes its place. They stay
    /// selected on the new page, which becomes the active one.
    fn transfer(
        &mut self,
        engine: &mut Gafira,
        (from, to): (PageId, PageId),
        nodes: &[NodeId],
        shift: (f64, f64),
    ) {
        let Some(now) = engine.edit().document() else {
            return;
        };
        let Some((command, ids)) = transfer::transfer_command(&now, from, nodes, to, shift) else {
            return;
        };
        if let Err(error) = engine.edit().undo() {
            eprintln!("transfer: {error}");
            return;
        }
        match engine.edit().execute(command) {
            Ok(_) => {
                self.selected = Selection::of(to, ids);
                self.active = Some(to);
                self.scope = None;
            }
            Err(error) => {
                eprintln!("transfer: {error}");
                // Put the drag back as it was.
                if let Err(error) = engine.edit().redo() {
                    eprintln!("transfer: {error}");
                }
            }
        }
    }

    /// The header over a page (spec 056, FD-02): «Página N» («Página
    /// oculta» when hidden), a dash and its title, written in place on a
    /// click, or «Agregar título de página»; on the right, Subir, Bajar,
    /// Ocultar, Bloquear, Duplicar, Eliminar and Añadir página.
    fn page_header(
        &mut self,
        ui: &mut egui::Ui,
        theme: &Theme,
        texts: &Texts,
        rect: Rect,
        (page, index, count): (&Page, usize, usize),
        asked: &mut Asked,
    ) {
        let mut row = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(rect)
                .layout(Layout::left_to_right(Align::Center)),
        );
        let ui = &mut row;
        let font = theme.font("ui-label");
        let number = if page.hidden {
            plain(texts, "editor.page_hidden")
        } else {
            texts.t(&gafira_doc::msg!(
                "editor.page",
                number = Count(index as u64 + 1)
            ))
        };
        ui.label(
            RichText::new(number)
                .font(font.clone())
                .color(theme.c("text")),
        );
        ui.label(
            RichText::new(" - ")
                .font(font.clone())
                .color(theme.c("text.secondary")),
        );
        let renaming = self
            .renaming_page
            .as_mut()
            .filter(|(id, _, _)| *id == page.id);
        if let Some((_, words, focused)) = renaming {
            let field = ui.add(
                egui::TextEdit::singleline(words)
                    .font(font.clone())
                    .desired_width(220.0)
                    .hint_text(plain(texts, "editor.page_title_hint")),
            );
            if !*focused {
                field.request_focus();
                *focused = true;
            } else if field.lost_focus() {
                // Enter or a click elsewhere keeps it; Esc does not.
                if !ui.input(|i| i.key_pressed(Key::Escape)) && words.trim() != page.name {
                    asked.page_action = Some(PageAction::Rename(page.id, words.clone()));
                }
                self.renaming_page = None;
            }
        } else {
            let empty = page.name.is_empty();
            let (words, color) = if empty {
                (
                    plain(texts, "editor.page_title_hint"),
                    theme.c("text.secondary"),
                )
            } else {
                (page.name.clone(), theme.c("text"))
            };
            let title = ui
                .add(
                    egui::Label::new(RichText::new(words).font(font).color(color))
                        .sense(egui::Sense::click()),
                )
                .on_hover_cursor(egui::CursorIcon::Text);
            if empty {
                // A dotted line under the hint, like a field to fill.
                let y = title.rect.bottom() + 1.0;
                ui.painter().extend(egui::Shape::dashed_line(
                    &[
                        Pos2::new(title.rect.left(), y),
                        Pos2::new(title.rect.right(), y),
                    ],
                    Stroke::new(1.0, theme.c("text.secondary")),
                    3.0,
                    2.0,
                ));
            }
            if title.clicked() {
                self.renaming_page = Some((page.id, page.name.clone(), false));
            }
        }
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            let button = |ui: &mut egui::Ui, icon, key: &'static str, enabled: bool| {
                ui.add_enabled_ui(enabled, |ui| {
                    IconButton::new(icon, plain(texts, key))
                        .small()
                        .show(ui, theme)
                })
                .inner
                .clicked()
            };
            let id = page.id;
            if button(ui, Icon::PageAdd, "page.add", true) {
                asked.page_action = Some(PageAction::AddAfter(id));
            }
            if button(ui, Icon::Delete, "page.delete", count > 1) {
                asked.page_action = Some(PageAction::Delete(id));
            }
            if button(ui, Icon::PageCopy, "page.duplicate", true) {
                asked.page_action = Some(PageAction::Duplicate(id));
            }
            let (icon, key) = if page.locked {
                (Icon::Locked, "page.unlock")
            } else {
                (Icon::Unlocked, "page.lock")
            };
            if button(ui, icon, key, true) {
                asked.page_action = Some(PageAction::Lock(id, !page.locked));
            }
            let (icon, key) = if page.hidden {
                (Icon::Hidden, "page.show")
            } else {
                (Icon::Visible, "page.hide")
            };
            if button(ui, icon, key, true) {
                asked.page_action = Some(PageAction::Hide(id, !page.hidden));
            }
            if button(ui, Icon::Expand, "page.down", index + 1 < count) {
                asked.page_action = Some(PageAction::Move(id, 1));
            }
            if button(ui, Icon::PageUp, "page.up", index > 0) {
                asked.page_action = Some(PageAction::Move(id, -1));
            }
        });
    }

    /// An action of a page header (spec 056), one undo step each: every
    /// command has a gesture of its own, so two quick ones never merge.
    fn page_action(&mut self, engine: &mut Gafira, action: &PageAction) {
        let Some(document) = engine.edit().document() else {
            return;
        };
        let pages = document.pages();
        let index_of = |id: PageId| pages.iter().position(|p| p.id == id);
        let gesture = engine.edit().begin_gesture().ok();
        let command: Box<dyn Command> = match action {
            PageAction::Move(page, step) => {
                let Some(to) = index_of(*page)
                    .and_then(|at| at.checked_add_signed(*step))
                    .filter(|to| *to < pages.len())
                else {
                    let _ = engine.edit().end_gesture();
                    return;
                };
                Box::new(gafira_edit::ReorderPage::new(*page, to))
            }
            PageAction::Hide(page, on) => {
                Box::new(SetProperty::page(*page, Property::PageHidden(*on), gesture))
            }
            PageAction::Lock(page, on) => {
                Box::new(SetProperty::page(*page, Property::PageLocked(*on), gesture))
            }
            PageAction::Rename(page, name) => Box::new(SetProperty::page(
                *page,
                Property::PageName(name.trim().to_owned()),
                gesture,
            )),
            PageAction::Duplicate(page) => Box::new(gafira_edit::DuplicatePage::new(*page)),
            PageAction::Delete(page) => {
                if pages.len() < 2 {
                    let _ = engine.edit().end_gesture();
                    return;
                }
                Box::new(gafira_edit::RemovePage::new(*page))
            }
            PageAction::AddAfter(page) => {
                let Some(at) = index_of(*page) else {
                    let _ = engine.edit().end_gesture();
                    return;
                };
                new_page(&document, at + 1, pages[at].size)
            }
        };
        // A new page takes the next id of the document.
        let next = PageId::new(document.next_id());
        match engine.edit().execute(command) {
            Ok(_) => match action {
                PageAction::Duplicate(_) | PageAction::AddAfter(_) => {
                    self.active = Some(next);
                    self.selected = Selection::default();
                    self.scope = None;
                }
                PageAction::Delete(page) => {
                    if self.selected.page() == Some(*page) {
                        self.selected = Selection::default();
                    }
                    if self.active == Some(*page) {
                        self.active = None;
                    }
                    self.scope = None;
                }
                _ => {}
            },
            Err(error) => eprintln!("page: {error}"),
        }
        let _ = engine.edit().end_gesture();
    }

    /// `family` in `bold` and `italic` for every span of the selected text,
    /// or of the text of the selected shape, one undo step (spec 062).
    fn apply_font(&mut self, engine: &mut Gafira, (family, bold, italic): (&str, bool, bool)) {
        let Some((page, node)) = self.selected.single() else {
            return;
        };
        let Some(document) = engine.edit().document() else {
            return;
        };
        let Some(found) = document.page(page).and_then(|p| p.node(node)) else {
            return;
        };
        let restyle = |content: &gafira_doc::RichText| {
            let mut content = content.clone();
            for span in content
                .paragraphs
                .iter_mut()
                .flat_map(|p| p.spans.iter_mut())
            {
                span.style.font = family.to_owned();
                span.style.bold = bold;
                span.style.italic = italic;
            }
            content
        };
        if matches!(found.kind, gafira_doc::NodeKind::Table(_)) {
            let font = tables::Act::Font(family.to_owned(), bold, italic);
            self.change_table(engine, &font, None);
            return;
        }
        let property = match &found.kind {
            gafira_doc::NodeKind::Text(text) => Property::TextContent(restyle(&text.content)),
            gafira_doc::NodeKind::Shape(shape) => match &shape.text {
                Some(text) => {
                    let mut text = text.clone();
                    text.content = restyle(&text.content);
                    Property::ShapeText(Some(text))
                }
                None => return,
            },
            _ => return,
        };
        let gesture = engine.edit().begin_gesture().ok();
        let command = SetProperty::new(page, vec![node], property, gesture);
        if let Err(error) = engine.edit().execute(Box::new(command)) {
            eprintln!("font: {error}");
        }
        if gesture.is_some() {
            let _ = engine.edit().end_gesture();
        }
    }

    /// The Fuente panel (spec 062): a search box, then the families of the
    /// document, the fonts of Gafira (each in its own face, with a sample
    /// and, opened, its faces) and the fonts of the system (by name). The
    /// list is one column of rows of the same height, and only the rows in
    /// view are drawn, however many fonts the system has. A click gives the
    /// selected text that family or face.
    fn fonts_panel(&self, ui: &mut egui::Ui, theme: &Theme, texts: &Texts, asked: &mut Asked) {
        let current = self.selection_font.clone();
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(plain(texts, "fonts.title"))
                    .font(theme.font("ui-label"))
                    .color(theme.c("text")),
            );
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui.button(plain(texts, "fonts.close")).clicked() {
                    asked.font_panel = Some(false);
                }
            });
        });
        ui.add_space(8.0);
        let search_id = ui.id().with("fonts.search");
        let mut search: String = ui.data_mut(|d| d.get_temp(search_id).unwrap_or_default());
        ui.add(widgets::field(&mut search, plain(texts, "fonts.search")));
        ui.data_mut(|d| d.insert_temp(search_id, search.clone()));
        let open_id = ui.id().with("fonts.open");
        let mut open: BTreeSet<String> = ui.data_mut(|d| d.get_temp(open_id).unwrap_or_default());
        let system = system_fonts::catalogue();
        let rows = font_rows(&search, &self.document_fonts, &open, &system);
        ui.add_space(8.0);
        egui::ScrollArea::vertical().show_rows(ui, FONT_ROW_HEIGHT, rows.len(), |ui, range| {
            for row in &rows[range] {
                match row {
                    PanelRow::Title(key) => {
                        let (rect, _) = ui.allocate_exact_size(
                            Vec2::new(ui.available_width(), FONT_ROW_HEIGHT),
                            egui::Sense::hover(),
                        );
                        ui.painter().text(
                            Pos2::new(rect.min.x + 4.0, rect.center().y),
                            Align2::LEFT_CENTER,
                            plain(texts, key),
                            theme.font("ui-label"),
                            theme.c("text"),
                        );
                    }
                    PanelRow::Family {
                        family,
                        faces,
                        system,
                    } => {
                        let is_open = open.contains(family);
                        let face = if *system {
                            egui::FontFamily::Proportional
                        } else {
                            library_family(ui.ctx(), &gafira_theme::face_key(family, false, false))
                        };
                        let clicked = font_row(
                            ui,
                            theme,
                            FontRow {
                                indent: 0.0,
                                chevron: (faces.len() > 1).then_some(is_open),
                                name: family.clone(),
                                face,
                                sample: !*system,
                                ticked: current.as_ref().is_some_and(|c| &c.0 == family),
                            },
                        );
                        match clicked {
                            Some(RowClick::Chevron) => {
                                if is_open {
                                    open.remove(family);
                                } else {
                                    open.insert(family.clone());
                                }
                            }
                            Some(RowClick::Row) => {
                                let (bold, italic) = current
                                    .as_ref()
                                    .filter(|c| &c.0 == family)
                                    .map_or((false, false), |c| (c.1, c.2));
                                asked.font = Some((family.clone(), bold, italic));
                            }
                            None => {}
                        }
                    }
                    PanelRow::Face {
                        family,
                        bold,
                        italic,
                        system,
                    } => {
                        let key = match (bold, italic) {
                            (false, false) => "fonts.regular",
                            (true, false) => "fonts.bold",
                            (false, true) => "fonts.italic",
                            (true, true) => "fonts.bold_italic",
                        };
                        let face = if *system {
                            egui::FontFamily::Proportional
                        } else {
                            library_family(
                                ui.ctx(),
                                &gafira_theme::face_key(family, *bold, *italic),
                            )
                        };
                        let clicked = font_row(
                            ui,
                            theme,
                            FontRow {
                                indent: 28.0,
                                chevron: None,
                                name: plain(texts, key),
                                face,
                                sample: false,
                                ticked: current.as_ref().is_some_and(|c| {
                                    &c.0 == family && c.1 == *bold && c.2 == *italic
                                }),
                            },
                        );
                        if clicked.is_some() {
                            asked.font = Some((family.clone(), *bold, *italic));
                        }
                    }
                    PanelRow::Note(key) => {
                        let (rect, _) = ui.allocate_exact_size(
                            Vec2::new(ui.available_width(), FONT_ROW_HEIGHT),
                            egui::Sense::hover(),
                        );
                        ui.painter().text(
                            Pos2::new(rect.min.x + 8.0, rect.center().y),
                            Align2::LEFT_CENTER,
                            plain(texts, key),
                            theme.font("body-sm"),
                            theme.c("text.secondary"),
                        );
                    }
                }
            }
        });
        ui.data_mut(|d| d.insert_temp(open_id, open));
    }

    /// The faces of the system families the document uses, installed in
    /// egui the first time they are needed (spec 062): the canvas then draws
    /// those texts in their own face. A family of Gafira is already there.
    fn load_system_fonts(&self, ctx: &egui::Context) {
        let system = system_fonts::catalogue();
        if !system.ready() {
            return;
        }
        let mut loaded = self.loaded_fonts.borrow_mut();
        for family in &self.document_fonts {
            if loaded.contains(family) {
                continue;
            }
            loaded.insert(family.clone());
            if gafira_theme::library().iter().any(|f| f.family == family) {
                continue;
            }
            for face in system.faces().iter().filter(|f| &f.family == family) {
                let Ok(bytes) = std::fs::read(&face.path) else {
                    continue;
                };
                let key = gafira_theme::face_key(&face.family, face.bold, face.italic);
                ctx.add_font(egui::epaint::text::FontInsert::new(
                    &key,
                    egui::FontData::from_owned(bytes),
                    vec![egui::epaint::text::InsertFontFamily {
                        family: egui::FontFamily::Name(key.clone().into()),
                        priority: egui::epaint::text::FontPriority::Highest,
                    }],
                ));
            }
        }
    }

    /// Writing on the canvas and the alignment of a shape's text (spec
    /// 055). A whole edit is one undo step: every change carries the
    /// gesture opened when it began.
    fn text_editing(&mut self, engine: &mut Gafira, asked: &Asked, theme: &Theme) {
        let ink = canvas::document_color(crate::style::color32(theme.tokens.brand.ink));
        let default = gafira_doc::SpanStyle {
            font: theme.tokens.typography.ui.clone(),
            size: 24.0,
            color: ink,
            bold: false,
            italic: false,
        };
        if let Some((page, node)) = asked.begin_edit {
            if self.editing.take().is_some_and(|e| e.gesture.is_some()) {
                let _ = engine.edit().end_gesture();
            }
            let Some(document) = engine.edit().document() else {
                return;
            };
            let words = match document
                .page(page)
                .and_then(|p| p.node(node))
                .map(|n| &n.kind)
            {
                Some(gafira_doc::NodeKind::Text(text)) => text.content.plain_text(),
                Some(gafira_doc::NodeKind::Shape(shape)) => shape
                    .text
                    .as_ref()
                    .map(|text| text.content.plain_text())
                    .unwrap_or_default(),
                _ => return,
            };
            let gesture = engine.edit().begin_gesture().ok();
            self.editing = Some(Editing {
                page,
                node,
                words,
                gesture,
                focused: false,
            });
            self.selected = Selection::one(page, node);
        }
        if let Some(words) = &asked.edit_text
            && let Some(editing) = self.editing.as_mut()
            && let Some(document) = engine.edit().document()
            && let Some(node) = document
                .page(editing.page)
                .and_then(|p| p.node(editing.node))
        {
            editing.words.clone_from(words);
            let property = match &node.kind {
                gafira_doc::NodeKind::Text(text) => {
                    let style = text.content.first_style().cloned().unwrap_or(default);
                    // The words and the height of the box in one command,
                    // so the whole edit stays one undo step (spec 061).
                    let fitted = asked
                        .edit_height
                        .and_then(|h| canvas::text_fit(text.sizing, &node.base.placement, h))
                        .unwrap_or(node.base.placement);
                    let commands: Vec<Box<dyn Command>> = vec![
                        Box::new(SetProperty::new(
                            editing.page,
                            vec![editing.node],
                            Property::TextContent(gafira_doc::RichText::plain(words, &style)),
                            editing.gesture,
                        )),
                        Box::new(SetProperty::new(
                            editing.page,
                            vec![editing.node],
                            Property::Placement(fitted),
                            editing.gesture,
                        )),
                    ];
                    if let Err(error) = engine.edit().execute(Box::new(Transaction::new(
                        MessageKey("edit.write"),
                        commands,
                    ))) {
                        eprintln!("write: {error}");
                    }
                    None
                }
                gafira_doc::NodeKind::Shape(shape) => {
                    // An empty shape keeps no text (S55-04).
                    Some(Property::ShapeText((!words.is_empty()).then(|| {
                        let old = shape.text.as_ref();
                        let style = old
                            .and_then(|text| text.content.first_style().cloned())
                            .unwrap_or(default);
                        let mut text = gafira_doc::ShapeText::plain(words, &style);
                        if let Some(old) = old {
                            text.align = old.align;
                            text.valign = old.valign;
                        }
                        text
                    })))
                }
                _ => None,
            };
            if let Some(property) = property {
                let command =
                    SetProperty::new(editing.page, vec![editing.node], property, editing.gesture);
                if let Err(error) = engine.edit().execute(Box::new(command)) {
                    eprintln!("write: {error}");
                }
            }
        }
        if asked.end_edit
            && let Some(editing) = self.editing.take()
            && editing.gesture.is_some()
            && let Err(error) = engine.edit().end_gesture()
        {
            eprintln!("write: {error}");
        }
        if let Some((align, valign)) = asked.shape_align
            && let Some((page, node)) = self.selected.single()
            && let Some(document) = engine.edit().document()
            && let Some(gafira_doc::NodeKind::Shape(shape)) = document
                .page(page)
                .and_then(|p| p.node(node))
                .map(|n| &n.kind)
            && let Some(mut text) = shape.text.clone()
        {
            text.align = align.unwrap_or(text.align);
            text.valign = valign.unwrap_or(text.valign);
            let gesture = engine.edit().begin_gesture().ok();
            let command =
                SetProperty::new(page, vec![node], Property::ShapeText(Some(text)), gesture);
            if let Err(error) = engine.edit().execute(Box::new(command)) {
                eprintln!("align text: {error}");
            }
            if gesture.is_some() {
                let _ = engine.edit().end_gesture();
            }
        }
    }

    /// The field of the text being written on the canvas (spec 055): over
    /// the element's box (a shape's box less its margin), in its size,
    /// colour and alignment, without a frame, so the words seem to be
    /// written in place. Each change asks to store the words; losing the
    /// keyboard (Esc or a click elsewhere) ends the edit.
    fn edit_field(
        &mut self,
        ui: &mut egui::Ui,
        theme: &Theme,
        pages: &[Arc<Page>],
        page_rects: &[(PageId, Rect)],
        zoom: f32,
        asked: &mut Asked,
    ) {
        let Some(editing) = self.editing.as_mut() else {
            return;
        };
        let Some((page, rect)) = pages.iter().find(|p| p.id == editing.page).zip(
            page_rects
                .iter()
                .find(|(id, _)| *id == editing.page)
                .map(|r| r.1),
        ) else {
            return;
        };
        let Some(node) = page.node(editing.node) else {
            asked.end_edit = true;
            return;
        };
        let at = rotation::screen_box(rect, zoom, &node.base.placement);
        let (field_rect, style, align, valign) = match &node.kind {
            gafira_doc::NodeKind::Text(text) => {
                (at, text.content.first_style().cloned(), text.align, None)
            }
            gafira_doc::NodeKind::Shape(shape) => (
                canvas::text_box(at),
                shape
                    .text
                    .as_ref()
                    .and_then(|t| t.content.first_style().cloned()),
                shape
                    .text
                    .as_ref()
                    .map_or(gafira_doc::TextAlign::Center, |t| t.align),
                Some(
                    shape
                        .text
                        .as_ref()
                        .map_or(gafira_doc::VerticalAlign::Middle, |t| t.valign),
                ),
            ),
            _ => {
                asked.end_edit = true;
                return;
            }
        };
        let size = style.as_ref().map_or(24.0, |s| s.size) as f32 * zoom;
        let color = style.as_ref().map_or_else(
            || crate::style::color32(theme.tokens.brand.ink),
            |s| canvas::screen_color(s.color),
        );
        let family = style.as_ref().map_or(egui::FontFamily::Proportional, |s| {
            text_family(ui.ctx(), theme, s)
        });
        let halign = match align {
            gafira_doc::TextAlign::Center => Align::Center,
            gafira_doc::TextAlign::Right => Align::RIGHT,
            gafira_doc::TextAlign::Left | gafira_doc::TextAlign::Justify => Align::LEFT,
        };
        let vertical = match valign {
            Some(gafira_doc::VerticalAlign::Top) | None => Align::TOP,
            Some(gafira_doc::VerticalAlign::Middle) => Align::Center,
            Some(gafira_doc::VerticalAlign::Bottom) => Align::BOTTOM,
        };
        let field = egui::TextEdit::multiline(&mut editing.words)
            .frame(false)
            .background_color(Color32::TRANSPARENT)
            .font(egui::FontId::new(size.max(1.0), family))
            .text_color(color)
            .horizontal_align(halign)
            .vertical_align(vertical)
            .desired_width(field_rect.width());
        let mut height = None;
        let response = ui.put(field_rect, |ui: &mut egui::Ui| {
            let output = field.show(ui);
            height = Some(output.galley.size().y);
            output.response
        });
        // The height of the words, for a text box that grows with them.
        if let (Some(height), gafira_doc::NodeKind::Text(_)) = (height, &node.kind) {
            asked.edit_height = Some(f64::from(height / zoom));
        }
        if !editing.focused {
            response.request_focus();
            editing.focused = true;
        } else if response.lost_focus() {
            asked.end_edit = true;
        }
        if response.changed() {
            asked.edit_text = Some(editing.words.clone());
        }
    }

    /// Ocultar, Bloquear, a new name and Alt+↑/↓ from Capas and the
    /// shortcuts (spec 048), each one undo step.
    fn layer_changes(&self, engine: &mut Gafira, asked: &Asked) {
        let log = |what: &str, result: Result<(), gafira_engine::EngineError>| {
            if let Err(error) = result {
                eprintln!("{what}: {error}");
            }
        };
        let mut flag = asked.flag.clone();
        if let Some(which) = asked.toggle
            && let Some(document) = engine.edit().document()
            && let Some(page) = self.selected.page().and_then(|id| document.page(id))
            && let Some(on) = layers::toggled(page, self.selected.nodes(), which)
        {
            flag = Some((page.id, self.selected.nodes().to_vec(), which, on));
        }
        if let Some((page, nodes, which, on)) = flag {
            let property = match which {
                Flag::Hidden => Property::Visible(!on),
                Flag::Locked => Property::Locked(on),
            };
            // Its own gesture: never merged with an earlier change (one
            // undo step per click or shortcut).
            let gesture = engine.edit().begin_gesture().ok();
            let command = SetProperty::new(page, nodes, property, gesture);
            log("layers", engine.edit().execute(Box::new(command)).map(drop));
        }
        if let Some((page, node, name)) = asked.rename.clone() {
            let gesture = engine.edit().begin_gesture().ok();
            let command = SetProperty::new(page, vec![node], Property::Name(name), gesture);
            log("rename", engine.edit().execute(Box::new(command)).map(drop));
        }
        if let Some(step) = asked.step
            && let Some((page, node)) = self.selected.single()
            && let Some(document) = engine.edit().document()
            && let Some(on) = document.page(page)
            && let Some((_, list)) = group::siblings(on, &[node])
            && let Some(index) = list.iter().position(|id| *id == node)
            && let Some(to) = index.checked_add_signed(step)
            && to < list.len()
        {
            let command = gafira_edit::Reorder::new(page, node, to);
            log(
                "reorder",
                engine.edit().execute(Box::new(command)).map(drop),
            );
        }
    }

    fn canvas(
        &mut self,
        ctx: &egui::Context,
        theme: &Theme,
        texts: &Texts,
        document: Option<&Document>,
        asked: &mut Asked,
    ) {
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(theme.c("workspace")))
            .show(ctx, |ui| {
                let full = ui.max_rect();
                // The rulers sit outside the canvas, along its top and its
                // left (S49-01): the canvas starts after them.
                let area = if self.shown.rulers {
                    Rect::from_min_max(full.min + Vec2::splat(ruler::RULER), full.max)
                } else {
                    full
                };
                // Ctrl (Cmd) + wheel and a pinch zoom (egui turns them into
                // a zoom factor and leaves the scroll alone); the page point
                // under the pointer stays under it (spec 059, FD-08).
                let (factor, hover) = ui.input(|i| (i.zoom_delta(), i.pointer.hover_pos()));
                let sizes = document.map(page_sizes).unwrap_or_default();
                // The scroll offset that keeps the page point under the
                // pointer under it, set in this very frame so nothing jumps.
                let mut anchored: Option<Vec2> = None;
                if factor != 1.0
                    && let Some(at) = hover
                    && area.contains(at)
                {
                    let next = zoom::by_factor(self.zoom, factor);
                    let ids: Vec<PageId> = document
                        .map(|d| d.pages().iter().map(|p| p.id).collect())
                        .unwrap_or_default();
                    if let Some(page) = ruler::page_at(&self.page_rects, at.y)
                        && let Some((_, rect)) = self.page_rects.iter().find(|(id, _)| *id == page)
                        && let Some(index) = ids.iter().position(|id| *id == page)
                    {
                        let point = (at - rect.min) / self.zoom;
                        let origin = zoom::page_origin(
                            &sizes,
                            index,
                            next,
                            area.size(),
                            (CANVAS_MARGIN, PAGE_GAP),
                        );
                        let offset = origin + point * next - (at - area.min);
                        anchored = Some(offset.max(Vec2::ZERO));
                    }
                    self.view = Zoom::Fixed(next);
                }
                let Some(first) = sizes.first().copied() else {
                    return;
                };
                self.zoom = self.view.resolve(fit_zoom(
                    (first.x, first.y),
                    (area.width(), area.height()),
                    CANVAS_MARGIN,
                ));
                let zoom = self.zoom;
                let dpi = document.map_or(96.0, |d| f64::from(d.dpi().get()));
                // Each page's id and screen box, for the rulers and the
                // guides (spec 044).
                let mut page_rects: Vec<(PageId, Rect)> = Vec::new();
                let mut guide_hit: Option<GuideDrag> = None;
                // The cursor of the handle under the pointer or held (spec
                // 045); it wins over a guide's.
                let mut grip_cursor: Option<egui::CursorIcon> = None;
                let width =
                    sizes.iter().map(|s| s.x).fold(0.0, f32::max) * zoom + 2.0 * CANVAS_MARGIN;
                // Room under the last page for «Añadir página» (spec 077).
                let height: f32 = sizes.iter().map(|s| s.y * zoom + PAGE_GAP).sum::<f32>()
                    + CANVAS_MARGIN
                    + ADD_PAGE_ROOM;
                ui.scope_builder(egui::UiBuilder::new().max_rect(area), |ui| {
                    let mut scroll = egui::ScrollArea::both().auto_shrink([false, false]);
                    if let Some(offset) = anchored {
                        scroll = scroll.scroll_offset(offset);
                    }
                    scroll.show(ui, |ui| {
                        let (rect, response) = ui.allocate_exact_size(
                            Vec2::new(width.max(area.width()), height.max(area.height())),
                            egui::Sense::click_and_drag(),
                        );
                        let painter = &ui.painter().clone();
                        let pages = document.map(Document::pages).unwrap_or_default();
                        // A drag starts only once the pointer has moved a few
                        // px; what it grabbed is decided where it was pressed.
                        let pointer = if response.drag_started() {
                            ui.input(|input| input.pointer.press_origin())
                                .or_else(|| response.interact_pointer_pos())
                        } else {
                            response.interact_pointer_pos()
                        };
                        // In crop mode a drag moves the photo, not the
                        // elements (S66-07).
                        let pressed =
                            (response.clicked() || response.drag_started()) && self.cropping.is_none();
                        if self.cropping.is_some() && response.dragged() {
                            asked.crop_drag = Some(response.drag_delta());
                        }
                        let mut crop_kept = false;
                        let (shift, press) =
                            ui.input(|i| (i.modifiers.shift, i.pointer.press_origin()));
                        let single = self.selected.single();
                        let mut picked: Option<(PageId, NodeId)> = None;
                        let mut grabbed: Option<Hold> = None;
                        // A press a table keeps for its cells (spec 081).
                        let mut on_cells = false;
                        // Each page's id, screen box and shapes, for the marquee.
                        let mut laid: Vec<(PageId, Rect, Vec<Shape>)> = Vec::new();
                        let selection = theme.c("canvas.selection");
                        for (index, size) in sizes.iter().enumerate() {
                            // Where the zoom expects it (spec 059).
                            let origin = zoom::page_origin(
                                &sizes,
                                index,
                                zoom,
                                area.size(),
                                (CANVAS_MARGIN, PAGE_GAP),
                            );
                            let page = Rect::from_min_size(rect.min + origin, *size * zoom);
                            if let Some(Shadow::Drop {
                                x,
                                y,
                                blur,
                                color,
                                alpha,
                            }) = theme.tokens.shadow.get(&1).copied()
                            {
                                let shadow = egui::epaint::Shadow {
                                    offset: [to_i8(x), to_i8(y)],
                                    blur: to_u8(blur),
                                    spread: 0,
                                    color: color32(color).gamma_multiply(alpha),
                                };
                                painter.add(shadow.as_shape(page, 0.0));
                            }
                            // The page colour belongs to the document (white
                            // by default, 03 §7.1), not to the theme.
                            painter.rect_filled(page, 0.0, Color32::WHITE);
                            // Its background, as the export draws it (spec 069).
                            if let Some(background) =
                                pages.get(index).and_then(|p| p.background.as_ref())
                            {
                                self.paint_background(ctx, painter, page, zoom, background);
                            }
                            // The active page is ringed (S50-02).
                            if self.active.is_some()
                                && self.active == pages.get(index).map(|p| p.id)
                            {
                                painter.rect_stroke(
                                    page,
                                    0.0,
                                    Stroke::new(2.0, selection),
                                    egui::StrokeKind::Outside,
                                );
                            }
                            // The header over the page (spec 056).
                            if let Some(page_doc) = pages.get(index) {
                                let header = Rect::from_min_max(
                                    Pos2::new(page.min.x, page.min.y - 36.0),
                                    Pos2::new(page.max.x, page.min.y - 4.0),
                                );
                                self.page_header(
                                    ui,
                                    theme,
                                    texts,
                                    header,
                                    (page_doc, index, pages.len()),
                                    asked,
                                );
                                // «Añadir página» under the last page.
                                if index + 1 == pages.len() {
                                    let bar = Rect::from_min_size(
                                        Pos2::new(page.min.x, page.max.y + 16.0),
                                        Vec2::new(page.width().max(220.0), 40.0),
                                    );
                                    add_page_bar(ui, theme, texts, bar, page_doc.id, asked);
                                }
                            }
                            if let Some(page_doc) = pages.get(index) {
                                let mut shapes = canvas::shapes(page_doc);
                                // The strokes the eraser has gone over fade
                                // until it lets go (spec 079).
                                if let Some((page, erased)) = &self.draw.borrow().erasing
                                    && *page == page_doc.id
                                {
                                    for shape in &mut shapes {
                                        if erased.contains(&shape.node) {
                                            shape.opacity *= 0.2;
                                        }
                                    }
                                }
                                // An image dragged over a frame: the frame
                                // shows the photo, the image fades (spec 066).
                                if let Some(target) =
                                    self.frame_target.filter(|t| t.page == page_doc.id)
                                    && let Some(Some(photo)) =
                                        self.photos.get(&target.asset.as_hash())
                                {
                                    for shape in &mut shapes {
                                        if shape.node == target.image {
                                            shape.opacity *= 0.35;
                                        }
                                        if shape.node == target.frame
                                            && let Some(frame) = shape.frame.as_mut()
                                        {
                                            frame.content = Some(gafira_doc::FrameContent {
                                                asset: target.asset,
                                                natural: (
                                                    photo.size[0] as u32,
                                                    photo.size[1] as u32,
                                                ),
                                            });
                                            shape.fill = None;
                                        }
                                    }
                                }
                                // A click away from the frame in crop mode
                                // ends it (S66-07).
                                if let Some(crop) = self.cropping
                                    && crop.page == page_doc.id
                                    && response.clicked()
                                    && let Some(at) = pointer
                                    && canvas::hit(&shapes, page, zoom, at) == Some(crop.node)
                                {
                                    crop_kept = true;
                                }
                                // Each page draws its elements cut to it (D-92).
                                let clip =
                                    painter.with_clip_rect(page.intersect(painter.clip_rect()));
                                for shape in &shapes {
                                    let at = canvas::on_screen(page, zoom, &shape.placement);
                                    // Where the local box lands on the screen,
                                    // turned and flipped as the render draws it
                                    // (spec 041).
                                    let turn = rotation::to_screen(page, zoom, &shape.placement);
                                    let upright = rotation::upright(&shape.placement);
                                    let outline = rotation::corners(turn, &shape.placement);
                                    if let Some(asset) = shape.image {
                                        let tint = Color32::WHITE
                                            .gamma_multiply(shape.opacity.clamp(0.0, 1.0));
                                        match self
                                            .textures
                                            .get(&asset.as_hash())
                                            .and_then(Option::as_ref)
                                        {
                                            Some(texture) => {
                                                clip.add(canvas::image_mesh(
                                                    texture.id(),
                                                    turn,
                                                    &shape.placement,
                                                    tint,
                                                ));
                                            }
                                            None => {
                                                clip.add(egui::Shape::convex_polygon(
                                                    outline.to_vec(),
                                                    theme.c("surface.sunken"),
                                                    Stroke::NONE,
                                                ));
                                            }
                                        }
                                    }
                                    let edited =
                                        self.editing.as_ref().is_some_and(|e| e.node == shape.node);
                                    // A curved text is drawn from its outline,
                                        // as the export draws it (FD-44).
                                    if let Some(text) = shape
                                        .text
                                        .as_ref()
                                        .filter(|t| !edited && t.styled())
                                    {
                                        let key = curved_key(text, &shape.placement, zoom);
                                        let texture = self.path_texture(ctx, key, || {
                                            let (image, cover) = canvas::curved_text_image(
                                                text,
                                                &shape.placement,
                                                zoom,
                                            )?;
                                            self.curved_covers.borrow_mut().insert(key, cover);
                                            Some(image)
                                        });
                                        let cover = self.curved_covers.borrow().get(&key).copied();
                                        if let (Some(texture), Some(cover)) = (texture, cover) {
                                            clip.add(canvas::image_mesh(
                                                texture,
                                                rotation::to_screen(page, zoom, &cover),
                                                &cover,
                                                Color32::WHITE
                                                    .gamma_multiply(shape.opacity.clamp(0.0, 1.0)),
                                            ));
                                        }
                                        // A box that grows with its words takes the
                                        // height of its lines (spec 061).
                                        let resized = self.drag.as_ref().is_some_and(|d| {
                                            matches!(d.hold, Some(Hold::Resize(_)))
                                                && d.origins.first().map(|o| o.0)
                                                    == Some(shape.node)
                                        });
                                        if (resized
                                            || self.fit_pending == Some((page_doc.id, shape.node)))
                                            && text.sizing == gafira_doc::TextSizing::AutoHeight
                                            && let Some(height) = canvas::curved_height(
                                                text,
                                                shape.placement.w,
                                                shape.placement.h,
                                            )
                                        {
                                            self.text_measure =
                                                Some((page_doc.id, shape.node, height));
                                        }
                                    } else if let Some(text) = shape.text.as_ref().filter(|_| !edited) {
                                        let laid =
                                            lay_text(&clip, theme, at, zoom, shape.opacity, text);
                                        // The height of the words, for a box that
                                        // grows with them (spec 061): a paste, or
                                        // a box being resized.
                                        let resized = self.drag.as_ref().is_some_and(|d| {
                                            matches!(d.hold, Some(Hold::Resize(_)))
                                                && d.origins.first().map(|o| o.0)
                                                    == Some(shape.node)
                                        });
                                        if (resized
                                            || self.fit_pending == Some((page_doc.id, shape.node)))
                                            && let Some(height) = laid_height(&laid, at, zoom)
                                        {
                                            self.text_measure =
                                                Some((page_doc.id, shape.node, height));
                                        }
                                        if upright {
                                            for (pos, galley, color) in laid {
                                                clip.galley(pos, galley, color);
                                            }
                                        } else {
                                            let atlas = ctx.fonts(|f| f.font_image_size());
                                            self.font_atlas = Some(atlas);
                                            clip.add(text_mesh(&laid, at, zoom, turn, atlas));
                                        }
                                    }
                                    // Crop mode: the whole photo, faded, where
                                    // it lies around the frame (S66-07).
                                    if let Some(crop) = self.cropping
                                        && crop.node == shape.node
                                        && let Some(frame) = &shape.frame
                                        && let Some(content) = frame.content
                                        && let Some(Some(photo)) =
                                            self.photos.get(&content.asset.as_hash())
                                        && let Some(bounds) =
                                            frame.photo_bounds(shape.placement.w, shape.placement.h)
                                    {
                                        let p = shape.placement;
                                        let mut whole = gafira_doc::Placement::new(
                                            p.x + bounds.x0,
                                            p.y + bounds.y0,
                                            bounds.width(),
                                            bounds.height(),
                                        );
                                        whole.rotation_deg = frame.content_rotation_deg;
                                        let key = photo_key(content.asset);
                                        let photo = Arc::clone(photo);
                                        if let Some(texture) =
                                            self.path_texture(ctx, key, || Some((*photo).clone()))
                                        {
                                            clip.add(canvas::image_mesh(
                                                texture,
                                                rotation::to_screen(page, zoom, &whole),
                                                &whole,
                                                Color32::WHITE.gamma_multiply(0.35),
                                            ));
                                        }
                                    }
                                    // An empty frame shows the sample photo,
                                    // cut out by its shape (S66-12).
                                    if shape.frame.as_ref().is_some_and(|f| f.content.is_none())
                                        && let Some(sample) = canvas::sample_photo()
                                    {
                                        let shown = canvas::with_sample(shape, &sample);
                                        let key = frame_key(&shown, zoom);
                                        if let Some(texture) = self.path_texture(ctx, key, || {
                                            canvas::frame_image(&shown, &sample, zoom)
                                        }) {
                                            clip.add(canvas::image_mesh(
                                                texture,
                                                rotation::to_screen(page, zoom, &shape.placement),
                                                &shape.placement,
                                                Color32::WHITE.gamma_multiply(shape.alpha()),
                                            ));
                                        }
                                    }
                                    // The photo of a frame, cut out by its
                                    // shape, under its outline (spec 066).
                                    if let Some(content) =
                                        shape.frame.as_ref().and_then(|f| f.content)
                                        && let Some(Some(photo)) =
                                            self.photos.get(&content.asset.as_hash())
                                    {
                                        let key = frame_key(shape, zoom);
                                        if let Some(texture) = self.path_texture(ctx, key, || {
                                            canvas::frame_image(shape, photo, zoom)
                                        }) {
                                            clip.add(canvas::image_mesh(
                                                texture,
                                                rotation::to_screen(page, zoom, &shape.placement),
                                                &shape.placement,
                                                Color32::WHITE.gamma_multiply(shape.alpha()),
                                            ));
                                        }
                                    }
                                    // A table is drawn from its picture,
                                    // without the words being written
                                    // (spec 081).
                                    if let Some(table) = &shape.table {
                                        let (w, h) = (shape.placement.w, shape.placement.h);
                                        let writing = self
                                            .cells
                                            .as_ref()
                                            .filter(|c| c.node == shape.node && c.writing.is_some())
                                            .map(|c| c.from);
                                        let key = tables::key(
                                            table,
                                            (w.to_bits(), h.to_bits(), zoom.to_bits(), writing),
                                        );
                                        let texture = self.path_texture(ctx, key, || {
                                            let mut outlines = self.table_outlines.borrow_mut();
                                            tables::image(
                                                table,
                                                (w, h),
                                                zoom,
                                                writing,
                                                &mut outlines,
                                            )
                                        });
                                        let margin = tables::overhang(table);
                                        let cover = grown(&shape.placement, margin);
                                        if let Some(texture) = texture {
                                            clip.add(canvas::image_mesh(
                                                texture,
                                                rotation::to_screen(page, zoom, &cover),
                                                &cover,
                                                Color32::WHITE.gamma_multiply(shape.alpha()),
                                            ));
                                        }
                                        continue;
                                    }
                                    // A QR code is drawn from its picture
                                    // (spec 080).
                                    if let Some(qr) = &shape.qr {
                                        let logo = qr
                                            .logo
                                            .and_then(|l| self.photos.get(&l.asset.as_hash()))
                                            .and_then(Option::as_deref);
                                        let (w, h) = (shape.placement.w, shape.placement.h);
                                        let key = crate::qr::key(
                                            qr,
                                            (
                                                w.to_bits(),
                                                h.to_bits(),
                                                zoom.to_bits(),
                                                logo.is_some(),
                                            ),
                                        );
                                        if let Some(texture) = self.path_texture(ctx, key, || {
                                            crate::qr::image(qr, (w, h), zoom, logo)
                                        }) {
                                            clip.add(canvas::image_mesh(
                                                texture,
                                                rotation::to_screen(page, zoom, &shape.placement),
                                                &shape.placement,
                                                Color32::WHITE.gamma_multiply(shape.alpha()),
                                            ));
                                        }
                                        continue;
                                    }
                                    // A light is drawn from its picture
                                    // instead of the shape (spec 065).
                                    if shape.effect.is_some() {
                                        let page_px = (
                                            f64::from(page.width() / zoom),
                                            f64::from(page.height() / zoom),
                                        );
                                        let key = light_key(shape, page_px, zoom);
                                        let cover = canvas::light_image_cover(shape, page_px);
                                        let texture = cover.and_then(|_| {
                                            self.path_texture(ctx, key, || {
                                                canvas::light_image(shape, page_px, zoom)
                                                    .map(|(image, _)| image)
                                            })
                                        });
                                        if let (Some(texture), Some(cover)) = (texture, cover) {
                                            clip.add(canvas::image_mesh(
                                                texture,
                                                rotation::to_screen(page, zoom, &cover),
                                                &cover,
                                                Color32::WHITE.gamma_multiply(shape.alpha()),
                                            ));
                                        }
                                        continue;
                                    }
                                    // A path is drawn from its texture, in
                                    // its own axes (spec 054).
                                    if canvas::textured(shape) {
                                        let cover = canvas::path_cover(shape, zoom);
                                        let key = path_key(shape, zoom);
                                        if let Some(texture) = self.path_texture(ctx, key, || {
                                            canvas::path_image(shape, zoom).map(|(image, _)| image)
                                        }) {
                                            clip.add(canvas::image_mesh(
                                                texture,
                                                rotation::to_screen(page, zoom, &cover),
                                                &cover,
                                                Color32::WHITE.gamma_multiply(shape.alpha()),
                                            ));
                                        }
                                    }
                                    // A line: its stroke and caps between its
                                    // resolved ends (spec 058).
                                    if let Some((line, ends)) = &shape.line {
                                        let [a, b] =
                                            ends.map(|p| canvas::point_on_screen(page, zoom, p));
                                        let color = canvas::screen_color(line.color)
                                            .gamma_multiply(shape.alpha());
                                        clip.extend(canvas::line_shapes(line, a, b, zoom, color));
                                        continue;
                                    }
                                    if upright {
                                        clip.add(canvas::painted(shape, at, zoom));
                                    } else {
                                        clip.add(canvas::painted_turned(shape, turn, zoom));
                                    }
                                    // The text inside a shape, over it
                                    // (spec 055).
                                    if let Some(inner) = shape.inner.as_ref().filter(|_| !edited) {
                                        let laid =
                                            lay_inner(&clip, theme, at, zoom, shape.alpha(), inner);
                                        if upright {
                                            for (pos, galley, color) in laid {
                                                clip.galley(pos, galley, color);
                                            }
                                        } else {
                                            let atlas = ctx.fonts(|f| f.font_image_size());
                                            self.font_atlas = Some(atlas);
                                            clip.add(text_mesh(&laid, at, zoom, turn, atlas));
                                        }
                                    }
                                }
                                // A click picks the child of the entered group
                                // that holds the drawn element; a double click
                                // on a group enters it (03 §9.1, spec 043).
                                if pressed
                                    && let Some(at) = pointer
                                    && page.contains(at)
                                    && let Some(leaf) = canvas::hit(&shapes, page, zoom, at)
                                    && let Some((node, scope)) = group::click(
                                        page_doc,
                                        leaf,
                                        self.scope_on(page_doc.id),
                                        response.double_clicked(),
                                    )
                                {
                                    picked = Some((page_doc.id, node));
                                    asked.scope = Some(scope.map(|group| (page_doc.id, group)));
                                    // A double click on a text or a shape
                                    // writes on it (spec 055).
                                    if response.double_clicked()
                                        && page_doc.node(node).is_some_and(|n| {
                                            matches!(
                                                n.kind,
                                                gafira_doc::NodeKind::Text(_)
                                                    | gafira_doc::NodeKind::Shape(_)
                                            )
                                        })
                                    {
                                        asked.begin_edit = Some((page_doc.id, node));
                                    }
                                    // A double click on a frame with a photo
                                    // enters crop mode (S66-07).
                                    if response.double_clicked()
                                        && page_doc.node(node).is_some_and(|n| {
                                            matches!(
                                                &n.kind,
                                                gafira_doc::NodeKind::Frame(f) if f.content.is_some()
                                            )
                                        })
                                    {
                                        asked.begin_crop = Some((page_doc.id, node));
                                    }
                                }
                                // Page guides, and the smart guides of the
                                // magnet while it holds (FD-03, FD-06).
                                if self.shown.guides {
                                    // The guide being dragged is drawn where
                                    // it goes, not where it was (S60-03).
                                    let held = self
                                        .guide_drag
                                        .filter(|g| g.page == page_doc.id)
                                        .and_then(|g| g.index);
                                    draw_guides(
                                        painter,
                                        theme,
                                        page_doc,
                                        dpi,
                                        (page, rect),
                                        zoom,
                                        held,
                                    );
                                }
                                if let Some((_, snap)) =
                                    self.snapped.filter(|(on, _)| *on == page_doc.id)
                                {
                                    draw_snap(painter, theme, &snap, page, zoom);
                                }
                                if self.shown.guides
                                    && response.drag_started()
                                    && let Some(at) = pointer
                                    && let Some(index) = ruler::hit(page_doc, dpi, page, zoom, at)
                                {
                                    guide_hit = Some(GuideDrag {
                                        page: page_doc.id,
                                        axis: page_doc.guides.0[index].axis,
                                        index: Some(index),
                                    });
                                }
                                if self.shown.guides
                                    && self.guide_drag.is_none()
                                    && grip_cursor.is_none()
                                    && let Some(at) = response.hover_pos()
                                    && let Some(index) = ruler::hit(page_doc, dpi, page, zoom, at)
                                {
                                    ui.ctx().set_cursor_icon(guide_cursor(
                                        page_doc.guides.0[index].axis,
                                    ));
                                }
                                page_rects.push((page_doc.id, page));
                                laid.push((page_doc.id, page, shapes));
                            }
                        }
                        // The selection, over every page and the gaps
                        // between them (D-92, S51-02).
                        for (id, page) in page_rects.clone() {
                            let Some(page_doc) = pages.iter().find(|p| p.id == id) else {
                                continue;
                            };
                            let page_doc: &Page = page_doc;
                            // The selection, on top of what is drawn: each
                            // selected element by its own box, a group by
                            // the box around its children (spec 043).
                            let chosen = chosen(page_doc, &self.selected);
                            for shape in &chosen {
                                // A line shows its two ends, which can be
                                // dragged (spec 058).
                                if let Some((_, ends)) = &shape.line {
                                    let [a, b] =
                                        ends.map(|p| canvas::point_on_screen(page, zoom, p));
                                    painter.line_segment([a, b], Stroke::new(1.0, selection));
                                    if single == Some((shape.page, shape.node)) && !shape.locked {
                                        for (which, at) in [a, b].into_iter().enumerate() {
                                            painter.circle(
                                                at,
                                                canvas::HANDLE_HALF,
                                                theme.c("surface"),
                                                Stroke::new(1.5, selection),
                                            );
                                            let near = |p: Option<Pos2>| {
                                                p.is_some_and(|p| {
                                                    p.distance(at) <= canvas::HANDLE_HALF * 2.5
                                                })
                                            };
                                            if response.drag_started() && near(pointer) {
                                                grabbed = Some(Hold::LineEnd(which));
                                            }
                                            if near(response.hover_pos()) {
                                                grip_cursor = Some(egui::CursorIcon::Grab);
                                            }
                                        }
                                    }
                                    continue;
                                }
                                let at = canvas::on_screen(page, zoom, &shape.placement);
                                let turn = rotation::to_screen(page, zoom, &shape.placement);
                                let upright = rotation::upright(&shape.placement);
                                let outline = rotation::corners(turn, &shape.placement);
                                if upright {
                                    painter.rect_stroke(
                                        at.expand(1.0),
                                        0.0,
                                        Stroke::new(2.0, selection),
                                        egui::StrokeKind::Outside,
                                    );
                                } else {
                                    painter.add(egui::Shape::closed_line(
                                        outline.to_vec(),
                                        Stroke::new(2.0, selection),
                                    ));
                                }
                                // Only a single, free selection resizes or
                                // turns by hand (D-91).
                                if single == Some((shape.page, shape.node)) && !shape.locked {
                                    let knob = rotation::rotate_handle(turn, &shape.placement);
                                    let top = canvas::handle_point(
                                        turn,
                                        &shape.placement,
                                        canvas::Handle { x: 0, y: -1 },
                                    );
                                    painter.line_segment([top, knob], Stroke::new(1.0, selection));
                                    painter.circle(
                                        knob,
                                        canvas::HANDLE_HALF,
                                        theme.c("surface"),
                                        Stroke::new(1.5, selection),
                                    );
                                    for handle in canvas::HANDLES.into_iter().filter(|h| {
                                        shape.text.is_none() || crate::textbox::shown_on_text(*h)
                                    }) {
                                        let square = Rect::from_center_size(
                                            canvas::handle_point(turn, &shape.placement, handle),
                                            Vec2::splat(canvas::HANDLE_HALF * 2.0),
                                        );
                                        painter.rect(
                                            square,
                                            2.0,
                                            theme.c("surface"),
                                            Stroke::new(1.5, selection),
                                            egui::StrokeKind::Inside,
                                        );
                                    }
                                    // The same hit test as the grab: the
                                    // nearest handle within 24 px.
                                    grip_cursor = match self.drag.as_ref().map(|d| d.hold) {
                                        Some(Some(Hold::Resize(handle))) => {
                                            Some(resize::cursor(turn, handle))
                                        }
                                        Some(Some(Hold::Rotate { .. } | Hold::LineEnd(_))) => {
                                            Some(egui::CursorIcon::Grabbing)
                                        }
                                        Some(None) => grip_cursor,
                                        None => response
                                            .hover_pos()
                                            .and_then(|at| {
                                                canvas::grip_hit(
                                                    turn,
                                                    &shape.placement,
                                                    at,
                                                    shape.text.is_some(),
                                                )
                                            })
                                            .map(|grip| match grip {
                                                canvas::Grip::Resize(handle) => {
                                                    resize::cursor(turn, handle)
                                                }
                                                canvas::Grip::Rotate => egui::CursorIcon::Grab,
                                            })
                                            .or(grip_cursor),
                                    };
                                    if let Some(drag) = &self.drag
                                        && let Some(Hold::Resize(handle)) = drag.hold
                                        && let Some((_, from)) = drag.origins.first()
                                        && let Some(at) = ui.input(|i| i.pointer.latest_pos())
                                    {
                                        size_label(
                                            painter,
                                            theme,
                                            at,
                                            resize::outward(turn, handle),
                                            area,
                                            (from, &shape.placement),
                                        );
                                    }
                                    if response.drag_started()
                                        && let Some(pos) = pointer
                                        && let Some(grip) =
                                            canvas::grip_hit(
                                                turn,
                                                &shape.placement,
                                                pos,
                                                shape.text.is_some(),
                                            )
                                    {
                                        grabbed = Some(match grip {
                                            canvas::Grip::Resize(handle) => Hold::Resize(handle),
                                            canvas::Grip::Rotate => Hold::Rotate {
                                                center: rotation::screen(
                                                    turn,
                                                    shape.placement.w / 2.0,
                                                    shape.placement.h / 2.0,
                                                ),
                                                from: pos,
                                            },
                                        });
                                    }
                                }
                            }
                            // The cells, the borders and the buttons of the
                            // selected table (spec 081).
                            if let Some(shape) = chosen.iter().find(|shape| {
                                single == Some((shape.page, shape.node))
                                    && shape.table.is_some()
                                    && !shape.locked
                            }) && self.drag.is_none()
                                && grabbed.is_none()
                            {
                                on_cells |= self.table_overlay(
                                    ui,
                                    (theme, texts),
                                    (&response, pointer),
                                    (page, zoom),
                                    shape,
                                    asked,
                                );
                            }
                            if self.selected.len() > 1 && self.selected.page() == Some(page_doc.id)
                            {
                                // The box around a multi-selection (spec 040).
                                let boxes = chosen
                                    .iter()
                                    .map(|s| rotation::screen_box(page, zoom, &s.placement));
                                if let Some(all) = canvas::bounds(boxes) {
                                    painter.rect_stroke(
                                        all.expand(4.0),
                                        0.0,
                                        Stroke::new(1.0, selection),
                                        egui::StrokeKind::Outside,
                                    );
                                }
                            }
                            // A press inside the box of a selected element
                            // takes it, also where it leaves its page
                            // (S51-03).
                            if pressed
                                && !response.double_clicked()
                                && let Some(at) = pointer
                            {
                                let boxes: Vec<Shape> = chosen
                                    .iter()
                                    .cloned()
                                    .map(|mut shape| {
                                        shape.geometry = ShapeGeometry::Rect;
                                        shape.locked = false;
                                        shape
                                    })
                                    .collect();
                                if let Some(node) = canvas::hit(&boxes, page, zoom, at) {
                                    picked = Some((id, node));
                                }
                            }
                        }
                        // The bar over the selection: Agrupar or Desagrupar,
                        // Bloquear, Duplicar, Eliminar and «···» (owner
                        // request); hidden while dragging, writing or
                        // cropping.
                        if self.drag.is_none()
                            && self.editing.is_none()
                            && self.cropping.is_none()
                            && let Some(page_id) = self.selected.page()
                            && let Some((_, rect)) = page_rects.iter().find(|(id, _)| *id == page_id)
                            && let Some(page_doc) = pages.iter().find(|p| p.id == page_id)
                        {
                            let boxes: Vec<Rect> = chosen(page_doc, &self.selected)
                                .iter()
                                .map(|shape| canvas::screen_box_of(shape, *rect, zoom))
                                .collect();
                            if let Some(first) = boxes.first() {
                                let around = boxes.iter().fold(*first, |a, b| a.union(*b));
                                self.selection_bar(ui.ctx(), theme, texts, around, page_doc, asked);
                            }
                        }
                        // Outside every page, the cut-away part of an
                        // element is still taken (S51-06).
                        if pressed
                            && picked.is_none()
                            && let Some(at) = pointer
                            && let Some((id, leaf)) = transfer::outside_hit(&laid, zoom, at)
                            && let Some(page_doc) = pages.iter().find(|p| p.id == id)
                            && let Some((node, scope)) = group::click(
                                page_doc,
                                leaf,
                                self.scope_on(id),
                                response.double_clicked(),
                            )
                        {
                            picked = Some((id, node));
                            asked.scope = Some(scope.map(|group| (id, group)));
                        }
                        // While a line end is dragged: the five points of
                        // the shapes near it, the outline of the one it
                        // sticks to, and its length and angle (spec 058).
                        if let Some(view) = &self.line_view
                            && let Some((_, rect)) =
                                page_rects.iter().find(|(id, _)| *id == view.page)
                            && let Some(page_doc) = pages.iter().find(|p| p.id == view.page)
                        {
                            let to_screen = |p| canvas::point_on_screen(*rect, zoom, p);
                            if let Some(node) = view.stuck {
                                for ring in lines::outline(page_doc, node) {
                                    let points: Vec<Pos2> =
                                        ring.into_iter().map(to_screen).collect();
                                    painter.add(egui::Shape::closed_line(
                                        points,
                                        Stroke::new(1.5, selection),
                                    ));
                                }
                            }
                            for node in &view.near {
                                for anchor in gafira_doc::Anchor::ALL {
                                    if let Some(p) = gafira_doc::LineNode::anchor_point(
                                        page_doc,
                                        *node,
                                        anchor.unit(),
                                    ) {
                                        painter.circle(
                                            to_screen(p),
                                            4.0,
                                            theme.c("surface"),
                                            Stroke::new(1.0, theme.c("text.secondary")),
                                        );
                                    }
                                }
                            }
                            let end = to_screen(view.end);
                            painter.circle_filled(end, 4.0, selection);
                            let (length, angle) = lines::length_and_angle(view.other, view.end);
                            let label = format!("l: {length:.0} {angle:.0}°");
                            let galley = painter.layout_no_wrap(
                                label,
                                theme.font("caption"),
                                theme.c("topbar.fg"),
                            );
                            let at = end + Vec2::new(14.0, 14.0);
                            let back =
                                Rect::from_min_size(at, galley.size()).expand2(Vec2::new(6.0, 3.0));
                            painter.rect_filled(back, 4.0, theme.c("topbar.bg"));
                            painter.galley(at, galley, theme.c("topbar.fg"));
                        }
                        // The field of a text being written on the
                        // canvas, over its element (spec 055).
                        self.edit_field(ui, theme, pages, &page_rects, zoom, asked);
                        self.cell_field(ui, theme, pages, &page_rects, zoom, asked);
                        if let Some(icon) = grip_cursor {
                            ui.ctx().set_cursor_icon(icon);
                        }
                        // A press on a page makes it the active one;
                        // outside every page none is. A handle or a
                        // guide keeps it (S50-01).
                        if (response.clicked() || response.drag_started())
                            && grabbed.is_none()
                            && guide_hit.is_none()
                            && let Some(at) = pointer
                        {
                            self.active =
                                active_after(&page_rects, at, picked.map(|(page, _)| page));
                        }
                        if let Some(tool) = self.drawing_tool() {
                            // Dibujo with a brush or the eraser: the
                            // pointer draws or erases (spec 079).
                            self.draw_input(
                                ui,
                                painter,
                                &response,
                                (pages, &page_rects, zoom),
                                (texts, tool),
                                asked,
                            );
                        } else if let Some(handle) = grabbed {
                            // A handle of the selection wins over whatever
                            // lies under it, and keeps the scope.
                            asked.drag_begin = Some(Some(handle));
                            asked.scope = None;
                        } else if let Some(held) = guide_hit {
                            // Then a guide, over the elements (spec 044).
                            self.guide_drag = Some(held);
                        } else if on_cells {
                            // The table took it.
                        } else if response.clicked() {
                            // Click selects one; Shift+click toggles it.
                            let mut next = self.selected.clone();
                            match (picked, shift) {
                                (Some((page, node)), true) => next.toggle(page, node),
                                (Some((page, node)), false) => next = Selection::one(page, node),
                                (None, true) => {}
                                // A locked element under the click is
                                // selected, to unlock it from its bar
                                // (D-110); empty canvas also leaves a group
                                // entered.
                                (None, false) => {
                                    next.clear();
                                    if let Some(at) = pointer
                                        && let Some((page, node)) =
                                            locked_under(pages, &page_rects, zoom, at)
                                    {
                                        next = Selection::one(page, node);
                                    }
                                    asked.scope = Some(None);
                                }
                            }
                            asked.select = Some(next);
                        } else if response.drag_started() {
                            match picked {
                                // A drag on a selected element moves the
                                // whole selection; on another one it selects
                                // it first (Shift adds it).
                                Some((page, node)) => {
                                    if !self.selected.contains(page, node) {
                                        let mut next = self.selected.clone();
                                        if shift {
                                            next.add(page, [node]);
                                        } else {
                                            next = Selection::one(page, node);
                                        }
                                        asked.select = Some(next);
                                    }
                                    asked.drag_begin = Some(None);
                                }
                                // On empty canvas it draws a marquee.
                                None => {
                                    if let Some(from) = press.or(pointer) {
                                        self.marquee = Some(Marquee {
                                            from,
                                            to: pointer.unwrap_or(from),
                                            add: shift,
                                        });
                                    }
                                }
                            }
                        }
                        if response.dragged() {
                            if let Some(marquee) = self.marquee.as_mut() {
                                if let Some(to) = pointer {
                                    marquee.to = to;
                                }
                            } else if self.drag.is_some() {
                                asked.drag_to = Some(response.drag_delta());
                                // Only the state of the keys counts (D-90):
                                // Shift frees a corner, Alt resizes from the
                                // centre and Ctrl lets the magnet go.
                                asked.hold_keys(resize::drag_keys(ui.input(|i| i.modifiers)));
                            }
                        }
                        if self.cropping.is_some() && response.clicked() && !crop_kept {
                            asked.end_crop = true;
                        }
                        // An image dragged over a frame (spec 066): the
                        // frame under the pointer, if any, takes it.
                        self.frame_target = None;
                        if self.drag.as_ref().is_some_and(|d| d.hold.is_none())
                            && let Some((page_id, image)) = self.selected.single()
                            && let Some(at) = ui.input(|i| i.pointer.latest_pos())
                            && let Some((_, rect, shapes)) =
                                laid.iter().find(|(id, rect, _)| *id == page_id && rect.contains(at))
                            && let Some(asset) =
                                shapes.iter().find(|s| s.node == image).and_then(|s| s.image)
                        {
                            let others: Vec<Shape> =
                                shapes.iter().filter(|s| s.node != image).cloned().collect();
                            if let Some(frame) = canvas::hit(&others, *rect, zoom, at)
                                && others.iter().any(|s| s.node == frame && s.frame.is_some())
                            {
                                self.frame_target = Some(FrameTarget {
                                    page: page_id,
                                    frame,
                                    image,
                                    asset,
                                });
                            }
                        }
                        // The frame that takes the image, or the one being
                        // cropped, is lit along its outline (spec 066).
                        let lit = self
                            .frame_target
                            .map(|t| (t.page, t.frame))
                            .or(self.cropping.map(|c| (c.page, c.node)));
                        if let Some((page_id, node)) = lit
                            && let Some(page_doc) = document.and_then(|d| d.page(page_id))
                            && let Some((_, rect, _)) = laid.iter().find(|(id, _, _)| *id == page_id)
                        {
                            let stroke = Stroke::new(2.0, theme.c("primary.bg"));
                            for ring in crate::lines::outline(page_doc, node) {
                                let points: Vec<Pos2> = ring
                                    .into_iter()
                                    .map(|p| canvas::point_on_screen(*rect, zoom, p))
                                    .collect();
                                painter.add(egui::Shape::closed_line(points, stroke));
                            }
                        }
                        if let Some(marquee) = self.marquee {
                            painter.rect(
                                Rect::from_two_pos(marquee.from, marquee.to),
                                0.0,
                                theme.c("selection.bg").gamma_multiply(0.5),
                                Stroke::new(1.0, selection),
                                egui::StrokeKind::Inside,
                            );
                        }
                        if response.drag_stopped() {
                            match self.marquee.take() {
                                Some(marquee) => {
                                    // What it touches counts by the child of
                                    // the scope that holds it (spec 043).
                                    let pick = |page: PageId, leaf: NodeId| {
                                        let on = pages.iter().find(|p| p.id == page)?;
                                        group::pick(on, leaf, self.scope_on(page))
                                    };
                                    asked.select = Some(marquee_selection(
                                        &self.selected,
                                        marquee,
                                        &laid,
                                        zoom,
                                        pick,
                                    ));
                                }
                                None => {
                                    asked.drag_end = true;
                                    // A move dropped on another page goes
                                    // there (S51-04).
                                    let rect_of = |id: PageId| {
                                        page_rects.iter().find(|(p, _)| *p == id).map(|r| r.1)
                                    };
                                    if let Some(drag) = &self.drag
                                        && drag.hold.is_none()
                                        && let Some(at) = ui.input(|i| i.pointer.latest_pos())
                                        && let Some(to) =
                                            transfer::drop_target(&page_rects, drag.page, at)
                                        && let (Some(a), Some(b)) =
                                            (rect_of(drag.page), rect_of(to))
                                    {
                                        let shift = (a.min - b.min) / zoom;
                                        asked.transfer = Some((
                                            drag.page,
                                            to,
                                            (f64::from(shift.x), f64::from(shift.y)),
                                        ));
                                    }
                                }
                            }
                        }
                    });
                });
                self.rulers_and_guides(ui, theme, full, document, &page_rects, asked);
                self.page_rects = page_rects;
            });
    }

    /// The rulers along the top and the left of the canvas, a guide dragged
    /// out of them or along the page, and where it is dropped (FD-03, spec
    /// 044): on the page it is placed, back on its ruler it is deleted.
    fn rulers_and_guides(
        &mut self,
        ui: &mut egui::Ui,
        theme: &Theme,
        area: Rect,
        document: Option<&Document>,
        page_rects: &[(PageId, Rect)],
        asked: &mut Asked,
    ) {
        let zoom = self.zoom;
        // The rulers measure from the corner of the page of the
        // selection, or the first page.
        let current = document
            .and_then(|d| self.current_page(d))
            .and_then(|page| page_rects.iter().find(|(id, _)| *id == page.id))
            .or(page_rects.first())
            .copied();
        if self.shown.rulers
            && let Some((page, origin)) = current
        {
            // The selection's boxes on screen, marked on both rulers
            // (S49-04).
            let boxes: Vec<Rect> = selected_shapes(document, &self.selected)
                .iter()
                .filter(|shape| shape.page == page)
                .map(|shape| rotation::screen_box(origin, zoom, &shape.placement))
                .collect();
            // The rulers count in the unit of the page's size (spec 076):
            // a page made in inches is measured in inches.
            let measure = document
                .and_then(|d| {
                    let unit = d.page(page)?.size.width.unit();
                    let px = gafira_geom::Length::new(1.0, unit)
                        .to_px(f64::from(d.dpi().get()))
                        .ok()?;
                    Some((unit, px))
                })
                .unwrap_or((gafira_geom::Unit::Px, 1.0));
            for axis in [GuideAxis::Horizontal, GuideAxis::Vertical] {
                let strip = ruler::strip(axis, area);
                let band = ruler::band(&boxes, axis);
                draw_ruler(
                    ui.painter(),
                    theme,
                    axis,
                    strip,
                    (origin, zoom, measure),
                    band,
                );
                let response = ui.interact(
                    strip,
                    ui.id().with(("ruler", axis == GuideAxis::Horizontal)),
                    egui::Sense::drag(),
                );
                if response.hovered() || response.dragged() {
                    ui.ctx().set_cursor_icon(guide_cursor(axis));
                }
                if response.drag_started() {
                    self.guide_drag = Some(GuideDrag {
                        page,
                        axis,
                        index: None,
                    });
                }
            }
            let corner = Rect::from_min_size(area.min, Vec2::splat(ruler::RULER));
            ui.painter().rect_filled(corner, 0.0, theme.c("surface"));
        }
        let Some(held) = self.guide_drag else {
            return;
        };
        let (at, released) = ui.input(|i| (i.pointer.latest_pos(), !i.pointer.primary_down()));
        let Some(at) = at else {
            return;
        };
        ui.ctx().set_cursor_icon(guide_cursor(held.axis));
        // A new guide goes to the page of the row it is dropped on.
        let page = match held.index {
            Some(_) => Some(held.page),
            None => ruler::page_at(page_rects, at.y),
        };
        let rect = page.and_then(|id| page_rects.iter().find(|(p, _)| *p == id));
        let on_ruler = ruler::back_to_ruler(held.axis, area, at);
        if !on_ruler && let Some((page_id, rect)) = rect {
            let px = ruler::page_coordinate(held.axis, *rect, zoom, at);
            let line = Stroke::new(1.0, theme.c("canvas.guide"));
            let offset = px as f32 * zoom;
            let points = match held.axis {
                GuideAxis::Vertical => [
                    Pos2::new(rect.min.x + offset, rect.min.y),
                    Pos2::new(rect.min.x + offset, rect.max.y),
                ],
                GuideAxis::Horizontal => [
                    Pos2::new(area.min.x, rect.min.y + offset),
                    Pos2::new(area.max.x, rect.min.y + offset),
                ],
            };
            ui.painter().line_segment(points, line);
            // Where it is, in the page's unit, on the ruler it runs to
            // (spec 076): a vertical guide on the top ruler, a horizontal
            // one on the left; it follows the pointer.
            if self.shown.rulers
                && let Some(page) = document.and_then(|d| d.page(*page_id))
            {
                let unit = page.size.width.unit();
                let per = gafira_geom::Length::new(1.0, unit)
                    .to_px(document.map_or(96.0, |d| f64::from(d.dpi().get())))
                    .unwrap_or(1.0);
                let text = ruler::position_label(px / per, unit);
                let font = egui::FontId::proportional(11.0);
                let galley = ui.painter().layout_no_wrap(text, font, Color32::WHITE);
                let size = galley.size() + Vec2::new(8.0, 2.0);
                let centre = match held.axis {
                    GuideAxis::Vertical => Pos2::new(points[0].x, area.min.y + ruler::RULER / 2.0),
                    GuideAxis::Horizontal => {
                        Pos2::new(area.min.x + ruler::RULER / 2.0, points[0].y)
                    }
                };
                let mut badge = Rect::from_center_size(centre, size);
                if held.axis == GuideAxis::Horizontal {
                    // Kept inside the left ruler's width, reading across.
                    badge =
                        badge.translate(Vec2::new(badge.width() / 2.0 - ruler::RULER / 2.0, 0.0));
                }
                let painter = ui.painter();
                painter.rect_filled(badge, 3.0, theme.c("canvas.guide"));
                painter.galley(badge.min + Vec2::new(4.0, 1.0), galley, Color32::WHITE);
            }
        }
        if !released {
            return;
        }
        self.guide_drag = None;
        let Some((id, rect)) = rect.copied() else {
            return;
        };
        let Some(current) = document.and_then(|d| d.page(id)) else {
            return;
        };
        let px = ruler::page_coordinate(held.axis, rect, zoom, at);
        let next = match (held.index, on_ruler) {
            // Back on its ruler: a new guide is not made, a page guide goes.
            (None, true) => return,
            (Some(index), true) => ruler::removed(&current.guides, index),
            (Some(index), false) => ruler::moved(&current.guides, index, px),
            (None, false) => ruler::added(&current.guides, held.axis, px),
        };
        asked.guides = Some((id, next));
        if !self.shown.guides {
            asked.shown = Some(ViewSettings::new(
                self.shown.rulers,
                true,
                self.shown.magnet,
            ));
        }
    }
}

impl Editor {
    /// Run what the person asked, through the engine (02 §5.5: every change
    /// is a command); a refused action is logged and the view stays.
    fn apply(
        &mut self,
        engine: &mut Gafira,
        document: &Option<Arc<Document>>,
        asked: &Asked,
        theme: &Theme,
    ) -> Go {
        let log = |what: &str, result: Result<(), gafira_engine::EngineError>| {
            if let Err(error) = result {
                eprintln!("{what}: {error}");
            }
        };
        if asked.undo {
            log("undo", engine.edit().undo().map(drop));
        }
        if asked.redo {
            log("redo", engine.edit().redo().map(drop));
        }
        let last = document
            .as_ref()
            .and_then(|d| d.pages().last().map(|page| (d.pages().len(), page.size)));
        if asked.add_page
            && let Some((at, size)) = last
            && let Some(document) = document
        {
            log(
                "add page",
                engine
                    .edit()
                    .execute(new_page(document, at, size))
                    .map(drop),
            );
        }
        if let Some(select) = &asked.select {
            self.selected = select.clone();
        }
        if let Some(scope) = asked.scope {
            self.scope = scope;
        }
        // Esc inside a group selects it and leaves it for its parent (spec
        // 043).
        if asked.leave
            && let Some((page, entered)) = self.scope
        {
            let parent = document
                .as_ref()
                .and_then(|d| d.page(page))
                .and_then(|p| group::parents(p).get(&entered).copied())
                .flatten();
            self.scope = parent.map(|group| (page, group));
            self.selected = Selection::one(page, entered);
        }
        if let Some(node) = asked.toggle_open
            && !self.open.remove(&node)
        {
            self.open.insert(node);
        }
        // Ctrl+A selects what the scope draws: the roots, or the children
        // of the group entered (spec 043).
        if asked.select_all
            && let Some(document) = engine.edit().document()
            && let Some(page) = self.current_page(&document)
        {
            let scope = self.scope_on(page.id);
            self.selected = Selection::of(page.id, group::items(page, scope));
        }
        if let Some(piece) = asked.insert
            && let Some(document) = document
            && let Some(page) = self.current_page(document)
            && let Ok(size) = page.size.to_px(document.dpi())
        {
            let px = Vec2::new(size.width() as f32, size.height() as f32);
            let spec = canvas::new_shape(px, piece, canvas::default_fill());
            let command = InsertNodes::new(page.id, None, page.roots.len(), vec![spec]);
            let next = NodeId::new(document.next_id());
            match engine.edit().execute(Box::new(command)) {
                Ok(_) => self.selected = Selection::one(page.id, next),
                Err(error) => eprintln!("insert: {error}"),
            }
        }
        // A light lands in the middle of the page, selected (spec 065).
        if let Some(item) = asked.insert_effect
            && let Some(document) = document
            && let Some(page) = self.current_page(document)
            && let Ok(size) = page.size.to_px(document.dpi())
        {
            let px = Vec2::new(size.width() as f32, size.height() as f32);
            let spec = canvas::new_effect(px, item.effect(), item.share());
            let command = InsertNodes::new(page.id, None, page.roots.len(), vec![spec]);
            let next = NodeId::new(document.next_id());
            match engine.edit().execute(Box::new(command)) {
                Ok(_) => self.selected = Selection::one(page.id, next),
                Err(error) => eprintln!("insert effect: {error}"),
            }
        }
        // A light dropped from the panel lands centred where it was
        // dropped, selected, its page active (D-105).
        if let Some((item, page, point)) = asked.drop_effect
            && let Some(document) = document
            && let Some(on) = document.page(page)
            && let Ok(size) = on.size.to_px(document.dpi())
        {
            let px = Vec2::new(size.width() as f32, size.height() as f32);
            let mut spec = canvas::new_effect(px, item.effect(), item.share());
            spec.placement.x = point.0 - spec.placement.w / 2.0;
            spec.placement.y = point.1 - spec.placement.h / 2.0;
            let command = InsertNodes::new(page, None, on.roots.len(), vec![spec]);
            let next = NodeId::new(document.next_id());
            match engine.edit().execute(Box::new(command)) {
                Ok(_) => {
                    self.selected = Selection::one(page, next);
                    self.active = Some(page);
                    self.scope = None;
                }
                Err(error) => eprintln!("drop light: {error}"),
            }
        }
        // A shape dropped from the panel lands centred where it was
        // dropped, selected, its page active (S52-03).
        if let Some((piece, page, point)) = asked.drop_piece
            && let Some(document) = document
            && let Some(on) = document.page(page)
            && let Ok(size) = on.size.to_px(document.dpi())
        {
            let px = Vec2::new(size.width() as f32, size.height() as f32);
            let fill = canvas::default_fill();
            let spec = canvas::piece_at(px, piece, fill, point);
            let command = InsertNodes::new(page, None, on.roots.len(), vec![spec]);
            let next = NodeId::new(document.next_id());
            match engine.edit().execute(Box::new(command)) {
                Ok(_) => {
                    self.selected = Selection::one(page, next);
                    self.active = Some(page);
                    self.scope = None;
                }
                Err(error) => eprintln!("drop shape: {error}"),
            }
        }
        if asked.import_image
            && let Some(document) = document
        {
            self.import_image(engine, document);
        }
        // A new empty frame, in the middle of the page or where it was
        // dropped, selected (spec 066, D-105).
        let frame_spec = |document: &Document, page: PageId, at: Option<(f64, f64)>, piece| {
            let on = document.page(page)?;
            let size = on.size.to_px(document.dpi()).ok()?;
            let px = Vec2::new(size.width() as f32, size.height() as f32);
            Some(match at {
                Some(at) => canvas::frame_at(px, piece, at),
                None => canvas::new_frame(px, piece),
            })
        };
        let frame_asked = match (asked.insert_frame, asked.drop_frame) {
            (_, Some((piece, page, at))) => Some((piece, Some(page), Some(at))),
            (Some(piece), None) => Some((piece, None, None)),
            (None, None) => None,
        };
        if let Some((piece, page, at)) = frame_asked
            && let Some(document) = document
            && let Some(page) = page.or_else(|| self.current_page(document).map(|p| p.id))
            && let Some(spec) = frame_spec(document, page, at, piece)
            && let Some(on) = document.page(page)
        {
            let command = InsertNodes::new(page, None, on.roots.len(), vec![spec]);
            let next = NodeId::new(document.next_id());
            match engine.edit().execute(Box::new(command)) {
                Ok(_) => {
                    self.selected = Selection::one(page, next);
                    self.active = Some(page);
                    self.scope = None;
                }
                Err(error) => eprintln!("insert frame: {error}"),
            }
        }
        self.qr_codes(engine, asked);
        self.tables(engine, asked);
        if asked.pick_frame_photo {
            self.pick_frame_photo(engine);
        }
        if let Some((file, design, at)) = asked.library_add.clone() {
            // An SVG of Biblioteca comes in the way it came in before: no
            // question (spec 068).
            if is_svg_path(&file) {
                self.pending_svg = Some((vec![file], at));
                self.place_svgs(engine, !design);
            } else {
                self.place_images(engine, &[file], at);
            }
        }
        // A colour of a group, changed in every shape of it in one step
        // (spec 067).
        if let Some((group, old, new)) = asked.recolor
            && let Some(page) = self.selected.page()
            && let Some(document) = engine.edit().document()
            && let Some(on) = document.page(page)
        {
            let mut commands: Vec<Box<dyn Command>> = crate::recolor::recolor(on, group, old, new)
                .into_iter()
                .map(|(node, value)| {
                    Box::new(SetProperty::new(page, vec![node], value, None)) as Box<dyn Command>
                })
                .collect();
            let command: Option<Box<dyn Command>> = match commands.len() {
                0 => None,
                1 => commands.pop(),
                _ => Some(Box::new(Transaction::new(
                    MessageKey("edit.set_property"),
                    commands,
                ))),
            };
            if let Some(command) = command
                && let Err(error) = engine.edit().execute(command)
            {
                eprintln!("recolor: {error}");
            }
        }
        // «Fondo de la página» (spec 069): the current page, every page, or
        // an image picked for it.
        let background_page = self.active.or_else(|| {
            engine
                .edit()
                .document()
                .and_then(|d| d.pages().first().map(|p| p.id))
        });
        if asked.background_pick
            && let Some(page) = background_page
        {
            let picked = rfd::FileDialog::new()
                .add_filter("Imagen", &["png", "jpg", "jpeg"])
                .pick_file();
            if let Some(path) = picked
                && let Ok(natural) = image::image_dimensions(&path)
            {
                match engine.assets().import(&path) {
                    Ok(asset) => {
                        self.remember(&path, false);
                        let background = gafira_doc::PageBackground::Image {
                            asset,
                            natural,
                            fit: gafira_doc::BackgroundFit::Fill,
                        };
                        let command = SetProperty::page(
                            page,
                            Property::PageBackground(Some(background)),
                            None,
                        );
                        if let Err(error) = engine.edit().execute(Box::new(command)) {
                            eprintln!("background: {error}");
                        }
                    }
                    Err(error) => eprintln!("background: {error}"),
                }
            }
        }
        if let Some(background) = asked.page_background.clone()
            && let Some(page) = background_page
        {
            let command = SetProperty::page(page, Property::PageBackground(background), None);
            if let Err(error) = engine.edit().execute(Box::new(command)) {
                eprintln!("background: {error}");
            }
        }
        if let Some((page, spec)) = asked.draw_stroke.clone()
            && let Some(on) = document.as_ref().and_then(|d| d.page(page))
        {
            let command = InsertNodes::new(page, None, on.roots.len(), vec![spec]);
            if let Err(error) = engine.edit().execute(Box::new(command)) {
                eprintln!("draw: {error}");
            }
            self.active = Some(page);
        }
        if let Some((page, nodes)) = asked.erase.clone()
            && !nodes.is_empty()
            && let Err(error) = engine
                .edit()
                .execute(Box::new(RemoveNodes::new(page, nodes)))
        {
            eprintln!("erase: {error}");
        }
        if let Some((page, node, asset)) = asked.image_to_background
            && let Some(natural) = natural_size(engine, asset.as_hash())
        {
            let background = gafira_doc::PageBackground::Image {
                asset,
                natural,
                fit: gafira_doc::BackgroundFit::Fill,
            };
            let commands: Vec<Box<dyn Command>> = vec![
                Box::new(SetProperty::page(
                    page,
                    Property::PageBackground(Some(background)),
                    None,
                )),
                Box::new(RemoveNodes::new(page, vec![node])),
            ];
            match engine.edit().execute(Box::new(Transaction::new(
                MessageKey("edit.set_property"),
                commands,
            ))) {
                Ok(_) => {
                    self.selected = Selection::default();
                    self.active = Some(page);
                }
                Err(error) => eprintln!("image to background: {error}"),
            }
        }
        if asked.background_all
            && let Some(page) = background_page
            && let Some(document) = engine.edit().document()
        {
            let background = document.page(page).and_then(|p| p.background.clone());
            let commands: Vec<Box<dyn Command>> = document
                .pages()
                .iter()
                .filter(|p| p.id != page)
                .map(|p| {
                    Box::new(SetProperty::page(
                        p.id,
                        Property::PageBackground(background.clone()),
                        None,
                    )) as Box<dyn Command>
                })
                .collect();
            if !commands.is_empty()
                && let Err(error) = engine.edit().execute(Box::new(Transaction::new(
                    MessageKey("edit.set_property"),
                    commands,
                )))
            {
                eprintln!("background: {error}");
            }
        }
        if !asked.library_remove.is_empty() {
            if let Some(library) = &self.library {
                library.remove(&asked.library_remove);
            }
            let mut view = self.library_view.borrow_mut();
            view.0.files = None;
            view.0.ticked.retain(|t| !asked.library_remove.contains(t));
            if view
                .0
                .info
                .as_ref()
                .is_some_and(|i| asked.library_remove.contains(&(i.path.clone(), i.design)))
            {
                view.0.info = None;
            }
        }
        if let Some(as_image) = asked.svg_as_image {
            self.place_svgs(engine, as_image);
        }
        if asked.svg_cancel {
            self.pending_svg = None;
        }
        if asked.import_frame_shape {
            self.import_frame_shape(engine);
        }
        if let Some((preset, words)) = &asked.insert_text
            && let Some(document) = document
            && let Some(page) = self.current_page(document)
        {
            let page = page.id;
            self.add_text(engine, document, theme, (*preset, words), (page, None));
        }
        if let Some((bend, words, at)) = &asked.insert_curved
            && let Some(document) = document
            && let Some(page) = at
                .map(|(page, _)| page)
                .or_else(|| self.current_page(document).map(|p| p.id))
        {
            self.add_curved(
                engine,
                document,
                theme,
                (*bend, words),
                (page, at.map(|a| a.1)),
            );
        }
        if let Some((index, words, at)) = &asked.insert_pairing
            && let Some(document) = document
            && let Some(page) = at
                .map(|(page, _)| page)
                .or_else(|| self.current_page(document).map(|p| p.id))
        {
            self.add_pairing(engine, document, (*index, words), (page, at.map(|a| a.1)));
        }
        // Text pasted from outside: a body text box on the current page,
        // fitted to its words once drawn (spec 061).
        // Spec 067: images from the system.
        if asked.pasted_any {
            self.paste_seen = true;
        }
        if asked.v_released {
            let image = !self.paste_seen;
            self.paste_seen = false;
            if image {
                self.paste_image(engine);
            }
        }
        if !asked.paste_files.is_empty() {
            self.place_images(engine, &asked.paste_files, None);
        }
        // Spec 071: a photo whose bytes arrived goes into the project's
        // store with its credit as its name, then on the page or into the
        // frame it was dropped on; one undo step.
        // A frame of Marcos: in the middle of the current page or where it
        // was dropped; selected, one undo step.
        if let Some((kind, dropped)) = asked.insert_frame_kind
            && let Some(document) = engine.edit().document()
            && let Some(page) = dropped
                .map(|(page, _)| page)
                .or_else(|| self.current_page(&document).map(|p| p.id))
            && let Some(on) = document.page(page)
            && let Ok(size) = on.size.to_px(document.dpi())
        {
            let (pw, ph) = (size.width(), size.height());
            let centre = dropped.map_or((pw / 2.0, ph / 2.0), |(_, point)| point);
            if let Some(spec) = crate::frames::spec(kind, (pw, ph), centre) {
                let next = NodeId::new(document.next_id());
                let command = InsertNodes::new(page, None, on.roots.len(), vec![spec]);
                match engine.edit().execute(Box::new(command)) {
                    Ok(_) => {
                        self.selected = Selection::one(page, next);
                        self.active = Some(page);
                        self.scope = None;
                    }
                    Err(error) => eprintln!("insert frame: {error}"),
                }
            }
        }
        // S66-09: a collage fills its whole page (owner request): one group
        // of frames, selected, one undo step.
        if let Some((index, dropped)) = asked.insert_collage
            && let Some(document) = engine.edit().document()
            && let Some(page) = dropped.or_else(|| self.current_page(&document).map(|p| p.id))
            && let Some(on) = document.page(page)
            && let Ok(size) = on.size.to_px(document.dpi())
            && let Some(layout) = crate::collage::LAYOUTS.get(index)
            && let Some(spec) = crate::collage::spec(layout, (size.width(), size.height()))
        {
            let next = NodeId::new(document.next_id());
            let command = InsertNodes::new(page, None, on.roots.len(), vec![spec]);
            match engine.edit().execute(Box::new(command)) {
                Ok(_) => {
                    self.selected = Selection::one(page, next);
                    self.active = Some(page);
                    self.scope = None;
                }
                Err(error) => eprintln!("collage: {error}"),
            }
        }
        // Spec 072: an illustration as a group of vector shapes, 30 % of
        // the page's shorter side, named by its credit.
        if let Some((svg, credit, at)) = &asked.insert_illustration {
            match gafira_io_vector::svg_drawing(svg) {
                Ok(drawing) => {
                    self.place_drawing(engine, &drawing, *at, Some((0.3, credit.clone())));
                }
                Err(error) => eprintln!("illustration: {error:?}"),
            }
        }
        let arrived = self.photo_panel.borrow_mut().pump();
        for arrived in arrived {
            match engine
                .assets()
                .import_bytes(&arrived.photo.credit(), &arrived.bytes)
            {
                Ok(asset) => self.place_image(engine, asset, arrived.natural, arrived.at, true),
                Err(error) => eprintln!("photo: {error}"),
            }
        }
        if !asked.drop_files.is_empty() {
            self.place_images(engine, &asked.drop_files, asked.drop_at);
        }
        if let Some(words) = &asked.paste_text
            && let Some(document) = document
            && let Some(page) = self.current_page(document).map(|p| p.id)
            && let Some(added) = self.add_text(
                engine,
                document,
                theme,
                (canvas::TextPreset::Body, words),
                (page, None),
            )
        {
            self.fit_pending = Some(added);
        }
        if let Some((preset, words, page, point)) = &asked.drop_text
            && let Some(document) = document
        {
            self.add_text(
                engine,
                document,
                theme,
                (*preset, words),
                (*page, Some(*point)),
            );
        }
        // A box that grows with its words takes their height: in the
        // resize being dragged, or alone after a paste (spec 061).
        if let Some((page, node, laid)) = self.text_measure.take()
            && let Some(document) = engine.edit().document()
            && let Some(found) = document.page(page).and_then(|p| p.node(node))
            && let gafira_doc::NodeKind::Text(text) = &found.kind
        {
            let dragged = self
                .drag
                .as_ref()
                .filter(|d| d.origins.first().map(|o| o.0) == Some(node))
                .map(|d| d.gesture);
            if let Some(fitted) = canvas::text_fit(text.sizing, &found.base.placement, laid) {
                let gesture = match dragged {
                    Some(gesture) => gesture,
                    None => engine.edit().begin_gesture().ok(),
                };
                let command =
                    SetProperty::new(page, vec![node], Property::Placement(fitted), gesture);
                log(
                    "fit text",
                    engine.edit().execute(Box::new(command)).map(drop),
                );
                if dragged.is_none() && gesture.is_some() {
                    log("end gesture", engine.edit().end_gesture());
                }
            }
            if dragged.is_none() {
                self.fit_pending = None;
            }
        }
        // «Curva»: the bend of every selected text and the height of its
        // box, which follows its arc, in one undo step (FD-44).
        if let Some(curve) = asked.text_curve
            && let Some(page) = self.selected.page()
            && let Some(document) = engine.edit().document()
            && let Some(on) = document.page(page)
        {
            let mut nodes = Vec::new();
            let mut commands: Vec<Box<dyn Command>> = Vec::new();
            for id in self.selected.nodes() {
                if let Some(node) = on.node(*id)
                    && let gafira_doc::NodeKind::Text(text) = &node.kind
                {
                    nodes.push(*id);
                    let mut bent = text.clone();
                    bent.curve = curve;
                    let mut placement = node.base.placement;
                    if let Some(h) = canvas::curved_height(&bent, placement.w, placement.h) {
                        placement.h = h.max(1.0);
                        commands.push(Box::new(SetProperty::new(
                            page,
                            vec![*id],
                            Property::Placement(placement),
                            None,
                        )));
                    }
                }
            }
            if !nodes.is_empty() {
                commands.insert(
                    0,
                    Box::new(SetProperty::new(
                        page,
                        nodes,
                        Property::TextCurve(curve),
                        None,
                    )),
                );
                let step = Transaction::new(MessageKey("edit.set_property"), commands);
                log(
                    "text curve",
                    engine.edit().execute(Box::new(step)).map(drop),
                );
            }
        }
        self.text_styles(engine, asked);
        if let (Some(text), Some((page, node))) = (&asked.text, self.selected.single()) {
            let command =
                SetProperty::new(page, vec![node], Property::TextContent(text.clone()), None);
            log("text", engine.edit().execute(Box::new(command)).map(drop));
            // A new size or colour may change the height of the words.
            self.fit_pending = Some((page, node));
        }
        if let Some(hold) = asked.drag_begin
            && let Some(page) = self.selected.page()
        {
            // Where every selected element starts, from the newest document;
            // what is locked stays where it is (D-91).
            let now = engine.edit().document();
            let free = now
                .as_deref()
                .and_then(|document| document.page(page))
                .map(|on| layers::movable(on, self.selected.nodes()))
                .unwrap_or_default();
            let origins = selected_shapes(now.as_deref(), &self.selected)
                .into_iter()
                .filter(|shape| free.contains(&shape.node))
                .map(|shape| (shape.node, shape.placement))
                .collect::<Vec<_>>();
            if !origins.is_empty() {
                let gesture = engine.edit().begin_gesture().ok();
                self.drag = Some(Drag {
                    gesture,
                    page,
                    hold: hold.filter(|_| origins.len() == 1),
                    origins,
                    start: now,
                    offset: Vec2::ZERO,
                    moved: false,
                });
            }
        }
        // The end of a line is dragged on its own (spec 058).
        let line_end = matches!(
            self.drag.as_ref().and_then(|d| d.hold),
            Some(Hold::LineEnd(_))
        );
        if line_end && let Some(delta) = asked.drag_to {
            self.drag_line_end(engine, delta, asked.free);
        }
        if let (Some(delta), Some(drag)) = (asked.drag_to.filter(|_| !line_end), self.drag.as_mut())
        {
            drag.offset += delta;
            let zoom = self.zoom;
            let start = drag.start.as_ref().and_then(|d| d.page(drag.page));
            // The magnet (FD-06, spec 044): the box of what moves, or the
            // edges a handle drags, go to the nearest target; Ctrl lets go
            // (D-90).
            let moving: Vec<NodeId> = drag.origins.iter().map(|(node, _)| *node).collect();
            let size = drag
                .start
                .as_ref()
                .and_then(|d| Some((start?.size.to_px(d.dpi()).ok()?, d.dpi())));
            let field = start.zip(size).map(|(page, (size, dpi))| magnet::Field {
                page,
                width: size.width(),
                height: size.height(),
                dpi: f64::from(dpi.get()),
                zoom: f64::from(zoom),
                guides: self.shown.guides,
                active: self.shown.magnet && !asked.free,
                moving: &moving,
            });
            let mut snap = Snap::default();
            // The letters a corner drag of a text box scales (owner rule).
            let mut letters: Vec<(NodeId, Property)> = Vec::new();
            if drag.hold.is_none()
                && let Some(field) = &field
                && let Some(from) = start.and_then(|page| magnet::selection_box(page, &moving))
            {
                let dx = f64::from(drag.offset.x / zoom);
                let dy = f64::from(drag.offset.y / zoom);
                snap = magnet::snap_box(field, from + gafira_geom::Vec2::new(dx, dy));
            }
            // One placement per element; several are one Transaction, so the
            // drag stays one undo step (spec 040). A group writes every node
            // below it, from where they were when the drag began (spec 043).
            let commands = drag
                .origins
                .iter()
                .flat_map(|(node, origin)| {
                    let to = match drag.hold {
                        Some(Hold::Resize(handle)) => {
                            // A corner keeps the proportions; Shift frees
                            // them (D-89, spec 045). Alt resizes from the
                            // centre (D-90, spec 047).
                            let keep = !asked.snap && resize::is_corner(handle);
                            let to = if asked.centre {
                                resize::resized_centred(origin, handle, drag.offset, zoom, keep)
                            } else {
                                resize::resized(origin, handle, drag.offset, zoom, keep)
                            };
                            // A text box: a corner scales its letters with it,
                            // a side keeps room for one letter (owner rule).
                            let words = start.and_then(|page| page.node(*node)).and_then(|n| {
                                match &n.kind {
                                    gafira_doc::NodeKind::Text(text) => Some(text.content.clone()),
                                    _ => None,
                                }
                            });
                            // A group: a corner scales the letters of every
                            // text below it by one factor, so they keep
                            // their proportion (owner rule).
                            let grouped = start
                                .filter(|page| group::is_group(page, *node))
                                .filter(|_| resize::is_corner(handle) && origin.w > 0.0);
                            if let Some(page) = grouped {
                                let (k, words) =
                                    group::scaled_letters(page, *node, to.w / origin.w);
                                for (id, words) in words {
                                    letters.push((
                                        id,
                                        match words {
                                            group::ScaledWords::Text(text) => {
                                                Property::TextContent(text)
                                            }
                                            group::ScaledWords::Shape(text) => {
                                                Property::ShapeText(Some(text))
                                            }
                                        },
                                    ));
                                }
                                if (k - to.w / origin.w).abs() > 1e-9 {
                                    if asked.centre {
                                        resize::scaled_centred(origin, k).unwrap_or(to)
                                    } else {
                                        resize::scaled(origin, handle, k).unwrap_or(to)
                                    }
                                } else {
                                    to
                                }
                            } else if let Some(words) = words {
                                if resize::is_corner(handle) && origin.w > 0.0 {
                                    let k = crate::textbox::corner_factor(&words, to.w / origin.w);
                                    let to = resize::scaled(origin, handle, k).unwrap_or(*origin);
                                    letters.push((
                                        *node,
                                        Property::TextContent(crate::textbox::scaled(&words, k)),
                                    ));
                                    to
                                } else {
                                    let least = crate::textbox::min_width(&words);
                                    let mut to = to;
                                    if to.w < least {
                                        if handle.x < 0 {
                                            to.x = origin.x + origin.w - least;
                                        }
                                        to.w = least;
                                    }
                                    to
                                }
                            } else {
                                match &field {
                                    Some(field) => {
                                        let (to, edges) = match (asked.centre, keep) {
                                            (true, _) => magnet::snap_resize_centred(
                                                field, origin, handle, to, keep,
                                            ),
                                            (false, true) => {
                                                magnet::snap_resize_kept(field, origin, handle, to)
                                            }
                                            (false, false) => {
                                                magnet::snap_resize(field, origin, handle, to)
                                            }
                                        };
                                        snap = edges;
                                        to
                                    }
                                    None => to,
                                }
                            }
                        }
                        Some(Hold::Rotate { center, from }) => {
                            rotation::rotated(origin, center, from, from + drag.offset, asked.snap)
                        }
                        // Handled by `drag_line_end`.
                        Some(Hold::LineEnd(_)) => *origin,
                        None => {
                            let mut to = canvas::moved(origin, drag.offset, zoom);
                            to.x += snap.dx;
                            to.y += snap.dy;
                            to
                        }
                    };
                    let placements = match start.filter(|page| group::is_group(page, *node)) {
                        Some(page) => {
                            group::changed(page, *node, group_change(origin, &to, drag.hold))
                        }
                        None => vec![(*node, to)],
                    };
                    placements.into_iter().map(|(node, to)| {
                        let command = SetProperty::new(
                            drag.page,
                            vec![node],
                            Property::Placement(to),
                            drag.gesture,
                        );
                        Box::new(command) as Box<dyn Command>
                    })
                })
                .collect();
            let mut commands: Vec<Box<dyn Command>> = commands;
            for (node, words) in letters {
                commands.push(Box::new(SetProperty::new(
                    drag.page,
                    vec![node],
                    words,
                    drag.gesture,
                )));
            }
            if let Some(command) = batch(commands) {
                match engine.edit().execute(command) {
                    Ok(applied) => drag.moved |= applied == gafira_edit::Applied::Changed,
                    Err(error) => eprintln!("move: {error}"),
                }
            }
            self.snapped = (snap.x.is_some() || snap.y.is_some()).then_some((drag.page, snap));
        }
        if asked.drag_end {
            self.snapped = None;
            self.line_view = None;
        }
        if asked.drag_end
            && let Some(drag) = self.drag.take()
        {
            log("end gesture", engine.edit().end_gesture());
            let moved: Vec<NodeId> = drag.origins.iter().map(|(node, _)| *node).collect();
            let into = self
                .frame_target
                .take()
                .filter(|_| drag.moved && drag.hold.is_none());
            if let Some(target) = into {
                self.drop_into_frame(engine, target);
            } else if let Some((from, to, shift)) = asked.transfer
                && drag.moved
            {
                self.transfer(engine, (from, to), &moved, shift);
            } else if drag.moved && drag.hold.is_none() {
                drop_outside(engine, &drag, &moved);
            }
        }
        self.crop(engine, document.as_deref(), asked);
        if let Some((dx, dy)) = asked.nudge
            && let Some(page) = self.selected.page()
            && self.drag.is_none()
        {
            self.nudge(engine, page, dx, dy);
        }
        if let Some((page, guides)) = &asked.guides {
            // The guides are the same on every page (D-99): the new set
            // goes to all of them, in a gesture of its own, one undo step
            // that never merges with the next guide (FD-03).
            let gesture = engine.edit().begin_gesture().ok();
            let pages: Vec<PageId> = engine
                .edit()
                .document()
                .map(|d| d.pages().iter().map(|p| p.id).collect())
                .unwrap_or_else(|| vec![*page]);
            let commands: Vec<Box<dyn Command>> = pages
                .into_iter()
                .map(|id| {
                    Box::new(SetProperty::page(
                        id,
                        Property::Guides(guides.clone()),
                        gesture,
                    )) as Box<dyn Command>
                })
                .collect();
            let command = Transaction::new(MessageKey("edit.guides"), commands);
            log("guides", engine.edit().execute(Box::new(command)).map(drop));
            if gesture.is_some() {
                log("end gesture", engine.edit().end_gesture());
            }
        }
        if let Some(view) = asked.shown {
            self.shown = view;
            let settings = engine.settings().get().with_view(view);
            if let Err(error) = engine.settings().set(settings) {
                eprintln!("settings: {error}");
            }
        }
        if (asked.turn.is_some() || asked.flip.is_some())
            && let Some(page) = self.selected.page()
        {
            self.reshape(engine, page, asked.turn, asked.flip);
        }
        if let Some(page) = self.selected.page() {
            self.arrange(engine, page, asked);
        }
        if let (Some(placement), Some((page, node))) = (asked.place, self.selected.single()) {
            let now = engine.edit().document();
            match now
                .as_deref()
                .and_then(|d| d.page(page))
                .filter(|p| group::is_group(p, node))
            {
                // A group's box typed in the Inspector reframes every node
                // below it, in one step (spec 043).
                Some(current) => {
                    if let Some(from) = group::item_box(current, node) {
                        let to = Change::Frame {
                            from,
                            to: placement,
                        };
                        let moves = group::changed(current, node, to);
                        place_in_one_step(engine, page, moves, "place group");
                    }
                }
                None => {
                    let command =
                        SetProperty::new(page, vec![node], Property::Placement(placement), None);
                    log("place", engine.edit().execute(Box::new(command)).map(drop));
                }
            }
        }
        self.structure(engine, asked);
        // Fill, outline, radius and opacity go to every selected element in
        // one SetProperty, so a change is one undo step (spec 040).
        let all = self.selected.nodes().to_vec();
        if let Some(page) = self.selected.page() {
            let values = [
                ("fill", asked.fill.map(Property::Fill)),
                ("gradient", asked.gradient.clone().map(Property::Gradient)),
                ("effect", asked.effect.clone().map(Property::Effect)),
                (
                    "frame content",
                    asked.frame_content.map(Property::FrameContent),
                ),
                ("frame fit", asked.frame_fit.map(Property::FrameFit)),
                ("outline", asked.stroke.map(Property::Stroke)),
                (
                    "corner radius",
                    asked.corner_radius.map(Property::CornerRadius),
                ),
                ("opacity", asked.opacity.map(Property::Opacity)),
            ];
            for (what, value) in values {
                if let Some(value) = value {
                    let command = SetProperty::new(page, all.clone(), value, None);
                    log(what, engine.edit().execute(Box::new(command)).map(drop));
                }
            }
        }
        if let Some((page, node, index)) = asked.reorder {
            let command = gafira_edit::Reorder::new(page, node, index);
            log(
                "reorder",
                engine.edit().execute(Box::new(command)).map(drop),
            );
        }
        self.layer_changes(engine, asked);
        self.text_editing(engine, asked, theme);
        if let Some(open) = asked.font_panel {
            self.font_panel = open;
        }
        if let Some((family, bold, italic)) = &asked.font {
            self.apply_font(engine, (family, *bold, *italic));
            // Another face may change the height of the words (spec 061).
            self.fit_pending = self.selected.single();
        }
        if let Some(action) = &asked.page_action {
            self.page_action(engine, action);
        }
        // Every selected element goes in one RemoveNodes: one undo step.
        if asked.delete
            && let Some(page) = self.selected.page()
        {
            // Suprimir on a frame with a photo takes the photo out and keeps
            // the frame, still selected; an empty frame and every other
            // element go (spec 066). One undo step either way.
            let (emptied, removed): (Vec<NodeId>, Vec<NodeId>) = all.iter().partition(|node| {
                document
                    .as_deref()
                    .and_then(|d| frame_of(d, page, **node))
                    .is_some_and(|frame| frame.content.is_some())
            });
            let mut commands: Vec<Box<dyn Command>> = Vec::new();
            if !emptied.is_empty() {
                commands.push(Box::new(SetProperty::new(
                    page,
                    emptied.clone(),
                    Property::FrameContent(None),
                    None,
                )));
            }
            if !removed.is_empty() {
                commands.push(Box::new(RemoveNodes::new(page, removed)));
            }
            self.selected = if emptied.is_empty() {
                Selection::default()
            } else {
                Selection::of(page, emptied)
            };
            let command: Option<Box<dyn Command>> = match commands.len() {
                0 => None,
                1 => commands.pop(),
                _ => Some(Box::new(Transaction::new(
                    MessageKey("edit.set_property"),
                    commands,
                ))),
            };
            if let Some(command) = command {
                log("delete", engine.edit().execute(command).map(drop));
            }
        }
        if let Some(dark) = asked.dark {
            Go::Dark(dark)
        } else if asked.settings {
            Go::Settings
        } else if let Some(name) = &asked.version {
            Go::SaveVersion(name.clone())
        } else if asked.export {
            Go::Export(self.selected.page())
        } else if asked.home {
            Go::Home
        } else if let Some(action) = asked.file {
            Go::File(action)
        } else {
            Go::Stay
        }
    }
}

impl Editor {
    /// Turn by `turn` degrees and then flip along `flip` every selected
    /// element, each about its own centre (spec 041). The commands share a
    /// gesture of their own, so the action is one undo step and never
    /// merges with the next one.
    fn reshape(&self, engine: &mut Gafira, page: PageId, turn: Option<f64>, flip: Option<Axis>) {
        let now = engine.edit().document();
        let current = now.as_deref().and_then(|d| d.page(page));
        let placements = selected_shapes(now.as_deref(), &self.selected)
            .iter()
            .flat_map(|shape| match current.filter(|_| shape.group) {
                // A group turns and flips about the centre of its box, with
                // every node below it (spec 043).
                Some(current) => group_reshaped(current, shape.node, turn, flip),
                None => {
                    let mut to = shape.placement;
                    if let Some(degrees) = turn {
                        to = rotation::turned_by(&to, degrees);
                    }
                    if let Some(axis) = flip {
                        to = rotation::flipped(&to, axis);
                    }
                    vec![(shape.node, to)]
                }
            })
            .collect();
        place_in_one_step(engine, page, placements, "turn or flip");
    }

    /// Move every selected element by `(dx, dy)` px of page (03 §9.2), a
    /// group with everything below it. The commands carry no gesture, so
    /// the nudges of a held or quickly repeated arrow, less than 500 ms
    /// apart, are one undo step (02 §4.3, ARQ-33).
    fn nudge(&self, engine: &mut Gafira, page: PageId, dx: f64, dy: f64) {
        let now = engine.edit().document();
        let Some(current) = now.as_deref().and_then(|d| d.page(page)) else {
            return;
        };
        // What is locked does not move (D-91).
        let free = layers::movable(current, self.selected.nodes());
        let commands = selected_shapes(now.as_deref(), &self.selected)
            .iter()
            .filter(|shape| free.contains(&shape.node))
            .flat_map(|shape| {
                let moves = if shape.group {
                    group::changed(current, shape.node, Change::Move { dx, dy })
                } else {
                    let mut to = shape.placement;
                    to.x += dx;
                    to.y += dy;
                    vec![(shape.node, to)]
                };
                moves.into_iter().map(|(node, to)| {
                    let command = SetProperty::new(page, vec![node], Property::Placement(to), None);
                    Box::new(command) as Box<dyn Command>
                })
            })
            .collect();
        if let Some(command) = batch(commands)
            && let Err(error) = engine.edit().execute(command)
        {
            eprintln!("nudge: {error}");
        }
    }

    /// Agrupar, Desagrupar, Duplicar, Copiar, Cortar and Pegar (spec 043),
    /// each one undo step; what they make is selected afterwards.
    fn structure(&mut self, engine: &mut Gafira, asked: &Asked) {
        let Some(document) = engine.edit().document() else {
            return;
        };
        let page = self.selected.page().and_then(|page| document.page(page));
        let nodes = self.selected.nodes().to_vec();
        let next = document.next_id();
        let run = |engine: &mut Gafira, what: &str, command: Box<dyn Command>| {
            let done = engine.edit().execute(command);
            if let Err(error) = &done {
                eprintln!("{what}: {error}");
            }
            done.is_ok()
        };
        if let Some(page) = page {
            if asked.group
                && let Some((command, made)) = group::group_command(page, &nodes, next)
                && run(engine, "group", command)
            {
                self.selected = Selection::one(page.id, made);
            } else if asked.ungroup
                && let Some((command, freed)) = group::ungroup_command(page, &nodes)
                && run(engine, "ungroup", command)
            {
                // The freed children, and what was selected and was not a
                // group.
                let kept = nodes
                    .iter()
                    .copied()
                    .filter(|id| !group::is_group(page, *id));
                self.selected = Selection::of(page.id, kept.chain(freed));
            } else if asked.duplicate {
                let rename = |name: &str| {
                    asked
                        .copy_names
                        .iter()
                        .find(|(from, _)| from == name)
                        .map_or_else(|| name.to_owned(), |(_, to)| to.clone())
                };
                if let Some((command, copies)) =
                    clipboard::duplicate_command(page, &nodes, next, rename)
                    && run(engine, "duplicate", command)
                {
                    self.selected = Selection::of(page.id, copies);
                }
            }
            if (asked.copy || asked.cut)
                && let Some(clip) = clipboard::copy(&document, page.id, &nodes)
            {
                self.clip = Some(Arc::new(clip));
                self.copied = true;
                if asked.cut
                    && run(
                        engine,
                        "cut",
                        Box::new(RemoveNodes::new(page.id, nodes.clone())),
                    )
                {
                    self.selected.clear();
                }
            }
        }
        // Pegar goes on top of the page of the selection, or the first
        // page; inside the group entered when there is one.
        if asked.paste
            && let Some(clip) = &self.clip
            && let Some(target) = self.current_page(&document)
        {
            let parent = self.scope_on(target.id);
            if let Some((command, pasted)) =
                clipboard::paste_command(&document, target.id, parent, clip)
            {
                let target = target.id;
                if run(engine, "paste", command) {
                    self.selected = Selection::of(target, pasted);
                }
            }
        }
    }

    /// Align, distribute or move in the layer order the selection of
    /// `page`, each action one undo step (spec 042).
    fn arrange(&self, engine: &mut Gafira, page: PageId, asked: &Asked) {
        let now = engine.edit().document();
        let Some(document) = now.as_deref() else {
            return;
        };
        let Some(current) = document.page(page) else {
            return;
        };
        let nodes = self.selected.nodes();
        if let Some(action) = asked.align
            && let Ok(size) = current.size.to_px(document.dpi())
        {
            let moves = arrange::aligned(current, (size.width(), size.height()), nodes, action);
            place_in_one_step(engine, page, moves, "align");
        }
        if let Some(axis) = asked.distribute {
            let moves = arrange::distributed(current, nodes, axis);
            place_in_one_step(engine, page, moves, "distribute");
        }
        if let Some(order) = asked.order {
            let to = arrange::reordered(&current.roots, nodes, order);
            let mut commands: Vec<Box<dyn Command>> = arrange::steps(&current.roots, &to)
                .into_iter()
                .map(|(node, index)| {
                    Box::new(gafira_edit::Reorder::new(page, node, index)) as Box<dyn Command>
                })
                .collect();
            // Several steps are one Transaction: one undo step.
            let command: Option<Box<dyn Command>> = match commands.len() {
                0 | 1 => commands.pop(),
                _ => Some(Box::new(Transaction::new(
                    MessageKey("edit.reorder"),
                    commands,
                ))),
            };
            if let Some(command) = command
                && let Err(error) = engine.edit().execute(command)
            {
                eprintln!("order: {error}");
            }
        }
    }

    /// Decode, once, the images the document shows into textures (spec
    /// 029), reading the bytes from the content store; large images are
    /// scaled down to 2048 px for the canvas.
    fn load_textures(&mut self, ctx: &egui::Context, engine: &Gafira, document: Option<&Document>) {
        let Some(document) = document else { return };
        for page in document.pages() {
            // The image of a page background, decoded once (spec 069).
            if let Some(asset) = page
                .background
                .as_ref()
                .and_then(gafira_doc::PageBackground::asset)
            {
                let hash = asset.as_hash();
                self.photos
                    .entry(hash)
                    .or_insert_with(|| decode(engine, hash).map(Arc::new));
            }
            for shape in canvas::shapes(page) {
                // The photo of a frame is decoded once and cut out per
                // frame (spec 066).
                if let Some(content) = shape.frame.as_ref().and_then(|f| f.content) {
                    let hash = content.asset.as_hash();
                    self.photos
                        .entry(hash)
                        .or_insert_with(|| decode(engine, hash).map(Arc::new));
                }
                // The logo of a QR code is decoded once (spec 080).
                if let Some(logo) = shape.qr.as_ref().and_then(|qr| qr.logo) {
                    let hash = logo.asset.as_hash();
                    self.photos
                        .entry(hash)
                        .or_insert_with(|| decode(engine, hash).map(Arc::new));
                }
                let Some(asset) = shape.image else { continue };
                let hash = asset.as_hash();
                // While an image is dragged its pixels are kept too, for the
                // preview in a frame (spec 066).
                if self.drag.is_some()
                    && self
                        .selected
                        .single()
                        .is_some_and(|(_, node)| node == shape.node)
                {
                    self.photos
                        .entry(hash)
                        .or_insert_with(|| decode(engine, hash).map(Arc::new));
                }
                if self.textures.contains_key(&hash) {
                    continue;
                }
                let texture = decode(engine, hash).map(|image| {
                    ctx.load_texture(
                        format!("asset-{}", asset.hex()),
                        image,
                        egui::TextureOptions::LINEAR,
                    )
                });
                self.textures.insert(hash, texture);
            }
        }
    }

    /// Biblioteca → Importar imagen…: pick a file, import it through
    /// `AssetService` (embedded, deduplicated by hash) and place it on the
    /// selected element's page, or the first one.
    /// Crop mode (S66-07): enter it with its own gesture, so every move of
    /// the photo and every change of its zoom merge into one undo step;
    /// leave it ending the gesture.
    fn crop(&mut self, engine: &mut Gafira, document: Option<&Document>, asked: &Asked) {
        if let Some((page, node)) = asked.begin_crop {
            if let Some(old) = self.cropping.take()
                && old.gesture.is_some()
                && let Err(error) = engine.edit().end_gesture()
            {
                eprintln!("end gesture: {error}");
            }
            let gesture = engine.edit().begin_gesture().ok();
            self.cropping = Some(Cropping {
                page,
                node,
                gesture,
            });
            self.selected = Selection::one(page, node);
            if let Some(document) = document
                && let Some(frame) = frame_of(document, page, node)
                && let Some(content) = frame.content
            {
                let hash = content.asset.as_hash();
                self.photos
                    .entry(hash)
                    .or_insert_with(|| decode(engine, hash).map(Arc::new));
            }
            return;
        }
        let Some(crop) = self.cropping else { return };
        let frame = document.and_then(|d| frame_of(d, crop.page, crop.node));
        let box_of = document
            .and_then(|d| d.page(crop.page))
            .and_then(|page| page.node(crop.node))
            .map(|node| node.base.placement);
        if asked.end_crop || frame.is_none() {
            self.cropping = None;
            if let Err(error) = engine.edit().end_gesture() {
                eprintln!("end gesture: {error}");
            }
            return;
        }
        let (Some(frame), Some(placement)) = (frame, box_of) else {
            return;
        };
        let (w, h) = (placement.w, placement.h);
        let next = if let Some(delta) = asked.crop_drag {
            // Screen px to px of the box (rotation is not taken into account).
            let zoom = f64::from(self.zoom.max(1e-3));
            let offset = frame.moved(w, h, f64::from(delta.x) / zoom, f64::from(delta.y) / zoom);
            Some((offset, frame.content_scale, frame.content_rotation_deg))
        } else {
            asked.crop_set.map(|(offset, scale, degrees)| {
                // A new zoom or turn keeps the photo covering the frame.
                let mut set = frame.clone();
                set.content_offset = offset;
                set.content_scale = scale;
                set.content_rotation_deg = degrees;
                (set.moved(w, h, 0.0, 0.0), scale, degrees)
            })
        };
        if let Some((offset, scale, degrees)) = next {
            let command = SetProperty::new(
                crop.page,
                vec![crop.node],
                Property::FrameCrop(offset, scale, degrees),
                crop.gesture,
            );
            if let Err(error) = engine.edit().execute(Box::new(command)) {
                eprintln!("crop: {error}");
            }
        }
    }

    /// An image dropped on a frame becomes its photo (spec 066): the move is
    /// undone and one step sets the photo and removes the image.
    fn drop_into_frame(&mut self, engine: &mut Gafira, target: FrameTarget) {
        let hash = target.asset.as_hash();
        let natural = natural_size(engine, hash).or_else(|| {
            self.photos
                .get(&hash)
                .and_then(Option::as_ref)
                .map(|p| (p.size[0] as u32, p.size[1] as u32))
        });
        let Some(natural) = natural else { return };
        if let Err(error) = engine.edit().undo() {
            eprintln!("into frame: {error}");
            return;
        }
        let content = gafira_doc::FrameContent {
            asset: target.asset,
            natural,
        };
        let commands: Vec<Box<dyn Command>> = vec![
            Box::new(SetProperty::new(
                target.page,
                vec![target.frame],
                Property::FrameContent(Some(content)),
                None,
            )),
            Box::new(RemoveNodes::new(target.page, vec![target.image])),
        ];
        match engine.edit().execute(Box::new(Transaction::new(
            MessageKey("edit.set_property"),
            commands,
        ))) {
            Ok(_) => self.selected = Selection::one(target.page, target.frame),
            Err(error) => {
                eprintln!("into frame: {error}");
                if let Err(error) = engine.edit().redo() {
                    eprintln!("redo: {error}");
                }
            }
        }
    }

    /// Image files from the system (spec 067): each imported and placed on
    /// the page under `at` (screen), centred there, one after the other a
    /// little apart; without `at`, in the middle of the current page. Over
    /// a frame, the first one becomes its photo. The last one placed is
    /// selected.
    fn place_images(
        &mut self,
        engine: &mut Gafira,
        paths: &[std::path::PathBuf],
        at: Option<Pos2>,
    ) {
        for (index, path) in paths.iter().enumerate() {
            let step = 24.0 * index as f32;
            // An SVG waits for the person to say how it comes in: as it is
            // or as a frame (spec 067).
            if is_svg_path(path) {
                let waiting = self.pending_svg.get_or_insert_with(|| (Vec::new(), at));
                waiting.0.push(path.clone());
                continue;
            }
            // A PDF makes a custom frame of its outline (S66-10).
            if is_vector_path(path) {
                if let Some(outline) = vector_outline(path) {
                    self.remember(path, true);
                    self.place_custom_frame(engine, outline, at.map(|at| at + Vec2::splat(step)));
                }
                continue;
            }
            let Ok(natural) = image::image_dimensions(path) else {
                eprintln!("import: {} is not a readable image", path.display());
                continue;
            };
            match engine.assets().import(path) {
                Ok(asset) => {
                    self.remember(path, false);
                    let at = at.map(|at| at + Vec2::splat(step));
                    self.place_image(engine, asset, natural, at, index == 0);
                }
                Err(error) => eprintln!("import: {error}"),
            }
        }
    }

    /// The SVG files waiting, brought in as the person chose (spec 067):
    /// drawn as they are, as images, or as custom frames of their outline.
    fn place_svgs(&mut self, engine: &mut Gafira, as_image: bool) {
        let Some((files, at)) = self.pending_svg.take() else {
            return;
        };
        for (index, file) in files.iter().enumerate() {
            let at = at.map(|at| at + Vec2::splat(24.0 * index as f32));
            if !as_image {
                if let Some(outline) = vector_outline(file) {
                    self.remember(file, true);
                    self.place_custom_frame(engine, outline, at);
                }
                continue;
            }
            let bytes = std::fs::read(file).ok();
            // «Tal cual»: the SVG's paths as vector shapes of Gafira, in a
            // group; only an SVG without paths comes in as pixels.
            if let Some(drawing) = bytes
                .as_deref()
                .and_then(|b| gafira_io_vector::svg_drawing(b).ok())
            {
                self.remember(file, false);
                self.place_drawing(engine, &drawing, at, None);
                continue;
            }
            let pixels = bytes.and_then(|bytes| gafira_io_vector::svg_pixels(&bytes, 2.0).ok());
            let Some((rgba, w, h)) = pixels else {
                eprintln!("svg: {} could not be drawn", file.display());
                continue;
            };
            let Some(image) = image::RgbaImage::from_raw(w, h, rgba) else {
                continue;
            };
            let mut png = Vec::new();
            if image
                .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
                .is_err()
            {
                continue;
            }
            let name = file
                .file_stem()
                .and_then(|n| n.to_str())
                .map_or_else(|| "svg.png".to_owned(), |n| format!("{n}.png"));
            match engine.assets().import_bytes(&name, &png) {
                Ok(asset) => {
                    self.remember(file, false);
                    // Drawn at twice its size: placed at its own size.
                    self.place_image(engine, asset, (w / 2, h / 2), at, index == 0);
                }
                Err(error) => eprintln!("svg: {error}"),
            }
        }
    }

    /// An SVG drawing as a group of shapes (spec 067): at its own size, or
    /// smaller to fit in the page, centred at `at` (screen) on the page under
    /// it, or in the middle of the current page; the group selected.
    fn place_drawing(
        &mut self,
        engine: &mut Gafira,
        drawing: &gafira_io_vector::Drawing,
        at: Option<Pos2>,
        sized: Option<(f64, String)>,
    ) {
        let Some(document) = engine.edit().document() else {
            return;
        };
        let dropped = at.and_then(|at| canvas::drop_point(&self.page_rects, self.zoom, at));
        let page = match dropped {
            Some((page, _)) => page,
            None => match self.current_page(&document) {
                Some(page) => page.id,
                None => return,
            },
        };
        let Some(on) = document.page(page) else {
            return;
        };
        let Ok(size) = on.size.to_px(document.dpi()) else {
            return;
        };
        let (pw, ph) = (size.width(), size.height());
        let (dw, dh) = drawing.size;
        if !(dw > 0.0 && dh > 0.0) {
            return;
        }
        // An illustration takes `share` of the page's shorter side; a file
        // keeps its own size, or less to fit (spec 067).
        let k = match &sized {
            Some((share, _)) => share * pw.min(ph) / dw.max(dh),
            None => (0.8 * pw.min(ph) / dw.max(dh)).min(1.0),
        };
        let centre = dropped.map_or((pw / 2.0, ph / 2.0), |(_, point)| point);
        let origin = (centre.0 - dw * k / 2.0, centre.1 - dh * k / 2.0);
        let children: Vec<gafira_edit::NodeSpec> = drawing
            .shapes
            .iter()
            .map(|shape| {
                let (x, y, w, h) = shape.bounds;
                let mut node =
                    gafira_doc::ShapeNode::new(gafira_doc::ShapeGeometry::Path(shape.path.clone()));
                match &shape.fill {
                    Some(gafira_io_vector::Fill::Solid(color)) => node.fill = Some(*color),
                    Some(gafira_io_vector::Fill::Gradient(gradient)) => {
                        node.gradient = Some(gradient.clone());
                    }
                    None => {}
                }
                node.stroke = shape
                    .stroke
                    .map(|(color, width)| gafira_doc::ShapeStroke::new(color, width * k));
                gafira_edit::NodeSpec::new(
                    gafira_doc::NodeKind::Shape(node),
                    gafira_doc::Placement::new(origin.0 + x * k, origin.1 + y * k, w * k, h * k),
                )
            })
            .collect();
        let group = gafira_edit::NodeSpec::new(
            gafira_doc::NodeKind::Group(gafira_doc::GroupNode {}),
            gafira_doc::Placement::new(origin.0, origin.1, dw * k, dh * k),
        )
        .with_children(children);
        let group = match sized {
            Some((_, name)) => gafira_edit::NodeSpec {
                name: Some(name),
                ..group
            },
            None => group,
        };
        let next = NodeId::new(document.next_id());
        let command = InsertNodes::new(page, None, on.roots.len(), vec![group]);
        match engine.edit().execute(Box::new(command)) {
            Ok(_) => {
                self.selected = Selection::one(page, next);
                self.active = Some(page);
            }
            Err(error) => eprintln!("svg: {error}"),
        }
    }

    /// Biblioteca remembers `file` as just imported (spec 068).
    fn remember(&self, file: &std::path::Path, design: bool) {
        if let Some(library) = &self.library {
            library.record(file, design);
        }
        self.library_view.borrow_mut().0.files = None;
    }

    /// Biblioteca (spec 068): «Importar archivo…», the search, the tabs
    /// «Imágenes» and «Diseños», and the previews of the imported files
    /// that still exist; a click adds one, a drag carries it to a page.
    /// Ticked files show a bar to take them out of the list, and «Más
    /// información» shows the details of one.
    fn library_panel(&self, ui: &mut egui::Ui, theme: &Theme, texts: &Texts, asked: &mut Asked) {
        use crate::library::{Tab, shown};
        let now = ui.input(|i| i.time);
        {
            let mut view = self.library_view.borrow_mut();
            // The list is read again every few seconds: a file deleted
            // outside Gafira leaves the panel (spec 068).
            if view.0.files.is_none() || now - view.1 > 3.0 {
                view.0.files = Some(
                    self.library
                        .as_ref()
                        .map(|l| l.existing())
                        .unwrap_or_default(),
                );
                view.1 = now;
            }
        }
        let info = self.library_view.borrow().0.info.clone();
        if let Some(entry) = info {
            self.library_info(ui, theme, texts, &entry, asked);
            return;
        }
        let width = ui.available_width();
        let import = ui.add_sized(
            [width, 36.0],
            egui::Button::new(
                RichText::new(plain(texts, "library.import_file")).color(theme.c("primary.fg")),
            )
            .fill(theme.c("primary.bg")),
        );
        if import.clicked() {
            asked.import_image = true;
        }
        ui.add_space(12.0);
        let (files, tab, ticked) = {
            let mut view = self.library_view.borrow_mut();
            ui.add(widgets::field(
                &mut view.0.query,
                plain(texts, "library.search"),
            ));
            ui.add_space(12.0);
            ui.horizontal(|ui| {
                for (tab, key) in [
                    (Tab::Images, "library.images"),
                    (Tab::Designs, "library.designs"),
                ] {
                    let on = view.0.tab == tab;
                    let label = RichText::new(plain(texts, key))
                        .font(theme.font("ui-label"))
                        .color(if on {
                            theme.c("text")
                        } else {
                            theme.c("text.muted")
                        });
                    let response = ui.add(egui::Button::new(label).frame(false));
                    if on {
                        let r = response.rect;
                        ui.painter().line_segment(
                            [r.left_bottom(), r.right_bottom()],
                            Stroke::new(2.0, theme.c("primary.bg")),
                        );
                    }
                    if response.clicked() && !on {
                        view.0.tab = tab;
                        view.0.ticked.clear();
                    }
                    ui.add_space(8.0);
                }
            });
            ui.add_space(12.0);
            let files = shown(
                view.0.files.as_deref().unwrap_or_default(),
                view.0.tab,
                &view.0.query,
            );
            (files, view.0.tab, view.0.ticked.clone())
        };
        if files.is_empty() {
            let key = match tab {
                Tab::Images => "library.empty_images",
                Tab::Designs => "library.empty_designs",
            };
            ui.label(
                RichText::new(plain(texts, key))
                    .font(theme.font("body-sm"))
                    .color(theme.c("text.muted")),
            );
            return;
        }
        let column = (ui.available_width() - 8.0) / 2.0;
        let bar = if ticked.is_empty() { 0.0 } else { 56.0 };
        egui::ScrollArea::vertical()
            .max_height((ui.available_height() - bar).max(80.0))
            .show(ui, |ui| {
                for row in files.chunks(2) {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing = Vec2::splat(8.0);
                        for entry in row {
                            let on = ticked.contains(&(entry.path.clone(), entry.design));
                            let look = (column, on, !ticked.is_empty());
                            self.library_tile(ui, theme, texts, entry, look, asked);
                        }
                    });
                    ui.add_space(8.0);
                }
            });
        // «N seleccionados»: clear the ticks, or take the files out of the
        // list; in both tabs (spec 068).
        if !ticked.is_empty() {
            // At the bottom of the panel.
            ui.add_space((ui.available_height() - 48.0).max(8.0));
            egui::Frame::NONE
                .fill(theme.c("surface"))
                .stroke(Stroke::new(1.0, theme.c("border")))
                .corner_radius(theme.radius_md())
                .inner_margin(egui::Margin::symmetric(12, 8))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        if ui
                            .add(egui::Button::new("×").frame(false))
                            .on_hover_text(plain(texts, "library.clear"))
                            .clicked()
                        {
                            self.library_view.borrow_mut().0.ticked.clear();
                        }
                        ui.label(texts.t(&gafira_doc::msg!(
                            "library.ticked",
                            count = Count(ticked.len() as u64)
                        )));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let remove =
                                IconButton::new(Icon::Delete, plain(texts, "library.remove"))
                                    .small()
                                    .show(ui, theme);
                            if remove.clicked() {
                                asked.library_remove = ticked.clone();
                            }
                        });
                    });
                });
        }
    }

    /// One file of Biblioteca: its preview, for a design its name and size;
    /// under the pointer a tick box and «···» with «Más información» and
    /// «Quitar de la lista» (spec 068).
    fn library_tile(
        &self,
        ui: &mut egui::Ui,
        theme: &Theme,
        texts: &Texts,
        entry: &crate::LibraryEntry,
        (width, on, ticking): (f32, bool, bool),
        asked: &mut Asked,
    ) {
        let file = entry.path.as_path();
        let design = entry.design;
        let picture_h = if design { width } else { width * 0.75 };
        let label_h = if design { 36.0 } else { 0.0 };
        let (rect, response) = ui.allocate_exact_size(
            Vec2::new(width, picture_h + label_h),
            egui::Sense::click_and_drag(),
        );
        let picture = Rect::from_min_size(rect.min, Vec2::new(width, picture_h));
        let preview = self.library_preview(ui.ctx(), file, design);
        {
            let painter = ui.painter();
            painter.rect_filled(picture, theme.radius_md(), theme.c("surface.sunken"));
            if let Some((texture, size)) = &preview {
                let fit = picture.shrink(if design { 12.0 } else { 0.0 });
                let tex = texture.size_vec2();
                let k = (fit.width() / tex.x).min(fit.height() / tex.y);
                let at = Rect::from_center_size(fit.center(), tex * k);
                let uv = Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0));
                painter.image(texture.id(), at, uv, Color32::WHITE);
                if design {
                    let name = file
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or_default();
                    painter.text(
                        Pos2::new(rect.min.x, picture.max.y + 4.0),
                        Align2::LEFT_TOP,
                        name,
                        theme.font("ui-label"),
                        theme.c("text"),
                    );
                    painter.text(
                        Pos2::new(rect.min.x, picture.max.y + 20.0),
                        Align2::LEFT_TOP,
                        format!("{:.0} × {:.0} px", size.0, size.1),
                        theme.font("caption"),
                        theme.c("text.secondary"),
                    );
                }
            }
        }
        let hovered = ui.rect_contains_pointer(rect);
        if hovered || on {
            ui.painter().rect_stroke(
                picture,
                theme.radius_md(),
                Stroke::new(2.0, theme.c("primary.bg")),
                egui::StrokeKind::Inside,
            );
        }
        let key = (entry.path.clone(), entry.design);
        let id = ui.id().with(("library.tile", &key));
        let menu_open = self.library_view.borrow().0.menu.as_ref() == Some(&key);
        // The tick box, top left, and «···», top right: drawn over the
        // preview, so they take no room in the row (spec 068).
        if hovered || on || ticking {
            let at = Rect::from_min_size(picture.min + Vec2::splat(6.0), Vec2::splat(20.0));
            let tick = ui.interact(at, id.with("tick"), egui::Sense::click());
            let painter = ui.painter();
            let fill = if on {
                theme.c("primary.bg")
            } else {
                theme.c("surface")
            };
            painter.rect(
                at,
                4.0,
                fill,
                Stroke::new(1.5, theme.c("border.strong")),
                egui::StrokeKind::Inside,
            );
            if on {
                let c = at.center();
                painter.line(
                    vec![
                        c + Vec2::new(-5.0, 0.0),
                        c + Vec2::new(-1.5, 3.5),
                        c + Vec2::new(5.0, -4.0),
                    ],
                    Stroke::new(2.0, theme.c("primary.fg")),
                );
            }
            if tick.clicked() {
                let mut view = self.library_view.borrow_mut();
                if on {
                    view.0.ticked.retain(|t| *t != key);
                } else {
                    view.0.ticked.push(key.clone());
                }
            }
        }
        if hovered || menu_open {
            let at = Rect::from_min_size(
                Pos2::new(picture.max.x - 32.0, picture.min.y + 6.0),
                Vec2::new(26.0, 20.0),
            );
            let dots = ui.interact(at, id.with("dots"), egui::Sense::click());
            let painter = ui.painter();
            painter.rect_filled(at, 10.0, theme.c("surface"));
            for dx in [-6.0, 0.0, 6.0] {
                painter.circle_filled(at.center() + Vec2::new(dx, 0.0), 1.8, theme.c("text"));
            }
            if dots.clicked() {
                self.library_view.borrow_mut().0.menu =
                    if menu_open { None } else { Some(key.clone()) };
            }
            if menu_open {
                let area = egui::Area::new(id.with("menu"))
                    .order(egui::Order::Foreground)
                    .fixed_pos(at.left_bottom() + Vec2::new(-120.0, 4.0))
                    .show(ui.ctx(), |ui| {
                        egui::Frame::popup(ui.style()).show(ui, |ui| {
                            ui.set_min_width(150.0);
                            if ui.button(plain(texts, "library.info")).clicked() {
                                let mut view = self.library_view.borrow_mut();
                                view.0.info = Some(entry.clone());
                                view.0.menu = None;
                            }
                            if ui.button(plain(texts, "library.remove")).clicked() {
                                asked.library_remove = vec![key.clone()];
                                self.library_view.borrow_mut().0.menu = None;
                            }
                        });
                    });
                // A click anywhere else closes it.
                let pressed = ui.input(|i| i.pointer.any_pressed());
                let pointer = ui.input(|i| i.pointer.interact_pos());
                if pressed
                    && !dots.contains_pointer()
                    && pointer.is_some_and(|p| !area.response.rect.contains(p))
                {
                    self.library_view.borrow_mut().0.menu = None;
                }
            }
        }
        // A click adds it; the second click of a double click does not add
        // it again (spec 068).
        if response.clicked() && !response.double_clicked() {
            asked.library_add = Some((entry.path.clone(), design, None));
        }
        let at = ui.input(|i| i.pointer.latest_pos());
        if response.dragged()
            && let Some(at) = at
        {
            ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
            if let Some((texture, _)) = &preview {
                let ghost = ui.ctx().layer_painter(egui::LayerId::new(
                    egui::Order::Tooltip,
                    egui::Id::new("editor.library_ghost"),
                ));
                let tex = texture.size_vec2();
                let k = 96.0 / tex.x.max(tex.y);
                let uv = Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0));
                ghost.image(
                    texture.id(),
                    Rect::from_center_size(at, tex * k),
                    uv,
                    Color32::WHITE.gamma_multiply(0.7),
                );
            }
        }
        if response.drag_stopped()
            && let Some(at) = at
            && canvas::drop_point(&self.page_rects, self.zoom, at).is_some()
        {
            asked.library_add = Some((entry.path.clone(), design, Some(at)));
        }
        let name = file
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default();
        response.on_hover_text(name);
    }

    /// «Más información» (spec 068): the file's name with a way back, its
    /// preview, and «Detalles»: folder, kind, format, weight, size in px and
    /// when it was imported.
    fn library_info(
        &self,
        ui: &mut egui::Ui,
        theme: &Theme,
        texts: &Texts,
        entry: &crate::LibraryEntry,
        asked: &mut Asked,
    ) {
        let file = entry.path.as_path();
        let name = file
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default();
        ui.horizontal(|ui| {
            let back = IconButton::new(Icon::Back, plain(texts, "library.back"))
                .small()
                .show(ui, theme);
            if back.clicked() {
                self.library_view.borrow_mut().0.info = None;
            }
            ui.label(
                RichText::new(name)
                    .font(theme.font("ui-label"))
                    .color(theme.c("text")),
            );
        });
        ui.add_space(12.0);
        let preview = self.library_preview(ui.ctx(), file, entry.design);
        let width = ui.available_width();
        let (rect, _) = ui.allocate_exact_size(Vec2::new(width, width * 0.7), egui::Sense::hover());
        ui.painter()
            .rect_filled(rect, theme.radius_md(), theme.c("surface.sunken"));
        if let Some((texture, _)) = &preview {
            let fit = rect.shrink(16.0);
            let tex = texture.size_vec2();
            let k = (fit.width() / tex.x).min(fit.height() / tex.y);
            let uv = Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0));
            ui.painter().image(
                texture.id(),
                Rect::from_center_size(fit.center(), tex * k),
                uv,
                Color32::WHITE,
            );
        }
        ui.add_space(16.0);
        ui.label(RichText::new(plain(texts, "library.details")).font(theme.font("ui-label")));
        ui.add_space(8.0);
        let folder = file
            .parent()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .unwrap_or_default()
            .to_owned();
        let format = file
            .extension()
            .and_then(|e| e.to_str())
            .map(str::to_ascii_uppercase)
            .unwrap_or_default();
        let kind = plain(
            texts,
            if entry.design {
                "library.kind_design"
            } else {
                "library.kind_image"
            },
        );
        let weight = std::fs::metadata(file)
            .map(|m| crate::library::bytes(m.len()))
            .unwrap_or_default();
        let pixels = preview
            .as_ref()
            .map(|(_, (w, h))| format!("{w:.0} × {h:.0} px"))
            .unwrap_or_default();
        let rows = [
            ("library.saved_in", folder),
            ("library.kind", kind),
            ("library.format", format),
            ("library.weight", weight),
            ("library.pixels", pixels),
            ("library.added", crate::library::date(entry.added)),
        ];
        egui::Grid::new("library.details")
            .num_columns(2)
            .spacing([16.0, 8.0])
            .show(ui, |ui| {
                for (key, value) in rows {
                    ui.label(RichText::new(plain(texts, key)).color(theme.c("text.secondary")));
                    ui.label(value);
                    ui.end_row();
                }
            });
        ui.add_space(16.0);
        if ui.button(plain(texts, "library.remove")).clicked() {
            asked.library_remove = vec![(entry.path.clone(), entry.design)];
        }
        ui.label(
            RichText::new(file.display().to_string())
                .small()
                .color(theme.c("text.muted")),
        );
    }

    /// The preview of a file of Biblioteca, made once: a thumbnail of an
    /// image, or the grey silhouette of a design with its size in px.
    fn library_preview(
        &self,
        ctx: &egui::Context,
        file: &std::path::Path,
        design: bool,
    ) -> Option<(egui::TextureHandle, (f64, f64))> {
        let mut previews = self.library_previews.borrow_mut();
        previews
            .entry(file.to_path_buf())
            .or_insert_with(|| {
                let name = format!("library:{}", file.display());
                if design {
                    let outline = vector_outline(file)?;
                    let size = outline.size;
                    let image = canvas::outline_preview(&outline, 256.0)?;
                    Some((
                        ctx.load_texture(name, image, egui::TextureOptions::LINEAR),
                        size,
                    ))
                } else {
                    let decoded = image::open(file).ok()?;
                    let size = (f64::from(decoded.width()), f64::from(decoded.height()));
                    let rgba = decoded.thumbnail(320, 320).to_rgba8();
                    let image = egui::ColorImage::from_rgba_unmultiplied(
                        [rgba.width() as usize, rgba.height() as usize],
                        rgba.as_raw(),
                    );
                    Some((
                        ctx.load_texture(name, image, egui::TextureOptions::LINEAR),
                        size,
                    ))
                }
            })
            .clone()
    }

    /// A custom frame of `outline` (S66-10), centred at `at` (screen) on the
    /// page under it, or in the middle of the current page; selected.
    fn place_custom_frame(
        &mut self,
        engine: &mut Gafira,
        outline: gafira_io_vector::Outline,
        at: Option<Pos2>,
    ) {
        let Some(document) = engine.edit().document() else {
            return;
        };
        let dropped = at.and_then(|at| canvas::drop_point(&self.page_rects, self.zoom, at));
        let page = match dropped {
            Some((page, _)) => page,
            None => match self.current_page(&document) {
                Some(page) => page.id,
                None => return,
            },
        };
        let Some(on) = document.page(page) else {
            return;
        };
        let Ok(size) = on.size.to_px(document.dpi()) else {
            return;
        };
        let px = Vec2::new(size.width() as f32, size.height() as f32);
        let mut spec = canvas::new_custom_frame(px, outline);
        if let Some((_, point)) = dropped {
            spec.placement.x = point.0 - spec.placement.w / 2.0;
            spec.placement.y = point.1 - spec.placement.h / 2.0;
        }
        let next = NodeId::new(document.next_id());
        let command = InsertNodes::new(page, None, on.roots.len(), vec![spec]);
        match engine.edit().execute(Box::new(command)) {
            Ok(_) => {
                self.selected = Selection::one(page, next);
                self.active = Some(page);
            }
            Err(error) => eprintln!("custom frame: {error}"),
        }
    }

    /// «Importar forma (SVG o PDF)…» of Elementos → Marcos (S66-10).
    fn import_frame_shape(&mut self, engine: &mut Gafira) {
        let picked = rfd::FileDialog::new()
            .add_filter("SVG, PDF", &["svg", "pdf"])
            .pick_file();
        if let Some(path) = picked
            && let Some(outline) = vector_outline(&path)
        {
            self.place_custom_frame(engine, outline, None);
        }
    }

    /// An image pasted from the system clipboard (spec 067): stored as a
    /// PNG and placed in the middle of the current page.
    fn paste_image(&mut self, engine: &mut Gafira) {
        let Ok(mut clipboard) = arboard::Clipboard::new() else {
            return;
        };
        let Ok(data) = clipboard.get_image() else {
            return;
        };
        let (Ok(w), Ok(h)) = (u32::try_from(data.width), u32::try_from(data.height)) else {
            return;
        };
        let Some(pixels) = image::RgbaImage::from_raw(w, h, data.bytes.into_owned()) else {
            return;
        };
        let mut png = Vec::new();
        if let Err(error) =
            pixels.write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
        {
            eprintln!("paste image: {error}");
            return;
        }
        match engine.assets().import_bytes("pasted.png", &png) {
            Ok(asset) => self.place_image(engine, asset, (w, h), None, false),
            Err(error) => eprintln!("paste image: {error}"),
        }
    }

    /// Place the image `asset` of `natural` px: in the frame under `at` when
    /// `into_frame`, else as an image centred at `at` (screen) on the page
    /// under it, or in the middle of the current page.
    fn place_image(
        &mut self,
        engine: &mut Gafira,
        asset: gafira_doc::timeline::AssetRef,
        natural: (u32, u32),
        at: Option<Pos2>,
        into_frame: bool,
    ) {
        let Some(document) = engine.edit().document() else {
            return;
        };
        let dropped = at.and_then(|at| {
            let (page, point) = canvas::drop_point(&self.page_rects, self.zoom, at)?;
            Some((page, point, at))
        });
        let page = match dropped {
            Some((page, _, _)) => page,
            None => match self.current_page(&document) {
                Some(page) => page.id,
                None => return,
            },
        };
        let Some(on) = document.page(page) else {
            return;
        };
        // Over a frame: the photo of the frame (spec 066).
        if into_frame
            && let Some((_, _, screen)) = dropped
            && let Some((_, rect)) = self.page_rects.iter().find(|(id, _)| *id == page)
            && let Some(frame) = canvas::hit(&canvas::shapes(on), *rect, self.zoom, screen)
            && frame_of(&document, page, frame).is_some()
        {
            let content = gafira_doc::FrameContent { asset, natural };
            let command = SetProperty::new(
                page,
                vec![frame],
                Property::FrameContent(Some(content)),
                None,
            );
            match engine.edit().execute(Box::new(command)) {
                Ok(_) => self.selected = Selection::one(page, frame),
                Err(error) => eprintln!("import into frame: {error}"),
            }
            return;
        }
        let Ok(size) = on.size.to_px(document.dpi()) else {
            return;
        };
        let px = Vec2::new(size.width() as f32, size.height() as f32);
        let mut spec = canvas::new_image(px, natural, asset);
        if let Some((_, point, _)) = dropped {
            spec.placement.x = point.0 - spec.placement.w / 2.0;
            spec.placement.y = point.1 - spec.placement.h / 2.0;
        }
        let next = NodeId::new(document.next_id());
        let command = InsertNodes::new(page, None, on.roots.len(), vec![spec]);
        match engine.edit().execute(Box::new(command)) {
            Ok(_) => {
                self.selected = Selection::one(page, next);
                self.active = Some(page);
            }
            Err(error) => eprintln!("import image: {error}"),
        }
    }

    /// The cells of the table `node` a change goes to: the chosen ones, or
    /// all.
    fn cell_range(&self, node: NodeId, table: &gafira_doc::TableNode) -> gafira_table::Range {
        self.cells
            .as_ref()
            .filter(|cells| cells.node == node)
            .map_or_else(
                || gafira_table::Range::all(table),
                |cells| cells.range(table),
            )
    }

    /// The controls of a table in the bar over it (S81-10).
    fn table_bar(
        &self,
        ui: &mut egui::Ui,
        theme: &Theme,
        texts: &Texts,
        (node, table): (NodeId, &gafira_doc::TableNode),
        asked: &mut Asked,
    ) {
        use egui::containers::menu::{MenuButton, MenuConfig};
        let range = self.cell_range(node, table);
        let stay =
            || MenuConfig::new().close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside);
        // A menu holds choices; these hold controls, in their own looks.
        let controls = |ui: &mut egui::Ui, width: f32| {
            *ui.style_mut() = (*ui.ctx().style()).clone();
            ui.set_width(width);
        };
        if let Some(act) = tables::look_controls(ui, theme, texts, table, range) {
            asked.table_act = Some(act);
        }
        MenuButton::new(plain(texts, "table.align"))
            .config(stay())
            .ui(ui, |ui| {
                controls(ui, 200.0);
                if let Some(act) = tables::align_controls(ui, theme, texts, table, range) {
                    asked.table_act = Some(act);
                }
            });
        MenuButton::new(plain(texts, "table.borders"))
            .config(stay())
            .ui(ui, |ui| {
                controls(ui, 200.0);
                let mut pen = self.table_pen.get();
                if let Some(act) = tables::border_controls(ui, theme, texts, &mut pen) {
                    asked.table_act = Some(act);
                }
                self.table_pen.set(pen);
            });
        MenuButton::new(plain(texts, "table.room"))
            .config(stay())
            .ui(ui, |ui| {
                controls(ui, 240.0);
                if let Some(act) = tables::spacing_controls(ui, theme, texts, table) {
                    asked.table_act = Some(act);
                }
            });
    }

    /// The selected table on the canvas (S81-08, S81-09): the cells chosen,
    /// the borders that resize, the buttons of the row and the column, the
    /// «+» at its ends and the one that moves it. Whether it took the press.
    fn table_overlay(
        &mut self,
        ui: &mut egui::Ui,
        (theme, texts): (&Theme, &Texts),
        (response, pointer): (&egui::Response, Option<Pos2>),
        (page, zoom): (Rect, f32),
        shape: &Shape,
        asked: &mut Asked,
    ) -> bool {
        use gafira_table::{Layout, Range};
        let Some(table) = shape.table.as_deref() else {
            return false;
        };
        let placement = shape.placement;
        let (w, h) = (placement.w, placement.h);
        let turn = rotation::to_screen(page, zoom, &placement);
        let layout = Layout::of(table, (w, h));
        let local = |at: Pos2| rotation::local(turn, at);
        let cell_at = |at: Pos2| local(at).and_then(|p| layout.at(table, p.x, p.y));
        let writing = self
            .cells
            .as_ref()
            .is_some_and(|c| c.node == shape.node && c.writing.is_some());
        let slack = f64::from(tables::EDGE_SLACK / zoom);
        // The border of a column or a row near a point of the screen.
        let edge_at = |at: Pos2| -> Option<(bool, usize)> {
            if canvas::grip_hit(turn, &placement, at, false).is_some() {
                return None;
            }
            let p = local(at)?;
            let inside = p.x >= -slack && p.x <= w + slack && p.y >= -slack && p.y <= h + slack;
            if !inside {
                return None;
            }
            layout
                .column_edge(p.x, slack)
                .map(|i| (true, i))
                .or_else(|| layout.row_edge(p.y, slack).map(|i| (false, i)))
        };
        let shift = ui.input(|i| i.modifiers.shift);
        let mut took = self.table_drag.is_some();
        if response.drag_started()
            && let Some(at) = pointer
        {
            if let (Some((column, index)), Some(from)) = (edge_at(at), local(at)) {
                let start = Box::new(TableStart {
                    table: table.clone(),
                    placement,
                    at: from,
                });
                self.table_drag = Some(if column {
                    TableDrag::Column(index, start)
                } else {
                    TableDrag::Row(index, start)
                });
                took = true;
            } else if self.cells.as_ref().is_some_and(|c| c.node == shape.node)
                && canvas::grip_hit(turn, &placement, at, false).is_none()
                && let Some(cell) = cell_at(at)
            {
                asked.cells = Some(Some(tables::Cells::one(shape.page, shape.node, cell)));
                self.table_drag = Some(TableDrag::Cells);
                took = true;
            }
        }
        if response.dragged()
            && let Some(at) = ui.input(|i| i.pointer.latest_pos())
        {
            match &self.table_drag {
                Some(TableDrag::Column(index, start) | TableDrag::Row(index, start)) => {
                    let column = matches!(self.table_drag, Some(TableDrag::Column(..)));
                    let turn = rotation::to_screen(page, zoom, &start.placement);
                    if let Some(p) = rotation::local(turn, at) {
                        let act = if column {
                            tables::Act::ResizeColumn(*index, p.x - start.at.x)
                        } else {
                            tables::Act::ResizeRow(*index, p.y - start.at.y)
                        };
                        let all = Range::all(&start.table);
                        asked.table_sized =
                            tables::change(&start.table, &start.placement, all, &act)
                                .map(|(table, placement, _)| (table, placement));
                    }
                }
                Some(TableDrag::Cells) => {
                    if let (Some(cell), Some(mut cells)) = (cell_at(at), self.cells.clone()) {
                        cells.to = cell;
                        asked.cells = Some(Some(cells));
                    }
                }
                None => {}
            }
        }
        if response.drag_stopped() && self.table_drag.take().is_some() {
            asked.table_rest = true;
        }
        if response.clicked()
            && let Some(at) = pointer
            && let Some(cell) = cell_at(at)
        {
            took = true;
            let mut cells = tables::Cells::one(shape.page, shape.node, cell);
            if shift && let Some(now) = self.cells.as_ref().filter(|c| c.node == shape.node) {
                cells.from = now.from;
            }
            asked.cells = Some(Some(cells));
            if response.double_clicked() {
                asked.write_cell = Some(None);
            }
        }
        if response.secondary_clicked()
            && let Some(at) = response.interact_pointer_pos()
            && let Some(cell) = cell_at(at)
        {
            if !self.cell_range(shape.node, table).contains(cell.0, cell.1) || self.cells.is_none()
            {
                asked.cells = Some(Some(tables::Cells::one(shape.page, shape.node, cell)));
            }
            self.table_menu = Some((tables::Side::Cells, at));
            took = true;
        }
        // The cells chosen.
        let painter = ui.painter().clone();
        let ring = tables::ring(theme);
        let corners = |rect: kurbo::Rect| {
            [
                (rect.x0, rect.y0),
                (rect.x1, rect.y0),
                (rect.x1, rect.y1),
                (rect.x0, rect.y1),
            ]
            .map(|(x, y)| rotation::screen(turn, x, y))
            .to_vec()
        };
        let chosen = self.cells.as_ref().filter(|c| c.node == shape.node);
        if let Some(cells) = chosen {
            let range = cells.range(table);
            let first = layout.cell(table, cells.from.0, cells.from.1);
            let outline = corners(layout.range(range));
            painter.add(egui::Shape::convex_polygon(
                outline.clone(),
                ring.gamma_multiply(if cells.from == cells.to { 0.0 } else { 0.15 }),
                Stroke::NONE,
            ));
            // A white line under it, to be seen over any fill.
            for (width, color) in [(4.0, Color32::WHITE), (2.0, ring)] {
                let stroke = Stroke::new(width, color);
                painter.add(egui::Shape::closed_line(outline.clone(), stroke));
            }
            painter.add(egui::Shape::closed_line(
                corners(first),
                Stroke::new(1.0, ring),
            ));
        }
        let hover = response.hover_pos();
        if !writing
            && self.table_drag.is_none()
            && let Some(at) = hover
            && let Some((column, _)) = edge_at(at)
        {
            ui.ctx().set_cursor_icon(if column {
                egui::CursorIcon::ResizeColumn
            } else {
                egui::CursorIcon::ResizeRow
            });
        }
        // The row and the column at hand: the chosen ones, or those of the
        // cell the pointer was over last, while it is near.
        if let Some(cell) = hover.and_then(cell_at) {
            self.table_hover = Some((shape.node, cell));
        } else if ui.input(|i| i.pointer.latest_pos()).is_none_or(|at| {
            !rotation::screen_box(page, zoom, &placement)
                .expand(48.0)
                .contains(at)
        }) {
            self.table_hover = None;
        }
        let at_hand = chosen.map(|c| c.range(table)).or_else(|| {
            self.table_hover
                .filter(|(node, _)| *node == shape.node)
                .map(|(_, (r, c))| Range::cell(r, c).over(table))
        });
        if self.table_drag.is_some() {
            return took;
        }
        let way = |x: f64, y: f64| {
            let d = rotation::screen(turn, x, y) - rotation::screen(turn, 0.0, 0.0);
            d / d.length().max(1e-3)
        };
        let (right, down) = (way(1.0, 0.0), way(0.0, 1.0));
        let id = ui.id().with(("table", shape.node));
        let all = Range::all(table);
        if let Some(range) = at_hand {
            let rect = layout.range(range);
            let spots = [
                (
                    tables::Side::Columns,
                    rotation::screen(turn, rect.center().x, 0.0) - down * 14.0,
                    Vec2::new(30.0, 16.0),
                    Vec2::new(6.0, 0.0),
                    "table.column_menu",
                ),
                (
                    tables::Side::Rows,
                    rotation::screen(turn, 0.0, rect.center().y) - right * 14.0,
                    Vec2::new(16.0, 30.0),
                    Vec2::new(0.0, 6.0),
                    "table.row_menu",
                ),
            ];
            let knob = rotation::rotate_handle(turn, &placement);
            for (side, mut centre, size, dots, key) in spots {
                // Clear of the turn handle.
                if centre.distance(knob) < 30.0 {
                    centre += right * 34.0;
                }
                let spot = Rect::from_center_size(centre, size);
                let name = plain(texts, key);
                let mark = tables::Mark::Dots(dots);
                if tables::pill(ui, theme, id.with(key), spot, mark, &name).clicked() {
                    let (from, to) = if side == tables::Side::Columns {
                        ((0, range.c0), (all.r1, range.c1))
                    } else {
                        ((range.r0, 0), (range.r1, all.c1))
                    };
                    let mut cells = tables::Cells::one(shape.page, shape.node, from);
                    cells.to = to;
                    asked.cells = Some(Some(cells));
                    self.table_menu = Some((side, spot.right_bottom()));
                    took = true;
                }
            }
        }
        let ends = [
            (
                tables::Act::ColumnLast,
                rotation::screen(turn, w, h / 2.0) + right * 22.0,
                "table.column_after",
            ),
            (
                tables::Act::RowLast,
                rotation::screen(turn, w / 2.0, h) + down * 22.0,
                "table.row_after",
            ),
        ];
        for (act, centre, key) in ends {
            let spot = Rect::from_center_size(centre, Vec2::splat(22.0));
            let name = plain(texts, key);
            let button = id.with(("end", key));
            if tables::change(table, &placement, all, &act).is_some()
                && tables::pill(ui, theme, button, spot, tables::Mark::Plus, &name).clicked()
            {
                asked.table_act = Some(act);
                took = true;
            }
        }
        // The table moves as a whole, also while its cells are chosen.
        let spot = Rect::from_center_size(
            rotation::screen(turn, w / 2.0, h) + down * 22.0 + right * 30.0,
            Vec2::splat(22.0),
        );
        let name = plain(texts, "table.move");
        let moving = tables::pill(ui, theme, id.with("move"), spot, tables::Mark::Move, &name);
        if moving.drag_started() {
            asked.drag_begin = Some(None);
            took = true;
        }
        self.table_menu(ui.ctx(), texts, (table, &placement), shape.node, asked);
        took
    }

    /// The menu open over a table (S81-09).
    fn table_menu(
        &mut self,
        ctx: &egui::Context,
        texts: &Texts,
        (table, placement): (&gafira_doc::TableNode, &Placement),
        node: NodeId,
        asked: &mut Asked,
    ) {
        let Some((side, at)) = self.table_menu else {
            return;
        };
        let range = self.cell_range(node, table);
        let mut open = true;
        let mut picked = None;
        let room = ctx.content_rect().height() - 48.0;
        let layer = egui::LayerId::new(egui::Order::Foreground, egui::Id::new("table.menu"));
        // The press that opened it does not close it.
        let fresh = ctx.input(|i| i.pointer.any_click());
        egui::Popup::new(
            egui::Id::new("table.menu"),
            ctx.clone(),
            egui::PopupAnchor::Position(at),
            layer,
        )
        .kind(egui::PopupKind::Menu)
        .style(egui::containers::menu::menu_style)
        .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
        .open_bool(&mut open)
        .show(|ui| {
            egui::ScrollArea::vertical()
                .max_height(room)
                .min_scrolled_height(room)
                .show(ui, |ui| {
                    picked = tables::menu(
                        ui,
                        texts,
                        (table, placement),
                        range,
                        side,
                        (
                            self.table_style.is_some(),
                            &ALIGNS.map(|(_, key, _, _)| key),
                        ),
                    );
                });
        });
        match picked {
            Some(tables::Picked::Act(act)) => asked.table_act = Some(act),
            Some(tables::Picked::Menu(menu)) => asked.table_menu = Some(menu),
            None => {}
        }
        let escaped = ctx.input(|i| i.key_pressed(egui::Key::Escape));
        if picked_or_closed(&asked.table_act, asked.table_menu, open || fresh) || escaped {
            self.table_menu = None;
        }
    }

    /// The keys of the cells chosen (S81-08, S81-11): moving, writing,
    /// emptying, and the clipboard. Whether there are any.
    fn cell_keys(
        &mut self,
        ctx: &egui::Context,
        document: Option<&Document>,
        asked: &mut Asked,
    ) -> bool {
        use egui::Key;
        let Some(cells) = self.cells.clone().filter(|c| c.writing.is_none()) else {
            return false;
        };
        let Some((table, _)) = document.and_then(|d| table_of(d, cells.page, cells.node)) else {
            return false;
        };
        if self.selected.single() != Some((cells.page, cells.node)) {
            return false;
        }
        let one = |cell| tables::Cells::one(cells.page, cells.node, cell);
        let (events, modifiers) = ctx.input_mut(|i| {
            // Tab stays here, it does not go to another control.
            i.consume_key(Modifiers::NONE, Key::Tab);
            i.consume_key(Modifiers::SHIFT, Key::Tab);
            (i.raw.events.clone(), i.modifiers)
        });
        for event in events {
            match event {
                egui::Event::Copy if modifiers.alt => {
                    asked.table_menu = Some(tables::Menu::CopyStyle)
                }
                egui::Event::Copy => asked.table_menu = Some(tables::Menu::Copy),
                egui::Event::Cut => {
                    asked.table_menu = Some(tables::Menu::Copy);
                    asked.table_act = Some(tables::Act::Clear);
                }
                egui::Event::Paste(_) if modifiers.alt => {
                    asked.table_menu = Some(tables::Menu::PasteStyle);
                }
                egui::Event::Paste(text) => {
                    asked.table_act = Some(tables::Act::Paste(gafira_table::parse_grid(&text)));
                }
                egui::Event::Text(text) if !modifiers.command => {
                    // Several letters may come in one frame.
                    match &mut asked.write_cell {
                        Some(Some(words)) => words.push_str(&text),
                        _ if !text.trim().is_empty() => asked.write_cell = Some(Some(text)),
                        _ => {}
                    }
                }
                egui::Event::Key {
                    key,
                    pressed: true,
                    modifiers,
                    ..
                } => {
                    let steps = match key {
                        Key::ArrowUp => Some((-1, 0)),
                        Key::ArrowDown => Some((1, 0)),
                        Key::ArrowLeft => Some((0, -1)),
                        Key::ArrowRight => Some((0, 1)),
                        _ => None,
                    };
                    if let Some((rows, cols)) = steps {
                        let mut next = cells.clone();
                        if modifiers.shift {
                            let end = one(cells.to).step(&table, rows, cols);
                            next.to = end;
                        } else {
                            next = one(cells.step(&table, rows, cols));
                        }
                        asked.cells = Some(Some(next));
                        continue;
                    }
                    match key {
                        Key::Escape => asked.cells = Some(None),
                        Key::Enter | Key::F2 => asked.write_cell = Some(None),
                        Key::Tab => match cells.tab(&table, modifiers.shift) {
                            Some(cell) => asked.cells = Some(Some(one(cell))),
                            None if !modifiers.shift => {
                                asked.table_act = Some(tables::Act::RowLast);
                            }
                            None => {}
                        },
                        Key::Delete | Key::Backspace => {
                            asked.table_act = Some(tables::Act::Clear);
                        }
                        Key::A if modifiers.command => {
                            let all = gafira_table::Range::all(&table);
                            let mut next = one((0, 0));
                            next.to = (all.r1, all.c1);
                            asked.cells = Some(Some(next));
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }
        true
    }

    /// The field of the cell being written (S81-08), over it, in its looks.
    fn cell_field(
        &mut self,
        ui: &mut egui::Ui,
        theme: &Theme,
        pages: &[Arc<Page>],
        page_rects: &[(PageId, Rect)],
        zoom: f32,
        asked: &mut Asked,
    ) {
        let Some(cells) = self.cells.as_mut() else {
            return;
        };
        let (page_id, node_id, (r, c)) = (cells.page, cells.node, cells.from);
        let Some(writing) = cells.writing.as_mut() else {
            return;
        };
        let Some((page, rect)) = pages.iter().find(|p| p.id == page_id).zip(
            page_rects
                .iter()
                .find(|(id, _)| *id == page_id)
                .map(|p| p.1),
        ) else {
            return;
        };
        let Some(node) = page.node(node_id) else {
            return;
        };
        let gafira_doc::NodeKind::Table(table) = &node.kind else {
            return;
        };
        let Some(cell) = table.cell(r, c) else {
            return;
        };
        let placement = node.base.placement;
        let layout = gafira_table::Layout::of(table, (placement.w, placement.h));
        let own = layout.cell(table, r, c);
        let turn = rotation::to_screen(rect, zoom, &placement);
        let centre = rotation::screen(turn, own.center().x, own.center().y);
        let size = Vec2::new(own.width() as f32, own.height() as f32) * zoom;
        let pad = table.padding as f32 * zoom;
        let place = Rect::from_center_size(centre, size);
        let inner = Rect::from_min_max(
            Pos2::new(place.left() + pad.min(size.x / 3.0), place.top()),
            Pos2::new(place.right() - pad.min(size.x / 3.0), place.bottom()),
        );
        // Enter goes down, Tab on, Escape ends; Shift+Enter breaks the line.
        let keys = ui.input_mut(|i| {
            let shift_tab = i.consume_key(Modifiers::SHIFT, egui::Key::Tab);
            (
                i.consume_key(Modifiers::NONE, egui::Key::Escape),
                shift_tab,
                i.consume_key(Modifiers::NONE, egui::Key::Tab),
                !i.modifiers.shift
                    && !i.modifiers.alt
                    && i.consume_key(Modifiers::NONE, egui::Key::Enter),
            )
        });
        let ended = match keys {
            (true, ..) => Some(CellMove::Stay),
            (_, true, ..) => Some(CellMove::Before),
            (_, _, true, _) => Some(CellMove::Next),
            (.., true) => Some(CellMove::Down),
            _ => None,
        };
        let style = span_of(&cell.style);
        let font = egui::FontId::new(
            (style.size as f32 * zoom).max(1.0),
            text_family(ui.ctx(), theme, &style),
        );
        let halign = match cell.style.align {
            gafira_doc::TextAlign::Center => Align::Center,
            gafira_doc::TextAlign::Right => Align::RIGHT,
            gafira_doc::TextAlign::Left | gafira_doc::TextAlign::Justify => Align::LEFT,
        };
        let valign = match cell.style.valign {
            gafira_doc::VerticalAlign::Top => Align::TOP,
            gafira_doc::VerticalAlign::Middle => Align::Center,
            gafira_doc::VerticalAlign::Bottom => Align::BOTTOM,
        };
        let field = egui::TextEdit::multiline(&mut writing.words)
            .id(cell_id(node_id, (r, c)))
            .lock_focus(true)
            .frame(false)
            .margin(egui::Margin::ZERO)
            .background_color(Color32::TRANSPARENT)
            .font(font)
            .text_color(canvas::screen_color(cell.style.color))
            .horizontal_align(halign)
            .vertical_align(valign)
            .desired_rows(1)
            .desired_width(inner.width());
        let response = ui.put(inner, field);
        if response.lost_focus() {
            asked.end_cell = Some(CellMove::Stay);
        }
        if response.changed() {
            asked.cell_words = Some(writing.words.clone());
        }
        if ended.is_some() {
            asked.end_cell = ended;
        }
    }

    /// Changes the selected table; the cells chosen follow. Within
    /// `gesture`, the changes are one undo step.
    fn change_table(
        &mut self,
        engine: &mut Gafira,
        act: &tables::Act,
        gesture: Option<GestureId>,
    ) -> bool {
        let Some((page, node)) = self.selected.single() else {
            return false;
        };
        let Some((table, placement)) = engine
            .edit()
            .document()
            .and_then(|d| table_of(&d, page, node))
        else {
            return false;
        };
        let range = self.cell_range(node, &table);
        let Some((next, box_of, now)) = tables::change(&table, &placement, range, act) else {
            return false;
        };
        let done = set_table(engine, (page, node), next, box_of, gesture);
        if done && let Some(cells) = self.cells.as_mut().filter(|c| c.node == node) {
            cells.from = (now.r0, now.c0);
            cells.to = (now.r1, now.c1);
        }
        done
    }

    /// The tables asked for this frame (spec 081).
    fn tables(&mut self, engine: &mut Gafira, asked: &Asked) {
        if let Some((template, at)) = asked.insert_table {
            self.insert_table(engine, template, at);
        }
        // The writing ends before anything else happens to the table.
        if let Some(words) = &asked.cell_words
            && let Some(cells) = self.cells.as_mut()
            && let Some(writing) = cells.writing.as_mut()
        {
            writing.words.clone_from(words);
            let (cell, gesture) = (cells.from, writing.gesture);
            self.change_table(engine, &tables::Act::Words(cell, words.clone()), gesture);
        }
        if let Some(to) = asked.end_cell {
            self.end_cell(engine, to);
        }
        if let Some(cells) = &asked.cells {
            if self.cells.as_ref().is_some_and(|c| c.writing.is_some()) {
                self.end_cell(engine, CellMove::Stay);
            }
            self.cells.clone_from(cells);
        }
        if let Some(words) = &asked.write_cell {
            self.write_cell(engine, words.clone());
        }
        if let Some((table, placement)) = &asked.table_sized
            && let Some(at) = self.selected.single()
        {
            let gesture = self.table_step(engine);
            set_table(engine, at, table.clone(), *placement, gesture);
        }
        if let Some(act) = &asked.table_act {
            // A slider or a colour picker goes on changing: one step until
            // it is let go.
            let gesture = match act {
                tables::Act::Fill(_)
                | tables::Act::Ink(_)
                | tables::Act::Size(_)
                | tables::Act::Spacing(_)
                | tables::Act::Padding(_) => self.table_step(engine),
                _ => {
                    self.end_table_step(engine);
                    None
                }
            };
            self.change_table(engine, act, gesture);
        }
        match asked.table_menu {
            Some(tables::Menu::CopyStyle) => {
                self.table_style = self.first_cell(engine).map(|cell| cell.style);
            }
            Some(tables::Menu::PasteStyle) => {
                if let Some(style) = self.table_style.clone() {
                    let act = tables::Act::PasteStyle(Box::new(style));
                    self.change_table(engine, &act, None);
                }
            }
            Some(tables::Menu::Excel) => self.save_excel(engine),
            // These are done elsewhere, in the same frame.
            Some(
                tables::Menu::Copy
                | tables::Menu::Duplicate
                | tables::Menu::Delete
                | tables::Menu::Lock
                | tables::Menu::Align(_),
            )
            | None => {}
        }
        if asked.table_rest {
            self.end_table_step(engine);
        }
    }

    /// The gesture of the changes being made to a table, opened if needed.
    fn table_step(&mut self, engine: &mut Gafira) -> Option<GestureId> {
        if self.table_gesture.is_none() {
            self.table_gesture = engine.edit().begin_gesture().ok();
        }
        self.table_gesture
    }

    fn end_table_step(&mut self, engine: &mut Gafira) {
        if self.table_gesture.take().is_some()
            && let Err(error) = engine.edit().end_gesture()
        {
            eprintln!("end gesture: {error}");
        }
    }

    /// The first of the cells chosen in the selected table.
    fn first_cell(&self, engine: &mut Gafira) -> Option<gafira_doc::TableCell> {
        let (page, node) = self.selected.single()?;
        let (table, _) = table_of(&*engine.edit().document()?, page, node)?;
        let range = self.cell_range(node, &table);
        table.cell(range.r0, range.c0).cloned()
    }

    /// Starts writing in the first cell chosen, over `words` or its own.
    fn write_cell(&mut self, engine: &mut Gafira, words: Option<String>) {
        self.end_table_step(engine);
        let Some(cell) = self.cells.as_ref().map(|c| c.from) else {
            return;
        };
        let own = self.cells.as_ref().and_then(|c| {
            let (table, _) = table_of(&*engine.edit().document()?, c.page, c.node)?;
            Some(table.cell(cell.0, cell.1)?.text.clone())
        });
        let gesture = engine.edit().begin_gesture().ok();
        if let Some(cells) = self.cells.as_mut() {
            cells.to = cells.from;
            cells.writing = Some(tables::Writing {
                words: words.clone().or(own).unwrap_or_default(),
                gesture,
                focused: false,
            });
        }
        if let Some(words) = words {
            self.change_table(engine, &tables::Act::Words(cell, words), gesture);
        }
    }

    /// Ends the writing in a cell and goes on to another; Tab in the last
    /// one adds a row (S81-08).
    fn end_cell(&mut self, engine: &mut Gafira, to: CellMove) {
        let Some(cells) = self.cells.as_mut() else {
            return;
        };
        if cells.writing.take().is_some_and(|w| w.gesture.is_some()) {
            let _ = engine.edit().end_gesture();
        }
        let cells = cells.clone();
        let Some((table, _)) = engine
            .edit()
            .document()
            .and_then(|d| table_of(&d, cells.page, cells.node))
        else {
            return;
        };
        let next = match to {
            CellMove::Stay => None,
            CellMove::Down => Some(cells.step(&table, 1, 0)),
            CellMove::Before => cells.tab(&table, true),
            CellMove::Next => match cells.tab(&table, false) {
                Some(cell) => Some(cell),
                None => {
                    self.change_table(engine, &tables::Act::RowLast, None);
                    None
                }
            },
        };
        if let Some(cell) = next {
            self.cells = Some(tables::Cells::one(cells.page, cells.node, cell));
        }
    }

    /// A new table, centred where it was dropped or in the middle of the
    /// current page, selected; one undo step (S81-07).
    fn insert_table(
        &mut self,
        engine: &mut Gafira,
        template: crate::table_templates::Template,
        at: Option<Pos2>,
    ) {
        let Some(document) = engine.edit().document() else {
            return;
        };
        let dropped = at.and_then(|at| canvas::drop_point(&self.page_rects, self.zoom, at));
        let page = match dropped {
            Some((page, _)) => Some(page),
            None => self.current_page(&document).map(|p| p.id),
        };
        let Some(on) = page.and_then(|page| document.page(page)) else {
            return;
        };
        let Ok(size) = on.size.to_px(document.dpi()) else {
            return;
        };
        let texts = Texts::new(engine.settings().locale().locale);
        let name = plain(&texts, template.key());
        let spec = crate::table_templates::spec(
            template.table(&texts),
            (size.width(), size.height()),
            dropped.map(|(_, point)| point),
            name,
        );
        let next = NodeId::new(document.next_id());
        let command = InsertNodes::new(on.id, None, on.roots.len(), vec![spec]);
        match engine.edit().execute(Box::new(command)) {
            Ok(_) => {
                self.selected = Selection::one(on.id, next);
                self.active = Some(on.id);
                self.scope = None;
                self.cells = None;
            }
            Err(error) => eprintln!("insert table: {error}"),
        }
    }

    /// «Guardar como Excel…» of the selected table (S81-14).
    fn save_excel(&self, engine: &mut Gafira) {
        let Some((page, node)) = self.selected.single() else {
            return;
        };
        let Some(document) = engine.edit().document() else {
            return;
        };
        let Some((table, placement)) = table_of(&document, page, node) else {
            return;
        };
        let name = document
            .page(page)
            .and_then(|p| p.node(node))
            .and_then(|n| n.base.name.clone())
            .unwrap_or_default();
        let picked = rfd::FileDialog::new()
            .add_filter("Excel", &["xlsx"])
            .set_file_name(format!(
                "{}.xlsx",
                if name.is_empty() { "tabla" } else { &name }
            ))
            .save_file();
        let Some(path) = picked else { return };
        let sheet = gafira_io_xlsx::Sheet {
            name: &name,
            table: &table,
            size: (placement.w.abs(), placement.h.abs()),
            dpi: f64::from(document.dpi().get()),
        };
        let saved = gafira_io_xlsx::workbook(&[sheet])
            .and_then(|bytes| std::fs::write(path.with_extension("xlsx"), bytes));
        if let Err(error) = saved {
            eprintln!("excel: {error}");
        }
    }

    /// The QR codes asked for this frame (spec 080): a new one, a change in
    /// Propiedades, a logo, or «Convertir en formas».
    fn qr_codes(&mut self, engine: &mut Gafira, asked: &Asked) {
        if let Some((code, at)) = &asked.insert_qr {
            self.insert_qr(engine, (**code).clone(), *at);
        }
        if asked.qr_panel_logo
            && let Some(logo) = self.pick_logo(engine)
        {
            self.qr_panel.borrow_mut().draft.logo = Some(logo);
        }
        let selected = self.selected.single();
        if let (Some(code), Some((page, node))) = (&asked.qr, selected) {
            self.set_qr(engine, page, node, (**code).clone());
        }
        if asked.qr_logo
            && let Some((page, node)) = selected
            && let Some(document) = engine.edit().document()
            && let Some(mut code) = qr_of(&document, page, node)
            && let Some(logo) = self.pick_logo(engine)
        {
            code.logo = Some(logo);
            self.set_qr(engine, page, node, code);
        }
        // The changes end with the focus, or with the selection.
        if let Some((node, _)) = self.qr_gesture
            && (asked.qr_rest || asked.qr_to_shapes || selected.map(|s| s.1) != Some(node))
        {
            self.qr_gesture = None;
            if let Err(error) = engine.edit().end_gesture() {
                eprintln!("end gesture: {error}");
            }
        }
        if asked.qr_to_shapes
            && let Some((page, node)) = selected
        {
            self.qr_to_shapes(engine, page, node);
        }
    }

    /// A new code, centred where it was dropped or in the middle of the
    /// current page, selected; one undo step (S80-11).
    fn insert_qr(&mut self, engine: &mut Gafira, mut code: gafira_doc::QrNode, at: Option<Pos2>) {
        let Some(document) = engine.edit().document() else {
            return;
        };
        // A logo picked in another document is not in this one.
        code.logo = code.logo.filter(|l| document.asset(&l.asset).is_some());
        let dropped = at.and_then(|at| canvas::drop_point(&self.page_rects, self.zoom, at));
        let page = match dropped {
            Some((page, _)) => Some(page),
            None => self.current_page(&document).map(|p| p.id),
        };
        let Some(on) = page.and_then(|page| document.page(page)) else {
            return;
        };
        let Ok(size) = on.size.to_px(document.dpi()) else {
            return;
        };
        let spec = crate::qr::spec(
            code,
            (size.width(), size.height()),
            dropped.map(|(_, point)| point),
        );
        let next = NodeId::new(document.next_id());
        let command = InsertNodes::new(on.id, None, on.roots.len(), vec![spec]);
        match engine.edit().execute(Box::new(command)) {
            Ok(_) => {
                self.selected = Selection::one(on.id, next);
                self.active = Some(on.id);
                self.scope = None;
            }
            Err(error) => eprintln!("insert QR code: {error}"),
        }
    }

    /// Sets the code `node`, within the gesture of its fields.
    fn set_qr(
        &mut self,
        engine: &mut Gafira,
        page: PageId,
        node: NodeId,
        code: gafira_doc::QrNode,
    ) {
        if self.qr_gesture.is_none_or(|(of, _)| of != node) {
            self.qr_gesture = engine.edit().begin_gesture().ok().map(|g| (node, g));
        }
        let gesture = self.qr_gesture.map(|(_, gesture)| gesture);
        let command = SetProperty::new(page, vec![node], Property::Qr(Box::new(code)), gesture);
        if let Err(error) = engine.edit().execute(Box::new(command)) {
            eprintln!("QR code: {error}");
        }
    }

    /// Asks for an image and brings it into the document (S80-10).
    fn pick_logo(&mut self, engine: &mut Gafira) -> Option<gafira_doc::QrLogo> {
        let path = rfd::FileDialog::new()
            .add_filter("PNG, JPEG", &["png", "jpg", "jpeg"])
            .pick_file()?;
        let Ok(natural) = image::image_dimensions(&path) else {
            eprintln!("logo: {} is not a readable image", path.display());
            return None;
        };
        match engine.assets().import(&path) {
            Ok(asset) => {
                let hash = asset.as_hash();
                self.photos
                    .entry(hash)
                    .or_insert_with(|| decode(engine, hash).map(Arc::new));
                Some(gafira_doc::QrLogo {
                    asset,
                    natural,
                    share: 0.2,
                    clear_behind: true,
                })
            }
            Err(error) => {
                eprintln!("logo: {error}");
                None
            }
        }
    }

    /// «Convertir en formas» (S80-13): a group of shapes in the place of
    /// the code, selected; one undo step.
    fn qr_to_shapes(&mut self, engine: &mut Gafira, page: PageId, node: NodeId) {
        let Some(document) = engine.edit().document() else {
            return;
        };
        let Some(on) = document.page(page) else {
            return;
        };
        let Some(found) = on.node(node) else { return };
        let gafira_doc::NodeKind::Qr(code) = &found.kind else {
            return;
        };
        let base = &found.base;
        let Some(mut spec) = crate::qr::as_shapes(code, &base.placement) else {
            return;
        };
        spec.name.clone_from(&base.name);
        spec.opacity = base.opacity;
        spec.blend = base.blend;
        let Some((parent, list)) = group::siblings(on, &[node]) else {
            return;
        };
        let index = list.iter().position(|id| *id == node).unwrap_or(0);
        let next = NodeId::new(document.next_id());
        let commands: Vec<Box<dyn Command>> = vec![
            Box::new(InsertNodes::new(page, parent, index + 1, vec![spec])),
            Box::new(RemoveNodes::new(page, vec![node])),
        ];
        let command = Transaction::new(MessageKey("edit.set_property"), commands);
        match engine.edit().execute(Box::new(command)) {
            Ok(_) => self.selected = Selection::one(page, next),
            Err(error) => eprintln!("QR code to shapes: {error}"),
        }
    }

    /// «Poner foto…» (spec 066, S66-06): pick an image, import it through
    /// `AssetService` and make it the photo of the selected frames, in one
    /// undo step; it fills them («Rellenar») unless they fit otherwise.
    fn pick_frame_photo(&mut self, engine: &mut Gafira) {
        let Some(page) = self.selected.page() else {
            return;
        };
        let nodes = self.selected.nodes().to_vec();
        let picked = rfd::FileDialog::new()
            .add_filter("Imagen", &["png", "jpg", "jpeg"])
            .pick_file();
        let Some(path) = picked else { return };
        let Ok(natural) = image::image_dimensions(&path) else {
            eprintln!("frame photo: {} is not a readable image", path.display());
            return;
        };
        match engine.assets().import(&path) {
            Ok(asset) => {
                let content = gafira_doc::FrameContent { asset, natural };
                let command =
                    SetProperty::new(page, nodes, Property::FrameContent(Some(content)), None);
                if let Err(error) = engine.edit().execute(Box::new(command)) {
                    eprintln!("frame photo: {error}");
                }
            }
            Err(error) => eprintln!("frame photo: {error}"),
        }
    }

    fn import_image(&mut self, engine: &mut Gafira, _document: &Document) {
        // Images, and SVG or PDF outlines for custom frames (spec 068).
        let picked = rfd::FileDialog::new()
            .add_filter("Imagen, SVG, PDF", &["png", "jpg", "jpeg", "svg", "pdf"])
            .pick_files();
        if let Some(paths) = picked {
            self.place_images(engine, &paths, None);
        }
    }
}

/// What a drag of a group does to every node below it (spec 043): the
/// move, resize or turn that took its box from `origin` to `to`.
fn group_change(origin: &Placement, to: &Placement, hold: Option<Hold>) -> Change {
    match hold {
        Some(Hold::Resize(_)) => Change::Frame {
            from: *origin,
            to: *to,
        },
        Some(Hold::Rotate { .. }) => Change::Turn {
            center: gafira_geom::Point::new(origin.x + origin.w / 2.0, origin.y + origin.h / 2.0),
            degrees: to.rotation_deg - origin.rotation_deg,
        },
        // A line end never drags a group; a move keeps it where it is.
        None | Some(Hold::LineEnd(_)) => Change::Move {
            dx: to.x - origin.x,
            dy: to.y - origin.y,
        },
    }
}

/// A group of `page` turned by `turn` degrees and then flipped along
/// `flip`, each about the centre of its box then (spec 043): every node
/// below it, parents first.
fn group_reshaped(
    page: &Page,
    node: NodeId,
    turn: Option<f64>,
    flip: Option<Axis>,
) -> Vec<(NodeId, Placement)> {
    let centre = |page: &Page| {
        group::item_box(page, node)
            .map(|b| gafira_geom::Point::new(b.x + b.w / 2.0, b.y + b.h / 2.0))
    };
    let mut now = page.clone();
    let mut out = Vec::new();
    for change in [
        turn.and_then(|degrees| {
            Some(Change::Turn {
                center: centre(&now)?,
                degrees,
            })
        }),
        flip.and_then(|axis| {
            Some(Change::Flip {
                center: centre(&now)?,
                axis,
            })
        }),
    ]
    .into_iter()
    .flatten()
    {
        out = group::changed(&now, node, change);
        now = group::with_placements(&now, &out);
    }
    out
}

/// The bytes of `hash` from the store, decoded and scaled for the canvas.
/// The live size of a resize next to the pointer, outside the element
/// and inside `view` (spec 045): «320 × 180» and, in the secondary
/// colour, how much each side changed. `sizes` is where the element
/// started and where it is now.
fn size_label(
    painter: &egui::Painter,
    theme: &Theme,
    pointer: Pos2,
    outward: Vec2,
    view: Rect,
    sizes: (&Placement, &Placement),
) {
    let (from, now) = sizes;
    let font = theme.font("caption");
    let size = painter.layout_no_wrap(
        resize::size_text(now.w, now.h),
        font.clone(),
        theme.c("text"),
    );
    let delta = painter.layout_no_wrap(
        resize::delta_text(now.w - from.w, now.h - from.h),
        font,
        theme.c("text.secondary"),
    );
    let pad = Vec2::new(8.0, 4.0);
    let gap = 8.0;
    let inner = Vec2::new(
        size.size().x + gap + delta.size().x,
        size.size().y.max(delta.size().y),
    );
    let rect = resize::label_rect(pointer, outward, inner + pad * 2.0, view);
    painter.rect(
        rect,
        theme.radius_sm(),
        theme.c("surface.raised"),
        Stroke::new(1.0, theme.c("border")),
        egui::StrokeKind::Inside,
    );
    let at = rect.min + pad;
    let size_width = size.size().x;
    painter.galley(at, size, theme.c("text"));
    painter.galley(
        at + Vec2::new(size_width + gap, 0.0),
        delta,
        theme.c("text.secondary"),
    );
}

/// The cursor over a guide or a ruler: it moves across its line.
fn guide_cursor(axis: GuideAxis) -> egui::CursorIcon {
    match axis {
        GuideAxis::Vertical => egui::CursorIcon::ResizeHorizontal,
        GuideAxis::Horizontal => egui::CursorIcon::ResizeVertical,
    }
}

/// A line of the canvas state, in `color` over a halo of the canvas
/// background so it shows on any content (DIS-12).
fn halo_line(painter: &egui::Painter, theme: &Theme, points: [Pos2; 2], color: Color32) {
    painter.line_segment(points, Stroke::new(3.0, theme.c("canvas.selection.halo")));
    painter.line_segment(points, Stroke::new(1.0, color));
}

/// The pages, their boxes on screen and the zoom, as the canvas drew them.
type CanvasView<'a> = (&'a [Arc<Page>], &'a [(PageId, Rect)], f32);

impl Editor {
    /// The tool that draws or erases on the canvas, when Dibujo is open
    /// with a brush or the eraser in hand (spec 079).
    fn drawing_tool(&self) -> Option<crate::draw::Tool> {
        let open = self.rail.open && TABS[self.rail.tab].0 == "tab.drawing";
        let tool = self.draw.borrow().tool;
        (open && tool != crate::draw::Tool::Select).then_some(tool)
    }

    /// What the pointer does on the canvas in Dibujo (spec 079): a brush
    /// follows the hand and, let go, asks for the stroke as one element; the
    /// eraser gathers the strokes it goes over and, let go, asks to remove
    /// them. The stroke being drawn is painted live.
    fn draw_input(
        &self,
        ui: &egui::Ui,
        painter: &egui::Painter,
        response: &egui::Response,
        (pages, page_rects, zoom): CanvasView<'_>,
        (texts, tool): (&Texts, crate::draw::Tool),
        asked: &mut Asked,
    ) {
        use crate::draw::{self, Live, Tool};
        let mut state = self.draw.borrow_mut();
        let pointer = ui.input(|i| i.pointer.latest_pos());
        let press = ui.input(|i| i.pointer.press_origin());
        let on_page = |at: Pos2, page: Option<PageId>| {
            page_rects
                .iter()
                .find(|(id, rect)| page.map_or(rect.contains(at), |p| p == *id))
                .map(|(id, rect)| {
                    let p = (at - rect.min) / zoom;
                    (*id, *rect, [f64::from(p.x), f64::from(p.y)])
                })
        };
        // The tool in hand is the cursor (spec 079).
        if (response.hovered() || response.dragged())
            && let Some(at) = pointer
        {
            draw::paint_cursor(ui.ctx(), at, tool, &state);
        }
        match tool {
            Tool::Brush(index) => {
                let setting = state.settings[index];
                let brush = draw::BRUSHES[index];
                let begin = response.drag_started() || response.clicked();
                if begin
                    && state.live.is_none()
                    && let Some((page, _, point)) =
                        press.or(pointer).and_then(|at| on_page(at, None))
                {
                    state.live = Some(Live {
                        page,
                        points: vec![point],
                    });
                    asked.select = Some(Selection::default());
                }
                if (response.dragged() || response.clicked())
                    && let Some(at) = pointer
                    && let Some(live) = state.live.as_mut()
                    && let Some((_, _, point)) = on_page(at, Some(live.page))
                    && live.points.last() != Some(&point)
                {
                    live.points.push(point);
                }
                if let Some(live) = &state.live
                    && let Some((_, rect)) = page_rects.iter().find(|(id, _)| *id == live.page)
                {
                    let screen =
                        |p: draw::Point| rect.min + Vec2::new(p[0] as f32, p[1] as f32) * zoom;
                    let colour = setting.colour.gamma_multiply(setting.opacity);
                    let mesh = draw::mesh(
                        &live.points,
                        (f64::from(setting.size), brush.cap, &setting.style),
                        screen,
                        colour,
                    );
                    painter.add(mesh);
                }
                let done = response.drag_stopped() || response.clicked();
                if done && let Some(live) = state.live.take() {
                    let outline = draw::outline(
                        &live.points,
                        f64::from(setting.size),
                        brush.cap,
                        &setting.style,
                    );
                    if let Some(((x, y, w, h), path)) = draw::to_path(&outline) {
                        let mut shape = gafira_doc::ShapeNode::new(ShapeGeometry::Path(path));
                        shape.fill = Some(canvas::document_color(setting.colour));
                        shape.drawn_with = Some(brush.id.to_owned());
                        let spec = gafira_edit::NodeSpec {
                            name: Some(texts.t(&gafira_doc::msg::Message {
                                key: MessageKey(brush.key),
                                args: Vec::new(),
                            })),
                            opacity: setting.opacity.clamp(0.0, 1.0),
                            blend: brush.blend,
                            ..gafira_edit::NodeSpec::new(
                                gafira_doc::NodeKind::Shape(shape),
                                gafira_doc::Placement::new(x, y, w, h),
                            )
                        };
                        asked.draw_stroke = Some((live.page, spec));
                    }
                }
            }
            Tool::Eraser => {
                let reach = f64::from(8.0 / zoom);
                let erasing = response.drag_started() || response.dragged() || response.clicked();
                if erasing
                    && let Some(at) = pointer
                    && let Some((page_id, _, point)) =
                        on_page(at, state.erasing.as_ref().map(|e| e.0))
                    && let Some(page) = pages.iter().find(|p| p.id == page_id)
                {
                    let (_, gone) = state.erasing.get_or_insert_with(|| (page_id, Vec::new()));
                    for id in &page.roots {
                        let Some(node) = page.node(*id) else { continue };
                        let gafira_doc::NodeKind::Shape(shape) = &node.kind else {
                            continue;
                        };
                        let ShapeGeometry::Path(path) = &shape.geometry else {
                            continue;
                        };
                        if shape.drawn_with.is_none() || node.base.locked || gone.contains(id) {
                            continue;
                        }
                        let p = node.base.placement;
                        let to_page = p.to_transform();
                        let polygon = draw::flatten(path, |u| {
                            let q = to_page * kurbo::Point::new(u[0] * p.w, u[1] * p.h);
                            [q.x, q.y]
                        });
                        if draw::touches(&polygon, point, reach) {
                            gone.push(*id);
                        }
                    }
                }
                if (response.drag_stopped() || response.clicked())
                    && let Some(erased) = state.erasing.take()
                {
                    asked.erase = Some(erased);
                }
            }
            Tool::Select => {}
        }
    }
}

/// «Añadir página» under the last page (spec 077): a wide button that adds
/// a page at the end, and an arrow with a menu to add one or duplicate
/// `last`.
fn add_page_bar(
    ui: &mut egui::Ui,
    theme: &Theme,
    texts: &Texts,
    bar: Rect,
    last: PageId,
    asked: &mut Asked,
) {
    let arrow_w = 40.0;
    let main = Rect::from_min_max(bar.min, Pos2::new(bar.max.x - arrow_w, bar.max.y));
    let arrow = Rect::from_min_max(Pos2::new(bar.max.x - arrow_w, bar.min.y), bar.max);
    let main_r = ui.interact(main, ui.id().with("add_page.main"), egui::Sense::click());
    let arrow_r = ui.interact(arrow, ui.id().with("add_page.more"), egui::Sense::click());
    let painter = ui.painter();
    let radius = theme.radius_md();
    painter.rect_filled(bar, radius, theme.c("surface"));
    let hover = theme.c("secondary.bg");
    if main_r.hovered() {
        painter.rect_filled(
            main,
            egui::CornerRadius {
                ne: 0,
                se: 0,
                ..radius
            },
            hover,
        );
    }
    if arrow_r.hovered() {
        painter.rect_filled(
            arrow,
            egui::CornerRadius {
                nw: 0,
                sw: 0,
                ..radius
            },
            hover,
        );
    }
    painter.rect_stroke(
        bar,
        radius,
        Stroke::new(1.0, theme.c("border.strong")),
        egui::StrokeKind::Inside,
    );
    painter.line_segment(
        [
            Pos2::new(arrow.min.x, bar.min.y + 1.0),
            Pos2::new(arrow.min.x, bar.max.y - 1.0),
        ],
        Stroke::new(1.0, theme.c("border.strong")),
    );
    let text = texts.t(&gafira_doc::msg!("page.add"));
    let galley = painter.layout_no_wrap(text, theme.font("ui-label"), theme.c("text"));
    let icon = 14.0;
    let total = icon + 6.0 + galley.size().x;
    let start = main.center().x - total / 2.0;
    icons::paint(
        painter,
        Rect::from_center_size(
            Pos2::new(start + icon / 2.0, main.center().y),
            Vec2::splat(icon),
        ),
        Icon::ZoomIn,
        icons::Weight::Regular,
        theme.c("text"),
    );
    painter.galley(
        Pos2::new(start + icon + 6.0, main.center().y - galley.size().y / 2.0),
        galley,
        theme.c("text"),
    );
    icons::paint(
        painter,
        Rect::from_center_size(arrow.center(), Vec2::splat(14.0)),
        Icon::Expand,
        icons::Weight::Regular,
        theme.c("text"),
    );
    if main_r.hovered() || arrow_r.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    if main_r.clicked() {
        asked.add_page = true;
    }
    egui::Popup::menu(&arrow_r).show(|ui| {
        if ui.button(texts.t(&gafira_doc::msg!("page.add"))).clicked() {
            asked.add_page = true;
        }
        if ui
            .button(texts.t(&gafira_doc::msg!("page.duplicate")))
            .clicked()
        {
            asked.page_action = Some(PageAction::Duplicate(last));
        }
    });
}

/// The guides of `page_doc` on the page drawn at `page`: a vertical one
/// along the page, a horizontal one across the canvas row `row` (FD-03).
fn draw_guides(
    painter: &egui::Painter,
    theme: &Theme,
    page_doc: &Page,
    dpi: f64,
    (page, row): (Rect, Rect),
    zoom: f32,
    held: Option<usize>,
) {
    let color = theme.c("canvas.guide");
    for (index, guide) in page_doc.guides.0.iter().enumerate() {
        if held == Some(index) {
            continue;
        }
        let Some(px) = ruler::guide_px(guide, dpi) else {
            continue;
        };
        let at = px as f32 * zoom;
        let points = match guide.axis {
            GuideAxis::Vertical => [
                Pos2::new(page.min.x + at, page.min.y),
                Pos2::new(page.min.x + at, page.max.y),
            ],
            GuideAxis::Horizontal => [
                Pos2::new(row.min.x, page.min.y + at),
                Pos2::new(row.max.x, page.min.y + at),
            ],
        };
        if guide.locked {
            // A locked guide is a fold or a fixed line (spec 076): dashed.
            painter.add(egui::Shape::dashed_line(
                &points,
                Stroke::new(3.0, theme.c("canvas.selection.halo")),
                6.0,
                4.0,
            ));
            painter.add(egui::Shape::dashed_line(
                &points,
                Stroke::new(1.0, color),
                6.0,
                4.0,
            ));
        } else {
            halo_line(painter, theme, points, color);
        }
    }
}

/// The smart guides of the magnet (FD-06): a dashed line across the page
/// where each axis snapped.
fn draw_snap(painter: &egui::Painter, theme: &Theme, snap: &Snap, page: Rect, zoom: f32) {
    let stroke = Stroke::new(1.0, theme.c("canvas.guide"));
    let halo = Stroke::new(3.0, theme.c("canvas.selection.halo"));
    let mut lines = Vec::new();
    if let Some(x) = snap.x {
        let x = page.min.x + x as f32 * zoom;
        lines.push([Pos2::new(x, page.min.y), Pos2::new(x, page.max.y)]);
    }
    if let Some(y) = snap.y {
        let y = page.min.y + y as f32 * zoom;
        lines.push([Pos2::new(page.min.x, y), Pos2::new(page.max.x, y)]);
    }
    for line in lines {
        painter.line_segment(line, halo);
        painter.extend(egui::Shape::dashed_line(&line, stroke, 4.0, 3.0));
    }
}

/// A ruler along `axis` in `strip` (FD-03, spec 049): numbered ticks
/// across the whole ruler with their value beside them, minor ticks from
/// its inner edge (the middle one longer), and the selection's extent
/// `band` shaded. The left ruler turns its numbers to read bottom to top.
fn draw_ruler(
    painter: &egui::Painter,
    theme: &Theme,
    axis: GuideAxis,
    strip: Rect,
    (origin, zoom, (unit, px_per_unit)): (Rect, f32, (gafira_geom::Unit, f64)),
    band: Option<(f32, f32)>,
) {
    let scale = f64::from(zoom) * px_per_unit;
    let finest = ruler::finest(unit);
    painter.rect_filled(strip, 0.0, theme.c("surface"));
    let border = Stroke::new(1.0, theme.c("border"));
    let mut font = theme.font("caption");
    // Smaller than any text token: it only has to fit the 20 px ruler.
    font.size = 10.0;
    let color = theme.c("text.secondary");
    let size = ruler::RULER;
    let horizontal = axis == GuideAxis::Horizontal;
    // The corner belongs to neither ruler.
    let (from, to) = if horizontal {
        (strip.min.x + size, strip.max.x)
    } else {
        (strip.min.y + size, strip.max.y)
    };
    let along = |a: f32, b: f32| {
        if horizontal {
            Rect::from_min_max(Pos2::new(a, strip.min.y), Pos2::new(b, strip.max.y))
        } else {
            Rect::from_min_max(Pos2::new(strip.min.x, a), Pos2::new(strip.max.x, b))
        }
    };
    let painter = painter.with_clip_rect(along(from, to));
    if let Some((a, b)) = band {
        painter.rect_filled(along(a, b), 0.0, theme.c("selection.bg"));
    }
    let origin_at = if horizontal {
        origin.min.x
    } else {
        origin.min.y
    };
    // A tick across `length` of the ruler, from its inner edge.
    let tick = |at: f32, length: f32| {
        let points = if horizontal {
            [
                Pos2::new(at, strip.max.y - length),
                Pos2::new(at, strip.max.y),
            ]
        } else {
            [
                Pos2::new(strip.max.x - length, at),
                Pos2::new(strip.max.x, at),
            ]
        };
        painter.line_segment(points, border);
    };
    for (at, middle) in ruler::minor_ticks_in(origin_at, scale, (from, to), finest) {
        tick(at, if middle { size / 2.0 } else { size / 4.0 });
    }
    for (at, value) in ruler::ticks_in(origin_at, scale, (from, to), finest) {
        tick(at, size);
        let galley = painter.layout_no_wrap(ruler::tick_label(value), font.clone(), color);
        if horizontal {
            painter.galley(Pos2::new(at + 3.0, strip.min.y + 1.0), galley, color);
        } else {
            // Turned a quarter left about its corner: it runs up from just
            // above the tick, its height across the ruler.
            let text =
                egui::epaint::TextShape::new(Pos2::new(strip.min.x + 1.0, at - 3.0), galley, color)
                    .with_angle(-std::f32::consts::FRAC_PI_2);
            painter.add(text);
        }
    }
    let edge = if horizontal {
        [strip.left_bottom(), strip.right_bottom()]
    } else {
        [strip.right_top(), strip.right_bottom()]
    };
    painter.with_clip_rect(strip).line_segment(edge, border);
}

/// A 16 px icon of the bottom bar in `color`, not a button.
fn bar_icon(ui: &mut egui::Ui, theme: &Theme, icon: Icon, color: Color32) {
    let side = px(theme.tokens.size.icon.sm);
    let image = icons::image(ui.ctx(), icon, Weight::Regular, side, color);
    ui.add(image);
}

/// A thin upright line between groups of the bottom bar.
fn bar_separator(ui: &mut egui::Ui, theme: &Theme) {
    ui.add_space(8.0);
    let (rect, _) = ui.allocate_exact_size(Vec2::new(1.0, 20.0), egui::Sense::hover());
    ui.painter().vline(
        rect.center().x,
        rect.y_range(),
        Stroke::new(1.0, theme.c("border")),
    );
    ui.add_space(8.0);
}

/// The page a press on the canvas leaves active (S50-01): the page of
/// the element it picked, else the page under `at`, else none (the
/// workspace around the pages).
fn active_after(pages: &[(PageId, Rect)], at: Pos2, picked: Option<PageId>) -> Option<PageId> {
    picked.or_else(|| {
        pages
            .iter()
            .find(|(_, rect)| rect.contains(at))
            .map(|(page, _)| *page)
    })
}

/// The page the editor works on (S50-03): the selection's, else the active
/// one, else the first; a page that is gone does not count.
fn current_page_id(
    pages: &[PageId],
    selected: Option<PageId>,
    active: Option<PageId>,
) -> Option<PageId> {
    let known = |id: Option<PageId>| id.filter(|id| pages.contains(id));
    known(selected)
        .or_else(|| known(active))
        .or_else(|| pages.first().copied())
}

/// The key of the texture of a path shape: what it shows, its size and the
/// zoom, but not where it is, so moving it draws the same texture.
/// The preview of a file of Biblioteca and its size in px; `None` when it
/// could not be read (spec 068).
type Preview = Option<(egui::TextureHandle, (f64, f64))>;

/// Whether `path` names a vector file whose outline makes a frame (spec
/// 066, S66-10): SVG or PDF.
fn is_vector_path(path: &std::path::Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| matches!(e.to_ascii_lowercase().as_str(), "svg" | "pdf"))
}

/// Whether `path` names an SVG, which asks how to come in (spec 067).
fn is_svg_path(path: &std::path::Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("svg"))
}

/// The outline of the vector file at `path` (spec 066, S66-10).
fn vector_outline(path: &std::path::Path) -> Option<gafira_io_vector::Outline> {
    let bytes = std::fs::read(path).ok()?;
    let pdf = path
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("pdf"));
    let outline = if pdf {
        gafira_io_vector::from_pdf(&bytes)
    } else {
        gafira_io_vector::from_svg(&bytes)
    };
    outline
        .inspect_err(|error| eprintln!("frame from {}: {error}", path.display()))
        .ok()
}

/// Whether `path` names an image Gafira imports (spec 067): PNG or JPEG.
fn is_image_path(path: &std::path::Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| matches!(e.to_ascii_lowercase().as_str(), "png" | "jpg" | "jpeg"))
}

/// The image files a pasted text names, one per line, as paths or
/// `file://` URIs (what a file manager copies); empty when any line is not
/// an existing image file (spec 067).
fn image_paths(text: &str) -> Vec<std::path::PathBuf> {
    let lines: Vec<&str> = text
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#') && *l != "copy" && *l != "cut")
        .collect();
    let paths: Vec<std::path::PathBuf> = lines
        .iter()
        .filter_map(|line| {
            let raw = line.strip_prefix("file://").unwrap_or(line);
            let path = std::path::PathBuf::from(percent_decode(raw));
            (path.is_absolute()
                && (is_image_path(&path) || is_vector_path(&path))
                && path.is_file())
            .then_some(path)
        })
        .collect();
    if paths.len() == lines.len() {
        paths
    } else {
        Vec::new()
    }
}

/// `%20` and the like decoded, as a `file://` URI writes them.
fn percent_decode(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && let Some(hex) = text.get(i + 1..i + 3)
            && let Ok(byte) = u8::from_str_radix(hex, 16)
        {
            out.push(byte);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// The frame `node` of `page`, if it is one.
fn frame_of(document: &Document, page: PageId, node: NodeId) -> Option<gafira_doc::FrameNode> {
    match &document.page(page)?.node(node)?.kind {
        gafira_doc::NodeKind::Frame(frame) => Some(frame.clone()),
        _ => None,
    }
}

/// The id of the field of a cell.
fn cell_id(node: NodeId, cell: (usize, usize)) -> egui::Id {
    egui::Id::new(("cell", node, cell))
}

/// Whether the menu of a table is done with: something was picked, or it
/// was closed.
fn picked_or_closed(act: &Option<tables::Act>, menu: Option<tables::Menu>, open: bool) -> bool {
    act.is_some() || menu.is_some() || !open
}

/// Puts `table` in the box `placement` of the node, in one command.
fn set_table(
    engine: &mut Gafira,
    (page, node): (PageId, NodeId),
    table: gafira_doc::TableNode,
    placement: Placement,
    gesture: Option<GestureId>,
) -> bool {
    let commands: Vec<Box<dyn Command>> = vec![
        Box::new(SetProperty::new(
            page,
            vec![node],
            Property::Table(Box::new(table)),
            gesture,
        )),
        Box::new(SetProperty::new(
            page,
            vec![node],
            Property::Placement(placement),
            gesture,
        )),
    ];
    let command = Transaction::new(MessageKey("edit.set_property"), commands);
    match engine.edit().execute(Box::new(command)) {
        Ok(_) => true,
        Err(error) => {
            eprintln!("table: {error}");
            false
        }
    }
}

/// `placement` with `margin` px more on each side, about the same centre.
fn grown(placement: &Placement, margin: f64) -> Placement {
    Placement {
        x: placement.x - margin,
        y: placement.y - margin,
        w: placement.w + 2.0 * margin,
        h: placement.h + 2.0 * margin,
        ..*placement
    }
}

/// The looks of the words of a cell, as a text holds them.
fn span_of(style: &gafira_doc::CellStyle) -> gafira_doc::SpanStyle {
    gafira_doc::SpanStyle {
        font: style.font.clone(),
        size: style.size,
        color: style.color,
        bold: style.bold,
        italic: style.italic,
    }
}

/// The table `node` of `page` and its box, if it is one.
fn table_of(
    document: &Document,
    page: PageId,
    node: NodeId,
) -> Option<(gafira_doc::TableNode, Placement)> {
    let found = document.page(page)?.node(node)?;
    match &found.kind {
        gafira_doc::NodeKind::Table(table) => Some(((**table).clone(), found.base.placement)),
        _ => None,
    }
}

/// The QR code `node` of `page`, if it is one.
fn qr_of(document: &Document, page: PageId, node: NodeId) -> Option<gafira_doc::QrNode> {
    match &document.page(page)?.node(node)?.kind {
        gafira_doc::NodeKind::Qr(qr) => Some((**qr).clone()),
        _ => None,
    }
}

/// The size in px of the image `hash` of the store, read from its header.
fn natural_size(engine: &Gafira, hash: gafira_doc::ContentHash) -> Option<(u32, u32)> {
    let mut bytes = Vec::new();
    let mut source = engine.ports().store.open(hash).ok()?;
    std::io::Read::read_to_end(&mut source, &mut bytes).ok()?;
    image::ImageReader::new(std::io::Cursor::new(bytes))
        .with_guessed_format()
        .ok()?
        .into_dimensions()
        .ok()
}

/// The key of the texture of a whole photo (crop mode, S66-07).
fn photo_key(asset: gafira_doc::timeline::AssetRef) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    format!("photo {}", asset.hex()).hash(&mut hasher);
    hasher.finish()
}

/// The key of the texture of a frame's photo (spec 066): the frame, its box
/// and the zoom.
fn frame_key(shape: &Shape, zoom: f32) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    format!(
        "frame {:?}{}{}{}",
        shape.frame, shape.placement.w, shape.placement.h, zoom
    )
    .hash(&mut hasher);
    hasher.finish()
}

/// The key of the texture of a text drawn from its outline: its words,
/// styles, bend, spacing and gradient, its box, where it is (the cover
/// moves with it) and the zoom.
fn curved_key(text: &gafira_doc::TextNode, placement: &Placement, zoom: f32) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    format!("curved {text:?}{placement:?}{zoom}").hash(&mut hasher);
    hasher.finish()
}

/// The key of the texture of a light (spec 065): the light, its box, the
/// page it may spread over and the zoom.
fn light_key(shape: &Shape, page_px: (f64, f64), zoom: f32) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    format!(
        "light {:?}{:?}{:?}{}",
        shape.effect, shape.placement, page_px, zoom
    )
    .hash(&mut hasher);
    hasher.finish()
}

fn path_key(shape: &Shape, zoom: f32) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    format!(
        "{:?}{:?}{:?}{:?}{}{}{}{}",
        shape.geometry,
        shape.fill,
        shape.stroke,
        shape.gradient,
        shape.corner_radius,
        shape.placement.w,
        shape.placement.h,
        zoom
    )
    .hash(&mut hasher);
    hasher.finish()
}

/// What a move dropped wholly outside its page is deleted (S51-07,
/// D-93), in one undo step with the move: the drag is undone and one
/// command sets where it left everything and removes what is gone.
fn drop_outside(engine: &mut Gafira, drag: &Drag, moved: &[NodeId]) {
    let Some(now) = engine.edit().document() else {
        return;
    };
    let gone = transfer::left_behind(&now, drag.page, moved);
    let (Some(before), Some(after)) = (
        drag.start.as_ref().and_then(|d| d.page(drag.page)),
        now.page(drag.page),
    ) else {
        return;
    };
    let Some(command) = transfer::drop_command(before, after, &gone) else {
        return;
    };
    if let Err(error) = engine.edit().undo() {
        eprintln!("drop: {error}");
        return;
    }
    if let Err(error) = engine.edit().execute(command) {
        eprintln!("drop: {error}");
        if let Err(error) = engine.edit().redo() {
            eprintln!("drop: {error}");
        }
    }
}

/// An empty page of `size` at `at` of `document`, born with the guides of
/// the first page (D-99), in one undo step.
fn new_page(document: &Document, at: usize, size: gafira_doc::PageSize) -> Box<dyn Command> {
    let insert = Box::new(InsertPage::new(at, String::new(), size));
    let guides = document
        .pages()
        .first()
        .map(|page| page.guides.clone())
        .filter(|guides| !guides.0.is_empty());
    match guides {
        None => insert,
        Some(guides) => {
            // The new page takes the next id of the document.
            let id = PageId::new(document.next_id());
            Box::new(Transaction::new(
                MessageKey("edit.insert_page"),
                vec![
                    insert,
                    Box::new(SetProperty::page(id, Property::Guides(guides), None)),
                ],
            ))
        }
    }
}

/// The style of a new text of `preset`: its size, the display or the UI
/// face, in the ink of the brand (spec 027).
fn text_style(theme: &Theme, preset: canvas::TextPreset) -> gafira_doc::SpanStyle {
    let (size, display) = preset.size_and_display();
    let tokens = &theme.tokens;
    gafira_doc::SpanStyle {
        font: if display {
            tokens.typography.display.clone()
        } else {
            tokens.typography.ui.clone()
        },
        size,
        color: canvas::document_color(crate::style::color32(tokens.brand.ink)),
        bold: false,
        italic: false,
    }
}

/// The height of every row of the Fuente panel, so only the rows in view
/// are laid out (spec 062).
const FONT_ROW_HEIGHT: f32 = 32.0;

/// One row of the Fuente panel.
#[derive(Clone, Debug, PartialEq)]
enum PanelRow {
    /// The title of a section, by catalog key.
    Title(&'static str),
    /// A family, with the faces it has and whether it is of the system.
    Family {
        family: String,
        faces: Vec<(bool, bool)>,
        system: bool,
    },
    /// A face of an open family.
    Face {
        family: String,
        bold: bool,
        italic: bool,
        system: bool,
    },
    /// A line of text under a section, by catalog key.
    Note(&'static str),
}

/// The rows of the Fuente panel for `search` (S62-06): the families of the
/// document, then those of Gafira, then those of the system (or a note while
/// they are being looked for), each section with its title; the faces of the
/// open families after them. A family of the system with the name of one of
/// Gafira is left out.
fn font_rows(
    search: &str,
    document: &[String],
    open: &BTreeSet<String>,
    system: &system_fonts::SystemFonts,
) -> Vec<PanelRow> {
    let needle = search.trim().to_lowercase();
    let shown = |family: &str| needle.is_empty() || family.to_lowercase().contains(&needle);
    let library = gafira_theme::library_families();
    let faces = system.faces();
    let faces_of = |family: &str| -> (Vec<(bool, bool)>, bool) {
        let gafira: Vec<(bool, bool)> = gafira_theme::library()
            .iter()
            .filter(|f| f.family == family)
            .map(|f| (f.bold, f.italic))
            .collect();
        if gafira.is_empty() {
            let mut own: Vec<(bool, bool)> = faces
                .iter()
                .filter(|f| f.family == family)
                .map(|f| (f.bold, f.italic))
                .collect();
            own.sort();
            (own, true)
        } else {
            (gafira, false)
        }
    };
    let mut rows = Vec::new();
    let section = |rows: &mut Vec<PanelRow>, families: Vec<String>| {
        for family in families {
            let (family_faces, is_system) = faces_of(&family);
            let is_open = open.contains(&family);
            rows.push(PanelRow::Family {
                family: family.clone(),
                faces: family_faces.clone(),
                system: is_system,
            });
            if is_open && family_faces.len() > 1 {
                for (bold, italic) in family_faces {
                    rows.push(PanelRow::Face {
                        family: family.clone(),
                        bold,
                        italic,
                        system: is_system,
                    });
                }
            }
        }
    };
    let used: Vec<String> = document.iter().filter(|f| shown(f)).cloned().collect();
    if !used.is_empty() {
        rows.push(PanelRow::Title("fonts.document"));
        section(&mut rows, used);
    }
    rows.push(PanelRow::Title("fonts.gafira"));
    let gafira: Vec<String> = library
        .iter()
        .filter(|f| shown(f))
        .map(|f| (*f).to_owned())
        .collect();
    if gafira.is_empty() {
        rows.push(PanelRow::Note("fonts.none"));
    }
    section(&mut rows, gafira);
    rows.push(PanelRow::Title("fonts.system"));
    if !system.ready() {
        rows.push(PanelRow::Note("fonts.looking"));
        return rows;
    }
    let own: Vec<String> = system
        .families(&library)
        .into_iter()
        .filter(|f| shown(f))
        .collect();
    if own.is_empty() {
        rows.push(PanelRow::Note("fonts.none"));
    }
    section(&mut rows, own);
    rows
}

/// What a row of the Fuente panel shows: its indent, its chevron (`Some`
/// open or closed when it has faces to show), its name in its face, a
/// sample after it and the tick of the current font.
struct FontRow {
    indent: f32,
    chevron: Option<bool>,
    name: String,
    face: egui::FontFamily,
    sample: bool,
    ticked: bool,
}

/// Where a row of the Fuente panel was clicked.
enum RowClick {
    Chevron,
    Row,
}

/// A row of the Fuente panel across the whole width: its background lights
/// up under the pointer, which turns into a hand; a click on the chevron
/// opens or closes it, anywhere else picks the font.
fn font_row(ui: &mut egui::Ui, theme: &Theme, row: FontRow) -> Option<RowClick> {
    let height = 32.0;
    let (rect, response) = ui.allocate_exact_size(
        Vec2::new(ui.available_width(), height),
        egui::Sense::click(),
    );
    let response = response.on_hover_cursor(egui::CursorIcon::PointingHand);
    let painter = ui.painter();
    if response.hovered() {
        painter.rect_filled(rect, theme.radius_md(), theme.c("secondary.bg"));
    }
    let mut x = rect.min.x + 8.0 + row.indent;
    let chevron = Rect::from_min_size(Pos2::new(x, rect.center().y - 8.0), Vec2::splat(16.0));
    if let Some(open) = row.chevron {
        paint_chevron(painter, chevron, open, theme.c("text"));
    }
    x += 24.0;
    let name = painter.layout_no_wrap(
        row.name,
        egui::FontId::new(15.0, row.face.clone()),
        theme.c("text"),
    );
    let width = name.size().x;
    painter.galley(
        Pos2::new(x, rect.center().y - name.size().y / 2.0),
        name,
        theme.c("text"),
    );
    if row.sample {
        let sample = painter.layout_no_wrap(
            "AaBbCc".to_owned(),
            egui::FontId::new(13.0, row.face),
            theme.c("text.secondary"),
        );
        let at = Pos2::new(x + width + 8.0, rect.center().y - sample.size().y / 2.0);
        painter.galley(at, sample, theme.c("text.secondary"));
    }
    if row.ticked {
        let tick = Rect::from_center_size(
            Pos2::new(rect.max.x - 16.0, rect.center().y),
            Vec2::splat(14.0),
        );
        paint_tick(painter, tick, theme.c("text"));
    }
    if !response.clicked() {
        return None;
    }
    let on_chevron = row.chevron.is_some()
        && response
            .interact_pointer_pos()
            .is_some_and(|at| chevron.expand(6.0).contains(at));
    Some(if on_chevron {
        RowClick::Chevron
    } else {
        RowClick::Row
    })
}

/// A thin chevron in `rect`: pointing right when closed, down when open.
fn paint_chevron(painter: &egui::Painter, rect: Rect, open: bool, color: Color32) {
    let at = |x: f32, y: f32| rect.min + Vec2::new(x, y) * rect.width();
    let points = if open {
        vec![at(0.25, 0.4), at(0.5, 0.65), at(0.75, 0.4)]
    } else {
        vec![at(0.4, 0.25), at(0.65, 0.5), at(0.4, 0.75)]
    };
    painter.add(egui::Shape::line(points, Stroke::new(1.5, color)));
}

/// A tick in `rect` drawn with lines.
fn paint_tick(painter: &egui::Painter, rect: Rect, color: Color32) {
    let at = |x: f32, y: f32| rect.min + Vec2::new(x, y) * rect.width();
    painter.add(egui::Shape::line(
        vec![at(0.15, 0.55), at(0.4, 0.8), at(0.85, 0.25)],
        Stroke::new(2.0, color),
    ));
}

/// The button of the Inspector that opens the Fuente panel (spec 062),
/// named after the font of `style`.
fn font_button(
    ui: &mut egui::Ui,
    theme: &Theme,
    texts: &Texts,
    style: Option<&gafira_doc::SpanStyle>,
    asked: &mut Asked,
) {
    let Some(style) = style else { return };
    // One row, centred down, so the label and the button line up.
    let row = Vec2::new(ui.available_width(), 28.0);
    ui.allocate_ui_with_layout(row, Layout::left_to_right(Align::Center), |ui| {
        ui.label(texts.t(&gafira_doc::msg!("fonts.title")));
        let face = text_family(ui.ctx(), theme, style);
        let button = ui.button(RichText::new(&style.font).font(egui::FontId::new(14.0, face)));
        if button.clicked() {
            asked.font_panel = Some(true);
        }
    });
    ui.add_space(8.0);
}

/// The font families the texts of `document` use, each once, in order.
fn document_fonts(document: &Document) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for page in document.pages() {
        for node in page.nodes.values() {
            let content = match &node.kind {
                gafira_doc::NodeKind::Text(text) => Some(&text.content),
                gafira_doc::NodeKind::Shape(shape) => shape.text.as_ref().map(|t| &t.content),
                _ => None,
            };
            for span in content.into_iter().flat_map(gafira_doc::RichText::spans) {
                if !out.contains(&span.style.font) {
                    out.push(span.style.font.clone());
                }
            }
        }
    }
    out
}

/// The number «n de N» shows (S49-05): the place of `current` among
/// `pages`, from 1; 1 when it is not one of them, 0 with no page.
fn page_number(pages: &[PageId], current: Option<PageId>) -> usize {
    if pages.is_empty() {
        return 0;
    }
    current
        .and_then(|id| pages.iter().position(|page| *page == id))
        .map_or(1, |index| index + 1)
}

/// `placements` of `page` as one undo step: their `SetProperty` commands
/// share a gesture of their own, so the step never merges with the next
/// action (specs 041, 042). Nothing to place is no step.
fn place_in_one_step(
    engine: &mut Gafira,
    page: PageId,
    placements: Vec<(NodeId, Placement)>,
    what: &str,
) {
    if placements.is_empty() {
        return;
    }
    let gesture = engine.edit().begin_gesture().ok();
    let commands = placements
        .into_iter()
        .map(|(node, to)| {
            let command = SetProperty::new(page, vec![node], Property::Placement(to), gesture);
            Box::new(command) as Box<dyn Command>
        })
        .collect();
    if let Some(command) = batch(commands)
        && let Err(error) = engine.edit().execute(command)
    {
        eprintln!("{what}: {error}");
    }
    if gesture.is_some()
        && let Err(error) = engine.edit().end_gesture()
    {
        eprintln!("end gesture: {error}");
    }
}

fn decode(engine: &Gafira, hash: gafira_doc::ContentHash) -> Option<egui::ColorImage> {
    let mut bytes = Vec::new();
    let mut source = engine.ports().store.open(hash).ok()?;
    std::io::Read::read_to_end(&mut source, &mut bytes).ok()?;
    let mut decoded = image::load_from_memory(&bytes).ok()?;
    if decoded.width() > 2048 || decoded.height() > 2048 {
        decoded = decoded.thumbnail(2048, 2048);
    }
    let rgba = decoded.to_rgba8();
    let size = [rgba.width() as usize, rgba.height() as usize];
    Some(egui::ColorImage::from_rgba_unmultiplied(
        size,
        rgba.as_raw(),
    ))
}

/// The Inspector fields of a shape's outline (colour, «Sin contorno»,
/// width) and, for a rectangle, its corner radius (spec 038). Each change
/// is one command, so one undo step.
/// «Relleno»: a colour picker showing `shape`'s fill and «Sin relleno».
/// A shape, not an image, also has «Color sólido» and «Degradado» (spec
/// 063).
fn fill_field(ui: &mut egui::Ui, texts: &Texts, shape: &Shape, asked: &mut Asked) {
    if shape.image.is_none() {
        ui.horizontal(|ui| {
            let solid = shape.gradient.is_none();
            let label = |key| plain(texts, key);
            if ui
                .selectable_label(solid, label("inspector.solid"))
                .clicked()
                && !solid
            {
                asked.gradient = Some(None);
            }
            if ui
                .selectable_label(!solid, label("inspector.gradient"))
                .clicked()
                && solid
            {
                asked.gradient = Some(Some(crate::gradients::start(shape.fill)));
            }
        });
        if let Some(gradient) = &shape.gradient {
            gradient_fields(ui, texts, gradient, asked);
            return;
        }
    }
    ui.horizontal(|ui| {
        ui.label(texts.t(&gafira_doc::msg!("inspector.fill")));
        if let Some(color) = color_button(ui, shape.fill) {
            asked.fill = Some(Some(color));
        }
        if shape.fill.is_some()
            && ui
                .small_button(texts.t(&gafira_doc::msg!("inspector.no_fill")))
                .clicked()
        {
            asked.fill = Some(None);
        }
    });
}

/// A colour picker with its transparency (spec 063) showing `color`; the
/// colour picked this frame, if any.
fn color_button(
    ui: &mut egui::Ui,
    color: Option<gafira_color::Color>,
) -> Option<gafira_color::Color> {
    let mut rgba = canvas::picker_rgba(color);
    ui.color_edit_button_srgba_unmultiplied(&mut rgba)
        .changed()
        .then(|| canvas::picked_color(color, rgba))
}

/// «Degradado» (spec 063): a picker per colour (a right click removes one
/// while more than two are left), «+» for one more, and the styles, each
/// drawn as a swatch of the gradient in it.
fn gradient_fields(
    ui: &mut egui::Ui,
    texts: &Texts,
    gradient: &gafira_doc::Gradient,
    asked: &mut Asked,
) {
    let label = |key| plain(texts, key);
    ui.label(label("inspector.gradient_colors"));
    ui.horizontal_wrapped(|ui| {
        for index in 0..gradient.stops.len() {
            if let Some(changed) = gradient_stop_button(ui, texts, gradient, index) {
                asked.gradient = Some(changed);
            }
        }
        if let Some(more) = crate::gradients::with_stop_added(gradient)
            && ui
                .button("+")
                .on_hover_text(label("inspector.gradient_add"))
                .clicked()
        {
            let added = more.stops.len() - 1;
            ui.ctx()
                .data_mut(|d| d.insert_temp(gradient_pending_id(), added));
            asked.gradient = Some(Some(more));
        }
    });
    ui.label(label("inspector.gradient_style"));
    ui.horizontal_wrapped(|ui| {
        for (kind, key) in crate::gradients::STYLES
            .into_iter()
            .zip(GRADIENT_STYLE_KEYS)
        {
            let styled = gafira_doc::Gradient {
                kind,
                stops: gradient.stops.clone(),
            };
            let (rect, response) =
                ui.allocate_exact_size(Vec2::new(36.0, 28.0), egui::Sense::click());
            ui.painter().add(gradient_swatch(&styled, rect));
            let stroke = if kind == gradient.kind {
                egui::Stroke::new(2.0, ui.visuals().selection.stroke.color)
            } else {
                ui.visuals().widgets.noninteractive.bg_stroke
            };
            ui.painter()
                .rect_stroke(rect, 4.0, stroke, egui::StrokeKind::Outside);
            let response = response
                .on_hover_text(label(key))
                .on_hover_cursor(egui::CursorIcon::PointingHand);
            if response.clicked() && kind != gradient.kind {
                asked.gradient = Some(Some(styled));
            }
        }
    });
}

/// The button of colour `index` of `gradient` (spec 063): a click opens
/// its picker, and while more than two colours are left a right click
/// offers «Quitar este color». What it changed this frame: the gradient
/// with the colour picked, or without the colour (`None` never happens
/// for a gradient of two or more).
fn gradient_stop_button(
    ui: &mut egui::Ui,
    texts: &Texts,
    gradient: &gafira_doc::Gradient,
    index: usize,
) -> Option<Option<gafira_doc::Gradient>> {
    let stop = gradient.stops.get(index)?;
    let mut changed = None;
    // Each colour keeps its own picker id, so adding or removing one does
    // not move another's picker.
    ui.push_id(("gradient_stop", index), |ui| {
        if ui
            .ctx()
            .data(|d| d.get_temp::<usize>(gradient_pending_id()))
            == Some(index)
        {
            egui::Popup::open_id(ui.ctx(), ui.auto_id_with("popup"));
            ui.ctx()
                .data_mut(|d| d.remove::<usize>(gradient_pending_id()));
        }
        let mut rgba = canvas::picker_rgba(Some(stop.color));
        let response = ui.color_edit_button_srgba_unmultiplied(&mut rgba);
        if response.changed() {
            changed = Some(Some(crate::gradients::with_color(
                gradient,
                index,
                canvas::picked_color(Some(stop.color), rgba),
            )));
        }
        if gradient.stops.len() > gafira_doc::Gradient::MIN_STOPS {
            // Its own id: egui's default for a menu is the picker's, and a
            // left click on the button would close the picker it opens.
            egui::Popup::context_menu(&response)
                .id(response.id.with("remove_colour"))
                .show(|ui| {
                    if ui
                        .button(plain(texts, "inspector.gradient_remove"))
                        .clicked()
                    {
                        changed = Some(crate::gradients::with_stop_removed(gradient, index));
                        ui.close();
                    }
                });
        }
    });
    changed
}

/// The colour «+» added, whose picker opens when it is drawn (spec 063).
fn gradient_pending_id() -> egui::Id {
    egui::Id::new("gafira.gradient.open_stop")
}

/// The catalog keys of the styles of `gradients::STYLES`, in order.
const GRADIENT_STYLE_KEYS: [&str; 7] = [
    "inspector.gradient_left_right",
    "inspector.gradient_right_left",
    "inspector.gradient_top_bottom",
    "inspector.gradient_bottom_top",
    "inspector.gradient_diagonal_down",
    "inspector.gradient_diagonal_up",
    "inspector.gradient_radial",
];

/// `gradient` painted over `rect`: a grid of 12 × 12 cells whose corners
/// take the gradient's colour there, blended across each cell.
fn gradient_swatch(gradient: &gafira_doc::Gradient, rect: Rect) -> egui::Shape {
    const CELLS: u32 = 12;
    let mut mesh = egui::Mesh::default();
    let (w, h) = (f64::from(rect.width()), f64::from(rect.height()));
    for j in 0..=CELLS {
        for i in 0..=CELLS {
            let (u, v) = (
                f64::from(i) / f64::from(CELLS),
                f64::from(j) / f64::from(CELLS),
            );
            let color = canvas::screen_color(gafira_color::Color::Srgb(gradient.color_at(
                u * w,
                v * h,
                w,
                h,
            )));
            mesh.colored_vertex(rect.min + Vec2::new((u * w) as f32, (v * h) as f32), color);
        }
    }
    let row = CELLS + 1;
    for j in 0..CELLS {
        for i in 0..CELLS {
            let a = j * row + i;
            mesh.add_triangle(a, a + 1, a + row);
            mesh.add_triangle(a + 1, a + row + 1, a + row);
        }
    }
    egui::Shape::mesh(mesh)
}

/// «Opacidad»: a slider showing `shape`'s opacity in percent.
fn opacity_field(ui: &mut egui::Ui, texts: &Texts, shape: &Shape, asked: &mut Asked) {
    ui.horizontal(|ui| {
        ui.label(texts.t(&gafira_doc::msg!("inspector.opacity")));
        let mut percent = shape.opacity * 100.0;
        // The unit comes from the catalog, like every visible text (ARQ-97).
        let slider = egui::Slider::new(&mut percent, 0.0..=100.0)
            .max_decimals(0)
            .custom_formatter(|value, _| {
                texts.t(&gafira_doc::msg!(
                    "format.percent",
                    value = Text(format!("{value:.0}"))
                ))
            });
        if ui.add(slider).changed() {
            asked.opacity = Some(percent / 100.0);
        }
    });
}

/// The Inspector of a multi-selection (spec 040): how many elements, then
/// the fields they all have, showing the primary's value and set on all of
/// them in one step: the fill when every one is a shape, and the opacity.
/// Position, size, outline, radius and text stay with a single selection.
fn many_fields(
    ui: &mut egui::Ui,
    theme: &Theme,
    texts: &Texts,
    shapes: &[Shape],
    can: CanArrange,
    asked: &mut Asked,
) {
    let Some(primary) = shapes.last() else { return };
    ui.label(
        RichText::new(texts.t(&gafira_doc::msg!(
            "inspector.count",
            count = Count(shapes.len() as u64)
        )))
        .font(theme.font("ui-label"))
        .color(theme.c("text")),
    );
    ui.add_space(16.0);
    turn_buttons(ui, theme, texts, asked);
    ui.add_space(16.0);
    arrange_buttons(ui, theme, texts, can, asked);
    ui.add_space(16.0);
    if shapes.iter().all(|shape| {
        shape.text.is_none()
            && shape.image.is_none()
            && shape.qr.is_none()
            && shape.table.is_none()
            && !shape.group
    }) {
        fill_field(ui, texts, primary, asked);
        ui.add_space(8.0);
    }
    opacity_field(ui, texts, primary, asked);
}

/// `value` degrees as the catalog writes them («45°»).
fn degrees_text(texts: &Texts, value: f64) -> String {
    texts.t(&gafira_doc::msg!(
        "unit.deg",
        value = Text(format!("{value:.1}"))
    ))
}

/// Girar 90° a la izquierda y a la derecha, Voltear horizontalmente y
/// verticalmente: icon buttons for the whole selection (spec 041).
fn turn_buttons(ui: &mut egui::Ui, theme: &Theme, texts: &Texts, asked: &mut Asked) {
    ui.horizontal(|ui| {
        for (icon, key, turn) in [
            (
                Icon::RotateLeft,
                gafira_doc::msg!("editor.rotate_left"),
                -90.0,
            ),
            (
                Icon::RotateRight,
                gafira_doc::msg!("editor.rotate_right"),
                90.0,
            ),
        ] {
            if IconButton::new(icon, texts.t(&key))
                .small()
                .show(ui, theme)
                .clicked()
            {
                asked.turn = Some(turn);
            }
        }
        for (icon, key, axis, keys) in [
            (
                Icon::FlipHorizontal,
                gafira_doc::msg!("editor.flip_horizontal"),
                Axis::Horizontal,
                FLIP_H,
            ),
            (
                Icon::FlipVertical,
                gafira_doc::msg!("editor.flip_vertical"),
                Axis::Vertical,
                FLIP_V,
            ),
        ] {
            let name = texts.t(&key);
            let tooltip = with_keys(ui.ctx(), texts, name.clone(), keys);
            if IconButton::new(icon, name)
                .small()
                .tooltip(tooltip)
                .show(ui, theme)
                .clicked()
            {
                asked.flip = Some(axis);
            }
        }
    });
}

/// The name of an arrange action and, when it has one, its shortcut.
fn arrange_tooltip(
    ctx: &egui::Context,
    texts: &Texts,
    key: &'static str,
    keys: Option<KeyboardShortcut>,
) -> String {
    let name = plain(texts, key);
    match keys {
        Some(keys) => with_keys(ctx, texts, name, keys),
        None => name,
    }
}

/// The tooltip of a Distribute action: while it is off, why.
fn distribute_tooltip(texts: &Texts, key: &'static str, enabled: bool) -> String {
    let name = plain(texts, key);
    if enabled {
        name
    } else {
        texts.t(&gafira_doc::msg!(
            "arrange.needs_three",
            action = Text(name)
        ))
    }
}

/// «Alinear» (six actions and the two of Distribute) and «Orden» (four
/// actions) in the Posición tab (03 §2.4, §7.3 wireframe; spec 042).
fn arrange_buttons(
    ui: &mut egui::Ui,
    theme: &Theme,
    texts: &Texts,
    can: CanArrange,
    asked: &mut Asked,
) {
    let heading = |ui: &mut egui::Ui, key| {
        ui.label(
            RichText::new(plain(texts, key))
                .font(theme.font("ui-label"))
                .color(theme.c("text")),
        );
    };
    heading(ui, "arrange.align");
    ui.horizontal_wrapped(|ui| {
        for (action, key, icon, keys) in ALIGNS {
            let tooltip = arrange_tooltip(ui.ctx(), texts, key, keys);
            let button = ui.add_enabled_ui(can.align, |ui| {
                IconButton::new(icon, plain(texts, key))
                    .small()
                    .tooltip(tooltip)
                    .show(ui, theme)
            });
            if button.inner.clicked() {
                asked.align = Some(action);
            }
        }
        for (axis, key, icon) in DISTRIBUTES {
            let tooltip = distribute_tooltip(texts, key, can.distribute);
            let button = ui.add_enabled_ui(can.distribute, |ui| {
                IconButton::new(icon, plain(texts, key))
                    .small()
                    .tooltip(tooltip)
                    .show(ui, theme)
            });
            if button.inner.clicked() {
                asked.distribute = Some(axis);
            }
        }
    });
    ui.add_space(8.0);
    heading(ui, "arrange.order");
    ui.horizontal(|ui| {
        for (index, (order, key, icon, keys)) in ORDERS.into_iter().enumerate() {
            let tooltip = arrange_tooltip(ui.ctx(), texts, key, Some(keys));
            let button = ui.add_enabled_ui(can.order[index], |ui| {
                IconButton::new(icon, plain(texts, key))
                    .small()
                    .tooltip(tooltip)
                    .show(ui, theme)
            });
            if button.inner.clicked() {
                asked.order = Some(order);
            }
        }
    });
}

/// Edición → Organizar: Align, Distribute and the layer order, with their
/// shortcuts; an action that cannot act now is off (FD-12).
fn arrange_menu(ui: &mut egui::Ui, texts: &Texts, can: CanArrange, asked: &mut Asked) {
    ui.menu_button(plain(texts, "arrange.menu"), |ui| {
        ui.set_min_width(220.0);
        for (action, key, _, keys) in ALIGNS {
            let mut item = egui::Button::new(plain(texts, key));
            if let Some(keys) = keys {
                item = item.shortcut_text(keys_text(ui.ctx(), &keys));
            }
            if ui.add_enabled(can.align, item).clicked() {
                asked.align = Some(action);
                ui.close();
            }
        }
        ui.separator();
        for (axis, key, _) in DISTRIBUTES {
            let item = ui
                .add_enabled(can.distribute, egui::Button::new(plain(texts, key)))
                .on_disabled_hover_text(distribute_tooltip(texts, key, false));
            if item.clicked() {
                asked.distribute = Some(axis);
                ui.close();
            }
        }
        ui.separator();
        for (index, (order, key, _, keys)) in ORDERS.into_iter().enumerate() {
            let item =
                egui::Button::new(plain(texts, key)).shortcut_text(keys_text(ui.ctx(), &keys));
            if ui.add_enabled(can.order[index], item).clicked() {
                asked.order = Some(order);
                ui.close();
            }
        }
    });
}

/// «Luz»: none, «Brillo» or «Destello de lente» (S65-06). Any shape turns
/// into a light with it; switching kind keeps the settings of the light,
/// «Recortar» (S66-07): the Inspector while a frame is cropped: close,
/// zoom of the photo, «Restablecer» and how to move it.
fn crop_fields(
    ui: &mut egui::Ui,
    theme: &Theme,
    texts: &Texts,
    frame: &gafira_doc::FrameNode,
    asked: &mut Asked,
) {
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(plain(texts, "crop.title"))
                .font(theme.font("ui-label"))
                .color(theme.c("text")),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui
                .button("×")
                .on_hover_text(plain(texts, "crop.done"))
                .clicked()
            {
                asked.end_crop = true;
            }
        });
    });
    ui.add_space(16.0);
    ui.label(
        RichText::new(plain(texts, "crop.zoom"))
            .small()
            .color(theme.c("text.secondary")),
    );
    ui.spacing_mut().slider_width = (ui.available_width() - 64.0).max(80.0);
    let mut zoom = frame.content_scale * 100.0;
    if ui
        .add(egui::Slider::new(&mut zoom, 100.0..=400.0).suffix(" %"))
        .changed()
    {
        asked.crop_set = Some((
            frame.content_offset,
            zoom / 100.0,
            frame.content_rotation_deg,
        ));
    }
    ui.add_space(12.0);
    ui.label(
        RichText::new(plain(texts, "crop.rotate"))
            .small()
            .color(theme.c("text.secondary")),
    );
    let mut degrees = frame.content_rotation_deg;
    if ui
        .add(egui::Slider::new(&mut degrees, -180.0..=180.0).suffix("°"))
        .changed()
    {
        asked.crop_set = Some((frame.content_offset, frame.content_scale, degrees));
    }
    ui.add_space(12.0);
    if ui.button(plain(texts, "crop.reset")).clicked() {
        asked.crop_set = Some(((0.0, 0.0), 1.0, 0.0));
    }
    ui.add_space(12.0);
    ui.label(
        RichText::new(plain(texts, "crop.hint"))
            .small()
            .color(theme.c("text.secondary")),
    );
    ui.add_space(12.0);
    if ui.button(plain(texts, "crop.done")).clicked() {
        asked.end_crop = true;
    }
}

/// A swatch of «Fondo de la página» (spec 069): 40 px, its look painted by
/// `paint`, ringed under the pointer; click to pick it.
fn swatch(
    ui: &mut egui::Ui,
    theme: &Theme,
    paint: impl FnOnce(&egui::Painter, Rect),
) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(40.0), egui::Sense::click());
    let painter = ui.painter();
    paint(painter, rect);
    let ring = if response.hovered() {
        Stroke::new(2.0, theme.c("primary.bg"))
    } else {
        Stroke::new(1.0, theme.c("border"))
    };
    painter.rect_stroke(rect, 6.0, ring, egui::StrokeKind::Inside);
    response
}

/// A linear gradient from `a` to `b` at `angle_deg` over `rect`, as a mesh
/// whose corners take the colour of where they fall along the direction.
fn paint_gradient_swatch(
    painter: &egui::Painter,
    rect: Rect,
    a: Color32,
    b: Color32,
    angle_deg: f64,
) {
    let (dy, dx) = angle_deg.to_radians().sin_cos();
    let corners = [
        rect.left_top(),
        rect.right_top(),
        rect.right_bottom(),
        rect.left_bottom(),
    ];
    let along = |p: Pos2| f64::from(p.x - rect.left()) * dx + f64::from(p.y - rect.top()) * dy;
    let values = corners.map(along);
    let (lo, hi) = values
        .iter()
        .fold((f64::MAX, f64::MIN), |(lo, hi), v| (lo.min(*v), hi.max(*v)));
    let mut mesh = egui::Mesh::default();
    for (corner, value) in corners.into_iter().zip(values) {
        let t = if hi > lo {
            ((value - lo) / (hi - lo)) as f32
        } else {
            0.0
        };
        mesh.colored_vertex(corner, a.lerp_to_gamma(b, t));
    }
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(0, 2, 3);
    painter.add(mesh);
}

/// Where a card of a side panel was let go: the page and the point on it.
type Dropped = (PageId, (f64, f64));

/// The locked element under `at` (D-110): the top one there, on the page
/// under the pointer, as [`layers::locked_pick`] chooses it.
fn locked_under(
    pages: &[Arc<Page>],
    page_rects: &[(PageId, Rect)],
    zoom: f32,
    at: Pos2,
) -> Option<(PageId, NodeId)> {
    let (id, rect) = page_rects.iter().find(|(_, rect)| rect.contains(at))?;
    let page = pages.iter().find(|p| p.id == *id)?;
    let locked: Vec<Shape> = canvas::shapes(page)
        .into_iter()
        .filter(|shape| shape.locked)
        .map(|mut shape| {
            shape.locked = false;
            shape
        })
        .collect();
    let leaf = canvas::hit(&locked, *rect, zoom, at)?;
    Some((*id, layers::locked_pick(page, leaf)?))
}

/// The bend of «Arco arriba» and, negated, «Arco abajo» (FD-44).
const CURVE_PRESET: f64 = 45.0;

/// The title of a section of a side panel.
fn section_title(ui: &mut egui::Ui, theme: &Theme, texts: &Texts, key: &'static str) {
    ui.label(
        RichText::new(plain(texts, key))
            .font(theme.font("ui-label"))
            .color(theme.c("text")),
    );
    ui.add_space(8.0);
}

/// The rounded card of a side panel at `rect`, lighter under the pointer.
fn card_frame(ui: &egui::Ui, theme: &Theme, rect: Rect, hovered: bool) {
    let fill = if hovered {
        theme.c("surface.sunken")
    } else {
        theme.c("surface")
    };
    ui.painter().rect(
        rect,
        theme.radius_md(),
        fill,
        Stroke::new(1.0, theme.c("border")),
        egui::StrokeKind::Inside,
    );
}

/// «Posición y tamaño» of the selected element.
fn position_fields(
    ui: &mut egui::Ui,
    theme: &Theme,
    texts: &Texts,
    shape: &Shape,
    asked: &mut Asked,
) {
    ui.label(
        RichText::new(plain(texts, "inspector.position_size"))
            .font(theme.font("body-sm"))
            .color(theme.c("text.secondary")),
    );
    ui.add_space(4.0);
    let mut placement = shape.placement;
    let mut changed = false;
    egui::Grid::new("inspector.position")
        .num_columns(2)
        .spacing([12.0, 8.0])
        .show(ui, |ui| {
            for (key, value, min) in [
                (gafira_doc::msg!("inspector.x"), &mut placement.x, f64::MIN),
                (gafira_doc::msg!("inspector.y"), &mut placement.y, f64::MIN),
                (gafira_doc::msg!("inspector.width"), &mut placement.w, 1.0),
                (gafira_doc::msg!("inspector.height"), &mut placement.h, 1.0),
            ] {
                ui.label(texts.t(&key));
                changed |= ui
                    .add(
                        egui::DragValue::new(value)
                            .speed(1.0)
                            .range(min..=f64::MAX)
                            .max_decimals(1),
                    )
                    .changed();
                ui.end_row();
            }
            // «Giro», in degrees within one turn (spec 041). A
            // group has no turn of its own: its box is upright
            // around its children (spec 043).
            if shape.group {
                return;
            }
            ui.label(texts.t(&gafira_doc::msg!("inspector.rotation")));
            let mut degrees = rotation::normalized(placement.rotation_deg);
            let field = egui::DragValue::new(&mut degrees)
                .speed(1.0)
                .max_decimals(1)
                .custom_formatter(|value, _| degrees_text(texts, value));
            if ui.add(field).changed() {
                placement.rotation_deg = rotation::normalized(degrees);
                changed = true;
            }
            ui.end_row();
        });
    if changed {
        asked.place = Some(placement);
    }
}

/// The title of «Propiedades» for `shape`: what kind of element it is.
fn kind_title(shape: &Shape) -> &'static str {
    if shape.table.is_some() {
        "layers.table"
    } else if shape.qr.is_some() {
        "layers.qr"
    } else if shape.frame.is_some() {
        "inspector.kind_frame"
    } else if shape.effect.is_some() {
        "inspector.kind_light"
    } else if shape.group {
        "inspector.kind_group"
    } else if shape.line.is_some() {
        "inspector.kind_line"
    } else if shape.text.is_some() {
        "inspector.kind_text"
    } else if shape.image.is_some() {
        "inspector.kind_image"
    } else {
        "inspector.kind_shape"
    }
}

/// «Colores» of a group (spec 067): a swatch for each colour of its shapes;
/// changing one changes it in all of them.
fn group_colors(
    ui: &mut egui::Ui,
    texts: &Texts,
    page: &gafira_doc::Page,
    group: NodeId,
    asked: &mut Asked,
) {
    let colors = crate::recolor::colors(page, group);
    if colors.is_empty() {
        return;
    }
    ui.label(plain(texts, "inspector.colors"));
    ui.horizontal_wrapped(|ui| {
        for old in colors {
            if let Some(new) = color_button(ui, Some(old)) {
                asked.recolor = Some((group, old, new));
            }
        }
    });
    ui.add_space(8.0);
}

/// The photo of a frame and how it sits (spec 066): «Poner foto…» or
/// «Cambiar foto…», «Quitar foto» and «Encaje».
fn frame_fields(
    ui: &mut egui::Ui,
    texts: &Texts,
    frame: &gafira_doc::FrameNode,
    asked: &mut Asked,
) {
    use gafira_doc::Fit;
    ui.label(plain(texts, "frame.photo"));
    ui.horizontal_wrapped(|ui| {
        let pick = if frame.content.is_some() {
            "frame.replace"
        } else {
            "frame.put"
        };
        if ui.button(plain(texts, pick)).clicked() {
            asked.pick_frame_photo = true;
        }
        if frame.content.is_some() && ui.button(plain(texts, "frame.remove")).clicked() {
            asked.frame_content = Some(None);
        }
    });
    if frame.content.is_none() {
        ui.label(
            egui::RichText::new(plain(texts, "frame.empty"))
                .small()
                .color(ui.visuals().weak_text_color()),
        );
    }
    ui.add_space(8.0);
    ui.label(plain(texts, "frame.fit"));
    let name = |fit: Fit| {
        plain(
            texts,
            match fit {
                Fit::Fill => "frame.fill",
                Fit::Fit => "frame.whole",
                Fit::Stretch => "frame.stretch",
                Fit::None => "frame.original",
            },
        )
    };
    let mut fit = frame.fit;
    egui::ComboBox::from_id_salt("inspector.frame_fit")
        .width(ui.available_width())
        .selected_text(name(fit))
        .show_ui(ui, |ui| {
            for option in [Fit::Fill, Fit::Fit, Fit::Stretch, Fit::None] {
                ui.selectable_value(&mut fit, option, name(option));
            }
        });
    if fit != frame.fit {
        asked.frame_fit = Some(fit);
    }
    ui.add_space(8.0);
}

/// «Luz»: none or one of the lights (S65-06), as a list the width of the
/// Inspector. Any shape turns into a light with it; switching kind keeps the
/// settings of the light, and «Ninguna» takes it off.
fn light_kind_row(
    ui: &mut egui::Ui,
    texts: &Texts,
    effect: Option<&gafira_doc::ShapeEffect>,
    asked: &mut Asked,
) {
    use elements::LightItem;
    let current = effect.map(LightItem::of);
    let name = |kind: Option<LightItem>| match kind {
        Some(kind) => plain(texts, kind.key()),
        None => plain(texts, "effect.none"),
    };
    ui.label(plain(texts, "effect.light"));
    let mut chosen = current;
    egui::ComboBox::from_id_salt("inspector.light")
        .width(ui.available_width())
        .selected_text(name(current))
        .show_ui(ui, |ui| {
            ui.selectable_value(&mut chosen, None, name(None));
            for kind in LightItem::ALL {
                ui.selectable_value(&mut chosen, Some(kind), name(Some(kind)));
            }
        });
    if chosen != current {
        asked.effect = Some(chosen.map(|kind| match effect {
            Some(effect) => kind.with(*effect.light()),
            None => kind.effect(),
        }));
    }
}

/// The settings of a light (spec 065, S65-05), one under the other: the
/// name of each setting over its slider, the slider the width of the
/// Inspector, so nothing is cut or crowded.
fn effect_fields(
    ui: &mut egui::Ui,
    texts: &Texts,
    effect: &gafira_doc::ShapeEffect,
    asked: &mut Asked,
) {
    use gafira_doc::{Light, ShapeEffect};
    light_kind_row(ui, texts, Some(effect), asked);
    ui.add_space(12.0);
    // Room for the value box after the slider.
    ui.spacing_mut().slider_width = (ui.available_width() - 64.0).max(80.0);
    let setting =
        |ui: &mut egui::Ui, key: &'static str, add: &mut dyn FnMut(&mut egui::Ui) -> bool| {
            ui.label(
                egui::RichText::new(plain(texts, key))
                    .small()
                    .color(ui.visuals().weak_text_color()),
            );
            let changed = add(ui);
            ui.add_space(8.0);
            changed
        };
    let mut next: Light = *effect.light();
    let mut changed = false;
    let (rays, width, colour) = match effect {
        ShapeEffect::Streak(_) => (None, "effect.thickness", "effect.color"),
        ShapeEffect::Bokeh(_) => (Some("effect.count"), "effect.size", "effect.color"),
        ShapeEffect::SunRays(_) => (Some("effect.rays"), "effect.ray_width", "effect.color"),
        ShapeEffect::Sparkle(_) | ShapeEffect::LensFlare(_) => {
            (Some("effect.rays"), "effect.ray_width", "effect.halo")
        }
    };
    if let Some(rays) = rays {
        changed |= setting(ui, rays, &mut |ui| {
            ui.add(egui::Slider::new(
                &mut next.rays,
                Light::MIN_RAYS..=Light::MAX_RAYS,
            ))
            .changed()
        });
    }
    let mut thick = next.ray_width * 100.0;
    changed |= setting(ui, width, &mut |ui| {
        ui.add(egui::Slider::new(&mut thick, 0.0..=100.0).suffix(" %"))
            .changed()
    });
    next.ray_width = thick / 100.0;
    let mut intensity = next.intensity * 100.0;
    changed |= setting(ui, "effect.intensity", &mut |ui| {
        ui.add(egui::Slider::new(&mut intensity, 0.0..=100.0).suffix(" %"))
            .changed()
    });
    next.intensity = intensity / 100.0;
    changed |= setting(ui, "effect.angle", &mut |ui| {
        ui.add(egui::Slider::new(&mut next.angle_deg, 0.0..=360.0).suffix("°"))
            .changed()
    });
    changed |= setting(ui, colour, &mut |ui| {
        ui.horizontal(|ui| {
            let mut changed = false;
            if let Some(color) = color_button(ui, next.halo) {
                next.halo = Some(color);
                changed = true;
            }
            if next.halo.is_some() && ui.small_button(plain(texts, "effect.no_halo")).clicked() {
                next.halo = None;
                changed = true;
            }
            changed
        })
        .inner
    });
    if changed {
        asked.effect = Some(Some(effect.with_light(next)));
    }
    ui.add_space(4.0);
    if ui.button(plain(texts, "effect.remove")).clicked() {
        asked.effect = Some(None);
    }
    ui.add_space(12.0);
}

fn outline_fields(ui: &mut egui::Ui, texts: &Texts, shape: &Shape, asked: &mut Asked) {
    let px = |value: f64| {
        texts.t(&gafira_doc::msg!(
            "unit.px",
            value = Text(format!("{value:.1}"))
        ))
    };
    ui.horizontal(|ui| {
        ui.label(texts.t(&gafira_doc::msg!("inspector.outline")));
        if let Some(color) = color_button(ui, shape.stroke.map(|stroke| stroke.color)) {
            let width = shape
                .stroke
                .map_or(canvas::NEW_OUTLINE_WIDTH, |stroke| stroke.width);
            asked.stroke = Some(Some(gafira_doc::ShapeStroke::new(color, width)));
        }
        if shape.stroke.is_some()
            && ui
                .small_button(texts.t(&gafira_doc::msg!("inspector.no_outline")))
                .clicked()
        {
            asked.stroke = Some(None);
        }
    });
    if let Some(stroke) = shape.stroke {
        ui.horizontal(|ui| {
            ui.label(texts.t(&gafira_doc::msg!("inspector.outline_width")));
            let mut width = stroke.width;
            let field = egui::DragValue::new(&mut width)
                .speed(0.5)
                .range(0.0..=gafira_doc::ShapeStroke::MAX_WIDTH)
                .custom_formatter(|value, _| px(value));
            if ui.add(field).changed() {
                asked.stroke = Some(Some(gafira_doc::ShapeStroke::new(stroke.color, width)));
            }
        });
    }
    if shape.geometry == ShapeGeometry::Rect {
        ui.horizontal(|ui| {
            ui.label(texts.t(&gafira_doc::msg!("inspector.corner_radius")));
            let half = shape.placement.w.min(shape.placement.h).max(0.0) / 2.0;
            let mut radius = shape.corner_radius.min(half);
            let field = egui::DragValue::new(&mut radius)
                .speed(0.5)
                .range(0.0..=half)
                .custom_formatter(|value, _| px(value));
            if ui.add(field).changed() {
                asked.corner_radius = Some(radius);
            }
        });
    }
}

/// The alignment of the text inside a shape (S55-05): across and down,
/// shown once the shape has text.
fn shape_text_fields(
    ui: &mut egui::Ui,
    theme: &Theme,
    texts: &Texts,
    shape: &Shape,
    asked: &mut Asked,
) {
    let Some(text) = &shape.inner else {
        return;
    };
    use gafira_doc::{TextAlign, VerticalAlign};
    ui.label(texts.t(&gafira_doc::msg!("inspector.shape_text")));
    ui.horizontal(|ui| {
        for (icon, key, align) in [
            (Icon::AlignLeft, "inspector.text_left", TextAlign::Left),
            (
                Icon::AlignCenter,
                "inspector.text_center",
                TextAlign::Center,
            ),
            (Icon::AlignRight, "inspector.text_right", TextAlign::Right),
        ] {
            let button = IconButton::new(icon, plain(texts, key))
                .active(text.align == align)
                .small()
                .show(ui, theme);
            if button.clicked() {
                asked.shape_align = Some((Some(align), None));
            }
        }
        ui.add_space(8.0);
        for (icon, key, valign) in [
            (Icon::AlignTop, "inspector.text_top", VerticalAlign::Top),
            (
                Icon::AlignMiddle,
                "inspector.text_middle",
                VerticalAlign::Middle,
            ),
            (
                Icon::AlignBottom,
                "inspector.text_bottom",
                VerticalAlign::Bottom,
            ),
        ] {
            let button = IconButton::new(icon, plain(texts, key))
                .active(text.valign == valign)
                .small()
                .show(ui, theme);
            if button.clicked() {
                asked.shape_align = Some((None, Some(valign)));
            }
        }
    });
}

/// The Inspector fields of a text element (spec 070): size, bold and
/// italic, alignment, letter spacing, line height and its colour, from the
/// picker, the colours of the design, the solid presets or the gradient
/// presets. One style per element for now (spec 027).
fn text_fields(
    ui: &mut egui::Ui,
    theme: &Theme,
    texts: &Texts,
    (text, design): (&gafira_doc::TextNode, &DesignColors),
    asked: &mut Asked,
) {
    use gafira_doc::TextAlign;
    let Some(style) = text.content.first_style().cloned() else {
        return;
    };
    let words = text.content.plain_text();
    let restyle = |asked: &mut Asked, style: gafira_doc::SpanStyle| {
        asked.text = Some(gafira_doc::RichText::plain(&words, &style));
    };
    // Size, bold and italic.
    ui.horizontal(|ui| {
        ui.label(plain(texts, "inspector.font_size"));
        let mut size = style.size;
        if ui
            .add(
                egui::DragValue::new(&mut size)
                    .speed(1.0)
                    .range(1.0..=gafira_doc::TextNode::MAX_SIZE),
            )
            .changed()
        {
            restyle(
                asked,
                gafira_doc::SpanStyle {
                    size,
                    ..style.clone()
                },
            );
        }
        ui.add_space(8.0);
        if format_tile(
            ui,
            theme,
            style.bold,
            plain(texts, "inspector.bold"),
            |p, r, c| {
                // A heavy «B»: the letter twice, a pixel apart.
                for dx in [0.0, 1.0] {
                    p.text(
                        r.center() + Vec2::new(dx - 0.5, 0.0),
                        Align2::CENTER_CENTER,
                        "B",
                        egui::FontId::proportional(15.0),
                        c,
                    );
                }
            },
        )
        .clicked()
        {
            restyle(
                asked,
                gafira_doc::SpanStyle {
                    bold: !style.bold,
                    ..style.clone()
                },
            );
        }
        let italic = text_family(
            ui.ctx(),
            theme,
            &gafira_doc::SpanStyle {
                font: "Inter".to_owned(),
                italic: true,
                ..style.clone()
            },
        );
        if format_tile(
            ui,
            theme,
            style.italic,
            plain(texts, "inspector.italic"),
            |p, r, c| {
                p.text(
                    r.center(),
                    Align2::CENTER_CENTER,
                    "I",
                    egui::FontId::new(15.0, italic),
                    c,
                );
            },
        )
        .clicked()
        {
            restyle(
                asked,
                gafira_doc::SpanStyle {
                    italic: !style.italic,
                    ..style.clone()
                },
            );
        }
    });
    ui.add_space(6.0);
    // Alignment: four lines each, set as the text sets them.
    ui.horizontal(|ui| {
        for (align, key) in [
            (TextAlign::Left, "inspector.text_left"),
            (TextAlign::Center, "inspector.text_center"),
            (TextAlign::Right, "inspector.text_right"),
            (TextAlign::Justify, "inspector.text_justify"),
        ] {
            let clicked = format_tile(
                ui,
                theme,
                text.align == align,
                plain(texts, key),
                |p, r, c| {
                    for (row, length) in [1.0_f32, 0.6, 1.0, 0.7].into_iter().enumerate() {
                        let length = if align == TextAlign::Justify {
                            1.0
                        } else {
                            length
                        };
                        let w = 16.0 * length;
                        let y = r.top() + 10.0 + row as f32 * 4.0;
                        let x = match align {
                            TextAlign::Left | TextAlign::Justify => r.center().x - 8.0,
                            TextAlign::Center => r.center().x - w / 2.0,
                            TextAlign::Right => r.center().x + 8.0 - w,
                        };
                        p.line_segment([Pos2::new(x, y), Pos2::new(x + w, y)], Stroke::new(1.5, c));
                    }
                },
            )
            .clicked();
            if clicked {
                asked.text_align = Some(align);
            }
        }
    });
    ui.add_space(6.0);
    // Letter spacing and line height (FD-41).
    egui::Grid::new("inspector.text_spacing")
        .num_columns(2)
        .spacing([12.0, 8.0])
        .show(ui, |ui| {
            let mut letters = text.letter_spacing;
            let mut lines = text.line_height.unwrap_or(1.0);
            let (lo, hi) = (
                *gafira_doc::TextNode::LETTER_SPACING.start(),
                *gafira_doc::TextNode::LETTER_SPACING.end(),
            );
            ui.label(plain(texts, "inspector.letter_spacing"));
            let a = ui.add(egui::Slider::new(&mut letters, lo..=hi).step_by(1.0));
            ui.end_row();
            let (lo, hi) = (
                *gafira_doc::TextNode::LINE_HEIGHT.start(),
                *gafira_doc::TextNode::LINE_HEIGHT.end(),
            );
            ui.label(plain(texts, "inspector.line_height"));
            let b = ui.add(egui::Slider::new(&mut lines, lo..=hi).step_by(0.05));
            ui.end_row();
            if a.changed() || b.changed() {
                let lines = (b.changed() || text.line_height.is_some()).then_some(lines);
                asked.text_spacing = Some((letters, lines));
            }
        });
    // «Curva»: from a cup (-100) through straight (0) to an arch (100).
    ui.add_space(6.0);
    let mut curve = text.curve;
    ui.horizontal(|ui| {
        ui.label(plain(texts, "inspector.curve"));
        let max = gafira_doc::TextNode::MAX_CURVE;
        if ui
            .add(egui::Slider::new(&mut curve, -max..=max).step_by(1.0))
            .changed()
        {
            asked.text_curve = Some(curve);
        }
    });
    ui.add_space(12.0);
    text_color_fields(ui, theme, texts, (text, &style, design), asked);
}

/// The colours a design already uses (spec 070): the solid colours and the
/// gradients of its shapes and texts, each once, in the order they appear.
#[derive(Clone, Debug, Default)]
struct DesignColors {
    solid: Vec<gafira_color::Color>,
    gradients: Vec<gafira_doc::Gradient>,
}

/// The colours of the design on `page`, at most twelve of each.
fn design_colors(page: &gafira_doc::Page) -> DesignColors {
    let mut out = DesignColors::default();
    let solid = |color: gafira_color::Color, out: &mut DesignColors| {
        if out.solid.len() < 12 && !out.solid.contains(&color) {
            out.solid.push(color);
        }
    };
    let gradient = |g: &gafira_doc::Gradient, out: &mut DesignColors| {
        if out.gradients.len() < 12 && !out.gradients.contains(g) {
            out.gradients.push(g.clone());
        }
    };
    let mut ids: Vec<&NodeId> = page.nodes.keys().collect();
    ids.sort();
    for id in ids {
        let Some(node) = page.nodes.get(id) else {
            continue;
        };
        match &node.kind {
            gafira_doc::NodeKind::Shape(shape) => {
                match &shape.gradient {
                    Some(g) => gradient(g, &mut out),
                    None => {
                        if let Some(fill) = shape.fill {
                            solid(fill, &mut out);
                        }
                    }
                }
                if let Some(text) = &shape.text
                    && let Some(style) = text.content.first_style()
                {
                    solid(style.color, &mut out);
                }
            }
            gafira_doc::NodeKind::Text(text) => match &text.gradient {
                Some(g) => gradient(g, &mut out),
                None => {
                    if let Some(style) = text.content.first_style() {
                        solid(style.color, &mut out);
                    }
                }
            },
            _ => {}
        }
    }
    out
}

/// «Color del texto» (spec 070): the picker, then the colours of the
/// design, the solid presets and the gradient presets, as round swatches;
/// the one in use is ringed.
fn text_color_fields(
    ui: &mut egui::Ui,
    theme: &Theme,
    texts: &Texts,
    (text, style, design): (&gafira_doc::TextNode, &gafira_doc::SpanStyle, &DesignColors),
    asked: &mut Asked,
) {
    let section = |ui: &mut egui::Ui, key: &'static str| {
        ui.add_space(8.0);
        ui.label(
            RichText::new(plain(texts, key))
                .font(theme.font("body-sm"))
                .color(theme.c("text.secondary")),
        );
        ui.add_space(4.0);
    };
    ui.label(
        RichText::new(plain(texts, "inspector.text_color"))
            .font(theme.font("ui-label"))
            .color(theme.c("text")),
    );
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        if let Some(picked) = color_button(ui, Some(style.color)) {
            asked.text_fill = Some(TextFill::Solid(picked));
        }
        ui.label(plain(texts, "inspector.custom_color"));
    });
    let in_use = |fill: &TextFill| match fill {
        TextFill::Solid(c) => text.gradient.is_none() && *c == style.color,
        TextFill::Gradient(g) => text.gradient.as_ref() == Some(g),
    };
    let row = |ui: &mut egui::Ui, fills: Vec<TextFill>, asked: &mut Asked| {
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = Vec2::splat(6.0);
            for fill in fills {
                if dot(ui, theme, in_use(&fill), &fill).clicked() {
                    asked.text_fill = Some(fill);
                }
            }
        });
    };
    if !design.solid.is_empty() || !design.gradients.is_empty() {
        section(ui, "inspector.design_colors");
        let fills = design
            .solid
            .iter()
            .copied()
            .map(TextFill::Solid)
            .chain(design.gradients.iter().cloned().map(TextFill::Gradient))
            .collect();
        row(ui, fills, asked);
    }
    section(ui, "inspector.solid_presets");
    row(
        ui,
        crate::colors::TEXT_PALETTE
            .iter()
            .map(|code| TextFill::Solid(crate::colors::hex(*code)))
            .collect(),
        asked,
    );
    section(ui, "inspector.gradient_presets");
    row(
        ui,
        crate::colors::TEXT_GRADIENTS
            .iter()
            .map(|(a, b, angle)| TextFill::Gradient(crate::colors::gradient(*a, *b, *angle)))
            .collect(),
        asked,
    );
}

/// A colour for the letters of a text: one colour, or a gradient over the
/// box (spec 070).
#[derive(Clone, Debug, PartialEq)]
enum TextFill {
    Solid(gafira_color::Color),
    Gradient(gafira_doc::Gradient),
}

/// A round swatch of `fill`, ringed when it is the one in use; a hand
/// over it.
fn dot(ui: &mut egui::Ui, theme: &Theme, on: bool, fill: &TextFill) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(28.0), egui::Sense::click());
    let painter = ui.painter();
    let inner = rect.shrink(2.0);
    match fill {
        TextFill::Solid(color) => {
            painter.circle_filled(
                inner.center(),
                inner.width() / 2.0,
                canvas::screen_color(*color),
            );
        }
        TextFill::Gradient(gradient) => {
            // The gradient swatch, cut round by the page colour around it.
            painter.add(gradient_swatch(gradient, inner));
            let ring = inner.width() / 2.0 + 4.0;
            painter.circle_stroke(
                inner.center(),
                ring - 2.0,
                Stroke::new(8.0, ui.visuals().panel_fill),
            );
        }
    }
    let ring = if on {
        Stroke::new(2.0, theme.c("primary.bg"))
    } else {
        Stroke::new(1.0, theme.c("border"))
    };
    painter.circle_stroke(rect.center(), rect.width() / 2.0 - 1.0, ring);
    if response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    response
}

/// A square button of the text formats, drawn by `paint` in the text colour
/// (or the brand colour when `on`), with `hint` on hover and a hand.
fn format_tile(
    ui: &mut egui::Ui,
    theme: &Theme,
    on: bool,
    hint: String,
    paint: impl FnOnce(&egui::Painter, Rect, Color32),
) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(30.0), egui::Sense::click());
    let painter = ui.painter();
    if on || response.hovered() {
        painter.rect_filled(rect, 6.0, theme.c("surface.sunken"));
    }
    let color = if on {
        theme.c("primary.bg")
    } else {
        theme.c("text")
    };
    paint(painter, rect, color);
    if response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    response.on_hover_text(hint)
}

/// Lay out a text element in its upright screen box `at`: one galley per
/// paragraph, wrapped to the box width, aligned, stacked downwards, each
/// with where it goes and its colour. The canvas draws with egui; the
/// document renderer (`gafira-text`) takes over when the canvas renders
/// through `gafira-render`.
fn lay_text(
    painter: &egui::Painter,
    theme: &Theme,
    at: Rect,
    zoom: f32,
    opacity: f32,
    text: &gafira_doc::TextNode,
) -> Vec<(Pos2, Arc<egui::Galley>, Color32)> {
    let mut laid = Vec::new();
    let mut y = at.min.y;
    for paragraph in &text.content.paragraphs {
        let Some(style) = paragraph.spans.first().map(|span| &span.style) else {
            continue;
        };
        let words: String = paragraph
            .spans
            .iter()
            .map(|span| span.text.as_str())
            .collect();
        let family = text_family(painter.ctx(), theme, style);
        let size = (style.size as f32 * zoom).max(1.0);
        let color = canvas::screen_color(style.color).gamma_multiply(opacity.clamp(0.0, 1.0));
        let galley = painter.layout(
            words,
            egui::FontId::new(size, family),
            color,
            at.width().max(1.0),
        );
        let x = match text.align {
            gafira_doc::TextAlign::Center => at.center().x - galley.size().x / 2.0,
            gafira_doc::TextAlign::Right => at.max.x - galley.size().x,
            gafira_doc::TextAlign::Left | gafira_doc::TextAlign::Justify => at.min.x,
        };
        let height = galley.size().y.max(size);
        laid.push((Pos2::new(x, y), galley, color));
        y += height;
    }
    laid
}

/// The text of a shape laid out in the upright box `at` (spec 055): in
/// the box less its margin, each line aligned across, the whole block
/// placed down by its vertical alignment.
fn lay_inner(
    painter: &egui::Painter,
    theme: &Theme,
    at: Rect,
    zoom: f32,
    opacity: f32,
    text: &gafira_doc::ShapeText,
) -> Vec<(Pos2, Arc<egui::Galley>, Color32)> {
    let inner = canvas::text_box(at);
    let mut laid = Vec::new();
    let mut y = 0.0;
    for paragraph in &text.content.paragraphs {
        let Some(style) = paragraph.spans.first().map(|span| &span.style) else {
            continue;
        };
        let words: String = paragraph
            .spans
            .iter()
            .map(|span| span.text.as_str())
            .collect();
        let size = (style.size as f32 * zoom).max(1.0);
        let color = canvas::screen_color(style.color).gamma_multiply(opacity.clamp(0.0, 1.0));
        let mut job = egui::text::LayoutJob::simple(
            words,
            egui::FontId::new(size, text_family(painter.ctx(), theme, style)),
            color,
            inner.width().max(1.0),
        );
        job.halign = match text.align {
            gafira_doc::TextAlign::Center => Align::Center,
            gafira_doc::TextAlign::Right => Align::RIGHT,
            gafira_doc::TextAlign::Left | gafira_doc::TextAlign::Justify => Align::LEFT,
        };
        let galley = painter.layout_job(job);
        let x = match text.align {
            gafira_doc::TextAlign::Center => inner.center().x,
            gafira_doc::TextAlign::Right => inner.max.x,
            gafira_doc::TextAlign::Left | gafira_doc::TextAlign::Justify => inner.min.x,
        };
        let height = galley.size().y.max(size);
        laid.push((Pos2::new(x, y), galley, color));
        y += height;
    }
    let top = canvas::text_top(inner, y, text.valign);
    for (pos, _, _) in &mut laid {
        pos.y += top;
    }
    laid
}

/// The height of `laid` text in px of page: from the top of its box `at`
/// to the bottom of its last line, at `zoom`.
fn laid_height(laid: &[(Pos2, Arc<egui::Galley>, Color32)], at: Rect, zoom: f32) -> Option<f64> {
    let (pos, galley, _) = laid.last()?;
    // Before the fonts are ready a galley has no size: no measure yet.
    if galley.size().y <= 0.0 {
        return None;
    }
    Some(f64::from((pos.y + galley.size().y - at.min.y) / zoom))
}

/// The egui family of a document font: the display face or the UI one.
fn text_family(
    ctx: &egui::Context,
    theme: &Theme,
    style: &gafira_doc::SpanStyle,
) -> egui::FontFamily {
    // A family of the library: its face, or the nearest one (spec 062).
    let library = gafira_theme::library();
    if let Some((bold, italic)) = gafira_theme::nearest_face(
        |b, i| {
            library
                .iter()
                .any(|f| f.family == style.font && f.bold == b && f.italic == i)
        },
        style.bold,
        style.italic,
    ) {
        let face = library_family(ctx, &gafira_theme::face_key(&style.font, bold, italic));
        if face != egui::FontFamily::Proportional {
            return face;
        }
    }
    // A family of the system, once its faces are installed (spec 062).
    let system = system_fonts::catalogue();
    let faces = system.faces();
    if let Some((bold, italic)) = gafira_theme::nearest_face(
        |b, i| {
            faces
                .iter()
                .any(|f| f.family == style.font && f.bold == b && f.italic == i)
        },
        style.bold,
        style.italic,
    ) {
        let face = library_family(ctx, &gafira_theme::face_key(&style.font, bold, italic));
        if face != egui::FontFamily::Proportional {
            return face;
        }
    }
    if style.font == theme.tokens.typography.display {
        egui::FontFamily::Name(crate::style::DISPLAY.into())
    } else {
        egui::FontFamily::Proportional
    }
}

/// The egui family a face of the library is installed under, or the
/// interface family when this context did not install it (a test).
fn library_family(ctx: &egui::Context, key: &str) -> egui::FontFamily {
    let family = egui::FontFamily::Name(key.into());
    if ctx.fonts(|f| f.definitions().families.contains_key(&family)) {
        family
    } else {
        egui::FontFamily::Proportional
    }
}

/// The text `laid` out in the upright box `at`, as one mesh mapped by
/// `turn` (spec 041): each glyph vertex goes back to the local box (`at`
/// at `zoom`) and out through the element's transform, so the text turns
/// and mirrors with its box. The glyph coordinates of a galley are in
/// texels of the font atlas of `atlas` size.
fn text_mesh(
    laid: &[(Pos2, Arc<egui::Galley>, Color32)],
    at: Rect,
    zoom: f32,
    turn: gafira_geom::Affine,
    atlas: [usize; 2],
) -> egui::Mesh {
    let mut mesh = egui::Mesh::default();
    let texel = Vec2::new(1.0 / atlas[0].max(1) as f32, 1.0 / atlas[1].max(1) as f32);
    for (pos, galley, fallback) in laid {
        for row in &galley.rows {
            let start = mesh.vertices.len() as u32;
            let origin = *pos + row.pos.to_vec2();
            mesh.indices
                .extend(row.visuals.mesh.indices.iter().map(|index| index + start));
            mesh.vertices
                .extend(row.visuals.mesh.vertices.iter().map(|vertex| {
                    let upright = origin + vertex.pos.to_vec2() - at.min;
                    let color = if vertex.color == Color32::PLACEHOLDER {
                        *fallback
                    } else {
                        vertex.color
                    };
                    egui::epaint::Vertex {
                        pos: rotation::screen(
                            turn,
                            f64::from(upright.x / zoom),
                            f64::from(upright.y / zoom),
                        ),
                        uv: Pos2::new(vertex.uv.x * texel.x, vertex.uv.y * texel.y),
                        color,
                    }
                }));
        }
    }
    mesh
}

/// A shadow offset as egui's `i8`; the tokens hold small whole numbers.
fn to_i8(value: f32) -> i8 {
    let mut out: i8 = 0;
    while f32::from(out) < value.min(127.0) {
        out += 1;
    }
    out
}

/// A shadow blur as egui's `u8`.
fn to_u8(value: f32) -> u8 {
    let mut out: u8 = 0;
    while f32::from(out) < value.min(255.0) {
        out += 1;
    }
    out
}

#[cfg(test)]
pub(crate) mod tests {
    use std::io;
    use std::path::Path;
    use std::sync::Arc;

    use gafira_doc::{ContentHash, Document};
    use gafira_engine::{
        EngineConfig, EventChannel, ExportPorts, Gafira, NoopWaker, Ports, SettingsPorts,
        SystemPorts,
    };
    use gafira_i18n::D08LocaleResolver;
    use gafira_platform::{
        EnvLocale, JsonSettings, RandomIds, StdFileSystem, StderrNotifier, SystemClock,
        SystemProcesses,
    };
    use gafira_ports::license::NoLicensing;
    use gafira_ports::repository::{LoadedProject, ProjectRepository, RepositoryError, Version};
    use gafira_ports::settings::Settings;
    use gafira_ports::store::{ContentStore, StoreError};
    use gafira_ports::system::ReadSeek;

    use super::*;
    use crate::format::StartFormat;
    use crate::shell::start;

    pub(crate) struct Nothing;

    fn none() -> io::Error {
        io::ErrorKind::Unsupported.into()
    }

    impl ProjectRepository for Nothing {
        fn load(&self, _: &Path) -> Result<LoadedProject, RepositoryError> {
            Err(RepositoryError::Io(none()))
        }
        fn save(&self, _: &Path, _: &Document, _: &[Version]) -> Result<(), RepositoryError> {
            Err(RepositoryError::Io(none()))
        }
    }

    impl ContentStore for Nothing {
        fn put(&self, _: &mut dyn io::Read) -> Result<ContentHash, StoreError> {
            Err(StoreError::Io(none()))
        }
        fn open(&self, _: ContentHash) -> Result<Box<dyn ReadSeek + Send>, StoreError> {
            Err(StoreError::Io(none()))
        }
        fn contains(&self, _: ContentHash) -> Result<bool, StoreError> {
            Ok(false)
        }
    }

    /// An engine over the real platform adapters, with no format and no
    /// store, as the app composes it (spec 018).
    pub(crate) fn engine() -> Gafira {
        engine_over(Arc::new(Nothing))
    }

    /// [`engine`] with `repository` in place of the absent format.
    pub(crate) fn engine_over(repository: Arc<dyn ProjectRepository>) -> Gafira {
        engine_with(repository, Arc::new(Nothing))
    }

    /// [`engine_over`] with `store` as the content store.
    pub(crate) fn engine_with(
        repository: Arc<dyn ProjectRepository>,
        store: Arc<dyn ContentStore>,
    ) -> Gafira {
        // A folder of its own: two tests saving their settings at once
        // would race on the same `settings.json.tmp`.
        static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("gafira-ui-{}-{n}", std::process::id()));
        engine_in(&dir, repository, store)
    }

    /// An engine whose settings and instances live in `dir`.
    pub(crate) fn engine_in(
        dir: &std::path::Path,
        repository: Arc<dyn ProjectRepository>,
        store: Arc<dyn ContentStore>,
    ) -> Gafira {
        let clock = Arc::new(SystemClock::new());
        let ports = Ports::new(
            repository,
            store,
            SystemPorts::new(
                Arc::new(StdFileSystem),
                clock.clone(),
                Arc::new(RandomIds),
                Arc::new(SystemProcesses::new(dir.join("instances"))),
            ),
            Arc::new(StderrNotifier),
            Arc::new(NoLicensing),
            SettingsPorts::new(
                Arc::new(JsonSettings::at(dir.join("settings.json"))),
                Arc::new(D08LocaleResolver::new(Arc::new(EnvLocale))),
            ),
            ExportPorts::new(
                crate::export::tests::pages(),
                Arc::new(gafira_io_image::RasterDecoder::new()),
            ),
        );
        Gafira::new(
            ports,
            crate::export::tests::registries(),
            EngineConfig::new(dir).expect("a data folder"),
            EventChannel::new(Arc::new(NoopWaker), clock),
        )
        .expect("an engine")
    }

    /// One frame's [`Asked`] applied by a fresh editor.
    fn run(engine: &mut Gafira, snapshot: &Option<Arc<Document>>, asked: &Asked) -> Go {
        Editor::default().apply(engine, snapshot, asked, &theme())
    }

    fn theme() -> Theme {
        Theme::new(gafira_theme::ThemeTokens::embedded().expect("tokens"))
    }

    fn pages(engine: &mut Gafira) -> Vec<Vec2> {
        page_sizes(&engine.edit().document().expect("a document"))
    }

    // S63-04: a click on a colour of a gradient of three opens its picker
    // (the menu of its right click used the picker's id and closed it).
    #[test]
    fn s63_04_a_click_opens_the_picker_of_a_third_colour() {
        use gafira_doc::{Gradient, GradientKind, GradientStop};
        let stop = |offset| GradientStop {
            offset,
            color: gafira_color::Color::Srgb(gafira_color::Rgba::new(1.0, 0.0, 0.0, 1.0)),
        };
        let gradient = Gradient {
            kind: GradientKind::Radial,
            stops: vec![stop(0.0), stop(0.5), stop(1.0)],
        };
        let ctx = egui::Context::default();
        theme().apply(&ctx);
        let texts = Texts::new(gafira_ports::i18n::Locale::Es);
        let at = std::cell::Cell::new(Pos2::ZERO);
        let draw = |events: Vec<egui::Event>| {
            let input = egui::RawInput {
                events,
                ..egui::RawInput::default()
            };
            let _ = ctx.run(input, |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    let _ = gradient_stop_button(ui, &texts, &gradient, 2);
                    let last = ui.min_rect();
                    at.set(last.min + Vec2::new(8.0, 8.0));
                });
            });
        };
        draw(Vec::new());
        let pos = at.get();
        let button = |pressed| egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        };
        draw(vec![egui::Event::PointerMoved(pos)]);
        draw(vec![button(true)]);
        draw(vec![button(false)]);
        draw(Vec::new());
        assert!(egui::Popup::is_any_open(&ctx), "the picker is open");
    }

    // S63-04: «+» asks for the picker of the colour it adds; the next frame
    // that draws that colour opens it, once.
    #[test]
    fn s63_04_the_added_colour_opens_its_picker() {
        use gafira_doc::{Gradient, GradientKind, GradientStop};
        let stop = |offset| GradientStop {
            offset,
            color: gafira_color::Color::Srgb(gafira_color::Rgba::new(1.0, 0.0, 0.0, 1.0)),
        };
        let gradient = Gradient {
            kind: GradientKind::Radial,
            stops: vec![stop(0.0), stop(0.5), stop(1.0)],
        };
        let ctx = egui::Context::default();
        theme().apply(&ctx);
        let texts = Texts::new(gafira_ports::i18n::Locale::Es);
        let draw = |ctx: &egui::Context| {
            let mut asked = Asked::default();
            let _ = ctx.run(egui::RawInput::default(), |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    gradient_fields(ui, &texts, &gradient, &mut asked);
                });
            });
        };
        draw(&ctx);
        assert!(!egui::Popup::is_any_open(&ctx));
        ctx.data_mut(|d| d.insert_temp(gradient_pending_id(), 2usize));
        draw(&ctx);
        assert!(
            egui::Popup::is_any_open(&ctx),
            "the picker of the new colour"
        );
        assert_eq!(
            ctx.data(|d| d.get_temp::<usize>(gradient_pending_id())),
            None
        );
    }

    #[test]
    fn s18_06_start_creates_one_page_of_the_format_with_an_identity() {
        let mut engine = engine();
        start(&mut engine, StartFormat::VideoVertical).expect("a new document");
        assert_eq!(pages(&mut engine), [Vec2::new(1080.0, 1920.0)]);
        let document = engine.edit().document().expect("a document");
        assert!(!document.project_id().is_nil(), "D-63");

        start(&mut engine, StartFormat::Print).expect("another new document");
        let a4 = pages(&mut engine)[0];
        assert!(
            (a4.x - 793.7).abs() < 0.1 && (a4.y - 1122.5).abs() < 0.1,
            "{a4:?}"
        );
    }

    // Spec 079: a hand stroke goes into the page as one vector element,
    // kept as a freehand stroke with its brush; the eraser removes it in
    // one step that undo brings back.
    #[test]
    fn s79_a_stroke_is_one_element_and_the_eraser_removes_it() {
        let mut engine = engine();
        start(&mut engine, StartFormat::Social).expect("a new document");
        let snapshot = engine.edit().document();
        let page = snapshot.as_ref().expect("doc").pages()[0].id;
        let brush = crate::draw::BRUSHES[1];
        let points: Vec<[f64; 2]> = (0..60)
            .map(|k| {
                [
                    100.0 + f64::from(k) * 5.0,
                    300.0 + 40.0 * (f64::from(k) / 9.0).sin(),
                ]
            })
            .collect();
        let outline = crate::draw::outline(&points, 10.0, brush.cap, &brush.style);
        let ((x, y, w, h), path) = crate::draw::to_path(&outline).expect("path");
        let mut shape = gafira_doc::ShapeNode::new(ShapeGeometry::Path(path));
        shape.fill = Some(canvas::document_color(Color32::RED));
        shape.drawn_with = Some(brush.id.to_owned());
        let spec = gafira_edit::NodeSpec::new(
            gafira_doc::NodeKind::Shape(shape),
            gafira_doc::Placement::new(x, y, w, h),
        );
        let draw = Asked {
            draw_stroke: Some((page, spec)),
            ..Asked::default()
        };
        run(&mut engine, &snapshot, &draw);
        let after = engine.edit().document();
        let on = &after.as_ref().expect("doc").pages()[0];
        assert_eq!(on.roots.len(), 1, "one element");
        let id = on.roots[0];
        let gafira_doc::NodeKind::Shape(drawn) = &on.node(id).expect("node").kind else {
            panic!("a shape");
        };
        assert_eq!(drawn.drawn_with.as_deref(), Some("marker"));
        let erase = Asked {
            erase: Some((page, vec![id])),
            ..Asked::default()
        };
        run(&mut engine, &after, &erase);
        assert!(
            engine.edit().document().as_ref().expect("doc").pages()[0]
                .roots
                .is_empty()
        );
        let snapshot = engine.edit().document();
        run(
            &mut engine,
            &snapshot,
            &Asked {
                undo: true,
                ..Asked::default()
            },
        );
        assert_eq!(
            engine.edit().document().as_ref().expect("doc").pages()[0]
                .roots
                .len(),
            1
        );
    }

    #[test]
    fn s18_07_add_page_undo_and_redo_go_through_the_engine() {
        let mut engine = engine();
        start(&mut engine, StartFormat::Social).expect("a new document");
        let snapshot = engine.edit().document();
        let add = Asked {
            add_page: true,
            ..Asked::default()
        };
        assert!(matches!(run(&mut engine, &snapshot, &add), Go::Stay));
        assert_eq!(pages(&mut engine).len(), 2);
        assert_eq!(
            pages(&mut engine)[1],
            Vec2::new(1080.0, 1080.0),
            "same size as the last"
        );
        assert_eq!(engine.edit().can_undo_redo(), (true, false));

        let snapshot = engine.edit().document();
        run(
            &mut engine,
            &snapshot,
            &Asked {
                undo: true,
                ..Asked::default()
            },
        );
        assert_eq!(pages(&mut engine).len(), 1);
        assert_eq!(
            engine.edit().can_undo_redo(),
            (false, true),
            "the first page belongs to the new document (spec 016)"
        );

        let snapshot = engine.edit().document();
        run(
            &mut engine,
            &snapshot,
            &Asked {
                redo: true,
                ..Asked::default()
            },
        );
        assert_eq!(pages(&mut engine).len(), 2);
        assert!(engine.projects().is_modified());
        assert!(matches!(
            run(
                &mut engine,
                &snapshot,
                &Asked {
                    home: true,
                    ..Asked::default()
                }
            ),
            Go::Home
        ));
    }

    #[test]
    fn s19_11_a_project_saved_as_gaf_reopens_with_its_pages_and_identity() {
        let dir = std::env::temp_dir().join(format!("gafira-ui-gaf-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("a folder");
        let path = dir.join("poster.gaf");
        let mut engine = engine_over(Arc::new(gafira_format::GafRepository::default()));
        start(&mut engine, StartFormat::Print).expect("a new document");
        let snapshot = engine.edit().document();
        run(
            &mut engine,
            &snapshot,
            &Asked {
                add_page: true,
                ..Asked::default()
            },
        );
        let before = engine.edit().document().expect("a document");

        engine.projects().save_as(&path).expect("saves");
        assert!(!engine.projects().is_modified());
        engine.projects().new_document().expect("another document");
        engine.projects().open(&path).expect("reopens");

        let after = engine.edit().document().expect("a document");
        assert_eq!(*after, *before);
        assert_eq!(engine.projects().path(), Some(path.clone()));
        assert!(!engine.projects().is_modified());
        std::fs::remove_dir_all(dir).expect("clean up");
    }

    fn the_shapes(engine: &mut Gafira) -> Vec<Shape> {
        let document = engine.edit().document().expect("a document");
        canvas::shapes(&document.pages()[0])
    }

    // S20-05
    #[test]
    fn s20_05_a_piece_is_inserted_selected_moved_in_one_undo_and_deleted() {
        let mut engine = engine();
        start(&mut engine, StartFormat::Social).expect("a new document");
        let theme = theme();
        let mut editor = Editor::default();
        let snapshot = engine.edit().document();
        editor.apply(
            &mut engine,
            &snapshot,
            &Asked {
                insert: Some(canvas::Piece::Circle),
                ..Asked::default()
            },
            &theme,
        );
        let shapes = the_shapes(&mut engine);
        assert_eq!(shapes.len(), 1);
        assert_eq!(shapes[0].geometry, ShapeGeometry::Ellipse);
        assert_eq!(
            editor.selected,
            Selection::one(shapes[0].page, shapes[0].node),
            "the new piece is selected"
        );
        let origin = shapes[0].placement;

        // A drag of three moves is one gesture: one undo takes it back.
        editor.zoom = 2.0;
        editor.apply(
            &mut engine,
            &snapshot,
            &Asked {
                drag_begin: Some(None),
                ..Asked::default()
            },
            &theme,
        );
        for _ in 0..3 {
            editor.apply(
                &mut engine,
                &snapshot,
                &Asked {
                    drag_to: Some(Vec2::new(10.0, 4.0)),
                    ..Asked::default()
                },
                &theme,
            );
        }
        editor.apply(
            &mut engine,
            &snapshot,
            &Asked {
                drag_end: true,
                ..Asked::default()
            },
            &theme,
        );
        let moved = the_shapes(&mut engine)[0].placement;
        assert_eq!((moved.x - origin.x, moved.y - origin.y), (15.0, 6.0));
        engine.edit().undo().expect("an undo");
        assert_eq!(
            the_shapes(&mut engine)[0].placement,
            origin,
            "one entry for the whole drag"
        );
        engine.edit().redo().expect("a redo");

        let mut typed = moved;
        typed.w = 500.0;
        editor.apply(
            &mut engine,
            &snapshot,
            &Asked {
                place: Some(typed),
                ..Asked::default()
            },
            &theme,
        );
        assert_eq!(the_shapes(&mut engine)[0].placement.w, 500.0);

        editor.apply(
            &mut engine,
            &snapshot,
            &Asked {
                delete: true,
                ..Asked::default()
            },
            &theme,
        );
        assert!(the_shapes(&mut engine).is_empty());
        assert_eq!(editor.selected, Selection::default());
        engine.edit().undo().expect("an undo");
        assert_eq!(the_shapes(&mut engine).len(), 1, "delete is undoable");
    }

    // S21-02
    #[test]
    fn s21_02_dark_mode_is_asked_for_and_the_settings_keep_it() {
        use gafira_ports::settings::Appearance;
        // Two engines over the same settings: the second reads what the
        // first saved.
        let dir = std::env::temp_dir().join(format!("gafira-ui-s21-{}", std::process::id()));
        let fresh = || engine_in(&dir, Arc::new(Nothing), Arc::new(Nothing));
        let mut engine = fresh();
        start(&mut engine, StartFormat::Social).expect("a new document");
        let snapshot = engine.edit().document();
        assert!(matches!(
            run(
                &mut engine,
                &snapshot,
                &Asked {
                    dark: Some(true),
                    ..Asked::default()
                }
            ),
            Go::Dark(true)
        ));
        let settings = engine.settings().get().with_appearance(Appearance::Dark);
        engine.settings().set(settings).expect("saves");
        let again = fresh();
        again.settings().start().expect("loads");
        assert_eq!(again.settings().get().appearance, Appearance::Dark);
        again.settings().set(Settings::default()).expect("resets");
    }

    // S22-03
    #[test]
    fn s22_03_dragging_a_handle_resizes_in_one_undo() {
        let mut engine = engine();
        start(&mut engine, StartFormat::Social).expect("a new document");
        let theme = theme();
        let mut editor = Editor::default();
        let snapshot = engine.edit().document();
        editor.apply(
            &mut engine,
            &snapshot,
            &Asked {
                insert: Some(canvas::Piece::Square),
                ..Asked::default()
            },
            &theme,
        );
        let origin = the_shapes(&mut engine)[0].placement;
        editor.zoom = 1.0;
        let corner = canvas::Handle { x: 1, y: 1 };
        editor.apply(
            &mut engine,
            &snapshot,
            &Asked {
                drag_begin: Some(Some(Hold::Resize(corner))),
                ..Asked::default()
            },
            &theme,
        );
        for _ in 0..2 {
            editor.apply(
                &mut engine,
                &snapshot,
                &Asked {
                    drag_to: Some(Vec2::new(20.0, 10.0)),
                    ..Asked::default()
                },
                &theme,
            );
        }
        editor.apply(
            &mut engine,
            &snapshot,
            &Asked {
                drag_end: true,
                ..Asked::default()
            },
            &theme,
        );
        let resized = the_shapes(&mut engine)[0].placement;
        assert_eq!(
            (resized.x, resized.y),
            (origin.x, origin.y),
            "the top-left corner stays"
        );
        // The corner keeps the square's proportions (D-89, spec 045): the
        // wider drag drives.
        assert_eq!((resized.w - origin.w, resized.h - origin.h), (40.0, 40.0));
        engine.edit().undo().expect("an undo");
        assert_eq!(the_shapes(&mut engine)[0].placement, origin);
    }

    // S23-02
    #[test]
    fn s23_02_fill_and_opacity_change_through_the_engine_and_undo() {
        let mut engine = engine();
        start(&mut engine, StartFormat::Social).expect("a new document");
        let theme = theme();
        let mut editor = Editor::default();
        let snapshot = engine.edit().document();
        editor.apply(
            &mut engine,
            &snapshot,
            &Asked {
                insert: Some(canvas::Piece::Square),
                ..Asked::default()
            },
            &theme,
        );
        let blue = canvas::document_color(Color32::from_rgb(0, 0, 255));
        editor.apply(
            &mut engine,
            &snapshot,
            &Asked {
                fill: Some(Some(blue)),
                ..Asked::default()
            },
            &theme,
        );
        assert_eq!(the_shapes(&mut engine)[0].fill, Some(blue));
        editor.apply(
            &mut engine,
            &snapshot,
            &Asked {
                opacity: Some(0.25),
                ..Asked::default()
            },
            &theme,
        );
        assert_eq!(the_shapes(&mut engine)[0].opacity, 0.25);
        editor.apply(
            &mut engine,
            &snapshot,
            &Asked {
                fill: Some(None),
                ..Asked::default()
            },
            &theme,
        );
        assert_eq!(the_shapes(&mut engine)[0].fill, None);
        engine.edit().undo().expect("an undo");
        assert_eq!(the_shapes(&mut engine)[0].fill, Some(blue));
    }

    // S38-08: the outline and the corner radius change through the engine,
    // one undo step each.
    #[test]
    fn s38_08_outline_and_corner_radius_change_through_the_engine_and_undo() {
        let mut engine = engine();
        start(&mut engine, StartFormat::Social).expect("a new document");
        let theme = theme();
        let mut editor = Editor::default();
        let snapshot = engine.edit().document();
        let ask = |editor: &mut Editor, engine: &mut Gafira, asked: Asked| {
            editor.apply(engine, &snapshot, &asked, &theme);
        };
        ask(
            &mut editor,
            &mut engine,
            Asked {
                insert: Some(canvas::Piece::Square),
                ..Asked::default()
            },
        );
        let blue = canvas::document_color(Color32::from_rgb(0, 0, 255));
        let outline = Some(gafira_doc::ShapeStroke::new(blue, 3.0));
        ask(
            &mut editor,
            &mut engine,
            Asked {
                stroke: Some(outline),
                ..Asked::default()
            },
        );
        assert_eq!(the_shapes(&mut engine)[0].stroke, outline);
        ask(
            &mut editor,
            &mut engine,
            Asked {
                corner_radius: Some(12.0),
                ..Asked::default()
            },
        );
        assert_eq!(the_shapes(&mut engine)[0].corner_radius, 12.0);
        ask(
            &mut editor,
            &mut engine,
            Asked {
                stroke: Some(None),
                ..Asked::default()
            },
        );
        assert_eq!(the_shapes(&mut engine)[0].stroke, None);
        engine.edit().undo().expect("an undo");
        assert_eq!(the_shapes(&mut engine)[0].stroke, outline);
        engine.edit().undo().expect("an undo");
        assert_eq!(the_shapes(&mut engine)[0].corner_radius, 0.0);
        assert_eq!(the_shapes(&mut engine)[0].stroke, outline);
        engine.edit().undo().expect("an undo");
        assert_eq!(the_shapes(&mut engine)[0].stroke, None);
    }

    // S24-02
    #[test]
    fn s24_02_bring_forward_reorders_through_the_engine() {
        let mut engine = engine();
        start(&mut engine, StartFormat::Social).expect("a new document");
        let theme = theme();
        let mut editor = Editor::default();
        for geometry in [ShapeGeometry::Rect, ShapeGeometry::Ellipse] {
            // A fresh snapshot each time, as every frame of the window has.
            let snapshot = engine.edit().document();
            editor.apply(
                &mut engine,
                &snapshot,
                &Asked {
                    insert: Some(match geometry {
                        ShapeGeometry::Ellipse => canvas::Piece::Circle,
                        _ => canvas::Piece::Square,
                    }),
                    ..Asked::default()
                },
                &theme,
            );
        }
        let page = engine.edit().document().expect("a document").pages()[0].clone();
        let rect = page.roots[0];
        assert_eq!(
            canvas::layers(&page, &BTreeSet::new())[0].kind,
            "pieces.ellipse",
            "the newest is in front"
        );
        let snapshot = engine.edit().document();
        editor.apply(
            &mut engine,
            &snapshot,
            &Asked {
                reorder: Some((page.id, rect, 1)),
                ..Asked::default()
            },
            &theme,
        );
        let page = engine.edit().document().expect("a document").pages()[0].clone();
        assert_eq!(
            canvas::layers(&page, &BTreeSet::new())[0].node,
            rect,
            "the rectangle came to the front"
        );
        engine.edit().undo().expect("an undo");
        let page = engine.edit().document().expect("a document").pages()[0].clone();
        assert_eq!(page.roots[0], rect);
    }

    // S27-04
    #[test]
    fn s27_04_a_text_is_added_edited_restyled_and_undone_through_the_engine() {
        let mut engine = engine();
        start(&mut engine, StartFormat::Social).expect("a new document");
        let theme = theme();
        let mut editor = Editor::default();
        let snapshot = engine.edit().document();
        let asked = Asked {
            insert_text: Some((canvas::TextPreset::Title, "Título".to_owned())),
            ..Asked::default()
        };
        editor.apply(&mut engine, &snapshot, &asked, &theme);
        let text = |engine: &mut Gafira| the_shapes(engine)[0].text.clone().expect("a text");
        let first = text(&mut engine);
        assert_eq!(first.content.plain_text(), "Título");
        let style = first.content.first_style().expect("a style").clone();
        assert_eq!(
            style.font, theme.tokens.typography.display,
            "a title uses the display family"
        );
        assert_eq!(style.size, 72.0);
        assert!(!editor.selected.is_empty(), "the new text is selected");

        let edited = gafira_doc::RichText::plain("Hola\nmundo", &style);
        editor.apply(
            &mut engine,
            &snapshot,
            &Asked {
                text: Some(edited.clone()),
                ..Asked::default()
            },
            &theme,
        );
        assert_eq!(text(&mut engine).content, edited);

        let bigger = gafira_doc::SpanStyle {
            size: 90.0,
            ..style.clone()
        };
        let restyled = gafira_doc::RichText::plain("Hola\nmundo", &bigger);
        editor.apply(
            &mut engine,
            &snapshot,
            &Asked {
                text: Some(restyled),
                ..Asked::default()
            },
            &theme,
        );
        assert!(
            text(&mut engine)
                .content
                .spans()
                .all(|span| span.style.size == 90.0)
        );

        let broken = gafira_doc::SpanStyle { size: 0.0, ..style };
        let refused = gafira_doc::RichText::plain("x", &broken);
        editor.apply(
            &mut engine,
            &snapshot,
            &Asked {
                text: Some(refused),
                ..Asked::default()
            },
            &theme,
        );
        assert_eq!(
            text(&mut engine).content.plain_text(),
            "Hola\nmundo",
            "a zero size is refused"
        );

        // Consecutive edits of one text merge into one entry (ARQ-33), so one
        // undo takes back the editing and leaves the inserted title.
        engine.edit().undo().expect("an undo");
        assert_eq!(text(&mut engine).content.plain_text(), "Título");
    }

    // S29-03
    #[test]
    fn s29_03_an_imported_image_is_placed_drawn_from_the_store_and_undone() {
        let dir = std::env::temp_dir().join(format!("gafira-ui-image-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("a folder");
        let png = dir.join("dot.png");
        image::RgbaImage::from_pixel(40, 20, image::Rgba([255, 0, 0, 255]))
            .save(&png)
            .expect("a png");
        let store = Arc::new(gafira_assets::FsStore::new(dir.join("store")));
        let mut engine = engine_with(Arc::new(Nothing), store);
        start(&mut engine, StartFormat::Social).expect("a new document");

        let asset = engine.assets().import(&png).expect("imports");
        let page = engine.edit().document().expect("a document").pages()[0].id;
        let spec = canvas::new_image(Vec2::new(1080.0, 1080.0), (40, 20), asset);
        engine
            .edit()
            .execute(Box::new(InsertNodes::new(page, None, 0, vec![spec])))
            .expect("inserts");
        let shapes = the_shapes(&mut engine);
        assert_eq!(shapes[0].image, Some(asset));
        assert_eq!((shapes[0].placement.w, shapes[0].placement.h), (40.0, 20.0));

        let ctx = egui::Context::default();
        let mut editor = Editor::default();
        let document = engine.edit().document();
        editor.load_textures(&ctx, &engine, document.as_deref());
        let texture = editor
            .textures
            .get(&asset.as_hash())
            .and_then(Option::as_ref)
            .expect("decoded");
        assert_eq!(texture.size(), [40, 20]);

        engine.edit().undo().expect("an undo");
        assert!(the_shapes(&mut engine).is_empty());
        std::fs::remove_dir_all(dir).expect("clean up");
    }

    // Spec 067: a text of image paths or file URIs (what a file manager
    // copies) is a list of images; anything else is text.
    #[test]
    fn s67_pasted_paths_of_images_are_images() {
        let dir = std::env::temp_dir().join(format!("gafira ui paths {}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("a folder");
        let png = dir.join("a b.png");
        image::RgbaImage::new(2, 2).save(&png).expect("a png");
        let uri = format!("file://{}", png.display()).replace(' ', "%20");
        assert_eq!(image_paths(&uri), vec![png.clone()]);
        assert_eq!(image_paths(&format!("copy\n{uri}")), vec![png.clone()]);
        assert_eq!(image_paths(&png.display().to_string()), vec![png.clone()]);
        assert!(image_paths("hola").is_empty());
        assert!(
            image_paths(&format!("{uri}\nhola")).is_empty(),
            "all or nothing"
        );
        assert!(image_paths("/no/such/file.png").is_empty());
        std::fs::remove_dir_all(dir).expect("clean up");
    }

    // Spec 067: an image dropped from the system lands centred where it was
    // dropped; dropped on a frame, it becomes the frame's photo.
    #[test]
    fn s67_a_dropped_image_lands_at_the_point_or_in_the_frame() {
        let dir = std::env::temp_dir().join(format!("gafira-ui-drop-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("a folder");
        let png = dir.join("drop.png");
        image::RgbaImage::from_pixel(40, 20, image::Rgba([9, 9, 9, 255]))
            .save(&png)
            .expect("a png");
        let store = Arc::new(gafira_assets::FsStore::new(dir.join("store")));
        let mut engine = engine_with(Arc::new(Nothing), store);
        start(&mut engine, StartFormat::Social).expect("a new document");
        let mut editor = Editor::default();
        let page = engine.edit().document().expect("doc").pages()[0].id;
        // The page drawn at (0, 0) at 50 %.
        editor.zoom = 0.5;
        editor.page_rects = vec![(page, Rect::from_min_size(Pos2::ZERO, Vec2::splat(540.0)))];
        editor.place_images(
            &mut engine,
            std::slice::from_ref(&png),
            Some(Pos2::new(100.0, 60.0)),
        );
        let document = engine.edit().document().expect("doc");
        let on = &document.pages()[0];
        let image = on
            .node(*on.roots.last().expect("a root"))
            .expect("the image");
        let p = image.base.placement;
        assert!(matches!(image.kind, gafira_doc::NodeKind::Image(_)));
        assert!((p.x + p.w / 2.0 - 200.0).abs() < 1e-6 && (p.y + p.h / 2.0 - 120.0).abs() < 1e-6);
        // A frame in the middle of the page takes the next one.
        ask(
            &mut engine,
            &mut editor,
            Asked {
                insert_frame: Some(canvas::Piece::Square),
                ..Asked::default()
            },
        );
        let (_, frame) = editor.selected.single().expect("the frame");
        editor.page_rects = vec![(page, Rect::from_min_size(Pos2::ZERO, Vec2::splat(540.0)))];
        editor.place_images(&mut engine, &[png], Some(Pos2::new(270.0, 270.0)));
        let document = engine.edit().document().expect("doc");
        let photo = frame_of(&document, page, frame).and_then(|f| f.content);
        assert_eq!(photo.map(|c| c.natural), Some((40, 20)));
        std::fs::remove_dir_all(dir).expect("clean up");
    }

    // Spec 067: an SVG brought in «tal cual» is a group of vector shapes
    // with its colours, at its own size, centred where it was dropped.
    #[test]
    fn s67_an_svg_as_it_is_is_a_group_of_shapes() {
        let dir = std::env::temp_dir().join(format!("gafira-ui-svgv-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("a folder");
        let svg = dir.join("logo.svg");
        std::fs::write(
            &svg,
            r##"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="50"><rect width="40" height="50" fill="#ff0000"/><circle cx="75" cy="25" r="25" fill="#0000ff"/></svg>"##,
        )
        .expect("an svg");
        let mut engine = engine();
        let mut editor = Editor::default();
        let (page, _, _) = two_pieces(&mut engine, &mut editor);
        editor.zoom = 1.0;
        editor.page_rects = vec![(page, Rect::from_min_size(Pos2::ZERO, Vec2::splat(800.0)))];
        editor.place_images(&mut engine, &[svg], Some(Pos2::new(300.0, 200.0)));
        ask(
            &mut engine,
            &mut editor,
            Asked {
                svg_as_image: Some(true),
                ..Asked::default()
            },
        );
        let document = engine.edit().document().expect("doc");
        let on = &document.pages()[0];
        let group = on
            .node(*on.roots.last().expect("a root"))
            .expect("the group");
        assert!(matches!(group.kind, gafira_doc::NodeKind::Group(_)));
        assert_eq!(group.children.len(), 2);
        let p = group.base.placement;
        assert_eq!((p.x, p.y, p.w, p.h), (250.0, 175.0, 100.0, 50.0));
        let red = on.node(group.children[0]).expect("the first shape");
        let gafira_doc::NodeKind::Shape(shape) = &red.kind else {
            panic!("a shape")
        };
        assert!(matches!(shape.geometry, gafira_doc::ShapeGeometry::Path(_)));
        assert!(shape.fill.is_some());
        assert_eq!(red.base.placement.x, 250.0);
        // Its colours: the red changed to green in one undo step.
        let colors = crate::recolor::colors(on, group.base.id);
        assert_eq!(colors.len(), 2, "red and blue");
        let green = gafira_color::Color::Srgb(gafira_color::Rgba::new(0.0, 1.0, 0.0, 1.0));
        let depth = undo_depth(&mut engine);
        editor.selected = Selection::one(page, group.base.id);
        let group_id = group.base.id;
        ask(
            &mut engine,
            &mut editor,
            Asked {
                recolor: Some((group_id, colors[0], green)),
                ..Asked::default()
            },
        );
        let document = engine.edit().document().expect("doc");
        let after = crate::recolor::colors(&document.pages()[0], group_id);
        assert!(after.iter().any(|c| crate::recolor::same(*c, green)));
        assert!(!after.iter().any(|c| crate::recolor::same(*c, colors[0])));
        assert_eq!(undo_depth(&mut engine), depth + 1);
        std::fs::remove_dir_all(dir).expect("clean up");
    }

    // S66-10: an SVG dropped on the canvas becomes an empty frame of its
    // outline, with its proportions, centred where it was dropped.
    #[test]
    fn s66_10_a_dropped_svg_is_a_custom_frame() {
        let dir = std::env::temp_dir().join(format!("gafira-ui-svg-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("a folder");
        let svg = dir.join("dog.svg");
        std::fs::write(
            &svg,
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="200" height="100"><path d="M0 100 L100 0 L200 100 Z"/></svg>"#,
        )
        .expect("an svg");
        let mut engine = engine();
        let mut editor = Editor::default();
        let (page, _, _) = two_pieces(&mut engine, &mut editor);
        editor.zoom = 1.0;
        editor.page_rects = vec![(page, Rect::from_min_size(Pos2::ZERO, Vec2::splat(800.0)))];
        editor.place_images(&mut engine, &[svg], Some(Pos2::new(300.0, 200.0)));
        assert!(editor.pending_svg.is_some(), "it asks how to come in");
        ask(
            &mut engine,
            &mut editor,
            Asked {
                svg_as_image: Some(false),
                ..Asked::default()
            },
        );
        assert!(editor.pending_svg.is_none());
        let document = engine.edit().document().expect("doc");
        let on = &document.pages()[0];
        let node = on
            .node(*on.roots.last().expect("a root"))
            .expect("the frame");
        let gafira_doc::NodeKind::Frame(frame) = &node.kind else {
            panic!("a frame")
        };
        assert!(matches!(
            frame.clip_shape,
            gafira_doc::ShapeGeometry::Path(_)
        ));
        assert!(frame.content.is_none());
        let p = node.base.placement;
        assert!(
            (p.w / p.h - 2.0).abs() < 1e-6,
            "the proportions of the drawing"
        );
        assert!((p.x + p.w / 2.0 - 300.0).abs() < 1e-6 && (p.y + p.h / 2.0 - 200.0).abs() < 1e-6);
        std::fs::remove_dir_all(dir).expect("clean up");
    }

    /// A Biblioteca in memory (spec 068).
    #[derive(Default)]
    struct MemoryLibrary(std::cell::RefCell<Vec<crate::LibraryEntry>>);

    impl crate::LibraryStore for MemoryLibrary {
        fn existing(&self) -> Vec<crate::LibraryEntry> {
            self.0
                .borrow()
                .iter()
                .filter(|e| e.path.is_file())
                .cloned()
                .collect()
        }

        fn record(&self, file: &std::path::Path, design: bool) {
            self.0.borrow_mut().insert(
                0,
                crate::LibraryEntry {
                    path: file.to_path_buf(),
                    design,
                    added: 0,
                },
            );
        }

        fn remove(&self, entries: &[(std::path::PathBuf, bool)]) {
            self.0
                .borrow_mut()
                .retain(|e| !entries.contains(&(e.path.clone(), e.design)));
        }
    }

    // S68-01, S68-04: an imported file is remembered by Biblioteca, and a
    // file of Biblioteca added again lands on the page.
    #[test]
    fn s68_04_imports_are_remembered_and_added_again() {
        use crate::LibraryStore as _;
        let dir = std::env::temp_dir().join(format!("gafira-ui-lib-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("a folder");
        let png = dir.join("sol.png");
        image::RgbaImage::from_pixel(30, 30, image::Rgba([250, 200, 0, 255]))
            .save(&png)
            .expect("a png");
        let store = Arc::new(gafira_assets::FsStore::new(dir.join("store")));
        let mut engine = engine_with(Arc::new(Nothing), store);
        start(&mut engine, StartFormat::Social).expect("a new document");
        let mut editor = Editor::default();
        let library = std::rc::Rc::new(MemoryLibrary::default());
        editor.library = Some(library.clone());
        editor.place_images(&mut engine, std::slice::from_ref(&png), None);
        let paths = |library: &MemoryLibrary| {
            library
                .existing()
                .into_iter()
                .map(|e| (e.path, e.design))
                .collect::<Vec<_>>()
        };
        assert_eq!(paths(&library), [(png.clone(), false)]);
        let count = |engine: &mut Gafira| {
            engine.edit().document().expect("doc").pages()[0]
                .roots
                .len()
        };
        let before = count(&mut engine);
        ask(
            &mut engine,
            &mut editor,
            Asked {
                library_add: Some((png.clone(), false, None)),
                ..Asked::default()
            },
        );
        assert_eq!(
            count(&mut engine),
            before + 1,
            "added again from Biblioteca"
        );
        ask(
            &mut engine,
            &mut editor,
            Asked {
                library_remove: vec![(png.clone(), false)],
                ..Asked::default()
            },
        );
        assert!(paths(&library).is_empty(), "taken out of the list");
        assert!(png.is_file(), "the file stays on the disk");
        library.record(&png, false);
        std::fs::remove_file(&png).expect("deleted");
        assert!(library.existing().is_empty(), "a deleted file is not shown");
        std::fs::remove_dir_all(dir).expect("clean up");
    }

    // Spec 070: alignment, spacing and a gradient go to the selected text,
    // each one undo step; a solid colour takes the gradient off.
    #[test]
    fn s70_06_text_styles_from_the_inspector() {
        let mut engine = engine();
        start(&mut engine, StartFormat::Social).expect("a new document");
        let mut editor = Editor::default();
        ask(
            &mut engine,
            &mut editor,
            Asked {
                insert_text: Some((canvas::TextPreset::Title, "Hola".to_owned())),
                ..Asked::default()
            },
        );
        let (page, node) = editor.selected.single().expect("the text");
        let read = |engine: &mut Gafira| {
            let document = engine.edit().document().expect("doc");
            let found = document
                .page(page)
                .and_then(|p| p.node(node))
                .expect("node");
            let gafira_doc::NodeKind::Text(text) = &found.kind else {
                panic!("a text")
            };
            text.clone()
        };
        let gradient = crate::colors::gradient(0xFF0000, 0x0000FF, 45.0);
        let depth = undo_depth(&mut engine);
        for asked in [
            Asked {
                text_align: Some(gafira_doc::TextAlign::Center),
                ..Asked::default()
            },
            Asked {
                text_spacing: Some((120.0, Some(1.5))),
                ..Asked::default()
            },
            Asked {
                text_fill: Some(TextFill::Gradient(gradient.clone())),
                ..Asked::default()
            },
        ] {
            ask(&mut engine, &mut editor, asked);
        }
        let text = read(&mut engine);
        assert_eq!(text.align, gafira_doc::TextAlign::Center);
        assert_eq!((text.letter_spacing, text.line_height), (120.0, Some(1.5)));
        assert_eq!(text.gradient, Some(gradient));
        assert_eq!(undo_depth(&mut engine), depth + 3);
        let red = crate::colors::hex(0xFF3131);
        ask(
            &mut engine,
            &mut editor,
            Asked {
                text_fill: Some(TextFill::Solid(red)),
                ..Asked::default()
            },
        );
        let text = read(&mut engine);
        assert_eq!(text.gradient, None);
        assert_eq!(text.content.first_style().map(|s| s.color), Some(red));
        assert_eq!(undo_depth(&mut engine), depth + 4);
    }

    // Marcos (owner request): a letter comes in as one frame cut by its
    // outline, a phone as a group holding a screen frame; each selected, in
    // one undo step.
    #[test]
    fn frames_of_marcos_come_in() {
        use crate::frames::{Holder, Kind};
        let mut engine = engine();
        start(&mut engine, StartFormat::Social).expect("a new document");
        let mut editor = Editor::default();
        for (kind, group) in [
            (Kind::Glyph('A'), false),
            (Kind::Holder(Holder::Phone), true),
        ] {
            let depth = undo_depth(&mut engine);
            ask(
                &mut engine,
                &mut editor,
                Asked {
                    insert_frame_kind: Some((kind, None)),
                    ..Asked::default()
                },
            );
            let document = engine.edit().document().expect("doc");
            let page = &document.pages()[0];
            let (_, node) = editor.selected.single().expect("selected");
            let node = page.node(node).expect("node");
            match (&node.kind, group) {
                (gafira_doc::NodeKind::Frame(frame), false) => {
                    assert!(matches!(frame.clip_shape, ShapeGeometry::Path(_)));
                }
                (gafira_doc::NodeKind::Group(_), true) => {
                    assert!(node.children.iter().any(|c| matches!(
                        page.node(*c).map(|n| &n.kind),
                        Some(gafira_doc::NodeKind::Frame(_))
                    )));
                }
                (other, _) => panic!("{other:?}"),
            }
            assert_eq!(undo_depth(&mut engine), depth + 1);
        }
    }

    // S66-09 (owner request): a Foto collage fills the whole current page,
    // a group of frames, selected; one undo step.
    #[test]
    fn s66_09_a_collage_fills_the_current_page() {
        let mut engine = engine();
        start(&mut engine, StartFormat::Social).expect("a new document");
        let mut editor = Editor::default();
        let depth = undo_depth(&mut engine);
        ask(
            &mut engine,
            &mut editor,
            Asked {
                insert_collage: Some((9, None)),
                ..Asked::default()
            },
        );
        let document = engine.edit().document().expect("doc");
        let page = &document.pages()[0];
        let size = page.size.to_px(document.dpi()).expect("px");
        let (_, group) = editor.selected.single().expect("the collage");
        let node = page.node(group).expect("group");
        let p = node.base.placement;
        assert_eq!(
            (p.x, p.y, p.w, p.h),
            (0.0, 0.0, size.width(), size.height())
        );
        assert_eq!(node.children.len(), 4);
        assert_eq!(undo_depth(&mut engine), depth + 1);
    }

    // Spec 072: an illustration comes in as one selected group of vector
    // shapes, 30 % of the page's shorter side, named by its credit; one
    // undo step.
    #[test]
    fn an_illustration_is_a_group_of_shapes() {
        let mut engine = engine();
        start(&mut engine, StartFormat::Social).expect("a new document");
        let mut editor = Editor::default();
        let svg = br##"<svg xmlns="http://www.w3.org/2000/svg" width="128" height="128" viewBox="0 0 128 128"><circle cx="64" cy="64" r="40" fill="#e33"/><rect x="10" y="10" width="20" height="20" fill="#3a3"/></svg>"##;
        let depth = undo_depth(&mut engine);
        ask(
            &mut engine,
            &mut editor,
            Asked {
                insert_illustration: Some((
                    Arc::new(svg.to_vec()),
                    "sunflower — Noto Emoji (Google, Apache 2.0)".to_owned(),
                    None,
                )),
                ..Asked::default()
            },
        );
        let document = engine.edit().document().expect("doc");
        let page = &document.pages()[0];
        let (_, group) = editor.selected.single().expect("the group");
        let node = page.node(group).expect("group");
        assert!(matches!(node.kind, gafira_doc::NodeKind::Group(_)));
        assert_eq!(node.children.len(), 2);
        assert_eq!(
            node.base.name.as_deref(),
            Some("sunflower — Noto Emoji (Google, Apache 2.0)")
        );
        let side = page.size.to_px(document.dpi()).expect("px");
        let shorter = side.width().min(side.height());
        let p = node.base.placement;
        assert!(
            (p.w.max(p.h) - 0.3 * shorter).abs() < 1.0,
            "{} vs {}",
            p.w,
            shorter
        );
        assert_eq!(undo_depth(&mut engine), depth + 1);
    }

    // A font pairing comes in as one selected group of a heading and a
    // body line, in one undo step.
    #[test]
    fn a_font_pairing_is_added_as_a_group() {
        let mut engine = engine();
        start(&mut engine, StartFormat::Social).expect("a new document");
        let mut editor = Editor::default();
        let depth = undo_depth(&mut engine);
        ask(
            &mut engine,
            &mut editor,
            Asked {
                insert_pairing: Some((1, ("Título".to_owned(), "Cuerpo".to_owned()), None)),
                ..Asked::default()
            },
        );
        let document = engine.edit().document().expect("doc");
        let page = &document.pages()[0];
        let (_, group) = editor.selected.single().expect("one selected");
        let node = page.node(group).expect("the group");
        assert!(matches!(node.kind, gafira_doc::NodeKind::Group(_)));
        let children = node.children.clone();
        assert_eq!(children.len(), 2);
        assert!(children.iter().all(|c| matches!(
            page.node(*c).map(|n| &n.kind),
            Some(gafira_doc::NodeKind::Text(_))
        )));
        assert_eq!(undo_depth(&mut engine), depth + 1);
    }

    // FD-44: a curved text from the panel fits its box, either way: no
    // letter outside it.
    #[test]
    fn s70_04_a_curved_text_fits_its_box() {
        use kurbo::Shape as _;
        for bend in [CURVE_PRESET, -CURVE_PRESET] {
            let mut engine = engine();
            start(&mut engine, StartFormat::Social).expect("a new document");
            let mut editor = Editor::default();
            ask(
                &mut engine,
                &mut editor,
                Asked {
                    insert_curved: Some((bend, "Texto curvo".to_owned(), None)),
                    ..Asked::default()
                },
            );
            let (page, node) = editor.selected.single().expect("the curved text");
            let document = engine.edit().document().expect("doc");
            let found = document
                .page(page)
                .and_then(|p| p.node(node))
                .expect("node");
            let gafira_doc::NodeKind::Text(text) = &found.kind else {
                panic!("a text")
            };
            let p = found.base.placement;
            let (path, _, _) =
                gafira_render::text_outline(text, (p.w, p.h), canvas::text_fonts()).expect("drawn");
            let b = path.bounding_box();
            assert!(b.x0 >= -0.5 && b.y0 >= -0.5, "bend {bend}: {b:?}");
            assert!(
                b.x1 <= p.w + 0.5 && b.y1 <= p.h + 0.5,
                "bend {bend}: {b:?} in {}x{}",
                p.w,
                p.h
            );
        }
    }

    // FD-44: «Curva» bends the text and its box takes the height of the arc,
    // in one undo step; a curved text from the panel comes in bent.
    #[test]
    fn s70_04_curving_a_text_fits_its_box() {
        let mut engine = engine();
        start(&mut engine, StartFormat::Social).expect("a new document");
        let mut editor = Editor::default();
        ask(
            &mut engine,
            &mut editor,
            Asked {
                insert_curved: Some((45.0, "Texto curvo".to_owned(), None)),
                ..Asked::default()
            },
        );
        let (page, node) = editor.selected.single().expect("the curved text");
        let read = |engine: &mut Gafira| {
            let document = engine.edit().document().expect("doc");
            let found = document
                .page(page)
                .and_then(|p| p.node(node))
                .expect("node");
            let gafira_doc::NodeKind::Text(text) = &found.kind else {
                panic!("a text")
            };
            (text.curve, found.base.placement.h)
        };
        let (curve, arched) = read(&mut engine);
        assert_eq!(curve, 45.0);
        let depth = undo_depth(&mut engine);
        ask(
            &mut engine,
            &mut editor,
            Asked {
                text_curve: Some(0.0),
                ..Asked::default()
            },
        );
        let (curve, straight) = read(&mut engine);
        assert_eq!(curve, 0.0);
        assert!(straight < arched - 5.0, "{straight} < {arched}");
        assert_eq!(undo_depth(&mut engine), depth + 1);
    }

    // «Usar como fondo de la página»: the image becomes its page's
    // background and leaves the page, in one undo step.
    #[test]
    fn an_image_becomes_the_page_background() {
        let dir = std::env::temp_dir().join(format!("gafira-ui-asbg-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("a folder");
        let png = dir.join("fondo.png");
        image::RgbaImage::from_pixel(64, 32, image::Rgba([10, 20, 30, 255]))
            .save(&png)
            .expect("a png");
        let store = Arc::new(gafira_assets::FsStore::new(dir.join("store")));
        let mut engine = engine_with(Arc::new(Nothing), store);
        start(&mut engine, StartFormat::Social).expect("a new document");
        let asset = engine.assets().import(&png).expect("imports");
        let page = engine.edit().document().expect("doc").pages()[0].id;
        let next = NodeId::new(engine.edit().document().expect("doc").next_id());
        engine
            .edit()
            .execute(Box::new(InsertNodes::new(
                page,
                None,
                0,
                vec![canvas::new_image(
                    Vec2::new(1080.0, 1080.0),
                    (64, 32),
                    asset,
                )],
            )))
            .expect("inserts");
        let mut editor = Editor::default();
        let depth = undo_depth(&mut engine);
        ask(
            &mut engine,
            &mut editor,
            Asked {
                image_to_background: Some((page, next, asset)),
                ..Asked::default()
            },
        );
        let document = engine.edit().document().expect("doc");
        let on = &document.pages()[0];
        assert!(on.node(next).is_none(), "the element is gone");
        assert_eq!(
            on.background,
            Some(gafira_doc::PageBackground::Image {
                asset,
                natural: (64, 32),
                fit: gafira_doc::BackgroundFit::Fill,
            })
        );
        assert_eq!(undo_depth(&mut engine), depth + 1);
        std::fs::remove_dir_all(dir).expect("clean up");
    }

    // S69-05: the background of the current page is set in one undo step,
    // «Aplicar a todas las páginas» copies it to the others, and «Quitar
    // fondo» makes the page white again.
    #[test]
    fn s69_05_a_background_for_one_page_or_all() {
        let mut engine = engine();
        let mut editor = Editor::default();
        let (page, _, _) = two_pieces(&mut engine, &mut editor);
        let size = engine.edit().document().expect("doc").pages()[0].size;
        engine
            .edit()
            .execute(Box::new(InsertPage::new(1, String::new(), size)))
            .expect("a second page");
        editor.active = Some(page);
        let blue = gafira_doc::PageBackground::Solid(crate::colors::hex(0x1E63E9));
        let depth = undo_depth(&mut engine);
        ask(
            &mut engine,
            &mut editor,
            Asked {
                page_background: Some(Some(blue.clone())),
                ..Asked::default()
            },
        );
        let backgrounds = |engine: &mut Gafira| {
            engine
                .edit()
                .document()
                .expect("doc")
                .pages()
                .iter()
                .map(|p| p.background.clone())
                .collect::<Vec<_>>()
        };
        assert_eq!(backgrounds(&mut engine), [Some(blue.clone()), None]);
        assert_eq!(undo_depth(&mut engine), depth + 1);
        ask(
            &mut engine,
            &mut editor,
            Asked {
                background_all: true,
                ..Asked::default()
            },
        );
        assert_eq!(backgrounds(&mut engine), [Some(blue.clone()), Some(blue)]);
        ask(
            &mut engine,
            &mut editor,
            Asked {
                page_background: Some(None),
                ..Asked::default()
            },
        );
        assert_eq!(backgrounds(&mut engine)[0], None, "white again");
    }

    // Spec 066: Suprimir on a frame with a photo empties it and keeps it;
    // on an empty frame it deletes it.
    #[test]
    fn s66_delete_empties_a_full_frame_and_removes_an_empty_one() {
        let dir = std::env::temp_dir().join(format!("gafira-ui-del-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("a folder");
        let png = dir.join("p.png");
        image::RgbaImage::from_pixel(10, 10, image::Rgba([0, 255, 0, 255]))
            .save(&png)
            .expect("a png");
        let store = Arc::new(gafira_assets::FsStore::new(dir.join("store")));
        let mut engine = engine_with(Arc::new(Nothing), store);
        start(&mut engine, StartFormat::Social).expect("a new document");
        let asset = engine.assets().import(&png).expect("imports");
        let mut editor = Editor::default();
        ask(
            &mut engine,
            &mut editor,
            Asked {
                insert_frame: Some(canvas::Piece::Circle),
                ..Asked::default()
            },
        );
        let (page, frame) = editor.selected.single().expect("the frame");
        ask(
            &mut engine,
            &mut editor,
            Asked {
                frame_content: Some(Some(gafira_doc::FrameContent {
                    asset,
                    natural: (10, 10),
                })),
                ..Asked::default()
            },
        );
        let depth = undo_depth(&mut engine);
        let delete = || Asked {
            delete: true,
            ..Asked::default()
        };
        ask(&mut engine, &mut editor, delete());
        let document = engine.edit().document().expect("a document");
        let emptied = frame_of(&document, page, frame).expect("the frame stays");
        assert!(emptied.content.is_none(), "the photo is gone");
        assert_eq!(
            editor.selected.single(),
            Some((page, frame)),
            "still selected"
        );
        assert_eq!(undo_depth(&mut engine), depth + 1);
        ask(&mut engine, &mut editor, delete());
        let document = engine.edit().document().expect("a document");
        assert!(
            document.pages()[0].node(frame).is_none(),
            "the empty frame goes"
        );
        std::fs::remove_dir_all(dir).expect("clean up");
    }

    // S66-06: an image dropped on a frame becomes its photo and leaves the
    // page, in one undo step; S66-07: crop mode moves the photo, every
    // move of the session in one undo step too.
    #[test]
    fn s66_06_s66_07_an_image_into_a_frame_then_cropped() {
        let dir = std::env::temp_dir().join(format!("gafira-ui-frame-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("a folder");
        let png = dir.join("wide.png");
        image::RgbaImage::from_pixel(400, 200, image::Rgba([0, 0, 255, 255]))
            .save(&png)
            .expect("a png");
        let store = Arc::new(gafira_assets::FsStore::new(dir.join("store")));
        let mut engine = engine_with(Arc::new(Nothing), store);
        start(&mut engine, StartFormat::Social).expect("a new document");
        let asset = engine.assets().import(&png).expect("imports");
        let page = engine.edit().document().expect("a document").pages()[0].id;
        let mut editor = Editor::default();
        ask(
            &mut engine,
            &mut editor,
            Asked {
                insert_frame: Some(canvas::Piece::Circle),
                ..Asked::default()
            },
        );
        let (_, frame) = editor.selected.single().expect("the frame");
        let spec = canvas::new_image(Vec2::new(1080.0, 1080.0), (400, 200), asset);
        let next = NodeId::new(engine.edit().document().expect("doc").next_id());
        engine
            .edit()
            .execute(Box::new(InsertNodes::new(page, None, 1, vec![spec])))
            .expect("inserts");
        // The drag of the image, then the drop on the frame.
        engine
            .edit()
            .execute(Box::new(SetProperty::new(
                page,
                vec![next],
                Property::Placement(gafira_doc::Placement::new(500.0, 500.0, 400.0, 200.0)),
                None,
            )))
            .expect("moves");
        let before_drag = undo_depth(&mut engine);
        editor.drop_into_frame(
            &mut engine,
            FrameTarget {
                page,
                frame,
                image: next,
                asset,
            },
        );
        let document = engine.edit().document().expect("a document");
        assert!(
            document.pages()[0].node(next).is_none(),
            "the image is gone"
        );
        let photo = frame_of(&document, page, frame).and_then(|f| f.content);
        assert_eq!(
            photo.map(|c| (c.asset, c.natural)),
            Some((asset, (400, 200)))
        );
        assert_eq!(
            undo_depth(&mut engine),
            before_drag,
            "the move became the drop"
        );
        assert_eq!(editor.selected.single(), Some((page, frame)));

        // Crop mode: two moves, one undo step; the photo stays covering.
        let before_crop = undo_depth(&mut engine);
        ask(
            &mut engine,
            &mut editor,
            Asked {
                begin_crop: Some((page, frame)),
                ..Asked::default()
            },
        );
        editor.zoom = 1.0;
        for _ in 0..2 {
            ask(
                &mut engine,
                &mut editor,
                Asked {
                    crop_drag: Some(Vec2::new(40.0, 0.0)),
                    ..Asked::default()
                },
            );
        }
        ask(
            &mut engine,
            &mut editor,
            Asked {
                end_crop: true,
                ..Asked::default()
            },
        );
        assert!(editor.cropping.is_none());
        let document = engine.edit().document().expect("a document");
        let cropped = frame_of(&document, page, frame).expect("the frame");
        assert!(cropped.content_offset.0 > 0.0, "moved right");
        let w = document.pages()[0]
            .node(frame)
            .expect("node")
            .base
            .placement
            .w;
        assert!(
            cropped.content_offset.0 <= 0.5 + 1e-9,
            "never uncovers the frame: {w}"
        );
        assert_eq!(undo_depth(&mut engine), before_crop + 1, "one step");
        engine.edit().undo().expect("an undo");
        let back = frame_of(&engine.edit().document().expect("doc"), page, frame).expect("frame");
        assert_eq!(back.content_offset, (0.0, 0.0));
        std::fs::remove_dir_all(dir).expect("clean up");
    }

    /// One frame of the whole editor in a 1440 × 900 window, with Ctrl/Cmd
    /// and `key` pressed when given.
    fn frame(ctx: &egui::Context, editor: &mut Editor, engine: &mut Gafira, key: Option<Key>) {
        frame_with(ctx, editor, engine, key, Modifiers::COMMAND);
    }

    /// [`frame`] with `modifiers` held instead of Ctrl/Cmd.
    fn frame_with(
        ctx: &egui::Context,
        editor: &mut Editor,
        engine: &mut Gafira,
        key: Option<Key>,
        modifiers: Modifiers,
    ) {
        let mut input = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1440.0, 900.0))),
            ..Default::default()
        };
        if let Some(key) = key {
            input.modifiers = modifiers;
            input.events.push(egui::Event::Key {
                key,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers,
            });
        }
        let (theme, texts) = (theme(), Texts::new(gafira_ports::i18n::Locale::Es));
        let _ = ctx.run(input, |ctx| {
            editor.show(ctx, engine, &theme, &texts, None, &mut None);
        });
    }

    // S37-07: the shortcuts of 03 §9.2 move the canvas zoom.
    #[test]
    fn s37_07_ctrl_1_ctrl_equals_ctrl_minus_and_ctrl_0_drive_the_canvas_zoom() {
        let mut engine = engine();
        start(&mut engine, StartFormat::Social).expect("a new document");
        let ctx = egui::Context::default();
        let mut editor = Editor::default();
        frame(&ctx, &mut editor, &mut engine, None);
        assert_eq!(editor.view, Zoom::Fit);
        let fit = editor.zoom;
        assert!(
            fit > 0.05 && fit < 1.0,
            "a 1080 px page fits below 100 %: {fit}"
        );

        frame(&ctx, &mut editor, &mut engine, Some(Key::Num1));
        assert_eq!(editor.view, Zoom::Fixed(1.0));
        assert_eq!(editor.zoom, 1.0);
        frame(&ctx, &mut editor, &mut engine, Some(Key::Equals));
        assert_eq!(editor.view, Zoom::Fixed(1.25));
        frame(&ctx, &mut editor, &mut engine, Some(Key::Minus));
        assert_eq!(editor.view, Zoom::Fixed(1.0));
        frame(&ctx, &mut editor, &mut engine, Some(Key::Num0));
        assert_eq!(editor.view, Zoom::Fit);
        assert_eq!(editor.zoom, fit);
    }

    /// A Social document with a rectangle and an ellipse, the ellipse moved
    /// 400 px right of the rectangle; `editor` has the ellipse selected.
    fn two_pieces(engine: &mut Gafira, editor: &mut Editor) -> (PageId, NodeId, NodeId) {
        start(engine, StartFormat::Social).expect("a new document");
        let theme = theme();
        for geometry in [ShapeGeometry::Rect, ShapeGeometry::Ellipse] {
            let snapshot = engine.edit().document();
            editor.apply(
                engine,
                &snapshot,
                &Asked {
                    insert: Some(match geometry {
                        ShapeGeometry::Ellipse => canvas::Piece::Circle,
                        _ => canvas::Piece::Square,
                    }),
                    ..Asked::default()
                },
                &theme,
            );
        }
        let shapes = the_shapes(engine);
        let mut aside = shapes[1].placement;
        aside.x += 400.0;
        let snapshot = engine.edit().document();
        editor.apply(
            engine,
            &snapshot,
            &Asked {
                place: Some(aside),
                ..Asked::default()
            },
            &theme,
        );
        (shapes[0].page, shapes[0].node, shapes[1].node)
    }

    /// `asked` applied by `editor` over the newest document.
    fn ask(engine: &mut Gafira, editor: &mut Editor, asked: Asked) -> Go {
        let snapshot = engine.edit().document();
        editor.apply(engine, &snapshot, &asked, &theme())
    }

    /// How many entries the history can undo.
    fn undo_depth(engine: &mut Gafira) -> usize {
        let mut depth = 0;
        while engine.edit().can_undo_redo().0 {
            engine.edit().undo().expect("an undo");
            depth += 1;
        }
        for _ in 0..depth {
            engine.edit().redo().expect("a redo");
        }
        depth
    }

    // S40-05: a drag of two selected elements moves both in one undo step.
    #[test]
    fn s40_05_a_drag_of_a_multi_selection_moves_all_in_one_undo() {
        let mut engine = engine();
        let mut editor = Editor::default();
        let (page, rect, ellipse) = two_pieces(&mut engine, &mut editor);
        let before = the_shapes(&mut engine);
        let depth = undo_depth(&mut engine);
        ask(
            &mut engine,
            &mut editor,
            Asked {
                select: Some(Selection::of(page, [rect, ellipse])),
                ..Asked::default()
            },
        );
        editor.zoom = 2.0;
        // A handle held with two selected moves instead of resizing.
        let corner = canvas::Handle { x: 1, y: 1 };
        for asked in [
            Asked {
                drag_begin: Some(Some(Hold::Resize(corner))),
                ..Asked::default()
            },
            Asked {
                drag_to: Some(Vec2::new(10.0, 4.0)),
                ..Asked::default()
            },
            Asked {
                drag_to: Some(Vec2::new(10.0, 4.0)),
                ..Asked::default()
            },
            Asked {
                drag_to: Some(Vec2::new(10.0, 4.0)),
                ..Asked::default()
            },
            Asked {
                drag_end: true,
                ..Asked::default()
            },
        ] {
            ask(&mut engine, &mut editor, asked);
        }
        let after = the_shapes(&mut engine);
        for (moved, origin) in after.iter().zip(&before) {
            assert_eq!(
                (moved.placement.x - origin.placement.x),
                15.0,
                "{:?}",
                moved.node
            );
            assert_eq!(moved.placement.y - origin.placement.y, 6.0);
            assert_eq!(
                (moved.placement.w, moved.placement.h),
                (origin.placement.w, origin.placement.h),
                "moved, not resized"
            );
        }
        assert_eq!(undo_depth(&mut engine), depth + 1, "one entry for the drag");
        engine.edit().undo().expect("an undo");
        assert_eq!(the_shapes(&mut engine), before, "one undo takes both back");
    }

    // S40-06: Delete removes every selected element in one undo step.
    #[test]
    fn s40_06_delete_removes_the_whole_selection_in_one_undo() {
        let mut engine = engine();
        let mut editor = Editor::default();
        let (page, rect, ellipse) = two_pieces(&mut engine, &mut editor);
        ask(
            &mut engine,
            &mut editor,
            Asked {
                insert: Some(canvas::Piece::Square),
                ..Asked::default()
            },
        );
        let kept = the_shapes(&mut engine)[2].node;
        let depth = undo_depth(&mut engine);
        ask(
            &mut engine,
            &mut editor,
            Asked {
                select: Some(Selection::of(page, [rect, ellipse])),
                ..Asked::default()
            },
        );
        ask(
            &mut engine,
            &mut editor,
            Asked {
                delete: true,
                ..Asked::default()
            },
        );
        let left: Vec<NodeId> = the_shapes(&mut engine).iter().map(|s| s.node).collect();
        assert_eq!(left, [kept]);
        assert!(editor.selected.is_empty());
        assert_eq!(undo_depth(&mut engine), depth + 1);
        engine.edit().undo().expect("an undo");
        assert_eq!(
            the_shapes(&mut engine).len(),
            3,
            "one undo brings both back"
        );
    }

    // S40-07: an undo that takes selected elements away prunes them.
    #[test]
    fn s40_07_an_undo_prunes_what_it_took_away() {
        let mut engine = engine();
        let mut editor = Editor::default();
        let (page, rect, ellipse) = two_pieces(&mut engine, &mut editor);
        editor.selected = Selection::of(page, [rect, ellipse]);
        // Undo the move aside, then the ellipse's insert.
        engine.edit().undo().expect("an undo");
        engine.edit().undo().expect("an undo");
        let kept = editor.prune(engine.edit().document().as_deref());
        assert_eq!(editor.selected, Selection::one(page, rect));
        assert_eq!(kept.iter().map(|s| s.node).collect::<Vec<_>>(), [rect]);
        engine.edit().undo().expect("an undo");
        assert!(editor.prune(engine.edit().document().as_deref()).is_empty());
        assert_eq!(editor.selected, Selection::default());
    }

    // S40-08: fill and opacity from the Inspector go to every selected
    // element, one undo step each.
    #[test]
    fn s40_08_fill_and_opacity_apply_to_the_whole_selection_in_one_undo() {
        let mut engine = engine();
        let mut editor = Editor::default();
        let (page, rect, ellipse) = two_pieces(&mut engine, &mut editor);
        ask(
            &mut engine,
            &mut editor,
            Asked {
                select: Some(Selection::of(page, [rect, ellipse])),
                ..Asked::default()
            },
        );
        let depth = undo_depth(&mut engine);
        ask(
            &mut engine,
            &mut editor,
            Asked {
                fill: Some(None),
                opacity: Some(0.5),
                ..Asked::default()
            },
        );
        for shape in the_shapes(&mut engine) {
            assert_eq!(shape.fill, None);
            assert_eq!(shape.opacity, 0.5);
        }
        assert_eq!(undo_depth(&mut engine), depth + 2, "fill, then opacity");
        engine.edit().undo().expect("an undo");
        for shape in the_shapes(&mut engine) {
            assert_eq!(shape.opacity, 1.0, "one undo restores both");
            assert!(shape.fill.is_none());
        }
        engine.edit().undo().expect("an undo");
        assert!(the_shapes(&mut engine).iter().all(|s| s.fill.is_some()));
    }

    // S40-02: a marquee takes the shapes on the page where it started (or
    // the first page it touches); Shift adds them; touching nothing clears
    // without Shift and keeps the selection with it.
    #[test]
    fn s40_02_a_marquee_selects_on_one_page_and_shift_adds() {
        let mut engine = engine();
        let mut editor = Editor::default();
        let (page, rect, ellipse) = two_pieces(&mut engine, &mut editor);
        let shapes = the_shapes(&mut engine);
        let first = Rect::from_min_size(Pos2::ZERO, Vec2::splat(1080.0));
        let second = Rect::from_min_size(Pos2::new(0.0, 1200.0), Vec2::splat(1080.0));
        let laid = vec![
            (page, first, shapes.clone()),
            (PageId::new(999), second, Vec::new()),
        ];
        let rect_at = canvas::on_screen(first, 1.0, &shapes[0].placement);
        let marquee = |from: Pos2, to: Pos2, add: bool| Marquee { from, to, add };
        let current = Selection::one(page, ellipse);

        let inside = marquee(rect_at.min - Vec2::splat(5.0), rect_at.center(), false);
        assert_eq!(
            marquee_selection(&current, inside, &laid, 1.0, |_, leaf| Some(leaf)),
            Selection::one(page, rect)
        );
        let added = Marquee {
            add: true,
            ..inside
        };
        assert_eq!(
            marquee_selection(&current, added, &laid, 1.0, |_, leaf| Some(leaf)),
            Selection::of(page, [ellipse, rect])
        );
        // Started above the pages, it still selects on the page it touches.
        let from_outside = marquee(Pos2::new(rect_at.min.x, -40.0), rect_at.center(), false);
        assert_eq!(
            marquee_selection(&current, from_outside, &laid, 1.0, |_, leaf| Some(leaf)),
            Selection::one(page, rect)
        );
        let empty = marquee(Pos2::new(5.0, 1210.0), Pos2::new(50.0, 1250.0), false);
        assert_eq!(
            marquee_selection(&current, empty, &laid, 1.0, |_, leaf| Some(leaf)),
            Selection::default()
        );
        let empty_added = Marquee { add: true, ..empty };
        assert_eq!(
            marquee_selection(&current, empty_added, &laid, 1.0, |_, leaf| Some(leaf)),
            current
        );
    }

    // S40-09: Ctrl+A selects the drawn roots of the page and Esc clears.
    #[test]
    fn s40_09_ctrl_a_selects_everything_and_escape_clears() {
        let mut engine = engine();
        let mut editor = Editor::default();
        let (page, rect, ellipse) = two_pieces(&mut engine, &mut editor);
        let ctx = egui::Context::default();
        frame(&ctx, &mut editor, &mut engine, Some(Key::A));
        assert_eq!(editor.selected, Selection::of(page, [rect, ellipse]));
        frame(&ctx, &mut editor, &mut engine, None);
        assert_eq!(editor.selected.len(), 2, "kept across frames");
        frame(&ctx, &mut editor, &mut engine, Some(Key::Escape));
        assert_eq!(editor.selected, Selection::default());
    }

    /// `asked` one after the other through `editor`.
    fn gesture(engine: &mut Gafira, editor: &mut Editor, steps: Vec<Asked>) {
        for asked in steps {
            ask(engine, editor, asked);
        }
    }

    // S41-15: a drag of the rotate handle turns the element about its
    // centre in one undo step; Shift snaps to 15°.
    #[test]
    fn s41_15_the_rotate_handle_turns_in_one_undo_and_shift_snaps() {
        let mut engine = engine();
        let mut editor = Editor::default();
        let (page, rect, _) = two_pieces(&mut engine, &mut editor);
        editor.selected = Selection::one(page, rect);
        let before = the_shapes(&mut engine)[0].placement;
        let depth = undo_depth(&mut engine);
        let center = Pos2::new(100.0, 100.0);
        let hold = Hold::Rotate {
            center,
            from: Pos2::new(100.0, 0.0),
        };
        // From straight up to straight right in three steps: a quarter turn.
        let mut steps = vec![Asked {
            drag_begin: Some(Some(hold)),
            ..Asked::default()
        }];
        for delta in [
            Vec2::new(40.0, 20.0),
            Vec2::new(40.0, 40.0),
            Vec2::new(20.0, 40.0),
        ] {
            steps.push(Asked {
                drag_to: Some(delta),
                ..Asked::default()
            });
        }
        steps.push(Asked {
            drag_end: true,
            ..Asked::default()
        });
        gesture(&mut engine, &mut editor, steps);
        let turned = the_shapes(&mut engine)[0].placement;
        assert!((turned.rotation_deg - 90.0).abs() < 1e-9, "{turned:?}");
        assert_eq!(
            (turned.x, turned.y, turned.w, turned.h),
            (before.x, before.y, before.w, before.h),
            "turned about its centre"
        );
        assert_eq!(undo_depth(&mut engine), depth + 1, "one entry for the turn");
        engine.edit().undo().expect("an undo");
        assert_eq!(the_shapes(&mut engine)[0].placement, before);

        // With Shift a 35° drag snaps to 30°.
        let a = (35.0f32 - 90.0).to_radians();
        let to = Pos2::new(100.0 + 100.0 * a.cos(), 100.0 + 100.0 * a.sin());
        gesture(
            &mut engine,
            &mut editor,
            vec![
                Asked {
                    drag_begin: Some(Some(hold)),
                    ..Asked::default()
                },
                Asked {
                    drag_to: Some(to - Pos2::new(100.0, 0.0)),
                    snap: true,
                    ..Asked::default()
                },
                Asked {
                    drag_end: true,
                    ..Asked::default()
                },
            ],
        );
        assert_eq!(the_shapes(&mut engine)[0].placement.rotation_deg, 30.0);
    }

    // S41-16: Edición's turns and flips go to the whole selection, each
    // element about its own centre, one undo step per action; two actions
    // in a row are two steps.
    #[test]
    fn s41_16_turn_and_flip_apply_to_the_selection_one_undo_each() {
        let mut engine = engine();
        let mut editor = Editor::default();
        let (page, rect, ellipse) = two_pieces(&mut engine, &mut editor);
        editor.selected = Selection::of(page, [rect, ellipse]);
        let before = the_shapes(&mut engine);
        let depth = undo_depth(&mut engine);
        let turn = |degrees| Asked {
            turn: Some(degrees),
            ..Asked::default()
        };
        ask(&mut engine, &mut editor, turn(90.0));
        ask(&mut engine, &mut editor, turn(90.0));
        ask(
            &mut engine,
            &mut editor,
            Asked {
                flip: Some(Axis::Horizontal),
                ..Asked::default()
            },
        );
        let after = the_shapes(&mut engine);
        for (shape, origin) in after.iter().zip(&before) {
            let (p, o) = (shape.placement, origin.placement);
            assert_eq!(
                (p.x, p.y, p.w, p.h),
                (o.x, o.y, o.w, o.h),
                "{:?}",
                shape.node
            );
            // 180° flipped horizontally is 180° again, mirrored.
            assert_eq!(p.rotation_deg, 180.0);
            assert!(p.flip.x && !p.flip.y);
        }
        assert_eq!(
            undo_depth(&mut engine),
            depth + 3,
            "three actions, three steps"
        );
        engine.edit().undo().expect("an undo");
        for shape in the_shapes(&mut engine) {
            assert_eq!(shape.placement.rotation_deg, 180.0);
            assert!(!shape.placement.flip.x, "the flip undone on both");
        }
        ask(&mut engine, &mut editor, turn(-90.0));
        assert!(
            the_shapes(&mut engine)
                .iter()
                .all(|s| s.placement.rotation_deg == 90.0)
        );
    }

    // S41-17: Mayús+H and Mayús+V flip the selection; with Ctrl they do not.
    #[test]
    fn s41_17_shift_h_and_shift_v_flip_the_selection() {
        let mut engine = engine();
        let mut editor = Editor::default();
        let (page, rect, _) = two_pieces(&mut engine, &mut editor);
        editor.selected = Selection::one(page, rect);
        let ctx = egui::Context::default();
        frame_with(
            &ctx,
            &mut editor,
            &mut engine,
            Some(Key::H),
            Modifiers::SHIFT,
        );
        assert!(the_shapes(&mut engine)[0].placement.flip.x);
        frame_with(
            &ctx,
            &mut editor,
            &mut engine,
            Some(Key::V),
            Modifiers::SHIFT,
        );
        assert!(the_shapes(&mut engine)[0].placement.flip.y);
        let command_shift = Modifiers::COMMAND.plus(Modifiers::SHIFT);
        let ellipse = the_shapes(&mut engine)[1].placement;
        assert!(!ellipse.flip.x && !ellipse.flip.y, "only the selection");
        // Since spec 048 Ctrl+Mayús+H hides the selection; it still does
        // not flip it.
        frame_with(&ctx, &mut editor, &mut engine, Some(Key::H), command_shift);
        assert!(
            placements(&mut engine, &[rect])[0].flip.x,
            "Ctrl+Mayús+H is not a flip"
        );
    }

    // S41-18: resizing a turned element by its handle works in its own axes
    // in one undo step.
    #[test]
    fn s41_18_a_turned_element_resizes_in_its_own_axes_in_one_undo() {
        let mut engine = engine();
        let mut editor = Editor::default();
        let (page, rect, _) = two_pieces(&mut engine, &mut editor);
        editor.selected = Selection::one(page, rect);
        ask(
            &mut engine,
            &mut editor,
            Asked {
                turn: Some(90.0),
                ..Asked::default()
            },
        );
        let before = the_shapes(&mut engine)[0].placement;
        let depth = undo_depth(&mut engine);
        editor.zoom = 1.0;
        // The local right edge faces down after a quarter turn.
        let right = canvas::Handle { x: 1, y: 0 };
        let mut steps = vec![Asked {
            drag_begin: Some(Some(Hold::Resize(right))),
            ..Asked::default()
        }];
        for _ in 0..2 {
            steps.push(Asked {
                drag_to: Some(Vec2::new(0.0, 10.0)),
                ..Asked::default()
            });
        }
        steps.push(Asked {
            drag_end: true,
            ..Asked::default()
        });
        gesture(&mut engine, &mut editor, steps);
        let after = the_shapes(&mut engine)[0].placement;
        assert!((after.w - before.w - 20.0).abs() < 1e-9, "{after:?}");
        assert_eq!(after.h, before.h);
        assert_eq!(after.rotation_deg, 90.0);
        assert_eq!(undo_depth(&mut engine), depth + 1);
    }

    // Owner rule: a corner of a group scales every letter below it by one
    // factor, so a large and a small one keep their proportion; one undo
    // step.
    #[test]
    fn a_group_corner_scales_its_letters_in_proportion() {
        let mut engine = engine();
        start(&mut engine, StartFormat::Social).expect("a new document");
        let mut editor = Editor::default();
        ask(
            &mut engine,
            &mut editor,
            Asked {
                insert_pairing: Some((0, ("Título".to_owned(), "Cuerpo".to_owned()), None)),
                ..Asked::default()
            },
        );
        let (_, group) = editor.selected.single().expect("the group");
        let read = |engine: &mut Gafira| {
            let document = engine.edit().document().expect("doc");
            let page = &document.pages()[0];
            let node = page.node(group).expect("group");
            let sizes: Vec<f64> = node
                .children
                .iter()
                .map(|c| match &page.node(*c).expect("child").kind {
                    gafira_doc::NodeKind::Text(t) => t.content.first_style().expect("style").size,
                    _ => panic!("a text"),
                })
                .collect();
            (node.base.placement, sizes)
        };
        let (before, sizes) = read(&mut engine);
        editor.zoom = 1.0;
        let depth = undo_depth(&mut engine);
        let corner = canvas::Handle { x: 1, y: 1 };
        let steps = vec![
            Asked {
                drag_begin: Some(Some(Hold::Resize(corner))),
                ..Asked::default()
            },
            Asked {
                drag_to: Some(Vec2::new(before.w as f32, 0.0)),
                ..Asked::default()
            },
            Asked {
                drag_end: true,
                ..Asked::default()
            },
        ];
        gesture(&mut engine, &mut editor, steps);
        let (after, bigger) = read(&mut engine);
        let k = after.w / before.w;
        assert!(k > 1.5, "the group grew: {k}");
        for (a, b) in sizes.iter().zip(&bigger) {
            assert!((b / a - k).abs() < 1e-6, "{a} -> {b}, k {k}");
        }
        assert!((bigger[0] / bigger[1] - sizes[0] / sizes[1]).abs() < 1e-9);
        assert_eq!(undo_depth(&mut engine), depth + 1);
    }

    // Owner rule: a corner of a text box scales the box and its letters
    // together, in one undo step; a side keeps room for one letter.
    #[test]
    fn a_text_box_corner_scales_its_letters() {
        let mut engine = engine();
        let mut editor = Editor::default();
        let (page, _, _) = two_pieces(&mut engine, &mut editor);
        editor.active = Some(page);
        ask(
            &mut engine,
            &mut editor,
            Asked {
                insert_text: Some((canvas::TextPreset::Body, "Hola".to_owned())),
                ..Asked::default()
            },
        );
        let (_, text) = editor.selected.single().expect("the text");
        let read = |engine: &mut Gafira| {
            let document = engine.edit().document().expect("doc");
            let node = document.pages()[0].node(text).expect("node").clone();
            let gafira_doc::NodeKind::Text(t) = &node.kind else {
                panic!("a text")
            };
            (
                node.base.placement,
                t.content.spans().next().expect("a span").style.size,
            )
        };
        let (before, size) = read(&mut engine);
        editor.zoom = 1.0;
        let depth = undo_depth(&mut engine);
        let corner = canvas::Handle { x: 1, y: 1 };
        let steps = vec![
            Asked {
                drag_begin: Some(Some(Hold::Resize(corner))),
                ..Asked::default()
            },
            Asked {
                drag_to: Some(Vec2::new(before.w as f32, 0.0)),
                ..Asked::default()
            },
            Asked {
                drag_end: true,
                ..Asked::default()
            },
        ];
        gesture(&mut engine, &mut editor, steps);
        let (after, bigger) = read(&mut engine);
        let k = after.w / before.w;
        assert!(k > 1.5, "the box grew: {k}");
        assert!((bigger / size - k).abs() < 1e-6, "the letters grew as much");
        assert_eq!(undo_depth(&mut engine), depth + 1);
        // A side never goes narrower than one letter.
        let side = canvas::Handle { x: 1, y: 0 };
        let steps = vec![
            Asked {
                drag_begin: Some(Some(Hold::Resize(side))),
                ..Asked::default()
            },
            Asked {
                drag_to: Some(Vec2::new(-10_000.0, 0.0)),
                ..Asked::default()
            },
            Asked {
                drag_end: true,
                ..Asked::default()
            },
        ];
        gesture(&mut engine, &mut editor, steps);
        let (narrow, letter) = read(&mut engine);
        assert!(
            narrow.w >= letter * 1.2 - 1e-9,
            "{} < {}",
            narrow.w,
            letter * 1.2
        );
    }

    // S41-14: a flipped text mirrors its glyphs about the box centre, with
    // the texture coordinates of the font atlas in [0, 1].
    #[test]
    fn s41_14_a_flipped_text_mirrors_its_glyphs() {
        let ctx = egui::Context::default();
        let theme = theme();
        theme.apply(&ctx);
        // The first pass loads the fonts.
        let _ = ctx.run(egui::RawInput::default(), |_| {});
        let style = gafira_doc::SpanStyle {
            font: theme.tokens.typography.ui.clone(),
            size: 20.0,
            color: canvas::document_color(Color32::BLACK),
            bold: false,
            italic: false,
        };
        let text = gafira_doc::TextNode::plain("Hola", &style);
        let page = Rect::from_min_size(Pos2::ZERO, Vec2::splat(400.0));
        let upright = Placement::new(100.0, 100.0, 200.0, 50.0);
        let mut mirrored = upright;
        mirrored.flip.x = true;
        let input = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::splat(400.0))),
            ..Default::default()
        };
        let _ = ctx.run(input, |ctx| {
            let painter = ctx.layer_painter(egui::LayerId::background());
            let at = canvas::on_screen(page, 1.0, &upright);
            let laid = lay_text(&painter, &theme, at, 1.0, 1.0, &text);
            let atlas = ctx.fonts(|f| f.font_image_size());
            let plain = text_mesh(
                &laid,
                at,
                1.0,
                rotation::to_screen(page, 1.0, &upright),
                atlas,
            );
            let flipped = text_mesh(
                &laid,
                at,
                1.0,
                rotation::to_screen(page, 1.0, &mirrored),
                atlas,
            );
            assert!(!plain.vertices.is_empty());
            assert_eq!(plain.indices, flipped.indices);
            for (a, b) in plain.vertices.iter().zip(&flipped.vertices) {
                assert!((a.pos.x - (400.0 - b.pos.x)).abs() < 1e-3, "{a:?} {b:?}");
                assert!((a.pos.y - b.pos.y).abs() < 1e-3);
                assert_eq!(a.uv, b.uv);
                assert!((0.0..=1.0).contains(&a.uv.x) && (0.0..=1.0).contains(&a.uv.y));
            }
            // Upright, the mesh is where the galleys are drawn.
            let (pos, galley, _) = &laid[0];
            let first = galley.rows[0].visuals.mesh.vertices[0].pos;
            let want = *pos + galley.rows[0].pos.to_vec2() + first.to_vec2();
            assert!((plain.vertices[0].pos - want).length() < 1e-3);
        });
    }

    // S41: a frame draws turned and flipped shapes, text and a missing
    // image with their handles without failing.
    #[test]
    fn s41_a_frame_draws_turned_elements() {
        let mut engine = engine();
        let mut editor = Editor::default();
        let (page, rect, ellipse) = two_pieces(&mut engine, &mut editor);
        ask(
            &mut engine,
            &mut editor,
            Asked {
                insert_text: Some((canvas::TextPreset::Title, "Hola".to_owned())),
                ..Asked::default()
            },
        );
        let all: Vec<NodeId> = the_shapes(&mut engine).iter().map(|s| s.node).collect();
        editor.selected = Selection::of(page, all);
        ask(
            &mut engine,
            &mut editor,
            Asked {
                turn: Some(90.0),
                flip: Some(Axis::Vertical),
                ..Asked::default()
            },
        );
        let ctx = egui::Context::default();
        theme().apply(&ctx);
        frame(&ctx, &mut editor, &mut engine, None);
        editor.selected = Selection::one(page, rect);
        frame(&ctx, &mut editor, &mut engine, None);
        let shapes = the_shapes(&mut engine);
        assert!(shapes.iter().all(|s| s.placement.flip.y));
        assert!(shapes.iter().all(|s| s.placement.rotation_deg == 270.0));
        assert!(shapes.iter().any(|s| s.node == ellipse));
    }

    /// [`two_pieces`] and a third rectangle 200 px down and 100 px right of
    /// the first one, with all three selected.
    fn three_pieces(engine: &mut Gafira, editor: &mut Editor) -> (PageId, [NodeId; 3]) {
        let (page, rect, ellipse) = two_pieces(engine, editor);
        ask(
            engine,
            editor,
            Asked {
                insert: Some(canvas::Piece::Square),
                ..Asked::default()
            },
        );
        let third = the_shapes(engine)[2].clone();
        let mut aside = third.placement;
        aside.x += 100.0;
        aside.y += 200.0;
        ask(
            engine,
            editor,
            Asked {
                place: Some(aside),
                ..Asked::default()
            },
        );
        let nodes = [rect, ellipse, third.node];
        editor.selected = Selection::of(page, nodes);
        (page, nodes)
    }

    /// The roots of the first page, back first.
    fn roots_now(engine: &mut Gafira) -> Vec<NodeId> {
        engine.edit().document().expect("a document").pages()[0]
            .roots
            .clone()
    }

    // S42-11: Align moves the whole selection in one undo step, two aligns
    // in a row are two steps, and one element aligns to its page.
    #[test]
    fn s42_11_align_is_one_undo_step_per_action() {
        let mut engine = engine();
        let mut editor = Editor::default();
        let (page, nodes) = three_pieces(&mut engine, &mut editor);
        let before = the_shapes(&mut engine);
        let left = before
            .iter()
            .map(|s| s.placement.x)
            .fold(f64::INFINITY, f64::min);
        let depth = undo_depth(&mut engine);
        let align = |action| Asked {
            align: Some(action),
            ..Asked::default()
        };
        ask(&mut engine, &mut editor, align(AlignAction::Left));
        let after = the_shapes(&mut engine);
        for (shape, origin) in after.iter().zip(&before) {
            assert_eq!(shape.placement.x, left, "{:?}", shape.node);
            assert_eq!(shape.placement.y, origin.placement.y);
        }
        ask(&mut engine, &mut editor, align(AlignAction::Bottom));
        assert_eq!(undo_depth(&mut engine), depth + 2, "two actions, two steps");
        engine.edit().undo().expect("an undo");
        assert_eq!(the_shapes(&mut engine), after, "the bottom align undone");
        engine.edit().undo().expect("an undo");
        assert_eq!(
            the_shapes(&mut engine),
            before,
            "one undo restores all three"
        );

        // Alone, the ellipse goes to the right edge of the 1080 px page.
        editor.selected = Selection::one(page, nodes[1]);
        ask(&mut engine, &mut editor, align(AlignAction::Right));
        let ellipse = the_shapes(&mut engine)[1].placement;
        assert_eq!(ellipse.x + ellipse.w, 1080.0);
        // Already there: nothing to do adds no step.
        let depth = undo_depth(&mut engine);
        ask(&mut engine, &mut editor, align(AlignAction::Right));
        assert_eq!(undo_depth(&mut engine), depth);
    }

    // S42-12: Distribute spreads three elements with equal gaps in one undo
    // step.
    #[test]
    fn s42_12_distribute_is_one_undo_step() {
        let mut engine = engine();
        let mut editor = Editor::default();
        let (_, _) = three_pieces(&mut engine, &mut editor);
        let before = the_shapes(&mut engine);
        let depth = undo_depth(&mut engine);
        ask(
            &mut engine,
            &mut editor,
            Asked {
                distribute: Some(DistributeAxis::Horizontal),
                ..Asked::default()
            },
        );
        let mut boxes: Vec<Placement> = the_shapes(&mut engine)
            .iter()
            .map(|s| s.placement)
            .collect();
        boxes.sort_by(|a, b| a.x.total_cmp(&b.x));
        let gaps: Vec<f64> = boxes
            .windows(2)
            .map(|w| w[1].x - (w[0].x + w[0].w))
            .collect();
        assert!((gaps[0] - gaps[1]).abs() < 0.01, "{gaps:?}");
        assert_ne!(the_shapes(&mut engine), before, "the middle one moved");
        assert_eq!(undo_depth(&mut engine), depth + 1);
        engine.edit().undo().expect("an undo");
        assert_eq!(the_shapes(&mut engine), before);
    }

    // S42-13: the layer order moves the whole selection in one undo step
    // and keeps its relative order.
    #[test]
    fn s42_13_layer_order_is_one_undo_step() {
        let mut engine = engine();
        let mut editor = Editor::default();
        let (page, [a, b, c]) = three_pieces(&mut engine, &mut editor);
        assert_eq!(roots_now(&mut engine), [a, b, c]);
        let depth = undo_depth(&mut engine);
        let order = |order| Asked {
            order: Some(order),
            ..Asked::default()
        };
        editor.selected = Selection::of(page, [b, a]);
        ask(&mut engine, &mut editor, order(Order::Front));
        assert_eq!(roots_now(&mut engine), [c, a, b], "a stays below b");
        assert_eq!(undo_depth(&mut engine), depth + 1);
        engine.edit().undo().expect("an undo");
        assert_eq!(roots_now(&mut engine), [a, b, c]);
        editor.selected = Selection::of(page, [c, b]);
        ask(&mut engine, &mut editor, order(Order::Back));
        assert_eq!(roots_now(&mut engine), [b, c, a], "b stays below c");
        assert_eq!(undo_depth(&mut engine), depth + 1, "two moves, one step");
        engine.edit().undo().expect("an undo");
        assert_eq!(
            roots_now(&mut engine),
            [a, b, c],
            "one undo takes both back"
        );

        editor.selected = Selection::of(page, [a, c]);
        ask(&mut engine, &mut editor, order(Order::Backward));
        assert_eq!(roots_now(&mut engine), [a, c, b]);
        ask(&mut engine, &mut editor, order(Order::Back));
        assert_eq!(roots_now(&mut engine), [a, c, b], "already at the back");
        ask(&mut engine, &mut editor, order(Order::Forward));
        assert_eq!(roots_now(&mut engine), [b, a, c]);
        assert_eq!(undo_depth(&mut engine), depth + 2, "no step for no change");
    }

    // S42-14: Alt+D/H/A align (D-88 replaced Ctrl+Alt+R/C/L, S44-01),
    // Ctrl+] and Ctrl+[ move one step, with Shift to the front or the back;
    // without Ctrl they do nothing.
    #[test]
    fn s42_14_shortcuts_align_and_order() {
        let mut engine = engine();
        let mut editor = Editor::default();
        let (page, [a, b, c]) = three_pieces(&mut engine, &mut editor);
        let ctx = egui::Context::default();
        let command_alt = Modifiers::COMMAND.plus(Modifiers::ALT);
        let command_shift = Modifiers::COMMAND.plus(Modifiers::SHIFT);
        let right = |s: &Shape| s.placement.x + s.placement.w;
        let rightmost = the_shapes(&mut engine)
            .iter()
            .map(right)
            .fold(f64::NEG_INFINITY, f64::max);
        frame_with(&ctx, &mut editor, &mut engine, Some(Key::D), Modifiers::ALT);
        assert!(
            the_shapes(&mut engine)
                .iter()
                .all(|s| right(s) == rightmost)
        );
        frame_with(&ctx, &mut editor, &mut engine, Some(Key::H), Modifiers::ALT);
        frame_with(&ctx, &mut editor, &mut engine, Some(Key::A), Modifiers::ALT);
        let shapes = the_shapes(&mut engine);
        assert!(
            shapes
                .iter()
                .all(|s| s.placement.x == shapes[0].placement.x)
        );

        editor.selected = Selection::one(page, a);
        let key = |editor: &mut Editor, engine: &mut Gafira, key, modifiers| {
            frame_with(&ctx, editor, engine, Some(key), modifiers);
            roots_now(engine)
        };
        assert_eq!(
            key(
                &mut editor,
                &mut engine,
                Key::CloseBracket,
                Modifiers::COMMAND
            ),
            [b, a, c]
        );
        assert_eq!(
            key(&mut editor, &mut engine, Key::CloseBracket, command_shift),
            [b, c, a]
        );
        assert_eq!(
            key(
                &mut editor,
                &mut engine,
                Key::OpenBracket,
                Modifiers::COMMAND
            ),
            [b, a, c]
        );
        assert_eq!(
            key(&mut editor, &mut engine, Key::OpenBracket, command_shift),
            [a, b, c]
        );
        // Shift+] reads as `}` on many layouts.
        assert_eq!(
            key(
                &mut editor,
                &mut engine,
                Key::CloseCurlyBracket,
                command_shift
            ),
            [b, c, a]
        );
        // With AltGr (Ctrl+Alt on Windows) for the bracket, as on Spanish
        // layouts.
        assert_eq!(
            key(&mut editor, &mut engine, Key::OpenBracket, command_alt),
            [b, a, c]
        );
        assert_eq!(
            key(
                &mut editor,
                &mut engine,
                Key::CloseBracket,
                Modifiers::COMMAND
            ),
            [b, c, a]
        );
        assert_eq!(
            key(&mut editor, &mut engine, Key::OpenBracket, Modifiers::NONE),
            [b, c, a],
            "[ alone is not an order action"
        );
        assert_eq!(
            key(&mut editor, &mut engine, Key::L, Modifiers::COMMAND),
            [b, c, a]
        );
    }

    // S42-15: what the arrange actions can do: align with one or more,
    // distribute with three or more, and only the order actions that would
    // change the layers (FD-12).
    #[test]
    fn s42_15_arrange_actions_are_off_when_they_cannot_act() {
        let mut engine = engine();
        let mut editor = Editor::default();
        let (page, [a, b, c]) = three_pieces(&mut engine, &mut editor);
        let document = engine.edit().document();
        let can = |selected: Selection| CanArrange::of(document.as_deref(), &selected);
        assert_eq!(can(Selection::default()), CanArrange::default());
        let all = can(Selection::of(page, [a, b, c]));
        assert!(all.align && all.distribute);
        assert_eq!(all.order, [false; 4], "all of them: no order changes");
        let top = can(Selection::one(page, c));
        assert!(top.align && !top.distribute);
        // ORDERS: forward, backward, front, back.
        assert_eq!(top.order, [false, true, false, true]);
        let bottom = can(Selection::of(page, [a, b]));
        assert_eq!(bottom.order, [true, false, true, false]);
        assert!(!bottom.distribute);
        assert_eq!(can(Selection::one(page, b)).order, [true; 4]);
    }

    // S42-16: frames draw the arrange buttons for one and for several
    // elements, and their tooltips name the shortcut.
    #[test]
    fn s42_16_frames_draw_the_arrange_buttons() {
        let mut engine = engine();
        let mut editor = Editor::default();
        let (page, [a, ..]) = three_pieces(&mut engine, &mut editor);
        let ctx = egui::Context::default();
        theme().apply(&ctx);
        frame(&ctx, &mut editor, &mut engine, None);
        editor.selected = Selection::one(page, a);
        frame(&ctx, &mut editor, &mut engine, None);
        assert_eq!(the_shapes(&mut engine).len(), 3);
        let texts = Texts::new(gafira_ports::i18n::Locale::Es);
        let tip = arrange_tooltip(&ctx, &texts, "arrange.front", Some(FRONT));
        assert!(tip.starts_with("Al frente · "), "{tip}");
        assert!(tip.ends_with(']') && !tip.contains("Bracket"), "{tip}");
        assert_eq!(
            arrange_tooltip(&ctx, &texts, "arrange.align_top", None),
            "Alinear arriba"
        );
        // S44-01: every Align action names its Alt shortcut (D-88).
        assert_eq!(
            arrange_tooltip(&ctx, &texts, "arrange.align_top", Some(ALIGN_TOP)),
            format!("Alinear arriba · {}", ctx.format_shortcut(&ALIGN_TOP)),
        );
        assert_eq!(
            distribute_tooltip(&texts, "arrange.distribute_vertical", false),
            "Distribuir verticalmente: selecciona tres elementos o más"
        );
    }

    /// A frame with `events` and `modifiers` held, and the session's
    /// clipboard; what the frame asked the platform for.
    fn frame_events(
        ctx: &egui::Context,
        editor: &mut Editor,
        engine: &mut Gafira,
        events: Vec<egui::Event>,
        modifiers: Modifiers,
        clip: &mut Option<Arc<Clip>>,
    ) -> egui::PlatformOutput {
        let input = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1440.0, 900.0))),
            modifiers,
            events,
            ..Default::default()
        };
        let (theme, texts) = (theme(), Texts::new(gafira_ports::i18n::Locale::Es));
        ctx.run(input, |ctx| {
            editor.show(ctx, engine, &theme, &texts, None, clip);
        })
        .platform_output
    }

    fn placements(engine: &mut Gafira, nodes: &[NodeId]) -> Vec<Placement> {
        let document = engine.edit().document().expect("a document");
        let page = &document.pages()[0];
        nodes
            .iter()
            .map(|id| page.node(*id).expect("a node").base.placement)
            .collect()
    }

    // S43-13: Ctrl+G groups the selection above its frontmost element and
    // selects the group; Ctrl+Shift+G gives the children back, selected;
    // nothing moves and each is one undo step.
    #[test]
    fn s43_13_ctrl_g_groups_and_ctrl_shift_g_ungroups() {
        let mut engine = engine();
        let mut editor = Editor::default();
        let (page, [a, b, c]) = three_pieces(&mut engine, &mut editor);
        editor.selected = Selection::of(page, [b, a]);
        let before = placements(&mut engine, &[a, b, c]);
        let depth = undo_depth(&mut engine);
        let ctx = egui::Context::default();
        frame(&ctx, &mut editor, &mut engine, Some(Key::G));
        let roots = roots_now(&mut engine);
        assert_eq!(roots.len(), 2);
        let group = roots[0];
        assert_eq!(roots[1], c, "the group sits where b was, below c");
        assert_eq!(editor.selected, Selection::one(page, group));
        assert_eq!(placements(&mut engine, &[a, b, c]), before);
        assert_eq!(undo_depth(&mut engine), depth + 1);
        frame(&ctx, &mut editor, &mut engine, None);
        assert_eq!(editor.selected, Selection::one(page, group), "kept");
        frame_with(
            &ctx,
            &mut editor,
            &mut engine,
            Some(Key::G),
            Modifiers::COMMAND.plus(Modifiers::SHIFT),
        );
        assert_eq!(roots_now(&mut engine), [a, b, c]);
        assert_eq!(editor.selected, Selection::of(page, [a, b]));
        assert_eq!(placements(&mut engine, &[a, b, c]), before);
        assert_eq!(undo_depth(&mut engine), depth + 2);
        engine.edit().undo().expect("an undo");
        assert_eq!(roots_now(&mut engine), [group, c]);
    }

    /// `three_pieces` with the first two grouped and the group selected.
    fn bases(engine: &mut Gafira, nodes: &[NodeId]) -> Vec<(bool, bool, Option<String>)> {
        let document = engine.edit().document().expect("a document");
        let page = &document.pages()[0];
        nodes
            .iter()
            .map(|id| {
                let base = &page.node(*id).expect("a node").base;
                (base.visible, base.locked, base.name.clone())
            })
            .collect()
    }

    // S51-05: a move dropped on another page takes the element there, in
    // one undo step with the move; it stays selected on its new page,
    // which becomes the active one.
    #[test]
    fn s51_05_a_drop_on_another_page_moves_the_element_there() {
        let mut engine = engine();
        let mut editor = Editor::default();
        let (one, rect, _) = two_pieces(&mut engine, &mut editor);
        ask(
            &mut engine,
            &mut editor,
            Asked {
                add_page: true,
                ..Asked::default()
            },
        );
        let two = engine.edit().document().expect("a document").pages()[1].id;
        let before = engine.edit().document().expect("a document");
        let depth = undo_depth(&mut engine);
        let from = placements(&mut engine, &[rect])[0];
        editor.selected = Selection::one(one, rect);
        for asked in [
            Asked {
                drag_begin: Some(None),
                ..Asked::default()
            },
            Asked {
                drag_to: Some(Vec2::new(0.0, 900.0)),
                free: true,
                ..Asked::default()
            },
            Asked {
                drag_end: true,
                transfer: Some((one, two, (0.0, -1100.0))),
                ..Asked::default()
            },
        ] {
            ask(&mut engine, &mut editor, asked);
        }
        let document = engine.edit().document().expect("a document");
        let (first, second) = (&document.pages()[0], &document.pages()[1]);
        assert!(first.node(rect).is_none(), "it left page 1");
        assert_eq!(second.roots.len(), 1);
        let moved = second.node(second.roots[0]).expect("on page 2");
        assert_eq!(moved.base.placement.x, from.x);
        assert_eq!(
            moved.base.placement.y,
            from.y + 900.0 / editor.zoom as f64 - 1100.0
        );
        assert_eq!(editor.selected.page(), Some(two));
        assert_eq!(editor.active, Some(two));
        assert_eq!(undo_depth(&mut engine), depth + 1, "one step");
        engine.edit().undo().expect("an undo");
        assert_eq!(engine.edit().document().expect("a document"), before);
    }

    // S51-07 (D-93): a move dropped wholly outside the page deletes the
    // element, in one undo step with the move.
    #[test]
    fn s51_07_a_drop_outside_the_page_deletes_the_element() {
        let mut engine = engine();
        let mut editor = Editor::default();
        let (page, rect, ellipse) = two_pieces(&mut engine, &mut editor);
        let before = engine.edit().document().expect("a document");
        let depth = undo_depth(&mut engine);
        editor.selected = Selection::one(page, rect);
        for asked in [
            Asked {
                drag_begin: Some(None),
                ..Asked::default()
            },
            Asked {
                drag_to: Some(Vec2::new(-5000.0, 0.0)),
                free: true,
                ..Asked::default()
            },
            Asked {
                drag_end: true,
                ..Asked::default()
            },
        ] {
            ask(&mut engine, &mut editor, asked);
        }
        let document = engine.edit().document().expect("a document");
        assert!(document.pages()[0].node(rect).is_none(), "deleted");
        assert!(document.pages()[0].node(ellipse).is_some());
        assert_eq!(undo_depth(&mut engine), depth + 1, "one step");
        engine.edit().undo().expect("an undo");
        assert_eq!(engine.edit().document().expect("a document"), before);
    }

    // S52-03: a shape dropped from the panel lands on its page, centred
    // where it was dropped, selected, its page active, in one undo step.
    #[test]
    fn s52_03_a_dropped_shape_lands_where_it_was_dropped() {
        let mut engine = engine();
        let mut editor = Editor::default();
        let (page, _, _) = two_pieces(&mut engine, &mut editor);
        let depth = undo_depth(&mut engine);
        ask(
            &mut engine,
            &mut editor,
            Asked {
                drop_piece: Some((canvas::Piece::Circle, page, (100.0, 120.0))),
                ..Asked::default()
            },
        );
        let document = engine.edit().document().expect("a document");
        let on = &document.pages()[0];
        let dropped = on
            .node(*on.roots.last().expect("a root"))
            .expect("the shape");
        let p = dropped.base.placement;
        assert_eq!((p.x + p.w / 2.0, p.y + p.h / 2.0), (100.0, 120.0));
        assert_eq!(editor.selected.single(), Some((page, dropped.base.id)));
        assert_eq!(editor.active, Some(page));
        assert_eq!(undo_depth(&mut engine), depth + 1);
    }

    // S66-08, D-105: a frame is added empty in the middle of the page with
    // a click, or where it is dropped with a drag, selected, in one undo
    // step each.
    #[test]
    fn s66_08_a_frame_is_added_by_click_or_drop() {
        let mut engine = engine();
        let mut editor = Editor::default();
        let (page, _, _) = two_pieces(&mut engine, &mut editor);
        let depth = undo_depth(&mut engine);
        ask(
            &mut engine,
            &mut editor,
            Asked {
                drop_frame: Some((canvas::Piece::Heart, page, (100.0, 120.0))),
                ..Asked::default()
            },
        );
        let document = engine.edit().document().expect("a document");
        let on = &document.pages()[0];
        let dropped = on
            .node(*on.roots.last().expect("a root"))
            .expect("the frame");
        let p = dropped.base.placement;
        assert!((p.x + p.w / 2.0 - 100.0).abs() < 1e-9 && (p.y + p.h / 2.0 - 120.0).abs() < 1e-9);
        let gafira_doc::NodeKind::Frame(frame) = &dropped.kind else {
            panic!("a frame")
        };
        assert!(frame.content.is_none(), "a new frame is empty");
        assert_eq!(editor.selected.single(), Some((page, dropped.base.id)));
        assert_eq!(undo_depth(&mut engine), depth + 1);
        ask(
            &mut engine,
            &mut editor,
            Asked {
                insert_frame: Some(canvas::Piece::Circle),
                ..Asked::default()
            },
        );
        assert_eq!(undo_depth(&mut engine), depth + 2);
    }

    // D-105: a light dragged from Elementos lands like a shape: on its
    // page, centred where it was dropped, selected, in one undo step; a
    // click adds one in the middle of the page.
    #[test]
    fn d105_a_dropped_light_lands_where_it_was_dropped() {
        let mut engine = engine();
        let mut editor = Editor::default();
        let (page, _, _) = two_pieces(&mut engine, &mut editor);
        let depth = undo_depth(&mut engine);
        ask(
            &mut engine,
            &mut editor,
            Asked {
                drop_effect: Some((elements::LightItem::LensFlare, page, (100.0, 120.0))),
                ..Asked::default()
            },
        );
        let document = engine.edit().document().expect("a document");
        let on = &document.pages()[0];
        let dropped = on
            .node(*on.roots.last().expect("a root"))
            .expect("the light");
        let p = dropped.base.placement;
        assert_eq!((p.x + p.w / 2.0, p.y + p.h / 2.0), (100.0, 120.0));
        let gafira_doc::NodeKind::Shape(shape) = &dropped.kind else {
            panic!("a shape")
        };
        assert!(matches!(
            shape.effect,
            Some(gafira_doc::ShapeEffect::LensFlare(_))
        ));
        assert_eq!(editor.selected.single(), Some((page, dropped.base.id)));
        assert_eq!(undo_depth(&mut engine), depth + 1);
        ask(
            &mut engine,
            &mut editor,
            Asked {
                insert_effect: Some(elements::LightItem::Bokeh),
                ..Asked::default()
            },
        );
        assert_eq!(undo_depth(&mut engine), depth + 2);
    }

    /// An editor with the plain table (4 × 3) in the middle of its page.
    fn with_table(engine: &mut Gafira) -> (Editor, PageId, NodeId) {
        start(engine, StartFormat::Social).expect("a new document");
        let mut editor = Editor::default();
        ask(
            engine,
            &mut editor,
            Asked {
                insert_table: Some((crate::table_templates::Template::Grid, None)),
                ..Asked::default()
            },
        );
        let (page, node) = editor.selected.single().expect("the table");
        (editor, page, node)
    }

    fn the_table(engine: &mut Gafira, page: PageId, node: NodeId) -> gafira_doc::TableNode {
        let document = engine.edit().document().expect("a document");
        table_of(&document, page, node).expect("a table").0
    }

    // S81-07, D-105: a click puts the table in the middle of the page; a
    // drop, where it was dropped; selected, one undo step each.
    #[test]
    fn d105_a_dropped_table_lands_where_it_was_dropped() {
        let mut engine = engine();
        let (mut editor, page, node) = with_table(&mut engine);
        assert_eq!(undo_depth(&mut engine), 1);
        editor.zoom = 0.5;
        editor.page_rects = vec![(page, Rect::from_min_size(Pos2::ZERO, Vec2::splat(540.0)))];
        ask(
            &mut engine,
            &mut editor,
            Asked {
                insert_table: Some((
                    crate::table_templates::Template::Budget,
                    Some(Pos2::new(250.0, 260.0)),
                )),
                ..Asked::default()
            },
        );
        let (_, dropped) = editor.selected.single().expect("the new table");
        assert_ne!(dropped, node);
        let document = engine.edit().document().expect("a document");
        let p = document.pages()[0]
            .node(dropped)
            .expect("it")
            .base
            .placement;
        assert!((p.x + p.w / 2.0 - 500.0).abs() < 1e-6);
        assert!((p.y + p.h / 2.0 - 520.0).abs() < 1e-6);
        assert_eq!(undo_depth(&mut engine), 2);
    }

    // S81-08: what is written in a cell is one undo step; Enter goes down
    // and Tab on; Tab in the last cell adds a row.
    #[test]
    fn s81_08_writing_in_cells() {
        let mut engine = engine();
        let (mut editor, page, node) = with_table(&mut engine);
        let depth = undo_depth(&mut engine);
        let cells = |cell| Some(Some(tables::Cells::one(page, node, cell)));
        ask(
            &mut engine,
            &mut editor,
            Asked {
                cells: cells((0, 0)),
                write_cell: Some(Some("H".to_owned())),
                ..Asked::default()
            },
        );
        for words in ["Ho", "Hola"] {
            ask(
                &mut engine,
                &mut editor,
                Asked {
                    cell_words: Some(words.to_owned()),
                    ..Asked::default()
                },
            );
        }
        let end = |to| Asked {
            end_cell: Some(to),
            ..Asked::default()
        };
        ask(&mut engine, &mut editor, end(CellMove::Down));
        assert_eq!(the_table(&mut engine, page, node).cells[0].text, "Hola");
        assert_eq!(undo_depth(&mut engine), depth + 1);
        let at = |editor: &Editor| editor.cells.as_ref().map(|c| (c.from, c.writing.is_some()));
        assert_eq!(at(&editor), Some(((1, 0), false)));
        ask(
            &mut engine,
            &mut editor,
            Asked {
                cells: cells((3, 2)),
                write_cell: Some(None),
                ..Asked::default()
            },
        );
        ask(&mut engine, &mut editor, end(CellMove::Next));
        assert_eq!(the_table(&mut engine, page, node).rows.len(), 5);
        assert_eq!(at(&editor), Some(((4, 0), false)));
        engine.edit().undo().expect("undo");
        engine.edit().undo().expect("undo");
        assert_eq!(the_table(&mut engine, page, node).cells[0].text, "");
    }

    // S81-06, S81-10: looks go to the cells chosen, or to the whole table;
    // a colour being picked is one undo step.
    #[test]
    fn s81_10_looks_go_to_the_cells_chosen() {
        let mut engine = engine();
        let (mut editor, page, node) = with_table(&mut engine);
        let depth = undo_depth(&mut engine);
        let mut cells = tables::Cells::one(page, node, (1, 0));
        cells.to = (1, 2);
        editor.cells = Some(cells);
        for code in [0x0011_0000, 0x0022_0000] {
            let act = tables::Act::Fill(Some(crate::colors::hex(code)));
            ask(
                &mut engine,
                &mut editor,
                Asked {
                    table_act: Some(act),
                    ..Asked::default()
                },
            );
        }
        ask(
            &mut engine,
            &mut editor,
            Asked {
                table_rest: true,
                ..Asked::default()
            },
        );
        assert_eq!(undo_depth(&mut engine), depth + 1);
        let table = the_table(&mut engine, page, node);
        let filled: Vec<bool> = table.cells.iter().map(|c| c.style.fill.is_some()).collect();
        assert_eq!(filled.iter().filter(|f| **f).count(), 3);
        assert!(filled[3] && filled[5]);
        // The whole table, with no cell chosen.
        editor.cells = None;
        ask(
            &mut engine,
            &mut editor,
            Asked {
                table_act: Some(tables::Act::Bold(true)),
                ..Asked::default()
            },
        );
        let table = the_table(&mut engine, page, node);
        assert!(table.cells.iter().all(|c| c.style.bold));
        assert_eq!(undo_depth(&mut engine), depth + 2);
        // The font panel too.
        ask(
            &mut engine,
            &mut editor,
            Asked {
                font: Some(("Manrope".to_owned(), false, true)),
                ..Asked::default()
            },
        );
        let table = the_table(&mut engine, page, node);
        assert!(table.cells.iter().all(|c| c.style.font == "Manrope"));
    }

    // S81-08: with cells chosen the keys are theirs: Delete empties them and
    // the table stays; Escape goes back to the table, whose Delete takes it.
    #[test]
    fn s81_08_the_keys_of_the_cells() {
        let mut engine = engine();
        let (mut editor, page, node) = with_table(&mut engine);
        let ctx = egui::Context::default();
        editor.cells = Some(tables::Cells::one(page, node, (0, 0)));
        ask(
            &mut engine,
            &mut editor,
            Asked {
                table_act: Some(tables::Act::Paste(vec![vec![
                    "a".to_owned(),
                    "b".to_owned(),
                ]])),
                ..Asked::default()
            },
        );
        let text =
            |engine: &mut Gafira, i: usize| the_table(engine, page, node).cells[i].text.clone();
        assert_eq!(
            (text(&mut engine, 0), text(&mut engine, 1)),
            ("a".into(), "b".into())
        );
        editor.cells = Some(tables::Cells::one(page, node, (0, 0)));
        frame(&ctx, &mut editor, &mut engine, None);
        frame_with(
            &ctx,
            &mut editor,
            &mut engine,
            Some(Key::Delete),
            Modifiers::NONE,
        );
        assert_eq!(
            (text(&mut engine, 0), text(&mut engine, 1)),
            (String::new(), "b".into())
        );
        frame_with(
            &ctx,
            &mut editor,
            &mut engine,
            Some(Key::ArrowRight),
            Modifiers::NONE,
        );
        assert_eq!(editor.cells.as_ref().map(|c| c.from), Some((0, 1)));
        frame_with(
            &ctx,
            &mut editor,
            &mut engine,
            Some(Key::Escape),
            Modifiers::NONE,
        );
        assert!(editor.cells.is_none());
        assert_eq!(
            editor.selected.single(),
            Some((page, node)),
            "still selected"
        );
        frame_with(
            &ctx,
            &mut editor,
            &mut engine,
            Some(Key::Delete),
            Modifiers::NONE,
        );
        let document = engine.edit().document().expect("a document");
        assert!(table_of(&document, page, node).is_none(), "the table goes");
    }

    /// An editor with a QR code of `text` in the middle of its page.
    fn with_qr(engine: &mut Gafira, text: &str) -> (Editor, PageId, NodeId, gafira_doc::QrNode) {
        start(engine, StartFormat::Social).expect("a new document");
        let mut editor = Editor::default();
        let mut code = crate::qr::fresh();
        code.content = gafira_doc::QrContent::Text {
            text: text.to_owned(),
        };
        ask(
            engine,
            &mut editor,
            Asked {
                insert_qr: Some((Box::new(code.clone()), None)),
                ..Asked::default()
            },
        );
        let (page, node) = editor.selected.single().expect("the code");
        (editor, page, node, code)
    }

    // S80-11, D-105: a click puts the code in the middle of the page, a
    // quarter of its shorter side; a drop leaves it where it was dropped;
    // selected, one undo step each.
    #[test]
    fn d105_a_dropped_qr_lands_where_it_was_dropped() {
        let mut engine = engine();
        let (mut editor, page, node, code) = with_qr(&mut engine, "hola");
        let document = engine.edit().document().expect("a document");
        let on = &document.pages()[0];
        let size = on.size.to_px(document.dpi()).expect("px");
        let p = on.node(node).expect("the code").base.placement;
        let side = size.width().min(size.height()) / 4.0;
        assert_eq!((p.w, p.h), (side, side));
        let middle = (size.width() / 2.0, size.height() / 2.0);
        assert_eq!((p.x + side / 2.0, p.y + side / 2.0), middle);
        assert_eq!(undo_depth(&mut engine), 1);
        editor.zoom = 0.5;
        editor.page_rects = vec![(page, Rect::from_min_size(Pos2::ZERO, Vec2::splat(540.0)))];
        ask(
            &mut engine,
            &mut editor,
            Asked {
                insert_qr: Some((Box::new(code), Some(Pos2::new(100.0, 150.0)))),
                ..Asked::default()
            },
        );
        let (_, dropped) = editor.selected.single().expect("the new code");
        assert_ne!(dropped, node);
        let document = engine.edit().document().expect("a document");
        let p = document.pages()[0]
            .node(dropped)
            .expect("it")
            .base
            .placement;
        assert_eq!((p.x + p.w / 2.0, p.y + p.h / 2.0), (200.0, 300.0));
        assert_eq!(undo_depth(&mut engine), 2);
    }

    // S80-12: a burst of changes in Propiedades is one undo step, until the
    // fields come to rest; what is written next is another.
    #[test]
    fn s80_12_a_burst_of_changes_is_one_undo_step() {
        let mut engine = engine();
        let (mut editor, page, node, code) = with_qr(&mut engine, "a");
        let depth = undo_depth(&mut engine);
        let write = |engine: &mut Gafira, editor: &mut Editor, text: &str, rest: bool| {
            let mut next = code.clone();
            next.content = gafira_doc::QrContent::Text {
                text: text.to_owned(),
            };
            let asked = Asked {
                qr: Some(Box::new(next)),
                qr_rest: rest,
                ..Asked::default()
            };
            ask(engine, editor, asked);
        };
        for text in ["ab", "abc"] {
            write(&mut engine, &mut editor, text, false);
        }
        write(&mut engine, &mut editor, "abcd", true);
        assert_eq!(undo_depth(&mut engine), depth + 1);
        write(&mut engine, &mut editor, "x", true);
        assert_eq!(undo_depth(&mut engine), depth + 2);
        engine.edit().undo().expect("undo");
        engine.edit().undo().expect("undo");
        let document = engine.edit().document().expect("a document");
        assert_eq!(qr_of(&document, page, node), Some(code));
    }

    // S80-13: «Convertir en formas» leaves a group of shapes in the place
    // of the code, selected; one undo step.
    #[test]
    fn s80_13_converting_leaves_a_group_of_shapes() {
        let mut engine = engine();
        let (mut editor, page, node, _) = with_qr(&mut engine, "hola");
        let depth = undo_depth(&mut engine);
        ask(
            &mut engine,
            &mut editor,
            Asked {
                qr_to_shapes: true,
                ..Asked::default()
            },
        );
        let document = engine.edit().document().expect("a document");
        let on = &document.pages()[0];
        assert!(on.node(node).is_none(), "the code is gone");
        let (_, group) = editor.selected.single().expect("the group");
        let group = on.node(group).expect("the group");
        assert!(matches!(group.kind, gafira_doc::NodeKind::Group(_)));
        assert_eq!(group.children.len(), 4, "background, body and eyes");
        assert_eq!(on.roots, vec![group.base.id]);
        assert_eq!(undo_depth(&mut engine), depth + 1);
        engine.edit().undo().expect("undo");
        let document = engine.edit().document().expect("a document");
        assert!(qr_of(&document, page, node).is_some());
    }

    // S55-04: writing on a shape on the canvas stores its words as the
    // shape's text, the whole edit in one undo step; emptying it leaves
    // the shape without text; a text element takes its words too.
    #[test]
    fn s55_04_editing_writes_one_undo_step() {
        let mut engine = engine();
        let mut editor = Editor::default();
        let (page, rect, _) = two_pieces(&mut engine, &mut editor);
        let inner = |engine: &mut Gafira| {
            let document = engine.edit().document().expect("a document");
            match &document.pages()[0].node(rect).expect("the shape").kind {
                gafira_doc::NodeKind::Shape(shape) => shape.text.clone(),
                _ => panic!("a shape"),
            }
        };
        let depth = undo_depth(&mut engine);
        let write = |engine: &mut Gafira, editor: &mut Editor, words: &[&str]| {
            ask(
                engine,
                editor,
                Asked {
                    begin_edit: Some((page, rect)),
                    ..Asked::default()
                },
            );
            for words in words {
                ask(
                    engine,
                    editor,
                    Asked {
                        edit_text: Some((*words).to_owned()),
                        ..Asked::default()
                    },
                );
            }
            ask(
                engine,
                editor,
                Asked {
                    end_edit: true,
                    ..Asked::default()
                },
            );
        };
        write(&mut engine, &mut editor, &["Ho", "Hola", "Hola mundo"]);
        let text = inner(&mut engine).expect("a text");
        assert_eq!(text.content.plain_text(), "Hola mundo");
        assert_eq!(
            (text.align, text.valign),
            (
                gafira_doc::TextAlign::Center,
                gafira_doc::VerticalAlign::Middle
            )
        );
        assert_eq!(undo_depth(&mut engine), depth + 1, "one step");
        assert!(editor.editing.is_none());
        write(&mut engine, &mut editor, &["Hola", ""]);
        assert_eq!(inner(&mut engine), None, "emptied");
        engine.edit().undo().expect("an undo");
        assert_eq!(inner(&mut engine), Some(text));
        engine.edit().undo().expect("an undo");
        assert_eq!(inner(&mut engine), None);
    }

    // S56-02: «Duplicar» in the header copies the page with everything on
    // it: same number of elements, same kinds, same places.
    #[test]
    fn s56_02_duplicate_copies_the_content() {
        let mut engine = engine();
        let mut editor = Editor::default();
        let (page, rect, _) = two_pieces(&mut engine, &mut editor);
        ask(
            &mut engine,
            &mut editor,
            Asked {
                begin_edit: Some((page, rect)),
                ..Asked::default()
            },
        );
        ask(
            &mut engine,
            &mut editor,
            Asked {
                edit_text: Some("Hola".to_owned()),
                ..Asked::default()
            },
        );
        ask(
            &mut engine,
            &mut editor,
            Asked {
                end_edit: true,
                ..Asked::default()
            },
        );
        ask(
            &mut engine,
            &mut editor,
            Asked {
                page_action: Some(PageAction::Duplicate(page)),
                ..Asked::default()
            },
        );
        let document = engine.edit().document().expect("a document");
        let (source, copy) = (&document.pages()[0], &document.pages()[1]);
        assert_eq!(copy.nodes.len(), source.nodes.len());
        assert_eq!(copy.roots.len(), source.roots.len());
        for (a, b) in source.roots.iter().zip(&copy.roots) {
            let (a, b) = (source.node(*a).unwrap(), copy.node(*b).unwrap());
            assert_eq!(a.base.placement, b.base.placement);
            assert_eq!(a.kind, b.kind);
        }
    }

    // S56-03: each action of a page header is one undo step: add a page
    // after, duplicate, move, hide, lock, rename and delete.
    #[test]
    fn s56_03_page_header_actions() {
        let mut engine = engine();
        let mut editor = Editor::default();
        start(&mut engine, StartFormat::Social).expect("a new document");
        let ids = |engine: &mut Gafira| -> Vec<PageId> {
            engine
                .edit()
                .document()
                .expect("a document")
                .pages()
                .iter()
                .map(|p| p.id)
                .collect()
        };
        let act = |engine: &mut Gafira, editor: &mut Editor, action| {
            ask(
                engine,
                editor,
                Asked {
                    page_action: Some(action),
                    ..Asked::default()
                },
            );
        };
        let first = ids(&mut engine)[0];
        let depth = undo_depth(&mut engine);
        act(&mut engine, &mut editor, PageAction::AddAfter(first));
        let second = ids(&mut engine)[1];
        assert_eq!(ids(&mut engine).len(), 2);
        assert_eq!(editor.active, Some(second));
        act(&mut engine, &mut editor, PageAction::Duplicate(first));
        let copy = ids(&mut engine)[1];
        assert_eq!(ids(&mut engine), [first, copy, second]);
        assert_eq!(editor.active, Some(copy));
        act(&mut engine, &mut editor, PageAction::Move(first, 1));
        assert_eq!(ids(&mut engine), [copy, first, second]);
        act(&mut engine, &mut editor, PageAction::Move(copy, -1));
        assert_eq!(
            ids(&mut engine),
            [copy, first, second],
            "the first stays first"
        );
        act(&mut engine, &mut editor, PageAction::Hide(second, true));
        act(&mut engine, &mut editor, PageAction::Lock(second, true));
        act(
            &mut engine,
            &mut editor,
            PageAction::Rename(second, "  Contraportada ".to_owned()),
        );
        let document = engine.edit().document().expect("a document");
        let page = document.page(second).expect("the page");
        assert!(page.hidden && page.locked);
        assert_eq!(page.name, "Contraportada");
        act(&mut engine, &mut editor, PageAction::Delete(copy));
        assert_eq!(ids(&mut engine), [first, second]);
        assert_eq!(undo_depth(&mut engine), depth + 7, "one step each");
    }

    // S60-01, S60-02: a guide made on one page is on every page, in one
    // undo step; a page added later is born with them.
    #[test]
    fn s60_01_guides_are_shared() {
        let mut engine = engine();
        let mut editor = Editor::default();
        start(&mut engine, StartFormat::Social).expect("a new document");
        ask(
            &mut engine,
            &mut editor,
            Asked {
                add_page: true,
                ..Asked::default()
            },
        );
        let pages = |engine: &mut Gafira| {
            engine
                .edit()
                .document()
                .expect("a document")
                .pages()
                .to_vec()
        };
        let first = pages(&mut engine)[0].id;
        let guides = GuideSet(vec![gafira_doc::Guide {
            axis: GuideAxis::Vertical,
            position: gafira_geom::Length::new(120.0, gafira_geom::Unit::Px),
            locked: false,
        }]);
        let depth = undo_depth(&mut engine);
        ask(
            &mut engine,
            &mut editor,
            Asked {
                guides: Some((first, guides.clone())),
                ..Asked::default()
            },
        );
        assert!(
            pages(&mut engine).iter().all(|p| p.guides == guides),
            "on every page"
        );
        assert_eq!(undo_depth(&mut engine), depth + 1, "one step");
        ask(
            &mut engine,
            &mut editor,
            Asked {
                add_page: true,
                ..Asked::default()
            },
        );
        assert_eq!(
            pages(&mut engine)[2].guides,
            guides,
            "a new page takes them"
        );
        let last = pages(&mut engine)[2].id;
        ask(
            &mut engine,
            &mut editor,
            Asked {
                page_action: Some(PageAction::AddAfter(last)),
                ..Asked::default()
            },
        );
        assert_eq!(pages(&mut engine)[3].guides, guides, "also from the header");
        engine.edit().undo().expect("an undo");
        engine.edit().undo().expect("an undo");
        engine.edit().undo().expect("an undo");
        assert!(pages(&mut engine).iter().all(|p| p.guides.0.is_empty()));
    }

    // S58-04: dragging the end of a line near the right point of a shape
    // connects it there, in one undo step; moving the shape then moves the
    // end with it.
    #[test]
    fn s58_04_a_dropped_end_connects() {
        use gafira_doc::{Anchor, Attachment, LineCap, LineEnd, LineNode};
        let mut engine = engine();
        let mut editor = Editor::default();
        let (page, rect, _) = two_pieces(&mut engine, &mut editor);
        let target = placements(&mut engine, &[rect])[0];
        let line = LineNode {
            start: LineEnd::new([0.0, 0.0], LineCap::None),
            end: LineEnd::new([1.0, 0.0], LineCap::Arrow),
            color: gafira_color::Color::Srgb(gafira_color::Rgba::new(0.0, 0.0, 0.0, 1.0)),
            width: 4.0,
        };
        let id = NodeId::new(engine.edit().document().expect("a document").next_id());
        engine
            .edit()
            .execute(Box::new(InsertNodes::new(
                page,
                None,
                2,
                vec![gafira_edit::NodeSpec::new(
                    gafira_doc::NodeKind::Line(line),
                    Placement::new(10.0, 10.0, 50.0, 0.0),
                )],
            )))
            .expect("a line");
        editor.selected = Selection::one(page, id);
        let depth = undo_depth(&mut engine);
        // The end is at (60, 10); the right point of the shape is at
        // (x + w, y + h / 2). Drop the end 5 px short of it.
        let right = Vec2::new(
            (target.x + target.w - 60.0 - 5.0) as f32,
            (target.y + target.h / 2.0 - 10.0) as f32,
        ) * editor.zoom;
        for asked in [
            Asked {
                drag_begin: Some(Some(Hold::LineEnd(1))),
                ..Asked::default()
            },
            Asked {
                drag_to: Some(right),
                ..Asked::default()
            },
            Asked {
                drag_end: true,
                ..Asked::default()
            },
        ] {
            ask(&mut engine, &mut editor, asked);
        }
        let end = |engine: &mut Gafira| {
            let document = engine.edit().document().expect("a document");
            let page = &document.pages()[0];
            let node = page.node(id).expect("the line");
            let gafira_doc::NodeKind::Line(line) = &node.kind else {
                panic!("a line")
            };
            (line.end.attached, line.ends(page, &node.base.placement)[1])
        };
        let (attached, at) = end(&mut engine);
        assert_eq!(attached, Some(Attachment::anchor(rect, Anchor::Right)));
        assert!((at.x - (target.x + target.w)).abs() < 1e-6);
        assert_eq!(undo_depth(&mut engine), depth + 1, "one step");
        editor.selected = Selection::one(page, rect);
        ask(
            &mut engine,
            &mut editor,
            Asked {
                nudge: Some((30.0, 0.0)),
                ..Asked::default()
            },
        );
        let (_, moved) = end(&mut engine);
        assert!(
            (moved.x - (target.x + target.w + 30.0)).abs() < 1e-6,
            "the end follows"
        );
    }

    // S61-01: text from outside Gafira pasted on the canvas becomes a body
    // text box with those words on the current page, selected; the next
    // frame fits its height to them.
    #[test]
    fn s61_01_foreign_text_pastes_a_text_box() {
        let mut engine = engine();
        let mut editor = Editor::default();
        let (page, _, _) = two_pieces(&mut engine, &mut editor);
        let ctx = egui::Context::default();
        let mut session = None;
        let before = the_shapes(&mut engine).len();
        frame_events(
            &ctx,
            &mut editor,
            &mut engine,
            vec![egui::Event::Paste("Hola desde otra app".to_owned())],
            Modifiers::COMMAND,
            &mut session,
        );
        assert_eq!(the_shapes(&mut engine).len(), before + 1);
        let (on, node) = editor.selected.single().expect("the text");
        assert_eq!(on, page);
        let document = engine.edit().document().expect("a document");
        let gafira_doc::NodeKind::Text(text) =
            &document.pages()[0].node(node).expect("a node").kind
        else {
            panic!("a text");
        };
        assert_eq!(text.content.plain_text(), "Hola desde otra app");
    }

    // S61-03: a text dropped from the Texto panel lands centred where it
    // was dropped, selected, in one undo step.
    #[test]
    fn s61_03_a_dropped_text_lands_where_it_was_dropped() {
        let mut engine = engine();
        let mut editor = Editor::default();
        let (page, _, _) = two_pieces(&mut engine, &mut editor);
        let depth = undo_depth(&mut engine);
        ask(
            &mut engine,
            &mut editor,
            Asked {
                drop_text: Some((
                    canvas::TextPreset::Title,
                    "Título".to_owned(),
                    page,
                    (300.0, 200.0),
                )),
                ..Asked::default()
            },
        );
        let (_, node) = editor.selected.single().expect("the text");
        let p = placements(&mut engine, &[node])[0];
        assert_eq!((p.x + p.w / 2.0, p.y + p.h / 2.0), (300.0, 200.0));
        assert_eq!(undo_depth(&mut engine), depth + 1);
    }

    // S61-02: a text whose letters grow takes the height of its words once
    // the canvas has drawn it.
    #[test]
    fn s61_02_a_bigger_size_fits_the_box() {
        let mut engine = engine();
        let mut editor = Editor::default();
        start(&mut engine, StartFormat::Social).expect("a new document");
        ask(
            &mut engine,
            &mut editor,
            Asked {
                insert_text: Some((canvas::TextPreset::Body, "Hola".to_owned())),
                ..Asked::default()
            },
        );
        let (_, node) = editor.selected.single().expect("the text");
        let before = placements(&mut engine, &[node])[0].h;
        let document = engine.edit().document().expect("a document");
        let gafira_doc::NodeKind::Text(text) =
            &document.pages()[0].node(node).expect("a node").kind
        else {
            panic!("a text");
        };
        let mut style = text.content.first_style().expect("a style").clone();
        style.size *= 4.0;
        ask(
            &mut engine,
            &mut editor,
            Asked {
                text: Some(gafira_doc::RichText::plain("Hola", &style)),
                ..Asked::default()
            },
        );
        // The window installs the theme fonts; so does this test, so the
        // canvas can lay the words out.
        let ctx = egui::Context::default();
        theme().apply(&ctx);
        for _ in 0..3 {
            frame_with(&ctx, &mut editor, &mut engine, None, Modifiers::NONE);
        }
        let after = placements(&mut engine, &[node])[0].h;
        assert!(after > before * 2.0, "{before} -> {after}");
    }

    // S62-06: the Fuente panel lists the document's families, then those of
    // Gafira, then those of the system (a note while they are looked for),
    // each under its title; an open family shows its faces; the search
    // keeps the families whose name holds it; a system family named like
    // one of Gafira is left out.
    #[test]
    fn s62_06_font_rows() {
        let system = system_fonts::SystemFonts::new();
        let none = BTreeSet::new();
        let rows = font_rows("", &[], &none, &system);
        assert_eq!(rows[0], PanelRow::Title("fonts.gafira"));
        assert!(rows.contains(&PanelRow::Title("fonts.system")));
        assert_eq!(rows.last(), Some(&PanelRow::Note("fonts.looking")));
        system.set(vec![
            system_fonts::SystemFont {
                family: "Ubuntu".to_owned(),
                bold: false,
                italic: false,
                path: "/f/u.ttf".into(),
            },
            system_fonts::SystemFont {
                family: "Roboto".to_owned(),
                bold: false,
                italic: false,
                path: "/f/r.ttf".into(),
            },
        ]);
        let rows = font_rows("", &["Lato".to_owned()], &none, &system);
        assert_eq!(rows[0], PanelRow::Title("fonts.document"));
        assert!(
            matches!(&rows[1], PanelRow::Family { family, system: false, .. } if family == "Lato")
        );
        let systems: Vec<&PanelRow> = rows
            .iter()
            .filter(|r| matches!(r, PanelRow::Family { system: true, .. }))
            .collect();
        assert_eq!(systems.len(), 1, "Roboto is one of Gafira");
        let open: BTreeSet<String> = ["Lato".to_owned()].into();
        let rows = font_rows("lat", &[], &open, &system);
        assert_eq!(
            rows.iter()
                .filter(|r| matches!(r, PanelRow::Face { .. }))
                .count(),
            4,
            "the four faces of Lato"
        );
        assert!(
            rows.iter()
                .all(|r| !matches!(r, PanelRow::Family { family, .. } if family == "Ubuntu"))
        );
        assert_eq!(
            rows.last(),
            Some(&PanelRow::Note("fonts.none")),
            "no system font matches"
        );
    }

    // S62-04: a font picked in the Fuente panel goes to every span of the
    // selected text, or of the text of the selected shape, in one undo step.
    #[test]
    fn s62_04_the_font_changes_in_one_step() {
        let mut engine = engine();
        let mut editor = Editor::default();
        let (page, rect, _) = two_pieces(&mut engine, &mut editor);
        ask(
            &mut engine,
            &mut editor,
            Asked {
                insert_text: Some((canvas::TextPreset::Body, "Hola\nmundo".to_owned())),
                ..Asked::default()
            },
        );
        let (_, text) = editor.selected.single().expect("the text");
        let spans = |engine: &mut Gafira, node: NodeId| -> Vec<gafira_doc::SpanStyle> {
            let document = engine.edit().document().expect("a document");
            let content = match &document.pages()[0].node(node).expect("a node").kind {
                gafira_doc::NodeKind::Text(t) => t.content.clone(),
                gafira_doc::NodeKind::Shape(s) => s.text.clone().expect("a text").content,
                _ => panic!("no text"),
            };
            content.spans().map(|span| span.style.clone()).collect()
        };
        let depth = undo_depth(&mut engine);
        ask(
            &mut engine,
            &mut editor,
            Asked {
                font: Some(("Lato".to_owned(), true, false)),
                ..Asked::default()
            },
        );
        let styles = spans(&mut engine, text);
        assert_eq!(styles.len(), 2);
        assert!(
            styles
                .iter()
                .all(|s| s.font == "Lato" && s.bold && !s.italic)
        );
        assert_eq!(undo_depth(&mut engine), depth + 1, "one step");
        ask(
            &mut engine,
            &mut editor,
            Asked {
                begin_edit: Some((page, rect)),
                ..Asked::default()
            },
        );
        ask(
            &mut engine,
            &mut editor,
            Asked {
                edit_text: Some("Forma".to_owned()),
                ..Asked::default()
            },
        );
        ask(
            &mut engine,
            &mut editor,
            Asked {
                end_edit: true,
                ..Asked::default()
            },
        );
        editor.selected = Selection::one(page, rect);
        ask(
            &mut engine,
            &mut editor,
            Asked {
                font: Some(("Pacifico".to_owned(), false, true)),
                ..Asked::default()
            },
        );
        assert!(
            spans(&mut engine, rect)
                .iter()
                .all(|s| s.font == "Pacifico" && s.italic)
        );
    }

    // S50-01: a press on an element activates its page, on a page's
    // background that page, outside every page none.
    #[test]
    fn s50_01_active_after_a_click() {
        let (one, two) = (PageId::new(1), PageId::new(2));
        let pages = [
            (
                one,
                Rect::from_min_size(Pos2::new(100.0, 50.0), Vec2::splat(400.0)),
            ),
            (
                two,
                Rect::from_min_size(Pos2::new(100.0, 500.0), Vec2::splat(400.0)),
            ),
        ];
        assert_eq!(
            active_after(&pages, Pos2::new(300.0, 600.0), None),
            Some(two)
        );
        assert_eq!(
            active_after(&pages, Pos2::new(300.0, 100.0), None),
            Some(one)
        );
        assert_eq!(active_after(&pages, Pos2::new(20.0, 100.0), None), None);
        assert_eq!(
            active_after(&pages, Pos2::new(300.0, 470.0), None),
            None,
            "the gap"
        );
        assert_eq!(
            active_after(&pages, Pos2::new(20.0, 100.0), Some(two)),
            Some(two),
            "the picked element's page wins"
        );
    }

    // S50-03: the selection's page, else the active one, else the first;
    // a page that is gone does not count.
    #[test]
    fn s50_03_current_page() {
        let pages = [PageId::new(1), PageId::new(2), PageId::new(3)];
        let (one, two, three) = (pages[0], pages[1], pages[2]);
        assert_eq!(current_page_id(&pages, Some(three), Some(two)), Some(three));
        assert_eq!(current_page_id(&pages, None, Some(two)), Some(two));
        assert_eq!(current_page_id(&pages, None, None), Some(one));
        assert_eq!(
            current_page_id(&pages, None, Some(PageId::new(9))),
            Some(one)
        );
        assert_eq!(
            current_page_id(&pages, Some(PageId::new(9)), Some(two)),
            Some(two)
        );
        assert_eq!(current_page_id(&[], None, Some(two)), None);
    }

    // S49-05: «n de N» counts from the page of the selection.
    #[test]
    fn s49_05_page_number() {
        let pages = [PageId::new(7), PageId::new(3), PageId::new(9)];
        assert_eq!(page_number(&pages, Some(PageId::new(9))), 3);
        assert_eq!(page_number(&pages, Some(PageId::new(7))), 1);
        assert_eq!(page_number(&pages, None), 1);
        assert_eq!(page_number(&pages, Some(PageId::new(1))), 1);
        assert_eq!(page_number(&[], None), 0);
    }

    // S48-01, S48-02: the eye hides and the lock locks one element, each
    // one undo step; a hidden element leaves the selection.
    #[test]
    fn s48_01_the_eye_and_the_lock_are_one_undo_step_each() {
        let mut engine = engine();
        let mut editor = Editor::default();
        let (page, [a, b, _]) = three_pieces(&mut engine, &mut editor);
        let depth = undo_depth(&mut engine);
        let flag = |nodes: Vec<NodeId>, which, on| Asked {
            flag: Some((page, nodes, which, on)),
            ..Asked::default()
        };
        ask(&mut engine, &mut editor, flag(vec![a], Flag::Locked, true));
        assert_eq!(bases(&mut engine, &[a])[0], (true, true, None));
        assert_eq!(undo_depth(&mut engine), depth + 1);
        editor.selected = Selection::of(page, [a, b]);
        ask(&mut engine, &mut editor, flag(vec![b], Flag::Hidden, true));
        assert_eq!(bases(&mut engine, &[b])[0], (false, false, None));
        assert_eq!(undo_depth(&mut engine), depth + 2);
        let document = engine.edit().document();
        assert_eq!(editor.prune(document.as_deref()).len(), 1, "b left it");
        engine.edit().undo().expect("an undo");
        assert_eq!(bases(&mut engine, &[b])[0], (true, false, None));
    }

    // S48-03, S48-04: Ctrl+Shift+L locks a mixed selection in one step and
    // again frees it; the arrows do not move what is locked; Ctrl+Shift+H
    // hides it all.
    #[test]
    fn s48_03_the_shortcuts_lock_and_hide_the_selection() {
        let mut engine = engine();
        let mut editor = Editor::default();
        let (page, [a, b, c]) = three_pieces(&mut engine, &mut editor);
        let ctx = egui::Context::default();
        let lock = Modifiers::COMMAND.plus(Modifiers::SHIFT);
        ask(
            &mut engine,
            &mut editor,
            Asked {
                flag: Some((page, vec![a], Flag::Locked, true)),
                ..Asked::default()
            },
        );
        editor.selected = Selection::of(page, [a, b]);
        let depth = undo_depth(&mut engine);
        frame_with(&ctx, &mut editor, &mut engine, Some(Key::L), lock);
        assert_eq!(bases(&mut engine, &[a, b]), vec![(true, true, None); 2]);
        assert_eq!(undo_depth(&mut engine), depth + 1, "one step");
        editor.selected = Selection::of(page, [a, b, c]);
        let before = placements(&mut engine, &[a, b, c]);
        frame_with(
            &ctx,
            &mut editor,
            &mut engine,
            Some(Key::ArrowRight),
            Modifiers::NONE,
        );
        let after = placements(&mut engine, &[a, b, c]);
        assert_eq!((after[0], after[1]), (before[0], before[1]), "locked stay");
        assert_eq!(after[2].x, before[2].x + 1.0, "the free one moves");
        editor.selected = Selection::of(page, [a, b]);
        frame_with(&ctx, &mut editor, &mut engine, Some(Key::L), lock);
        assert_eq!(bases(&mut engine, &[a, b]), vec![(true, false, None); 2]);
        frame_with(&ctx, &mut editor, &mut engine, Some(Key::H), lock);
        assert_eq!(bases(&mut engine, &[a, b]), vec![(false, false, None); 2]);
    }

    // S48-05: a typed name is stored trimmed in one undo step; an empty
    // one goes back to the name by kind.
    #[test]
    fn s48_05_a_rename_is_one_undo_step() {
        let mut engine = engine();
        let mut editor = Editor::default();
        let (page, [a, _, _]) = three_pieces(&mut engine, &mut editor);
        let rename = |name: Option<&str>| Asked {
            rename: Some((page, a, name.map(str::to_owned))),
            ..Asked::default()
        };
        ask(&mut engine, &mut editor, rename(Some("Fondo")));
        assert_eq!(bases(&mut engine, &[a])[0].2.as_deref(), Some("Fondo"));
        ask(&mut engine, &mut editor, rename(None));
        assert_eq!(bases(&mut engine, &[a])[0].2, None);
        engine.edit().undo().expect("an undo");
        assert_eq!(bases(&mut engine, &[a])[0].2.as_deref(), Some("Fondo"));
    }

    // S48-06: Alt+↑ brings the selected element one step forward and
    // Alt+↓ sends it one back; at the end of the list nothing happens.
    #[test]
    fn s48_06_alt_arrows_reorder_one_step() {
        let mut engine = engine();
        let mut editor = Editor::default();
        let (page, [a, b, c]) = three_pieces(&mut engine, &mut editor);
        let ctx = egui::Context::default();
        let roots = |engine: &mut Gafira| {
            engine.edit().document().expect("a document").pages()[0]
                .roots
                .clone()
        };
        assert_eq!(roots(&mut engine), [a, b, c]);
        editor.selected = Selection::one(page, a);
        frame_with(
            &ctx,
            &mut editor,
            &mut engine,
            Some(Key::ArrowUp),
            Modifiers::ALT,
        );
        assert_eq!(roots(&mut engine), [b, a, c]);
        frame_with(
            &ctx,
            &mut editor,
            &mut engine,
            Some(Key::ArrowDown),
            Modifiers::ALT,
        );
        frame_with(
            &ctx,
            &mut editor,
            &mut engine,
            Some(Key::ArrowDown),
            Modifiers::ALT,
        );
        assert_eq!(roots(&mut engine), [a, b, c], "the back stays the back");
    }

    fn grouped(engine: &mut Gafira, editor: &mut Editor) -> (PageId, NodeId, [NodeId; 3]) {
        let (page, [a, b, c]) = three_pieces(engine, editor);
        editor.selected = Selection::of(page, [a, b]);
        ask(
            engine,
            editor,
            Asked {
                group: true,
                ..Asked::default()
            },
        );
        let group = editor.selected.single().expect("the group").1;
        (page, group, [a, b, c])
    }

    // S43-14: a selected group drags, turns, aligns and deletes as one,
    // with every element in it, one undo step each.
    #[test]
    fn s43_14_a_selected_group_moves_turns_and_deletes_as_one() {
        let mut engine = engine();
        let mut editor = Editor::default();
        let (page, group, [a, b, c]) = grouped(&mut engine, &mut editor);
        let before = placements(&mut engine, &[a, b, c]);
        let depth = undo_depth(&mut engine);
        editor.zoom = 2.0;
        let step = Asked {
            drag_to: Some(Vec2::new(10.0, 4.0)),
            ..Asked::default()
        };
        gesture(
            &mut engine,
            &mut editor,
            vec![
                Asked {
                    drag_begin: Some(None),
                    ..Asked::default()
                },
                step,
                Asked {
                    drag_to: Some(Vec2::new(10.0, 4.0)),
                    ..Asked::default()
                },
                Asked {
                    drag_end: true,
                    ..Asked::default()
                },
            ],
        );
        let moved = placements(&mut engine, &[a, b, c]);
        for i in 0..2 {
            assert_eq!(moved[i].x - before[i].x, 10.0, "{i}");
            assert_eq!(moved[i].y - before[i].y, 4.0);
        }
        assert_eq!(moved[2], before[2], "c is not in the group");
        assert_eq!(undo_depth(&mut engine), depth + 1, "one step for the drag");
        // A turn goes about the group's centre, to each child.
        ask(
            &mut engine,
            &mut editor,
            Asked {
                turn: Some(90.0),
                ..Asked::default()
            },
        );
        let turned = placements(&mut engine, &[a, b]);
        assert!(turned.iter().all(|p| p.rotation_deg == 90.0), "{turned:?}");
        assert_eq!(undo_depth(&mut engine), depth + 2);
        ask(
            &mut engine,
            &mut editor,
            Asked {
                delete: true,
                ..Asked::default()
            },
        );
        assert_eq!(roots_now(&mut engine), [c]);
        assert_eq!(the_shapes(&mut engine).len(), 1);
        engine.edit().undo().expect("an undo");
        assert_eq!(roots_now(&mut engine), [group, c]);
        assert_eq!(the_shapes(&mut engine).len(), 3);
        // The Inspector's box of the group reframes its children.
        editor.selected = Selection::one(page, group);
        let document = engine.edit().document();
        let shape = selected_shapes(document.as_deref(), &editor.selected);
        assert!(shape[0].group);
        let mut wide = shape[0].placement;
        wide.w *= 2.0;
        ask(
            &mut engine,
            &mut editor,
            Asked {
                place: Some(wide),
                ..Asked::default()
            },
        );
        let document = engine.edit().document();
        let now = selected_shapes(document.as_deref(), &editor.selected);
        assert!((now[0].placement.w - wide.w).abs() < 1e-6, "{:?}", now[0]);
    }

    // S43-15: Ctrl+D duplicates the selection with new ids, moved by the
    // copy offset and selected, in one undo step.
    #[test]
    fn s43_15_ctrl_d_duplicates_offset_and_selected() {
        let mut engine = engine();
        let mut editor = Editor::default();
        let (page, [a, _, c]) = three_pieces(&mut engine, &mut editor);
        let name = SetProperty::new(page, vec![a], Property::Name(Some("Logo".into())), None);
        engine.edit().execute(Box::new(name)).expect("a name");
        editor.selected = Selection::of(page, [a, c]);
        let depth = undo_depth(&mut engine);
        let ctx = egui::Context::default();
        frame(&ctx, &mut editor, &mut engine, Some(Key::D));
        assert_eq!(the_shapes(&mut engine).len(), 5);
        let copies = editor.selected.nodes().to_vec();
        assert_eq!(copies.len(), 2);
        assert!(!copies.contains(&a) && !copies.contains(&c));
        let from = placements(&mut engine, &[a, c]);
        let to = placements(&mut engine, &copies);
        for (copy, origin) in to.iter().zip(&from) {
            assert_eq!((copy.x - origin.x, copy.y - origin.y), (10.0, 10.0));
        }
        let document = engine.edit().document().expect("a document");
        let named = document.pages()[0].node(copies[0]).expect("the copy");
        assert_eq!(named.base.name.as_deref(), Some("Logo copia"));
        assert_eq!(roots_now(&mut engine).last(), copies.last(), "on top of c");
        assert_eq!(undo_depth(&mut engine), depth + 1);
    }

    // S43-16: Ctrl+C puts the selection in the clipboard and its plain text
    // on the system clipboard; the paste of that text pastes it moved off
    // its original, other text pastes nothing, Pegar in the menu pastes
    // too, and Ctrl+X cuts in one undo step.
    #[test]
    fn s43_16_copy_cut_and_paste_through_the_clipboard() {
        let mut engine = engine();
        let mut editor = Editor::default();
        let (page, [a, ..]) = three_pieces(&mut engine, &mut editor);
        editor.selected = Selection::one(page, a);
        let ctx = egui::Context::default();
        let mut session = None;
        let out = frame_events(
            &ctx,
            &mut editor,
            &mut engine,
            vec![egui::Event::Copy],
            Modifiers::COMMAND,
            &mut session,
        );
        let copied = out.commands.iter().find_map(|c| match c {
            egui::OutputCommand::CopyText(text) => Some(text.clone()),
            _ => None,
        });
        assert_eq!(copied.as_deref(), Some("Rectángulo"));
        assert_eq!(
            session.as_ref().map(|c| c.text.as_str()),
            Some("Rectángulo")
        );
        let depth = undo_depth(&mut engine);
        let paste = |text: &str| vec![egui::Event::Paste(text.to_owned())];
        frame_events(
            &ctx,
            &mut editor,
            &mut engine,
            paste("Rectángulo"),
            Modifiers::COMMAND,
            &mut session,
        );
        assert_eq!(the_shapes(&mut engine).len(), 4);
        let pasted = editor.selected.single().expect("the paste").1;
        let [origin, copy] = placements(&mut engine, &[a, pasted])[..] else {
            panic!("two placements")
        };
        assert_eq!((copy.x - origin.x, copy.y - origin.y), (10.0, 10.0));
        assert_eq!(undo_depth(&mut engine), depth + 1);
        ask(
            &mut engine,
            &mut editor,
            Asked {
                paste: true,
                ..Asked::default()
            },
        );
        assert_eq!(the_shapes(&mut engine).len(), 5, "Edición → Pegar");
        let last = editor.selected.single().expect("the paste").1;
        assert_eq!(placements(&mut engine, &[last])[0].x - origin.x, 20.0);
        frame_events(
            &ctx,
            &mut editor,
            &mut engine,
            vec![egui::Event::Cut],
            Modifiers::COMMAND,
            &mut session,
        );
        assert_eq!(the_shapes(&mut engine).len(), 4);
        assert!(editor.selected.is_empty());
        engine.edit().undo().expect("an undo");
        assert_eq!(the_shapes(&mut engine).len(), 5, "one undo brings it back");
        // The clipboard of the session reaches another project.
        let mut other = engine_with_new_document();
        let mut fresh = Editor::default();
        frame_events(
            &ctx,
            &mut fresh,
            &mut other,
            paste("Rectángulo"),
            Modifiers::COMMAND,
            &mut session,
        );
        assert_eq!(the_shapes(&mut other).len(), 1);
    }

    fn engine_with_new_document() -> Gafira {
        let mut engine = engine();
        start(&mut engine, StartFormat::Social).expect("a new document");
        engine
    }

    // S43-17: Esc inside a group selects the group and leaves it; Ctrl+A in
    // a group selects its children; Alt+H aligns to the centre (D-88
    // replaced Ctrl+Alt+C, which the window turns into a copy).
    #[test]
    fn s43_17_escape_leaves_the_group_and_alt_h_aligns() {
        let mut engine = engine();
        let mut editor = Editor::default();
        let (page, group, [a, b, c]) = grouped(&mut engine, &mut editor);
        let ctx = egui::Context::default();
        editor.scope = Some((page, group));
        editor.selected = Selection::one(page, a);
        frame(&ctx, &mut editor, &mut engine, Some(Key::A));
        assert_eq!(editor.selected, Selection::of(page, [a, b]));
        frame_with(
            &ctx,
            &mut editor,
            &mut engine,
            Some(Key::Escape),
            Modifiers::NONE,
        );
        assert_eq!(editor.selected, Selection::one(page, group));
        assert_eq!(editor.scope, None);
        frame_with(
            &ctx,
            &mut editor,
            &mut engine,
            Some(Key::Escape),
            Modifiers::NONE,
        );
        assert!(editor.selected.is_empty());
        editor.selected = Selection::of(page, [group, c]);
        let before = placements(&mut engine, &[a, b, c]);
        frame_with(&ctx, &mut editor, &mut engine, Some(Key::H), Modifiers::ALT);
        assert_ne!(placements(&mut engine, &[a, b, c]), before, "it aligned");
        let document = engine.edit().document();
        let boxes = selected_shapes(document.as_deref(), &editor.selected);
        let centre = |p: &Placement| p.x + p.w / 2.0;
        assert!(
            (centre(&boxes[0].placement) - centre(&boxes[1].placement)).abs() < 1e-6,
            "the group and c share their centre: {boxes:?}"
        );
    }

    // S43-18: Agrupar needs a selection and Desagrupar a group; the order
    // actions work among the children of a group.
    #[test]
    fn s43_18_group_actions_are_off_when_they_cannot_act() {
        let mut engine = engine();
        let mut editor = Editor::default();
        let (page, group, [a, b, c]) = grouped(&mut engine, &mut editor);
        let document = engine.edit().document();
        let can = |selected: Selection| CanArrange::of(document.as_deref(), &selected);
        assert_eq!(can(Selection::default()), CanArrange::default());
        let one = can(Selection::one(page, c));
        assert!(one.group && !one.ungroup);
        assert!(can(Selection::of(page, [group, c])).ungroup);
        // Inside the group: a is at the back of its two siblings.
        assert_eq!(
            can(Selection::one(page, a)).order,
            [true, false, true, false]
        );
        assert_eq!(
            can(Selection::one(page, b)).order,
            [false, true, false, true]
        );
        assert_eq!(
            can(Selection::of(page, [a, c])).order,
            [false; 4],
            "two lists"
        );
    }

    // S44-01: Alt+W, Alt+V and Alt+S align to the top, middle and bottom
    // (D-88). Ctrl+Alt with any Align letter (AltGr on Windows) and the old
    // Ctrl+Alt+L/C/R do not align, and a copy with Alt held is a copy.
    #[test]
    fn s44_01_alt_letters_align_and_ctrl_alt_does_not() {
        let mut engine = engine();
        let mut editor = Editor::default();
        let (page, nodes) = three_pieces(&mut engine, &mut editor);
        let ctx = egui::Context::default();
        let before = placements(&mut engine, &nodes);
        let top = before.iter().map(|p| p.y).fold(f64::INFINITY, f64::min);
        let low = before.iter().map(|p| p.y).fold(f64::NEG_INFINITY, f64::max);
        let command_alt = Modifiers::COMMAND.plus(Modifiers::ALT);
        for key in [
            Key::A,
            Key::H,
            Key::D,
            Key::W,
            Key::V,
            Key::L,
            Key::C,
            Key::R,
        ] {
            frame_with(&ctx, &mut editor, &mut engine, Some(key), command_alt);
        }
        assert_eq!(
            placements(&mut engine, &nodes),
            before,
            "Ctrl+Alt never aligns"
        );
        editor.selected = Selection::of(page, nodes);
        let ys = |engine: &mut Gafira| {
            placements(engine, &nodes)
                .iter()
                .map(|p| p.y)
                .collect::<Vec<_>>()
        };
        for (key, y) in [
            (Key::W, top),
            (Key::S, low),
            (Key::V, f64::midpoint(top, low)),
        ] {
            frame_with(&ctx, &mut editor, &mut engine, Some(key), Modifiers::ALT);
            assert!(
                ys(&mut engine).iter().all(|at| (at - y).abs() < 1e-9),
                "{key:?}"
            );
            engine.edit().undo().expect("undoes");
            assert_eq!(placements(&mut engine, &nodes), before);
        }
        let mut session = None;
        frame_events(
            &ctx,
            &mut editor,
            &mut engine,
            vec![egui::Event::Copy],
            command_alt,
            &mut session,
        );
        assert!(session.is_some(), "a copy");
        assert_eq!(placements(&mut engine, &nodes), before, "no align");
    }

    /// A drag of the selection by `offset` screen px, with Ctrl held (the
    /// magnet lets go, D-90) or not.
    fn drag_by(
        engine: &mut Gafira,
        editor: &mut Editor,
        hold: Option<Hold>,
        offset: Vec2,
        free: bool,
    ) {
        gesture(
            engine,
            editor,
            vec![
                Asked {
                    drag_begin: Some(hold),
                    ..Asked::default()
                },
                Asked {
                    drag_to: Some(offset),
                    free,
                    ..Asked::default()
                },
            ],
        );
    }

    fn drag_end(engine: &mut Gafira, editor: &mut Editor) {
        ask(
            engine,
            editor,
            Asked {
                drag_end: true,
                ..Asked::default()
            },
        );
    }

    // S44-13: a move snaps the box to the page centre within 6 screen px,
    // draws the smart guide while it holds, is one undo step, and lets go
    // with Ctrl (Alt until D-90) or beyond the threshold.
    #[test]
    fn s44_13_a_drag_snaps_to_the_page_centre_and_ctrl_lets_go() {
        let mut engine = engine();
        let mut editor = Editor::default();
        let (page, rect, _) = two_pieces(&mut engine, &mut editor);
        editor.zoom = 1.0;
        editor.selected = Selection::one(page, rect);
        let origin = placements(&mut engine, &[rect])[0];
        assert_eq!(origin.x + origin.w / 2.0, 540.0, "centred on the page");
        let depth = undo_depth(&mut engine);
        drag_by(&mut engine, &mut editor, None, Vec2::new(3.0, 0.0), false);
        assert_eq!(
            placements(&mut engine, &[rect])[0].x,
            origin.x,
            "back on the centre"
        );
        let (on, snap) = editor.snapped.expect("a smart guide");
        assert_eq!((on, snap.x), (page, Some(540.0)));
        drag_end(&mut engine, &mut editor);
        assert_eq!(editor.snapped, None);
        assert_eq!(
            undo_depth(&mut engine),
            depth,
            "a move back to where it was"
        );
        drag_by(&mut engine, &mut editor, None, Vec2::new(3.0, 0.0), true);
        assert_eq!(
            placements(&mut engine, &[rect])[0].x,
            origin.x + 3.0,
            "Ctrl"
        );
        assert_eq!(editor.snapped, None);
        drag_end(&mut engine, &mut editor);
        drag_by(&mut engine, &mut editor, None, Vec2::new(7.0, 0.0), false);
        assert_eq!(
            placements(&mut engine, &[rect])[0].x,
            origin.x + 10.0,
            "10 px off"
        );
        drag_end(&mut engine, &mut editor);
        assert_eq!(undo_depth(&mut engine), depth + 2);
        editor.shown.magnet = false;
        drag_by(&mut engine, &mut editor, None, Vec2::new(-7.0, 0.0), false);
        assert_eq!(
            placements(&mut engine, &[rect])[0].x,
            origin.x + 3.0,
            "Imán off"
        );
        drag_end(&mut engine, &mut editor);
    }

    // S44-14: a resize snaps the dragged edge to another element's edge
    // and keeps the opposite one; at 200 % the threshold is 3 px of page.
    #[test]
    fn s44_14_a_resize_snaps_the_dragged_edge() {
        let mut engine = engine();
        let mut editor = Editor::default();
        let (page, rect, ellipse) = two_pieces(&mut engine, &mut editor);
        let target = placements(&mut engine, &[ellipse])[0].x;
        editor.selected = Selection::one(page, rect);
        let origin = placements(&mut engine, &[rect])[0];
        let right = canvas::Handle { x: 1, y: 0 };
        editor.zoom = 2.0;
        let gap = target - (origin.x + origin.w);
        // 3 px of page short of the ellipse: 6 screen px at 200 %.
        let offset = Vec2::new(((gap - 3.0) * 2.0) as f32, 0.0);
        drag_by(
            &mut engine,
            &mut editor,
            Some(Hold::Resize(right)),
            offset,
            false,
        );
        let to = placements(&mut engine, &[rect])[0];
        assert_eq!(to.x, origin.x, "the left edge stays");
        assert_eq!(to.x + to.w, target, "the right edge on the ellipse");
        assert_eq!((to.y, to.h), (origin.y, origin.h));
        drag_end(&mut engine, &mut editor);
        engine.edit().undo().expect("an undo");
        let offset = Vec2::new(((gap - 3.5) * 2.0) as f32, 0.0);
        drag_by(
            &mut engine,
            &mut editor,
            Some(Hold::Resize(right)),
            offset,
            false,
        );
        let to = placements(&mut engine, &[rect])[0];
        assert!(
            (to.x + to.w - (target - 3.5)).abs() < 1e-3,
            "3.5 px is too far"
        );
        drag_end(&mut engine, &mut editor);
    }

    // S44-15: arrows nudge 1 px and Shift+arrows 10 px; the presses less
    // than 500 ms apart are one undo step (02 §4.3); with Ctrl or Alt they
    // do not nudge; a group nudges with its children.
    #[test]
    fn s44_15_arrows_nudge_and_merge_into_one_undo_step() {
        let mut engine = engine();
        let mut editor = Editor::default();
        let (page, group, [a, b, c]) = grouped(&mut engine, &mut editor);
        let ctx = egui::Context::default();
        editor.selected = Selection::of(page, [group, c]);
        let before = placements(&mut engine, &[a, b, c]);
        let depth = undo_depth(&mut engine);
        for _ in 0..3 {
            frame_with(
                &ctx,
                &mut editor,
                &mut engine,
                Some(Key::ArrowRight),
                Modifiers::NONE,
            );
        }
        frame_with(
            &ctx,
            &mut editor,
            &mut engine,
            Some(Key::ArrowDown),
            Modifiers::SHIFT,
        );
        let after = placements(&mut engine, &[a, b, c]);
        for (from, to) in before.iter().zip(&after) {
            assert_eq!((to.x - from.x, to.y - from.y), (3.0, 10.0));
        }
        assert_eq!(undo_depth(&mut engine), depth + 1, "one step");
        for modifiers in [Modifiers::COMMAND, Modifiers::ALT] {
            frame_with(
                &ctx,
                &mut editor,
                &mut engine,
                Some(Key::ArrowLeft),
                modifiers,
            );
        }
        assert_eq!(placements(&mut engine, &[a, b, c]), after);
        engine.edit().undo().expect("an undo");
        assert_eq!(placements(&mut engine, &[a, b, c]), before);
        editor.selected.clear();
        frame_with(
            &ctx,
            &mut editor,
            &mut engine,
            Some(Key::ArrowLeft),
            Modifiers::NONE,
        );
        assert_eq!(
            placements(&mut engine, &[a, b, c]),
            before,
            "nothing selected"
        );
    }

    // S44-16: a guide created, moved or deleted is one undo step each,
    // never merged with the next.
    #[test]
    fn s44_16_guide_changes_are_one_undo_step_each() {
        let mut engine = engine();
        let mut editor = Editor::default();
        let (page, _, _) = two_pieces(&mut engine, &mut editor);
        let guides_now = |engine: &mut Gafira| {
            engine.edit().document().expect("a document").pages()[0]
                .guides
                .clone()
        };
        let depth = undo_depth(&mut engine);
        let one = ruler::added(&GuideSet::default(), GuideAxis::Vertical, 100.0);
        let moved = ruler::moved(&one, 0, 250.0);
        for set in [one.clone(), moved.clone()] {
            ask(
                &mut engine,
                &mut editor,
                Asked {
                    guides: Some((page, set)),
                    ..Asked::default()
                },
            );
        }
        assert_eq!(guides_now(&mut engine), moved);
        assert_eq!(undo_depth(&mut engine), depth + 2);
        engine.edit().undo().expect("an undo");
        assert_eq!(guides_now(&mut engine), one);
        engine.edit().undo().expect("an undo");
        assert_eq!(guides_now(&mut engine), GuideSet::default());
    }

    // S44-17: Ctrl+Shift+R, Ctrl+; and Ctrl+Shift+' switch the rulers, the
    // guides and the magnet, and the settings keep them.
    #[test]
    fn s44_17_view_shortcuts_switch_and_persist() {
        let mut engine = engine();
        let mut editor = Editor::default();
        let ctx = egui::Context::default();
        start(&mut engine, StartFormat::Social).expect("a new document");
        frame(&ctx, &mut editor, &mut engine, None);
        editor.shown = ViewSettings::default();
        let command_shift = Modifiers::COMMAND.plus(Modifiers::SHIFT);
        frame_with(&ctx, &mut editor, &mut engine, Some(Key::R), command_shift);
        assert_eq!(editor.shown, ViewSettings::new(false, true, true));
        frame_with(
            &ctx,
            &mut editor,
            &mut engine,
            Some(Key::Semicolon),
            Modifiers::COMMAND,
        );
        assert_eq!(editor.shown, ViewSettings::new(false, false, true));
        frame_with(
            &ctx,
            &mut editor,
            &mut engine,
            Some(Key::Quote),
            command_shift,
        );
        assert_eq!(editor.shown, ViewSettings::new(false, false, false));
        assert_eq!(engine.settings().get().view, editor.shown, "persisted");
        frame_with(
            &ctx,
            &mut editor,
            &mut engine,
            Some(Key::R),
            Modifiers::COMMAND,
        );
        assert!(!editor.shown.rulers, "Ctrl+R alone is not Reglas");
        let reset = engine.settings().get().with_view(ViewSettings::default());
        engine.settings().set(reset).expect("resets");
    }

    // S44-19: frames draw the rulers, the guides and a smart guide, and a
    // guide hidden or shown changes nothing in the document.
    #[test]
    fn s44_19_frames_draw_rulers_guides_and_smart_guides() {
        let mut engine = engine();
        let mut editor = Editor::default();
        let (page, rect, _) = two_pieces(&mut engine, &mut editor);
        let ctx = egui::Context::default();
        theme().apply(&ctx);
        let set = ruler::added(&GuideSet::default(), GuideAxis::Horizontal, 300.0);
        ask(
            &mut engine,
            &mut editor,
            Asked {
                guides: Some((page, ruler::added(&set, GuideAxis::Vertical, 200.0))),
                ..Asked::default()
            },
        );
        editor.selected = Selection::one(page, rect);
        editor.snapped = Some((
            page,
            Snap {
                x: Some(540.0),
                y: Some(300.0),
                ..Snap::default()
            },
        ));
        frame(&ctx, &mut editor, &mut engine, None);
        editor.shown.rulers = true;
        editor.shown.guides = true;
        frame(&ctx, &mut editor, &mut engine, None);
        editor.shown.guides = false;
        frame(&ctx, &mut editor, &mut engine, None);
        assert_eq!(
            engine.edit().document().expect("a document").pages()[0]
                .guides
                .0
                .len(),
            2
        );
    }

    // S45-09: a corner keeps the proportions in one undo step, Shift frees
    // them, the magnet snaps the driving edge and the other follows; while
    // the handle is held the frame shows its resize cursor and the size
    // label.
    #[test]
    fn s45_09_a_corner_keeps_the_proportions_through_the_editor() {
        let mut engine = engine();
        let mut editor = Editor::default();
        let (page, rect, _) = two_pieces(&mut engine, &mut editor);
        editor.zoom = 1.0;
        editor.selected = Selection::one(page, rect);
        let origin = placements(&mut engine, &[rect])[0];
        let corner = Some(Hold::Resize(canvas::Handle { x: 1, y: 1 }));
        let depth = undo_depth(&mut engine);
        let drag = |engine: &mut Gafira, editor: &mut Editor, offset: Vec2, shift: bool| {
            gesture(
                engine,
                editor,
                vec![
                    Asked {
                        drag_begin: Some(corner),
                        ..Asked::default()
                    },
                    Asked {
                        drag_to: Some(offset),
                        snap: shift,
                        ..Asked::default()
                    },
                ],
            );
        };
        drag(&mut engine, &mut editor, Vec2::new(30.0, 5.0), false);
        let to = placements(&mut engine, &[rect])[0];
        assert_eq!((to.x, to.y, to.w, to.h), (origin.x, origin.y, 390.0, 390.0));
        // The frame while the handle is held.
        let ctx = egui::Context::default();
        theme().apply(&ctx);
        let mut session = None;
        let output = frame_events(
            &ctx,
            &mut editor,
            &mut engine,
            vec![],
            Modifiers::NONE,
            &mut session,
        );
        assert_eq!(output.cursor_icon, egui::CursorIcon::ResizeNwSe);
        // The frame fits the zoom to its window; the drags below are at 100 %.
        editor.zoom = 1.0;
        drag_end(&mut engine, &mut editor);
        assert_eq!(undo_depth(&mut engine), depth + 1);
        engine.edit().undo().expect("an undo");
        drag(&mut engine, &mut editor, Vec2::new(30.0, 15.0), true);
        let to = placements(&mut engine, &[rect])[0];
        assert_eq!((to.w, to.h), (390.0, 375.0), "Shift frees them");
        drag_end(&mut engine, &mut editor);
        engine.edit().undo().expect("an undo");
        // 3 px short of the ellipse: the right edge snaps and the height
        // follows.
        drag(&mut engine, &mut editor, Vec2::new(37.0, 0.0), false);
        let to = placements(&mut engine, &[rect])[0];
        assert_eq!((to.x, to.y, to.w, to.h), (origin.x, origin.y, 400.0, 400.0));
        assert_eq!(editor.snapped.and_then(|(_, s)| s.x), Some(760.0));
        drag_end(&mut engine, &mut editor);
    }

    // S47-07: Alt resizes from the centre and Ctrl lets the magnet go,
    // through the editor (D-90); Ctrl held mid-drag fires nothing else.
    #[test]
    fn s47_07_alt_resizes_from_the_centre_and_ctrl_frees_the_magnet() {
        let mut engine = engine();
        let mut editor = Editor::default();
        let (page, rect, _) = two_pieces(&mut engine, &mut editor);
        editor.zoom = 1.0;
        editor.selected = Selection::one(page, rect);
        let origin = placements(&mut engine, &[rect])[0];
        assert_eq!((origin.x, origin.y, origin.w), (360.0, 360.0, 360.0));
        let corner = canvas::Handle { x: 1, y: 1 };
        let right = canvas::Handle { x: 1, y: 0 };
        let depth = undo_depth(&mut engine);
        // One drag of `handle` by `offset` with these keys, then the box.
        let drag = |engine: &mut Gafira,
                    editor: &mut Editor,
                    handle: canvas::Handle,
                    offset: Vec2,
                    keys: resize::DragKeys| {
            gesture(
                engine,
                editor,
                vec![
                    Asked {
                        drag_begin: Some(Some(Hold::Resize(handle))),
                        ..Asked::default()
                    },
                    Asked {
                        drag_to: Some(offset),
                        snap: keys.shift,
                        centre: keys.centre,
                        free: keys.free,
                        ..Asked::default()
                    },
                ],
            );
            let to = placements(engine, &[rect])[0];
            (to.x, to.y, to.w, to.h)
        };
        // The keys reach the drag as the canvas reads them.
        let mut asked = Asked::default();
        asked.hold_keys(resize::drag_keys(Modifiers::ALT.plus(Modifiers::SHIFT)));
        assert_eq!((asked.snap, asked.centre, asked.free), (true, true, false));
        asked.hold_keys(resize::drag_keys(Modifiers::COMMAND));
        assert_eq!((asked.snap, asked.centre, asked.free), (false, false, true));
        let alt = resize::drag_keys(Modifiers::ALT);
        let at = drag(&mut engine, &mut editor, corner, Vec2::new(30.0, 5.0), alt);
        assert_eq!(at, (330.0, 330.0, 420.0, 420.0), "kept, from the centre");
        // The frame while the handle is held, with Ctrl down and no key:
        // no copy, no edit, and the drag goes on.
        let ctx = egui::Context::default();
        theme().apply(&ctx);
        let mut session = None;
        let before = placements(&mut engine, &[rect]);
        frame_events(
            &ctx,
            &mut editor,
            &mut engine,
            vec![],
            Modifiers::COMMAND,
            &mut session,
        );
        assert!(session.is_none(), "no copy");
        assert!(editor.drag.is_some(), "still dragging");
        assert_eq!(placements(&mut engine, &[rect]), before, "no edit");
        editor.zoom = 1.0;
        drag_end(&mut engine, &mut editor);
        assert_eq!(undo_depth(&mut engine), depth + 1, "one undo step");
        engine.edit().undo().expect("an undo");
        let alt_shift = resize::drag_keys(Modifiers::ALT.plus(Modifiers::SHIFT));
        let at = drag(
            &mut engine,
            &mut editor,
            corner,
            Vec2::new(30.0, 15.0),
            alt_shift,
        );
        assert_eq!(at, (330.0, 345.0, 420.0, 390.0), "free, from the centre");
        drag_end(&mut engine, &mut editor);
        engine.edit().undo().expect("an undo");
        // The right edge 3 px short of the ellipse (760) snaps; the left
        // one mirrors it.
        let at = drag(&mut engine, &mut editor, right, Vec2::new(37.0, 0.0), alt);
        assert_eq!(at, (320.0, 360.0, 440.0, 360.0));
        assert_eq!(editor.snapped.and_then(|(_, s)| s.x), Some(760.0));
        drag_end(&mut engine, &mut editor);
        engine.edit().undo().expect("an undo");
        let alt_ctrl = resize::drag_keys(Modifiers::ALT.plus(Modifiers::COMMAND));
        let at = drag(
            &mut engine,
            &mut editor,
            right,
            Vec2::new(37.0, 0.0),
            alt_ctrl,
        );
        assert_eq!(at, (323.0, 360.0, 434.0, 360.0), "Alt+Ctrl: no magnet");
        assert_eq!(editor.snapped, None);
        drag_end(&mut engine, &mut editor);
        engine.edit().undo().expect("an undo");
        let ctrl = resize::drag_keys(Modifiers::COMMAND);
        let at = drag(&mut engine, &mut editor, right, Vec2::new(37.0, 0.0), ctrl);
        assert_eq!(at, (360.0, 360.0, 397.0, 360.0), "Ctrl: no magnet");
        drag_end(&mut engine, &mut editor);
        // A group grows from the centre of its box.
        let mut engine = self::engine();
        let mut editor = Editor::default();
        let (page, group, _) = grouped(&mut engine, &mut editor);
        editor.zoom = 1.0;
        editor.selected = Selection::one(page, group);
        let frame = |engine: &mut Gafira, editor: &Editor| {
            let document = engine.edit().document();
            selected_shapes(document.as_deref(), &editor.selected)[0].placement
        };
        let from = frame(&mut engine, &editor);
        let both = resize::drag_keys(Modifiers::ALT.plus(Modifiers::COMMAND));
        gesture(
            &mut engine,
            &mut editor,
            vec![
                Asked {
                    drag_begin: Some(Some(Hold::Resize(right))),
                    ..Asked::default()
                },
                Asked {
                    drag_to: Some(Vec2::new(20.0, 0.0)),
                    centre: both.centre,
                    free: both.free,
                    ..Asked::default()
                },
            ],
        );
        let to = frame(&mut engine, &editor);
        assert!((to.w - from.w - 40.0).abs() < 1e-6, "{from:?} {to:?}");
        assert!((to.x + to.w / 2.0 - from.x - from.w / 2.0).abs() < 1e-6);
        assert!((to.h - from.h).abs() < 1e-6);
        drag_end(&mut engine, &mut editor);
    }
}
